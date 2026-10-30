# AIEN Research Plan — V3 (Substrate Edition)

**Status: RATIFIED. Frozen edition.**
**Date: 27 September 2026**
**Ratified by the conductor, 27 September 2026, after a five-point corrective pass.**

This edition supersedes the frozen 27 September 2026 V2 brief as the
current research plan. V2 remains archived unchanged.
Nothing in this edition may be quoted as a result. Every new claim below
is a hypothesis with a named falsifier, not a finding.

---

## 1. What changed from V2

V2 established the research program: a sovereign synthesis intelligence
built from a small inspectable lineage, where programs explain, models
suggest, verification decides, and evidence teaches.

V3 adds one architectural thesis and six testable hypotheses that follow
from it. The thesis concerns the physical machine the system actually
inhabits, and it changes what Omega synthesizes, what Forge reports, what
AEGIS proves, and what counts as a good realization.

**The substrate thesis:** the DGX Spark (GB10) is one coherent computational
medium, not three separate boxes labeled CPU, memory, and GPU. 128 GB of
coherent unified LPDDR5x at 273 GB/s is shared by Grace and Blackwell with
no ownership-transfer copies required for cooperation. From registers down
to NVMe the machine is one continuous gradient of faster/smaller/private
versus slower/bigger/shared. The one remaining physical cliff is power-loss
survival: DRAM forgets, NVMe remembers.

Therefore AIEN's continuity is a **logical boundary, not a physical place**.
The architecture promises continuity for a defined region of logical state
(the resident object graph: identity, generations, rights) and promises
nothing for processor microstate (registers, caches, in-flight warps).
"Resident" means *inside the continuity boundary*, not *inside the RAM chips*.

**Language correction adopted in V3:** earlier formulations said "AIEN lives
in memory; hardware visits it." That phrasing reifies an engineering
convenience into an ontological claim and contradicts the architecture's own
invariant that logical identity is not physical address. V3 replaces it:

> **The box is one thing; the promise is the architecture.**

AIEN is a pattern of organization within one medium. Continuity is the
promise. Everything outside the boundary is execution froth the system
promises nothing about.

### Four doctrine corrections (adopted)

1. **Host/device copying is not the canonical programming model.** (Not
   "there is nothing to copy to.") Data still moves through caches, memory
   channels, DMA/copy engines (GB10 exposes two), and GPU mappings. The
   break is semantic: ordinary CPU/GPU cooperation must not be expressed as
   explicit ownership transfer between separate memory universes.
   Ownership-transfer copies are not mandatory for cooperation; that is the
   rule. It does not follow that copy engines never participate in
   cooperation. Whether a copy engine is the cheapest realization of a given
   task is decided by Omega's physical cost calculus on measured physics,
   not by architectural fiat. Checkpoint staging, compaction migration, and
   snapshot writes are candidate uses, not assigned ones.

2. **Shared substrate, separate authority domains, logical references.**
   Private address spaces are not abolished; they are abolished *as the
   interchange model*. MMU/SMMU isolation, capabilities, bounds, and
   protection domains remain. What crosses an authority boundary is never a
   raw pointer: it is ObjectId + Generation + Offset/Region + Bounds +
   Rights. This distinction is load-bearing for AEGIS.

3. **Checkpointing uses an epoch barrier.** A single coherent pool removes
   duplicated application-state reconciliation between separately owned
   host/device copies, but a consistent cut is still hard
   while CPU, GPU, J-Space, Cortex, and external effects mutate concurrently.
   GPU internal execution state still exists; it is classified as execution
   froth outside the continuity boundary, in exactly the language the
   boundary definition uses.
   The primitive: Generation N active -> open checkpoint epoch N+1 -> new
   mutations become N+1 -> drain and resolve N in-flight work -> freeze
   reachable committed N state -> publish Generation N. AIEN keeps running;
   no global stop. **Open sub-problem (named, not solved): the device
   frontier.** Effects already outside memory (in-flight DMA, half-done
   network sends, device state) cannot be epoch-tagged. Devices must either
   reach the barrier as participants or journal their effects as intents.
   This is Forge's problem and it is on the list.

