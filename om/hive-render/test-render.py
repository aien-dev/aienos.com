#!/usr/bin/env python3
"""Independent Python/HTML parser oracle; never part of the serving path."""
import copy
import datetime
import html
from html.parser import HTMLParser
import json
from pathlib import Path
import subprocess
import sys

BINARY, OMC, SCRATCH = map(Path, sys.argv[1:])
BINARY, OMC = BINARY.resolve(), OMC.resolve()


def canonical(value):
    return (json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(',', ':')) + '\n').encode()


def agent(identity='a1', parent=None):
    return dict(id=identity, parent_id=parent, name='Worker', runtime='OMP', provider='provider',
                model='model', role='worker', task='A public task', status='working', activity='A public action',
                activity_state='running', started_at='2026-10-25T23:59:59.900Z',
                updated_at=None, observed_at='2026-10-26T00:00:00.100Z')


def snapshot():
    return dict(schema_version=1, generated_at='2026-10-26T00:00:01.100Z', sequence='snapshot',
                poll_ms=1000, stale_after_ms=15000,
                coverage=dict(label='OMP', roots=1, agents=1, complete=False, notice='Partial observation'),
                agents=[agent()], interactions=[])


def run(raw, good=True):
    result = subprocess.run([BINARY], input=raw, capture_output=True, timeout=30)
    if good:
        assert result.returncode == 0, (result.returncode, result.stderr)
        assert result.stdout.startswith(b'<!doctype html>')
        assert result.stdout.endswith(b'</html>\n')
    else:
        assert result.returncode != 0, 'invalid input was accepted'
        assert result.stdout == b'', 'refusal leaked partial output'
    return result.stdout


class Document(HTMLParser):
    def __init__(self, value):
        super().__init__(convert_charrefs=True)
        self.ids, self.tags, self.text, self.attrs = [], [], [], []
        self.feed(value.decode())

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        self.tags.append(tag)
        self.attrs.append(attrs)
        if 'id' in attrs:
            self.ids.append(attrs['id'])
        assert not any(key.startswith('on') for key in attrs), attrs
        assert not any('javascript:' in value for value in attrs.values() if value), attrs

    def handle_data(self, data):
        self.text.append(data)


base = snapshot()
raw = canonical(base)
first = run(raw)
assert first == run(raw), 'render depends on wall clock or mutable state'
doc = Document(first)
assert len(doc.ids) == len(set(doc.ids)), 'duplicate DOM id'
assert '1s' in ''.join(doc.text)  # Subsecond timestamps must not round up.
assert 'script' not in doc.tags
assert next(a for a in doc.attrs if a.get('id') == 'hive-board')['data-generated'] == base['generated_at']

# Every byte is tested through the exact production escaper, without the JSON
# parser (which correctly refuses invalid UTF-8 and disallowed controls).
source = Path('render.om').read_text().split('fn main to int\n')[0]
escape_main = '''fn main to int
n = ulong 0
while lt n 16000
r = read 0 dec n sub ulong 16000 n
if lt r 0
ret 1
end
if eq r 0
break
end
n = add n ulong r
end
escape 0 n
r = write 1 out 0 get st 5
if ne r long get st 5
ret 2
end
ret 0
end
'''
probe = SCRATCH / 'escape'
compiled = subprocess.run([OMC], input=(source + escape_main).encode(), capture_output=True)
assert compiled.returncode == 0, compiled.stderr
probe.write_bytes(compiled.stdout)
probe.chmod(0o700)
for value in [bytes([i]) for i in range(256)] + [bytes(range(256)), b"&<>\"'" * 144, b"&<>\"'" * 3200]:
    escaped = subprocess.run([probe], input=value, capture_output=True, check=True).stdout
    expected = html.escape(value.decode('latin1'), quote=True).encode('latin1')
    assert escaped == expected, (value[:20], escaped[:100], expected[:100])
    if b'<' in value:
        assert escaped != expected.replace(b'&lt;', b'<'), 'negative escaper oracle did not discriminate'

probes = ['<img src=x onerror=alert(1)>', '</li><script>alert(1)</script>',
          '\"><a href="javascript:alert(1)">', '</details>']
