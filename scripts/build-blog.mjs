// Static blog generator for aienos.com.
// Reads scripts/posts.mjs and src/index.css, writes fully self-contained HTML
// (inline CSS, zero JavaScript, zero external requests) to dist/blog/.
// Runs as the npm `postbuild` step so every deploy regenerates the pages.

import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { posts } from "./posts.mjs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const css = readFileSync(join(root, "src", "index.css"), "utf8");
const distBlog = join(root, "dist", "blog");

const SITE = "https://www.aienos.com";

const terminalIcon = `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--accent-blue)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="4 17 10 11 4 5"></polyline><line x1="12" y1="19" x2="20" y2="19"></line></svg>`;

const blogCss = `
.blog-wrap { max-width: 760px; margin: 0 auto; padding: 64px 24px 96px; }
.blog-kicker { margin-bottom: 16px; }
.blog-title { font-size: clamp(32px, 5vw, 48px); font-weight: 700; letter-spacing: -0.02em; line-height: 1.1; margin-bottom: 16px; }
.blog-date { font-family: var(--font-mono); font-size: 12px; color: var(--text-muted); letter-spacing: 0.06em; text-transform: uppercase; margin-bottom: 32px; }
.blog-body p { color: var(--text-secondary); font-size: 17px; line-height: 1.75; margin-bottom: 20px; }
.blog-body p.lead { color: var(--text-primary); font-size: 19px; }
.blog-body h2 { font-size: 24px; font-weight: 700; letter-spacing: -0.02em; margin: 44px 0 16px; color: var(--text-primary); }
.blog-body strong { color: var(--text-primary); }
.blog-body blockquote { border-left: 2px solid var(--accent-green); padding: 4px 0 4px 20px; margin: 28px 0; color: var(--text-primary); font-size: 18px; font-style: italic; }
.pdf-callout { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 20px; border: 1px solid var(--accent-blue); border-radius: 8px; background: var(--accent-blue-glow); padding: 24px; margin: 36px 0; }
.pdf-callout-title { font-weight: 700; font-size: 17px; color: var(--text-primary); margin-bottom: 4px; }
.pdf-callout-sub { font-size: 13px; color: var(--text-secondary); font-family: var(--font-mono); }
.pdf-button { display: inline-block; background: var(--accent-blue); color: #fff; font-weight: 700; font-size: 15px; padding: 12px 22px; border-radius: 6px; white-space: nowrap; }
.pdf-button:hover { filter: brightness(1.12); }
.post-card { display: block; border: 1px solid var(--border-subtle); background: var(--bg-card); border-radius: 8px; padding: 28px; transition: border-color 0.2s ease, box-shadow 0.2s ease; }
.post-card:hover { border-color: var(--border-active); box-shadow: 0 0 24px var(--accent-blue-glow); }
.post-card h2 { font-size: 24px; font-weight: 700; letter-spacing: -0.02em; color: var(--text-primary); margin: 12px 0 10px; }
.post-card p { color: var(--text-secondary); font-size: 15px; line-height: 1.65; }
.post-card .read-more { display: inline-block; margin-top: 14px; color: var(--accent-blue); font-weight: 600; font-size: 14px; }
.back-link { display: inline-block; margin-bottom: 32px; color: var(--text-secondary); font-size: 14px; font-weight: 500; }
.back-link:hover { color: var(--text-primary); }
.blog-header { border-bottom: 1px solid var(--border-subtle); background: rgba(8, 9, 12, 0.92); padding: 16px 0; }
.blog-header-inner { max-width: 1200px; margin: 0 auto; padding: 0 24px; display: flex; align-items: center; justify-content: space-between; }
.blog-brand { display: flex; align-items: center; gap: 10px; font-weight: 700; font-size: 17px; letter-spacing: -0.02em; color: var(--text-primary); }
.blog-nav { display: flex; align-items: center; gap: 24px; font-size: 14px; font-weight: 500; }
.blog-nav a { color: var(--text-secondary); }
.blog-nav a:hover { color: var(--text-primary); }
.blog-footer { border-top: 1px solid var(--border-subtle); padding: 32px 0; font-size: 13px; color: var(--text-muted); }
.blog-footer-inner { max-width: 1200px; margin: 0 auto; padding: 0 24px; display: flex; justify-content: space-between; flex-wrap: wrap; gap: 16px; }
.blog-footer a { color: var(--text-secondary); }
.blog-footer a:hover { color: var(--text-primary); }
`;

