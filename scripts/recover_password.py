#!/usr/bin/env python3
"""Human-operated password recovery smoke test. Changes the test account password and revokes ALL its sessions."""
import argparse
import getpass
import sys
import warnings
import subprocess
from verify_email import Api, VerificationError, local_flow, require


def flow(api, email, password, register, prompt=getpass.getpass):
    if register:
        status, _ = api.post("/v1/auth/register", {"email": email, "password": password})
        require(status, 202, "Registration was not accepted; check password policy and server availability.")
    status, pair = api.post("/v1/auth/login", {"email": email, "password": password})
    require(status, 200, "Initial login failed; use a test account with its current password.")
    if not isinstance(pair, dict) or not isinstance(pair.get("refresh_token"), str):
        raise VerificationError("Unexpected login response.")
    status, _ = api.post("/v1/auth/password-recovery/request", {"email": email})
    require(status, 202, "Recovery request failed; check mail configuration, migrations and request limits.")
    print("Request accepted, not a delivery guarantee. Check your inbox and spam folder.")
    print("Unknown accounts and suppressed resends receive the same accepted response.")
    token = prompt("Paste the pr1_ token from the email (hidden; Ctrl+C to cancel): ")
    if len(token) != 68 or not token.startswith("pr1_") or any(c not in "0123456789abcdef" for c in token[4:]):
        raise VerificationError("Invalid recovery-token format.")
    new_password = prompt("New test-account password (hidden): ")
    repeated = prompt("Repeat new test-account password (hidden): ")
    if new_password != repeated or new_password == password:
        raise VerificationError("The new passwords must match and differ from the old password for this smoke test.")
    body = {"token": token, "new_password": new_password}
    status, result = api.post("/v1/auth/password-recovery/confirm", body)
    require(status, 204, "Password reset failed; check policy, token expiry and server availability.")
    if result is not None:
        raise VerificationError("Unexpected password-reset response; no automatic login was expected.")
    status, result = api.post("/v1/auth/password-recovery/confirm", body)
    if status != 400 or not isinstance(result, dict) or result.get("code") != "invalid_challenge":
        raise VerificationError("Password changed, but the replay-rejection check failed.")
    status, _ = api.post("/v1/auth/login", {"email": email, "password": password})
    require(status, 401, "Password changed, but the old-password rejection check failed.")
    status, _ = api.post("/v1/auth/refresh", {"refresh_token": pair["refresh_token"]})
    require(status, 401, "Password changed, but the old-session rejection check failed.")
    status, _ = api.post("/v1/auth/login", {"email": email, "password": new_password})
    require(status, 200, "Password changed, but normal new-password login failed.")
    print("Password reset (204). Replay, old password and old refresh rejected; new-password login passed.")
    print("API success does not prove notification delivery.")
    prompt("Wait for the password-change notification, then press Enter to finish (Ctrl+C to cancel): ")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    location = parser.add_mutually_exclusive_group()
    location.add_argument("--api", help="Existing test API base URL; defaults to loopback:3000. Resets that account and revokes all sessions.")
    location.add_argument("--local", action="store_true", help="Use disposable PostgreSQL and a loopback API; account is discarded on exit")
    parser.add_argument("--register", action="store_true", help="Register a fresh test account before testing recovery")
    args = parser.parse_args()
    if args.local and not args.register:
        parser.error("--local requires --register for its fresh database")
    if not sys.stdin.isatty():
        parser.error("An interactive terminal is required; never pipe credentials into this helper")
    warnings.simplefilter("error", getpass.GetPassWarning)
    try:
        email = input("Your test email address: ")
        password = getpass.getpass("Current test-account password (hidden): ")
        if args.local:
            local_flow(email, password, True, runner=flow)
        else:
            flow(Api(args.api or "http://127.0.0.1:3000"), email, password, args.register)
    except VerificationError as error:
        print(f"Recovery test could not complete: {error}", file=sys.stderr)
        return 1
    except subprocess.CalledProcessError:
        print("Recovery test could not complete: API build failed.", file=sys.stderr)
        return 1
    except getpass.GetPassWarning:
        print("Secure hidden input is unavailable; test stopped before reading credentials.", file=sys.stderr)
        return 1
    except (KeyboardInterrupt, EOFError):
        print("\nRecovery test cancelled; no credentials were printed.", file=sys.stderr)
        return 130
    return 0


if __name__ == "__main__":
    sys.exit(main())
