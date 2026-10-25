"""Static presentation checks. Does not substitute for a Rust build or browser QA."""
from pathlib import Path
import hashlib,re,unittest,xml.etree.ElementTree as ET
ROOT=Path(__file__).resolve().parents[1]
class CosmicChecks(unittest.TestCase):
 def test_omega_bytes_untouched(self):
  p=ROOT/'om/hive-page/main.html'
  self.assertEqual(hashlib.sha256(p.read_bytes()).hexdigest(),(ROOT/'om/hive-page/main.html.sha256').read_text().split()[0])
 def test_decorative_asset(self):
  p=ROOT/'public/assets/hive-universe.svg'
  self.assertEqual(ET.parse(p).getroot().attrib['viewBox'],'0 0 1600 900')
  self.assertNotRegex(p.read_text(),r'<script|<foreignObject|href=')
 def test_contracts(self):
  css=(ROOT/'public/assets/hive.css').read_text()
  self.assertIn("url('/assets/hive-universe.svg')",css)
  self.assertIn('pointer-events:none',css)
  self.assertIn('prefers-reduced-motion:reduce',css)
  self.assertNotRegex(css,r'#[0-9a-fA-F]{3}(?:[0-9a-fA-F]{3})?\b')
  self.assertNotIn('@keyframes',css)
  self.assertIn(':focus-visible',css)
 def test_scene_layer_is_stable(self):
  css=(ROOT/'public/assets/hive.css').read_text()
  # The galaxy must stay a composited layer: a negative z-index under
  # isolation made Chromium rebuild it on scroll (the reported flicker). It
  # stays cover sized, because the band is wide and short and a height-relative
  # size leaves a bare strip of page background on the right.
  self.assertNotRegex(css,r'z-index\s*:\s*-')
  self.assertNotIn('isolation:isolate',css)
  self.assertIn('will-change:transform',css)
  self.assertIn('transform:translateZ(0)',css)
  self.assertIn('/ cover no-repeat',css)
  self.assertNotIn('auto 120%',css)
 def test_omega_provenance_is_visible(self):
  page=(ROOT/'om/hive-page/main.html').read_text()
  self.assertIn('hive-omega',page)
  self.assertIn('This page is written in Omega.',page)
  self.assertIn('bear with the glitches while we iron it out',page)
  self.assertIn('Read the Omega source',page)
  self.assertIn('.hive-omega {',(ROOT/'public/assets/hive.css').read_text())
if __name__=='__main__':unittest.main()