4. **Movement is a first-class synthesis-time cost.** (Not "compilers ignore
   memory"; roofline models, tiling, and tensor compilers are real.) The
   genuinely new claim: physical data movement must enter Omega's *search
   objective during synthesis*, not arrive as an after-the-fact compiler
   optimization. That changes the search problem itself. The claim is not
   that GB10 is universally bandwidth-bound; boundness is workload-dependent
   (compute, latency, synchronization, or bandwidth can each dominate). The
   defensible claim: on GB10, data movement and shared-memory contention can
   dominate important AIEN workloads (1 PFLOP FP4 compute fed through a 273
   GB/s straw shared by both engines), so Omega must price movement as a
   first-class synthesis cost rather than presuming operation count is the
   dominant resource. Whether movement-first synthesis wins is what H9
   tests; this edition does not assume the outcome.

---

## 2. New hypotheses (H5-H10)

Each follows the V2 discipline: a claim, its boundary, the test protocol,
and the named falsifier. If the falsifier fires, the hypothesis is weakened
or dropped. No silent tense changes.

### H5. Shared Object Pool

**Claim:** A single generation-tagged object pool with logical identity
(resident ObjectId + Generation + rights; see the identity ladder invariant
under H8, no raw pointers crossing authority boundaries) can serve as the
sole canonical allocation substrate for cross-engine AIEN semantic state on
GB10, with zero ownership-transfer copies in the cooperation hot path.
Hardware-private structures (queue metadata, page tables, firmware state,
device control objects, registers, caches, specialized scratch) are outside
this claim.

**Boundary:** GB10 silicon, DGX Spark. Cooperation workloads: graph frontier
expansion, shared work queues, tensor staging. Does not yet claim
multi-machine pools.

**Test protocol:**
- Implement the pool; run CPU+GPU cooperative workloads through it.
- Instrument the hot path; assert zero ownership-transfer copies.
- Run the no-raw-pointer-persistence gate over all durable artifacts:
  reject boot-specific virtual addresses, physical addresses, process
  handles, GPU handles, transient queue pointers, kernel addresses.
- Run the stale-generation rejection suite: relocated objects keep logical
  identity; stale handles are refused.

**Falsifier:** a workload class in scope where correctness or performance
requires explicit ownership-transfer copies; or any authority escape; or
any stale reference accepted.

**Status:** embryonic. Omega's accelerator world already carries
generation-tagged handles (world_epoch, object_id, object_generation,
object_type, permissions) with cpu_addr/gpu_va internal to the physical
registry, plus persistent coherent buffer registries, stale-handle
rejection, bounded permissions, a scratch arena, and a persistent
accelerator context. Items H5/H6 are therefore not greenfield; the missing
jump is from dispatch/wait to shared-world participation.

**Maps to:** M15-M19 era extension (accelerator cognition substrate).

### H6. Cross-Engine Object ABI

**Claim:** Logical references (resident ObjectId + Generation +
Offset/Region + Bounds + Rights; see the identity ladder invariant under
H8) can fully replace raw pointers across authority boundaries
with cooperation overhead inside budget versus a raw-pointer baseline, while
MMU/SMMU isolation and capabilities hold.

**Boundary:** Same hardware. Budget to be frozen per workload before
measurement (proposed: within 10% of baseline handoff latency; AEGIS
authority proofs must pass absolutely, not relatively).

**Test protocol:**
- Microbenchmark handoff latency: logical-reference path vs raw-pointer
  baseline, same workload, same silicon.
- AEGIS proofs: no authority escape from shared-state references; object
  relocation does not alter logical identity; CPU and GPU cannot gain
  authority from shared references.
- Adversarial suite: forged generations, out-of-bounds offsets, rights
  escalation attempts, replayed handles.

**Falsifier:** overhead exceeds the frozen budget; or any authority escape;
or any stale generation accepted.

**Maps to:** M15-M19 era extension; AEGIS verification duties.

### H7. Resident Cooperation Protocol

**Claim:** Fine-grained CPU/GPU co-production on shared objects (both
engines consume and publish work against the shared object world) beats
bulk offload (CPU prepares dispatch, rings GPU, waits) on latency and/or
bytes moved for interactive frontier-style workloads on the shared
273 GB/s pipe.

**Boundary:** Representative workloads: J-Space frontier expansion,
concurrent graph mutation, mixed CPU/GPU pipelines. Not claimed for
embarrassingly parallel bulk kernels where offload is already optimal.

**Test protocol:**
- Head-to-head: cooperation protocol vs dispatch/wait, identical semantics,
  identical silicon.
- Measure wall-clock latency, bytes through the shared pipe, cache-line
  touches, coherence transitions, synchronization events.
- The workload set and the win criteria are frozen before measurement.

**Falsifier:** dispatch/wait wins across the frozen representative set on
both latency and bytes moved.

**Maps to:** proposed era extension between M19 and M20.

### H8. Generation/Checkpoint Barrier

**Claim:** An epoch barrier (open N+1, drain N, freeze committed N, publish
Generation N) produces consistent semantic cuts of the live world without
global pauses exceeding budget, and cold reconstruction from sealed
generations reproduces logically identical state.

**Boundary:** Single machine. Pause budget frozen before measurement
(proposed: no global pause longer than 10 ms; checkpoints progress during
soak). Crash recovery must yield OLD or NEW generation, never half of each.

**Test protocol:**
- AIEN_RESIDENT_WORLD_RESTORE_PASS: create state, checkpoint, destroy all
  volatile state, reboot, reconstruct from durable artifacts only, verify
  logically identical committed state.
- Crash-at-every-boundary: crash at each write/flush step of checkpoint
  publication; only complete previous or new generations may be visible.
- Accelerator-loss gate: reset Blackwell mid-run; CPU-resident authority
  intact; accelerator realizations rebuilt without corrupting logical
  identity.
- Long soak: bounded allocation growth, checkpoint progress, stable
  latency, no generation exhaustion.

**Falsifier:** any torn generation; reconstructed state logically diverges;
pauses exceed budget; soak shows unbounded growth.

**Identity ladder (invariant).** Three identifier concepts already exist in
the system and must never accidentally collapse into one:

- Omega SemanticId = identity of meaning.
- Resident ObjectId + Generation (+ Rights) = identity of a live logical
  object incarnation.
- AIENOS Store ObjectId = identity of durable stored bytes.
- Physical address / GPU VA = disposable realization.

The complete ladder:

MEANING (SemanticId)
-> LIVE LOGICAL OBJECT (ObjectId + Generation + Rights)
-> DURABLE REPRESENTATION (Store ObjectId)
-> PHYSICAL REALIZATION (CPU VA / GPU VA / pages / device objects)

Wherever H5, H6, or H8 says "ObjectId" without qualification, it means the
resident live-object identifier. Builders must not substitute the Omega
SemanticId or the Store ObjectId.

**Maps to:** redefines M37 (AIEN_RESIDENT); touches the AIENOS trust lane
(power-cycle continuity for one agent identity).

### H9. Physical Cost Calculus

**Claim:** Synthesis guided by a movement-first physical cost model produces
realizations with measurably lower GB10-measured physical cost than
synthesis guided by op-count cost, for identical semantics.

**Boundary:** The cost model counts bytes read/written per engine,
cache-line touches, coherence transitions, synchronization events and
barriers, estimated reuse distance, arithmetic intensity, peak live bytes,
and contention-aware pricing, with measured latency/bandwidth/energy
feeding back to correct the predictor. Predicted cost (used in search) and
measured cost (empirical ledger) are kept distinct.

**Test protocol:**
- Freeze a semantics set (starting with: bounded concurrent work queue).
- Synthesize under op-count cost and under physical cost.
- Measure both realizations on GB10 silicon: time, bytes moved, energy.
- The metric set and significance bar are frozen before synthesis.

**Falsifier:** no statistically significant measured-cost difference across
the frozen set.

**Maps to:** extends M8 (cost models), M13/M14 (machine graph gains
measured contention facts); evolves OmegaCost toward OmegaPhysicalCost.

### H10. The Closed Loop

**Claim:** The loop OMEGA proposes -> AEGIS proves -> FORGE realizes ->
GB10 executes -> EVIDENCE measures -> OMEGA searches again can run
end-to-end without human intervention, and can produce a substrate
realization that holds all invariants and beats the human reference oracle
on measured physical cost.

**Boundary:** First target: the bounded concurrent work queue, with
semantics, machine description (GB10 coherent memory), invariants (no stale
generation, no authority escape, no torn publication, exactly-once state
transition), and objective (minimize measured physical cost) all frozen
before the loop starts. Humans build the first reference oracle.

**Test protocol:**
- Milestone 1 (keystone): one full loop turn with zero human intervention.
  The loop running closed matters more than winning.
- Milestone 2: a loop-produced realization that AEGIS proves and that beats
  the oracle on measured physical cost.
- Every turn emits immutable receipts: proposal identity, proof outcome,
  realization identity, machine identity, measurements, and the decision.

**Falsifier:** the loop cannot close without human repair; AEGIS rejects all
proposals over N frozen iterations; or no measured improvement over K
frozen iterations with honest measurement.

**Maps to:** the concrete mechanism for M38 (continual library learning)
and for the RSI story: this is what "the machine improves itself" cashes
out to. Long-horizon, it is the engineering shape of M40 succession.

### Deferred: H11 Semantic Reconstruction (candidate, not in this edition)

**Candidate claim:** a sealed generation can be destroyed physically and
reconstructed at different addresses, possibly in a fresh Store region,
retaining the same LogicalAgentId, the same Omega SemanticIds, the same
committed relationships, and the same authority semantics, with a different
physical realization.

This would connect the substrate research directly to the
resident-continuity architecture instead of leaving persistence as an
AIENOS-side detail. Deferred deliberately: H5-H10 must stand first.

---

## 3. Implementation order (ratified)

1. Shared Object Pool: logical identity + generations + rights.
2. Cross-Engine Object ABI: no pointers crossing authority boundaries.
3. Resident Cooperation Protocol: both engines consume and publish.
4. Generation/Checkpoint Barrier: consistent semantic cuts of the live world.
5. Physical Cost Calculus: bytes + locality + synchronization + compute,
   contention-aware, predicted vs measured split.
6. Omega Synthesizes the Above: humans build reference oracles first; then
   Omega proposes realizations against frozen semantics, machine facts,
   invariants, and measured-cost objectives.

Steps 1-2 are embryonic in the current Omega accelerator world. Steps 3-4
are buildable now. Step 5 is instrumentation plus model work. Step 6 is the
research program. Treating 6 as a build ticket will produce fake compliance
or stalled builders; it is a research track with the keystone experiment
(H10, milestone 1) as its first gate.

---

## 4. New research questions

- **RQ7:** Can CPU/GPU cooperation be expressed without ownership-transfer
  copies while preserving isolation between authority domains?
- **RQ8:** Can checkpointing produce consistent cuts of a live world
  without stopping cognition, and can cold reconstruction preserve logical
  identity?
- **RQ9:** Does movement-first synthesis produce better realizations than
  op-count synthesis on coherent shared-memory hardware?
- **RQ10:** Can the propose/prove/realize/execute/measure/research loop run
  closed, and does it compound?

---

## 5. New measurement families

**Movement:** bytes read/written per engine, cache-line touches, coherence
transitions, synchronization events and barriers, all measured on GB10
silicon under declared contention conditions. Contention conditions are
part of the measurement contract: bytes moved during GPU saturation are
priced differently from bytes moved idle.

**Continuity:** checkpoint pause durations (max and distribution),
generation publish atomicity (OLD-or-NEW under crash injection),
cold-reconstruction logical-equivalence rate, soak resource-growth curves,
generation exhaustion distance.

**Loop:** closed-loop turns completed without human intervention, AEGIS
acceptance rate per turn, measured-cost delta vs oracle per turn, invariant
violations (must be zero; any violation fails the turn, not the hypothesis).

---

## 6. New risks (added to the V2 risk table)

| Risk | Why it matters | Research response |
|---|---|---|
| The fragmentation dragon | Shared-everything plus intended infinite lifetime is one of the hardest memory-management problems there is. Systems famous for long uptime isolate heaps; this architecture shares everything. | Name it as the central research risk. Generational addressing is the mitigation bet; the soak gates are the detectors, not the cure. A soak test detects fragmentation; it does not solve it. |
| Contention mispricing | On a shared 273 GB/s pipe, the cost of moving bytes depends on what else is moving. A static machine graph misprices under contention. | Price movement against live contention; contention conditions are part of every measurement contract; predicted vs measured cost split with the empirical ledger correcting the predictor. |
| Device frontier | Epoch barriers tag memory, not packets. External effects escape the consistent cut. | Name it now. Devices reach the barrier as participants or journal effects as intents. Forge owns this problem. |
| Live substrate update | "Remain resident" has no story for patching ATLAS, AIENOS, or Omega semantics beneath a running world. The first security patch ends "never shuts off" without one. | Named as an open problem. Candidate direction: dual-world handoff (build new world beside old, migrate generations, cut over). Not claimed, not scheduled. |
| Capacity budget | 128 GB is the hard ceiling: weights, KV, semantic graph, branch worlds, Cortex hot set, sealed generations. A resident intelligence that does not fit in RAM is not resident. | Working-set discipline and a capacity budget are required before M37 can be claimed. "Save the minimum information necessary to recreate the same logical intelligence" is the principle; the budget makes it testable. |
| Closed-loop theater | A loop that runs but never improves, or that AEGIS waves through, is process theater. | The keystone milestone is a *closed* loop, honestly measured. Improvement is milestone 2. Any invariant violation fails the turn. |

---

## 7. What V3 does not change

- V3 is the frozen current edition as of 27 September 2026. V2 remains
  archived unchanged. No silent edits.
- The Infinite Game method (propose, freeze contract, admit, decompose,
  build isolated, test at the right layer, hostile review, escalate, receipt,
  ratify/reject/reopen, feed forward) is unchanged. The closed loop of H10
  is that method turned inward on the substrate.
- The claim ledger discipline is unchanged: what exists, what is being
  repaired, what remains a target. Every H5-H10 entry above is a target.
- M19 remains reopened. Nothing in V3 builds on a misleading layer.
- The lineage boundary is unchanged: contemporary AI systems are
  construction tools and oracles, not permanent ancestors.
- No sovereign training has occurred. AIEN_0 and M20-M40 remain planned.

---

## 8. Roadmap mapping

| V3 element | Roadmap touchpoint | Note |
|---|---|---|
| H5, H6 | M15-M19 era extension | Embryonic in current Omega accelerator work |
| H7 | Proposed era extension, M19-M20 | Head-to-head vs dispatch/wait |
| H8 | Redefines M37; AIENOS trust lane | Epoch barrier; power-cycle continuity |
| H9 | Extends M8, M13, M14 | OmegaPhysicalCost; measured contention facts |
| H10 | Mechanism for M38; shape of M40 | Keystone: first closed loop |
| Copy engines (cost-selected, not doctrinally assigned) | Forge duties | Omega's physical cost calculus decides per task; candidate uses include checkpoint staging, compaction, snapshots |
| Authority domains | AEGIS duties | Capabilities + shared substrate |

Roadmap numbering changes, if any, require a new dated edition per the V2
freeze rule. Nothing here renumbers milestones unilaterally.

---

*Ratified 27 September 2026. H5-H10 are the canonical research program.
Every claim above is a hypothesis with a named falsifier; none is a
finding.*
