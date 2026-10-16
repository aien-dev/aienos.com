# aienos.com

The public front door of AIEN. Live at [www.aienos.com](https://www.aienos.com).

One static page, built from Rust source. It loads no fonts, scripts or trackers from anyone, and every figure on it that can change is read from the public GitHub record at build time and links back to it.

## How it stays current

The page has two kinds of content:

- **Editorial copy** (what AIEN is, principles, how a change gets in) lives in `crates/site-builder/src/components/` and changes only through a pull request.
- **Facts** (the latest verification receipt, each component's last merged change and open changes, the newest decision records) live in `facts.json`, written by the builder's `facts` subcommand from the GitHub API. The workflow refreshes them once a day and on every push, so nobody has to edit the site to keep it true.

If a figure cannot be resolved to a public record, it does not go on the page.

## Build

```sh
cargo run --release --manifest-path crates/site-builder/Cargo.toml -- facts facts.json   # needs gh, read-only
cargo run --release --manifest-path crates/site-builder/Cargo.toml -- build dist facts.json
cargo test --manifest-path crates/site-builder/Cargo.toml
```

`build` also verifies the output: no scripts, no outside fonts, no comparisons with other systems, no em or en dashes, and the receipt, components table and licence line present. A build that breaks an invariant does not deploy.

## Layout

```text
crates/site-builder/   the builder (Rust, maud)
  src/facts.rs         gathers facts.json from GitHub
  src/components/      hero + receipt, what, status table, how a change gets in, principles, run it, footer
  src/layouts/base.rs  the document shell and metadata
public/                static files copied as-is: stylesheet, favicon, install.sh, CNAME, og.png
facts.json             the latest gathered facts (committed so the build works offline)
.github/workflows/     pages.yml builds, verifies and deploys; runs daily
```

## Deploy

GitHub Pages from the `pages` workflow on `main`, custom domain `www.aienos.com`, HTTPS enforced. DNS is at the registrar: `www` is a CNAME to `aien-dev.github.io`, and the apex points at the GitHub Pages addresses.

## Licence

AGPL-3.0-or-later, like everything in AIEN. See [LICENSE](LICENSE).
