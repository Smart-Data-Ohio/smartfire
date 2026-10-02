#!/usr/bin/env python3
import unittest
from copy import deepcopy
from proxy_headers import compare_headers, SECURITY

class FullHeaders(unittest.TestCase):
    def fixtures(self):
        rails = {'content-type':['image/webp'], 'content-length':['3326'], 'vary':['Accept-Encoding'], 'content-security-policy':["script-src 'self' 'nonce-AAECAwQFBgcICQoLDA0ODw=='"]}
        rust = deepcopy(rails)
        rust.update({k:[v] for k,v in SECURITY.items()})
        return rust,rails
    def test_only_named_approved_differences_pass(self):
        rust,rails=self.fixtures()
        rust.update({'x-request-id':['3574925f-479d-44f8-82b7-fc039af5367c'],'x-runtime':['0.000001'],'date':['Mon, 02 Mar 2026 16:00:00 GMT']})
        compare_headers(rust,rails,True)
    def test_every_other_header_value_is_compared(self):
        rust,rails=self.fixtures()
        for key in rails:
            changed=deepcopy(rust);changed[key]=['unexpected']
            with self.assertRaises(AssertionError):compare_headers(changed,rails,True)
    def test_unexpected_and_missing_names_fail(self):
        rust,rails=self.fixtures()
        for key in ['content-transfer-encoding','x-unexpected','content-range','set-cookie']:
            changed=deepcopy(rust);changed[key]=['binary']
            with self.assertRaises(AssertionError):compare_headers(changed,rails,True)
        for key in rails:
            changed=deepcopy(rust);del changed[key]
            with self.assertRaises(AssertionError):compare_headers(changed,rails,True)
    def test_duplicate_values_fail_even_for_approved_names(self):
        rust,rails=self.fixtures()
        for key in set(rust)|{'x-request-id','date','x-runtime'}:
            changed=deepcopy(rust);changed[key]=['same','same']
            with self.assertRaises(AssertionError):compare_headers(changed,rails,True)
    def test_duplicate_case_insensitive_names_fail(self):
        rust,rails=self.fixtures();rust['Content-Type']=rust['content-type']
        with self.assertRaises(AssertionError):compare_headers(rust,rails,True)
    def test_security_values_and_unapproved_routes_are_not_masked(self):
        rust,rails=self.fixtures()
        for key in SECURITY:
            changed=deepcopy(rust);changed[key]=['unsafe']
            with self.assertRaises(AssertionError):compare_headers(changed,rails,True)
        with self.assertRaises(AssertionError):compare_headers(rust,rails,False)

if __name__=='__main__':unittest.main()
