# Autonomous Software Factory (eve / Foreman) — Deployment-Scenario Gap Assessment

**Date:** 2026-08-18 **Status:** Brainstorm — assessment of Anvil **as a
product** for a customer whose development happens inside an autonomous software
factory. **Outcome: the north star fits this customer better than it fits the
laptop customer, but the shipped product does not — roughly the whole
activation / daemon / interception / watch / insights half of the surface area
delivers nothing in an ephemeral, human-less pipeline, and four gaps
(entitlement unit, attribution anchor, witness ingress, evidence egress) sit
between "anvil runs there" and "anvil is sellable there". Anvil's engine,
baseline model, gate contract, and evidence artefacts transfer cleanly and are
worth _more_ in this scenario than on a developer machine. Take zero code from
the template. Disposition: Product Note, promotion to Specification
demand-pulled by a first factory design partner, per the Horizon 3 convention.**
**Source:** <https://github.com/vercel-labs/eve-software-factory-template> —
**MIT**. Vercel Labs template ("Foreman") built on the Claude Agent SDK and
Vercel's eve platform. Facts read from the public repo landing, README, and
`agent/` tree on 2026-08-18.

---

## 0. What this document is

A **deployment-scenario** gap assessment, in the format of the borrow
assessments in this directory. The difference: the earlier documents mine an
external repo for a primitive to borrow. This one uses an external repo as a
**customer environment specimen** and asks what breaks when Anvil is deployed
into it.

The specimen matters because it is not exotic. Foreman is one instance of a
category — cloud coding agents that take an issue, plan it, implement it in an
ephemeral sandbox, review it, and hand a human a draft PR. The category also
holds Claude Code on the web, Codex cloud, Copilot coding agent, and every
in-house equivalent. Assessing one specimen concretely beats assessing the
category abstractly.

All Anvil claims below are cited to source in §9 rather than asserted.

---

## 1. Understanding the candidate

**What it is.** An AI software factory. Work arrives from a GitHub issue label,
a bot mention, a Linear agent session, a local TUI, or red CI on one of its own
PRs. It moves through four stations, each an independent agent with its own
instructions, sandbox, and tools:

| Station         | Does                                                          | Sees                        |
| --------------- | ------------------------------------------------------------- | --------------------------- |
| **Classifier**  | Triages type, priority, complexity, actionability             | The task                    |
| **Analyst**     | Produces a plan with acceptance criteria from a live checkout | Repo + task                 |
| **Implementer** | Executes the plan in its own sandbox, runs the repo's checks, pushes a branch | Repo + plan |
| **Reviewer**    | Judges the real diff against requirements, with evidence      | **The pushed branch only** |

Between runs a persistent "factory brain" carries repository knowledge forward.
Every pipeline ends at a **draft PR awaiting a human**. Configuration is four
environment variables (`FACTORY_REPO`, `FACTORY_SETUP_COMMAND`, `FACTORY_LABEL`,
`FACTORY_BRANCH_PREFIX`) plus Vercel Connect connector UIDs.

**Who it serves.** A team that wants issue-to-PR throughput without a developer
in the loop until review time.

**Why it exists.** To move human attention from writing to judging.

**The environment Anvil would be deployed into**, stated plainly, because every
gap below follows from it:

- No developer machine. No editor. No file saves. No persistent host.
- A fresh clone in a fresh microVM per station, destroyed minutes later.
- No TTY, no interactive consent, no "restart your client".
- The only human touchpoint is a PR, reviewed by someone who was not present
  when any of the code was written.
- Code volume per unit of human attention goes up by an order of magnitude.

---

## 2. The primitive

**Name:** Station-boundary evidence handoff under an information barrier.

**Description.** Foreman's load-bearing mechanism is not the agents — it is the
**barrier between them**. The Reviewer never sees the Implementer's reasoning;
it sees the pushed branch and judges it independently. Trust flows between
stations only as artefacts, never as narrative. The factory brain is the one
deliberate exception, and it is scoped to repository facts rather than run
reasoning.

