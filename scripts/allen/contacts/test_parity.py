#!/usr/bin/python3
"""Parity: the seeded-Omega ingest against the Python reference on one fixture set.

  nice -n 19 python3 test_parity.py OMEGA_BINARY [--expect-differ]

Each phase copies the same drops (same names, same mtimes) into two drop directories, runs
the Python oracle and the Omega binary with the same fixed EPOCH into separate stores, then
compares byte for byte: contacts.jsonl, audit.log, every quarantined file (content, sorted),
the entries left in the drop dir, the journal summary line and the exit status.  It prints
sha256 per file for both sides.  --expect-differ turns it into the negative control: exit 0
only if some phase differs.
"""
import hashlib
import json
import os
import random
import shutil
import subprocess
import sys
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
PY = [sys.executable, "-I", "-B", os.path.join(HERE, "allen_contacts_ingest.py")]
EPOCH = "1793028123"            # fixed clock for audit and quarantine stamps
NOW = time.time_ns()


def sha(b):
    return hashlib.sha256(b).hexdigest()


def canon(obj):
    return json.dumps(obj, ensure_ascii=False, sort_keys=True).encode()


def intro(i, **over):
    rec = {"v": 1, "type": "introduce", "id": "c_%016x" % (0xa0000000 + i),
           "received": "2026-10-26T15:28:%02dZ" % (i % 60), "via": "chat", "kind": "human",
           "name": "Person %d" % i, "org": "Org", "contact": "", "github": "", "origin": "",
           "purpose": "", "homepage": "", "framework": "", "model": "", "human": "", "vid": "",
           "delete_token_sha256": sha(b"tok-%d" % i)}
    rec.update(over)
    return rec


VA, VB, VC = (sha(b"visitor-a"), sha(b"visitor-b"), sha(b"visitor-c"))


def fvis(vid):
    return {"v": 1, "type": "forget_visitor", "vid": vid, "received": "2026-10-26T16:30:00Z"}




def forget(i, token=None, cid=None):
    return {"v": 1, "type": "forget", "id": cid or "c_%016x" % (0xa0000000 + i),
            "received": "2026-10-26T16:00:00Z",
            "token_sha256": sha(token if token is not None else b"tok-%d" % i)}


def name(i):
    return "%016x.json" % (0xd0000000 + i)


