# ADR-115: eddacraft-tui `Surface` Trait Evolution

## Status

**Proposed** — 2026-08-01

## Date

2026-08-01

## Context

`eddacraft-tui` grades the downstream-implemented `Surface` trait as stable.
Since 0.4.1, the trait gained a defaulted `text_entry_active` method so event
loops can distinguish free-text entry from navigation without requiring every
existing implementation to add boilerplate.

The default body preserves existing `impl Surface` blocks, but that is not the
whole source-compatibility contract. A downstream crate that imports another
trait with a method of the same name can gain an ambiguous method call after
updating `eddacraft-tui`. The release runbook requires a breaking 0.x bucket and
an ADR for compatibility hazards on stable trait methods.

The next crate release must therefore decide whether to retain the hook and
acknowledge the break, redesign the hook outside `Surface`, or remove it from
the release.

## Decision

Retain `Surface::text_entry_active` and publish the accumulated post-0.4.1
changes as `eddacraft-tui` 0.5.0.

For pre-1.0 releases, adding a method to a stable trait that downstream crates
implement is treated as a compatibility hazard even when the method has a
default body. Such changes require the crate's breaking bucket: a minor version
bump plus an ADR. The changelog must name the ambiguity risk explicitly.

This decision does not change the stability grade of experimental or unstable
surfaces. Their existing no-compatibility-guarantee contract remains in force.

## Rationale

The hook belongs on `Surface`: it is a property of the active surface state,
and keeping the default avoids boilerplate for navigation-only implementations.
Publishing 0.5.0 is more honest than describing implementor compatibility as
complete source compatibility.

### Alternatives Considered

| Option | Pros | Cons |
| ------ | ---- | ---- |
| **Retain the method and publish 0.5.0** | Keeps the direct event-loop contract; accurately signals the stable-trait hazard | Requires a breaking release for an otherwise additive API |
| Move the hook to an extension trait | Keeps `Surface` unchanged | Splits the event-loop contract and still risks method-name collisions when both traits are imported |
| Remove the hook and keep 0.4.2 | Preserves the previous stable trait exactly | Reintroduces literal text-entry bugs or requires a larger event-routing redesign |

## Consequences

- **Positive:** The release version communicates the full downstream
  compatibility risk instead of only implementor compatibility.
- **Positive:** Existing `impl Surface` blocks continue to compile because the
  method remains defaulted to `false`.
- **Negative:** Consumers must opt into the 0.5 series and may need explicitly
  qualified calls if another trait defines `text_entry_active`.
- **Risk:** Future additive stable-trait changes may appear mechanically safe
  because they have defaults.
- **Mitigation:** The release checklist and this ADR make the minor-bump rule
  explicit for downstream-implemented stable traits.

## References

- Related ADRs: [ADR-047](047-eddacraft-tui-canonical-source-mirror.md),
  [ADR-050](050-eddacraft-tui-runner-and-cli-policy.md)
- Release procedure:
  [`docs/runbooks/eddacraft-tui-release.md`](../../docs/runbooks/eddacraft-tui-release.md)
- Stable trait:
  [`crates/eddacraft-tui/src/surface.rs`](../../crates/eddacraft-tui/src/surface.rs)
- APS: operational crate release; no Anvil `releaseIntent` or `releaseScope`
  metadata changes
