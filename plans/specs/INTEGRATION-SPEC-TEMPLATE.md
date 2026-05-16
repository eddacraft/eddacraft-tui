# Integration Spec Template

> Use this template for the **integration phase** of any non-trivial APS work
> item — the phase that wires primitives together across module boundaries,
> rather than implementing a single self-contained primitive.
>
> An execution plan (`plans/execution/<id>.steps.md`) tells you *what to do
> next*. An integration spec tells you *what the wire and the seams are*.
> The two are complementary: the spec is the data contract, the execution
> plan is the TDD checklist.
>
> **When to write one:** if you cannot answer all four of these without
> opening a code editor, you need an integration spec before TDD —
>
> 1. What new fields cross every component boundary, with their serde
>    attributes and wire-additive guarantees?
> 2. Which function in which file is the call site for the new
>    cross-component method, and what is in scope there?
> 3. Which existing production callers must migrate, and which can stay?
> 4. For every error path, what is the deny / allow / record verdict?
>
> If you can answer them, you probably don't need this template. Skip it.

---

## File location

`plans/specs/<date>-<slug>.md` where `<date>` is `YYYY-MM-DD` and `<slug>`
is kebab-case. One spec per integration phase. Reference it from the
relevant APS module file and from the execution plan that follows it.

## Required sections (in order)

Each section header below is verbatim. Drop a section only if the
"Drop when" guidance applies; never reorder.

### 1. Identifiers and status

| Field | Value |
| ----- | ----- |
| Spec id | `<date>-<slug>` |
| Status | `Draft` / `In Review` / `Accepted` / `Superseded by <id>` |
| Date | `YYYY-MM-DD` |
| Owners | `@<user>` |
| Work item | `MODULE-NNN` (the APS task this spec contracts) |
| Supersedes | `<spec-id>` or `—` |
| Council tier | `quick` / `mini` / `full` (for the spec review, not the impl) |

### 2. Problem statement

One paragraph. State the integration question this spec answers and why
the existing surface cannot answer it without ambiguity. Reference the
specific blocker(s) from prior planning that prompted this spec.

Do not restate the APS Intent verbatim — that's the strategic outcome.
This is the *contract question*.

### 3. Data shapes

Every struct, enum, and field that crosses a component boundary. For
each:

- Field name, Rust type, optionality, serde attributes.
- For new fields: which version added them, whether old readers tolerate
  their presence (`#[serde(default)]` + `skip_serializing_if`).
- Concrete example payloads. JSON for IPC, Rust literals for in-process.

Reuse the in-tree wire-additive precedent every time: `#[serde(default,
skip_serializing_if = "Option::is_none")]` for new optional fields.
Don't bump struct versions; add fields.

Drop when: this is a pure-Rust change with no boundary-crossing types.

### 4. Message flow

A numbered sequence of arrows from initiator → … → response. For every
arrow, note:

- The function being called (with the file:line if it exists today).
- The arguments crossing the seam.
- What lock or scope is held while it happens.
- Whether the arrow can fail and which error variant it returns.

ASCII or Mermaid; either is fine. The point is the arrows and what they
carry, not the rendering.

Drop when: no cross-component call path. Rare for integration specs.

### 5. Function signatures

`pub fn` declarations for every method that's new, changed, or
deprecated. For each:

- Full signature, with lifetimes if any.
- Pre-conditions (what the caller must have already done).
- Post-conditions (what state the function leaves the world in).
- Errors returned and the meaning of each.
- Lock-acquisition ordering if more than one mutex is involved.

If a function takes a closure or hook, document the closure's lifetime
and whether it runs under any lock.

Drop when: the spec only introduces serde changes with no method-level
surface. Rare.

### 6. Lifecycle and invariants

When things are created, when they're dropped, what is guaranteed to be
consistent at each program point. Section breaks:

- **Creation:** what triggers it, who owns the result.
- **Liveness:** what keeps it alive, what removes it.
- **Consistency invariants:** facts that hold at every observable
  point. State them as `inv-N:` so they can be cited from §7.

Same role as DB transaction boundaries. If your spec has no shared
mutable state, this section is short.

Drop when: no shared mutable state. Don't drop just to save space.

### 7. Error channel

Every error variant the new path can produce, with its handling. For
each:

- The variant.
- Whether it's a wire error (JSON-RPC) or in-process.
- The recovery path: deny / allow / record fence / record telemetry.
- For security-critical surfaces: the deny-by-default verdict.

If a security boundary, name it explicitly. Spell out the trust
assumption: who is trusted, what privilege boundary is crossed, what
the failure mode is when the assumption breaks.

Drop when: the spec only adds optimistic-path code with no new failure
modes. Rare.

### 8. Observability contract

Every telemetry event the new code emits.

- Notification envelope: target / source / reason / payload schema.
- `tracing::*!` calls: macro level, target string, structured fields.
- `pub const` literals for any reason strings (single find-target for
  future migrations).

Notification and tracing are two channels — say which one each event
goes through, and whether both are required. Defer the decision until
the spec is approved, not until impl.

Drop when: the new code adds no observability surface.

### 9. Migration plan

A table of every existing production caller affected by the integration:

| Site | File:line | Action | Reason |
| ---- | --------- | ------ | ------ |

Actions: `migrate`, `unchanged`, `delete`, `defer-to-<id>`.

Test sites are usually `unchanged`. Document why if not.

Drop when: this is a greenfield surface with no migration. Note the
greenfield status explicitly so reviewers don't wonder.

### 10. Open questions

Explicit, before-implementation. Each question:

- Quote the question precisely.
- List the candidate answers.
- State the spec's chosen answer with one-sentence rationale.
- If no chosen answer: mark `BLOCKING` and route to council debate.

The Open Questions section is the integration-spec's most important
section. If reviewers add questions during review, append them here
with the same structure.

A spec with `BLOCKING` open questions is not yet `Accepted`.

---

## Reviewer checklist (for the spec review, not the impl)

- [ ] §3 — every new wire field has a `#[serde]` attribute and an
      explicit additive guarantee.
- [ ] §4 — every arrow has a file:line citation or a "greenfield"
      label.
- [ ] §5 — every new signature has pre-conditions and post-conditions.
- [ ] §7 — every error has a recovery verdict, not just a description.
- [ ] §9 — every migration site has a chosen action.
- [ ] §10 — no `BLOCKING` questions; or, if any, the spec is not
      `Accepted` yet.

## Notes for spec writers

- One fact per line. The spec is a contract, not prose.
- Cite file:line liberally; line numbers rot but the citation is still
  load-bearing during the review.
- A spec that says "TBD during impl" for any boundary-crossing detail
  has failed its purpose. Decide it in the spec, or label it
  `BLOCKING` in §10.
- The spec's review is `quick` or `mini` Council. Reserve `full` for
  surfaces that change a security boundary or a release-operating-model
  contract.
