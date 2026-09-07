# ripwire — Borrow Assessment

**Date:** 2026-09-07
**Status:** Brainstorm — Anvil opportunity assessment of ripwire, nominated on
the observation that "this looks like a version of what we ship in our graphs".
**Outcome: the observation is half right and the half that is wrong is the
important one.** ripwire's verb surface overlaps Anvil's shipped GCTX tools
almost one-for-one, but it is a different product on a different axis: a
stateless, one-shot retrieval tool optimised for token cost, versus a
daemon-backed privacy-gated projection over five joined graphs that exists to
serve enforcement and provenance. **Decline code adoption outright** (150K lines
of C++23 with 21 vendored tree-sitter grammars, inside a pure-Rust product).
**Decline dependency.** Take one primitive, and it is cheap because the
mechanism is already half-built: the **self-attesting answer envelope** —
confidence with a margin, the caps it hit and why, the cost it imposes, and
per-edge uncertainty. A second idea is arguably worth more than the first: their
**adversarial eval protocol**, where gold labels are authored from source
*before* the ranker is run, so the instrument is allowed to say the tool is
wrong. Anvil's equivalent bench currently runs on a fixture it generates itself.
**Disposition: Specification** (GCTX answer self-attestation) — filed 2026-09-07
as the `graph-answer-attestation` (GATT) module, Draft, with the eval and
cold-path threads tracked rather than filed.
**Source:** https://github.com/redhat-et/ripwire (Apache-2.0, C++23, v0.4.0
released 2026-09-06, last commit 2026-09-06, vendor: Red Hat Emerging
Technologies). Adoption metrics were not retrievable from this session — no star
or commit count is published here rather than guessed.

---

## 0. What this document is

A borrow assessment of an external repository, in the format of
[`2026-06-06-node9-borrow-assessment.md`](./2026-06-06-node9-borrow-assessment.md)
and its siblings, produced through the `anvil-opportunity-assessment` skill. The
goal is **not** to adopt ripwire but to mine it for reusable ideas, scope-guard
each one, and name the gaps. Facts were read from a clone of the public
repository at commit dated 2026-09-06 and cross-checked against
[`docs/architecture/graph-context-delivery-spec.md`](../../docs/architecture/graph-context-delivery-spec.md),
[`docs/architecture/graph-v2-foundation-spec.md`](../../docs/architecture/graph-v2-foundation-spec.md),
[`docs/guides/ai-context-delivery.md`](../../docs/guides/ai-context-delivery.md),
[`docs/reviews/2026-08-16-gctx-dogfood-failure-points.md`](../../docs/reviews/2026-08-16-gctx-dogfood-failure-points.md),
`crates/anvil-gctx-types/src/lib.rs`, and `crates/anvil-bench/src/scenarios/token_reduction.rs`.

No shared module file is edited (CIB is multi-writer — see
`plans/project-context.md#keeping-plans-current`). One APS module was filed from
this assessment on the same day — `graph-answer-attestation` (GATT), Draft — and
§9 records what was filed and what was deliberately left unfiled.

---

## Anvil Opportunity Assessment

### Executive Summary

ripwire is "the ripgrep of AI context": point one self-contained binary at a
repository and an agent gets a ranked, deterministic call graph — what to touch,
what it breaks, which tests to run — as a single token-budgeted XML bundle. No
API key, no embeddings, no index server, no daemon. Twenty-one vendored
tree-sitter grammars; a CLI first, an MCP server second.

The surface similarity to Anvil is real and close. Anvil ships
`anvil_search_symbols`, `anvil_find_callers`, `anvil_find_dependents`,
`anvil_impact_of_change`, `anvil_affected_tests`, `anvil_query_boundary` and a
gated snippet verb; ripwire ships `--for`, `--callers`, `--uses`, `--impact`,
`--situ`, `--expand`, `--pr-context`, `--from-trace`. Two teams solved the same
retrieval questions with the same vocabulary. That is parallel evolution, and it
should be cited as such rather than treated as either threat or template.

The architectures diverge underneath, and the divergence is the whole
assessment:

