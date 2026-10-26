# Security policy

## Reporting a vulnerability

Report privately. Do not open a public issue, pull request or discussion about it.

1. GitHub: open this repository's Security tab and choose "Report a vulnerability". This
   creates a private advisory that only the maintainers can see. If that option is not shown,
   private vulnerability reporting has not been enabled for this repository yet; use email.
2. Email: drake@aienos.com with the subject `[aienos.com] security report`.

Include the commit or release you tested, the component (path), steps to reproduce, and what an
attacker gains. If the report involves a secret, say where it was exposed and do not send the
secret itself. Encrypted mail is not required; do not post proof-of-concept payloads publicly.

## Response targets

This project has one maintainer; the targets below are best effort and stated honestly.

- Acknowledgement: within 3 working days.
- Triage and severity: within 7 days of acknowledgement.
- Fix or mitigation for confirmed issues: aim for 30 days for high and critical, 90 days
  otherwise, with a status update at least every 14 days until closed.
- Disclosure: coordinated. An advisory is published with credit (unless you ask otherwise)
  once the fix is on the default branch. Default embargo 90 days, negotiable.

## Scope

In scope:

- Cross-site scripting or markup injection in any generated page, the Markdown mirrors, or the
  Hive board renderer (`om/hive-render/render.om`), including feed strings that are not escaped.
- Private data reaching the public site: the `/now/` ledger projection (`site-builder
  ledger-project` and `ledger-check`), `/dash/` account data, and Hive feed privacy screening
  (`crates/hive-feed/src/privacy.rs`).
- The consent gate in `public/js/allen-intro.js`: any X request or `window.twq` before explicit
  consent, or after refusal.
- `public/install.sh` differing from the signed copy in the release it names.
- `.github/workflows/pages.yml` and `generated-hive.yml`: token scope, deploy conditions.
- Committed binaries under `om/` whose sha256 differs from the recorded digest.

Out of scope: GitHub Pages and the DNS provider; the X pixel service itself; the nginx edge host
configuration beyond the templates in `scripts/hive/`; content accuracy that is not a security
issue.

## Supported versions

Only the default branch `main` and the site it deploys; there is no release. Fixes land on the
default branch only.

## Status

The site is live and rebuilt hourly from the repository. There is no certification and no bug
bounty. See AUDIT_SCOPE.md for what to review and how to build and test it.
