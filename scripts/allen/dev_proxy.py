#!/usr/bin/env python3
"""Development only: serve a built site and pass /allen/api/ to a local gateway,
so the page and the API share an origin in a local browser test. Never deployed;
production routing is hub nginx (nginx-allen-location.conf)."""

import argparse
import http.client
import os
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlsplit

TYPES = {".html": "text/html; charset=utf-8", ".js": "text/javascript", ".css": "text/css",
         ".svg": "image/svg+xml", ".wasm": "application/wasm", ".json": "application/json",
         ".png": "image/png", ".ico": "image/x-icon", ".txt": "text/plain"}
# Upstream headers the page reads or the browser needs; nothing else is forwarded.
FORWARD = ("Content-Type", "Cache-Control", "Retry-After", "X-Allen-Source", "X-OH-Model",
           "X-OH-Volunteer", "X-Content-Type-Options", "Content-Security-Policy")


def index(dist):
    """URL path -> file for every regular file under dist, built once at start, so a
    request path is only ever a dictionary key and never reaches the filesystem."""
    root = os.path.realpath(dist)
    files = {}
    for d, _, names in os.walk(root):
        for n in names:
            full = os.path.join(d, n)
            if os.path.isfile(full):
                url = "/" + os.path.relpath(full, root).replace(os.sep, "/")
                files[url] = full
                if n == "index.html":
                    files[url[:-len("index.html")]] = full
                    files[url[:-len("/index.html")] or "/"] = full
    return files


class H(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *a):
        pass

    def proxy(self):
        n = int(self.headers.get("Content-Length", "0") or 0)
        body = self.rfile.read(n) if n else None
        host, _, port = self.server.gateway.rpartition(":")
        c = http.client.HTTPConnection(host, int(port), timeout=200)
        hdrs = {"Content-Type": self.headers.get("Content-Type", "application/json"),
                "X-Real-IP": self.headers.get("X-Real-IP", self.client_address[0])}
        c.request(self.command, self.path, body, hdrs)
        r = c.getresponse()
        self.send_response(r.status)
        for k in FORWARD:
            v = r.getheader(k)
            # A value carrying CR or LF would split this response; drop it.
            if v is not None and "\r" not in v and "\n" not in v:
                self.send_header(k, v.replace("\r", "").replace("\n", ""))
        self.send_header("Connection", "close")
        self.end_headers()
        self.close_connection = True
        while True:
            chunk = r.read1(65536)
            if not chunk:
                break
            self.wfile.write(chunk)
            self.wfile.flush()

    def static(self):
        full = self.server.files.get(urlsplit(self.path).path)
        if full is None:
            self.send_error(404)
            return
        with open(full, "rb") as f:
            data = f.read()
        self.send_response(200)
        self.send_header("Content-Type", TYPES.get(os.path.splitext(full)[1], "application/octet-stream"))
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        return self.proxy() if self.path.startswith("/allen/api/") else self.static()

    def do_POST(self):
        return self.proxy() if self.path.startswith("/allen/api/") else self.send_error(405)


ap = argparse.ArgumentParser()
ap.add_argument("--dist", required=True)
ap.add_argument("--listen", default="127.0.0.1:18112")
ap.add_argument("--gateway", default="127.0.0.1:18111")
a = ap.parse_args()
h, _, p = a.listen.rpartition(":")
srv = ThreadingHTTPServer((h, int(p)), H)
srv.daemon_threads = True
srv.files, srv.gateway = index(a.dist), a.gateway
print("dev proxy on %s -> %s" % (a.listen, a.gateway), flush=True)
srv.serve_forever()
