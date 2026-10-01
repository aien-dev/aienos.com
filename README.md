# AIENOS Web Platform (aienos.com)

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![GitHub Pages](https://github.com/aien-dev/aienos.com/actions/workflows/pages.yml/badge.svg)](https://github.com/aien-dev/aienos.com/actions/workflows/pages.yml)

The official web platform and documentation portal for **AIENOS**, the GPU-Native Neural Operating Environment.

Live: [https://www.aienos.com](https://www.aienos.com)

---

## What the site covers

The site presents [aien-sovereign-core](https://github.com/aien-dev/aien-sovereign-core), a sovereign agent and inference runtime in native Rust with Modular MAX bridges, built on NVIDIA DGX Spark (Grace Blackwell GB10). Site copy tracks the core README: the architecture section lists what is implemented today, and the results section follows the evidence standard, where withdrawn figures stay off the page until their artifact bundles are regenerated.

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

## Universal Developer Installer

```bash
curl -fsSL https://aienos.com/install.sh | bash
```

---

## Local Development & Build

```bash
npm install
npm run dev
npm run build
npm run preview
```

## Rust waitlist API

The waitlist form posts to the Rust API in [`server/`](server/). It stores
unique email addresses, a consent version, and signup time in SQLite. The API
accepts browser requests only from `aienos.com` and `www.aienos.com`; the
database is never exposed by the static website.

```bash
cargo test --manifest-path server/Cargo.toml
bash server/deploy-local.sh
```

The deployment script builds a Rust release binary and starts a container on
`127.0.0.1:18171` with persistent data in
`~/.local/share/aienos-waitlist`. The GitHub Pages form uses
`https://spark.tail987627.ts.net/aienos-waitlist/api/signup` by default;
`VITE_WAITLIST_ENDPOINT` can override it at build time. The public Tailscale
Funnel route must forward `/aienos-waitlist` to the local listener.

---

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE). The runtime itself, aien-sovereign-core, is Apache-2.0 with LLVM Exception. Project values live in the nonbinding [COVENANT.md](https://github.com/aien-dev/aien-sovereign-core/blob/main/COVENANT.md), which grants and restricts no legal rights.

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
