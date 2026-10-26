#!/usr/bin/env python3
"""Tests for POST /allen/api/transcribe (push-to-talk), each guard with a control.

Starts throwaway gateways on loopback (never the production unit, never the
model: chat is not called). Fixtures are cut from whisper.cpp's samples/jfk.wav
with ffmpeg (a verification tool here). Under strace, the gateway and its
sandboxes must create no file while transcribing.

Usage: test_transcribe.py --omc P --irwasm P --system-prompt P --whisper-cli P --whisper-model P
       --sample jfk.wav [--gw-dir DIR] [--latency-runs N]
"""

import argparse
import http.client
import json
import os
import re
import shutil
import socket
import statistics
import struct
import subprocess
import sys
import tempfile
import threading
import time

HERE = os.path.dirname(os.path.abspath(__file__))
results = []


def check(name, ok, detail):
    results.append((name, ok))
    print("%s %s: %s" % ("PASS" if ok else "FAIL", name, detail), flush=True)


def free_port():
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    p = s.getsockname()[1]
    s.close()
    return p


def ff(*args, data=None):
    return subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y"] + list(args),
                          input=data, stdout=subprocess.PIPE, check=True).stdout


class Gateway:
    def __init__(self, a, *extra, strace=None):
        self.port = free_port()
        self.tmp = tempfile.mkdtemp(prefix="transcribe-test-")
        argv = [sys.executable, "-B", os.path.join(a.gw_dir, "allen_chat.py"), "--listen", "127.0.0.1:%d" % self.port,
                "--upstream", "127.0.0.1:9", "--system-prompt", a.system_prompt, "--omc", a.omc,
                "--irwasm", a.irwasm, "--trusted-proxy", "127.0.0.1", "--warm-every", "0"] + list(extra)
        if strace:
            argv = ["strace", "-f", "-qq", "-e", "trace=openat,open,creat,mknodat,linkat,renameat2,memfd_create",
                    "-o", strace] + argv
        self.p = subprocess.Popen(argv, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, cwd=self.tmp)
        self.first = self.p.stdout.readline().decode()

    def close(self):
        # Under strace the gateway is strace's child: stop it, and strace exits after it.
        try:
            with open("/proc/%d/task/%d/children" % (self.p.pid, self.p.pid)) as f:
                kids = [int(x) for x in f.read().split()]
        except OSError:
            kids = []
        for k in kids or [self.p.pid]:
            os.kill(k, 15)
        self.p.wait(20)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def presence(self):
        c = http.client.HTTPConnection("127.0.0.1", self.port, timeout=10)
        c.request("GET", "/allen/api/presence", headers={"X-Real-IP": "203.0.113.1"})
        r = c.getresponse()
        obj = json.loads(r.read())
        c.close()
        return obj, r.getheader("Cache-Control")

    def post(self, body, ctype, ip="203.0.113.1", timeout=60, length=None):
        c = http.client.HTTPConnection("127.0.0.1", self.port, timeout=timeout)
        c.putrequest("POST", "/allen/api/transcribe")
        c.putheader("Content-Type", ctype)
        c.putheader("X-Real-IP", ip)
        c.putheader("Content-Length", str(len(body) if length is None else length))
        c.endheaders()
        if length is None:
            c.send(body)
        r = c.getresponse()
        data = r.read()
        c.close()
        try:
            obj = json.loads(data)
        except ValueError:
            obj = {"raw": data[:200]}
        return r.status, obj, r


def wav(pcm, rate=16000, ch=1, bits=16, tag=1):
    blk = ch * bits // 8
    return (b"RIFF" + struct.pack("<I", 36 + len(pcm)) + b"WAVEfmt " +
            struct.pack("<IHHIIHH", 16, tag, ch, rate, rate * blk, blk, bits) + b"data" + struct.pack("<I", len(pcm)) + pcm)


