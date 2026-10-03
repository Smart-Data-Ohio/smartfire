#!/usr/bin/env python3
"""Prove each inbox lifecycle branch catches a real writer defect."""
from discrimination import require_baseline, require_rejected, run_tests

test = 'ws11ui_next_inbox_lifecycle_matches_rails_producers_and_response_bytes'
source = 'crates/campfire/src/controllers/activity_items/tests/lifecycle.rs'
require_baseline(test)
for defect, assertion in [
    ('reminder', 'real reminder producer must execute successfully: Sqlite(QueryReturnedNoRows)'),
    ('recurrence', 'Inbox lifecycle response bytes must match Rails: recurrent reminder inbox'),
    ('deleted-source', 'deleted source must actually be removed by the real writer'),
]:
    result = run_tests(test, {'WS11UI_INBOX_LIFECYCLE_DEFECT': defect})
    require_rejected(result, {test: (source, assertion)})
    print(f'Inbox lifecycle discrimination: {defect} writer defect rejected at "{assertion}"', flush=True)
