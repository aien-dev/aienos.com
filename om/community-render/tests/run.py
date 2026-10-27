#!/usr/bin/env python3
"""Host verification driver for render.om; never part of the serving path.

run.py BINARY [--fast]
Runs the hand corpus (cases.jsonl) against the binary and the oracle, a deterministic
fuzz set compared byte for byte with the oracle, an XSS property check on every output,
and self-tests that prove the property checker rejects known-bad markup.
Prints one RESULT line; exit 0 only when everything passed. --fast stops at the first failure.
"""
import json
import os
import random
import re
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.dont_write_bytecode = True
sys.path.insert(0, str(HERE))
import oracle  # noqa: E402

FUZZ_SEED = 20261027
FUZZ_CASES = 2400


def run_binary(binary, data):
    r_fd, w_fd = os.pipe()
    try:
        proc = subprocess.Popen([binary], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE, pass_fds=(w_fd, 3),
                                preexec_fn=lambda: os.dup2(w_fd, 3))
    finally:
        os.close(w_fd)
    try:
        html, err = proc.communicate(data, timeout=20)
    except BrokenPipeError:
        html, err = proc.stdout.read(), b''
        proc.wait(timeout=20)
    with os.fdopen(r_fd, 'rb') as meta_pipe:
        meta = meta_pipe.read()
    return proc.returncode, html, meta, err


# ---- XSS property check (oracle side) ----
TEXT_ENT = re.compile(r'&(amp|lt|gt|quot|#39|#xFFFD);')
ATTR_ENT = re.compile(r'&(amp|#39);')
SIMPLE = {'<p>': 'p', '</p>': '/p', '<br>': 'br', '<code>': 'code', '</code>': '/code',
          '</pre>': '/pre', '</a>': '/a'}
PRE_RE = re.compile(r'<pre class="cm-code"(?: data-lang="([a-z0-9+#-]{1,16})")?>')
LINK_RE = re.compile(r'<a href="([^"<>\']*)" rel="nofollow ugc noopener noreferrer" target="_blank">')
CMA_RE = re.compile(r'<a class="cm-(tag|at)" href="([^"<>\']*)">')
URL_OK = re.compile(r"(?i)https?://[A-Za-z0-9\-._~:/?#\[\]@!$&'()*+,;=%]+")
TAG_HREF = re.compile(r'/community/\?tag=[a-z][a-z0-9_]{0,31}')
AT_HREF = re.compile(r'/community/\?u=[a-z0-9][a-z0-9_-]{0,38}')
META_RE = re.compile(r"t [a-z][a-z0-9_]{0,31}|m [a-z0-9][a-z0-9_-]{0,38}|"
                     r"l (?i:https?)://[A-Za-z0-9\-._~:/?#\[\]@!$&'()*+,;=%]+|c [a-z0-9+#-]{0,16}")
PARENTS = {'p': {None}, 'pre': {None}, 'br': {'p'}, 'code': {'p', 'pre'}, 'a': {'p'}}


def attr_value(raw):
    if '<' in raw or '>' in raw or '"' in raw or "'" in raw:
        raise ValueError('raw markup byte in attribute: %r' % raw)
    stripped = ATTR_ENT.sub('', raw)
    if '&' in stripped:
        raise ValueError('bad entity in attribute: %r' % raw)
    return raw.replace('&#39;', "'").replace('&amp;', '&')


def check_text(segment):
    if any(ch in segment for ch in '<>"\''):
        raise ValueError('raw markup byte in text: %r' % segment)
    if '&' in TEXT_ENT.sub('', segment):
        raise ValueError('bad entity in text: %r' % segment)
    if re.search(r'[\x00-\x08\x0b-\x1f\x7f]', segment):
        raise ValueError('control byte in text')


