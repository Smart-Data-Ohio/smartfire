#!/usr/bin/env python3
"""Reject real output and query-growth faults in the slice-I comparisons.

Only producer source is changed. Fixtures, query capture, deadlines and existing
comparators are untouched. Sources are restored byte-for-byte in finally.
"""
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
runner = sys.argv[1:]
if runner[:1] == ['--']:
    runner = runner[1:]
assert runner
originals = {}

def replace(relative, old, new):
    path = ROOT / relative
    raw = path.read_bytes()
    text = raw.decode()
    assert text.count(old) == 1, (relative, old)
    originals.setdefault(path, raw)
    path.write_text(text.replace(old, new))

selector = ('campfire_db::slash_commands::time_parser::WS8_LIST_MUTANT'
            '.load(std::sync::atomic::Ordering::SeqCst)')
try:
    replace('rust/crates/db/src/slash_commands/time_parser.rs',
            'fn from_match(c: &Captures',
            'pub static WS8_LIST_MUTANT: std::sync::atomic::AtomicU8 = '
            'std::sync::atomic::AtomicU8::new(0);\nfn from_match(c: &Captures')
    replace('rust/crates/db/src/slash_commands/time_parser.rs',
            'if unit.starts_with("min") { 60 } else { 3600 }',
            'if unit.starts_with("min") { 60 } else if '
            'WS8_LIST_MUTANT.load(std::sync::atomic::Ordering::SeqCst)==4 {1800} else {3600}')
    replace('rust/crates/campfire/src/controllers/rooms/files.rs',
            'filename: b.filename.to_string(),',
            f'filename: if {selector}==1 {{"mutated actual upload".into()}} '
            'else {b.filename.to_string()},')
    replace('rust/crates/campfire/src/controllers/rooms/files.rs',
            '.map(|a| {\n                    let b = blobs',
            f'.map(|a| {{\n                    if {selector}==3 {{'
            'Blob::find(conn,a.blob_id).map_err(storage_error)?;}\n                    let b = blobs')
    replace('rust/crates/campfire/src/controllers/autocompletable.rs',
            'Ok(c.render(StatusCode::OK, &format::JSON, body))',
            f'let body = if {selector}==2 {{body.replace("Scaling", "Mutated")}} else {{body}};\n'
            '        Ok(c.render(StatusCode::OK, &format::JSON, body))')
    list_path = 'rust/crates/campfire/src/controllers/message_features/list_scaling_tests.rs'
    replace(list_path,
            '#[tokio::test]\nasync fn files_and_mentions_match_fresh_rails_with_flat_visible_row_reads()',
            'async fn files_and_mentions_match_fresh_rails_with_flat_visible_row_reads()')
    witnesses = [(1, 'actual visible-list envelope differs from Rails: /rooms/918001/files'),
                 (2, 'actual visible-list envelope differs from Rails: /autocompletable/users'),
                 (3, 'visible-list physical reads grow per row: /rooms/918001/files'),
                 (4, 'actual represented extreme differs from Rails:')]
    with (ROOT / list_path).open('a') as out:
        for number, _ in witnesses:
            call = ('super::extreme_range_tests::represented_extreme_relative_and_calendar_values_match_fresh_rails();'
                    if number == 4 else
                    'files_and_mentions_match_fresh_rails_with_flat_visible_row_reads().await;')
            out.write(f'\n#[tokio::test] async fn ws8_list_producer_mutant_{number}() {{\n'
                      f'campfire_db::slash_commands::time_parser::WS8_LIST_MUTANT.store({number},'
                      f'std::sync::atomic::Ordering::SeqCst);\n{call}\n}}\n')
    replace('rust/crates/campfire/src/controllers/message_features/extreme_range_tests.rs',
            '#[test]\nfn represented_extreme_relative_and_calendar_values_match_fresh_rails()',
            'pub(super) fn represented_extreme_relative_and_calendar_values_match_fresh_rails()')
    for number, witness in witnesses:
        r = subprocess.run(runner + ['test', '--locked', '-p', 'campfire',
            f'ws8_list_producer_mutant_{number}', '--', '--test-threads=4', '--nocapture'],
            cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        print(r.stdout, flush=True)
        assert r.returncode != 0 and f'assertion `left == right` failed: {witness}' in r.stdout, witness
        print(f'WS8bm2 slice-I producer mutant {number}: rejected at {witness}', flush=True)
    print('WS8bm2 slice-I producer controls: 4/4 rejected at intended real-output/read assertions', flush=True)
finally:
    for path, raw in originals.items():
        path.write_bytes(raw)
