#!/usr/bin/env python3
"""Human-operated verification smoke test. Never prints or persists credentials."""
import argparse
import getpass
import json
import os
import re
from pathlib import Path
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import warnings

ROOT = Path(__file__).resolve().parents[1]
_NO_BODY = object()

class VerificationError(Exception):
    """Contains only fixed, non-secret diagnostics."""

class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None

class Api:
    def __init__(self, base):
        parsed = urllib.parse.urlsplit(base)
        if (parsed.scheme not in ("http", "https") or not parsed.hostname or parsed.username
                or parsed.password or parsed.query or parsed.fragment
                or (parsed.scheme == "http" and parsed.hostname not in ("127.0.0.1", "localhost", "::1"))):
            raise VerificationError("Use HTTPS or an HTTP loopback API, without credentials/query/fragment.")
        try:
            if parsed.port == 0:
                raise ValueError()
        except ValueError:
            raise VerificationError("Invalid API port.") from None
        self.base = base.rstrip("/")
        self.opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())

    def post(self, path, body, bearer=None):
        return self.request("POST", path, body, bearer)

    def request(self, method, path, body=_NO_BODY, bearer=None, *, idempotency_key=None):
        headers = {
            "Content-Type": "application/json",
            "User-Agent": "Cuenora-Identity-Helper/0.1",
        }
        if bearer:
            headers["Authorization"] = "Bearer " + bearer
        if idempotency_key:
            headers["Idempotency-Key"] = idempotency_key
        data = None if body is _NO_BODY else json.dumps(body).encode()
        request = urllib.request.Request(self.base + path, data=data, headers=headers, method=method)
        try:
            response = self.opener.open(request, timeout=15)
        except urllib.error.HTTPError as error:
            response = error
        except (urllib.error.URLError, OSError):
            raise VerificationError("API connection failed; check the local server and environment.") from None
        with response:
            status = response.code
            media_type = response.headers.get("Content-Type", "").split(";", 1)[0].strip().lower()
            safe_media_type = media_type if media_type in ("application/json", "text/plain", "text/html") else "other"
            raw = response.read(16385)
        if len(raw) > 16384:
            raise VerificationError("Unexpected API response size.")
        try:
            value = json.loads(raw) if raw else None
        except (ValueError, UnicodeError):
            raise VerificationError(
                f"Unexpected API response format (HTTP {status}; content type: {safe_media_type})."
            ) from None
        return status, value

def require(status, expected, message):
    if status != expected:
        raise VerificationError(message)

def flow(api, email, password, register, prompt=getpass.getpass):
    if register:
        status, _ = api.post("/v1/auth/register", {"email": email, "password": password})
        require(status, 202, "Registration was not accepted; check password policy and server availability.")
    status, pair = api.post("/v1/auth/login", {"email": email, "password": password})
    require(status, 200, "Login failed; check the account/password and server availability.")
    if not isinstance(pair, dict) or not isinstance(pair.get("access_token"), str) or not isinstance(pair.get("refresh_token"), str):
        raise VerificationError("Unexpected login response.")
    status, _ = api.post("/v1/auth/email-verification/request", {}, pair["access_token"])
    require(status, 202, "Mail request failed; check configuration, migrations and request limits.")
    print("Request accepted, not a delivery guarantee. Check your inbox and spam folder.")
    print("No message may be sent for an already verified account or a suppressed resend.")
    token = prompt("Paste the ev1_ token from the email (hidden; Ctrl+C to cancel): ")
    if len(token) != 68 or not token.startswith("ev1_") or any(c not in "0123456789abcdef" for c in token[4:]):
        raise VerificationError("Invalid verification-token format.")
    # Refresh only to keep the same account authenticated while the user reads mail.
    # Confirmation itself never extends a session or issues credentials.
    status, renewed = api.post("/v1/auth/refresh", {"refresh_token": pair["refresh_token"]})
    require(status, 200, "Session refresh failed; restart the helper and log in again.")
    if not isinstance(renewed, dict) or not isinstance(renewed.get("access_token"), str):
        raise VerificationError("Unexpected refresh response.")
    bearer = renewed["access_token"]
    status, _ = api.post("/v1/auth/email-verification/confirm", {"token": token}, bearer)
    require(status, 204, "Confirmation failed; token may be expired, consumed or belong to another account.")
    status, body = api.post("/v1/auth/email-verification/confirm", {"token": token}, bearer)
    if status != 400 or not isinstance(body, dict) or body.get("code") != "invalid_challenge":
        raise VerificationError("Account confirmed, but the replay-rejection check failed.")
    print("Email verified (204). Token replay rejected (400 invalid_challenge).")


