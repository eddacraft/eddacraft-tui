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

/// The hook kinds bootstrap covers. Husky entrypoints are generated
/// for exactly these; keep in lockstep with the
/// [`BootstrapPlan::InstallPlain`] list in [`build_bootstrap_plan`].
const BOOTSTRAP_HOOK_KINDS: [HookKind; 5] = [
    HookKind::PreCommit,
    HookKind::PostCommit,
    HookKind::PrePush,
    HookKind::PostMerge,
    HookKind::PostRewrite,
];

/// Generate the minimal Husky v9 runtime under `.husky/_/`.
///
/// Husky v9's runtime layout is:
///
/// - `.husky/_/<hook>` — the entrypoint Git actually executes. A
///   Husky repo sets `core.hooksPath=.husky/_`, so Git runs
///   `.husky/_/pre-commit`, **not** `.husky/pre-commit`. Each
///   entrypoint forwards its own fixed hook name and the original
///   Git arguments to `h`.
/// - `.husky/_/h` — the shared runtime that runs the matching
///   `.husky/<hook>` file with the right env.
/// - `.husky/_/husky.sh` — kept as a no-op stub for backwards-
///   compat with v8 hook chains that may still source it.
///
/// Regenerating `h` alone is not enough. With `.husky/_/` wiped —
/// the exact state this bootstrap path exists to repair — Git finds
/// no `.husky/_/pre-commit` to execute, so the user's
/// `.husky/pre-commit` never runs and no hook fires. Silently: an
/// absent hook is not an error to Git. The entrypoints are what make
/// the recovered runtime actually fire.
///
/// We do NOT regenerate the hook scripts themselves (the user's
/// `.husky/pre-commit`, etc.) — those carry the Anvil hook line
/// already and shouldn't be touched. We only restore the runtime
/// shim that `pnpm install` would normally install via the
/// `husky` package's postinstall.
pub fn generate_husky_runtime() -> Vec<HuskyRuntime> {
    let mut files = vec![
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
    ];
    files.extend(BOOTSTRAP_HOOK_KINDS.into_iter().map(|k| HuskyRuntime {
        relative_path: format!(".husky/_/{}", k.filename()),
        contents: husky_entrypoint(k),
        executable: true,
    }));
    files
}

