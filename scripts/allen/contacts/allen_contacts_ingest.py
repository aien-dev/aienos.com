#!/usr/bin/python3 -I
"""Private contact store for public Allen: Python reference (parity oracle) and CLI library.

The production ingest is the seeded-Omega port `allen_contacts_ingest.om`; this file
specifies the same behaviour and is run against it on a shared fixture set
(test_parity.py).  The `allen-contacts` CLI imports this module.

`ingest` moves consented contact drops written by the gateway
(/var/lib/allen-chat/contacts-drop/<16 hex>.json, see CONTRACT.md) into the private
store /var/lib/allen-contacts/contacts.jsonl:

  introduce -> the drop bytes plus "\\n" are appended (O_APPEND, fsync).  Append-only.
  forget    -> the store is rewritten without the record only when the ids match
               and hmac.compare_digest(token_sha256, record.delete_token_sha256)
               holds (tmp file + fsync + rename).
  forget_visitor -> the store is rewritten without every record whose "vid" equals the
               drop's 64-hex vid (an empty vid is invalid, so "" never matches).
  anything else -> copied to <store>/quarantine/, nothing stored.

A drop is accepted only in the gateway's exact serialization,
json.dumps(record, ensure_ascii=False, sort_keys=True), with the CONTRACT field
rules.  So a store line is the canonical drop, byte for byte.

Every processed drop is deleted.  The journal gets one summary line of counts.  No
contact field is ever logged.  audit.log keeps one tombstone line per deletion with
the timestamp, the event and the count removed; it records no id and no field.

Store writers (ingest, CLI delete) serialize through the lock directory
<store>/.lock.d (mkdir is atomic); a lock older than LOCK_STALE_S is broken.
"""
import errno
import hmac
import json
import os
import re
import secrets
import stat
import sys
import time

# Environment overrides exist for the tests' temporary directories only; the unit
# and the sudo re-exec of the CLI run with a clean environment.
DROP_DIR = os.environ.get("ALLEN_CONTACTS_DROP", "/var/lib/allen-chat/contacts-drop")
STORE_DIR = os.environ.get("ALLEN_CONTACTS_STORE", "/var/lib/allen-contacts")
STORE_NAME = "contacts.jsonl"
AUDIT_NAME = "audit.log"
LOCK_NAME = ".lock.d"
QUARANTINE = "quarantine"

MAX_DROP_BYTES = 64 * 1024
MAX_ENTRIES = 4096
QUARANTINE_CAP = 1000
TEMP_STALE_S = 60
LOCK_WAIT_S = 10
LOCK_STALE_S = 300

DROP_NAME = re.compile(rb"[0-9a-f]{16}\.json")
ID_RE = re.compile(r"c_[0-9a-f]{16}", re.ASCII)
SHA_RE = re.compile(r"[0-9a-f]{64}", re.ASCII)
RECEIVED_RE = re.compile(r"([0-9]{4})-([0-9]{2})-([0-9]{2})T([0-9]{2}):([0-9]{2}):([0-9]{2})Z",
                         re.ASCII)
HOMEPAGE_RE = re.compile(r"https://[^\s]{4,}")

# Field caps from CONTRACT.md (POST introduce); same table as the gateway.
TEXT_FIELDS = {"name": 100, "org": 100, "contact": 200, "github": 39, "origin": 200,
               "purpose": 1000, "homepage": 200, "framework": 100, "model": 100,
               "human": 200}
INTRODUCE_KEYS = frozenset(TEXT_FIELDS) | {"v", "type", "id", "received", "via", "kind",
                                           "delete_token_sha256", "vid"}
FORGET_KEYS = frozenset({"v", "type", "id", "token_sha256", "received"})
FORGET_VISITOR_KEYS = frozenset({"v", "type", "vid", "received"})

O_DIR = os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC


class Invalid(Exception):
    """A drop that fails validation."""


def _reject(*_):
    raise Invalid()


def _text(v, cap):
    if not isinstance(v, str) or len(v) > cap or v != v.strip():
        raise Invalid()
    for c in v:
        o = ord(c)
        if (o < 32 and c not in "\n\t") or o == 127:
            raise Invalid()


