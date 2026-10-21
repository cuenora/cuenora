#!/usr/bin/env python3
"""Validate repository text files using only the Python standard library."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REQUIRED = (
    ".editorconfig",
    ".gitattributes",
    ".gitignore",
    "LICENSE",
    "README.md",
    ".github/workflows/ci.yml",
    ".github/dependabot.yml",
    ".devcontainer/devcontainer.json",
    "backend/rust-toolchain.toml",
)
TEXT_SUFFIXES = {
    ".md", ".yml", ".yaml", ".json", ".toml", ".rs",
    ".kt", ".kts", ".swift", ".sql", ".sh", ".py", ".xml",
    ".properties", ".gradle", ".gitignore", ".editorconfig",
    ".gitattributes",
}
TEXT_FILENAMES = {"LICENSE", "gradlew", ".env.example"}
MAX_FILE_SIZE = 2 * 1024 * 1024


def discover_files() -> list[Path]:
    result = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=ROOT,
        check=True,
        capture_output=True,
    )
    return sorted(
        (ROOT / item.decode("utf-8", errors="surrogateescape"))
        for item in result.stdout.split(b"\0")
        if item
    )


def is_text_file(path: Path) -> bool:
    return path.suffix in TEXT_SUFFIXES or path.name in TEXT_FILENAMES


def validate_file(path: Path) -> list[str]:
    relative = path.relative_to(ROOT).as_posix()
    if path.is_symlink() or not path.is_file() or not is_text_file(path):
        return []
    if path.stat().st_size > MAX_FILE_SIZE:
        return [f"{relative}: text file exceeds 2 MiB"]

    data = path.read_bytes()
    errors: list[str] = []
    try:
        contents = data.decode("utf-8")
    except UnicodeDecodeError:
        return [f"{relative}: file must use UTF-8"]

    if b"\r" in data:
        errors.append(f"{relative}: use LF line endings")
    if contents and not contents.endswith("\n"):
        errors.append(f"{relative}: missing final newline")
    for line_number, line in enumerate(contents.splitlines(), 1):
        if line.rstrip(" \t") != line:
            errors.append(f"{relative}:{line_number}: trailing whitespace")
        if line.startswith(("<<<<<<< ", ">>>>>>> ")):
            errors.append(f"{relative}:{line_number}: unresolved merge marker")
    return errors


def main() -> int:
    errors = [f"Missing required file: {name}" for name in REQUIRED if not (ROOT / name).is_file()]
    try:
        files = discover_files()
    except (OSError, subprocess.CalledProcessError) as error:
        print(f"Unable to list Git files: {error}", file=sys.stderr)
        return 1

    for path in files:
        errors.extend(validate_file(path))

    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        print(f"Repository validation failed: {len(errors)} issue(s).", file=sys.stderr)
        return 1

    print(f"Repository validation passed ({len(files)} Git-visible files examined).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
