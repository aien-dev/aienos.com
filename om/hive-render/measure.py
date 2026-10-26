#!/usr/bin/env python3
"""Read-only host measurement; canonicalization mirrors receiver serde_json.
Python, gzip and time are independent measurement tools, not serving dependencies.
"""
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time

binary, snapshot, destination = map(Path, sys.argv[1:4])
binary = binary.resolve()
value = json.loads(snapshot.read_bytes())
raw = (json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False) + '\n').encode()
start = time.perf_counter_ns()
first = subprocess.run([binary], input=raw, capture_output=True, check=True).stdout
elapsed = time.perf_counter_ns() - start
second = subprocess.run([binary], input=raw, capture_output=True, check=True).stdout
assert first == second
report = dict(generated_at=value['generated_at'], agents=len(value['agents']), interactions=len(value['interactions']),
              input_bytes=len(raw), board_bytes=len(first), board_gzip_bytes=len(gzip.compress(first, mtime=0)),
              spark_render_ns=elapsed, board_sha256=hashlib.sha256(first).hexdigest(),
              renderer_sha256=hashlib.sha256(binary.read_bytes()).hexdigest())
# Only a per-run scratch directory under /tmp is created on hub. No service,
# configuration or web-root files are read or changed by this measurement.
remote = subprocess.check_output(['ssh', 'hub', 'mktemp -d /tmp/hive-render-measure.XXXXXXXX'], text=True).strip()
try:
    subprocess.run(['scp', '-q', str(binary), 'hub:' + remote + '/hive-render'], check=True)
    result = subprocess.run(['ssh', 'hub', 'python3 -c ' + __import__('shlex').quote('''import hashlib,json,subprocess,sys,time
raw=sys.stdin.buffer.read()
start=time.perf_counter_ns()
out=subprocess.run([sys.argv[1]],input=raw,capture_output=True,check=True).stdout
elapsed=time.perf_counter_ns()-start
print(json.dumps(dict(hub_render_ns=elapsed,hub_board_bytes=len(out),hub_board_sha256=hashlib.sha256(out).hexdigest())))
''') + ' ' + remote + '/hive-render'], input=raw, capture_output=True, check=True)
    edge = json.loads(result.stdout)
    assert edge['hub_board_sha256'] == report['board_sha256']
    report.update(edge)
finally:
    subprocess.run(['ssh', 'hub', 'rm -f ' + remote + '/hive-render && rmdir ' + remote], check=True)
destination.write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report, indent=2))
