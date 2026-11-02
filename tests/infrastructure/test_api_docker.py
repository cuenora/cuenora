#!/usr/bin/env python3
"""Build and exercise the API image without connecting to a database."""

from __future__ import annotations

import os
import re
import subprocess
import time
import urllib.error
import urllib.request
import uuid

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
CONTAINER_PORT = 3000


def docker(*args: str, timeout: int = 30, check: bool = True) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        ["docker", *args],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
        check=False,
    )
    if check and result.returncode != 0:
        raise AssertionError(
            f"docker {' '.join(args[:3])} failed with {result.returncode}: "
            f"{result.stderr.strip()}"
        )
    return result


def wait_for_http(url: str, container: str, timeout: float = 15.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        state = docker("inspect", "--format", "{{.State.Running}}", container, check=False)
        if state.returncode != 0 or state.stdout.strip() != "true":
            logs = docker("logs", container, check=False)
            raise AssertionError(f"API container stopped before liveness check: {logs.stderr}{logs.stdout}")
        try:
            with urllib.request.urlopen(url, timeout=1) as response:
                if response.status == 200 and response.read() == b"OK":
                    return
        except (urllib.error.URLError, TimeoutError, OSError):
            time.sleep(0.2)
    raise AssertionError(f"API did not become live at {url}")


def check_default_loopback_bind(image: str, name: str) -> None:
    docker("run", "--detach", "--name", name, image)
    try:
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            state = docker("inspect", "--format", "{{.State.Running}}", name, check=False)
            if state.returncode != 0 or state.stdout.strip() != "true":
                logs = docker("logs", name, check=False)
                raise AssertionError(f"default-configured API container stopped: {logs.stderr}{logs.stdout}")
            probe = docker(
                "exec",
                name,
                "sh",
                "-ec",
                "awk '$2 == \"0100007F:0BB8\" && $4 == \"0A\" { found=1 } END { exit !found }' /proc/net/tcp",
                check=False,
            )
            if probe.returncode == 0:
                return
            time.sleep(0.2)
        raise AssertionError("API did not preserve its default 127.0.0.1:3000 listener")
    finally:
        docker("rm", "--force", name, check=False)


def check_overridden_bind_and_environment(image: str, name: str) -> None:
    docker(
        "run",
        "--detach",
        "--name",
        name,
        "--publish",
        f"127.0.0.1::{CONTAINER_PORT}",
        "--env",
        "CUENORA_LISTEN_ADDR=0.0.0.0:3000",
        "--env",
        "CUENORA_PUBLIC_API_URL=https://api.example.test",
        "--env",
        "CUENORA_ACCESS_TOKEN_TTL_SECONDS=120",
        "--env",
        "CUENORA_REFRESH_TOKEN_TTL_SECONDS=7200",
        "--env",
        "CUENORA_DB_MAX_CONNECTIONS=8",
        "--env",
        "CUENORA_DB_ACQUIRE_TIMEOUT_MS=4000",
        "--env",
        "CUENORA_DB_STATEMENT_TIMEOUT_MS=6000",
        "--env",
        "CUENORA_DB_LOCK_TIMEOUT_MS=1000",
        image,
    )
    try:
        mapping = docker("port", name, f"{CONTAINER_PORT}/tcp").stdout.strip()
        match = re.fullmatch(r"127\.0\.0\.1:(\d+)", mapping)
        if not match:
            raise AssertionError(f"unexpected loopback port mapping: {mapping!r}")
        wait_for_http(f"http://127.0.0.1:{match.group(1)}/v1/health/live", name)
    finally:
        docker("rm", "--force", name, check=False)


def check_openapi_url(image: str, url: str | None, expected: str) -> None:
    args = ["run", "--rm"]
    if url is not None:
        args.extend(["--env", f"CUENORA_PUBLIC_API_URL={url}"])
    args.extend(["--entrypoint", "cuenora-openapi", image])
    output = docker(*args).stdout
    match = re.search(r"(?m)^  - url: (.+)$", output)
    actual = match.group(1).strip() if match else None
    if actual and len(actual) >= 2 and actual[0] == actual[-1] == "'":
        actual = actual[1:-1].replace("''", "'")
    if actual != expected:
        raise AssertionError(f"OpenAPI renderer did not emit expected server URL {expected!r}")


def check_invalid_openapi_urls(image: str) -> None:
    invalid_urls = (
        "http://example.test:99999",
        "http://example.test:abc",
        "http://example.test:-1",
        "http:///path",
        "http://[]",
        "http://[notipv6]",
    )
    for url in invalid_urls:
        result = docker(
            "run",
            "--rm",
            "--env",
            f"CUENORA_PUBLIC_API_URL={url}",
            "--entrypoint",
            "cuenora-openapi",
            image,
            check=False,
        )
        if result.returncode != 2:
            raise AssertionError(
                f"OpenAPI renderer returned {result.returncode} for malformed public URL"
            )
        if url in result.stdout + result.stderr:
            raise AssertionError("OpenAPI renderer printed a malformed public URL")


def check_invalid_setting_does_not_log_values(image: str, name: str) -> None:
    marker = "DO_NOT_LOG_CUENORA_SECRET_MARKER"
    for setting in ("CUENORA_TRUSTED_PROXY_IPS", "CUENORA_ACCESS_TOKEN_TTL_SECONDS"):
        result = docker(
            "run",
            "--name",
            name,
            "--env",
            f"{setting}={marker}",
            image,
            check=False,
        )
        if result.returncode == 0:
            raise AssertionError(f"API accepted an invalid {setting} setting")
        output = result.stdout + result.stderr
        logs = docker("logs", name, check=False)
        output += logs.stdout + logs.stderr
        docker("rm", "--force", name, check=False)
        if marker in output:
            raise AssertionError(f"invalid {setting} value was printed to container logs")


def main() -> None:
    suffix = uuid.uuid4().hex[:12]
    image = f"cuenora-api-docker-test:{suffix}"
    names = [f"cuenora-api-docker-test-{suffix}-{index}" for index in range(3)]
    built = False
    try:
        docker("build", "--tag", image, "--file", "infra/Dockerfile.api", ".", timeout=900)
        built = True
        runtime_user = docker("inspect", "--format", "{{.Config.User}}", image).stdout.strip()
        if runtime_user != "65532:65532":
            raise AssertionError(f"API image must run as the unprivileged runtime user, got {runtime_user!r}")
        check_default_loopback_bind(image, names[0])
        check_overridden_bind_and_environment(image, names[1])
        check_openapi_url(image, None, "http://127.0.0.1:3000")
        check_openapi_url(image, "https://api.example.test", "https://api.example.test")
        check_invalid_openapi_urls(image)
        check_invalid_setting_does_not_log_values(image, names[2])
    finally:
        for name in names:
            docker("rm", "--force", name, check=False)
        if built:
            docker("image", "rm", image, check=False)
    print("Docker API runtime checks passed")


if __name__ == "__main__":
    main()
