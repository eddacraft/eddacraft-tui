# Language and Coverage Design Spec

> Defines which programming languages, governance surfaces, and semantic packs
> Anvil should govern, and at what depth. Replaces the stale `lang-*` placeholder
> modules in `plans/index.aps.md` with a coherent, criteria-driven coverage
> strategy grounded in real early-access user stacks.

**Date:** 2026-04-08
**Last refreshed:** 2026-04-19
**Status:** Accepted in principle (pending §10 + §17 actions)
**Relates to:** All `lang-*` placeholder modules in `plans/modules/`, the `### Multi-Language Support (Draft)` section of `plans/index.aps.md`, and the `### Task Status — Multi-Language (Draft)` section of `plans/index.aps.md`
**Supersedes:** The Multi-Language Support section of `plans/index.aps.md` and all ten `lang-*.aps.md` placeholders (to be rewritten, merged, or archived per §10)

## Refresh log — 2026-04-19

Eleven days after the original draft. Changes:

1. **New early-access user (User C) added to §3.3** — an almost-pure Python
   stack. Bumps Python demand from 1 → 2 and resolves the §12.1 open
   question. Python is now a firm anchor, held in its existing sequence
   position (third, after TS audit and Rust T3) — User C raises Python
   *confidence*, not Python's relative priority versus Rust.
2. **Python-substrate semantic packs re-scored** (§8.4): Django and FastAPI
   each gain demand = 1 (unconfirmed which framework User C uses; treated
   as a floor until implementation discovery). The Python-substrate LLM
   Provider extension (§12.9) gains demand = 1.
3. **Factual corrections applied** from the 2026-04-08 council review
   (§16): C-001 Drizzle demand corrected 2 → 1; C-002 Dart marker in §11
   corrected ✅ → 🟡 with the headline moving from "9 of 14" to "8 of 14".
4. **Hard Phase 1 line added to §9** per council finding C-011/C-021:
   TS audit + SQL T2 + Pulumi pack + LLM Provider pack (warn-only) is the
   MVP; everything else is Phase 2+.
5. **LLM Provider pack** (§8.4.1) explicitly scoped as **warn-only by
   default** per council finding C-010, removing the fail-safe concern.
6. **Open questions updated** (§12): 12.1 resolved, 12.9 promoted from
   design-hedge to "plan this now".
7. The deeper architectural amendments from §16.5 (kernel prerequisite
   work, pack architecture decision, Rust T3 enforcement location, drift
   schema versioning, feature-flag rollout, acceptance-bar revision) are
   **not inlined here** — they belong in the implementation plans
   produced downstream per §15. §16 remains the canonical record of the
   council session; amendments that reshape §1–15 are listed in §17.

---

## 1. Purpose

Decide which languages, file types, and semantic domains Anvil should be able
to govern, and at what depth.

The ten placeholder `lang-*` modules in `plans/index.aps.md` were drafted under
outdated assumptions: regex-based parsing, an `HTMLCSS-001` prerequisite that
has since been archived, and a mental model where every additional "thing to
support" looked like "add a new language". They need to be replaced with a
coverage strategy that:

1. Matches what early-access users (including Anvil itself) actually have in
   their repos.
2. Acknowledges that "coverage" is not a single thing — it is three different
   shapes of work with different costs and different value curves.
3. Ranks candidates honestly against demand, blast radius, strategic value, and
   pack-unlock potential.
4. Produces a roadmap users can read and feel heard by — "yes, your stack is
   coming, here is where".

## 2. Non-Goals

- Prose-quality linting of markdown (spelling, style, grammar).
- Replacing type checkers (mypy, pyright, rust-analyzer, tsc). Anvil is
  governance, not type-checking. Play alongside these, not instead of them.
- Virtualenv / pip / poetry / conda / Cargo dep-graph analysis as part of any
  language anchor. Dependency-graph intelligence lives in the separate
  `config-intelligence` module already in the index.
- Framework-specific patterns baked into language anchors. Framework rules
  belong in semantic packs, not anchors. This keeps the boundary clean.
- Any commitment to calendar dates, quarters, or weeks. Sequencing in this
  design is ordinal ("first", "then", "unblocks after X"), not timed.

---

## 3. Background: the current state

### 3.1 What Anvil parses today

The kernel parses TypeScript, TSX, JavaScript, and JSX via tree-sitter. From
`crates/anvil-kernel/src/parser/languages.rs`:

```rust
pub enum Language {
    TypeScript,
    Tsx,
    JavaScript,
    Jsx,
}
```

Symbol and import extraction lives in `crates/anvil-kernel/src/parser/extract.rs`
and handles function declarations, class declarations, imports, exports, and
CommonJS `require()` patterns.

Anti-pattern checks exist in `crates/anvil-checks/`. Architecture/boundary
analysis exists in `core/src/architecture/`. These pieces exist but their
wiring to TS specifically is not audited — see §7.3.

### 3.2 What the placeholder modules claim

