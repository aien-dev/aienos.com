"""Live tests for allen_memory against cortex-visitors (127.0.0.1:18096).

Run (engineering token injected by atlas-vault, never printed; visitor token read with sudo):
  atlas-vault run CORTEX_TOKEN -- nice -n 19 python3 -m unittest -v test_memory
The engineering Cortex (127.0.0.1:18080) is only read (GET search/get), never written.
"""
import json
import os
import secrets
import socket
import subprocess
import sys
import time
import unittest
import urllib.error
import urllib.parse
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import allen_memory  # noqa: E402
from allen_memory import Memory  # noqa: E402

VIS = 'http://127.0.0.1:18096'
ENG = 'http://127.0.0.1:18080'
ENG_SPACES = ['atlas-memory', 'aien-lessons', 'aien-sources']


def _visitor_token():
    r = subprocess.run(['sudo', '-n', 'sed', '-n', 's/^CORTEX_TOKEN=//p', '/etc/cortex-visitors/env'],
                       capture_output=True, text=True)
    return r.stdout.strip()


VTOK = _visitor_token()
ETOK = os.environ.get('CORTEX_TOKEN', '')
_NOPROXY = urllib.request.build_opener(urllib.request.ProxyHandler({}))


def http(method, base, path, token, body=None):
    req = urllib.request.Request(base + path, method=method,
                                 data=None if body is None else json.dumps(body).encode(),
                                 headers={'Authorization': 'Bearer ' + token, 'Content-Type': 'application/json'})
    try:
        with _NOPROXY.open(req, timeout=5) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, None


def search(base, token, q, space):
    return http('GET', base, '/api/cortex/search?' + urllib.parse.urlencode(
        {'q': q, 'space': space, 'limit': 50}), token)


def vid():
    return secrets.token_hex(32)


def db_rows_with(marker):
    """Count rows in the visitors DB whose content still holds marker (read-only, as the owner)."""
    r = subprocess.run(['sudo', '-n', '-u', 'cortex-visitors', '/usr/bin/sqlite3', '-readonly',
                        '/var/lib/cortex-visitors/cortex.db',
                        "SELECT count(*) FROM entities WHERE instr(content, '%s') > 0;" % marker],
                       capture_output=True, text=True)
    assert r.returncode == 0, r.stderr
    return int(r.stdout.strip())


class Base(unittest.TestCase):
    def setUp(self):
        self.assertEqual(len(VTOK), 64, 'visitor token not readable')
        self.m = Memory(VIS, VTOK)
        self.assertTrue(self.m.enabled)
        self.vids = []

    def new(self):
        v = vid()
        self.vids.append(v)
        return v

    def tearDown(self):
        for v in self.vids:
            self.m.forget(v)


class RoundTrip(Base):
    def test_roundtrip_and_cross_visitor(self):
        a, b = self.new(), self.new()
        self.assertEqual(self.m.recall(a), [])
        self.assertEqual(self.m.remember(a, ['Builds a ray tracer in Omega', 'Lives near Austin']), 2)
        got = self.m.recall(a)
        self.assertEqual(sorted(got), ['Builds a ray tracer in Omega', 'Lives near Austin'])
        # B sees nothing of A; after B writes, neither sees the other's entries.
        self.assertEqual(self.m.recall(b), [])
        self.assertEqual(self.m.remember(b, ['Prefers Rust examples']), 1)
        self.assertEqual(self.m.recall(b), ['Prefers Rust examples'])
        self.assertNotIn('Prefers Rust examples', self.m.recall(a))
        self.assertEqual(len(self.m.recall(a)), 2)

    def test_limit(self):
        a = self.new()
        self.m.remember(a, ['alpha %s' % secrets.token_hex(12) for _ in range(5)])
        self.assertEqual(len(self.m.recall(a, limit=3)), 3)
        self.assertEqual(len(self.m.recall(a)), 5)

    def test_client_side_filter_rejects_foreign_rows(self):
        # A row placed in A's space without A's tag (as a buggy/hostile writer would) is not returned.
        a, b = self.new(), self.new()
        sp = 'v-' + a
        for alias, meta in ((['vid:' + b], {'vid': b}), (['vid:' + a], {'vid': b}), ([], {'vid': a})):
            s, _ = http('POST', VIS, '/api/cortex/write', VTOK, {'kind': 'entity', 'value': {
                'space': sp, 'entityType': 'visitor_note', 'canonicalName': 'm-' + secrets.token_hex(16),
                'content': 'foreign row', 'aliases': alias, 'metadata': meta}})
            self.assertEqual(s, 201)
        s, r = http('POST', VIS, '/api/cortex/search', VTOK, {'q': '', 'space': sp, 'limit': 50})
        self.assertEqual(len(r['results']), 3)          # control: the server returns them
        self.assertEqual(self.m.recall(a), [])          # the client drops all three
        self.m.remember(a, ['own row'])
        self.assertEqual(self.m.recall(a), ['own row'])
        # cleanup of the foreign rows: same wipe-then-retract the module uses
        for e in r['results']:
            self.m._wipe(a, {'id': e['id'], 'name': e['canonicalName']})
        self.assertEqual(db_rows_with('foreign row'), 0)


