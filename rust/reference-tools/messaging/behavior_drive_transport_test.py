"""The pinned external transport rejects wrong queries and always releases its listener."""
import socket
import unittest
from urllib.request import Request, urlopen
from urllib.error import HTTPError
from behavior_drive_transport import drive_transport

class DriveTransportTests(unittest.TestCase):
    def test_real_http_scopes_and_teardown_after_diagnostic_failure(self):
        payloads = {'list': {'files': []}, 'files': {'fixture': {'name': 'Q3 Planning'}}}
        with self.assertRaisesRegex(RuntimeError, 'diagnostic failure'):
            with drive_transport(payloads, 0) as calls:
                port = calls.port  # Held ephemeral listener, no close/rebind gap.
                headers = {'Host': 'www.googleapis.com', 'Authorization': 'Bearer fixture'}
                with urlopen(Request(f'http://127.0.0.1:{port}/drive/v3/files?pageSize=10', headers=headers)) as reply:
                    self.assertEqual(reply.status, 200)
                # Like the original WebMock stub, a matching URL/query is
                # registered independently of its Authorization header.
                with urlopen(Request(f'http://127.0.0.1:{port}/drive/v3/files?pageSize=10', headers={'Host': 'www.googleapis.com'})) as reply:
                    self.assertEqual(reply.status, 200)
                with self.assertRaises(HTTPError) as rejected:
                    urlopen(Request(f'http://127.0.0.1:{port}/drive/v3/files?pageSize=11', headers=headers))
                rejected.exception.close()
                with urlopen(Request(f'http://127.0.0.1:{port}/drive/v3/files/fixture?supportsAllDrives=true', headers=headers)) as reply:
                    self.assertEqual(reply.status, 200)
                self.assertEqual([row['valid'] for row in calls], [True, True, False, True])
                raise RuntimeError('diagnostic failure')
        with socket.socket() as check:
            self.assertNotEqual(check.connect_ex(('127.0.0.1', port)), 0)

if __name__ == '__main__':
    unittest.main()
