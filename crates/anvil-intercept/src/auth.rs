//! DRVR-007: v1 driver trust boundary — manifest allowlist + workspace-root
//! validation.
//!
//! See `plans/specs/anvil-driver-framework/editor-and-mcp-driver-design.md`
//! §2.3a "Driver trust boundary (v1)" for the contract this module implements.
//!
//! What ships in v1:
//!
//! - [`is_driver_allowed`] — checks a driver binary path against a
//!   newline-delimited allowlist file (default location:
//!   `~/.config/anvil/drivers.allow`). Drivers requesting
//!   `capability.enforcementCandidate: true` MUST pass this gate before
//!   the daemon promotes them to `Participating`. Same-UID
//!   `SO_PEERCRED` is the floor; this is the next layer.
//! - [`DriverManifest::validate_workspace_roots`] — cross-checks the
//!   `workspaceRoots` claimed by a driver manifest against the live
//!   `SessionRecord` set (INTD-003). Three-way semantic per §2.3a:
//!   an empty claim is the "any-workspace observer" case (Ok); a
//!   non-empty claim with at least one matching session worktree is
//!   accepted (consumer drops the unmatched roots); a non-empty
//!   claim with **no** match is rejected with
//!   `AuthError::NoMatchingWorkspaceRoot` so the daemon refuses to
//!   silently attach a driver whose declared scope is empty against
//!   reality.
//!
//! Intentionally NOT in v1 (deferred):
//!
//! - The driver consumer that wires this API into the handshake. That
//!   is DRVR-001 (Wave 2). This crate ships the API and unit tests; no
//!   `lib.rs` consumer side-effect is added in this PR.
//! - Reliability-budget quarantine on stable identity. The trust
//!   boundary spec mandates the contract; the runtime ledger lands
//!   with DRVR-001.
//! - Daemon-side response redaction (§4.4). That is an MCP-driver
//!   filter wired by RMCPF-010; this module deliberately does not
//!   import the kernel-types diagnostic surface.
//!
//! `forbid(unsafe_code)` is inherited from the crate-level lint in
//! `lib.rs`. Path comparison uses `Path::canonicalize` only when both
//! sides exist on disk; the allowlist file is read as text and parsed
//! into owned `PathBuf`s so callers cannot smuggle un-validated paths
//! past the gate by re-using a borrowed slice.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anvil_intercept_proto::SessionRecord;
use anvil_intercept_proto::protocol::{
    ANVIL_ENFORCEMENT_ACK, ANVIL_GATE_REQUEST, ANVIL_PUBLISH_DIAGNOSTICS, ANVIL_SCAN_BUFFER,
    ANVIL_STATUS_QUERY, ANVIL_SUPPRESSION_APPLY, Capability,
};
use thiserror::Error;

/// Errors returned by the v1 driver trust boundary surface.
///
/// Wire-layer mapping (when DRVR-001 wires the consumer) is the
/// daemon's job. Keeping the error enum transport-agnostic lets the
/// auth module stay independent of JSON-RPC framing.
#[derive(Debug, Error)]
pub enum AuthError {
    /// The allowlist file could not be read. `path` is the file the
    /// caller asked us to consult; `source` carries the underlying io
    /// error. Distinct from a "file exists but driver is not on it"
    /// case (`DriverNotAllowed`) because the policy decision differs:
    /// missing allowlist closes the gate (no driver listed); unreadable
    /// allowlist surfaces as a hard error so an operator notices.
    #[error("failed to read driver allowlist {path:?}: {source}")]
    AllowlistUnreadable {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// The driver binary path could not be canonicalised. Same shape as
    /// [`RegistryError::WorktreePathInvalid`] in spirit: v1 refuses to
    /// match an allowlist entry against a path it cannot resolve to a
    /// concrete inode, because that is the only honest defence against
    /// `..`/symlink shenanigans on the request side.
    #[error("driver binary path could not be canonicalised: {path:?}: {source}")]
    DriverPathInvalid {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// The driver presented an empty `workspaceRoots` claim while
    /// requesting a capability that requires at least one claimed root
    /// (telemetry subscription scoping in particular). v1 refuses to
    /// auto-attach a driver to "all sessions" when the manifest is
    /// silent; the daemon would otherwise have no scope to apply.
    ///
    /// **Note:** v1 [`DriverManifest::validate_workspace_roots`]
    /// itself does NOT raise this — an empty `workspace_roots` vec
    /// models the "any-workspace observer" case the spec allows
    /// (§2.3a). This variant remains for callers that explicitly
    /// require a non-empty claim before promoting a capability; see
    /// [`AuthError::NoMatchingWorkspaceRoot`] for the "claimed but
    /// unmatched" case the validator actually returns.
    #[error("driver manifest claims no workspace roots")]
    NoWorkspaceRootsClaimed,

    /// The driver presented one or more `workspace_roots` claims, but
    /// **none** canonicalised to a path that matches any active
    /// session worktree. v1 refuses to attach the driver: the
    /// manifest stated a specific scope, and that scope is empty
    /// against the live session set, so the driver has nothing
    /// legitimate to operate on. Empty claim sets do NOT take this
    /// path (see [`AuthError::NoWorkspaceRootsClaimed`] above).
    ///
    /// `claimed` carries the original (pre-canonicalisation) paths so
    /// driver consumers can surface the rejection diagnostically; the
    /// bytes are echoed verbatim from the manifest, not normalised.
    #[error("driver manifest workspace_roots ({claimed:?}) match no active session worktree")]
    NoMatchingWorkspaceRoot { claimed: Vec<PathBuf> },
}

/// `PartialEq` is hand-written because [`io::Error`] is not `PartialEq`.
/// Equality compares the path and the io-error kind, which is what
/// tests actually need.
impl PartialEq for AuthError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::AllowlistUnreadable {
                    path: a,
                    source: ae,
                },
                Self::AllowlistUnreadable {
                    path: b,
                    source: be,
                },
            )
            | (
                Self::DriverPathInvalid {
                    path: a,
                    source: ae,
                },
                Self::DriverPathInvalid {
                    path: b,
                    source: be,
                },
            ) => a == b && ae.kind() == be.kind(),
            (Self::NoWorkspaceRootsClaimed, Self::NoWorkspaceRootsClaimed) => true,
            (
                Self::NoMatchingWorkspaceRoot { claimed: a },
                Self::NoMatchingWorkspaceRoot { claimed: b },
            ) => a == b,
            _ => false,
        }
    }
}

