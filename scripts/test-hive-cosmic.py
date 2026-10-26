"""Static presentation checks. Does not substitute for a Rust build or browser QA.

The scene behind the /hive/ masthead is Allen's two black holes: om/hive-page/hive.om
computes the inspiral (native/kernel/ui/night.c orbit_angle and ck_night_at) and
writes the samples into om/hive-page/main.html as CSS keyframes. These checks
recompute that law here, independently, and compare every stop with it, and they
pin the two contracts the artifact cannot state about itself: only transform and
opacity are animated, and nothing animates under prefers-reduced-motion.
"""
from pathlib import Path
import hashlib, math, re, unittest, xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
PAGE = ROOT / 'om/hive-page/main.html'
CSS = ROOT / 'public/assets/hive.css'
GUIDE = ROOT / 'AGENTS.md'

CYCLE = 19000        # ms, one pass of the sequence plus the hold in the sky
APPEAR = 2600        # CK_NIGHT_HOLES_MS
SHADOW = 4000        # the shadow starts to grow (APPEAR + 1400)
SPIRAL = 6000        # CK_NIGHT_SPIRAL_MS
MERGE = 12000        # CK_NIGHT_MERGE_MS
FADE = 12900         # MERGE + 900, the last hole starts to dissolve
GONE = 15200         # CK_NIGHT_GONE_MS
RIPPLE = 15600       # MERGE + CK_NIGHT_RIPPLE_MS
A0 = 270.0           # separation of the wide orbit, Allen units
ORBIT = 5200.0       # ms per turn of the wide orbit
SF = (SPIRAL - APPEAR) * 65536 // ORBIT          # 42850, 1/65536 turns
TURNS = 65536 * 8 * (MERGE - SPIRAL) // (5 * ORBIT)  # 120989, 1/65536 turns
STEP = 40            # ms between position samples


def phase(page):
    m = re.search(r'<style>(.*?)</style>', page, re.S)
    assert m, 'the artifact no longer carries the emitted style block'
    return m.group(1)


def rules(css):
    """Top level rules of a stylesheet, comments stripped, as (header, body)."""
    css = re.sub(r'/\*.*?\*/', '', css, flags=re.S)
    out, i, n = [], 0, len(css)
    while i < n:
        open_at = css.find('{', i)
        if open_at < 0:
            break
        depth, k = 1, open_at + 1
        while k < n and depth:
            depth += {'{': 1, '}': -1}.get(css[k], 0)
            k += 1
        out.append((css[i:open_at].strip(), css[open_at + 1:k - 1]))
        i = k
    return out


def keyframes(css):
    return {h.split(None, 1)[1]: b for h, b in rules(css) if h.startswith('@keyframes')}


def stops(body):
    """[(ms, declarations)] of one @keyframes body."""
    body = re.sub(r'/\*.*?\*/', '', body, flags=re.S)
    return [(round(float(m.group(1)) * CYCLE / 100), m.group(2).strip())
            for m in re.finditer(r'([\d.]+)%\s*\{([^}]*)\}', body)]


def at(table, ms):
    """The declaration of the stop at ms, or of the nearest stop to it."""
    t, decl = min(table, key=lambda row: abs(row[0] - ms))
    return decl


def smooth(u):
    u = max(0, min(256, u))
    return u * u * (768 - 2 * u) >> 16


def prog(t, a, b):
    if t <= a:
        return 0
    if t >= b:
        return 256
    return (t - a) * 256 // (b - a)


def law(t):
    """Separation (Allen units) and angle (turns) at t ms, from night.c."""
    t = max(t, APPEAR)
    if t < SPIRAL:
        return A0, (t - APPEAR) / ORBIT
    left = max(0.0, (MERGE - t) / (MERGE - SPIRAL))
    r8 = left ** 0.125
    return A0 * r8 * r8, SF / 65536 + (TURNS / 65536) * (1.0 - r8 ** 5)


def place(t, sign):
    sep, turn = law(t)
    a = 2 * math.pi * turn
    return sign * 0.5 * sep * math.cos(a), sign * 0.25 * sep * math.sin(a)


def translate(decl):
    return [float(v) for v in re.findall(r'(-?[\d.]+)em', decl)]


