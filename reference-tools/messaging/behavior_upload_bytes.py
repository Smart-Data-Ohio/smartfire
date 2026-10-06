"""Read the real disk service used by this upload, before unconditional teardown."""
from pathlib import Path


def uploaded_bytes(storage: Path, key: str):
    return (storage / key[:2] / key[2:4] / key).read_bytes()
