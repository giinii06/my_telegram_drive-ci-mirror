import unittest
from unittest.mock import patch

from test_rust_gha_run import REPO, SHA, sample
from verify_rust_gha_run import fetch_evidence, validate_run


class EvidenceBoundaryTests(unittest.TestCase):
    def test_go_auth_lock_step_is_mandatory(self):
        run, jobs = sample()
        job = next(j for j in jobs if j["name"] == "legacy-go-tests")
        job["steps"] = [s for s in job["steps"]
                        if s["name"] != "Go data/auth/lock tests"]
        self.assertTrue(validate_run(run, jobs, REPO, SHA))

    def test_mismatched_requested_run_id_fails_before_job_fetch(self):
        run, jobs = sample()
        with patch("verify_rust_gha_run._get_json", side_effect=[
            run, {"total_count": len(jobs), "jobs": jobs}
        ]) as fetch:
            with self.assertRaises(ValueError):
                fetch_evidence(REPO, 999)
            self.assertEqual(fetch.call_count, 1)


if __name__ == "__main__":
    unittest.main()
