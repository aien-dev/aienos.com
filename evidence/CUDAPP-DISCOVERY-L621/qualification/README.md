# Exact-source Mac qualification and intake

Source branch: `feat/cuda-inference-discovery`, source head `1a76ee477207fece95b555c1b2af1242c703da2b`, tree `0e5441d62da6bd268a035745bb7cfe11ce4f23df`.

This evidence-only commit is not a site implementation or merge candidate. It adds the detached qualification and acknowledgement for the named source without creating a self-referential source receipt. The implementation and public audit package are on the source branch.

`receipt.json` is canonical JSON signed with the existing Mac attestor's Ed25519 key. `receipt.sig` is base64 detached signature; raw command logs are compressed without alteration. Each uncompressed log SHA-256 matches its receipt row. All three exact installed recipe commands exited zero. Tracked source was clean before and after execution. The copied public key is for reproduction, not an authority selected by candidate source: the deployed verifier used the already-pinned Spark key and returned eligible=true.

`intake-acknowledgement.json` was read back from the existing website owner's local queue and signature store. State is ACCEPTED_LOCAL_NOT_PUBLISHED. It is not a PR, merge approval or Pages deployment. Renda retains full desk review and fresh remote checks. Ledger L633 owns publication and live readback; L621 implementation and L628 independent review are separate.
