#!/usr/bin/env python3
"""Draw the social cards (1200 x 630 PNG) in the site's look.

Near-black ground, a faint grid, a blue and a green glow, the name, one
claim, one line under it, and the page address. Fonts are rendered into
the PNG here, so the site itself loads no outside font. Run from the repo
root with any Python that has Pillow:

    python3 scripts/og-cards.py

Every card is listed in CARDS; the Rust builder maps pages to these files
in crates/site-builder/src/seo.rs (fn image_for). Keep the two in step.
"""
from PIL import Image, ImageDraw, ImageFilter, ImageFont
import os, sys

OUT = os.path.join(os.path.dirname(__file__), "..", "public")
W, H = 1200, 630
BG = (8, 9, 12)
GRID = (28, 37, 54)
BLUE = (45, 127, 249)
GREEN = (0, 229, 153)
TEXT = (241, 245, 249)
MUTED = (148, 163, 184)
DIM = (100, 116, 139)

CARDS = [
    # file, kicker, claim (two lines max), sub line, address
    ("og.png", "AienOS",
     "Persistent machine intelligence\non a machine you own",
     "Search turned into verified understanding. Every claim carries a public receipt.",
     "www.aienos.com"),
    ("og-machine.png", "THE MACHINE",
     "Five real machines,\none verified loop",
     "A Spark, an iMac, a Pi 5, a QEMU lab and a rig. Speeds stand beside the physical ceiling only.",
     "www.aienos.com/machine/"),
    ("og-now.png", "NOW",
     "What every agent\nis doing right now",
     "The live coordination ledger: open work, settled work with evidence, decisions waiting on the operator.",
     "www.aienos.com/now/"),
    ("og-turing.png", "THE TURING",
     "A unit of verified\nexplanatory gain",
     "Net held-out explanatory compression: one operational component of machine understanding.",
     "www.aienos.com/turing/"),
    ("og-research.png", "AIEN RESEARCH",
     "Preprints, experiments\nand public evidence",
     "The Turing papers, the Crumb Doctrine, the Omega language, the doctrine. Not peer reviewed; receipts attached.",
     "www.aienos.com/research/"),
    ("og-evidence.png", "EVIDENCE",
     "Receipts over claims",
     "Every number on this site carries its environment, its gate, its date and a link to the record. Failures stay on the record.",
     "www.aienos.com/evidence/"),
    ("og-blog.png", "BLOG",
     "Research notes\nfrom the build",
     "What shipped, what the evidence says, and what remains an open question.",
     "www.aienos.com/blog/"),
    ("og-discovery.png", "SCIENTIFIC DISCOVERY",
     "The machine is never\ngiven the named answer",
     "Physics Zero, Dirac, Zeta and supersymmetry as examinations of representation discovery.",
     "www.aienos.com/discovery/"),
    ("og-philosophy.png", "PHILOSOPHY",
     "Position, not evidence",
     "The Stapleton Doctrine and the Post-LLM Case: the stance behind the machine, argued in the open.",
     "www.aienos.com/philosophy/"),
    ("og-experiments.png", "EXPERIMENTS",
     "Every experiment,\nfailures included",
     "The research ladder rendered from one canonical status source, verdicts and limits attached.",
     "www.aienos.com/experiments/"),
]

FONT_DIRS = ["/System/Library/Fonts/Supplemental", "/System/Library/Fonts",
             "/usr/share/fonts/truetype/dejavu", "/usr/share/fonts/truetype/liberation"]
def font(names, size):
    # Names in preference order: Arial and Menlo on macOS; on Linux, Liberation
    # Sans (Arial's metrics) and DejaVu Sans Mono (Menlo's source), so a card
    # drawn on either machine keeps the same line breaks and badge widths.
    for n in names:
        for d in FONT_DIRS:
            p = os.path.join(d, n)
            if os.path.exists(p):
                return ImageFont.truetype(p, size)
    return ImageFont.load_default(size)

