#!/usr/bin/env python3
"""Fail-closed GH Actions actual-job evidence verifier for TDrive Rust migration.

Public GH REST endpoints only. A generic GitLab/GitHub success callback without
job details is NEVER sufficient. A successful run on GitHub's default branch
is not evidence for a different private GitLab source SHA.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import sys
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

EXPECTED_JOBS = (
    "legacy-contracts",
    "rust-style",
    "go-fixtures",
    "rust-contracts",
    "legacy-go-tests",
    "web-checks",
)
REQUIRED_STEPS = {
    "legacy-contracts": (
        "Verify checkout SHA",
        "Legacy contract check",
        "Rust workflow contract check",
        "Rust workflow regression tests",
        "GitHub evidence verifier regression tests",
    ),
    "rust-style": ("Verify checkout SHA", "Rust format"),
    "go-fixtures": (
        "Verify checkout SHA",
        "Generate Go V16 fixtures",
        "Generate Go TDE1 fixtures",
        "Generate Go name fixtures",
        "Require all Go fixture outputs",
        "Upload Go/Rust fixtures",
    ),
    "rust-contracts": (
        "Verify checkout SHA",
        "Assert Go-generated artifacts",
        "Rust format",
        "Rust Clippy",
        "Rust unit tests",
    ),
    "legacy-go-tests": (
        "Verify checkout SHA",
        "Go projection tests",
        "Go crypto tests",
        "Go Telegram tests",
        "Go backup tests",
        "Go media range tests",
    ),
    "web-checks": ("Verify checkout SHA", "Bun lint", "Bun unit tests"),
}
EXPECTED_WORKFLOW_NAME = "Rust migration parallel CI"
EXPECTED_WORKFLOW_PATH = ".github/workflows/rust-migration-parallel.yml"


def validate_run(run: object, jobs: object, repo: str, sha: str) -> list[str]:
    """Validate *supplied* GH run/job responses; not independent authenticity."""
    errors: list[str] = []
    if not isinstance(run, dict):
        return ["run JSON is not an object"]
    if not isinstance(jobs, list):
        return ["job list JSON is not an array"]
    github_repo = run.get("repository")
    if not isinstance(github_repo, dict) or github_repo.get("full_name") != repo:
        errors.append("GitHub repository mismatch")
    if run.get("head_sha") != sha:
        errors.append("run HEAD SHA differs from requested GitLab commit")
    if run.get("name") != EXPECTED_WORKFLOW_NAME:
        errors.append("run belongs to wrong GitHub workflow")
    path = run.get("path")
    if not isinstance(path, str) or path.split("@", 1)[0] != EXPECTED_WORKFLOW_PATH:
        errors.append("run workflow file path is not the Rust migration workflow")
    if run.get("status") != "completed" or run.get("conclusion") != "success":
        errors.append("GitHub run has not completed successfully")

    found: dict[str, dict] = {}
    for i, job in enumerate(jobs):
        if not isinstance(job, dict) or not isinstance(job.get("name"), str):
            errors.append(f"job record {i + 1} is malformed")
            continue
        name = job["name"]
        if name in found:
            errors.append(f"duplicate job: {name}")
        found[name] = job
    if set(found) != set(EXPECTED_JOBS) or len(jobs) != len(EXPECTED_JOBS):
        errors.append("missing, duplicate or unexpected GitHub jobs; six actual jobs required")

    for name in EXPECTED_JOBS:
        job = found.get(name)
        if job is None:
            errors.append(f"required job not found: {name}")
            continue
        if job.get("status") != "completed" or job.get("conclusion") != "success":
            errors.append(f"required job not successful: {name}")
        steps = job.get("steps")
        if not isinstance(steps, list):
            errors.append(f"no step execution evidence: {name}")
            continue
        observed: dict[str, dict] = {}
        for step in steps:
            if isinstance(step, dict) and isinstance(step.get("name"), str):
                if step["name"] in observed:
                    errors.append(f"duplicate named step in {name}: {step['name']}")
                observed[step["name"]] = step
        for required in REQUIRED_STEPS[name]:
            step = observed.get(required)
            if step is None:
                errors.append(f"missing executed step {name}/{required}")
            elif step.get("status") != "completed" or step.get("conclusion") != "success":
                errors.append(f"step not successful {name}/{required}")
    return sorted(set(errors))


def _get_json(api_path: str, token: str | None) -> object:
    # Caller constructs api_path exclusively from tightly validated repo and run id.
    url = "https://api.github.com" + api_path
    headers = {
        "Accept": "application/vnd.github+json",
        "User-Agent": "TDrive-Rust-CI-evidence-checker/1",
        "X-GitHub-Api-Version": "2022-11-28",
    }
    if token:
        headers["Authorization"] = "Bearer " + token
    req = Request(url, headers=headers)
    try:
        with urlopen(req, timeout=12) as response:
            # Bound public API responses so malicious/incorrect replies do not
            # exhaust memory. An oversized response is a failure.
            body = response.read(2_000_001)
    except (OSError, HTTPError, URLError) as error:
        raise ValueError("GitHub API request unavailable (network/permissions/status)") from error
    if len(body) > 2_000_000:
        raise ValueError("GitHub API JSON response exceeded size limit")
    try:
        return json.loads(body)
    except (UnicodeError, ValueError) as error:
        raise ValueError("GitHub API did not return valid JSON") from error


def fetch_evidence(repo: str, run_id: int, token: str | None = None) -> tuple[dict, list[dict]]:
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repo):
        raise ValueError("invalid owner/repository")
    if run_id < 1:
        raise ValueError("run ID must be positive")
    base = f"/repos/{repo}/actions/runs/{run_id}"
    run = _get_json(base, token)
    if not isinstance(run, dict):
        raise ValueError("run endpoint returned invalid data")
    jobs: list[dict] = []
    total: int | None = None
    for page in range(1, 21):
        response = _get_json(base + f"/jobs?per_page=100&page={page}", token)
        if not isinstance(response, dict):
            raise ValueError("jobs endpoint returned invalid data")
        count = response.get("total_count")
        records = response.get("jobs")
        if not isinstance(count, int) or count < 0 or count > 2000 or not isinstance(records, list):
            raise ValueError("jobs API response is incomplete/invalid")
        if total is None:
            total = count
        if count != total:
            raise ValueError("jobs API pagination changed during collection")
        jobs.extend(records)
        if len(jobs) >= total:
            break
        if not records:
            raise ValueError("job pagination ended early")
    if total is None or len(jobs) != total:
        raise ValueError("job pagination incomplete")
    return run, jobs


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Verify real GitHub run/job/step evidence for exact GitLab SHA"
    )
    parser.add_argument("--repo", required=True)
    parser.add_argument("--run-id", required=True, type=int)
    parser.add_argument("--sha", required=True)
    args = parser.parse_args()
    if not re.fullmatch(r"[a-fA-F0-9]{40}", args.sha):
        parser.error("--sha must be a full 40-character GitLab commit SHA")
    token = os.environ.get("GITHUB_TOKEN")
    try:
        run, jobs = fetch_evidence(args.repo, args.run_id, token)
        errors = validate_run(run, jobs, args.repo, args.sha)
    except ValueError as error:
        errors = [str(error)]
        run, jobs = {}, []
    report = {
        "result": "jobs_verified" if not errors else "blocked",
        "repo": args.repo, "source_sha": args.sha,
        "run_id": args.run_id, "run_sha": run.get("head_sha"),
        "workflow_path": run.get("path"), "job_count": len(jobs),
        "validated_job_names": [j.get("name") for j in jobs if isinstance(j, dict)],
        "errors": errors,
        "evidence_origin": "GitHub REST actual run/jobs (not GitLab external callback)",
        "caution": "A valid job report proves only these named CI steps, not full Telegram E2E or macOS operation.",
    }
    print(json.dumps(report, indent=2, ensure_ascii=False))
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
