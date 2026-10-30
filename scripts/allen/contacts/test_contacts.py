#!/usr/bin/python3
"""Tests for allen_contacts_ingest + allen-contacts.  Every positive check has a negative
control.  Run:  nice -n 19 python3 test_contacts.py            (temporary dirs only)
                RUN_INSTALLED=1 nice -n 19 python3 test_contacts.py   (+ installed smoke)
"""
import contextlib
import hashlib
import io
import json
import os
import secrets
import shutil
import stat
import subprocess
import sys
import tempfile
import time
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import allen_contacts_ingest as aci  # noqa: E402

REAL_DROP = "/var/lib/allen-chat/contacts-drop"
REAL_STORE_DIR = "/var/lib/allen-contacts"
REAL_STORE = REAL_STORE_DIR + "/contacts.jsonl"
INSTALLED_CLI = "/usr/local/bin/allen-contacts"

# The gateway-side properties under test (CONTRACT.md: system unit, User=allen-chat).
GATEWAY_PROPS = [
    "User=allen-chat",
    "NoNewPrivileges=yes",
    "CapabilityBoundingSet=",
    "PrivateUsers=yes",
    "ProtectProc=invisible",
    "ProtectSystem=strict",
    "ProtectHome=yes",
    "PrivateTmp=yes",
    "ReadWritePaths=/var/lib/allen-chat/contacts-drop /var/lib/allen-chat/spool-in",
]


def sha(token):
    return hashlib.sha256(token.encode()).hexdigest()


def intro(**over):
    rec = {"v": 1, "type": "introduce", "id": "c_" + secrets.token_hex(8),
           "received": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "via": "chat",
           "kind": "human", "name": "Ada Canary", "org": "Canary Org",
           "contact": "ada@example.invalid", "github": "ada-canary", "origin": "Dublin",
           "purpose": "Building a canary\nsecond line", "homepage": "https://example.invalid/x",
           "framework": "", "model": "", "human": "", "delete_token_sha256": sha("tok-A"),
           "vid": ""}
    rec.update(over)
    return rec


def canon(obj):
    """The gateway's exact drop serialization."""
    return json.dumps(obj, ensure_ascii=False, sort_keys=True).encode()


def write_drop(d, obj, name=None):
    """Gateway-style: '.tmp-<hex>' then rename to '<hex>.json'."""
    name = name or secrets.token_hex(8) + ".json"
    data = obj if isinstance(obj, bytes) else canon(obj)
    tmp = os.path.join(d, ".tmp-" + secrets.token_hex(8))
    fd = os.open(tmp, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, "wb") as f:
        f.write(data)
    os.rename(tmp, os.path.join(d, name))
    return name


def rd(path):
    with open(path) as f:
        return f.read()


def wr(path, text):
    with open(path, "w") as f:
        f.write(text)


class Tmp(unittest.TestCase):
    def setUp(self):
        self.base = tempfile.mkdtemp(prefix="allen-contacts-test-")
        self.drop = os.path.join(self.base, "drop")
        self.store = os.path.join(self.base, "store")
        os.mkdir(self.drop, 0o700)
        os.mkdir(self.store, 0o700)

    def tearDown(self):
        shutil.rmtree(self.base)

    def ingest(self):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            counts = aci.ingest(self.drop, self.store)
        self.log = out.getvalue()
        for canary in ("Ada", "Canary", "ada@", "Dublin", "example.invalid"):
            self.assertNotIn(canary, self.log, "a contact field reached the log")
        return counts

    def cli(self, *args, cli=os.path.join(HERE, "allen-contacts")):
        env = dict(os.environ, ALLEN_CONTACTS_STORE=self.store)
        return subprocess.run([sys.executable, "-I", cli, *args], env=env,
                              capture_output=True, text=True, timeout=30)

    def records(self):
        with aci.Store(self.store) as s:
            return s.records()

    def quarantined(self):
        q = os.path.join(self.store, "quarantine")
        return len(os.listdir(q)) if os.path.isdir(q) else 0


