#!/usr/bin/env python3
"""Validate physical citations and complete, manually reviewed assertion maps.

Current maps declare helper_inventory_version: 1 and must match the independent
pinned-source helper inventory. Registered historical B/C snapshots retain their
declaration-only policy. This checks integrity, not semantic equivalence. Each
mapping is reviewed against the real setup/path and sampled with behavior
mutations. --render writes the tables; --pass-log checks named test execution.
"""
import argparse
import hashlib
import json
import pathlib
import re
import subprocess

from helper_inventory import VERSION, completeness_errors, from_pin

root = pathlib.Path(__file__).resolve().parents[3]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('maps', nargs='+', type=pathlib.Path)
parser.add_argument('--render', action='store_true')
parser.add_argument('--pass-log', type=pathlib.Path)
args = parser.parse_args()
ledger = {r['id']: r for r in json.loads((root / 'rust/plans/ledger-ws14-ws15.json').read_text())['records']}
reference = (root / 'rust/parity/reference.sha').read_text().strip()
maps = [(path, json.loads(path.read_text())) for path in args.maps]
# B/C are historical declaration-only maps. Preserve their explicit reopened
# and superseded records, but never let a new map opt out by changing its pin.
historical = {}
for name in ('b', 'c'):
    data = json.loads((root / f'rust/plans/ledger-ws14-ws15-{name}-assertions.json').read_text())
    for record in data['records']:
        historical[data['reference'], record['id']] = record
strict_maps = []
for path, data in maps:
    if data['reference'] == reference or 'helper_inventory_version' in data:
        if type(data.get('helper_inventory_version')) is not int or data['helper_inventory_version'] != VERSION or data['reference'] != reference:
            raise SystemExit(f'{path}: current assertion maps require helper_inventory_version: {VERSION} '
                             f'and pinned reference {reference}')
        strict_maps.append((path, data))
    else:
        if any(historical.get((data['reference'], r['id'])) != r for r in data['records']):
            raise SystemExit(f'{path}: unregistered historical declaration-only assertion map')
        print(f'{path}: historical declaration-only map; helper completeness is not claimed')
inventory = from_pin(root, reference, [ledger[r['id']] for _, data in strict_maps for r in data['records']]) if strict_maps else None
helper_errors = []
expected_helpers = mapped_helpers = 0
for path, data in strict_maps:
    for record in data['records']:
        original = ledger[record['id']]
        required = inventory.required(original['rails'], original['rails_line'])
        expected_helpers += len(required)
        mapped_helpers += len(record.get('helper_assertions', []))
        helper_errors.extend(f'{path}: {error}' for error in completeness_errors(record, required))
if helper_errors:
    raise SystemExit(f'Helper inventory v{VERSION}: {expected_helpers} required; {mapped_helpers} mapped\n' +
                     '\n'.join(helper_errors))
pass_log = args.pass_log.read_text() if args.pass_log else None
total_records = total_assertions = total_helper_assertions = missing = 0
original_browser = None

def validate_original_browser(rust, record, ruby):
    global original_browser
    if original_browser is None:
        original_browser = json.loads((root / 'rust/reference-tools/users/original_browser/manifest.json').read_text())
        assert original_browser['reference'] == (root / 'rust/parity/reference.sha').read_text().strip()
        for original, source in original_browser['files'].items():
            raw = (root / source['file']).read_bytes()
            assert hashlib.sha256(raw).hexdigest() == source['sha256'], original
            assert raw == subprocess.check_output(['git', 'show', original_browser['reference'] + ':' + original], cwd=root), original
    metadata = next(r for r in original_browser['records'] if r['id'] == record['id'])
    source = original_browser['files'][ruby['file']]
    assert rust['file'] == source['file'] and rust['original'] == {'file': ruby['file'], 'line': ruby['line']}
    assert rust['line'] == ruby['line'] and rust['text'] == ruby['text']
    if rust['kind'] == 'original_browser_helper':
        assert any(h['file'] == ruby['file'] and h['line'] == ruby['line'] and h['text'] == ruby['text']
                   for h in metadata['helper_assertions']), (record['id'], ruby)
    else:
        assert ruby['file'] == metadata['file'] and ruby['line'] in metadata['assertion_lines']
    function = 'original_browser_' + record['id'].lower().replace('-', '_')
    assert rust['function'] == function and record['test'].endswith('::' + function)
    wrapper_file = metadata.get('wrapper_file', 'rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs')
    wrapper_module = metadata.get('wrapper_module', 'controllers::ws14_original_browser_tests')
    assert record['test'] == 'campfire::bin/campfire ' + wrapper_module + '::' + function
    wrappers = (root / wrapper_file).read_text()
    assert re.search(r'async fn ' + function + r'\(\)\s*\{\s*(?:super::ws14_original_browser_tests::)?original\("' + record['id'] + r'"\)\.await;', wrappers)

def code(text):
    return '`' + ' '.join(text.split()).replace('|', '\\|').replace('`', '&#96;') + '`'

def cite(source):
    path, line = source['file'], source['line']
    return f'[{path}:{line}](../../{path}#L{line})'