/// The `.husky/_/<hook>` entrypoint Git executes.
///
/// Passes the hook name explicitly rather than deriving it from
/// `$0`, so `h` keeps one documented calling convention
/// (`h <hook-name> [git args…]`). `exec` hands the process over so
/// the hook's exit status reaches Git unmodified.
fn husky_entrypoint(kind: HookKind) -> String {
    format!(
        "#!/usr/bin/env sh\nexec \"$(dirname -- \"$0\")/h\" {} \"$@\"\n",
        kind.filename()
    )
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
//
// The target resolves to `.husky/<hook>` — the PARENT of the `_`
// directory holding this script, not a sibling. `$0` here is
// `.husky/_/h`, so we take dirname twice. A sibling lookup would
// resolve to `.husky/_/<hook>`, which is the entrypoint that just
// exec'd us: the runtime would re-enter itself until the process
// died. It also matches Husky's real layout, where the user's hook
// scripts live in `.husky/` and only the runtime lives in `.husky/_/`.
const HUSKY_RUNTIME_H: &str = "#!/usr/bin/env sh\n[ \"$HUSKY\" = \"0\" ] && exit 0\nh=\"$(dirname -- \"$(dirname -- \"$0\")\")/$1\"\n[ -f \"$h\" ] && [ -x \"$h\" ] || exit 0\nshift\n\"$h\" \"$@\"\n";

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
    fn husky_runtime_includes_shared_runtime_files_and_entrypoints() {
        let files = generate_husky_runtime();
        // `h` + `husky.sh` + one entrypoint per hook kind.
        assert_eq!(files.len(), 2 + BOOTSTRAP_HOOK_KINDS.len());
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
                assert_eq!(files.len(), 2 + BOOTSTRAP_HOOK_KINDS.len());
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

    /// Materialise the generated runtime into a repo-shaped tempdir:
    /// `<root>/.husky/_/{h,husky.sh,<hook>…}`. Returns the root.
    ///
    /// The layout matters: `h` resolves its target relative to the
    /// PARENT of its own directory, so a flat tempdir would not
    /// exercise the real path resolution.
    #[cfg(unix)]
    fn materialise_runtime(root: &std::path::Path) {
        use std::os::unix::fs::PermissionsExt;
        for f in generate_husky_runtime() {
            let path = root.join(&f.relative_path);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            std::fs::write(&path, &f.contents).expect("write runtime file");
            if f.executable {
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                    .expect("chmod");
            }
        }
    }

    /// Write an executable user hook at `.husky/<name>`.
    #[cfg(unix)]
    fn write_user_hook(root: &std::path::Path, name: &str, body: &str) {
        use std::os::unix::fs::PermissionsExt;
        let path = root.join(".husky").join(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(&path, body).expect("write user hook");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    }

    /// Run `.husky/_/<hook>` the way Git would, from the repo root.
    #[cfg(unix)]
    fn run_entrypoint(
        root: &std::path::Path,
        hook: &str,
        args: &[&str],
    ) -> std::process::ExitStatus {
        std::process::Command::new("sh")
            .arg(format!(".husky/_/{hook}"))
            .args(args)
            .current_dir(root)
            .status()
            .expect("run entrypoint")
    }

    #[test]
    fn husky_runtime_generates_an_entrypoint_for_every_hook_kind() {
        // Git executes `.husky/_/<hook>` (core.hooksPath=.husky/_).
        // Without these, a wiped `.husky/_` leaves every hook dead.
        let files = generate_husky_runtime();
        for kind in BOOTSTRAP_HOOK_KINDS {
            let path = format!(".husky/_/{}", kind.filename());
            let entry = files
                .iter()
                .find(|f| f.relative_path == path)
                .unwrap_or_else(|| panic!("missing entrypoint {path}"));
            assert!(entry.executable, "{path} must be executable");
            assert!(
                entry.contents.contains(kind.filename()),
                "{path} must forward its own hook name",
            );
            assert!(entry.contents.starts_with("#!/usr/bin/env sh"));
        }
    }

    /// Behavioural regression: execute the generated `h` against a
    /// failing target and require the runtime exit code to match.
    #[cfg(unix)]
    #[test]
    fn husky_h_runtime_propagates_failing_hook_exit_status() {
        let dir = tempfile::tempdir().expect("tempdir");
        materialise_runtime(dir.path());
        write_user_hook(dir.path(), "pre-commit", "#!/usr/bin/env sh\nexit 1\n");

        let status = run_entrypoint(dir.path(), "pre-commit", &[]);
        assert_eq!(
            status.code(),
            Some(1),
            "runtime must propagate the failing hook's exit status, got {status:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn husky_h_runtime_propagates_success_exit_status() {
        let dir = tempfile::tempdir().expect("tempdir");
        materialise_runtime(dir.path());
        write_user_hook(dir.path(), "pre-commit", "#!/usr/bin/env sh\nexit 0\n");

        assert_eq!(
            run_entrypoint(dir.path(), "pre-commit", &[]).code(),
            Some(0)
        );
    }

    #[cfg(unix)]
    #[test]
    fn husky_h_runtime_missing_target_is_noop() {
        let dir = tempfile::tempdir().expect("tempdir");
        materialise_runtime(dir.path());
        // No `.husky/pre-commit` written at all.
        assert_eq!(
            run_entrypoint(dir.path(), "pre-commit", &[]).code(),
            Some(0),
            "absent target must remain a no-op (Husky contract)"
        );
    }

    /// The entrypoint supplies the hook name; git's own arguments must
    /// reach the user hook unchanged (commit-msg path, pre-push refs, …).
    #[cfg(unix)]
    #[test]
    fn husky_h_runtime_shifts_target_selector_before_invocation() {
        let dir = tempfile::tempdir().expect("tempdir");
        materialise_runtime(dir.path());
        let out_path = dir.path().join("args.out");
        write_user_hook(
            dir.path(),
            "pre-commit",
            &format!(
                "#!/usr/bin/env sh\nprintf '%s\\n' \"$@\" > '{}'\n",
                out_path.display()
            ),
        );

        let status = run_entrypoint(dir.path(), "pre-commit", &["COMMIT_MSG_PATH", "extra"]);
        assert_eq!(status.code(), Some(0));

        let captured = std::fs::read_to_string(&out_path).expect("read args.out");
        assert_eq!(
            captured, "COMMIT_MSG_PATH\nextra\n",
            "target selector must not leak into hook args; got {captured:?}"
        );
    }

    /// The runtime must not re-enter itself. `h` resolving its target
    /// as a sibling would find `.husky/_/<hook>` — the entrypoint that
    /// just exec'd it — and loop until the process died.
    #[cfg(unix)]
    #[test]
    fn husky_entrypoint_does_not_recurse_into_the_runtime() {
        let dir = tempfile::tempdir().expect("tempdir");
        materialise_runtime(dir.path());
        let out_path = dir.path().join("ran.out");
        write_user_hook(
            dir.path(),
            "pre-commit",
            &format!("#!/usr/bin/env sh\necho ran >> '{}'\n", out_path.display()),
        );

        assert_eq!(
            run_entrypoint(dir.path(), "pre-commit", &[]).code(),
            Some(0)
        );
        let captured = std::fs::read_to_string(&out_path).expect("read ran.out");
        assert_eq!(captured, "ran\n", "user hook must run exactly once");
    }

    /// End-to-end: a real Git repo whose `core.hooksPath` is
    /// `.husky/_` with that directory wiped — the exact state
    /// bootstrap repairs. After materialising the plan, `git commit`
    /// must execute the user's `.husky/pre-commit`.
    #[cfg(unix)]
    #[test]
    fn git_commit_runs_the_user_hook_after_bootstrap() {
        use std::process::Command;

        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let git = |args: &[&str]| {
            let out = Command::new("git")
                .args(args)
                .current_dir(root)
                .env("HOME", root)
                .env("GIT_CONFIG_GLOBAL", root.join("gitconfig"))
                .env("GIT_CONFIG_SYSTEM", "/dev/null")
                .output()
                .expect("run git");
            (
                out.status,
                String::from_utf8_lossy(&out.stderr).into_owned(),
            )
        };

        assert!(git(&["init", "--quiet"]).0.success());
        assert!(git(&["config", "user.email", "t@example.com"]).0.success());
        assert!(git(&["config", "user.name", "Test"]).0.success());
        // The Husky contract: git looks for hooks in `.husky/_`.
        assert!(git(&["config", "core.hooksPath", ".husky/_"]).0.success());

        // The user's hook survives; only `.husky/_` was lost.
        let sentinel = root.join("hook-ran");
        write_user_hook(
            root,
            "pre-commit",
            &format!("#!/usr/bin/env sh\necho fired > '{}'\n", sentinel.display()),
        );
        assert!(
            !root.join(".husky/_").exists(),
            "precondition: the runtime directory is missing",
        );

        // Bootstrap repairs it.
        materialise_runtime(root);

        std::fs::write(root.join("file.txt"), "x\n").expect("write file");
        assert!(git(&["add", "file.txt"]).0.success());
        let (status, stderr) = git(&["commit", "--quiet", "-m", "test"]);
        assert!(status.success(), "commit failed: {stderr}");

        assert!(
            sentinel.exists(),
            "git commit did not execute .husky/pre-commit after bootstrap",
        );
    }
}
