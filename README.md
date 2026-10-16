# AIENOS Web Platform (aienos.com)

[![License: AGPL-3.0-or-later](https://img.shields.io/badge/License-AGPL--3.0--or--later-blue.svg)](LICENSE)
[![GitHub Pages](https://github.com/aien-dev/aienos.com/actions/workflows/pages.yml/badge.svg)](https://github.com/aien-dev/aienos.com/actions/workflows/pages.yml)

The source of the AIENOS website. It is a static site built by a Rust program and served by GitHub Pages.

Live: [https://www.aienos.com](https://www.aienos.com)

---

## What the site covers

The homepage opens with "AIENOS is an owned experimental machine for turning search into verified understanding" (`crates/site-builder/src/components/home_loop.rs`) and goes on to the loop, the story, the receipts behind each claim, how the pieces fit together, the research, an interactive terminal, the current status of each layer, and where to follow the work. Other pages: `/progress/`, `/blog/`, `/research/status/`, `/machine/`, `/discovery/` and the hubs `/experiments/`, `/evidence/` and `/philosophy/`. The build also writes `status.json`, `research-status.json` and `llms.txt`; `status.json` and `llms.txt` come from the same code as the homepage status section, and `research-status.json` is `crates/site-builder/data/research_status.json`. The homepage status section says "Public release" is PLANNED: pre-alpha, no supported release or installer yet.

---

## Gandi.net DNS Record Configuration Reference

To bind `aienos.com` on Gandi.net to GitHub Pages:

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
| `CNAME` | `www` | `aien-dev.github.io.` | 1800 |

---

## Installer script

`public/install.sh` is published at `https://aienos.com/install.sh`. It needs `git`, clones [aien-sovereign-core](https://github.com/aien-dev/aien-sovereign-core) (the earlier Linux-hosted Rust runtime) into a temporary directory and runs that repository's own `install.sh`. The website itself says there is no supported release or installer yet, and no page links to this script.

```bash
curl -fsSL https://aienos.com/install.sh | bash
```

---

## Local Development & Build

There is no Node build and no `package.json`. The site is rendered by the Rust program in `crates/site-builder`, which copies `public/` into the output directory. Run these from the repository root:

```bash
cargo test --manifest-path crates/site-builder/Cargo.toml
cargo run --release --manifest-path crates/site-builder/Cargo.toml -- build dist
cargo run --release --manifest-path crates/site-builder/Cargo.toml -- verify dist
```

`build dist` writes the site to `dist/` and then verifies it; `verify dist` re-checks an existing output directory. There is no dev server or preview command. `.github/workflows/pages.yml` runs the same steps (plus the Node progress collection described below) and deploys `dist/` to GitHub Pages on pushes to `main`.

## License

Licensed under the GNU Affero General Public License v3.0 or later (AGPL-3.0-or-later). See [LICENSE](LICENSE). The runtime itself, aien-sovereign-core, is also AGPL-3.0-or-later. Project values live in the nonbinding [COVENANT.md](https://github.com/aien-dev/aien-sovereign-core/blob/main/COVENANT.md), which grants and restricts no legal rights.

## Project progress dashboard

[Progress](https://www.aienos.com/progress/) is built by the existing Rust generator. It plots daily default-branch snapshots from the beginning of the retained public project history, exact-content-deduplicated physical source lines, named test-file footprint, distinct reachable commit SHAs, all PRs and available GitHub Actions runs. These metrics measure repository activity, not qualification or individual executed tests.

The Pages workflow collects on pushes to main, manual runs, and hourly at minute 17 (UTC). GitHub can delay scheduled runs. The browser checks the published JSON every five minutes, preserves a selected historical day and marks snapshots older than three hours. Collection and reconciliation must succeed before deployment; a failed refresh retains the last good live page. Collection uses the built-in read-only Actions token against public endpoints; no token is shipped to browsers and no generated hourly commits are made. PR builds use the committed snapshot.

```bash
node --test scripts/collect-progress.test.mjs
node scripts/collect-progress.mjs
cargo test --manifest-path crates/site-builder/Cargo.toml
cargo run --release --manifest-path crates/site-builder/Cargo.toml -- build dist
```

Source boundaries, exclusions and the initial public repository allowlist are in scripts/collect-progress.mjs. Newly discovered project-named public repositories are included automatically; forks/private repositories are excluded. The JSON includes the collection timestamp and every measured default-branch head. Deleted CI records cannot be reconstructed. Historical milestones and experiment examples are curated and linked to evidence; recent PR activity is collected automatically. D3 7.9.0 is self-hosted with its ISC license in public/vendor/.

### Crumb coordination

This integration adopts [RFC-0001](https://github.com/aien-dev/crumb-spec/blob/10b8251/SPEC.md), following the [current protocol pointer](https://github.com/aien-dev/aien-architecture/blob/main/docs/CRUMB_PROTOCOL.md). Durable .crumb files describe the changed directory boundaries. Using the canonical aien-architecture/tools/crumb tool: sniff before edits, respect unexpired locks, claim multi-step work, then close with the result. .crumb.local is ephemeral, ignored and excluded from the public site. Adoption started after the initial dashboard draft; the coordination record does not claim earlier edits were locked.
