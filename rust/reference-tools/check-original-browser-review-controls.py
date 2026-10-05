#!/usr/bin/env python3
"""Reject the review's real producer defects at their intended original predicates.

Startup, transport and mutation-setup failures are invalid controls. Each runner
uses its normal database, router, served assets and isolated browser network.
"""
import concurrent.futures
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parents[1]
out = root / '.scratch/original-browser-review-controls'
out.mkdir(parents=True, exist_ok=True)
controls = [
    ('tours', 'tour-stamp', 'first_run_tour_test.rb:95: original assertion'),
    ('tours', 'tour-auto-start', '#tour[data-tour-auto-start-value'),
    ('group', 'group-notice', 'Group renamed.'),
    ('group', 'group-navigation', 'page.waitForURL'),
    ('members', 'member-visibility', 'page.waitForFunction'),
    ('stars', 'menu-rendered', '#member-row-menu:visible'),
    ('stars', 'identity-visibility', '.member-panel__identity:visible'),
    ('pickers', 'picker-visibility', 'people_group_dms_test.rb:192: original assertion'),
]


def reject(control):
    mode, name, marker = control
    result = subprocess.run(
        ['python3', str(root / 'reference-tools/users/run_original_browser_assertions.py'),
         mode, '--mutation', name],
        text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
    )
    (out / f'{name}.log').write_text(result.stdout)
    assert result.returncode != 0, f'{name}: surviving producer defect'
    assert 'ORIGINAL_MUTATION' in result.stdout, f'{name}: producer not reached'
    assert 'INVALID_CONTROL' not in result.stdout, f'{name}: invalid control'
    assert 'WS8br2 browser failure diagnostics:' in result.stdout, f'{name}: startup/runner failure'
    assert marker in result.stdout, f'{name}: wrong predicate rejected the defect'
    print(f'Original review control {name}: intended defect rejected', flush=True)


with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
    list(pool.map(reject, controls))
print('Original browser review discrimination: 8 producer defects rejected; 0 invalid controls')
