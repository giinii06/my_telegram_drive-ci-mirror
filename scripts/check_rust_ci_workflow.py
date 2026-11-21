#!/usr/bin/env python3
"""Strict dependency-free contract checker for TDrive's Rust GitHub workflow.

This checker validates our *specific* known six-job workflow and rejects the
duplicate/truncated fixture commands that escaped earlier edits. It is NOT a
general YAML parser; the GitHub Actions workflow loader and, when available,
a YAML parser/actionlint must also validate the document.
"""
from __future__ import annotations

import argparse
import re
from pathlib import Path

EXPECTED_JOBS = (
    "legacy-contracts", "rust-style", "go-fixtures",
    "rust-contracts", "legacy-go-tests", "web-checks",
)
GO_ORACLES = (
    "TestExportRustV16Fixture",
    "TestExportRustTDE1Fixtures",
    "TestExportRustNameSealFixtures",
)
NEEDED_ENV = (
    "TDRIVE_COMPAT_FIXTURE_OUT",
    "TDRIVE_COMPAT_TDE1_OUT",
    "TDRIVE_COMPAT_NAME_OUT",
    "TDRIVE_COMPAT_FOLDER_LISTING_OUT",
    "TDRIVE_COMPAT_REPLAY_OUT",
    "TDRIVE_COMPAT_DIGEST_OUT",
    "TDRIVE_GO_NAME_FIXTURE",
    "TDRIVE_GO_SQLITE_FIXTURE",
    "TDRIVE_GO_TDE1_FIXTURES_DIR",
)
REQUIRED_ARTIFACTS = (
    "tests/fixtures/sqlite/go-v16.sqlite",
    "tests/fixtures/sqlite/go-v16.listing.tsv",
    "tests/fixtures/sqlite/go-v16.replay.tsv",
    "tests/fixtures/sqlite/go-v16.digest.tsv",
    "tests/fixtures/tde1/generated/tde1-0.bin",
    "tests/fixtures/tde1/generated/tde1-seeded-0.bin",
    "tests/fixtures/name/go-v1.tsv",
)