class Forget(Base):
    def test_forget_deletes_all_and_spares_others(self):
        a, b = self.new(), self.new()
        mark = 'zqforget' + secrets.token_hex(8)
        self.assertEqual(self.m.remember(a, ['%s one' % mark, '%s two xyz' % mark, 'third thing']), 3)
        self.m.remember(b, ['B keeps this'])
        self.assertEqual(db_rows_with(mark), 2)         # control: text is in the DB before forget
        self.assertEqual(self.m.forget(a), 3)
        self.assertEqual(self.m.recall(a), [])
        s, r = http('POST', VIS, '/api/cortex/search', VTOK, {'q': '', 'space': 'v-' + a, 'limit': 50})
        self.assertEqual((s, r['results']), (200, []))
        s, r = search(VIS, VTOK, mark, 'v-' + a)
        self.assertEqual((s, r['results']), (200, []))
        self.assertEqual(db_rows_with(mark), 0)         # content overwritten, not only hidden
        self.assertEqual(self.m.recall(b), ['B keeps this'])
        self.assertEqual(self.m.forget(a), 0)           # nothing left
        # A can be remembered again after forget
        self.m.remember(a, ['%s again' % mark])
        self.assertEqual(self.m.recall(a), ['%s again' % mark])


class Dedupe(Base):
    def test_near_identical_updates(self):
        a = self.new()
        self.m.remember(a, ['Works on a compiler for Omega in Texas'])
        self.m.remember(a, ['works on a compiler for omega, in Texas.', 'Works on  a compiler for Omega in Texas!'])
        self.assertEqual(len(self.m.recall(a)), 1)
        self.m.remember(a, ['Works on the compiler for Omega in Texas'])
        got = self.m.recall(a)
        self.assertEqual(got, ['Works on the compiler for Omega in Texas'])  # updated, not piled up
        # within one batch too
        self.m.remember(a, ['Has two cats named Ada and Bob', 'has two cats named ada and bob'])
        self.assertEqual(len(self.m.recall(a)), 2)
        # negative control: different facts are both kept
        self.m.remember(a, ['Plays chess on weekends'])
        self.assertEqual(len(self.m.recall(a)), 3)

    def test_text_is_trimmed_and_capped(self):
        a = self.new()
        self.m.remember(a, ['  spaced \n\t out  ', 'x' * 1000, '', '   ', 7, None])
        got = self.m.recall(a)
        self.assertIn('spaced out', got)
        self.assertIn('x' * allen_memory.MAX_CHARS, got)
        self.assertEqual(len(got), 2)