/// Resolve the default v1 driver allowlist path
/// (`~/.config/anvil/drivers.allow` on Unix, `%APPDATA%/anvil/drivers.allow`
/// on Windows). Tests inject an explicit path instead of calling this.
///
/// This helper exists so consumers (DRVR-001 / RMCPF) and operator
/// docs can reference one canonical location, but [`is_driver_allowed`]
/// itself takes the path as an argument so the auth module never
/// implicitly reaches into the operator's home directory.
///
/// Returns `None` rather than erroring on systems where neither
/// `XDG_CONFIG_HOME` / `HOME` nor `APPDATA` is set; callers decide
/// whether that is a hard failure or a "no allowlist configured"
/// signal.
#[must_use]
pub fn default_allowlist_path() -> Option<PathBuf> {
    let config_home = config_home_dir()?;
    Some(config_home.join("anvil").join("drivers.allow"))
}

#[cfg(unix)]
fn config_home_dir() -> Option<PathBuf> {
    if let Some(value) = std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(value));
    }
    let home = std::env::var_os("HOME").filter(|v| !v.is_empty())?;
    Some(PathBuf::from(home).join(".config"))
}

#[cfg(windows)]
fn config_home_dir() -> Option<PathBuf> {
    let appdata = std::env::var_os("APPDATA").filter(|v| !v.is_empty())?;
    Some(PathBuf::from(appdata))
}

#[cfg(not(any(unix, windows)))]
fn config_home_dir() -> Option<PathBuf> {
    None
}

/// Decide whether a driver binary is allowed to request enforcement
/// participation under the v1 trust boundary.
///
/// Returns:
///
/// - `Ok(true)` — the canonicalised `binary_path` matches a
///   canonicalised entry on the allowlist.
/// - `Ok(false)` — the allowlist is missing, empty, or contains no
///   entry matching `binary_path`. v1 closes the gate by default; a
///   missing file is treated as "no driver permitted to escalate".
/// - `Err(AuthError)` — the allowlist exists but cannot be read, or
///   the driver binary path cannot be canonicalised. Both are policy
///   decisions for the caller (typically: surface to the operator and
///   refuse promotion).
///
/// **Same-UID is not enough.** `SO_PEERCRED` confirms the connecting
/// process runs as the daemon's user; that is the floor (§2.3) and is
/// the responsibility of the IPC listener, not this function. The
/// allowlist is the next layer (§2.3a) and gates
/// `capability.enforcementCandidate: true`.
///
/// **Allowlist format (v1):** newline-delimited absolute paths. Lines
/// that are blank, whitespace-only, or start with `#` after trimming
/// are ignored (so operators can comment out entries). Paths that do
/// not exist on disk at evaluation time are skipped — they cannot
/// match anything and silently dropping them avoids surfacing
/// transient FS races as policy errors.
///
/// **Match policy:** equality on canonicalised paths. We refuse to
/// fall back to lexical comparison because `/usr/local/bin/anvil-vscode`
/// and `/usr/local/bin/../bin/anvil-vscode` would otherwise be treated
/// as distinct. v1 takes the strictest available comparison; v2+ may
/// add fingerprint / signature checks alongside.
pub fn is_driver_allowed(binary_path: &Path, allowlist: &Path) -> Result<bool, AuthError> {
    let allowlist_contents = match fs::read_to_string(allowlist) {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            // Missing allowlist == nothing permitted.
            return Ok(false);
        }
        Err(err) => {
            return Err(AuthError::AllowlistUnreadable {
                path: allowlist.to_path_buf(),
                source: err,
            });
        }
    };

    let canonical_driver =
        binary_path
            .canonicalize()
            .map_err(|err| AuthError::DriverPathInvalid {
                path: binary_path.to_path_buf(),
                source: err,
            })?;

    let mut allowed = HashSet::new();
    for raw in allowlist_contents.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let entry = PathBuf::from(trimmed);
        // Skip entries that do not resolve. Treating a missing entry
        // as "match" would invert the gate; treating it as a hard
        // error would let one stale operator entry block every
        // driver. Skipping is the only safe choice.
        if let Ok(canonical) = entry.canonicalize() {
            allowed.insert(canonical);
        }
    }

    Ok(allowed.contains(&canonical_driver))
}

