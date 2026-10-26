# AienOS Web Platform (aienos.com)

[![License: Apache-2.0 WITH LLVM-exception](https://img.shields.io/badge/License-Apache--2.0%20WITH%20LLVM--exception-blue.svg)](LICENSE)
[![GitHub Pages](https://github.com/aien-dev/aienos.com/actions/workflows/pages.yml/badge.svg)](https://github.com/aien-dev/aienos.com/actions/workflows/pages.yml)

The source of the AienOS website. A Rust program builds the pages, GitHub Pages serves their origin, and the existing nginx edge adds Markdown negotiation and the opt-in live Hive feed.

Part of [Aienix](https://github.com/aien-dev/aienix), the core tree of AIEN: start there for what the project is and how the pieces fit.

Live: [https://www.aienos.com](https://www.aienos.com), rebuilt hourly from the repository's own facts.

---

## What the site covers

The homepage opens with "AienOS is an owned experimental machine for turning search into verified understanding" (`crates/site-builder/src/components/home_loop.rs`) and goes on to the loop, the story, the receipts behind each claim, how the pieces fit together, the research, an interactive terminal, the current status of each layer, and where to follow the work. Other pages: `/start/` (the newcomer route through what AIENOS is, what exists today, how to check the evidence, how to reproduce or contribute, and where the author and Interplane fit), `/progress/`, `/blog/`, `/research/status/`, `/machine/`, `/discovery/` and the hubs `/experiments/`, `/evidence/` and `/philosophy/`. The build also writes `status.json`, `research-status.json` and `llms.txt`; `status.json` and `llms.txt` come from the same code as the homepage status section, and `research-status.json` is `crates/site-builder/data/research_status.json`. The homepage status section says "Public release" is PLANNED, with no supported release or installer yet.

## Introduce yourself to Allen: consent-first X pixel

Every built page gets one first-party `/js/allen-intro.js` loader from the final shared document pass, including hand-written pages, translations and the 404 page. Nothing from X is loaded until the visitor explicitly chooses **Introduce yourself to Allen** (or has a saved Yes). The unchanged official X base code configures pixel `rfrlz` once per document. **No** and Escape dismiss the native modal, remember refusal, and do not initialize `twq` or make an X tracking request. The browser tab stays open. `/privacy/` explains what X receives, normal hosting logs, browser-local storage, and how to refuse after consenting. It is readable without an automatic popup.

Run `node scripts/test-allen-intro.cjs` (Node's built-in test runner, verification only) alongside the Rust suite and `build dist`. To exercise the built asset use `node scripts/test-allen-intro.cjs dist/js/allen-intro.js`. Browser verification must observe requests before navigation with cache disabled; check absence of X requests and `window.twq` before consent, after No and after reload, then require the exact loader URL and `config('rfrlz')` after saying hello. Check keyboard focus, Escape and mobile width 390. An intercepted loader tests gating, not ingestion by X. Confirm the real deployed pixel separately with [X Pixel Helper](https://chrome.google.com/webstore/detail/twitter-pixel-helper/jepminnlebllinfmkhfbkpckogoiefpd).

No new CSP is installed by this feature. If an edge adds one, the Allen introduction needs `script-src 'self' https://static.ads-twitter.com`; `connect-src` and `img-src` need the first-party origin plus `https://analytics.twitter.com`, `https://analytics.x.com`, and `https://t.co`. The official loader inspected on 2026-10-26 names those endpoints; do not permit all X subdomains. CSP permissions cannot vary after a button click: the first-party script is the consent gate. A separate no-JS board should keep its strict CSP and not receive the allen-intro loader.

The community board, project links, leave-times and storage of public check-ins are **deferred**, pending Drake's storage decision. Saying hello here consents only to the pixel and publishes no public entry.

For consenting visitors only, `allen-intro.js` retains the landing URL's `twclid` in browser local storage (`aien-allen-intro-twclid`); No removes it. Drake chose pixel-only operation for now: the server Conversion API is deferred until the introduction flow has a backend. There is no conversion endpoint, server-side send or `X_PIXEL_TOKEN` in the site bundle.

The single `eventIds` constant in `allen-intro.js` contains `pageVisit` and `introducedToAllen`. Both stay empty until Drake supplies their real `tw-rfrlz-...` IDs; empty IDs emit no custom event and generate no conversion ID. Once configured, the client sends `twq('event', id, { conversion_id })` for one consented visit per document and one explicit introduced-to-Allen event per document. A returning saved Yes sends only the visit, not a new introduction. Each event gets its own `crypto.randomUUID()` conversion ID, reused by neither another event nor a second document. The future backend must receive and forward that exact ID for the matching server event, not generate a replacement.

When the backend exists, its Conversion API destination is `POST https://ads-api.x.com/12/measurement/conversions/rfrlz`, with `X-Pixel-Token` read only from private server environment `X_PIXEL_TOKEN`. Match with `twclid`, or the IP address plus user agent pair, or SHA-256 hex of a trimmed/lowercased email or an E.164-normalized phone. Do not collect those identifiers or expose a token merely to prepare a future integration.

---

## Gandi.net DNS Record Configuration Reference

To bind `aienos.com` on Gandi.net to the live site:

| Type | Name | Target Value | TTL |
| :--- | :--- | :--- | :--- |
| `A` | `@` | `185.199.108.153` | 1800 |
| `A` | `@` | `185.199.109.153` | 1800 |
| `A` | `@` | `185.199.110.153` | 1800 |
| `A` | `@` | `185.199.111.153` | 1800 |
| `AAAA` | `@` | `2606:50c0:8000::153` | 1800 |
| `AAAA` | `@` | `2606:50c0:8001::153` | 1800 |
| `AAAA` | `@` | `2606:50c0:8002::153` | 1800 |
| `AAAA` | `@` | `2606:50c0:8003::153` | 1800 |
| `A` | `www` | hub public IPv4 (see live DNS) | 300 |

`www` terminates on the hub nginx edge, which proxies GitHub Pages as the origin. The edge exists because GitHub Pages cannot negotiate `Accept: text/markdown` or emit a `Link` header on a `.md` response. The apex still points at GitHub Pages and canonicalises to `www`.

Rollback: delete the `www` A record and restore `CNAME www aien-dev.github.io.` at TTL 1800. The site then returns to GitHub Pages directly. The hub configuration can stay in place, unused.

---

## Installer script

`public/install.sh` is published at `https://aienos.com/install.sh`. It is the Sovereign Intelligence bootstrap (built on AienOS) for an NVIDIA DGX Spark (GB10), byte-identical to the signed copy in the [alpha-20261026.2 release](https://github.com/aien-dev/sovereign-intelligence/releases/tag/alpha-20261026.2) of aien-dev/sovereign-intelligence (sha256 `b3ba6a3938f418a575c605f8314f62f1aff58de2278509fa23b7266fbb82c93b`, listed in that release's signed SHA256SUMS). It checks the platform first, then downloads that release, checks its Ed25519 signature and SHA-256 before unpacking, and installs into `~/.local/share/aienos`. That release offers one model, Qwen3.6-35B-A3B-FP8. Its source is `installer/install.sh` in aien-dev/sovereign-intelligence; change it there and copy it here, never edit this copy alone. The /alpha/ page links to it.

```sh
curl -fsSL https://aienos.com/install.sh | sh
```

---

## Local Development & Build

There is no Node build and no `package.json`. The site is rendered by the Rust program in `crates/site-builder`, which copies `public/` into the output directory. Run these from the repository root:

```bash
cargo test --manifest-path crates/site-builder/Cargo.toml
cargo run --release --manifest-path crates/site-builder/Cargo.toml -- build dist
cargo run --release --manifest-path crates/site-builder/Cargo.toml -- verify dist
```

`build dist` writes the site to `dist/` and then verifies it; `verify dist` re-checks an existing output directory. There is no dev server or preview command. `.github/workflows/pages.yml` runs the same steps (plus the progress collection described below) and deploys `dist/` to GitHub Pages on pushes to `main`.

## Agent surface

The build also writes the files AI agents probe, all from the same page registry in `crates/site-builder/src/seo.rs`:

- `/llms.txt` and `/llms-full.txt`: the site index and the full text.
- `/sitemap.md`: the readable twin of `sitemap.xml`, H2 sections of described links.
- `/AGENTS.md`: the repository brief for coding agents (`AGENTS.md` at the repository root is published unchanged). It carries Installation, Configuration and Usage with executable commands.
- A Markdown mirror for every page at the same path with `.md`, so `/` is `/index.md` and `/machine/` is `/machine.md`. Each mirror carries YAML frontmatter (`title`, `description`, `last_updated`, `canonical_url`), the page body converted from the final HTML, and a `## Sitemap` section.
- Head signals on every page: `<link rel="alternate" type="text/markdown">`, `<link rel="describedby">` to `/llms.txt`, and robots directives for `max-snippet:-1` and `max-image-preview:large`.
- `/glossary/`: the terms the project uses in its own sense, linked from the footer of every page.

Search and security discovery:
- `/.well-known/security.txt`: the RFC 9116 security contact.
- `/61088fe61cf5283afad8253cec1087ab.txt`: the IndexNow key for Bing and Yandex. The file name is the key. After a deploy, submit changed URLs to `https://api.indexnow.org/indexnow` with that key. The key is public by design.

GitHub Pages serves static files only. It cannot vary a response on `Accept: text/markdown` and cannot add a `Link` header to a Markdown response, so `www.aienos.com` terminates on the hub nginx edge instead. The edge:

- proxies GitHub Pages as the origin (the four Pages IPs with `Host: www.aienos.com`), so every deploy still goes live without an edge change;
- rewrites a page path to its mirror when the request carries `Accept: text/markdown`;
- adds `Vary: Accept` and `Link: <canonical HTML>; rel="canonical"` on every `.md` response.

Configuration lives on the hub at `/etc/nginx/sites-available/aienos.com` (edge server) and `/etc/nginx/stream.d/sni-router.conf` (SNI map to `127.0.0.1:4445`). TLS is an acme.sh DNS-01 certificate for `www.aienos.com`, installed at `/etc/nginx/certs/www.aienos.com/`. The backup of the pre-edge SNI router is `sni-router.conf.bak-20261024T0127Z`, and the pre-cutover DNS record is `/root/aienos-edge-rollback-cname.json` on the hub.

## Hive and learning to build with AI

`/now/` links to [Hive](https://www.aienos.com/hive/), an animated orbital view of observed coding sessions. It shows nested agents, current task summaries, reported tool activity, exact runtime/provider/model/role, and a recent **Drake ↔ agents** conversation window. `/learn/` is the practical entry point: a bounded starter brief, ownership and delegation, checking real behavior, and learning from a correction. The homepage and `/start/` link both surfaces.

The page and assets are generated normally; `/hive/live.json` is not a build artifact or a GitHub receipt. The separate, explicitly started `hive-feed` collector observes OMP collaboration hosts and their incremental session records. Recognized live process-only workers remain visible with unknown task/activity when no reliable transcript association exists. Coverage is always disclosed as partial; neither a live process nor a fresh snapshot proves productive or successful work.

```sh
cargo test --workspace
cargo build --release --workspace
target/release/site-builder build dist
# Local opt-in preview; choose a publication cutoff you have authorized:
target/release/hive-feed --out dist/hive/live.json \
  --salt "$HOME/.local/state/aienos-hive/id-salt" \
  --since 2026-10-25T20:21:45.897Z
```

The collector samples every second; transport and browsers add latency. The page displays source and receipt timestamps separately, retains the last observation on failure, and expires freshness after 15 seconds. Missing agents become unknown, not completed. A quiet, still-observed long tool remains running. Pause motion and device reduced-motion preferences stop animation, not data updates. Background and offscreen decoration is paused.

### Public data boundary

- Agent and interaction IDs are HMAC-derived using a private persistent salt. Runtime IDs, PIDs, session paths and control links are never emitted.
- Task titles, short `solutionSpace` assignment summaries and action intents are explicitly selected fields, not raw tool payloads.
- Drake messages require direct-user attribution on a root session. Agent replies use only public text blocks; their model/provider identity is recorded from that message, not the agent's later model selection.
- Conversation publication starts at the authorized `--since` timestamp. The most recent 100 accepted messages are retained, up to 4,000 Unicode characters each. This is a live window, not a complete historical transcript.
- Full source strings are screened before truncation. Detected credential/key material or injected context withholds the entire string. Private paths and addresses are redacted. Thinking blocks, system prompts, tool arguments/results, provider payloads and raw operational errors are not public fields.
- Filtering is heuristic: arbitrary unlabeled short secrets cannot be guaranteed detectable. Do not put credentials into conversations. Publication cannot be recalled from visitors or scrapers after disclosure.

The JSON v1 envelope contains `schema_version`, `generated_at`, `sequence`, `poll_ms`, `stale_after_ms`, `coverage`, `agents` and `interactions`. Each agent has opaque `id`/`parent_id`, `name`, `runtime`, `provider`, `model`, `role`, `task`, `status`, `activity`, `activity_state`, `started_at`, `updated_at` and `observed_at`. Each interaction has `id`, `agent_id`, `role` (`drake` or `assistant`), `text`, `at` and message-specific `runtime`/`provider`/`model`. Missing identity is `unknown`; missing timestamps are null. Browser feed text is rendered as text nodes, never HTML.

### Operating the live feed

Templates are in `scripts/hive/`. Install the three release binaries under `$HOME/.local/lib/aienos-hive/bin/` on the collector host; the receiver alone also goes to `/usr/local/libexec/hive-receive` on the edge. Keep keys, the HMAC salt, `publication.env` (`HIVE_SINCE=...`, `HIVE_EDGE_HOST=...`, `HIVE_EDGE_USER=...`) and the pinned SSH `known_hosts` in the private `$HOME/.local/state/aienos-hive/` directory; the edge target never appears in a tracked file. The two user units are opt-in: `hive-feed.service` collects, `hive-publish.service` uploads. Enable them only after reviewing the cutoff and local filtered snapshot.

Uploads use a dedicated SSH key with `restrict,command="/usr/local/libexec/hive-receive /var/www/aienos-hive"` in the edge's authorized keys. The receiver refuses any original command other than `hive-feed`, bounds each newline-delimited snapshot to 2 MiB, validates the exact schema and hierarchy, refuses a `generated_at` older than the published one so a delayed upload cannot overwrite newer observations, serializes only the checked object, and atomically replaces the fixed `live.json`. A partial, stale, or invalid upload leaves the preceding snapshot intact. No public listener is opened on the collector host.

Include `scripts/hive/nginx-location.conf` inside the existing `www.aienos.com` TLS server, validate with `nginx -t`, then reload. It exposes only the exact GET/HEAD `/hive/live.json` path with `no-store` and security headers; there is no directory, upload, or agent-control route. The dynamic endpoint is linked from Hive rather than put into the static builder's file-existence registry.

To stop publication, stop both user units and remove the edge's `live.json`; remove the exact nginx include and restricted upload key to retire the transport. Do not stop or restart agent sessions or model services. To inspect operation, use `systemctl --user status hive-feed hive-publish` and request the public endpoint; compare `generated_at` across responses rather than trusting a successful HTTP status alone.


## Public coordination ledger boundary

The Spark's private `~/handoffs/LEDGER.tsv` is not a deployable site asset. `site-builder ledger-project INPUT` screens the complete TSV before writing any output: it withholds credentials and email-shaped identifiers, redacts private paths, network addresses, local ports, process IDs and UUID-shaped identifiers, and preserves rows and line numbers. `site-builder ledger-check FILE` refuses anything that is not already that public projection. This is a heuristic screen, not permission to put secrets in the source ledger.

The cron publisher must run both commands into a temporary `now.tsv.new` before replacing or staging `data/now/now.tsv`. On refusal it keeps the last published file and logs only the line and category, not the private value. The Pages workflow checks the laid-in file before build, and the builder checks it before copying assets; rejecting at Pages alone cannot remove a leak already committed to the public data branch. Install the matching reviewed `site-builder` binary before atomically installing any publisher change. Test the publisher with a temporary state and a deliberately refusing projector; never test against its live state. If the private ledger carries a credential-shaped entry, publication remains withheld until that entry is corrected by its owner.

Previously published `data/now` commits remain in Git history and may have been cached by readers. This patch does not rewrite that evidence or certify historical privacy. The operator must decide any historical disclosure response and which classes of operational notes should be public at all.

## License

Licensed under the Apache License, Version 2.0 with LLVM Exceptions. See [LICENSE](LICENSE) and [NOTICE](NOTICE). Project values live in the nonbinding [COVENANT.md](https://github.com/aien-dev/aien-sovereign-core/blob/main/COVENANT.md), which grants and restricts no legal rights.

## Project progress dashboard

[Progress](https://www.aienos.com/progress/) is built by the existing Rust generator. It plots daily default-branch snapshots from the beginning of the retained public project history, exact-content-deduplicated physical source lines, named test-file footprint, distinct reachable commit SHAs, all PRs and available GitHub Actions runs. These metrics measure repository activity, not qualification or individual executed tests.

The Pages workflow collects on pushes to main or data/now, manual runs, and hourly at minute 17 (UTC). GitHub can delay scheduled runs. The browser checks the published JSON every five minutes, preserves a selected historical day and marks snapshots older than three hours. If progress collection times out or fails, the workflow keeps the committed `project-progress.json` and can still deploy changed pages; that fallback is not a fresh reading and does not retain the previous deployment as a whole. Collection uses the built-in read-only Actions token against public endpoints; no token is shipped to browsers and no generated hourly commits are made. Pull requests do not get a hosted Pages build; local builds use the committed snapshot until they explicitly recollect it.

```bash
cargo run --release --manifest-path crates/site-builder/Cargo.toml -- progress public/data/project-progress.json
cargo test --manifest-path crates/site-builder/Cargo.toml
cargo run --release --manifest-path crates/site-builder/Cargo.toml -- build dist
```

Source boundaries, exclusions and the initial public repository allowlist are in crates/site-builder/src/collector.rs (`site-builder progress [out.json] [--asof T] [--cache DIR] [--no-fetch]`; it reads public data through `gh api` and mirror clones made with `git`, and keeps the curated events and policy text of the existing file). Daily boundaries are America/Chicago with US daylight saving rules written out in the same file. Newly discovered project-named public repositories are included automatically; forks/private repositories are excluded. The JSON includes the collection timestamp and every measured default-branch head. Deleted CI records cannot be reconstructed. Historical milestones and experiment examples are curated and linked to evidence; recent PR activity is collected automatically. D3 7.9.0 is self-hosted with its ISC license in public/vendor/.

### Crumb coordination

This integration adopts [RFC-0001](https://github.com/aien-dev/crumb-spec/blob/10b8251/SPEC.md), following the [current protocol pointer](https://github.com/aien-dev/aien-architecture/blob/main/docs/CRUMB_PROTOCOL.md). Durable .crumb files describe the changed directory boundaries. Using the canonical aien-architecture/tools/crumb tool: sniff before edits, respect unexpired locks, claim multi-step work, then close with the result. .crumb.local is ephemeral, ignored and excluded from the public site. Adoption started after the initial dashboard draft; the coordination record does not claim earlier edits were locked.
