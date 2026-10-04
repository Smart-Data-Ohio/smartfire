#!/usr/bin/env python3
"""Validate physical citations and complete, manually reviewed assertion maps.

This checks integrity, not semantic equivalence. Each mapping is reviewed against
the real setup/path and sampled with behavior mutations. --render writes the
human-readable tables; --pass-log verifies that every named native test ran.
"""
import argparse
import json
import pathlib
import re

root = pathlib.Path(__file__).resolve().parents[3]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('maps', nargs='+', type=pathlib.Path)
parser.add_argument('--render', action='store_true')
parser.add_argument('--pass-log', type=pathlib.Path)
args = parser.parse_args()
ledger = {r['id']: r for r in json.loads((root / 'rust/plans/ledger-ws14-ws15.json').read_text())['records']}
pass_log = args.pass_log.read_text() if args.pass_log else None
total_records = total_assertions = missing = 0

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
                    if re.search(r'\bassert(?:_\w+)?\b|\.expects\(', lines[i]) and not lines[i].lstrip().startswith('#')}
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
        for entry in record['assertions']:
            ruby = entry['rails']
            assert ruby['file'] == original['rails'], rid
            assert lines[ruby['line'] - 1].strip() == ruby['text'], (rid, ruby)
            native = entry['rust']
            if entry['disposition'] == 'covered':
                assert native, (rid, ruby)
            else:
                assert original['disposition'] == 'unsupported_assertion', rid
                missing += 1
            citations = []
            if not native:
                assert entry.get('reason'), (rid, ruby)
                citations.append('**Open:** ' + entry['reason'])
            for rust in native:
                source = (root / rust['file']).read_text().splitlines()
                quoted = '\n'.join(source[rust['line'] - 1:rust['end_line']])
                assert rust['text'] in quoted, (rid, rust)
                if rust.get('kind') == 'compile_fail':
                    assert '```compile_fail' in rust['text'], rid
                else:
                    assert re.match(r'assert(?:_eq|_ne)?!\(', rust['text']), (rid, rust)
                    assert re.search(r'\bfn\s+' + re.escape(rust['function']) + r'\(', '\n'.join(source)), rid
                citations.append(cite(rust) + '<br>' + code(rust['text']))
            markdown.append('| ' + cite(ruby) + '<br>' + code(ruby['text']) + ' | ' + '<br><br>'.join(citations) + ' |')
        markdown.append('')
        total_records += 1
        total_assertions += len(record['assertions'])
    if args.render:
        path.with_suffix('.md').write_text('\n'.join(markdown).rstrip() + '\n')
print(f'Assertion maps: {total_records} records; {total_assertions} original Rails assertion calls; {missing} unmapped')