class TestIngest(Tmp):
    def test_valid_introduce_lands_and_cli_show(self):
        rec = intro()
        name = write_drop(self.drop, rec)
        c = self.ingest()
        self.assertEqual(c.get("introduce_stored"), 1)
        self.assertFalse(os.path.exists(os.path.join(self.drop, name)), "drop not deleted")
        self.assertEqual(self.records(), [rec])
        self.assertEqual(stat.S_IMODE(os.stat(os.path.join(self.store, "contacts.jsonl")).st_mode), 0o600)
        r = self.cli("show", rec["id"])
        self.assertEqual(r.returncode, 0, r.stderr)
        for line in ("id: " + rec["id"], "name: Ada Canary", "org: Canary Org",
                     "contact: ada@example.invalid", "purpose: Building a canary\\nsecond line"):
            self.assertIn(line, r.stdout.splitlines())
        self.assertNotIn(rec["delete_token_sha256"], r.stdout)
        r = self.cli("list")
        self.assertEqual(r.stdout, "\t".join([rec["id"], rec["received"], "human", "Ada Canary",
                                              "Canary Org"]) + "\n")
        # negative control: an unknown id shows nothing
        r = self.cli("show", "c_" + "0" * 16)
        self.assertEqual(r.returncode, 1)
        self.assertEqual(r.stdout, "")

    def test_malformed_quarantined_not_stored(self):
        good = intro()
        bad = {
            "json": b"{not json",
            "list": b"[1,2]",
            "nan": canon(intro()).replace(b'"v": 1', b'"v": NaN'),
            "dupkey": canon(intro()).replace(b'"v": 1', b'"v": 1, "v": 1'),
            "unsorted": json.dumps(intro(), ensure_ascii=False).encode(),
            "compact": json.dumps(intro(), ensure_ascii=False, sort_keys=True,
                                  separators=(",", ":")).encode(),
            "ascii_escaped": canon(intro(name="Zoë")).replace("ë".encode(), b"\\u00eb"),
            "trailing_nl": canon(intro()) + b"\n",
            "bad_utf8": canon(intro()).replace(b"Ada", b"A\xffa"),
            "extra": intro(extra="x"),
            "missing": {k: v for k, v in intro().items() if k != "org"},
            "id": intro(id="c_XYZ"),
            "id_upper": intro(id="c_" + "A" * 16),
            "too_long": intro(name="x" * 101),
            "v_true": intro(v=True),
            "v_2": intro(v=2),
            "type": intro(type="other"),
            "kind": intro(kind="robot"),
            "via": intro(via="email"),
            "sha_short": intro(delete_token_sha256="a" * 63),
            "sha_upper": intro(delete_token_sha256="A" * 64),
            "control": intro(name="Ada\x07"),
            "not_str": intro(org=5),
            "homepage": intro(homepage="http://example.invalid/"),
            "received": intro(received="2026-10-26 10:00:00"),
            "received_date": intro(received="2026-13-40T10:00:00Z"),
            "nobody": intro(name="", contact=""),
            "padded": intro(name=" Ada"),
            "forget_bad_sha": {"v": 1, "type": "forget", "id": good["id"], "token_sha256": "zz",
                               "received": good["received"]},
            "oversize": b" " * (aci.MAX_DROP_BYTES + 10) + b"{}",
        }
        n = 0
        for label, payload in bad.items():
            with self.subTest(label):
                name = write_drop(self.drop, payload)
                c = self.ingest()
                n += 1
                self.assertEqual(c.get("quarantined"), 1, label)
                self.assertNotIn("introduce_stored", c)
                self.assertEqual(self.quarantined(), n)
                self.assertFalse(os.path.exists(os.path.join(self.drop, name)))
                self.assertEqual(self.records(), [])
        # a valid record under a non-contract file name is quarantined too
        write_drop(self.drop, intro(), name="notes.txt")
        self.assertEqual(self.ingest().get("q_name"), 1)
        self.assertEqual(self.records(), [])
        # positive control: the same machinery stores a good record
        write_drop(self.drop, good)
        self.assertEqual(self.ingest().get("introduce_stored"), 1)
        self.assertEqual(self.records(), [good])
        q = os.path.join(self.store, "quarantine")
        self.assertEqual(stat.S_IMODE(os.stat(q).st_mode), 0o700)
        for f in os.listdir(q):
            self.assertEqual(stat.S_IMODE(os.stat(os.path.join(q, f)).st_mode), 0o600)

    def test_forget_wrong_token_then_right_token(self):
        a = intro(delete_token_sha256=sha("tok-A"))
        b = intro(name="Bob Canary", delete_token_sha256=sha("tok-B"))
        write_drop(self.drop, a)
        self.ingest()
        write_drop(self.drop, b)
        self.ingest()
        self.assertEqual(self.records(), [a, b])

        def forget(cid, token):
            return {"v": 1, "type": "forget", "id": cid, "token_sha256": sha(token),
                    "received": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())}
        # negative controls: wrong token; right token of another record; unknown id
        for cid, tok in ((a["id"], "tok-WRONG"), (a["id"], "tok-B"), ("c_" + "1" * 16, "tok-A")):
            write_drop(self.drop, forget(cid, tok))
            c = self.ingest()
            self.assertEqual(c.get("forget_unmatched"), 1)
            self.assertEqual(self.records(), [a, b])
        self.assertFalse(os.path.exists(os.path.join(self.store, "audit.log")))
        # right token: exactly record a goes
        write_drop(self.drop, forget(a["id"], "tok-A"))
        c = self.ingest()
        self.assertEqual(c.get("forget_matched"), 1)
        self.assertEqual(self.records(), [b])
        audit = rd(os.path.join(self.store, "audit.log"))
        self.assertRegex(audit, r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ forget removed=1\n$")
        self.assertEqual(os.listdir(self.drop), [])
        self.assertEqual(sorted(f for f in os.listdir(self.store) if f.startswith(".contacts")), [])

    def test_forget_visitor_removes_only_that_vid(self):
        va, vb = sha("visitor-a"), sha("visitor-b")
        a1, a2, b, none = intro(vid=va), intro(vid=va), intro(vid=vb), intro()
        for r in (a1, a2, b, none):
            write_drop(self.drop, r)
            self.ingest()
        self.assertEqual(len(self.records()), 4)

        def fv(vid):
            return {"v": 1, "type": "forget_visitor", "vid": vid,
                    "received": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())}
        # negative controls: an unknown vid and an empty vid remove nothing
        write_drop(self.drop, fv(sha("visitor-c")))
        self.assertEqual(self.ingest().get("forget_visitor_unmatched"), 1)
        write_drop(self.drop, fv(""))
        self.assertEqual(self.ingest().get("q_invalid"), 1)
        self.assertEqual(len(self.records()), 4)
        write_drop(self.drop, fv(va))
        self.assertEqual(self.ingest().get("forget_visitor_matched"), 1)
        self.assertEqual(self.records(), [b, none])
        audit = rd(os.path.join(self.store, "audit.log"))
        self.assertRegex(audit, r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ forget_visitor removed=2\n$")
        self.assertNotIn(va, audit)

    def test_cli_delete(self):
        a, b = intro(), intro(name="Bob Canary")
        write_drop(self.drop, a)
        write_drop(self.drop, b)
        self.ingest()
        self.assertEqual(len(self.records()), 2)
        r = self.cli("delete", "c_" + "2" * 16)          # negative control
        self.assertEqual(r.returncode, 1)
        self.assertEqual(len(self.records()), 2)
        r = self.cli("delete", a["id"])
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.records(), [b])
        self.assertEqual(self.cli("list").stdout.count("\n"), 1)
        self.assertEqual(self.cli("delete", a["id"]).returncode, 1)
        self.assertEqual(self.cli("show", "../etc").returncode, 64)
        audit = rd(os.path.join(self.store, "audit.log"))
        self.assertTrue(audit.endswith(" cli-delete removed=1\n"))
        self.assertNotIn(a["id"], audit)

    def test_temp_symlink_and_duplicates(self):
        fresh = os.path.join(self.drop, ".tmp-" + secrets.token_hex(8))
        stale = os.path.join(self.drop, ".tmp-" + secrets.token_hex(8))
        for p in (fresh, stale):
            wr(p, canon(intro()).decode())
        old = time.time() - aci.TEMP_STALE_S - 5
        os.utime(stale, (old, old))
        outside = os.path.join(self.base, "outside.txt")
        wr(outside, "keep me")
        os.symlink(outside, os.path.join(self.drop, "a" * 16 + ".json"))
        c = self.ingest()
        self.assertEqual(c.get("temp_pending"), 1)
        self.assertEqual(c.get("temp_stale_removed"), 1)
        self.assertEqual(c.get("removed_nonregular"), 1)
        self.assertTrue(os.path.exists(fresh), "a fresh temp file must be left for the writer")
        self.assertFalse(os.path.exists(stale))
        self.assertEqual(rd(outside), "keep me")
        self.assertEqual(self.quarantined(), 0)
        self.assertEqual(self.records(), [])
        os.unlink(fresh)
        a = intro()
        write_drop(self.drop, a)
        write_drop(self.drop, a)                      # replay of the same drop
        write_drop(self.drop, dict(a, name="Mallory"))  # same id, other content
        c = self.ingest()
        self.assertEqual((c.get("introduce_stored"), c.get("introduce_already_stored"),
                          c.get("q_duplicate_id")), (1, 1, 1))
        self.assertEqual(self.records(), [a])

    def test_refuses_open_store_dir(self):
        os.chmod(self.store, 0o755)
        write_drop(self.drop, intro())
        with self.assertRaises(SystemExit):
            aci.ingest(self.drop, self.store)
        self.assertEqual(len(os.listdir(self.drop)), 1, "drop must stay when the store is unsafe")
        os.chmod(self.store, 0o700)
        os.chmod(self.drop, 0o2733)                  # world-writable drop dir: refuse
        with self.assertRaises(SystemExit):
            aci.ingest(self.drop, self.store)
        os.chmod(self.drop, 0o2730)                  # the contract mode: accepted
        self.assertEqual(self.ingest().get("introduce_stored"), 1)


