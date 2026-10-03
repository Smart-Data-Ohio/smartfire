#!/usr/bin/env python3
"""Exact cache identities for the archived Rails image and its validated seeds."""

import argparse
import hashlib
import json
from pathlib import Path
import re

IMAGE_INPUTS = (
    "parity/reference.sha",
    "../db/schema.rb",
    "../db/migrate",
    "parity/docker",
    "parity/bin/reference",
    "parity/bin/ci-seed",
    "parity/ci_seed.py",
)
SEED_INPUTS = (
    *IMAGE_INPUTS,
    "parity/.env.reference",
    "parity/bin/seed",
    "parity/seeds",
    "reference-tools/campfire/verify_parity_seed.rb",
)


def fingerprint(root, inputs):
    digest = hashlib.sha256()
    for name in inputs:
        source = root / name
        files = sorted(source.rglob("*")) if source.is_dir() else [source]
        for file in files:
            if file.is_dir():
                continue
            if file.is_symlink() or not file.is_file():
                raise ValueError(f"cache input must be a regular file: {file}")
            # Frame both the path and contents to avoid ambiguous concatenations.
            for value in (file.relative_to(root).as_posix().encode(), file.read_bytes()):
                digest.update(len(value).to_bytes(8, "big"))
                digest.update(value)
    return digest.hexdigest()


def cache_keys(root):
    pin = (root / "parity/reference.sha").read_text().strip()
    if not re.fullmatch(r"[0-9a-f]{40}", pin):
        raise ValueError("parity/reference.sha must contain a full Rails commit SHA")
    # The pin covers Rails behavior, fixtures and build inputs. The checkout's
    # schema/migrations are overlaid so seeds match the generated Rust schema.
    return {
        "pin": pin,
        "image_key": "rust-parity-image-v1-" + fingerprint(root, IMAGE_INPUTS),
        "seed_key": "rust-parity-seed-v1-" + fingerprint(root, SEED_INPUTS),
    }


def archive_sha256(archive):
    digest = hashlib.sha256()
    with archive.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def archive_identity_path(archive):
    return archive.with_name(archive.name + ".identity.json")


def archive_matches(root, archive):
    expected = cache_keys(root)["image_key"]
    try:
        identity = json.loads(archive_identity_path(archive).read_text())
        return (isinstance(identity, dict) and identity.get("image_key") == expected
                and identity.get("sha256") == archive_sha256(archive))
    except (OSError, ValueError):
        return False


def record_archive_identity(root, archive):
    identity = {"image_key": cache_keys(root)["image_key"], "sha256": archive_sha256(archive)}
    receipt = archive_identity_path(archive)
    temporary = receipt.with_suffix(".tmp")
    temporary.write_text(json.dumps(identity, sort_keys=True) + "\n")
    temporary.replace(receipt)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group()
    action.add_argument("--check-archive", type=Path)
    action.add_argument("--record-archive", type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    if args.check_archive:
        raise SystemExit(0 if archive_matches(root, args.check_archive) else 1)
    if args.record_archive:
        record_archive_identity(root, args.record_archive)
    else:
        for key, value in cache_keys(root).items():
            print(f"{key}={value}")
