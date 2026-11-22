"""Offline tests for cutover gate, independent of GitLab/GitHub/Go."""
import unittest

from scripts.audit_rust_cutover import summarize


class RustCutoverAuditTests(unittest.TestCase):
    def test_go_source_blocks_go_free_cutover(self):
        result = summarize(["backend/crypto/vault.go", "cmd/tdrive/main.go",
                            "go.mod", "go.sum", "crates/tdrive-crypto/src/lib.rs"])
        self.assertFalse(result["go_free"])
        self.assertEqual(result["go_source_files"], 2)
        self.assertEqual(result["go_module_files"], ["go.mod", "go.sum"])

    def test_go_only_docs_do_not_count_as_go_runtime(self):
        result = summarize(["docs/migration/go-to-rust.md",
                            "crates/tdrive-types/src/lib.rs", "Cargo.toml"])
        self.assertTrue(result["go_free"])
        self.assertEqual(result["go_source_files"], 0)

    def test_go_module_alone_prevents_cutover(self):
        result = summarize(["go.mod", "Cargo.toml"])
        self.assertFalse(result["go_free"])

    def test_top_level_and_unclassified_go_are_counted_in_other_area(self):
        result = summarize(["smoke.go", "tools/legacy/main.go", "backend/crypto/vault.go"])
        self.assertEqual(result["go_source_files"], 3)
        self.assertEqual(result["remaining_go_by_area"]["other/"], 2)

    def test_go_oracles_count_until_removed(self):
        result = summarize(["backend/crypto/stream_test.go", "tests/fixture_oracle.go"])
        self.assertEqual(result["go_source_files"], 2)
        self.assertFalse(result["go_free"])


if __name__ == "__main__":
    unittest.main()
