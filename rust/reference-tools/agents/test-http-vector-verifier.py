#!/usr/bin/env python3
"""Inject status, body, header and dispatch differences into the actual verifier."""
from copy import deepcopy
import json
import hashlib
from pathlib import Path
import runpy
import unittest

api = runpy.run_path(str(Path(__file__).with_name('verify-http-vectors.py')))
source_api = runpy.run_path(str(Path(__file__).with_name('check-http-reference.py')))


class VerifierInjections(unittest.TestCase):
    def setUp(self):
        root = Path(__file__).resolve().parents[2]
        self.vector = json.loads((root / 'vectors/agent_http.json').read_bytes())
        self.expected = json.dumps(self.vector).encode()

    def reject(self, field, value):
        changed = deepcopy(self.vector)
        changed['cases'][0][field] = value
        with self.assertRaises(AssertionError):
            api['compare_vectors'](json.dumps(changed).encode(), self.expected, 'injected')

    def test_status_difference(self):
        self.reject('status', 599)

    def test_body_difference(self):
        self.reject('response', {'injected': True})

    def test_header_difference(self):
        self.reject('response_headers', {'Retry-After': '999999'})

    def test_byte_difference(self):
        api['compare_vectors'](self.expected, self.expected, 'equal')
        with self.assertRaises(AssertionError):
            api['compare_vectors'](self.expected + b'\n', self.expected, 'injected')

    def test_missing_alternative_is_rejected(self):
        names = {'register_slash_command', 'unregister_slash_command'}
        api['check_dispatch'](names, '"register_slash_command" | "unregister_slash_command" => {}')
        with self.assertRaises(AssertionError):
            api['check_dispatch'](names, '"unregister_slash_command" => {}')

    def test_image_source_drift(self):
        digest = hashlib.sha256(self.expected).hexdigest()
        source_api['check_source'](self.expected, digest, self.expected, 'equal')
        changed = hashlib.sha256(self.expected + b'\n').hexdigest()
        with self.assertRaises(AssertionError):
            source_api['check_source'](self.expected, changed, self.expected, 'image drift')

    def test_checkout_source_drift(self):
        digest = hashlib.sha256(self.expected).hexdigest()
        with self.assertRaises(AssertionError):
            source_api['check_source'](self.expected, digest, self.expected + b'\n', 'checkout drift')


if __name__ == '__main__':
    unittest.main()
