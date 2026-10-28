#!/usr/bin/env python3
"""Isolated local integration oracle; starts only loopback test listeners.
NGINX_BIN names an installed nginx binary. Optional argv[1] replaces the route snippet.
No live configuration, profile data or recovery material is read or changed.
"""
import http.server
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import threading
import time

ROOT = Path(__file__).resolve().parents[1]
DEPLOY = ROOT / 'om/agora/deploy'
seen = []

class Upstream(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        seen.append((self.command, self.path, dict(self.headers), getattr(self, 'body', b'')))
        self.send_response(200)
        self.end_headers()
        self.wfile.write(b'registration oracle\n')
    def do_POST(self):
        self.body = self.rfile.read(int(self.headers.get('Content-Length', '0')))
        self.do_GET()
    def log_message(self, *_):
        pass

def port():
    with socket.socket() as s:
        s.bind(('127.0.0.1', 0))
        return s.getsockname()[1]

with tempfile.TemporaryDirectory(prefix='agora-nginx-') as td:
    tmp = Path(td)
    backend = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Upstream)
    threading.Thread(target=backend.serve_forever, daemon=True).start()
    proxy, plain = port(), port()
    routes = Path(sys.argv[1]).read_text() if len(sys.argv) > 1 else (DEPLOY / 'nginx-agora.conf').read_text()
    (tmp / 'upstream.conf').write_text(f'server 127.0.0.1:{backend.server_port};\n')
    http = (DEPLOY / 'nginx-agora-http.conf').read_text().replace('/etc/nginx/snippets/agora-upstream.conf', str(tmp / 'upstream.conf'))
    config = tmp / 'nginx.conf'
    config.write_text(f'''pid {tmp}/pid;
error_log {tmp}/error.log debug;
worker_processes 1;
events {{ worker_connections 128; }}
http {{
fastcgi_temp_path {tmp}/fastcgi;
uwsgi_temp_path {tmp}/uwsgi;
scgi_temp_path {tmp}/scgi;
access_log {tmp}/access.log;
client_body_temp_path {tmp}/body;
proxy_temp_path {tmp}/proxy;
{http}
server {{
listen 127.0.0.1:{proxy} proxy_protocol;
listen 127.0.0.1:{plain};
server_name localhost;
{routes}
# A hostile sibling regex must not override the API prefix boundary.
location ~ /agora/api/ {{ return 418; }}
location / {{ return 200 'logging control'; }}
}}
}}
''')
    nginx = os.environ.get('NGINX_BIN', 'nginx')
    version = subprocess.run([nginx, '-V'], capture_output=True, text=True, check=True)
    assert '--with-debug' in version.stderr, 'INVALID: debug-capable nginx required'
    args = [nginx, '-p', td + '/', '-c', str(config)]
    subprocess.run(args + ['-t'], check=True)
    proc = subprocess.Popen(args + ['-g', 'daemon off;'], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    def request(path, method='GET', identity='198.51.100.7', body=b'', spoof='203.0.113.99'):
        target = proxy if identity is not None else plain
        with socket.create_connection(('127.0.0.1', target), timeout=5) as s:
            if identity is not None:
                s.sendall(f'PROXY TCP4 {identity} 127.0.0.1 12345 {proxy}\r\n'.encode())
            s.sendall((f'{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nX-Real-IP: {spoof}\r\nX-Forwarded-For: {spoof}\r\nContent-Length: {len(body)}\r\n\r\n').encode() + body)
            data = b''
            while True:
                chunk = s.recv(65536)
                if not chunk:
                    break
                data += chunk
        head = data.split(b'\r\n\r\n')[0].decode()
        return int(head.split()[1]), head.lower()
    def expect(path, status, **kwargs):
        before = len(seen)
        actual, headers = request(path, **kwargs)
        assert actual == status, (path, kwargs, status, actual)
        assert 'cache-control: no-store' in headers, (path, headers)
        assert 'x-content-type-options: nosniff' in headers, (path, headers)
        assert len(seen) == before + (1 if status == 200 else 0), (path, 'unexpected upstream traffic')
    try:
        for _ in range(100):
            try:
                with socket.create_connection(('127.0.0.1', plain), timeout=.1):
                    break
            except OSError:
                if proc.poll() is not None:
                    raise RuntimeError(proc.stderr.read().decode())
                time.sleep(.02)
        allowed = [('/agora/api/challenge', 'GET'), ('/agora/api/hello', 'POST'), ('/agora/api/agent/0123456789abcdef', 'GET')]
        for path, method in allowed:
            expect(path, 200, method=method)
            assert seen[-1][2]['X-Real-IP'] == '198.51.100.7'
            assert 'X-Forwarded-For' not in seen[-1][2]
            expect(path, 503, method=method, identity=None)
            for wrong in ['HEAD', 'PUT', 'DELETE', 'OPTIONS', 'GET' if method == 'POST' else 'POST']:
                expect(path, 405, method=wrong)
        for path in ['/agora/api', '/agora/api/post', '/agora/api/import', '/agora/api/note', '/agora/api/peer', '/agora/api/stream', '/agora/api/challenge/extra', '/agora/api/hello/', '/agora/api/agent/0123456789abcdef/extra', '/agora/api/agent/ABCDEF0123456789', '/agora/api/agent/0123']:
            expect(path, 404)
        accepted = b'public-accepted-profile-sentinel;synthetic-recovery-sentinel=' + b'x' * 100
        expect('/agora/api/hello', 200, method='POST', body=accepted)
        assert seen[-1][3] == accepted, 'accepted body changed upstream'
        bounded = accepted.ljust(16384, b'x')
        expect('/agora/api/hello', 200, method='POST', body=bounded)
        assert seen[-1][3] == bounded, 'upper-bound accepted body changed upstream'
        expect('/agora/api/hello', 413, method='POST', body=b'public-privacy-sentinel' * 1024)
        for i in range(25):
            before = len(seen)
            status, headers = request('/agora/api/challenge', identity='198.51.100.8', spoof=f'203.0.113.{i}')
            assert status in (200, 429)
            assert len(seen) == before + (1 if status == 200 else 0), 'rate refusal reached upstream'
            if status == 429:
                assert 'cache-control: no-store' in headers and 'x-content-type-options: nosniff' in headers
                break
        else:
            raise AssertionError('forged headers escaped same-client quota')
        expect('/agora/api/challenge', 200, identity='198.51.100.9')
        request('/logging-control', identity=None)
        # nginx flushes the unbuffered access log before the response is complete.
        access = (tmp / 'access.log').read_text()
        assert '/logging-control' in access, 'logging negative control did not log'
        assert '/agora/api' not in access, 'API leaked into inherited access log'
        debug = (tmp / 'error.log').read_text()
        assert 'http request line: "GET /logging-control HTTP/1.1"' in debug, 'INVALID: debug logging control inactive'
        for sentinel in ['public-privacy-sentinel', 'public-accepted-profile-sentinel', 'synthetic-recovery-sentinel']:
            assert sentinel not in debug, 'synthetic request body leaked to debug log'
        assert not list((tmp / 'body').glob('*')), 'retained temporary request body'
        print('AGORA DEPLOY PASS exact routes/methods, no upstream on refusal, non-PROXY refusal, spoof identity/quota, response privacy headers, API access-log exclusion. Synthetic request-body sentinels absent from observed debug log; debug active. Suppression causality not established (override-removal mutant survived). No retained body files; transient spill and secret-bearing response logs not qualified.')
    finally:
        proc.terminate()
        proc.communicate(timeout=10)
        backend.shutdown()