**Why it matters to Anvil.** This is the same shape as the witness chain and the
review capsule: a downstream party who was not there reconstructs what happened
from artefacts alone. Foreman validates the shape commercially — a serious
vendor built an autonomous pipeline and concluded that the barrier is what makes
it trustworthy — but its barrier is enforced with a **probabilistic** judge.
Anvil's contribution to this pattern is the deterministic half: a verdict that
reproduces, cannot be talked out of, and leaves an artefact a fourth party can
verify later.

Foreman therefore reads less as a competitor and more as **the customer
environment Anvil's evidence model was designed for, arriving earlier than the
roadmap assumed**.

---

## 3. Hypothesis assessment

**Status:** Not provided (operator question rather than a Morgan nomination).
The operator's implicit hypothesis — "would Anvil work as a product if a
customer develops this way?" — is answered in §5 and §6.

**Better framing identified during analysis.** The interesting question is not
whether Anvil _runs_ in a sandbox. It does; §5 lists the surfaces that work
today unchanged. The interesting question is that this customer **inverts
Anvil's product geometry**. Anvil's shipped mission sentence places value at
file-save time on a developer's machine [C1]; this customer has neither. Every
gap in §6 is a consequence of that single inversion, which is why they cluster
rather than scatter.

---

## 4. Customer surface test

**What becomes stronger:** the ability to prove that a machine-authored change
was checked, by whom, against which rules, with a verdict that reproduces.

**Why a customer cares.** The person merging a factory PR is accountable for
code no human wrote, produced by a pipeline they cannot fully inspect, at a
volume they cannot fully read. "Another model reviewed it" does not survive
contact with an auditor, an incident review, or a regulator. A deterministic
gate verdict plus a portable evidence capsule does.

**Buyer shift, which is the commercially significant finding.** On a laptop the
buyer is a developer or a team lead who wants fewer bad diffs — a habit-adoption
sale. In a factory the buyer is whoever is accountable for merging machine-
written code: platform engineering, engineering leadership, or compliance. That
buyer has budget, has a sharper pain, and — critically — **does not require any
developer to change behaviour**, because there are no developers in the loop to
change. That removes Anvil's hardest adoption dependency. It replaces it with a
procurement motion and a pricing axis the product does not currently have (§6,
G1).

Customer impact is **strong**, and stronger than in the current ICP.

---

## 5. What already works, unchanged

Recording this precisely matters: the gap register is only credible if the
non-gaps are stated with equal precision.

| Surface                                                                   | Status in a factory | Why                                                                                    |
| ------------------------------------------------------------------------- | ------------------- | -------------------------------------------------------------------------------------- |
| Tracked project state — `.anvil.yaml`, `anvil/baseline.json`, `anvil/project-id` | **Works** | Durable state is in-repo (ADR-073), so a fresh clone inherits it. New-edges-only survives ephemerality with zero per-sandbox onboarding [C2] |
| `anvil mcp serve --stdio`                                                 | **Works**           | Runs standalone; daemon-backed cases are separate [C3]. Wires into the Agent SDK's `mcpServers` |
| `anvil gate` / `anvil check` + exit codes + SARIF / JSON                  | **Works**           | Documented exit-code contract; SARIF for code scanning [C4]                            |
| `--profile ai`                                                            | **Works, and is the right seed** | Curated checks for AI-generated code, JSON output pinned for AI consumers, `strict_config` turns "missing config, skipping" into a blocking diagnostic [C5] |
| Air-gapped core operation                                                | **Works**           | Local checks make no network calls; guarded by a network-namespace harness [C6]        |
| Graph-context MCP tools                                                   | **Works**           | Bounded and deterministic; a natural fit for the Analyst station                       |
| `anvil-intercept` (+ macos / win32), daemon, workspace consent            | **Dead**            | No saves, no editor, no persistent host to protect                                     |
| `anvil start` activation, MCP client registry, restart-and-verify         | **Dead**            | No human clicks a consent list; no registered client id for eve / Foreman              |
| `watch`, TUI, `dashboard`, `insights`                                     | **Dead**            | State lives in a microVM destroyed minutes later; nobody is watching                   |

