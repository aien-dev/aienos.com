# Real local-queue projection (native Omega)

This supersedes the metadata-only envelope slice in `community-qualification-tooling.md` for queue rows. The earlier envelope remains historical; do not install its empty output as a live queue feed.

## Executed boundary

An existing `jq` host utility selects only public PR identity/title/check fields from the authoritative local Renda queue. The new native Omega program validates the framed projection, rejects duplicate identities and unsafe/malformed fields, and constructs the site's schema-version-1 JSON. It does not read raw queue diagnostics into output, run tests, fetch networks, import receipts or write any trusted status. This is disclosed GNU/Linux host orchestration plus a seeded-Omega transformation, not a claim that the whole publisher is native Omega.

The parent exercised it against an authentic `aien-dev/omega-mv` queue observation and obtained 63 exact-source PR rows. The repository/number/head/base tuple set and row count matched the captured projection exactly. That is a bounded repository sample, not all AIEN repos and not evidence that these PRs only lack hardware tests.

Every exported row is blocked from execution. `commands` and `requirements` remain empty; `instructions_reviewed` is false; gate contract hashes are null. Reported GitHub check states are explicitly labeled as reported checks, not accepted gate evidence. A missing check is unknown. Real runnable hardware recipes require separate contract/preflight work. No pass is invented.

## Framing

Input is UTF-8 text with LF-terminated records, no literal tab or newline inside a field:

```text
M<TAB>1<TAB>YYYY-MM-DDTHH:MM:SSZ<TAB>https://github.com/public/source<LF>
J<TAB>owner/repository<TAB>PR-number<TAB>40hex-head<TAB>40hex-base<TAB>public-title<TAB>Spark-state<TAB>hosted-state<LF>
```

Check states are `SUCCESS`, `FAILURE`, `ERROR`, `PENDING`, `UNKNOWN`; all other tokens are refused. SUCCESS becomes pass_reported, FAILURE/ERROR become fail_reported, and the others become unknown. These are imported check observations, not hardware attestation.

Use the queue's `observed_at` converted to UTC as the metadata sample time, not the time the exporter happens to run. Select only `.entries[]` whose `kind` is `pr` and `active` is true. Preserve `.repository`, `.pr.number`, `.pr.headRefOid`, `.pr.baseRefOid`, `.pr.title` and the two named check observations. Do not infer a PR number for a local READY-only record. READY work without a PR is not yet represented by this schema.

Existing jq TSV output escapes backslash/tab/newline in titles; these remain literal escaped display characters, not source identifiers. The native output JSON-quotes title content and normalizes en/em dashes for the site's prose rule. Canonical repo/PR/head/base values are validated and preserved, never repaired.

## Build and exercise

From a clean source export on a Linux AArch64 host, using the immutable bundled compiler copied from source pin `57f5f4258cebf3d5b2c22853cd062e351265c525`:

```sh
cat tools/community-qualification/src/queue-snapshot.om tools/community-qualification/src/queue-main.om | tools/community-qualification/bin/omc-57f5f425 > queue-snapshot
# Require compiler exit 0 and a nonempty AArch64 ELF before chmod/execution.
chmod +x queue-snapshot
./queue-snapshot < public-queue-projection.tsv > snapshot.json

cat tools/community-qualification/src/queue-snapshot.om tools/community-qualification/tests/queue-tests.om | tools/community-qualification/bin/omc-57f5f425 > queue-tests
# Again require successful compile and nonempty correctly targeted ELF.
chmod +x queue-tests
./queue-tests
```

The production and test entry points are both written in `.om`; `cat`, the compiler invocation, chmod and stream redirection are external host orchestration. No new Python/Rust/C or shell runtime is introduced for the projection or native structural tests. Do not count a compiler failure as a behavioral refusal test.

The test program checks a valid synthetic record, duplicate repo/PR refusal, distinct repositories sharing a PR number, malformed calendar time, traversal-like repository, malformed source pin, private token-like title, unsupported state and truncated record. A direct-core truncated-record case first returned 26 (RED), then returned 0 after the framing correction (GREEN). No fake fixture is used as a production job.

Limits: input must be strictly smaller than 1 MiB, output is bounded to 2 MiB, at most 1,000 PR rows and 8,192 total fields. Titles are bounded to 4,096 bytes. All input is read and checked before any JSON is written; positive partial writes are completed, output errors return 3. Input refusal returns 2 with no JSON. The site performs an independent stricter schema/link/time/UTF-8 validation before publication, and never copies raw input into its output assets.

## Remaining acceptance

- This exports the public projection of one observed repository sample. The automated multi-repository source adapter and existing publisher integration are not installed.
- Underlying gate receipts, exact portable hardware requirements, dependencies and reviewed commands still need mapping before a job is advertised as runnable.
- Receipt parsing/import and provenance acceptance are not implemented. GitHub comments remain a manual contributor submission path, not automatic trust.
- No second physical Spark replay, PR merge or public deployment is claimed.
- Source review of the parent's new native projection is still required; a successful native test and JSON decode are not independent review.