def _received(v):
    m = RECEIVED_RE.fullmatch(v) if isinstance(v, str) else None
    if not m:
        raise Invalid()
    y, mo, d, h, mi, s = (int(x) for x in m.groups())
    leap = y % 4 == 0 and (y % 100 != 0 or y % 400 == 0)
    dim = (31, 29 if leap else 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31)
    if not (y >= 1 and 1 <= mo <= 12 and 1 <= d <= dim[mo - 1] and h <= 23 and mi <= 59
            and s <= 59):
        raise Invalid()


def parse_drop(data):
    """Bytes -> validated record dict.  Raises Invalid."""
    try:
        rec = json.loads(data.decode("utf-8"), object_pairs_hook=_pairs,
                         parse_constant=_reject)
    except (ValueError, RecursionError):
        raise Invalid() from None
    if not isinstance(rec, dict):
        raise Invalid()
    if json.dumps(rec, ensure_ascii=False, sort_keys=True).encode("utf-8") != data:
        raise Invalid()
    if type(rec.get("v")) is not int or rec["v"] != 1:
        raise Invalid()
    kind = rec.get("type")
    if kind == "introduce":
        if set(rec) != INTRODUCE_KEYS:
            raise Invalid()
        for k, cap in TEXT_FIELDS.items():
            _text(rec[k], cap)
        if rec["kind"] not in ("human", "agent") or rec["via"] not in ("chat", "api"):
            raise Invalid()
        if not (rec["name"] or rec["contact"]):
            raise Invalid()
        if rec["homepage"] and not HOMEPAGE_RE.fullmatch(rec["homepage"]):
            raise Invalid()
        if rec["vid"] != "" and not (isinstance(rec["vid"], str) and SHA_RE.fullmatch(rec["vid"])):
            raise Invalid()
        sha = rec["delete_token_sha256"]
    elif kind == "forget":
        if set(rec) != FORGET_KEYS:
            raise Invalid()
        sha = rec["token_sha256"]
    elif kind == "forget_visitor":
        if set(rec) != FORGET_VISITOR_KEYS:
            raise Invalid()
        if not isinstance(rec["vid"], str) or not SHA_RE.fullmatch(rec["vid"]):
            raise Invalid()                      # an empty vid never matches anything
        _received(rec["received"])
        return rec
    else:
        raise Invalid()
    if not isinstance(sha, str) or not SHA_RE.fullmatch(sha):
        raise Invalid()
    if not isinstance(rec["id"], str) or not ID_RE.fullmatch(rec["id"]):
        raise Invalid()
    _received(rec["received"])
    return rec


def _pairs(pairs):
    d = {}
    for k, v in pairs:
        if k in d:
            raise Invalid()
        d[k] = v
    return d


# ---------------------------------------------------------------- store

def _fsync_dir(path):
    fd = os.open(path, O_DIR)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def _check_private_dir(path, group_ok=0):
    """Fail closed unless `path` is a real directory we own with no access beyond
    owner (+ the `group_ok` group bits: the drop dir is allen-contacts:allen-drop 2730)."""
    st = os.lstat(path)
    if not stat.S_ISDIR(st.st_mode):
        raise SystemExit("allen-contacts: %s is not a directory" % path)
    if st.st_uid != os.getuid() or st.st_mode & 0o077 & ~group_ok:
        raise SystemExit("allen-contacts: %s must be owned by uid %d, mode 0700%s"
                         % (path, os.getuid(), " or 2730" if group_ok else ""))


def stamp(fmt, now=None):
    return time.strftime(fmt, time.gmtime(time.time() if now is None else now))


