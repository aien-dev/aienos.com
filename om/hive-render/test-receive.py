#!/usr/bin/env python3
"""Exercise actual forced-command receiver and atomic publication failure paths."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

receiver, renderer = (Path(p).resolve() for p in sys.argv[1:])
value = dict(schema_version=1, generated_at='2026-10-26T01:00:00Z', sequence='integration',
             poll_ms=1000, stale_after_ms=15000, agents=[], interactions=[],
             coverage=dict(label='OMP', roots=0, agents=0, complete=False, notice='Integration oracle'))
raw = (json.dumps(value) + '\n').encode()
with tempfile.TemporaryDirectory(prefix='hive-receiver-integration-') as tmp:
    directory = Path(tmp)
    board = directory / 'board.html'
    fail = directory / 'fail-renderer'
    fail.write_text('#!/bin/sh\nprintf incomplete\nexit 7\n')
    fail.chmod(0o700)
    env = dict(os.environ, SSH_ORIGINAL_COMMAND='hive-feed', HIVE_RENDER=str(fail))
    board.write_bytes(b'old useful board')
    result = subprocess.run([receiver, directory], input=raw, env=env, capture_output=True, check=True)
    assert board.read_bytes() == b'old useful board'
    assert b'retained previous observation' in result.stderr
    assert json.loads((directory / 'live.json').read_bytes()) == value
    assert not list(directory.glob('.board-tmp-*'))
    env['HIVE_RENDER'] = str(renderer)
    result = subprocess.run([receiver, directory], input=raw, env=env, capture_output=True, check=True)
    assert result.stderr == b''
    expected = subprocess.run([renderer], input=(directory / 'live.json').read_bytes(), capture_output=True, check=True).stdout
    assert board.read_bytes() == expected
    before = board.read_bytes()
    bad = dict(value, schema_version=2)
    result = subprocess.run([receiver, directory], input=(json.dumps(bad)+'\n').encode(), env=env, capture_output=True)
    assert result.returncode != 0
    assert board.read_bytes() == before
    assert json.loads((directory / 'live.json').read_bytes()) == value
print('receiver integration passed: failed renderer logs/retains, successful renderer publishes exact bytes, rejected feed retains both artifacts')
