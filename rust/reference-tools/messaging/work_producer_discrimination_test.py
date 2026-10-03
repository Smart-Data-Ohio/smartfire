"""Real producer rejections need their own application's intended assertion."""
import importlib.util
from pathlib import Path
import unittest

spec=importlib.util.spec_from_file_location('work_producer_discrimination',Path(__file__).with_name('work-producer-discrimination.py'))
module=importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

class WorkProducerAttributionTest(unittest.TestCase):
    def failures(self,rails,rust):
        return f'WS8bm positive application FAILED: Rails: {rails}\nWS8bm failed application: rust-url\nWS8bm positive application FAILED: Rust: {rust}\nWS8bm browser flow FAILED: aggregate\n'

    def test_intended_errors_on_both_apps(self):
        self.assertTrue(module.intended_failure_on_both(self.failures('work-event-count: 2 != 1','work-event-count: 2 != 1'),'work-event-count:'))

    def test_companion_marker_does_not_credit_a_startup_failure(self):
        self.assertFalse(module.intended_failure_on_both(self.failures('agent-event-count: 0 != 1','startup timeout'),'agent-event-count:'))

    def test_row_receipts_cannot_credit_an_earlier_browser_failure(self):
        text=self.failures('agent-event-count: 0 != 1','startup timeout')+'WS8bm real producer rows: {"row_error":"agent-event-count: 0"}\n'
        self.assertFalse(module.intended_failure_on_both(text,'agent-event-count:'))

    def test_multiline_http_status_assertion(self):
        self.assertTrue(module.intended_failure_on_both(self.failures('AssertionError\n200 !== 403\n    at request','AssertionError\n200 !== 403\n    at request'),'200 !== 403'))

if __name__=='__main__':unittest.main()
