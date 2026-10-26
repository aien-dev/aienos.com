# Public Allen: host dependencies and their Omega successors

First-party code belongs in seeded Omega. Everything below that is not `.om`
is a recorded host dependency: what it does, why it is not `.om` yet, the `.om`
program that replaces it, and the Omega feature that blocks the replacement.
Host words for sockets, HTTP and JSON are proposed in aien-dev/omega-mv#788
(bounded TLS HTTP/JSON host primitives); file and process host words already
exist (OMEGESE.md section 10, native host extension).

| Program | Language, version | What it does | Why not `.om` yet | `.om` successor | Blocking Omega feature |
|---|---|---|---|---|---|
| `allen_chat.py` (gateway, `/opt/allen-chat`) | CPython 3.12.3, stdlib only | HTTP/1.1 server on the tailnet, SSE relay to 18091, rate limits, queue, launches the compile sandbox, writes spool and contact drops | Needs a listening socket, an HTTP client to 18091, threads or poll, JSON in and out | `allen-gw.om`: one-process poll loop, fixed-shape JSON for the six routes | Socket/accept/poll host words and HTTP framing (omega-mv#788); JSON parse/emit library |
| `allen_spool.py` (shared validation) | CPython 3.12.3, stdlib (`re`, `json`, `hashlib`) | Validates confirmed issues/projects (slug, filenames, sizes, text-only, secret shapes, external loads), canonical id, atomic spool write, status read | Regular expressions and JSON; used by both Python programs | `spool.om`: byte-level validators (no regex needed for these fixed rules), sha256 in Omega (existing sha256 core), fixed-schema JSON | JSON library; Unicode code point walk over UTF-8 (byte loops exist) |
| `catalog.py` (`/opt/allen-catalog`, timer) | CPython 3.12.3, stdlib | Builds the public catalog from the unauthenticated GitHub API, raw READMEs, `git ls-remote` and `seo.rs` | HTTPS client and JSON | `catalog.om` | TLS HTTP client and JSON (omega-mv#788) |
| `omega_projects_pusher.py` (`/opt/omega-projects-pusher`) | CPython 3.12.3, stdlib | Re-validates spool items, scrubs emails/phones, files one issue or one PR on aien-dev/omega-projects with a scoped token | HTTPS client to api.github.com and JSON | `pusher.om` | TLS HTTP client and JSON (omega-mv#788) |
| `allen_contacts_ingest.om` (`/opt/allen-contacts/allen-contacts-ingest`) | seeded Omega, omc sha256 `befb8199...8a1d` (omega-mv `3364a1df`) | Production contact ingest (introduce, forget, forget_visitor) | n/a: it is `.om`; byte-identical to the Python oracle on the shared fixtures (`contacts/PARITY-RECEIPT.log`) | done | Proposal: a `flock fd op` host word (the ingest and CLI lock with an atomic `makedir` of `.lock.d` instead) |
| `allen_contacts_ingest.py` | CPython 3.12.3 | Parity oracle and the CLI's library only | Verification oracle | n/a | n/a |
| `allen-contacts` CLI (`/usr/local/bin/allen-contacts`) | CPython 3.12.3, stdlib | Drake's list/show/delete over the private store; re-execs through `sudo -u allen-contacts` | Not ported yet; the production ingest moved first | `allen_contacts_cli.om`, sharing the ingest's parser, lock and rewrite | none blocking |
| `cortex-visitors` (`/opt/cortex-visitors/cortex-rs`, 127.0.0.1:18096) | Rust, cortex-rs 0.1.0 (Axum + SQLite FTS5), byte copy of `~/.local/bin/cortex-rs`, sha256 `91ccba9403601ed4d4b4c762dd489fd9b7de58d4b0017e44102005eb51398afe`, BuildID 8daee336b6e69c90cd07f08ef80369669c3c89db (source: archived aien-dev/cortex-rs, canonical aien-sovereign-core crates/cortex-rs) | Separate Cortex instance holding only public visitors' memories (space `v-<vid>`), own bearer token, loopback only, no embedding encoder (URL points at closed 127.0.0.1:9), started by `/opt/cortex-visitors/start` as user `cortex-visitors` (no systemd unit) | Cortex is being rewritten in Omega (aien-dev/cortex, agent CortexOmega); cortex-rs 0.1.0 has one token per instance and no hard delete, so isolation is a second process and forget = overwrite + retract + VACUUM on start | aien-dev/cortex (Omega Cortex) in parity mode, with per-space tokens and `delete`/`delete-space` write kinds | Socket/HTTP host words (omega-mv#788), JSON, an Omega storage/index layer replacing SQLite FTS5 |
| `allen_memory.py` (gateway module) | CPython 3.12.3, stdlib (`urllib`, `json`, `difflib`) | Visitor memory client: recall/remember (dedupe, cap 40)/forget against cortex-visitors | Imported by the Python gateway; HTTP client + JSON | part of `allen-gw.om` | TLS-less loopback HTTP client words and JSON (omega-mv#788) |
| sqlite3 CLI | 3.45.1 (Ubuntu) | `PRAGMA wal_checkpoint(TRUNCATE); VACUUM;` on every cortex-visitors start, so forgotten text leaves the DB file and WAL | cortex-rs has no compaction endpoint | dropped when Omega Cortex compacts on `delete` | none on our side; lands with aien-dev/cortex hard delete |
| `test_*.py`, `probe.py`, `dev_proxy.py` | CPython 3.12.3 | Verification and local preview only; never deployed in the request path | Verification oracles are exempt | n/a | n/a |
| bubblewrap | 0.9.0 (Ubuntu) | Namespace sandbox around omc and irwasm | Isolation tool around the compiler, not first-party logic | kept | n/a |
| nginx (hub) | 1.28.3 (Ubuntu) | TLS, PROXY protocol, rate limits, routing | Existing edge | kept for now | Omega TLS server words (omega-mv#788) |

The compiler `omc` (sha256 `734f0a39319fbfa930bcc62cb30dbf06d47d78e126372b74f5baf478e6126b61`)
and `irwasm` (sha256 `2d235a70c39ac1461a09c41e10e058d962b767ae83893cdcfc693ef6c47182e6`)
are seeded Omega. The page's JavaScript is generated by Omega (`om/allen-page/`).
