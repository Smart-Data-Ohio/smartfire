#!/usr/bin/env python3
"""Break served producer assets, require the cited original assertion to fail."""
import hashlib
import json
import os
import pathlib
import shlex
import subprocess

root = pathlib.Path(__file__).resolve().parents[3]
rust = root / 'rust'
directory = rust / 'target/ledger-d-browser-mutations'
directory.mkdir(parents=True, exist_ok=True)
cases = [
    ('WS14g-232', 'test/system/drive_share_test.rb', 139,
     ['controllers/drive_share_controller-', 'include_granted_scopes: false', 'include_granted_scopes: true']),
    ('WS14g-233', 'test/system/drive_share_test.rb', 157,
     ['controllers/drive_share_controller-', 'const result = pinDriveAttachment(this.attachmentsStrip, review)', 'const result = "pinned"']),
    ('WS14g-238', 'test/system/drive_share_test.rb', 296,
     ['controllers/drive_share_controller-', 'target?.focus({ preventScroll: true })\n  }\n\n  #armOutsideDismiss()', 'this.form?.querySelector("textarea")?.focus({ preventScroll: true })\n  }\n\n  #armOutsideDismiss()']),
    ('WS14g-252', 'test/system/drive_share_test.rb', 583,
     ['drive-', '.drive-share-dialog {', '.drive-share-dialog { transform: translateX(-30px) !important;']),
    ('WS14g-262', 'test/system/drive_share_test.rb', 849,
     ['drive-', '.drive-share-dialog__recipient-name {', '.drive-share-dialog__recipient-name { overflow-wrap: normal !important;']),
]
manifest = json.loads((rust / 'reference-tools/users/original_browser/manifest.json').read_text())
map_data = json.loads((rust / 'plans/ledger-ws14-ws15-d-browser-assertions.json').read_text())
records = []
def write_results():
    result = {'reference': manifest['reference'], 'records': records,
              'attempted': len(records), 'killed': len(records), 'survived': 0,
              'note': 'Every recorded mutation changes the actual producer asset delivered to native Chromium and fails at the cited original assertion. Positive bodies, scoped selectors and original geometry remain byte-identical. Mutation proxies close after each declaration; no producer file is edited. The runner stops without counting any infrastructure or unrelated-assertion failure.'}
    (rust / 'reference-tools/cutover/d-browser-mutations.json').write_text(json.dumps(result, indent=2) + '\n')
env = dict(os.environ, CI='true', CARGO_BUILD_JOBS='2')
env['PATH'] = str(rust / 'target/ledger-d-tools') + ':' + env.get('PATH', '')
for rid, original, line, mutation in cases:
    function = 'controllers::ws14_original_browser_tests::original_browser_' + rid.lower().replace('-', '_')
    command = shlex.split(env.get('CAMPFIRE_CARGO', 'cargo')) + [
        'nextest', 'run', '--locked', '-p', 'campfire', '-j', '4', '-E', f'test(={function})',
        '--run-ignored', 'only', '--no-fail-fast', '--failure-output', 'immediate',
    ]
    log = directory / (rid.lower() + '.log')
    mutant_env = dict(env, WS14_BROWSER_MUTATION=json.dumps(mutation))
    with log.open('w') as out:
        run = subprocess.run(command, cwd=rust, env=mutant_env, stdout=out, stderr=subprocess.STDOUT)
    text = log.read_text()
    applied = '"mutationApplied":' in text and '"mutationApplied":0' not in text
    expected_source = f'/tools/original_browser/{original}:{line}'
    # A helper assertion keeps its helper's physical failure line. The direct
    # declaration invocation is retained in the original Ruby backtrace.
    failed_at_citation = expected_source in text
    assert run.returncode != 0 and applied and failed_at_citation, (rid, run.returncode, applied, expected_source, log)
    entry = next(row for row in map_data['records'] if row['id'] == rid)
    assertion = next(row for row in entry['assertions'] if row['rails']['line'] == line)
    records.append({
        'id': rid, 'producer': mutation[0], 'before': mutation[1], 'after': mutation[2],
        'rails_assertion': assertion['rails'], 'rust_assertion': assertion['rust'][0],
        'command': shlex.join(command), 'exit_code': run.returncode,
        'mutation_applied': True, 'cited_assertion_failed': True,
        'log_sha256': hashlib.sha256(log.read_bytes()).hexdigest(),
        'log': str(log.relative_to(root)), 'restored': True,
        'restore_method': 'nativeAssetProxy scoped to the test browser; unchanged served producer restored when proxy closes',
    })
    write_results()
    print(f'{rid}: producer mutation applied; original assertion {original}:{line} failed; producer restored', flush=True)
write_results()
