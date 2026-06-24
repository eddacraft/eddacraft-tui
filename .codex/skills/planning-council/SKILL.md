---
name: planning-council
description: |
  Multi-persona planning council inspired by BMAD party mode. Multiple specialist
  personas interrogate the problem, negotiate the best approach, then synthesize
  architecture docs, specifications, and APS plans.
---

# Planning Council

Multi-persona planning council. Specialist agents interrogate the problem, negotiate the best solution, then produce architecture decisions, module specs, and APS plans.

## Trigger

Activate when the user says: `planning-council`, `planning council`, `plan council`, `architecture council`, `design council`, `party mode planning`, or variants like "get the council to plan this", "multi-persona planning".

## Parse Arguments

```
/planning-council                      → Full 4-phase council on a new problem
/planning-council --extended           → Use all 5 personas in interrogation
/planning-council interrogate          → Run only the question phase
/planning-council negotiate            → Continue to negotiation (requires prior interrogation)
/planning-council synthesize           → Generate deliverables from existing session
/planning-council status               → Show current planning session state
/planning-council resume <session-id>  → Resume an interrupted session
/planning-council --no-review          → Skip Phase 4 review
```

If the user provides a problem description inline (e.g., `/planning-council "build a real-time notification system"`), use it as the problem statement. Otherwise, ask for it.

## Infrastructure

Reuses the Council session infrastructure at `.claude/council/`:

| Script                | Purpose                                                    |
| --------------------- | ---------------------------------------------------------- |
| `council-session.sh`  | Session CRUD (with `--prefix plan` for planning sessions)  |
| `council-finding.sh`  | Not used directly (planning uses objections, not findings) |
| `council-evidence.sh` | Not used directly                                          |
| `council-publish.sh`  | Not used directly                                          |

Schema: `.claude/council/schema.json` (see `planningSession` definition)
Sessions: `.claude/council/sessions/plan-{8hex}.json`
Synthesizer: `.claude/agents/plan-synthesizer.md`

## Personas

### Standard Pack (default)

| Role              | Agent                  | Planning Focus                                                                   |
| ----------------- | ---------------------- | -------------------------------------------------------------------------------- |
| Systems Architect | `architect`            | System boundaries, components, data flow, scalability, technology choices        |
| Delivery Lead     | `pragmatic-lead`       | Scope control, phasing, MVP vs full scope, "what can we cut?", timeline reality  |
| Devil's Advocate  | `adversarial-reviewer` | Challenge assumptions, find missing requirements, poke holes, "what if X fails?" |

### Extended Pack (`--extended`)

Adds to standard pack:

| Role                 | Agent                 | Added Perspective                                                      |
| -------------------- | --------------------- | ---------------------------------------------------------------------- |
| Security Strategist  | `security-analyst`    | Threat model, auth design, data classification, compliance constraints |
| Reliability Engineer | `operations-reviewer` | Deployment strategy, failure modes, observability, scaling concerns    |

### Synthesizer (Phase 3 only)

| Role             | Agent              | Purpose                                                       |
| ---------------- | ------------------ | ------------------------------------------------------------- |
| Plan Synthesizer | `plan-synthesizer` | Converts negotiation outcomes into APS-compliant deliverables |

## Phase 1: INTERROGATION

**Goal:** Surface unknowns, assumptions, and missing requirements before any design work. This phase is **interactive** — questions are presented in themed rounds using AskUserQuestion, not dumped as a batch.

### Step 1: Initialize session

```bash
SESSION_ID=$(bash .claude/council/council-session.sh init \
  --prefix plan \
  --mode streaming \
  --target worktree \
  --pack quick)
```

Note: We reuse the council init command for session creation. The `--prefix plan` flag generates a `plan-{8hex}` ID. The mode/target/pack fields are inherited from the review schema but not semantically meaningful for planning — the planning-specific fields (phase, problem, interrogation, negotiations) are managed directly via jq updates to the session file.

After init, immediately patch the session file to add planning-specific fields:

```bash
SESSION_FILE=".claude/council/sessions/${SESSION_ID}.json"
jq --arg problem "$PROBLEM" \
   --arg pack "$PACK" \
   '. + {
     phase: "interrogation",
     problem: $problem,
     pack: $pack,
     personas: [],
     interrogation: {rounds: [], answers: {}},
     negotiations: [],
     objections: [],
     deliverables: {}
   }' "$SESSION_FILE" > "${SESSION_FILE}.tmp" && mv "${SESSION_FILE}.tmp" "$SESSION_FILE"
```

### Step 2: Dispatch persona questions

Spawn personas in parallel using the Agent tool. Each persona receives the problem statement and a planning-specific prompt overlay.

**For each persona, use this prompt template:**