class Cap(Base):
    def test_cap_drops_oldest(self):
        a = self.new()
        facts = ['fact %02d %s' % (i, secrets.token_hex(12)) for i in range(45)]
        for i in range(0, 40, 10):
            self.m.remember(a, facts[i:i + 10])
            time.sleep(0.01)
        got = self.m.recall(a, limit=40)
        self.assertEqual(len(got), 40)                  # control: at the cap nothing is dropped
        self.assertIn(facts[0], got)
        self.m.remember(a, facts[40:45])
        got = self.m.recall(a, limit=40)
        self.assertEqual(len(got), 40)
        self.assertEqual(set(got), set(facts[5:45]))   # the five oldest went
        self.assertIn(got[0], facts[40:45])            # newest first
        s, r = http('POST', VIS, '/api/cortex/search', VTOK, {'q': '', 'space': 'v-' + a, 'limit': 50})
        self.assertEqual(len(r['results']), 40)


class Rejects(unittest.TestCase):
    def test_bad_vids(self):
        m = Memory(VIS, VTOK)
        good = vid()
        bads = [good.upper(), good[:-1], good + '0', good[:-1] + 'g', ' ' + good[1:], good[:-1] + '\n',
                None, 123, b'\x00' * 64, '../' + good[3:], '']
        for bad in bads:
            self.assertEqual(m.recall(bad), [])
            self.assertEqual(m.remember(bad, ['should not be stored']), 0)
            self.assertEqual(m.forget(bad), 0)
        for bad in (good.upper(), good[:-1], good[:-1] + 'g'):
            s, r = http('POST', VIS, '/api/cortex/search', VTOK, {'q': '', 'space': 'v-' + bad, 'limit': 50})
            self.assertEqual(r['results'], [])
    def test_failures_never_raise(self):
        v = vid()
        for m in (Memory('http://127.0.0.1:9', VTOK),       # nothing listening
                  Memory(VIS, 'wrong' * 13),                # 401
                  Memory(VIS, ''),                          # disabled
                  Memory('http://10.1.2.3:18096', VTOK),    # not loopback: disabled
                  Memory('https://127.0.0.1:18096', VTOK)):
            t = time.monotonic()
            self.assertEqual(m.recall(v), [])
            self.assertEqual(m.remember(v, ['x']), 0)
            self.assertEqual(m.forget(v), 0)
            self.assertLess(time.monotonic() - t, 7)
        self.assertFalse(Memory('http://10.1.2.3:18096', VTOK).enabled)
        self.assertFalse(Memory(VIS, '').enabled)
        # negative control: the same vid with the right token works
        m = Memory(VIS, VTOK)
        self.assertEqual(m.remember(v, ['ok']), 1)
        self.assertEqual(m.recall(v), ['ok'])
        self.assertEqual(m.forget(v), 1)

    def test_timeout_is_two_seconds(self):
        srv = socket.socket()
        srv.bind(('127.0.0.1', 0))
        srv.listen(1)                                   # accepts, never answers
        m = Memory('http://127.0.0.1:%d' % srv.getsockname()[1], VTOK)
        t = time.monotonic()
        self.assertEqual(m.recall(vid()), [])
        self.assertLess(time.monotonic() - t, 3.0)
        srv.close()


@unittest.skipUnless(ETOK, 'run under atlas-vault run CORTEX_TOKEN')
class Isolation(Base):
    def test_visitor_entry_never_in_engineering(self):
        a = self.new()
        mark = 'zqiso' + secrets.token_hex(10)
        self.m.remember(a, ['marker %s here' % mark])
        s, r = search(VIS, VTOK, mark, 'v-' + a)        # control: findable where it lives
        self.assertEqual((s, len(r['results'])), (200, 1))
        for sp in ENG_SPACES + ['v-' + a]:
            s, r = search(ENG, ETOK, mark, sp)
            self.assertEqual(s, 200)
            self.assertEqual(r['results'], [], sp)
        s, r = http('POST', ENG, '/api/cortex/recall', ETOK, {'query': mark, 'space': 'v-' + a, 'limit': 5})
        self.assertEqual(s, 200)
        self.assertFalse([x for x in r.get('results', []) if mark in json.dumps(x)])

    def test_tokens_do_not_cross(self):
        s, r = search(ENG, ETOK, 'GB10', 'aien-lessons')
        self.assertEqual(s, 200)                        # control: engineering path works
        self.assertGreater(len(r['results']), 0)
        eng_ids = [e['id'] for e in r['results']]
        self.assertEqual(search(ENG, VTOK, 'GB10', 'aien-lessons')[0], 401)   # visitor token refused
        self.assertEqual(http('GET', ENG, '/api/cortex/get?' + urllib.parse.urlencode(
            {'id': eng_ids[0], 'space': 'aien-lessons'}), VTOK)[0], 401)
        self.assertEqual(search(VIS, ETOK, 'GB10', 'aien-lessons')[0], 401)   # and the reverse
        self.assertNotEqual(VTOK, ETOK)

    def test_engineering_data_absent_from_visitors(self):
        s, r = search(ENG, ETOK, 'GB10', 'aien-lessons')
        self.assertGreater(len(r['results']), 0)
        e = r['results'][0]
        for sp in ENG_SPACES:
            for q in ('GB10', e['canonicalName'].split()[-1]):
                s2, r2 = search(VIS, VTOK, q, sp)
                self.assertEqual((s2, r2['results']), (200, []), (sp, q))
            s2, r2 = http('POST', VIS, '/api/cortex/search', VTOK, {'q': '', 'space': sp, 'limit': 50})
            self.assertEqual(r2['results'], [], sp)
        self.assertEqual(http('GET', VIS, '/api/cortex/get?' + urllib.parse.urlencode(
            {'id': e['id'], 'space': 'aien-lessons'}), VTOK)[0], 404)


