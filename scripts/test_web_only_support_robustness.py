"""Adversarial fixtures for the Android retirement contract checker."""
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest import TestCase
from unittest.mock import patch

from scripts import test_web_only_support as web_only


class WebOnlySupportRobustnessTests(TestCase):
    def run_workflow_contract(self, workflow, method):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            workflow_path = root / ".github" / "workflows" / "installers.yml"
            workflow_path.parent.mkdir(parents=True)
            workflow_path.write_text(workflow, encoding="utf-8")
            with patch.object(web_only, "ROOT", root):
                case = web_only.WebOnlySupportTests(method)
                with self.assertRaises(AssertionError):
                    getattr(case, method)()

    def test_renamed_apk_job_is_rejected(self):
        self.run_workflow_contract(
            "jobs:\n  mobile-release:\n    steps:\n      - run: ./gradlew assembleRelease\n",
            "test_release_workflow_has_no_android_apk_job_or_build_path",
        )

    def test_installer_names_in_comments_do_not_count_as_jobs(self):
        self.run_workflow_contract(
            "jobs:\n  no-installers:\n    runs-on: ubuntu-latest\n"
            "# linux-deb:\n# macos-dmg:\n# windows-exe:\n# windows-backup-exe:\n",
            "test_headless_host_installers_are_retained",
        )
