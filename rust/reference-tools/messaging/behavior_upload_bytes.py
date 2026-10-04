"""Read the real disk service used by this upload, before unconditional teardown."""
from pathlib import Path
import subprocess


def uploaded_bytes(storage: Path, key: str, *, rails_test_port=None, read_container=None):
    relative = Path(key[:2]) / key[2:4] / key
    if rails_test_port is not None:
        # d7c7de92 config/storage.yml:3: test.root = Rails.root/tmp/storage.
        # Unlike local's mounted storage/files, this disk lives in the live
        # reference container. Read it while the app is still running.
        reader = read_container or subprocess.check_output
        return reader([
            "docker", "exec", f"ws8bm-behavior-reference-{rails_test_port}",
            "cat", str(Path("/rails/tmp/storage") / relative),
        ])
    return (storage / relative).read_bytes()
