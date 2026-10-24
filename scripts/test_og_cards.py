"""Social-card regression checks: python3 -m unittest discover -s scripts -p 'test_og_cards.py'."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

from PIL import Image, ImageDraw

SPEC = importlib.util.spec_from_file_location("og_cards", Path(__file__).with_name("og-cards.py"))
cards = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(cards)
ROOT = Path(__file__).resolve().parents[1]


class SocialCardsTests(unittest.TestCase):
    def test_license_matches_site_metadata_and_controlling_text(self):
        seo = (ROOT / "crates/site-builder/src/seo.rs").read_text()
        self.assertIn(f'LICENSE_NAME: &str = "{cards.LICENSE_NAME}"', seo)
        self.assertIn("LLVM Exceptions", (ROOT / "LICENSE").read_text())
        self.assertIn(cards.LICENSE_NAME, (ROOT / "public/licensing/index.html").read_text())
        self.assertEqual(cards.BADGES[0][0], f"Open source, {cards.LICENSE_NAME}")
        self.assertNotIn("AGPL", cards.BADGES[0][0])

    def test_badges_fit_inside_card_margins(self):
        draw = ImageDraw.Draw(Image.new("RGB", (cards.W, cards.H)))
        font = cards.badge_font(draw)
        width = sum(draw.textlength(label, font=font) + 30 for label, _ in cards.BADGES)
        self.assertLessEqual(width + 16 * (len(cards.BADGES) - 1), cards.W - 160)

    def test_all_cards_render_deterministically_at_published_dimensions(self):
        old_out = cards.OUT
        try:
            with tempfile.TemporaryDirectory() as directory:
                cards.OUT = directory
                for card in cards.CARDS:
                    with self.subTest(card=card[0]):
                        cards.card(*card)
                        rendered = Path(directory, card[0])
                        first = rendered.read_bytes()
                        cards.card(*card)
                        self.assertEqual(rendered.read_bytes(), first)
                        with Image.open(ROOT / "public" / card[0]) as published:
                            self.assertEqual(published.size, (1200, 630))
                        with Image.open(rendered) as image:
                            self.assertEqual(image.size, (1200, 630))
                            self.assertEqual(image.mode, "RGB")
        finally:
            cards.OUT = old_out


if __name__ == "__main__":
    unittest.main()
