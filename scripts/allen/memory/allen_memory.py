"""Visitor memory for public Allen, stored in the isolated cortex-visitors instance.

Stdlib only. Every public method returns a plain value and never raises: a Cortex outage,
timeout, bad reply or bad vid gives [] / 0. Memory text is never logged or put in a URL.

Scoping (cortex-rs 0.1.0 has one bearer token per instance and no hard delete):
- instance: a separate cortex-rs process on loopback with its own token (cortex-visitors),
  so the engineering Cortex and its token are never involved;
- per visitor: space "v-<vid>", alias "vid:<vid>", metadata.vid == vid. Recall re-checks all three
  client-side, so a row is returned only when the server's space filter and the row agree;
- forget: each row is overwritten with empty content/aliases/metadata, then retracted
  (cortex-rs keeps retracted rows; the instance VACUUMs on start so old text leaves the file).
"""
import json
import os
import re
import secrets
import threading
import unicodedata
import urllib.request
from datetime import datetime, timezone
from difflib import SequenceMatcher

VID_RE = re.compile(r'[0-9a-f]{64}')
CAP = 40            # memories kept per visitor; oldest dropped first
PAGE = 50           # cortex-rs search limit ceiling
MAX_ITEMS = 10      # items accepted per remember() call
MAX_CHARS = 300     # per memory
TIMEOUT = 2.0
KIND = 'visitor_note'
_URL_RE = re.compile(r'http://(127\.0\.0\.1|localhost):\d{1,5}')


def valid_vid(vid):
    return isinstance(vid, str) and VID_RE.fullmatch(vid) is not None


def _norm(text):
    t = unicodedata.normalize('NFKC', text).casefold()
    return ' '.join(re.sub(r'[^\w]+', ' ', t).split())


def _near(a, b):
    """a, b are normalized texts. Near-identical: equal, very similar strings, or same word set."""
    if a == b:
        return True
    if SequenceMatcher(None, a, b, autojunk=False).ratio() >= 0.9:
        return True
    wa, wb = set(a.split()), set(b.split())
    if min(len(wa), len(wb)) >= 4 and len(wa & wb) / len(wa | wb) >= 0.8:
        return True
    return False


def _clean_item(x):
    if not isinstance(x, str):
        return ''
    t = ' '.join(x.split())
    return t[:MAX_CHARS].strip()


def _now():
    return datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%S.%fZ')