def main():
    ap = argparse.ArgumentParser()
    for k in ("omc", "irwasm", "system-prompt", "whisper-cli", "whisper-model", "sample"):
        ap.add_argument("--" + k, required=True)
    ap.add_argument("--gw-dir", default=HERE)
    ap.add_argument("--threads", default="4")
    ap.add_argument("--latency-runs", type=int, default=12)
    a = ap.parse_args()

    clip5_wav = ff("-i", a.sample, "-t", "5", "-ar", "16000", "-ac", "1", "-f", "wav", "-")
    clip5_webm = ff("-i", "-", "-c:a", "libopus", "-b:a", "32k", "-f", "webm", "-", data=clip5_wav)
    clip5_ogg = ff("-i", "-", "-c:a", "libopus", "-b:a", "32k", "-f", "ogg", "-", data=clip5_wav)
    clip5_44st = ff("-i", a.sample, "-t", "5", "-ar", "44100", "-ac", "2", "-f", "wav", "-")
    long_pcm = ff("-i", a.sample, "-i", a.sample, "-filter_complex", "concat=n=2:v=0:a=1", "-t", "20",
                  "-ar", "16000", "-ac", "1", "-f", "s16le", "-")
    long_webm = ff("-f", "s16le", "-ar", "16000", "-ac", "1", "-i", "-", "-c:a", "libopus", "-b:a", "16k",
                   "-f", "webm", "-", data=long_pcm)
    long_wav8k = ff("-f", "s16le", "-ar", "16000", "-ac", "1", "-i", "-", "-ar", "8000", "-f", "wav", "-",
                    data=long_pcm)
    vorbis = ff("-i", "-", "-c:a", "libvorbis", "-f", "webm", "-", data=clip5_wav)
    clip15 = wav(long_pcm[:15 * 32000])  # exactly the limit: allowed
    words = re.compile(r"fellow\s+americans", re.I)
    flags = ["--whisper-cli", a.whisper_cli, "--whisper-model", a.whisper_model, "--transcribe-threads", a.threads]

    # Disabled unless configured (control: the configured gateway below answers 200).
    g = Gateway(a)
    try:
        st, obj, _ = g.post(clip5_wav, "audio/wav")
        check("off_without_whisper", st == 404, "%s %s" % (st, obj))
        obj, cc = g.presence()
        check("presence_transcribe_false", obj.get("transcribe") is False and cc == "no-store", "%s %s" % (obj, cc))
    finally:
        g.close()

    # Pins: an unpinned model refuses startup (control: the pinned model starts below).
    fake = tempfile.NamedTemporaryFile(suffix=".bin", delete=False)
    fake.write(b"\0" * 4096)
    fake.close()
    p = subprocess.run([sys.executable, os.path.join(a.gw_dir, "allen_chat.py"), "--listen", "127.0.0.1:%d" % free_port(),
                        "--system-prompt", a.system_prompt, "--omc", a.omc, "--irwasm", a.irwasm, "--warm-every", "0",
                        "--whisper-cli", a.whisper_cli, "--whisper-model", fake.name],
                       capture_output=True, timeout=60)
    os.unlink(fake.name)
    check("unpinned_model_refused", p.returncode != 0 and b"not a pinned whisper model" in p.stderr,
          "rc %s %r" % (p.returncode, p.stderr[-120:]))

    trace = tempfile.mktemp(prefix="transcribe-strace-")
    g = Gateway(a, *(flags + ["--chat-burst", "40", "--chat-refill", "1"]), strace=trace)
    try:
        check("started", "listening" in g.first, g.first.strip())
        obj, cc = g.presence()
        check("presence_transcribe_true", obj.get("transcribe") is True and cc == "no-store", "%s %s" % (obj, cc))
        for name, body, ctype in (("webm_opus", clip5_webm, "audio/webm;codecs=opus"),
                                  ("ogg_opus", clip5_ogg, "audio/ogg; codecs=opus"),
                                  ("wav_16k_mono", clip5_wav, "audio/wav"),
                                  ("wav_44k_stereo", clip5_44st, "audio/wave")):
            st, obj, _ = g.post(body, ctype, ip="198.51.100.%d" % len(name))
            check("ok_" + name, st == 200 and bool(words.search(obj.get("text", ""))) and obj.get("seconds") == 5.0,
                  "%s %s" % (st, obj))
        # Over-length audio is refused, never truncated (control: the 5 s clips above pass).
        st, obj, _ = g.post(long_webm, "audio/webm", ip="198.51.100.20")
        check("too_long_webm_20s", st == 413 and obj.get("error") == "too_long", "%d bytes -> %s %s" % (len(long_webm), st, obj))
        st, obj, _ = g.post(long_wav8k, "audio/wav", ip="198.51.100.21")
        check("too_long_wav_20s", st == 413 and obj.get("error") == "too_long", "%d bytes -> %s %s" % (len(long_wav8k), st, obj))
        # Oversize body: refused from Content-Length before any byte is read; connection closed.
        st, obj, r = g.post(b"", "audio/webm", ip="198.51.100.22", length=(1 << 20) + 1)
        check("oversize_body", st == 413 and r.getheader("Connection") == "close", "%s %s" % (st, obj))
        st, obj, _ = g.post(b"", "audio/webm", ip="198.51.100.23")
        check("empty_body", st == 400, "%s %s" % (st, obj))
        # Bad formats.
        st, obj, _ = g.post(clip5_wav, "text/plain", ip="198.51.100.24")
        check("bad_content_type", st == 415 and obj.get("error") == "format", "%s %s" % (st, obj))
        st, obj, _ = g.post(clip5_wav, "audio/webm", ip="198.51.100.25")
        check("mislabelled_wav_as_webm", st == 415, "%s %s" % (st, obj))
        st, obj, _ = g.post(b"\x1a\x45\xdf\xa3" + os.urandom(4000), "audio/webm", ip="198.51.100.26")
        check("garbage_webm", st == 422 and obj.get("error") == "audio", "%s %s" % (st, obj))
        st, obj, _ = g.post(vorbis, "audio/webm", ip="198.51.100.27")
        check("vorbis_not_opus", st == 422, "%s %s" % (st, obj))
        st, obj, _ = g.post(wav(b"\x80" * 16000, bits=8), "audio/wav", ip="198.51.100.28")
        check("wav_8bit", st == 422 and "16-bit" in obj.get("message", ""), "%s %s" % (st, obj))
        st, obj, _ = g.post(wav(b"\0" * 3200)[:40], "audio/wav", ip="198.51.100.29")
        check("wav_truncated_header", st == 422, "%s %s" % (st, obj))
        st, obj, _ = g.post(wav(b"\0" * 3200), "audio/wav", ip="198.51.100.30")
        check("too_short_0_1s", st == 422 and "short" in obj.get("message", ""), "%s %s" % (st, obj))
        # Busy: one transcription at a time; a second concurrent clip gets 503 (control: both alone pass).
        out = []
        ts = [threading.Thread(target=lambda i=i: out.append(g.post(clip15, "audio/wav", ip="192.0.2.%d" % i)[0]))
              for i in range(3)]
        for t in ts:
            t.start()
        for t in ts:
            t.join()
        check("cap_busy_503", sorted(out)[0] == 200 and 503 in out, "statuses %s" % sorted(out))
    finally:
        g.close()
    # No audio file: every file the gateway and its sandboxes created or opened for writing.
    # Allowed: /dev/null, the bwrap user-namespace maps (procfs), and the bind-mount targets bwrap
    # creates on the sandbox's own fresh tmpfs root (/newroot/..., private mount namespace, gone
    # at exit). Control: the trace must show whisper-cli opening /model inside the sandbox.
    lines = open(trace).read().splitlines()
    writes = [ln for ln in lines if re.search(r"O_WRONLY|O_RDWR|O_CREAT|creat\(|mknodat|linkat|renameat2|memfd_create", ln)
              and "= -1" not in ln]
    writes_bad = [ln for ln in writes if not re.search(r'"/dev/(null|tty)"|"(uid_map|gid_map|setgroups)"|"/newroot/', ln)]
    followed = sum(1 for ln in lines if '"/model"' in ln and "= -1" not in ln)
    check("no_file_written", followed > 0 and not writes_bad, "%d sandboxed /model opens traced, %d write-opens, unexpected: %s"
          % (followed, len(writes), [w.strip()[:160] for w in writes_bad[:5]]))
    os.unlink(trace)

    # Rate limit: the chat route's bucket (burst 2 here); another address passes (control).
    g = Gateway(a, *(flags + ["--chat-burst", "2", "--chat-refill", "600"]))
    try:
        sts = [g.post(clip5_webm, "audio/webm", ip="203.0.113.9")[0] for _ in range(2)]
        st, obj, r = g.post(clip5_webm, "audio/webm", ip="203.0.113.9")
        check("rate_limited", sts == [200, 200] and st == 429 and r.getheader("Retry-After") is not None,
              "%s then %s %s" % (sts, st, obj))
        st2, _, _ = g.post(clip5_webm, "audio/webm", ip="203.0.113.10")
        check("rate_other_address", st2 == 200, str(st2))
    finally:
        g.close()

    # Latency through the gateway (HTTP on loopback, webm/opus 5 s), sequential runs.
    g = Gateway(a, *(flags + ["--chat-burst", "1000", "--chat-refill", "0.01"]))
    try:
        ms, server_ms = [], []
        for i in range(a.latency_runs):
            t0 = time.monotonic()
            st, obj, _ = g.post(clip5_webm, "audio/webm", ip="203.0.113.50")
            ms.append((time.monotonic() - t0) * 1000)
            server_ms.append(obj.get("ms", -1))
        q = statistics.quantiles(ms, n=20, method="inclusive")
        print("LATENCY model=%s threads=%s n=%d p50=%.0f p95=%.0f min=%.0f max=%.0f (gateway-measured p50=%.0f)" % (
            os.path.basename(a.whisper_model), a.threads, len(ms), statistics.median(ms), q[18], min(ms), max(ms),
            statistics.median(server_ms)), flush=True)
    finally:
        g.close()

    bad = [n for n, ok in results if not ok]
    print("%d/%d passed%s" % (len(results) - len(bad), len(results), (": failed " + ", ".join(bad)) if bad else ""))
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