def validate_text(content: str) -> list[str]:
    errors: list[str] = []
    lines = content.splitlines()
    if content.count("\t"):
        errors.append("tabs are prohibited in YAML indentation")
    if lines.count("jobs:") != 1:
        errors.append("exactly one top-level jobs section is required")
        return errors
    job_lines = lines[lines.index("jobs:") + 1:]
    heads = [(i, line[2:-1]) for i, line in enumerate(job_lines)
             if re.fullmatch(r"  [a-z][a-z0-9-]*:", line)]
    actual = [name for _, name in heads]
    if actual != list(EXPECTED_JOBS):
        errors.append("expected exactly six unique jobs in order; got " + repr(actual))
    sections: dict[str, str] = {}
    for pos, (start, name) in enumerate(heads):
        end = heads[pos + 1][0] if pos + 1 < len(heads) else len(job_lines)
        if name in sections:
            errors.append("duplicate top-level job " + name)
        sections[name] = "\n".join(job_lines[start:end])
    if "on:" not in lines[:lines.index("jobs:")]:
        errors.append("missing GitHub Actions on event block")
    if not any(line.strip() == "contents: read" for line in lines[:lines.index("jobs:")]):
        errors.append("global GitHub permissions must be contents: read")
    if "pull_request:" in content or "packages: write" in content or "secrets." in content:
        errors.append("unexpected pull request, write privilege or secret in CI workflow")
    if re.search(r"(?m)^\s*-count=\d+\s*$", content):
        errors.append("orphaned -count=N; likely damaged go test command")
    if re.search(r"(?m)^\s{2}publish:", content):
        errors.append("publishing is forbidden in Rust migration checks")

    for name in EXPECTED_JOBS:
        section = sections.get(name, "")
        if not section:
            errors.append("missing job " + name)
            continue
        if "runs-on: ubuntu-24.04" not in section or "steps:" not in section:
            errors.append(name + " is missing the expected runner/steps")
        sha_attestation = (
            '      - name: Verify checkout SHA\n'
            '        run: test "$(git rev-parse HEAD)" = "$GITHUB_SHA"'
        )
        if section.count(sha_attestation) != 1:
            errors.append(name + " must execute exactly one checkout SHA attestation")
        if name == "rust-contracts":
            if not re.search(r"(?m)^    needs: go-fixtures$", section):
                errors.append("rust-contracts must depend on go-fixtures")
        elif re.search(r"(?m)^    needs:", section):
            errors.append(name + " must be independent of other jobs")
    fixtures = sections.get("go-fixtures", "")
    for oracle in GO_ORACLES:
        command = f"-run '^{oracle}$' -count=1"
        if command not in fixtures:
            errors.append("missing complete Go oracle command: " + oracle)
        if content.count(command) != 1:
            errors.append("Go oracle must occur exactly once: " + oracle)
    if "actions/upload-artifact@" not in fixtures or "if-no-files-found: error" not in fixtures:
        errors.append("fixture job must upload mandatory nonempty artifacts")
    rust = sections.get("rust-contracts", "")
    for command in ("actions/download-artifact@", "cargo +1.92.0 fmt --all --check",
                    "cargo +1.92.0 clippy --workspace --all-targets --locked -- -D warnings",
                    "cargo +1.92.0 test --workspace --locked"):
        if command not in rust:
            errors.append("missing Rust contract step: " + command)
    style = sections.get("rust-style", "")
    if "cargo +1.92.0 fmt --all --check" not in style:
        errors.append("missing independent rustfmt job")
    for entry in NEEDED_ENV:
        if entry not in content:
            errors.append("missing Go/Rust fixture environment: " + entry)
    for path in REQUIRED_ARTIFACTS:
        if path not in fixtures or path not in rust:
            errors.append("fixture must be generated and asserted: " + path)
    for step in ("go test ./backend/crypto", "go test ./backend/projection"):
        if step not in sections.get("legacy-go-tests", ""):
            errors.append("missing Go regression suite: " + step)
    for step in ("bun run --cwd cmd/tdrive/webui lint", "bun run --cwd cmd/tdrive/webui test:unit"):
        if step not in sections.get("web-checks", ""):
            errors.append("missing Bun regression suite: " + step)
    if "python3 scripts/check_rust_ci_workflow.py" not in sections.get("legacy-contracts", ""):
        errors.append("workflow must enforce its own contract check")
    if "python3 scripts/test_rust_gha_run.py" not in sections.get("legacy-contracts", ""):
        errors.append("workflow must execute its GH job-evidence regression tests")
    for idx, line in enumerate(lines):
        if re.fullmatch(r"      - name: .+", line):
            body = []
            for following in lines[idx + 1:]:
                if following.startswith("      - ") or (
                    following.strip() and len(following) - len(following.lstrip()) < 6
                ):
                    break
                body.append(following)
            if not any(re.match(r"        (?:run|uses):", item) for item in body):
                errors.append("name-only step lacks run/uses: " + line.strip())
    for line in lines:
        if "- run:" in line and line.count("'") % 2 == 1:
            errors.append("unbalanced quoted step: " + line.strip()[:120])
    return sorted(set(errors))


def main() -> int:
    parser = argparse.ArgumentParser(description="Check TDrive Rust CI workflow contract")
    parser.add_argument(
        "path", nargs="?",
        default=".github/workflows/rust-migration-parallel.yml",
        type=Path,
    )
    args = parser.parse_args()
    errors = validate_text(args.path.read_text(encoding="utf-8"))
    for error in errors:
        print("FAIL", error)
    if errors:
        return 1
    print("PASS: six Rust/Go/Bun GitHub CI jobs and required fixture edges")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
