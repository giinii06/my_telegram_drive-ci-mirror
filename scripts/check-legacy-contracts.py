#!/usr/bin/env python3
"""Check source-backed TDrive compatibility invariants without Telegram credentials.

This is an inventory guard, NOT a substitute for executing Go tests, Rust
contract tests, or a real database migration.
"""

import argparse
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXPECTED = ROOT / "tests" / "fixtures" / "legacy-contracts.json"


def read(rel):
    return (ROOT / rel).read_text(encoding="utf-8")


def extract_version(source, expression, label):
    match = re.search(expression, source)
    if not match:
        raise AssertionError(f"missing {label} declaration in legacy source")
    return int(match.group(1))


def snapshot():
    schema = read("backend/projection/schema.go")
    ipc = read("backend/daemon/protocol.go")
    wire = read("backend/projection/wire.go")
    op_types = read("backend/projection/types.go")
    crypto = read("backend/crypto/stream.go")
    web = read("cmd/tdrive/web.go")
    docker = read("docker-compose.yml")
    return {
        "schema_version": extract_version(schema, r"currentSchemaVersion\s*=\s*(\d+)", "schema version"),
        "daemon_protocol_version": extract_version(ipc, r"const ProtocolVersion\s*=\s*(\d+)", "daemon protocol"),
        "wire_magic": re.search(r'wireVersionPrefix\s*=\s*"([^"]+)"', wire).group(1),
        "encryption_magic": re.search(r'streamMagic\s*=\s*"([^"]+)"', crypto).group(1),
        "encryption_chunk_log2": extract_version(crypto, r"chunkSizeLog2\s*=\s*(\d+)", "chunk size"),
        "op_types": sorted(set(re.findall(r'Op\w+\s+OpType\s*=\s*"([^"]+)"', op_types))),
        "http_routes": sorted(set(re.findall(r'mux\.HandleFunc\("([^"]+)"', web))),
        "docker_uses_tailscale_network": "network_mode: service:tailscale" in docker,
        "docker_disables_os_mount": "TDRIVE_NO_OS_MOUNT=1" in docker,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--print", action="store_true", help="print source-derived inventory")
    args = parser.parse_args()
    observed = snapshot()
    if args.print:
        print(json.dumps(observed, ensure_ascii=False, indent=2))
        return
    expected = json.loads(EXPECTED.read_text(encoding="utf-8"))
    differences = {key: {"expected": expected.get(key), "observed": observed.get(key)}
                   for key in sorted(set(expected) | set(observed))
                   if expected.get(key) != observed.get(key)}
    if differences:
        raise SystemExit("Legacy contract drift:\n" + json.dumps(differences, ensure_ascii=False, indent=2))
    print(f"Legacy source contract inventory OK: {len(observed['op_types'])} op types, "
          f"{len(observed['http_routes'])} HTTP routes")


if __name__ == "__main__":
    main()