def systemd_run(props, cmd):
    argv = ["sudo", "-n", "systemd-run", "--wait", "--pipe", "-q"]
    for p in props:
        argv += ["-p", p]
    return subprocess.run(argv + ["/bin/sh", "-c", cmd], capture_output=True, text=True,
                          timeout=60, stdin=subprocess.DEVNULL)


GW_WRITE = r"""
import os, secrets, sys
os.umask(0o027)
d = sys.argv[1]
n = secrets.token_hex(8)
t = os.path.join(d, ".tmp-" + n)
fd = os.open(t, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o640)
os.write(fd, sys.stdin.buffer.read())
os.fsync(fd)
os.close(fd)
os.rename(t, os.path.join(d, n + ".json"))
print(n + ".json")
"""


def as_user(user, argv, data=None):
    return subprocess.run(["sudo", "-n", "-u", user, "--"] + argv, input=data,
                          capture_output=True, timeout=60)


def gw_drop(obj):
    """Write a drop exactly as the gateway user allen-chat does (0640, group allen-drop)."""
    r = as_user("allen-chat", ["/usr/bin/python3", "-I", "-c", GW_WRITE, REAL_DROP],
                canon(obj))
    assert r.returncode == 0, r.stderr
    return r.stdout.decode().strip()


def drop_entries():
    r = as_user("allen-contacts", ["/bin/ls", "-A", REAL_DROP])
    assert r.returncode == 0, r.stderr
    return r.stdout.decode().split()


