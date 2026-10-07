#!/usr/bin/env python3
"""Validate the SPA mode and emit an allowlisted payload to SSH stdin."""

import json
import os
import re
import sys

MODES = {
    "unchanged": {},
    "off": {"SPA_ENABLED": "0", "SPA_DEFAULT": "classic"},
    "opt-in": {"SPA_ENABLED": "1", "SPA_DEFAULT": "classic"},
    "default-next": {"SPA_ENABLED": "1", "SPA_DEFAULT": "next"},
}


def configuration(environ):
    host = environ["GCP_APP_HOST"]
    if not re.fullmatch(r"[a-zA-Z0-9](?:[a-zA-Z0-9.-]*[a-zA-Z0-9])?", host):
        raise ValueError("Invalid application host")
    mode = environ["SPA_MODE"]
    if mode not in MODES:
        raise ValueError("Invalid SPA mode")
    dry_run = environ["DRY_RUN"]
    if dry_run not in {"true", "false"}:
        raise ValueError("Invalid dry-run mode")
    return {"mode": "plan" if dry_run == "true" else "apply", "host": host,
            "environment": dict(MODES[mode])}


if __name__ == "__main__":
    try:
        payload = configuration(os.environ)
        if sys.argv[1:] == ["--validate"]:
            print(json.dumps({"spa_mode": os.environ["SPA_MODE"], "environment": payload["environment"]}))
        elif not sys.argv[1:]:
            if not payload["environment"]:
                raise ValueError("Unchanged mode has no settings to apply")
            payload["expected_revision"] = os.environ["EXPECTED_REVISION"]
            payload["expected_image"] = os.environ["EXPECTED_IMAGE"]
            print(json.dumps(payload))
        else:
            raise ValueError("Unsupported arguments")
    except Exception:
        print("SPA configuration is invalid; check spa_mode and the deployment inputs", file=sys.stderr)
        sys.exit(1)
