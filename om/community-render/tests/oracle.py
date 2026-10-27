#!/usr/bin/env python3
"""Independent verification oracle for render.om; never part of the serving path.

Written from the DESIGN.md render contract and the resolution notes at the top of
render.om, using regular expressions instead of the byte scanner in render.om.
render(data) -> (exit_code, html_bytes, meta_bytes).
"""
import re

CAP = 4000
MAX_LINKS = 16
MAX_BLOCKS = 8

ENT = {'&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;'}
LANG = re.compile(r'[a-z0-9+#-]{0,16}')
URL_RUN = re.compile(r"(?i)(https?)://([A-Za-z0-9\-._~:/?#\[\]@!$&'()*+,;=%]*)")
TAG = re.compile(r'#([A-Za-z][A-Za-z0-9_]{0,31})(?![A-Za-z0-9_])')
MENTION = re.compile(r'@([A-Za-z0-9][A-Za-z0-9_-]{0,38})(?![A-Za-z0-9_-])')
REL = 'nofollow ugc noopener noreferrer'


def escape(text):
    out = []
    for ch in text:
        if ch in ENT:
            out.append(ENT[ch])
        elif (ord(ch) < 32 and ch not in '\t\n') or ord(ch) == 127:
            out.append('&#xFFFD;')
        else:
            out.append(ch)
    return ''.join(out)


class Doc:
    def __init__(self):
        self.links = 0
        self.blocks = 0
        self.meta = []

    def note(self, line, dedup=True):
        if dedup and line in self.meta:
            return
        self.meta.append(line)

    def url_at(self, line, i):
        m = URL_RUN.match(line, i)
        if not m:
            return None
        rest = m.group(2)
        while rest:
            if rest[-1] in ".,;:!?'":
                rest = rest[:-1]
            elif rest[-1] == ')' and rest.count(')') > rest.count('('):
                rest = rest[:-1]
            else:
                break
        host = re.split(r'[/?#]', rest, maxsplit=1)[0]
        if not host:
            return None
        return m.group(1) + '://' + rest, rest

    def inline(self, line):
        out = []
        i = 0
        while i < len(line):
            ch = line[i]
            prev = line[i - 1] if i else None
            if ch == '`':
                j = line.find('`', i + 1)
                if j >= 0:
                    out.append('<code>' + escape(line[i + 1:j]) + '</code>')
                    i = j + 1
                    continue
            if ch in 'hH' and self.links < MAX_LINKS and (prev is None or not re.fullmatch(r'[A-Za-z0-9]', prev)):
                found = self.url_at(line, i)
                if found:
                    url, shown = found
                    self.links += 1
                    if len(shown) > 48:
                        shown = shown[:48] + '\u2026'
                    out.append('<a href="%s" rel="%s" target="_blank">%s</a>' % (escape(url), REL, escape(shown)))
                    self.note('l ' + url)
                    i += len(url)
                    continue
            if ch == '#' and (prev is None or not re.fullmatch(r'[A-Za-z0-9_&/]', prev)):
                m = TAG.match(line, i)
                if m:
                    word = m.group(1)
                    out.append('<a class="cm-tag" href="/community/?tag=%s">#%s</a>' % (word.lower(), word))
                    self.note('t ' + word.lower())
                    i = m.end()
                    continue
            if ch == '@' and (prev is None or not re.fullmatch(r'[A-Za-z0-9_.]', prev)):
                m = MENTION.match(line, i)
                if m:
                    word = m.group(1)
                    out.append('<a class="cm-at" href="/community/?u=%s">@%s</a>' % (word.lower(), word))
                    self.note('m ' + word.lower())
                    i = m.end()
                    continue
            out.append(escape(ch))
            i += 1
        return ''.join(out)

    def render(self, text):
        lines = text.split('\n')
        if lines and lines[-1] == '':
            lines.pop()
        html = []
        para = None
        k = 0
        while k < len(lines):
            line = lines[k]
            if line.startswith('```') and self.blocks < MAX_BLOCKS:
                if para is not None:
                    html.append('<p>' + '<br>'.join(para) + '</p>')
                    para = None
                self.blocks += 1
                info = line[3:]
                lang = info if LANG.fullmatch(info) else ''
                attr = ' data-lang="%s"' % lang if lang else ''
                self.note('c ' + lang, dedup=False)
                body = []
                k += 1
                while k < len(lines) and lines[k] != '```':
                    body.append(lines[k])
                    k += 1
                k += 1
                html.append('<pre class="cm-code"%s><code>%s</code></pre>' % (attr, escape('\n'.join(body))))
                continue
            if line.strip(' \t') == '':
                if para is not None:
                    html.append('<p>' + '<br>'.join(para) + '</p>')
                    para = None
            else:
                para = (para or []) + [self.inline(line)]
            k += 1
        if para is not None:
            html.append('<p>' + '<br>'.join(para) + '</p>')
        meta = ''.join(m + '\n' for m in self.meta)
        return ''.join(html), meta


def render(data):
    if len(data) > CAP:
        return 2, b'', b''
    try:
        text = data.decode('utf-8', errors='strict')
    except UnicodeDecodeError:
        return 3, b'', b''
    html, meta = Doc().render(text)
    return 0, html.encode('utf-8'), meta.encode('utf-8')


if __name__ == '__main__':
    import sys
    import os
    code, html, meta = render(sys.stdin.buffer.read())
    sys.stdout.buffer.write(html)
    os.write(3, meta)
    sys.exit(code)