Roughly the whole activation / daemon / interception / watch / insights half of
the shipped surface earns nothing here. That is not a defect — it is the correct
product for a different customer — but it means **what you would sell a factory
customer is not what is currently in the box**.

---

## 6. Gap register

Severity is commercial, not technical: **Blocker** stops the sale, **High**
breaks the headline claim, **Medium** degrades it, **Low** is friction.

### G1 — No entitlement unit that survives a human-less fleet · **Blocker**

Accounts carry a per-user `plan` (today only `beta`), minted through device-flow
login, with credentials on a machine [C7]. `ANVIL_LICENSE` is the sole
automation path [C8] — a single shared secret, unscoped to repo, run, or
station. A factory has no users in Anvil's sense; it has N ephemeral sandboxes
per PR. Per-seat pricing against that is either trivially undercounted or
absurdly overcounted.

Compounding failure mode: unauthenticated, the pre-write gate deliberately
distinguishes _gate-unavailable_ from _content-veto_ — the wire shape carries
`decision: "gateUnavailable"` (not `block`), `isError: false`, and
`safeDefault: "allow-with-warning"`, so a well-behaved agent surfaces the
warning and **proceeds with the write** rather than refusing to onboard [C9].
That is the right call for agent ergonomics and for first-run onboarding, and it
is commercially dangerous here: a misconfigured factory runs all quarter with
zero protection, writing happily, with no visible symptom. On a laptop the human
eventually notices they never signed in. Nobody notices in a sandbox.

**Closes with:** an entitlement unit that is not a human seat — repo-scoped or
run-scoped — plus a loud, fail-fast preflight for automation contexts. This is a
commercial-model change, not an engine change, which is why it is the blocker.

### G2 — Attribution has no trust anchor without the daemon · **High**

`AgentTag` is **daemon-minted** from `(driver_id, claimed_agent_id,
pid_starttime)`; the `ANVIL_AGENT_TAG` / `ANVIL_TASK_ID` environment variables
are explicitly advisory and forgeable by any same-UID peer, with a process-tree
walk as the fallback anchor [C10]. In a daemonless sandbox there is no minting
authority and no meaningful process lineage — every station is pid 1-ish in a
fresh microVM.

This is the painful one, because `ANVIL_TASK_ID` would otherwise carry exactly
the right value: the GitHub issue or Linear task the factory is working. The
identifier the provenance story wants is right there and cannot be trusted.

**Closes with:** a signed run-attestation ingress — the factory's own run
identity, signed at station start, replacing pid lineage as the anchor. Related
to the parked Horizon 5 "lineage & authorship confidence" bet, but narrower and
demand-pulled.

### G3 — Witness chain depends on git hooks that a clone does not carry · **High**

Witness lines are appended by the `pre-commit`, `post-merge`, and `post-rewrite`
hooks [C11], and `audit-chain` reports commits lacking a corresponding witness
[C12]. Git hooks are not cloned. A fresh factory sandbox therefore commits
**unwitnessed by default**, and `audit-chain` reports near-zero coverage on
precisely the commits whose provenance matters most.

**Closes with:** cheaply, `anvil hooks install` in the run bootstrap.
Durably, a witness write on the MCP or gate path, so evidence does not depend on
git plumbing surviving a clone. The second is the better answer and needs a
design decision, not a patch.

### G4 — Evidence does not survive the compute · **High**

Witness records live at `<repo>/anvil/witness` [C11]; capsules default to an
external `--out`, with in-repo staging as an opt-in that accumulates
indefinitely (ADR-078) [C13]. Both are destroyed with the microVM unless the
pushed branch carries them. The highest-value artefact in this scenario — the
one that answers "prove this machine-written commit passed" — evaporates by
default.

**Closes with:** an evidence egress contract. Note the collision to resolve
first: ADR-059 pins a single operator-hosted sink with the CLI local-first and
never auto-exporting, and ADR-035's three-pipe rule keeps the ephemeral pipe
out of source-of-truth. An egress path for ephemeral compute needs an ADR that
reconciles with both, not a feature that quietly ignores them.

### G5 — The honest-claim doctrine can never say "protected" here · **Medium**

