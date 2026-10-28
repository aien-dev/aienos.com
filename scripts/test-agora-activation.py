#!/usr/bin/env python3
"""Root-only isolated activation rehearsal and private browser control.

Only the Fixture adapter performs mutations. All observations use the same
file-backed code intended for live activation; no production paths are accepted.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import socket
import sys

from agora_activation_namespace import Fixture
from agora_activation_observers import Commands, Observers, Refused, properties, save_private


class Rehearsal:
    def __init__(self, args):
        self.root = Path(args.root)
        self.root.mkdir(mode=0o700, parents=True, exist_ok=False)
        self.run = Commands()
        self.fixture = Fixture(self.root, Path(args.backend), Path(args.nginx), self.run,
                               Path(args.page), Path(args.client))
        self.native, self.startup = args.native, args.startup
        self.gates = []
        self.keys = []
        self.pre_browser_passed = False
        self.browser_passed = False
        self.markers = [b'ACTIVATION_PUBLIC_BODY_SENTINEL',
                        b'ACTIVATION_SYNTHETIC_KEY_SENTINEL']
        self.config = {}
        prefix = self.fixture.command([])
        self.observer = Observers(self.run, self.native, self.startup, prefix=prefix)
        self.pid = None

    def setup(self):
        self.config = self.fixture.setup()

    def gate(self, name, value=None):
        self.gates.append({'gate': name, 'status': 'PASS', 'observation': value})

    def startup_check(self, narrow=False):
        self.pid = self.observer.startup(self.config['unit'],
                    self.config['executable'],
                    self.config['backend_host'] + ':' + str(self.config['backend_port']))
        self.observer.effective(self.config, self.pid,
                                '127.0.0.1' if narrow else None)
        return self.pid

    def direct(self, source):
        return self.run(self.fixture.command(['curl', '--silent', '--show-error',
            '--max-time', '5', '--interface', source, '--output', '/dev/null',
            '--write-out', '%{http_code}',
            'http://' + self.config['backend_host'] + ':' +
            str(self.config['backend_port']) + '/agora/api/challenge']), check=False)

    def request(self, path, method='GET', body=None, plain=False, headers=()):
        base = self.config['plain_url'] if plain else self.config['tls_url']
        argv = ['curl', '--silent', '--show-error', '--max-time', '10',
                '--insecure', '--noproxy', '*', '--include']
        if not plain and self.config.get('curl_resolve'):
            argv += ['--resolve', self.config['curl_resolve']]
        argv += ['--head'] if method == 'HEAD' else ['--request', method]
        for header in headers:
            argv += ['--header', header]
        if body is not None:
            argv += ['--data-binary', '@-']
        argv.append(base.rstrip('/') + path)
        result = self.run(argv, input=body, private=True)
        head, separator, content = result.stdout.partition(b'\r\n\r\n')
        if not separator:
            raise Refused('Incomplete actual TLS HTTP response')
        status = int(head.splitlines()[0].split()[1])
        fields = {}
        for line in head.splitlines()[1:]:
            key, sep, value = line.partition(b':')
            if not sep:
                raise Refused('Malformed HTTP header')
            fields.setdefault(key.lower(), []).append(value.strip().lower())
        return status, fields, content

    @staticmethod
    def expected(response, code):
        status, headers, _ = response
        if status != code:
            raise Refused('HTTP expected ' + str(code) + ', observed ' + str(status))
        if headers.get(b'cache-control') != [b'no-store']:
            raise Refused('No-store absent or ambiguous')
        if headers.get(b'x-content-type-options') != [b'nosniff']:
            raise Refused('Nosniff absent or ambiguous')

    def no_upstream(self, request):
        expression = 'tcp dst port ' + str(self.config['backend_port'])
        with self.observer.capture(self.config['interface'], expression) as observed:
            response = request()
        import re
        if any(int(size) > 0 for size in re.findall(r'\blength (\d+)\b', observed['packets'])):
            raise Refused('Blocked route generated upstream payload')
        return response

    def pre_browser(self):
        self.observer.copied_artifacts(self.config['copied_artifacts'])
        self.gate('copied backend/page/client exact source bytes, root ownership and regular-file modes')
        self.startup_check()
        self.gate('bounded startup/effective native guards/current namespace/cgroup BPF')
        # Independently establish .3 reachability to this exact endpoint in a
        # separate declared control phase, then restore the exact acceptance set.
        self.fixture.set_policy(['127.0.0.1/32', '127.0.0.2/32', '127.0.0.3/32'])
        self.observer.startup(self.config['unit'], self.config['executable'],
                             self.config['backend_host'] + ':' + str(self.config['backend_port']))
        reachable = self.observer.syn_probe(self.config, '127.0.0.3',
                                            lambda: self.direct('127.0.0.3'), True)
        self.gate('independent same-endpoint second-source reachability control', reachable)
        self.fixture.set_policy(['127.0.0.1/32', '127.0.0.2/32'])
        self.startup_check()
        for source, accept in [('127.0.0.2', True), ('127.0.0.3', False)]:
            observation = self.observer.syn_probe(self.config, source,
                lambda source=source: self.direct(source), accept)
            self.gate('actual source-bound packet allow/deny', observation)
        self.fixture.set_policy(['127.0.0.1/32'])
        self.startup_check(narrow=True)
        self.observer.syn_probe(self.config, '127.0.0.2',
                                lambda: self.direct('127.0.0.2'), False)
        self.gate('independent real BPF denial with no firewall mutation')
        self.fixture.set_policy(['127.0.0.1/32', '127.0.0.2/32'])
        self.startup_check()
        self.observer.syn_probe(self.config, '127.0.0.2',
                                lambda: self.direct('127.0.0.2'), True)
        self.gate('exact service policy restored and allowed source recovers')
        self.observer.reload(self.config)
        self.expected(self.observer.counted_get(self.config,
                      lambda: self.request('/agora/api/challenge')), 200)
        self.gate('nginx active-worker adoption plus independently counted TLS GET')
        refused = [('/agora/api/challenge', 'POST', 405),
                   ('/agora/api/hello', 'GET', 405),
                   ('/agora/api/agent/0123456789abcdef', 'POST', 405),
                   ('/agora/api/agent/ABCDEF0123456789', 'GET', 404),
                   ('/agora/api/agent/0123', 'GET', 404),
                   ('/agora/api/challenge/extra', 'GET', 404),
                   ('/agora/api/import', 'POST', 404),
                   ('/agora/api/hello/', 'POST', 404)]
        refused += [(path, 'HEAD', 405) for path in (
            '/agora/api/challenge', '/agora/api/hello',
            '/agora/api/agent/0123456789abcdef')]
        refused += [(path, 'POST', 404) for path in (
            '/agora/api/post', '/agora/api/note', '/agora/api/peer',
            '/agora/api/stream', '/agora/api/other')]
        refused += [(path, 'GET', 404) for path in (
            '/agora/api', '/agora/api/agent/0123456789abcdef/extra')]
        for path, method, code in refused:
            response = self.no_upstream(lambda path=path, method=method:
                                      self.request(path, method))
            self.expected(response, code)
        for path, method in (('/agora/api/challenge', 'GET'),
                             ('/agora/api/hello', 'POST'),
                             ('/agora/api/agent/0123456789abcdef', 'GET')):
            self.expected(self.no_upstream(
                lambda path=path, method=method: self.request(path, method, plain=True)), 503)
        oversized = self.markers[0] * 1024
        self.expected(self.no_upstream(lambda: self.request('/agora/api/hello',
                      'POST', oversized)), 413)
        self.gate('actual TLS method/id/trailing/disabled/oversize and non-PROXY refusals, zero upstream payload')
        synthetic = b'ACTIVATION_PUBLIC_BODY_SENTINEL\nkey ACTIVATION_SYNTHETIC_KEY_SENTINEL\n'
        self.observer.effective_logging(self.config)
        self.expected(self.request('/agora/api/hello', 'POST', synthetic), 400)
        self.gate('synthetic body/key real backend failure path')
        # Browser gates run before quota exhaustion; these requests preserve the
        # real client bucket rather than relying on arbitrary forged headers.
        self.observer.privacy(self.config['logs'], self.config['temp_dirs'], self.markers)
        self.gate('effective actual fixture access/error/temp sinks have no synthetic body/key markers')
        self.pre_browser_passed = True
        return {'gates': self.gates, 'config': self.config}

    def save_key(self, key):
        if not isinstance(key, str) or not key or '\n' in key:
            raise Refused('Invalid private recovery material')
        path = self.root / ('recovery-' + str(len(self.keys)) + '.key')
        save_private(path, key)
        if path.stat().st_uid != 0:
            raise Refused('Recovery file is not root-owned')
        self.keys.append(path)
        return {'path': str(path), 'mode': '600', 'owner': 'root'}

    def restart(self):
        self.fixture.restart_backend()
        self.startup_check()
        return {'status': 'READY'}

    def read_profile(self, identity):
        if len(identity) != 16 or any(ch not in '0123456789abcdef' for ch in identity):
            raise Refused('Malformed public profile id')
        response = self.request('/agora/api/agent/' + identity)
        self.expected(response, 200)
        return response[2].decode()

    def after_browser(self, browser_result):
        if not self.pre_browser_passed or browser_result.get('status') != 'PASS':
            raise Refused('Full pre-browser and browser sequence must pass')
        if not self.keys:
            raise Refused('Browser did not preserve actual recovery material')
        self.gate('real browser consent/ack/readback/uncertainty/durability', browser_result)
        def burst():
            def forged(index):
                response = self.request('/agora/api/challenge', headers=(
                    'X-Real-IP: 203.0.113.' + str(index),
                    'X-Forwarded-For: 198.51.100.' + str(index)))
                if response[0] not in (200, 429):
                    raise Refused('Unexpected actual TLS quota response')
                self.expected(response, response[0])
                return response[0]
            with ThreadPoolExecutor(max_workers=32) as workers:
                return list(workers.map(forged, range(1, 33)))
        statuses = self.observer.counted_get(
            self.config, burst, expected=lambda rows: rows.count(200))
        if not statuses.count(200) or not statuses.count(429):
            raise Refused('Forged headers did not exercise the shared real-client bucket')
        self.gate('actual TLS forged identity headers share quota; refusals produce zero upstream requests',
                  {'accepted': statuses.count(200), 'limited': statuses.count(429)})
        markers = self.markers + [path.read_bytes() for path in self.keys]
        self.observer.privacy(self.config['logs'], self.config['temp_dirs'], markers)
        self.observer.privacy(self.config['access_logs'], [], [b'/agora/api'])
        self.gate('actual response recovery keys absent from every effective log/temp-body sink')
        self.fixture.enable_backend()
        enabled = self.run(['systemctl', 'is-enabled', self.config['unit']])
        if enabled.stdout.strip() != b'enabled-runtime':
            raise Refused('Throwaway unit did not become enabled-runtime')
        self.gate('enable only after every acceptance gate passes, isolated runtime target only')
        self.browser_passed = True
        return {'status': 'PASS', 'private_recovery_paths': list(map(str, self.keys))}

    def state_hashes(self):
        root = Path(self.config['state'])
        return {str(path.relative_to(root)): hashlib.sha256(path.read_bytes()).hexdigest()
                for path in root.rglob('*') if path.is_file()}

    def rollback_proof(self):
        if not self.browser_passed:
            raise Refused('Entire browser, privacy and enable sequence has not passed')
        before = self.state_hashes()
        try:
            raise Refused('Intentional dry-run acceptance-failure control')
        except Refused:
            self.fixture.rollback()
        disabled = self.run(['systemctl', 'is-enabled', self.config['unit']], check=False)
        if disabled.stdout.strip() != b'disabled':
            raise Refused('Rollback did not disable the isolated unit')
        unit = properties(self.observer.show(self.config['unit'], 'MainPID', 'ActiveState'))
        if int(unit['MainPID']) != 0 or unit['ActiveState'] != 'inactive':
            raise Refused('Rollback backend not stopped')
        if self.state_hashes() != before:
            raise Refused('Rollback changed accepted fixture profiles/state')
        if self.request('/')[0] != 200 or self.request('/agora/api/challenge')[0] != 404:
            raise Refused('Rollback did not preserve ordinary site and close API')
        self.gate('injected first-failure rollback closes API/stops backend/preserves profiles/site200')
        return {'status': 'PASS', 'state_files_retained': len(before)}

    def receipt(self, status):
        result = {'status': status, 'scope': 'ISOLATED_NAMESPACE_ONLY_NOT_DEPLOYED',
                  'gates': self.gates, 'commands': self.run.records,
                  'observations': self.observer.observations,
                  'private_state_path': str(self.root / 'retained-state'),
                  'private_recovery_paths': list(map(str, self.keys))}
        (self.root / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
        os.chmod(self.root / 'result.json', 0o600)
        return result


def control(path, action):
    payload = {'action': action}
    if action in ('save-key', 'read-profile', 'after-browser'):
        payload['value'] = json.loads(sys.stdin.read())
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as connection:
        connection.connect(path)
        connection.sendall(json.dumps(payload).encode() + b'\n')
        connection.shutdown(socket.SHUT_WR)
        answer = b''
        while True:
            part = connection.recv(65536)
            if not part:
                break
            answer += part
    response = json.loads(answer)
    if not response['ok']:
        raise Refused(response['error'])
    print(json.dumps(response['result']))


def serve(args):
    fixture = None
    listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    try:
        fixture = Rehearsal(args)
        fixture.setup()
        listener.bind(str(fixture.root / 'control.sock'))
        os.chmod(fixture.root / 'control.sock', 0o600)
        listener.listen(1)
        print('ACTIVATION_FIXTURE_READY ' + str(fixture.root / 'control.sock'), flush=True)
        done = False
        while not done:
            connection, _ = listener.accept()
            with connection:
                raw = b''
                while True:
                    part = connection.recv(65536)
                    if not part:
                        break
                    raw += part
                    if len(raw) > 1048576:
                        raise Refused('Oversized private control request')
                request = json.loads(raw)
                action = request['action']
                if action == 'ready':
                    result = fixture.config
                elif action == 'pre-browser':
                    result = fixture.pre_browser()
                elif action == 'save-key':
                    result = fixture.save_key(request['value'])
                elif action == 'restart-backend':
                    result = fixture.restart()
                elif action == 'read-profile':
                    result = fixture.read_profile(request['value'])
                elif action == 'after-browser':
                    result = fixture.after_browser(request['value'])
                elif action == 'finish':
                    result = fixture.rollback_proof()
                    fixture.receipt('PASS')
                    done = True
                else:
                    raise Refused('Unknown private fixture action')
                connection.sendall(json.dumps({'ok': True, 'result': result}).encode())
    except Exception as error:
        if fixture is not None:
            try:
                fixture.fixture.rollback()
            except Exception as rollback_error:
                fixture.gates.append({'gate': 'fixture rollback unavailable',
                                      'status': 'FAIL',
                                      'error': type(rollback_error).__name__ + ': ' + str(rollback_error)})
            finally:
                fixture.gates.append({'gate': 'unexpected fixture failure',
                                      'status': 'FAIL', 'error': type(error).__name__ + ': ' + str(error)})
                fixture.receipt('FAIL')
        raise
    finally:
        listener.close()
        if fixture is not None:
            fixture.fixture.cleanup()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--root')
    parser.add_argument('--backend')
    parser.add_argument('--nginx')
    parser.add_argument('--native')
    parser.add_argument('--startup')
    parser.add_argument('--page')
    parser.add_argument('--client')
    parser.add_argument('--control')
    parser.add_argument('--action')
    args = parser.parse_args()
    if os.geteuid() != 0:
        raise Refused('Private fixture requires root')
    if args.control:
        control(args.control, args.action)
    else:
        if not all((args.root, args.backend, args.nginx, args.native, args.startup,
                    args.page, args.client)):
            parser.error('serve requires explicit fixture root, published page/client and pinned binaries')
        serve(args)


if __name__ == '__main__':
    main()
