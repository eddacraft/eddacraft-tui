//! Decide whether a workspace root may attach to this daemon instance.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};

use crate::dos::DEFAULT_MAX_ADMITTED_ROOTS;
use crate::workspace_anchor::WorkspaceAnchor;

/// CIB-154: the outcome of [`AdmittedRoots::authorise_within_budget`], which
/// canonicalises the incoming root **exactly once** and then makes the
/// budget-check-then-admit decision on that single resolved path — closing the
/// TOCTOU window a split "would-block?" / "authorise" pair would otherwise leave
/// (a same-uid writer swapping a symlink component between the two resolutions).
#[derive(Debug)]
pub enum AdmitOutcome<'a> {
    /// The root is authorised on this connection; carries the held read anchor.
    Authorised(&'a WorkspaceAnchor),
    /// The root is admissible but would push the connection past its
    /// [`root_budget`](AdmittedRoots::root_budget) — refuse with a structured
    /// budget error, distinct from a plain not-admitted refusal.
    OverBudget,
    /// The root is refused: unresolvable, or (in `Allowlist` mode) unlisted.
    Refused,
}

/// How the admitted-root set decides whether to admit a not-yet-seen root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionMode {
    /// First-touch adopt: any nameable root is auto-admitted on first contact.
    Open,
    /// Confinement: only operator-pre-admitted roots are authorised.
    Allowlist,
}

/// The set of roots permitted in `Allowlist` mode: a set of *exact* canonical
/// roots plus a list of *prefix* canonical roots (a root at or beneath a prefix
/// is permitted, so a single `prefix` entry confines a whole subtree).
///
/// Built by [`crate::confinement`] from operator config (DSV-008, Task 14) and
/// applied here, so the daemon's admission decision understands both the exact
/// and the subtree (`prefix`) forms of an operator allow entry. Empty in `Open`
/// mode (the set grows on first contact instead).
#[derive(Debug, Default, Clone)]
pub struct AllowPolicy {
    /// Canonical roots matched exactly.
    exact: BTreeSet<PathBuf>,
    /// Canonical roots whose entire subtree is permitted.
    prefixes: Vec<PathBuf>,
}

impl AllowPolicy {
    /// Build a policy from already-canonicalised exact + prefix roots. The
    /// caller is responsible for canonicalisation so a `prefix` match compares
    /// like-for-like against a canonical incoming root.
    #[must_use]
    pub fn new(
        exact: impl IntoIterator<Item = PathBuf>,
        prefixes: impl IntoIterator<Item = PathBuf>,
    ) -> Self {
        Self {
            exact: exact.into_iter().collect(),
            prefixes: prefixes.into_iter().collect(),
        }
    }

    /// Whether `canonical_root` is permitted: an exact match, or at/beneath any
    /// prefix root. `starts_with` is component-wise, so `/a/b` permits `/a/b/c`
    /// but never the sibling `/a/b-other`.
    #[must_use]
    pub fn permits(&self, canonical_root: &Path) -> bool {
        self.exact.contains(canonical_root)
            || self
                .prefixes
                .iter()
                .any(|prefix| canonical_root.starts_with(prefix))
    }
}

/// A per-connection set of admitted workspace roots, each paired with its held
/// [`WorkspaceAnchor`] (read anchor + identity — a Unix dirfd or a Windows
/// directory handle).
#[derive(Debug)]
pub struct AdmittedRoots {
    mode: AdmissionMode,
    /// Roots permitted in `Allowlist` mode. Empty in `Open` mode.
    allow: AllowPolicy,
    /// Canonical root → held anchor. Insertion-once; never re-resolved.
    admitted: BTreeMap<PathBuf, WorkspaceAnchor>,
    /// CIB-154: the per-connection ceiling on distinct admitted roots. Once
    /// `admitted.len()` reaches this budget, a not-yet-admitted root that would
    /// otherwise be admissible is refused — see [`Self::root_budget_would_block`].
    /// Defaults to [`DEFAULT_MAX_ADMITTED_ROOTS`]; the daemon threads the
    /// operator-resolved `IpcLimits::max_admitted_roots` through
    /// [`Self::with_root_budget`].
    root_budget: usize,
}

impl AdmittedRoots {
    /// Open mode: the set grows on first contact.
    #[must_use]
    pub fn new_open() -> Self {
        Self {
            mode: AdmissionMode::Open,
            allow: AllowPolicy::default(),
            admitted: BTreeMap::new(),
            root_budget: DEFAULT_MAX_ADMITTED_ROOTS,
        }
    }

