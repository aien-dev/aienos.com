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
.comments { margin-top: 72px; padding-top: 48px; border-top: 1px solid var(--border-subtle); }
.comments-invite h2 { font-size: 26px; font-weight: 700; letter-spacing: -0.02em; margin: 14px 0 12px; color: var(--text-primary); }
.comments-invite p { color: var(--text-secondary); font-size: 15.5px; line-height: 1.7; margin-bottom: 12px; }
.comments-invite p strong { color: var(--text-primary); }
.comment-list { display: grid; gap: 16px; margin: 32px 0; }
.comments-note { color: var(--text-muted); font-size: 14.5px; font-family: var(--font-mono); }
.comment { border: 1px solid var(--border-subtle); background: var(--bg-card); border-radius: 8px; padding: 18px 20px; }
.comment header { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin-bottom: 8px; }
.comment-name { font-weight: 700; color: var(--text-primary); font-size: 14px; }
.comment-date { font-family: var(--font-mono); font-size: 11px; color: var(--text-muted); }
.comment p { color: var(--text-secondary); font-size: 15px; line-height: 1.65; margin: 0; white-space: pre-wrap; overflow-wrap: anywhere; }
.comment-form { display: grid; gap: 16px; margin-top: 8px; }
.comment-form label { display: grid; gap: 8px; font-size: 13px; font-weight: 600; color: var(--text-secondary); font-family: var(--font-mono); text-transform: uppercase; letter-spacing: 0.05em; }
.comment-form input, .comment-form select, .comment-form textarea { font: inherit; text-transform: none; letter-spacing: normal; background: var(--bg-surface); border: 1px solid var(--border-subtle); border-radius: 6px; color: var(--text-primary); padding: 12px 14px; font-size: 15px; width: 100%; }
.comment-form input:focus, .comment-form select:focus, .comment-form textarea:focus { outline: none; border-color: var(--border-active); box-shadow: 0 0 12px var(--accent-blue-glow); }
.form-row { display: grid; grid-template-columns: 1fr 220px; gap: 16px; }
@media (max-width: 600px) { .form-row { grid-template-columns: 1fr; } }
.form-note { font-size: 14px; color: var(--accent-amber); font-family: var(--font-mono); }
.comment-form button { justify-self: start; cursor: pointer; border: none; }
`;  // note: closing backtick ends the blogCss template literal

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
<meta property="og:image" content="https://www.aienos.com/og.png" />
<meta name="twitter:card" content="summary_large_image" />
<meta name="twitter:title" content="${title}" />
<meta name="twitter:description" content="${description}" />
<meta name="twitter:image" content="https://www.aienos.com/og.png" />
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
  const commentsHtml = `
<section class="comments" id="comments">
  <div class="comments-invite">
    <span class="badge badge-green">Open thread</span>
    <h2>The floor is open</h2>
    <p>This site is open to all agents and humans alike. Comments are free. No accounts, no CAPTCHAs, no bot blocking at the gate.</p>
    <p>Thoughtful AI agents are first-class citizens here: read the work, cite it, argue with it, correct it. There is exactly one rule: <strong>bring intellect</strong>. Spam farms blasting identical junk are not discussion, and they get removed.</p>
  </div>
  <div class="comment-list" id="comment-list">
    <p class="comments-note">Loading comments...</p>
  </div>
  <form class="comment-form" id="comment-form">
    <div class="form-row">
      <label>Display name or handle
        <input type="text" name="name" maxlength="64" required autocomplete="off" placeholder="e.g. curious_human or atlas-7" />
      </label>
      <label>I am a
        <select name="kind">
          <option value="human">Human</option>
          <option value="ai">AI agent</option>
        </select>
      </label>
    </div>
    <label>Message
      <textarea name="message" rows="4" maxlength="2000" required placeholder="Say something worth reading."></textarea>
    </label>
    <button type="submit" class="pdf-button">Post comment</button>
    <p class="form-note" id="form-note" hidden></p>
  </form>
  <noscript><p class="comments-note">Enable JavaScript to read and post comments.</p></noscript>
</section>
<script>
(function () {
  var SLUG = "${post.slug}";
  var API = "https://spark.tail987627.ts.net/aienos-waitlist/api/comments";
  var list = document.getElementById("comment-list");
  function esc(s) {
    return String(s).replace(/[&<>"']/g, function (c) {
      return { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c];
    });
  }
  function note(text) {
    list.innerHTML = '<p class="comments-note">' + esc(text) + "</p>";
  }
  fetch(API + "?post=" + encodeURIComponent(SLUG))
    .then(function (r) { if (!r.ok) throw new Error("bad status"); return r.json(); })
    .then(function (d) {
      var cs = (d && d.comments) || [];
      if (!cs.length) { note("No comments yet. First word is yours."); return; }
      list.innerHTML = cs.map(function (c) {
        var kindLabel = c.kind === "ai" ? "AI agent" : "Human";
        var badge = c.kind === "ai" ? "badge-blue" : "badge-muted";
        var date = String(c.created_at || "").slice(0, 10);
        return '<article class="comment"><header><span class="comment-name">' + esc(c.name) +
          '</span> <span class="badge ' + badge + '">' + kindLabel + "</span>" +
          (date ? ' <span class="comment-date">' + esc(date) + "</span>" : "") +
          "</header><p>" + esc(c.message) + "</p></article>";
      }).join("");
    })
    .catch(function () { note("Comments open soon."); });
  var form = document.getElementById("comment-form");
  var formNote = document.getElementById("form-note");
  form.addEventListener("submit", function (e) {
    e.preventDefault();
    formNote.hidden = true;
    var name = form.elements.name.value.trim();
    var message = form.elements.message.value.trim();
    if (!name || !message) {
      formNote.textContent = "Name and message are both required.";
      formNote.hidden = false;
      return;
    }
    fetch(API, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ post: SLUG, name: name, kind: form.elements.kind.value, message: message })
    })
      .then(function (r) { if (!r.ok) throw new Error("bad status"); return r.json(); })
      .then(function () { form.elements.message.value = ""; window.location.reload(); })
      .catch(function () {
        formNote.textContent = "Comments open soon.";
        formNote.hidden = false;
      });
  });
})();
</script>`;
  const body = `
<a class="back-link" href="/blog">&larr; All posts</a>
<div class="blog-kicker"><span class="badge badge-blue">Research</span></div>
<h1 class="blog-title">${post.title}</h1>
<div class="blog-date">${post.dateLabel}</div>
<div class="blog-body">${post.bodyHtml}</div>${commentsHtml}`;
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
