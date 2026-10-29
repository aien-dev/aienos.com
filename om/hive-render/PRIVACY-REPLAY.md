# Hive privacy replay

Host-only verification. The collector is run only with newly created empty source
directories. No live transcripts, service configuration or publication endpoint
are used. Existing Python oracles remain unchanged; this replay exercises their
renderer and receiver cases without invoking Python, and adds collector startup
configuration controls. Node is a verification tool, not a serving dependency.

## Run

Use Linux AArch64 and the committed compiler whose SHA-256 is
`734f0a39319fbfa930bcc62cb30dbf06d47d78e126372b74f5baf478e6126b61`.
From the repository root:

```sh
cargo test --workspace
cargo clippy -p hive-feed --all-targets -- -D warnings
cargo build -p hive-feed --bins
scratch=$(mktemp -d)
om/hive-page/omc < om/hive-render/render.om > "$scratch/render"
chmod +x "$scratch/render"
cmp "$scratch/render" om/hive-render/hive-render
cd om/hive-render
node test-replay.mjs "$scratch/render" ../hive-page/omc "$scratch/replay" ../../target/debug/hive-receive ../../target/debug/hive-feed
```

Check the compiler digest before execution, and stop if compilation or any check
fails. The `native-privacy` CI job performs that digest check and runs the replay
on an isolated AArch64 host. Its trigger includes collector and renderer changes.

## Exercised evidence

Local execution used Node v24.21.0 and the exact compiler above. It passed:

- Native renderer regeneration equals the committed executable.
- Deterministic output, every byte through the production escaper, Unicode, XSS
  payloads, malformed input, schema and field limits, hierarchy, dates, deep
  trees and both output pools.
- Every tested human interaction role refuses with no partial renderer output;
  an assistant reply remains accepted.
- Actual receiver publication, failed-render retention, malformed and human-role
  snapshot refusal, and overlapping-stream ordering.
- Actual collector startup with isolated empty source directories: missing list
  and invalid alias refuse before output; explicit empty and valid lists work.
- Rust unit controls first failed against the draft for a missing private list
  and an arbitrary public alias. They pass with the corrections. The full
  workspace suite and strict hive-feed clippy check also pass.

The original main renderer accepted the canonical synthetic `drake`-role
snapshot with exit 0. The corrected renderer refused that same snapshot with
exit 108 and no output. An earlier noncanonical fixture was rejected by both
and is not counted as the negative control.

The date oracle was obtained from WolframLanguageEvaluator:
`Table[{y,DateDifference[{y,2,28},{y,3,1},"Second"]},{y,{1900,2000,2024,2026}}]`
returned 86400, 172800, 172800 and 86400 seconds respectively. These are the
expected values used by the replay, not timings measured from the program.

## Limits

This checks source and isolated host behavior. It does not activate the live
collector, remove previously published material, or promise perfect secret or
quotation detection. Quote matching is bounded and heuristic. Unlisted private
repository names cannot be identified by the configured-name filter.

Before any separate live deployment, provide and review the private repository
list. The collector now refuses a missing list; an explicit empty file is the
operator's declaration that no private names are configured.