    /// Allowlist mode over exact roots only. Each entry is canonicalised at
    /// construction; entries that do not currently resolve are dropped (they
    /// cannot match a real, openable root anyway). For prefix (subtree) entries
    /// or operator-config-driven policies use [`Self::new_allowlist_with_policy`].
    #[must_use]
    pub fn new_allowlist<I>(allowed: I) -> Self
    where
        I: IntoIterator<Item = PathBuf>,
    {
        let exact = allowed
            .into_iter()
            .filter_map(|p| std::fs::canonicalize(p).ok());
        Self::new_allowlist_with_policy(AllowPolicy::new(exact, std::iter::empty()))
    }

    /// Allowlist mode driven by an explicit [`AllowPolicy`] (exact + prefix).
    /// This is the seam [`crate::confinement`] uses to apply operator
    /// confinement config (DSV-008): the policy already carries the
    /// canonicalised allow roots plus the implicitly-admitted primary root.
    #[must_use]
    pub fn new_allowlist_with_policy(allow: AllowPolicy) -> Self {
        Self {
            mode: AdmissionMode::Allowlist,
            allow,
            admitted: BTreeMap::new(),
            root_budget: DEFAULT_MAX_ADMITTED_ROOTS,
        }
    }

    /// CIB-154: override the per-connection admitted-root budget (builder form).
    /// The daemon threads the operator-resolved `IpcLimits::max_admitted_roots`
    /// through here from [`crate::confinement::Confinement::to_admitted_roots_with_budget`].
    /// A `0` budget is clamped to `1` — a connection must be able to admit at
    /// least its own workspace root or no verb could ever be served (mirrors the
    /// `IpcLimits::from_config` defensive clamp).
    #[must_use]
    pub fn with_root_budget(mut self, root_budget: usize) -> Self {
        self.root_budget = root_budget.max(1);
        self
    }

    /// This connection's admission mode.
    #[must_use]
    pub fn mode(&self) -> AdmissionMode {
        self.mode
    }

    /// CIB-154: this connection's distinct-admitted-root budget.
    #[must_use]
    pub fn root_budget(&self) -> usize {
        self.root_budget
    }

    /// CIB-154: whether admitting `workspace_root` would push this connection
    /// past its [`root_budget`](Self::root_budget). Returns `true` **only** for a
    /// root that (a) is not already admitted, (b) is otherwise admissible under
    /// this connection's mode (first-touch in `Open`, allow-policy match in
    /// `Allowlist`), and (c) would be the `root_budget + 1`-th distinct root.
    ///
    /// Ordering matters: an *unlisted* root in `Allowlist` mode is an ordinary
    /// allowlist refusal, not a budget refusal, so this returns `false` for it
    /// (letting [`Self::authorise`] report the plain refusal). A caller checks
    /// this **before** [`Self::authorise`] and, on `true`, produces a structured
    /// budget error distinct from the plain `workspace-not-admitted` refusal —
    /// so a peer probing the descriptor-exhaustion vector gets an unambiguous
    /// signal rather than a silent/ambiguous refusal.
    ///
    /// A root that does not resolve is never a budget refusal (it cannot be
    /// admitted at all); this returns `false` so `authorise` reports the plain
    /// refusal.
    #[must_use]
    pub fn root_budget_would_block(&self, workspace_root: &Path) -> bool {
        let Ok(canonical) = std::fs::canonicalize(workspace_root) else {
            return false;
        };
        self.budget_would_block_canonical(&canonical)
    }

    /// CIB-154: the budget decision on an **already-canonicalised** root — the
    /// shared core of [`Self::root_budget_would_block`] and
    /// [`Self::authorise_within_budget`], so both operate on the same resolved
    /// path without re-running `canonicalize`.
    fn budget_would_block_canonical(&self, canonical: &Path) -> bool {
        if self.admitted.contains_key(canonical) {
            return false;
        }
        let admissible = match self.mode {
            AdmissionMode::Open => true,
            AdmissionMode::Allowlist => self.allow.permits(canonical),
        };
        admissible && self.admitted.len() >= self.root_budget
    }

    /// Whether `canonical_root` is currently admitted (already has a held fd).
    #[must_use]
    pub fn is_admitted(&self, canonical_root: &Path) -> bool {
        self.admitted.contains_key(canonical_root)
    }

