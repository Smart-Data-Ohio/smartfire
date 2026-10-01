#!/usr/bin/env python3
"""Reject the four newly unblocked cases with compiled missing adapters; restore in finally."""
import os
from pathlib import Path
import re
import subprocess
root = Path(__file__).resolve().parents[3]
scratch = root / '.scratch'
router = root / 'rust/crates/campfire/src/controllers.rs'
rooms = root / 'rust/crates/campfire/src/controllers/rooms.rs'
originals = {p: p.read_bytes() for p in [router, rooms]}
env = dict(os.environ, CI='1', TMPDIR=str(scratch), CARGO_TARGET_DIR=str(root/'rust/target'),
           CABLE_TEST_PORT_RANGE='52100-52149', MAIL_TEST_PORT_RANGE='52100-52149')
base = ['mise', 'exec', 'rust@1.98.1', '--', 'cargo', 'test', '--locked', '-j4', '--manifest-path', 'rust/Cargo.toml', '-p', 'campfire', '--bin', 'campfire', '--']
def reject(label, selectors):
    run = subprocess.run(base + selectors + ['--test-threads=4'], cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    (scratch/f'deferred-{label}-discrimination.log').write_text(run.stdout)
    summaries = [line for line in run.stdout.splitlines() if line.startswith('test result:')]
    assert run.returncode == 101 and len(summaries) == 1 and '0 passed; 2 failed;' in summaries[0], run.stdout
    print(summaries[0], flush=True)
    for path, data in originals.items(): path.write_bytes(data)
try:
    changed = re.sub(r'^\s*"(?:rooms/directs#update|messages#update|messages#destroy)" => arc\([^\n]+\),\n', '', originals[router].decode(), flags=re.M)
    assert changed != originals[router].decode()
    router.write_text(changed)
    reject('notes', ['a_member_can_rename_the_group_and_everyone_sees_the_compact_system_note', 'group_dm_notes_cannot_be_edited_or_deleted'])
    changed = originals[rooms].decode().replace('message_list:Some(list)', 'message_list:Some(String::new())')
    assert changed != originals[rooms].decode()
    rooms.write_text(changed)
    reject('unread', ['show_renders_the_unread_divider_above_the_first_unread_message_on_the_page', 'show_keeps_the_last_page_when_the_first_unread_fell_off_it_and_links_the_pill_to_it'])
finally:
    for path, data in originals.items(): path.write_bytes(data)
print('Deferred case discrimination: four compiled missing-adapter regressions rejected; source restored')
