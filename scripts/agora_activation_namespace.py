#!/usr/bin/env python3
"""Owned, disposable network/systemd infrastructure for the activation oracle.

This is verification infrastructure, not an alternate registration implementation.
Only the copied native backend answers API requests. Call setup/rollback/cleanup
under root; rollback deliberately retains all profile state until cleanup.
"""

import argparse
import ctypes
import ipaddress
import os
from pathlib import Path
import pwd
import grp
import selectors
import shutil
import socket
import sys
import threading
import time
import uuid


REPOSITORY = Path(__file__).resolve().parents[1]


class Fixture:
    """A single owner of a namespace, backend unit, nginx and fixed tunnels.

    root is the caller's scratch/evidence directory, not the repository. The
    callback runs argv without a shell and returns CompletedProcess with bytes.
    setup returns observations/configuration, never an acceptance verdict.
    """

    def __init__(self, root: Path, backend: Path, nginx: Path, run, page: Path, client: Path):
        self.root = Path(root).resolve()
        self.backend = Path(backend).resolve()
        self.nginx = Path(nginx).resolve()
        self.page = Path(page).resolve()
        self.client = Path(client).resolve()
        self.run = run
        self.ident = uuid.uuid4().hex
        self.name = 'aien-agora-oracle-' + self.ident
        self.namespace = self.name
        self.unit = self.name + '.service'
        # /run is noexec on Spark; only this uniquely owned fixture directory
        # lives on the executable root filesystem. Never use /opt/aien/bin.
        self.runtime = Path('/opt') / self.name
        self.state = Path('/var/lib') / self.name
        self.unit_path = Path('/run/systemd/system') / self.unit
        self.dropin_dir = self.unit_path.with_name(self.unit + '.d')
        self.dropin = self.dropin_dir / 'policy.conf'
        self.config = {}
        self._namespace_created = False
        self._runtime_created = False
        self._state_created = False
        self._unit_created = False
        self._runtime_enabled = False
        self._state_retained = False
        self._routing_units = []
        self._ports = []
        self._setup = False
        self._closed = False

    def command(self, argv):
        """Prefix commands whose sockets/listener observations belong in netns."""
        return ['ip', 'netns', 'exec', self.namespace, *map(str, argv)]

    def _call(self, argv, **kwargs):
        return self.run(list(map(str, argv)), **kwargs)

    def _write(self, path, text, mode=0o644):
        path.write_text(text)
        path.chmod(mode)

    def _reserve_host_port(self):
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.bind(('127.0.0.1', 0))
        self._ports.append(sock)
        return sock.getsockname()[1]

    def _release_port(self, port):
        for sock in self._ports[:]:
            if sock.getsockname()[1] == port:
                sock.close()
                self._ports.remove(sock)

    def setup(self):
        if self._setup or self._closed:
            raise RuntimeError('Fixture setup is single use')
        if os.geteuid() != 0:
            raise PermissionError('Fixture must be hosted by the root oracle process')
        if pwd.getpwnam('aien-agora').pw_uid != 976:
            raise RuntimeError('Unexpected aien-agora UID')
        if grp.getgrnam('aien-agora').gr_gid != 964:
            raise RuntimeError('Unexpected aien-agora GID')
        # Every mutable or executable service path is outside host /tmp, which
        # PrivateTmp hides. No production state, env, unit or nginx file is used.
        try:
            self.runtime.mkdir(mode=0o755)
            self._runtime_created = True
            self.state.mkdir(mode=0o750)
            self._state_created = True
            os.chown(self.state, 976, 964)
            executable = self.runtime / 'agora'
            shutil.copyfile(self.backend, executable)
            executable.chmod(0o755)
            nginx = self.runtime / 'nginx'
            shutil.copyfile(self.nginx, nginx)
            nginx.chmod(0o755)
            module = self.nginx.parent.parent / 'lib/nginx/modules/ngx_stream_module.so'
            shutil.copyfile(module, self.runtime / module.name)
            shutil.copyfile(Path(__file__).resolve(), self.runtime / 'tunnel.py')
            self._provision_site()
            self._call(['ip', 'netns', 'add', self.namespace])
            self._namespace_created = True
            self._call(self.command(['ip', 'link', 'set', 'lo', 'up']))
            # These high ports exist only inside this fixture's isolated netns.
            backend_port, frontend_port, tls_port, plain_port, normal_port = (
                28132, 28443, 28444, 28080, 28445)
            host_tls = self._reserve_host_port()
            host_plain = self._reserve_host_port()
            display_argv = (f'{executable} serve --state {self.state} '
                            f'--listen 127.0.0.1 --port {backend_port} --bits 20')
            self._write(self.runtime / 'runtime.env',
                        'AGORA_LISTEN=127.0.0.1\n', 0o600)
            template = (REPOSITORY / 'om/agora/deploy/aien-agora.service').read_text()
            template = template.replace('StateDirectory=aien-agora',
                                        'StateDirectory=' + self.name)
            template = template.replace('/var/lib/aien-agora', str(self.state))
            template = template.replace('/etc/aien-agora/runtime.env',
                                        str(self.runtime / 'runtime.env'))
            template = template.replace(
                '/opt/aien/bin/agora serve --state ' + str(self.state)
                + ' --listen ${AGORA_LISTEN} --port 18132 --bits 20', display_argv)
            template = template.replace('Restart=always', 'Restart=no')
            template = template.replace('[Service]\n',
                '[Service]\nNetworkNamespacePath=/run/netns/' + self.namespace + '\n')
            if '/opt/aien/' in template or '--port 18132' in template:
                raise RuntimeError('Backend template relocation was incomplete')
            # Exclusive paths ensure cleanup cannot acquire another owner's unit.
            with self.unit_path.open('x') as stream:
                stream.write(template)
            self._unit_created = True
            self.dropin_dir.mkdir(mode=0o755)
            self._write_policy(['127.0.0.1/32', '127.0.0.2/32'])
            nginx_config = self._provision_nginx(backend_port, frontend_port,
                                                tls_port, plain_port, normal_port)
            nginx_args = [str(nginx), '-p', str(self.runtime) + '/',
                          '-c', str(nginx_config)]
            self.config = {
                'namespace': self.namespace, 'unit': self.unit,
                'state': str(self.state), 'executable': str(executable),
                'display_argv': display_argv,
                'fragment': str(self.unit_path), 'dropin': str(self.dropin),
                'backend_host': '127.0.0.1', 'backend_port': backend_port,
                'allowed_source': '127.0.0.2', 'denied_source': '127.0.0.3',
                'interface': 'lo', 'uid': 976, 'gid': 964,
                'tls_url': f'https://www.aienos.com:{host_tls}',
                'browser_url': f'https://www.aienos.com:{host_tls}/aienet/',
                'plain_url': f'http://127.0.0.1:{host_plain}',
                'browser_host': 'www.aienos.com', 'browser_address': '127.0.0.1',
                'curl_resolve': f'www.aienos.com:{host_tls}:127.0.0.1',
                'namespace_prefix': self.command([]),
                'host_tls_port': host_tls, 'host_plain_port': host_plain,
                'frontend_host': '127.0.0.1', 'frontend_port': frontend_port,
                'proxy_tls_port': tls_port, 'plain_port': plain_port,
                'normal_tls_port': normal_port,
                'normal_site_host': 'normal.aienet.invalid',
                'normal_site_path': '/normal-site-control.txt',
                'nginx_config': str(nginx_config), 'nginx_args': nginx_args,
                'nginx_pid_file': str(self.runtime / 'nginx.pid'),
                'certificate': str(self.runtime / 'cert.pem'),
                'logs': [str(self.runtime / 'access.log'),
                         str(self.runtime / 'error.log')],
                'temp_dirs': [str(self.runtime / name) for name in
                              ('body', 'proxy', 'fastcgi', 'uwsgi', 'scgi')],
                'runtime': str(self.runtime), 'routing_units': [],
                'copied_artifacts': [
                    (str(executable), str(self.backend), 0o755),
                    (str(self.runtime / 'public/aienet/index.html'), str(self.page), 0o644),
                    (str(self.runtime / 'public/js/aienet.js'), str(self.client), 0o644)],
            }
            self._call(['systemctl', 'daemon-reload'])
            self.start_backend()
            self.start_routing()
            self._setup = True
            return self.config
        except BaseException:
            self.cleanup()
            raise

    def _provision_site(self):
        public = self.runtime / 'public'
        public.mkdir(mode=0o755)
        for name in ('assets', 'js', 'vendor', 'wasm'):
            source = REPOSITORY / 'public' / name
            if source.is_dir():
                shutil.copytree(source, public / name)
        for source in (REPOSITORY / 'public').iterdir():
            if source.is_file() and source.suffix in ('.svg', '.ico', '.png'):
                shutil.copyfile(source, public / source.name)
        (public / 'aienet').mkdir()
        shutil.copyfile(self.page, public / 'aienet/index.html')
        shutil.copyfile(self.client, public / 'js/aienet.js')
        self._write(public / 'normal-site-control.txt', 'activation fixture normal site\n')
        self._write(public / 'index.html',
                    '<!doctype html><title>Fixture normal site</title>'
                    '<p>activation fixture normal site</p>\n')
        # Readable assets are separate from root-only scratch and private state.
        for directory, subdirs, files in os.walk(public):
            Path(directory).chmod(0o755)
            for name in files:
                (Path(directory) / name).chmod(0o644)
        self._call(['openssl', 'req', '-x509', '-newkey', 'rsa:2048', '-nodes',
                    '-keyout', self.runtime / 'key.pem',
                    '-out', self.runtime / 'cert.pem', '-days', '1',
                    '-subj', '/CN=www.aienos.com', '-addext',
                    'subjectAltName=DNS:www.aienos.com,DNS:normal.aienet.invalid'])
        (self.runtime / 'key.pem').chmod(0o600)

    def _provision_nginx(self, backend, frontend, tls, plain, normal):
        deploy = REPOSITORY / 'om/agora/deploy'
        upstream = self.runtime / 'upstream.conf'
        self._write(upstream, f'server 127.0.0.1:{backend};\n')
        http = (deploy / 'nginx-agora-http.conf').read_text().replace(
            '/etc/nginx/snippets/agora-upstream.conf', str(upstream))
        routes = (deploy / 'nginx-agora.conf').read_text()
        # Preserve the reviewed route template; outgoing socket identity is a
        # server-level directive inherited by every nested registration route.
        for name in ('body', 'proxy', 'fastcgi', 'uwsgi', 'scgi'):
            path = self.runtime / name
            path.mkdir(mode=0o700)
            os.chown(path, 976, 964)
        for name in ('access.log', 'error.log'):
            self._write(self.runtime / name, '', 0o600)
        path = self.runtime / 'nginx.conf'
        self._write(path, f'''load_module {self.runtime}/ngx_stream_module.so;
user aien-agora aien-agora;
pid {self.runtime}/nginx.pid;
lock_file {self.runtime}/nginx.lock;
error_log {self.runtime}/error.log notice;
worker_processes 1;
events {{ worker_connections 256; }}
stream {{
    map $ssl_preread_server_name $fixture_sni {{
        www.aienos.com 127.0.0.1:{tls};
        default 127.0.0.1:{normal};
    }}
    server {{
        listen 127.0.0.1:{frontend};
        ssl_preread on;
        proxy_protocol on;
        proxy_pass $fixture_sni;
    }}
}}
http {{
    default_type application/octet-stream;
    types {{ text/html html; text/css css; application/javascript js;
             image/svg+xml svg; application/wasm wasm; }}
    log_format fixture '$remote_addr $request_method $uri $status';
    access_log {self.runtime}/access.log fixture;
    client_body_temp_path {self.runtime}/body;
    proxy_temp_path {self.runtime}/proxy;
    fastcgi_temp_path {self.runtime}/fastcgi;
    uwsgi_temp_path {self.runtime}/uwsgi;
    scgi_temp_path {self.runtime}/scgi;
    ssl_certificate {self.runtime}/cert.pem;
    ssl_certificate_key {self.runtime}/key.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    {http}
    server {{
        listen 127.0.0.1:{tls} ssl proxy_protocol;
        listen 127.0.0.1:{plain};
        server_name www.aienos.com;
        root {self.runtime}/public;
        index index.html;
        proxy_bind 127.0.0.2;
        {routes}
        location / {{ try_files $uri $uri/ =404; }}
    }}
    server {{
        listen 127.0.0.1:{normal} ssl proxy_protocol;
        server_name normal.aienet.invalid;
        root {self.runtime}/public;
        location / {{ try_files $uri =404; }}
    }}
}}
''')
        baseline = path.read_text().replace(routes, '''
        location ^~ /agora/api/ {
            access_log off;
            error_log /dev/null crit;
            add_header Cache-Control "no-store" always;
            return 404;
        }
        location = /agora/api {
            access_log off;
            error_log /dev/null crit;
            return 404;
        }
''').replace(http, '')
        self._write(self.runtime / 'nginx-baseline.conf', baseline)
        return path

    def _write_policy(self, allow):
        allowed = []
        permitted = {'127.0.0.1/32', '127.0.0.2/32', '127.0.0.3/32'}
        for address in allow:
            address = str(ipaddress.IPv4Network(address, strict=True))
            if address not in permitted or address in allowed:
                raise ValueError('Only distinct fixture source host prefixes are allowed')
            allowed.append(address)
        self._write(self.dropin, '[Service]\nIPAddressAllow=\n'
                    + ''.join('IPAddressAllow=' + address + '\n' for address in allowed)
                    + 'IPAddressDeny=\nIPAddressDeny=any\n')

    def set_policy(self, allow):
        """Real dedicated-unit policy change; callers label control phases."""
        self._require_open()
        self._write_policy(allow)
        self._call(['systemctl', 'daemon-reload'])
        self.restart_backend()

    def _require_open(self):
        if not self._unit_created or self._closed:
            raise RuntimeError('Fixture unit is not provisioned')

    def start_backend(self):
        self._require_open()
        self.config['journal_since'] = time.strftime('%Y-%m-%d %H:%M:%S UTC', time.gmtime())
        self._call(['systemctl', 'start', self.unit])
        self._refresh_backend_identity()

    def restart_backend(self):
        self._require_open()
        self.config['journal_since'] = time.strftime('%Y-%m-%d %H:%M:%S UTC', time.gmtime())
        self._call(['systemctl', 'restart', self.unit])
        self._refresh_backend_identity()

    def _refresh_backend_identity(self):
        result = self._call(['systemctl', 'show', self.unit, '-p', 'MainPID',
                             '-p', 'ControlGroup'])
        fields = dict(line.split('=', 1) for line in result.stdout.decode().splitlines()
                      if '=' in line)
        pid = int(fields.get('MainPID', '0'))
        if pid <= 0:
            raise RuntimeError('Fixture backend has no MainPID; readiness is not proven')
        self.config['backend_pid'] = pid
        self.config['backend_cgroup'] = fields['ControlGroup']

    def _start_routing_unit(self, suffix, argv, namespace=False):
        unit = self.name + '-' + suffix + '.service'
        self._routing_units.append(unit)
        args = ['systemd-run', '--quiet', '--unit=' + unit, '--collect',
                '--service-type=exec', '--property=Restart=no',
                '--property=StandardOutput=null', '--property=StandardError=null']
        if namespace:
            args.append('--property=NetworkNamespacePath=/run/netns/' + self.namespace)
        self._call(args + ['--', *map(str, argv)])
        return unit

    def start_routing(self):
        if self._routing_units or self._closed:
            raise RuntimeError('Routing already started or fixture closed')
        nginx_unit = self._start_routing_unit(
            'nginx', self.config['nginx_args'] + ['-g', 'daemon off;'], namespace=True)
        # Wait only for our nginx pidfile, not a backend readiness verdict.
        pidfile = Path(self.config['nginx_pid_file'])
        deadline = time.monotonic() + 10
        while not pidfile.exists():
            if time.monotonic() >= deadline:
                raise RuntimeError('Fixture nginx did not create its pidfile')
            time.sleep(0.05)
        self.config['nginx_pid'] = int(pidfile.read_text().strip())
        actual = self._call(['systemctl', 'show', nginx_unit, '-p', 'MainPID',
                             '--value']).stdout.strip()
        if int(actual) != self.config['nginx_pid']:
            raise RuntimeError('Fixture nginx pidfile does not match its unit MainPID')
        self.config['tunnel_pids'] = {}
        for suffix, host_key, destination in (
                ('tls', 'host_tls_port', self.config['frontend_port']),
                ('plain', 'host_plain_port', self.config['plain_port'])):
            port = self.config[host_key]
            self._release_port(port)
            tunnel_unit = self._start_routing_unit(
                suffix, [sys.executable, self.runtime / 'tunnel.py',
                         '--tunnel', '--namespace', self.namespace,
                         '--listen-port', str(port), '--destination-port', str(destination)])
            # A readiness file is written only after bind. Match its PID to the
            # dedicated unit rather than accepting any other host port listener.
            ready = self.runtime / f'tunnel-{port}.ready'
            deadline = time.monotonic() + 10
            while not ready.exists():
                if time.monotonic() >= deadline:
                    raise RuntimeError('Fixture host tunnel did not bind')
                time.sleep(0.05)
            pid = int(ready.read_text())
            actual = self._call(['systemctl', 'show', tunnel_unit, '-p', 'MainPID',
                                 '--value']).stdout.strip()
            if pid <= 0 or int(actual) != pid:
                raise RuntimeError('Fixture tunnel readiness does not match its MainPID')
            self.config['tunnel_pids'][suffix] = pid
        self.config['routing_units'] = self._routing_units.copy()

    def enable_backend(self):
        """Exercise enable-last without leaving a persistent boot target."""
        self._require_open()
        self._runtime_enabled = True
        self._call(['systemctl', 'enable', '--runtime', self.unit])

    def _disable_backend(self):
        if self._runtime_enabled:
            self._call(['systemctl', 'disable', '--runtime', self.unit])
            self._runtime_enabled = False

    def rollback(self):
        """Remove registration, keep ordinary site/tunnels and preserve profiles."""
        self._require_open()
        active = Path(self.config['nginx_config'])
        baseline = self.runtime / 'nginx-baseline.conf'
        # Both candidates were constructed from the same reviewed input.
        active.write_bytes(baseline.read_bytes())
        master = self.config['nginx_pid']
        children = Path(f'/proc/{master}/task/{master}/children')
        old_workers = children.read_text().split()
        self._call(self.command(self.config['nginx_args'] + ['-s', 'reload']))
        deadline = time.monotonic() + 15
        while any(Path('/proc', pid).exists() for pid in old_workers):
            if time.monotonic() >= deadline:
                raise RuntimeError('Fixture rollback workers did not drain')
            time.sleep(0.05)
        self._call(['systemctl', 'stop', self.unit])
        self._disable_backend()

    def _stop_owned_units(self):
        errors = []
        for unit in reversed(self._routing_units):
            result = self._call(['systemctl', 'stop', unit], check=False)
            if result.returncode:
                # --collect may already have unloaded a failed transient unit.
                state = self._call(['systemctl', 'show', unit, '-p', 'LoadState',
                                    '--value'], check=False)
                if state.stdout.strip() != b'not-found':
                    errors.append(unit)
        if self._unit_created:
            result = self._call(['systemctl', 'stop', self.unit], check=False)
            if result.returncode:
                errors.append(self.unit)
        if errors:
            raise RuntimeError('Could not stop owned fixture units: ' + ', '.join(errors))

    def cleanup(self):
        if self._closed:
            return
        # Never unlink files or namespace while an owned service may still run.
        self._stop_owned_units()
        self._disable_backend()
        for sock in self._ports:
            sock.close()
        self._ports.clear()
        if self._namespace_created:
            self._call(['ip', 'netns', 'delete', self.namespace])
            self._namespace_created = False
        if self._unit_created:
            if self.dropin_dir.exists():
                shutil.rmtree(self.dropin_dir)
            self.unit_path.unlink()
            self._unit_created = False
            self._call(['systemctl', 'daemon-reload'])
            self._call(['systemctl', 'reset-failed', self.unit], check=False)
        if self._state_created:
            if not self._state_retained:
                retained = self.root / 'retained-state'
                shutil.copytree(self.state, retained)
                for directory, _, files in os.walk(retained):
                    Path(directory).chmod(0o700)
                    for name in files:
                        (Path(directory) / name).chmod(0o600)
                self._state_retained = True
            shutil.rmtree(self.state)
            self._state_created = False
        if self._runtime_created:
            shutil.rmtree(self.runtime)
            self._runtime_created = False
        self._closed = True