    /// Explicitly admit a root: open its [`WorkspaceAnchor`] once and store it
    /// under its canonical path. Idempotent — a second call for an
    /// already-admitted root keeps the original anchor (identity is pinned at
    /// first admission).
    ///
    /// # Errors
    /// Propagates canonicalisation / open failures (root missing or not a
    /// directory).
    pub fn admit(&mut self, root: &Path) -> io::Result<()> {
        let canonical = std::fs::canonicalize(root)?;
        if self.admitted.contains_key(&canonical) {
            return Ok(());
        }
        let anchor = WorkspaceAnchor::open(&canonical)?;
        self.admitted.insert(canonical, anchor);
        Ok(())
    }

    /// Authorise a verb against `workspace_root`, returning the held read
    /// [`WorkspaceAnchor`] iff the root is authorised on this connection.
    ///
    /// - `Open` mode: a not-yet-admitted root is auto-admitted (first-touch)
    ///   and its anchor returned.
    /// - `Allowlist` mode: an already-admitted root returns its anchor; an
    ///   unadmitted-but-allowed root is admitted then returned; an unlisted
    ///   root returns `Ok(None)` (refused).
    ///
    /// # Errors
    /// Propagates the open/canonicalise error when admission is attempted for
    /// a root that should be admissible but cannot be opened.
    pub fn authorise(&mut self, workspace_root: &Path) -> io::Result<Option<&WorkspaceAnchor>> {
        // A root that does not resolve is never authorised — but this is a
        // refusal, not a hard error (the client named a vanished path).
        let Ok(canonical) = std::fs::canonicalize(workspace_root) else {
            return Ok(None);
        };
        self.authorise_canonical(&canonical)
    }

    /// Admit/authorise an **already-canonicalised** root — the shared core of
    /// [`Self::authorise`] and [`Self::authorise_within_budget`], so neither
    /// re-runs `canonicalize` on a path a caller already resolved.
    fn authorise_canonical(&mut self, canonical: &Path) -> io::Result<Option<&WorkspaceAnchor>> {
        if !self.admitted.contains_key(canonical) {
            let admissible = match self.mode {
                AdmissionMode::Open => true,
                AdmissionMode::Allowlist => self.allow.permits(canonical),
            };
            if !admissible {
                return Ok(None);
            }
            let anchor = WorkspaceAnchor::open(canonical)?;
            self.admitted.insert(canonical.to_path_buf(), anchor);
        }

        Ok(self.admitted.get(canonical))
    }

