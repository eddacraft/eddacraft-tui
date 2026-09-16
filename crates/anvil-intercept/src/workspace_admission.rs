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

/// CIB-414: the outcome of [`AdmittedRoots::authorise_graph_root_within_budget`]
/// — the single, non-bypassable entry point the `anvil/gctx/*` verbs use.
///
/// The graph-root rule and the admit step are deliberately **not** separately
/// callable (S1): CIB-398 → CIB-414 is exactly the failure shape where one code
/// path ran the check and another did not, so there is no public "admit this
/// already-canonical root" seam a future verb handler could reach for instead.
#[derive(Debug)]
pub enum GraphAdmitOutcome<'a> {
    /// The root is a graph root and is authorised; carries the held anchor.
    Authorised(&'a WorkspaceAnchor),
    /// Admissible but over the connection's [`AdmittedRoots::root_budget`].
    OverBudget,
    /// Refused by the graph-root rule: the root is a directory inside a
    /// repository, or its repository structure could not be confirmed. Never
    /// admitted, and never charged to the CIB-154 root budget.
    NotGraphRoot,
    /// Refused by ordinary admission: unresolvable, or (in `Allowlist` mode)
    /// unlisted.
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

    /// CIB-414: the **only** way to admit a root for a graph (`anvil/gctx/*`)
    /// verb. The graph-root rule and the admit step are fused here so a verb
    /// handler cannot structurally skip the rule — there is no public
    /// already-canonical admit seam beside it (S1). `canonical` must be the
    /// caller's single resolution of the client spelling; that same path is
    /// used for the rule, the budget check and the admission, so a same-uid
    /// writer cannot swap a component between two `canonicalize` calls (the
    /// CIB-154 TOCTOU shape).
    ///
    /// The rule runs **before** admission, so a refused nested root is never
    /// first-touch-adopted in `Open` mode and never consumes the CIB-154
    /// per-connection root budget.
    ///
    /// # Errors
    /// Propagates the open error when admission is attempted for a root that is
    /// a graph root, admissible and within budget but cannot be opened.
    pub fn authorise_graph_root_within_budget(
        &mut self,
        canonical: &Path,
    ) -> io::Result<GraphAdmitOutcome<'_>> {
        if !self.permits_graph_root(canonical) {
            return Ok(GraphAdmitOutcome::NotGraphRoot);
        }
        Ok(match self.authorise_canonical_within_budget(canonical)? {
            AdmitOutcome::Authorised(anchor) => GraphAdmitOutcome::Authorised(anchor),
            AdmitOutcome::OverBudget => GraphAdmitOutcome::OverBudget,
            AdmitOutcome::Refused => GraphAdmitOutcome::Refused,
        })
    }

    /// [`Self::authorise_within_budget`] on an **already-canonicalised** root.
    /// Private: the graph verbs reach it only through
    /// [`Self::authorise_graph_root_within_budget`], which cannot skip the
    /// CIB-414 rule.
    ///
    /// # Errors
    /// Propagates the open error when admission is attempted for a root that is
    /// admissible and within budget but cannot be opened.
    fn authorise_canonical_within_budget(
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
    ///
    /// Private (S1): reachable only through
    /// [`Self::authorise_graph_root_within_budget`], so no caller can admit a
    /// graph root without it.
    #[must_use]
    fn permits_graph_root(&self, canonical_root: &Path) -> bool {
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

/// A tri-state filesystem answer. `Unknown` is the whole point: a syscall that
/// fails for any reason **other than** "does not exist" tells us nothing, and
/// must never be collapsed into `No` (M2). Every caller fails CLOSED on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Confirm {
    Yes,
    No,
    Unknown,
}

impl Confirm {
    /// `Yes` only when every part is `Yes`; `Unknown` as soon as any part is
    /// unknown (an unknown part could have been the missing `Yes`).
    fn all(parts: impl IntoIterator<Item = Self>) -> Self {
        let mut verdict = Self::Yes;
        for part in parts {
            match part {
                Self::Yes => {}
                Self::No => {
                    if verdict != Self::Unknown {
                        verdict = Self::No;
                    }
                }
                Self::Unknown => verdict = Self::Unknown,
            }
        }
        verdict
    }
}

/// Is `path` a real (non-symlink) directory? `NotFound` ⇒ `No` (definitely
/// absent); a symlink ⇒ `No` (git metadata is never followed through a symlink
/// here); any other error ⇒ `Unknown` (fail closed).
fn confirm_non_symlink_dir(path: &Path) -> Confirm {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Confirm::No,
        Ok(meta) if meta.is_dir() => Confirm::Yes,
        Ok(_) => Confirm::No,
        Err(err) if err.kind() == io::ErrorKind::NotFound => Confirm::No,
        Err(_) => Confirm::Unknown,
    }
}

/// Is `path` a real (non-symlink) regular file? Same failure directions as
/// [`confirm_non_symlink_dir`].
fn confirm_non_symlink_file(path: &Path) -> Confirm {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Confirm::No,
        Ok(meta) if meta.is_file() => Confirm::Yes,
        Ok(_) => Confirm::No,
        Err(err) if err.kind() == io::ErrorKind::NotFound => Confirm::No,
        Err(_) => Confirm::Unknown,
    }
}

/// What `<root>/.git` says about `<root>`.
#[derive(Debug)]
enum GitDirVerdict {
    /// `<root>` **is** a Git checkout; carries the resolved git directory.
    Checkout(PathBuf),
    /// `<root>` is definitely **not** a Git checkout: `.git` does not exist,
    /// or exists but is not a minimum-viable repository by git's own rule.
    /// Safe to keep walking upward.
    Absent,
    /// Could not confirm either way — an I/O or permission error, a `.git` of
    /// a shape we cannot parse (non-UTF-8 content, a socket/FIFO, a symlinked
    /// gitfile), or a gitfile whose pointer is not recognisable Git authority.
    /// This is NOT "not a checkout": the caller must refuse (M2).
    Inconclusive,
}