class CosmicChecks(unittest.TestCase):

    def test_omega_bytes_untouched(self):
        self.assertEqual(hashlib.sha256(PAGE.read_bytes()).hexdigest(),
                         (ROOT / 'om/hive-page/main.html.sha256').read_text().split()[0])

    def test_residual_sky_asset(self):
        p = ROOT / 'public/assets/hive-universe.svg'
        self.assertEqual(ET.parse(p).getroot().attrib['viewBox'], '0 0 1600 900')
        self.assertNotRegex(p.read_text(), r'<script|<foreignObject|href=')

    def test_scene_is_allens_pair(self):
        page = PAGE.read_text()
        scene = re.search(r'<div class="hive-scene".*?</div></div>', page, re.S)
        self.assertTrue(scene, 'the artifact lost the scene markup')
        box = scene.group(0)
        self.assertIn('aria-hidden="true"', box)
        self.assertIn('data-scene="allen"', box)
        for name, ms in [('cycle', CYCLE), ('appear', APPEAR), ('spiral', SPIRAL),
                         ('merge', MERGE), ('gone', GONE), ('ripple', RIPPLE)]:
            self.assertIn(f'data-{name}="{ms}"', box, f'the scene must state {name} = {ms} ms')
        self.assertIn(f'--hive-cycle:{CYCLE}ms', box)
        self.assertEqual(box.count('class="hive-hole '), 2, 'two black holes, not one and not three')
        for part in ['hive-hole-a', 'hive-hole-b', 'hive-bend', 'hive-plate',
                     'hive-merge', 'hive-mglow', 'hive-mplate', 'hive-ring', 'hive-ripple']:
            self.assertIn(part, box, f'the scene must carry {part}')
        # The static frame: each hole has its place in the cycle written inline,
        # so with motion off the reader still sees the pair, at a separation.
        for cls, sign in [('hive-hole-a', 1), ('hive-hole-b', -1)]:
            m = re.search(r'class="hive-hole ' + cls + r'" style="([^"]+)"', box)
            self.assertTrue(m, f'{cls} must carry the static frame inline')
            x, y = translate(m.group(1))
            wx, wy = place(7000, sign)
            self.assertAlmostEqual(x, wx, delta=0.05)
            self.assertAlmostEqual(y, wy, delta=0.05)
            self.assertGreater(x * sign, 0.0, 'the two holes sit on opposite sides of the stage')
            self.assertGreater(math.hypot(x, y), 40.0, 'the static frame must show a visible separation')

    def test_keyframes_animate_transform_and_opacity_only(self):
        css = phase(PAGE.read_text())
        animated = keyframes(css)
        self.assertEqual(sorted(animated), ['hive-bend', 'hive-mkeep', 'hive-mring', 'hive-msiz',
                                            'hive-posa', 'hive-posb', 'hive-ripp', 'hive-size'])
        for name, body in animated.items():
            for block in re.findall(r'\{([^{}]*)\}', body):
                for decl in block.split(';'):
                    if not decl.strip():
                        continue
                    prop = decl.split(':')[0].strip()
                    self.assertIn(prop, ('transform', 'opacity'),
                                  f'{name} animates {prop}: the scene must stay on the compositor')
        # The other half of the same rule: nothing that would cost layout or a
        # repaint of the disc, and no shorthand that could hide one.
        for word in ['filter', 'box-shadow', 'background-position', 'background-size',
                     'width', 'height', 'left', 'top', 'font-size', 'animation-name']:
            for name, body in animated.items():
                self.assertNotIn(word, re.sub(r'scale\(|translate3d\(|opacity:', '', body),
                                 f'{name} must not touch {word}')

    def test_keyframes_follow_allens_law(self):
        animated = keyframes(phase(PAGE.read_text()))
        for name, sign in [('hive-posa', 1), ('hive-posb', -1)]:
            table = stops(animated[name])
            times = [t for t, _ in table]
            self.assertEqual(len(table), 1 + (MERGE - APPEAR) // STEP + 2,
                             f'{name} samples APPEAR to MERGE every {STEP} ms, and holds at both ends')
            self.assertEqual(times[0], 0)
            self.assertEqual(times[1], APPEAR)
            self.assertEqual(times[-2], MERGE)
            self.assertEqual(times[-1], CYCLE)
            self.assertEqual(times[2] - times[1], STEP)
            worst, where = 0.0, None
            for t, decl in table:
                x, y = translate(decl)
                wx, wy = place(t, sign)
                err = max(abs(x - wx), abs(y - wy))
                if err > worst:
                    worst, where = err, (t, x, y, wx, wy)
            self.assertLess(worst, 0.05,
                            f'{name} leaves the law at {where}: the samples are not night.c')
            # The pair starts wide, is still wide when the wide orbit ends, and
            # has collapsed to the merge point.
            for t in (0, APPEAR):
                x, y = translate(at(table, t))
                self.assertAlmostEqual(2 * abs(x), A0, delta=0.05, msg='the wide orbit is 270 units')
                self.assertAlmostEqual(y, 0.0, delta=0.05)
            self.assertAlmostEqual(law(SPIRAL)[0], A0, delta=0.05)
            x, y = translate(at(table, MERGE))
            self.assertEqual((x, y), (0.0, 0.0), 'the pair must collapse to the centre at the merger')
            x, y = translate(at(table, CYCLE))
            self.assertEqual((x, y), (0.0, 0.0))

    def test_merge_and_shadow_sizes(self):
        animated = keyframes(phase(PAGE.read_text()))
        size = stops(animated['hive-size'])
        self.assertAlmostEqual(float(re.search(r'([\d.]+)', at(size, APPEAR)).group(1)), 0.0,
                               msg='no shadow exists before it grows in')
        self.assertAlmostEqual(float(re.search(r'([\d.]+)', at(size, SHADOW)).group(1)), 0.0)
        self.assertAlmostEqual(float(re.search(r'([\d.]+)', at(size, SPIRAL)).group(1)), 1.0)
        self.assertAlmostEqual(float(re.search(r'([\d.]+)', at(size, MERGE - STEP)).group(1)), 1.0)
        self.assertAlmostEqual(float(re.search(r'([\d.]+)', at(size, MERGE)).group(1)), 0.0)
        merged = stops(animated['hive-msiz'])
        self.assertAlmostEqual(float(re.search(r'([\d.]+)', at(merged, MERGE)).group(1)), 1.26171875,
                               places=2, msg='the merged shadow starts at 323/256 of a single one')
        self.assertAlmostEqual(float(re.search(r'([\d.]+)', at(merged, MERGE + 180)).group(1)), 2.0,
                               places=2, msg='and reaches twice a single one after 180 ms')
        # The shadow keeps its merged size to the end; the group's opacity is what
        # dissolves it, so the shadow does not shrink as well (night.c shrinks it by
        # the square of the fade and the page folds that into the fade).
        self.assertAlmostEqual(float(re.search(r'([\d.]+)', at(merged, GONE)).group(1)), 2.0)
        # The ring wobbles at 95 ms and dies with a 260 ms time constant.
        ring = stops(animated['hive-mring'])
        peak = max(float(re.search(r'([\d.]+)', d).group(1)) for _, d in ring)
        self.assertLess(peak, 1.075, 'the ring stays inside 7 percent of the shadow radius')
        self.assertGreater(peak, 1.03, 'and it does move')
        late = float(re.search(r'([\d.]+)', at(ring, MERGE + 1500)).group(1))
        self.assertLess(abs(late - 1.0), 0.004, 'by 1.5 s the wobble is gone')
        keep = stops(animated['hive-mkeep'])
        self.assertAlmostEqual(float(re.search(r'([\d.]+)', at(keep, MERGE)).group(1)), 1.0)
        self.assertAlmostEqual(float(re.search(r'([\d.]+)', at(keep, FADE)).group(1)), 1.0)
        self.assertAlmostEqual(float(re.search(r'([\d.]+)', at(keep, GONE)).group(1)), 0.0)

    def test_ripple_runs_and_fades(self):
        ripp = stops(keyframes(phase(PAGE.read_text()))['hive-ripp'])
        first = at(ripp, MERGE)
        self.assertIn('scale(0.00', first)
        self.assertIn('opacity:0.00', first)
        peak = max(float(re.search(r'opacity:([\d.]+)', d).group(1)) for _, d in ripp)
        self.assertGreater(peak, 0.1, 'the ripple must be visible')
        self.assertLess(peak, 0.5, 'and it is a faint wave, not a flash')
        end = at(ripp, RIPPLE)
        self.assertIn('scale(1000.00', end)
        self.assertIn('opacity:0.00', end, 'the ripple has died by 15600 ms')

    def test_timeline_is_in_the_artifact(self):
        css = phase(PAGE.read_text())
        for ms in (APPEAR, SHADOW, SPIRAL, MERGE, FADE, GONE, RIPPLE, CYCLE):
            self.assertIn(f'{ms}ms', css, f'the artifact must name {ms} ms where the eye can read it')

    def test_nothing_animates_under_reduced_motion(self):
        page = PAGE.read_text()
        flat = re.sub(r'/\*.*?\*/', '', phase(page), flags=re.S)
        top = dict(rules(flat))
        gate = [h for h in top if h.startswith('@media') and 'prefers-reduced-motion:no-preference' in h]
        self.assertEqual(len(gate), 1, 'exactly one motion gate, so there is one place to look')
        inside = top[gate[0]]
        for name in ['hive-posa', 'hive-posb', 'hive-bend', 'hive-size', 'hive-msiz',
                     'hive-mring', 'hive-mkeep', 'hive-ripp']:
            self.assertIn(f'animation:{name} var(--hive-cycle) linear infinite', inside,
                          f'{name} must be attached inside the motion gate')
        self.assertEqual(inside.count('animation:'), 8)
        # The page's own control is the second key: the rule only fires on data-motion="on".
        self.assertEqual(inside.count('.hive[data-motion="on"]'), 8)
        # Every animation in the artifact is inside the gate: one place to look.
        self.assertEqual(flat.count('animation:'), inside.count('animation:'),
                         'an animation escaped the prefers-reduced-motion gate')
        outside = flat.replace(inside, '')
        self.assertNotIn('animation:', outside)
        sheet = CSS.read_text()
        self.assertNotIn('@keyframes', sheet)
        for value in re.findall(r'animation:([^;}]+)', sheet):
            self.assertTrue(value.strip().startswith('none'),
                            f'only the motion off rules may name an animation: {value!r}')

    def test_stylesheet_places_and_paints_only(self):
        sheet = CSS.read_text()
        self.assertIn("url('/assets/hive-universe.svg')", sheet)
        self.assertIn('pointer-events:none', sheet)
        self.assertIn('prefers-reduced-motion', sheet)
        self.assertIn('.hive[data-motion="off"]', sheet)
        self.assertNotRegex(sheet, r'#[0-9a-fA-F]{3}(?:[0-9a-fA-F]{3})?\b')
        self.assertIn(':focus-visible', sheet)
        for part in ['.hive-masthead > .hive-scene', '.hive-stage', '.hive-bend', '.hive-plate',
                     '.hive-mglow', '.hive-mplate', '.hive-ring', '.hive-ripple',
                     'font-size:var(--hive-u,1px)', '--hive-u:.86px']:
            self.assertIn(part, sheet, f'the stylesheet must still place {part}')
        # No negative z-index, no isolation: see the flicker note in the sheet.
        self.assertNotRegex(sheet, r'z-index\s*:\s*-')
        self.assertNotIn('isolation:isolate', sheet)
        self.assertIn('will-change:transform', sheet)
        self.assertIn('transform:translateZ(0)', sheet)
        self.assertIn('/ cover no-repeat', sheet)
        self.assertNotIn('auto 120%', sheet)
        # The scene's own layer sits at 0, the scrim at 1, the prose at 2.
        self.assertIn('.hive-masthead > * { position:relative; z-index:2; }', sheet)
        self.assertIn('z-index:1; pointer-events:none', sheet)

    def test_scrim_and_provenance_stay(self):
        sheet = CSS.read_text()
        for stop in ['92%, transparent) 0%', '90%, transparent) 68%',
                     '55%, transparent) 86%',
                     '94%, transparent) 0%', '90%, transparent) 55%',
                     '60%, transparent) 80%']:
            self.assertIn(stop, sheet, f'the scrim stop {stop} is a contrast contract')
        page = PAGE.read_text()
        self.assertIn('hive-omega', page)
        self.assertIn('This page is written in Omega.', page)
        self.assertIn('bear with the glitches while we iron it out', page)
        self.assertIn('Read the Omega source', page)
        self.assertIn('.hive-omega {', sheet)

    def test_the_guide_says_what_the_scene_is(self):
        guide = GUIDE.read_text()
        self.assertIn("Allen's law", guide)
        guide_low = guide.lower()
        self.assertIn('illustration', guide_low)
        for word in ['ripple', 'ring']:
            self.assertIn(word, guide_low)
        self.assertIn('om/hive-page/hive.om', guide)
        for text in [PAGE.read_text(), CSS.read_text(), guide]:
            self.assertNotIn('\u2014', text, 'no em dash')
            self.assertNotIn('\u2013', text, 'no en dash')


if __name__ == '__main__':
    unittest.main()