The protection claim is daemon-sourced; the MCP shim falls back to embedded
scanning when no daemon answers [C14], and `anvil start --verify` reserves
`protecting` for a proven pre-write path [C15]. Under "never say Protected when
a layer is unverified", a factory run is permanently unverified — correct
behaviour producing a permanently downgraded claim, which is a poor thing to
sell.

**Closes with:** a claim tier for daemonless ephemeral execution in which the
gate verdict, not daemon liveness, is the evidence. Doctrine work, cheap in
code, load-bearing for the pitch.

### G6 — The default posture assumes a human reads the warning · **Medium**

Warnings-over-blocks with exit 0 [C16] is correct when a human exercises
judgement. Unattended, an unread warning is indistinguishable from no product.
The knobs exist (`--fail-on-warnings`, `ANVIL_FAIL_ON_WARNINGS`, the MCP shim's
`interrupt` fallback on no-config [C17]) and `--profile ai` is the right seed
[C5], but no shipped profile is fail-closed at a station boundary and none is
documented as the autonomous posture.

**Closes with:** profile hardening plus documentation. Small, and the highest
value-per-unit-effort item in the register.

### G7 — No aggregate surface for the actual buyer · **Medium**

`insights` is local-only and weekly [C18], on a machine that no longer exists;
the dashboard is a local read-only surface over local state. The factory buyer
wants "across 400 factory PRs this month, what did the agents keep trying to
do?" That is a cross-run surface Anvil has no local answer for.

Scope-guard constraint: observability is allowed **only** when tied to
enforcement [C19]. So the framing must be policy feedback that changes a gate,
not a dashboard that informs for its own sake. Adjacent to Horizon 3's gateway
control plane observability event model.

### G8 — The adoption path assumes an interactive first minute · **Medium**

"First-touch wow" is explicit posture [C20], and the onboarding surfaces
(`start`, `welcome`, `wizard`, `tutorial`, the MCP client registry) all assume a
TTY and a human. None exists here, and the client registry has no id for eve /
Foreman, so the `mcpServers` entry is hand-written in the factory's own code.

**Closes with:** a documented non-interactive bootstrap — a human runs
`anvil init` plus baseline **once** on the target repo and commits the result;
every subsequent run is `install → licence → gate`. Mostly documentation, and it
is what makes G1's repo-scoped entitlement coherent.

### G9 — Cold graph cache on every run · **Low–Medium**

Each sandbox rebuilds from nothing, so graph-context tools and full scans pay
first-run cost on every issue. Mitigable today by scoping the inner loop to
`--changed` / `--staged` and baking the install into `FACTORY_SETUP_COMMAND`;
durably wants a commit-keyed cacheable graph artefact.

---

## 7. Roadmap position

There is **no horizon for headless, ephemeral, or agent-fleet deployment**
[C21]. The nearest neighbour is Horizon 3 (Enterprise Readiness — gateway
control plane, enforcement contract, observability event model), which is
explicitly demand-pulled by a first enterprise prospect. Horizon 5 parks
"lineage & authorship confidence" and provider-agnostic agent infrastructure.

The register maps onto that shape without needing a new horizon:

| Gap    | Nearest existing home                              | Increment                        |
| ------ | -------------------------------------------------- | -------------------------------- |
| G1     | Account plan / entitlements (ADR-121 territory)    | New entitlement unit — **M**     |
| G2     | Horizon 5 lineage bet, narrowed                    | Signed run attestation — **M**   |
| G3     | Witness chain + hooks (Horizon 1 big bet)          | Ingress that survives a clone — **S–M** |
| G4     | EXPORT / capsule, constrained by ADR-059 + ADR-035 | Egress contract + ADR — **M**    |
| G5     | Honest-claim doctrine                              | Claim tier — **S**               |
| G6     | Gate profiles                                      | Profile hardening + docs — **S** |
| G7     | Horizon 3 observability event model                | Folds in — **defer**             |
| G8     | Onboarding / docs                                  | Bootstrap guide — **S**          |
| G9     | Graph cache                                        | Commit-keyed artefact — **M**    |

