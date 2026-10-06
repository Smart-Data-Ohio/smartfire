#!/usr/bin/env python3
"""Discriminate three easily confused renderers using isolated compiled mutations.

These are plain output partials: an appended comment must affect any byte receipt
that actually selects them. This is deliberately not a general probe for template
inheritance, macro declarations, serializers or non-output source files.
"""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
PROBES = {
    'messages/_footer_composer.html': 'room_native',
    'messages/_template.html': 'room_native',
    'users/sidebars/rooms/_empty_venue_children.html': 'sidebar_calls',
}


def run_receipts(checkout, receipts, env, log):
    packages = sorted({Path(r['test_file']).parts[1] for r in receipts})
    package_names = {'views': 'campfire_views', 'campfire': 'campfire'}
    expression = ' | '.join(f'test({r["test"]})' for r in receipts)
    command = ['cargo', 'nextest', 'run', '--locked', '-j', '4',
               '--no-fail-fast', '--color', 'never', '-E', expression]
    for package in packages:
        command += ['-p', package_names[package]]
    print(' '.join(command), flush=True)
    with log.open('w') as output:
        result = subprocess.run(command, cwd=checkout, env=env,
                                stdout=output, stderr=subprocess.STDOUT)
    text = log.read_text()
    summary = next((line.strip() for line in text.splitlines()
                    if re.search(r'\bSummary \[', line)), '')
    print(summary or text[-4000:], flush=True)
    return result.returncode, summary


def main():
    coverage = json.loads((ROOT / 'parity/template-coverage.json').read_text())
    receipts = {name: [coverage['evidence'][key]
                       for key in coverage['templates'][name]['evidence']]
                for name in PROBES}
    target = Path(os.environ.get('CARGO_TARGET_DIR', ROOT / 'target')).resolve()
    logs = target / 'template-rendering-evidence'
    logs.mkdir(parents=True, exist_ok=True)
    # Use current working files, including uncommitted fixes. No source file in
    # the real worktree is ever mutated, and no git stash or checkout is needed.
    tracked = subprocess.check_output(
        ['git', 'ls-files', '-z'], cwd=ROOT).decode().split('\0')
    with tempfile.TemporaryDirectory(prefix='template-rendering-', dir=target) as tmp:
        checkout = Path(tmp)
        for relative in filter(None, tracked):
            source = ROOT / relative
            destination = checkout / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, destination)
        (checkout / 'parity/.seed').symlink_to(ROOT / 'parity/.seed',
                                                  target_is_directory=True)
        env = dict(os.environ, CI='1', CARGO_TARGET_DIR=str(target))
        (checkout / 'tmp').mkdir()
        env['TMPDIR'] = str(checkout / 'tmp')
        controls = {r['test']: r for group in receipts.values() for r in group}
        controls.update({coverage['evidence'][key]['test']: coverage['evidence'][key]
                         for key in PROBES.values()})
        code, summary = run_receipts(checkout, list(controls.values()), env,
                                     logs / 'control.log')
        if code or not re.search(fr'{len(controls)} tests run: {len(controls)} passed', summary):
            raise SystemExit(f'Clean receipt controls did not pass; see {logs}/control.log')
        for index, name in enumerate(PROBES):
            template = checkout / 'crates/views/templates' / name
            original = template.read_bytes()
            template.write_bytes(original + f'\n<!-- TEMPLATE-RENDER-PROBE-{index} -->\n'.encode())
            try:
                for receipt in receipts[name]:
                    log = logs / f'probe-{index}-{receipt["test"]}.log'
                    code, summary = run_receipts(checkout, [receipt], env, log)
                    # A compile error or empty selection is not evidence. The
                    # unchanged test must pass, then the same test must fail.
                    if not code or not re.search(r'1 test run: 0 passed, 1 failed', summary):
                        raise SystemExit(f'{name}: cited test did not detect its compiled mutation; see {log}')
                # The originally misattributed renderer must remain insensitive
                # to this file, proving we discriminate the bypassing receipt.
                bypass = coverage['evidence'][PROBES[name]]
                log = logs / f'bypass-{index}.log'
                code, summary = run_receipts(checkout, [bypass], env, log)
                if code or not re.search(r'1 test run: 1 passed', summary):
                    raise SystemExit(f'{name}: bypass control no longer discriminates this renderer; see {log}')
            finally:
                template.write_bytes(original)
    print(f'Render selection: {len(controls)} clean controls passed; '
          f'{len(PROBES)} compiled template mutations detected', flush=True)


if __name__ == '__main__':
    main()
