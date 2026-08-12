//! Hook bootstrap recovery (MLP-008): regenerate minimal runtime files so
//! hooks fire again without a full framework reinstall.

use crate::framework::HookFramework;
use crate::shell::{HookKind, shell_template};

/// One Husky runtime file: a relative path under `.husky/` and the
/// exact bytes to write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HuskyRuntime {
    /// Relative to repo root, e.g. `".husky/_/h"` or
    /// `".husky/_/husky.sh"`. Forward slashes only (the CLI
    /// translates per-platform).
    pub relative_path: String,
    pub contents: String,
    /// Whether the file needs the executable bit set on Unix.
    pub executable: bool,
}

/// Generate the minimal Husky v9 runtime under `.husky/_/`.
///
/// Husky v9's runtime layout is:
///
/// - `.husky/_/h` — the per-hook bootstrap script that runs the
///   matching `.husky/<hook>` file with the right env.
/// - `.husky/_/husky.sh` — kept as a no-op stub for backwards-
///   compat with v8 hook chains that may still source it.
///
/// We do NOT regenerate the hook scripts themselves (the user's
/// `.husky/pre-commit`, etc.) — those carry the Anvil hook line
/// already and shouldn't be touched. We only restore the runtime
/// shim that `pnpm install` would normally install via the
/// `husky` package's postinstall.
pub fn generate_husky_runtime() -> Vec<HuskyRuntime> {
    vec![
        HuskyRuntime {
            relative_path: ".husky/_/h".to_string(),
            contents: HUSKY_RUNTIME_H.to_string(),
            executable: true,
        },
        HuskyRuntime {
            relative_path: ".husky/_/husky.sh".to_string(),
            contents: HUSKY_RUNTIME_HUSKY_SH.to_string(),
            executable: false,
        },
    ]
}

// The runtime files below are taken from Husky v9's published
// runtime. They are intentionally minimal and stable; if Husky
// ever changes the contract, the user can re-run `pnpm install`
// to get the matching version. This bootstrap path exists
// because `pnpm install` hasn't been run yet, not because we
// want to replace Husky.
// Preserve the target hook's exit status. Only treat a missing or
// non-executable regular-file target as a no-op (Husky contract). Do
// not append `|| true` after the invocation — that would swallow
// block decisions and any failing user hook (see ADR-038 noise
// discipline). `shift` drops the target selector (`$1`) so the hook
// sees only the original git hook arguments.
const HUSKY_RUNTIME_H: &str = "#!/usr/bin/env sh\n[ \"$HUSKY\" = \"0\" ] && exit 0\nh=\"$(dirname -- \"$0\")/$1\"\n[ -f \"$h\" ] && [ -x \"$h\" ] || exit 0\nshift\n\"$h\" \"$@\"\n";

const HUSKY_RUNTIME_HUSKY_SH: &str =
    "# husky v8 back-compat stub; husky v9 uses .husky/_/h directly.\n";

/// The structured plan returned by [`build_bootstrap_plan`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootstrapPlan {
    /// Husky was detected; regenerate the runtime files.
    HuskyRegenerate { files: Vec<HuskyRuntime> },
    /// Lefthook / pre-commit-framework / cargo-husky / Plain —
    /// nothing to regenerate at bootstrap time; the framework's own
    /// install path is the right answer.
    NothingToDo { framework: HookFramework },
    /// Install the Anvil hooks fresh at `.git/hooks/<kind>` because
    /// no framework is present. Returns one `(filename, contents)`
    /// per hook in [`crate::HookKind`]'s v1 set.
    InstallPlain { files: Vec<PlainHookFile> },
}

/// One hook file the CLI writes under `.git/hooks/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlainHookFile {
    /// Hook filename (matches git's naming).
    pub filename: String,
    /// Template body from [`crate::shell_template`].
    pub contents: String,
}