function page({ title, description, canonical, body }) {
  return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8" />
<meta name="viewport" content="width=device-width, initial-scale=1.0" />
<title>${title}</title>
<meta name="description" content="${description}" />
<meta name="theme-color" content="#08090c" />
<meta name="robots" content="index, follow" />
<link rel="canonical" href="${canonical}" />
<meta property="og:type" content="article" />
<meta property="og:site_name" content="AIENOS" />
<meta property="og:title" content="${title}" />
<meta property="og:description" content="${description}" />
<meta property="og:url" content="${canonical}" />
<style>${css}
${blogCss}</style>
</head>
<body>
<header class="blog-header">
  <div class="blog-header-inner">
    <a class="blog-brand" href="/" aria-label="AIENOS home">${terminalIcon}<span>AIENOS</span></a>
    <nav class="blog-nav" aria-label="Blog navigation">
      <a href="/">Home</a>
      <a href="/blog">Blog</a>
      <a href="https://github.com/aien-dev" target="_blank" rel="noopener noreferrer">GitHub</a>
    </nav>
  </div>
</header>
<main class="blog-wrap">
${body}
</main>
<footer class="blog-footer">
  <div class="blog-footer-inner">
    <span>AIENOS: a sovereign agent and inference runtime in native Rust.</span>
    <span><a href="https://github.com/aien-dev" target="_blank" rel="noopener noreferrer">GitHub</a> &nbsp;&middot;&nbsp; <a href="mailto:aien@aienos.com">aien@aienos.com</a></span>
  </div>
</footer>
</body>
</html>
`;
}

function writePost(post) {
  const body = `
<a class="back-link" href="/blog">&larr; All posts</a>
<div class="blog-kicker"><span class="badge badge-blue">Research</span></div>
<h1 class="blog-title">${post.title}</h1>
<div class="blog-date">${post.dateLabel}</div>
<div class="blog-body">${post.bodyHtml}</div>`;
  const html = page({
    title: `${post.title} | AIENOS Blog`,
    description: post.excerpt,
    canonical: `${SITE}/blog/${post.slug}`,
    body
  });
  const dir = join(distBlog, post.slug);
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "index.html"), html);
  console.log(`blog: wrote ${post.slug}/index.html`);
}

function writeIndex() {
  const cards = posts
    .map(
      (p) => `
<a class="post-card" href="/blog/${p.slug}">
  <span class="badge badge-muted">${p.dateLabel}</span>
  <h2>${p.title}</h2>
  <p>${p.excerpt}</p>
  <span class="read-more">Read the post &rarr;</span>
</a>`
    )
    .join("\n");
  const body = `
<div class="blog-kicker"><span class="badge badge-green">Blog</span></div>
<h1 class="blog-title">Notes from the build</h1>
<div class="blog-date">Research notes on AIEN, the sovereign machine lineage</div>
<div style="display: grid; gap: 20px;">${cards}</div>`;
  const html = page({
    title: "Blog | AIENOS",
    description:
      "Research notes on AIEN: building machine intelligence from a small, inspectable software lineage.",
    canonical: `${SITE}/blog`,
    body
  });
  mkdirSync(distBlog, { recursive: true });
  writeFileSync(join(distBlog, "index.html"), html);
  console.log("blog: wrote index.html");
}

writeIndex();
for (const post of posts) writePost(post);
console.log(`blog: done (${posts.length} post${posts.length === 1 ? "" : "s"})`);