def _relay(client, namespace_fd, destination_port):
    """A byte relay to one compile-time-selected loopback fixture destination."""
    remote = None
    try:
        # setns affects this worker's OS thread, not the host acceptor thread.
        # Use libc because supported Python versions do not all expose os.setns.
        libc = ctypes.CDLL(None, use_errno=True)
        if libc.setns(namespace_fd, 0) != 0:
            raise OSError(ctypes.get_errno(), 'Could not enter fixture namespace')
        remote = socket.create_connection(('127.0.0.1', destination_port), timeout=10)
        client.settimeout(30)
        remote.settimeout(30)
        with selectors.DefaultSelector() as ready:
            ready.register(client, selectors.EVENT_READ, remote)
            ready.register(remote, selectors.EVENT_READ, client)
            while ready.get_map():
                events = ready.select(timeout=30)
                if not events:
                    return
                for key, _ in events:
                    data = key.fileobj.recv(65536)
                    if not data:
                        ready.unregister(key.fileobj)
                        key.data.shutdown(socket.SHUT_WR)
                    else:
                        key.data.sendall(data)
    except OSError:
        # Neither payloads nor connection exceptions are logged.
        pass
    finally:
        client.close()
        if remote is not None:
            remote.close()


def _tunnel_main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--tunnel', action='store_true', required=True)
    parser.add_argument('--namespace', required=True)
    parser.add_argument('--listen-port', required=True, type=int)
    parser.add_argument('--destination-port', required=True, type=int)
    args = parser.parse_args()
    if not args.namespace.startswith('aien-agora-oracle-') or not all(
            char in '0123456789abcdef' for char in args.namespace.removeprefix(
                'aien-agora-oracle-')):
        parser.error('Not a fixture namespace')
    if args.destination_port not in (28443, 28080):
        parser.error('Destination is not an allowed fixture frontend')
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        listener.bind(('127.0.0.1', args.listen_port))
        listener.listen(32)
        namespace_fd = os.open('/run/netns/' + args.namespace, os.O_RDONLY)
        ready = Path(__file__).resolve().parent / f'tunnel-{args.listen_port}.ready'
        with ready.open('x') as stream:
            stream.write(str(os.getpid()))
        try:
            while True:
                client, _ = listener.accept()
                threading.Thread(target=_relay, args=(client, namespace_fd,
                                 args.destination_port), daemon=True).start()
        finally:
            os.close(namespace_fd)


if __name__ == '__main__':
    _tunnel_main()
