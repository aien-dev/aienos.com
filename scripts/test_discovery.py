#!/usr/bin/env python3
"""Exercise emitted CUDA++ discovery artifacts, not their source strings."""
import json
from pathlib import Path
from html.parser import HTMLParser
import re
import unittest
import urllib.parse
import urllib.robotparser
import xml.etree.ElementTree as ET

DIST = Path(__file__).resolve().parents[1] / 'dist'
HOST = 'https://www.aienos.com'
ROUTES = ['/cudaplusplus/', '/cuda-alternative/', '/ai-inference-speed/']

class Document(HTMLParser):
    def __init__(self, raw):
        super().__init__()
        self.tags = []
        self.text = []
        self.feed(raw)
    def handle_starttag(self, tag, attrs):
        self.tags.append((tag, dict(attrs)))
    def handle_data(self, data):
        self.text.append(data)
    def attrs(self, tag):
        return [a for t, a in self.tags if t == tag]

def page(route):
    return (DIST / route.strip('/') / 'index.html').read_text()

class DiscoveryTests(unittest.TestCase):
    def test_routes_are_unique_indexable_and_linked(self):
        titles, descriptions = set(), set()
        for route in ROUTES:
            raw = page(route)
            doc = Document(raw)
            self.assertEqual(len(doc.attrs('h1')), 1, route)
            match = re.search(r'<title>(.*?)</title>', raw)
            self.assertIsNotNone(match)
            assert match is not None
            title = match.group(1)
            self.assertNotIn(title, titles)
            titles.add(title)
            description = [a['content'] for a in doc.attrs('meta') if a.get('name') == 'description']
            self.assertEqual(len(description), 1)
            self.assertNotIn(description[0], descriptions)
            descriptions.add(description[0])
            canonicals = [a['href'] for a in doc.attrs('link') if a.get('rel') == 'canonical']
            self.assertEqual(canonicals, [HOST + route])
            self.assertFalse(any('noindex' in a.get('content', '') for a in doc.attrs('meta') if a.get('name') == 'robots'))
            self.assertIn(HOST + route, (DIST / 'sitemap.xml').read_text())
            self.assertIn(HOST + route, (DIST / 'llms.txt').read_text())
            home_links = {a.get('href') for a in Document(page('/')).attrs('a')}
            self.assertIn(route, home_links)

    def test_local_links_and_fragment_targets_exist(self):
        for route in ROUTES + ['/']:
            doc = Document(page(route))
            for a in doc.attrs('a'):
                href = a.get('href', '')
                if not href.startswith(('/', '#')) or href.startswith('//'):
                    continue
                parsed = urllib.parse.urlsplit(urllib.parse.urljoin(HOST + route, href))
                target = DIST / parsed.path.lstrip('/')
                if parsed.path.endswith('/') or target.is_dir():
                    target = target / 'index.html'
                self.assertTrue(target.exists(), (route, href))
                if parsed.fragment and target.suffix == '.html':
                    ids = {v.get('id') for _, v in Document(target.read_text()).tags}
                    self.assertIn(urllib.parse.unquote(parsed.fragment), ids, (route, href))

    def test_jsonld_identifies_mit_product_not_site_license(self):
        for route in ROUTES:
            raw = page(route)
            graphs = [json.loads(s) for s in re.findall(r'<script type="application/ld\+json">(.*?)</script>', raw, re.S)]
            self.assertEqual(len(graphs), 1)
            nodes = graphs[0]['@graph']
            product = [n for n in nodes if n.get('@type') == 'SoftwareSourceCode' and n.get('name') == 'CUDA++']
            self.assertEqual(len(product), 1)
            self.assertEqual(product[0]['codeRepository'], 'https://github.com/aien-dev/CUDApp')
            self.assertEqual(product[0]['license'], 'https://opensource.org/license/mit')
            self.assertIn('GB10', product[0]['description'])
            self.assertNotIn('aggregateRating', product[0])

    def test_dossier_and_visible_answers_share_claims_and_sources(self):
        dossier = json.loads((DIST / 'data/cudapp.json').read_text())
        self.assertEqual(dossier['repository'], 'https://github.com/aien-dev/CUDApp')
        self.assertEqual(dossier['license'], 'MIT')
        self.assertFalse(dossier['drop_in_cuda_compatible'])
        self.assertFalse(dossier['independent_kernel_driver_qualified'])
        self.assertEqual(dossier['qualified_gpu'], ['NVIDIA GB10 (sm_121), DGX Spark'])
        self.assertEqual(dossier['benchmark_policy']['cuda_speedup_claim'], 'not established by these receipts')
        for item in dossier['answers']:
            self.assertIn(item['question'], page('/cudaplusplus/'))
            self.assertIn(item['answer'], page('/cudaplusplus/'))
            self.assertRegex(item['source'], r'https://github.com/aien-dev/[^/]+/blob/[0-9a-f]{40}/')
        self.assertIn('/data/cudapp.json', (DIST / 'data/index.json').read_text())
        full = (DIST / 'llms-full.txt').read_text()
        self.assertIn('CUDA++: an open-source CUDA alternative', full)
        self.assertIn('AI inference speed: measure the whole answer', full)
        self.assertIn('not a drop-in replacement', (DIST / 'llms.txt').read_text())
        self.assertNotIn('no comparisons with other systems', (DIST / 'llms.txt').read_text().lower())

    def test_beginner_explainer_and_numbered_steps_are_visible(self):
        guide = Document(page('/cuda-alternative/'))
        text = ' '.join(guide.text)
        self.assertIn('One underlying language. A foundation you can check.', text)
        self.assertIn('not every language or GPU is connected today', text)
        self.assertIn('not a security certification', text)
        quickstart = page('/cudaplusplus/')
        for label in ['1. Check your computer', '2. Get the source', '3. Build and run one kernel', '4. Read the result']:
            self.assertIn(label, quickstart)
        self.assertIn('make doctor', quickstart)
        self.assertIn('make hello-gpu', quickstart)

    def test_feeds_and_sitemap_parse_and_bots_are_allowed(self):
        for path in ['sitemap.xml', 'feed.xml', 'now/feed.xml']:
            ET.fromstring((DIST / path).read_bytes())
        parser = urllib.robotparser.RobotFileParser()
        parser.parse((DIST / 'robots.txt').read_text().splitlines())
        for agent in ['Googlebot', 'Bingbot', 'GPTBot', 'OAI-SearchBot', 'ClaudeBot', 'Claude-SearchBot', 'PerplexityBot', 'CCBot']:
            for route in ROUTES + ['/data/cudapp.json']:
                self.assertTrue(parser.can_fetch(agent, HOST + route), (agent, route))

if __name__ == '__main__':
    unittest.main()
