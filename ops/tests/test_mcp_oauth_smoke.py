"""Run the deployment smoke script with real curl against a disposable HTTP boundary."""
import copy
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import subprocess
import threading
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "ops/scripts/mcp-oauth-smoke.sh"
BASE = "http://127.0.0.1:4398"
AS = "/.well-known/oauth-authorization-server"
RESOURCE = "/.well-known/oauth-protected-resource/agents/mcp"


def responses():
    server = {
        "issuer": BASE,
        "authorization_endpoint": BASE + "/oauth/authorize",
        "token_endpoint": BASE + "/oauth/token",
        "registration_endpoint": BASE + "/oauth/register",
        "revocation_endpoint": BASE + "/oauth/revoke",
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none", "private_key_jwt"],
        "revocation_endpoint_auth_methods_supported": ["none", "private_key_jwt"],
        "token_endpoint_auth_signing_alg_values_supported": ["RS256", "ES256"],
        "revocation_endpoint_auth_signing_alg_values_supported": ["RS256", "ES256"],
        "client_id_metadata_document_supported": True,
        "authorization_response_iss_parameter_supported": True,
        "scopes_supported": ["agent"],
    }
    resource = {"resource": BASE + "/agents/mcp", "authorization_servers": [BASE],
                "scopes_supported": ["agent"], "bearer_methods_supported": ["header"]}
    return {
        AS: [200, {}, server],
        RESOURCE: [200, {}, resource],
        "/.well-known/oauth-protected-resource": [200, {}, copy.deepcopy(resource)],
        "/oauth/register": [204, {"Access-Control-Allow-Origin": "*",
                                  "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
                                  "Access-Control-Allow-Headers": "Content-Type, Authorization"}, None],
        "/agents/mcp": [401, {"WWW-Authenticate": 'Bearer resource_metadata="' + BASE + RESOURCE + '", scope="agent"'}, None],
    }


class Handler(BaseHTTPRequestHandler):
    def respond(self):
        self.server.requests.append((self.command, self.path, dict(self.headers)))
        status, headers, body = self.server.responses[self.path]
        self.send_response(status)
        for name, value in headers.items():
            self.send_header(name, value)
        self.end_headers()
        if body is not None:
            self.wfile.write(json.dumps(body).encode())

    do_GET = respond
    do_OPTIONS = respond
    do_POST = respond

    def log_message(self, *args):
        pass


class SmokeTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.server = ThreadingHTTPServer(("127.0.0.1", 4398), Handler)
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        cls.thread.join()

    def setUp(self):
        self.server.responses = responses()
        self.server.requests = []

    def run_script(self, *args):
        return subprocess.run([str(SCRIPT), *args], cwd=ROOT, capture_output=True, text=True,
                              env={**os.environ, "NO_PROXY": "127.0.0.1"})

    def rejects(self):
        result = self.run_script(BASE)
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_passes_without_credentials_and_uses_only_read_operations(self):
        result = self.run_script(BASE + "/")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual([(method, path) for method, path, _ in self.server.requests],
                         [("GET", AS), ("GET", RESOURCE), ("GET", "/.well-known/oauth-protected-resource"),
                          ("OPTIONS", "/oauth/register"), ("POST", "/agents/mcp")])
        for _, _, headers in self.server.requests:
            self.assertFalse({name.lower() for name in headers} & {"authorization", "cookie"})

    def test_rejects_each_missing_or_wrong_authorization_server_field(self):
        for name in responses()[AS][2]:
            for replacement in [None, "wrong"]:
                with self.subTest(name=name, replacement=replacement):
                    self.server.responses = responses()
                    self.server.responses[AS][2][name] = replacement
                    self.rejects()

    def test_rejects_each_wrong_resource_field_on_both_aliases(self):
        for path in [RESOURCE, "/.well-known/oauth-protected-resource"]:
            for name in responses()[path][2]:
                with self.subTest(path=path, name=name):
                    self.server.responses = responses()
                    self.server.responses[path][2][name] = None
                    self.rejects()

    def test_rejects_status_cors_cookies_credentials_and_missing_challenges(self):
        mutations = [(path, 0, 200 if path == "/agents/mcp" else 404) for path in responses()]
        mutations += [("/oauth/register", 1, headers) for headers in [
            {}, {"Access-Control-Allow-Origin": "https://client.example"},
            {**responses()["/oauth/register"][1], "Set-Cookie": "session=bad"},
            {**responses()["/oauth/register"][1], "Access-Control-Allow-Credentials": "true"},
            {**responses()["/oauth/register"][1], "Access-Control-Allow-Methods": "GET"},
            {**responses()["/oauth/register"][1], "Access-Control-Allow-Methods": "NOTPOST"},
            {**responses()["/oauth/register"][1], "Access-Control-Allow-Headers": "other"},
            {**responses()["/oauth/register"][1], "Access-Control-Allow-Headers": "other-content-type"},
        ]]
        mutations += [("/agents/mcp", 1, {"WWW-Authenticate": value}) for value in [
            "", "Basic", 'Bearer resource_metadata="https://other.example/metadata", scope="agent"',
            'Bearer resource_metadata="' + BASE + RESOURCE + '", scope="other"',
            'Bearer resource_metadata="wrong", resource_metadata="' + BASE + RESOURCE + '", scope="agent"',
        ]]
        for path, index, value in mutations:
            with self.subTest(path=path, value=value):
                self.server.responses = responses()
                self.server.responses[path][index] = value
                self.rejects()

    def test_rejects_invalid_arguments_before_network_requests(self):
        for args in [[], [BASE, BASE], [BASE + "/path"], ["ftp://example.com"],
                     ["http://user:password@example.com"], [BASE + "?query=yes"]]:
            with self.subTest(args=args):
                self.assertEqual(self.run_script(*args).returncode, 2)
        self.assertEqual(self.server.requests, [])
