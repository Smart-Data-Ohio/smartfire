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

    def test_representation_response_differences_are_rejected(self):
        root = Path(__file__).resolve().parents[2]
        vector = json.loads((root / 'vectors/agent_review192r5_representations.json').read_bytes())
        expected = json.dumps(vector).encode()
        for response, field, value in [('redirect', 'status', 500), ('proxy', 'body_base64', 'eA=='), ('disk', 'headers', {})]:
            changed = deepcopy(vector)
            changed['cases'][0][response][field] = value
            with self.assertRaises(AssertionError):
                api['compare_vectors'](json.dumps(changed).encode(), expected, 'missing representation drift')

    def test_proxy_transfer_encoding_drift_is_rejected(self):
        root = Path(__file__).resolve().parents[2]
        vector = json.loads((root / 'vectors/agent_review192r5_representations.json').read_bytes())
        expected = json.dumps(vector).encode()
        for case_index in range(len(vector['cases'])):
            for response in ['baseline_proxy', 'proxy']:
                headers = vector['cases'][case_index][response]['headers']
                self.assertNotIn('content-transfer-encoding', headers)
                for name, value in [('content-transfer-encoding', ['binary']), ('x-unexpected', ['new']), ('content-type', ['wrong']), ('x-request-id', ['a', 'b'])]:
                    changed = deepcopy(vector)
                    changed['cases'][case_index][response]['headers'][name] = value
                    with self.assertRaises(AssertionError):
                        api['compare_vectors'](json.dumps(changed).encode(), expected, 'complete proxy header drift')

    def test_approved_difference_is_narrow(self):
        root = Path(__file__).resolve().parents[2]
        vector = json.loads((root / 'vectors/agent_review192r2_attachment.json').read_bytes())
        api['check_approved_difference'](vector)
        for section, field, value in [('rails', 'status', 201), ('approved', 'status', 500), ('state', 'variant_file_exists', True)]:
            changed = deepcopy(vector)
            changed['cases'][0][section][field] = value
            with self.assertRaises(AssertionError):
                api['check_approved_difference'](changed)

    def test_approved_state_and_headers_are_not_masked(self):
        root = Path(__file__).resolve().parents[2]
        vector = json.loads((root / 'vectors/agent_review192r2_attachment.json').read_bytes())
        expected = json.dumps(vector).encode()
        for section, field, value in [('state', 'jobs', []), ('state', 'source_metadata', {}), ('approved', 'headers', {}), ('approved', 'response', '{}')]:
            changed = deepcopy(vector)
            changed['cases'][0][section][field] = value
            with self.assertRaises(AssertionError):
                api['compare_vectors'](json.dumps(changed).encode(), expected, 'approved drift')

    def test_approved_video_difference_is_narrow(self):
        root = Path(__file__).resolve().parents[2]
        vector = json.loads((root / 'vectors/agent_review192r3_attachment.json').read_bytes())
        api['check_approved_video_difference'](vector)
        changes = [
            ('rails', 'status', 201), ('approved', 'status', 500),
            ('exception', 'open_transactions', 0), ('rails_state', 'jobs', ['InjectedJob']),
            ('rails_state', 'row_deltas', {'messages': 1}), ('state', 'variant_count', 0),
        ]
        for section, field, value in changes:
            changed = deepcopy(vector)
            changed['cases'][0][section][field] = value
            with self.assertRaises(AssertionError):
                api['check_approved_video_difference'](changed)
        for index in range(3):
            changed = deepcopy(vector)
            changed['cases'][0]['state']['blobs'][index]['file_exists'] = False
            with self.assertRaises(AssertionError):
                api['check_approved_video_difference'](changed)

    def test_approved_video_attachment_targets_are_not_masked(self):
        root = Path(__file__).resolve().parents[2]
        vector = json.loads((root / 'vectors/agent_review192r3_attachment.json').read_bytes())
        expected = json.dumps(vector).encode()
        for index in range(3):
            for field, value in [('record_id', -1), ('record_type', 'WrongTarget')]:
                changed = deepcopy(vector)
                changed['cases'][0]['state']['attachments'][index][field] = value
                with self.assertRaises(AssertionError):
                    api['compare_vectors'](json.dumps(changed).encode(), expected, 'attachment target drift')
                with self.assertRaises(AssertionError):
                    api['check_approved_video_difference'](changed)

    def test_approved_video_state_and_headers_are_not_masked(self):
        root = Path(__file__).resolve().parents[2]
        vector = json.loads((root / 'vectors/agent_review192r3_attachment.json').read_bytes())
        expected = json.dumps(vector).encode()
        for section, field, value in [('state', 'jobs', []), ('state', 'attachments', []), ('approved', 'headers', {}), ('approved', 'response', '{}')]:
            changed = deepcopy(vector)
            changed['cases'][0][section][field] = value
            with self.assertRaises(AssertionError):
                api['compare_vectors'](json.dumps(changed).encode(), expected, 'approved video drift')


if __name__ == '__main__':
    unittest.main()
