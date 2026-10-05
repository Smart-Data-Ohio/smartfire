import importlib.util
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('system_inventory_hashes', Path(__file__).with_name('system-inventory-hashes.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class SystemInventoryHashesTest(unittest.TestCase):
    def test_rehash_keeps_every_closure_and_deferral_field(self):
        import copy
        import hashlib
        source = b'  test "original declaration" do\n  end\n'
        cases = [{'name': 'original declaration', 'status': 'deferred', 'remaining_reason': 'exact reason'}]
        original = {'reference': 'old', 'files': [{'file': 'test.rb', 'source_sha256': 'old', 'cases': cases}]}
        actual = module.refresh(copy.deepcopy(original), 'new', lambda file: source)
        self.assertEqual(actual['files'][0]['cases'], cases)
        self.assertEqual(actual['files'][0]['source_sha256'], hashlib.sha256(source).hexdigest())
        self.assertEqual(actual['reference'], 'new')

    def test_generator_refuses_unmapped_pin_declaration(self):
        with self.assertRaises(AssertionError):
            module.refresh({'files': [{'file': 'test.rb', 'cases': []}]}, 'new', lambda _: b'test "unmapped" do\n')
