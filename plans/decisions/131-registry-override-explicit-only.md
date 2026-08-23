# ADR-131: Anti-pattern registry resolution is explicit-override only

## Status

Proposed

## Date

2026-08-23

## Context

ADR-026 §1 shipped a four-tier registry lookup: explicit path, then
`ANVIL_REGISTRY_PATH`, then a cwd upward walk, then an executable-directory
upward walk, then (later) the compile-time embedded fallback. The walks
look for `patterns/compiled/registry.json`. Nothing outside the ADR, an
internal authoring guide, and the loader itself said this was a product
surface.

ADR-129 inventoried that chain as surface 7 and classified it **Internal
until POLFIT-008**. D-4.8 recorded the shipped behaviour without freezing
it as a product merge rule: the chain is unsigned, first-found, and a
found path silently replaces the embedded catalogue.

POLFIT-008 has to choose: document the four-tier chain as a supported
override with its trust boundary stated, or bound it so a cloned
repository cannot silently replace the anti-pattern catalogue.

The hazard is the implicit walks, not the explicit override. A clone that
contains `patterns/compiled/registry.json` — this repository does, and any
adopting project could — wins over the catalogue baked into the binary,
with no operator action and no integrity check. Documenting that walk as
supported would turn a silent catalogue swap into a product feature.

`ANVIL_REGISTRY_PATH` is already named as a trust boundary in
`docs/guides/anvil-rule-authoring.md`. Rule authors and INSEC validation
already use it as an explicit local override. Closing that path would
break a documented developer workflow. Closing the walks would not.

## Decision

1. **Default catalogue is the compile-time embedded registry.** Stock
   installs, cloned adopting projects, and this workspace itself load
   `<embedded>` unless an explicit override is set. Rebuilding the binary
   refreshes the default catalogue.

2. **The supported override is explicit only.** Resolution order is:

   1. `LoadRegistryOptions.registry_path` (API / tests).
   2. `ANVIL_REGISTRY_PATH` (operator / developer environment).
   3. Compile-time embedded `patterns/compiled/registry.json`.

   Set-but-missing still warns and falls back to embedded (existing
   `#1630` behaviour). There is **no** cwd walk and **no**
   executable-directory walk.

3. **Surface 7 classification (amends ADR-129 D-3).** Anti-pattern
   registry resolution is **Supported as an explicit operator override**.
   It is not an implicit project file and not a pack. A cloned
   `patterns/compiled/registry.json` does not replace the catalogue.

4. **Trust boundary.** The override is unsigned: no hash, no signature,
   no canonicalisation. First explicit winner replaces the whole
   catalogue. Stricter-wins (ADR-129 D-4.4) does not apply. A syntactically
   valid poisoned registry (every `enabled: false`, patterns rewritten to
   match nothing) will not be caught by `registry-patterns-compile`. Do
   not feed untrusted input into `ANVIL_REGISTRY_PATH`. CI jobs should
   rely on the embedded catalogue of the binary under test, not an env
   override from a PR.

5. **Amends ADR-026 §1.** The four-tier walk is no longer the lookup
   contract. The compiled registry remains the authoring contract; only
   how the scanner *finds* it changes.

6. **Public inventory (amends ADR-129 D-6).**
   `docs/public/anvil/concepts/policy-model.md` names the explicit
   override and states that a cloned registry file does not win. It does
   not republish a drop-a-file-in-the-clone recipe. The authoring guide
   remains the operator-facing trust-boundary write-up.

## Rationale

The item's expected outcome is honest about two complete options. The
named hazard is silent clone replacement. Bounding the implicit walks
closes that hazard; documenting the remaining explicit override keeps
the developer path that already exists and already carries a trust
warning.

Keeping the walks and documenting them would satisfy "stated surface"
and fail "cloned repository cannot silently replace". Closing env and
API overrides as well would force rule authors to rebuild the binary for
every catalogue edit.

Signing or hashing the registry is a different decision (already named
as possible follow-up in the authoring guide). This ADR does not add
integrity machinery.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| Explicit override + closed walks (chosen) | Closes clone replacement; keeps the documented env/API path; matches embedded stock-install behaviour | Rule authors editing `patterns/` must rebuild or set `ANVIL_REGISTRY_PATH` |
| Document the four-tier chain as supported | Describes what the loader did; no code change | Makes silent clone replacement a product feature; unsigned winner-takes-all on cwd |
| Close every override, embedded only | Smallest trust surface | Breaks INSEC/dev fixtures and `LoadRegistryOptions.registry_path` tests |
| Keep exe-dir walk, drop cwd walk | Helps source-tree binaries find a sibling `patterns/` | Still implicit; still unsigned; still not an operator action |
| Require hash/signature on any override | Integrity | Out of scope; named follow-up, not this item |

## Consequences

- **Positive:** adopting clones cannot swap the scanner catalogue by
  vendoring `patterns/compiled/registry.json`; public inventory can name
  surface 7 without publishing an implicit recipe; ADR-129's Internal
  holding classification is discharged.
- **Negative:** running a stale binary in this workspace no longer picks
  up an unrebuild `registry.json` via cwd; authors must set
  `ANVIL_REGISTRY_PATH` or rebuild.
- **Risks:** readers treat `ANVIL_REGISTRY_PATH` as a project config key
  rather than a process-environment trust boundary; a future feature
  reintroduces cwd discovery for "custom catalogues".
- **Mitigations:** public page and authoring guide both say the override
  is explicit and unsigned; a unit test fails if default load returns
  the workspace registry path.

## References

- Related ADRs: ADR-026 §1 (lookup chain; amended here), ADR-033
  (TS scanner retired), ADR-129 D-3 / D-4.8 / D-5 / D-6 (inventory;
  amended here)
- APS modules: POLFIT-008 (this record), INSEC (owns families, not
  resolution), DOCDEF (public inventory page)
- Evidence: `crates/anvil-checks/src/antipattern/registry_loader.rs`;
  `docs/guides/anvil-rule-authoring.md` Registry integrity; POLFIT
  audit against `origin/main` @ `7524a599b`
