"""Current parity reference identity for producer metadata and image selection."""
import os
from pathlib import Path

PIN_FULL = (Path(__file__).resolve().parents[2] / "parity/reference.sha").read_text().strip()
if len(PIN_FULL) != 40 or any(character not in "0123456789abcdef" for character in PIN_FULL):
    raise ValueError("parity/reference.sha must contain a complete Git SHA")
PIN = PIN_FULL[:8]
PIN_IMAGE = os.environ.get("PARITY_IMAGE", "campfire-reference")
