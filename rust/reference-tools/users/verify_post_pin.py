#!/usr/bin/env python3
"""Verify the ten approved post-pin oracle inputs against their actual Git objects."""
from pathlib import Path
import hashlib
import json
import subprocess

root=Path(__file__).resolve().parents[2]
base=root / "test-support/post-pin"
ledger=json.loads((base / "source-hashes.json").read_text())
assert len(ledger)==10
for path,digest in ledger.items():
    source=path if path.startswith(("app/","config/")) else "app/views/"+path
    expected=subprocess.check_output(["git","show","2e20b24c3f2be9db8a646a1352c159b4afacad0e:"+source],cwd=root)
    actual=(base / path).read_bytes()
    assert actual==expected and hashlib.sha256(actual).hexdigest()==digest,source
for source,logical in [("app/assets/stylesheets/people.css","people.css"),("app/javascript/controllers/profile_card_controller.js","controllers/profile_card_controller.js")]:
    assert (root / "crates/assets/overrides" / logical).read_bytes()==(base / source).read_bytes(),logical
print("WS8br2 status asset source verification: both Rust asset inputs match the approved Rails bytes")
print("WS8br2 post-pin verification: all 10 approved source files match 2e20b24c byte for byte")
