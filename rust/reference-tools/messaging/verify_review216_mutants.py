#!/usr/bin/env python3
"""Mutate real production output/writes and require WS8's comparisons to reject it.

Pass a configured Cargo runner after --, for example:
  python3 reference-tools/messaging/verify_review216_mutants.py comparison -- cargo
Source bytes are restored even when compilation or verification fails. Run only
in an isolated, idle worker checkout; this intentionally rebuilds the test binary.
"""
import argparse
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('mode', choices=['comparison', 'calendar-error', 'siblings'])
p.add_argument('runner', nargs=argparse.REMAINDER)
a = p.parse_args()
runner = a.runner[1:] if a.runner[:1] == ['--'] else a.runner
assert runner, 'Supply a configured Cargo runner after --'
changes = {}

def replace(path, old, new):
    path = ROOT / path
    original = path.read_bytes()
    text = original.decode()
    assert text.count(old) == 1, f'mutation site moved: {path}'
    changes[path] = original
    path.write_text(text.replace(old, new))

try:
    if a.mode == 'comparison':
        replace('rust/crates/db/src/slash_commands.rs', 'r.room_id = Some(c.room_id);', 'r.room_id = Some(-1);')
        path = ROOT / 'rust/crates/campfire/src/channels/sink.rs'
        original = path.read_bytes()
        text = original.decode()
        start, end = text.index('fn status_badge('), text.index('fn ooo_notice(')
        section = text[start:end]
        assert section.count(',"status"') == 1
        changes[path] = original
        path.write_text(text[:start] + section.replace(',"status"', ',"ooo_notice"') + text[end:])
        path = ROOT / 'rust/crates/db/src/models/google_entry.rs'
        original = path.read_bytes()
        text = original.decode()
        start, end = text.index('pub fn success('), text.index('\npub fn failure(')
        changes[path] = original
        path.write_text(text[:start] + "pub fn success(_tx: &mut Tx<'_>, _entry: &Entry) -> Result<()> { Ok(()) }\n" + text[end:])
        checks = [('slash_named_tests', ['builtin_huddle_starts_a_call_when_configured ... FAILED', 'builtin_ooo_broadcasts_the_badge_and_the_notice ... FAILED']),
                  ('older_calendar_execution_tests', ['older_calendar_inbound_and_sync_jobs_execute_queued_children_like_rails_with_flat_reads ... FAILED', 'calendar_fatal_writes_reach_queue_outcomes ... FAILED'])]
    elif a.mode == 'calendar-error':
        replace('rust/crates/campfire/src/integrations/google/entry_sync.rs', 'app.db.write(move |tx| entries::success(tx, &entry)).await?;', 'let _ = app.db.write(move |tx| entries::success(tx, &entry)).await;')
        checks = [('calendar_fatal_writes_reach_queue_outcomes', ['calendar_fatal_writes_reach_queue_outcomes ... FAILED'])]
    else:
        replace('rust/crates/campfire/src/integrations/link_embed/store.rs', '''tx.before_commit_record_latest("LinkEmbed#stale_siblings", self.id, move |tx| {
            embed.request_stale_siblings(tx)
        })?;''', 'embed.request_stale_siblings(tx)?;')
        checks = [('final_state_sibling_claims_match_rails', ['final_state_sibling_claims_match_rails ... FAILED'])]
    for test, failures in checks:
        result = subprocess.run(runner + ['test', '--locked', '-p', 'campfire', '--bin', 'campfire', test, '-j', '2', '--', '--test-threads=4', '--nocapture'], cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        print(result.stdout, flush=True)
        assert result.returncode != 0 and all(f in result.stdout for f in failures), f'{test}: mutation survived or compilation failed'
    print(f'WS8bm2 review216 mutations: {a.mode} rejected by actual-output regressions', flush=True)
finally:
    for path, original in changes.items():
        path.write_bytes(original)
