import copy
import hashlib
import http.server
import json
import os
from pathlib import Path
import ssl
import subprocess
import tempfile
import threading
import unittest
from unittest.mock import patch

from install_apt import download_archives, install, validate_dependency_graph, validate_lock, verify_deb


LOCK = json.loads((Path(__file__).parent / "apt-amd64.lock.json").read_text())


class AptTransport(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.scratch = tempfile.TemporaryDirectory(prefix='ci-apt-transport-')
        cls.addClassCleanup(cls.scratch.cleanup)
        cls.certificate = Path(cls.scratch.name) / 'certificate.pem'
        cls.key = Path(cls.scratch.name) / 'key.pem'
        subprocess.run(['openssl', 'req', '-x509', '-newkey', 'rsa:2048', '-nodes',
                        '-days', '1', '-subj', '/CN=localhost', '-addext', 'subjectAltName=IP:127.0.0.1',
                        '-keyout', str(cls.key), '-out', str(cls.certificate)],
                       check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    def server(self, reject_extra_handshakes=False):
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(self.certificate, self.key)

        class Server(http.server.HTTPServer):
            handshakes = 0

            def get_request(self):
                connection, address = self.socket.accept()
                self.handshakes += 1
                if reject_extra_handshakes and self.handshakes > 1:
                    connection.close()  # Deterministic peer TLS EOF on a new connection.
                    raise OSError('additional TLS handshake rejected')
                return context.wrap_socket(connection, server_side=True), address

        class Handler(http.server.BaseHTTPRequestHandler):
            protocol_version = 'HTTP/1.1'

            def do_GET(self):
                self.server.requests.append(self.path)
                body = b'' if self.path in ('/missing', '/redirect') else self.path.encode()
                self.send_response(404 if self.path == '/missing' else 302 if self.path == '/redirect' else 200)
                if self.path == '/redirect':
                    self.send_header('Location', f'http://127.0.0.1:{self.server.server_port}/unsafe')
                self.send_header('Content-Length', str(len(body)))
                self.end_headers()
                self.wfile.write(body)

            def log_message(self, *args):
                pass

        server = Server(('127.0.0.1', 0), Handler)
        server.requests = []
        thread = threading.Thread(target=server.serve_forever, kwargs={'poll_interval': 0.01})
        thread.start()

        def stop():
            server.shutdown()
            thread.join()
            server.server_close()

        self.addCleanup(stop)
        return server

    def test_package_downloads_survive_peer_rejecting_new_tls_handshakes(self):
        server = self.server(reject_extra_handshakes=True)
        packages = [{'package': name, 'url': f'https://127.0.0.1:{server.server_port}/{name}'}
                    for name in ('first', 'second')]
        with tempfile.TemporaryDirectory() as scratch, \
             patch.dict(os.environ, {'CURL_CA_BUNDLE': str(self.certificate)}):
            archives = download_archives(packages, Path(scratch))
            self.assertEqual([Path(archive).read_bytes() for archive in archives], [b'/first', b'/second'])
        self.assertEqual(server.handshakes, 1)
        self.assertEqual(server.requests, ['/first', '/second'])

    def test_transfer_failure_does_not_request_a_later_archive(self):
        server = self.server()
        packages = [{'package': name, 'url': f'https://127.0.0.1:{server.server_port}/{name}'}
                    for name in ('first', 'missing', 'last')]
        with tempfile.TemporaryDirectory() as scratch, \
             patch.dict(os.environ, {'CURL_CA_BUNDLE': str(self.certificate)}):
            with self.assertRaises(subprocess.CalledProcessError) as failure:
                download_archives(packages, Path(scratch))
        self.assertEqual(failure.exception.returncode, 22)
        self.assertEqual(server.requests, ['/first', '/missing'])

    def test_second_transfer_cannot_follow_a_plaintext_redirect(self):
        server = self.server()
        packages = [{'package': name, 'url': f'https://127.0.0.1:{server.server_port}/{name}'}
                    for name in ('first', 'redirect')]
        with tempfile.TemporaryDirectory() as scratch, \
             patch.dict(os.environ, {'CURL_CA_BUNDLE': str(self.certificate)}):
            with self.assertRaises(subprocess.CalledProcessError) as failure:
                download_archives(packages, Path(scratch))
        self.assertEqual(failure.exception.returncode, 1)
        self.assertEqual(server.requests, ['/first', '/redirect'])


class AptPrerequisitePins(unittest.TestCase):
    def test_package_batch_keeps_one_transport_connection_pool(self):
        lock = copy.deepcopy(LOCK)
        lock['packages'] = lock['packages'][:2]
        lock['requested'] = [package['package'] for package in lock['packages']]
        for package in lock['packages']:
            package['sha256'] = hashlib.sha256(b'archive').hexdigest()
        packages = {package['package']: package for package in lock['packages']}

        def command_output(command, **kwargs):
            if command[:2] == ['dpkg', '--print-architecture']:
                return 'amd64\n'
            if command[0] == 'dpkg-deb':
                if '${Package}' in command[3]:
                    package = packages[Path(command[-1]).stem]
                    return '\t'.join(package[key] for key in ('package', 'version', 'architecture'))
                return '\t\t'
            if command[:2] == ['dpkg', '--audit']:
                return ''
            if command[0] == 'dpkg-query':
                return 'installed\t' + packages[command[-1]]['version']
            self.fail(f'unexpected package command: {command}')

        def download(command, **kwargs):
            if command[0] == 'curl':
                for index, argument in enumerate(command):
                    if argument == '--output':
                        Path(command[index + 1]).write_bytes(b'archive')

        with patch('install_apt.subprocess.run', side_effect=download) as run, \
             patch('install_apt.subprocess.check_output', side_effect=command_output):
            install(lock)
        transfers = [call.args[0] for call in run.call_args_list if call.args[0][0] == 'curl']
        self.assertEqual(len(transfers), 1, 'each curl process discards its TLS connection cache')
        self.assertIn('--fail-early', transfers[0])
        self.assertNotIn('--retry', transfers[0])

    def test_runtime_dependencies_require_locked_providers(self):
        packages = [{"package": "root"}, {"package": "provider"}]
        metadata = {"root": ("virtual-library:any (>= 1) | fallback", "provider", ""),
                    "provider": ("", "", "virtual-library (= 1)")}
        validate_dependency_graph(packages, metadata)
        metadata["root"] = ("unlocked-library (>= 1)", "", "")
        with self.assertRaisesRegex(ValueError, "unlocked runtime dependency"):
            validate_dependency_graph(packages, metadata)

    def test_lock_covers_prerequisites_and_rejects_missing_pins(self):
        validate_lock(LOCK)
        for mutation in ("checksum", "root", "duplicate"):
            lock = copy.deepcopy(LOCK)
            if mutation == "checksum":
                lock["packages"][0]["sha256"] = ""
            elif mutation == "root":
                lock["requested"].append("unlocked-dependency")
            else:
                lock["packages"].append(lock["packages"][0])
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                validate_lock(lock)

    def test_corrupted_archive_cannot_execute_package_tools(self):
        with tempfile.TemporaryDirectory() as scratch:
            archive = Path(scratch) / "test.deb"
            archive.write_bytes(b"corrupt")
            with patch("install_apt.subprocess.check_output") as command:
                with self.assertRaisesRegex(ValueError, "checksum mismatch"):
                    verify_deb(LOCK["packages"][0], archive)
                command.assert_not_called()

    def test_all_archives_are_verified_before_installation(self):
        lock = copy.deepcopy(LOCK)
        lock["packages"] = lock["packages"][:2]
        lock["requested"] = [package["package"] for package in lock["packages"]]
        lock["packages"][0]["sha256"] = hashlib.sha256(b"archive").hexdigest()

        def download(command, **kwargs):
            self.assertEqual(command[0], "curl", "package installation began before all checksums passed")
            for index, argument in enumerate(command):
                if argument == '--output':
                    Path(command[index + 1]).write_bytes(b"archive")

        with patch("install_apt.subprocess.run", side_effect=download), \
             patch("install_apt.subprocess.check_output", side_effect=["amd64\n", "\t".join(lock["packages"][0][key] for key in ("package", "version", "architecture"))]):
            with self.assertRaisesRegex(ValueError, "checksum mismatch"):
                install(lock)
