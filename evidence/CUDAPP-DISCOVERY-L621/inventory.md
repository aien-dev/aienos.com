# CUDA++ discovery source inventory

Checked 2026-10-22 for L621. This is a source inventory, not a new hardware measurement.

| Claim or surface | Source | Scope |
|---|---|---|
| CUDA++ identity, MIT, first kernel and qualified module | https://github.com/aien-dev/CUDApp/blob/232332422bfdc31c8918085b50f13c1e18ab702e/README.md | Public main snapshot; GB10 GPU, not all GPUs or full CUDA API |
| Driver-client path | https://github.com/aien-dev/CUDApp/blob/232332422bfdc31c8918085b50f13c1e18ab702e/docs/DRIVER.md | NVIDIA kernel module and firmware remain underneath |
| Hardware portability | https://github.com/aien-dev/CUDApp/blob/232332422bfdc31c8918085b50f13c1e18ab702e/docs/runtime/PORTABILITY.md | CPU/host receipts do not qualify other GPUs |
| Replacement coverage | https://github.com/aien-dev/CUDApp/blob/232332422bfdc31c8918085b50f13c1e18ab702e/ROADMAP.md | Full library/runtime/independent-driver qualification is unfinished |
| No-CUDA server process | https://github.com/aien-dev/omega-mv/blob/89aad7c99b4fced80f92b656d2a3f350699456f0/evidence/WHAT-OMEGA-IS-20261021/live-serve-no-cuda.txt | Captured 2026-10-21 Qwen3.6-35B-A3B FP8 process, not current uptime or speed |
| Underlying language and verification foundation | https://github.com/aien-dev/omega-mv/blob/89aad7c99b4fced80f92b656d2a3f350699456f0/docs/OMEGA.md | Universal interoperability is intent; supported subsets need their own gates |
| Existing CUDA++ page correction | https://github.com/aien-dev/aienos.com/commit/d3f98f87d8a13257f7d5af19691392894aa6c9c8 | Carried, not rewritten; host vs chip limits retained |
| Existing failed Pages repair | https://github.com/aien-dev/aienos.com/commit/9e40f7b85200e47c3f71aaab13c8b46258612636 | Carried comparison-policy and coalescing repair; full tests rerun |
| Existing dashboard qualification prerequisite | https://github.com/aien-dev/aienos.com/commit/d411e1e030f1c06c6863953cce02bbf432b74cb4 | Carried to preserve dashboard and installed collector qualification recipe |

## Audit findings addressed

- The existing bot summary prohibited comparison even after that policy was reversed.
- The existing CUDA++ page metadata used an old repo name and implied a complete replacement.
- CUDA++ had no dedicated software entity with its MIT licence in page JSON-LD.
- No focused CUDA-alternative or AI-inference-speed guide was registered.
- The llms-full exporter dropped the CUDA++ overview because its content-bearing wrapper was a header; it is now a section.
- The local motor link pointed at an absent section; it now points to the pinned design.
- Old snapshot ledger inputs do not contain newer fragment targets. Browser/link audits use the actual published data/now snapshot, separately from source.

## Claims deliberately not made

No universal CUDA API/C++ compatibility, all-language connection, all-GPU support, independent-driver qualification, security certification, current server uptime or measured victory over CUDA/llama.cpp/vLLM. The new performance guide explains how to establish a fair comparison without inventing one. No search indexing, AI answer inclusion or model-training inclusion is guaranteed by robots.txt or llms.txt.
