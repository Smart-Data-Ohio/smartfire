#!/usr/bin/env python3
"""Exact cache identities for the archived Rails image and its validated seeds."""

import hashlib
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


if __name__ == "__main__":
    for key, value in cache_keys(Path(__file__).resolve().parent.parent).items():
        print(f"{key}={value}")
