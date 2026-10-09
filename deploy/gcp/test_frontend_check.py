"""Exercise the post-cutover check over HTTP, including redirects and missing build assets."""
import importlib.util
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import threading
import unittest

spec = importlib.util.spec_from_file_location("frontend_check", Path(__file__).with_name("check-frontend.py"))
frontend = importlib.util.module_from_spec(spec)
spec.loader.exec_module(frontend)

AUTH = '<script src="/assets/auth-abc123.js"></script><link href="/assets/auth-abc123.css" rel="stylesheet">'
SPA = '<div id="root"></div><script src="/app/assets/index-Abc12345.js"></script><link href="/app/assets/index-Abc12345.css" rel="stylesheet">'


class FrontendCheckTest(unittest.TestCase):
    def setUp(self):
        self.pages = {
            "/session/new": (200, "text/html", AUTH),
            "/": (302, "text/html", ""),
            "/app/offline.html": (200, "text/html", SPA),
            "/assets/auth-abc123.js": (200, "text/javascript", "export {}"),
            "/assets/auth-abc123.css": (200, "text/css", "body {}"),
            "/app/assets/index-Abc12345.js": (200, "text/javascript", "export {}"),
            "/app/assets/index-Abc12345.css": (200, "text/css", "body {}"),
        }
        pages = self.pages

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                status, content_type, body = pages.get(self.path, (404, "text/plain", "missing"))
                self.send_response(status)
                self.send_header("Content-Type", content_type)
                if status == 302:
                    self.send_header("Location", "/session/new")
                self.end_headers()
                self.wfile.write(body.encode())

            def log_message(self, *args):
                pass

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.origin = f"http://127.0.0.1:{self.server.server_port}"

    def tearDown(self):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join()

    def test_redirected_root_and_auth_pages_load_real_assets_and_spa_shell(self):
        receipt = frontend.check(self.origin, True)
        self.assertEqual(receipt, {
            "/session/new": {"status": 200, "assets": ["/assets/auth-abc123.css", "/assets/auth-abc123.js"]},
            "/": {"status": 200, "assets": ["/assets/auth-abc123.css", "/assets/auth-abc123.js"]},
            "/app/offline.html": {"status": 200, "assets": ["/app/assets/index-Abc12345.css", "/app/assets/index-Abc12345.js"]},
        })

    def test_missing_dist_and_unserved_assets_fail(self):
        for path, replacement, message in [
            ("/app/offline.html", (200, "text/html", "stub page"), "missing the built"),
            ("/app/assets/index-Abc12345.js", (404, "text/plain", "missing"), "returned 404"),
            ("/assets/auth-abc123.css", (200, "text/html", AUTH), "did not return text/css"),
            ("/session/new", (500, "text/html", "failed"), "returned 500"),
        ]:
            with self.subTest(path=path):
                saved = self.pages[path]
                self.pages[path] = replacement
                with self.assertRaisesRegex(ValueError, message):
                    frontend.check(self.origin, True)
                self.pages[path] = saved

    def test_disabled_spa_still_checks_auth_without_requiring_spa_routes(self):
        del self.pages["/app/offline.html"]
        self.assertEqual(set(frontend.check(self.origin, False)), {"/session/new", "/"})
