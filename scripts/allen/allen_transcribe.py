"""Push-to-talk transcription for public Allen: POST /allen/api/transcribe.

The page records a short clip and posts the raw bytes (no JSON, no base64) with
Content-Type audio/webm, audio/ogg (Opus) or audio/wav. The reply is
{"text", "ms", "seconds", "model"}; refusals use the gateway's {"error",
"message"} shape.

Audio never touches a disk. The body is read into one bytearray; Opus is decoded
by a pinned-version ffmpeg inside the same bubblewrap sandbox shape as the
compiler (no network, empty root, no host files beyond read-only /usr for its
shared libraries), stdin to stdout. The PCM goes over a pipe to a pinned static
whisper-cli (whisper.cpp) in a second sandbox that sees only its binary and the
pinned model, read-only. Both buffers are zeroed and dropped before the reply is
sent. The transcript is returned and never logged; only aggregate counters are
kept.

Limits: --transcribe-max-bytes body, MAX_SECONDS of audio (longer clips are
refused, never truncated), the chat route's per-address bucket parameters (own
bucket), at most --transcribe-cap transcriptions at once (503 busy otherwise).
"""

import hashlib
import os
import resource
import signal
import struct
import subprocess
import threading
import time

BWRAP = "/usr/bin/bwrap"
RATE = 16000
MAX_SECONDS = 15
MIN_SECONDS = 0.3
PCM_CAP = MAX_SECONDS * RATE * 2
TYPES = {"audio/webm": "webm", "video/webm": "webm", "audio/ogg": "ogg",
         "audio/wav": "wav", "audio/wave": "wav", "audio/x-wav": "wav", "audio/vnd.wave": "wav"}
DEMUX = {"webm": "matroska", "ogg": "ogg"}
# whisper.cpp v1.9.5 (commit d1be6fde11ac6e0407606b4e42fe72d34add8037), static, CPU only,
# GGML_OPENMP=OFF, GGML_NATIVE=ON on the Spark (build steps: DEPENDENCIES.md).
WHISPER_PIN = "0b3d63f4a90d5df14031479ed039fe1f96ffc52fff8b193e85f71daa3762ed5a"
# ggerganov/whisper.cpp model repo revision 5359861c739e955e79d9a303bcbc70fb988958b1 (LFS sha256).
MODEL_PINS = {"a03779c86df3323075f5e796cb2ce5029f00ec8869eee3fdfb897afe36c6d002": "base.en",
              "921e4cf8686fdd993dcd081a5da5b6c365bfde1162e72b08d75ac75289920b1f": "tiny.en"}


def sniff(data):
    if data[:4] == b"\x1a\x45\xdf\xa3":
        return "webm"
    if data[:4] == b"OggS":
        return "ogg"
    if data[:4] == b"RIFF" and data[8:12] == b"WAVE":
        return "wav"
    return None


def wav_pcm(data):
    """(pcm16 bytes view, rate, channels) from a RIFF/WAVE PCM 16-bit file, or a reason string."""
    pos, fmt = 12, None
    while pos + 8 <= len(data):
        cid, size = data[pos:pos + 4], struct.unpack_from("<I", data, pos + 4)[0]
        body = pos + 8
        if cid == b"fmt ":
            if size < 16 or body + 16 > len(data):
                return "the WAV format chunk is truncated"
            tag, ch, rate, _, _, bits = struct.unpack_from("<HHIIHH", data, body)
            if tag not in (1, 0xFFFE) or bits != 16 or ch not in (1, 2) or not 8000 <= rate <= 48000:
                return "only 16-bit PCM WAV, mono or stereo, 8 to 48 kHz"
            fmt = (rate, ch)
        elif cid == b"data":
            if fmt is None:
                return "the WAV data chunk comes before its format"
            end = min(body + size, len(data))
            n = (end - body) // (2 * fmt[1]) * (2 * fmt[1])
            return memoryview(data)[body:body + n], fmt[0], fmt[1]
        pos = body + size + (size & 1)
    return "the WAV file has no audio data"


def wav_header(n):
    return (b"RIFF" + struct.pack("<I", 36 + n) + b"WAVEfmt " + struct.pack("<IHHIIHH", 16, 1, 1, RATE, RATE * 2, 2, 16)
            + b"data" + struct.pack("<I", n))


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def limits(cpu_s, mem):
    def apply():
        os.setsid()
        resource.setrlimit(resource.RLIMIT_CPU, (cpu_s, cpu_s + 1))
        resource.setrlimit(resource.RLIMIT_AS, (mem, mem))
        resource.setrlimit(resource.RLIMIT_FSIZE, (0, 0))
        resource.setrlimit(resource.RLIMIT_NOFILE, (64, 64))
        resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    return apply