/// Build the bootstrap plan for the detected framework.
pub fn build_bootstrap_plan(framework: HookFramework) -> BootstrapPlan {
    match framework {
        HookFramework::Husky => BootstrapPlan::HuskyRegenerate {
            files: generate_husky_runtime(),
        },
        HookFramework::Plain => {
            let files = [
                HookKind::PreCommit,
                HookKind::PostCommit,
                HookKind::PrePush,
                HookKind::PostMerge,
                HookKind::PostRewrite,
            ]
            .into_iter()
            .map(|k| PlainHookFile {
                filename: k.filename().to_string(),
                contents: shell_template(k),
            })
            .collect();
            BootstrapPlan::InstallPlain { files }
        }
        other => BootstrapPlan::NothingToDo { framework: other },
    }
}

/// Pinned `validation_at` string for retroactive witnesses produced
/// by `anvil hook bootstrap --witness-recent` (MLP2-037). Lets
/// downstream readers distinguish a worktree-bootstrap recovery walk
/// from the pre-commit / post-rewrite-recovery sources. Stable; do
/// not drift.
pub const BOOTSTRAP_RECOVERY_VALIDATION_AT: &str = "bootstrap-recovery";

/// Build the one-line success message per the MLP-008 spec.
///
/// Format: `anvil: bootstrapped (N commits witnessed retroactively)`.
/// When `commits_witnessed` is zero (e.g. bootstrap was just
/// runtime-file regeneration with no `--witness-recent`), the
/// message drops the count parenthetical.
pub fn render_success_message(commits_witnessed: usize) -> String {
    if commits_witnessed == 0 {
        "anvil: bootstrapped".to_string()
    } else {
        format!("anvil: bootstrapped ({commits_witnessed} commits witnessed retroactively)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn husky_runtime_includes_both_files() {
        let files = generate_husky_runtime();
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.relative_path == ".husky/_/h"));
        assert!(files.iter().any(|f| f.relative_path == ".husky/_/husky.sh"));
    }

    #[test]
    fn husky_h_is_executable() {
        let files = generate_husky_runtime();
        let h = files
            .iter()
            .find(|f| f.relative_path == ".husky/_/h")
            .unwrap();
        assert!(h.executable);
    }

    #[test]
    fn husky_h_has_posix_shebang() {
        let files = generate_husky_runtime();
        let h = files
            .iter()
            .find(|f| f.relative_path == ".husky/_/h")
            .unwrap();
        assert!(h.contents.starts_with("#!/usr/bin/env sh"));
    }

    #[test]
    fn husky_h_respects_husky_zero_env_var() {
        // Husky's contract: `HUSKY=0` skips all hooks. Don't drift.
        let files = generate_husky_runtime();
        let h = files
            .iter()
            .find(|f| f.relative_path == ".husky/_/h")
            .unwrap();
        assert!(h.contents.contains("HUSKY"));
        assert!(h.contents.contains("= \"0\""));
        assert!(h.contents.contains("exit 0"));
    }

    #[test]
    fn bootstrap_plan_for_husky_regenerates_runtime() {
        let plan = build_bootstrap_plan(HookFramework::Husky);
        match plan {
            BootstrapPlan::HuskyRegenerate { files } => {
                assert_eq!(files.len(), 2);
            }
            other => panic!("expected HuskyRegenerate, got {other:?}"),
        }
    }

    #[test]
    fn bootstrap_plan_for_plain_installs_all_v1_hooks() {
        let plan = build_bootstrap_plan(HookFramework::Plain);
        match plan {
            BootstrapPlan::InstallPlain { files } => {
                let names: Vec<&str> = files.iter().map(|f| f.filename.as_str()).collect();
                assert!(names.contains(&"pre-commit"));
                assert!(names.contains(&"post-commit"));
                assert!(names.contains(&"pre-push"));
                assert!(names.contains(&"post-merge"));
                assert!(names.contains(&"post-rewrite"));
                // Each file uses the shared shell template.
                for f in &files {
                    assert!(f.contents.starts_with("#!/bin/sh"));
                    assert!(f.contents.contains("command -v anvil"));
                    assert!(f.contents.contains("exec anvil hook"));
                }
            }
            other => panic!("expected InstallPlain, got {other:?}"),
        }
    }

    #[test]
    fn bootstrap_plan_for_lefthook_does_nothing() {
        let plan = build_bootstrap_plan(HookFramework::Lefthook);
        assert!(matches!(
            plan,
            BootstrapPlan::NothingToDo {
                framework: HookFramework::Lefthook
            }
        ));
    }

    #[test]
    fn bootstrap_plan_for_pre_commit_framework_does_nothing() {
        // The pre-commit-framework's own `pre-commit install` is the
        // user's responsibility; bootstrap doesn't second-guess it.
        let plan = build_bootstrap_plan(HookFramework::PreCommitFramework);
        assert!(matches!(plan, BootstrapPlan::NothingToDo { .. }));
    }

    #[test]
    fn bootstrap_plan_for_cargo_husky_does_nothing() {
        let plan = build_bootstrap_plan(HookFramework::CargoHusky);
        assert!(matches!(plan, BootstrapPlan::NothingToDo { .. }));
    }

    #[test]
    fn success_message_zero_commits_drops_parenthetical() {
        let m = render_success_message(0);
        assert_eq!(m, "anvil: bootstrapped");
        assert!(!m.contains('('));
    }

    #[test]
    fn success_message_n_commits_includes_count() {
        let m = render_success_message(3);
        // Pin the exact MLP-008 spec format.
        assert_eq!(m, "anvil: bootstrapped (3 commits witnessed retroactively)");
    }

    #[test]
    fn success_message_one_commit_uses_plural_for_simplicity() {
        // ADR-038 noise discipline prefers terse over grammatically
        // correct. "1 commits" is uglier than necessary but adding
        // the singular branch isn't worth the conditional. Pin
        // current behaviour so future "fixes" go through review.
        let m = render_success_message(1);
        assert!(m.contains("1 commits"));
    }

    #[test]
    fn success_message_is_one_line() {
        for n in [0, 1, 5, 100] {
            let m = render_success_message(n);
            assert!(!m.contains('\n'), "message for n={n} was multi-line: {m:?}");
        }
    }

    #[test]
    fn bootstrap_recovery_validation_at_constant_is_pinned() {
        // MLP2-037: retroactive witnesses written by
        // `anvil hook bootstrap --witness-recent` MUST carry this
        // exact tag so downstream readers can distinguish a recovery
        // walk from the regular pre-commit / post-rewrite-recovery
        // sources. Stable wire-format; do not drift.
        assert_eq!(BOOTSTRAP_RECOVERY_VALIDATION_AT, "bootstrap-recovery");
    }

    #[test]
    fn husky_h_does_not_contain_or_true_swallow() {
        // ADR-038 / clawpatch: `|| true` after the target invocation
        // would swallow block decisions and any failing user hook.
        let files = generate_husky_runtime();
        let h = files
            .iter()
            .find(|f| f.relative_path == ".husky/_/h")
            .unwrap();
        assert!(
            !h.contents.contains("|| true"),
            "generated husky runtime must not swallow hook failures with `|| true`"
        );
    }

    /// Behavioural regression: execute the generated `h` against a
    /// failing target and require the runtime exit code to match.
    #[cfg(unix)]
    #[test]
    fn husky_h_runtime_propagates_failing_hook_exit_status() {
        use std::os::unix::fs::PermissionsExt;
        use std::process::Command;

        let dir = tempfile::tempdir().expect("tempdir");
        let runtime_path = dir.path().join("h");
        let target_name = "failing-hook";
        let target_path = dir.path().join(target_name);

        let files = generate_husky_runtime();
        let runtime = files
            .iter()
            .find(|f| f.relative_path == ".husky/_/h")
            .expect("h runtime present");
        std::fs::write(&runtime_path, &runtime.contents).expect("write runtime");
        std::fs::set_permissions(&runtime_path, std::fs::Permissions::from_mode(0o755))
            .expect("chmod runtime");

        // Target under the same directory as the runtime, named via $1.
        std::fs::write(&target_path, "#!/usr/bin/env sh\nexit 1\n").expect("write target");
        std::fs::set_permissions(&target_path, std::fs::Permissions::from_mode(0o755))
            .expect("chmod target");

        let status = Command::new("sh").arg(&runtime_path)
            .arg(target_name)
            .status()
            .expect("run runtime");
        assert_eq!(
            status.code(),
            Some(1),
            "runtime must propagate the failing hook's exit status, got {status:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn husky_h_runtime_propagates_success_exit_status() {
        use std::os::unix::fs::PermissionsExt;
        use std::process::Command;

        let dir = tempfile::tempdir().expect("tempdir");
        let runtime_path = dir.path().join("h");
        let target_name = "ok-hook";
        let target_path = dir.path().join(target_name);

        let files = generate_husky_runtime();
        let runtime = files
            .iter()
            .find(|f| f.relative_path == ".husky/_/h")
            .expect("h runtime present");
        std::fs::write(&runtime_path, &runtime.contents).expect("write runtime");
        std::fs::set_permissions(&runtime_path, std::fs::Permissions::from_mode(0o755))
            .expect("chmod runtime");

        std::fs::write(&target_path, "#!/usr/bin/env sh\nexit 0\n").expect("write target");
        std::fs::set_permissions(&target_path, std::fs::Permissions::from_mode(0o755))
            .expect("chmod target");

        let status = Command::new("sh").arg(&runtime_path)
            .arg(target_name)
            .status()
            .expect("run runtime");
        assert_eq!(status.code(), Some(0));
    }

    #[cfg(unix)]
    #[test]
    fn husky_h_runtime_missing_target_is_noop() {
        use std::os::unix::fs::PermissionsExt;
        use std::process::Command;

        let dir = tempfile::tempdir().expect("tempdir");
        let runtime_path = dir.path().join("h");

        let files = generate_husky_runtime();
        let runtime = files
            .iter()
            .find(|f| f.relative_path == ".husky/_/h")
            .expect("h runtime present");
        std::fs::write(&runtime_path, &runtime.contents).expect("write runtime");
        std::fs::set_permissions(&runtime_path, std::fs::Permissions::from_mode(0o755))
            .expect("chmod runtime");

        let status = Command::new("sh").arg(&runtime_path)
            .arg("does-not-exist")
            .status()
            .expect("run runtime");
        assert_eq!(
            status.code(),
            Some(0),
            "absent target must remain a no-op (Husky contract)"
        );
    }

    /// The first positional arg selects the target; remaining args must
    /// reach the hook unchanged (git's commit-msg path, pre-push refs, …).
    #[cfg(unix)]
    #[test]
    fn husky_h_runtime_shifts_target_selector_before_invocation() {
        use std::os::unix::fs::PermissionsExt;
        use std::process::Command;

        let dir = tempfile::tempdir().expect("tempdir");
        let runtime_path = dir.path().join("h");
        let target_name = "echo-args";
        let target_path = dir.path().join(target_name);
        let out_path = dir.path().join("args.out");

        let files = generate_husky_runtime();
        let runtime = files
            .iter()
            .find(|f| f.relative_path == ".husky/_/h")
            .expect("h runtime present");
        std::fs::write(&runtime_path, &runtime.contents).expect("write runtime");
        std::fs::set_permissions(&runtime_path, std::fs::Permissions::from_mode(0o755))
            .expect("chmod runtime");

        // Write each positional arg on its own line for easy assertion.
        let script = format!(
            "#!/usr/bin/env sh\nprintf '%s\\n' \"$@\" > '{}'\n",
            out_path.display()
        );
        std::fs::write(&target_path, script).expect("write target");
        std::fs::set_permissions(&target_path, std::fs::Permissions::from_mode(0o755))
            .expect("chmod target");

        let status = Command::new("sh").arg(&runtime_path)
            .arg(target_name)
            .arg("COMMIT_MSG_PATH")
            .arg("extra")
            .status()
            .expect("run runtime");
        assert_eq!(status.code(), Some(0));

        let captured = std::fs::read_to_string(&out_path).expect("read args.out");
        assert_eq!(
            captured, "COMMIT_MSG_PATH\nextra\n",
            "target selector must not leak into hook args; got {captured:?}"
        );
    }
}