class Memory:
    def __init__(self, base_url='http://127.0.0.1:18096', token=''):
        base_url = (base_url or '').rstrip('/')
        # Loopback only; an empty token or other URL disables the store instead of raising.
        self.enabled = bool(token) and _URL_RE.fullmatch(base_url) is not None
        self._base = base_url
        self._auth = 'Bearer ' + (token or '')
        self._lock = threading.Lock()
        self._http = urllib.request.build_opener(urllib.request.ProxyHandler({}))

    @classmethod
    def from_env(cls, env=os.environ):
        return cls(env.get('CORTEX_VISITORS_URL', 'http://127.0.0.1:18096'),
                   env.get('CORTEX_VISITORS_TOKEN', ''))

    # ---------- transport ----------
    def _post(self, path, body):
        req = urllib.request.Request(
            self._base + path, data=json.dumps(body).encode(), method='POST',
            headers={'Authorization': self._auth, 'Content-Type': 'application/json'})
        try:
            with self._http.open(req, timeout=TIMEOUT) as r:
                if r.status not in (200, 201):
                    return None
                return json.loads(r.read(4 << 20))
        except Exception:
            return None

    @staticmethod
    def _space(vid):
        return 'v-' + vid

    def _list(self, vid):
        """This visitor's live rows, newest savedAt first, or None when Cortex did not answer."""
        sp = self._space(vid)
        r = self._post('/api/cortex/search', {'q': '', 'space': sp, 'limit': PAGE})
        if not isinstance(r, dict) or not isinstance(r.get('results'), list):
            return None
        tag = 'vid:' + vid
        rows = []
        for e in r['results']:
            if not isinstance(e, dict):
                continue
            meta = e.get('metadata') if isinstance(e.get('metadata'), dict) else {}
            aliases = e.get('aliases') if isinstance(e.get('aliases'), list) else []
            if (e.get('spaceSlug') != sp or e.get('retracted') or tag not in aliases
                    or meta.get('vid') != vid or not isinstance(e.get('content'), str)
                    or not e['content'] or not isinstance(e.get('canonicalName'), str)
                    or not isinstance(e.get('id'), str)):
                continue
            rows.append({'id': e['id'], 'name': e['canonicalName'], 'text': e['content'],
                         'saved': str(meta.get('savedAt', ''))})
        rows.sort(key=lambda w: w['saved'], reverse=True)
        return rows

    def _put(self, vid, name, text):
        r = self._post('/api/cortex/write', {'kind': 'entity', 'value': {
            'space': self._space(vid), 'entityType': KIND, 'canonicalName': name, 'content': text,
            'aliases': ['vid:' + vid], 'metadata': {'vid': vid, 'savedAt': _now()}}})
        return isinstance(r, dict) and r.get('recorded') is True

    def _wipe(self, vid, row):
        r = self._post('/api/cortex/write', {'kind': 'entity', 'value': {
            'space': self._space(vid), 'entityType': 'forgotten', 'canonicalName': row['name'],
            'content': '', 'aliases': [], 'metadata': {}}})
        if not (isinstance(r, dict) and r.get('recorded') is True):
            return False
        target = (r.get('receipt') or {}).get('targetId') or row['id']
        r = self._post('/api/cortex/write', {'kind': 'retract', 'value': {
            'targetType': 'entity', 'targetId': target}})
        return isinstance(r, dict) and r.get('recorded') is True

    # ---------- public API ----------
    def recall(self, vid, limit=8):
        if not self.enabled or not valid_vid(vid):
            return []
        try:
            n = max(0, min(int(limit), CAP))
            rows = self._list(vid) or []
            return [w['text'] for w in rows[:n]]
        except Exception:
            return []

    def remember(self, vid, items):
        """Store short memories for vid. Near-identical text updates the existing entry.
        Returns the number of entries written (new or updated)."""
        if not self.enabled or not valid_vid(vid) or not isinstance(items, (list, tuple)):
            return 0
        try:
            with self._lock:
                rows = self._list(vid)
                if rows is None:
                    return 0
                known = [(_norm(w['text']), w) for w in rows]
                written = 0
                for raw in list(items)[:MAX_ITEMS]:
                    text = _clean_item(raw)
                    n = _norm(text)
                    if not n:
                        continue
                    hit = next((w for k, w in known if _near(k, n)), None)
                    name = hit['name'] if hit else 'm-' + secrets.token_hex(16)
                    if self._put(vid, name, text):
                        written += 1
                        if hit:
                            known = [(k, w) for k, w in known if w is not hit]
                        known.append((n, {'name': name, 'text': text}))
                self._trim(vid)
                return written
        except Exception:
            return 0

    def _trim(self, vid):
        for _ in range(4):
            rows = self._list(vid)
            if rows is None or len(rows) <= CAP:
                return
            for w in rows[CAP:]:
                self._wipe(vid, w)

    def forget(self, vid):
        """Delete every entry for vid. Returns how many were deleted, only after a re-list shows
        none remain; 0 when nothing was stored or the deletion could not be verified."""
        if not self.enabled or not valid_vid(vid):
            return 0
        try:
            with self._lock:
                deleted = 0
                for _ in range(10):
                    rows = self._list(vid)
                    if rows is None:
                        return 0
                    if not rows:
                        return deleted
                    for w in rows:
                        if self._wipe(vid, w):
                            deleted += 1
                return 0
        except Exception:
            return 0
