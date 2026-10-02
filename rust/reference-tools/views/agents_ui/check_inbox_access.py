#!/usr/bin/env python3
"""Prove the inbox privacy test rejects the wrong administrator role; restore source."""
import os, pathlib, subprocess
root = pathlib.Path(__file__).resolve().parents[4]
source = root/'rust/crates/db/src/models/activity_item/access.sql'
original = source.read_bytes()
mutant = original.replace(b'OR users.role = 1', b'OR users.role = 0')
assert mutant != original
try:
    source.write_bytes(mutant)
    result = subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--manifest-path','rust/Cargo.toml','--locked','-p','campfire','--bin','campfire','inbox_reader_excludes_other_owners_and_revoked_message_sources','--','--test-threads=8'], cwd=root, env={**os.environ,'CI':'1','CARGO_BUILD_JOBS':'2'},stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
    assert result.returncode != 0 and '0 passed; 1 failed' in result.stdout, result.stdout
    print('Inbox privacy mutant: 0 passed; 1 failed (wrong administrator role rejected)')
finally:
    source.write_bytes(original)
