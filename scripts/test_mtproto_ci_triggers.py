"""Regression test for native workflow push filters (not a workflow runner)."""
from fnmatch import fnmatchcase
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github/workflows/rust-mtproto-native.yml"


def filters(text, key):
    match = re.search(r"(?m)^    " + key + r":\n((?:      - \"[^\"\n]+\"\n)+)", text)
    if match is None:
        raise AssertionError(f"missing push {key} filters")
    return re.findall(r'"([^"\n]+)"', match.group(1))


class NativeTriggerTests(unittest.TestCase):
    def test_rust_branches_trigger_native_workflow(self):
        branches = filters(WORKFLOW.read_text(), "branches")
        for branch in ("refactor/rust-only-cutover-20261122",
                       "refactor/rust-integration-converged-20261124",
                       "refactor/rust-aggregate-03-multipart-download",
                       "refactor/rust-converged-07-backup-cli"):
            with self.subTest(branch=branch):
                self.assertTrue(any(fnmatchcase(branch, p) for p in branches))
        self.assertFalse(any(fnmatchcase("feature/unrelated", p) for p in branches))

    def test_all_native_build_inputs_trigger_workflow(self):
        paths = filters(WORKFLOW.read_text(), "paths")
        for path in ("apps/tdrive-mtproto/src/main.rs",
                     "crates/tdrive-storage/src/lib.rs",
                     "crates/tdrive-projection/src/wire.rs",
                     "apps/tdrive-inspect/src/main.rs",
                     "Cargo.toml", "Cargo.lock", "Dockerfile.mtproto",
                     ".dockerignore", "docker-compose.rust.yml",
                     "docker-compose.rust-watch.yml",
                     "scripts/test_mtproto_docker_contract.py",
                     "scripts/test_mtproto_ci_triggers.py"):
            with self.subTest(path=path):
                self.assertTrue(any(fnmatchcase(path, p) for p in paths))
        self.assertFalse(any(fnmatchcase("docs/notes.md", p) for p in paths))


if __name__ == "__main__":
    unittest.main()