/// CIB-414: whether a canonical path is a **graph root** by repository
/// structure — equality with the nearest enclosing Git checkout. A path that
/// is in no Git checkout at all has no enclosing repository to be nested
/// inside, so it is its own root; the admitted-set half of
/// [`AdmittedRoots::permits_graph_root`] is what guards those.
///
/// Why this matters (CIB-398): the daemon keys its graph on the root it is
/// handed and every file identity it projects is root-relative, so
/// `<repo>/secrets` as the root rebases `secrets/token.ts` to `token.ts` and
/// the CE-3 sensitive-path deny-list never sees the denied segment.
///
/// Fails CLOSED (M2): an ancestor whose `.git` cannot be read or parsed makes
/// the whole judgement `Inconclusive`, which refuses. "Could not confirm" is
/// never treated as "not a checkout" — that conflation is what let an
/// unreadable or symlinked parent `.git` promote `<repo>/secrets` to a root.
///
/// ## Documented residual — read before widening this (operator decision)
///
/// Git's on-disk model carries **no authenticity signal**: repository and
/// worktree metadata are plain files any same-uid process can write. This
/// predicate is therefore a *structural* check, not an authenticity check. An
/// attacker who can write inside the repository can also build a genuinely
/// git-valid repository at `<repo>/secrets` (`HEAD` + `objects/` + `refs/`)
/// and it will pass — exactly as a legitimately vendored nested repository
/// does, because on disk they are indistinguishable.
///
/// The MCP layer escapes this because its rule is *relational*: it is anchored
/// on a trusted `server_root` (the process cwd), so it can refuse any checkout
/// nested strictly inside that anchor. The daemon has **no such anchor** on
/// first contact — a socket client simply names a root. Applying MCP's rule
/// unconditionally here would refuse every legitimate repository under a
/// `$HOME`-as-checkout (dotfiles) or any vendored nested repo, and
/// over-refusal is a real defect too. The operator has decided this repair is
/// bounded: the fabrication residual is DOCUMENTED here, not closed.
///
/// What this predicate therefore *does* guarantee: a plain directory inside a
/// repository is never a graph root; a `.git` that is not a minimum-viable git
/// repository by git's own rule is not Git authority; a gitfile is authority
/// only under a reciprocal, non-symlinked worktree layout; and anything it
/// cannot confirm is refused.
///
/// Reads the on-disk gitdir layout; never shells out to `git`.
#[must_use]
fn is_graph_root(canonical_root: &Path) -> bool {
    match containing_git_worktree_root(canonical_root) {
        // A confirmed enclosing checkout: only its own root is a graph root.
        ContainingCheckout::Root(checkout) => checkout == canonical_root,
        // Confirmed absent all the way to the filesystem root: nothing encloses
        // this path, so it is its own root (the admitted-set half guards it).
        ContainingCheckout::None => true,
        // Could not confirm ⇒ refuse. Fail closed.
        ContainingCheckout::Inconclusive => false,
    }
}

/// The result of walking upward for the nearest enclosing checkout.
#[derive(Debug)]
enum ContainingCheckout {
    Root(PathBuf),
    /// Every ancestor was **confirmed** not to be a checkout.
    None,
    /// Some ancestor could not be judged; the walk stopped there.
    Inconclusive,
}

/// The nearest enclosing Git worktree root at or above `path`. `.git` is a
/// checkout when it is a minimum-viable git directory (or a symlink to one), a
/// main-worktree gitfile (`git clone --separate-git-dir`), or a linked-worktree
/// gitfile with a fail-closed reciprocal `gitdir` layout. `path` is assumed
/// canonical, so no component is re-resolved here.
///
/// The walk only continues upward on a **confirmed** `Absent`; an
/// `Inconclusive` ancestor stops it and refuses (M2).
fn containing_git_worktree_root(path: &Path) -> ContainingCheckout {
    let mut current = path.to_path_buf();
    loop {
        match inspect_git_dir(&current) {
            GitDirVerdict::Checkout(_) => return ContainingCheckout::Root(current),
            GitDirVerdict::Inconclusive => return ContainingCheckout::Inconclusive,
            GitDirVerdict::Absent => {}
        }
        if !current.pop() {
            // Reached the filesystem root with every level confirmed absent.
            return ContainingCheckout::None;
        }
    }
}

/// Whether `checkout` sits inside another Git worktree without being a
/// registered linked worktree of that repository. Daemon-side form of MCP
/// `nested_untrusted_git_root`, applied to `.git` **files** (see the residual
/// note on [`is_graph_root`] for why it is not applied to `.git` directories).
///
/// Failure directions: no parent ⇒ `false` (nothing encloses it); an
/// inconclusive ancestor ⇒ `true` (we cannot confirm the enclosing repository
/// registers this path, so treat it as untrusted — fail closed).
fn nested_untrusted_git_checkout(checkout: &Path) -> bool {
    let mut parent = checkout.to_path_buf();
    if !parent.pop() {
        return false;
    }
    match containing_git_worktree_root(&parent) {
        ContainingCheckout::Root(outer) => !registered_worktree_roots(&outer)
            .iter()
            .any(|worktree| worktree == checkout),
        ContainingCheckout::None => false,
        ContainingCheckout::Inconclusive => true,
    }
}

