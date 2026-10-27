#!/usr/bin/python3
"""community: the aienos.com Community backend (INTERIM host program).

Serves /community/api/* and /community/feed/* behind the hub's nginx on 127.0.0.1:18130.
CPython 3 standard library plus sqlite3 only (runs on the hub's Python 3.14 and the
Spark's 3.12). Its `.om` successor is `community.om` on omega-mv's seed/net/http.om and
seed/json/json.om with an append-only log store; see scripts/allen/DEPENDENCIES.md,
section "Community (/community/)", for the blocking Omega features.

The contract is /home/drakestapleton/handoffs/Community/DESIGN.md ("Backend contract" and
"AUTH REVISION 2026-10-27"). Post HTML comes ONLY from the seeded Omega renderer
(om/community-render/render): this program never renders text itself. It runs the binary
once per post at write time and stores the HTML and the fd 3 metadata (tags, mentions,
links, code languages).

Identity is passkeys (WebAuthn) plus an invite tree. This program parses the ceremony
(strict CBOR, clientDataJSON, authenticator data, COSE keys) and checks challenge, origin,
RP ID hash, flags and sign counts; every signature and public key check is done by the
seeded Omega verifier (om/community-verify/verify). GitHub OAuth is optional (link a
"Verified GitHub" badge, sign in, or sign up with an established GitHub account): the
client secret arrives once on stdin (--secret-stdin) and is held in memory only; the
GitHub user token is discarded (and revoked in the background) right after /user is read.

  community.py --db /var/lib/aien-community/community.db --render /opt/aien-community/render \
      --verify /opt/aien-community/verify [--github-client-id Iv1.xxxx --secret-stdin] [--aien]
  community.py admin-claim --db /var/lib/aien-community/community.db
"""

import argparse
import base64
import calendar
import fcntl
import hashlib
import hmac
import http.client
import http.server
import json
import os
import re
import secrets
import select
import signal
import socket
import sqlite3
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.parse
import urllib.request

# ---------------------------------------------------------------- constants

RESERVED = {"aien", "admin", "mod", "moderator", "community", "system", "root", "support",
            "drake", "aienos", "api", "help", "staff", "official"}
USER_HANDLE = re.compile(r"^[a-z][a-z0-9-]{2,19}$")
ORIGINS = ("https://www.aienos.com", "https://aienos.com")
GITHUB_WEB = "https://github.com"
GITHUB_API = "https://api.github.com"
STUB_BASE = re.compile(r"^http://127\.0\.0\.1:[0-9]{1,5}$")
AVATAR_PREFIX = "https://avatars.githubusercontent.com/"
PROJECTS_REPO = "aien-dev/omega-projects"
PROJECT_PATH = re.compile(r"^projects/([a-z0-9-]{1,64})/([a-z0-9-]{1,64})(?:/|$)")
PROJECT_REF = re.compile(r"^([a-z0-9-]{1,64})/([a-z0-9-]{1,64})$")
HANDLE = re.compile(r"^[a-z0-9][a-z0-9_-]{0,43}$")
LOGIN = re.compile(r"^[A-Za-z0-9](?:[A-Za-z0-9-]{0,38})$")
BOT_NAME = re.compile(r"^[a-z0-9][a-z0-9-]{0,34}$")
TAG = re.compile(r"^[a-z][a-z0-9_]{0,31}$")
META_MENTION = re.compile(r"^[a-z0-9][a-z0-9_-]{0,38}$")
META_LANG = re.compile(r"^[a-z0-9+#-]{0,16}$")
META_LINK = re.compile(r"^(?i:https?)://[A-Za-z0-9\-._~:/?#\[\]@!$&'()*+,;=%]+$")
NEXT_PATH = re.compile(r"^/community/[A-Za-z0-9/_.?=&%~-]{0,200}$")
SESSION_DAYS = 30
STATE_SECONDS = 600
DAY = 86400
FEATURED_TAGS = ("omega", "cudapp", "opencompute", "gb10", "showcase", "aienos")
# tier -> (posts per day, links per post, mentions per post)
LIMITS = {"probation": (8, 1, 3), "member": (100, 4, 8), "bot": (48, 4, 8), "system": (1000, 16, 16)}
SHORTENERS = {"bit.ly", "t.co", "tinyurl.com", "goo.gl", "ow.ly", "is.gd", "buff.ly", "cutt.ly",
              "rebrand.ly", "shorturl.at", "tiny.cc", "rb.gy", "bl.ink", "t.ly", "s.id", "lnkd.in",
              "shorte.st", "adf.ly", "bit.do", "v.gd", "qr.ae", "trib.al", "dlvr.it", "tiny.one",
              "shorturl.com", "soo.gd", "clck.ru", "y2u.be", "surl.li", "u.to"}
FLOOD_EXEMPT = {"aienos.com", "github.com"}
FLOOD_POSTS = 3  # earlier posts in 24 h linking one domain; the next one is held
REPORT_REASONS = ("spam", "harassment", "illegal", "impersonation", "other")
MOD_ACTIONS = ("hide", "unhide", "dismiss", "suspend", "unsuspend", "set_tier", "set_role",
               "restore_branch", "unfreeze_invites")
SUSPEND_CAUSES = ("spam", "abuse", "other")
# role -> (open invites, invites created per rolling 7 days; None = unlimited)
INVITE_LIMITS = {"member": (3, 5), "moderator": (20, 50), "admin": (100, None)}
INVITE_DAYS = 14
CLAIM_HOURS = 48
INVITE_TRIES = 5
REVOKE_WINDOW = 90 * 86400
CROCKFORD = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"
CODE_SHAPE = re.compile(r"^[0-9A-HJKMNP-TV-Z]{20}$")
GITHUB_SIGNUP_AGE = 365 * 86400
PENDING_SECONDS = 600
MAX_PASSKEYS = 10
MAX_BODY = 16 * 1024
MAX_TEXT = 4000
AIEN_MAX_REPLY = 600
AIEN_PER_HOUR = 6

ALLOWED_TAG = re.compile(
    r"<p>|</p>|<br>|<code>|</code>|</pre>|</a>"
    r'|<pre class="cm-code"(?: data-lang="[a-z0-9+#-]{1,16}")?>'
    r'|<a href="(?i:https?)://[A-Za-z0-9\-._~:/?#\[\]@!$&;()*+,=%]+" '
    r'rel="nofollow ugc noopener noreferrer" target="_blank">'
    r'|<a class="cm-tag" href="/community/\?tag=[a-z][a-z0-9_]{0,31}">'
    r'|<a class="cm-at" href="/community/\?u=[a-z0-9][a-z0-9_-]{0,38}">')

SCHEMA = """
CREATE TABLE IF NOT EXISTS meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS accounts(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  handle TEXT NOT NULL UNIQUE,
  display_name TEXT NOT NULL DEFAULT '',
  bio TEXT NOT NULL DEFAULT '',
  projects TEXT NOT NULL DEFAULT '[]',
  joined INTEGER NOT NULL,
  inviter_id INTEGER,
  tier_override TEXT,
  role TEXT NOT NULL DEFAULT 'member',
  suspended INTEGER NOT NULL DEFAULT 0,
  suspend_cause TEXT NOT NULL DEFAULT '',
  suspended_at INTEGER,
  vouch_revoked INTEGER NOT NULL DEFAULT 0,
  invites_frozen INTEGER NOT NULL DEFAULT 0,
  webauthn_user_id BLOB,
  github_id INTEGER UNIQUE,
  github_login TEXT,
  avatar TEXT NOT NULL DEFAULT '',
  is_bot INTEGER NOT NULL DEFAULT 0,
  bot_owner INTEGER,
  bot_token TEXT UNIQUE,
  label TEXT NOT NULL DEFAULT '');
CREATE INDEX IF NOT EXISTS accounts_owner ON accounts(bot_owner);
CREATE INDEX IF NOT EXISTS accounts_inviter ON accounts(inviter_id);
CREATE TABLE IF NOT EXISTS sessions(token TEXT PRIMARY KEY, account_id INTEGER NOT NULL,
  csrf TEXT NOT NULL, created INTEGER NOT NULL, expires INTEGER NOT NULL);
CREATE INDEX IF NOT EXISTS sessions_account ON sessions(account_id);
CREATE TABLE IF NOT EXISTS credentials(id INTEGER PRIMARY KEY AUTOINCREMENT, cred_id TEXT NOT NULL UNIQUE,
  account_id INTEGER NOT NULL, alg INTEGER NOT NULL, pub BLOB NOT NULL, sign_count INTEGER NOT NULL,
  transports TEXT NOT NULL DEFAULT '[]', name TEXT NOT NULL DEFAULT '', created INTEGER NOT NULL,
  last_used INTEGER, flagged INTEGER NOT NULL DEFAULT 0);
CREATE INDEX IF NOT EXISTS credentials_account ON credentials(account_id);
CREATE TABLE IF NOT EXISTS challenges(hash TEXT PRIMARY KEY, cookie TEXT NOT NULL, kind TEXT NOT NULL,
  account_id INTEGER, data TEXT NOT NULL, created INTEGER NOT NULL);
CREATE INDEX IF NOT EXISTS challenges_cookie ON challenges(cookie);
CREATE TABLE IF NOT EXISTS invites(id INTEGER PRIMARY KEY AUTOINCREMENT, kind TEXT NOT NULL,
  code_hash TEXT UNIQUE, inviter_id INTEGER NOT NULL, note TEXT NOT NULL DEFAULT '', created INTEGER NOT NULL,
  expires INTEGER NOT NULL, used_by INTEGER, used_at INTEGER, cancelled INTEGER NOT NULL DEFAULT 0,
  tries INTEGER NOT NULL DEFAULT 0);
CREATE INDEX IF NOT EXISTS invites_inviter ON invites(inviter_id, created);
CREATE TABLE IF NOT EXISTS oauth_states(state TEXT PRIMARY KEY, next TEXT NOT NULL, created INTEGER NOT NULL,
  mode TEXT NOT NULL, account_id INTEGER);
CREATE TABLE IF NOT EXISTS github_pending(token TEXT PRIMARY KEY, profile TEXT NOT NULL, created INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS posts(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  author_id INTEGER NOT NULL,
  text TEXT NOT NULL,
  html TEXT NOT NULL,
  text_hash TEXT NOT NULL,
  created INTEGER NOT NULL,
  reply_to INTEGER,
  root_id INTEGER,
  quote_of INTEGER,
  project TEXT,
  hidden INTEGER NOT NULL DEFAULT 0,
  tags TEXT NOT NULL DEFAULT '[]',
  links TEXT NOT NULL DEFAULT '[]',
  domains TEXT NOT NULL DEFAULT '[]',
  code_langs TEXT NOT NULL DEFAULT '[]');
CREATE INDEX IF NOT EXISTS posts_author ON posts(author_id, created);
CREATE INDEX IF NOT EXISTS posts_reply ON posts(reply_to);
CREATE INDEX IF NOT EXISTS posts_quote ON posts(quote_of);
CREATE TABLE IF NOT EXISTS post_tags(post_id INTEGER NOT NULL, tag TEXT NOT NULL, created INTEGER NOT NULL,
  PRIMARY KEY(post_id, tag));
CREATE INDEX IF NOT EXISTS post_tags_tag ON post_tags(tag, post_id);
CREATE TABLE IF NOT EXISTS post_mentions(post_id INTEGER NOT NULL, account_id INTEGER NOT NULL,
  PRIMARY KEY(post_id, account_id));
CREATE INDEX IF NOT EXISTS post_mentions_account ON post_mentions(account_id);
CREATE TABLE IF NOT EXISTS feed(id INTEGER PRIMARY KEY AUTOINCREMENT, kind TEXT NOT NULL,
  post_id INTEGER NOT NULL, actor_id INTEGER NOT NULL, created INTEGER NOT NULL, top INTEGER NOT NULL);
CREATE INDEX IF NOT EXISTS feed_actor ON feed(actor_id, id);
CREATE INDEX IF NOT EXISTS feed_post ON feed(post_id);
CREATE TABLE IF NOT EXISTS likes(account_id INTEGER NOT NULL, post_id INTEGER NOT NULL, created INTEGER NOT NULL,
  PRIMARY KEY(account_id, post_id));
CREATE INDEX IF NOT EXISTS likes_post ON likes(post_id);
CREATE TABLE IF NOT EXISTS reposts(account_id INTEGER NOT NULL, post_id INTEGER NOT NULL, created INTEGER NOT NULL,
  PRIMARY KEY(account_id, post_id));
CREATE INDEX IF NOT EXISTS reposts_post ON reposts(post_id);
CREATE TABLE IF NOT EXISTS follows(follower INTEGER NOT NULL, followee INTEGER NOT NULL, created INTEGER NOT NULL,
  UNIQUE(follower, followee));
CREATE INDEX IF NOT EXISTS follows_followee ON follows(followee);
CREATE TABLE IF NOT EXISTS mutes(account_id INTEGER NOT NULL, target_id INTEGER NOT NULL, created INTEGER NOT NULL,
  PRIMARY KEY(account_id, target_id));
CREATE INDEX IF NOT EXISTS mutes_target ON mutes(target_id);
CREATE TABLE IF NOT EXISTS blocks(account_id INTEGER NOT NULL, target_id INTEGER NOT NULL, created INTEGER NOT NULL,
  PRIMARY KEY(account_id, target_id));
CREATE INDEX IF NOT EXISTS blocks_target ON blocks(target_id);
CREATE TABLE IF NOT EXISTS notifications(id INTEGER PRIMARY KEY AUTOINCREMENT, account_id INTEGER NOT NULL,
  kind TEXT NOT NULL, actor_id INTEGER NOT NULL, post_id INTEGER NOT NULL DEFAULT 0, created INTEGER NOT NULL,
  read INTEGER NOT NULL DEFAULT 0, UNIQUE(account_id, kind, actor_id, post_id));
CREATE INDEX IF NOT EXISTS notifications_account ON notifications(account_id, id);
CREATE INDEX IF NOT EXISTS notifications_actor ON notifications(actor_id);
CREATE TABLE IF NOT EXISTS reports(id INTEGER PRIMARY KEY AUTOINCREMENT, reporter_id INTEGER,
  target_kind TEXT NOT NULL, post_id INTEGER, account_id INTEGER, reason TEXT NOT NULL,
  note TEXT NOT NULL DEFAULT '', created INTEGER NOT NULL, status TEXT NOT NULL DEFAULT 'open');
CREATE INDEX IF NOT EXISTS reports_status ON reports(status, id);
CREATE TABLE IF NOT EXISTS modlog(id INTEGER PRIMARY KEY AUTOINCREMENT, at INTEGER NOT NULL, action TEXT NOT NULL,
  target_kind TEXT NOT NULL, target_post INTEGER, target_account INTEGER, value TEXT NOT NULL DEFAULT '',
  reason_public TEXT NOT NULL DEFAULT '', note_private TEXT NOT NULL DEFAULT '', by_id INTEGER,
  by_handle TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS bans(hash TEXT PRIMARY KEY, created INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS aien_jobs(id INTEGER PRIMARY KEY AUTOINCREMENT, post_id INTEGER NOT NULL,
  root_id INTEGER NOT NULL, created INTEGER NOT NULL, state TEXT NOT NULL DEFAULT 'queued',
  reason TEXT NOT NULL DEFAULT '', reply_id INTEGER, done_at INTEGER);
CREATE INDEX IF NOT EXISTS aien_jobs_state ON aien_jobs(state, id);
CREATE VIRTUAL TABLE IF NOT EXISTS posts_fts USING fts5(text, content='posts', content_rowid='id');
CREATE TRIGGER IF NOT EXISTS posts_ai AFTER INSERT ON posts BEGIN
  INSERT INTO posts_fts(rowid, text) VALUES (new.id, new.text); END;
CREATE TRIGGER IF NOT EXISTS posts_ad AFTER DELETE ON posts BEGIN
  INSERT INTO posts_fts(posts_fts, rowid, text) VALUES ('delete', old.id, old.text); END;
CREATE TRIGGER IF NOT EXISTS posts_au AFTER UPDATE OF text ON posts BEGIN
  INSERT INTO posts_fts(posts_fts, rowid, text) VALUES ('delete', old.id, old.text);
  INSERT INTO posts_fts(rowid, text) VALUES (new.id, new.text); END;
"""


class ApiError(Exception):
    def __init__(self, status, code, message, **extra):
        super().__init__(message)
        self.status, self.code, self.message, self.extra = status, code, message, extra


def iso(t):
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(int(t))) if t is not None else None


def parse_iso(s):
    try:
        return calendar.timegm(time.strptime(str(s), "%Y-%m-%dT%H:%M:%SZ"))
    except (ValueError, TypeError):
        return None


def sha256(s):
    return hashlib.sha256(s.encode() if isinstance(s, str) else s).hexdigest()


def clean_line(s, limit, multiline=False):
    """User-chosen display text: no C0 controls (LF kept for bios), no DEL, bounded length."""
    if not isinstance(s, str):
        raise ApiError(400, "bad_request", "Expected text.")
    s = s.replace("\r\n", "\n").replace("\r", "\n")
    out = "".join(ch for ch in s if (ch >= " " and ch != "\x7f") or (multiline and ch == "\n"))
    out = out.strip()
    if len(out) > limit:
        raise ApiError(400, "too_long", "That is longer than %d characters." % limit)
    return out


