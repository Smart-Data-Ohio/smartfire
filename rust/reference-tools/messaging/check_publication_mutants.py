#!/usr/bin/env python3
"""Reverse real producer batches; require rejection at the ordered Hub assertion.

Run only in an idle worker checkout. Source bytes are restored in finally. The
receiver comparison, fixture bytes, credentials, deadlines and counts stay intact.
"""
import argparse
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--expect', choices=['escape', 'reject'], required=True)
p.add_argument('--producer', choices=['presenter', 'batch'], required=True)
p.add_argument('runner', nargs=argparse.REMAINDER)
a = p.parse_args()
runner = a.runner[1:] if a.runner[:1] == ['--'] else a.runner
assert runner
originals = {}

def replace(relative, old, new):
    path = ROOT / relative
    raw = path.read_bytes()
    text = raw.decode()
    assert text.count(old) == 1, (relative, old)
    originals[path] = raw
    path.write_text(text.replace(old, new))

try:
    if a.producer == 'presenter':
        replace('rust/crates/campfire/src/controllers/presenters/link_embeds.rs',
                'let messages = message_batches::next(conn, Source::LinkEmbed(embed_id), after)?;',
                'let mut messages = message_batches::next(conn, Source::LinkEmbed(embed_id), after)?; messages.reverse();')
        filters = ['older_embed_children_tests::queued_stale']
    else:
        replace('rust/crates/campfire/src/integrations/message_batches.rs',
                'messages.sort_by_key(|m| m.id);',
                'messages.sort_by_key(|m| m.id); messages.reverse();')
        filters = ['older_embed_children_tests::queued_stale', 'older_embed_job_tests',
                   'mapped_provider_tests', 'older_provider_tests', 'older_owner_tests',
                   'older_calendar_tests',
                   'comparison_support::ordered_capture_waits_for_actual_callback']
    if a.expect == 'escape':
        replace('rust/crates/campfire/src/controllers/message_features/comparison_support.rs',
                'assert_eq!(json!(actual), *expected, "ordered publication differs from Rails: {context}");',
                'println!("WS8bm2 reversed producer observed: {} publications; ordered_equal={}", actual.len(), json!(actual)==*expected);')
    for name in filters:
        result = subprocess.run(runner + ['test', '--locked', '-p', 'campfire', '--bin', 'campfire', name,
                                         '-j2', '--', '--test-threads=4', '--nocapture'],
                                cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        print(result.stdout, flush=True)
        if a.expect == 'escape':
            assert result.returncode == 0 and 'ordered_equal=false' in result.stdout, name
        else:
            assert result.returncode != 0 and 'assertion `left == right` failed: ordered publication differs from Rails:' in result.stdout, name
        print(f'WS8bm2 publication mutant {a.producer}/{name}: {a.expect} confirmed', flush=True)
finally:
    for path, raw in originals.items():
        path.write_bytes(raw)