BOLD = ["Arial Bold.ttf", "LiberationSans-Bold.ttf", "DejaVuSans-Bold.ttf"]
REG = ["Arial.ttf", "LiberationSans-Regular.ttf", "DejaVuSans.ttf"]
MONO = ["Menlo.ttc", "DejaVuSansMono.ttf", "LiberationMono-Regular.ttf"]

def glow(base, xy, r, color, alpha):
    layer = Image.new("RGBA", base.size, (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    x, y = xy
    d.ellipse((x - r, y - r, x + r, y + r), fill=color + (alpha,))
    layer = layer.filter(ImageFilter.GaussianBlur(r * 0.6))
    base.alpha_composite(layer)

def wrap(draw, text, f, width):
    lines = []
    for para in text.split("\n"):
        words, cur = para.split(), ""
        for w in words:
            t = (cur + " " + w).strip()
            if draw.textlength(t, font=f) <= width:
                cur = t
            else:
                lines.append(cur); cur = w
        lines.append(cur)
    return lines

def card(file, kicker, claim, sub, address):
    im = Image.new("RGBA", (W, H), BG + (255,))
    d = ImageDraw.Draw(im)
    for x in range(0, W, 60):
        d.line((x, 0, x, H), fill=GRID + (40,), width=1)
    for y in range(0, H, 60):
        d.line((0, y, W, y), fill=GRID + (40,), width=1)
    glow(im, (1040, 120), 300, BLUE, 70)
    glow(im, (160, 600), 260, GREEN, 45)
    d = ImageDraw.Draw(im)
    # Brand mark: a small bracket in a blue box, like the site header.
    d.rounded_rectangle((80, 72, 134, 126), radius=8, outline=BLUE, width=2, fill=(14, 18, 25, 255))
    d.line((96, 112, 112, 99, 96, 86), fill=GREEN, width=3)
    d.line((112, 112, 122, 112), fill=GREEN, width=3)
    fk = font(BOLD, 28)
    d.text((152, 80), kicker, font=fk, fill=TEXT)
    fm = font(MONO, 22)
    aw = d.textlength(address, font=fm)
    d.text((W - 80 - aw, 86), address, font=fm, fill=DIM)
    fc = font(BOLD, 64)
    lines = wrap(d, claim, fc, 1040)
    y = 200
    for ln in lines[:3]:
        d.text((80, y), ln, font=fc, fill=TEXT)
        y += 74
    fs = font(REG, 28)
    y += 18
    for ln in wrap(d, sub, fs, 1000)[:2]:
        d.text((80, y), ln, font=fs, fill=MUTED)
        y += 38
    # Badges along the bottom.
    bx = 80
    for label, col in (("Open source, Apache-2.0", GREEN), ("Alpha testing", (245, 166, 35)), ("Receipts over claims", BLUE)):
        tw = d.textlength(label, font=fm)
        d.rounded_rectangle((bx, 518, bx + tw + 30, 558), radius=6, outline=col, width=1, fill=(14, 18, 25, 255))
        d.text((bx + 15, 526), label, font=fm, fill=col)
        bx += tw + 46
    im.convert("RGB").save(os.path.join(OUT, file), optimize=True)
    print("wrote", file)

if __name__ == "__main__":
    for c in CARDS:
        card(*c)

def logo():
    """A 512 x 512 mark for structured data (Organization.logo wants a raster)."""
    s = 512
    im = Image.new("RGBA", (s, s), BG + (255,))
    glow(im, (420, 90), 160, BLUE, 80)
    glow(im, (90, 430), 140, GREEN, 50)
    d = ImageDraw.Draw(im)
    d.rounded_rectangle((96, 96, 416, 416), radius=48, outline=BLUE, width=10, fill=(14, 18, 25, 255))
    d.line((176, 328, 268, 256, 176, 184), fill=GREEN, width=22, joint="curve")
    d.line((268, 328, 336, 328), fill=GREEN, width=22)
    im.convert("RGB").save(os.path.join(OUT, "logo.png"), optimize=True)
    print("wrote logo.png")

if __name__ == "__main__":
    logo()