def normalize_post(text):
    """The renderer's input rules: UTF-8, <= 4000 bytes, CRLF/CR -> LF, no C0 except LF/TAB, no DEL."""
    if not isinstance(text, str):
        raise ApiError(400, "bad_request", "Send the post text as a string.")
    text = text.replace("\r\n", "\n").replace("\r", "\n")
    for ch in text:
        if (ch < " " and ch not in "\n\t") or ch == "\x7f":
            raise ApiError(400, "bad_text", "The text contains a control character.")
    try:
        raw = text.encode("utf-8")
    except UnicodeEncodeError:
        raise ApiError(400, "bad_text", "The text is not valid UTF-8.")
    if not text.strip():
        raise ApiError(400, "empty", "Write something first.")
    if len(raw) > MAX_TEXT:
        raise ApiError(413, "too_long", "A post holds at most %d bytes." % MAX_TEXT)
    return text, raw


def text_fingerprint(text):
    return sha256(" ".join(text.casefold().split()))


def link_domain(url):
    try:
        host = (urllib.parse.urlsplit(url).hostname or "").lower()
    except ValueError:
        return ""
    return host[4:] if host.startswith("www.") else host


def fts_query(q):
    """Every term becomes one quoted FTS5 string, so operators and syntax characters are inert."""
    terms = [t for t in q.split() if t][:8]
    return " ".join('"' + t.replace('"', '""') + '"' for t in terms)


def new_code():
    """20 Crockford base32 characters = 100 random bits."""
    v = secrets.randbits(100)
    return "".join(CROCKFORD[(v >> (5 * i)) & 31] for i in range(20))


def norm_code(s):
    """Typed code -> canonical form (case, dashes and spaces ignored; I/L read as 1, O as 0)."""
    if not isinstance(s, str) or len(s) > 64:
        return None
    s = s.strip().upper().replace("-", "").replace(" ", "").translate(str.maketrans("ILO", "110"))
    return s if CODE_SHAPE.match(s) else None


# ---------------------------------------------------------------- WebAuthn
#
# Parsing and policy only. Signatures and public keys are checked by the Omega verifier
# (om/community-verify/verify), never here.

RP_ID = "aienos.com"
RP_NAME = "aienos.com Community"
RP_ID_HASH = hashlib.sha256(RP_ID.encode()).digest()
CEREMONY_SECONDS = 300
FLAG_UP, FLAG_UV, FLAG_AT, FLAG_ED = 0x01, 0x04, 0x40, 0x80
TRANSPORTS = ("usb", "nfc", "ble", "internal", "hybrid", "smart-card")
ALGS = (-7, -257)
B64U = re.compile(r"^[A-Za-z0-9_-]+$")


class WebAuthnError(Exception):
    """A ceremony failed a check. The detail is for tests and stderr, never for the client."""


class VerifierUnavailable(Exception):
    pass


def b64u(b):
    return base64.urlsafe_b64encode(b).rstrip(b"=").decode()


def b64u_decode(s, limit=8192):
    """Strict base64url: alphabet only, no padding, canonical trailing bits."""
    if not isinstance(s, str) or not s or len(s) > limit or len(s) % 4 == 1 or not B64U.match(s):
        raise WebAuthnError("base64url")
    raw = base64.urlsafe_b64decode(s + "=" * (-len(s) % 4))
    if b64u(raw) != s:
        raise WebAuthnError("base64url not canonical")
    return raw


class _Cbor:
    """CTAP2-style strict CBOR: definite lengths, minimal heads, no tags, no floats, no
    duplicate map keys, int/text map keys, bounded depth and item count."""

    def __init__(self, data, pos, max_depth=6, max_items=512):
        self.d, self.i, self.max_depth, self.items = data, pos, max_depth, max_items

    def take(self, n):
        if self.i + n > len(self.d):
            raise WebAuthnError("cbor truncated")
        b = self.d[self.i:self.i + n]
        self.i += n
        return b

    def head(self, ai):
        if ai < 24:
            return ai
        if ai > 27:
            raise WebAuthnError("cbor indefinite or reserved length")
        n = 1 << (ai - 24)
        v = int.from_bytes(self.take(n), "big")
        if v < (24 if n == 1 else 1 << (4 * n)):
            raise WebAuthnError("cbor non-minimal head")
        return v

    def item(self, depth=0):
        if depth > self.max_depth:
            raise WebAuthnError("cbor too deep")
        self.items -= 1
        if self.items < 0:
            raise WebAuthnError("cbor too many items")
        ib = self.take(1)[0]
        major, ai = ib >> 5, ib & 31
        if major == 7:
            if ai in (20, 21, 22):
                return (False, True, None)[ai - 20]
            raise WebAuthnError("cbor simple/float value")
        if major == 6:
            raise WebAuthnError("cbor tag")
        n = self.head(ai)
        if major == 0:
            return n
        if major == 1:
            return -1 - n
        if major == 2:
            return bytes(self.take(n))
        if major == 3:
            try:
                return bytes(self.take(n)).decode("utf-8")
            except UnicodeDecodeError:
                raise WebAuthnError("cbor text not UTF-8")
        if n > self.items:
            raise WebAuthnError("cbor too many items")
        if major == 4:
            return [self.item(depth + 1) for _ in range(n)]
        m = {}
        for _ in range(n):
            k = self.item(depth + 1)
            if isinstance(k, bool) or not isinstance(k, (int, str)):
                raise WebAuthnError("cbor map key type")
            if k in m:
                raise WebAuthnError("cbor duplicate map key")
            m[k] = self.item(depth + 1)
        return m


def cbor_one(data, pos=0):
    """Decode one item at pos -> (value, end)."""
    c = _Cbor(data, pos)
    v = c.item()
    return v, c.i


def cbor_all(data, limit=16384):
    if len(data) > limit:
        raise WebAuthnError("cbor too large")
    v, end = cbor_one(data)
    if end != len(data):
        raise WebAuthnError("cbor trailing bytes")
    return v


def parse_auth_data(ad, create):
    """Authenticator data -> dict(rp, flags, count[, cred_id, cose])."""
    if not isinstance(ad, (bytes, bytearray)) or len(ad) < 37 or len(ad) > 4096:
        raise WebAuthnError("authData size")
    ad = bytes(ad)
    out = {"rp": ad[:32], "flags": ad[32], "count": int.from_bytes(ad[33:37], "big")}
    i = 37
    if ad[32] & FLAG_AT:
        if not create:
            raise WebAuthnError("attested data in an assertion")
        if len(ad) < i + 18:
            raise WebAuthnError("authData truncated")
        i += 16
        n = int.from_bytes(ad[i:i + 2], "big")
        i += 2
        if n < 16 or n > 1023 or len(ad) < i + n:
            raise WebAuthnError("credential id length")
        out["cred_id"] = ad[i:i + n]
        i += n
        out["cose"], i = cbor_one(ad, i)
    elif create:
        raise WebAuthnError("no attested credential")
    if ad[32] & FLAG_ED:
        ext, i = cbor_one(ad, i)
        if not isinstance(ext, dict):
            raise WebAuthnError("extensions not a map")
    if i != len(ad):
        raise WebAuthnError("authData trailing bytes")
    return out


