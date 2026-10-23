# CUDA++ discovery qualification

## Delivered surfaces

- CUDA++ leads the homepage, with a direct product link and a beginner entry point.
- `/cudaplusplus/`: public CUDApp identity, supported hardware and a four-step source-pinned first-kernel walkthrough, expected exits, shared FAQ and source links.
- `/cuda-alternative/`: plain-language Omega/CUDA++/AienOS/Aienux foundation explainer, scope and verification boundaries, replacement map, language interoperability as a goal, and a route to the first kernel.
- `/ai-inference-speed/`: TTFT, decode, aggregate throughput, memory, correctness and a matched-benchmark checklist. No invented comparative performance claim.
- `/data/cudapp.json`: product identity, MIT licence, source pin, supported hardware, dated dependency receipt, FAQ, quickstart, foundation scope and benchmark policy.
- Unique metadata, product JSON-LD, dedicated social cards, sitemap/feed registration, llms summary and full-text export.

## Source and review

Independent review: `d2e81b398cdc198b31cb9e2e250fcd0ed3932884`, from main `89683470c430829cef2bcc871dc90657fedac3ab`. See `independent-review.md`. No material correctness findings. One P3 log-whitespace finding was corrected without changing production code: the exact raw logs remain in gzip, SHA-256 bindings are in `raw-log-preservation.json`, and the display copies have trailing whitespace removed.

This branch carries and supersedes the existing CUDA++ page correction `site/cudapp-other-hardware@d3f98f87d8a13257f7d5af19691392894aa6c9c8`, the Pages failure/policy repair `fix/pages-failure-churn@9e40f7b85200e47c3f71aaab13c8b46258612636`, and dashboard `feat/now-renda-speedometers@d411e1e030f1c06c6863953cce02bbf432b74cb4`. Their features and checks are retained, not reimplemented. The workflow merge retained both the non-cancelling publication policy and dashboard data overlay.

## Executed checks

- Rust suite: 140 passed, none failed or skipped.
- Emitted-artifact discovery suite: 6 passed. Checks unique/indexable routes, local links/fragments, MIT product identity, visible/JSON FAQ agreement, readable full-text export, foundation explainer and numbered instructions, XML/feeds and robot permission.
- Existing metrics collector: 8 passed.
- Carried publication coalescing suite: 8 passed.
- Browser: homepage plus the three CUDA++/guide pages at 1440px and 390px; 8 page/viewport checks, no document overflow or JavaScript errors; interactive FAQ tested. `browser-checks.json`.
- Schema.org's live validator: all three product/guide JSON-LD snippets returned zero errors and zero warnings. `schema-validation.json`.
- Pinned source documents and primary definitions: all 13 requested URLs returned HTTP 200, redirects recorded. `source-links.json`.

### Final Lighthouse mobile audit

| Page | Performance | Accessibility | Best practices | SEO |
|---|---:|---:|---:|---:|
| CUDA alternative | 90 | 100 | 100 | 100 |
| AI inference speed | 94 | 100 | 100 | 100 |
| CUDA++ | 92 | 100 | 100 | 100 |

Raw Lighthouse JSON is retained in `lighthouse-*.json.gz`; timings, source head, exact scores and an inherited unscored brand-link accessible-name warning are in `lighthouse-summary.json`. A 100 SEO score is a technical audit result, not a ranking or indexing guarantee. Shared render-blocking CSS contributes to performance results; no best-run substitution was made.

## Data inputs and replay

The HTML/local-fragment/browser audits use the actual published `data/now` ledger, pulse and metrics overlay, matching the Pages workflow. These generated snapshots are not committed as new source facts; exact archived inputs are retained in `input-*.gz` with SHA-256 bindings in `deployment-inputs.json`. The repository's older checked-in sample ledger lacks newer item IDs and is not the production feed. For an artifact replay, fetch `data/now`, overlay its root `now.tsv`, `now-pulse.tsv` and `renda-metrics.json` under `public/data/`, then:

```sh
cargo test -q --manifest-path crates/site-builder/Cargo.toml
cargo run -q --release --manifest-path crates/site-builder/Cargo.toml -- build dist
python3 scripts/test_discovery.py
python3 scripts/test_renda_metrics.py
python3 tests/now_publish_test.py
```

The installed signed Mac site-qualification recipe separately runs its exact three commands on a clean final source head and records raw log hashes, commit and tree. Its detached signature is verified with the existing Spark deployment-pinned public key, never a key provided by this branch. Qualification receipts are published separately to avoid self-referential commit pins.

## Publication boundary

Source, testing, independent review and local intake are separate from a PR, merge or live Pages deployment. Renda remains the sole website publication owner. A READY line or local acknowledgement is not a claim that these pages are live. Keep exact-head qualification, full-diff desk review and fresh remote checks intact.