def run(binds, argv, data, wall_s, out_cap, cpu_s, mem):
    """One sandboxed run, stdin to stdout. Nothing is bound writable; /tmp is a 1 MiB tmpfs."""
    cmd = [BWRAP, "--unshare-all", "--die-with-parent", "--new-session", "--clearenv", "--cap-drop", "ALL"]
    for src, dst in binds:
        cmd += ["--ro-bind", src, dst]
    cmd += ["--size", str(1 << 20), "--tmpfs", "/tmp", "--dev", "/dev", "--remount-ro", "/", "--chdir", "/tmp",
            "--"] + argv
    p = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                         preexec_fn=limits(cpu_s, mem), close_fds=True)
    try:
        out, err = p.communicate(data, timeout=wall_s)
    except subprocess.TimeoutExpired:
        os.killpg(p.pid, signal.SIGKILL)
        p.communicate()
        return None, b"timeout", -9
    if len(out) > out_cap:
        return None, b"output too large", -1
    return out, err[-2048:], p.returncode


class Transcriber:
    def __init__(self, whisper, model, ffmpeg, threads, cap, max_bytes):
        have = sha256_file(whisper)
        if have != WHISPER_PIN:
            raise SystemExit("allen-chat: %s sha256 %s, pinned %s" % (whisper, have, WHISPER_PIN))
        have = sha256_file(model)
        if have not in MODEL_PINS:
            raise SystemExit("allen-chat: %s sha256 %s is not a pinned whisper model" % (model, have))
        if not os.access(ffmpeg, os.X_OK) or not os.access(BWRAP, os.X_OK):
            raise SystemExit("allen-chat: transcription needs %s and %s" % (ffmpeg, BWRAP))
        self.whisper, self.model, self.ffmpeg = whisper, model, ffmpeg
        self.model_name = MODEL_PINS[have]
        self.threads = threads
        self.max_bytes = max_bytes
        self.slots = threading.BoundedSemaphore(cap)
        # Fail closed: both sandboxes must work here, or the gateway does not start.
        silent = bytearray(RATE // 2 * 2)
        text = self.transcribe(bytearray(wav_header(len(silent)) + silent), "wav")
        if not isinstance(text, tuple):
            raise SystemExit("allen-chat: the transcription sandbox does not work here: %s" % text)
        out, _, rc = run(self.usr(), [ffmpeg, "-hide_banner", "-version"], b"", 5, 1 << 16, 5, 1 << 30)
        if rc != 0 or not out:
            raise SystemExit("allen-chat: the decoder sandbox does not work here (rc %s)" % rc)

    @staticmethod
    def usr():
        """ffmpeg is the distribution's dynamic build: it sees /usr, /lib and the loader's
        /etc/ld.so.cache and /etc/alternatives (library symlinks) read-only, nothing else."""
        return [(p, p) for p in ("/usr", "/lib", "/etc/ld.so.cache", "/etc/alternatives") if os.path.exists(p)]

    def decode(self, data, kind):
        """16 kHz mono PCM16 (bytearray) or a reason string. Never truncates: too long is refused.
        Only the forced demuxer and the native Opus decoder are reachable; only pipe: I/O."""
        argv = [self.ffmpeg, "-hide_banner", "-loglevel", "error", "-nostats", "-threads", "1",
                "-protocol_whitelist", "pipe", "-f", DEMUX[kind], "-c:a", "opus", "-i", "pipe:0",
                "-map", "0:a:0", "-t", str(MAX_SECONDS + 0.05), "-ac", "1", "-ar", str(RATE),
                "-f", "s16le", "pipe:1"]
        out, _, rc = run(self.usr(), argv, data, 5, PCM_CAP + RATE * 2, 10, 1 << 30)
        if rc != 0 or out is None:
            return "the audio could not be decoded as Opus"
        pcm = bytearray(out)
        del out  # the immutable pipe copy is dropped here; the bytearray is zeroed by the caller
        if len(pcm) > PCM_CAP:
            pcm[:] = bytes(len(pcm))
            return "too_long"
        return pcm

    def transcribe(self, data, kind):
        """(text, seconds) or a reason string ("too_long" for over-length audio)."""
        if kind == "wav":
            got = wav_pcm(data)
            if isinstance(got, str):
                return got
            pcm, rate, ch = got
            seconds = len(pcm) / (2.0 * rate * ch)
            if seconds > MAX_SECONDS:
                return "too_long"
            if rate == RATE and ch == 1:
                wav = bytearray(wav_header(len(pcm))) + pcm
            else:  # whisper-cli resamples and downmixes WAV itself; pass a canonical header
                wav = bytearray(b"RIFF" + struct.pack("<I", 36 + len(pcm)) + b"WAVEfmt "
                                + struct.pack("<IHHIIHH", 16, 1, ch, rate, rate * ch * 2, ch * 2, 16)
                                + b"data" + struct.pack("<I", len(pcm))) + pcm
            pcm.release()
        else:
            pcm = self.decode(data, kind)
            if isinstance(pcm, str):
                return pcm
            seconds = len(pcm) / (2.0 * RATE)
            wav = bytearray(wav_header(len(pcm))) + pcm
            pcm[:] = bytes(len(pcm))
        try:
            if seconds < MIN_SECONDS:
                return "the clip is too short"
            argv = ["/tool", "-m", "/model", "-t", str(self.threads), "-l", "en", "-bs", "1", "-bo", "1",
                    # With stdin input whisper-cli would name its output "-" and open /dev/stdout,
                    # which needs /proc; a named -of (no -o* format, so no file is written)
                    # keeps its segment printer on stdout instead.
                    "-nt", "-np", "-sns", "-of", "/tmp/none", "-f", "-"]
            out, err, rc = run([(self.whisper, "/tool"), (self.model, "/model")], argv, wav, 15, 64 << 10,
                               60, 1 << 30)
        finally:
            wav[:] = bytes(len(wav))
        if rc != 0 or out is None:
            return "transcription failed (%s)" % ("timeout" if rc == -9 else "exit %d" % rc)
        return clean(out.decode("utf-8", "replace")), seconds


def clean(text):
    """Join segment lines; drop whisper's non-speech markers such as [BLANK_AUDIO] and (music)."""
    words = []
    for line in text.splitlines():
        line = line.strip()
        while line[:1] in "[(" and line[-1:] in "])" and line:
            close = "]" if line[0] == "[" else ")"
            cut = line.find(close)
            line = line[cut + 1:].strip() if cut >= 0 else ""
        if line:
            words.append(line)
    return " ".join(words)[:2000]


def handle(h):
    """POST /allen/api/transcribe on the gateway's Handler `h`."""
    app = h.app
    t = app.transcriber
    if t is None:
        return h.send_json(404, {"error": "not_found"})
    app.stats.add("transcribe_requests")
    try:
        n = int(h.headers.get("Content-Length", ""))
    except ValueError:
        return h.refuse(411, "length", "Send the clip with a Content-Length.")
    if n > t.max_bytes:
        return h.refuse(413, "too_long", "Voice clips are limited to %d KiB and %d seconds here."
                        % (t.max_bytes >> 10, MAX_SECONDS))
    if n <= 0:
        return h.refuse(400, "bad_request", "The clip is empty.")
    ctype = h.headers.get("Content-Type", "").split(";")[0].strip().lower()
    kind = TYPES.get(ctype)
    if kind is None:
        return h.refuse(415, "format", "Send audio/webm or audio/ogg (Opus) or audio/wav (16-bit PCM).")
    retry = app.transcribe_rate.take(app.client_key(h))
    if retry:
        return h.refuse(429, "rate", "You are sending clips quickly. Try again shortly.", retry)
    data = bytearray(n)
    try:
        got = h.rfile.readinto(data)
    except (OSError, ValueError):
        got = -1
    if got != n:
        data[:] = bytes(n)
        h.close_connection = True
        return
    h.body_read = True
    try:
        if sniff(data) != kind:
            return h.refuse(415, "format", "The clip is not the format its Content-Type says.")
        if not t.slots.acquire(blocking=False):
            return h.refuse(503, "busy", "Voice input is busy right now. Try again in a moment.", 2)
        try:
            t0 = time.monotonic()
            got = t.transcribe(data, kind)
        finally:
            t.slots.release()
    finally:
        data[:] = bytes(n)
        del data
    if got == "too_long":
        return h.refuse(413, "too_long", "Voice clips are limited to %d seconds here." % MAX_SECONDS)
    if isinstance(got, str):
        app.stats.add("transcribe_fail")
        return h.refuse(422, "audio", "Aien could not hear that clip: %s." % got)
    text, seconds = got
    app.stats.add("transcribe_ok")
    h.send_json(200, {"text": text, "ms": int((time.monotonic() - t0) * 1000), "seconds": round(seconds, 2),
                      "model": t.model_name})
