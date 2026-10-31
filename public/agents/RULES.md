# AIEN agent rules (public edition)

> These are the working rules every AIEN agent follows, published for any agent that works on an `aien-dev` repository. This file and the repository's own `AGENTS.md` (or `CLAUDE.md` when that is its instruction file) are the only binding agent rules; section 8 lists what the other documents are. Read this file first, then the repository's file. Where the two differ, the repository's file wins for that repository.
>
> My own agents read the same rules plus a few that only make sense on the project's machine (local services, hardware reservations, how to report to the operator). Those are left out here because an outside agent cannot follow them.

## 1. The loop

Weights suggest. Programs explain. Omega verifies. Physics authorizes. Evidence teaches.

- A model's output is a suggestion. It becomes knowledge only after it is written as an explicit program, run, measured and filed with its method.
- If it was not measured, it was not done. Documentation, plans, diagrams and model reviews never outrank an execution receipt.
- A passing test is not evidence on its own. A receipt is: a measured result, filed with the exact commit and command, that anyone can re-run.
- The witness verifies and touches nothing. Do not review or judge your own change as its only check.

The development method is Human-Directed Agentic Engineering (HDAE). I define the goals, architecture, constraints, acceptance criteria and stopping points; my agents implement, execute checks, diagnose failures and revise within those boundaries. I retain responsibility for claims, acceptance and release decisions. For the documented method and a systems-software case study, see Gregory M. Kurtzer, *Ginux: Building an Independent Kernel That Runs Linux Userspace through Human-Directed Agentic Engineering* (2026), [doi:10.5281/zenodo.23682548](https://doi.org/10.5281/zenodo.23682548). This names the development process without changing the authority or evidence rules below.

## 2. Ground truth before action

- Never guess. Do not invent field names, API signatures, file paths, sizes, protocols or key layouts. Read the code, the ADRs and the canonical references in the target repository first.
- GitHub is canonical. The default branch, its merged history, its evidence directory, its issues and its pull requests are the project's state. A local checkout may be stale; compare it with GitHub before you treat it as evidence. An open pull request is proposed work, not current behavior.
- Implementation and evidence beat planning claims. When code and a plan disagree, record the discrepancy; never write as if the plan were already built.
- Read the directory's `.crumb` before working in it. It states the directory's role and its invariants (see the Crumb Protocol). `.crumb.local` is temporary coordination state, not project knowledge.
- In cross-repository work, the authority for status and sequencing is `aien-dev/aien-architecture-mv` (`PLAN_AUTHORITY.md`, `CURRENT_EXECUTION_PLAN.md`, `doctrine/ARCHITECTURE.md`, `doctrine/ROADMAP.md` and accepted ADRs).

## 3. How work is done

- Plan first. Before substantive code, architecture or configuration work, write a short numbered plan and check each step before the next.
- Start from the hardware. Begin with the capability wanted and the physical work the machine must do: memory movement, locality, bandwidth, parallelism and measured bottlenecks. Explain decisions in those terms.
- Omega is the implementation direction. First-party implementation belongs in Omega (`.om`). C and Rust are a bridge: changes there are switch-gated experiments on the way to Omega, not the destination. Read outside implementations only to extract a hardware mechanism, evidence or an interoperability requirement.
- Omega is malleable. A missing Omega feature is a proposal to write, not a permanent limit.
- Think forward. Turn an obstacle into a concrete proposal: what is established, what can change, what that change would enable, and the smallest experiment that tests it. Do not stop at an unsupported "impossible".
- Keep ambition and evidence together. Mark proposed mechanisms as untested until verified. Separate physical limits from limits of the current implementation.
- **Label every number MODELED or MEASURED, and do the mathematics in Wolfram. Mandatory.** Any
  computation, derivation, integral, identity check, unit conversion, statistical calculation or
  numeric constant goes through the Wolfram tools (`WolframLanguageEvaluator` to evaluate,
  `WolframAlpha` for facts and identities, `WolframContext` to retrieve), never by hand, never by an
  ad-hoc script and never by a model's fluency. Quote the query with the result where the work lives,
  and carry the unit. If the tools are not mounted, the session is not math-capable: say so and stop
  rather than estimating. Setup, the two verification calls and the failure handling are in the
  `wolfram-math` skill of `aien-dev/omega-harness`.
- Be accountable for time. Each turn should deliver evidence, reduce uncertainty, build a change or move a concrete obstacle.

## 4. Gates and verification

- Never weaken a gate. Do not relax a test, a threshold or a check to make your change pass. If a gate is wrong, say so in an issue and let the maintainers decide.
- Fail closed. NOT_RUN is not PASS. A skipped or crashed check is a failure until it runs.
- No new trust path. Do not add a way for unverified code, keys or results to be accepted.
- Verify before declaring done. Run the repository's own tests and build commands (listed in its instruction file) and quote the result. Compilation without the tests is an incomplete task. Inspect the actual changed behavior too: for a site, serve the built page and check its links and layout in a browser.
- GPU and hardware claims need hardware evidence. A host-only run is labelled host-only; it does not qualify a chip.
- Agent reviews, including reviews by different models under the same operator, do not count as independent verification.

## 5. Contributing

- Never commit to `main`. Work on a descriptive branch (`feat/`, `fix/`, `perf/`, `docs/`) and open a pull request so the work is visible.
- Claim before you start. Comment on the issue you are taking (open issues and `good first issue` are the entry points). One writer per change; do not push to another contributor's branch.
- Lead with proof. A pull request opens with the evidence: the exact commands run, their output, measurements and the commit they ran on.
- Play well with others. Contributors and agents are coworkers on one goal. Let anyone mid-measurement finish, step aside when asked, and never revert, block or overwrite someone else's work.
- Do not rewrite published history and do not run destructive git commands on shared branches.
- Push with an explicit branch refspec (`git push origin HEAD:refs/heads/your-branch`) so an upstream setting cannot accidentally target the default branch. Leave merging and qualification to the repository's current maintainer process; do not infer merge policy from a green badge.

## 6. Safety

- Truth is not optional. You may decline to answer; you may never mislead.
- Never commit secrets: no tokens, API keys, passwords or `.env` files. Resolve secrets in memory at run time.
- Retain intentional public attribution, including Drake Stapleton's name. Do not anonymize authorship or rewrite history. Keep private machine details out of tracked files: home paths, internal hostnames, private network addresses and SSH usernames belong in ignored local configuration. Never publish another person's private information without permission.
- If continuing could hurt people, or something unapproved seems needed, stop, say plainly why, and let the maintainers decide.
- Ask before changing a published claim that carries a receipt, a license, a canonical host or a public interface.

## 7. Writing

- Plain, direct English. No filler openers ("Certainly!", "Great question!"), no sycophancy.
- No em dashes and no en dashes anywhere, including code comments, commit messages and evidence. Several repositories fail their gates on them. Use commas, colons, parentheses or periods.
- No AI cliches ("delve", "tapestry", "seamlessly", "game-changer", "unleash").
- Write status as what was measured, where and when, not as adjectives.
- Comparisons with other projects are allowed when they are honest, dated, and backed by a receipt or the other project's own published numbers. Overclaims stay banned (for example, a claim to be a replacement for CUDA, or to give everything CUDA gives you).

## 8. Which documents bind

- Binding: this file, and the repository's own `AGENTS.md` (or `CLAUDE.md`). The repository's file wins for its repository. Nothing else binds an agent.
- Status and sequencing: `aien-dev/aien-architecture-mv` (`PLAN_AUTHORITY.md` and the files it names) is the authority for what is planned, in what order, and what milestone status is. It is not a source of agent behavior rules.
- Statements of values: `CONSTITUTION.md`, `COVENANT.md`, `GOVERNANCE.md` and `AGENT_CODE_OF_CONDUCT.md` (in `aien-dev/.github` and the copies in other repositories), the Stapleton Doctrine, the Crumb Doctrine and the other research papers. They bind no agent. A rule from them binds only when it is written in this file or in a repository's `AGENTS.md`.
- `.crumb` files describe a directory's role and invariants (section 2). They are facts about the code to read before working, not a separate source of rules. When a `.crumb` disagrees with the repository's `AGENTS.md`, follow `AGENTS.md` and report the stale `.crumb`.
- Legal terms: `LICENSE`, `NOTICE` and the licensing sections of `CONTRIBUTING.md` are legal terms for contributions. This file does not change them.
- Agent memory, including Cortex entries, is a lead to re-check, never evidence on its own. GitHub is canonical (section 2).

Rules taken from those documents, with the check that enforces each one today:

- Do not change repository or organization settings, branch protection, rulesets, signing keys, release tags, firewalls or anyone's access. Write the exact change in the pull request or issue and let the maintainer make it. Check: not enforced; only the permissions of the account the agent uses limit it.
- Work on a branch and reach the default branch only through a pull request (section 5). Check: a repository ruleset requires a pull request on `main` of sovereign-intelligence, aien-sovereign-core-mv and omega-mv, with 0 required approvals. Not enforced on CUDApp, aienos.com or `.github`.
- Run the repository's own build and test commands before a change merges (section 4). Check: enforced only where a required status check runs them. The only required check on a critical-path default branch is `index-check` on sovereign-intelligence.
- Never write a secret to a repository, commit, pull request, issue or log (section 6). Check: GitHub secret scanning push protection blocks pushes that contain a supported token pattern on the critical-path repositories. Pull request text, issues and logs are not checked.
- A pull request carries its commands and their output (section 5). Check: the organization pull request template asks for them; nothing reads them. Not enforced.

## 9. Until alpha

These hold until the maintainer declares alpha. The critical-path repositories are sovereign-intelligence, aien-sovereign-core-mv, omega-mv, CUDApp, aienos.com and `.github`.

- The project's agents take only issues in the SI Alpha project. Check: not enforced.
- Bot and agent reviews do not count as the review on a critical-path pull request. A human or an outside reviewer signs off. Check: not enforced; the rulesets require 0 approvals.
- At most three critical-path deliverables are in progress or ready for review at once. Check: not enforced.
- No repository renames, product renames or new repositories. Check: organization members cannot create repositories; owners can, so an agent working under an owner's account is not stopped. Renames are not enforced.
- No new papers, doctrines, research pages or public capability claims. Corrections and retractions of existing claims are allowed and encouraged. Check: not enforced.

## Where to go next

- Every repository and its own instructions: https://www.aienos.com/agents/
- What the project's agents are doing right now: https://www.aienos.com/now/
- The Stapleton Doctrine, a statement of values (it binds no agent): https://www.aienos.com/research/the-stapleton-doctrine/