```
You are participating in a Planning Council as the {ROLE}.

**Problem Statement:**
{user's problem description}

**Your task:** Generate 3-5 questions that MUST be answered before designing a solution.
For each question, also suggest 2-3 likely answer options the user might pick from.

Focus on your specialty:
- {ROLE-SPECIFIC FOCUS AREAS}

Return ONLY a JSON object:
{
  "questions": [
    {
      "id": "Q-001",
      "category": "requirements|constraints|scope|risk|integration",
      "question": "The actual question",
      "why": "Why this matters for the design",
      "default_assumption": "If not answered, I will assume...",
      "options": [
        {"label": "Short option name", "description": "What this choice means"},
        {"label": "Another option", "description": "What this choice means"}
      ]
    }
  ]
}
```

**Role-specific focus areas:**

- **Systems Architect:** Component boundaries, data flow, integration points, technology constraints, scaling requirements, existing system dependencies
- **Delivery Lead:** MVP scope, must-have vs nice-to-have, team capabilities, deployment timeline, phasing strategy, maintenance burden
- **Devil's Advocate:** Failure modes, edge cases, assumptions being made, what happens when things go wrong, migration risks, hidden complexity
- **Security Strategist** (extended only): Authentication model, data sensitivity, compliance requirements, threat vectors, trust boundaries
- **Reliability Engineer** (extended only): SLO targets, deployment model, monitoring needs, disaster recovery, capacity planning

### Step 3: Collect, deduplicate, and group into rounds

After all personas return:

1. Parse JSON from each persona's response
2. Assign unique IDs across all questions (Q-001 through Q-NNN, renumbering if needed)
3. Tag each question with its source persona
4. Deduplicate: if two personas ask essentially the same question, merge them (keep both sources noted, combine option lists)
5. **Group into themed rounds** of 2-3 questions each. Typical round themes:
   - **Scope & Requirements** — what are we building, for whom, at what scale
   - **Architecture & Constraints** — technology choices, integration points, existing systems
   - **Risk & Edge Cases** — failure modes, what-ifs, migration concerns
   - **Delivery & Phasing** — MVP scope, timeline, maintenance
   - (Extended pack adds: **Security & Compliance**, **Reliability & Operations**)
6. Store all rounds in the session file under `interrogation.rounds`

### Step 4: Interactive guided rounds

Present each round to the user using **AskUserQuestion**. This is a conversation, not a questionnaire.

**For each round:**

1. Announce the round theme in a brief sentence (e.g., "Let's nail down scope and requirements.")
2. Use AskUserQuestion with up to 4 questions per call. For each question:
   - `question`: The persona's question text (include a brief "why" in the description of the first option or as question preamble if it helps)
   - `header`: Short category tag (e.g., "Scale", "MVP", "Auth")
   - `options`: 2-3 answer options from the persona output, plus always include an "Accept default" option with the default assumption as its description
   - The built-in "Other" option gives the user free text input
   - `multiSelect: false`
3. After the user responds, record answers in `interrogation.answers`
4. **Adapt:** If a user's answer reveals a new constraint or changes the problem scope, note it. You may:
   - Reword remaining questions to account for the new context
   - Drop questions that are now irrelevant
   - Add a follow-up question to the next round (max 1 per round)

**Escape hatch:** If at any point the user says "skip the rest", "defaults for remaining", or similar:

- Accept all remaining default assumptions
- Record them as `"skip: {default_assumption}"` in answers
- Move immediately to Phase 2

**After all rounds are complete (or skipped), update the session:**

```bash
jq '.phase = "negotiation"
    | .events += [{type: "interrogation_completed", timestamp: now | todate, detail: "Interrogation complete"}]' \
  "$SESSION_FILE" > "${SESSION_FILE}.tmp" && mv "${SESSION_FILE}.tmp" "$SESSION_FILE"
```

**Summary gate:** Before moving to negotiation, present a brief summary of the key answers and assumptions. Ask the user to confirm or flag anything to revisit. This is a single AskUserQuestion:

- "Ready to move to negotiation?" with options: "Looks good, proceed" / "I want to revisit something" / "Add a constraint I forgot to mention"

## Phase 2: NEGOTIATION

**Goal:** Converge on architecture decisions through structured multi-agent debate.

### Step 1: Identify decision topics

From the interrogation answers, identify 2-4 topics that need negotiation:

1. Questions where the user answered `"negotiate"` — these are explicit debate topics
2. Answers that imply a significant design choice (e.g., "we need real-time updates" → debate on WebSocket vs SSE vs polling)
3. Conflicting constraints surfaced during interrogation

For each topic, select the most relevant agent pair:

