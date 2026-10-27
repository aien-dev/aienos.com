#!/usr/bin/env python3
"""Host verification driver for verify.om; never part of the serving path.

run.py BINARY [--fast]
Keys and signatures come from the openssl CLI (the oracle): P-256 ECDSA SHA-256 (DER) and RSA
2048/3072 PKCS#1 v1.5 SHA-256. Every signature-level vector is also checked with
`openssl dgst -verify`, whose verdict must match the expected one. Exit codes are compared exactly.
Prints one RESULT line; exit 0 only when every case matched.
"""
import os
import random
import re
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
EC_NOTES = (HERE.parent / 'vendor' / 'ec.om').read_text()


def curve_const(name):
    # First occurrence is P-256 (vendor/ec.om fn ec_p256 notes, values from FIPS 186-4 D.1.2.3).
    return int(re.search(r'^note %s = 0x([0-9a-f]+)' % name, EC_NOTES, re.M).group(1), 16)


P, N, B = curve_const('p'), curve_const('n'), curve_const('b')
RNG = random.Random(20261027)


def sh(args, data=None):
    return subprocess.run(args, input=data, capture_output=True, check=True).stdout


class Ctx:
    def __init__(self, binary, tmp, fast):
        self.binary, self.tmp, self.fast = binary, Path(tmp), fast
        self.cases = self.valid = self.oracle = 0
        self.failures = []
        self.seq = 0

    def path(self, suffix):
        self.seq += 1
        return str(self.tmp / ('f%d%s' % (self.seq, suffix)))

    def run(self, data, args=()):
        return subprocess.run([self.binary, *args], input=data, capture_output=True, timeout=60)

    def expect(self, name, data, want, args=()):
        self.cases += 1
        proc = self.run(data, args)
        if proc.returncode != want or proc.stdout or proc.stderr:
            self.failures.append('%s: want %d got %d' % (name, want, proc.returncode))
            if self.fast:
                print('FAIL', self.failures[-1])
                sys.exit(1)
        elif want == 0 and not args:
            self.valid += 1

    def openssl_agrees(self, name, pem_pub, msg, sig, valid):
        mp, sp = self.path('.msg'), self.path('.sig')
        Path(mp).write_bytes(msg)
        Path(sp).write_bytes(sig)
        r = subprocess.run(['openssl', 'dgst', '-sha256', '-verify', pem_pub, '-signature', sp, mp],
                           capture_output=True)
        self.oracle += 1
        if (r.returncode == 0) != valid:
            self.failures.append('%s: openssl verdict %d disagrees with expected valid=%s'
                                 % (name, r.returncode, valid))
            if self.fast:
                print('FAIL', self.failures[-1])
                sys.exit(1)


def req(alg, pub, msg=None, sig=None, upper=False):
    h = (lambda b: b.hex().upper()) if upper else (lambda b: b.hex())
    s = 'alg %s\npub %s\n' % (alg, h(pub))
    if msg is not None:
        s += 'msg %s\nsig %s\n' % (h(msg), h(sig))
    return s.encode()


def der_len(data, i):
    n = data[i]
    if n < 0x80:
        return n, i + 1
    k = n & 0x7f
    return int.from_bytes(data[i + 1:i + 1 + k], 'big'), i + 1 + k


def der_ints(data):
    """Parse SEQUENCE { INTEGER, INTEGER } and return the two integers."""
    assert data[0] == 0x30
    _, i = der_len(data, 1)
    out = []
    for _ in range(2):
        assert data[i] == 0x02
        n, i = der_len(data, i + 1)
        out.append(int.from_bytes(data[i:i + n], 'big'))
        i += n
    return out