/// Resolve `<root>/.git` as a minimum-viable git directory, a symlink to one,
/// or a recognised gitfile. Portable: the `graph_base_trigger` copy is
/// `cfg(unix)`, while the save-time admission path is served on Unix and
/// Windows alike.
///
/// A `.git` **directory** is Git authority only when it is a minimum-viable
/// repository by git's own `is_git_directory` rule — a `HEAD` file plus
/// `objects/` and `refs/` (see [`confirm_minimum_viable_git_dir`]). A bare
/// `mkdir <repo>/secrets/.git` is not a repository to git and must not be one
/// here either (M1 / CIB-414).
///
/// A `.git` **file** is Git authority only when it is a main-worktree gitfile
/// (the pointer is a git directory *not* under `worktrees/`) or a linked-
/// worktree admin directory with a reciprocal `gitdir` backlink under
/// `<common>/worktrees/<name>` — and, when nested inside another checkout,
/// only when that enclosing repository lists this path as a worktree. Any
/// readable `gitdir:` line is not enough: a client that can create
/// `<repo>/secrets/.git` must not make this path a checkout (CIB-414 / CE-3).
///
/// Every branch below states its failure direction. The invariant: only a
/// *confirmed* absence returns `Absent`.
fn inspect_git_dir(repo_root: &Path) -> GitDirVerdict {
    let dot_git = repo_root.join(".git");
    let meta = match std::fs::symlink_metadata(&dot_git) {
        Ok(meta) => meta,
        // Confirmed: there is no `.git` here at all → keep walking upward.
        Err(err) if err.kind() == io::ErrorKind::NotFound => return GitDirVerdict::Absent,
        // Permission denied / any other I/O error → cannot judge → refuse.
        Err(_) => return GitDirVerdict::Inconclusive,
    };
    match dot_git_is_directory(&meta, &dot_git) {
        // A directory: Git authority only if it is a real repository.
        Confirm::Yes => {
            return match confirm_minimum_viable_git_dir(&dot_git) {
                Confirm::Yes => GitDirVerdict::Checkout(dot_git),
                // Confirmed not a repository by git's own rule (a planted bare
                // `.git` dir) → keep walking to the enclosing checkout.
                Confirm::No => GitDirVerdict::Absent,
                // Could not confirm the repository shape → refuse.
                Confirm::Unknown => GitDirVerdict::Inconclusive,
            };
        }
        // Not a directory: fall through to the gitfile shapes.
        Confirm::No => {}
        // Could not confirm the type (e.g. a symlink we cannot stat) → refuse.
        Confirm::Unknown => return GitDirVerdict::Inconclusive,
    }
    // A `.git` that is neither a directory nor a regular file (socket, FIFO,
    // device, or a symlink to one) is a shape we cannot judge → refuse.
    if !meta.is_file() {
        return GitDirVerdict::Inconclusive;
    }
    let Some(git_dir) = parse_gitdir_pointer(&dot_git) else {
        // Unreadable, non-UTF-8, or carrying no `gitdir:` line. A regular file
        // named `.git` is not an ordinary "no repository here" state, so this
        // is inconclusive, never absent (M2).
        return GitDirVerdict::Inconclusive;
    };
    let recognised = gitfile_layout_matches_linked_worktree(&dot_git, &git_dir)
        || gitfile_is_main_worktree(&git_dir);
    // An unrecognised or nested-untrusted gitfile is refused rather than
    // ignored: a planted `<repo>/secrets/.git` file must not become a root,
    // and must not be silently treated as "no `.git` here" either.
    if !recognised || nested_untrusted_git_checkout(repo_root) {
        return GitDirVerdict::Inconclusive;
    }
    GitDirVerdict::Checkout(git_dir)
}

/// Git's own minimum-viable-repository rule (`setup.c: is_git_directory`): a
/// `HEAD` entry plus `objects/` and `refs/`. `HEAD` is per-worktree; `objects/`
/// and `refs/` live in the **common** directory, which for a linked worktree's
/// admin dir is reached through its `commondir` pointer.
///
/// Failure direction: `Unknown` from any probe propagates (fail closed).
fn confirm_minimum_viable_git_dir(git_dir: &Path) -> Confirm {
    let head = confirm_non_symlink_file(&git_dir.join("HEAD"));
    if head != Confirm::Yes {
        return head;
    }
    let common = resolve_common_dir(git_dir);
    Confirm::all([
        confirm_non_symlink_dir(&common.join("objects")),
        confirm_non_symlink_dir(&common.join("refs")),
    ])
}

/// Is `.git` a directory — following a symlink, because git stat()s through
/// one and a symlinked `.git` is a real, git-recognised layout. `NotFound`
/// (a dangling symlink) ⇒ `No`; any other error ⇒ `Unknown` (fail closed).
/// Before M2 this used `symlink_metadata` only, so a symlinked `.git` was
/// neither a directory nor readable as a gitfile and the walk climbed past the
/// whole checkout.
fn dot_git_is_directory(meta: &std::fs::Metadata, dot_git: &Path) -> Confirm {
    if meta.is_dir() {
        return Confirm::Yes;
    }
    if !meta.file_type().is_symlink() {
        return Confirm::No;
    }
    match std::fs::metadata(dot_git) {
        Ok(followed) if followed.is_dir() => Confirm::Yes,
        // A symlink to a regular file: not a directory. The caller then sees a
        // non-regular `.git` and refuses — we never follow a symlinked gitfile.
        Ok(_) => Confirm::No,
        Err(err) if err.kind() == io::ErrorKind::NotFound => Confirm::No,
        Err(_) => Confirm::Unknown,
    }
}

