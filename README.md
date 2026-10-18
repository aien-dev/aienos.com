# aienos.com

[![License: AGPL-3.0-or-later](https://img.shields.io/badge/License-AGPL--3.0--or--later-blue.svg)](LICENSE)
[![GitHub Pages](https://github.com/aien-dev/aienos.com/actions/workflows/pages.yml/badge.svg)](https://github.com/aien-dev/aienos.com/actions/workflows/pages.yml)

The source of the AIENOS website. A Rust program reads the public GitHub record, renders the pages, checks them, and GitHub Pages serves the result.

Part of [Aienix](https://github.com/aien-dev/aienix), the core tree of AIEN. Start there to see what the project is and how the pieces fit.

**Status (2026-10-18):** live at [www.aienos.com](https://www.aienos.com) and rebuilt every hour. The site describes a pre-alpha project: its own status section says there is no supported release or installer yet.

---

## What the site covers

The homepage opens with "AIENOS is an owned experimental machine for turning search into verified understanding" (`crates/site-builder/src/components/home_loop.rs`) and goes on to the loop, the story, the receipts behind each claim, how the pieces fit together, the research, an interactive terminal, the live status of each core component, and where to follow the work.

Pages the generator writes: `/`, `/progress/`, `/blog/`, `/research/status/`, `/machine/`, `/discovery/` and the hubs `/experiments/`, `/evidence/` and `/philosophy/`. Hand-kept pages under `public/` are copied as they are: `/turing/`, `/research/` (papers), `/post-llm-case/` and `/licensing/`.

The build also writes `facts.json`, `status.json`, `research-status.json` and `llms.txt`.

## How the site is built

There is no Node build and no `package.json`. Everything is the Rust program in `crates/site-builder`. Run these from the repository root:

```bash
cargo test --manifest-path crates/site-builder/Cargo.toml
cargo run --release --manifest-path crates/site-builder/Cargo.toml -- facts facts.json
cargo run --release --manifest-path crates/site-builder/Cargo.toml -- build dist
cargo run --release --manifest-path crates/site-builder/Cargo.toml -- verify dist
```

- `facts` reads the public GitHub record through the `gh` command line tool and writes `facts.json`: each core component's repository, open pull requests and last merge, recent decision records, and the latest verdict the Spark judge posted as a commit status. Nothing on the page that can change is typed by hand (`crates/site-builder/src/facts.rs`).
- `build dist` renders the site into `dist/` from `facts.json` and `public/`, then verifies it.
- `verify dist` re-checks an existing output directory.

There is no dev server or preview command.

## How it is deployed

`.github/workflows/pages.yml` runs on pushes to `main`, on manual runs, and hourly at minute 17 (UTC). It collects project progress, refreshes `facts.json`, runs the tests, builds `dist/` and deploys it to GitHub Pages. The refreshed files are published with the site; no hourly commits are made to the repository. Pull request builds use the committed snapshot.

`.github/workflows/automerge.yml` only runs when started by hand for a named branch.

## Project progress dashboard

[Progress](https://www.aienos.com/progress/) plots daily default-branch snapshots from the beginning of the retained public project history: exact-content-deduplicated physical source lines, named test-file footprint, distinct reachable commit SHAs, all pull requests and available GitHub Actions runs. These metrics measure repository activity, not qualification or individual executed tests.

```bash
cargo run --release --manifest-path crates/site-builder/Cargo.toml -- progress public/data/project-progress.json
```

Source boundaries, exclusions and the public repository list are in `crates/site-builder/src/collector.rs` (`site-builder progress [out.json] [--asof T] [--cache DIR] [--no-fetch]`). It reads public data through `gh api` and mirror clones made with `git`, and keeps the curated events and policy text of the existing file. Daily boundaries are America/Chicago with US daylight saving rules written out in the same file. Forks and private repositories are excluded. The JSON includes the collection timestamp and every measured default-branch head. Deleted CI records cannot be reconstructed. The browser checks the published JSON every five minutes and marks snapshots older than three hours. Collection uses the built-in read-only Actions token against public endpoints; no token is shipped to browsers. D3 7.9.0 is self-hosted with its ISC license in `public/vendor/`.

## Installer script

`public/install.sh` is published at `https://aienos.com/install.sh`. It needs `git`, clones [aien-sovereign-core](https://github.com/aien-dev/aien-sovereign-core) (the Linux-hosted Rust runtime) into a temporary directory and runs that repository's own `install.sh`. The website says there is no supported release or installer yet, and no page links to this script.

## DNS records

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

## Crumb coordination

The repository follows the Crumb Protocol ([RFC-0001](https://github.com/aien-dev/crumb-spec/blob/10b8251/SPEC.md), [current protocol pointer](https://github.com/aien-dev/aien-architecture/blob/main/docs/CRUMB_PROTOCOL.md)). Each directory's `.crumb` file says what that directory is for. `.crumb.local` is temporary coordination state, ignored by git and excluded from the public site.

## License

GNU Affero General Public License v3.0 or later (AGPL-3.0-or-later). See [LICENSE](LICENSE). Project values live in the nonbinding [COVENANT.md](https://github.com/aien-dev/aien-sovereign-core/blob/main/COVENANT.md), which grants and restricts no legal rights.
