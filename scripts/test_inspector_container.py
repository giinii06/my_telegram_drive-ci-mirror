import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


class OfflineContainerContract(unittest.TestCase):
    def test_separate_nonroot_locked_inspector_image(self):
        text = (ROOT / "Dockerfile.rust-inspect").read_text(encoding="utf-8")
        self.assertIn("cargo build --release --locked -p tdrive-inspect", text)
        self.assertIn("USER 10001:10001", text)
        self.assertIn('ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/tdrive-inspect"]', text)
        self.assertIn('CMD ["--version"]', text)
        self.assertNotIn("EXPOSE", text)
        self.assertNotIn("COPY . .", text)
        self.assertNotIn("VOLUME", text)


if __name__ == "__main__":
    unittest.main()