    /// CIB-154: authorise `workspace_root` against this connection's admission
    /// mode **and** its distinct-root budget in a single step, canonicalising
    /// the incoming path **exactly once**.
    ///
    /// This is the seam the daemon's per-verb admission gate uses. Folding the
    /// former separate `root_budget_would_block` (check) + [`Self::authorise`]
    /// (act) pair into one canonicalise closes a TOCTOU: with two independent
    /// `canonicalize` calls, a same-uid writer could swap a symlink component
    /// between them so the budget check resolves to an already-admitted
    /// (non-blocking) path while the admit step resolves to a genuinely new,
    /// distinct root — admitting it without ever passing the budget check and
    /// defeating CIB-154's own resource-exhaustion defence. Here the budget
    /// check and the admit decision operate on the same resolved `canonical`
    /// with no re-resolution in between.
    ///
    /// # Errors
    /// Propagates the open error when admission is attempted for a root that is
    /// admissible and within budget but cannot be opened.
    pub fn authorise_within_budget(
        &mut self,
        workspace_root: &Path,
    ) -> io::Result<AdmitOutcome<'_>> {
        // Canonicalise ONCE; every subsequent decision uses `canonical`.
        let Ok(canonical) = std::fs::canonicalize(workspace_root) else {
            // A vanished/unresolvable root is a plain refusal, not an error.
            return Ok(AdmitOutcome::Refused);
        };
        self.authorise_canonical_within_budget(&canonical)
    }

    /// [`Self::authorise_within_budget`] on an **already-canonicalised** root.
    /// The CIB-414 graph-root gate resolves the incoming root once to make its
    /// structural decision, and hands that same resolved path straight here —
    /// so the gate and the admit step cannot be split by a same-uid writer
    /// swapping a path component between two `canonicalize` calls (the CIB-154
    /// TOCTOU shape).
    ///
    /// # Errors
    /// Propagates the open error when admission is attempted for a root that is
    /// admissible and within budget but cannot be opened.
    pub fn authorise_canonical_within_budget(
        &mut self,
        canonical: &Path,
    ) -> io::Result<AdmitOutcome<'_>> {
        // Budget guard first, so an admissible-but-over-budget root reports the
        // structured budget refusal rather than being admitted (or reported as
        // a plain not-admitted refusal). An unlisted or already-admitted root is
        // never a budget block, so it falls through to `authorise_canonical`.
        if self.budget_would_block_canonical(canonical) {
            return Ok(AdmitOutcome::OverBudget);
        }
        match self.authorise_canonical(canonical)? {
            Some(anchor) => Ok(AdmitOutcome::Authorised(anchor)),
            None => Ok(AdmitOutcome::Refused),
        }
    }

    /// CIB-414: whether `canonical_root` may key a **graph** on this connection
    /// — the daemon-side form of the MCP rule `workspace_root_is_graph_root`
    /// (CIB-398). Two independent refusals, both of which must pass:
    ///
    /// 1. [`is_graph_root`] — a repository-structure rule that holds on FIRST
    ///    contact, before anything is admitted. `Open` mode first-touch-adopts
    ///    whatever root a client names, so a rule that only compared against the
    ///    already-admitted set would still let a fresh connection make
    ///    `<repo>/secrets` its own graph root.
    /// 2. not a directory nested inside an already-admitted root of this
    ///    connection, unless it is a registered worktree root of that
    ///    repository. This covers the roots rule 1 cannot judge — an admitted
    ///    root that is not a Git checkout at all has no structural "repo root"
    ///    to compare against.
    ///
    /// This predicate only ever *refuses*: it never admits a root the
    /// connection's mode would not admit, so `Allowlist` confinement (ADR-097)
    /// stays explicit-entries-only.
    #[must_use]
    pub fn permits_graph_root(&self, canonical_root: &Path) -> bool {
        is_graph_root(canonical_root) && !self.nested_inside_admitted(canonical_root)
    }

    /// Whether `canonical_root` is strictly inside an already-admitted root
    /// without being a registered worktree root of that repository.
    fn nested_inside_admitted(&self, canonical_root: &Path) -> bool {
        self.admitted.keys().any(|admitted| {
            canonical_root != admitted
                && canonical_root.starts_with(admitted)
                && !registered_worktree_roots(admitted)
                    .iter()
                    .any(|worktree| worktree == canonical_root)
        })
    }
}

/// CIB-414: whether a canonical path is a **graph root** by repository
/// structure — the root of the Git checkout that contains it (its main
/// checkout or its own registered linked worktree), never a directory inside
/// one. A path that is in no Git checkout at all has no enclosing repository
/// to be nested inside, so it is its own root; the admitted-set half of
/// [`AdmittedRoots::permits_graph_root`] is what guards those.
///
/// Why this matters (CIB-398): the daemon keys its graph on the root it is
/// handed and every file identity it projects is root-relative, so
/// `<repo>/secrets` as the root rebases `secrets/token.ts` to `token.ts` and
/// the CE-3 sensitive-path deny-list never sees the denied segment.
///
/// Reads the on-disk gitdir layout; never shells out to `git`.
#[must_use]
pub fn is_graph_root(canonical_root: &Path) -> bool {
    match containing_git_worktree_root(canonical_root) {
        Some(checkout) => checkout == canonical_root,
        None => true,
    }
}

