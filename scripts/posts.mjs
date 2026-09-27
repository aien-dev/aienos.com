// Post data for the aienos.com static blog.
// Plain JS module imported by scripts/build-blog.mjs. Body is hand-written HTML.
// Copy rule: no em dashes or en dashes anywhere in user-facing copy.

export const posts = [
  {
    slug: "aien-v2-research-plan",
    title: "The AIEN V2 Research Plan",
    dateISO: "2026-09-27",
    dateLabel: "September 27, 2026",
    excerpt:
      "AIEN V2 is a research program asking whether machine intelligence can be built from a small, inspectable software lineage instead of an inherited opaque stack. This post introduces the public research brief: the thesis, the construction method, and an honest ledger of what is proven, what is underway, and what remains ahead.",
    bodyHtml: `
<p class="lead">Today we are publishing the first public research brief for AIEN V2: a 25-page document that lays out the program in full. What has been demonstrated. What is actively being built or repaired. What remains planned. It is written to be read as a research program, not a manifesto, and it holds itself to one standard: separate what is proven from what is hoped for, and let evidence decide.</p>

<div class="pdf-callout">
  <div>
    <div class="pdf-callout-title">AIEN Research Plan</div>
    <div class="pdf-callout-sub">The full 25-page public brief, September 27, 2026 edition</div>
  </div>
  <a class="pdf-button" href="/research/aien-research-plan.pdf">Read the full 25-page research brief (PDF)</a>
</div>

<h2>The thesis</h2>
<p>The brief opens with a single sentence: <strong>"Intelligence should be allowed to propose. Evidence should decide."</strong> It then turns that into a falsifiable engineering question: can intelligence be built from a small, inspectable lineage that learns through verified programs and machine evidence, rather than permanently inheriting an opaque software tower?</p>
<p>The lineage is deliberately small. A minimal seed (Atlas) awakens AIENOS, the trusted substrate. Above it, OMEGA defines, synthesizes, verifies, and realizes programs, keeping the ones that work in a reusable library. AIEN, the neural component, learns to guide the search from OMEGA's own verified search traces, never from human text pretraining. The architecture is explicit on this point: neural weights suggest where to search; they do not define meaning, truth, or authority. Verification does.</p>

<h2>The Infinite Game</h2>
<p>The brief also documents how the system is being built, because the construction method mirrors the system itself. Multiple independent AI systems (Gemini, Claude, Codex, Grok, DeepSeek, and Muse) each contribute parts under a human conductor. No single system is trusted with the whole picture. Independent AI watchers review every commit, checking both the work and whether the other reviewers caught the mistakes, and every finding feeds back into a compounding loop that improves the building and the oversight together.</p>
<p>The loop is designed to keep going, including through succession beyond its original conductor. That is the name: this is not a game played to win and finish. It is a game played to keep playing.</p>

<h2>The claim ledger</h2>
<p>The most important discipline in the brief is also the simplest. Every claim is sorted into one of three buckets: demonstrated (built and verified, with evidence), active (being built or repaired right now), or target (planned, with the conditions that would weaken or falsify the hypothesis stated up front). The document gives the project a way to fail, and says so plainly:</p>
<blockquote>"The correct response to weak evidence is not stronger language. It is a stronger experiment."</blockquote>
<p>A brief is a snapshot. This post accompanies the September 27, 2026 edition of the research brief; the live project roadmap remains canonical and will keep moving as the work proceeds.</p>

<div class="pdf-callout">
  <div>
    <div class="pdf-callout-title">Read the whole thing</div>
    <div class="pdf-callout-sub">Thesis, lineage, method, evidence, risks, and the full claim ledger</div>
  </div>
  <a class="pdf-button" href="/research/aien-research-plan.pdf">Read the full 25-page research brief (PDF)</a>
</div>
`
  }
];