`plans/index.aps.md:220-236` lists ten `lang-*.aps.md` placeholders (Python,
Go, Rust, Java, Kotlin, .NET/C#, Dart, Swift, C/C++, Zig), all in Draft, all
citing `HTMLCSS-001` as a prerequisite, all sized at 3-6 tasks each, all
assuming regex-based import extraction. These assumptions are stale. The
tree-sitter-based kernel changes the implementation shape entirely.

### 3.3 Real user stacks (as known at time of writing)

**Anvil itself** (2,206 tracked files):

| Category | Files | LoC (where measured) |
|---|---|---|
| TypeScript (.ts + .tsx) | 911 (~41%) | 189,759 |
| Markdown (docs, plans, agent instructions) | 762 (~35%) | 173,136 |
| Rust (.rs) | 198 (~9%) | 63,538 |
| JSON (configs, fixtures) | 122 | — |
| YAML (CI, configs) | 38 | — |
| Vitest snapshots (.snap) | 34 | — |
| `.anvil` format | 23 | — |
| Shell scripts | 18 | — |
| TOML (Cargo, configs) | 13 | — |
| Rego (OPA policies) | 6 | — |
| CSS | 4 | — |
| SQL migrations | 3 | — |
| Python | 1 | — |

Key observations: Anvil's own kernel is Rust and Anvil currently cannot see
it. Markdown is the single largest file-count category and is completely
invisible to current analysis. The polyglot long tail is small by count but
each file has high blast radius per occurrence (one SQL migration, one
Dockerfile, one GitHub Actions YAML can do enormous damage).

**User B** (polyglot early-access user):

TypeScript, Rust, Python, Pulumi (as a TypeScript library), Postgres
(migrations + Drizzle ORM), Dart (mobile), Zod (validation), OpenAI API
(calls to LLM providers), Next.js, React Query, Tokio.

Twelve distinct coverage concerns for a single user. User B is used
throughout this design as an explicit validation case — see §11.

**User C** (added 2026-04-19, almost-pure Python stack):

Python-first. The specific web framework (Django vs FastAPI) is not yet
confirmed — design treats both as substrate candidates with demand = 1
until implementation discovery pins it down. Python-side LLM provider
calls (`openai`, `anthropic`, `langchain` family) are in scope — this is
the demand signal that promotes the Python-substrate LLM Provider
extension from "design leans same pack" to "plan this now" (§12.9).

User C's impact on the design:

- Python gains a second independent demand point (§7.2), resolving
  the §12.1 open question. Python's position in the strict sequence
  remains below Rust — User C raises Python *confidence*, not Python's
  relative priority versus Rust's "governs systems code" strategic
  unlock.
- Django and FastAPI each gain demand = 1 (§8.4), moving from 0 to
  "substrate-gated but demanded" on the semantic-pack roadmap.
- The Python-substrate LLM Provider pack extension (§8.4.1) gains a
  concrete demand point rather than being purely aspirational.

---

## 4. Coverage is three axes, not one

The original placeholders treated this as a flat list of "languages to
support". That framing breaks as soon as SQL or Pulumi are taken seriously.
Coverage decomposes into three axes:

| Axis | What it is | Implementation shape | Examples |
|---|---|---|---|
| **A. Programming language** | A source language Anvil parses into a symbol/import graph and reasons about structurally | Tree-sitter grammar + symbol/import extraction in `crates/anvil-kernel/src/parser/` | TS, JS, Rust, Python, Go |
| **B. Governance surface** | A file type Anvil scans for patterns, secrets, and policy violations without needing a symbol graph | Pattern catalogues + policy hooks; no parser required | SQL migrations, Dockerfile, GitHub Actions YAML, shell, Terraform |
| **C. Semantic pack** | Domain knowledge layered on a language Anvil already parses | Anti-pattern + policy bundles tied to specific imports/symbols of an underlying language | Pulumi (on TS/Python/Go), Next.js (on TS), Django (on Python), Tokio (on Rust) |

These axes are not interchangeable and do not compete for the same engineering
work. A new programming language is months of parser/extractor/architecture
work. A new governance surface is days to weeks of pattern-catalogue work. A
semantic pack is days to weeks of domain rules on top of an existing parser.

Treating them as one list leads to bad sequencing. Treating them as separate
tracks lets each axis be paced according to its own cost/value curve.

**A fourth shape — markdown as governance artefact — does not fit any of the
three axes cleanly and is handled separately in §8.5.**

---

## 5. Tier definitions

Each axis has its own tier ladder. "Supported" means different things in each.

### 5.1 Programming languages (Axis A)

| Tier | Name | What ships |
|---|---|---|
| T1 | Parsed | Tree-sitter grammar wired in `parser/languages.rs`; file detected; basic symbol extraction (functions, classes, imports); appears in the symbol graph; no language-specific anti-patterns |
| T2 | Linted | T1 + curated anti-pattern catalogue (5-10 patterns) + comment-suppression syntax + entry-point detection |
| T3 | Governed | T2 + layer/boundary enforcement + policy hook integration + drift baseline + included in `architecture-validate` |
| T4 | Packed | T3 + at least one semantic pack on top |

### 5.2 Governance surfaces (Axis B)

| Tier | Name | What ships |
|---|---|---|
| T1 | Scanned | File recognised; existing secret scanner runs; basic destructive-pattern catalogue (e.g. `DROP TABLE`, `acl=public-read`, `chmod 777`) |
| T2 | Policy | T1 + suppression syntax + policy hook + drift baseline |
| T3 | Resource-aware | T2 + structured model (SQL schema graph, Terraform resource graph, Actions workflow graph) |

### 5.3 Semantic packs (Axis C)

Packs are binary — either the pack exists or it does not. Each pack declares:

- Its **substrate language** (the language it analyses)
- Its **minimum substrate tier** (the tier the substrate must be at for the
  pack to be viable — most useful packs require substrate ≥ T2, some require
  T3)

### 5.4 Markdown (special)

Markdown is governed under its own mini-ladder because it is neither a
programming language nor a typical governance surface. See §8.5.

| Tier | Name | What ships |
|---|---|---|
| M1 | Structural | APS wellformedness + cross-reference integrity |
| M2 | Claim hygiene | M1 + stale claim detection + decision record hygiene |
| M3 | Capability-aware | M2 + agent capability-manifest integration (depends on AGOV-007) |

---

## 6. Prioritisation criteria

Every candidate (language, surface, or pack) is scored on the same four
criteria. No hidden tiebreakers, no "but it would be nice to have".

| Criterion | Weight | What it measures | How it is evaluated |
|---|---|---|---|
| **Demand** | High | How many users, including Anvil itself, actively have this in their repo. Anvil counts as one user. | Count of confirmed users with this in their stack. Confirmed = mix data, not speculation. |
| **Blast radius** | High | How much damage one ungoverned file in this category can do. A SQL migration outranks a CSS file by orders of magnitude. | Ordinal: low / medium / high / critical. Calibrated against examples (CSS=low, Dockerfile=high, SQL migration=critical, IAM-touching Terraform=critical). |
| **Strategic** | High | Does this unlock a market/positioning story Anvil wants to be able to tell? E.g. "Anvil governs systems code" needs Rust; "Anvil governs AI/ML stacks" needs Python and an LLM-provider pack; "Anvil governs infrastructure as code" needs Terraform/Pulumi. | Ordinal: none / supports / unlocks. |
| **Pack unlock** | Bonus | Does this open the door to one or more high-value semantic packs that would not otherwise be viable? | Count of named packs it would enable, capped at +2. |

**Explicitly not criteria** (these inform sequencing within a track but not
ranking):

- **Cost.** Rank by value first, then ask "can we afford the top of the list".
  Cost is a sequencing question, not a ranking question.
- **Dogfooding.** Anvil-ness shows up as Demand=1 (Anvil counts as a user)
  and as Strategic where applicable. It is not a separate criterion — that
  would double-count.
- **Effort symmetry.** "It's only one extra grammar" is not a reason to add
  a language.

Scoring is qualitative — no false precision. Each candidate gets Demand
(count) + Blast (low/med/high/critical) + Strategic (none/supports/unlocks)
+ Pack bonus (0/+1/+2). Items are sorted within each track using these four
together; ties break on blast radius first, then strategic.

---

## 7. The anchor set

### 7.1 Definition

The anchor set is the small group of programming languages Anvil commits to
bringing to T3 (T4 if a pack lands on top). These are the languages where
Anvil tells a complete governance story. Everything not in the anchor set
either sits in the tail at T1 or is not a programming language at all.

### 7.2 The anchors

| Anchor | Demand | Blast | Strategic | Pack unlock | Rationale |
|---|---|---|---|---|---|
| **TypeScript / TSX / JavaScript / JSX** | 2 confirmed (Anvil, User B); assumed universal across early access | High | Unlocks (existing positioning) | +2 (Pulumi, Next.js, Drizzle, Hono, LLM Provider — effectively +5, capped to +2 for scoring hygiene) | The language Anvil already partially supports. **Not actually at T3 today.** First anchor work item is closing that gap before any new language is added. |
| **Rust** | 2 (Anvil + User B) | High | Unlocks ("Anvil governs systems code") | +1 (Tokio) | Two independent demand confirmations. Anvil's own kernel is in Rust. Strategic: Rust is the credibility test for "governs systems code". |
| **Python** | 2 (User B + User C) | Medium-high | Unlocks ("Anvil governs AI/ML stacks") | +2 (Django, FastAPI) | Two independent demand points as of 2026-04-19. User C is an almost-pure Python stack (§3.3) and resolves the original single-user-demand risk. Python remains the strongest strategic unlock on the anchor list — AI/ML stacks and Python-substrate LLM Provider coverage (§12.9) both land here. Sequencing note: Python stays below Rust in the strict anchor order (§9 step 10) because Rust's "governs systems code" strategic narrative is still the higher-ROI next anchor after TS; the bump in Python demand raises confidence in committing to Python, not its ordering. |

**Explicitly not in the anchor set:** Go, Java, Kotlin, C#/.NET, C/C++,
Dart, Swift, Zig. Zero confirmed anchor-level demand. They sit in the tail
(§8.2) at T1 or are cut entirely.

### 7.3 The TS audit (anchor work item zero)

Before any Rust or Python anchor work begins, the design commits to a
calibration pass on TS itself. Current state, based on code present in the
repo:

| T3 capability | Status | Gap |
|---|---|---|
| Tree-sitter grammar wired | ✅ `parser/languages.rs` | None |
| Symbol/import extraction | ✅ `parser/extract.rs` | Possibly incomplete (dynamic imports, re-exports, namespace exports) |
| Anti-pattern catalogue | ✅ Runtime checks exist | Enumerate and confirm parity with the bar for a new anchor |
| Comment-suppression syntax | ✅ Exists | Confirm coverage |
| Entry-point detection | ✅ Partial | Confirm |
| Layer/boundary enforcement | ✅ `core/src/architecture/` | Confirm it reaches TS specifically |
| Policy hook integration | ✅ OPA pipeline exists | Confirm reachable from TS pipeline |
| Drift baseline | ✅ Exists | Confirm enabled by default for TS |
| `architecture-validate` inclusion | ✅ Exists | Confirm |

TS is very close to T3. The audit may be small. But the audit itself is the
work item, and it produces three outputs:

1. A definitive statement of TS's current tier.
2. A list of the specific gaps to close, if any.
3. A **T3 acceptance checklist** that becomes the bar Rust and Python must
   hit afterwards — calibrated against code that exists, not against
   aspirational definitions.

The third output is the critical one. Without it, "T3" is a made-up label.
With it, "Rust at T3" means something concrete and verifiable.

The TS audit also folds in Zod-creep rules (`z.any()`, `z.unknown()`,
`.passthrough()` over-exposure) into the TS T2 anti-pattern catalogue
alongside existing rules for `any`, `as any`, `@ts-ignore`. Zod patterns
live here because Zod is cross-cutting infrastructure used by every TS
framework — it does not deserve a standalone pack but it does deserve
language-level attention.

---

## 8. The five tracks

The design runs as **five parallel tracks**. Parallel in the sense that they
do not block each other, not in the sense that they receive equal engineering
attention.

```
TRACK 1: ANCHORS          → TS audit → Rust → Python         (heavy, sequenced)
TRACK 2: TAIL             → single T1 batch wave              (light, one-shot)
TRACK 3: GOV SURFACES     → SQL → GH Actions → Dockerfile → … (medium, demand-ordered)
TRACK 4: SEMANTIC PACKS   → Pulumi → Drizzle → Next.js → …    (medium, demand-pulled)
TRACK 5: MARKDOWN         → APS + cross-ref integrity         (special, small)
```

Each track has its own acceptance criteria and its own failure mode. Ruthless
track-level acceptance is what prevents the "everything reaches 80%, nothing
reaches 100%" failure.

### 8.1 Track 1 — Anchors

**Scope.** Bring TS → T3, then Rust → T3, then Python → T3. Strictly
sequenced — not because the work cannot parallelise, but because each language
validates the T3 acceptance checklist and surfaces infrastructure gaps for
the next one. Also because anchor work is engineering-heavy and parallelising
two anchors risks neither shipping.

**Sub-sequence.**

1. **TS audit + gap-close** (anchor item zero, §7.3). Produces the
   authoritative T3 acceptance checklist. Nothing else in Track 1 starts
   until this completes.
2. **Rust → T3.** Grammar already exists (`tree-sitter-rust`). Focus is
   extraction (Rust has `mod` / `use` / `pub` / `crate::` shapes that do not
   map cleanly to the current JS-shaped extractor), anti-pattern catalogue
   (see below), layer enforcement across crates.
3. **Python → T3.** Grammar exists (`tree-sitter-python`). Extraction must
   handle `import` / `from import` / relative imports / namespace packages.
   Anti-pattern catalogue (see below). Entry points (`if __name__ ==
   "__main__"`, `pyproject.toml`, `setup.py`).

**Rust anchor T2 anti-pattern catalogue** (non-exhaustive):

- `unwrap()` / `expect()` in non-test code
- `unsafe` blocks without safety comment
- `.clone()` in hot loops (flaggable, not blocking)
- `todo!()` / `unimplemented!()` shipped
- `panic!()` in library code
- **Serde deserialisation hygiene** — `#[serde(deny_unknown_fields)]` missing
  on external-input structs, `#[serde(flatten)]` without validation,
  `Deserialize` on types containing secret fields, custom `deserialize_with`
  without bounds. Serde is folded into the Rust anchor rather than being its
  own pack because it is so ubiquitous in Rust that for governance purposes
  it is part of the language.

**Python anchor T2 anti-pattern catalogue** (non-exhaustive):

- `# type: ignore` without justification comment
- `# noqa` without specific rule
- Bare `except:` / `except Exception: pass`
- `Any` type usage from `typing`
- `print()` in production code
- `# pylint: disable` without justification
- Import star (`from foo import *`)

**Acceptance per anchor.** Passes the T3 checklist produced by the TS audit.
No partials. "Rust is at T3 but architecture enforcement is not wired yet"
is not Rust-at-T3.

**Failure mode this prevents.** Shipping "Rust support" that means "we parse
`.rs` files and put them in the graph" while policy, drift, and architecture
quietly do not apply. That would be a Tier 1 deliverable labelled as Tier 3,
and it would burn trust with User B fast.

### 8.2 Track 2 — Tail T1 wave

**Scope.** One batched sprint that brings a set of tail languages to T1
simultaneously: grammar wired, file detected, symbol graph inclusion. Then
the wave ends. No per-language anti-pattern catalogues, no suppression
syntax, no policy hooks. Just "appears in the graph".

**Candidates** (sorted by plausible future promotion):

| Language | Tree-sitter grammar | Demand | Pack potential | Promotion lever |
|---|---|---|---|---|
| Dart | `tree-sitter-dart` | 1 (User B mobile) | Flutter | Second Dart user or Flutter pack demand |
| Go | `tree-sitter-go` | 0 | Cobra, `net/http` | First Go user |
| Java | `tree-sitter-java` | 0 | Spring | First Java user |
| C# / .NET | `tree-sitter-c-sharp` | 0 | ASP.NET | First .NET user |
| Kotlin | `tree-sitter-kotlin` | 0 | Ktor, Android | First Kotlin user |
| C / C++ | `tree-sitter-c`, `tree-sitter-cpp` | 0 | — | First systems-code user |

**Cut from the wave:** Swift, Zig. Zero demand, no plausible near-term
users, would dilute the batch. Re-enter only with a demand signal.

**Rationale for batching.** The per-language work is genuinely small:
grammar crate + `Language::from_path` match arm + extraction function.
Batching amortises the test harness, the fixtures, and the "does the graph
actually include this" validation. Doing them one at a time later costs more
in aggregate.

**Acceptance.** The wave is not done until all batched languages parse
real-world files without panicking, appear in the graph via
`architecture-validate`, and have at least one fixture test each. No
half-batches. If C/C++ turns out to be a swamp, drop it from the wave rather
than letting the wave stall.

**Failure mode this prevents.** A tail that rots. Either the T1 wave ships
cleanly, or the tail is explicitly closed until demand pulls something into
promotion.

### 8.3 Track 3 — Governance surfaces

**Scope.** Bring named file types to T2 or T1 depending on per-candidate
scoring. This track is pattern-catalogue work, not parser work.

**Ranked list:**

| # | Surface | Target tier | Demand | Blast | Strategic | Notes |
|---|---|---|---|---|---|---|
| 1 | **SQL migrations** | T2 | 2 (User B Postgres + Anvil) | **Critical** | Unlocks (data governance) | Ranked #1 over GH Actions because the pack-unlock bonus (Drizzle pack layers directly on SQL governance) gives SQL a higher composite score despite lower confirmed demand. Patterns: `DROP TABLE`, `DROP COLUMN` without guard, `TRUNCATE`, `DELETE` without `WHERE`, `ALTER TABLE … DROP CONSTRAINT`, unversioned migrations, schema+data in one transaction, missing `IF NOT EXISTS` / `IF EXISTS` guards. |
| 2 | **GitHub Actions YAML** | T2 | 2 confirmed (Anvil, User B); assumed universal | **Critical** | Unlocks (supply chain) | Patterns: `pull_request_target` with write permissions, `workflow_run` reaching into forks, unpinned `@main` / `@master` action refs, `secrets:` in `env:` passed to untrusted code, write permissions on default `GITHUB_TOKEN`, self-hosted runners on public repos. |
| 3 | **Dockerfile** | T2 | 3 | High | Supports | Patterns: `ADD https://…`, `RUN curl … \| sh`, `:latest` base images, running as root, `sudo` in containers, layered `apt-get` without `--no-install-recommends`, build secrets in layers. |
| 4 | **Shell scripts** | T1 | 2 (Anvil + User B) | High | Supports | Extends existing `command_safety` runtime check to static `.sh` analysis. Patterns: `rm -rf /`, `curl \| sh`, unquoted variables in destructive contexts, `eval` on user input. |
| 5 | **`.env` files** | T1 | 2 confirmed (Anvil, User B); assumed universal | Critical | Supports | Mostly covered by existing secret scanner — structural checks are the delta: committed `.env`, `.env` not in `.gitignore`, production values in non-prod files. |
| 6 | Terraform / HCL | T1 | 1 (indirect via Pulumi) | Critical | Supports | Mostly deferred — the Pulumi pack in Track 4 covers the bulk of User B's infra story. T2 promotion lever: a direct-Terraform user or a strategic "govern HCL" story. |
| 7 | k8s YAML / Helm | T1 | 0 | High | None | Deferred. No current demand. Promotion lever: first k8s-native user. |

**Cut for now:** CloudFormation, Bicep, Ansible, Jenkins Groovy, Buildkite
YAML, CircleCI YAML. Zero demand signals. Re-scoreable later.

**Acceptance per surface.** Pattern catalogue exists, suppression syntax
works for that comment style (`-- @anvil-ignore`, `# @anvil-ignore`, etc.),
zero false positives on Anvil's own repo before shipping to other users,
drift baseline can be captured.

**Failure mode this prevents.** A surface shipped with a noisy catalogue
that forces users to mass-suppress on day one. The "zero false positives on
Anvil's own files" bar means we eat our own dogfood before each surface
goes out.

### 8.4 Track 4 — Semantic packs

**Scope.** Domain-specific packs layered on anchor languages. Each pack
declares its substrate language and the minimum substrate tier it requires.

**Ranked list:**

| # | Pack | Substrate | Min substrate tier | Demand | Blast | Strategic | Notes |
|---|---|---|---|---|---|---|---|
| 1 | **Pulumi** | TS | T3 | 2 (User B + Anvil `infra/`) | **Critical** | Supports | First pack. Unlocks when TS audit (Track 1 item 0) completes. Catches: `acl: "public-read"` on S3, wide IAM trust policies, `versioning` disabled on state-holding resources, stack-crossing resource references, hardcoded secrets in resource definitions. |
| 2 | **Drizzle** | TS | T3 | 1 (User B) | **Critical** | Supports (data governance) | Demand corrected per council finding C-001 — Anvil does not use Drizzle (`apps/anvil-api` uses raw SQL via NeonClient). Still ranked #2 on blast radius: a `.delete()` without `.where()` ships production data loss. Patterns: `.delete()` without `.where()`, `.update()` without `.where()`, raw `sql` template interpolation of user input, missing transactions around multi-statement operations, schema drift between `schema.ts` and actual migrations, `.execute()` on prepared statements without input validation. |
| 3 | **Next.js** | TS | T3 | 2 (User B + Anvil `apps/website`) | Medium-high | Supports | Patterns: raw HTML insertion via React's dangerous prop without sanitisation, Server Components leaking secrets via props, `revalidate` misconfigurations, middleware matching root routes, client components with server-only imports, server actions without Zod validation. |
| 4 | **LLM Provider** | TS (Python later) | T3 | 1 (User B) | **Critical** | **Unlocks** (AI governance) | Targets `openai`, `@anthropic-ai/sdk`, `ai` (Vercel AI SDK) imports. Catches: PII in prompt construction, hardcoded system prompts conflicting with policy, uncapped `max_tokens`, unsanitised response rendering (injection-shaped risks), missing streaming cancellation, tool calls without JSON schemas. **Strategically the most important pack on this list** — see §8.4.1. |
| 5 | **Hono** | TS | T3 | 1 (Anvil `apps/anvil-api`) | **High** | Supports | Patterns: routes missing auth middleware, `c.req.parseBody()` without size limits, CORS with `origin: '*'`, `c.html()` with interpolated values, error handlers leaking stack traces, `c.env.SECRET` without typed Bindings, route order bugs (`app.get('*')` before specific routes), unvalidated `c.req.param()` / `c.req.query()` consumed into DB queries, missing `@hono/zod-validator` on body-accepting routes. |
| 6 | **Tokio** | Rust | T2+ | 2 (Anvil kernel + User B) | High | Supports | Unblocks when Rust reaches T2 in Track 1. Catches: blocking calls in async context, `.await` on held locks, unbounded channels, `tokio::spawn` without `JoinHandle` tracking, missing `select!` cancellation branches. |
| 7 | Django | Python | T2+ | 1 (User C floor) | High | Unlocks (AI/ML) | Strategic AI/ML holder. User C is a Python-first stack but the specific framework is not yet confirmed — Django and FastAPI each carry demand = 1 as a floor until implementation discovery pins it to one of them (at which point the other drops back to 0). Unblocks when Python reaches T2. |
| 8 | FastAPI | Python | T2+ | 1 (User C floor) | High | Unlocks (AI/ML) | Same floor-of-1 logic as Django. One of these becomes demand = 1 and the other becomes 0 once User C's framework is confirmed. |
| 9 | Axum | Rust | T2+ | 0 | Medium | Supports | Deferred. |

**Cut for now:** Express, NestJS, Flask, Spring, Rails. Low or zero current
demand, no strategic fit.

**Near-miss (worth tracking):** **tRPC.** Nearly added by Anvil and User B
but not currently in either stack. Re-scores as a Track 4 candidate if
either actually ships it.

**Pack-count-per-substrate force multiplier:**

| Substrate | Anchor investment | Packs unlocked | ROI |
|---|---|---|---|
| TS (T3 audit, small) | Low | **5** (Pulumi, Drizzle, Next.js, LLM Provider, Hono) | Very high |
| Rust (T2+) | Medium | 1 (Tokio), +1 deferred (Axum) | Medium |
| Python (T2+) | Medium | 0 current demand, 2 strategic (Django, FastAPI) | Medium |

This is the strongest argument for "TS audit first, then Rust, then Python":
the TS audit alone unlocks five packs. No other anchor investment has
comparable pack ROI.

**Acceptance per pack.** Substrate is at required tier. Pack has at least
five rules. Pack is tested against a real-world codebase (Anvil's `infra/`
for Pulumi, Anvil's `apps/website` for Next.js, Anvil's `apps/anvil-api` for
Hono, etc.). False-positive rate acceptable on the test codebase.

**Failure mode this prevents.** Packs built against an immature substrate.
A Django pack on a Python parser at T1 cannot see enough to be useful — it
would have to reimplement Python analysis inside the pack, which is the
wrong abstraction layer.

#### 8.4.1 On the LLM Provider pack specifically

The LLM Provider pack deserves a dedicated note because it may be the
single most strategically valuable item in this entire design.

- **Warn-only by default** (per council finding C-010, 2026-04-08). Static
  PII detection in LLM calls is heuristic — a false positive could break a
  production call path, not just a build. The pack ships in warn-only mode
  so findings surface without blocking exit codes. Projects that want
  hard-fail behaviour opt in per-rule via the standard policy-hook mechanism
  once their own FP profile is known. This also removes the §6 methodology
  concern where "Unlocks" was scored twice (strategic + sequencing) —
  warn-only behaviour is now the explicit reason the pack can ship on thin
  demand without risking user call paths.
- **Strategic = Unlocks.** Only two items on this entire design score
  "Unlocks" on strategic: Python (for AI/ML anchor) and the LLM Provider
  pack. The LLM Provider pack alone proves the AI/ML governance story from
  the TS side without waiting for Python to reach T3. That is a meaningful
  chunk of calendar time saved on the strategic narrative, and it happens on
  the already-highest-ROI substrate.
- **Demonstrability.** The LLM Provider pack is the single pack that can be
  demo'd to a prospect live: "paste your Vercel AI SDK code, Anvil flags the
  PII going to the provider." Very concrete. Hard to wave off as "just a
  linter".
- **Positioning defensibility.** PII detection in LLM calls is obviously
  governance, not code quality. The LLM Provider pack is Anvil's hardest
  argument against the "Anvil is just a fancy linter" framing.
- **Aligned with the vision doc.** `docs/vision/anvil-vision.md` opens with
  *"Anvil exists to ensure that AI and humans cannot produce unsafe
  software."* The LLM Provider pack is that sentence made concrete.
- **Python-substrate extension is now demand-backed** (updated 2026-04-19).
  User C's Python-first stack (§3.3) includes Python-side LLM calls
  (`openai`, `anthropic`, `langchain`). That promotes the Python-substrate
  LLM Provider extension from the §12.9 design hedge ("same pack, multiple
  substrates — TBD") to a concrete Phase 2 deliverable that unblocks as
  soon as Python reaches T2. The TS-first LLM Provider pack remains the
  Phase 1 deliverable; the Python extension follows Python-T2 completion
  (§9 step 10) and reuses the same rule catalogue where semantics align.

The spec's implementation plan (produced in the next phase) should treat the
TS LLM Provider pack as a priority Phase 1 deliverable (§9 Phase 1 line) at
demand count 1. The Python extension is Phase 2, gated on Python T2.

### 8.5 Track 5 — Markdown

**Scope.** Markdown is its own track because it fits none of the other
axes. Not a programming language (no symbol graph), not a typical governance
surface (pattern catalogues alone miss the point), not a pack (no
substrate). Markdown in Anvil's world is **governance artefacts written in
prose**. 762 files, ~173k LoC in Anvil's own repo alone, almost all
load-bearing.

**What Track 5 analyses:**

1. **APS plan wellformedness.** `plans/modules/*.aps.md` files conform to
   the APS schema. Missing headers, broken status transitions, orphaned
   work-item IDs, duplicated IDs, cross-module reference drift. This is
   essentially the `aps-planning` skill logic promoted to a check.
2. **Agent prompt / skill capability declaration.** Files under `.claude/`,
   `.codex/`, `skills/` — do they declare their capabilities? Do they
   reference tools that do not exist? Do they violate the capability
   manifest (AGOV-007)?
3. **Stale claim detection.** Public docs (`docs/public/anvil/`) referencing
   features that no longer exist, commands that have been renamed, URLs
   that now 404, version numbers that have moved on.
4. **Cross-reference integrity.** Markdown `[link](path)` references to
   files that do not exist. `plans/index.aps.md` references to modules that
   have been archived without updating the index.
5. **Secrets in code blocks.** Existing secret scanner covers this, but
   markdown is where "paste your API key here" examples tend to sneak in —
   making it explicit.
6. **Decision record hygiene.** `plans/decisions/NNN-*.md` — numbered
   contiguously, dated, statused.

**Not in scope for Track 5:**

- Grammar, spelling, style (that is editorial, not governance)
- Rendering correctness
- Markdown-as-source (literate programming) — if anyone does that, it needs
  its own design
- Natural-language understanding of prose content

**Initial target.** **M1 only** — APS wellformedness + cross-reference
integrity. Deterministic, high-confidence, no natural-language reasoning.
M2 and M3 queue for later.

**Acceptance for M1.** All existing APS plans and all existing
cross-references in `plans/index.aps.md` pass with zero false positives.
That is the bar — Anvil's own `plans/` directory is clean before the check
ships.

**Failure mode this prevents.** Markdown work that tries to be a
documentation linter. The point is governance of artefacts, not prose
quality.

---

## 9. Track interactions and sequencing

Tracks do not share engineering attention equally:

- **Track 1 (Anchors)** is the heaviest and always has priority on
  engineering bandwidth.
- **Track 2 (Tail)** is a single sprint, scheduled once when capacity
  permits. Not continuous.
- **Track 3 (Governance surfaces)** runs continuously at lower priority.
  Does not depend on anchor completion — surfaces can start in parallel
  with Track 1.
- **Track 4 (Packs)** is demand-pulled and substrate-gated. Most TS packs
  unblock after Track 1 item 0 (TS audit). Rust packs unblock after Rust
  reaches T2. Python packs unblock after Python reaches T2.
- **Track 5 (Markdown)** is small and self-contained. Can start any time
  with no dependencies on other tracks.

**Ordinal sequencing** (no calendar commitments):

1. Track 1 item 0 — TS audit + gap-close (anchor item zero)
2. Track 3 item 1 — SQL migrations T2 (parallel; does not depend on anchor)
3. Track 4 item 1 — Pulumi pack (unblocks after step 1)
4. Track 4 item 4 — TS LLM Provider pack, warn-only (unblocks after step 1
   — see §8.4.1)

**━━━ Phase 1 line (MVP) ━━━**

Steps 1–4 constitute the hard Phase 1 / MVP boundary added per council
findings C-011/C-021. On completion of step 4, Anvil has a shippable
"coverage + governance + strategic narrative" bundle: TS at audited T3,
SQL migrations governed, Pulumi infrastructure governed, and the AI/ML
governance story provable via warn-only LLM Provider detection. Every
item below this line is Phase 2+ and is not required to declare the
language-and-coverage design a success.

5. Track 5 — Markdown M1 (parallel; self-contained — can slot anywhere
   in Phase 2 or earlier if bandwidth allows)
6. Track 1 item 1 — Rust → T3
7. Track 3 item 2 — GitHub Actions YAML T2
8. Track 4 item 2 — Drizzle pack
9. Track 2 — Tail T1 wave (single sprint)
10. Track 1 item 2 — Python → T3
11. Track 4 item 4b — Python-substrate LLM Provider extension (unblocks
    after step 10 — see §8.4.1, §12.9)
12. Track 4 item 3 — Next.js pack
13. Track 4 item 5 — Hono pack
14. Track 4 item 6 — Tokio pack (unblocks after Rust T2+ in step 6)

**━━━ Phase 2 line (named deliverables complete) ━━━**

Steps 5–14 constitute Phase 2 — every named, user-visible deliverable
scoped by this design. On completion of step 14, all three anchors are
at T3, all five named TS packs ship, the Python-substrate LLM Provider
extension ships, the top two governance surfaces (SQL, GitHub Actions)
are at T2, the tail T1 wave has shipped, and the Tokio pack rides on
Rust T3. References elsewhere in this spec to "end of Phase 2" or
"by end of Phase 2" mean step 14 here.

15. Remaining Track 3 surfaces (Dockerfile, shell, `.env`, and any new
    surfaces that arrive with future user demand)
16. Remaining Track 4 packs (Django, FastAPI, Axum — gated on substrate
    tier and on User C's framework choice resolving)

Steps 15 and 16 are **Phase 3 / open-ended** — explicitly not part of
Phase 2's named scope. They ship when demand pulls them forward.

This ordinal sequence is a sanity check, not a commitment. Real sequencing
is decided in the APS modules produced downstream from this spec. Skipping
or reordering items within a track is allowed as demand shifts. The Phase 1
and Phase 2 lines, however, are **not** reorderable boundaries — they are
the explicit shippable-MVP cut and the full-scope-complete cut the design
commits to.

---

## 10. Archival and replacement actions

On approval of this spec, the following cleanup in `plans/` is triggered:

1. **Replace** the `### Multi-Language Support (Draft)` section of
   `plans/index.aps.md` with the Track 1-5 structure from this design.
2. **Replace** the `### Task Status — Multi-Language (Draft)` section of
   `plans/index.aps.md` with the ranked tables from §8.
3. **Rewrite** the existing `lang-*.aps.md` placeholder modules:
   - `lang-rust.aps.md` → promoted to full module, rewritten for T3 target
     and tree-sitter reality.
   - `lang-python.aps.md` → promoted to full module, rewritten for T3
     target and tree-sitter reality.
   - `lang-dart.aps.md`, `lang-go.aps.md`, `lang-java.aps.md`,
     `lang-kotlin.aps.md`, `lang-dotnet.aps.md`, `lang-c-cpp.aps.md` →
     merged into a single `lang-tail-wave.aps.md` module for the Track 2
     batch sprint.
   - `lang-swift.aps.md`, `lang-zig.aps.md` → archived with a one-line note
     (`cut by language and coverage design 2026-04-08`).
4. **Create** new modules for governance surfaces:
   `surface-sql-migrations.aps.md`, `surface-github-actions.aps.md`,
   `surface-dockerfile.aps.md`, `surface-shell.aps.md`,
   `surface-env-files.aps.md`.
5. **Create** new modules for semantic packs: `pack-pulumi.aps.md`,
   `pack-drizzle.aps.md`, `pack-nextjs.aps.md`, `pack-llm-provider.aps.md`,
   `pack-hono.aps.md`, `pack-tokio.aps.md`.
6. **Create** a markdown-governance module: `markdown-governance.aps.md`.
7. **Update** `.claude/rules/aps-project.md` with the new module IDs once
   they are defined.

Exact module names are indicative; final names are decided during
implementation planning.

---

## 11. User validation cases

Because User B has the polyglot test stack that exercises the most
dimensions of this design, and User C is the first almost-pure Python
stack surveyed, the spec commits to concrete validation cases against
both. The question: **by end of Phase 2 (§9 step 14), when each user
opens Anvil on their repo, what can Anvil see?**

### 11.1 User B (polyglot stack)

| User B stack item | Covered by end of Phase 2 | By |
|---|---|---|
| TypeScript | ✅ T3 | Track 1 item 0 (TS audit + gap-close; Phase 1) |
| Rust | ✅ T3 | Track 1 item 1 |
| Python | ⚠️ In progress toward T3 | Track 1 item 2 |
| Pulumi | ✅ Pack | Track 4 item 1 (Phase 1) |
| Drizzle | ✅ Pack | Track 4 item 2 |
| OpenAI API calls (TS-side) | ✅ Pack (LLM Provider, warn-only) | Track 4 item 4 (Phase 1) |
| Postgres SQL migrations | ✅ T2 | Track 3 item 1 (Phase 1) |
| GitHub Actions | ✅ T2 | Track 3 item 2 |
| Dart (mobile) | 🟡 T1 queued — unblocks after tail wave | Track 2 wave |
| Next.js | 🟡 Follow-up round | Track 4 item 3 |
| Hono | 🟡 Follow-up round | Track 4 item 5 |
| Tokio | 🟡 Follow-up round (after Rust T2+) | Track 4 item 6 |
| Zod usage | ✅ Cross-cutting | TS anchor T2 anti-patterns (Phase 1) |
| React Query | ⬜ Deferred | See §12 open questions |

**8 of 14 items ✅ by end of Phase 2** (Dart marker corrected from ✅ to 🟡
per council finding C-002 — Track 2 wave is §9 step 9, past the original
first-round boundary). 5 items queued for follow-up (🟡 Dart, Next.js,
Hono, Tokio plus ⬜ React Query), 1 in progress toward T3 (Python).

If the ranking ever produces a spec where User B can see less than this,
the ranking is wrong and needs re-scoring. This is the design's sanity
check for the polyglot case.

### 11.2 User C (Python-first stack, added 2026-04-19)

User C's stack is not yet surveyed in the per-item detail that User B has.
What is known so far:

| User C stack item | Covered by end of Phase 2 | By |
|---|---|---|
| Python (general) | ⚠️ In progress toward T3 | Track 1 item 2 |
| Python-side LLM calls (`openai`, `anthropic`, `langchain`) | 🟡 Follow-up round (Python-substrate LLM Provider, unblocks after Python T2) | Track 4 item 4b (§8.4.1, §9 step 11) |
| Django (if User C's framework) | 🟡 Follow-up round | Track 4 item 7 |
| FastAPI (if User C's framework) | 🟡 Follow-up round | Track 4 item 8 |
| Python SQL migration usage (if any) | ✅ T2 (covered by SQL migrations surface regardless of substrate) | Track 3 item 1 (Phase 1) |
| Python `.env` / secrets | ✅ T1 | Track 3 item 5 |
| GitHub Actions (assumed) | ✅ T2 | Track 3 item 2 |

**Phase 1 coverage for User C is thin on purpose.** Python anchor work is
explicitly sequenced below Rust (§7.2, §9 step 10) because Rust's
strategic unlock is the higher-ROI next anchor after TS. User C is the
second-demand point that justifies the Python commitment; they are not
the signal to reorder Python ahead of Rust. What Phase 1 *does* deliver
for User C: governance of SQL migrations, `.env` files, and GitHub
Actions — surfaces that apply regardless of the surrounding source
language. User C's stack-specific coverage arrives in Phase 2 as Python
progresses toward T3.

This table is a sketch, not a commitment — User C's concrete validation
run happens when implementation reaches Python T2 and we can actually
run the checks against their repo. Expect this section to be rewritten
at that point with real findings.

---

## 12. Open questions

Things the design consciously does not settle — either because data is
missing, or because the call belongs to implementation planning.

1. **Is Python really an anchor on one demand point?** **RESOLVED
   2026-04-19.** User C (§3.3) added as a second independent Python demand
   point. Python remains an anchor, at its existing sequence position
   (§9 step 10 — below Rust). No further re-evaluation required unless
   Python demand drops back to 1. Original concern preserved for history:
   *"Single-user demand is thin. The design keeps Python in the anchor
   set because of strategic and pack-unlock reasoning, but if a second
   Python user does not appear by the time Rust-T3 completes, the anchor
   decision should be re-evaluated."*
2. **Are there other early-access users whose stacks would reorder Track 2
   or Track 3?** Only two mixes have been surveyed at time of writing
   (Anvil + User B). The design should be re-scored whenever a new
   early-access user's mix comes in. The scoring criteria are designed to
   absorb new data without restructuring.
3. **Does the Track 2 "one T1 wave" survive contact with C/C++?** C++
   grammars are known to be finicky. The design's answer is to drop C/C++
   from the wave rather than let it stall — but that is a judgement call
   worth confirming during implementation.
4. **Markdown M1 scope — APS-only or APS + cross-ref?** The design proposes
   APS + cross-reference integrity. A stricter read would say APS-only
   (tighter scope, faster ship). A looser read would fold in stale-claim
   detection from M2. The design leans M1-as-written; implementation may
   narrow it.
5. **Pulumi-in-TS detection.** The Pulumi pack needs to distinguish
   "Pulumi program" from "plain TS file". Probably detect `@pulumi/*`
   imports at file level, but corner cases (conditional imports,
   re-exports) are implementation detail. Not resolved here.
6. **Should the TS audit be its own mini-module?** Easier to track as its
   own item (e.g. `LANG-001: TS T3 audit`) but possibly too small to
   justify a module boundary. Implementation-plan territory.
7. **React Query pack — useful or noise?** Currently deferred to a "watch
   list". Most React Query footguns are perf/correctness, not governance.
   The pack could become viable if a concrete high-value governance rule
   surfaces (e.g. "sensitive queries with long `gcTime`").
8. **Zod cross-cutting rules — anchor anti-patterns or per-pack?** The
   design currently splits them: language-level rules (`z.any()`,
   `.passthrough()`) go into TS anchor T2; boundary-application rules
   ("Zod validator actually applied at a Hono route") go into each
   framework pack. This split may turn out to be awkward in implementation
   and may need revisiting.
9. **Python-substrate LLM Provider coverage.** **Plan this now (updated
   2026-04-19).** User C's Python-first stack (§3.3) includes
   `openai`/`anthropic`/`langchain` usage — concrete demand, not
   speculation. The Python-substrate LLM Provider extension is a Phase 2
   deliverable (§9 step 11) that unblocks after Python T2 (§9 step 10).
   Design still leans "same pack, multiple substrates" — the rule
   catalogue reuses across TS and Python wherever semantics align, with
   substrate-specific rules added only where the language forces it (e.g.
   `langchain` has no TS equivalent). Final one-pack-vs-two decision
   belongs in the `pack-llm-provider.aps.md` module when it is written.

---

## 13. Explicit cuts

The design is as much about what Anvil will **not** do as what it will.
Listing cuts explicitly so they do not rot as placeholder promises.

**Programming languages — cut entirely:**

- **Swift** — zero demand, no plausible near-term user.
- **Zig** — zero demand, no plausible near-term user.

**Frameworks / semantic packs — cut entirely:**

- **Express, NestJS** — no demand. Mature tooling exists elsewhere. Modern
  TS server work tends toward Hono, not Express.
- **Flask** — no demand. Django and FastAPI cover the modern Python web
  story.
- **Spring** — no demand. Java is not an anchor, so the substrate does not
  exist.
- **Rails** — no demand. Ruby is not on the language axis.
- **tRPC** — near-miss. Not currently in either known user's stack.
  Re-scores if it actually ships.

**Governance surfaces — cut entirely:**

- **CloudFormation, Bicep** — no demand. Pulumi covers the IaC story for
  current users.
- **Ansible, Jenkins Groovy** — no demand.
- **Buildkite YAML, CircleCI YAML** — no demand. GitHub Actions covers
  current users.

**Out of scope for anchor work:**

- **Markdown as a documentation linter** — prose quality is editorial, not
  governance.
- **Pip / poetry / conda / virtualenv analysis** — dependency-graph
  intelligence belongs in the separate `config-intelligence` module.
- **Cargo dep-graph as a Rust anchor feature** — same.
- **Type checker replacement** — Anvil is governance, not typing.
- **Framework-specific patterns inside language anchors** — pack concern,
  not anchor concern.
- **"General markdown parsing"** — explicitly out. APS-shaped markdown and
  capability-declaration markdown are specific enough to scope.

**Rule for re-entry.** Items on the cut list stay cut until a demand signal
appears. No silent re-adds. If a new early-access user brings one back,
it re-scores under the §6 criteria like any other candidate.

---

## 14. Success criteria

The design is successful if, after execution:

1. Anvil can show User B coverage across their entire stack (TS, Rust,
   Python at minimum T2, Pulumi, Drizzle, LLM Provider, SQL, Dart,
   GitHub Actions, Zod cross-cutting) in one view.
2. Anvil can audit itself — markdown plans, TS, Rust, Hono API, Next.js
   web app, Pulumi infra, LLM calls — without relying on external tools.
3. The prioritisation criteria in §6 are defensible: any ranked item's
   position can be explained by its score.
4. No "mystery modules" — every entry in every track has a named user, a
   named blast radius, or a named strategic reason.
5. The explicit cuts in §13 survive: things on the cut list stay cut until
   a demand signal appears.
6. The LLM Provider pack proves the AI/ML governance story from the TS side
   and is demonstrable to prospects as a live-demo artefact.

---

## 15. Next steps

On approval of this spec:

1. Transition to implementation planning via the `writing-plans` skill.
   Produce the module-level APS plans for each Track 1-5 entry per the
   archival actions in §10.
2. Update `plans/index.aps.md` to reflect the new track structure.
3. Archive or rewrite the existing `lang-*.aps.md` placeholder modules per
   §10.
4. Schedule the TS audit (anchor item zero) as the literal first work item
   in the resulting plan set.

---

## 16. Council review findings (2026-04-08)

Full-pack council review (session `council-8673070d`) ran 5 reviewers —
council-reviewer, adversarial-reviewer, kernel-maintainer,
operations-reviewer, pragmatic-lead — against this spec. 27 findings
recorded (9 critical, 17 major, 1 minor). Findings are reproduced here so
the spec carries its own review evidence; resolution status is tracked in
the council session and in downstream APS modules.

### 16.1 Verdict summary

**Coverage strategy is sound. Execution substrate is not.** The three-axis
framing, tier ladders, ranking criteria, and sequencing instinct all
survive review. What does not survive is the assumption that the current
kernel can absorb 9+ new grammars without prerequisite refactoring, and
the assumption that a small team can execute 26 deliverables across 5
tracks without a hard Phase 1 line. Two factual errors and three
architectural gaps are showstoppers; the remainder are amendments.

### 16.2 Critical findings (9)

| ID | Source | Summary |
|---|---|---|
| C-003 | kernel-maintainer | `crates/anvil-kernel/src/parser/extract.rs` is a JS/TS-specific AST walker, not a generic extractor. No `LanguageExtractor` trait. Adding Rust requires redesigning the extraction layer, not adding a match arm. Prerequisite infrastructure treated as implementation detail. |
| C-004 | kernel-maintainer | `AstCache` in `parser/cache.rs` keys on `(PathBuf, content_hash)` only. `tree_sitter::Tree` node kind IDs are grammar-version-specific. Upgrading any grammar crate returns wrong node kinds silently with no error. Latent data corruption risk with 9+ grammars. |
| C-005 | kernel-maintainer | Track 2 adds 7 grammars in one batch with no maturity or crates.io availability audit. `tree-sitter-dart` lacks stable 0.26 ABI publication. `tree-sitter-kotlin` is community-maintained with known regressions. `tree-sitter-cpp` has partial-parse issues on C++20/23. Binary size and LTO cost unaccounted. |
| C-006 | adversarial-reviewer | Single-data-point fragility. Anchor set, pack rank, surface rank all calibrated against 2 users. §12 Q2 notes risk but provides no process gate. A third user with Go/k8s/Java invalidates Python's anchor silently. No named owner for re-scoring. |
| C-007 | adversarial-reviewer | TS audit is an unmitigated single point of failure. 5 of 9 Track 4 packs gate on TS→T3. §7.3 marks 8 capabilities ✅ with hedging ("Confirm it reaches TS specifically"). If audit reveals months of gaps, pack ROI argument collapses and §11 coverage promise breaks. |
| C-008 | operations-reviewer | `AVAILABLE_CHECKS` in `gate.rs` is a hardcoded 7-entry static array. Spec adds ~50 new checks. `--skip_checks` does not extend. If a newly shipped check generates widespread false positives, only rollback path is binary downgrade. |
| C-009 | operations-reviewer | Drift baseline schema is hardcoded `SCHEMA_VERSION = "1.0.0"` in `drift.rs`. Spec adds 7 new surfaces each with new baseline fields. No migration plan, no schema versioning, no `anvil drift migrate`. Existing user baselines break silently on upgrade. |
| C-010 | operations-reviewer | LLM Provider pack has no fail-safe mode defined. §8.4.1 positions it as top strategic deliverable detecting PII but doesn't say warn or block. Static PII detection is heuristic. False positive breaks a production call path, not just a build. |
| C-011 | pragmatic-lead | "First round of execution" in §11 is the whole roadmap, not a round. 8 deliverables across 4 tracks = entire body of high-priority work. No partial-value story. No explicit MVP cut. |

### 16.3 Major findings (17)

**Factual corrections:**

| ID | Source | Summary |
|---|---|---|
| C-001 | council-reviewer | Drizzle demand is 1, not 2. Anvil does not use Drizzle — `apps/anvil-api` uses raw SQL via NeonClient; no `drizzle-orm` dependency in the monorepo. Affects Drizzle rank and §11 headline. |
| C-002 | council-reviewer | Dart is marked ✅ in §11 first round but Track 2 is ordinal step 9, past the step-8 first-round boundary. Should be 🟡. Coverage claim becomes 8 of 14. |

**Scope and sequencing:**

| ID | Source | Summary |
|---|---|---|
| C-012 | adversarial-reviewer | Python anchor on 1 demand point is circular. Django/FastAPI packs have 0 demand. TS LLM Provider pack delivers the AI/ML story faster on already-invested substrate. Move trip-wire earlier. |
| C-013 | adversarial-reviewer | Track parallelism is nominal. "Always has priority" + small team = Tracks 3/5 starve. §9 ordinal sequence interleaves tracks as if parallel; in practice sequential with disguised dependencies. |
| C-021 | pragmatic-lead | Total deliverable count is 26+ across 5 tracks. 18-24 months of work dressed as a single design. Three roadmaps in a trench coat. No Phase 1 done line. |
| C-022 | pragmatic-lead | User B overweights one external stakeholder. §11 makes User B's 14-item stack the explicit sanity check. By the spec's own demand-count logic, Hono (demand 1, Anvil's own API) should rank ahead of LLM Provider (demand 1, User B only). |

**Methodology consistency:**

| ID | Source | Summary |
|---|---|---|
| C-014 | adversarial-reviewer | Zero-FP-on-Anvil's-repo acceptance bar can be gamed (suppression file) or block shipping indefinitely. Tests applicability to one codebase, not rule quality. |
| C-015 | adversarial-reviewer | LLM Provider post-hoc promotion in §9 step 8 violates §6's "no hidden tiebreakers". Strategic was already scored in the rank table. Promoting a second time for the same reason is double-counting. |
| C-016 | adversarial-reviewer | Markdown M1 "zero false positives on Anvil's own plans" acceptance will fail on first run. §3.2 itself notes the stale cross-references this design is replacing. Spec is blocked by its own content. |

**Architectural gaps:**

| ID | Source | Summary |
|---|---|---|
| C-017 | kernel-maintainer | Markdown Track 5 does not belong in the kernel. Spec leaves implementation location unassigned. Either forces a markdown fast-path in the parser or adds `tree-sitter-markdown` as another grammar dep — neither is acceptable. |
| C-018 | kernel-maintainer | Semantic pack architecture entirely unspecified. §8.4 defines 9 packs but doesn't say where they live, how substrate-tier gates are enforced, whether packs are compiled or dynamically loaded. Pulumi/Drizzle need symbol-graph access, not regex — fundamentally different architectural shape from existing infrastructure. |
| C-019 | kernel-maintainer | T3 architecture enforcement for Rust unresolved. `analysableExtensions` in `architecture.check.ts` is hardcoded to JS/TS extensions. Rust crate/module semantics are structurally different. Month-scale decision being deferred. |
| C-026 | kernel-maintainer | Parser pool thread-safety. `tree_sitter::Parser` is not `Send`. With rayon + 9 languages, pool becomes `N workers × 9 parsers`. Spec silent on thread-locality strategy. |
| C-027 | kernel-maintainer | `Parser::get_parser()` panics on grammar version mismatch via `.expect()` in `mod.rs:54`. In the long-running watcher process, this kills the entire process. |

**Operations:**

| ID | Source | Summary |
|---|---|---|
| C-020 | operations-reviewer | No feature-flag or per-track progressive-rollout scheme. Every binary upgrade ships all tracks simultaneously. For enterprise CI users, every upgrade is a potential gate-failure event. |
| C-023 | operations-reviewer | CI runtime budget undefined. No per-check wall-time cap, no file-presence guard for absent substrates. A repo with no `.sql` files should pay zero cost for the SQL surface. |
| C-024 | operations-reviewer | No telemetry or FP reporting mechanism. Anvil's repo is one controlled data point. Only production signal is support tickets — late and anecdotal. |

### 16.4 Minor findings (1)

| ID | Source | Summary |
|---|---|---|
| C-025 | council-reviewer | Suppression parser conflict between reviewers. Council-reviewer claims `packages/anvil/core/src/suppression/parser.ts` is TS-comment-only. Kernel-maintainer claims `crates/anvil-checks/src/antipattern/scanner.rs:152` already handles `//`, `#`, `/*`, `<!--`, `--`. Both may be right — two parsers in two layers. Spec should clarify which is authoritative for new surfaces. |

### 16.5 Required spec amendments before `writing-plans`

The council recommends the spec is **approved in principle, blocked on
amendment**. Minimum edits before transitioning to implementation planning:

1. **Hard Phase 1 line** (C-011, C-021). Proposed: TS audit + SQL T2 +
   Pulumi pack + LLM Provider pack (warn-only) as a 3-4 sprint MVP.
   Everything else becomes Phase 2+.
2. **Factual corrections** (C-001, C-002). Drizzle demand → 1; Dart → 🟡;
   §11 headline → 8 of 14.
3. **New section: kernel prerequisite work** (C-003, C-004, C-005, C-026,
   C-027). Extractor refactor, `CacheEntry.grammar_version`, parser
   thread-safety strategy, panic removal, grammar maturity audit. Becomes
   Track 1 item 0.5, not implicit Rust anchor sub-tasks.
4. **Pack architecture decision** (C-018) in §8.4. Symbol-graph access or
   content-only, stated explicitly, with a named crate location.
5. **Rust T3 architecture enforcement decision** (C-019) in §8.1. TS shim
   or Rust-native, stated explicitly.
6. **LLM Provider warn-only default** (C-010, C-015). Removes the
   fail-safe concern and the §6 methodology violation in one edit.
7. **Operational supplement** (C-008, C-009, C-020, C-023, C-024). Either
   inline or as a companion doc: check registry with stable IDs, baseline
   schema versioning and `anvil drift migrate`, per-track feature flags,
   CI wall-time budget + file-presence guards, FP reporting channel.
8. **Anchor re-scoring process gate** (C-006). Before Rust anchor work
   begins, re-score against any new user mixes. Name the gate owner.
9. **Acceptance bar revision** (C-014). Replace "zero FPs on Anvil's repo"
   with "FP rate < N% on Anvil's repo AND ≥1 external codebase validation
   run".
10. **Markdown M1 acceptance softening** (C-016). "All findings reviewed
    and fixed-or-suppressed" rather than clean-run-required.
11. **Track 5 crate assignment** (C-017). Assign markdown governance to a
    standalone crate or the TS layer, not the Rust kernel.
12. **Parallelism language fix** (C-013). Explicit note that "parallel" is
    a logical-dependency claim, not concurrent execution, on a small team.

### 16.6 Findings the council declined to escalate

- Terraform/HCL demand speculation (minor, noted by council-reviewer).
- Axis C Pulumi example overstatement (minor, noted by council-reviewer).
- Dockerfile demand count (minor, unexplained, noted by council-reviewer).
- Pack ROI table cap-bypassing (nit, noted by both council and
  adversarial-reviewer).
- 786-line spec readability (nit, pragmatic-lead).
- Cuts list confidence on thin signal (minor, adversarial + pragmatic).
- Track 5 vs Track 3 boundary (minor, adversarial-reviewer).
- Python namespace package extraction complexity (minor,
  kernel-maintainer).

These are captured in the council session but not reproduced here.

---

## 17. Amendments applied (2026-04-19 refresh)

This section tracks the concrete edits the 2026-04-19 refresh made to
§1–15 of the spec. The §16 council review record is preserved unchanged
as the canonical snapshot of the original review; amendments that
reshape §1–15 are listed here so future readers can see what the spec
originally said and how it changed.

### 17.1 Amendments inlined in this refresh

| # | Section touched | Amendment | Council finding(s) addressed |
|---|---|---|---|
| 1 | §3.3 | Added User C (almost-pure Python stack) as a third surveyed user. | — (new data) |
| 2 | §7.2 | Python demand 1 → 2 (User B + User C). Rationale rewritten to cover the new demand point and to explicitly state that sequence position (below Rust) is unchanged. | C-012 (partial — the single-demand concern is now moot; the TS-first AI/ML story is already prioritised in §9 Phase 1) |
| 3 | §8.4 Drizzle row | Demand corrected 2 → 1. Rank held at #2 on blast radius. | **C-001** |
| 4 | §8.4 Django row | Demand 0 → 1 (User C floor). | — (new data) |
| 5 | §8.4 FastAPI row | Demand 0 → 1 (User C floor). | — (new data) |
| 6 | §8.4.1 | LLM Provider pack declared warn-only by default. Python-substrate extension promoted to a concrete Phase 2 deliverable. | **C-010**, **C-015** (warn-only removes the fail-safe concern and the double-counting concern in one edit) |
| 7 | §9 | Hard Phase 1 / MVP line inserted after step 4 (TS audit + SQL T2 + Pulumi pack + LLM Provider pack warn-only). Python-substrate LLM Provider added as Phase 2 step 11. | **C-011**, **C-021** |
| 8 | §11 | Dart marker ✅ → 🟡. Headline "9 of 14" → "8 of 14". Section renamed "User validation cases". User C validation table added as §11.2. | **C-002** |
| 9 | §12.1 | Marked RESOLVED (Python now has 2 demand points). Original text preserved for history. | — (question answered by new data) |
| 10 | §12.9 | Promoted from "design leans same pack — TBD" to "plan this now" with a Phase 2 sequence position. | — (question answered by new data) |

### 17.2 Amendments **not** inlined — belong in downstream APS modules

The council review recommended deeper architectural amendments (§16.5
items 3–12) that would reshape the spec itself. These are **intentionally
not** inlined here because they belong in the implementation plans
produced under §15 (`writing-plans`), not in the design spec. Listing
them so no one believes they were dropped:

| Council item | Owning module when written |
|---|---|
| 16.5 #3 — kernel prerequisite work (extractor refactor, grammar version in cache key, parser thread-safety, panic removal, grammar maturity audit) | Track 1 item 0.5, in the TS-audit APS module |
| 16.5 #4 — pack architecture decision (symbol-graph vs content-only, named crate location) | `pack-pulumi.aps.md` (first pack; sets the pattern) |
| 16.5 #5 — Rust T3 architecture enforcement decision (TS shim vs Rust-native) | `lang-rust.aps.md` rewrite |
| 16.5 #7 — operational supplement (check registry, drift schema versioning + `anvil drift migrate`, per-track feature flags, CI wall-time budgets, FP reporting) | New operational module or companion doc, referenced from each track module |
| 16.5 #8 — anchor re-scoring process gate | Governance process, owner named in `lang-rust.aps.md` rewrite |
| 16.5 #9 — acceptance bar revision (FP rate + external codebase validation) | Referenced in each track's acceptance section |
| 16.5 #10 — Markdown M1 acceptance softening | `markdown-governance.aps.md` |
| 16.5 #11 — Track 5 crate assignment (not the Rust kernel) | `markdown-governance.aps.md` |
| 16.5 #12 — parallelism-is-logical-dependency clarification | Inline in §9 of whichever track module is first to bake against it |

### 17.3 What is left to action for the 2026-04-19 refresh

The §10 archival/replacement actions from the original 2026-04-08 spec
are still **not yet executed** — the placeholder `lang-*.aps.md` modules
remain in `plans/modules/`. The 2026-04-19 refresh does not change the
§10 action list; it still reads cleanly with the updated content above.
Next concrete work, in order:

1. Archive `lang-swift.aps.md` and `lang-zig.aps.md`.
2. Merge `lang-dart.aps.md`, `lang-go.aps.md`, `lang-java.aps.md`,
   `lang-kotlin.aps.md`, `lang-dotnet.aps.md`, `lang-c-cpp.aps.md` into
   `lang-tail-wave.aps.md`.
3. Rewrite `lang-rust.aps.md` for T3 target (incorporates §16.5 #3, #5,
   #8).
4. Rewrite `lang-python.aps.md` for T3 target.
5. Create surface modules: `surface-sql-migrations.aps.md` (Phase 1),
   `surface-github-actions.aps.md`, `surface-dockerfile.aps.md`,
   `surface-shell.aps.md`, `surface-env-files.aps.md`.
6. Create pack modules: `pack-pulumi.aps.md` (Phase 1),
   `pack-llm-provider.aps.md` (Phase 1, TS + Python-substrate extension
   in one module per §12.9), `pack-drizzle.aps.md`, `pack-nextjs.aps.md`,
   `pack-hono.aps.md`, `pack-tokio.aps.md`.
7. Create `markdown-governance.aps.md`.
8. Replace the Multi-Language section in `plans/index.aps.md` with the
   Track 1–5 structure.

**Pack modules explicitly not in this list.** Django, FastAPI, and Axum
are still named as Track 4 packs in §8.4 but have **no dedicated pack
module scheduled here** because they are Phase 3 / open-ended deliverables
(see §9 steps 15–16) gated on substrate tier and, for Django/FastAPI, on
User C's framework choice resolving. A pack module file
(`pack-django.aps.md`, `pack-fastapi.aps.md`, `pack-axum.aps.md`) is
created only when that specific pack is promoted from Phase 3 to active
work — not pre-stubbed here. If the "no Phase 3 pack stubs" choice turns
out to create discovery friction for downstream planners, revisit in a
future refresh; for now, keeping §17.3 to Phase 1 + Phase 2 modules
avoids the rot that killed the original ten `lang-*` placeholders.

Order of module creation matters only insofar as the Phase 1 modules
(TS audit work item, `surface-sql-migrations.aps.md`, `pack-pulumi.aps.md`,
`pack-llm-provider.aps.md`) are the ones that need to exist for the MVP
to be actionable. Everything else can lag.