def startup_failure(process):
    # Never forward raw child stderr: only recognized, static variable names.
    raw = process.stderr.read(4096) if process.stderr is not None else b""
    text = raw.decode("utf-8", errors="replace")
    allowed = {
        "CUENORA_RESEND_API_KEY", "CUENORA_EMAIL_FROM", "CUENORA_DATABASE_URL",
        "CUENORA_LISTEN_ADDR", "CUENORA_PUBLIC_API_URL", "CUENORA_TRUSTED_PROXY_IPS",
        "CUENORA_ACCESS_TOKEN_TTL_SECONDS", "CUENORA_REFRESH_TOKEN_TTL_SECONDS",
        "CUENORA_DB_MAX_CONNECTIONS", "CUENORA_DB_ACQUIRE_TIMEOUT_MS",
        "CUENORA_DB_STATEMENT_TIMEOUT_MS", "CUENORA_DB_LOCK_TIMEOUT_MS",
    }
    names = sorted(set(re.findall(r"CUENORA_[A-Z_]+", text)) & allowed)
    if names:
        return "Local API rejected environment configuration: " + ", ".join(names) + ". Values were not displayed."
    return "Local API stopped during startup; no raw diagnostics were displayed. Check configuration and port availability."


def local_flow(email, password, register, *, runner=None):
    # Reuse the repository's disposable PostgreSQL harness, never a personal stack.
    sys.path.insert(0, str(ROOT / "tests" / "infrastructure"))
    from test_postgres_compose import PostgresComposeTest
    if not os.environ.get("CUENORA_RESEND_API_KEY") or not os.environ.get("CUENORA_EMAIL_FROM"):
        raise VerificationError("Export CUENORA_RESEND_API_KEY and CUENORA_EMAIL_FROM first.")
    build_env = {k: v for k, v in os.environ.items() if k not in ("CUENORA_RESEND_API_KEY", "CUENORA_EMAIL_FROM")}
    build_env["CARGO_TARGET_DIR"] = str(ROOT / "backend" / "target")
    subprocess.run(["cargo", "build", "--locked", "--bin", "cuenora-api"], cwd=ROOT / "backend", env=build_env, check=True)
    harness = PostgresComposeTest()
    harness.setUp()
    harness.env.pop("CUENORA_RESEND_API_KEY", None)
    harness.env.pop("CUENORA_EMAIL_FROM", None)
    process = None
    try:
        project = harness.identity_project()
        host, port = harness.succeeds(project, "port", "postgres", "5432").rsplit(":", 1)
        if host != "127.0.0.1":
            raise VerificationError("Disposable database must bind loopback.")
        with socket.socket() as available:
            available.bind(("127.0.0.1", 0))
            api_port = available.getsockname()[1]
        env = dict(os.environ)
        env["CUENORA_DATABASE_URL"] = f"postgres://cuenora:{harness.env['CUENORA_POSTGRES_PASSWORD']}@127.0.0.1:{port}/cuenora"
        env["CUENORA_LISTEN_ADDR"] = f"127.0.0.1:{api_port}"
        env.pop("CUENORA_TRUSTED_PROXY_IPS", None)
        process = subprocess.Popen([str(ROOT / "backend" / "target" / "debug" / "cuenora-api")], cwd=ROOT / "backend", env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        api = Api(f"http://127.0.0.1:{api_port}")
        deadline = time.monotonic() + 15
        while True:
            if process.poll() is not None:
                raise VerificationError(startup_failure(process))
            try:
                with api.opener.open(api.base + "/v1/health/live", timeout=1) as response:
                    if response.status == 200:
                        break
            except (urllib.error.URLError, OSError):
                pass
            if time.monotonic() >= deadline:
                raise VerificationError("Local API did not start in time.")
            time.sleep(0.1)
        (runner or flow)(api, email, password, register)
    finally:
        if process is not None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            if process.stderr is not None:
                process.stderr.close()
        harness.doCleanups()
        print("Disposable database and local API stopped; test account was not retained.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    location = parser.add_mutually_exclusive_group()
    location.add_argument("--api", default=None, help="Existing API base URL; defaults to loopback:3000")
    location.add_argument("--local", action="store_true", help="Start disposable PostgreSQL and a loopback API; account is discarded on exit")
    parser.add_argument("--register", action="store_true", help="Register before logging in (required for a fresh --local account)")
    args = parser.parse_args()
    if args.local and not args.register:
        parser.error("--local requires --register for its fresh database")
    if not sys.stdin.isatty():
        parser.error("An interactive terminal is required; never pipe credentials into this helper")
    warnings.simplefilter("error", getpass.GetPassWarning)
    try:
        email = input("Your test email address: ")
        password = getpass.getpass("Test-account password (hidden): ")
        if args.local:
            local_flow(email, password, args.register)
        else:
            flow(Api(args.api or "http://127.0.0.1:3000"), email, password, args.register)
    except VerificationError as error:
        # These messages are authored locally and never contain response bodies or inputs.
        print(f"Verification test could not complete: {error}", file=sys.stderr)
        return 1
    except subprocess.CalledProcessError:
        print("Verification test could not complete: API build failed.", file=sys.stderr)
        return 1
    except getpass.GetPassWarning:
        print("Secure hidden input is unavailable; test stopped before reading credentials.", file=sys.stderr)
        return 1
    except (KeyboardInterrupt, EOFError):
        print("\nVerification test cancelled; no credentials were printed.", file=sys.stderr)
        return 130
    return 0

if __name__ == "__main__":
    sys.exit(main())