| | ripwire | Anvil GCTX |
| --- | --- | --- |
| State model | stateless one-shot; optional content-hash cache | `anvil-intercept` daemon keeps a warm graph; `ready`/`warming`/`stale` state machine |
| Reason for existing | token-efficient retrieval | a projection over the GV2 enforcement/provenance substrate |
| Answer shape | one composed, ranked, budgeted bundle | one verb per question, paged, identity-only by default |
| Privacy posture | reads and quotes source freely | source text is a gated escalation behind `gctx.egress` + a per-request capability |
| Graphs | one semantic call graph | five joined graphs (semantic, dependency, trust/policy, control/session, plan/provenance) |
| Evidence | published head-to-head evals, held-out labels, counterexamples | a synthetic-fixture bench scenario |

ripwire has no trust graph, no policy evidence, no session attribution, no
provenance join, and no egress boundary — it is a reader, not a governor. Anvil
should not try to win the retrieval race it is running. What Anvil should take
is the one thing ripwire does that is squarely in Anvil's own mission and that
Anvil currently does incompletely: **it makes every answer state what it does
not know.**

### Roadmap Disposition

**Specification.**

Not *Reject*: there is a concrete, bounded, high-fit primitive with a shipped
surface to land it on.

Not *APS Plan* or *Prototype*: nothing here needs to be discovered by building.
The GCTX projection contract is frozen and already carries most of the plumbing
(`RedactionSummary` with `fields_suppressed` / `snippets_truncated` /
`fully_suppressed_symbols`; `workspace_assurance.state` with a `reason`). What
is missing is a contract for the *other* class of disclosure — ranking
confidence, per-edge resolution uncertainty, and self-reported cost — and a
contract is a specification, not an experiment.

Not *Dependency Evaluation*: adopting C++ into a pure-Rust binary is not a live
option, so there is no dependency to evaluate.

The specification should be narrow: extend the GCTX egress DTOs with a
self-attestation block, and state the rule that governs it. It is an additive
change to `crates/anvil-gctx-types` and the six tool projections; it does not
touch GV2 schemas, so the "GV2 wins and this projection adapts" framing rule
holds without amendment.

### Candidate Primitive

- **Name:** Self-attesting context answer.
- **Description:** Every answer carries, in-band, four things the caller cannot
  otherwise recover: **how confident** the ranking is (ripwire emits
  `confidence="high"` with the score `margin_pct` that produced it — a flat
  ranking self-reports `low`); **what was cut and why** (`shown` / `total` /
  `capped` per section, and separately-counted truncation reasons — ripwire
  discloses three distinct module-outline truncations with individual counts and
  reasons in one figure); **what it costs** (`est_tokens` on the envelope, so the
  caller can budget before spending); and **which individual edges are guesses**
  (a call the resolver could not pin to one definition is marked per *edge*, not
  per symbol — their stated reason for the distinction is that a symbol-level
  mark "would be a lie about most of them").
- **Why it matters to Anvil:** Anvil's product claim is that its output can be
  trusted without reading its source. Every disclosure above is a claim about the
  evidence, which is Anvil's category, not a retrieval nicety. Anvil already
  discloses *redaction* (what privacy removed) and *assurance* (whether the graph
  was warm). It does not disclose *epistemic* state — whether the ranking was a
  near-tie, whether a blast-radius edge was inferred or resolved, or what the
  answer will cost the caller's context window. An assistant that receives
  "these 12 symbols are affected" cannot tell that three of those edges were
  guesses. That is exactly the failure mode the GCTX dogfood note recorded from
  the other direction: a stale graph returned `crate::mcp::tools::validate_write`
  as a `Module` identity, and the envelope had no vocabulary to say so.

### Morgan Hypothesis Assessment

