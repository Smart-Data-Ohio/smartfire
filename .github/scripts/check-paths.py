#!/usr/bin/env python3
"""Select CI steps from the event diff; unavailable history runs the checks."""
import json
import os
import re
import subprocess


def needed(event, pattern):
    if event.get("pull_request"):
        pr = event["pull_request"]
        revision = f'{pr["base"]["sha"]}...{pr["head"]["sha"]}'
    elif "before" in event and set(event["before"]) != {"0"}:
        revision = f'{event["before"]}..{event["after"]}'
    else:
        return True
    paths = subprocess.check_output([
        "git", "diff", "--no-renames", "--name-only", "-z", revision, "--",
    ]).decode().split("\0")
    return any(re.search(pattern, path) for path in paths if path)


def main():
    run = True
    try:
        with open(os.environ["GITHUB_EVENT_PATH"]) as source:
            run = needed(json.load(source), os.environ["PATTERN"])
    except (OSError, KeyError, ValueError, TypeError, subprocess.CalledProcessError) as error:
        print(f"::warning::Diff unavailable ({type(error).__name__}); running the checks.")
    with open(os.environ["GITHUB_OUTPUT"], "a") as output:
        output.write(f"run={str(run).lower()}\n")
    print("Run the checks" if run else "No relevant inputs changed; checks passed without setup.")


if __name__ == "__main__":
    main()
