"""Real producer rejections need the application's intended assertion."""
import importlib.util
from pathlib import Path
import unittest

spec=importlib.util.spec_from_file_location('work_producer_discrimination',Path(__file__).with_name('work-producer-discrimination.py'))
module=importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

class WorkProducerAttributionTest(unittest.TestCase):
    def failure(self,message):
        return f'WS8bm failed application: rust-url\nWS8bm positive application FAILED: converts a thread: {message}\nWS8bm browser flow FAILED: aggregate\n'

    def test_intended_error(self):
        self.assertTrue(module.intended_failure(self.failure('work-event-count: 2 != 1'),'work-event-count:'))

    def test_another_failure_earns_no_credit(self):
        self.assertFalse(module.intended_failure(self.failure('startup timeout'),'agent-event-count:'))

    def test_row_receipts_cannot_credit_an_earlier_browser_failure(self):
        text=self.failure('startup timeout')+'WS8bm real producer rows: {"row_error":"agent-event-count: 0"}\n'
        self.assertFalse(module.intended_failure(text,'agent-event-count:'))

    def test_multiline_http_status_assertion(self):
        self.assertTrue(module.intended_failure(self.failure('AssertionError\n200 !== 403\n    at request'),'200 !== 403'))

if __name__=='__main__':unittest.main()
