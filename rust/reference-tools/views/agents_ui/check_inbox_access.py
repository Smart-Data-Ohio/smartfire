#!/usr/bin/env python3
"""Prove the inbox privacy test rejects the wrong administrator role; restore source."""
import pathlib
from discrimination import require_baseline, require_rejected, run_tests
root = pathlib.Path(__file__).resolve().parents[4]
source = root/'rust/crates/db/src/models/activity_item/access.sql'
original = source.read_bytes()
mutant = original.replace(b'OR users.role = 1', b'OR users.role = 0')
assert mutant != original
require_baseline('inbox_reader_excludes_other_owners_and_revoked_message_sources')
try:
    source.write_bytes(mutant)
    result = run_tests('inbox_reader_excludes_other_owners_and_revoked_message_sources')
    require_rejected(result, {'inbox_reader_excludes_other_owners_and_revoked_message_sources': ('crates/campfire/src/controllers/presenters/accounts/tests/navigation_inbox.rs', 'inbox must exclude revoked message sources and approvals owned by another user')})
    print('Inbox privacy mutant: 0 passed; 1 failed (wrong administrator role rejected)')
finally:
    source.write_bytes(original)
