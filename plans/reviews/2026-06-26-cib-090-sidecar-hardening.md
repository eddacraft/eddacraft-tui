# CIB-090 Mini Council — Kindling Sidecar Hardening

| Type   | Authority | Owner | Status |
| ------ | --------- | ----- | ------ |
| Review | Advisory  | CIB   | Closed |

## Scope

- Work item: CIB-090
- Target: `crates/anvil-cli/src/usage.rs`
- Review tier: mini
- Roles: security, adversarial
- Date: 2026-06-26

## Verdict

**WARN → conditions addressed in implementation.**

The initial leaf-only `O_NOFOLLOW` plan was not enough to satisfy the parent-dir
writer threat in CIB-090. The implementation therefore expands the hardening to:

- validate and tighten the final `kindling/` parent directory on Unix;
- open sidecar leaves relative to a validated parent directory fd with
  `O_NOFOLLOW`;
- read sidecars for retention through the same no-follow path;
- replace the deterministic `.trim.tmp` path with unique `create_new` temp
  files;
- track Windows reparse-point parity separately as CIB-105 rather than claiming
  cross-platform completion.

## Findings and resolution

| Severity | Role | Finding | Resolution |
| -------- | ---- | ------- | ---------- |
| Major | Adversarial | Leaf-only `O_NOFOLLOW` does not protect a symlinked or swapped parent directory. | Added Unix parent validation/tightening and parent-fd anchored leaf opens. |
| Major | Adversarial | Deterministic trim temp path can be pre-created, swapped, or clobbered. | Replaced with unique `create_new` temp names and parent-fd anchored `renameat` on Unix. |
| Major | Security | Trim reads could still follow a symlink before the hardened append open. | Retention metadata, first-line, and full-read paths now use no-follow read opens. |
| Minor | Security | Unix-only hardening must not overclaim platform-equivalent Windows behaviour. | CIB-090 text narrowed to Unix; CIB-105 records Windows reparse-point parity. |

## Evidence

- `cargo test -p eddacraft-anvil usage::tests::`
- `cargo test -p eddacraft-anvil --test usage_observation --test usage_views --test report_fp`
- `cargo clippy -p eddacraft-anvil --all-targets -- -D warnings`
- `cargo fmt --check`
- `cargo hakari generate --diff && cargo hakari verify`
- `pnpm aps:index:check`
- `pnpm aps:active-lint`
- `pnpm docs:check`

## Notes

`pnpm rebuild` was attempted as a dependency-artifact refresh check but failed in
the Nx postinstall lifecycle with Node `spawn E2BIG`, even when rerun with a
stripped environment and elevated filesystem access. The repo's actual Rust
dependency freshness gate for this change is Hakari; `cargo hakari generate
--diff && cargo hakari verify` passed.