/// Parse a `gitdir: <path>` pointer relative to the file that contains it.
///
/// Failure direction: `None` means "this is not a parsable gitfile" — an I/O
/// error, non-UTF-8 content (`read_to_string` returns `InvalidData`), no
/// `gitdir:` line, an empty target, or no parent directory to resolve a
/// relative target against. The caller maps every `None` to `Inconclusive`,
/// never to `Absent` (M2).
fn parse_gitdir_pointer(gitdir_file: &Path) -> Option<PathBuf> {
    let contents = std::fs::read_to_string(gitdir_file).ok()?;
    let line = contents.lines().find_map(|l| {
        l.trim()
            .strip_prefix("gitdir:")
            .map(str::trim)
            .filter(|p| !p.is_empty())
    })?;
    let pointer = Path::new(line);
    Some(if pointer.is_absolute() {
        pointer.to_path_buf()
    } else {
        lexical_join(gitdir_file.parent()?, pointer)
    })
}

/// A main-worktree gitfile (`git clone --separate-git-dir`): `git_dir` is a
/// minimum-viable git directory whose parent component is not `worktrees`.
///
/// Failure direction: every probe that cannot be confirmed returns `false`
/// ("not recognised as Git authority"), which the caller turns into
/// `Inconclusive` — a refusal, not an "absent".
fn gitfile_is_main_worktree(git_dir: &Path) -> bool {
    if !is_non_symlink_dir(git_dir) {
        return false;
    }
    let Ok(canonical) = std::fs::canonicalize(git_dir) else {
        return false;
    };
    let Some(parent) = canonical.parent() else {
        return false;
    };
    if parent.file_name().is_some_and(|name| name == "worktrees") {
        return false;
    }
    confirm_minimum_viable_git_dir(&canonical) == Confirm::Yes
}

/// Confirmed-real directory. Anything unconfirmed is `false` — every caller
/// treats `false` as "not Git authority", which is the fail-closed direction.
fn is_non_symlink_dir(path: &Path) -> bool {
    confirm_non_symlink_dir(path) == Confirm::Yes
}

/// Confirmed-real regular file; same fail-closed direction as
/// [`is_non_symlink_dir`].
fn is_non_symlink_file(path: &Path) -> bool {
    confirm_non_symlink_file(path) == Confirm::Yes
}

/// Internal gitfile layout: `git_dir` is `<common>/worktrees/<name>` and its
/// `gitdir` backlink points at this gitfile, whose `gitdir:` target
/// canonicalises back to that admin dir. Rejects symlinks at every hop.
///
/// M3: this is the **bidirectional** check. `registered_worktree_roots` must
/// not trust `<common>/worktrees/<name>/gitdir` on its own — a planted
/// `gitdir` naming `<repo>/secrets/.git` would otherwise register
/// `<repo>/secrets` as a linked worktree and exempt it from every nesting
/// rule. The registration is honoured only when `<root>/.git`'s own `gitdir:`
/// pointer resolves back to the admin dir that named it.
///
/// Failure direction: every unreadable, mismatched, symlinked or
/// non-canonicalisable hop returns `false` — "not a validated registration",
/// which removes authority rather than granting it.
fn gitfile_layout_matches_linked_worktree(dot_git: &Path, git_dir: &Path) -> bool {
    if !is_non_symlink_file(dot_git) {
        return false;
    }
    let Ok(git_dir_meta) = std::fs::symlink_metadata(git_dir) else {
        return false;
    };
    if git_dir_meta.file_type().is_symlink() || !git_dir_meta.is_dir() {
        return false;
    }
    let common = resolve_common_dir(git_dir);
    let Ok(common) = std::fs::canonicalize(&common) else {
        return false;
    };
    let Ok(git_dir) = std::fs::canonicalize(git_dir) else {
        return false;
    };
    let worktrees = common.join("worktrees");
    let Ok(worktrees_meta) = std::fs::symlink_metadata(&worktrees) else {
        return false;
    };
    if worktrees_meta.file_type().is_symlink() || !worktrees_meta.is_dir() {
        return false;
    }
    let Ok(worktrees) = std::fs::canonicalize(&worktrees) else {
        return false;
    };
    let Some(name) = git_dir
        .file_name()
        .filter(|name| !name.is_empty() && *name != "." && *name != "..")
    else {
        return false;
    };
    let Some(parent) = git_dir.parent() else {
        return false;
    };
    if parent != worktrees.as_path() {
        return false;
    }
    let Ok(expected) = std::fs::canonicalize(worktrees.join(name)) else {
        return false;
    };
    if expected != git_dir {
        return false;
    }
    let backlink = git_dir.join("gitdir");
    let Ok(back_meta) = std::fs::symlink_metadata(&backlink) else {
        return false;
    };
    if back_meta.file_type().is_symlink() || !back_meta.is_file() {
        return false;
    }
    let Ok(raw) = std::fs::read_to_string(&backlink) else {
        return false;
    };
    let target = Path::new(raw.trim());
    if target.as_os_str().is_empty() {
        return false;
    }
    let pointed = if target.is_absolute() {
        target.to_path_buf()
    } else {
        lexical_join(&git_dir, target)
    };
    let Ok(pointed) = std::fs::canonicalize(&pointed) else {
        return false;
    };
    let Ok(dot_git) = std::fs::canonicalize(dot_git) else {
        return false;
    };
    if pointed != dot_git {
        return false;
    }
    let Some(forward) = parse_gitdir_pointer(&dot_git) else {
        return false;
    };
    let Ok(forward) = std::fs::canonicalize(&forward) else {
        return false;
    };
    forward == git_dir
}

