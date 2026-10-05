"""Deterministic helper assertion inventory from the pinned Ruby source."""
import pathlib
import re

ASSERTION = re.compile(r'\b(?:assert|refute)(?:_\w+)?\b|\.expects\(')


def helper_assertions(directory: pathlib.Path, manifest, record):
    selected = record['file']
    files = [name for name in manifest['files']
             if '/test_helpers/' in name or name == 'test/support/drive_share_mocks.rb']
    files.append(selected)
    methods = {}
    sources = {}
    for name in files:
        lines = (directory / name).read_text().splitlines()
        sources[name] = lines
        for i, line in enumerate(lines):
            match = re.match(r'^(\s*)def\s+(\w+[!?]?)', line)
            if not match:
                continue
            indent, method = match.groups()
            end = next(j for j in range(i + 1, len(lines))
                       if lines[j] == indent + 'end')
            methods[method] = (name, i + 1, end)
    lines = sources[selected]
    start = record['line'] - 1
    end = next(i for i in range(start + 1, len(lines)) if lines[i] == '  end')
    roots = lines[start + 1:end]
    for i, line in enumerate(lines):
        if line == '  setup do':
            end = next(j for j in range(i + 1, len(lines)) if lines[j] == '  end')
            roots.extend(lines[i + 1:end])
    seen, assertions = set(), {}

    def follow(body):
        text = '\n'.join(line for line in body if not line.lstrip().startswith('#'))
        for method, (name, start, end) in methods.items():
            if method in seen or not re.search(r'(?<![\w:])' + re.escape(method) + r'(?!\w)', text):
                continue
            seen.add(method)
            body = sources[name][start:end]
            for i in range(start, end):
                line = sources[name][i]
                if not line.lstrip().startswith('#') and ASSERTION.search(line):
                    assertions[name, i + 1] = {
                        'file': name, 'line': i + 1,
                        'text': line.strip(), 'helper': method,
                    }
            follow(body)

    follow(roots)
    return [assertions[key] for key in sorted(assertions)]
