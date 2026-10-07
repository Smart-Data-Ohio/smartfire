#!/usr/bin/env python3
"""Record or verify shared CI payload bytes against the pinned Git checkout."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys


GIT_SHA = re.compile(r"[0-9a-f]{40}")
SHA256 = re.compile(r"[0-9a-f]{64}")


def checkout_head(expected):
    if not GIT_SHA.fullmatch(expected):
        raise ValueError("--head must be a full 40-character Git SHA")
    result = subprocess.run(["git", "rev-parse", "--verify", "HEAD"],
                            capture_output=True, text=True)
    if result.returncode:
        raise ValueError("cannot read checkout HEAD")
    head = result.stdout.strip()
    if head != expected:
        raise ValueError(f"checkout HEAD mismatch: expected {expected}, got {head}")
    return head


def digest(path):
    hasher = hashlib.sha256()
    try:
        with path.open("rb") as payload:
            for chunk in iter(lambda: payload.read(1024 * 1024), b""):
                hasher.update(chunk)
    except OSError as error:
        raise ValueError(f"cannot read payload {path}: {error.strerror}") from error
    return hasher.hexdigest()


def record(path, head):
    metadata = dict(version=1, file=path.name, head=head, sha256=digest(path))
    Path(str(path) + ".json").write_text(json.dumps(metadata, indent=2) + "\n")


def verify(path, head):
    sidecar = Path(str(path) + ".json")
    try:
        metadata = json.loads(sidecar.read_text())
    except (OSError, ValueError) as error:
        raise ValueError(f"cannot read metadata {sidecar}") from error
    if not isinstance(metadata, dict) or type(metadata.get("version")) is not int or metadata["version"] != 1:
        raise ValueError("invalid metadata version")
    if metadata.get("file") != path.name:
        raise ValueError("metadata file mismatch")
    producer = metadata.get("head")
    if not isinstance(producer, str) or not GIT_SHA.fullmatch(producer):
        raise ValueError("invalid metadata producer HEAD")
    if producer != head:
        raise ValueError(f"producer HEAD mismatch: expected {head}, got {producer}")
    expected_digest = metadata.get("sha256")
    if not isinstance(expected_digest, str) or not SHA256.fullmatch(expected_digest):
        raise ValueError("invalid metadata SHA256")
    if digest(path) != expected_digest:
        raise ValueError(f"payload SHA256 mismatch: {path}")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("record", "verify"))
    parser.add_argument("file", type=Path)
    parser.add_argument("--head", required=True)
    args = parser.parse_args(argv)
    try:
        head = checkout_head(args.head)
        {"record": record, "verify": verify}[args.command](args.file, head)
    except (OSError, ValueError) as error:
        print(f"artifact provenance: {error}", file=sys.stderr)
        return 1
    print(f"{args.command}: {args.file.name} at {head}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
