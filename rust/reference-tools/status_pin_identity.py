"""Current parity reference identity for producer metadata and image selection."""
import os
import json
import subprocess
from pathlib import Path

PIN_FULL = (Path(__file__).resolve().parents[1] / "parity/reference.sha").read_text().strip()
if len(PIN_FULL) != 40 or any(character not in "0123456789abcdef" for character in PIN_FULL):
    raise ValueError("parity/reference.sha must contain a complete Git SHA")
PIN = PIN_FULL[:8]
PIN_IMAGE = os.environ.get("PARITY_IMAGE", "campfire-reference")


def verify_image():
    """Reject a stale image before running a direct Docker producer."""
    environment = json.loads(subprocess.check_output(
        ["docker", "image", "inspect", "--format", "{{json .Config.Env}}", PIN_IMAGE],
        text=True,
    ))
    revision = next(value.split("=", 1)[1] for value in environment
                    if value.startswith("GIT_REVISION="))
    assert revision == PIN_FULL, (PIN_IMAGE, revision, PIN_FULL)
    return revision