| Decision Type                   | Agent Pair                              |
| ------------------------------- | --------------------------------------- |
| System architecture, API design | architect + pragmatic-lead              |
| Security vs usability tradeoff  | architect + security-analyst            |
| Performance vs maintainability  | architect + adversarial-reviewer        |
| Deployment & scaling strategy   | architect + operations-reviewer         |
| Scope & phasing                 | pragmatic-lead + adversarial-reviewer   |
| Risk assessment                 | adversarial-reviewer + security-analyst |

### Step 2: Run negotiations

For each topic, follow the existing **agent-negotiation** protocol from `.claude/skills/agent-negotiation/SKILL.md`:

1. Spawn Agent A with the topic + interrogation context → returns position ending with CONSENSUS/COUNTER/QUESTION
2. Spawn Agent B with Agent A's position → responds with CONSENSUS/COUNTER/QUESTION
3. Continue until CONSENSUS or max rounds (default: 4, configurable via `CLAUDE_NEGOTIATION_MAX_ROUNDS`)
4. If DEADLOCK after max rounds: record both positions, mark as open question

**Important context to include in negotiation prompts:**

- The full problem statement
- All interrogation Q&A (so agents have the same information)
- The specific topic being debated
- Any constraints the user stated as non-negotiable

Independent topics can be negotiated in parallel (use multiple Agent tool calls in one message).

### Step 3: Record outcomes

For each negotiation, store the result in the session:

```json
{
  "topic": "Real-time update mechanism",
  "participants": ["architect", "pragmatic-lead"],
  "outcome": "consensus",
  "result": "Use Server-Sent Events for unidirectional updates with WebSocket upgrade path for bidirectional needs",
  "rounds": 2,
  "history": [
    { "agent": "architect", "position": "WebSocket for flexibility", "signal": "COUNTER" },
    { "agent": "pragmatic-lead", "position": "SSE simpler, upgrade later", "signal": "COUNTER" },
    { "agent": "architect", "position": "SSE now, WebSocket when needed", "signal": "CONSENSUS" },
    { "agent": "pragmatic-lead", "position": "Agreed", "signal": "CONSENSUS" }
  ]
}
```

### Step 4: Optional panel round

If extended pack and cross-cutting concerns exist (e.g., security implications of the architecture choice):

1. Compile all pairwise negotiation outcomes
2. Send to all 5 personas simultaneously: "Review these decisions. Raise a COUNTER if any decision conflicts with your area of expertise. Otherwise respond CONSENSUS."
3. If any COUNTER is raised, run one final focused negotiation between the objecting persona and the relevant decision's original participants

Update session phase:

```bash
jq '.phase = "synthesis"
    | .events += [{type: "negotiation_completed", timestamp: now | todate, detail: "All negotiations resolved"}]' \
  "$SESSION_FILE" > "${SESSION_FILE}.tmp" && mv "${SESSION_FILE}.tmp" "$SESSION_FILE"
```

## Phase 3: SYNTHESIS

**Goal:** Convert negotiation outcomes into concrete APS deliverables.

### Step 1: Prepare synthesis context

Gather the full session context:

- Problem statement
- All interrogation Q&A
- All negotiation outcomes (consensus and deadlock)
- Existing plan state (read `plans/index.aps.md` and `plans/modules/` if they exist)
- Next available decision number (check `plans/decisions/`)
- Next available module number (check `plans/modules/`)

### Step 2: Dispatch plan-synthesizer

Spawn the `plan-synthesizer` agent with the full context:

```
Use Agent tool with:
  subagent_type: plan-synthesizer (or use the general agent with .claude/agents/plan-synthesizer.md prompt)
  prompt: Include all session context + explicit instructions to generate:
    1. Architecture decision document in plans/decisions/
    2. One or more APS module specs in plans/modules/
    3. Updates to plans/index.aps.md if needed
```

The synthesizer writes files directly.

### Step 3: Record deliverables

After the synthesizer completes, record the file paths in the session:

```bash
jq --arg arch "plans/decisions/NNN-title.md" \
   --argjson modules '["plans/modules/NN-name.aps.md"]' \
   --argjson indexUpdated true \
   '.deliverables = {architecture: $arch, modules: $modules, indexUpdated: $indexUpdated}
    | .phase = "review"
    | .events += [{type: "synthesis_completed", timestamp: now | todate, detail: "Deliverables generated"}]' \
  "$SESSION_FILE" > "${SESSION_FILE}.tmp" && mv "${SESSION_FILE}.tmp" "$SESSION_FILE"
```

## Phase 4: REVIEW (Optional)

**Goal:** Quick validation of synthesized deliverables. Skip with `--no-review`.

### Step 1: Dispatch reviewers

