#!/usr/bin/env python3
"""Read-only HTTP oracle for the pre-cutover sitemap and generated internal links.

This is verification tooling, never part of page generation. The original sitemap
is a captured independent input, not regenerated from the candidate registry.
"""
import argparse
from html.parser import HTMLParser
import json
from pathlib import Path
import re
import sys
import urllib.error
import urllib.parse
import urllib.request
import xml.etree.ElementTree as ET

CANONICAL = "https://www.aienos.com"
EDGE = {"/hive/board/", "/hive/live.json"}


class Document(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.links = []
        self.ids = set()
        self.canonical = None
        self.refresh = None

    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        if "id" in a:
            self.ids.add(a["id"])
        if tag == "a" and "name" in a:
            self.ids.add(a["name"])
        if tag == "link" and a.get("rel", "").lower() == "canonical":
            self.canonical = a.get("href")
        if tag == "meta" and a.get("http-equiv", "").lower() == "refresh":
            match = re.fullmatch(r"\s*\d+(?:\.\d+)?\s*;\s*url\s*=\s*(.*?)\s*", a.get("content", ""), re.I)
            if not match:
                raise ValueError("malformed meta refresh")
            self.refresh = match[1].strip("\"'")
        for key in ("href", "src", "poster"):
            if a.get(key):
                self.links.append(a[key])
        if a.get("srcset"):
            self.links.extend(item.strip().split()[0] for item in a["srcset"].split(",") if item.strip())


def same_site(url):
    return urllib.parse.urlsplit(url).hostname in {"aienos.com", "www.aienos.com"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True, help="Origin serving candidate output")
    parser.add_argument("--dist", required=True, type=Path)
    parser.add_argument("--old-sitemap", required=True, type=Path)
    parser.add_argument("--edge-base", help="Explicit existing edge origin for edge-only routes")
    parser.add_argument("--report", required=True, type=Path)
    args = parser.parse_args()
    base = args.base.rstrip("/")
    parsed = ET.parse(args.old_sitemap)
    old = [n.text for n in parsed.findall(".//{http://www.sitemaps.org/schemas/sitemap/0.9}loc")]
    if not old or any(not u or not same_site(u) for u in old) or len(old) != len(set(old)):
        parser.error("old sitemap must contain distinct canonical-site URLs")
    report = {"base": base, "old_sitemap": str(args.old_sitemap), "old_count": len(old), "old_urls": [], "links": [], "failures": []}
    cache = {}

    def fetch(path, fragment="", trail=()):
        key = path
        if key in trail or len(trail) >= 10:
            raise ValueError("redirect cycle or excessive redirects: " + path)
        if key not in cache:
            origin = args.edge_base.rstrip("/") if path.split("?", 1)[0] in EDGE and args.edge_base else base
            url = origin + path
            request = urllib.request.Request(url, headers={"User-Agent": "aienos-cutover-readonly-oracle", "Cache-Control": "no-cache"})
            with urllib.request.urlopen(request, timeout=30) as response:
                status = response.status
                final = response.url
                if status != 200:
                    raise ValueError(f"{url}: HTTP {status}")
                if urllib.parse.urlsplit(final).netloc != urllib.parse.urlsplit(origin).netloc:
                    raise ValueError(f"unexpected cross-origin HTTP redirect: {final}")
                body = response.read()
                mime = response.headers.get_content_type()
            doc = None
            if mime == "text/html":
                doc = Document()
                doc.feed(body.decode("utf-8"))
                doc.close()
            cache[key] = {"path": path, "status": status, "final": final, "bytes": len(body), "document": doc}
        row = cache[key]
        doc = row["document"]
        if doc and doc.refresh:
            refresh = urllib.parse.urljoin(CANONICAL + path, doc.refresh)
            canonical = urllib.parse.urljoin(CANONICAL + path, doc.canonical or "")
            if not doc.canonical or canonical != refresh or not same_site(refresh):
                raise ValueError(f"{path}: refresh lacks matching same-site canonical")
            dest = urllib.parse.urlsplit(refresh)
            fetch(dest.path + ("?" + dest.query if dest.query else ""), dest.fragment, trail + (key,))
        elif fragment and doc and fragment not in doc.ids:
            raise ValueError(f"{path}: missing fragment #{fragment}")
        return {k: v for k, v in row.items() if k != "document"}

    for url in old:
        p = urllib.parse.urlsplit(url)
        try:
            report["old_urls"].append(fetch(p.path, p.fragment))
        except (ValueError, OSError, UnicodeError) as exc:
            report["failures"].append({"source": "old-sitemap", "url": url, "error": str(exc)})

    html_files = sorted(args.dist.rglob("*.html"))
    if not html_files:
        report["failures"].append({"source": "candidate", "error": "no generated HTML"})
    seen = set()
    for file in html_files:
        rel = file.relative_to(args.dist).as_posix()
        route = "/" + (rel[:-10] if rel.endswith("index.html") else rel)
        doc = Document()
        try:
            doc.feed(file.read_text(encoding="utf-8"))
            doc.close()
        except (ValueError, UnicodeError) as exc:
            report["failures"].append({"source": route, "error": str(exc)})
            continue
        for href in doc.links:
            dest = urllib.parse.urlsplit(urllib.parse.urljoin(CANONICAL + route, href))
            if dest.scheme not in {"http", "https"} or not same_site(dest.geturl()):
                continue
            # APIs are runtime services, not static document links or resources.
            if dest.path.startswith("/allen/api/"):
                continue
            path = dest.path + ("?" + dest.query if dest.query else "")
            key = (path, dest.fragment)
            if key in seen:
                continue
            seen.add(key)
            try:
                report["links"].append(fetch(path, dest.fragment))
            except (ValueError, OSError, UnicodeError) as exc:
                report["failures"].append({"source": route, "url": href, "error": str(exc)})
    report["checked_old"] = len(report["old_urls"])
    report["checked_internal"] = len(report["links"])
    report["failure_count"] = len(report["failures"])
    report["verdict"] = "FAIL" if report["failures"] else "PASS"
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"SITE-ROUTES {report['verdict']} old={report['checked_old']}/{report['old_count']} internal={report['checked_internal']} failures={report['failure_count']}")
    return 1 if report["failures"] else 0


if __name__ == "__main__":
    sys.exit(main())
