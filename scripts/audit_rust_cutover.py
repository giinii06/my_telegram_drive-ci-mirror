#!/usr/bin/env python3
"""Read-only tracked-source audit for a future Go-free Rust cutover.

--report never claims the migration is complete. --enforce intentionally fails
until all tracked Go source files and Go module files are actually removed.
Never edits a file or invokes Go.
"""
from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path


def summarize(paths: list[str]) -> dict[str, object]:
    go_files = sorted(path for path in paths if path.endswith(".go"))
    modules = sorted(path for path in paths if path in {"go.mod", "go.sum"})
    return {
        "go_source_files": len(go_files),
        "go_module_files": modules,
        "go_free": not go_files and not modules,
        "remaining_go_by_area": {
            area: sum(1 for path in go_files if path.startswith(area))
            for area in ("backend/", "cmd/", "tests/", "other/")
        },
    }


def tracked_paths(root: Path) -> list[str]:
    response = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=True,
        timeout=15,
    )
    return [
        entry.decode("utf-8", errors="surrogateescape")
        for entry in response.stdout.split(b"\0")
        if entry
    ]


def main() -> int:
    parser = argparse.ArgumentParser(description="Audit remaining tracked Go code")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--report", action="store_true")
    group.add_argument("--enforce", action="store_true")
    args = parser.parse_args()
    try:
        root = Path(__file__).resolve().parents[1]
        report = summarize(tracked_paths(root))
    except (OSError, subprocess.SubprocessError) as error:
        print(json.dumps({"result": "blocked", "reason": type(error).__name__}))
        return 2
    print(json.dumps(report, sort_keys=True))
    return 1 if args.enforce and not report["go_free"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