def der_int(v):
    b = v.to_bytes(max(1, (v.bit_length() + 7) // 8), 'big')
    if b[0] & 0x80:
        b = b'\x00' + b
    return b'\x02' + bytes([len(b)]) + b


def ecdsa_der(r, s):
    body = der_int(r) + der_int(s)
    return b'\x30' + bytes([len(body)]) + body


def on_curve(x, y):
    return (y * y - (x * x * x - 3 * x + B)) % P == 0


def new_ec(ctx):
    key = ctx.path('.pem')
    sh(['openssl', 'genpkey', '-algorithm', 'EC', '-pkeyopt', 'ec_paramgen_curve:P-256', '-out', key])
    pem = ctx.path('.pub')
    sh(['openssl', 'pkey', '-in', key, '-pubout', '-out', pem])
    raw = sh(['openssl', 'pkey', '-in', key, '-pubout', '-outform', 'DER'])[-65:]
    return key, pem, raw


def new_rsa(ctx, bits):
    key = ctx.path('.pem')
    sh(['openssl', 'genpkey', '-algorithm', 'RSA', '-pkeyopt', 'rsa_keygen_bits:%d' % bits, '-out', key])
    pem = ctx.path('.pub')
    sh(['openssl', 'pkey', '-in', key, '-pubout', '-out', pem])
    der = sh(['openssl', 'rsa', '-in', key, '-RSAPublicKey_out', '-outform', 'DER'])
    return key, pem, der


def sign(ctx, key, msg, digest='-sha256'):
    mp = ctx.path('.msg')
    Path(mp).write_bytes(msg)
    return sh(['openssl', 'dgst', digest, '-sign', key, mp])


def message(i):
    if i % 5 == 0:  # WebAuthn shape: authenticatorData (37 bytes) || SHA-256(clientDataJSON)
        return bytes(RNG.randrange(256) for _ in range(37 + 32))
    return bytes(RNG.randrange(256) for _ in range(RNG.choice([0, 1, 31, 32, 55, 64, 100, 300, 1000])))


def flip(b, i, bit=1):
    a = bytearray(b)
    a[i] ^= bit
    return bytes(a)


def ec_cases(ctx, keys):
    for ki, (key, pem, pub) in enumerate(keys):
        other = keys[(ki + 1) % len(keys)]
        for j in range(45):
            name = 'es%d.%d' % (ki, j)
            msg = message(j)
            sig = sign(ctx, key, msg)
            r, s = der_ints(sig)
            ctx.expect(name + ' valid', req(-7, pub, msg, sig, upper=(j % 9 == 1)), 0)
            if j % 3:
                continue
            ctx.openssl_agrees(name + ' valid', pem, msg, sig, True)
            hi = ecdsa_der(r, N - s)  # high-S: the negated s is an equally valid ECDSA signature
            ctx.expect(name + ' high-S', req(-7, pub, msg, hi), 0)
            ctx.openssl_agrees(name + ' high-S', pem, msg, hi, True)
            fm = flip(msg, 0) if msg else b'\x00'
            ctx.expect(name + ' flipped msg', req(-7, pub, fm, sig), 1)
            ctx.openssl_agrees(name + ' flipped msg', pem, fm, sig, False)
            s2 = s ^ 1
            fs = ecdsa_der(r, s2)
            ctx.expect(name + ' flipped sig', req(-7, pub, msg, fs), 1 if 0 < s2 < N else 4)
            ctx.openssl_agrees(name + ' flipped sig', pem, msg, fs, False)
            ctx.expect(name + ' wrong key', req(-7, other[2], msg, sig), 1)
            ctx.openssl_agrees(name + ' wrong key', other[1], msg, sig, False)
            for label, bad in (('s=0', ecdsa_der(r, 0)), ('s=n', ecdsa_der(r, N)), ('s=n+1', ecdsa_der(r, N + 1)),
                               ('r=0', ecdsa_der(0, s)), ('r=n', ecdsa_der(N, s))):
                ctx.expect(name + ' ' + label, req(-7, pub, msg, bad), 4)
                ctx.openssl_agrees(name + ' ' + label, pem, msg, bad, False)
            ctx.expect(name + ' truncated DER', req(-7, pub, msg, sig[:-1]), 2)
            ctx.expect(name + ' trailing byte DER', req(-7, pub, msg, sig + b'\x00'), 2)
            ctx.expect(name + ' empty sig', req(-7, pub, msg, b''), 2)
            x = int.from_bytes(pub[1:33], 'big')
            y2 = (int.from_bytes(pub[33:], 'big') + 1) % P
            assert not on_curve(x, y2)
            off = pub[:33] + y2.to_bytes(32, 'big')
            ctx.expect(name + ' off-curve', req(-7, off, msg, sig), 3)
            ctx.expect(name + ' off-curve --key', req(-7, off), 3, ['--key'])
            ctx.expect(name + ' compressed --key', req(-7, bytes([2 + (y2 & 1)]) + pub[1:33]), 3, ['--key'])
            ctx.expect(name + ' x>=p --key', req(-7, b'\x04' + b'\xff' * 64), 3, ['--key'])
            ctx.expect(name + ' --key', req(-7, pub), 0, ['--key'])


def rsa_cases(ctx, keys):
    for ki, (key, pem, der, bits) in enumerate(keys):
        other = keys[(ki + 1) % len(keys)]
        n = der_ints(der)[0]
        k = (bits + 7) // 8
        count = 16 if bits == 2048 else 12
        for j in range(count):
            name = 'rs%d.%d' % (ki, j)
            msg = message(j)
            sig = sign(ctx, key, msg)
            ctx.expect(name + ' valid', req(-257, der, msg, sig, upper=(j % 7 == 2)), 0)
            if j % 3:
                continue
            ctx.openssl_agrees(name + ' valid', pem, msg, sig, True)
            fm = flip(msg, len(msg) - 1) if msg else b'\x01'
            ctx.expect(name + ' flipped msg', req(-257, der, fm, sig), 21)
            ctx.openssl_agrees(name + ' flipped msg', pem, fm, sig, False)
            fs = flip(sig, k - 1)
            ctx.expect(name + ' flipped sig', req(-257, der, msg, fs), 21 if int.from_bytes(fs, 'big') < n else 24)
            ctx.openssl_agrees(name + ' flipped sig', pem, msg, fs, False)
            if other[3] == bits:
                on = der_ints(other[2])[0]  # a signature >= the other modulus is code 4 there
                ctx.expect(name + ' wrong key', req(-257, other[2], msg, sig), 21 if int.from_bytes(sig, 'big') < on else 24)
                ctx.openssl_agrees(name + ' wrong key', other[1], msg, sig, False)
            ctx.expect(name + ' short sig', req(-257, der, msg, sig[1:]), 24)
            ctx.expect(name + ' s=n', req(-257, der, msg, n.to_bytes(k, 'big')), 24)
            sha1 = sign(ctx, key, msg, '-sha1')
            ctx.expect(name + ' sha1 sig', req(-257, der, msg, sha1), 21)
            ctx.openssl_agrees(name + ' sha1 sig', pem, msg, sha1, False)
            ctx.expect(name + ' truncated key', req(-257, der[:-1], msg, sig), 22)
            ctx.expect(name + ' --key', req(-257, der), 0, ['--key'])
            ctx.expect(name + ' ES256 key as RS256', req(-257, b'\x04' + b'\x01' * 64, msg, sig), 22)


def malformed_cases(ctx, ec, rsa):
    _, _, pub = ec
    msg = b'webauthn'
    sig = sign(ctx, ec[0], msg)
    good = req(-7, pub, msg, sig)
    ctx.expect('no final LF', good[:-1], 0)
    bad = {
        'empty input': b'',
        'alg -8': good.replace(b'alg -7', b'alg -8'),
        'alg 7': good.replace(b'alg -7', b'alg 7'),
        'alg -70': good.replace(b'alg -7', b'alg -70'),
        'missing sig line': good[:good.index(b'sig ')],
        'odd hex': good.replace(b'msg ', b'msg 0'),
        'non-hex': good.replace(b'msg ', b'msg zz'),
        'extra line': good + b'x\n',
        'extra LF': good + b'\n',
        'CRLF': good.replace(b'\n', b'\r\n'),
        'leading space': b' ' + good,
        'two spaces': good.replace(b'pub ', b'pub  '),
        'reordered': b'\n'.join([good.split(b'\n')[i] for i in (0, 2, 1, 3)]) + b'\n',
        'uppercase keyword': good.replace(b'sig ', b'SIG '),
        'pub 8193 bytes': req(-7, b'\x04' * 8193, msg, sig),
        'trailing bytes': good + b'#' * 9,
        'oversized 1 MiB': b'a' * (1 << 20),
    }
    for label, data in bad.items():
        ctx.expect('malformed ' + label, data, 9)
    ctx.expect('malformed --key with msg', good, 9, ['--key'])
    ctx.expect('bad argument', good, 9, ['--bogus'])
    ctx.expect('two arguments', req(-7, pub), 9, ['--key', '--key'])
    # Every request is even in length (fixed keywords 24 bytes plus hex). The request of 16384 bytes
    # is accepted; a valid request of 16385 bytes (16386 minus its optional final LF) is refused.
    rkey, rpem, rder = rsa
    base = len(req(-257, rder, b'', b'\x00' * 256))
    big = bytes(RNG.randrange(256) for _ in range((16384 - base) // 2))
    data = req(-257, rder, big, sign(ctx, rkey, big))
    assert len(data) == 16384, len(data)
    ctx.expect('exactly 16384 bytes', data, 0)
    ctx.openssl_agrees('exactly 16384 bytes', rpem, big, sign(ctx, rkey, big), True)
    big2 = big + b'\x01'
    data2 = req(-257, rder, big2, sign(ctx, rkey, big2))[:-1]
    assert len(data2) == 16385, len(data2)
    ctx.expect('valid request of 16385 bytes', data2, 9)


def timing(ctx, data):
    ts = []
    for _ in range(30):
        t0 = time.perf_counter()
        assert ctx.run(data).returncode == 0
        ts.append(time.perf_counter() - t0)
    return min(ts) * 1000, statistics.median(ts) * 1000


def main():
    binary = os.path.abspath(sys.argv[1])
    fast = '--fast' in sys.argv
    with tempfile.TemporaryDirectory() as tmp:
        ctx = Ctx(binary, tmp, fast)
        ec_keys = [new_ec(ctx) for _ in range(5)]
        rsa_keys = [(*new_rsa(ctx, b), b) for b in (2048, 2048, 3072, 3072)]
        weak = new_rsa(ctx, 1024)
        strong = new_rsa(ctx, 4096)
        ctx.expect('rsa1024 --key refused', req(-257, weak[2]), 23, ['--key'])
        msg = b'weak key'
        wsig = sign(ctx, weak[0], msg)
        ctx.openssl_agrees('rsa1024 valid for openssl', weak[1], msg, wsig, True)
        ctx.expect('rsa1024 signature refused by policy', req(-257, weak[2], msg, wsig), 23)
        ctx.expect('rsa4096 --key', req(-257, strong[2]), 0, ['--key'])
        ssig = sign(ctx, strong[0], msg)
        ctx.expect('rsa4096 valid', req(-257, strong[2], msg, ssig), 0)
        rsa3 = new_rsa_e3(ctx)
        ctx.expect('rsa2048 e=3 --key', req(-257, rsa3[2]), 0, ['--key'])
        ec_cases(ctx, ec_keys)
        rsa_cases(ctx, rsa_keys)
        malformed_cases(ctx, ec_keys[0], rsa_keys[0][:3])
        m = message(5)
        es = timing(ctx, req(-7, ec_keys[0][2], m, sign(ctx, ec_keys[0][0], m)))
        r2 = timing(ctx, req(-257, rsa_keys[0][2], m, sign(ctx, rsa_keys[0][0], m)))
        r3 = timing(ctx, req(-257, rsa_keys[2][2], m, sign(ctx, rsa_keys[2][0], m)))
        if ctx.failures:
            for f in ctx.failures[:20]:
                print('FAIL', f)
            print('RESULT FAIL failures=%d' % len(ctx.failures))
            sys.exit(1)
        if ctx.valid < 200:
            print('RESULT FAIL valid=%d (< 200)' % ctx.valid)
            sys.exit(1)
        print('RESULT cases=%d valid=%d openssl_crosschecks=%d' % (ctx.cases, ctx.valid, ctx.oracle))
        print('TIMING ms(min/median of 30, process spawn included): es256=%.2f/%.2f rs2048=%.2f/%.2f rs3072=%.2f/%.2f'
              % (es + r2 + r3))


def new_rsa_e3(ctx):
    key = ctx.path('.pem')
    sh(['openssl', 'genpkey', '-algorithm', 'RSA', '-pkeyopt', 'rsa_keygen_bits:2048',
        '-pkeyopt', 'rsa_keygen_pubexp:3', '-out', key])
    der = sh(['openssl', 'rsa', '-in', key, '-RSAPublicKey_out', '-outform', 'DER'])
    return key, None, der


if __name__ == '__main__':
    main()
