"""Offline tests for the opt-in private-checkout Rust verification runner.

Synthetic Git checkout and fake compiler commands only; never reads user repos.
"""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("verify_mtproto_local.sh")


def exec_(args, *, cwd, env=None):
    return subprocess.run(args, cwd=cwd, env=env, text=True, capture_output=True, check=False)


class LocalGateTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="tdrive-mtproto-ci-")
        self.root = Path(self.temp.name)
        (self.root / "scripts").mkdir()
        (self.root / "apps" / "tdrive-mtproto").mkdir(parents=True)
        if SCRIPT.exists():
            (self.root / "scripts" / SCRIPT.name).write_bytes(SCRIPT.read_bytes())
        (self.root / "Cargo.toml").write_text('[workspace]\nmembers = []\n')
        (self.root / "Cargo.lock").write_text("# synthetic root lock\n")
        (self.root / "apps" / "tdrive-mtproto" / "Cargo.toml").write_text('[package]\nname="dummy"\nversion="0.1.0"\nedition="2024"\n')
        self.assertEqual(exec_(["git", "init", "-q"], cwd=self.root).returncode, 0)
        self.assertEqual(exec_(["git", "add", "."], cwd=self.root).returncode, 0)
        self.assertEqual(exec_(["git", "-c", "user.email=test@example.invalid", "-c", "user.name=Fixture", "commit", "-qm", "synthetic fixture"], cwd=self.root).returncode, 0)
        self.env = dict(os.environ)
        self.env.pop("TDRIVE_EXPECTED_SHA", None)
        self.env.pop("TDRIVE_CHECK_DOCKER", None)

    def tearDown(self):
        self.temp.cleanup()

    def run_gate(self, env=None, args=()):
        combined = dict(self.env, **(env or {}))
        return exec_(["bash", "scripts/verify_mtproto_local.sh", *args], cwd=self.root, env=combined)

    def mock_tools(self, *, fail_step=""):
        bindir = self.root / "mockbin"
        bindir.mkdir(exist_ok=True)
        log = self.root / "fake-exec.log"
        # These files are ignored by git so the checkout remains clean.
        (self.root / ".gitignore").write_text("mockbin/\nfake-exec.log\n")
        for name in ("cargo", "rustc", "docker"):
            binary = bindir / name
            binary.write_text(f'#!/bin/sh\nprintf "%s\\n" "{name} $*" >> "$TDRIVE_FAKE_LOG"\ncase "$*" in *" {fail_step} "*) exit 23;; esac\ncase "{name} $*" in *" build "*) mkdir -p apps/tdrive-mtproto/target/release; printf "#!/bin/sh\\nexit 0\\n" > apps/tdrive-mtproto/target/release/tdrive-mtproto; chmod +x apps/tdrive-mtproto/target/release/tdrive-mtproto;; esac\nexit 0\n')
            binary.chmod(0o755)
        exec_(["git", "add", ".gitignore"], cwd=self.root)
        exec_(["git", "-c", "user.email=test@example.invalid", "-c", "user.name=Fixture", "commit", "-qm", "ignore test harness"], cwd=self.root)
        self.env["PATH"] = str(bindir) + os.pathsep + self.env["PATH"]
        self.env["TDRIVE_FAKE_LOG"] = str(log)
        return log

    def add_lock(self):
        (self.root / "apps" / "tdrive-mtproto" / "Cargo.lock").write_text("# synthetic test-only lock\n")
        exec_(["git", "add", "apps/tdrive-mtproto/Cargo.lock"], cwd=self.root)
        exec_(["git", "-c", "user.email=test@example.invalid", "-c", "user.name=Fixture", "commit", "-qm", "test-only lock"], cwd=self.root)

    def test_root_only_runs_without_nested_lock_and_never_claims_native(self):
        log = self.mock_tools()
        got = self.run_gate(args=("--root-only",))
        self.assertEqual(got.returncode, 0, got.stdout + got.stderr)
        lines = log.read_text().splitlines()
        self.assertTrue(any("test --workspace --all-targets --locked" in line for line in lines))
        self.assertTrue(any("build --workspace --release --locked" in line for line in lines))
        self.assertFalse(any("--manifest-path" in line or line.startswith("docker ") for line in lines))
        self.assertIn("root-only", got.stdout)
        self.assertIn("native MTProto checks not performed", got.stdout)
        self.assertFalse((self.root / "apps/tdrive-mtproto/Cargo.lock").exists())

    def test_root_only_rejects_docker_and_unknown_arguments(self):
        log = self.mock_tools()
        for args, env in [(("--root-only",), {"TDRIVE_CHECK_DOCKER": "1"}), (("--typo",), {})]:
            got = self.run_gate(env, args)
            self.assertNotEqual(got.returncode, 0)
            self.assertFalse(log.exists())

    def test_wrong_sha_fails_before_toolchain(self):
        got = self.run_gate({"TDRIVE_EXPECTED_SHA": "0" * 40})
        self.assertNotEqual(got.returncode, 0)
        self.assertIn("checkout SHA mismatch", got.stderr)

    def test_dirty_checkout_is_refused(self):
        (self.root / "Cargo.toml").write_text("# dirty\n")
        got = self.run_gate()
        self.assertNotEqual(got.returncode, 0)
        self.assertIn("checkout is dirty", got.stderr)

    def test_missing_root_lockfile_fails_without_auto_generating(self):
        self.add_lock()
        self.assertEqual(exec_(["git", "rm", "-q", "Cargo.lock"], cwd=self.root).returncode, 0)
        self.assertEqual(exec_(["git", "-c", "user.email=test@example.invalid", "-c", "user.name=Fixture", "commit", "-qm", "remove root lock"], cwd=self.root).returncode, 0)
        self.mock_tools()
        got = self.run_gate()
        self.assertNotEqual(got.returncode, 0)
        self.assertIn("root Cargo.lock missing", got.stderr)
        self.assertFalse((self.root / "Cargo.lock").exists())

    def test_missing_lockfile_fails_without_auto_generating(self):
        got = self.run_gate()
        self.assertNotEqual(got.returncode, 0)
        self.assertIn("nested Cargo.lock missing", got.stderr)
        self.assertFalse((self.root / "apps" / "tdrive-mtproto" / "Cargo.lock").exists())

    def test_mocked_fmt_clippy_tests_release_and_optional_docker_are_ordered(self):
        self.add_lock()
        log = self.mock_tools()
        got = self.run_gate({"TDRIVE_CHECK_DOCKER": "1", "TG_API_HASH": "SENSITIVE_FAKE_VALUE"})
        self.assertEqual(got.returncode, 0, got.stdout + got.stderr)
        lines = log.read_text().splitlines()
        self.assertTrue(any("fmt" in line for line in lines))
        self.assertTrue(any("clippy --workspace --all-targets --locked -- -D warnings" in line for line in lines))
        self.assertTrue(any("test --workspace --all-targets --locked" in line for line in lines))
        self.assertTrue(any("build --workspace --release --locked" in line for line in lines))
        self.assertTrue(any("clippy --manifest-path apps/tdrive-mtproto/Cargo.toml" in line for line in lines))
        self.assertTrue(any("clippy" in line and "--locked" in line for line in lines))
        self.assertTrue(any("test" in line and "--locked" in line for line in lines))
        self.assertTrue(any("build" in line and "--locked" in line for line in lines))
        self.assertTrue(any(line.startswith("docker build") for line in lines))
        self.assertNotIn("SENSITIVE_FAKE_VALUE", got.stdout + got.stderr + "\n".join(lines))

    def test_first_failed_cargo_command_stops_before_tests_and_docker(self):
        self.add_lock()
        log = self.mock_tools(fail_step="clippy")
        got = self.run_gate({"TDRIVE_CHECK_DOCKER": "1"})
        self.assertNotEqual(got.returncode, 0)
        lines = log.read_text().splitlines()
        self.assertTrue(any("clippy --workspace --all-targets --locked" in line for line in lines))
        self.assertFalse(any("clippy --manifest-path apps/tdrive-mtproto/Cargo.toml" in line for line in lines))
        self.assertFalse(any(" test " in line or " build " in line for line in lines))
        self.assertFalse(any(line.startswith("docker ") for line in lines))


if __name__ == "__main__":
    unittest.main()