def xss_check(html_bytes):
    html = html_bytes.decode('utf-8')
    stack = []
    i = 0
    while i < len(html):
        j = html.find('<', i)
        if j < 0:
            check_text(html[i:])
            break
        check_text(html[i:j])
        name = None
        for literal, tag in SIMPLE.items():
            if html.startswith(literal, j):
                name, end = tag, j + len(literal)
                break
        if name is None:
            m = PRE_RE.match(html, j) or LINK_RE.match(html, j) or CMA_RE.match(html, j)
            if not m:
                raise ValueError('disallowed markup at %d: %r' % (j, html[j:j + 40]))
            end = m.end()
            if m.re is PRE_RE:
                name = 'pre'
            elif m.re is LINK_RE:
                name = 'a'
                url = attr_value(m.group(1))
                if not URL_OK.fullmatch(url):
                    raise ValueError('bad href %r' % url)
            else:
                name = 'a'
                href = attr_value(m.group(2))
                pattern = TAG_HREF if m.group(1) == 'tag' else AT_HREF
                if not pattern.fullmatch(href):
                    raise ValueError('bad internal href %r' % href)
        if name.startswith('/'):
            if not stack or stack[-1] != name[1:]:
                raise ValueError('unbalanced close %s' % name)
            stack.pop()
        else:
            parent = stack[-1] if stack else None
            if parent not in PARENTS[name]:
                raise ValueError('%s inside %s' % (name, parent))
            if name != 'br':
                stack.append(name)
        i = end
    if stack:
        raise ValueError('unclosed %s' % stack)


def meta_check(meta_bytes):
    text = meta_bytes.decode('utf-8')
    if text and not text.endswith('\n'):
        raise ValueError('meta without final LF')
    seen = set()
    for line in text.split('\n')[:-1]:
        if not META_RE.fullmatch(line):
            raise ValueError('bad meta line %r' % line)
        if line[0] != 'c':
            if line in seen:
                raise ValueError('duplicate meta %r' % line)
            seen.add(line)


def checker_self_test():
    """Negative controls: the property checker must reject each of these."""
    rel = 'rel="nofollow ugc noopener noreferrer" target="_blank"'
    bad = ['<script>alert(1)</script>', '<p onclick="x">a</p>', '<p>a"b</p>', "<p>a'b</p>", '<p>a>b</p>',
           '<a href="javascript:alert(1)" %s>x</a>' % rel, '<a href="data:x" %s>x</a>' % rel,
           '<a href="https://a.com/&quot;x" %s>x</a>' % rel, '<p>&#x6a;</p>', '<p>&</p>',
           '<pre class="cm-code" data-lang="x y"><code></code></pre>', '<p><p>a</p></p>',
           '<a class="cm-tag" href="/community/?tag=A">#A</a>', '<p><a class="cm-at" href="//evil">@x</a></p>',
           '<p>a', '<img src=x>', '<p>\x00</p>', '<p><code><a class="cm-at" href="/community/?u=x">@x</a></code></p>']
    for sample in bad:
        try:
            xss_check(sample.encode())
        except ValueError:
            continue
        raise AssertionError('checker accepted bad sample %r' % sample)
    for sample in [b't Tag\n', b'x y\n', b'l javascript:alert(1)\n', b't a\nt a\n', b'c py']:
        try:
            meta_check(sample)
        except ValueError:
            continue
        raise AssertionError('meta checker accepted %r' % sample)
    xss_check(b'<p>a<br><code>&lt;</code></p><pre class="cm-code" data-lang="om"><code>x\ny</code></pre>')
    return len(bad) + 5


# ---- fuzz generator ----
TOKENS = ['a', 'b', 'Z', '9', ' ', ' ', '\t', '\n', '\n', '\n\n', '`', '``', '```', '```om\n', '```py\n', '\n```\n',
          '```Py\n', '```' + 'x' * 17 + '\n', '#', '#tag', '#Omega', '#1', '#a_b', '#' + 'x' * 32, '#' + 'x' * 33,
          '@', '@drake', '@A-b', '@-x', '@' + 'q' * 39, '@' + 'q' * 40, '&', '&#', '&#x6a;', '/', '.', ',', ';',
          ':', '!', '?', "'", '"', '(', ')', '[', ']', '<', '>', '<script>', '</p>', '" onmouseover="',
          'javascript:', 'http://', 'https://', 'HTTP://', 'hTtPs://', 'https://a.com', 'http://x.y/z?q=1&r=2',
          'https://en.wikipedia.org/wiki/A_(b)', 'https://' + 'w' * 50, 'https:///', 'http://?', 'h', 'é', '世',
          '🎉', '\u2026', '\x00', '\x01', '\x7f', '\r', '\u0800', '\U0010ffff', '_', '-', '~', '%', '=', '+', '*']