/// Driver manifest workspace-roots claim, as carried by the §2.2
/// manifest. v1 cross-checks each claimed root against the active
/// session set: a non-empty claim where no root matches any session
/// is rejected (`AuthError::NoMatchingWorkspaceRoot`); a non-empty
/// claim with at least one match is accepted with the unmatched
/// roots dropped by the consumer; an empty claim is the
/// "any-workspace observer" path the spec allows. See
/// [`DriverManifest::validate_workspace_roots`] for the full
/// three-way semantic.
///
/// In Wave 3 (DRVR-008) the manifest also carries
/// [`Self::supported_anvil_methods`] — the list of `anvil/` JSON-RPC
/// methods this driver implements. Drivers that omit a method from
/// the list are advertising "I do not understand this method"; the
/// daemon caps such drivers at [`Capability::Attached`] and emits a
/// [`CapabilityDowngrade`] event so the operator and the driver both
/// see why enforcement was refused. Stock LSP clients (Neovim, Zed,
/// Helix) that connect without speaking the Anvil namespace satisfy
/// this contract by sending an empty list — they get diagnostics for
/// free without being silently fenced for missing
/// `anvil/enforcement/ack`.
///
/// We do not import the full §2.2 manifest type into this crate to
/// keep the dependency surface small — the trust boundary cares
/// about exactly two fields (workspace roots + supported methods).
/// DRVR-001 (Wave 2) owns the full `DriverManifest` decoder; this is
/// the v1 slice the daemon needs to run the workspace-root validation
/// and capability-negotiation contracts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverManifest {
    /// Absolute paths the driver claims it operates on.
    ///
    /// **Empty** models the §2.3a "any-workspace observer" case
    /// (e.g. a diagnostic-only sidecar that subscribes to telemetry
    /// without binding to a specific worktree). The
    /// [`DriverManifest::validate_workspace_roots`] contract returns
    /// `Ok(())` for this case; capability scoping is enforced at the
    /// negotiation layer instead.
    ///
    /// **Non-empty** declares a specific scope: at least one path
    /// MUST canonicalise and match an active session worktree, or
    /// the validator returns
    /// [`AuthError::NoMatchingWorkspaceRoot`]. The legacy
    /// [`AuthError::NoWorkspaceRootsClaimed`] variant remains for
    /// callers (outside the validator itself) that explicitly require
    /// a non-empty claim before promoting capability.
    pub workspace_roots: Vec<PathBuf>,

    /// `anvil/` JSON-RPC method names this driver advertises support
    /// for. Use the `ANVIL_*` constants in
    /// `anvil_intercept_proto::protocol`; arbitrary strings are
    /// accepted at this layer so the wire format can evolve without
    /// the auth API blocking unknown future methods.
    ///
    /// **Default-deny semantics (DRVR-008):** a driver requesting
    /// [`Capability::Participating`] but missing
    /// `anvil/enforcement/ack` from this list is automatically capped
    /// at [`Capability::Attached`]. The `.anvil.yaml` workspace
    /// config has no power to override this — capability promotion
    /// requires the method present in the advertised list.
    pub supported_anvil_methods: Vec<String>,
}

impl DriverManifest {
    /// Build a manifest from a roots list. Path canonicalisation is
    /// deferred to [`Self::validate_workspace_roots`] so callers can
    /// hand off raw inputs (e.g. JSON-decoded paths) without first
    /// touching the filesystem.
    ///
    /// Use [`Self::with_supported_anvil_methods`] (or assign directly)
    /// to populate the DRVR-008 method-advertisement list. The default
    /// is an empty list, which models a stock LSP client that does
    /// not speak the `anvil/` namespace at all.
    #[must_use]
    pub fn new(workspace_roots: Vec<PathBuf>) -> Self {
        Self {
            workspace_roots,
            supported_anvil_methods: Vec::new(),
        }
    }

    /// Builder helper: replace the supported-methods list. Convenience
    /// for tests and for the DRVR-001 manifest decoder; the daemon
    /// reads the field directly when negotiating capability.
    #[must_use]
    pub fn with_supported_anvil_methods<I, S>(mut self, methods: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.supported_anvil_methods = methods.into_iter().map(Into::into).collect();
        self
    }

    /// True iff the manifest advertises `method` in
    /// [`Self::supported_anvil_methods`]. Comparison is exact-match;
    /// callers MUST use the constants from
    /// `anvil_intercept_proto::protocol` rather than re-typing the
    /// strings.
    #[must_use]
    pub fn advertises(&self, method: &str) -> bool {
        self.supported_anvil_methods.iter().any(|m| m == method)
    }