**No new APS module is warranted today.** G6 and G8 are small enough to file as
continuous-improvement candidates; G5 is a doctrine note. G1–G4 are the
substance and should not be filed speculatively — they are demand-pulled by a
design partner, and filing them without one would be exactly the roadmap drift
the scope guard exists to prevent.

---

## 8. Assessment

### Roadmap disposition

**Product Note.** The finding is not a feature to build; it is a **deployment
posture the product does not yet have**, with a commercial-model blocker (G1) in
front of it. The right artefact is a short product thesis — "Anvil for
autonomous pipelines: what the unit of protection becomes when there is no
developer" — that names the four substantive gaps and the entitlement question.

**Promotion trigger, stated so it is not left ambiguous:** the first factory
design partner — an operator running an autonomous pipeline against a repo they
want gated — flips this to **Specification**, mirroring the Horizon 3
demand-pull convention. Absent that partner, G5, G6, and G8 are worth doing
anyway on their own merits, because they cost little and improve the CI story
for existing customers too.

### Criteria scorecard

| Criterion                   | Score      | Note                                                                        |
| --------------------------- | ---------- | --------------------------------------------------------------------------- |
| Direct Anvil Fit            | 9          | The purest expression of the north star; the shipped mission sentence is narrower |
| Borrowable Primitive        | 7          | Information-barrier evidence handoff is real and validated commercially      |
| Developer-Native Usefulness | 4          | Deliberately low — there is no developer. This _is_ the finding             |
| Evidence Before Enforcement | 8          | Gate and SARIF deliver advisory value at zero adoption cost                 |
| Deterministic Governance    | 9          | Determinism is the whole differentiator against a probabilistic reviewer    |
| Audit and Export Value      | 9          | Highest-value axis, and where G3 / G4 bite hardest                          |
| Narrow Beta Wedge           | 6          | A wedge exists; G1 blocks a clean one                                       |
| Strategic Differentiation   | 8          | Non-bypassable, reproducible, evidence-producing                            |
| Clean-Room Feasibility      | 10         | Nothing to copy; zero code taken                                            |
| Buyer Language Strength     | 9          | "Prove the machine-written commit passed" needs no education                |
| **Overall**                 | **79/100** |                                                                             |

### Licensing assessment

- **Licence:** MIT (template repository).
- **Risk level:** None — no code is taken.
- **Dependency suitability:** Not applicable. Anvil would be deployed
  _alongside_ a factory, never depend on one; binding to Vercel's eve platform
  or connector model would contradict provider-agnostic posture.
- **Vendoring suitability:** Not applicable.
- **Clean-room preference:** Total. The value is environmental understanding.
- **Notes:** Treat the template as a **specimen**, not a partner. Anything built
  must work for the whole category, not for Foreman specifically.

### Acquisition strategy

**Inspiration Only.** Nothing in the template is worth taking. The value is (a)
proof that a serious vendor shipped an information-barrier pipeline, and (b) a
concrete, testable environment to measure Anvil's headless deployment gaps
against.

### Anvil integration surface

Where the work would land if the trigger fires: entitlement / licence issuance
(G1); attribution ingress and `AgentTag` minting (G2); witness writer ingress
(G3); capsule export contract plus a reconciling ADR (G4); protection-claim
vocabulary (G5); gate profile table (G6); Horizon 3 observability event model
(G7); onboarding docs plus a factory bootstrap guide (G8); graph cache
artefact (G9).

### Risks and concerns

- **Category risk.** The factory vendor may ship adequate built-in checks,
  capping Anvil at the compliance tier. This argues for leaning into
  audit-chain, capsules, and attribution — the artefacts a factory vendor will
  not build, because they exist to hold the factory accountable.
- **Scope drift.** "Serve autonomous pipelines" could pull Anvil towards agent
  orchestration, explicitly out of scope [C19]. Every increment above must pass
  the prevention test.
- **Doctrine collision.** G4 cannot be built without reconciling ADR-059 and
  ADR-035. Building it first and reconciling later would be the wrong order.
- **Dilution.** The existing ICP is a developer at a keyboard. Serving both
  without a clear packaging split risks a product that onboards neither well.