def fuzz_cases(count):
    rng = random.Random(FUZZ_SEED)
    for k in range(count):
        mode = k % 10
        if mode < 7:
            n = rng.choice([1, 2, 3, 5, 8, 20, 60, 200, 800])
            text = ''.join(rng.choice(TOKENS) for _ in range(n))
            data = text.encode('utf-8')
            if mode == 6:
                pad = rng.choice([3990, 3999, 4000, 4001, 4010]) - len(data)
                data = data + b'a' * max(pad, 0)
        elif mode == 7:
            data = bytes(rng.randrange(256) for _ in range(rng.randrange(0, 40)))
        elif mode == 8:
            cps = []
            for _ in range(rng.randrange(0, 60)):
                cp = rng.choice([rng.randrange(0, 128), rng.randrange(128, 0x800),
                                 rng.randrange(0x800, 0xd800), rng.randrange(0x10000, 0x110000)])
                cps.append(chr(cp))
            data = ''.join(cps).encode('utf-8')
        else:
            base = ''.join(rng.choice(TOKENS) for _ in range(30)).encode('utf-8')
            pos = rng.randrange(len(base) + 1)
            data = base[:pos] + bytes([rng.randrange(128, 256)]) + base[pos:]
        yield 'fuzz%04d' % k, data


def load_corpus():
    cases = []
    for line in (HERE / 'cases.jsonl').read_text(encoding='utf-8').splitlines():
        c = json.loads(line)
        data = bytes.fromhex(c['input_hex']) if 'input_hex' in c else c['input'].encode('utf-8')
        cases.append((c['name'], data, c['rc'], c['html'].encode('utf-8'), c['meta'].encode('utf-8')))
    return cases


def main():
    binary = os.path.abspath(sys.argv[1])
    fast = '--fast' in sys.argv
    failures = []

    def fail(msg):
        failures.append(msg)
        if fast:
            print('FAIL', msg)
            sys.exit(1)

    controls = checker_self_test()
    corpus = load_corpus()
    outputs = 0
    for name, data, rc, html, meta in corpus:
        want = (rc, html, meta)
        if oracle.render(data) != want:
            fail('oracle disagrees with hand expectation: %s' % name)
        got_rc, got_html, got_meta, err = run_binary(binary, data)
        if (got_rc, got_html, got_meta) != want or err:
            fail('binary disagrees with hand expectation: %s rc=%s html=%r meta=%r'
                 % (name, got_rc, got_html[:200], got_meta[:200]))
            continue
        try:
            xss_check(got_html)
            meta_check(got_meta)
            outputs += 1
        except ValueError as e:
            fail('property %s: %s' % (name, e))
    fuzz = 0
    rcs = {}
    for name, data in fuzz_cases(FUZZ_CASES):
        want = oracle.render(data)
        got_rc, got_html, got_meta, err = run_binary(binary, data)
        rcs[got_rc] = rcs.get(got_rc, 0) + 1
        if (got_rc, got_html, got_meta) != want or err:
            fail('fuzz mismatch %s input=%r rc=%s/%s' % (name, data[:120], got_rc, want[0]))
            continue
        try:
            xss_check(got_html)
            meta_check(got_meta)
            outputs += 1
        except ValueError as e:
            fail('property %s: %s' % (name, e))
        fuzz += 1
    # Exit status for a closed fd 3: refuse rather than drop metadata silently.
    proc = subprocess.run([binary], input=b'#tag', capture_output=True, timeout=20)
    if proc.returncode != 4:
        fail('closed fd3 should exit 4, got %s' % proc.returncode)
    # One render's wall time, process spawn included; best of 20 on a 4000-byte rich post.
    rich = (('hi @drake see https://example.com/x #omega `code` <b>&\n' * 80)[:3990]).encode()
    best = min(_timed(binary, rich) for _ in range(20))
    if failures:
        for f in failures[:20]:
            print('FAIL', f)
        print('RESULT FAIL failures=%d' % len(failures))
        sys.exit(1)
    print('RESULT cases=%d fuzz=%d xss_checked=%d checker_controls=%d rc_mix=%s render_ms=%.2f'
          % (len(corpus), fuzz, outputs, controls, json.dumps(rcs, sort_keys=True), best * 1000))


def _timed(binary, data):
    start = time.perf_counter()
    rc, _, _, _ = run_binary(binary, data)
    elapsed = time.perf_counter() - start
    assert rc == 0
    return elapsed


if __name__ == '__main__':
    main()
