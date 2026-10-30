#!/usr/bin/env python3
"""Read-only activation observers shared by isolated rehearsal and live gates.

These host tools are verification oracles, not the Omega registration backend.
No payload capture, response-body logging or implicit mutation retries.
"""
from contextlib import contextmanager
from datetime import datetime, timezone
import json
import hashlib
import os
from pathlib import Path
import re
import select
import subprocess
import stat
import time


class Refused(RuntimeError):
    pass


class Commands:
    def __init__(self):
        self.records = []

    def __call__(self, argv, input=None, timeout=30, check=True, private=False,
                 read_only=False):
        attempts = 3 if read_only and argv[0] == 'ssh' else 1
        for attempt in range(1, attempts + 1):
            record = {'command': [str(arg) for arg in argv], 'attempt': attempt,
                      'utc': datetime.now(timezone.utc).isoformat()}
            self.records.append(record)
            try:
                result = subprocess.run(argv, input=input, capture_output=True,
                                        timeout=timeout)
            except subprocess.TimeoutExpired:
                record['result'] = 'TIMEOUT'
                if attempt == attempts:
                    raise
                continue
            record['exit'] = result.returncode
            if not private:
                record['stdout'] = result.stdout.decode(errors='replace')
                record['stderr'] = result.stderr.decode(errors='replace')
            else:
                record['output'] = 'PRIVATE_NOT_RECORDED'
            if check and result.returncode:
                if attempt != attempts:
                    continue
                raise Refused('Command failed: ' + str(argv[:3]) +
                              ' exit=' + str(result.returncode))
            return result
        raise Refused('Read-only SSH attempts exhausted')


def properties(raw):
    result = {}
    for line in raw.splitlines():
        if not line:
            continue
        key, separator, value = line.partition('=')
        if not separator or key in result:
            raise Refused('Malformed or duplicate property: ' + key)
        result[key] = value
    return result


