#!/usr/bin/env python3
"""Independent HTTP oracle for native Omega services; not a production dependency."""
import argparse
import contextlib
import datetime
import http.client
import json
import pathlib
import subprocess
import tempfile
import time

parser = argparse.ArgumentParser()
parser.add_argument("core")
parser.add_argument("edge")
parser.add_argument("--only", choices=("capacity", "quota"))
parser.add_argument("--expiry", action="store_true", help="also wait through an actual 600-second expiry")
args = parser.parse_args()


@contextlib.contextmanager
def service(binary, *argv):
    proc = subprocess.Popen([binary, *map(str, argv)], stdout=subprocess.PIPE, text=True)
    try:
        line = proc.stdout.readline().split()
        assert len(line) == 2 and line[0] in ("listening", "PORT"), line
        yield int(line[1])
    finally:
        proc.terminate()
        proc.wait(timeout=10)


def request(port, path, data=None, headers=(), conn=None):
    own = conn is None
    conn = conn or http.client.HTTPConnection("127.0.0.1", port, timeout=10)
    try:
        conn.putrequest("POST" if data is not None else "GET", path)
        for key, value in headers:
            conn.putheader(key, value)
        if data is not None:
            conn.putheader("Content-Length", str(len(data)))
        conn.endheaders(data)
        response = conn.getresponse()
        return response.status, response.read()
    finally:
        if own:
            conn.close()


def challenge(port):
    code, body = request(port, "/agora/api/challenge")
    assert code == 200, (code, body)
    fields = body.split(b"challenge\t")[1].strip().split(b"\t")
    assert fields[1] == b"0", fields
    return fields[0], fields[2]


def hello(port, token):
    return request(port, "/agora/api/hello", b"challenge\t" + token + b"\nnonce\t0\nname\tCapacity\nkind\thuman\n")


def capacity(root):
    root.mkdir()
    with service(args.core, "serve", "--state", root, "--port", 0, "--bits", 0) as port:
        tokens = [challenge(port) for _ in range(1024)]
        for _ in range(3):
            assert request(port, "/agora/api/challenge") == (429, b"agora 1\nerr slow challenge capacity reached\n")
        assert datetime.datetime.now(datetime.timezone.utc) < datetime.datetime.fromisoformat(tokens[0][1].decode().replace("Z", "+00:00"))
        assert hello(port, tokens[0][0])[0] == 200, "live oldest challenge was evicted"
        assert hello(port, tokens[0][0])[0] == 403, "used challenge accepted twice"
        replacement, _ = challenge(port)
        assert replacement not in [token for token, _ in tokens]
        assert hello(port, tokens[-1][0])[0] == 200, "live last challenge was evicted"
        assert hello(port, replacement)[0] == 200
        print("limits: PASS full 1024-slot table refuses issuance; oldest/latest live tokens survive; used slots recycle; single use", flush=True)
        if args.expiry:
            # Seal both used slots with fresh, unconsumed tokens. A later successful issuance must
            # reclaim expired unused capacity, not accidentally recycle an already-used slot.
            challenge(port)
            challenge(port)
            assert request(port, "/agora/api/challenge") == (429, b"agora 1\nerr slow challenge capacity reached\n")
            print("limits: waiting 601 seconds for actual expiry (no clock injection)", flush=True)
            time.sleep(601)
            assert hello(port, tokens[1][0]) == (403, b"agora 1\nerr work challenge unknown expired or used\n")
            fresh, _ = challenge(port)
            assert hello(port, fresh)[0] == 200
            print("limits: PASS actual expiry rejects retained token and reclaims expired full-table slots", flush=True)


def rpc(port, protocol, fields, headers, conn=None):
    if protocol == "mcp":
        value = {"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "agora_hello", "arguments": fields}}
    else:
        value = {"jsonrpc": "2.0", "id": 1, "method": "message/send", "params": {"message": {"role": "user", "messageId": "quota", "parts": [], "metadata": {"skill": "agora_hello", **fields}}}}
    code, body = request(port, "/agora/" + protocol, json.dumps(value).encode(), headers, conn)
    assert code == 200, (code, body)
    result = json.loads(body)["result"]
    text = result["content" if protocol == "mcp" else "parts"][0]["text"]
    return text


def edgehello(core, edge, protocol, headers=(), conn=None):
    # A real bits=0 core isolates quota behavior; the full gate separately tests nonzero PoW.
    token, _ = challenge(core)
    return rpc(edge, protocol, {"kind": "human", "name": "Quota", "challenge": token.decode(), "nonce": 0}, headers, conn)


def quota(root):
    (root / "node").mkdir()
    with service(args.core, "serve", "--state", root / "node", "--port", 0, "--bits", 0) as core:
        # Existing core ingress trusts the operator proxy. These are oracle-supplied proxy identities,
        # not evidence that untrusted direct-core access is safe.
        for ip in ("192.0.2.1", "2001:db8::2"):
            for _ in range(20):
                token, _ = challenge(core)
                data = b"challenge\t" + token + b"\nnonce\t0\nname\tCoreQuota\nkind\thuman\n"
                assert request(core, "/agora/api/hello", data, (("X-Real-IP", ip),))[0] == 200
            token, _ = challenge(core)
            data = b"challenge\t" + token + b"\nnonce\t0\nname\tCoreQuota\nkind\thuman\n"
            assert request(core, "/agora/api/hello", data, (("X-Real-IP", ip),)) == (429, b"agora 1\nerr slow ip rate limit reached\n")
        print("limits: PASS core independent proxy-client IP quotas at 20/21 (existing restricted ingress contract)", flush=True)
        with service(args.edge, "--api", core, "--port", 0) as edge:
            for _ in range(20):
                assert edgehello(core, edge, "mcp").startswith("agora 1\nok\n")
            # The unresolved edge pooling must stay fail-closed: changing forwarding claims must not
            # buy another quota. Exercise both protocols, duplicate headers and keep-alive requests.
            claims = [(), (("X-Real-IP", "192.0.2.9"),), (("X-Forwarded-For", "192.0.2.9"),), (("X-Real-IP", "2001:db8::9"), ("X-Forwarded-For", "192.0.2.8")), (("X-Real-IP", "192.0.2.9"), ("x-real-ip", "192.0.2.8"))]
            conn = http.client.HTTPConnection("127.0.0.1", edge, timeout=10)
            try:
                for headers in claims:
                    for protocol in ("mcp", "a2a"):
                        text = edgehello(core, edge, protocol, headers, conn)
                        assert "err slow ip rate limit reached" in text, text
            finally:
                conn.close()
            print("limits: PASS MCP/A2A shared local quota retained; spoofed forwarded headers cannot bypass exhausted quota", flush=True)
            print("limits: NOT_IMPLEMENTED independent edge client quotas (AG-2 accepted-peer interface prerequisite)", flush=True)


with tempfile.TemporaryDirectory(prefix="agora-limits-") as scratch:
    root = pathlib.Path(scratch)
    if args.only != "quota":
        capacity(root / "capacity")
    if args.only != "capacity":
        quota(root)
print("limits: PASS", flush=True)
