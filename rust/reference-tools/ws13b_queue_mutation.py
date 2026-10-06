#!/usr/bin/env python3
"""Prove an actual duplicate producer enqueue fails the unchanged Rails gate.

Run in a disposable checkout, with no concurrent source edits:
  python3 rust/reference-tools/ws13b_queue_mutation.py SCRATCH_DIRECTORY
Restores the producer byte-for-byte even if a command or assertion fails.
"""
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
directory = Path(sys.argv[1]).resolve()
directory.mkdir(parents=True, exist_ok=True)
corpus = json.loads((ROOT/'rust/crates/db/src/models/huddle_observed_matrix.json').read_text())
case = next(c for c in corpus['cases'] if c['spec']['name'] == 'delayed_initial_0/item/oldest')
corpus['cases'] = [case]
oracle = directory/'oracle.json'
oracle.write_text(json.dumps(corpus)+'\n')
env = dict(os.environ, CARGO_BUILD_JOBS='2', CI='1',
           CABLE_TEST_PORT_RANGE='53000-53049', MAIL_TEST_PORT_RANGE='53050-53099',
           WS13B_OBSERVED_ORACLE=str(oracle), WS13B_DIFFERENTIAL_OUTPUT=str(directory))
command = ['cargo','test','--locked',
           '--manifest-path','rust/Cargo.toml','--workspace','--exclude','html5ever',
           'observed_rails_differential_','--','--test-threads=8','--nocapture']

def run(label):
    with (directory/(label+'.log')).open('w') as log:
        result = subprocess.run(command, cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT)
    lines = (directory/(label+'.log')).read_text().splitlines()
    for line in lines:
        if line.startswith('test result:') and ('8 passed;' in line or 'FAILED.' in line):
            print(label+': '+line, flush=True)
    return result.returncode

producer = ROOT/'rust/crates/db/src/models/huddle_invitations.rs'
original = producer.read_bytes()
enqueue = b'''    tx.emit_after_commit(Event::job(&PushInvitationJob {
        activity_item_id: item.id,
    }));'''
assert original.count(enqueue) == 1, 'producer enqueue changed; inspect before mutating'
assert run('baseline') == 0, 'unchanged Rails expectation must first pass'
try:
    producer.write_bytes(original.replace(enqueue, enqueue+b'\n'+enqueue))
    assert run('duplicate') != 0, 'duplicate producer enqueue escaped the gate'
    failure = json.loads((directory/'mismatches-0.json').read_text())[0]
    error = failure['error']
    assert 'queue differs after step 0 (issue)' in error, error
    left, right = error.split('  left: ')[1].split('\n right: ')
    assert left.count('Huddle::PushInvitationJob') == 2, error
    assert right.count('Huddle::PushInvitationJob') == 1, error
    (directory/'duplicate-evidence.json').write_text(json.dumps(failure, indent=2)+'\n')
    print('Injected duplicate rejected after issue: 2 queued PushInvitationJob entries versus Rails 1', flush=True)
finally:
    producer.write_bytes(original)
assert run('restored') == 0, 'restored producer must pass again'
assert producer.read_bytes() == original
