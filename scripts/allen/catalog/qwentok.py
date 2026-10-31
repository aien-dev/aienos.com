"""Exact Qwen3.6 token count for ASCII text, stdlib only (no model call).

Byte-level BPE from the model's tokenizer.json; the pre-tokenizer regex is the
model's with \\p{L}/\\p{M}/\\p{N} narrowed to ASCII, which is exact for ASCII input.
Usage: qwentok.py TOKENIZER_JSON FILE...
"""

import json
import re
import sys

PRE = re.compile(r"(?i:'s|'t|'re|'ve|'m|'ll|'d)|[^\r\nA-Za-z0-9]?[A-Za-z]+|[0-9]| ?[^\sA-Za-z0-9]+[\r\n]*"
                 r"|\s*[\r\n]+|\s+(?!\S)|\s+")


def byte_map():
    bs = list(range(33, 127)) + list(range(161, 173)) + list(range(174, 256))
    cs, n = bs[:], 0
    for b in range(256):
        if b not in bs:
            bs.append(b)
            cs.append(256 + n)
            n += 1
    return {b: chr(c) for b, c in zip(bs, cs)}


class Tok:
    def __init__(self, path):
        m = json.load(open(path, encoding="utf-8"))["model"]
        self.vocab = m["vocab"]
        self.ranks = {tuple(x.split(" ", 1)) if isinstance(x, str) else tuple(x): i
                      for i, x in enumerate(m["merges"])}
        self.bmap = byte_map()

    def bpe(self, word):
        parts = list(word)
        while len(parts) > 1:
            best = min(((self.ranks.get((a, b), 1 << 30), i) for i, (a, b) in enumerate(zip(parts, parts[1:]))))
            if best[0] == 1 << 30:
                break
            i = best[1]
            parts[i:i + 2] = [parts[i] + parts[i + 1]]
        return parts

    def count(self, text):
        text.encode("ascii")
        n = 0
        for piece in PRE.findall(text):
            n += len(self.bpe("".join(self.bmap[b] for b in piece.encode())))
        return n


if __name__ == "__main__":
    t = Tok(sys.argv[1])
    for f in sys.argv[2:]:
        print(t.count(open(f, encoding="ascii").read()), f)
