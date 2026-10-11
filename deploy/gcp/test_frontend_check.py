"""Exercise the post-cutover check over HTTP, including redirects and missing build assets."""
import importlib.util
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import threading
import unittest

spec = importlib.util.spec_from_file_location("frontend_check", Path(__file__).with_name("check-frontend.py"))
frontend = importlib.util.module_from_spec(spec)
spec.loader.exec_module(frontend)

SPA = '<div id="root"></div><script src="/app/assets/index-Abc12345.js"></script><link href="/app/assets/index-Abc12345.css" rel="stylesheet">'


class FrontendCheckTest(unittest.TestCase):
    def setUp(self):
        self.pages = {
            "/session/new": (302, "text/html", "/app/session/new"),
            "/": (302, "text/html", "/session/new"),
            "/about": (301, "text/html", "/app/about"),
            "/app/session/new": (200, "text/html", SPA),
            "/app/about": (200, "text/html", SPA),
            "/app/offline.html": (200, "text/html", SPA),
            "/app/assets/index-Abc12345.js": (200, "text/javascript", "export {}"),
            "/app/assets/index-Abc12345.css": (200, "text/css", "body {}"),
        }
        pages = self.pages

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                status, content_type, body = pages.get(self.path, (404, "text/plain", "missing"))
                self.send_response(status)
                self.send_header("Content-Type", content_type)
                if status in (301, 302):
                    self.send_header("Location", body)
                    body = ""
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

    def test_redirected_public_pages_land_on_the_spa_shell_with_real_assets(self):
        assets = ["/app/assets/index-Abc12345.css", "/app/assets/index-Abc12345.js"]
        receipt = frontend.check(self.origin, True)
        self.assertEqual(receipt, {
            "/session/new": {"status": 200, "location": "/app/session/new", "assets": assets},
            "/": {"status": 200, "location": "/app/session/new", "assets": assets},
            "/about": {"status": 200, "location": "/app/about", "assets": assets},
            "/app/offline.html": {"status": 200, "location": "/app/offline.html", "assets": assets},
        })

    def test_missing_dist_unserved_assets_and_wrong_landings_fail(self):
        retained = '<script src="/assets/auth-abc123.js"></script><link href="/assets/auth-abc123.css" rel="stylesheet">'
        for path, replacement, message in [
            ("/app/offline.html", (200, "text/html", "stub page"), "missing the built"),
            ("/app/assets/index-Abc12345.js", (404, "text/plain", "missing"), "returned 404"),
            ("/app/assets/index-Abc12345.css", (200, "text/html", SPA), "did not return text/css"),
            ("/app/session/new", (500, "text/html", "failed"), "returned 500"),
            ("/session/new", (200, "text/html", SPA), "ended at /session/new, expected /app/session/new"),
            ("/about", (302, "text/html", "/app/session/new"), "ended at /app/session/new, expected /app/about"),
            ("/app/session/new", (200, "text/html", retained), "missing the built"),
        ]:
            with self.subTest(path=path, replacement=replacement):
                saved = self.pages[path]
                self.pages[path] = replacement
                with self.assertRaisesRegex(ValueError, message):
                    frontend.check(self.origin, True)
                self.pages[path] = saved

    def test_disabled_spa_flag_skips_the_offline_page(self):
        del self.pages["/app/offline.html"]
        self.assertEqual(set(frontend.check(self.origin, False)), {"/session/new", "/", "/about"})