for path, data in maps:
    records = data['records']
    assert len({r['id'] for r in records}) == len(records), path
    markdown = ['# Rails assertion to Rust assertion map', '',
                f"Reference: `{data['reference']}`. Each row names an original Rails assertion call, "
                'its discriminating Rust assertion and the real test that executes it. Shared helper '
                'assertions and repeated loop cases are cited explicitly. These tables retain '
                'compiler checks as compiler checks; they do not claim matching exception classes '
                'between Ruby and the Rust type system.', '']
    markdown += [f'Helper policy: independently derived pinned-source inventory v{VERSION}.'
                 if data.get('helper_inventory_version') == VERSION else
                 'Historical declaration-only policy; helper completeness is not claimed.', '']
    for record in records:
        rid = record['id']
        original = ledger[rid]
        strict = data.get('helper_inventory_version') == VERSION
        if strict:
            required = set(inventory.declarations[f"{original['rails']}:{original['rails_line']}"]['assertions'])
        else:
            lines = (root / original['rails']).read_text().splitlines()
            start = original['rails_line'] - 1
            end = next(i for i in range(start + 1, len(lines)) if lines[i] == '  end')
            required = {i + 1 for i in range(start, end)
                        if re.search(r'\b(?:assert|refute)(?:_\w+)?\b|\.expects\(', lines[i]) and not lines[i].lstrip().startswith('#')}
        actual = {a['rails']['line'] for a in record['assertions']}
        assert actual == required, (rid, 'omitted or extra assertion calls', required ^ actual)
        assert len(actual) == len(record['assertions']), rid
        test = record['test']
        if pass_log:
            name = test.split(' ', 1)[1]
            assert any('PASS' in line and line.endswith(' ' + name) for line in pass_log.splitlines()), (rid, test)
        markdown += [f'## {rid}', '', f"Rails declaration: `{original['rails']}:{original['rails_line']}` — {original['title']}", '',
                     f'Executed test: `{test}`.', '', record['note'], '',
                     '| Rails assertion | Discriminating Rust assertion |', '|---|---|']
        helper_assertions = record.get('helper_assertions', [])
        assert len({(a['rails']['file'], a['rails']['line']) for a in helper_assertions}) == len(helper_assertions), rid
        for entry in record['assertions'] + helper_assertions:
            ruby = entry['rails']
            ruby_lines = (root / ruby['file']).read_text().splitlines()
            if entry not in helper_assertions:
                assert ruby['file'] == original['rails'], rid
            assert ruby_lines[ruby['line'] - 1].strip() == ruby['text'], (rid, ruby)
            if strict:
                assert inventory.sources[ruby['file']][ruby['line'] - 1].strip() == ruby['text'], (rid, 'citation differs from pinned Ruby', ruby)
            if entry in helper_assertions:
                # Strict entries have already been checked against parsed
                # assertion callsites, including assertions inside helpers.
                assert strict or re.match(r'(?:assert|refute)(?:_\w+)?\b', ruby['text']), (rid, 'helper citation must name an assertion call', ruby)
                assert entry['disposition'] == 'covered', (rid, ruby)
            native = entry['rust']
            assert entry['disposition'] in {'covered', 'unsupported_assertion'}, (rid, ruby)
            if entry['disposition'] == 'covered':
                assert native, (rid, ruby)
            else:
                assert original['disposition'] == 'unsupported_assertion' or record.get('superseded_by'), rid
                assert not native and entry.get('reason'), (rid, ruby)
                missing += 1
            citations = []
            if not native:
                assert entry.get('reason'), (rid, ruby)
                citations.append('**Open:** ' + entry['reason'])
            for rust in native:
                source = (root / rust['file']).read_text().splitlines()
                assert 1 <= rust['line'] <= rust['end_line'] <= len(source), (rid, 'invalid physical Rust citation range', rust)
                quoted = '\n'.join(source[rust['line'] - 1:rust['end_line']])
                assert rust['text'] in quoted, (rid, rust)
                if rust.get('kind') in {'original_browser', 'original_browser_helper'}:
                    validate_original_browser(rust, record, ruby)
                elif rust.get('kind') == 'compile_fail':
                    assert '```compile_fail' in rust['text'], rid
                else:
                    assert re.match(r'assert(?:_eq|_ne)?!\(', rust['text']), (rid, rust)
                    assert re.search(r'\bfn\s+' + re.escape(rust['function']) + r'\(', '\n'.join(source)), rid
                citations.append(cite(rust) + '<br>' + code(rust['text']))
            markdown.append('| ' + cite(ruby) + '<br>' + code(ruby['text']) + ' | ' + '<br><br>'.join(citations) + ' |')
        markdown.append('')
        total_records += 1
        total_assertions += len(record['assertions'])
        total_helper_assertions += len(helper_assertions)
    if args.render:
        path.with_suffix('.md').write_text('\n'.join(markdown).rstrip() + '\n')
print(f'Assertion maps: {total_records} records; {total_assertions} original Rails declaration assertion calls; '
      f'{total_helper_assertions} explicit nested helper assertion calls; {missing} unmapped')
