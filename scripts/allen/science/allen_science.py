#!/usr/bin/env python3
"""Loopback-only stateless compute service. No HTTP client, credentials or project store."""
import argparse
import json
import os
from pathlib import Path
import re
import secrets
import socket
import socketserver
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer

# Isolated interpreter (-I) intentionally excludes cwd; only the root-owned install dir is imported.
import sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from science_protocol import MAX_BYTES, TOOLS, event, packed, validate
from science_protocol import content as validate_content
from science_sandbox import AIEN_MATH_PIN, run_math, run_omega, selftest
ENGINE = 'aien-math'
SOURCE = ''  # Source publication and distribution license await the copyright holder.
MANIFEST = Path(__file__).with_name('engine.json')


def load_manifest():
    manifest = json.loads(MANIFEST.read_text())
    if type(manifest) is not dict or not re.fullmatch(r'[0-9a-f]{40}', manifest.get('commit','')):
        raise ValueError('Missing engine source identity')
    if manifest.get('binary_sha256') != AIEN_MATH_PIN:
        raise ValueError('Engine provenance does not match enforced binary pin')
    return manifest


def evidence(status, expression='', result=''):
    # No default guessed source pin or corpus score.
    manifest = load_manifest()
    label = 'math engine: Drake’s own, source not yet published'
    corpus, alpha, guard = manifest.get('corpus'), manifest.get('alpha'), manifest.get('guard')
    if corpus and alpha and guard:
        label += (f". aien-math {manifest['commit']}: full corpus {corpus['pass']}/{corpus['total']} pass, "
                  f"{corpus['intentional_differences']} intentional differences, {corpus['failures']} failures, exit {corpus['exit']}; "
                  f"Alpha {alpha['pass']}/{alpha['total']}, exit {alpha['exit']}; "
                  f"guard tests {guard['tests']} {guard['result']}, exit {guard['exit']}. Experimental, not independent verification.")
    return {'status': status, 'engine': label if expression else '', 'expression': expression,
            'result': result, 'source': SOURCE if expression else '', 'commit': manifest['commit'] if expression else ''}


def perform(name, arguments):
    args = validate(name, arguments)
    pane_id = 'p_' + secrets.token_hex(12)
    kind = {'math_compute':'math','omega_compile':'omega','web_app':'app','paper':'doc','research':'research','plot':'plot'}[name]
    provenance = {'status':'unverified','engine':'','expression':'','result':'','source':'','commit':''}
    result = 'Created a draft pane. Its claims are unverified; do not describe them as computed.'
    if name == 'math_compute':
        outcome = run_math(args['expression'])
        raw = outcome.get('output', '')
        # A zero exit can still be an unsupported symbolic call. Require a nonempty
        answer = raw.strip()
        ok = outcome.get('ok') is True and bool(answer)
        provenance = evidence('computed' if ok else 'unsupported', args['expression'], raw)
        content = {'text': answer if ok else 'Not supported yet. aien-math did not produce a supported result for this expression.'}
        result = ('aien-math exact output (not a proof of correctness): ' + answer) if ok else 'NOT SUPPORTED. Do not invent an answer or substitute model arithmetic.'
        if len(result) > 8000:
            result = 'The complete aien-math output is in the pane. It exceeds the model context limit; do not infer omitted values or claim to have read the full result.'
    elif name == 'omega_compile':
        outcome = run_omega(args['source'])
        content = {'source':args['source'],'language':'omega'}
        if outcome.get('ok') is True:
            content.update({k:outcome[k] for k in ('wasm','functions') if k in outcome})
            result = 'Omega compiled to browser WASM. It has not been run. The visitor chooses whether to run it.'
            provenance['result'] = packed({'omc_sha256':outcome['omc_sha256'],
                                           'irwasm_sha256':outcome['irwasm_sha256'],
                                           'stage':'compiled','executed':False})
        else:
            result = 'Omega compilation refused: ' + str(outcome.get('error', outcome.get('message','unsupported source')))[:2000]
            provenance['status'] = 'unsupported'
        provenance['engine'] = 'seeded-omc'
        provenance['expression'] = args['source']
        provenance['source'] = 'https://github.com/aien-dev/omega-mv'
    elif name == 'web_app':
        content = {'html':args['html']}
        result = 'App draft created. It has not run; only a visitor click may run it in an allow-scripts-only iframe with no network.'
    elif name == 'paper': content = {'markdown':args['markdown'],'citations':args['citations']}
    elif name == 'research':
        content = {key:args[key] for key in ('notes','hypotheses','derivations','sources')}
        result = 'Research draft created from supplied notes. No live web search was performed; sources and model claims are unverified.'
    else:
        content = {key:args[key] for key in ('series','x_label','y_label')}
        provenance['status'] = 'input-data'
        provenance['expression'] = packed(args['series'])
        result = 'Plot created from supplied data, not from a new calculation. Data provenance is input-data, not verified science.'
    validate_content(kind, content)
    events = [{'event':'pane.open','data':{'v':1,'id':pane_id,'kind':kind,'title':args['title']}},
              {'event':'pane.update','data':{'v':1,'id':pane_id,'revision':1,'content':content,'provenance':provenance}}]
    for item in events:
        event(item['event'], item['data'])
        if len(packed(item['data']).encode()) > MAX_BYTES:
            raise ValueError('Pane result exceeds output limit')
    return {'events':events,'result':result}


