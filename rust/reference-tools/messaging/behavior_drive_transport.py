"""Local external Drive HTTP, accepting exactly the pinned WebMock query scopes."""
from contextlib import contextmanager
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import threading
from urllib.parse import urlsplit, parse_qs


@contextmanager
def drive_transport(payloads, port):
    class Calls(list):
        pass
    calls = Calls()
    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            path = urlsplit(self.path)
            query = parse_qs(path.query)
            # The pinned WebMock stubs match URL/method/query, not auth headers.
            # Forward the real headers, but do not add a stricter stub predicate.
            valid = self.headers.get("Host") == "www.googleapis.com"
            if path.path == "/drive/v3/files":
                valid &= query.get("pageSize") == ["10"]
                body = payloads["list"]
            else:
                valid &= query.get("supportsAllDrives") == ["true"] and path.path.removeprefix("/drive/v3/files/") in payloads["files"]
                body = payloads["files"].get(path.path.removeprefix("/drive/v3/files/"))
            calls.append({"path": self.path, "valid": valid})
            data = json.dumps(body if valid else {"error": "unregistered pinned Drive request"}).encode()
            self.send_response(200 if valid else 500)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)
        def log_message(self, *_):
            pass
    server = ThreadingHTTPServer(("127.0.0.1", port), Handler)
    calls.port = server.server_port
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield calls
    finally:
        server.shutdown()
        server.server_close()
        thread.join()
