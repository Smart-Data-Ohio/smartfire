#!/usr/bin/env python3
"""Run ignored correctness tests from the producer's nextest archive."""

import argparse
from pathlib import Path
import subprocess


def run(archive, repo, filterset, *, env=None):
    repo = Path(repo).resolve()
    archive = Path(archive).resolve()
    if not archive.is_file() or archive.stat().st_size == 0:
        raise ValueError(f"Correctness archive is missing or empty: {archive}")
    extract = repo / "target/correctness-archive"
    extract.mkdir(parents=True, exist_ok=True)
    subprocess.run([
        "cargo", "nextest", "run", "--archive-file", str(archive),
        "--workspace-remap", str(repo), "--extract-to", str(extract),
        "--profile", "ci", "-j", "4", "--no-fail-fast", "--success-output", "final",
        "--run-ignored", "only", "--no-tests", "fail", "-E", filterset,
    ], cwd=repo, env=env, check=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--filter", required=True)
    args = parser.parse_args()
    try:
        run(args.archive, args.repo, args.filter)
    except (ValueError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"{error}\n")