/// Resolve the **common** gitdir of a (possibly per-worktree) gitdir: a linked
/// worktree's gitdir carries a `commondir` pointer, a main worktree has none.
///
/// Failure direction: an unreadable or non-UTF-8 `commondir` falls back to
/// `git_dir` itself. Safe because every consumer then *probes* the result
/// (`objects/`, `refs/`, `worktrees/`); a wrong fallback makes a probe fail —
/// a refusal, never extra authority.
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
///
/// Failure direction: none — it is total and never consults the filesystem, so
/// it cannot fail open. Its output is always probed or canonicalised by the
/// caller before it is trusted.
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
/// Failure direction: an `Absent` or `Inconclusive` `.git`, an unreadable
/// admin entry, or a registration that does not validate yields **fewer**
/// exemptions — i.e. more refusals, never more authority.
fn registered_worktree_roots(repo_root: &Path) -> Vec<PathBuf> {
    let GitDirVerdict::Checkout(git_dir) = inspect_git_dir(repo_root) else {
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

    let worktrees = common.join("worktrees");
    if !is_non_symlink_dir(&worktrees) {
        return roots;
    }
    let Ok(entries) = std::fs::read_dir(&worktrees) else {
        return roots;
    };
    for entry in entries.flatten() {
        let admin = entry.path();
        if !is_non_symlink_dir(&admin) {
            continue;
        }
        let gitdir_file = admin.join("gitdir");
        if !is_non_symlink_file(&gitdir_file) {
            continue;
        }
        let Ok(raw) = std::fs::read_to_string(&gitdir_file) else {
            continue;
        };
        let pointed = Path::new(raw.trim());
        if pointed.as_os_str().is_empty() {
            continue;
        }
        let pointed = if pointed.is_absolute() {
            pointed.to_path_buf()
        } else {
            lexical_join(&admin, pointed)
        };
        if !gitfile_layout_matches_linked_worktree(&pointed, &admin) {
            continue;
        }
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
    ///
    /// Skip the structural `is_graph_root` half when an ancestor checkout
    /// exists (a stale `/tmp/.git` would make every tempfile descendant fail).
    #[test]
    fn nested_dir_inside_a_non_git_admitted_root_is_refused_by_the_admitted_set() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = std::fs::canonicalize(tmp.path()).expect("root canonicalises");
        let nested = root.join("secrets");
        std::fs::create_dir_all(&nested).expect("nested dir");

        let mut roots = AdmittedRoots::new_open();
        roots.admit(&root).expect("admit root");

        if is_graph_root(&root) {
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
        }
        assert!(
            !roots.permits_graph_root(&nested),
            "a directory inside an admitted root is not a graph root: {}",
            nested.to_str().expect("utf-8 nested path"),
        );
    }

    /// Main checkout plus a registered linked worktree nested inside it
    /// (`<main>/.worktrees/linked`). Mirrors the MCP CIB-398 on-disk layout.
    fn repo_with_linked_worktree(root: &Path) -> (PathBuf, PathBuf) {
        let main = root.join("main");
        let common = main.join(".git");
        write_minimum_viable_git_dir(&common);

        let admin = common.join("worktrees").join("linked");
        std::fs::create_dir_all(&admin).expect("worktree admin dir");
        std::fs::write(admin.join("HEAD"), b"ref: refs/heads/feature\n").expect("linked HEAD");
        std::fs::write(admin.join("commondir"), b"../..\n").expect("commondir");

        let linked = main.join(".worktrees").join("linked");
        std::fs::create_dir_all(&linked).expect("linked worktree");
        let git_file = linked.join(".git");
        std::fs::write(&git_file, format!("gitdir: {}\n", admin.display())).expect(".git file");
        std::fs::write(admin.join("gitdir"), format!("{}\n", git_file.display()))
            .expect("gitdir back-pointer");

        let main = std::fs::canonicalize(&main).expect("main canonicalises");
        let linked = std::fs::canonicalize(&linked).expect("linked canonicalises");
        (main, linked)
    }

    fn nested_secrets_dir(main: &Path) -> PathBuf {
        let nested = main.join("secrets");
        std::fs::create_dir_all(&nested).expect("nested dir");
        std::fs::canonicalize(&nested).expect("nested canonicalises")
    }

    /// Git's minimum-viable repository (`is_git_directory`): `HEAD` plus
    /// `objects/` and `refs/`. A bare `mkdir .git` is deliberately NOT this.
    fn write_minimum_viable_git_dir(git_dir: &Path) {
        std::fs::create_dir_all(git_dir.join("refs")).expect("git refs");
        std::fs::create_dir_all(git_dir.join("objects")).expect("git objects");
        std::fs::write(git_dir.join("HEAD"), b"ref: refs/heads/main\n").expect("HEAD");
    }

    fn write_git_directory(root: &Path) {
        write_minimum_viable_git_dir(&root.join(".git"));
    }

    /// Main worktree whose `.git` is a gitfile pointing at a sibling git
    /// directory (`git clone --separate-git-dir`).
    fn repo_with_separate_git_dir(root: &Path) -> PathBuf {
        let main = root.join("main");
        let git_dir = root.join("main.git");
        write_minimum_viable_git_dir(&git_dir);
        std::fs::create_dir_all(&main).expect("worktree");
        std::fs::write(
            main.join(".git"),
            format!("gitdir: {}\n", git_dir.display()),
        )
        .expect("gitfile");
        std::fs::canonicalize(&main).expect("main canonicalises")
    }

    fn assert_nested_secrets_refused_parent_authorised(main: &Path) {
        let nested = nested_secrets_dir(main);
        let mut roots = AdmittedRoots::new_open().with_root_budget(1);
        assert!(
            !roots.permits_graph_root(&nested),
            "first-contact nested secrets must be refused: {}",
            nested.to_str().expect("utf-8 nested path"),
        );
        if !is_graph_root(main) {
            // A stale ancestor checkout (e.g. `/tmp/.git`) captured the walk
            // for a main-worktree gitfile. Nested secrets must still not win.
            return;
        }
        assert!(
            roots.permits_graph_root(main),
            "parent checkout must stay a graph root: {}",
            main.to_str().expect("utf-8 main path"),
        );
        assert!(
            !roots.root_budget_would_block(main),
            "refusing nested secrets must not consume the CIB-154 root budget"
        );
        assert!(matches!(
            roots.authorise_within_budget(main).expect("io"),
            AdmitOutcome::Authorised(_)
        ));
        assert!(
            !roots.permits_graph_root(&nested),
            "nested secrets stay refused after the parent is admitted"
        );
    }

    #[test]
    fn nested_dir_inside_a_git_checkout_is_not_a_graph_root() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let (main, linked) = repo_with_linked_worktree(tmp.path());
        let nested = nested_secrets_dir(&main);

        assert!(is_graph_root(&main), "main checkout is a graph root");
        assert!(
            is_graph_root(&linked),
            "registered linked worktree is a graph root"
        );
        assert!(
            !is_graph_root(&nested),
            "a directory inside the checkout is not a graph root"
        );

        let mut roots = AdmittedRoots::new_open().with_root_budget(1);
        assert!(
            !roots.permits_graph_root(&nested),
            "first contact must refuse a nested directory"
        );
        assert!(
            !roots.root_budget_would_block(&main),
            "a refused nested directory must not consume the CIB-154 root budget"
        );
        roots.admit(&main).expect("admit main");
        assert!(roots.permits_graph_root(&main));
        assert!(roots.permits_graph_root(&linked));
        assert!(
            !roots.permits_graph_root(&nested),
            "nested directory stays refused after the parent is admitted"
        );
    }

    #[test]
    fn nested_dir_inside_a_git_checkout_is_refused_in_allowlist_mode() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let (main, linked) = repo_with_linked_worktree(tmp.path());
        let nested = nested_secrets_dir(&main);
        let allow = AllowPolicy::new(std::iter::empty(), [main.clone()]);
        let mut roots = AdmittedRoots::new_allowlist_with_policy(allow).with_root_budget(1);

        assert!(
            !roots.permits_graph_root(&nested),
            "first contact must refuse a nested directory even when a prefix allow entry permits it"
        );
        assert!(roots.permits_graph_root(&main));
        assert!(matches!(
            roots.authorise_within_budget(&main).expect("io"),
            AdmitOutcome::Authorised(_)
        ));
        assert!(roots.permits_graph_root(&linked));
        assert!(
            !roots.permits_graph_root(&nested),
            "nested directory stays refused after the allow-listed parent is admitted"
        );
    }

    #[test]
    fn forged_nested_gitdir_pointer_is_not_a_graph_root() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let (main, _linked) = repo_with_linked_worktree(tmp.path());
        let nested = nested_secrets_dir(&main);
        std::fs::write(
            nested.join(".git"),
            format!("gitdir: {}\n", main.join(".git").display()),
        )
        .expect("forged gitdir pointer");

        assert!(
            !is_graph_root(&nested),
            "an unregistered gitdir pointer must not make nested secrets a graph root"
        );
        let roots = AdmittedRoots::new_open();
        assert!(
            !roots.permits_graph_root(&nested),
            "first contact must refuse a forged nested gitdir pointer"
        );
    }

    #[test]
    fn planted_self_consistent_nested_gitfile_is_not_a_graph_root() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let (main, _linked) = repo_with_linked_worktree(tmp.path());
        let nested = nested_secrets_dir(&main);
        let fake_common = nested.join("fake-git");
        let admin = fake_common.join("worktrees").join("planted");
        std::fs::create_dir_all(&admin).expect("planted admin");
        std::fs::write(admin.join("commondir"), b"../..\n").expect("planted commondir");
        let git_file = nested.join(".git");
        std::fs::write(&git_file, format!("gitdir: {}\n", admin.display()))
            .expect("planted gitfile");
        std::fs::write(admin.join("gitdir"), format!("{}\n", git_file.display()))
            .expect("planted backlink");

        assert!(
            !is_graph_root(&nested),
            "a self-consistent planted gitfile inside a checkout is not a graph root"
        );
    }

    /// M1: the demonstrated CIB-414 hole. A single `mkdir <repo>/secrets/.git`
    /// made the daemon accept `<repo>/secrets` as its own graph root on first
    /// contact, because the detector accepted a self-declared `.git` on
    /// `is_dir()` alone — MORE permissive than git, which needs `HEAD` plus
    /// `objects/` and `refs/` before it will treat a directory as a repository.
    ///
    /// RED without the `confirm_minimum_viable_git_dir` call in
    /// `inspect_git_dir`.
    #[test]
    fn planted_bare_git_directory_inside_a_checkout_is_not_a_graph_root() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let (main, _linked) = repo_with_linked_worktree(tmp.path());
        let nested = nested_secrets_dir(&main);

        // Shape 1: a bare `mkdir .git`, nothing inside it.
        std::fs::create_dir_all(nested.join(".git")).expect("planted bare .git dir");
        assert!(
            !is_graph_root(&nested),
            "a bare `mkdir .git` is not a repository to git and must not be one here"
        );
        let mut roots = AdmittedRoots::new_open().with_root_budget(1);
        assert!(
            !roots.permits_graph_root(&nested),
            "first contact must refuse a planted bare `.git` directory"
        );
        assert!(
            !roots.root_budget_would_block(&main),
            "the refusal must not consume the CIB-154 root budget"
        );

        // Shape 2: partially furnished — `HEAD` but no `objects/`/`refs/`.
        std::fs::write(nested.join(".git").join("HEAD"), b"ref: refs/heads/main\n").expect("HEAD");
        assert!(
            !is_graph_root(&nested),
            "a `.git` directory missing objects/ and refs/ is not a repository"
        );
        assert!(!roots.permits_graph_root(&nested));

        // The real parent repository is unaffected.
        assert!(roots.permits_graph_root(&main), "parent stays a graph root");
    }

    /// M1: a `.git` **file** whose `gitdir:` pointer never resolves. The
    /// pointer used to be read and handed back without ever being checked.
    #[test]
    fn planted_git_file_with_unresolvable_pointer_is_not_a_graph_root() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let (main, _linked) = repo_with_linked_worktree(tmp.path());
        let nested = nested_secrets_dir(&main);
        std::fs::write(nested.join(".git"), b"gitdir: /nonexistent/anywhere\n")
            .expect("planted pointer");

        assert!(
            !is_graph_root(&nested),
            "an unresolvable gitdir pointer is not Git authority"
        );
        let roots = AdmittedRoots::new_open();
        assert!(!roots.permits_graph_root(&nested));
    }

    /// M2: a `.git` shape the detector cannot parse must mean "cannot safely
    /// judge — refuse", never "not a checkout". Before the repair, a non-UTF-8
    /// `gitdir:` line on the PARENT made `read_to_string` fail with
    /// `InvalidData`, the `?`-chained `Option` turned that into "no `.git`
    /// here", the walk climbed past the whole checkout and `<repo>/secrets`
    /// (and `<repo>/secrets/deep/deeper`) became graph roots.
    ///
    /// RED without the `Inconclusive` verdict in `inspect_git_dir`.
    #[test]
    fn unparsable_parent_git_file_refuses_the_nested_root() {
        for (label, bytes) in [
            (
                "non-UTF-8 gitdir pointer",
                b"gitdir: \xff\xfe/elsewhere\n".to_vec(),
            ),
            ("no gitdir line at all", b"not a gitfile\n".to_vec()),
        ] {
            let tmp = tempfile::tempdir().expect("tempdir");
            let main = tmp.path().join("main");
            std::fs::create_dir_all(&main).expect("worktree");
            std::fs::write(main.join(".git"), &bytes).expect("unparsable .git file");
            let main = std::fs::canonicalize(&main).expect("main canonicalises");
            let nested = nested_secrets_dir(&main);
            let deeper = nested.join("deep").join("deeper");
            std::fs::create_dir_all(&deeper).expect("deeper dir");
            let deeper = std::fs::canonicalize(&deeper).expect("deeper canonicalises");

            let roots = AdmittedRoots::new_open();
            assert!(
                !is_graph_root(&nested),
                "{label}: an unconfirmable parent must refuse, not fail open"
            );
            assert!(
                !is_graph_root(&deeper),
                "{label}: the fail-open used to reach arbitrarily deep"
            );
            assert!(!roots.permits_graph_root(&nested), "{label}");
            assert!(!roots.permits_graph_root(&deeper), "{label}");
            assert!(
                !roots.permits_graph_root(&main),
                "{label}: the unconfirmable root itself is refused too"
            );
        }
    }

    /// M2: the same fail-closed rule when the unparsable `.git` is on the
    /// candidate root itself.
    #[test]
    fn non_utf8_gitdir_pointer_on_the_root_is_not_a_graph_root() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let (main, _linked) = repo_with_linked_worktree(tmp.path());
        let nested = nested_secrets_dir(&main);
        std::fs::write(nested.join(".git"), b"gitdir: \xff\xfe/elsewhere\n")
            .expect("non-UTF-8 gitfile");

        assert!(!is_graph_root(&nested));
        let roots = AdmittedRoots::new_open();
        assert!(!roots.permits_graph_root(&nested));
    }

    /// DOCUMENTED RESIDUAL (operator decision — do NOT "fix" by widening the
    /// rule without re-reading the note on [`is_graph_root`]).
    ///
    /// Git's on-disk model has no authenticity signal. An attacker who can
    /// write inside the repository can build a fully git-valid repository at
    /// `<repo>/secrets`, and on disk that is indistinguishable from a
    /// legitimately vendored nested repository — so the structural rule
    /// accepts it. Refusing it would mean refusing every checkout nested
    /// inside another checkout, which over-refuses a real repository under a
    /// `$HOME`-as-checkout (dotfiles) and every vendored nested repo; MCP
    /// escapes this only because its rule is anchored on a trusted
    /// `server_root`, which the daemon does not have on first contact.
    ///
    /// This test pins the limit so it cannot drift silently into a claim the
    /// gate does not make.
    #[test]
    fn fully_valid_nested_repository_is_the_documented_residual() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let (main, _linked) = repo_with_linked_worktree(tmp.path());
        let nested = nested_secrets_dir(&main);
        write_git_directory(&nested);

        let roots = AdmittedRoots::new_open();
        assert!(
            roots.permits_graph_root(&nested),
            "RESIDUAL: a fully git-valid nested repository is accepted on first \
             contact — structural, not authenticity. See is_graph_root."
        );
    }

    #[test]
    fn descendant_repo_under_an_ancestor_checkout_still_authorises() {
        // `$HOME`/dotfiles analogue: a nested `.git` **directory** is its own
        // nearest checkout. Residual: the same equality authorises a planted
        // `<repo>/secrets/.git` directory on first contact.
        let tmp = tempfile::tempdir().expect("tempdir");
        let ancestor = tmp.path().join("dotfiles");
        write_git_directory(&ancestor);
        let descendant = ancestor.join("projects").join("app");
        write_git_directory(&descendant);
        let ancestor = std::fs::canonicalize(&ancestor).expect("ancestor canonicalises");
        let descendant = std::fs::canonicalize(&descendant).expect("descendant canonicalises");

        assert!(
            is_graph_root(&ancestor),
            "ancestor checkout is a graph root"
        );
        assert!(
            is_graph_root(&descendant),
            "a descendant repo under an ancestor checkout is still a graph root"
        );
        let mut roots = AdmittedRoots::new_open();
        assert!(
            roots.permits_graph_root(&descendant),
            "first contact must authorise the descendant repo"
        );
        roots.admit(&descendant).expect("admit descendant");
        assert!(roots.permits_graph_root(&descendant));
    }

    #[test]
    fn separate_git_dir_parent_refuses_nested_secrets() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let main = repo_with_separate_git_dir(tmp.path());
        assert_nested_secrets_refused_parent_authorised(&main);
    }

    #[cfg(unix)]
    #[test]
    fn git_symlink_to_directory_parent_refuses_nested_secrets() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let main = tmp.path().join("main");
        let git_dir = tmp.path().join("git-store");
        write_minimum_viable_git_dir(&git_dir);
        std::fs::create_dir_all(&main).expect("worktree");
        std::os::unix::fs::symlink(&git_dir, main.join(".git")).expect("symlink .git");
        let main = std::fs::canonicalize(&main).expect("main canonicalises");
        assert!(
            is_graph_root(&main),
            "a .git symlink to a directory is a graph root"
        );
        assert_nested_secrets_refused_parent_authorised(&main);
    }

    #[test]
    fn planted_worktrees_gitdir_without_reciprocal_gitfile_does_not_authorise_secrets() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let (main, _linked) = repo_with_linked_worktree(tmp.path());
        let nested = nested_secrets_dir(&main);
        let evil = main.join(".git").join("worktrees").join("evil");
        std::fs::create_dir_all(&evil).expect("planted admin");
        std::fs::write(evil.join("commondir"), b"../..\n").expect("planted commondir");
        std::fs::write(
            evil.join("gitdir"),
            format!("{}\n", nested.join(".git").display()),
        )
        .expect("planted gitdir without reciprocal gitfile");

        let mut roots = AdmittedRoots::new_open();
        roots.admit(&main).expect("admit parent");
        assert!(
            !roots.permits_graph_root(&nested),
            "a planted worktrees/*/gitdir without a reciprocal gitfile must not authorise secrets after the parent is admitted"
        );
    }

    /// M3: `registered_worktree_roots` must not trust
    /// `<common>/worktrees/<name>/gitdir` on its own. The registration is the
    /// *only* thing that exempts a root from the nesting rules, so an
    /// unvalidated one launders a planted nested checkout.
    ///
    /// The attack: plant a self-consistent gitfile at `<repo>/secrets` (which
    /// `nested_untrusted_git_checkout` would refuse, because the enclosing
    /// repository does not list it), then plant
    /// `<repo>/.git/worktrees/evil/gitdir` naming `<repo>/secrets/.git` so the
    /// enclosing repository *appears* to list it. Without the bidirectional
    /// check the exemption applies and `<repo>/secrets` becomes a graph root.
    ///
    /// RED by replacing the `gitfile_layout_matches_linked_worktree` guard in
    /// `registered_worktree_roots` with `if false`.
    #[test]
    fn planted_worktree_registration_cannot_launder_a_nested_gitfile() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let (main, linked) = repo_with_linked_worktree(tmp.path());
        let nested = nested_secrets_dir(&main);

        // A self-consistent gitfile at `<repo>/secrets`, pointing at an admin
        // directory the attacker also owns.
        let fake_admin = nested.join("fake-git").join("worktrees").join("planted");
        std::fs::create_dir_all(&fake_admin).expect("planted admin");
        std::fs::write(fake_admin.join("commondir"), b"../..\n").expect("planted commondir");
        let git_file = nested.join(".git");
        std::fs::write(&git_file, format!("gitdir: {}\n", fake_admin.display()))
            .expect("planted gitfile");
        std::fs::write(fake_admin.join("gitdir"), format!("{}\n", git_file.display()))
            .expect("planted backlink");

        // The forged registration inside the REAL repository's admin area.
        let evil = main.join(".git").join("worktrees").join("evil");
        std::fs::create_dir_all(&evil).expect("forged admin");
        std::fs::write(evil.join("commondir"), b"../..\n").expect("forged commondir");
        std::fs::write(evil.join("gitdir"), format!("{}\n", git_file.display()))
            .expect("forged registration");

        let mut roots = AdmittedRoots::new_open();
        assert!(
            !is_graph_root(&nested),
            "an unvalidated worktree registration must not make nested secrets a checkout"
        );
        assert!(
            !roots.permits_graph_root(&nested),
            "first contact must refuse the laundered nested root"
        );
        roots.admit(&main).expect("admit parent");
        assert!(
            !roots.permits_graph_root(&nested),
            "the forged registration must not exempt nested secrets after admission"
        );
        // Over-refusal guard: the genuine registered linked worktree is
        // untouched by the stricter validation.
        assert!(
            roots.permits_graph_root(&linked),
            "a genuinely registered linked worktree stays authorised"
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