for payload in probes + ['\b\f\n\r\t\\"', 'é 🐝 漢字', "&<>\"'" * 3200]:
    value = snapshot()
    value['agents'][0]['activity'] = payload
    value['interactions'] = [dict(id='m1', agent_id='a1', role='assistant', text=payload,
                                  at=value['generated_at'], runtime='OMP', provider='provider', model='model')]
    output = run(canonical(value))
    assert html.escape(payload).encode() in output
    parsed = Document(output)
    assert not {'img', 'script'} & set(parsed.tags)
    assert payload in parsed.text
    if payload in probes:
        (SCRATCH / ('xss-' + str(probes.index(payload)) + '.html')).write_bytes(output)

for field, size in [('name', 720), ('task', 16000), ('activity', 16000)]:
    value = snapshot()
    value['agents'][0][field] = 'x' * size
    run(canonical(value))
    value['agents'][0][field] += 'x'
    run(canonical(value), False)

bad = [raw[:-2], raw.replace(b'"Worker"', b'"unterminated'), b'x' * (2097152 + 1),
       raw.replace(b'"Worker"', b'"\\u0001"'), raw.replace(b'"Worker"', b'"\\q"'),
       raw.replace(b'"Worker"', b'"\\u0041"'), raw.replace(b'"Worker"', b'"\xc0\xbc"'),
       raw.replace(b'"agents":[', b'"agents":[],"agents":['),
       raw.replace(b'"coverage":{', b'"unknown":0,"coverage":{')]
for encoded in bad:
    run(encoded, False)
for mutation in ['bad-id', 'parent', 'cycle', 'count', 'date', 'enum', 'fraction', 'duplicate-id']:
    value = snapshot()
    if mutation == 'bad-id': value['agents'][0]['id'] = 'a" onclick="x'
    if mutation == 'parent': value['agents'][0]['parent_id'] = 'missing'
    if mutation == 'cycle':
        value['agents'][0]['parent_id'] = 'a1'
        value['coverage']['roots'] = 0
    if mutation == 'count': value['coverage']['agents'] = 2
    if mutation == 'date': value['generated_at'] = '2026-02-29T00:00:01Z'
    if mutation == 'enum': value['agents'][0]['status'] = 'invented'
    if mutation == 'fraction': value['generated_at'] = '2026-10-26T00:00:01.Z'
    if mutation == 'duplicate-id':
        value['agents'].append(agent())
        value['coverage'].update(agents=2, roots=2)
    run(canonical(value), False)

# Deep valid trees are traversed iteratively, not via the process stack.
value = snapshot()
value['agents'] = [agent('a' + str(i), 'a' + str(i - 1) if i else None) for i in range(500)]
value['coverage']['agents'] = 500
output = run(canonical(value))
assert output.count(b'<summary class="hive-record">') == 500
assert output.count(b'</ul></details></li>') == 500

# Receiver maximum byte budget with worst-case escaping can exceed one output
# pool. This fixture exercises both pools rather than silently reducing scope.
value = snapshot()
value['agents'] = [agent('a' + str(i)) for i in range(60)]
for item in value['agents']:
    item['activity'] = "'" * 16000
    item['task'] = "'" * 16000
value['coverage'].update(agents=60, roots=60)
encoded = canonical(value)
assert len(encoded) < 2097152
output = run(encoded)
assert len(output) > 16777216
Document(output)

# Date arithmetic independently compared with Python's datetime oracle.
for year in [1900, 2000, 2024, 2026]:
    value = snapshot()
    value['generated_at'] = f'{year}-03-01T00:00:00Z'
    value['agents'][0]['started_at'] = f'{year}-02-28T00:00:00Z'
    value['agents'][0]['observed_at'] = value['generated_at']
    expected = int((datetime.datetime(year, 3, 1) - datetime.datetime(year, 2, 28)).total_seconds())
    assert f'>{expected}s</span>'.encode() in run(canonical(value))

(SCRATCH / 'fixture.json').write_bytes(raw)
(SCRATCH / 'fixture.html').write_bytes(first)
print('renderer oracle checks passed (byte escaper, Unicode, XSS, limits, negative controls, dates, deep tree, two output pools)')