    /// Cross-check the manifest's `workspace_roots` against the live
    /// session set.
    ///
    /// Three-way semantic per §2.3a:
    ///
    /// 1. **Empty claim** (`workspace_roots: []`) — the driver has
    ///    declared itself an "any-workspace observer" (e.g. a
    ///    diagnostic-only sidecar that subscribes to telemetry across
    ///    every active worktree). v1 returns `Ok(())` and lets the
    ///    daemon's capability negotiator scope the subscription
    ///    elsewhere.
    /// 2. **Non-empty claim, at least one match** — at least one
    ///    claimed path canonicalises to a path that equals a
    ///    canonicalised session worktree. v1 returns `Ok(())`; roots
    ///    that did not match are dropped from the effective scope by
    ///    the consumer (DRVR-001), surfaced to the driver as a
    ///    "downgrade to read-only observer of the matched subset"
    ///    event.
    /// 3. **Non-empty claim, zero matches** — every claimed path
    ///    either fails to canonicalise or canonicalises to a path
    ///    that no active session knows about. v1 returns
    ///    `Err(AuthError::NoMatchingWorkspaceRoot { claimed })` so
    ///    the daemon refuses the attach: the driver named a specific
    ///    scope and that scope is empty against reality.
    ///
    /// The pre-`v0.6.0-beta` implementation discarded the boolean
    /// match result and accepted any non-empty manifest, which let a
    /// driver claim arbitrary unrelated paths and still attach. The
    /// new contract closes that gap without changing the empty-claim
    /// fall-through, which the spec deliberately permits.
    ///
    /// `claimed` paths in the error variant are the **pre-canonical**
    /// values from the manifest, returned verbatim so a driver author
    /// can debug the rejection without guessing what the daemon
    /// canonicalised them to.
    ///
    /// Future direction: DRVR-001's handshake response will upgrade
    /// the success path to return the matched/dropped sets directly
    /// so a driver can render its effective scope in real time.
    pub fn validate_workspace_roots(&self, sessions: &[SessionRecord]) -> Result<(), AuthError> {
        if self.workspace_roots.is_empty() {
            // §2.3a "any-workspace observer" — the driver opted out
            // of declaring a specific scope. The daemon enforces
            // scoping at the capability layer instead; this validator
            // does not block.
            return Ok(());
        }

        // Canonicalise the session worktrees once. Sessions whose
        // worktree path no longer canonicalises (race against
        // worktree deletion) are skipped — they cannot be active
        // attach targets.
        let mut session_roots: HashSet<PathBuf> = HashSet::new();
        for record in sessions {
            if let Ok(canonical) = record.worktree.canonicalize() {
                session_roots.insert(canonical);
            }
        }

        // Walk the claim set; success requires at least one
        // canonicalised match. A claimed path that fails to
        // canonicalise (`Err` return from `canonicalize`) is treated
        // identically to one that canonicalises to a non-matching
        // path: it cannot be an honest attach target. Both paths
        // contribute to the count-of-matches even though neither
        // can succeed — there is no leak between the two cases.
        let any_match = self
            .workspace_roots
            .iter()
            .filter_map(|claimed| claimed.canonicalize().ok())
            .any(|canonical| session_roots.contains(&canonical));

        if any_match {
            Ok(())
        } else {
            Err(AuthError::NoMatchingWorkspaceRoot {
                claimed: self.workspace_roots.clone(),
            })
        }
    }
}

/// Reasons a [`Capability::Participating`] request can be downgraded
/// at handshake time. Each variant maps to a structured warning the
/// daemon surfaces back to the driver and emits to its log so an
/// operator can see why their `.anvil.yaml`-mandated enforcement was
/// refused.
///
/// The variants are deliberately specific — the operator-facing
/// remediation is different for each (allowlist vs missing method
/// vs absent capability advertisement). Collapsing them into a single
/// "downgraded" string would force the operator back into the daemon
/// log to figure out which fix to apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityDowngradeReason {
    /// The driver advertised `enforcement_candidate: false` (or
    /// equivalent in §2.2 manifest terms). The handshake never asked
    /// for participation; the daemon honours that.
    NotEnforcementCandidate,
    /// The driver requested participation but did not advertise
    /// [`ANVIL_ENFORCEMENT_ACK`] in
    /// [`DriverManifest::supported_anvil_methods`]. DRVR-008's central
    /// case: a stock LSP client cannot be silently fenced for missing
    /// a method it never claimed to implement.
    MissingEnforcementAckMethod,
}

impl CapabilityDowngradeReason {
    /// Stable wire string for log / telemetry emission. Kebab-case to
    /// match the rest of the daemon's structured-log vocabulary.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::NotEnforcementCandidate => "not-enforcement-candidate",
            Self::MissingEnforcementAckMethod => "missing-enforcement-ack-method",
        }
    }
}

/// Structured event the daemon emits when a driver's requested
/// capability is downgraded at handshake. Sibling consumers
/// (DRVR-001, RMCPF) surface this back to the driver alongside the
/// accepted capability so the operator sees a clear "enforcement
/// requested but downgraded because <reason>" message rather than a
/// silent demotion.
///
/// `negotiated` is always less-than-or-equal-to `requested` per the
/// capability lattice: v1 only ever downgrades, never promotes,
/// regardless of operator request. This is the contract DRVR-008
/// hardens: a stock LSP client (no `anvil/` methods) connecting to a
/// project with `enforcement_required: true` in `.anvil.yaml` cannot
/// be promoted to [`Capability::Participating`] regardless of
/// configuration; the manifest is the floor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityDowngrade {
    /// What the driver / config asked for at handshake.
    pub requested: Capability,
    /// What the daemon actually granted.
    pub negotiated: Capability,
    /// Why the downgrade fired. See [`CapabilityDowngradeReason`].
    pub reason: CapabilityDowngradeReason,
    /// Methods the driver advertised. Captured at downgrade time so
    /// the log entry is reproducible without re-reading the manifest;
    /// this is what the operator inspects when triaging.
    pub advertised_methods: Vec<String>,
}