class Observers:
    def __init__(self, run, native, startup, prefix=(), sudo=('sudo', '-n')):
        self.run = run
        self.native = str(native)
        self.startup_native = str(startup)
        self.prefix = list(prefix)
        self.sudo = list(sudo)
        self.observations = []

    def command(self, argv):
        return self.prefix + list(argv)

    def text(self, argv, **kwargs):
        return self.run(argv, **kwargs).stdout.decode()

    def show(self, unit, *fields):
        argv = ['systemctl', 'show', unit]
        for field in fields:
            argv += ['-p', field]
        return self.text(argv)

    def native_check(self, mode, *args, statuses=(0,)):
        result = self.run([self.native, mode, *map(str, args)], check=False)
        if result.returncode not in statuses:
            raise Refused('Native ' + mode + ' refused: ' + str(result.returncode))
        return result.returncode

    def startup(self, unit, executable, endpoint):
        deadline = time.monotonic_ns() + 10000000000
        pid = int(properties(self.show(unit, 'MainPID'))['MainPID'])
        if pid <= 0:
            raise Refused('No positive initial MainPID')
        while time.monotonic_ns() < deadline:
            before = properties(self.show(unit, 'MainPID'))['MainPID']
            exe_before = self.text(self.sudo + ['readlink', '-f', f'/proc/{pid}/exe'])
            ps_before = self.text(['ps', '-p', str(pid), '-o', 'euid=,egid='])
            ss = self.text(self.command(self.sudo +
                          ['ss', '-H', '-ltnp', '( sport = :' + endpoint.rsplit(':', 1)[1] + ' )']))
            exe_after = self.text(self.sudo + ['readlink', '-f', f'/proc/{pid}/exe'])
            ps_after = self.text(['ps', '-p', str(pid), '-o', 'euid=,egid='])
            after = properties(self.show(unit, 'MainPID'))['MainPID']
            elapsed = 10000000000 - (deadline - time.monotonic_ns())
            row = {'pid': pid, 'before': before, 'exe_before': exe_before,
                   'ps_before': ps_before, 'listener': ss, 'exe_after': exe_after,
                   'ps_after': ps_after, 'after': after, 'elapsed_ns': elapsed}
            self.observations.append(row)
            status = self.native_check('snapshot', pid, before, exe_before,
                                      ps_before, ss, exe_after, ps_after, after,
                                      executable, 976, 964, endpoint, elapsed,
                                      10000000000, statuses=(0, 2))
            if time.monotonic_ns() >= deadline:
                raise Refused('Late startup observation')
            if status == 0:
                result = self.run([self.startup_native, str(deadline),
                                   '1', '976', '964', '1'], check=False)
                if result.returncode != 1 or time.monotonic_ns() >= deadline:
                    raise Refused('Startup policy did not return READY in time')
                if int(properties(self.show(unit, 'MainPID'))['MainPID']) != pid:
                    raise Refused('MainPID changed after readiness')
                return pid
            time.sleep(0.01)
        raise Refused('Startup deadline expired')

    def effective(self, config, pid, allowed_source=None):
        proxy = allowed_source or config['allowed_source']
        raw = self.show(config['unit'], 'MainPID', 'User', 'Group', 'FragmentPath',
                        'DropInPaths', 'IPAddressAllow', 'IPAddressDeny')
        actual = properties(raw)
        self.native_check('unit', raw, pid, 'aien-agora', 'aien-agora',
                          proxy + '/32', '127.0.0.1/32',
                          config.get('fragment', actual['FragmentPath']),
                          config.get('dropin', actual['DropInPaths']))
        state = self.show(config['unit'], 'ActiveState', 'SubState',
                          'IPAccounting', 'ControlGroup')
        group = '/system.slice/' + config['unit']
        self.native_check('state', state, group)
        for ancestor in ('-.slice', 'system.slice'):
            ancestors = self.text(['systemctl', 'show', '-p', 'IPAddressAllow',
                                   '-p', 'IPAddressDeny', '--', ancestor])
            self.native_check('ancestors', ancestors, 1)
        execution = self.show(config['unit'], 'ExecStart')
        self.native_check('exec', execution, config['executable'],
                          config['display_argv'], pid)
        actual_group = self.text(self.sudo + ['cat', f'/proc/{pid}/cgroup'])
        state_info = Path(config['state']).stat()
        if (not stat.S_ISDIR(state_info.st_mode) or state_info.st_uid != 976
                or state_info.st_gid != 964 or stat.S_IMODE(state_info.st_mode) != 0o750):
            raise Refused('State directory identity or permissions differ from the approved policy')
        activation = properties(self.show(config['unit'], 'TriggeredBy'))
        if activation.get('TriggeredBy') != '':
            raise Refused('Unexpected socket or other activation trigger')
        sockets = self.text(self.command(self.sudo + ['ss', '-H', '-ltnp']))
        owned_rows = [line for line in sockets.splitlines()
                      if re.search(r'\bpid=' + str(pid) + r',', line)]
        expected_endpoint = config['backend_host'] + ':' + str(config['backend_port'])
        if len(owned_rows) != 1 or owned_rows[0].split()[3] != expected_endpoint:
            raise Refused('Alternate or unexpected backend listener')
        self.observations.append({'state_directory_mode': '750',
                                  'state_uid': state_info.st_uid,
                                  'state_gid': state_info.st_gid,
                                  'owned_listener_count': len(owned_rows),
                                  'activation_triggers': ''})
        if '0::' + group not in actual_group.splitlines():
            raise Refused('Process does not belong to observed cgroup')
        if config.get('namespace'):
            actual_ns = self.text(self.sudo + ['stat', '-Lc', '%i',
                                               f'/proc/{pid}/ns/net']).strip()
            expected_ns = self.text(self.sudo + ['stat', '-Lc', '%i',
                           '/run/netns/' + config['namespace']]).strip()
            if int(actual_ns) != int(expected_ns):
                raise Refused('Backend not inside fixture network namespace')
        bpf = json.loads(self.text(self.sudo + ['bpftool', '-j', 'cgroup', 'show',
                                  '/sys/fs/cgroup' + group, 'effective']))
        if ({item['attach_type'] for item in bpf} !=
                {'cgroup_inet_ingress', 'cgroup_inet_egress'} or
                any(int(item['id']) <= 0 for item in bpf)):
            raise Refused('Effective ingress/egress BPF not proved')
        self.observations.append({'gate': 'effective', 'unit': raw, 'state': state,
                                  'exec': execution, 'bpf': bpf})
        journal = self.text(self.sudo + ['journalctl', '--unit', config['unit'],
                           '--since', config['journal_since'], '--no-pager', '-o', 'cat'],
                            private=True)
        if re.search(r'(?i)(not supported|unsupported|failed to attach|failed to apply).*(BPF|IP firewall)|'
                     r'(BPF|IP firewall).*(not supported|unsupported|failed to attach|failed to apply)', journal):
            raise Refused('Current invocation reports unsupported or failed IP filtering')
        self.observations.append({'current_invocation_filter_warning': 'NONE',
                                  'since': config['journal_since']})
        return bpf

    def copied_artifacts(self, entries):
        observed = []
        for destination, source, mode in entries:
            target = Path(destination)
            info = target.lstat()
            digest = hashlib.sha256(target.read_bytes()).hexdigest()
            expected = hashlib.sha256(Path(source).read_bytes()).hexdigest()
            if (not stat.S_ISREG(info.st_mode) or info.st_uid != 0 or info.st_gid != 0
                    or stat.S_IMODE(info.st_mode) != mode or digest != expected):
                raise Refused('Copied artifact bytes, identity or mode do not match the approved input')
            observed.append({'path': str(target), 'source': str(source),
                             'sha256': digest, 'uid': info.st_uid,
                             'gid': info.st_gid, 'mode': oct(mode)})
        self.observations.append({'copied_artifacts': observed})

    @contextmanager
    def capture(self, interface, expression):
        argv = self.command(self.sudo + ['timeout', '10', 'tcpdump', '-ni',
                     interface, '-nn', '-tt', '-S', '-l', expression])
        record = {'command': argv, 'kind': 'passive_packet_metadata'}
        self.run.records.append(record)
        proc = subprocess.Popen(argv, stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE, bufsize=0)
        error = b''
        deadline = time.monotonic_ns() + 10000000000
        try:
            while b'listening on ' + interface.encode() not in error:
                if proc.poll() is not None or time.monotonic_ns() >= deadline:
                    raise Refused('Capture not ready')
                ready, _, _ = select.select([proc.stderr], [], [], 0.1)
                if ready:
                    error += os.read(proc.stderr.fileno(), 8192)
            observation = {}
            yield observation
            out, rest = proc.communicate(timeout=12)
            error += rest
            record.update(exit=proc.returncode, stdout=out.decode(),
                          stderr=error.decode())
            if proc.returncode not in (0, 124):
                raise Refused('Passive capture failed')
            observation['packets'] = out.decode()
        finally:
            if proc.poll() is None:
                proc.terminate()
                out, rest = proc.communicate(timeout=5)
                record.update(exit=proc.returncode, stdout=out.decode(),
                              stderr=(error + rest).decode(), interrupted=True)

    def syn_probe(self, config, source, request, accept):
        target, port = config['backend_host'], config['backend_port']
        expression = (f'tcp port {port} and host {source} and '
                      '(tcp[tcpflags] & tcp-syn != 0)')
        with self.capture(config['interface'], expression) as observation:
            response = request()
        packets = []
        for match in re.finditer(r'IP (\d+\.\d+\.\d+\.\d+)\.(\d+) > '
                                 r'(\d+\.\d+\.\d+\.\d+)\.(\d+): '
                                 r'Flags \[([^\]]+)\]', observation['packets']):
            packets.append(match.groups())
        syn = [row for row in packets if row[0] == source and row[2] == target
               and int(row[3]) == port and 'S' in row[4] and '.' not in row[4]]
        ack = [row for row in packets if row[0] == target and int(row[1]) == port
               and row[2] == source and 'S' in row[4] and '.' in row[4]]
        if not syn or bool(ack) != accept:
            raise Refused('Source SYN/SYNACK proof disagrees with expected policy')
        if response.returncode != (0 if accept else 28):
            raise Refused('Source HTTP outcome disagrees with expected policy')
        if response.stdout.strip() != (b'200' if accept else b'000'):
            raise Refused('Unexpected challenge status')
        result = {'source': source, 'syn': len(syn), 'synack': len(ack),
                  'accept': accept}
        self.observations.append(result)
        return result

    def counted_get(self, config, request, expected=1):
        expression = (f'tcp dst port {config["backend_port"]} and src host '
                      f'{config["allowed_source"]} and '
                      'tcp[((tcp[12] & 0xf0)>>2):4] = 0x47455420')
        with self.capture(config['interface'], expression) as observation:
            result = request()
        identities = set()
        for match in re.finditer(r'IP (\S+) > (\S+): .*seq (\d+):\d+',
                                 observation['packets']):
            identities.add(match.groups())
        if callable(expected):
            expected = expected(result)
        if len(identities) != expected:
            raise Refused('Independent backend GET count: ' + str(len(identities)))
        self.observations.append({'backend_gets': len(identities)})
        return result

    def reload(self, config):
        args = config['nginx_args']
        self.run(self.command(self.sudo + args + ['-t']))
        master = int(config['nginx_pid'])
        def workers():
            rows = self.text(self.sudo + ['ps', '--ppid', str(master), '-o',
                                         'pid=,args='], check=False)
            result = {}
            for row in rows.splitlines():
                pid, title = row.strip().split(maxsplit=1)
                if 'nginx: worker process' in title:
                    result[int(pid)] = 'shutting down' in title
            return result
        def listeners():
            raw = self.text(self.command(self.sudo + ['ss', '-H', '-ltnp']))
            return {int(pid) for pid in re.findall(r'\bpid=(\d+),', raw)}
        before = workers()
        old_active = {pid for pid, draining in before.items() if not draining}
        if not old_active or any(pid in listeners() for pid, draining in
                                 before.items() if draining):
            raise Refused('Initial nginx worker/listener state invalid')
        self.run(self.command(self.sudo + args + ['-s', 'reload']))
        deadline = time.monotonic_ns() + 30000000000
        while time.monotonic_ns() < deadline:
            current = workers()
            listening = listeners()
            new = {pid for pid, draining in current.items()
                   if not draining and pid not in before}
            ready = new and all(pid not in current or
                               (current[pid] and pid not in listening)
                               for pid in old_active)
            if ready and time.monotonic_ns() < deadline:
                self.observations.append({'reload_before': before,
                    'reload_after': current, 'new_active': sorted(new),
                    'listening_pids': sorted(listening)})
                return
            time.sleep(0.01)
        raise Refused('Nginx worker adoption deadline expired')

    def effective_logging(self, config):
        raw = self.text(self.command(self.sudo + config['nginx_args'] + ['-T']),
                        private=True)
        sinks = set()
        access = set()
        temps = set()
        for kind, arguments in re.findall(
                r'(?m)^\s*(access_log|error_log|[a-z_]+_temp_path)\s+([^;\n]+);', raw):
            path = arguments.split()[0]
            if kind.endswith('_temp_path'):
                temps.add(path)
            elif path not in ('off', '/dev/null'):
                if not path.startswith('/') or '$' in path:
                    raise Refused('Effective log sink cannot be directly inspected')
                sinks.add(path)
                if kind == 'access_log':
                    access.add(path)
        if sinks != set(config['logs']) or temps != set(config['temp_dirs']):
            raise Refused('Declared log/temp paths differ from effective nginx configuration')
        config['access_logs'] = sorted(access)
        self.observations.append({'effective_log_sinks': sorted(sinks),
                                  'effective_temp_dirs': sorted(temps)})

    def journal_privacy(self, config, markers):
        units = [config['unit'], *config.get('routing_units', [])]
        argv = self.sudo + ['journalctl']
        for unit in units:
            argv += ['--unit', unit]
        result = self.run(argv + ['--since', config['journal_since'],
            '--no-pager', '-o', 'cat'], private=True)
        if any(marker in result.stdout for marker in markers):
            raise Refused('Privacy marker found in fixture service journal')
        self.observations.append({'journal_privacy': 'PASS', 'units': units,
                                  'since': config['journal_since'],
                                  'markers': len(markers)})

    def privacy(self, logs, temp_dirs, markers):
        paths = list(logs)
        for directory in temp_dirs:
            result = self.run(self.sudo + ['find', str(directory), '-type', 'f',
                                          '-print0'])
            paths += [name.decode() for name in result.stdout.split(b'\0') if name]
        if not paths:
            raise Refused('No effective log sinks observed')
        result = self.run(self.sudo + ['grep', '-F', '-l', '-f', '-', '--', *paths],
                          input=b'\n'.join(markers) + b'\n', check=False,
                          private=True)
        if result.returncode != 1:
            raise Refused('Privacy marker found or effective sink unreadable')
        self.observations.append({'privacy': 'PASS', 'log_sinks': list(logs),
                                  'temp_dirs': list(map(str, temp_dirs)),
                                  'markers': len(markers)})


def save_private(path, value):
    target = Path(path)
    descriptor = os.open(target, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        os.write(descriptor, value.encode())
        os.fsync(descriptor)
    finally:
        os.close(descriptor)
    if target.stat().st_mode & 0o077:
        raise Refused('Recovery material permissions not private')