class Handler(BaseHTTPRequestHandler):
    server_version = 'AllenScience/1'
    protocol_version = 'HTTP/1.0'
    def log_message(self, *args):
        pass  # Never persist expressions, drafts, IPs or visitor information.
    def setup(self):
        super().setup()
        self.connection.settimeout(20)
    def reply(self, status, body):
        data = packed(body).encode()
        self.send_response(status)
        self.send_header('Content-Type','application/json')
        self.send_header('Content-Length',str(len(data)))
        self.send_header('Cache-Control','no-store')
        self.end_headers()
        self.wfile.write(data)
    def do_POST(self):
        if self.path != '/compute' or self.client_address[0] != '127.0.0.1':
            self.reply(404, {'error':'not_found'}); return
        if self.headers.get('Transfer-Encoding') or self.headers.get('Content-Type','').split(';')[0] != 'application/json':
            self.reply(400, {'error':'invalid_framing'}); return
        try:
            lengths = self.headers.get_all('Content-Length', [])
            if len(lengths) != 1: raise ValueError('Invalid content length')
            size = int(lengths[0])
            if not 0 < size <= MAX_BYTES: raise ValueError('Body too large')
            raw = self.rfile.read(size)
            if len(raw) != size: raise ValueError('Truncated request')
            from science_protocol import parse_call
            name, args = parse_call(raw.decode('utf-8'))
            if name not in self.server.tools:
                self.reply(403, {'error':'tool_not_allowed'}); return
            output = perform(name,args)
        except (ValueError, UnicodeError, json.JSONDecodeError):
            self.reply(422, {'error':'invalid_compute_request'}); return
        except Exception:
            self.reply(503, {'error':'compute_unavailable','message':'Not supported yet. Please try a smaller expression.'}); return
        self.reply(200, output)


class Server(socketserver.ThreadingMixIn, HTTPServer):
    daemon_threads = True
    allow_reuse_address = True
    slots = threading.BoundedSemaphore(2)
    tools = frozenset(TOOLS)
    def process_request(self, request, client_address):
        if not self.slots.acquire(False):
            request.sendall(b'HTTP/1.0 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n')
            self.shutdown_request(request)
            return
        try: super().process_request(request,client_address)
        except BaseException:
            self.slots.release(); raise
    def process_request_thread(self, request, client_address):
        try: super().process_request_thread(request,client_address)
        finally: self.slots.release()

def allowed_tools(text):
    """The --tools list: a non-empty, duplicate-free subset of TOOLS. A public gateway gets an
    instance that serves only what it calls, so its reach is the tool, not the whole service."""
    names = text.split(',')
    if not text or len(set(names)) != len(names) or not set(names) <= set(TOOLS):
        raise argparse.ArgumentTypeError('--tools takes a comma list of ' + ','.join(TOOLS))
    return frozenset(names)

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--listen-fd', type=int)
    parser.add_argument('--port', type=int, default=18113, choices=[18113, 18114])
    parser.add_argument('--tools', type=allowed_tools, default=frozenset(TOOLS))
    options = parser.parse_args()
    load_manifest()
    problems = selftest()
    if problems:
        raise SystemExit('Science sandbox self-test refused startup: ' + '; '.join(problems))
    if options.listen_fd is None:
        raise SystemExit('Science service requires the isolated inherited-socket launcher')
    server = Server(('127.0.0.1',options.port),Handler,bind_and_activate=False)
    server.tools = options.tools
    server.socket.close()
    server.socket = socket.socket(fileno=options.listen_fd)
    server.server_address = server.socket.getsockname()
    if server.server_address != ('127.0.0.1',options.port):
        raise SystemExit('Unexpected inherited listener')
    server.serve_forever()
