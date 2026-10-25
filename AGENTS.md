# AienOS website (aienos.com)

> Static site for aienos.com. One Rust program, `crates/site-builder`, renders every page from Maud templates, copies hand-written files from `public/`, then rewrites every head and writes the machine surface. GitHub Pages serves the output; `.github/workflows/pages.yml` builds and deploys it on every push to `main`.

Shared rules: read https://www.aienos.com/agents/RULES.md first. This file adds what is specific to this repository; where the two differ, this file wins here. The rules live in `public/agents/RULES.md`; every public repository is listed at https://www.aienos.com/agents/ (`components/agents.rs`, data refreshed by `site-builder agents`). The catalog follows every GitHub API page, includes archived repositories as read-only references, and links the actual `AGENTS.md` or `CLAUDE.md` when present. Missing instructions are marked explicitly. AienOS is the whole system at its current `aien-dev/aienix` URL until renamed; Aienux is the separate kernel.

## Scope

- Applies to the whole repository.
- Source of truth is `crates/site-builder/src` plus `public/`. The root `index.html` and `dist/` are not the live site.
- `dist/` is generated output. Never edit it by hand; rebuild it.
- `crates/hive-feed` is opt-in Rust website telemetry tooling. It does not run during static builds. `scripts/hive/` holds its service and exact-route edge templates; README documents installation and the publication boundary.

## Installation

```bash
git clone https://github.com/aien-dev/aienos.com.git
cd aienos.com
rustup toolchain install stable
cargo build --release --manifest-path crates/site-builder/Cargo.toml
```

There is no Node build, no `package.json`, and no package manager. The only tool needed is a Rust toolchain; the site depends on `maud`, `serde` and `serde_json`.

## Configuration

- `facts.json` at the repository root is read by every build. Refresh it from the public GitHub record when the fact sheet must move:

```bash
GH_TOKEN="$(gh auth token)" cargo run --release --manifest-path crates/site-builder/Cargo.toml -- facts facts.json
```

- `public/data/project-progress.json` feeds `/progress/`. Recollect it with:

```bash
GH_TOKEN="$(gh auth token)" cargo run --release --manifest-path crates/site-builder/Cargo.toml -- progress public/data/project-progress.json
```

- `CNAME` holds the custom domain. `.github/workflows/pages.yml` is the deployment configuration.
- `crates/site-builder/data/research_status.json` is the canonical research ladder and experiment verdicts.
- The page registry in `crates/site-builder/src/seo.rs` (`seo::PAGES`) drives the sitemap, `llms.txt`, `sitemap.md`, the feeds and the JSON-LD. Add every new page there or the build fails.

## Usage

```bash
# Full test suite
cargo test --workspace

# Build the site into dist/ (this also runs the integrity verifier)
cargo run --release --manifest-path crates/site-builder/Cargo.toml -- build dist

# Re-check an existing dist/ without rebuilding
cargo run --release --manifest-path crates/site-builder/Cargo.toml -- verify dist

# Preview the generated output locally
python3 -m http.server 8080 --directory dist
```

Open `http://127.0.0.1:8080/` after the preview server starts. GitHub Pages serves directories through `index.html`, so local previews match the live routes.

## Conventions

- Generated pages are Maud markup in `crates/site-builder/src/components/`. Hand-written pages live under `public/<route>/index.html` and are copied verbatim.
- Every page gets its head rewritten by `seo::enrich`: canonical on `https://www.aienos.com`, Open Graph and Twitter cards, hreflang, JSON-LD, `rel="alternate" type="text/markdown"`, `rel="describedby"` to `/llms.txt`, and robots snippet directives. Do not duplicate those tags in a component.
- Every page also gets a Markdown mirror at the same path with `.md` (`/` becomes `/index.md`), with YAML frontmatter and a `## Sitemap` section. The mirror is generated from the final HTML; do not hand-write one.
- Copy rules: no em dashes, no en dashes, no outside fonts, scripts or trackers. The verifier fails the build on a dash in a page.
- Keep one source for machine-readable state. `status.json`, the status section, and the current-state lines in `llms.txt` all read `components::status::layer_states()`.
- Tests live beside the code in `#[cfg(test)]` modules. Add a test when a rule can regress silently.
- Hive's orbital scene is decorative; task/activity/lifecycle data must come from real observations. Preserve source freshness, partial coverage, unknown states, message-time model identity, keyboard focus, and reduced-motion support.
- Hive text passes a field whitelist and full-string privacy screening before truncation. Never serialize raw sessions, system/task prompts, tool payloads, hidden reasoning, credentials, or control metadata. Direct Drake messages, short task-summary fields and public agent replies require the authorized publication boundary. Render feed strings through text nodes, not HTML.

## Safety and approvals

- Ask before changing the canonical host, the page registry shape, or a published claim that carries a receipt.
- Never commit tokens, API keys, or `.env` files. `GH_TOKEN` is read from the environment and is never written into the site.
- Never publish `dist/`, `.crumb.local`, or private evidence. `.gitignore` and the copy step already exclude local coordination files.
- Do not run destructive git commands or rewrite published history. Work on a branch, run the full test suite and `build dist`, then merge through a pull request.

## Further reading

- `README.md`: the human-facing build and deployment guide.
- `crates/site-builder/src/seo.rs`: the page registry, head pass, sitemap, llms files and machine surface.
- `https://www.aienos.com/llms.txt`: the site as AI readers see it.
- `https://www.aienos.com/sitemap.md`: every page, grouped and described.
