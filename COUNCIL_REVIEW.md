# Council Review: fix/rcli-tier1-surgical

**Branch:** fix/rcli-tier1-surgical **Date:** 2026-03-31 **Commits:** 7 (6
fixes + 1 review fixup)

## Scope

6 surgical fixes across the Rust CLI and kernel crates:

| Item      | Commit     | Files Changed                                                                 |
| --------- | ---------- | ----------------------------------------------------------------------------- |
| RENG-006  | `a385eea1` | engine_mode.rs                                                                |
| RCLI-052  | `8541ae9b` | credentials.rs                                                                |
| RCLI-049  | `251512a0` | gate.rs                                                                       |
| RCLI-013a | `5c4a1bdd` | embedded.rs, gate.rs, kernel.rs, dual_run.rs                                  |
| RCLI-014a | `ad522267` | watch.rs (cli + kernel), architecture.rs                                      |
| RCLI-048  | `0d66fbd7` | util.rs, tutorial.rs, welcome.rs, credentials.rs, init.rs, wizard.rs, main.rs |

## Build & Test

- `cargo build --workspace` — clean
- `cargo test --workspace` — 767 passed, 0 failed

## Findings

### Fixed (in commit `6ccdfa8c`)

| Severity  | File        | Finding                                                         | Resolution                                            |
| --------- | ----------- | --------------------------------------------------------------- | ----------------------------------------------------- |
| Important | embedded.rs | `plan` field doc comment claimed filtering was implemented      | Updated doc to say "not yet consumed by run_embedded" |
| Important | gate.rs     | `checks.last().unwrap()` in production loop                     | Replaced with local `failed` variable                 |
| Important | util.rs     | `with_extension("tmp")` replaced extension instead of appending | Changed to `OsString::push(".tmp")`                   |

### Accepted / Deferred

| Severity | File              | Finding                                                                           | Rationale                                                                                                                                                            |
| -------- | ----------------- | --------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Minor    | credentials.rs    | macOS fallback returns different dir for reads vs writes                          | Intentional migration path: read from old location, write to new. First `save()` creates the XDG path, subsequent reads use it.                                      |
| Minor    | credentials.rs    | `is_expired` has `#[allow(dead_code)]`                                            | Pre-existing scaffold, not from this branch.                                                                                                                         |
| Minor    | gate.rs           | `workspace_root()` duplicated in watch.rs                                         | Pre-existing duplication. Extracting to `crate::util` is a separate cleanup (MAINT backlog).                                                                         |
| Minor    | gate.rs           | `no_cache` field scaffold with dead_code allow                                    | Pre-existing, tracked in RCLI backlog.                                                                                                                               |
| Minor    | watch.rs (cli)    | `file` and `action` args are dead_code scaffolds                                  | Pre-existing scaffolds, not from this branch.                                                                                                                        |
| Minor    | architecture.rs   | File-level `#![allow(dead_code)]`                                                 | Pre-existing, not from this branch.                                                                                                                                  |
| Minor    | architecture.rs   | `architecture watch` blocking recv with no ctrlc handler                          | Pre-existing design. The `commands/watch.rs` pattern was not backported here.                                                                                        |
| Minor    | wizard.rs         | Unsanitised project name used as path                                             | Pre-existing — TUI constrains input.                                                                                                                                 |
| Minor    | embedded.rs       | `plan` field is accepted but not consumed by `run_embedded`                       | By design — the field wires CLI to kernel config for future plan-scoped filtering. Consuming it requires file-list extraction from APS plans (separate work item).   |
| Minor    | watch.rs (kernel) | `include_patterns` and `exclude_patterns` accepted but not consumed by watch loop | Same as `plan` — the fields wire CLI to kernel config. The kernel's FileFilter already provides base filtering; glob matching against these patterns is a follow-up. |
| Nit      | util.rs           | No test for missing parent directory                                              | `atomic_write` callers create dirs before calling.                                                                                                                   |
| Nit      | welcome.rs        | `first_run_marker_path` uses relative path                                        | Per-project marker by design.                                                                                                                                        |
| Nit      | init.rs           | `.gitignore` read twice in `append_gitignore_entry`                               | Pre-existing, cosmetic.                                                                                                                                              |

## Risk Assessment

**Low risk.** All changes are additive or simplifying:

- RENG-006 removes dead code (Legacy/Dual modes were never functional)
- RCLI-052 adds a fallback read path, no existing behaviour changes
- RCLI-049 wires an unimplemented check to the kernel (new capability)
- RCLI-013a/014a add fields to config structs (non-breaking)
- RCLI-048 replaces direct writes with atomic write-then-rename (strictly safer)
