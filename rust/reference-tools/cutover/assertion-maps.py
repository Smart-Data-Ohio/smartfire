#!/usr/bin/env python3
"""Validate physical citations and complete, manually reviewed assertion maps.

This checks integrity, not semantic equivalence. Each mapping is reviewed against
the real setup/path and sampled with behavior mutations. --render writes the
human-readable tables; --pass-log verifies that every named native test ran.
"""
import argparse
import hashlib
import json
import pathlib
import re
import subprocess

root = pathlib.Path(__file__).resolve().parents[3]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('maps', nargs='+', type=pathlib.Path)
parser.add_argument('--render', action='store_true')
parser.add_argument('--pass-log', type=pathlib.Path)
args = parser.parse_args()
ledger = {r['id']: r for r in json.loads((root / 'rust/plans/ledger-ws14-ws15.json').read_text())['records']}
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

for path in args.maps:
    data = json.loads(path.read_text())
    records = data['records']
    assert len({r['id'] for r in records}) == len(records), path
    markdown = ['# Rails assertion to Rust assertion map', '',
                f"Reference: `{data['reference']}`. Each row names an original Rails assertion call, "
                'its discriminating Rust assertion and the real test that executes it. Shared helper '
                'assertions and repeated loop cases are cited explicitly. These tables retain '
                'compiler checks as compiler checks; they do not claim matching exception classes '
                'between Ruby and the Rust type system.', '']
    for record in records:
        rid = record['id']
        original = ledger[rid]
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
            if entry in helper_assertions:
                assert re.match(r'(?:assert|refute)(?:_\w+)?\b', ruby['text']), (rid, 'helper citation must name an assertion call', ruby)
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
