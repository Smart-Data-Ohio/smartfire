"""Derive declaration helper assertions from the pinned Ruby syntax and dispatch.

Version 1 follows statically named test methods, included modules, inherited
setup/teardown callbacks, and recursive helper calls. Assertions are physical
callsites, counted once per declaration even when a helper runs in a loop.
No assertion map or copied-browser manifest participates in the inventory.
"""
import hashlib
import io
import json
import pathlib
import subprocess
import tarfile

VERSION = 1
# These Rails test bases inherit ActiveSupport::TestCase, which the pinned
# test/test_helper.rb opens to include the project's shared test helpers.
RAILS_TEST_BASES = {
    'ActionDispatch::IntegrationTest', 'ActionDispatch::SystemTestCase',
    'ActionView::TestCase', 'ActiveJob::TestCase',
}


def pinned_sources(root, reference, declaration_files):
    names = subprocess.check_output(
        ['git', 'ls-tree', '-r', '--name-only', reference, '--', 'test', 'app/helpers'],
        cwd=root, text=True).splitlines()
    selected = set(declaration_files) | {'test/test_helper.rb', 'test/application_system_test_case.rb'}
    selected.update(name for name in names if name.endswith('.rb') and name.startswith(
        ('test/test_helpers/', 'test/support/', 'app/helpers/')))
    absent = selected - set(names)
    if absent:
        raise ValueError('Pinned Ruby sources absent: ' + ', '.join(sorted(absent)))
    archive = subprocess.check_output(['git', 'archive', reference, '--', *sorted(selected)], cwd=root)
    with tarfile.open(fileobj=io.BytesIO(archive)) as files:
        return {member.name: files.extractfile(member).read().decode()
                for member in files if member.isfile()}


class HelperInventory:
    def __init__(self, sources):
        self.sources = {name: source.splitlines() for name, source in sources.items()}
        self.source_sha256 = {name: hashlib.sha256(source.encode()).hexdigest() for name, source in sources.items()}
        parser = pathlib.Path(__file__).with_name('helper-inventory.rb')
        parsed = subprocess.run(['ruby', str(parser)], input=json.dumps(sources), text=True,
                                capture_output=True)
        if parsed.returncode:
            raise ValueError(parsed.stderr.strip())
        data = json.loads(parsed.stdout)
        self.scopes = data['scopes']
        self.declarations = data['declarations']

    def qualify(self, name, owner):
        if name in self.scopes or name in RAILS_TEST_BASES:
            return name
        namespace = owner.split('::')[:-1]
        while namespace:
            candidate = '::'.join([*namespace, name])
            if candidate in self.scopes:
                return candidate
            namespace.pop()
        return name

    def ancestors(self, owner, seen=None):
        seen = set() if seen is None else seen
        if owner in seen:
            return []
        seen.add(owner)
        if owner in RAILS_TEST_BASES:
            return self.ancestors('ActiveSupport::TestCase', seen)
        scope = self.scopes.get(owner)
        if not scope:
            return []
        result = [owner]
        for include in reversed(scope['includes']):
            result.extend(self.ancestors(self.qualify(include, owner), seen))
        if scope.get('parent'):
            result.extend(self.ancestors(self.qualify(scope['parent'], owner), seen))
        return result

    def required(self, file, line):
        declaration = self.declarations.get(f'{file}:{line}')
        if declaration is None:
            raise ValueError(f'Pinned Ruby test declaration absent: {file}:{line}')
        ancestors = self.ancestors(declaration['owner'])
        methods = {}
        callbacks = []
        for owner in ancestors:
            scope = self.scopes[owner]
            for name, method in scope['methods'].items():
                methods.setdefault(name, method)
            callbacks.extend(scope['callbacks'])
        roots = [declaration, *callbacks]
        roots.extend(methods[name] for name in ('setup', 'teardown') if name in methods)
        seen, required = set(), {}

        def retain_assertions(body, helper):
            helper_file = body['file']
            for assertion_line in body['assertions']:
                required[helper_file, assertion_line] = {
                    'file': helper_file, 'line': assertion_line,
                    'text': self.sources[helper_file][assertion_line - 1].strip(),
                    'helper': helper,
                }

        def follow(body):
            for call in body['calls']:
                name = call['name']
                if call['receiver'] != 'self' or name not in methods or name in seen:
                    continue
                seen.add(name)
                method = methods[name]
                retain_assertions(method, name)
                follow(method)

        # Callback bodies execute for every declaration, including inherited
        # callbacks whose assertions do not pass through a named helper.
        for callback in callbacks:
            retain_assertions(callback, callback['helper'])
        for name in ('setup', 'teardown'):
            if name in methods:
                retain_assertions(methods[name], name)
        for body in roots:
            follow(body)
        return [required[key] for key in sorted(required)]


def from_pin(root, reference, records):
    return HelperInventory(pinned_sources(root, reference, {r['rails'] for r in records}))


def completeness_errors(record, required):
    expected = {(a['file'], a['line']) for a in required}
    actual = {(a['rails']['file'], a['rails']['line']) for a in record.get('helper_assertions', [])}
    errors = []
    for kind, entries in [('missing', expected - actual), ('unexpected', actual - expected)]:
        if entries:
            errors.append(f"{record['id']}: {kind} helper assertions: " +
                          ', '.join(f'{file}:{line}' for file, line in sorted(entries)))
    return errors


if __name__ == '__main__':
    import argparse

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('maps', nargs='+', type=pathlib.Path)
    parser.add_argument('--output', type=pathlib.Path, help='write the independently derived inventory and gaps as JSON')
    args = parser.parse_args()
    root = pathlib.Path(__file__).resolve().parents[3]
    reference = (root / 'rust/parity/reference.sha').read_text().strip()
    ledger = {r['id']: r for r in json.loads((root / 'rust/plans/ledger-ws14-ws15.json').read_text())['records']}
    maps = [(path, json.loads(path.read_text())) for path in args.maps]
    inventory = from_pin(root, reference, [ledger[r['id']] for _, data in maps for r in data['records']])
    report = {'reference': reference, 'helper_inventory_version': VERSION, 'maps': [],
              'sources': inventory.source_sha256}
    gaps = required_total = mapped_total = 0
    for path, data in maps:
        mapped = {'file': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'records': []}
        for record in data['records']:
            original = ledger[record['id']]
            required = inventory.required(original['rails'], original['rails_line'])
            actual = {(a['rails']['file'], a['rails']['line']) for a in record.get('helper_assertions', [])}
            missing = [a for a in required if (a['file'], a['line']) not in actual]
            errors = completeness_errors(record, required)
            mapped['records'].append({'id': record['id'], 'required': required, 'missing': missing, 'errors': errors})
            required_total += len(required)
            mapped_total += len(record.get('helper_assertions', []))
            gaps += len(missing)
        report['maps'].append(mapped)
    report['summary'] = {'records': sum(len(m['records']) for m in report['maps']),
                         'required': required_total, 'mapped': mapped_total, 'missing': gaps,
                         'records_with_gaps': sum(bool(r['errors']) for m in report['maps'] for r in m['records'])}
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report['summary']))
    for mapped in report['maps']:
        for record in mapped['records']:
            for error in record['errors']:
                print(error)
    raise SystemExit(1 if any(r['errors'] for m in report['maps'] for r in m['records']) else 0)
