"""Static checks that Rust MTProto Docker builds have every path crate.
This is NOT a Docker image build, Cargo run, or production security audit.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class MtprotoDockerContract(unittest.TestCase):
    def test_workspace_files_and_rust_crates_are_in_builder(self):
        docker = (ROOT / "Dockerfile.mtproto").read_text()
        for snippet in (
            "COPY Cargo.toml Cargo.lock ./",
            "COPY crates/ ./crates/",
            "COPY apps/tdrive-inspect/ ./apps/tdrive-inspect/",
            "COPY apps/tdrive-mtproto/ ./apps/tdrive-mtproto/",
        ):
            self.assertIn(snippet, docker)

    def test_final_image_is_nonroot_without_account_baking(self):
        docker = (ROOT / "Dockerfile.mtproto").read_text()
        self.assertIn("USER 10001:10001", docker)
        self.assertNotIn("EXPOSE ", docker)
        self.assertNotIn("ARG TG_API_", docker)
        self.assertNotIn("ENV TG_API_", docker)
        self.assertIn('ENTRYPOINT ["/usr/local/bin/tdrive-mtproto"]', docker)

    def test_docker_context_excludes_session_secret_artifacts(self):
        ignore = (ROOT / ".dockerignore").read_text()
        for pattern in ("**/session.sqlite", "**/*.session", "**/tdrive-inbox.sqlite"):
            self.assertIn(pattern, ignore)


if __name__ == "__main__":
    unittest.main()