- **Silent-no-op reputational risk (G1).** A customer who believes they are
  protected and is not is worse than one who knows they are unprotected —
  directly against the honest-claim doctrine.
- **Timing.** No design partner exists today. Everything past G5 / G6 / G8 is
  speculative until one does.

### Final verdict

If I were making the decision today, I would **write the product note, close G5,
G6, and G8 on their own merits, and hold G1–G4 behind a named design partner**,
because the north star and the evidence model already fit this customer better
than they fit the current one — but the blocker is a commercial-model question
(what is a seat when the developer is a fleet?) that no amount of engineering
answers, and building the ephemeral-deployment surface before someone is waiting
for it would be roadmap drift dressed up as foresight.

---

## 9. Evidence citations

Every non-obvious claim above, traced to source. First read at commit
`5598086` (2026-08-18); **re-verified against `9b2ea85`** (`main`, 2026-08-20)
after 388 commits of churn moved several anchors. Line numbers below are the
re-verified ones. Every claim survived re-verification unchanged; the only
substantive movement was C9, where the current source states the contract more
sharply than the original reading did (`decision: "gateUnavailable"`,
`safeDefault: "allow-with-warning"`), which strengthens G1 rather than
weakening it.

| Ref  | Claim                                            | Source                                                                     |
| ---- | ------------------------------------------------ | -------------------------------------------------------------------------- |
| C1   | Mission places value at file-save time           | `ROADMAP.md` §Mission                                                      |
| C2   | Durable state tracked in-repo                    | `crates/anvil-baseline/src/io.rs:16`; `.gitignore:81-83`; ADR-073          |
| C3   | Stdio MCP server runs standalone                 | `crates/anvil-cli/tests/mcp_serve_stdio.rs`                                |
| C4   | Exit-code contract; SARIF                        | `docs/public/anvil/reference/cli.md:345`; `docs/public/anvil/integrations/github.md` |
| C5   | `ai` profile: curated checks, JSON default, `strict_config` | `crates/anvil-cli/src/commands/gate.rs:81-98`                  |
| C6   | Air-gapped core                                  | `crates/anvil-cli/tests/air_gapped.rs`                                     |
| C7   | Per-user plan; device-flow login                 | `docs/guides/account-plan-activity-and-entitlements.md:28-56`              |
| C8   | `ANVIL_LICENSE` is the automation path           | `crates/anvil-cli/src/auth/credentials.rs:141-162`; `docs/public/anvil/integrations/github.md` |
| C9   | Auth-missing returns `gateUnavailable`, not `block` | `crates/anvil-cli/src/commands/mcp.rs:501-535`                                 |
| C10  | `AgentTag` daemon-minted; env advisory/forgeable | `crates/anvil-intercept-proto/src/session.rs:31-65`                              |
| C11  | Witness appended by git hooks; path              | `crates/anvil-cli/src/commands/hook.rs:204`; `crates/anvil-witness/src/manifest.rs:50` |
| C12  | `audit-chain` reports unwitnessed commits        | `crates/anvil-cli/src/commands/audit_chain.rs:1-6`                         |
| C13  | Capsule `--out` default; in-repo staging opt-in  | ADR-078 via `plans/decisions/DECISION-LOG.md`                              |
| C14  | Protection claim daemon-sourced; embedded fallback | `crates/anvil-cli/src/mcp/validation.rs:12-45`                           |
| C15  | `protecting` requires a proven pre-write path    | `docs/public/anvil/integrations/mcp.md`                                    |
| C16  | Warnings do not fail the gate by default         | `docs/public/anvil/concepts/gates.md`                                      |
| C17  | MCP no-config fallback is `interrupt`            | `crates/anvil-cli/src/mcp/enforcement.rs:25`                               |
| C18  | Insights are local-only                          | `docs/public/anvil/reference/cli.md:66`                                    |
| C19  | Observability conditional; orchestration out of scope | `docs/vision/anvil-scope-guard.md`                                    |
| C20  | First-touch wow posture                          | `ROADMAP.md` §Posture                                                      |
| C21  | No headless / ephemeral horizon                  | `ROADMAP.md` §Horizons 3–5, §Big bets                                      |