Spawn architect and adversarial-reviewer in parallel. Each receives:

- The generated deliverables (architecture doc + module specs)
- The original problem statement and interrogation answers (for reference)

**Prompt template:**

```
You are reviewing Planning Council deliverables as the {ROLE}.

**Original Problem:** {problem statement}

**Generated Deliverables:**
{contents of architecture doc and module specs}

Review for:
- Completeness: do the deliverables cover all negotiation outcomes?
- Consistency: do decisions and specs align?
- APS compliance: do tasks describe intent, not implementation?
- Gaps: is anything from the interrogation/negotiation missing?

Return ONLY a JSON object:
{
  "objections": [
    {
      "target": "plans/decisions/NNN-title.md|plans/modules/NN-name.aps.md",
      "severity": "critical|major|minor",
      "description": "What's wrong",
      "suggestion": "How to fix it"
    }
  ],
  "approved": true|false
}

If no issues: {"objections": [], "approved": true}
```

### Step 2: Handle objections

If objections exist:

1. Record objections in the session
2. For critical/major: re-dispatch plan-synthesizer with the objections as revision instructions
3. For minor: note them but don't block convergence
4. After revision, mark objections as resolved

### Step 3: Converge

When no open critical/major objections remain:

```bash
bash .claude/council/council-session.sh close "$SESSION_ID" --status converged
```

Present the final deliverable summary to the user.

## Workflow: Status

```bash
# Find planning sessions specifically
ls -t .claude/council/sessions/plan-*.json 2>/dev/null | head -1
# Or use council-session.sh
bash .claude/council/council-session.sh status [session-id]
```

Show: session ID, current phase, problem statement, persona pack, number of questions answered, number of negotiations completed, deliverable paths.

## Workflow: Resume

```bash
bash .claude/council/council-session.sh resume <session-id>
```

Read the session file and continue from the current phase:

- `interrogation` → continue asking unanswered questions
- `negotiation` → continue with remaining topics
- `synthesis` → re-run synthesizer
- `review` → re-run reviewers

## Integration

- **`/plan`**: Planning council is a superset. Its APS output is identical format. `/plan-status` works on files from either.
- **`/negotiate`**: Phase 2 reuses the agent-negotiation CONSENSUS/COUNTER/QUESTION protocol directly.
- **`/council`**: Independent skill. Shares session storage (distinguished by `plan-` prefix).
- **`parallel-agents`**: Fan-out pattern used in Phase 1 (interrogation) and Phase 4 (review).

## Judgement-Only Mode

For routine council passes between full planning sessions, use the council
infrastructure as a thin orchestrator. This mode is judgement evidence only —
plan files remain the authority for intent and readiness; deterministic checks
remain validation authority. Use when the 4-phase flow above is overkill
(direction validation, pre-execution sign-off, mid-flight amendment) and you
need a multi-role pass that produces a decision plus evidence.

### Modes

| Mode                   | When to use                                                |
| ---------------------- | ---------------------------------------------------------- |
| Create                 | New module, spec, or multi-item plan needing first council |
| Direction validate     | Before Draft/Proposed planning becomes Ready               |
| Pre-execution validate | Before non-trivial Ready work starts                       |
| Amend                  | Repo reality, review, or validation changes the plan       |

### Role Lenses

Keep stable role names separate from runtime agent IDs so a swap in agent
implementation does not invalidate findings or evidence:

| Stable role           | Common runtime agents                         |
| --------------------- | --------------------------------------------- |
| `pragmatic`           | `pragmatic-lead`, `council-pragmatic`         |
| `operations`          | `operations-reviewer`, `council-ops`          |
| `security`            | `security-analyst`, `council-security`        |
| `adversarial`         | `adversarial-reviewer`, `council-adversarial` |
| `general`             | `council-general`, `council-reviewer`         |
| `planner-synthesizer` | `plan-synthesizer`                            |

### Required Outputs

Every pass produces:

- **decision**: `proceed`, `amend`, `split`, `replan`, or `block`
- items reviewed (file paths or identifiers)
- repo reality checked: base branch, changed files, relevant specs/docs
- risks and unresolved questions
- required deterministic checks (commands that must run green)
- plan file updates required before execution, if any

If the decision is not `proceed`, stop execution and update the relevant plan
file first.

## Principles

1. **Questions before answers** — don't design until you understand the problem
2. **Negotiate, don't dictate** — multiple perspectives produce better architecture
3. **Specs describe intent** — deliverables follow APS rules, no implementation detail
4. **Reuse infrastructure** — session management, negotiation protocol, and agent dispatch are existing systems
5. **User stays in control** — interactive Q&A, can skip/negotiate/resume at any point
6. **Existing plans are respected** — new modules extend, they don't replace
