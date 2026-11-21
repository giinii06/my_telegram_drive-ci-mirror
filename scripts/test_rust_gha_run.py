#!/usr/bin/env python3
"""Offline regression cases for GitHub Actions *actual job* evidence checks.

Synthetic JSON tests check the validator logic, not execution of real jobs.
Run with: python3 scripts/test_rust_gha_run.py
"""
import copy
import unittest

from verify_rust_gha_run import EXPECTED_JOBS, REQUIRED_STEPS, validate_run

REPO = "giinii06/my_telegram_drive-ci-mirror"
SHA = "1a2b3c4d" * 5


def sample():
    run = {
        "id": 123, "repository": {"full_name": REPO},
        "name": "Rust migration parallel CI",
        "path": ".github/workflows/rust-migration-parallel.yml@refs/heads/ci/rust-parallel-workflow-repair-20261122",
        "head_sha": SHA, "status": "completed", "conclusion": "success",
    }
    jobs = []
    for name in EXPECTED_JOBS:
        jobs.append({
            "name": name, "status": "completed", "conclusion": "success",
            "steps": [
                {"name": s, "status": "completed", "conclusion": "success"}
                for s in REQUIRED_STEPS[name]
            ],
        })
    return run, jobs


class EvidenceTests(unittest.TestCase):
    def test_exact_run_and_all_six_jobs_pass(self):
        run, jobs = sample()
        self.assertEqual(validate_run(run, jobs, REPO, SHA), [])

    def test_wrong_sha_rejected(self):
        run, jobs = sample()
        run["head_sha"] = "e" * 40
        self.assertTrue(validate_run(run, jobs, REPO, SHA))

    def test_wrong_receiver_workflow_rejected(self):
        run, jobs = sample()
        run["path"] = ".github/workflows/gitlab-request.yml@refs/heads/main"
        self.assertTrue(validate_run(run, jobs, REPO, SHA))

    def test_generic_success_callback_is_not_job_evidence(self):
        run, _ = sample()
        self.assertTrue(validate_run(run, [], REPO, SHA))

    def test_wrong_repo_rejected(self):
        run, jobs = sample()
        run["repository"]["full_name"] = "someone/another-repo"
        self.assertTrue(validate_run(run, jobs, REPO, SHA))

    def test_incomplete_provider_run_rejected(self):
        run, jobs = sample()
        for status, conclusion in [("queued", None), ("in_progress", None), ("completed", "failure")]:
            run["status"], run["conclusion"] = status, conclusion
            self.assertTrue(validate_run(run, jobs, REPO, SHA))

    def test_unfinished_or_failed_job_rejected(self):
        run, jobs = sample()
        jobs[1]["status"] = "in_progress"
        self.assertTrue(validate_run(run, jobs, REPO, SHA))
        run, jobs = sample()
        jobs[2]["conclusion"] = "skipped"
        self.assertTrue(validate_run(run, jobs, REPO, SHA))

    def test_missing_or_duplicated_jobs_rejected(self):
        run, jobs = sample()
        self.assertTrue(validate_run(run, jobs[:-1], REPO, SHA))
        run, jobs = sample()
        self.assertTrue(validate_run(run, jobs + [copy.deepcopy(jobs[0])], REPO, SHA))

    def test_missing_checkout_sha_step_rejected(self):
        run, jobs = sample()
        jobs[2]["steps"] = [s for s in jobs[2]["steps"] if s["name"] != "Verify checkout SHA"]
        self.assertTrue(validate_run(run, jobs, REPO, SHA))

    def test_failed_sha_attestation_rejected(self):
        run, jobs = sample()
        jobs[2]["steps"][0]["conclusion"] = "failure"
        self.assertTrue(validate_run(run, jobs, REPO, SHA))

    def test_missing_cargo_test_or_clippy_rejected(self):
        run, jobs = sample()
        j = next(j for j in jobs if j["name"] == "rust-contracts")
        for required in ("Rust Clippy", "Rust unit tests"):
            altered = copy.deepcopy(jobs)
            target = next(k for k in altered if k["name"] == "rust-contracts")
            target["steps"] = [s for s in target["steps"] if s["name"] != required]
            self.assertTrue(validate_run(run, altered, REPO, SHA))

    def test_missing_go_fixture_generation_rejected(self):
        run, jobs = sample()
        j = next(j for j in jobs if j["name"] == "go-fixtures")
        j["steps"] = [s for s in j["steps"] if s["name"] != "Generate Go name fixtures"]
        self.assertTrue(validate_run(run, jobs, REPO, SHA))

    def test_missing_bun_checks_rejected(self):
        run, jobs = sample()
        j = next(j for j in jobs if j["name"] == "web-checks")
        j["steps"] = [s for s in j["steps"] if s["name"] != "Bun unit tests"]
        self.assertTrue(validate_run(run, jobs, REPO, SHA))

    def test_failed_required_step_rejected(self):
        run, jobs = sample()
        j = next(j for j in jobs if j["name"] == "legacy-go-tests")
        j["steps"][-1]["conclusion"] = "failure"
        self.assertTrue(validate_run(run, jobs, REPO, SHA))

    def test_no_steps_is_not_success(self):
        run, jobs = sample()
        jobs[2]["steps"] = []
        self.assertTrue(validate_run(run, jobs, REPO, SHA))

    def test_invalid_json_not_accepted(self):
        run, jobs = sample()
        self.assertTrue(validate_run({"status": "completed", "conclusion": "success"}, jobs, REPO, SHA))
        self.assertTrue(validate_run(run, [{"name": "legacy-contracts"}], REPO, SHA))


if __name__ == "__main__":
    unittest.main(verbosity=2)
