#!/usr/bin/env python3
"""Regression contract: Android APK support must stay retired from current releases."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class WebOnlySupportTests(unittest.TestCase):
    def test_android_packaging_project_is_absent(self):
        self.assertFalse((ROOT / "packaging" / "android").exists())

    def test_release_workflow_has_no_android_apk_job_or_build_path(self):
        workflow = (ROOT / ".github" / "workflows" / "installers.yml").read_text()
        self.assertNotRegex(workflow, r"(?m)^\\s*android-apk:")
        self.assertNotIn("packaging/android", workflow)

    def test_current_install_docs_do_not_advertise_android_apk(self):
        for relative in ("README.md", "packaging/README.md"):
            content = (ROOT / relative).read_text().lower()
            self.assertNotIn(".apk", content)
            self.assertNotIn("android", content)

    def test_headless_host_installers_are_retained(self):
        workflow = (ROOT / ".github" / "workflows" / "installers.yml").read_text()
        for job in ("linux-deb:", "macos-dmg:", "windows-exe:", "windows-backup-exe:"):
            self.assertIn(job, workflow)


if __name__ == "__main__":
    unittest.main()