/// Negotiate a driver's capability against its manifest.
///
/// **Contract (DRVR-008):**
///
/// - If `requested` is [`Capability::Attached`] (or weaker), the result
///   is `(Attached, None)` — no negotiation needed; read-only is the
///   default and is always available.
/// - If `requested` is [`Capability::Participating`] but the manifest
///   does not advertise [`ANVIL_ENFORCEMENT_ACK`], the result is
///   `(Attached, Some(downgrade))` with reason
///   [`CapabilityDowngradeReason::MissingEnforcementAckMethod`]. The
///   daemon caps the driver at read-only and surfaces the structured
///   warning in the downgrade event.
/// - If `requested` is [`Capability::Participating`] AND
///   [`ANVIL_ENFORCEMENT_ACK`] is advertised, the result is
///   `(Participating, None)`. (DRVR-007's allowlist gate is a
///   *separate* layer that the caller must ALSO satisfy via
///   [`is_driver_allowed`]; this function does not consult the
///   allowlist because it operates on the manifest claim alone.)
///
/// **Why the manifest, not `.anvil.yaml`, is the floor:** an LSP
/// client speaking only stock LSP cannot honour
/// `anvil/enforcement/ack`. If the workspace config could override
/// the manifest, a team-mandated enforcement policy would silently
/// fence Neovim users whose plugins do not implement the namespace.
/// The §2.2 manifest is what the driver itself signed up for; it is
/// the tightest authentic source. `.anvil.yaml` decides *whether* to
/// request enforcement from drivers that can support it.
///
/// **Reconnect survival:** the negotiation reads only the manifest
/// (an in-memory clone) and method-name constants. There is no
/// daemon-side state for the negotiation result; a reconnecting
/// driver MUST re-present its manifest, and the daemon MUST re-run
/// this function. A driver cannot smuggle a stale `Participating`
/// capability across reconnects by relying on the daemon to remember
/// the previous handshake. This is the property that survives
/// daemon restart per the §3.3 capability state machine.
#[must_use]
pub fn negotiate_capability(
    requested: Capability,
    manifest: &DriverManifest,
) -> (Capability, Option<CapabilityDowngrade>) {
    match requested {
        // Attached is always available.
        Capability::Attached => (Capability::Attached, None),
        Capability::Participating => {
            if manifest.advertises(ANVIL_ENFORCEMENT_ACK) {
                (Capability::Participating, None)
            } else {
                let downgrade = CapabilityDowngrade {
                    requested: Capability::Participating,
                    negotiated: Capability::Attached,
                    reason: CapabilityDowngradeReason::MissingEnforcementAckMethod,
                    advertised_methods: manifest.supported_anvil_methods.clone(),
                };
                emit_downgrade_log(&downgrade);
                (Capability::Attached, Some(downgrade))
            }
        }
    }
}

/// Emit a structured `tracing` event for an operator log when a
/// downgrade fires. The event is at `WARN` because the operator
/// configured enforcement and is not getting it; INFO would be too
/// quiet for the actual policy gap, ERROR is reserved for failures
/// the daemon could not work around.
fn emit_downgrade_log(downgrade: &CapabilityDowngrade) {
    tracing::warn!(
        target: "anvil_intercept::auth",
        requested = %downgrade.requested.as_str(),
        negotiated = %downgrade.negotiated.as_str(),
        reason = %downgrade.reason.as_str(),
        advertised_method_count = downgrade.advertised_methods.len(),
        "driver capability downgraded at handshake (DRVR-008)",
    );
}

/// Constants re-exported for callers who only need the
/// `anvil/`-method names. Mirrors the `protocol` module in
/// `anvil-intercept-proto` without forcing every consumer of
/// [`DriverManifest`] to import it twice. The constants stay
/// authoritative in the proto crate; this re-export is convenience
/// only.
pub mod methods {
    pub use anvil_intercept_proto::protocol::{
        ANVIL_ENFORCEMENT_ACK, ANVIL_GATE_REQUEST, ANVIL_PUBLISH_DIAGNOSTICS, ANVIL_SCAN_BUFFER,
        ANVIL_STATUS_QUERY, ANVIL_SUPPRESSION_APPLY,
    };
}

