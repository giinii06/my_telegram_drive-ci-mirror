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

    def test_experimental_compose_does_not_replace_go_or_publish_ports(self):
        compose = (ROOT / "docker-compose.rust.yml").read_text()
        self.assertIn("rust-experimental", compose)
        self.assertIn("Dockerfile.mtproto", compose)
        self.assertIn("read_only: true", compose)
        self.assertIn("cap_drop:", compose)
        self.assertIn("no-new-privileges:true", compose)
        self.assertNotIn("ports:", compose)
        self.assertNotIn("network_mode: service:tailscale", compose)
        self.assertIn("TDRIVE_INBOX_PATH: /private/tdrive-inbox.sqlite", compose)
        self.assertIn("TDRIVE_SESSION_PATH: /private/session.sqlite", compose)
        self.assertIn('TG_API_ID: "${TG_API_ID:-}"', compose)
        self.assertIn('TG_API_HASH: "${TG_API_HASH:-}"', compose)
        self.assertNotIn('TG_API_ID:?Set', compose)
        self.assertNotIn('TG_API_HASH:?Set', compose)

    def test_opt_in_rust_watch_has_restart_without_public_ports(self):
        compose = (ROOT / 'docker-compose.rust-watch.yml').read_text()
        self.assertIn('rust-experimental', compose)
        self.assertIn('watch', compose)
        self.assertIn('restart: on-failure:5', compose)
        self.assertIn('read_only: true', compose)
        self.assertIn('cap_drop:', compose)
        self.assertIn('no-new-privileges:true', compose)
        self.assertIn('stop_grace_period: 30s', compose)
        self.assertIn('TDRIVE_INBOX_PATH: /private/tdrive-inbox.sqlite', compose)
        self.assertIn('TDRIVE_SESSION_PATH: /private/session.sqlite', compose)
        self.assertIn('TDRIVE_RUST_PEER', compose)
        self.assertNotIn('ports:', compose)
        self.assertNotIn('network_mode: service:tailscale', compose)

    def test_native_ci_must_build_headless_watch_configuration(self):
        workflow = (ROOT / '.github/workflows/rust-mtproto-native.yml').read_text()
        self.assertIn('docker-compose.rust-watch.yml', workflow)
        self.assertIn('docker compose -f docker-compose.rust-watch.yml config --quiet', workflow)

    def test_docker_context_excludes_session_secret_artifacts(self):
        ignore = (ROOT / ".dockerignore").read_text()
        for pattern in ("**/session.sqlite", "**/*.session", "**/tdrive-inbox.sqlite"):
            self.assertIn(pattern, ignore)


    def test_sqlite_session_sidecars_are_excluded(self):
        patterns = (ROOT / ".dockerignore").read_text().splitlines()
        for database in ("session.sqlite", "tdrive-inbox.sqlite"):
            for suffix in ("-wal", "-shm", "-journal"):
                self.assertIn("**/" + database + suffix, patterns)


if __name__ == "__main__":
    unittest.main()
