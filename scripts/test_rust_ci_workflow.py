#!/usr/bin/env python3
"""Regression checks for a previously duplicated/truncated Rust CI workflow."""
import unittest
from pathlib import Path
from check_rust_ci_workflow import EXPECTED_JOBS, GO_ORACLES, REQUIRED_ARTIFACTS, validate_text


def fixture() -> str:
    lines = [
        "name: Rust migration parallel CI", "on:", "  push:",
        "permissions:", "  contents: read", "jobs:",
    ]
    for name in EXPECTED_JOBS:
        lines += [f"  {name}:", "    runs-on: ubuntu-24.04"]
        if name == "rust-contracts":
            lines.append("    needs: go-fixtures")
        lines.append("    steps:")
        lines += [
            "      - name: Verify checkout SHA",
            '        run: test "$(git rev-parse HEAD)" = "$GITHUB_SHA"',
        ]
        if name == "legacy-contracts":
            lines.append("      - run: python3 scripts/check_rust_ci_workflow.py")
            lines.append("      - run: python3 scripts/test_rust_gha_run.py")
        if name == "rust-style":
            lines.append("      - run: cargo +1.92.0 fmt --all --check")
        if name == "go-fixtures":
            lines += [f"      - run: go test ./backend/crypto -run '^{x}$' -count=1"
                      for x in GO_ORACLES[1:]]
            lines.append(f"      - run: go test ./backend/projection -run '^{GO_ORACLES[0]}$' -count=1")
            lines += [
                "      - uses: actions/upload-artifact@dummy",
                "        with:",
                "          if-no-files-found: error",
            ]
            lines += [f"          path: {f}" for f in REQUIRED_ARTIFACTS]
        if name == "rust-contracts":
            lines += ["      - uses: actions/download-artifact@dummy"]
            lines += [f"      - run: test -s {f}" for f in REQUIRED_ARTIFACTS]
            lines += [
                "      - run: cargo +1.92.0 fmt --all --check",
                "      - run: cargo +1.92.0 clippy --workspace --all-targets --locked -- -D warnings",
                "      - run: cargo +1.92.0 test --workspace --locked",
            ]
        if name == "legacy-go-tests":
            lines += ["      - run: go test ./backend/crypto",
                      "      - run: go test ./backend/projection"]
        if name == "web-checks":
            lines += ["      - run: bun run --cwd cmd/tdrive/webui lint",
                      "      - run: bun run --cwd cmd/tdrive/webui test:unit"]
    lines += [f"# {name}" for name in (
        "TDRIVE_COMPAT_FIXTURE_OUT", "TDRIVE_COMPAT_TDE1_OUT",
        "TDRIVE_COMPAT_NAME_OUT", "TDRIVE_COMPAT_FOLDER_LISTING_OUT",
        "TDRIVE_COMPAT_REPLAY_OUT", "TDRIVE_COMPAT_DIGEST_OUT",
        "TDRIVE_GO_NAME_FIXTURE", "TDRIVE_GO_SQLITE_FIXTURE",
        "TDRIVE_GO_TDE1_FIXTURES_DIR")]
    return "\n".join(lines) + "\n"


class WorkflowContractTests(unittest.TestCase):
    def test_good_workflow_contract(self):
        self.assertEqual(validate_text(fixture()), [])

    def test_malformed_existing_duplicate_job(self):
        sample = fixture() + "\n  rust-contracts:\n    runs-on: ubuntu-24.04\n    steps:\n"
        self.assertTrue(any("duplicate" in error or "six" in error
                            for error in validate_text(sample)))

    def test_orphaned_count_fragment(self):
        sample = fixture() + "\n -count=1\n"
        self.assertTrue(any("orphaned" in error for error in validate_text(sample)))

    def test_seeded_command_truncation(self):
        sample = fixture().replace(
            "-run '^TestExportRustTDE1Fixtures$' -count=1",
            "-run '^TestExportRustTDE1Fixtures",
        )
        self.assertTrue(any("Go oracle" in error or "unbalanced" in error
                            for error in validate_text(sample)))

    def test_name_fixture_command_truncation(self):
        sample = fixture().replace(
            "-run '^TestExportRustNameSealFixtures$' -count=1",
            "-run '^TestExportRustNameSealFixtures",
        )
        self.assertTrue(any("Go oracle" in error for error in validate_text(sample)))

    def test_missing_checkout_sha_check_is_rejected(self):
        sample = fixture().replace(
            '      - name: Verify checkout SHA\n'
            '        run: test "$(git rev-parse HEAD)" = "$GITHUB_SHA"',
            "", 1,
        )
        self.assertTrue(any("checkout SHA attestation" in x for x in validate_text(sample)))

    def test_wrong_sha_check_command_is_rejected(self):
        sample = fixture().replace(
            'run: test "$(git rev-parse HEAD)" = "$GITHUB_SHA"',
            'run: echo skip source SHA assertion', 1,
        )
        self.assertTrue(any("checkout SHA attestation" in x for x in validate_text(sample)))

    def test_missing_github_evidence_tests_rejected(self):
        sample = fixture().replace(
            "      - run: python3 scripts/test_rust_gha_run.py", "",
        )
        self.assertTrue(any("GH job-evidence" in x for x in validate_text(sample)))

    def test_no_tests_is_not_success(self):
        sample = fixture().replace(
            "      - run: cargo +1.92.0 test --workspace --locked", "")
        self.assertTrue(any("Rust contract step" in error for error in validate_text(sample)))

    def test_no_publish_or_secret(self):
        sample = fixture() + "  publish:\n    runs-on: ubuntu-24.04\n"
        self.assertTrue(validate_text(sample))
        sample = fixture().replace("contents: read", "packages: write")
        self.assertTrue(validate_text(sample))

    def test_real_source_contract(self):
        root = Path(__file__).resolve().parents[1]
        path = root / ".github/workflows/rust-migration-parallel.yml"
        self.assertEqual(validate_text(path.read_text(encoding="utf-8")), [])


if __name__ == "__main__":
    unittest.main(verbosity=2)