// Compile-time assertion: keep the `methods` re-exports in lockstep
// with the imports. If a method moves out of the proto crate this
// fails to compile; if a new method is added it should be added in
// both places.
const _: &str = ANVIL_ENFORCEMENT_ACK;
const _: &str = ANVIL_GATE_REQUEST;
const _: &str = ANVIL_PUBLISH_DIAGNOSTICS;
const _: &str = ANVIL_SCAN_BUFFER;
const _: &str = ANVIL_STATUS_QUERY;
const _: &str = ANVIL_SUPPRESSION_APPLY;

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::io::Write;

    use anvil_intercept_proto::{SessionId, SessionRecord, SessionStatus};
    use tempfile::TempDir;

    use super::*;

    /// Helper: build a `SessionRecord` for the given worktree, with
    /// fixed timestamps so equality checks are reproducible.
    fn session_for(worktree: &Path, id: &str) -> SessionRecord {
        SessionRecord {
            id: SessionId::new(id),
            worktree: worktree.to_path_buf(),
            pid: None,
            pgid: None,
            started_at_unix: 1_700_000_000,
            last_heartbeat_unix: 1_700_000_010,
            status: SessionStatus::Active,
            agent_tag: None,
            daemon_issued_tag: None,
        }
    }

    /// Helper: write `lines` to `path`, joined by `\n` with a trailing
    /// newline. Mirrors the v1 wire format expectation (newline-
    /// delimited paths, optional comments).
    fn write_allowlist(path: &Path, lines: &[&str]) {
        let mut file = File::create(path).expect("create allowlist");
        for line in lines {
            writeln!(file, "{line}").expect("write line");
        }
    }

    #[test]
    fn allowlisted_binary_is_allowed() {
        let tmp = TempDir::new().unwrap();
        let driver_bin = tmp.path().join("anvil-vscode");
        File::create(&driver_bin).expect("create driver bin");
        let allowlist = tmp.path().join("drivers.allow");
        write_allowlist(&allowlist, &[driver_bin.to_str().unwrap()]);

        let allowed =
            is_driver_allowed(&driver_bin, &allowlist).expect("allowlist read should succeed");
        assert!(allowed, "driver binary on allowlist must be allowed");
    }

    #[test]
    fn driver_not_on_allowlist_is_refused() {
        let tmp = TempDir::new().unwrap();
        let driver_bin = tmp.path().join("anvil-vscode");
        let other_bin = tmp.path().join("not-anvil");
        File::create(&driver_bin).expect("create driver bin");
        File::create(&other_bin).expect("create other bin");
        let allowlist = tmp.path().join("drivers.allow");
        write_allowlist(&allowlist, &[other_bin.to_str().unwrap()]);

        let allowed = is_driver_allowed(&driver_bin, &allowlist).expect("read should succeed");
        assert!(
            !allowed,
            "driver binary not on allowlist must be refused (default deny)"
        );
    }

    #[test]
    fn missing_allowlist_closes_gate() {
        let tmp = TempDir::new().unwrap();
        let driver_bin = tmp.path().join("anvil-vscode");
        File::create(&driver_bin).expect("create driver bin");
        // Note: allowlist file does NOT exist.
        let allowlist = tmp.path().join("drivers.allow");

        let allowed =
            is_driver_allowed(&driver_bin, &allowlist).expect("missing allowlist must not error");
        assert!(
            !allowed,
            "missing allowlist closes the gate; v1 default deny",
        );
    }

    #[test]
    fn unreadable_allowlist_surfaces_error() {
        let tmp = TempDir::new().unwrap();
        let driver_bin = tmp.path().join("anvil-vscode");
        File::create(&driver_bin).expect("create driver bin");
        // Use the tempdir itself as the "allowlist path" — read_to_string
        // on a directory returns an error other than NotFound, which is
        // exactly the surface we test here.
        let allowlist_path = tmp.path().to_path_buf();

        let err = is_driver_allowed(&driver_bin, &allowlist_path)
            .expect_err("reading a directory as allowlist must error");
        assert!(matches!(err, AuthError::AllowlistUnreadable { .. }));
    }

    #[test]
    fn driver_path_invalid_when_binary_does_not_exist() {
        let tmp = TempDir::new().unwrap();
        let allowlist = tmp.path().join("drivers.allow");
        write_allowlist(&allowlist, &["/usr/bin/anvil-vscode"]);
        // Driver bin path does not exist on disk.
        let driver_bin = tmp.path().join("missing-driver");

        let err = is_driver_allowed(&driver_bin, &allowlist)
            .expect_err("nonexistent driver path must error");
        assert!(matches!(err, AuthError::DriverPathInvalid { .. }));
    }

    #[test]
    fn allowlist_skips_blanks_and_comments() {
        let tmp = TempDir::new().unwrap();
        let driver_bin = tmp.path().join("anvil-vscode");
        File::create(&driver_bin).expect("create driver bin");
        let allowlist = tmp.path().join("drivers.allow");
        write_allowlist(
            &allowlist,
            &[
                "# anvil drivers v1",
                "",
                "   # comment with leading whitespace",
                driver_bin.to_str().unwrap(),
                "",
            ],
        );

        let allowed = is_driver_allowed(&driver_bin, &allowlist).expect("read");
        assert!(
            allowed,
            "blank lines and comments must not block a real entry"
        );
    }

    #[test]
    fn allowlist_canonicalises_entries_for_matching() {
        let tmp = TempDir::new().unwrap();
        let nested = tmp.path().join("nested");
        fs::create_dir(&nested).expect("create nested");
        let driver_bin = nested.join("anvil-vscode");
        File::create(&driver_bin).expect("create driver bin");

        // Allowlist entry uses an unnormalised path traversal — canonicalisation must collapse it.
        let allowlist = tmp.path().join("drivers.allow");
        let traversal = format!("{}/../nested/anvil-vscode", nested.display());
        write_allowlist(&allowlist, &[&traversal]);

        let allowed = is_driver_allowed(&driver_bin, &allowlist).expect("read");
        assert!(
            allowed,
            "canonicalised allowlist entry must match canonicalised driver path",
        );
    }

    #[test]
    fn allowlist_skips_nonexistent_entries() {
        let tmp = TempDir::new().unwrap();
        let driver_bin = tmp.path().join("anvil-vscode");
        File::create(&driver_bin).expect("create driver bin");
        let allowlist = tmp.path().join("drivers.allow");
        write_allowlist(
            &allowlist,
            &[
                "/nonexistent/path/that/will/never/exist",
                driver_bin.to_str().unwrap(),
            ],
        );

        let allowed = is_driver_allowed(&driver_bin, &allowlist).expect("read");
        assert!(allowed, "stale allowlist entries must not block live ones");
    }

    #[test]
    fn manifest_with_zero_claimed_roots_validates_against_any_session() {
        // §2.3a "any-workspace observer" — a manifest that declines
        // to declare a specific scope is the spec's diagnostic-only
        // sidecar case (e.g. a telemetry mirror). The validator must
        // return Ok(()); capability scoping is enforced elsewhere.
        // This is the inverse of the previous v1 hard-error contract,
        // which incorrectly rejected the legitimate empty-claim path.
        let manifest = DriverManifest::new(vec![]);

        manifest
            .validate_workspace_roots(&[])
            .expect("empty claim is the any-workspace observer case — must Ok");
    }

    #[test]
    fn manifest_with_zero_claimed_roots_validates_against_live_sessions() {
        // Same any-workspace observer semantic, but with live
        // sessions present. The match check is skipped entirely when
        // the claim set is empty.
        let tmp = TempDir::new().unwrap();
        let worktree = tmp.path().join("workspace");
        fs::create_dir(&worktree).expect("create worktree");
        let session = session_for(&worktree, "sess-1");
        let manifest = DriverManifest::new(vec![]);

        manifest
            .validate_workspace_roots(&[session])
            .expect("empty claim with live sessions must still Ok");
    }

    #[test]
    fn manifest_with_matching_root_validates() {
        let tmp = TempDir::new().unwrap();
        let worktree = tmp.path().join("workspace");
        fs::create_dir(&worktree).expect("create worktree");
        let session = session_for(&worktree, "sess-1");
        let manifest = DriverManifest::new(vec![worktree.clone()]);

        manifest
            .validate_workspace_roots(&[session])
            .expect("matching root must validate");
    }

    #[test]
    fn manifest_with_only_unknown_roots_returns_error() {
        // Contract change in v0.6.0-beta: a non-empty claim where
        // every claimed root either fails to canonicalise or matches
        // no active session is rejected. The pre-fix code returned
        // Ok(()) here (discarding the boolean match result), which
        // let a hostile or buggy driver claim arbitrary unrelated
        // paths and still attach.
        let tmp = TempDir::new().unwrap();
        let real_worktree = tmp.path().join("workspace");
        let bogus_worktree = tmp.path().join("not-a-workspace");
        fs::create_dir(&real_worktree).expect("create worktree");
        let session = session_for(&real_worktree, "sess-1");
        let manifest = DriverManifest::new(vec![bogus_worktree.clone()]);

        let err = manifest
            .validate_workspace_roots(&[session])
            .expect_err("unknown-only claim must error");
        match err {
            AuthError::NoMatchingWorkspaceRoot { claimed } => {
                assert_eq!(
                    claimed,
                    vec![bogus_worktree],
                    "error must echo the original (pre-canonical) claim",
                );
            }
            other => panic!("expected NoMatchingWorkspaceRoot, got {other:?}"),
        }
    }

    #[test]
    fn manifest_with_partial_match_validates_dropping_the_unknown() {
        // Non-empty claim with one matching root and one bogus root
        // is accepted: the consumer (DRVR-001) drops the bogus path
        // and downgrades the effective scope. This pins the
        // "at least one match" semantic so a driver listing an
        // optional sub-worktree path that does not exist on every
        // host is not bounced.
        let tmp = TempDir::new().unwrap();
        let real_worktree = tmp.path().join("workspace");
        let bogus_worktree = tmp.path().join("not-a-workspace");
        fs::create_dir(&real_worktree).expect("create worktree");
        let session = session_for(&real_worktree, "sess-1");
        let manifest = DriverManifest::new(vec![real_worktree.clone(), bogus_worktree]);

        manifest
            .validate_workspace_roots(&[session])
            .expect("partial match must validate (consumer drops the unknown)");
    }

    #[test]
    fn manifest_with_only_unknown_roots_returns_error_with_empty_session_list() {
        // Symmetric with the unknown-only case above: when no
        // sessions exist, every claimed root is unmatched by
        // definition. Earlier code accepted this as
        // "non-empty manifest with no live sessions"; the new
        // contract treats it as an explicit reject, because the
        // driver named a scope that does not exist anywhere.
        let tmp = TempDir::new().unwrap();
        let claimed_worktree = tmp.path().join("workspace");
        fs::create_dir(&claimed_worktree).expect("create worktree");
        let manifest = DriverManifest::new(vec![claimed_worktree.clone()]);

        let err = manifest
            .validate_workspace_roots(&[])
            .expect_err("non-empty manifest with no live sessions must error");
        match err {
            AuthError::NoMatchingWorkspaceRoot { claimed } => {
                assert_eq!(claimed, vec![claimed_worktree]);
            }
            other => panic!("expected NoMatchingWorkspaceRoot, got {other:?}"),
        }
    }

    // -------- DRVR-008 capability negotiation tests --------
    //
    // The fixtures below deliberately use stub `DriverManifest`s. The
    // daemon-minted driver identity (`originating_driver_id` from
    // INTD-015) is the input to the callers that build a manifest;
    // the manifest itself stays self-contained at this layer so the
    // negotiation function is unit-testable. The reconnect-survival
    // property is exercised by `negotiate_capability_is_pure_recompute`.

    #[test]
    fn negotiate_capability_attached_is_always_granted() {
        // Every successful handshake reaches Attached; this is the
        // §3.3 read-only floor. Even an empty manifest gets it.
        let manifest = DriverManifest::new(vec![PathBuf::from("/tmp/wt")]);
        let (granted, downgrade) = negotiate_capability(Capability::Attached, &manifest);
        assert_eq!(granted, Capability::Attached);
        assert!(
            downgrade.is_none(),
            "attached request never produces a downgrade event"
        );
    }

    #[test]
    fn negotiate_capability_participating_with_ack_is_granted() {
        // Driver advertises enforcement/ack; daemon promotes.
        let manifest = DriverManifest::new(vec![PathBuf::from("/tmp/wt")])
            .with_supported_anvil_methods([
                ANVIL_PUBLISH_DIAGNOSTICS,
                ANVIL_ENFORCEMENT_ACK,
                ANVIL_SCAN_BUFFER,
            ]);
        let (granted, downgrade) = negotiate_capability(Capability::Participating, &manifest);
        assert_eq!(granted, Capability::Participating);
        assert!(
            downgrade.is_none(),
            "honoured promotion does not emit a downgrade event"
        );
    }

    #[test]
    fn negotiate_capability_downgrades_when_ack_method_missing() {
        // Stock LSP client that connects without the `anvil/`
        // namespace: the manifest's supported methods list is empty.
        // A request to participate must be capped at Attached and
        // surface a structured warning.
        let manifest = DriverManifest::new(vec![PathBuf::from("/tmp/wt")]);
        let (granted, downgrade) = negotiate_capability(Capability::Participating, &manifest);
        assert_eq!(granted, Capability::Attached);
        let downgrade = downgrade.expect("downgrade event must fire");
        assert_eq!(downgrade.requested, Capability::Participating);
        assert_eq!(downgrade.negotiated, Capability::Attached);
        assert_eq!(
            downgrade.reason,
            CapabilityDowngradeReason::MissingEnforcementAckMethod
        );
        assert!(
            downgrade.advertised_methods.is_empty(),
            "advertised methods captured at downgrade time"
        );
    }

    #[test]
    fn negotiate_capability_downgrades_when_ack_advertised_via_unrelated_methods() {
        // The driver advertises some `anvil/` methods but NOT
        // enforcement/ack — same downgrade as the empty case. This
        // is the M10 council finding: the daemon must not rely on a
        // partial advertisement to imply enforcement support.
        let manifest = DriverManifest::new(vec![PathBuf::from("/tmp/wt")])
            .with_supported_anvil_methods([
                ANVIL_PUBLISH_DIAGNOSTICS,
                ANVIL_SCAN_BUFFER,
                ANVIL_STATUS_QUERY,
            ]);
        let (granted, downgrade) = negotiate_capability(Capability::Participating, &manifest);
        assert_eq!(granted, Capability::Attached);
        let downgrade = downgrade.expect("downgrade event must fire");
        assert_eq!(
            downgrade.reason,
            CapabilityDowngradeReason::MissingEnforcementAckMethod
        );
        // Surface the exact advertised set in the event so the
        // operator-facing log can name what the driver claimed.
        assert_eq!(
            downgrade.advertised_methods,
            vec![
                ANVIL_PUBLISH_DIAGNOSTICS.to_string(),
                ANVIL_SCAN_BUFFER.to_string(),
                ANVIL_STATUS_QUERY.to_string(),
            ]
        );
    }

    #[test]
    fn negotiate_capability_is_pure_recompute() {
        // Reconnect-survival contract (DRVR-008): the negotiation
        // function is a pure function of (request, manifest). Two
        // calls with the same inputs produce the same outputs, so a
        // reconnecting driver cannot smuggle a stale `Participating`
        // capability across a reconnect — the daemon recomputes from
        // the freshly-presented manifest each time.
        let manifest = DriverManifest::new(vec![PathBuf::from("/tmp/wt")]);
        let (granted_a, downgrade_a) = negotiate_capability(Capability::Participating, &manifest);
        let (granted_b, downgrade_b) = negotiate_capability(Capability::Participating, &manifest);
        assert_eq!(granted_a, granted_b);
        assert_eq!(downgrade_a, downgrade_b);

        // Now mutate the manifest to advertise the method and
        // confirm the second call produces a different result. This
        // pins the property that the function reads ONLY the
        // manifest passed in — no hidden daemon-side state.
        let promoted = manifest
            .clone()
            .with_supported_anvil_methods([ANVIL_ENFORCEMENT_ACK]);
        let (granted_c, downgrade_c) = negotiate_capability(Capability::Participating, &promoted);
        assert_eq!(granted_c, Capability::Participating);
        assert!(downgrade_c.is_none());
    }

    #[test]
    fn negotiate_capability_attached_request_ignores_advertised_methods() {
        // A driver that asked for read-only is granted read-only
        // regardless of what it advertises — no upgrade ever happens
        // implicitly. v1's lattice only ever downgrades.
        let manifest = DriverManifest::new(vec![PathBuf::from("/tmp/wt")])
            .with_supported_anvil_methods([ANVIL_ENFORCEMENT_ACK]);
        let (granted, downgrade) = negotiate_capability(Capability::Attached, &manifest);
        assert_eq!(granted, Capability::Attached);
        assert!(downgrade.is_none());
    }

    #[test]
    fn driver_manifest_advertises_returns_true_only_for_listed_methods() {
        // Pin the method-name comparison policy: exact-match string
        // equality. A driver advertising `anvil/enforcement` (without
        // the `/ack` suffix) must NOT count as advertising
        // `anvil/enforcement/ack`, otherwise an over-eager driver
        // could fake support.
        let manifest = DriverManifest::new(vec![PathBuf::from("/tmp/wt")])
            .with_supported_anvil_methods(["anvil/enforcement"]);
        assert!(!manifest.advertises(ANVIL_ENFORCEMENT_ACK));
        assert!(manifest.advertises("anvil/enforcement"));
    }

    #[test]
    fn driver_manifest_supported_anvil_methods_defaults_empty() {
        // Newly-minted manifests advertise nothing. DRVR-001's
        // decoder will populate the list from §2.2 manifest input;
        // the constructor default is the safe-deny case.
        let manifest = DriverManifest::new(vec![PathBuf::from("/tmp/wt")]);
        assert!(manifest.supported_anvil_methods.is_empty());
    }

    #[test]
    fn capability_downgrade_reason_strings_are_kebab_case() {
        // Pin the wire vocabulary so an operator-facing log entry
        // doesn't drift from `missing-enforcement-ack-method` to
        // `MissingEnforcementAck` after a future refactor.
        assert_eq!(
            CapabilityDowngradeReason::NotEnforcementCandidate.as_str(),
            "not-enforcement-candidate"
        );
        assert_eq!(
            CapabilityDowngradeReason::MissingEnforcementAckMethod.as_str(),
            "missing-enforcement-ack-method"
        );
    }

    #[test]
    fn default_allowlist_path_returns_some_when_env_present() {
        // We don't assert the exact value (depends on platform / env)
        // but on a sane test harness at least HOME/APPDATA is set,
        // so the helper should resolve.
        let resolved = default_allowlist_path();
        if std::env::var_os("HOME").is_some() || std::env::var_os("APPDATA").is_some() {
            assert!(resolved.is_some(), "expected resolvable config home");
            let path = resolved.unwrap();
            assert!(path.ends_with("drivers.allow"));
            assert!(path.to_string_lossy().contains("anvil"));
        }
    }
}