def fixtures():
    """Phases of (filename, bytes, age_seconds or None)."""
    p1 = []
    good = [intro(1), intro(2, name="Zoë Ünïcode ✓ 𝄞", org="Ørg", purpose="line1\nline2\tTab \"q\" \\b",
                           homepage="https://example.invalid/𝄞", kind="agent", via="api",
                           model="m", framework="f", human="Drake", github="x" * 39),
            intro(3, name="", contact="c@example.invalid"),
            intro(4, name="n" * 100, org="é" * 100, purpose="𝄞" * 1000),
            intro(5, received="2024-02-29T23:59:59Z"),
            intro(6, homepage="https://abcd"),
            intro(7, vid=VA), intro(8, vid=VA, name="Second A"), intro(9, vid=VB)]
    for k, r in enumerate(good):
        p1.append((name(k), canon(r), None))
    bad = [
        b"{not json", b"", b"[1]", b"{}",
        canon(intro(20, extra="x")),                                 # unknown field
        canon({k: v for k, v in intro(21).items() if k != "org"}),   # missing field
        json.dumps(intro(22), ensure_ascii=False).encode(),          # unsorted keys
        json.dumps(intro(23), ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode(),
        canon(intro(24)) + b"\n",
        canon(intro(25, name="Zoë")).replace("ë".encode(), b"\\u00eb"),
        canon(intro(26)).replace(b"Person", b"Pe\xffrson"),          # invalid UTF-8
        canon(intro(27)).replace(b"Person", b"Pe\xed\xa0\x80rson"),  # encoded surrogate
        canon(intro(28)).replace(b"Person", b"Pe\xc0\xafrson"),      # overlong
        canon(intro(29)).replace(b"Person", b"Pe\x7frson"),
        canon(intro(30)).replace(b"Person", b"Pe\x01rson"),
        canon(intro(31, name="n" * 101)),
        canon(intro(32, purpose="𝄞" * 1001)),
        canon(intro(33, github="x" * 40)),
        canon(intro(34, name=" lead")), canon(intro(35, name="trail\u3000")),
        canon(intro(36, name="tab\t")), canon(intro(37, org="\u00a0x")),
        canon(intro(38, kind="robot")), canon(intro(39, via="mail")),
        canon(intro(40, name="", contact="")),
        canon(intro(41, homepage="http://example.invalid")),
        canon(intro(42, homepage="https://abc")), canon(intro(43, homepage="https://ab cd")),
        canon(intro(44, id="c_XYZ")), canon(intro(45, id="c_" + "A" * 16)),
        canon(intro(46, delete_token_sha256="a" * 63)), canon(intro(47, delete_token_sha256="G" * 64)),
        canon(intro(48, received="2025-02-29T00:00:00Z")),
        canon(intro(49, received="2026-13-01T00:00:00Z")),
        canon(intro(50, received="2026-10-26 15:28:00Z")),
        canon(intro(51, received="2026-10-26T24:00:00Z")),
        canon(intro(52, received="2026-10-26T15:28:60Z")),
        canon(intro(53, received="0000-01-01T00:00:00Z")),
        canon(intro(54, v=2)), canon(intro(55, v=True)), canon(intro(56, type="other")),
        canon(intro(57, org=5)),
        canon(intro(58)).replace(b'"v": 1', b'"v": 1, "v": 1'),
        canon(intro(59)).replace(b'"v": 1', b'"v": NaN'),
        canon(intro(60, purpose="a\u2028b")),                       # valid: U+2028 is not stripped inside
        canon(intro(61, name="x\\/y")).replace(b"\\\\/", b"\\/"),
        canon(forget(62, b"x")).replace(b'"v": 1', b'"v": 1, "extra": ""'),
        b" " * 65536 + canon(intro(63)),                              # oversize
        canon(intro(64, purpose="p" * 1000)) + b" " * 70000,           # oversize, valid head
        canon(intro(65, vid="A" * 64)), canon(intro(66, vid="a" * 63)),
        canon({k: v for k, v in intro(67).items() if k != "vid"}),    # pre-vid schema
        canon(fvis(VA)).replace(b'"forget_visitor"', b'"forget_visitors"'),
    ]
    for k, data in enumerate(bad):
        p1.append((name(100 + k), data, None))
    p1.append(("0123456789ABCDEF.json", canon(intro(70)), None))      # non-hex name (upper)
    p1.append(("notes.txt", canon(intro(71)), None))                  # wrong name
    p1.append((".tmp-%016x" % 1, canon(intro(72)), None))            # fresh temp: left
    p1.append((".tmp-%016x" % 2, canon(intro(73)), 3600))            # stale temp: removed
    p1.append((name(200), canon(intro(1)), None))                     # replay: already stored
    p1.append((name(201), canon(intro(2, name="Other")), None))       # same id: duplicate
    p2 = [
        (name(300), canon(forget(1)), None),                          # right token
        (name(301), canon(forget(2, b"wrong")), None),                # wrong token
        (name(302), canon(forget(3, b"tok-4")), None),                # another record's token
        (name(303), canon(forget(9, cid="c_" + "1" * 16)), None),     # unknown id
        (name(304), canon(forget(5, b"tok-5")).replace(b'"forget"', b'"forgot"'), None),
        (name(305), canon(forget(4)), None),                          # right token
    ]
    p2v = [
        (name(350), canon(fvis(VA)), None),                           # removes 7 and 8 only
        (name(351), canon(fvis(VC)), None),                           # no such visitor
        (name(352), canon(fvis("")), None),                           # empty vid: invalid
        (name(353), canon(fvis("B" * 64)), None),                     # uppercase: invalid
        (name(354), canon(dict(fvis(VB), id="c_" + "2" * 16)), None),  # extra key: invalid
    ]
    rng = random.Random(1792)
    p3 = []
    base = [canon(intro(400 + i, name="Fuzz %d ë𝄞" % i, purpose="a\nb\"c\\",
                        vid=VB if i % 2 else "")) for i in range(8)]
    base += [canon(fvis(VB)), canon(fvis(VC))]
    for i in range(600):
        b = bytearray(rng.choice(base))
        for _ in range(rng.randint(1, 3)):
            op = rng.random()
            pos = rng.randrange(len(b))
            if op < 0.4:
                b[pos] = rng.randrange(256)
            elif op < 0.7:
                del b[pos]
            else:
                b.insert(pos, rng.choice(b' "\\:,{}1\x00\x09\x0a\xc3\xa9\xe2\x80\xa8\xf0'))
        p3.append((name(1000 + i), bytes(b), None))
    p4 = [(name(5000 + i), canon(forget(400 + i, b"tok-%d" % (400 + i))), None) for i in range(8)]
    return [p1, p2, p2v, p3, p4]


def check_vids(py_store, om_store):
    """After forget_visitor(VA): both VA records gone; VB and every empty-vid record kept."""
    ok = True
    for side, data in (("py", py_store), ("om", om_store)):
        recs = [json.loads(x) for x in (data or b"").splitlines() if x.strip()]
        vids = [r["vid"] for r in recs]
        good = (vids.count(VA) == 0 and vids.count(VB) == 1 and vids.count("") >= 1)
        print("  vid check %s: VA=%d VB=%d empty=%d %s" % (side, vids.count(VA), vids.count(VB),
                                                        vids.count(""), "OK" if good else "WRONG"))
        ok = ok and good
    return ok


def lay(drop, phase, order):
    for k, (fn, data, age) in enumerate(phase):
        p = os.path.join(drop, fn)
        with open(p, "wb") as f:
            f.write(data)
        t = NOW - (age * 1_000_000_000 if age else 0) - (len(phase) - order[k]) * 1_000_000
        os.utime(p, ns=(t, t))


def snapshot(drop, store):
    def rd(p):
        try:
            with open(p, "rb") as f:
                return f.read()
        except FileNotFoundError:
            return None
    q = os.path.join(store, "quarantine")
    qs = sorted(rd(os.path.join(q, f)) for f in os.listdir(q)) if os.path.isdir(q) else []
    return {"contacts.jsonl": rd(os.path.join(store, "contacts.jsonl")),
            "audit.log": rd(os.path.join(store, "audit.log")),
            "quarantine": qs,
            "drop_left": sorted(os.listdir(drop)),
            "lock_left": os.path.exists(os.path.join(store, ".lock.d"))}


def digest(v):
    if v is None:
        return "absent"
    if isinstance(v, bytes):
        return "%s (%d bytes)" % (sha(v), len(v))
    if isinstance(v, list) and v and isinstance(v[0], bytes):
        return "%d files, sha256 of sorted list %s" % (len(v), sha(b"".join(sha(x).encode() for x in v)))
    return repr(v)


def edge_checks(cmds):
    """Same outcome for both sides: a held lock, a stale lock, an unsafe drop dir, bad usage."""
    ok = True
    for side, cmd in cmds.items():
        base = tempfile.mkdtemp(prefix="allen-edge-")
        drop, store = os.path.join(base, "drop"), os.path.join(base, "store")
        os.mkdir(drop, 0o700)
        os.mkdir(store, 0o700)
        lock = os.path.join(store, ".lock.d")
        try:
            with open(os.path.join(drop, name(1)), "wb") as f:
                f.write(canon(intro(1)))
            os.mkdir(lock)                                     # held by someone else
            t0 = time.monotonic()
            r = subprocess.run(cmd + ["ingest", drop, store, EPOCH], capture_output=True)
            held = (r.returncode == 1 and os.listdir(drop) == [name(1)]
                    and not os.path.exists(os.path.join(store, "contacts.jsonl"))
                    and os.path.isdir(lock) and time.monotonic() - t0 >= 9.5)
            old = time.time() - 400
            os.utime(lock, (old, old))                         # stale: broken and taken
            r = subprocess.run(cmd + ["ingest", drop, store, EPOCH], capture_output=True)
            stale = (r.returncode == 0 and os.listdir(drop) == [] and not os.path.exists(lock)
                     and os.path.getsize(os.path.join(store, "contacts.jsonl")) > 0)
            with open(os.path.join(drop, name(2)), "wb") as f:
                f.write(canon(intro(2)))
            os.chmod(drop, 0o777)
            r = subprocess.run(cmd + ["ingest", drop, store, EPOCH], capture_output=True)
            unsafe = r.returncode == 1 and os.listdir(drop) == [name(2)]
            os.chmod(drop, 0o700)
            r = subprocess.run(cmd + ["ingest", drop], capture_output=True)
            usage = r.returncode == 64
        finally:
            os.chmod(drop, 0o700)
            shutil.rmtree(base)
        print("edge %s: held_lock_waits_then_refuses=%s stale_lock_broken=%s unsafe_drop_refused=%s"
              " usage_64=%s" % (side, held, stale, unsafe, usage))
        ok = ok and held and stale and unsafe and usage
    return ok


def main():
    omega = os.path.abspath(sys.argv[1])
    expect_differ = "--expect-differ" in sys.argv
    base = tempfile.mkdtemp(prefix="allen-parity-")
    dirs = {}
    for side in ("py", "om"):
        for d in ("drop", "store"):
            p = os.path.join(base, side, d)
            os.makedirs(p, 0o700)
            os.chmod(p, 0o700)
            os.chmod(os.path.dirname(p), 0o700)
        dirs[side] = (os.path.join(base, side, "drop"), os.path.join(base, side, "store"))
    cmds = {"py": PY, "om": [omega]}
    differ = False
    rng = random.Random(7)
    try:
        for n, phase in enumerate(fixtures(), 1):
            order = list(range(len(phase)))
            rng.shuffle(order)
            res = {}
            for side in ("py", "om"):
                drop, store = dirs[side]
                lay(drop, phase, order)
                r = subprocess.run(cmds[side] + ["ingest", drop, store, EPOCH],
                                   capture_output=True, timeout=300)
                res[side] = (r.returncode, r.stdout, snapshot(drop, store))
            print("phase %d: %d drops" % (n, len(phase)))
            for side in ("py", "om"):
                rc, out, snap = res[side]
                print("  %s rc=%d %s" % (side, rc, out.decode(errors="replace").strip()))
            same = res["py"][:2] == res["om"][:2]
            for key in res["py"][2]:
                a, b = res["py"][2][key], res["om"][2][key]
                ok = a == b
                same = same and ok
                print("  %-15s %s  py %s" % (key, "SAME" if ok else "DIFF", digest(a)))
                if not ok:
                    print("  %-15s       om %s" % ("", digest(b)))
            print("  phase %d %s" % (n, "IDENTICAL" if same else "DIFFERENT"))
            differ = differ or not same
            if n == 3:
                vid_ok = check_vids(res["py"][2]["contacts.jsonl"], res["om"][2]["contacts.jsonl"])
                differ = differ or not vid_ok
    finally:
        shutil.rmtree(base)
    if expect_differ:
        print("NEGATIVE CONTROL %s" % ("DETECTED" if differ else "NOT DETECTED"))
        return 0 if differ else 1
    edges = edge_checks(cmds)
    print("PARITY %s" % ("FAIL" if differ or not edges else "PASS"))
    return 1 if differ or not edges else 0


if __name__ == "__main__":
    sys.exit(main())