PROBE = r'''
import os, socket, sys
m = Memory.from_env()
v = sys.argv[1] if len(sys.argv) > 1 else os.environ['PROBE_VID']
print('enabled', m.enabled)
print('recall', m.recall(v))
def tcp(host, port):
    s = socket.socket(); s.settimeout(2)
    try:
        s.connect((host, port)); return 'connected'
    except OSError as e:
        return 'errno %s %s' % (e.errno, type(e).__name__)
    finally:
        s.close()
print('loopback', tcp('127.0.0.1', 18096))
print('other', tcp('127.0.0.53', 53))
'''

GATEWAY_PROPS = ['User=allen-chat', 'Group=allen-chat', 'SupplementaryGroups=allen-spool allen-drop',
                 'EnvironmentFile=-/etc/allen-chat/env', 'NoNewPrivileges=yes', 'CapabilityBoundingSet=',
                 'PrivateUsers=yes', 'ProtectSystem=strict', 'ProtectHome=yes', 'PrivateTmp=yes',
                 'RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX AF_NETLINK',
                 'IPAddressDeny=any', 'IPAddressAllow=127.0.0.1 100.64.0.2 100.64.0.3']


def run_probe(props, v):
    with open(os.path.join(HERE, 'allen_memory.py')) as f:
        src = f.read() + PROBE
    cmd = ['sudo', '-n', 'systemd-run', '--quiet', '--wait', '--pipe', '--collect',
           '-E', 'PROBE_VID=' + v]
    for p in props:
        cmd += ['-p', p]
    cmd += ['/usr/bin/python3', '-I', '-']
    r = subprocess.run(cmd, input=src, capture_output=True, text=True, timeout=60)
    return r.returncode, r.stdout + r.stderr


class GatewayReach(Base):
    def test_gateway_sandbox_reaches_visitors_store(self):
        a = self.new()
        self.m.remember(a, ['probe fact'])
        rc, out = run_probe(GATEWAY_PROPS, a)
        self.assertEqual(rc, 0, out)
        self.assertIn('enabled True', out)               # token present in /etc/allen-chat/env
        self.assertIn("recall ['probe fact']", out)
        self.assertIn('loopback connected', out)
        self.assertIn('other errno', out)                # IPAddressDeny drops the rest (timeout/EPERM)
        # negative control: without IPAddressDeny/Allow the other address is reachable,
        # so the failure above comes from the gateway's IP filter.
        rc, out = run_probe([p for p in GATEWAY_PROPS if not p.startswith('IPAddress')], a)
        self.assertIn('other connected', out)
        # negative control: without the env file the store is disabled
        rc, out = run_probe([p for p in GATEWAY_PROPS if not p.startswith('EnvironmentFile')], a)
        self.assertIn('enabled False', out)


if __name__ == '__main__':
    unittest.main(verbosity=2)