- **Status:** Partially Confirmed.
- **Explanation:** "A version of what we ship in our graphs" is correct at the
  verb surface and incorrect at every layer beneath it. Confirmed: the question
  set is near-identical, both ship a CLI and an MCP server over one graph, both
  are deterministic by construction (ripwire sorts the crawl before assigning
  node IDs precisely so top-K cutoffs cannot churn between identical runs — the
  same reasoning as Anvil's determinism principle), and both ship agent skills
  that teach a harness *when* to reach for the tool. Rejected: ripwire is a
  single semantic call graph with no trust, session, or provenance layer and no
  egress boundary; it quotes source by default where Anvil's default is
  identity-only behind a flag plus a per-request capability; it is stateless
  where Anvil is daemon-warmed; and it is optimised for token cost where Anvil's
  graph is optimised to be the substrate enforcement reads. Treating ripwire as
  "our graph, done by someone else" would invite exactly the scope drift ADR-075
  and the GV2 framing rule exist to prevent — GCTX is a projection over the
  substrate, never a driver of it.

### Better Primitive Identified

**The adversarial eval instrument.** ripwire's held-out retrieval eval carries
its labelling protocol in the label file itself:

> every gold symbol below was chosen by READING the source … and deciding which
> symbol IS the on-task answer — never by running `--for` … and transcribing the
> current top ranks. The ranker was run on these queries only AFTER this file
> was complete.

The stated consequence is the point: **the eval is allowed to say the current
ranker is wrong — and it has.** They then publish a head-to-head against four
named alternatives on a frozen 560-instance public dataset (N=60 paired, zero
exclusions, same gold, *same metric code imported unmodified*), and a
counterexample section (`docs/EVALS.md` §7) that documents cases where the tool
loses to reading the file — including one where the ranked bundle would cost 5×
the file, so it serves the file instead and discloses `mode="whole-file"` rather
than doing it silently. A separate §8 lists figures that circulate informally
but could not be pinned, marked *not published*, with the reason.

Anvil's nearest equivalent, `crates/anvil-bench/src/scenarios/token_reduction.rs`,
builds its own fixture (`build_fixture`, `render_files`), measures against it,
and asserts `graph_context_beats_both_baselines` and
`reductions_clear_defensible_floors`. That is a useful regression guard on the
projection shape, and it is a self-confirming instrument for the product claim:
the corpus, the baseline and the tool all come from the same source. It cannot
tell us we are wrong.

This is the more valuable idea of the two, because it is upstream of every claim
Anvil makes about its graph, and because "we publish the cases where our tool
loses" is a governance posture, not a benchmarking technique. It is also the
harder one to adopt, which is why it is tracked in §9 rather than folded into
the specification.

### Customer Surface Test

**The capability that gets stronger: a blast radius a customer can audit.**

Anvil's buyer-facing question is not "did you find the right symbols?" — it is
"how do I know the impact set is right?". Today `anvil_impact_of_change` returns
a summary and a report; a reviewer who wants to trust it has to trust it. With
self-attestation the same answer says: this ranking was a near-tie, these three
of the eleven edges were unresolved name collisions, this list was capped at 20
of 47 and here is why, and reading it will cost you 4K tokens. That converts an
opaque assertion into reviewable evidence — the same move Anvil already made for
findings, applied to context.

Two specific customer moments improve. A developer reviewing an agent's plan can
see which parts of the map the agent was handed as guesses. An auditor asking
"what did the assistant know when it made this change?" gets an answer whose
completeness is stated rather than assumed — the caps and the confidence are part
of the record, so an exported receipt is honest about the map's limits without
anyone having to reconstruct them later.

Customer impact is **moderate-to-strong**, and honestly stated: it does not open
a new market, and no one buys Anvil for it. It removes a specific objection from
the people who evaluate Anvil most sceptically, and it is language Anvil's
existing positioning already earns.

### Criteria Scorecard

| Criterion | Score |
| --- | --- |
| Direct Anvil Fit | 7 |
| Borrowable Primitive | 9 |
| Developer-Native Usefulness | 8 |
| Evidence Before Enforcement | 9 |
| Deterministic Governance | 9 |
| Audit and Export Value | 8 |
| Narrow Beta Wedge | 8 |
| Strategic Differentiation | 5 |
| Clean-Room Feasibility | 8 |
| Buyer Language Strength | 8 |
| **Overall** | **79/100** |

Two scores carry the reasoning. **Strategic Differentiation is 5** because
retrieval quality is a crowded race that ripwire is currently winning on
published numbers, and nothing here helps Anvil win it; the differentiation is in
joining the map to the trust and provenance graphs, which ripwire does not have
and this borrow does not advance. **Direct Anvil Fit is 7, not 9**, because GCTX
is explicitly a projection and not the mission — a borrow that lands only on GCTX
is real but bounded value.

### Key Ideas Worth Exploring

1. **Confidence with a margin, not a label.** *Description:* the ranking reports
   `confidence` alongside the score margin that justifies it, and self-reports
   `low` on a flat ranking rather than presenting a near-tie as an answer.
   *Value:* an assistant can route on it — a `low` answer becomes a starting
   point instead of a conclusion. *Action:* fold into the specification.
2. **Per-edge uncertainty.** *Description:* an unresolvable call is marked on the
   edge, not on the symbol, with the explicit rationale that marking the symbol
   defames its other edges. *Value:* directly improves impact and
   affected-tests honesty; `crates/anvil-graph-cache/src/call_graph.rs` already
   makes the resolution decision, so the information exists and is discarded.
   *Action:* fold into the specification.
3. **Self-reported cost on the envelope.** *Description:* `est_tokens` on every
   bundle. *Value:* lets a caller budget before spending, and makes Anvil's own
   token-reduction claim checkable in the field instead of only in the bench.
   *Action:* fold into the specification.
4. **Truncation disclosed with a per-reason count.** *Description:* every cap
   carries `shown`/`total`/`capped` and separately-counted reasons.
   *Value:* Anvil pages with an `OpaqueCursor` and an `effective_limit`; a caller
   that ignores the cursor cannot currently tell a complete answer from a first
   page. *Action:* fold into the specification.
5. **Adversarial eval labels + published counterexamples.** *Description:* §Better
   Primitive above. *Value:* the strongest evidence-model idea in the repository.
   *Action:* track — needs a real corpus and a decision about publishing losses,
   both of which are bigger than this pass.
6. **A stateless cold path.** *Description:* ripwire indexes in ~0.31 s with no
   daemon. *Value:* not a borrow — a **warning**. The dogfood note records that
   Anvil's 60-second full-scan budget never completes on anvil-001, producing a
   restart-and-timeout loop that makes the surface look broken (CIB-341).
   ripwire is evidence that answering cold on a repository this size is not
   physically hard, which reframes that timeout as a design question rather than
   a corpus-size fact. *Action:* cite as evidence in the existing CIB-341 lane;
   do not open a new one.

### Patterns and Processes Worth Replicating

These are mechanisms from `docs/METHODOLOGY.md`, and they transfer to Anvil
independently of anything about graphs.

- **A gate that cannot observe what it asserts is worse than no gate, because it
  reports confidence.** Two shipped failures are documented: a degrade-path gate
  asserting on a diagnostic the release build compiled out (green for three
  cycles because it could not see the thing it checked), and a gate anchored on
  "the most recent commit" that went inert the moment that commit was
  documentation-only. Both are the same bug — *the measurement's precondition was
  never itself measured*. Their fixes are a **presence guard** (assert the target
  exists, then assert the property) and periodically **forcing each gate to
  fail** to confirm it can. Anvil runs a large gate suite and has the same
  exposure.
- **Capture-audit.** Periodically run *every* verb on a real repository, record
  the real output into one document, then read it as a stranger and write down
  every place the output is misleading, over-confident, or silently incomplete.
  The defect class it catches is invisible to unit tests because each output is
  individually valid: a count that reads like a total but is a floor; a `0` that
  means "not found" and will be read as "does not exist"; two verbs using one
  word for different quantities; an estimate presented with the confidence of a
  measurement. Every finding becomes a gate. Two hard-won rules come with it: the
  capture must live where the crawler skips it or it wins every query about the
  tool, and it must be regenerated against the *final* binary.
- **Sibling-completeness.** Their stated dominant defect class: a fix lands on
  one member of a family and never on its siblings — a floor marked on
  `--callers` but not `--callees`/`--uses`/`--impact`; an MCP verb rendering
  differently from its CLI sibling. "None of these is a bug in the fix. Each is a
  bug in its *scope*." Anvil has exactly this shape in triplicate: six GCTX
  tools, CLI/MCP parity, and the twelve-client activation ladder that the dogfood
  note found still iterating only two `McpClient` impls (CIB-343). Worth adopting
  as a review question — *which siblings does this fix not cover?* — in the
  Council checklist rather than as new tooling.
- **A number without provenance is not published.** `docs/EVALS.md` §8 lists
  figures that circulate informally but could not be pinned, marked *not
  published*, with the reason. Anvil's freshness-table discipline is the same
  instinct applied to documents; this applies it to numbers.

### Licensing Assessment

- **Licence:** Apache-2.0 (`LICENSE`). First-party code under `src/` is
  Apache-2.0; `third_party/` is kept byte-for-byte with original licence blocks —
  MIT (unordered_dense, svector, tree-sitter core and grammars), Apache-2.0
  (gtl), zlib (pdqsort), and doctest. `THIRD_PARTY.md` is explicit that nothing
  is relicensed.
- **Risk Level:** **Low legally, high architecturally.** No copyleft anywhere in
  the closure; Apache-2.0 carries an express patent grant, which is favourable.
  The obligations on redistribution are ordinary: retain notices, state changes,
  ship the licence.
- **Dependency Suitability:** **Unsuitable.** Anvil ships a pure-Rust binary. A
  C++23/CMake dependency would break the single-binary posture, the build story,
  and the zero-runtime-dependency claim, and would put a second parser stack
  (tree-sitter) beside the existing one.
- **Vendoring Suitability:** **Unsuitable for the same reasons**, not for legal
  ones. 150K lines of C++ across ~150 headers, with 21 vendored grammars, is a
  permanent maintenance liability inside a Rust workspace, and it duplicates a
  capability Anvil has already shipped.
- **Clean-Room Preference:** **Strongly preferred, and mostly moot.** What this
  assessment recommends taking — a disclosure vocabulary, a labelling protocol,
  three review habits — is ideas and interface design, not expression. No ripwire
  source should be consulted while implementing; `docs/METHODOLOGY.md`,
  `docs/EVALS.md` and the README wire-format description are sufficient and are
  the only artefacts anyone needs to read.
- **Notes:** If Anvil ever cites ripwire's published numbers in its own material,
  attribute them and date them — they are measurements on ripwire's corpus, and
  ripwire says so itself.

### Acquisition Strategy

- **Selected Strategy:** **Inspiration Only.**
- **Reasoning:** The capability is not the thing worth having — Anvil already
  ships it, backed by a substrate ripwire does not have. This is not a
  clean-room reimplementation, because there is nothing to reimplement: the
  borrow is a vocabulary bolted onto surfaces that already exist. It is not
  *Adapt*, because no implementation concept survives the language boundary. It
  is not *Reject*, because the disclosure primitive is genuinely good and cheap.
  Inspiration Only is the honest label, and it is the outcome this assessment
  format explicitly says is often the most valuable one.

### Anvil Integration Surface

Where the primitive would appear, in dependency order:

1. **`crates/anvil-gctx-types/src/lib.rs`** — an additive self-attestation block
   on the projection DTOs (`SearchSymbolsProjection`, `FindCallersProjection`,
   `FindDependentsProjection`, `ImpactReport`, `AffectedTestsReport`,
   `GraphEdgesProjection`), sitting beside the existing `RedactionSummary`. It
   carries no source text and no new identity, so it stays inside the
   identity-only default posture (CE-1) and needs no PV-9 re-review — but the
   spec must say that explicitly, because "the answer was a near-tie" is itself a
   fact about the workspace and should be reasoned about, not assumed harmless.
2. **`crates/anvil-graph-cache/src/call_graph.rs`** — surface the edge-resolution
   confidence the resolver already computes and currently discards.
3. **The six tools in `crates/anvil-cli/src/mcp/tools/`** — populate it. This is
   where sibling-completeness bites: partial adoption across six tools is worse
   than none, because a caller cannot tell a tool that has no caps from one that
   does not report them.
4. **`docs/architecture/graph-context-delivery-spec.md`** — the contract, as a
   scoped amendment; the frozen delivery contract gains a section, it is not
   reopened.
5. **`docs/guides/ai-context-delivery.md`** and the shipped
   `anvil-developer-functions` skill — teach assistants to *read* the confidence
   and route on it. An unread disclosure is decoration.
6. **`crates/anvil-bench`** — the eval thread, if §9's tracked item is ever
   picked up.

### Risks and Concerns

- **Scope drift into the retrieval race (highest risk).** The seductive reading
  of ripwire is "we should rank like that". Ranking quality is where ripwire has
  invested years and published numbers, and it is not what Anvil is for. The GV2
  framing rule — *Anvil-first; if assistant context and enforcement/provenance
  conflict, GV2 wins and the projection adapts* — is the guard, and this borrow
  must be held inside it. Adopting a task-shaped `--for` ranker is out of scope
  for this assessment and should not be smuggled in behind the disclosure work.
- **Competitive, not just architectural.** ripwire ships an MCP server and skills
  that install into Claude Code, Cursor, Codex, Gemini and others in one line. In
  a user's harness it competes for the same tool slots as Anvil's graph tools.
  Anvil's answer is the enforcement half (`anvil_validate_write`) that ripwire
  has no equivalent of — that is the positioning, and it should be stated
  plainly in Anvil's own materials rather than defended on retrieval quality.
- **Disclosure has a floor.** Confidence that is always `high` is worse than no
  confidence field, because it launders a guess. Any implementation needs a gate
  proving the low-confidence path actually fires on a flat ranking — which is
  precisely the "gate that cannot observe what it asserts" failure ripwire
  documents, so the gate must be written before the field.
- **The GCTX contract is frozen.** This must land as a scoped amendment with its
  own review, not as an ambient extension. ADR-095 (CLI secondary surface) is
  still Proposed over the same RPC spine, so any envelope change needs to hold
  for both surfaces or it will create the CLI/MCP sibling divergence the
  methodology section warns about.
- **The disclosure is worthless while the graph is stale.** CIB-341/342 are
  upstream of all of this: an honest confidence signal on a graph that timed out
  at 60 seconds and served cold just tells the user, precisely, that the answer
  is bad. That is better than lying, and it is not a product. Sequence the
  dogfood failures first.
- **Effort is small; attention is not.** The code change is genuinely additive
  and modest. The specification, the gates, the six-tool parity sweep, and the
  skill/doc updates are the real cost, and under-scoping them produces a
  half-disclosed surface, which is the worst outcome available.

### Final Verdict

If I were making the decision today, I would **write a scoped GCTX
self-attestation specification and file nothing else from this repository**,
because the one primitive worth having — an answer that states its own
confidence, caps, cost and per-edge guesses — lands on a surface Anvil has
already shipped and is squarely in Anvil's evidence category, while everything
else ripwire does well is either already built here on a stronger substrate or
belongs to a retrieval race Anvil should not enter.

---

## 9. Suggested follow-ups

> **Filing update (2026-09-07):** thread 1 was filed the same day as the
> `graph-answer-attestation` (GATT) module — Draft, six work items, three Ready
> gates open. Two corrections were forced by reading the code while planning it,
> and they narrow the borrow: `anvil_search_symbols` is conjunctive filters with
> no ranker, so **ranking confidence has nothing to attach to and was dropped**;
> and `RedactionSummary` already carries a pre-cap `matched` total, so the
> "count that is really a floor" defect is **already solved** on that tool and
> the parity work is about the report DTOs instead. Threads 2–5 remain unfiled.



Thread 1 is filed (see the update above). The rest are named for a bookkeeping
or planning pass; **no CIB row is written by this document** — CIB is
multi-writer and is reconciled on a bookkeeping branch only
(`plans/project-context.md#keeping-plans-current`).

| # | Thread | Suggested home | Notes |
| --- | --- | --- | --- |
| 1 | GCTX answer self-attestation contract (caps with reasons, `est_tokens`, per-edge uncertainty) | **Filed 2026-09-07** as [`graph-answer-attestation`](../modules/graph-answer-attestation.aps.md) (GATT), Draft, 0/6 | The recommended disposition. Ranking confidence was dropped on inspection — `anvil_search_symbols` has no ranker, so there is nothing for it to attest to. Gates are written before the fields they guard (GATT-002). |
| 2 | Real-corpus retrieval eval with source-authored held-out labels | Track — needs a corpus decision first | Replaces nothing; sits beside the existing synthetic `token_reduction` scenario, which stays as a shape regression. |
| 3 | Publish counterexamples where graph context loses to reading the file | Track — product/positioning call, not an engineering one | Highest-trust, highest-nerve item here. |
| 4 | Sibling-completeness as a standing Council review question | Council checklist | "Which siblings does this fix not cover?" — no tooling required. |
| 5 | Cite ripwire's cold-start as evidence in the CIB-341 full-scan-timeout lane | Existing CIB-341 | Evidence only; do **not** open a parallel lane. |

## 10. Sources

- `redhat-et/ripwire` at the 2026-09-06 commit — `README.md`,
  `docs/ARCHITECTURE.md`, `docs/METHODOLOGY.md`, `docs/EVALS.md`,
  `THIRD_PARTY.md`, `LICENSE`, `CHANGELOG.md`, `skills/`, `src/`.
- Anvil: [`graph-context-delivery-spec.md`](../../docs/architecture/graph-context-delivery-spec.md),
  [`graph-v2-foundation-spec.md`](../../docs/architecture/graph-v2-foundation-spec.md),
  [`ai-context-delivery.md`](../../docs/guides/ai-context-delivery.md),
  [`2026-08-16-gctx-dogfood-failure-points.md`](../../docs/reviews/2026-08-16-gctx-dogfood-failure-points.md),
  `crates/anvil-gctx-types/src/lib.rs`, `crates/anvil-cli/src/mcp/tools/`,
  `crates/anvil-bench/src/scenarios/token_reduction.rs`.