def _der_len(n):
    if n < 128:
        return bytes([n])
    b = n.to_bytes((n.bit_length() + 7) // 8, "big")
    return bytes([0x80 | len(b)]) + b


def _der_int(v):
    b = v.to_bytes(v.bit_length() // 8 + 1, "big")
    return b"\x02" + _der_len(len(b)) + b


def cose_key(k):
    """COSE_Key -> (alg, verifier pub bytes). ES256: SEC1 04||X||Y. RS256: DER RSAPublicKey."""
    if not isinstance(k, dict):
        raise WebAuthnError("COSE key not a map")
    if k.get(1) == 2 and k.get(3) == -7:
        if set(k) != {1, 3, -1, -2, -3} or k[-1] != 1:
            raise WebAuthnError("ES256 key fields")
        x, y = k[-2], k[-3]
        if not isinstance(x, bytes) or not isinstance(y, bytes) or len(x) != 32 or len(y) != 32:
            raise WebAuthnError("ES256 coordinates")
        return -7, b"\x04" + x + y
    if k.get(1) == 3 and k.get(3) == -257:
        if set(k) != {1, 3, -1, -2} or not isinstance(k[-1], bytes) or not isinstance(k[-2], bytes):
            raise WebAuthnError("RS256 key fields")
        n, e = int.from_bytes(k[-1], "big"), int.from_bytes(k[-2], "big")
        if not 2048 <= n.bit_length() <= 4096 or e < 3 or e % 2 == 0 or len(k[-2]) > 8:
            raise WebAuthnError("RS256 modulus or exponent")
        body = _der_int(n) + _der_int(e)
        return -257, b"\x30" + _der_len(len(body)) + body
    raise WebAuthnError("unsupported key type")


def _no_dup_pairs(pairs):
    d = {}
    for k, v in pairs:
        if k in d:
            raise WebAuthnError("clientDataJSON duplicate key")
        d[k] = v
    return d


def parse_client_data(raw, kind, origins):
    """clientDataJSON -> challenge bytes, after the type, origin and crossOrigin checks."""
    if len(raw) > 4096:
        raise WebAuthnError("clientDataJSON size")
    try:
        cd = json.loads(raw.decode("utf-8"), object_pairs_hook=_no_dup_pairs)
    except (ValueError, UnicodeDecodeError, RecursionError):
        raise WebAuthnError("clientDataJSON not JSON")
    if not isinstance(cd, dict) or cd.get("type") != kind:
        raise WebAuthnError("clientData type")
    if not isinstance(cd.get("origin"), str) or cd["origin"] not in origins:
        raise WebAuthnError("clientData origin")
    if cd.get("crossOrigin", False) is not False:
        raise WebAuthnError("clientData crossOrigin")
    ch = b64u_decode(cd.get("challenge"), 128)
    if len(ch) != 32:
        raise WebAuthnError("challenge length")
    return ch


def credential_parts(cred, create):
    """The PublicKeyCredential JSON -> (raw id, clientDataJSON, rest dict of decoded fields)."""
    if not isinstance(cred, dict) or cred.get("type") != "public-key":
        raise WebAuthnError("credential type")
    raw_id = b64u_decode(cred.get("rawId"), 1400)
    if b64u_decode(cred.get("id"), 1400) != raw_id or not 16 <= len(raw_id) <= 1023:
        raise WebAuthnError("credential id")
    resp = cred.get("response")
    if not isinstance(resp, dict):
        raise WebAuthnError("response")
    cdj = b64u_decode(resp.get("clientDataJSON"), 6000)
    if create:
        rest = {"attestationObject": b64u_decode(resp.get("attestationObject"), 12000)}
        tr = cred.get("transports", resp.get("transports", []))
        if not isinstance(tr, list) or len(tr) > 6:
            raise WebAuthnError("transports")
        rest["transports"] = [t for t in tr if t in TRANSPORTS]
    else:
        rest = {"authenticatorData": b64u_decode(resp.get("authenticatorData"), 6000),
                "signature": b64u_decode(resp.get("signature"), 1400)}
        uh = resp.get("userHandle")
        rest["userHandle"] = None if uh in (None, "") else b64u_decode(uh, 128)
    return raw_id, cdj, rest


def parse_attestation(att_obj):
    """attestationObject (fmt/attStmt ignored: attestation "none" policy) -> authData dict."""
    att = cbor_all(att_obj)
    if not isinstance(att, dict) or set(att) != {"fmt", "attStmt", "authData"} \
            or not isinstance(att["fmt"], str) or not isinstance(att["attStmt"], dict) \
            or not isinstance(att["authData"], bytes):
        raise WebAuthnError("attestationObject shape")
    return parse_auth_data(att["authData"], True)


def check_flags(ad, create):
    if not hmac.compare_digest(ad["rp"], RP_ID_HASH):
        raise WebAuthnError("rpIdHash")
    need = FLAG_UP | FLAG_UV | (FLAG_AT if create else 0)
    if ad["flags"] & need != need:
        raise WebAuthnError("flags UP/UV" + ("/AT" if create else ""))


def run_verify(path, alg, pub, msg=None, sig=None, timeout=5.0):
    """The Omega verifier. Returns its exit code (0 = valid); raises VerifierUnavailable."""
    lines = ["alg %d" % alg, "pub " + pub.hex()]
    argv = [path]
    if msg is None:
        argv.append("--key")
    else:
        lines += ["msg " + msg.hex(), "sig " + sig.hex()]
    try:
        r = subprocess.run(argv, input=("\n".join(lines) + "\n").encode(), stdout=subprocess.DEVNULL,
                           stderr=subprocess.DEVNULL, timeout=timeout, env={"LANG": "C"}, close_fds=True)
    except (OSError, subprocess.SubprocessError):
        raise VerifierUnavailable()
    if r.returncode < 0:
        raise VerifierUnavailable()
    return r.returncode


# ---------------------------------------------------------------- renderer

class RenderError(Exception):
    def __init__(self, status):
        super().__init__(status)
        self.status = status


def _high_fd(fd):
    """Move a pipe end above 9 so a posix_spawn dup2 onto 0/1/3 never sees a same-number fd."""
    new = fcntl.fcntl(fd, fcntl.F_DUPFD_CLOEXEC, 10)
    os.close(fd)
    return new


def run_render(path, data, timeout=3.0, cap=1 << 18):
    """render < data > html 3> meta. Returns (exit code, html bytes, meta bytes)."""
    fds = []
    try:
        in_r, in_w = (_high_fd(f) for f in os.pipe())
        fds += [in_r, in_w]
        out_r, out_w = (_high_fd(f) for f in os.pipe())
        fds += [out_r, out_w]
        meta_r, meta_w = (_high_fd(f) for f in os.pipe())
        fds += [meta_r, meta_w]
        pid = os.posix_spawn(path, [path], {"LANG": "C"}, file_actions=[
            (os.POSIX_SPAWN_DUP2, in_r, 0), (os.POSIX_SPAWN_DUP2, out_w, 1),
            (os.POSIX_SPAWN_OPEN, 2, "/dev/null", os.O_WRONLY, 0),
            (os.POSIX_SPAWN_DUP2, meta_w, 3)])
    except OSError:
        for f in fds:
            os.close(f)
        raise RenderError("unavailable")
    for f in (in_r, out_w, meta_w):
        os.close(f)
    deadline = time.monotonic() + timeout
    bufs = {out_r: bytearray(), meta_r: bytearray()}
    pending_in = memoryview(bytes(data))
    os.set_blocking(in_w, False)
    open_r = {out_r, meta_r}
    killed = False
    try:
        while open_r or in_w is not None:
            left = deadline - time.monotonic()
            if left <= 0:
                killed = True
                break
            wl = [in_w] if in_w is not None else []
            rr, ww, _ = select.select(list(open_r), wl, [], left)
            if ww:
                try:
                    n = os.write(in_w, pending_in[:65536])
                    pending_in = pending_in[n:]
                except (BrokenPipeError, BlockingIOError):
                    pending_in = pending_in[:0] if not isinstance(sys.exc_info()[1], BlockingIOError) else pending_in
                if not pending_in:
                    os.close(in_w)
                    in_w = None
            for f in rr:
                chunk = os.read(f, 65536)
                if not chunk:
                    open_r.discard(f)
                    continue
                bufs[f] += chunk
                if len(bufs[f]) > cap:
                    killed = True
                    open_r.clear()
                    break
            if killed:
                break
    finally:
        if in_w is not None:
            os.close(in_w)
        for f in (out_r, meta_r):
            os.close(f)
    if killed:
        try:
            os.kill(pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    while True:
        done, status = os.waitpid(pid, os.WNOHANG)
        if done:
            break
        if time.monotonic() > deadline + 1:
            try:
                os.kill(pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            killed = True
            os.waitpid(pid, 0)
            status = 9
            break
        time.sleep(0.005)
    if killed:
        raise RenderError("timeout")
    code = os.waitstatus_to_exitcode(status)
    return code, bytes(bufs[out_r]), bytes(bufs[meta_r])


def render_post(path, raw):
    """Run the Omega renderer, check its output shape, parse fd 3. Returns (html, meta dict)."""
    code, out, meta = run_render(path, raw)
    if code == 2:
        raise ApiError(413, "too_long", "A post holds at most %d bytes." % MAX_TEXT)
    if code == 3:
        raise ApiError(400, "bad_text", "The text is not valid UTF-8.")
    if code != 0:
        raise RenderError("exit %d" % code)
    try:
        html = out.decode("utf-8")
        meta_text = meta.decode("utf-8")
    except UnicodeDecodeError:
        raise RenderError("output not UTF-8")
    rest = ALLOWED_TAG.sub("", html)
    if "<" in rest or ">" in rest or '"' in rest:
        raise RenderError("output outside the allowed tag set")
    m = {"tags": [], "mentions": [], "links": [], "langs": []}
    for line in meta_text.split("\n"):
        if not line:
            continue
        kind, _, value = line.partition(" ")
        if kind == "t" and TAG.match(value) and value not in m["tags"]:
            m["tags"].append(value)
        elif kind == "m" and META_MENTION.match(value) and value not in m["mentions"]:
            m["mentions"].append(value)
        elif kind == "l" and META_LINK.match(value) and value not in m["links"]:
            m["links"].append(value)
        elif kind == "c" and META_LANG.match(value):
            m["langs"].append(value)
        else:
            raise RenderError("metadata line outside the contract")
    return html, m


# ---------------------------------------------------------------- GitHub

class _NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


class GitHub:
    """OAuth exchange, /user, token revoke, public user refresh and the omega-projects tree."""

    def __init__(self, web, api, client_id, secret):
        for base, real in ((web, GITHUB_WEB), (api, GITHUB_API)):
            if base != real and not STUB_BASE.match(base):
                raise ValueError("refusing GitHub base %r" % base)
        self.web, self.api, self.client_id = web, api, client_id
        self._secret = secret
        self._open = urllib.request.build_opener(_NoRedirect())

    @property
    def ready(self):
        return bool(self.client_id and self._secret)

    def _call(self, method, url, data=None, headers=(), timeout=10):
        req = urllib.request.Request(url, data=data, method=method)
        req.add_header("User-Agent", "aienos-community")
        for k, v in headers:
            req.add_header(k, v)
        try:
            with self._open.open(req, timeout=timeout) as r:
                status, raw = r.status, r.read(1 << 23)
        except urllib.error.HTTPError as e:
            status, raw = e.code, e.read(1 << 16)
        except (OSError, http.client.HTTPException):
            return None, None
        try:
            return status, json.loads(raw) if raw else None
        except ValueError:
            return status, None

    def authorize_url(self, redirect_uri, state):
        return self.web + "/login/oauth/authorize?" + urllib.parse.urlencode(
            {"client_id": self.client_id, "redirect_uri": redirect_uri, "state": state,
             "allow_signup": "true"})

    def exchange(self, code, redirect_uri):
        """code -> GitHub /user object. The user token never leaves this function except to
        the background revoke."""
        body = urllib.parse.urlencode({"client_id": self.client_id, "client_secret": self._secret,
                                       "code": code, "redirect_uri": redirect_uri}).encode()
        st, j = self._call("POST", self.web + "/login/oauth/access_token", body,
                           [("Accept", "application/json"),
                            ("Content-Type", "application/x-www-form-urlencoded")])
        token = j.get("access_token") if st == 200 and isinstance(j, dict) else None
        if not isinstance(token, str) or not token:
            return None
        st, user = self._call("GET", self.api + "/user", None,
                              [("Accept", "application/vnd.github+json"),
                               ("Authorization", "Bearer " + token),
                               ("X-GitHub-Api-Version", "2022-11-28")])
        threading.Thread(target=self._revoke, args=(token,), daemon=True).start()
        del token
        return user if st == 200 and isinstance(user, dict) else None

    def _revoke(self, token):
        basic = base64.b64encode(("%s:%s" % (self.client_id, self._secret)).encode()).decode()
        self._call("DELETE", self.api + "/applications/%s/token" % urllib.parse.quote(self.client_id),
                   json.dumps({"access_token": token}).encode(),
                   [("Accept", "application/vnd.github+json"), ("Authorization", "Basic " + basic),
                    ("Content-Type", "application/json")])

    def project_tree(self):
        st, j = self._call("GET", self.api + "/repos/%s/git/trees/main?recursive=1" % PROJECTS_REPO, None,
                           [("Accept", "application/vnd.github+json")], timeout=15)
        if st != 200 or not isinstance(j, dict) or not isinstance(j.get("tree"), list):
            return None
        found = set()
        for e in j["tree"]:
            if isinstance(e, dict) and isinstance(e.get("path"), str):
                m = PROJECT_PATH.match(e["path"])
                if m:
                    found.add(m.group(1) + "/" + m.group(2))
        return found


class ProjectIndex:
    """Existing projects/<handle>/<project>/ paths on omega-projects main, fetched with the
    public unauthenticated API at most once an hour (failed attempts count)."""

    def __init__(self, gh, clock, every=3600):
        self.gh, self.clock, self.every = gh, clock, every
        self.paths, self.tried, self.fetches = None, None, 0
        self.lock = threading.Lock()

    def current(self):
        with self.lock:
            now = self.clock()
            if self.tried is None or now - self.tried >= self.every:
                self.tried = now
                self.fetches += 1
                got = self.gh.project_tree()
                if got is not None:
                    self.paths = got
            return self.paths

    def check(self, refs):
        if not refs:
            return []
        paths = self.current()
        if paths is None:
            raise ApiError(503, "projects_unavailable",
                           "The omega-projects listing is not reachable right now. Try again later.")
        for r in refs:
            if not isinstance(r, str) or not PROJECT_REF.match(r) or r not in paths:
                raise ApiError(400, "bad_project", "Not an existing omega-projects project: %s"
                               % (r if isinstance(r, str) else "?")[:80])
        return list(refs)


# ---------------------------------------------------------------- live updates

class Live:
    def __init__(self):
        self.cond = threading.Condition()
        self.latest = 0
        self.versions = {}
        self.conns = 0

    def new_post(self, feed_id):
        with self.cond:
            self.latest = max(self.latest, feed_id)
            self.cond.notify_all()

    def notify(self, account_id):
        with self.cond:
            self.versions[account_id] = self.versions.get(account_id, 0) + 1
            self.cond.notify_all()


class Window:
    """At most n takes per key in any span seconds."""

    def __init__(self, n, span):
        self.n, self.span, self.state, self.lock = n, float(span), {}, threading.Lock()

    def take(self, key, now):
        with self.lock:
            q = [t for t in self.state.get(key, ()) if now - t < self.span]
            if len(q) >= self.n:
                self.state[key] = q
                return int(self.span - (now - q[0])) + 1
            q.append(now)
            self.state[key] = q
            if len(self.state) > 50000:
                for k in [k for k, v in self.state.items() if not v or now - v[-1] > self.span]:
                    del self.state[k]
            return 0

    def full(self, key, now):
        with self.lock:
            return len([t for t in self.state.get(key, ()) if now - t < self.span]) >= self.n


# ---------------------------------------------------------------- the application

POST_SELECT = """
SELECT p.id, p.author_id, p.text, p.html, p.created, p.reply_to, p.root_id, p.quote_of, p.project,
  p.hidden, p.tags, p.code_langs,
  a.handle AS a_handle, a.display_name AS a_dn, a.avatar AS a_avatar, a.is_bot AS a_bot,
  a.role AS a_role, a.label AS a_label, a.suspended AS a_susp, a.github_login AS a_gh, o.handle AS o_handle,
  (SELECT COUNT(*) FROM likes WHERE post_id=p.id) AS n_likes,
  (SELECT COUNT(*) FROM reposts WHERE post_id=p.id) AS n_reposts,
  (SELECT COUNT(*) FROM posts r WHERE r.reply_to=p.id AND r.hidden=0) AS n_replies,
  EXISTS(SELECT 1 FROM likes WHERE post_id=p.id AND account_id=:v) AS liked,
  EXISTS(SELECT 1 FROM reposts WHERE post_id=p.id AND account_id=:v) AS reposted,
  (SELECT ra.handle FROM posts rp JOIN accounts ra ON ra.id=rp.author_id WHERE rp.id=p.reply_to) AS reply_handle
"""
POST_FROM = " FROM posts p JOIN accounts a ON a.id=p.author_id LEFT JOIN accounts o ON o.id=a.bot_owner "
VISIBLE = " ((p.hidden=0 AND a.suspended=0) OR :mod=1 OR p.author_id=:v) "
UNMUTED = (" p.author_id NOT IN (SELECT target_id FROM mutes WHERE account_id=:v)"
           " AND p.author_id NOT IN (SELECT target_id FROM blocks WHERE account_id=:v) ")


class App:
    def __init__(self, args, secret=None, clock=time.time):
        self.args = args
        self.clock = clock
        self.lock = threading.RLock()
        self.db = sqlite3.connect(args.db, check_same_thread=False, isolation_level=None, timeout=10)
        self.db.row_factory = sqlite3.Row
        self.db.execute("PRAGMA journal_mode=WAL")
        self.db.execute("PRAGMA synchronous=NORMAL")
        self.db.execute("PRAGMA foreign_keys=OFF")
        self.db.executescript(SCHEMA)
        self.gh = GitHub(args.github_web, args.github_api, args.github_client_id or "", secret or "")
        self.projects = ProjectIndex(self.gh, clock)
        self.live = Live()
        self.writes = Window(args.writes_per_minute, 60)
        self.auth_rate = Window(args.auth_per_minute, 60)
        self.redeem_fail = Window(10, 3600)
        self.aien_wake = threading.Event()
        with self.lock:
            salt = self.db.execute("SELECT value FROM meta WHERE key='ban_salt'").fetchone()
            if salt is None:
                self.db.execute("INSERT INTO meta(key, value) VALUES ('ban_salt', ?)", (secrets.token_hex(32),))
            self.salt = self.db.execute("SELECT value FROM meta WHERE key='ban_salt'").fetchone()[0]
            if self.db.execute("SELECT 1 FROM accounts WHERE handle='aien'").fetchone() is None:
                self.db.execute("INSERT INTO accounts(handle, display_name, bio, joined, is_bot, label, role)"
                                " VALUES ('aien', 'Aien', 'The aienos.com site assistant. Mention @aien to ask.',"
                                " ?, 1, 'Site assistant', 'member')", (int(clock()),))
            self.aien_id = self.db.execute("SELECT id FROM accounts WHERE handle='aien'").fetchone()[0]
            if self.db.execute("SELECT 1 FROM accounts WHERE handle='drake'").fetchone() is None:
                self.db.execute("INSERT INTO accounts(handle, display_name, joined, role, webauthn_user_id)"
                                " VALUES ('drake', 'Drake Stapleton', ?, 'admin', ?)",
                                (int(clock()), secrets.token_bytes(16)))
            self.admin_id = self.db.execute("SELECT id FROM accounts WHERE handle='drake'").fetchone()[0]
            if self.db.execute("SELECT 1 FROM credentials WHERE account_id=?", (self.admin_id,)).fetchone() is None:
                print("community: @drake has no passkey yet; run `community.py admin-claim --db ...`",
                      file=sys.stderr, flush=True)
            self.live.latest = self._latest()

    @property
    def github_open(self):
        return self.gh.ready

    # -- db helpers
    def q(self, sql, params=()):
        with self.lock:
            return self.db.execute(sql, params).fetchall()

    def q1(self, sql, params=()):
        with self.lock:
            return self.db.execute(sql, params).fetchone()

    def tx(self):
        app = self

        class Tx:
            def __enter__(self):
                app.lock.acquire()
                app.db.execute("BEGIN IMMEDIATE")
                return app.db

            def __exit__(self, et, ev, tb):
                try:
                    app.db.execute("COMMIT" if et is None else "ROLLBACK")
                finally:
                    app.lock.release()
                return False
        return Tx()

    def now(self):
        return int(self.clock())

    def _latest(self):
        with self.lock:
            r = self.db.execute("SELECT MAX(f.id) FROM feed f JOIN posts p ON p.id=f.post_id JOIN accounts a"
                                " ON a.id=p.author_id WHERE f.kind='post' AND f.top=1 AND p.hidden=0"
                                " AND a.suspended=0").fetchone()
        return r[0] or 0

    # -- tiers
    def tier(self, acct):
        if acct["is_bot"]:
            return "system" if acct["id"] == self.aien_id else "bot"
        if acct["suspended"] or acct["vouch_revoked"]:
            return "readonly"
        if acct["tier_override"]:
            return acct["tier_override"]
        if acct["role"] in ("moderator", "admin"):
            return "member"
        return "probation" if self.now() - acct["joined"] < 7 * DAY else "member"

    # -- accounts
    def account(self, aid):
        return self.q1("SELECT * FROM accounts WHERE id=?", (aid,))

    def by_handle(self, handle):
        if not isinstance(handle, str) or not HANDLE.match(handle):
            return None
        return self.q1("SELECT * FROM accounts WHERE handle=?", (handle,))

    def ban_hash(self, kind, value):
        return sha256("%s:%s:%s" % (self.salt, kind, value))

    def banned(self, db, kind, value):
        return db.execute("SELECT 1 FROM bans WHERE hash=?", (self.ban_hash(kind, value),)).fetchone() is not None

    def new_session(self, db, aid):
        token, csrf = secrets.token_urlsafe(32), secrets.token_urlsafe(24)
        now = self.now()
        db.execute("DELETE FROM sessions WHERE expires<?", (now,))
        db.execute("INSERT INTO sessions(token, account_id, csrf, created, expires) VALUES (?,?,?,?,?)",
                   (sha256(token), aid, csrf, now, now + SESSION_DAYS * DAY))
        return token, csrf

    def check_new_handle(self, db, handle):
        if not isinstance(handle, str) or not USER_HANDLE.match(handle):
            raise ApiError(400, "bad_handle", "A handle is 3 to 20 characters: a letter first, then lowercase "
                           "letters, digits or dashes.")
        if handle in RESERVED:
            raise ApiError(400, "reserved_handle", "That handle is reserved.")
        if db.execute("SELECT 1 FROM accounts WHERE handle=?", (handle,)).fetchone():
            raise ApiError(409, "handle_taken", "That handle is taken.")

    def check_display_name(self, dn, fallback):
        dn = clean_line(dn if dn is not None else "", 50)
        return dn or fallback

    # -- invites
    def invite_left(self, acct):
        """(invites this account can create right now, reason when 0)."""
        if acct["is_bot"]:
            return 0, "Bots cannot invite."
        if acct["suspended"] or acct["vouch_revoked"]:
            return 0, "This account is read-only."
        if acct["invites_frozen"]:
            return 0, "Invites are paused for this account."
        role = acct["role"] if acct["role"] in INVITE_LIMITS else "member"
        now = self.now()
        if role == "member" and (self.tier(acct) != "member" or now - acct["joined"] < 7 * DAY):
            return 0, "Invites open after your first 7 days as a member."
        open_lim, week_lim = INVITE_LIMITS[role]
        n_open = self.q1("SELECT COUNT(*) FROM invites WHERE inviter_id=? AND kind='invite' AND used_by IS NULL"
                         " AND cancelled=0 AND expires>? AND tries<?", (acct["id"], now, INVITE_TRIES))[0]
        left = open_lim - n_open
        if week_lim is not None:
            n_week = self.q1("SELECT COUNT(*) FROM invites WHERE inviter_id=? AND kind IN ('invite','revouch')"
                             " AND created>?", (acct["id"], now - 7 * DAY))[0]
            left = min(left, week_lim - n_week)
        left = max(left, 0)
        return left, ("" if left else "You have used all your invites for now.")

    def create_invite(self, acct, note):
        note = clean_line(note if note is not None else "", 80)
        left, why = self.invite_left(acct)
        if not left:
            raise ApiError(403, "cannot_invite", why)
        code, now = new_code(), self.now()
        with self.tx() as db:
            iid = db.execute("INSERT INTO invites(kind, code_hash, inviter_id, note, created, expires)"
                             " VALUES ('invite',?,?,?,?,?)",
                             (sha256(code), acct["id"], note, now, now + INVITE_DAYS * DAY)).lastrowid
        return {"id": iid, "code": code, "url": "https://www.aienos.com/community/?invite=" + code,
                "expires": iso(now + INVITE_DAYS * DAY), "note": note}

    def admin_claim(self):
        """A fresh single-use 48 h claim code for @drake (first passkey, or recovery)."""
        code, now = new_code(), self.now()
        with self.tx() as db:
            db.execute("UPDATE invites SET cancelled=1 WHERE kind='claim' AND used_by IS NULL AND cancelled=0")
            db.execute("INSERT INTO invites(kind, code_hash, inviter_id, note, created, expires)"
                       " VALUES ('claim',?,?,'admin claim',?,?)",
                       (sha256(code), self.admin_id, now, now + CLAIM_HOURS * 3600))
        return code

    def invite_usable(self, db, inv):
        if inv is None or inv["used_by"] is not None or inv["cancelled"] or inv["expires"] <= self.now() \
                or inv["tries"] >= INVITE_TRIES:
            return False
        if inv["kind"] == "invite":
            who = db.execute("SELECT * FROM accounts WHERE id=?", (inv["inviter_id"],)).fetchone()
            if who is None or who["suspended"] or who["vouch_revoked"] or who["invites_frozen"]:
                return False
        return True

    def invite_tried(self, invite_id):
        with self.tx() as db:
            db.execute("UPDATE invites SET tries=tries+1 WHERE id=?", (invite_id,))

    def redeem_lookup(self, typed, kind, ip):
        """Typed invite/claim code -> usable invite row, else 400 (and one more per-IP failure)."""
        now = self.now()
        if self.redeem_fail.full(ip, now):
            raise ApiError(429, "rate_limited", "Too many wrong codes. Please wait an hour.", retry_after=3600)
        code = norm_code(typed)
        inv = self.q1("SELECT * FROM invites WHERE code_hash=? AND kind=?", (sha256(code), kind)) if code else None
        with self.lock:
            ok = self.invite_usable(self.db, inv)
        if not ok:
            self.redeem_fail.take(ip, now)
            raise ApiError(400, "invite_invalid", "That code is not valid, was already used, or has expired.")
        return inv

    def subtree(self, db, aid):
        return [r[0] for r in db.execute(
            "WITH RECURSIVE sub(id) AS (SELECT ? UNION SELECT a.id FROM accounts a JOIN sub ON a.inviter_id=sub.id)"
            " SELECT id FROM sub", (aid,))]

    # -- passkey ceremonies
    def ceremony_begin(self, binding, kind, account_id, data):
        challenge, now = secrets.token_bytes(32), self.now()
        bh = sha256(binding)
        with self.tx() as db:
            db.execute("DELETE FROM challenges WHERE created<=?", (now - CEREMONY_SECONDS,))
            db.execute("DELETE FROM challenges WHERE cookie=? AND hash NOT IN (SELECT hash FROM challenges"
                       " WHERE cookie=? ORDER BY created DESC LIMIT 3)", (bh, bh))
            db.execute("INSERT INTO challenges(hash, cookie, kind, account_id, data, created) VALUES (?,?,?,?,?,?)",
                       (sha256(challenge), bh, kind, account_id, json.dumps(data), now))
        return challenge

    def ceremony_take(self, challenge, binding, kind, account_id):
        """Single use: the row is deleted whether or not the rest of the checks pass."""
        with self.tx() as db:
            row = db.execute("SELECT * FROM challenges WHERE hash=?", (sha256(challenge),)).fetchone()
            if row is not None:
                db.execute("DELETE FROM challenges WHERE hash=?", (row["hash"],))
        if row is None:
            raise WebAuthnError("challenge unknown or already used")
        if self.now() - row["created"] >= CEREMONY_SECONDS:
            raise WebAuthnError("challenge expired")
        if not binding or not hmac.compare_digest(row["cookie"], sha256(binding)):
            raise WebAuthnError("challenge bound to another browser")
        if row["kind"] != kind or row["account_id"] != account_id:
            raise WebAuthnError("challenge for another ceremony")
        return json.loads(row["data"])

    def create_options(self, challenge, uid, name, display_name, exclude=()):
        return {"challenge": b64u(challenge), "rp": {"id": RP_ID, "name": RP_NAME},
                "user": {"id": b64u(uid), "name": name, "displayName": display_name},
                "pubKeyCredParams": [{"type": "public-key", "alg": -7}, {"type": "public-key", "alg": -257}],
                "authenticatorSelection": {"residentKey": "required", "requireResidentKey": True,
                                           "userVerification": "required"},
                "attestation": "none", "timeout": CEREMONY_SECONDS * 1000,
                "excludeCredentials": [{"type": "public-key", "id": b64u(bytes.fromhex(c))} for c in exclude]}

    def register_options(self, body, binding, ip):
        if "claim" in body:
            inv = self.redeem_lookup(body.get("claim"), "claim", ip)
            a = self.account(inv["inviter_id"])
            data = {"invite": inv["id"], "claim": True}
            excl = [r[0] for r in self.q("SELECT cred_id FROM credentials WHERE account_id=?", (a["id"],))]
            ch = self.ceremony_begin(binding, "register", None, data)
            return self.create_options(ch, a["webauthn_user_id"], a["handle"], a["display_name"], excl)
        inv = self.redeem_lookup(body.get("invite"), "invite", ip)
        handle = body.get("handle")
        try:
            with self.lock:
                self.check_new_handle(self.db, handle)
            dn = self.check_display_name(body.get("display_name"), handle)
        except ApiError:
            self.invite_tried(inv["id"])
            raise
        uid = secrets.token_bytes(16)
        data = {"invite": inv["id"], "handle": handle, "display_name": dn, "uid": uid.hex()}
        ch = self.ceremony_begin(binding, "register", None, data)
        return self.create_options(ch, uid, handle, dn)

    def add_options(self, acct, binding):
        excl = [r[0] for r in self.q("SELECT cred_id FROM credentials WHERE account_id=?", (acct["id"],))]
        if len(excl) >= MAX_PASSKEYS:
            raise ApiError(400, "too_many_passkeys", "An account holds at most %d passkeys." % MAX_PASSKEYS)
        ch = self.ceremony_begin(binding, "add", acct["id"], {})
        return self.create_options(ch, acct["webauthn_user_id"], acct["handle"], acct["display_name"], excl)

    def login_options(self, binding):
        ch = self.ceremony_begin(binding, "login", None, {})
        return {"challenge": b64u(ch), "rpId": RP_ID, "allowCredentials": [],
                "userVerification": "required", "timeout": CEREMONY_SECONDS * 1000}

    def finish_create(self, cred, binding, acct=None):
        """register/verify (acct None) or me/passkeys/add/verify. Returns (account id, token, csrf);
        token is None when a signed-in account added a passkey."""
        raw_id, cdj, rest = credential_parts(cred, True)
        challenge = parse_client_data(cdj, "webauthn.create", self.args.allow_origin)
        kind = "register" if acct is None else "add"
        data = self.ceremony_take(challenge, binding, kind, None if acct is None else acct["id"])
        try:
            ad = parse_attestation(rest["attestationObject"])
            check_flags(ad, True)
            if ad["cred_id"] != raw_id:
                raise WebAuthnError("credential id differs from rawId")
            alg, pub = cose_key(ad["cose"])
            code = run_verify(self.args.verify, alg, pub)
            if code != 0:
                raise WebAuthnError("verifier refused the key (exit %d)" % code)
        except WebAuthnError:
            if data.get("invite"):
                self.invite_tried(data["invite"])
            raise
        cid, now = raw_id.hex(), self.now()
        token = csrf = None
        with self.tx() as db:
            if db.execute("SELECT 1 FROM credentials WHERE cred_id=?", (cid,)).fetchone() \
                    or self.banned(db, "cred", cid):
                raise ApiError(409, "passkey_taken", "This passkey is already registered.")
            if kind == "register":
                inv = db.execute("SELECT * FROM invites WHERE id=?", (data["invite"],)).fetchone()
                if not self.invite_usable(db, inv):
                    raise ApiError(400, "invite_invalid", "That code is not valid, was already used, or has expired.")
                if data.get("claim"):
                    aid = inv["inviter_id"]
                else:
                    self.check_new_handle(db, data["handle"])
                    aid = db.execute("INSERT INTO accounts(handle, display_name, joined, inviter_id, webauthn_user_id)"
                                     " VALUES (?,?,?,?,?)", (data["handle"], data["display_name"], now,
                                                             inv["inviter_id"], bytes.fromhex(data["uid"]))).lastrowid
                    self.notify(db, set(), inv["inviter_id"], "invite_used", aid)
                db.execute("UPDATE invites SET used_by=?, used_at=? WHERE id=?", (aid, now, inv["id"]))
            else:
                aid = acct["id"]
            n = db.execute("SELECT COUNT(*) FROM credentials WHERE account_id=?", (aid,)).fetchone()[0]
            if n >= MAX_PASSKEYS:
                raise ApiError(400, "too_many_passkeys", "An account holds at most %d passkeys." % MAX_PASSKEYS)
            db.execute("INSERT INTO credentials(cred_id, account_id, alg, pub, sign_count, transports, name, created)"
                       " VALUES (?,?,?,?,?,?,?,?)", (cid, aid, alg, pub, ad["count"], json.dumps(rest["transports"]),
                                                    "Passkey %d" % (n + 1), now))
            if kind == "register":
                token, csrf = self.new_session(db, aid)
        if kind == "register" and not data.get("claim"):
            self.live.notify(self.q1("SELECT inviter_id FROM accounts WHERE id=?", (aid,))[0])
        return aid, token, csrf

    def finish_login(self, cred, binding):
        raw_id, cdj, rest = credential_parts(cred, False)
        challenge = parse_client_data(cdj, "webauthn.get", self.args.allow_origin)
        self.ceremony_take(challenge, binding, "login", None)
        row = self.q1("SELECT c.*, a.webauthn_user_id FROM credentials c JOIN accounts a ON a.id=c.account_id"
                      " WHERE c.cred_id=?", (raw_id.hex(),))
        if row is None:
            raise WebAuthnError("unknown credential")
        ad = parse_auth_data(rest["authenticatorData"], False)
        check_flags(ad, False)
        uh = rest["userHandle"]
        if uh is not None and not hmac.compare_digest(uh, bytes(row["webauthn_user_id"] or b"")):
            raise WebAuthnError("userHandle")
        code = run_verify(self.args.verify, row["alg"], bytes(row["pub"]),
                          rest["authenticatorData"] + hashlib.sha256(cdj).digest(), rest["signature"])
        if code != 0:
            raise WebAuthnError("signature refused (exit %d)" % code)
        now, new = self.now(), ad["count"]
        with self.tx() as db:
            cur = db.execute("SELECT sign_count, flagged FROM credentials WHERE id=?", (row["id"],)).fetchone()
            stored = cur["sign_count"]
            regressed = (stored > 0 or new > 0) and new <= stored
            if regressed:
                db.execute("UPDATE credentials SET flagged=1 WHERE id=?", (row["id"],))
                if not cur["flagged"]:
                    db.execute("INSERT INTO reports(reporter_id, target_kind, account_id, reason, note, created)"
                               " VALUES (NULL, 'user', ?, 'other', ?, ?)",
                               (row["account_id"], "System: a passkey's signature counter went backwards"
                                " (%d after %d). The authenticator may be cloned." % (new, stored), now))
            else:
                db.execute("UPDATE credentials SET sign_count=?, last_used=? WHERE id=?", (new, now, row["id"]))
                token, csrf = self.new_session(db, row["account_id"])
        if regressed:
            raise ApiError(403, "passkey_counter", "This passkey sent an old signature counter, so I did not sign "
                           "you in. A moderator will look at it. Another passkey still works.")
        return row["account_id"], token, csrf

    # -- GitHub (optional)
    def github_profile(self, user):
        gh_id, login = user.get("id"), user.get("login")
        if not isinstance(gh_id, int) or isinstance(gh_id, bool) or gh_id <= 0 \
                or not isinstance(login, str) or not LOGIN.match(login):
            raise ApiError(502, "github", "GitHub sent an unexpected profile.")
        avatar = user.get("avatar_url") if isinstance(user.get("avatar_url"), str) else ""
        if not avatar.startswith(AVATAR_PREFIX) or any(c in avatar for c in "\"'<> \\"):
            avatar = ""
        name = user.get("name") if isinstance(user.get("name"), str) else ""
        try:
            name = clean_line(name, 200)[:50]
        except ApiError:
            name = ""
        num = (lambda k: user.get(k) if isinstance(user.get(k), int) and not isinstance(user.get(k), bool) else 0)
        return {"id": gh_id, "login": login, "avatar": avatar, "name": name or login,
                "created": parse_iso(user.get("created_at")), "repos": num("public_repos"),
                "followers": num("followers")}

    def github_link(self, aid, prof):
        with self.tx() as db:
            if self.banned(db, "gh", prof["id"]):
                raise ApiError(403, "banned", "This GitHub account cannot be linked.")
            other = db.execute("SELECT id FROM accounts WHERE github_id=?", (prof["id"],)).fetchone()
            if other is not None and other[0] != aid:
                raise ApiError(409, "github_taken", "That GitHub account is linked to another account.")
            db.execute("UPDATE accounts SET github_id=?, github_login=?, avatar=? WHERE id=?",
                       (prof["id"], prof["login"], prof["avatar"], aid))

    def github_signin(self, prof):
        """Existing linked account -> (aid, token, csrf), else None."""
        with self.tx() as db:
            row = db.execute("SELECT id FROM accounts WHERE github_id=?", (prof["id"],)).fetchone()
            if row is None:
                return None
            db.execute("UPDATE accounts SET github_login=?, avatar=? WHERE id=?",
                       (prof["login"], prof["avatar"], row[0]))
            token, csrf = self.new_session(db, row[0])
        return row[0], token, csrf

    def github_established(self, prof):
        return prof["created"] is not None and self.now() - prof["created"] >= GITHUB_SIGNUP_AGE \
            and (prof["repos"] >= 1 or prof["followers"] >= 3)

    def github_pending(self, prof):
        token, now = secrets.token_urlsafe(32), self.now()
        with self.tx() as db:
            db.execute("DELETE FROM github_pending WHERE created<=?", (now - PENDING_SECONDS,))
            db.execute("INSERT INTO github_pending(token, profile, created) VALUES (?,?,?)",
                       (sha256(token), json.dumps(prof), now))
        return token

    def github_finish(self, token, handle, display_name):
        now = self.now()
        with self.tx() as db:
            row = db.execute("SELECT * FROM github_pending WHERE token=?", (sha256(token or ""),)).fetchone()
            if row is None or now - row["created"] >= PENDING_SECONDS:
                raise ApiError(400, "github_expired", "That GitHub sign-up expired. Please start again.")
            prof = json.loads(row["profile"])
            self.check_new_handle(db, handle)
            dn = self.check_display_name(display_name, prof["name"] or handle)
            if db.execute("SELECT 1 FROM accounts WHERE github_id=?", (prof["id"],)).fetchone() \
                    or self.banned(db, "gh", prof["id"]):
                raise ApiError(409, "github_taken", "That GitHub account cannot start a new account.")
            aid = db.execute("INSERT INTO accounts(handle, display_name, joined, webauthn_user_id, github_id,"
                             " github_login, avatar) VALUES (?,?,?,?,?,?,?)",
                             (handle, dn, now, secrets.token_bytes(16), prof["id"], prof["login"],
                              prof["avatar"])).lastrowid
            db.execute("DELETE FROM github_pending WHERE token=?", (row["token"],))
            token, csrf = self.new_session(db, aid)
        return aid, token, csrf

    # -- vouching
    def revoke_vouch(self, viewer, target, cascade, reason):
        staff = viewer["role"] in ("moderator", "admin")
        if target["is_bot"] or target["role"] == "admin" or target["id"] == viewer["id"]:
            raise ApiError(403, "forbidden", "That account cannot be changed here.")
        if staff:
            if target["role"] == "moderator" and viewer["role"] != "admin":
                raise ApiError(403, "forbidden", "Only the admin can act on a moderator.")
        elif target["inviter_id"] != viewer["id"] or self.now() - target["joined"] > REVOKE_WINDOW \
                or viewer["suspended"] or viewer["vouch_revoked"]:
            raise ApiError(403, "forbidden", "Only the person who invited this account can withdraw the vouch, "
                           "within 90 days of the join.")
        now, bumped = self.now(), set()
        with self.tx() as db:
            ids = self.subtree(db, target["id"]) if cascade else [target["id"]]
            hit = []
            for aid in ids:
                if aid == viewer["id"]:
                    continue
                cur = db.execute("UPDATE accounts SET vouch_revoked=1 WHERE id=? AND vouch_revoked=0"
                                 " AND role!='admin' AND is_bot=0", (aid,))
                if cur.rowcount:
                    hit.append(aid)
                    self.notify(db, bumped, aid, "vouch_revoked", viewer["id"], force=True)
            db.execute("INSERT INTO modlog(at, action, target_kind, target_account, value, reason_public, by_id,"
                       " by_handle) VALUES (?,'revoke_vouch','account',?,?,?,?,?)",
                       (now, target["id"], json.dumps({"count": len(hit), "cascade": bool(cascade)}), reason,
                        viewer["id"], viewer["handle"]))
        for r in bumped:
            self.live.notify(r)
        self.live.new_post(self._latest())
        return len(hit)

    def revouch(self, viewer, target):
        if not target["vouch_revoked"]:
            raise ApiError(400, "not_revoked", "That account still has its vouch.")
        if target["suspended"]:
            raise ApiError(403, "suspended", "That account is suspended.")
        left, why = self.invite_left(viewer)
        if not left:
            raise ApiError(403, "cannot_invite", why)
        now, bumped = self.now(), set()
        with self.tx() as db:
            if viewer["id"] in self.subtree(db, target["id"]):
                raise ApiError(400, "cycle", "You cannot vouch for someone who invited you.")
            db.execute("INSERT INTO invites(kind, code_hash, inviter_id, note, created, expires, used_by, used_at)"
                       " VALUES ('revouch', NULL, ?, 'vouch', ?, ?, ?, ?)", (viewer["id"], now, now, target["id"], now))
            db.execute("UPDATE accounts SET inviter_id=?, vouch_revoked=0 WHERE id=?", (viewer["id"], target["id"]))
            self.notify(db, bumped, target["id"], "vouch_restored", viewer["id"], force=True)
        for r in bumped:
            self.live.notify(r)

    # -- relations
    def blocked(self, by, target):
        return self.q1("SELECT 1 FROM blocks WHERE account_id=? AND target_id=?", (by, target)) is not None

    def muted(self, by, target):
        return self.q1("SELECT 1 FROM mutes WHERE account_id=? AND target_id=?", (by, target)) is not None

    def notify(self, db, bumped, recipient, kind, actor, post_id=0, force=False):
        """One notification, unless it is the actor's own or the recipient blocked/muted the actor.
        force (account standing changes) skips the block/mute filter and re-raises an old one.
        Recipients that got one are added to `bumped` for the live stream."""
        if recipient is None or recipient == actor:
            return
        if not force and db.execute("SELECT 1 FROM blocks WHERE account_id=? AND target_id=? UNION ALL"
                                    " SELECT 1 FROM mutes WHERE account_id=? AND target_id=?",
                                    (recipient, actor, recipient, actor)).fetchone():
            return
        sql = ("INSERT INTO notifications(account_id, kind, actor_id, post_id, created) VALUES (?,?,?,?,?)" +
               (" ON CONFLICT(account_id, kind, actor_id, post_id) DO UPDATE SET created=excluded.created, read=0"
                if force else " ON CONFLICT DO NOTHING"))
        cur = db.execute(sql, (recipient, kind, actor, post_id or 0, self.now()))
        if cur.rowcount:
            bumped.add(recipient)

    def unread(self, aid):
        if not aid:
            return 0
        return self.q1("SELECT COUNT(*) FROM notifications WHERE account_id=? AND read=0", (aid,))[0]

    # -- output shapes
    def author_obj(self, r):
        return {"handle": r["a_handle"], "display_name": r["a_dn"], "avatar": r["a_avatar"] or None,
                "is_bot": bool(r["a_bot"]), "bot_owner": r["o_handle"], "role": r["a_role"],
                "github_login": r["a_gh"], "label": r["a_label"] or ("Bot" if r["a_bot"] else "")}

    def post_obj(self, r, quote=None, repost_by=None, feed_id=None):
        out = {"id": r["id"], "author": self.author_obj(r), "text": r["text"], "html": r["html"],
               "created": iso(r["created"]), "reply_to": r["reply_to"], "reply_to_author": r["reply_handle"],
               "reply_to_deleted": r["reply_to"] is not None and r["reply_handle"] is None,
               "quote": quote, "likes": r["n_likes"], "reposts": r["n_reposts"], "replies": r["n_replies"],
               "liked": bool(r["liked"]), "reposted": bool(r["reposted"]), "tags": json.loads(r["tags"]),
               "project": r["project"], "hidden": bool(r["hidden"]), "held": r["hidden"] == 2,
               "code_langs": json.loads(r["code_langs"])}
        if repost_by is not None:
            out["repost_by"] = repost_by
        if feed_id is not None:
            out["feed_id"] = feed_id
        return out

    def params(self, viewer, **kw):
        p = {"v": viewer["id"] if viewer else 0,
             "mod": 1 if viewer and viewer["role"] in ("moderator", "admin") else 0}
        p.update(kw)
        return p

    def posts_out(self, rows, viewer, feed=False):
        qids = sorted({r["quote_of"] for r in rows if r["quote_of"]})
        quotes = {}
        if qids:
            sql = POST_SELECT + POST_FROM + " WHERE p.id IN (SELECT value FROM json_each(:ids)) AND " + VISIBLE
            for q in self.q(sql, self.params(viewer, ids=json.dumps(qids))):
                quotes[q["id"]] = self.post_obj(q)
        out = []
        for r in rows:
            quote = None
            if r["quote_of"]:
                quote = quotes.get(r["quote_of"], {"id": r["quote_of"], "deleted": True})
            rb = None
            if feed and r["kind"] == "repost":
                rb = r["actor_handle"]
            out.append(self.post_obj(r, quote, rb, r["fid"] if feed else None))
        return out

    def user_obj(self, a, viewer=None, full=True):
        owner = self.q1("SELECT handle FROM accounts WHERE id=?", (a["bot_owner"],)) if a["bot_owner"] else None
        u = {"handle": a["handle"], "display_name": a["display_name"], "avatar": a["avatar"] or None,
             "is_bot": bool(a["is_bot"]), "bot_owner": owner[0] if owner else None, "role": a["role"],
             "github_login": a["github_login"], "label": a["label"] or ("Bot" if a["is_bot"] else "")}
        if not full:
            return u
        inviter = self.q1("SELECT handle FROM accounts WHERE id=?", (a["inviter_id"],)) if a["inviter_id"] else None
        with self.lock:
            descendants = len(self.subtree(self.db, a["id"])) - 1
        can_revoke = bool(viewer) and not a["is_bot"] and a["role"] != "admin" and a["id"] != viewer["id"] and (
            viewer["role"] in ("moderator", "admin") or (
                a["inviter_id"] == viewer["id"] and self.now() - a["joined"] <= REVOKE_WINDOW
                and not viewer["suspended"] and not viewer["vouch_revoked"]))
        u.update({
            "bio": a["bio"], "projects": json.loads(a["projects"]), "created": iso(a["joined"]),
            "tier": self.tier(a),
            "inviter": inviter[0] if inviter else None,
            "invitee_count": self.q1("SELECT COUNT(*) FROM accounts WHERE inviter_id=?", (a["id"],))[0],
            "descendants": descendants,
            "vouch_revoked": bool(a["vouch_revoked"]), "invites_frozen": bool(a["invites_frozen"]),
            "can_revoke": can_revoke,
            "followers": self.q1("SELECT COUNT(*) FROM follows WHERE followee=?", (a["id"],))[0],
            "following": self.q1("SELECT COUNT(*) FROM follows WHERE follower=?", (a["id"],))[0],
            "posts": self.q1("SELECT COUNT(*) FROM posts WHERE author_id=? AND hidden=0", (a["id"],))[0],
            "suspended": bool(a["suspended"]),
            "followed": bool(viewer) and self.q1("SELECT 1 FROM follows WHERE follower=? AND followee=?",
                                                 (viewer["id"], a["id"])) is not None,
            "muted": bool(viewer) and self.muted(viewer["id"], a["id"]),
            "blocked": bool(viewer) and self.blocked(viewer["id"], a["id"])})
        return u

    def me_obj(self, a):
        u = self.user_obj(a, None, full=False)
        bots = self.q("SELECT handle, display_name FROM accounts WHERE bot_owner=? AND bot_token IS NOT NULL",
                      (a["id"],))
        inviter = self.q1("SELECT handle FROM accounts WHERE id=?", (a["inviter_id"],)) if a["inviter_id"] else None
        u.update({"tier": self.tier(a), "bio": a["bio"], "projects": json.loads(a["projects"]),
                  "bots": [{"handle": b[0], "display_name": b[1]} for b in bots],
                  "inviter": inviter[0] if inviter else None,
                  "passkeys": self.q1("SELECT COUNT(*) FROM credentials WHERE account_id=?", (a["id"],))[0],
                  "suspended": bool(a["suspended"]), "vouch_revoked": bool(a["vouch_revoked"]),
                  "invites_frozen": bool(a["invites_frozen"])})
        return u

    # -- posting
    def create_post(self, acct, body):
        now = self.now()
        if acct["suspended"]:
            raise ApiError(403, "suspended", "This account is suspended.")
        tier = self.tier(acct)
        if tier == "readonly":
            raise ApiError(403, "readonly", "This account is read-only right now. Reading, export and delete "
                           "still work.")
        per_day, max_links, max_mentions = LIMITS[tier]
        text, raw = normalize_post(body.get("text"))
        n_today = self.q1("SELECT COUNT(*) FROM posts WHERE author_id=? AND created>?", (acct["id"], now - DAY))[0]
        if n_today >= per_day:
            oldest = self.q1("SELECT MIN(created) FROM posts WHERE author_id=? AND created>?",
                             (acct["id"], now - DAY))[0] or now
            raise ApiError(429, "rate", "That is the daily post limit for this account (%d). Try again later."
                           % per_day, retry_after=max(1, oldest + DAY - now))
        fp = text_fingerprint(text)
        if tier != "system" and self.q1("SELECT 1 FROM posts WHERE author_id=? AND text_hash=? AND created>?",
                                        (acct["id"], fp, now - DAY)):
            raise ApiError(409, "duplicate", "The same text was already posted in the last 24 hours.")
        viewer = acct
        reply_to = root_id = parent_author = None
        if body.get("reply_to") is not None:
            parent = self.visible_post(body.get("reply_to"), viewer)
            if self.blocked(parent["author_id"], acct["id"]):
                raise ApiError(403, "blocked", "This account cannot reply to that person.")
            reply_to, root_id, parent_author = parent["id"], parent["root_id"] or parent["id"], parent["author_id"]
        quote_of = quote_author = None
        if body.get("quote_of") is not None:
            quoted = self.visible_post(body.get("quote_of"), viewer)
            if self.blocked(quoted["author_id"], acct["id"]):
                raise ApiError(403, "blocked", "This account cannot quote that person.")
            quote_of, quote_author = quoted["id"], quoted["author_id"]
        project = body.get("project")
        if project is not None:
            project = self.projects.check([project])[0]
        try:
            html, meta = render_post(self.args.render, raw)
        except RenderError:
            raise ApiError(503, "render_unavailable", "Posting is paused for a moment. Try again shortly.")
        if len(meta["links"]) > max_links:
            raise ApiError(422, "too_many_links", "This account can include at most %d link%s per post."
                           % (max_links, "" if max_links == 1 else "s"))
        if len(meta["mentions"]) > max_mentions:
            raise ApiError(422, "too_many_mentions", "This account can mention at most %d people per post."
                           % max_mentions)
        domains = []
        for link in meta["links"]:
            d = link_domain(link)
            if d and d not in domains:
                domains.append(d)
        if tier == "probation" and any(d in SHORTENERS for d in domains):
            raise ApiError(422, "shortener", "Link shorteners are not allowed for new accounts. Link the "
                           "full address instead.")
        held = False
        for d in domains:
            if d in FLOOD_EXEMPT or tier == "system":
                continue
            n = self.q1("SELECT COUNT(*) FROM posts WHERE author_id=? AND created>? AND domains LIKE ?",
                        (acct["id"], now - DAY, '%' + json.dumps(d).replace("%", "").replace("_", "") + '%'))[0]
            if n >= FLOOD_POSTS:
                held = True
        bumped = set()
        with self.tx() as db:
            cur = db.execute(
                "INSERT INTO posts(author_id, text, html, text_hash, created, reply_to, root_id, quote_of, project,"
                " hidden, tags, links, domains, code_langs) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
                (acct["id"], text, html, fp, now, reply_to, root_id, quote_of, project, 2 if held else 0,
                 json.dumps(meta["tags"]), json.dumps(meta["links"]), json.dumps(domains),
                 json.dumps(meta["langs"])))
            pid = cur.lastrowid
            for t in meta["tags"]:
                db.execute("INSERT OR IGNORE INTO post_tags(post_id, tag, created) VALUES (?,?,?)", (pid, t, now))
            fcur = db.execute("INSERT INTO feed(kind, post_id, actor_id, created, top) VALUES ('post',?,?,?,?)",
                              (pid, acct["id"], now, 1 if reply_to is None else 0))
            feed_id = fcur.lastrowid
            aien_job = False
            if held:
                db.execute("INSERT INTO reports(reporter_id, target_kind, post_id, account_id, reason, note, created)"
                           " VALUES (NULL, 'post', ?, ?, 'spam', 'Held automatically: many posts linking one"
                           " domain in 24 hours.', ?)", (pid, acct["id"], now))
            else:
                if parent_author:
                    self.notify(db, bumped, parent_author, "reply", acct["id"], pid)
                if quote_author:
                    self.notify(db, bumped, quote_author, "quote", acct["id"], pid)
                for h in meta["mentions"]:
                    m = db.execute("SELECT id FROM accounts WHERE handle=?", (h,)).fetchone()
                    if m is None:
                        continue
                    db.execute("INSERT OR IGNORE INTO post_mentions(post_id, account_id) VALUES (?,?)", (pid, m[0]))
                    if m[0] == self.aien_id and acct["id"] != self.aien_id:
                        db.execute("INSERT INTO aien_jobs(post_id, root_id, created) VALUES (?,?,?)",
                                   (pid, root_id or pid, now))
                        aien_job = True
                    elif m[0] not in (parent_author, quote_author):
                        self.notify(db, bumped, m[0], "mention", acct["id"], pid)
        for r in bumped:
            self.live.notify(r)
        if not held and reply_to is None:
            self.live.new_post(feed_id)
        if aien_job:
            self.aien_wake.set()
        return pid, held

    def visible_post(self, pid, viewer):
        if isinstance(pid, str) and pid.isdigit():
            pid = int(pid)
        if not isinstance(pid, int) or isinstance(pid, bool) or pid <= 0:
            raise ApiError(400, "bad_request", "Not a post id.")
        r = self.q1(POST_SELECT + POST_FROM + " WHERE p.id=:id AND " + VISIBLE, self.params(viewer, id=pid))
        if r is None:
            raise ApiError(404, "not_found", "That post does not exist or was removed.")
        return r

    def delete_post_rows(self, db, where, params):
        """Hard-delete posts matching `where` (on posts p) and every row hanging off them."""
        sub = "SELECT p.id FROM posts p WHERE " + where
        for sql in ("DELETE FROM likes WHERE post_id IN (%s)", "DELETE FROM reposts WHERE post_id IN (%s)",
                    "DELETE FROM feed WHERE post_id IN (%s)", "DELETE FROM post_tags WHERE post_id IN (%s)",
                    "DELETE FROM post_mentions WHERE post_id IN (%s)",
                    "DELETE FROM notifications WHERE post_id IN (%s)",
                    "DELETE FROM aien_jobs WHERE post_id IN (%s) OR reply_id IN (%s)",
                    "UPDATE reports SET status='gone', post_id=NULL WHERE post_id IN (%s)"):
            n = sql.count("%s")
            db.execute(sql % ((sub,) * n), params * n)
        db.execute("DELETE FROM posts WHERE id IN (%s)" % sub, params)

    def delete_account(self, acct):
        """Hard delete: the account, its bots, passkeys, invites, and everything either of them
        stored. Accounts it invited stay (inviter cleared). A suspended account leaves salted
        hashes of its passkey ids and GitHub id behind so it cannot simply return."""
        ids = [acct["id"]] + [r[0] for r in self.q("SELECT id FROM accounts WHERE bot_owner=?", (acct["id"],))]
        now = self.now()
        with self.tx() as db:
            if acct["suspended"]:
                keys = [("cred", r[0]) for r in db.execute("SELECT cred_id FROM credentials WHERE account_id=?",
                                                           (acct["id"],))]
                if acct["github_id"]:
                    keys.append(("gh", acct["github_id"]))
                for kind, value in keys:
                    db.execute("INSERT OR IGNORE INTO bans(hash, created) VALUES (?,?)",
                               (self.ban_hash(kind, value), now))
            for aid in ids:
                self.delete_post_rows(db, "p.author_id=?", (aid,))
                for sql in ("DELETE FROM likes WHERE account_id=?", "DELETE FROM reposts WHERE account_id=?",
                            "DELETE FROM feed WHERE actor_id=?", "DELETE FROM post_mentions WHERE account_id=?",
                            "DELETE FROM notifications WHERE account_id=? OR actor_id=?",
                            "DELETE FROM follows WHERE follower=? OR followee=?",
                            "DELETE FROM mutes WHERE account_id=? OR target_id=?",
                            "DELETE FROM blocks WHERE account_id=? OR target_id=?",
                            "DELETE FROM sessions WHERE account_id=?",
                            "DELETE FROM credentials WHERE account_id=?",
                            "DELETE FROM challenges WHERE account_id=?",
                            "DELETE FROM invites WHERE inviter_id=?",
                            "UPDATE invites SET used_by=NULL WHERE used_by=?",
                            "UPDATE accounts SET inviter_id=NULL WHERE inviter_id=?",
                            "DELETE FROM reports WHERE reporter_id=? OR account_id=?",
                            "UPDATE modlog SET target_account=NULL WHERE target_account=?",
                            "UPDATE modlog SET by_id=NULL, by_handle='a former moderator' WHERE by_id=?"):
                    db.execute(sql, (aid,) * sql.count("?"))
            for aid in ids:
                db.execute("DELETE FROM accounts WHERE id=?", (aid,))
        self.live.new_post(self.live.latest)

    def export(self, acct):
        aid = acct["id"]

        def rows(sql, *p):
            return [dict(r) for r in self.q(sql, p)]
        a = dict(acct)
        a.pop("bot_token", None)
        a["webauthn_user_id"] = b64u(bytes(a["webauthn_user_id"])) if a["webauthn_user_id"] else None
        a["joined"] = iso(a["joined"])
        a["suspended_at"] = iso(a["suspended_at"]) if a["suspended_at"] is not None else None
        a["projects"] = json.loads(a["projects"])
        a["tier"] = self.tier(acct)
        inviter = self.q1("SELECT handle FROM accounts WHERE id=?", (a["inviter_id"],)) if a["inviter_id"] else None
        a["inviter"] = inviter[0] if inviter else None
        return {
            "exported": iso(self.now()), "account": a,
            "posts": rows("SELECT id, text, html, created, reply_to, quote_of, project, hidden, tags, links,"
                          " code_langs FROM posts WHERE author_id=? ORDER BY id", aid),
            "likes": rows("SELECT post_id, created FROM likes WHERE account_id=?", aid),
            "reposts": rows("SELECT post_id, created FROM reposts WHERE account_id=?", aid),
            "following": rows("SELECT b.handle, f.created FROM follows f JOIN accounts b ON b.id=f.followee"
                              " WHERE f.follower=?", aid),
            "followers": rows("SELECT b.handle, f.created FROM follows f JOIN accounts b ON b.id=f.follower"
                              " WHERE f.followee=?", aid),
            "mutes": rows("SELECT b.handle, m.created FROM mutes m JOIN accounts b ON b.id=m.target_id"
                          " WHERE m.account_id=?", aid),
            "blocks": rows("SELECT b.handle, m.created FROM blocks m JOIN accounts b ON b.id=m.target_id"
                           " WHERE m.account_id=?", aid),
            "notifications": rows("SELECT n.id, n.kind, b.handle AS actor, n.post_id, n.created, n.read FROM"
                                  " notifications n LEFT JOIN accounts b ON b.id=n.actor_id WHERE n.account_id=?", aid),
            "reports_filed": rows("SELECT id, target_kind, post_id, reason, note, created, status FROM reports"
                                  " WHERE reporter_id=?", aid),
            "sessions": rows("SELECT created, expires FROM sessions WHERE account_id=?", aid),
            "bots": rows("SELECT handle, display_name, bio, joined, bot_token IS NOT NULL AS active FROM accounts"
                         " WHERE bot_owner=?", aid),
            "passkeys": [{"id": r["id"], "name": r["name"], "alg": r["alg"], "created": iso(r["created"]),
                          "last_used": iso(r["last_used"]), "sign_count": r["sign_count"],
                          "transports": json.loads(r["transports"]), "credential_id": r["cred_id"],
                          "public_key": bytes(r["pub"]).hex()}
                         for r in self.q("SELECT * FROM credentials WHERE account_id=? ORDER BY id", (aid,))],
            "invites": rows("SELECT i.id, i.kind, i.note, i.created, i.expires, i.cancelled, u.handle AS used_by,"
                            " i.used_at FROM invites i LEFT JOIN accounts u ON u.id=i.used_by WHERE i.inviter_id=?"
                            " ORDER BY i.id", aid),
            "note": "Session tokens, bot tokens and invite codes are stored only as SHA-256 hashes and are not "
                    "included.",
        }

    # -- Aien
    def aien_step(self):
        """Take the oldest queued Aien job and finish it (reply, or skip with a reason)."""
        job = self.q1("SELECT * FROM aien_jobs WHERE state='queued' ORDER BY id LIMIT 1")
        if job is None:
            return None
        now = self.now()

        def finish(state, reason, reply_id=None):
            with self.tx() as db:
                db.execute("UPDATE aien_jobs SET state=?, reason=?, reply_id=?, done_at=? WHERE id=?",
                           (state, reason, reply_id, self.now(), job["id"]))
            return state, reason
        post = self.q1("SELECT p.*, a.handle, a.suspended FROM posts p JOIN accounts a ON a.id=p.author_id"
                       " WHERE p.id=?", (job["post_id"],))
        if post is None or post["hidden"] or post["suspended"]:
            return finish("skipped", "gone")
        if now - job["created"] > 3600:
            return finish("skipped", "stale")
        if self.q1("SELECT COUNT(*) FROM aien_jobs WHERE state='done' AND done_at>?", (now - 3600,))[0] >= AIEN_PER_HOUR:
            return finish("skipped", "global")
        if self.q1("SELECT 1 FROM aien_jobs WHERE state='done' AND root_id=? AND done_at>?",
                   (job["root_id"], now - 3600)):
            return finish("skipped", "thread")
        if self.blocked(post["author_id"], self.aien_id):
            return finish("skipped", "blocked")
        reply, why = self.ask_gateway(post["text"], post["handle"])
        if reply is None:
            return finish("skipped", why)
        aien = self.account(self.aien_id)
        try:
            pid, _ = self.create_post(aien, {"text": reply, "reply_to": post["id"]})
        except ApiError as e:
            return finish("skipped", "post:" + e.code)
        return finish("done", "", pid)

    def ask_gateway(self, text, handle):
        """Ask allen-chat exactly like an anonymous visitor would (public chat route, never 18091).
        Returns (reply text, None) or (None, reason). A queue position means the shared model is
        busy: the job is skipped instead of waiting for a slot."""
        u = urllib.parse.urlsplit(self.args.aien_gateway)
        if u.port == 18091:
            return None, "refused_port"
        prompt = ("A post on the aienos.com Community by @%s mentions you. Reply in plain text, in at most %d "
                  "characters.\n\n%s" % (handle, AIEN_MAX_REPLY, text[:3500]))
        body = json.dumps({"messages": [{"role": "user", "content": prompt}]}).encode()
        try:
            c = http.client.HTTPConnection(u.hostname, u.port or 80, timeout=60)
            c.request("POST", "/allen/api/chat", body, {"Content-Type": "application/json",
                                                        "X-Allen-Anonymous": "1", "X-Allen-Tools": "0"})
            r = c.getresponse()
        except (OSError, http.client.HTTPException):
            return None, "unreachable"
        try:
            if r.status == 503 or r.status == 429:
                return None, "busy"
            if r.status != 200:
                return None, "http_%d" % r.status
            deadline = time.monotonic() + 150
            event, said, finished = None, [], False
            while time.monotonic() < deadline:
                line = r.readline(1 << 20)
                if not line:
                    break
                line = line.rstrip(b"\r\n")
                if not line:
                    event = None
                    continue
                if line.startswith(b"event: "):
                    event = line[7:].decode("utf-8", "replace")
                    if event == "queue":
                        return None, "busy"
                    continue
                if not line.startswith(b"data: "):
                    continue
                try:
                    data = json.loads(line[6:])
                except ValueError:
                    continue
                if event is None and isinstance(data, str):
                    said.append(data)
                elif event == "error":
                    code = data.get("code") if isinstance(data, dict) else ""
                    return None, "busy" if code == "busy" else "error_%s" % str(code)[:20]
                elif event == "done":
                    finished = True
                    break
            if not finished:
                return None, "incomplete"
        except (OSError, http.client.HTTPException):
            return None, "unreachable"
        finally:
            c.close()
        reply = "".join(ch for ch in "".join(said) if ch >= " " or ch in "\n\t").strip()
        if not reply:
            return None, "empty"
        if len(reply) > AIEN_MAX_REPLY:
            cut = reply[:AIEN_MAX_REPLY - 3]
            sp = cut.rfind(" ")
            reply = (cut[:sp] if sp > AIEN_MAX_REPLY // 2 else cut).rstrip() + "..."
        return reply, None

    def aien_loop(self):
        while True:
            self.aien_wake.wait(30)
            self.aien_wake.clear()
            try:
                while self.aien_step() is not None:
                    pass
            except Exception as e:  # keep the worker alive; the job stays visible in aien_jobs
                print("community: aien worker error %s" % type(e).__name__, file=sys.stderr, flush=True)

    def nightly_loop(self):
        while True:
            time.sleep(DAY)
            try:
                self.refresh_tiers()
            except Exception as e:
                print("community: tier refresh error %s" % type(e).__name__, file=sys.stderr, flush=True)


# ---------------------------------------------------------------- HTTP

ROUTES = []


def route(method, pattern, auth=None):
    """auth: None (anyone), 'user' (signed in), 'mod' (moderator/admin), 'admin'."""
    def deco(fn):
        ROUTES.append((method, re.compile("^" + pattern + "$"), fn, auth))
        return fn
    return deco


def int_param(qs, name, default=0, lo=0, hi=1 << 62):
    v = qs.get(name, [""])[0]
    if v == "":
        return default
    if not v.isdigit() or len(v) > 18:
        raise ApiError(400, "bad_request", "Bad %s." % name)
    return max(lo, min(hi, int(v)))


class Server(http.server.ThreadingHTTPServer):
    daemon_threads = True
    request_queue_size = 64

    def __init__(self, addr, app, max_threads):
        self.app = app
        self.slots = threading.BoundedSemaphore(max_threads)
        super().__init__(addr, Handler)

    def process_request(self, request, client_address):
        if not self.slots.acquire(blocking=False):
            try:
                request.sendall(b"HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\n"
                                b"Retry-After: 5\r\nConnection: close\r\nContent-Length: 62\r\n\r\n"
                                b'{"error":"busy","message":"The Community is busy. Try again."}')
            except OSError:
                pass
            self.shutdown_request(request)
            return
        try:
            super().process_request(request, client_address)
        except Exception:
            self.slots.release()
            raise

    def process_request_thread(self, request, client_address):
        try:
            super().process_request_thread(request, client_address)
        finally:
            self.slots.release()


SEC_HEADERS = (("Cache-Control", "no-store"), ("X-Content-Type-Options", "nosniff"),
               ("Referrer-Policy", "no-referrer"),
               ("Content-Security-Policy", "default-src 'none'; frame-ancestors 'none'"))


class Handler(http.server.BaseHTTPRequestHandler):
    server_version = "aien-community"
    sys_version = ""
    protocol_version = "HTTP/1.1"
    timeout = 30

    def log_message(self, *a):
        pass  # no access log here: nginx owns logging, and query strings are never logged

    @property
    def app(self):
        return self.server.app

    def client_ip(self):
        if self.client_address[0] == "127.0.0.1":
            v = self.headers.get("X-Real-IP", "").strip()
            if v:
                return v[:64]
        return self.client_address[0]

    # -- responses
    def send(self, code, body, ctype, headers=()):
        self.send_response(code)
        for k, v in SEC_HEADERS:
            self.send_header(k, v)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        for k, v in headers:
            self.send_header(k, v)
        self.end_headers()
        if self.command != "HEAD":
            self.wfile.write(body)

    def send_json(self, code, obj, headers=()):
        raw = json.dumps(obj, ensure_ascii=False, separators=(",", ":")).encode()
        self.send(code, raw, "application/json; charset=utf-8", headers)

    def redirect(self, loc, cookies=()):
        self.send(302, b"", "text/plain; charset=utf-8", [("Location", loc)] + [("Set-Cookie", c) for c in cookies])

    def cookie(self, name):
        for part in self.headers.get("Cookie", "").split(";"):
            k, _, v = part.strip().partition("=")
            if k == name:
                return v
        return None

    # -- auth context
    def load_viewer(self):
        self.viewer, self.session, self.bot = None, None, None
        auth = self.headers.get("Authorization", "")
        if auth.startswith("Bearer "):
            tok = auth[7:].strip()
            if not tok.startswith("cmb_") or len(tok) > 200:
                raise ApiError(401, "bad_token", "That bot token is not valid.")
            a = self.app.q1("SELECT * FROM accounts WHERE bot_token=? AND is_bot=1", (sha256(tok),))
            if a is None:
                raise ApiError(401, "bad_token", "That bot token is not valid.")
            owner = self.app.account(a["bot_owner"]) if a["bot_owner"] else None
            if a["suspended"] or owner is None or owner["suspended"]:
                raise ApiError(403, "suspended", "This bot is suspended.")
            self.viewer, self.bot = a, a
            return
        tok = self.cookie("cm_s")
        if tok and len(tok) < 200:
            s = self.app.q1("SELECT * FROM sessions WHERE token=? AND expires>?", (sha256(tok), self.app.now()))
            if s is not None:
                self.viewer, self.session = self.app.account(s["account_id"]), s

    def check_post(self):
        """Every state-changing request: JSON body, the site's Origin, the session's CSRF token."""
        ctype = self.headers.get("Content-Type", "").split(";")[0].strip().lower()
        if ctype != "application/json":
            raise ApiError(415, "content_type", "Send the request as application/json.")
        origin = self.headers.get("Origin")
        if self.bot is not None:
            if origin is not None and origin not in self.app.args.allow_origin:
                raise ApiError(403, "origin", "Requests come from the aienos.com page only.")
            return
        if origin not in self.app.args.allow_origin:
            raise ApiError(403, "origin", "Requests come from the aienos.com page only.")
        if self.session is not None:
            got = self.headers.get("X-CM-CSRF", "")
            if not hmac.compare_digest(got.encode(), self.session["csrf"].encode()):
                raise ApiError(403, "csrf", "The page is out of date. Reload it and try again.")

    def read_body(self):
        n = self.headers.get("Content-Length")
        if n is None or not n.isdigit():
            if self.headers.get("Transfer-Encoding"):
                raise ApiError(411, "length", "Send a Content-Length.")
            return {}
        n = int(n)
        if n > MAX_BODY:
            raise ApiError(413, "too_large", "The request is too large.")
        raw = self.rfile.read(n) if n else b""
        if not raw:
            return {}
        try:
            body = json.loads(raw.decode("utf-8"))
        except (ValueError, UnicodeDecodeError):
            raise ApiError(400, "bad_json", "The request is not valid JSON.")
        if not isinstance(body, dict):
            raise ApiError(400, "bad_json", "Send a JSON object.")
        return body

    def do_HEAD(self):
        self.dispatch("GET")

    def do_GET(self):
        self.dispatch("GET")

    def do_POST(self):
        self.dispatch("POST")

    def dispatch(self, method):
        try:
            u = urllib.parse.urlsplit(self.path)
            self.qs = urllib.parse.parse_qs(u.query, keep_blank_values=True, max_num_fields=20)
            path = u.path
            if path.startswith("/community/feed/") and method == "GET":
                return feeds(self, path[len("/community/feed/"):])
            if not path.startswith("/community/api/"):
                raise ApiError(404, "not_found", "No such route.")
            sub = path[len("/community/api/"):]
            self.load_viewer()
            allowed = False
            for m, rx, fn, auth in ROUTES:
                mt = rx.match(sub)
                if not mt:
                    continue
                if m != method:
                    allowed = True
                    continue
                if self.bot is not None and (method, fn.__name__) != ("POST", "api_post_create"):
                    raise ApiError(403, "bot_token", "A bot token can only create posts.")
                if method == "POST":
                    self.check_post()
                if auth and self.viewer is None:
                    raise ApiError(401, "signin", "Sign in with GitHub first.")
                if auth == "mod" and self.viewer["role"] not in ("moderator", "admin"):
                    raise ApiError(403, "forbidden", "Moderators only.")
                if auth == "admin" and self.viewer["role"] != "admin":
                    raise ApiError(403, "forbidden", "Admin only.")
                self.body = self.read_body() if method == "POST" else {}
                if method == "POST" and self.viewer is not None and fn.__name__ not in ("api_logout", "api_post_create"):
                    retry = self.app.writes.take(self.viewer["id"], self.app.clock())
                    if retry:
                        raise ApiError(429, "rate", "That is quick. Try again shortly.", retry_after=retry)
                return fn(self, *mt.groups())
            if allowed:
                raise ApiError(405, "method", "Method not allowed.")
            raise ApiError(404, "not_found", "No such route.")
        except ApiError as e:
            body = {"error": e.code, "message": e.message}
            body.update(e.extra)
            headers = [("Retry-After", str(e.extra["retry_after"]))] if "retry_after" in e.extra else []
            try:
                self.send_json(e.status, body, headers)
            except OSError:
                pass
        except (BrokenPipeError, ConnectionResetError, socket.timeout, TimeoutError):
            self.close_connection = True
        except Exception as e:
            print("community: internal error %s" % type(e).__name__, file=sys.stderr, flush=True)
            try:
                self.send_json(500, {"error": "internal", "message": "Something went wrong on my side."})
            except OSError:
                pass


# ---------------------------------------------------------------- routes

@route("GET", "health")
def api_health(h):
    h.send_json(200, {"ok": True, "signin_open": h.app.signin_open})


@route("GET", "auth/login")
def api_login(h):
    app = h.app
    if not app.signin_open:
        raise ApiError(503, "signin_closed", "Sign-in is not open yet. Reading works now; please check back soon.")
    retry = app.auth_rate.take(h.client_ip(), app.clock())
    if retry:
        raise ApiError(429, "rate", "Too many sign-in attempts. Try again shortly.", retry_after=retry)
    nxt = h.qs.get("next", ["/community/"])[0]
    if not NEXT_PATH.match(nxt) or "//" in nxt or "/../" in nxt:
        nxt = "/community/"
    state = secrets.token_urlsafe(32)
    now = app.now()
    with app.tx() as db:
        db.execute("DELETE FROM oauth_states WHERE created<?", (now - STATE_SECONDS,))
        db.execute("INSERT INTO oauth_states(state, next, created) VALUES (?,?,?)", (sha256(state), nxt, now))
    h.redirect(app.gh.authorize_url(app.args.public_base + "/community/api/auth/callback", state),
               ["cm_st=%s; HttpOnly; Secure; SameSite=Lax; Path=/community/api/auth/; Max-Age=%d"
                % (state, STATE_SECONDS)])


@route("GET", "auth/callback")
def api_callback(h):
    app = h.app
    clear = "cm_st=; HttpOnly; Secure; SameSite=Lax; Path=/community/api/auth/; Max-Age=0"
    state = h.qs.get("state", [""])[0]
    code = h.qs.get("code", [""])[0]
    cookie = h.cookie("cm_st") or ""
    if not app.signin_open:
        return h.redirect("/community/?signin=closed", [clear])
    retry = app.auth_rate.take(h.client_ip(), app.clock())
    if retry:
        return h.redirect("/community/?signin=rate", [clear])
    ok_shape = 20 <= len(state) <= 100 and 1 <= len(code) <= 200 and re.match(r"^[A-Za-z0-9_-]+$", state) \
        and re.match(r"^[A-Za-z0-9_.-]+$", code)
    if not ok_shape or not hmac.compare_digest(cookie.encode(), state.encode()):
        return h.redirect("/community/?signin=failed", [clear])
    now = app.now()
    with app.tx() as db:
        row = db.execute("SELECT next, created FROM oauth_states WHERE state=?", (sha256(state),)).fetchone()
        db.execute("DELETE FROM oauth_states WHERE state=?", (sha256(state),))
    if row is None or now - row["created"] > STATE_SECONDS:
        return h.redirect("/community/?signin=failed", [clear])
    user = app.gh.exchange(code, app.args.public_base + "/community/api/auth/callback")
    if user is None:
        return h.redirect("/community/?signin=failed", [clear])
    try:
        _, token, _ = app.sign_in(user)
    except ApiError as e:
        return h.redirect("/community/?signin=%s" % ("refused" if e.code == "banned" else "failed"), [clear])
    h.redirect(row["next"], [clear, "cm_s=%s; HttpOnly; Secure; SameSite=Lax; Path=/community/; Max-Age=%d"
                             % (token, SESSION_DAYS * DAY)])


@route("POST", "auth/logout")
def api_logout(h):
    if h.session is not None:
        with h.app.tx() as db:
            db.execute("DELETE FROM sessions WHERE token=?", (h.session["token"],))
    h.send_json(200, {"ok": True}, [("Set-Cookie", "cm_s=; HttpOnly; Secure; SameSite=Lax; Path=/community/; Max-Age=0")])


@route("GET", "me")
def api_me(h):
    app = h.app
    v = h.viewer
    h.send_json(200, {"user": app.me_obj(v) if v else None, "csrf": h.session["csrf"] if h.session else None,
                      "unread": app.unread(v["id"]) if v else 0, "signin_open": app.signin_open})


@route("POST", "me/profile", "user")
def api_profile(h):
    app, v, b = h.app, h.viewer, h.body
    sets, vals = [], []
    if "display_name" in b:
        dn = clean_line(b["display_name"], 50)
        if not dn:
            raise ApiError(400, "bad_request", "The display name cannot be empty.")
        sets.append("display_name=?")
        vals.append(dn)
    if "bio" in b:
        sets.append("bio=?")
        vals.append(clean_line(b["bio"], 280, multiline=True))
    if "projects" in b:
        p = b["projects"]
        if not isinstance(p, list) or len(p) > 6 or len(set(map(str, p))) != len(p):
            raise ApiError(400, "bad_request", "List up to 6 different projects.")
        sets.append("projects=?")
        vals.append(json.dumps(app.projects.check(p)))
    if v["suspended"]:
        raise ApiError(403, "suspended", "This account is suspended.")
    if sets:
        with app.tx() as db:
            db.execute("UPDATE accounts SET %s WHERE id=?" % ", ".join(sets), vals + [v["id"]])
    h.send_json(200, {"user": app.me_obj(app.account(v["id"]))})


def target_user(h, handle):
    a = h.app.by_handle(handle)
    if a is None:
        raise ApiError(404, "not_found", "No such account.")
    return a


@route("GET", "users/([a-z0-9_-]{1,44})")
def api_user(h, handle):
    h.send_json(200, {"user": h.app.user_obj(target_user(h, handle), h.viewer)})


def feed_page(h, where, params, limit=20):
    app = h.app
    before = int_param(h.qs, "before")
    limit = int_param(h.qs, "limit", limit, 1, 40)
    p = app.params(h.viewer, before=before, n=limit, **params)
    rows = app.q("SELECT f.id AS fid, f.kind, ra.handle AS actor_handle, " + POST_SELECT[len("\nSELECT "):]
                 + " FROM feed f JOIN posts p ON p.id=f.post_id JOIN accounts a ON a.id=p.author_id"
                 " LEFT JOIN accounts o ON o.id=a.bot_owner JOIN accounts ra ON ra.id=f.actor_id"
                 " WHERE (:before=0 OR f.id<:before) AND ra.suspended=0 AND " + VISIBLE + " AND " + UNMUTED
                 + " AND f.actor_id NOT IN (SELECT target_id FROM mutes WHERE account_id=:v)"
                 " AND f.actor_id NOT IN (SELECT target_id FROM blocks WHERE account_id=:v)"
                 " AND (f.kind='post' OR p.hidden=0) AND " + where + " ORDER BY f.id DESC LIMIT :n", p)
    posts = app.posts_out(rows, h.viewer, feed=True)
    return {"posts": posts, "next": rows[-1]["fid"] if len(rows) == limit else None}


@route("GET", "users/([a-z0-9_-]{1,44})/posts")
def api_user_posts(h, handle):
    a = target_user(h, handle)
    h.send_json(200, feed_page(h, "f.actor_id=:who", {"who": a["id"]}))


@route("GET", "users/([a-z0-9_-]{1,44})/(followers|following)")
def api_user_follows(h, handle, which):
    app = h.app
    a = target_user(h, handle)
    before = int_param(h.qs, "before")
    mine, other = ("followee", "follower") if which == "followers" else ("follower", "followee")
    rows = app.q("SELECT f.rowid AS rid, b.* FROM follows f JOIN accounts b ON b.id=f.%s WHERE f.%s=? AND"
                 " (?=0 OR f.rowid<?) AND b.suspended=0 ORDER BY f.rowid DESC LIMIT 40" % (other, mine),
                 (a["id"], before, before))
    h.send_json(200, {"users": [app.user_obj(r, None, full=False) for r in rows],
                      "next": rows[-1]["rid"] if len(rows) == 40 else None})


def require_active(h):
    if h.viewer["suspended"]:
        raise ApiError(403, "suspended", "This account is suspended.")


@route("POST", "users/([a-z0-9_-]{1,44})/(follow|unfollow|mute|unmute|block|unblock)", "user")
def api_relation(h, handle, action):
    app, v = h.app, h.viewer
    require_active(h)
    t = target_user(h, handle)
    if t["id"] == v["id"]:
        raise ApiError(400, "self", "That is your own account.")
    now = app.now()
    app._notified = set()
    with app.tx() as db:
        if action == "follow":
            if db.execute("SELECT 1 FROM blocks WHERE account_id=? AND target_id=?", (t["id"], v["id"])).fetchone():
                raise ApiError(403, "blocked", "This account cannot follow that person.")
            cur = db.execute("INSERT OR IGNORE INTO follows(follower, followee, created) VALUES (?,?,?)",
                             (v["id"], t["id"], now))
            if cur.rowcount:
                app.notify(db, t["id"], "follow", v["id"])
        elif action == "unfollow":
            db.execute("DELETE FROM follows WHERE follower=? AND followee=?", (v["id"], t["id"]))
        elif action in ("mute", "block"):
            db.execute("INSERT OR IGNORE INTO %ss(account_id, target_id, created) VALUES (?,?,?)" % action,
                       (v["id"], t["id"], now))
            if action == "block":
                db.execute("DELETE FROM follows WHERE (follower=? AND followee=?) OR (follower=? AND followee=?)",
                           (v["id"], t["id"], t["id"], v["id"]))
        else:
            db.execute("DELETE FROM %ss WHERE account_id=? AND target_id=?" % action[2:], (v["id"], t["id"]))
    for r in app._notified:
        app.live.notify(r)
    h.send_json(200, {"user": app.user_obj(app.account(t["id"]), app.account(v["id"]))})


@route("POST", "posts", "user")
def api_post_create(h):
    app, v = h.app, h.viewer
    if not v["is_bot"]:
        retry = app.writes.take(v["id"], app.clock())
        if retry:
            raise ApiError(429, "rate", "That is quick. Try again shortly.", retry_after=retry)
    pid, held = app.create_post(v, h.body)
    post = app.posts_out([app.visible_post(pid, v)], v)[0]
    out = {"post": post}
    if held:
        out["held"] = True
        out["message"] = "This post is waiting for a moderator because of many links to one site today."
    h.send_json(201, out)


@route("GET", "posts/([0-9]{1,18})")
def api_post_get(h, pid):
    app, v = h.app, h.viewer
    r = app.visible_post(int(pid), v)
    post = app.posts_out([r], v)[0]
    ancestors, cur, seen = [], r["reply_to"], 0
    while cur and seen < 20:
        seen += 1
        try:
            pr = app.visible_post(cur, v)
        except ApiError:
            ancestors.append({"id": cur, "deleted": True})
            break
        ancestors.append(app.posts_out([pr], v)[0])
        cur = pr["reply_to"]
    ancestors.reverse()
    rows = app.q(POST_SELECT + POST_FROM + " WHERE p.reply_to=:id AND " + VISIBLE + " AND " + UNMUTED
                 + " ORDER BY p.id LIMIT 100", app.params(v, id=r["id"]))
    h.send_json(200, {"post": post, "ancestors": ancestors, "replies": app.posts_out(rows, v)})


@route("POST", "posts/([0-9]{1,18})/(delete|like|unlike|repost|unrepost)", "user")
def api_post_action(h, pid, action):
    app, v = h.app, h.viewer
    require_active(h)
    r = app.visible_post(int(pid), v)
    now = app.now()
    app._notified = set()
    if action == "delete":
        owner = app.account(r["author_id"])
        if r["author_id"] != v["id"] and not (owner and owner["bot_owner"] == v["id"]):
            raise ApiError(403, "forbidden", "Only the author can delete a post.")
        with app.tx() as db:
            app.delete_post_rows(db, "p.id=?", (r["id"],))
        app.live.new_post(app.live.latest)
        return h.send_json(200, {"ok": True, "deleted": r["id"]})
    if r["hidden"]:
        raise ApiError(409, "hidden", "That post is hidden.")
    if action in ("like", "repost") and app.blocked(r["author_id"], v["id"]):
        raise ApiError(403, "blocked", "This account cannot interact with that person.")
    if action == "repost" and app.tier(v) == "readonly":
        raise ApiError(403, "readonly", "Reposting opens with posting for this account.",
                       posting_opens=app.posting_opens(v))
    feed_id = None
    with app.tx() as db:
        if action == "like":
            if db.execute("INSERT OR IGNORE INTO likes(account_id, post_id, created) VALUES (?,?,?)",
                          (v["id"], r["id"], now)).rowcount:
                app.notify(db, r["author_id"], "like", v["id"], r["id"])
        elif action == "unlike":
            db.execute("DELETE FROM likes WHERE account_id=? AND post_id=?", (v["id"], r["id"]))
        elif action == "repost":
            if db.execute("INSERT OR IGNORE INTO reposts(account_id, post_id, created) VALUES (?,?,?)",
                          (v["id"], r["id"], now)).rowcount:
                feed_id = db.execute("INSERT INTO feed(kind, post_id, actor_id, created, top) VALUES"
                                     " ('repost',?,?,?,1)", (r["id"], v["id"], now)).lastrowid
                app.notify(db, r["author_id"], "repost", v["id"], r["id"])
        else:
            db.execute("DELETE FROM reposts WHERE account_id=? AND post_id=?", (v["id"], r["id"]))
            db.execute("DELETE FROM feed WHERE kind='repost' AND post_id=? AND actor_id=?", (r["id"], v["id"]))
    for x in app._notified:
        app.live.notify(x)
    h.send_json(200, {"post": app.posts_out([app.visible_post(r["id"], v)], v)[0]})


@route("GET", "timeline")
def api_timeline(h):
    view = h.qs.get("view", ["everyone"])[0]
    if view == "following":
        if h.viewer is None:
            raise ApiError(401, "signin", "Sign in with GitHub to see who you follow.")
        where = "f.top=1 AND (f.actor_id=:v OR f.actor_id IN (SELECT followee FROM follows WHERE follower=:v))"
    elif view == "everyone":
        where = "f.top=1"
    else:
        raise ApiError(400, "bad_request", "view is everyone or following.")
    h.send_json(200, feed_page(h, where, {}))


@route("GET", "live")
def api_live(h):
    app, v = h.app, h.viewer
    live = app.live
    with live.cond:
        if live.conns >= app.args.max_sse:
            raise ApiError(503, "busy", "Too many live connections. The page still works; reload later.",
                           retry_after=30)
        live.conns += 1
    try:
        h.close_connection = True
        h.send_response(200)
        for k, val in SEC_HEADERS:
            h.send_header(k, val)
        h.send_header("Content-Type", "text/event-stream; charset=utf-8")
        h.send_header("X-Accel-Buffering", "no")
        h.send_header("Connection", "close")
        h.end_headers()
        aid = v["id"] if v else None

        def emit(event, data):
            h.wfile.write(("event: %s\ndata: %s\n\n" % (event, json.dumps(data, separators=(",", ":")))).encode())
            h.wfile.flush()
        h.wfile.write(b"retry: 10000\n\n")
        h.wfile.flush()
        latest = live.latest
        ver = live.versions.get(aid)
        if aid:
            emit("notify", {"unread": app.unread(aid)})
        end = time.monotonic() + app.args.sse_lifetime
        beat = time.monotonic() + app.args.heartbeat
        while time.monotonic() < end:
            with live.cond:
                live.cond.wait(max(0.05, min(beat, end) - time.monotonic()))
                new_latest, new_ver = live.latest, live.versions.get(aid)
            if new_latest != latest:
                latest = new_latest
                emit("new", {"latest": latest})
            if aid and new_ver != ver:
                ver = new_ver
                emit("notify", {"unread": app.unread(aid)})
            if time.monotonic() >= beat:
                h.wfile.write(b": hb\n\n")
                h.wfile.flush()
                beat = time.monotonic() + app.args.heartbeat
    except (BrokenPipeError, ConnectionResetError, socket.timeout, TimeoutError, OSError):
        pass
    finally:
        with live.cond:
            live.conns -= 1


@route("GET", "tags")
def api_tags(h):
    app = h.app
    since = app.now() - 7 * DAY
    counts = {r[0]: r[1] for r in app.q(
        "SELECT t.tag, COUNT(*) FROM post_tags t JOIN posts p ON p.id=t.post_id JOIN accounts a ON a.id=p.author_id"
        " WHERE t.created>? AND p.hidden=0 AND a.suspended=0 GROUP BY t.tag ORDER BY COUNT(*) DESC, t.tag LIMIT 30",
        (since,))}
    tags = [{"tag": t, "count": counts.get(t, 0), "featured": True} for t in FEATURED_TAGS]
    tags += [{"tag": t, "count": c, "featured": False} for t, c in counts.items() if t not in FEATURED_TAGS][:20]
    h.send_json(200, {"tags": tags})


def post_list(h, where, params):
    app = h.app
    before = int_param(h.qs, "before")
    p = app.params(h.viewer, before=before, **params)
    rows = app.q(POST_SELECT + POST_FROM + " WHERE (:before=0 OR p.id<:before) AND " + VISIBLE + " AND " + UNMUTED
                 + " AND p.hidden=0 AND " + where + " ORDER BY p.id DESC LIMIT 20", p)
    return app.posts_out(rows, h.viewer), rows[-1]["id"] if len(rows) == 20 else None


@route("GET", "tags/([A-Za-z][A-Za-z0-9_]{0,31})")
def api_tag(h, tag):
    posts, nxt = post_list(h, "p.id IN (SELECT post_id FROM post_tags WHERE tag=:tag)", {"tag": tag.lower()})
    h.send_json(200, {"tag": tag.lower(), "posts": posts, "next": nxt})


@route("GET", "search")
def api_search(h):
    app = h.app
    q = h.qs.get("q", [""])[0][:200]
    match = fts_query(q)
    if not match:
        return h.send_json(200, {"posts": [], "users": [], "next": None})
    posts, nxt = post_list(h, "p.id IN (SELECT rowid FROM posts_fts WHERE posts_fts MATCH :m)", {"m": match})
    users = []
    if not h.qs.get("before", [""])[0]:
        needle = q.strip().lstrip("@").lower()[:60]
        rows = app.q("SELECT * FROM accounts WHERE suspended=0 AND (instr(handle, ?)>0 OR instr(lower(display_name),"
                     " ?)>0) ORDER BY length(handle) LIMIT 10", (needle, needle)) if needle else []
        users = [app.user_obj(r, None, full=False) for r in rows]
    h.send_json(200, {"posts": posts, "users": users, "next": nxt})


@route("GET", "notifications", "user")
def api_notifications(h):
    app, v = h.app, h.viewer
    before = int_param(h.qs, "before")
    rows = app.q("SELECT n.*, b.handle, b.display_name, b.avatar FROM notifications n JOIN accounts b ON"
                 " b.id=n.actor_id WHERE n.account_id=? AND (?=0 OR n.id<?) AND n.actor_id NOT IN (SELECT target_id"
                 " FROM mutes WHERE account_id=?) AND n.actor_id NOT IN (SELECT target_id FROM blocks WHERE"
                 " account_id=?) ORDER BY n.id DESC LIMIT 30", (v["id"], before, before, v["id"], v["id"]))
    out = []
    for r in rows:
        item = {"id": r["id"], "kind": r["kind"], "actor": {"handle": r["handle"], "display_name": r["display_name"],
                                                             "avatar": r["avatar"]},
                "at": iso(r["created"]), "read": bool(r["read"])}
        if r["post_id"]:
            try:
                item["post"] = app.posts_out([app.visible_post(r["post_id"], v)], v)[0]
            except ApiError:
                item["post"] = {"id": r["post_id"], "deleted": True}
        out.append(item)
    h.send_json(200, {"notifications": out, "unread": app.unread(v["id"]),
                      "next": rows[-1]["id"] if len(rows) == 30 else None})


@route("POST", "notifications/read", "user")
def api_notifications_read(h):
    app, v = h.app, h.viewer
    with app.tx() as db:
        db.execute("UPDATE notifications SET read=1 WHERE account_id=? AND read=0", (v["id"],))
    app.live.notify(v["id"])
    h.send_json(200, {"unread": 0})


@route("POST", "reports", "user")
def api_report(h):
    app, v, b = h.app, h.viewer, h.body
    if b.get("reason") not in REPORT_REASONS:
        raise ApiError(400, "bad_request", "Pick a reason: " + ", ".join(REPORT_REASONS) + ".")
    note = clean_line(b.get("note", ""), 500, multiline=True)
    if (b.get("post") is None) == (b.get("user") is None):
        raise ApiError(400, "bad_request", "Report one post or one account.")
    if app.q1("SELECT COUNT(*) FROM reports WHERE reporter_id=? AND created>?", (v["id"], app.now() - DAY))[0] >= 20:
        raise ApiError(429, "rate", "That is the daily report limit. Thank you for the help.", retry_after=3600)
    if b.get("post") is not None:
        r = app.visible_post(b["post"], v)
        kind, post_id, acct = "post", r["id"], r["author_id"]
    else:
        t = app.by_handle(b["user"]) if isinstance(b["user"], str) else None
        if t is None:
            raise ApiError(404, "not_found", "No such account.")
        kind, post_id, acct = "user", None, t["id"]
    with app.tx() as db:
        rid = db.execute("INSERT INTO reports(reporter_id, target_kind, post_id, account_id, reason, note, created)"
                         " VALUES (?,?,?,?,?,?,?)", (v["id"], kind, post_id, acct, b["reason"], note,
                                                      app.now())).lastrowid
    h.send_json(201, {"ok": True, "report": rid})


@route("GET", "mod/queue", "mod")
def api_mod_queue(h):
    app, v = h.app, h.viewer
    rows = app.q("SELECT r.*, rp.handle AS reporter FROM reports r LEFT JOIN accounts rp ON rp.id=r.reporter_id"
                 " WHERE r.status='open' ORDER BY r.id LIMIT 100")
    out = []
    for r in rows:
        item = {"id": r["id"], "at": iso(r["created"]), "reason": r["reason"], "note": r["note"],
                "reporter": r["reporter"] or "system", "target_kind": r["target_kind"]}
        if r["post_id"]:
            try:
                item["post"] = app.posts_out([app.visible_post(r["post_id"], v)], v)[0]
            except ApiError:
                item["post"] = {"id": r["post_id"], "deleted": True}
        if r["account_id"]:
            a = app.account(r["account_id"])
            if a is not None:
                item["user"] = app.user_obj(a, None, full=False)
                item["user"]["tier"] = app.tier(a)
                item["user"]["suspended"] = bool(a["suspended"])
        out.append(item)
    h.send_json(200, {"reports": out, "next": None})


@route("POST", "mod/action", "mod")
def api_mod_action(h):
    app, v, b = h.app, h.viewer, h.body
    action, target = b.get("action"), b.get("target")
    if action not in MOD_ACTIONS or not isinstance(target, str):
        raise ApiError(400, "bad_request", "Unknown action or target.")
    reason = clean_line(b.get("reason_public", ""), 200)
    note = clean_line(b.get("note_private", ""), 500, multiline=True)
    value = b.get("value")
    kind, _, ref = target.partition(":")
    report = post = acct = None
    if kind == "report" and ref.isdigit():
        report = app.q1("SELECT * FROM reports WHERE id=?", (int(ref),))
        if report is None:
            raise ApiError(404, "not_found", "No such report.")
        post = app.q1("SELECT * FROM posts WHERE id=?", (report["post_id"],)) if report["post_id"] else None
        acct = app.account(report["account_id"]) if report["account_id"] else None
    elif kind == "post" and ref.isdigit():
        post = app.q1("SELECT * FROM posts WHERE id=?", (int(ref),))
        if post is None:
            raise ApiError(404, "not_found", "No such post.")
        acct = app.account(post["author_id"])
    elif kind == "user":
        acct = app.by_handle(ref)
        if acct is None:
            raise ApiError(404, "not_found", "No such account.")
    else:
        raise ApiError(400, "bad_request", "Target is post:<id>, user:<handle> or report:<id>.")
    if action in ("hide", "unhide") and post is None:
        raise ApiError(400, "bad_request", "Hide and unhide act on a post.")
    if action == "dismiss" and report is None:
        raise ApiError(400, "bad_request", "Dismiss acts on a report.")
    if action in ("suspend", "unsuspend", "set_tier", "set_role"):
        if acct is None:
            raise ApiError(400, "bad_request", "That action acts on an account.")
        if acct["id"] == app.aien_id or acct["gh_id"] == ADMIN_GH_ID or acct["id"] == v["id"]:
            raise ApiError(403, "forbidden", "That account cannot be changed here.")
        if acct["role"] in ("moderator", "admin") and v["role"] != "admin":
            raise ApiError(403, "forbidden", "Only the admin can act on a moderator.")
    if action == "set_tier" and value not in ("readonly", "probation", "member", "auto"):
        raise ApiError(400, "bad_request", "Tier is readonly, probation, member or auto.")
    if action == "set_role":
        if v["role"] != "admin":
            raise ApiError(403, "forbidden", "Admin only.")
        if value not in ("member", "moderator") or acct["is_bot"]:
            raise ApiError(400, "bad_request", "Role is member or moderator, for a person.")
    now = app.now()
    with app.tx() as db:
        if action == "hide":
            db.execute("UPDATE posts SET hidden=1 WHERE id=?", (post["id"],))
        elif action == "unhide":
            db.execute("UPDATE posts SET hidden=0 WHERE id=?", (post["id"],))
        elif action == "suspend":
            db.execute("UPDATE accounts SET suspended=1 WHERE id=? OR bot_owner=?", (acct["id"], acct["id"]))
        elif action == "unsuspend":
            db.execute("UPDATE accounts SET suspended=0 WHERE id=? OR bot_owner=?", (acct["id"], acct["id"]))
        elif action == "set_tier":
            db.execute("UPDATE accounts SET tier_override=? WHERE id=?", (None if value == "auto" else value, acct["id"]))
        elif action == "set_role":
            db.execute("UPDATE accounts SET role=? WHERE id=?", (value, acct["id"]))
        if report is not None:
            db.execute("UPDATE reports SET status=? WHERE id=?",
                       ("dismissed" if action == "dismiss" else "actioned", report["id"]))
        elif post is not None and action in ("hide", "unhide"):
            db.execute("UPDATE reports SET status='actioned' WHERE post_id=? AND status='open'", (post["id"],))
        if action == "dismiss":
            tk = "report"
        elif action in ("hide", "unhide"):
            tk = "post"
        else:
            tk = "account"
        lid = db.execute("INSERT INTO modlog(at, action, target_kind, target_post, target_account, value,"
                         " reason_public, note_private, by_id, by_handle) VALUES (?,?,?,?,?,?,?,?,?,?)",
                         (now, action, tk, post["id"] if post is not None and tk == "post" else None,
                          acct["id"] if acct is not None else None, str(value or "") if action.startswith("set_") else "",
                          reason, note, v["id"], v["handle"])).lastrowid
    app.live.new_post(app._latest_locked())
    h.send_json(200, {"ok": True, "modlog": lid})


def _latest_locked(self):
    with self.lock:
        return self._latest()


App._latest_locked = _latest_locked


@route("GET", "modlog")
def api_modlog(h):
    app = h.app
    before = int_param(h.qs, "before")
    rows = app.q("SELECT * FROM modlog WHERE (?=0 OR id<?) ORDER BY id DESC LIMIT 40", (before, before))
    out = []
    for r in rows:
        if r["target_kind"] == "post":
            target = str(r["target_post"])
        elif r["target_kind"] == "report":
            target = "a report"
        else:
            target = "an account"
        out.append({"id": r["id"], "at": iso(r["at"]), "action": r["action"], "target_kind": r["target_kind"],
                    "target": target, "reason_public": r["reason_public"], "by": r["by_handle"]})
    h.send_json(200, {"entries": out, "next": rows[-1]["id"] if len(rows) == 40 else None})


@route("POST", "bots", "user")
def api_bot_create(h):
    app, v, b = h.app, h.viewer, h.body
    require_active(h)
    if v["is_bot"] or app.tier(v) != "member":
        raise ApiError(403, "tier", "Bots can be created by member-tier accounts.")
    name = b.get("name")
    if not isinstance(name, str) or not BOT_NAME.match(name):
        raise ApiError(400, "bad_request", "A bot name is 1 to 35 lowercase letters, digits or dashes.")
    handle = name + "_bot"
    dn = clean_line(b.get("display_name", name), 50) or name
    bio = clean_line(b.get("bio", ""), 280, multiline=True)
    token = "cmb_" + secrets.token_urlsafe(32)
    with app.tx() as db:
        if db.execute("SELECT COUNT(*) FROM accounts WHERE bot_owner=? AND bot_token IS NOT NULL",
                      (v["id"],)).fetchone()[0] >= 2:
            raise ApiError(409, "bot_limit", "An account can run at most 2 bots.")
        if db.execute("SELECT 1 FROM accounts WHERE handle=?", (handle,)).fetchone():
            raise ApiError(409, "taken", "That bot name is taken.")
        db.execute("INSERT INTO accounts(handle, display_name, bio, joined, is_bot, bot_owner, bot_token, label)"
                   " VALUES (?,?,?,?,1,?,?,'Bot')", (handle, dn, bio, app.now(), v["id"], sha256(token)))
    h.send_json(201, {"handle": handle, "token": token})


@route("POST", "bots/([a-z0-9-]{1,35}_bot)/revoke", "user")
def api_bot_revoke(h, handle):
    app, v = h.app, h.viewer
    a = app.by_handle(handle)
    if a is None or not a["is_bot"] or (a["bot_owner"] != v["id"] and v["role"] not in ("moderator", "admin")):
        raise ApiError(404, "not_found", "No such bot of yours.")
    with app.tx() as db:
        db.execute("UPDATE accounts SET bot_token=NULL WHERE id=?", (a["id"],))
    h.send_json(200, {"ok": True})


@route("GET", "me/export", "user")
def api_export(h):
    v = h.viewer
    raw = json.dumps(h.app.export(v), ensure_ascii=False, indent=1).encode()
    h.send(200, raw, "application/json; charset=utf-8",
           [("Content-Disposition", 'attachment; filename="aienos-community-%s.json"' % v["handle"])])


@route("POST", "me/delete", "user")
def api_delete_me(h):
    v = h.viewer
    if h.body.get("confirm") != v["handle"]:
        raise ApiError(400, "confirm", "Type your handle to confirm.")
    h.app.delete_account(v)
    h.send_json(200, {"ok": True, "deleted": True},
                [("Set-Cookie", "cm_s=; HttpOnly; Secure; SameSite=Lax; Path=/community/; Max-Age=0")])


# ---------------------------------------------------------------- Atom feeds

def xml_escape(s):
    return (str(s).replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
            .replace('"', "&quot;").replace("'", "&#39;"))


def feeds(h, rest):
    app = h.app
    base = app.args.public_base
    if rest == "all.atom":
        where, params, title, page = "1=1", {}, "aienos.com Community", "/community/"
    elif rest.startswith("u/") and rest.endswith(".atom"):
        a = app.by_handle(rest[2:-5])
        if a is None or a["suspended"]:
            raise ApiError(404, "not_found", "No such account.")
        where, params = "p.author_id=:who", {"who": a["id"]}
        title, page = "@%s on the aienos.com Community" % a["handle"], "/community/?u=" + a["handle"]
    elif rest.startswith("tag/") and rest.endswith(".atom") and TAG.match(rest[4:-5].lower()):
        tag = rest[4:-5].lower()
        where, params = "p.id IN (SELECT post_id FROM post_tags WHERE tag=:tag)", {"tag": tag}
        title, page = "#%s on the aienos.com Community" % tag, "/community/?tag=" + tag
    else:
        raise ApiError(404, "not_found", "No such feed.")
    rows = app.q(POST_SELECT + POST_FROM + " WHERE p.hidden=0 AND a.suspended=0 AND " + where
                 + " ORDER BY p.id DESC LIMIT 40", dict(params, v=0, mod=0))
    updated = iso(rows[0]["created"] if rows else app.now())
    out = ['<?xml version="1.0" encoding="utf-8"?>', '<feed xmlns="http://www.w3.org/2005/Atom">',
           "<title>%s</title>" % xml_escape(title),
           '<link rel="alternate" type="text/html" href="%s"/>' % xml_escape(base + page),
           '<link rel="self" href="%s"/>' % xml_escape(base + "/community/feed/" + rest),
           "<id>%s</id>" % xml_escape(base + "/community/feed/" + rest), "<updated>%s</updated>" % updated]
    for r in rows:
        first = " ".join(r["text"].split())
        out += ["<entry>", "<title>%s</title>" % xml_escape(first[:80] + ("..." if len(first) > 80 else "")),
                '<link rel="alternate" type="text/html" href="%s"/>' % xml_escape(base + "/community/?p=%d" % r["id"]),
                "<id>tag:aienos.com,2026:community/post/%d</id>" % r["id"],
                "<published>%s</published>" % iso(r["created"]), "<updated>%s</updated>" % iso(r["created"]),
                "<author><name>%s</name><uri>%s</uri></author>" % (
                    xml_escape(r["a_dn"] or r["a_handle"]), xml_escape(base + "/community/?u=" + r["a_handle"])),
                '<content type="html">%s</content>' % xml_escape(r["html"]), "</entry>"]
    out.append("</feed>")
    h.send(200, "\n".join(out).encode(), "application/atom+xml; charset=utf-8")


# ---------------------------------------------------------------- main

def parser():
    p = argparse.ArgumentParser(description="aienos.com Community backend")
    p.add_argument("--host", default="127.0.0.1")
    p.add_argument("--port", type=int, default=18130)
    p.add_argument("--db", default="/var/lib/aien-community/community.db")
    p.add_argument("--render", default="/opt/aien-community/render")
    p.add_argument("--client-id", default=os.environ.get("COMMUNITY_GITHUB_CLIENT_ID", ""))
    p.add_argument("--secret-stdin", action="store_true",
                   help="read the OAuth client secret as one line on stdin (empty: sign-in closed)")
    p.add_argument("--public-base", default="https://www.aienos.com")
    p.add_argument("--allow-origin", action="append", default=None)
    p.add_argument("--github-web", default=GITHUB_WEB)
    p.add_argument("--github-api", default=GITHUB_API)
    p.add_argument("--max-threads", type=int, default=96)
    p.add_argument("--max-sse", type=int, default=64)
    p.add_argument("--heartbeat", type=float, default=25.0)
    p.add_argument("--sse-lifetime", type=float, default=1800.0)
    p.add_argument("--writes-per-minute", type=int, default=30)
    p.add_argument("--aien", action="store_true", help="answer @aien mentions through allen-chat")
    p.add_argument("--aien-gateway", default="http://100.64.0.2:18110")
    p.add_argument("--no-nightly", action="store_true")
    return p


def build(argv=None, secret=None, clock=time.time):
    args = parser().parse_args(argv)
    if args.allow_origin is None:
        args.allow_origin = list(ORIGINS)
    if args.max_sse >= args.max_threads:
        raise SystemExit("--max-sse must be below --max-threads")
    app = App(args, secret, clock)
    srv = Server((args.host, args.port), app, args.max_threads)
    return app, srv


def main(argv=None):
    secret = None
    if "--secret-stdin" in (argv if argv is not None else sys.argv[1:]):
        secret = sys.stdin.readline().strip() or None
        sys.stdin.close()
    app, srv = build(argv, secret)
    del secret
    if app.args.aien:
        threading.Thread(target=app.aien_loop, daemon=True).start()
    if not app.args.no_nightly:
        threading.Thread(target=app.nightly_loop, daemon=True).start()
    print("community: listening on %s:%d, sign-in %s, aien %s" % (
        app.args.host, app.args.port, "open" if app.signin_open else "closed",
        "on" if app.args.aien else "off"), file=sys.stderr, flush=True)
    signal.signal(signal.SIGTERM, lambda *a: (_ for _ in ()).throw(KeyboardInterrupt()))
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        pass
    srv.server_close()


if __name__ == "__main__":
    main()
