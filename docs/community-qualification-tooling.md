# Community qualification tooling (bounded Omega slice)

This branch contributes one native seeded-Omega projection exporter. It is deliberately **not** the queue collector, receipt importer, qualification engine, or an attestation mechanism.

## Current executable contract

`tools/community-qualification/src/snapshot.om` reads one bounded ASCII framing record from stdin:

```text
M<TAB>1<TAB>YYYY-MM-DDTHH:MM:SSZ<TAB>https://github.com/<public-source-path><LF>
```

It accepts a maximum input shorter than 8192 bytes, a fixed 20-byte timestamp ending in `Z`, and a public source URL beginning `https://github.com/`. It rejects malformed framing, truncated data, control bytes in the URL, quotes and backslashes (rather than attempting JSON escaping). It emits schema-version-1 JSON with the supplied timestamp/source URL and `jobs: []`. Exit 2 means input refusal; exit 3 means output write failure. No partial JSON is emitted for input refusals.

The output only describes an empty runnable snapshot. It does **not** claim that the local queue is empty: no queue contents are read. No gates are emitted, so there is no invented gate digest. If a future blocked row carries a gate, `contract_sha256` is optional and must be null/absent unless the actual gate contract digest is available. A job is never runnable without reviewed commands and full gate-contract binding. Never invent digests.

## Trust and source boundary

The authoritative Renda queue implementation is in the installed local release source at `~/.local/lib/renda/releases/*/local_queue.py` (schema `renda-local-queue/1`; bounded queue and methods include `load`, `sync`, `source`, and `receipt`). We inspected the release source identities and API names, but did not read or export private queue records. This Omega program consumes a framing envelope only; there is not yet a safe queue-to-projection adapter. The existing `renda-harness` symlink targets an installed release, but its import paths may be stale/missing. Do not repair or invoke it as part of this tooling.

The current `aien-test` executable is at `~/workspace/pipeline/target-sc/release/aien-test`; its actual `--help` response says that `--help` is an unknown command and prints usage. It is not called by this exporter. No generic failed gate is interpreted as missing hardware.

No receipt parser/importer is implemented. A future receipt-submission contract must accept only allowlisted fields, bind canonical repo/PR/full head SHA and gate-contract digest, report contributor outcomes separately from accepted status, and never run commands, fetch receipt-supplied URLs, or write trusted status. Parsing/hash validation alone cannot establish hardware provenance.

## Build and exercise on the qualified Spark

The bundled compiler is an immutable copy of seed compiler source pin `57f5f4258cebf3d5b2c22853cd062e351265c525`, binary SHA-256 in `evidence/community-qualification/compiler.sha256`. Build and run the native artifact with:

```sh
tools/community-qualification/tests/snapshot-red-green.sh
```

The test compiles Omega source to a nonempty AArch64 Linux ELF and executes it against a valid empty projection plus wrong-prefix, malformed timestamp, truncated, quote-injection, and 8192-byte refusal inputs. Evidence produced by the current test run is under `evidence/community-qualification/`.

## Explicit gaps

- No local queue read, queue-to-projection conversion, job rows, multi-job/list handling, or output for actual queue contents.
- No validation of job identities, duplicate PRs, head/base mismatches, gate statuses, commands, reports, review flags, or secret exclusion from queue records.
- No receipt validation/submission tool, receipt-size/duplicate-field checks, or policy acceptance workflow.
- Timestamp checking is fixed-width/character-level, not full calendar validation.
- One-record empty snapshot only; no JSON parsing, no network access, and no live-service interaction.
- Parent owns site integration and renderer fixtures. No site files, CSS, SEO, workflow, live queue, or service were modified. No second-Spark contributor portability was tested.

This is a tested intermediate, not the design's complete acceptance or deployable public queue snapshot.
