/// Hook kind — the closed set Anvil installs per ADR-038 §D-3.
///
/// v1 set: `pre-commit`, `post-commit`, `pre-push`, `post-merge`,
/// `post-rewrite`. The `prepare-commit-msg` / `commit-msg` v1.5
/// hooks are intentionally excluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookKind {
    PreCommit,
    PostCommit,
    PrePush,
    PostMerge,
    PostRewrite,
}

impl HookKind {
    /// Every git hook installed by anvil.
    ///
    /// Production installation and host-completeness checks consume this
    /// registry so adding a hook cannot leave either surface out of sync.
    pub const ALL: [Self; 5] = [
        Self::PreCommit,
        Self::PostCommit,
        Self::PrePush,
        Self::PostMerge,
        Self::PostRewrite,
    ];

    /// The on-disk filename, e.g. `pre-commit`. Matches git's hook
    /// naming convention exactly.
    pub fn filename(self) -> &'static str {
        match self {
            HookKind::PreCommit => "pre-commit",
            HookKind::PostCommit => "post-commit",
            HookKind::PrePush => "pre-push",
            HookKind::PostMerge => "post-merge",
            HookKind::PostRewrite => "post-rewrite",
        }
    }

    /// The `anvil hook <subcommand>` keyword the wrapper invokes.
    /// Identical to `filename()` today but kept as a separate method
    /// in case the subcommand naming ever diverges from git's.
    pub fn subcommand(self) -> &'static str {
        self.filename()
    }
}

/// Build the 3-line shell template from ADR-038 §D-5.
///
/// Output verbatim:
///
/// ```sh
/// #!/bin/sh
/// command -v anvil >/dev/null 2>&1 || exit 0
/// exec anvil hook <subcommand> "$@"
/// ```
///
/// Notes pinned by the ADR:
///
/// - `#!/bin/sh` (not bash) so the hook works on minimal containers.
/// - The `command -v` guard makes the hook a no-op when the anvil
///   binary isn't installed (zero noise on uninstall).
/// - `exec` so the parent git process directly sees anvil's exit
///   code (no shell-wrapper interposed).
/// - No `|| true` — the binary's panic catcher converts internal
///   crashes to exit-0; adding `|| true` here would also swallow
///   legitimate block decisions.
pub fn shell_template(kind: HookKind) -> String {
    format!(
        "#!/bin/sh\ncommand -v anvil >/dev/null 2>&1 || exit 0\nexec anvil hook {} \"$@\"\n",
        kind.subcommand()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_is_exactly_three_lines_plus_shebang() {
        // ADR-038 §D-5 explicitly says "3 lines": shebang + guard +
        // exec. Anything else is drift.
        let t = shell_template(HookKind::PreCommit);
        let lines: Vec<&str> = t.lines().collect();
        assert_eq!(lines.len(), 3, "got {} lines: {:?}", lines.len(), lines);
    }

    #[test]
    fn template_starts_with_posix_shebang() {
        let t = shell_template(HookKind::PreCommit);
        assert!(t.starts_with("#!/bin/sh\n"), "got prefix: {:?}", &t[..16]);
    }

    #[test]
    fn template_uses_command_v_guard_not_which() {
        // `which` isn't POSIX; `command -v` is. Don't switch.
        let t = shell_template(HookKind::PreCommit);
        assert!(t.contains("command -v anvil"));
        assert!(!t.contains("which "));
    }

    #[test]
    fn template_redirects_command_v_to_devnull() {
        let t = shell_template(HookKind::PreCommit);
        assert!(t.contains("command -v anvil >/dev/null 2>&1"));
    }

    #[test]
    fn template_uses_exec_for_pid_passthrough() {
        // `exec` so anvil's exit code IS the hook's exit code, no
        // intermediate shell process.
        let t = shell_template(HookKind::PreCommit);
        assert!(t.contains("exec anvil hook"));
    }

    #[test]
    fn template_passes_args_through() {
        // `"$@"` (quoted!) is required so git's positional arguments
        // for hooks like pre-push and post-rewrite reach anvil.
        let t = shell_template(HookKind::PrePush);
        assert!(t.contains("\"$@\""));
    }

    #[test]
    fn template_does_not_contain_or_true_swallow() {
        // ADR-038: no `|| true`. The binary owns exit-code policy.
        for kind in [
            HookKind::PreCommit,
            HookKind::PostCommit,
            HookKind::PrePush,
            HookKind::PostMerge,
            HookKind::PostRewrite,
        ] {
            let t = shell_template(kind);
            assert!(!t.contains("|| true"), "kind {kind:?} contains `|| true`");
        }
    }

    #[test]
    fn template_ends_with_newline() {
        let t = shell_template(HookKind::PreCommit);
        assert!(t.ends_with('\n'));
    }

    #[test]
    fn each_kind_uses_correct_subcommand_keyword() {
        assert!(shell_template(HookKind::PreCommit).contains("hook pre-commit"));
        assert!(shell_template(HookKind::PostCommit).contains("hook post-commit"));
        assert!(shell_template(HookKind::PrePush).contains("hook pre-push"));
        assert!(shell_template(HookKind::PostMerge).contains("hook post-merge"));
        assert!(shell_template(HookKind::PostRewrite).contains("hook post-rewrite"));
    }

    #[test]
    fn filename_matches_git_naming() {
        // Don't rename these — git looks them up by exact name in
        // .git/hooks/.
        assert_eq!(HookKind::PreCommit.filename(), "pre-commit");
        assert_eq!(HookKind::PostCommit.filename(), "post-commit");
        assert_eq!(HookKind::PrePush.filename(), "pre-push");
        assert_eq!(HookKind::PostMerge.filename(), "post-merge");
        assert_eq!(HookKind::PostRewrite.filename(), "post-rewrite");
    }

    #[test]
    fn templates_for_all_kinds_differ_only_in_subcommand_keyword() {
        // Sanity: anything else differing would be a bug in the
        // template builder.
        let pre = shell_template(HookKind::PreCommit);
        let post = shell_template(HookKind::PostCommit);
        // Replace "post-commit" with "pre-commit" in post → should
        // equal pre.
        assert_eq!(post.replace("post-commit", "pre-commit"), pre);
    }
}