/// The nearest enclosing Git worktree root at or above `path` (a directory
/// whose `.git` is a directory or a `gitdir:` file). `path` is assumed
/// canonical, so no component is re-resolved here.
fn containing_git_worktree_root(path: &Path) -> Option<PathBuf> {
    let mut current = path.to_path_buf();
    loop {
        if resolve_git_dir(&current).is_some() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Resolve `<root>/.git` as a directory or as a linked-worktree `gitdir:` file.
/// Portable: the `graph_base_trigger` copy is `cfg(unix)`, while the
/// save-time admission path is served on Unix and Windows alike.
fn resolve_git_dir(repo_root: &Path) -> Option<PathBuf> {
    let dot_git = repo_root.join(".git");
    let meta = std::fs::symlink_metadata(&dot_git).ok()?;
    if meta.is_dir() {
        return Some(dot_git);
    }
    let contents = std::fs::read_to_string(&dot_git).ok()?;
    let line = contents.lines().find_map(|l| {
        l.trim()
            .strip_prefix("gitdir:")
            .map(str::trim)
            .filter(|p| !p.is_empty())
    })?;
    let git_dir = Path::new(line);
    Some(if git_dir.is_absolute() {
        git_dir.to_path_buf()
    } else {
        lexical_join(repo_root, git_dir)
    })
}

/// Resolve the **common** gitdir of a (possibly per-worktree) gitdir: a linked
/// worktree's gitdir carries a `commondir` pointer, a main worktree has none.
fn resolve_common_dir(git_dir: &Path) -> PathBuf {
    match std::fs::read_to_string(git_dir.join("commondir")) {
        Ok(raw) => {
            let rel = Path::new(raw.trim());
            if rel.is_absolute() {
                rel.to_path_buf()
            } else {
                lexical_join(git_dir, rel)
            }
        }
        Err(_) => git_dir.to_path_buf(),
    }
}

/// Lexically join `rel` onto `base`, resolving `..`/`.` without touching the
/// filesystem (the pointed-at dirs need not all exist).
fn lexical_join(base: &Path, rel: &Path) -> PathBuf {
    let mut out = base.to_path_buf();
    for comp in rel.components() {
        match comp {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Registered Git worktree roots of the repository that owns `repo_root`,
/// including the main worktree. Empty when `repo_root` is not a Git checkout.
/// Does not spawn `git`; reads the on-disk gitdir layout (the same shape the
/// MCP-side CIB-398 rule reads).
fn registered_worktree_roots(repo_root: &Path) -> Vec<PathBuf> {
    let Some(git_dir) = resolve_git_dir(repo_root) else {
        return Vec::new();
    };
    let common = resolve_common_dir(&git_dir);
    let Ok(common) = std::fs::canonicalize(&common) else {
        return Vec::new();
    };

    let mut roots = Vec::new();
    if common.file_name().is_some_and(|name| name == ".git")
        && let Some(parent) = common.parent()
        && let Ok(main) = std::fs::canonicalize(parent)
    {
        roots.push(main);
    }

    let Ok(entries) = std::fs::read_dir(common.join("worktrees")) else {
        return roots;
    };
    for entry in entries.flatten() {
        let admin = entry.path();
        if !admin.is_dir() {
            continue;
        }
        let Ok(raw) = std::fs::read_to_string(admin.join("gitdir")) else {
            continue;
        };
        let pointed = Path::new(raw.trim());
        let pointed = if pointed.is_absolute() {
            pointed.to_path_buf()
        } else {
            lexical_join(&admin, pointed)
        };
        let Some(root) = pointed.parent() else {
            continue;
        };
        if let Ok(canonical) = std::fs::canonicalize(root)
            && !roots.iter().any(|existing| existing == &canonical)
        {
            roots.push(canonical);
        }
    }
    roots
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CIB-414: an admitted root that is not a Git checkout has no structural
    /// "repo root" for [`is_graph_root`] to compare against — the admitted-set
    /// half of [`AdmittedRoots::permits_graph_root`] is what refuses a directory
    /// nested inside it.
    #[test]
    fn nested_dir_inside_a_non_git_admitted_root_is_not_a_graph_root() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = std::fs::canonicalize(tmp.path()).expect("root canonicalises");
        let nested = root.join("secrets");
        std::fs::create_dir_all(&nested).expect("nested dir");

        let mut roots = AdmittedRoots::new_open();
        roots.admit(&root).expect("admit root");

        assert!(
            is_graph_root(&nested),
            "no enclosing checkout ⇒ the structural rule cannot judge {}",
            nested.to_str().expect("utf-8 nested path"),
        );
        assert!(
            roots.permits_graph_root(&root),
            "the admitted root itself stays a graph root: {}",
            root.to_str().expect("utf-8 root path"),
        );
        assert!(
            !roots.permits_graph_root(&nested),
            "a directory inside an admitted root is not a graph root: {}",
            nested.to_str().expect("utf-8 nested path"),
        );
    }

    #[test]
    fn validate_paths_authorised_for_session_root() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::write(tmp.path().join("marker"), b"present").expect("write marker");
        let mut roots = AdmittedRoots::new_open();
        roots.admit(tmp.path()).expect("admit root");

        let anchor = roots
            .authorise(tmp.path())
            .expect("authorise io")
            .expect("an admitted root is authorised");
        // A live, readable anchor: it reads a file beneath the held root.
        assert_eq!(
            anchor.read_rel("marker").expect("read via anchor"),
            b"present"
        );
    }

    #[test]
    fn validate_paths_refused_for_unrelated_root_in_allowlist_mode() {
        let allowed = tempfile::tempdir().expect("tempdir");
        let other = tempfile::tempdir().expect("tempdir");

        let mut roots = AdmittedRoots::new_allowlist([allowed.path().to_path_buf()]);
        // The allowed root authorises...
        assert!(
            roots.authorise(allowed.path()).expect("io").is_some(),
            "allowlisted root must authorise"
        );
        // ...an unrelated root does not, and is NOT silently admitted.
        assert!(
            roots.authorise(other.path()).expect("io").is_none(),
            "an unlisted root must be refused in allowlist mode"
        );
        let canonical_other = std::fs::canonicalize(other.path()).unwrap();
        assert!(!roots.is_admitted(&canonical_other));
    }

    #[test]
    fn root_set_grows_on_first_touch_in_open_mode() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut roots = AdmittedRoots::new_open();
        let canonical = std::fs::canonicalize(tmp.path()).unwrap();

        assert!(
            !roots.is_admitted(&canonical),
            "not admitted before first touch"
        );
        // First authorise auto-admits in open mode.
        assert!(roots.authorise(tmp.path()).expect("io").is_some());
        assert!(roots.is_admitted(&canonical), "admitted after first touch");
    }

    #[test]
    fn admission_pins_the_anchor_identity_across_calls() {
        // The anchor is opened once and reused — re-authorising the same root
        // returns the same held anchor, not a freshly re-resolved one (C2).
        // Identity is checked by address: the second authorise must hand back
        // the exact same stored `WorkspaceAnchor` (no second open).
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut roots = AdmittedRoots::new_open();

        let first = std::ptr::from_ref(roots.authorise(tmp.path()).expect("io").unwrap()) as usize;
        let second = std::ptr::from_ref(roots.authorise(tmp.path()).expect("io").unwrap()) as usize;
        assert_eq!(
            first, second,
            "the held anchor is pinned at first admission"
        );
    }

    #[test]
    fn allow_policy_prefix_permits_subtree_not_sibling() {
        // A single `prefix` entry confines a whole subtree, but the
        // component-wise `starts_with` must not leak to a sibling whose name
        // merely shares a textual prefix.
        let policy = AllowPolicy::new(
            [PathBuf::from("/srv/exact")],
            [PathBuf::from("/home/op/projects")],
        );
        assert!(
            policy.permits(Path::new("/srv/exact")),
            "exact root permitted"
        );
        assert!(
            policy.permits(Path::new("/home/op/projects")),
            "the prefix root itself is permitted"
        );
        assert!(
            policy.permits(Path::new("/home/op/projects/foo/bar")),
            "a root beneath the prefix is permitted"
        );
        assert!(
            !policy.permits(Path::new("/home/op/projects-other")),
            "a textual-prefix sibling is NOT permitted (component-wise match)"
        );
        assert!(
            !policy.permits(Path::new("/srv/other")),
            "an unlisted root is refused"
        );
    }

    #[test]
    fn unresolvable_root_is_refused_not_errored() {
        let mut roots = AdmittedRoots::new_open();
        let result = roots.authorise(Path::new("/no/such/anvil/root"));
        assert!(
            matches!(result, Ok(None)),
            "a vanished root is a refusal, not a hard error: {result:?}"
        );
    }

    #[test]
    fn open_mode_refuses_root_past_budget() {
        // CIB-154: with a budget of 2, the first two distinct roots admit
        // normally; the third distinct root trips the budget guard while roots
        // already admitted keep working.
        let a = tempfile::tempdir().expect("tempdir");
        let b = tempfile::tempdir().expect("tempdir");
        let c = tempfile::tempdir().expect("tempdir");

        let mut roots = AdmittedRoots::new_open().with_root_budget(2);
        assert_eq!(roots.root_budget(), 2);

        // First two distinct roots: within budget, admitted normally, and the
        // budget guard does not fire for them.
        assert!(!roots.root_budget_would_block(a.path()));
        assert!(roots.authorise(a.path()).expect("io").is_some());
        assert!(!roots.root_budget_would_block(b.path()));
        assert!(roots.authorise(b.path()).expect("io").is_some());

        // The third distinct root is over budget: the guard fires BEFORE
        // authorise, so the caller can raise a structured budget error.
        assert!(
            roots.root_budget_would_block(c.path()),
            "the (budget+1)th distinct root must trip the budget guard"
        );

        // An already-admitted root is never blocked and keeps authorising —
        // the budget caps distinct roots, not repeat access.
        assert!(!roots.root_budget_would_block(a.path()));
        assert!(roots.authorise(a.path()).expect("io").is_some());
    }

    #[test]
    fn allowlist_mode_refuses_root_past_budget_but_not_unlisted() {
        // CIB-154: in Allowlist mode the budget caps admissible roots. An
        // over-budget but ALLOW-LISTED root trips the budget guard; an UNLISTED
        // root is an ordinary allowlist refusal, never a budget refusal (so the
        // caller reports the right, distinct error for each).
        let a = tempfile::tempdir().expect("tempdir");
        let b = tempfile::tempdir().expect("tempdir");
        let unlisted = tempfile::tempdir().expect("tempdir");

        let allow = AllowPolicy::new(
            [
                std::fs::canonicalize(a.path()).unwrap(),
                std::fs::canonicalize(b.path()).unwrap(),
            ],
            std::iter::empty(),
        );
        let mut roots = AdmittedRoots::new_allowlist_with_policy(allow).with_root_budget(1);

        // First allow-listed root fills the budget.
        assert!(!roots.root_budget_would_block(a.path()));
        assert!(roots.authorise(a.path()).expect("io").is_some());

        // Second allow-listed root is admissible but over budget → budget guard.
        assert!(
            roots.root_budget_would_block(b.path()),
            "an admissible-but-over-budget root trips the budget guard"
        );

        // An UNLISTED root is a plain allowlist refusal, NOT a budget refusal —
        // the guard must stay silent so the caller reports workspace-not-admitted.
        assert!(
            !roots.root_budget_would_block(unlisted.path()),
            "an unlisted root is an allowlist refusal, not a budget refusal"
        );
        assert!(roots.authorise(unlisted.path()).expect("io").is_none());
    }

    #[test]
    fn authorise_within_budget_matches_split_semantics_open_mode() {
        // CIB-154: the single-canonicalise gate must produce exactly the same
        // Authorised / OverBudget decisions as the former split
        // `root_budget_would_block` + `authorise` pair.
        let a = tempfile::tempdir().expect("tempdir");
        let b = tempfile::tempdir().expect("tempdir");
        let c = tempfile::tempdir().expect("tempdir");

        let mut roots = AdmittedRoots::new_open().with_root_budget(2);

        assert!(matches!(
            roots.authorise_within_budget(a.path()).expect("io"),
            AdmitOutcome::Authorised(_)
        ));
        assert!(matches!(
            roots.authorise_within_budget(b.path()).expect("io"),
            AdmitOutcome::Authorised(_)
        ));
        // Third distinct root is over budget — and must NOT be admitted.
        assert!(matches!(
            roots.authorise_within_budget(c.path()).expect("io"),
            AdmitOutcome::OverBudget
        ));
        let canonical_c = std::fs::canonicalize(c.path()).unwrap();
        assert!(
            !roots.is_admitted(&canonical_c),
            "an over-budget root must never be admitted (no descriptor opened)"
        );
        // An already-admitted root keeps authorising past budget.
        assert!(matches!(
            roots.authorise_within_budget(a.path()).expect("io"),
            AdmitOutcome::Authorised(_)
        ));
    }

    #[test]
    fn authorise_within_budget_distinguishes_unlisted_from_over_budget() {
        // CIB-154: in Allowlist mode an over-budget allow-listed root is
        // OverBudget; an unlisted root is a plain Refused — never conflated.
        let a = tempfile::tempdir().expect("tempdir");
        let b = tempfile::tempdir().expect("tempdir");
        let unlisted = tempfile::tempdir().expect("tempdir");

        let allow = AllowPolicy::new(
            [
                std::fs::canonicalize(a.path()).unwrap(),
                std::fs::canonicalize(b.path()).unwrap(),
            ],
            std::iter::empty(),
        );
        let mut roots = AdmittedRoots::new_allowlist_with_policy(allow).with_root_budget(1);

        assert!(matches!(
            roots.authorise_within_budget(a.path()).expect("io"),
            AdmitOutcome::Authorised(_)
        ));
        // Allow-listed but over budget.
        assert!(matches!(
            roots.authorise_within_budget(b.path()).expect("io"),
            AdmitOutcome::OverBudget
        ));
        // Unlisted → plain refusal, distinct from the budget refusal.
        assert!(matches!(
            roots.authorise_within_budget(unlisted.path()).expect("io"),
            AdmitOutcome::Refused
        ));
    }

    #[cfg(unix)]
    #[test]
    fn authorise_within_budget_resolves_symlink_components_consistently() {
        // CIB-154 TOCTOU: a root named through a symlink must be budget-checked
        // and admitted against ONE resolved canonical path. Fill the budget via
        // the real path, then name a DISTINCT new dir through a symlink: the
        // gate must see it as over-budget (its single resolution is genuinely
        // new) and refuse it — never admit it because a re-resolution disagreed.
        let dir_a = tempfile::tempdir().expect("tempdir");
        let dir_b = tempfile::tempdir().expect("tempdir");
        let link_root = tempfile::tempdir().expect("tempdir");
        let link_to_b = link_root.path().join("link-to-b");
        std::os::unix::fs::symlink(dir_b.path(), &link_to_b).expect("symlink");

        let mut roots = AdmittedRoots::new_open().with_root_budget(1);

        // Budget filled by the first distinct root.
        assert!(matches!(
            roots.authorise_within_budget(dir_a.path()).expect("io"),
            AdmitOutcome::Authorised(_)
        ));

        // Naming a distinct new dir through a symlink resolves once to dir_b,
        // which is genuinely new → over budget, and must not be admitted.
        assert!(matches!(
            roots.authorise_within_budget(&link_to_b).expect("io"),
            AdmitOutcome::OverBudget
        ));
        let canonical_b = std::fs::canonicalize(dir_b.path()).unwrap();
        assert!(
            !roots.is_admitted(&canonical_b),
            "a symlinked over-budget root must not slip past the budget guard"
        );

        // But the ALREADY-admitted root remains authorised when named through a
        // symlink that resolves to it — consistent single resolution both ways.
        let link_to_a = link_root.path().join("link-to-a");
        std::os::unix::fs::symlink(dir_a.path(), &link_to_a).expect("symlink");
        assert!(matches!(
            roots.authorise_within_budget(&link_to_a).expect("io"),
            AdmitOutcome::Authorised(_)
        ));
    }

    #[test]
    fn root_budget_defaults_when_unset() {
        // Constructors without an explicit budget default to the pinned ceiling.
        // Assert against the constant, not the literal, so the test tracks the
        // default if it is ever tuned in `dos.rs`.
        assert_eq!(
            AdmittedRoots::new_open().root_budget(),
            DEFAULT_MAX_ADMITTED_ROOTS
        );
        assert_eq!(
            AdmittedRoots::new_allowlist_with_policy(AllowPolicy::default()).root_budget(),
            DEFAULT_MAX_ADMITTED_ROOTS,
        );
    }

    #[test]
    fn root_budget_zero_clamps_to_one() {
        // A 0 budget would refuse every admission; clamp to 1 so the connection
        // can always admit its own workspace root (operator-typo defence).
        assert_eq!(
            AdmittedRoots::new_open().with_root_budget(0).root_budget(),
            1
        );
    }

    #[test]
    fn no_cwd_in_auth_path() {
        // The workspace is identified by its canonical path + held dirfd, never
        // by the peer's working directory. Guard against a regression that
        // reaches for the procfs per-pid working directory or the process
        // working directory in the admission/read path. Needles are assembled
        // at runtime so this test's own source does not trip the check.
        let admission = include_str!("workspace_admission.rs");
        let path_safety = include_str!("path_safety.rs");
        let proc_needle = format!("/{}/", "proc");
        let cwd_needle = format!("current{}dir", "_");
        for (name, src) in [
            ("workspace_admission", admission),
            ("path_safety", path_safety),
        ] {
            assert!(
                !src.contains(&proc_needle),
                "{name} must not consult the procfs per-pid working directory in the auth/read path"
            );
            assert!(
                !src.contains(&cwd_needle),
                "{name} must not consult the process working directory in the auth/read path"
            );
        }
    }
}