class Store:
    """contacts.jsonl; writers hold the lock directory <store>/.lock.d."""

    def __init__(self, root=None, now=None):
        self.root = root or STORE_DIR
        _check_private_dir(self.root)
        self.path = os.path.join(self.root, STORE_NAME)
        self.lock = os.path.join(self.root, LOCK_NAME)
        self.now = now

    def __enter__(self):
        deadline = time.monotonic() + LOCK_WAIT_S
        while True:
            try:
                os.mkdir(self.lock, 0o700)
                return self
            except FileExistsError:
                pass
            try:
                if time.time() - os.lstat(self.lock).st_mtime > LOCK_STALE_S:
                    os.rmdir(self.lock)
                    continue
            except FileNotFoundError:
                continue
            if time.monotonic() >= deadline:
                raise SystemExit("allen-contacts: store is locked (%s)" % self.lock)
            time.sleep(0.05)

    def __exit__(self, *exc):
        os.rmdir(self.lock)

    def raw(self):
        try:
            fd = os.open(self.path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW)
        except FileNotFoundError:
            return b""
        with os.fdopen(fd, "rb") as f:
            return f.read()

    def lines(self):
        """[(raw_line, record_or_None)]; unparsable lines are kept verbatim."""
        out = []
        for raw in self.raw().split(b"\n"):
            if not raw.strip():
                continue
            try:
                rec = json.loads(raw)
                if not isinstance(rec, dict):
                    rec = None
            except ValueError:
                rec = None
            out.append((raw, rec))
        return out

    def records(self):
        return [r for _, r in self.lines() if r is not None]

    def find(self, cid):
        for raw, r in self.lines():
            if r is not None and r.get("id") == cid:
                return raw, r
        return None, None

    def append(self, line):
        created = not os.path.exists(self.path)
        fd = os.open(self.path, os.O_WRONLY | os.O_APPEND | os.O_CREAT | os.O_CLOEXEC
                     | os.O_NOFOLLOW, 0o600)
        try:
            os.fchmod(fd, 0o600)
            view = memoryview(line + b"\n")
            while view:
                view = view[os.write(fd, view):]
            os.fsync(fd)
        finally:
            os.close(fd)
        if created:
            _fsync_dir(self.root)

    def remove(self, key, value):
        """Atomically rewrite without the records whose `key` equals `value` (a non-empty
        id or vid).  Unparsable lines are kept.  Returns (removed, remaining)."""
        entries = self.lines()
        kept = [raw for raw, rec in entries if rec is None or rec.get(key) != value]
        removed = len(entries) - len(kept)
        if removed == 0:
            return 0, len(kept)
        tmp = os.path.join(self.root, ".%s.tmp-%s" % (STORE_NAME, secrets.token_hex(8)))
        fd = os.open(tmp, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC, 0o600)
        try:
            with os.fdopen(fd, "wb") as f:
                f.write(b"".join(raw + b"\n" for raw in kept))
                f.flush()
                os.fsync(f.fileno())
            os.replace(tmp, self.path)
        except BaseException:
            try:
                os.unlink(tmp)
            except FileNotFoundError:
                pass
            raise
        _fsync_dir(self.root)
        return removed, len(kept)

    def audit(self, event, removed):
        line = "%s %s removed=%d\n" % (stamp("%Y-%m-%dT%H:%M:%SZ", self.now), event, removed)
        fd = os.open(os.path.join(self.root, AUDIT_NAME),
                     os.O_WRONLY | os.O_APPEND | os.O_CREAT | os.O_CLOEXEC | os.O_NOFOLLOW, 0o600)
        try:
            os.write(fd, line.encode("ascii"))
            os.fsync(fd)
        finally:
            os.close(fd)


# ---------------------------------------------------------------- ingest

def _bump(counts, key):
    counts[key] = counts.get(key, 0) + 1


def _quarantine(store, data, counts):
    qdir = os.path.join(store.root, QUARANTINE)
    try:
        os.mkdir(qdir, 0o700)
    except FileExistsError:
        pass
    _check_private_dir(qdir)
    if len(os.listdir(qdir)) >= QUARANTINE_CAP:
        _bump(counts, "quarantine_full_discarded")
        return
    name = "%s-%s.json" % (stamp("%Y%m%dT%H%M%SZ", store.now), secrets.token_hex(4))
    fd = os.open(os.path.join(qdir, name),
                 os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, "wb") as f:
        f.write(data[:MAX_DROP_BYTES])
        f.flush()
        os.fsync(f.fileno())
    _fsync_dir(qdir)


def _remove_entry(dfd, name, counts, key):
    """Remove an entry without following it: unlink, else rmdir (an empty directory)."""
    try:
        os.unlink(name, dir_fd=dfd)
    except FileNotFoundError:
        return
    except OSError:
        try:
            os.rmdir(name, dir_fd=dfd)
        except OSError:
            _bump(counts, "stuck_nonregular")
            return
    _bump(counts, key)


