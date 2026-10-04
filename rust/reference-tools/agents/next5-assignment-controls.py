#!/usr/bin/env python3
"""Mutate assignment producers in a private clone; never change a vector or assertion.

Install once and build the DB test binary, then run the 16 selectors and restore.
Receipts require a producer activation and the exact persisted-fact assertion.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
catalog = json.loads(Path(__file__).with_suffix('.json').read_text())
spec = importlib.util.spec_from_file_location('owner_controls', ROOT/'rust/reference-tools/users/ws12_assertion_mutations.py')
owner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(owner)
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('action', choices=['install', 'run', 'restore'])
p.add_argument('--scratch', type=Path, required=True)
p.add_argument('--binary', type=Path)
a = p.parse_args()
a.scratch = a.scratch.resolve()
if a.action == 'install':
    owner.install(catalog, a.scratch)
elif a.action == 'restore':
    owner.restore(a.scratch)
else:
    assert a.binary and a.binary.is_file(), 'supply the compiled DB test binary'
    binary = a.binary.resolve()
    names = [s[:-6] for s in subprocess.check_output([binary, '--list'], text=True).splitlines() if s.endswith(': test')]
    logs = a.scratch/'logs'
    logs.mkdir(parents=True, exist_ok=True)
    receipts = []
    for case in catalog['declarations']:
        selected = [n for n in names if n.rsplit('::',1)[-1] == case['rust_test']]
        assert len(selected) == 1, case['rust_test']
        env = dict(os.environ, CI='1', WS12_ASSERTION_MUTATION=case['mutation'])
        run = subprocess.run([str(binary), selected[0], '--exact', '--test-threads=8', '--nocapture'],
                             cwd=ROOT/'rust/crates/db', env=env, text=True, capture_output=True)
        output = run.stdout+run.stderr
        path = logs/(case['key']+'.log')
        path.write_text(output)
        summary = re.findall(r'^test result:.*$', output, re.M)
        hits = output.count('WS12_COVERAGE_HIT '+case['mutation'])
        intended = 'assertion `left == right` failed: '+case['intended_assertion']
        receipt = dict(case, exit=run.returncode, hits=hits,
                       intended_assertion_rejected=intended in output, summaries=summary,
                       log=path.relative_to(a.scratch).as_posix())
        receipts.append(receipt)
        print(f"WS11_ASSIGNMENT_CONTROL {case['key']} hits={hits} intended={intended in output} exit={run.returncode}", flush=True)
        assert run.returncode and hits and intended in output and len(summary)==1, receipt
        assert '0 passed; 1 failed; 0 ignored' in summary[0], receipt
    (a.scratch/'receipts.json').write_text(json.dumps(receipts, indent=2)+'\n')
    print(f'WS11 assignment producer controls: {len(receipts)} activated; {len(receipts)} rejected at intended assertions; 0 unsupported credits')