@unittest.skipUnless(os.environ.get("RUN_INSTALLED") == "1", "set RUN_INSTALLED=1")
class TestInstalled(unittest.TestCase):
    def test_installed_smoke_invisibility_and_delete(self):
        r = subprocess.run(["systemctl", "is-active", "allen-contacts-ingest.path"],
                           capture_output=True, text=True)
        self.assertEqual(r.stdout.strip(), "active")
        since = time.strftime("%Y-%m-%d %H:%M:%S")
        rec = intro(name="Smoke Test " + secrets.token_hex(4), org="allen-contacts smoke",
                    contact="", github="", origin="", purpose="installed-path smoke",
                    homepage="", delete_token_sha256=sha("smoke-token"))
        gw_drop(rec)
        cli = lambda *a: subprocess.run([INSTALLED_CLI, *a], capture_output=True, text=True,
                                        timeout=30)
        deadline = time.time() + 30
        while time.time() < deadline and (drop_entries() or cli("show", rec["id"]).returncode):
            time.sleep(0.5)
        try:
            r = cli("show", rec["id"])
            self.assertEqual(r.returncode, 0, "path unit did not ingest the smoke drop")
            self.assertIn("name: " + rec["name"], r.stdout.splitlines())
            self.assertEqual(drop_entries(), [])
            j = subprocess.run(["sudo", "-n", "journalctl", "-u", "allen-contacts-ingest.service",
                                "--since", since, "--no-pager", "-o", "cat"],
                               capture_output=True, text=True).stdout
            self.assertIn("introduce_stored=1", j)
            self.assertNotIn(rec["name"], j)

            # The gateway user cannot read the store; the store user can (control).
            gw = as_user("allen-chat", ["/bin/cat", REAL_STORE])
            self.assertNotEqual(gw.returncode, 0)
            self.assertEqual(gw.stdout, b"")
            ctl = as_user("allen-contacts", ["/bin/cat", REAL_STORE])
            self.assertEqual(ctl.returncode, 0, ctl.stderr)
            self.assertIn(rec["id"].encode(), ctl.stdout)
            print("\n[sudo -u allen-chat cat] rc=%d %s" % (gw.returncode, gw.stderr.decode().strip()))
            # Same inside the gateway unit's properties; control: same command as allen-contacts.
            cmd = "cat %s; ls %s" % (REAL_STORE, REAL_STORE_DIR)
            gwu = systemd_run(GATEWAY_PROPS, cmd)
            self.assertNotEqual(gwu.returncode, 0)
            self.assertEqual(gwu.stdout, "")
            print("[gateway unit props] rc=%d\n%s" % (gwu.returncode, gwu.stderr.strip()))
            ctlu = systemd_run(["User=allen-contacts"], "cat " + REAL_STORE)
            self.assertEqual(ctlu.returncode, 0, ctlu.stderr)
            self.assertIn(rec["id"], ctlu.stdout)
            # Drake's own login cannot read it without sudo either.
            with self.assertRaises(PermissionError):
                rd(REAL_STORE)

            def settle():
                deadline = time.time() + 30
                while time.time() < deadline and drop_entries():
                    time.sleep(0.5)
                self.assertEqual(drop_entries(), [])

            def now():
                return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())

            # forget with a wrong token through the real path unit changes nothing
            gw_drop({"v": 1, "type": "forget", "id": rec["id"], "token_sha256": sha("wrong"),
                     "received": now()})
            settle()
            self.assertEqual(cli("show", rec["id"]).returncode, 0)

            # a second record, forgotten with its right token: removed, the first stays
            rec2 = dict(rec, id="c_" + secrets.token_hex(8), delete_token_sha256=sha("smoke-2"))
            gw_drop(rec2)
            settle()
            self.assertEqual(cli("show", rec2["id"]).returncode, 0)
            gw_drop({"v": 1, "type": "forget", "id": rec2["id"], "token_sha256": sha("smoke-2"),
                     "received": now()})
            settle()
            self.assertEqual(cli("show", rec2["id"]).returncode, 1)
            self.assertEqual(cli("show", rec["id"]).returncode, 0)
            audit = as_user("allen-contacts", ["/usr/bin/tail", "-n", "1",
                                               REAL_STORE_DIR + "/audit.log"]).stdout.decode()
            self.assertRegex(audit, r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ forget removed=1\n$")

            # a malformed drop is quarantined (random name, getrandom under the unit's filter)
            qdir = REAL_STORE_DIR + "/quarantine"
            before = set(as_user("allen-contacts", ["/bin/ls", "-A", qdir]).stdout.split())
            gw_drop({"v": 1, "type": "introduce", "smoke": "malformed"})
            settle()
            new = set(as_user("allen-contacts", ["/bin/ls", "-A", qdir]).stdout.split()) - before
            self.assertEqual(len(new), 1)
            q = new.pop().decode()
            got = as_user("allen-contacts", ["/bin/cat", qdir + "/" + q]).stdout
            self.assertEqual(got, canon({"v": 1, "type": "introduce", "smoke": "malformed"}))
            self.assertEqual(as_user("allen-contacts", ["/bin/rm", qdir + "/" + q]).returncode, 0)
        finally:
            d = cli("delete", rec["id"])
        self.assertEqual(d.returncode, 0, d.stderr)
        self.assertEqual(cli("show", rec["id"]).returncode, 1)


if __name__ == "__main__":
    unittest.main(verbosity=2)