def _process(dfd, name, store, counts):
    try:
        fd = os.open(name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC,
                     dir_fd=dfd)
    except FileNotFoundError:
        return
    except OSError:
        _remove_entry(dfd, name, counts, "removed_nonregular")
        return
    try:
        st = os.fstat(fd)
        regular = stat.S_ISREG(st.st_mode) and st.st_nlink == 1
        data = b""
        while regular and len(data) <= MAX_DROP_BYTES:
            b = os.read(fd, MAX_DROP_BYTES + 1 - len(data))
            if not b:
                break
            data += b
    finally:
        os.close(fd)
    if not regular:
        _remove_entry(dfd, name, counts, "removed_nonregular")
        return

    reason = None
    rec = None
    if not DROP_NAME.fullmatch(name):
        reason = "q_name"
    elif len(data) > MAX_DROP_BYTES:
        reason = "q_oversize"
    else:
        try:
            rec = parse_drop(data)
        except Invalid:
            reason = "q_invalid"
    if reason is None and rec["type"] == "introduce":
        existing, _ = store.find(rec["id"])
        if existing is None:
            store.append(data)
            _bump(counts, "introduce_stored")
        elif existing == data:
            _bump(counts, "introduce_already_stored")
        else:
            reason = "q_duplicate_id"
    elif reason is None and rec["type"] == "forget":
        _, target = store.find(rec["id"])
        have = target.get("delete_token_sha256") if target else None
        if not (isinstance(have, str) and SHA_RE.fullmatch(have)):
            have = "0" * 64
        match = hmac.compare_digest(rec["token_sha256"].encode("ascii"), have.encode("ascii"))
        if match and target is not None:
            removed, _ = store.remove("id", rec["id"])
            store.audit("forget", removed)
            _bump(counts, "forget_matched")
        else:
            _bump(counts, "forget_unmatched")
    elif reason is None:
        removed, _ = store.remove("vid", rec["vid"])
        if removed:
            store.audit("forget_visitor", removed)
            _bump(counts, "forget_visitor_matched")
        else:
            _bump(counts, "forget_visitor_unmatched")
    if reason is not None:
        _quarantine(store, data, counts)
        _bump(counts, "quarantined")
        _bump(counts, reason)
    os.unlink(name, dir_fd=dfd)


def ingest(drop_dir=None, store_root=None, now=None):
    """One pass over the drop directory.  Dot files are the gateway's in-flight temp files
    (".tmp-<16hex>"); systemd's DirectoryNotEmpty= ignores hidden entries, so they never
    trigger a run, and the rename to "<16hex>.json" triggers the next one.  Temp files older
    than TEMP_STALE_S (a writer that died mid-write) are removed.  Entries are taken in
    (mtime, name) order, at most MAX_ENTRIES per run."""
    drop_dir = drop_dir or DROP_DIR
    _check_private_dir(drop_dir, group_ok=0o030)
    store = Store(store_root, now)
    counts = {}
    dfd = os.open(drop_dir, O_DIR)
    try:
        entries = []
        with os.scandir(dfd) as it:
            for e in it:
                if len(entries) >= MAX_ENTRIES:
                    break
                try:
                    st = e.stat(follow_symlinks=False)
                except FileNotFoundError:
                    continue
                entries.append((st.st_mtime_ns, os.fsencode(e.name)))
        with store:
            for mtime_ns, name in sorted(entries):
                if name.startswith(b"."):
                    if time.time_ns() - mtime_ns > TEMP_STALE_S * 1_000_000_000:
                        _remove_entry(dfd, name, counts, "temp_stale_removed")
                    else:
                        _bump(counts, "temp_pending")
                    continue
                _process(dfd, name, store, counts)
    finally:
        os.close(dfd)
    summary = " ".join("%s=%d" % kv for kv in sorted(counts.items())) or "nothing"
    print("allen-contacts-ingest: " + summary, flush=True)
    return counts


def main(argv):
    if argv == ["ingest"]:
        ingest()
        return 0
    if len(argv) == 4 and argv[0] == "ingest" and argv[3].isdigit():
        ingest(argv[1], argv[2], int(argv[3]))
        return 0
    sys.stderr.write("usage: allen_contacts_ingest.py ingest [DROP STORE EPOCH]\n")
    return 64


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
