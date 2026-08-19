//! Capsule directory writer (GITGOV-004): assemble collected evidence
//! into the ADR-074 file-first layout with a digest-complete manifest.
//!
//! The writer owns the **placeholder discipline** for evidence whose
//! collectors have not landed yet: every ADR-074 file is written
//! (present-but-empty, never omitted), so a missing file is always a
//! tamper signal and never "not collected yet". `verification.json`
//! is the degraded no-checks placeholder — an unverified capsule
//! carries a machine-readable `degraded`, never silence.

use std::path::Path;
#[cfg(unix)]
use std::path::PathBuf;
#[cfg(unix)]
use std::sync::atomic::{AtomicU64, Ordering};

use anvil_checks::secret::{SecretCheckConfig, scan_content_with_compiled_patterns};

use crate::collect::CommitsDocument;
use crate::collect_diagnostics::CollectedDiagnostics;
use crate::collect_digests::CollectedDigests;
use crate::collect_exceptions::CollectedExceptions;
use crate::collect_witness::CollectedWitness;
use crate::errors::CapsuleError;
use crate::manifest::{CapsuleManifest, CapsuleRange, Producer};
use crate::verification::CapsuleVerification;

/// Everything the writer needs to assemble a capsule directory.
#[derive(Debug, Clone)]
pub struct CapsuleContent {
    /// The collected commit range (GITGOV-005).
    pub commits: CommitsDocument,
    /// The collected digest documents (GITGOV-006).
    pub digests: CollectedDigests,
    /// The collected witness chain + range window (GITGOV-007).
    pub witness: CollectedWitness,
    /// The rendered SARIF diagnostics document (GITGOV-008).
    pub diagnostics: CollectedDiagnostics,
    /// The collected active exception grants (EXCEPT-009).
    pub exceptions: CollectedExceptions,
    /// Producer identity recorded in the manifest.
    pub producer: Producer,
}

/// Write a complete capsule directory at `out_dir` and return the
/// manifest that was written.
///
/// `out_dir` is created if missing; an existing **non-empty**
/// directory is refused so a capsule can never silently mix with (or
/// partially overwrite) prior content. Files are written into a
/// private sibling directory and the completed capsule is published
/// with one rename, so a concurrent final-path swap cannot redirect
/// any evidence write.
///
/// `witness.ndjson` carries the verbatim full witness chain
/// (GITGOV-007); the manifest range's `witness_seq_start`/`_end` mark
/// the PR-relevant window into it. An empty chain (fresh-adoption repo)
/// is present-but-empty — valid NDJSON with no lines, which the
/// verifier reads as missing witness evidence → `degraded`.
///
/// `diagnostics.sarif` is rendered by the GITGOV-008 collector via the
/// shared ADR-058 emitter; with no diagnostics it is a complete,
/// schema-valid SARIF document with empty `results[]` (never a 0-byte
/// file).
///
/// `exceptions.json` carries the active grants from the exception
/// store — tracked `anvil/exceptions/store.json`, or the legacy
/// `.anvil/exceptions.json` fallback for unmigrated repos
/// (EXCEPT-009); the verifier's `exceptions` check re-verifies each
/// one (scope/expiry/revocation/attribution).
///
/// Placeholders written by this step (their collectors land later):
///
/// - `edda-context.json` — `{}` (reference-only, gated on EDDA-SEAL)
/// - `verification.json` — [`CapsuleVerification::from_checks`] with
///   no checks: the degraded placeholder GITGOV-009's verify step
///   overwrites
///
/// # Errors
///
/// [`CapsuleError::Collect`] for output-directory refusal or I/O
/// failure; [`CapsuleError::Serialise`] if a document cannot be
/// encoded.
pub fn write_capsule(
    out_dir: &Path,
    content: &CapsuleContent,
) -> Result<CapsuleManifest, CapsuleError> {
    let mut manifest = CapsuleManifest::new(
        CapsuleRange {
            base: content.commits.base.clone(),
            head: content.commits.head.clone(),
            // The window of witness `seq` attesting commits in the
            // range (GITGOV-007); absent when none are witnessed.
            witness_seq_start: content.witness.seq_start,
            witness_seq_end: content.witness.seq_end,
        },
        content.producer.clone(),
    );

    let verification = CapsuleVerification::from_checks(vec![]);
    let readme = render_readme(content);

    let files: [(&str, Vec<u8>); 10] = [
        ("commits.json", content.commits.to_canonical_bytes()?),
        ("policy.json", content.digests.policy.to_canonical_bytes()?),
        (
            "baseline.json",
            content.digests.baseline.to_canonical_bytes()?,
        ),
        ("rules.json", content.digests.rules.to_canonical_bytes()?),
        ("witness.ndjson", content.witness.ndjson.clone()),
        ("diagnostics.sarif", content.diagnostics.sarif.clone()),
        ("exceptions.json", content.exceptions.to_canonical_bytes()?),
        ("edda-context.json", b"{}".to_vec()),
        ("verification.json", verification.to_canonical_bytes()?),
        ("README.md", readme.into_bytes()),
    ];

    // Scan-on-write (ADR-072 §3, GITGOV-012): refuse a capsule whose
    // evidence carries secret-shaped content *before* touching the
    // filesystem — so secret-bearing evidence never reaches a tracked
    // write and no partial capsule directory is left behind.
    scan_evidence_for_secrets(&files)?;
    scan_exception_prose_for_secrets(&content.exceptions)?;

    for (name, bytes) in &files {
        manifest.record_file(name, bytes);
    }
    // Layout invariant: every ADR-074 required file must be recorded
    // before the manifest is written. `files` is type-pinned to the
    // required count, but the names could still drift.
    let missing = manifest.missing_required();
    if !missing.is_empty() {
        return Err(CapsuleError::Serialise(format!(
            "capsule writer missed required files: {missing:?}"
        )));
    }
    let manifest_bytes = manifest.to_canonical_bytes()?;
    validate_out_dir(out_dir)?;
    publish_capsule_files(out_dir, &files, &manifest_bytes)?;
    Ok(manifest)
}

#[cfg(unix)]
fn publish_capsule_files(
    out_dir: &Path,
    files: &[(&str, Vec<u8>)],
    manifest_bytes: &[u8],
) -> Result<(), CapsuleError> {
    let mut staging_dir = create_staging_dir(out_dir)?;
    for (name, bytes) in files {
        staging_dir.write_file(name, bytes)?;
    }
    staging_dir.write_file("manifest.json", manifest_bytes)?;
    staging_dir.publish(out_dir)
}

#[cfg(windows)]
fn publish_capsule_files(
    out_dir: &Path,
    files: &[(&str, Vec<u8>)],
    manifest_bytes: &[u8],
) -> Result<(), CapsuleError> {
    let mut publish_files: Vec<(&str, &[u8])> = files
        .iter()
        .map(|(name, bytes)| (*name, bytes.as_slice()))
        .collect();
    publish_files.push(("manifest.json", manifest_bytes));
    anvil_intercept_win32::path_nofollow::publish_directory_nofollow(out_dir, &publish_files)
        .map_err(|error| out_dir_error(out_dir, &format!("could not be published safely: {error}")))
}

#[cfg(not(any(unix, windows)))]
fn publish_capsule_files(
    out_dir: &Path,
    _files: &[(&str, Vec<u8>)],
    _manifest_bytes: &[u8],
) -> Result<(), CapsuleError> {
    Err(out_dir_error(
        out_dir,
        "safe capsule publication is unsupported on this platform",
    ))
}

/// Scan-on-write enforcement (ADR-072 §3, GITGOV-012): every byte
/// bound for a tracked capsule file is scanned for structurally
/// unambiguous secret shapes. Durable Git evidence must never carry
/// raw secrets; this covers content that never passed through a
/// producer's redaction — committed paths, future applied-exception
/// reason strings, README prose. A finding fails capsule creation
/// (the caller runs this before any filesystem write).
///
/// Two deliberate deviations from [`SecretCheckConfig::default`]:
///
/// - **Entropy disabled.** Capsule evidence is digest-dense (SHA-256
///   hex, base64 SARIF); statistical entropy detection would
///   false-positive and block legitimate capsules. Scan-on-write
///   enforces *identifiable* secret shapes (API keys, tokens,
///   private-key headers) — the honest line ADR-072 §3 draws — not
///   entropy guesses. Commit SHAs and digests are additionally
///   shape-allowlisted by anvil-checks, and every high-confidence
///   pattern is prefix-anchored, so bare hex never matches.
/// - **Per-line guard lifted.** Canonical capsule JSON is compact (a
///   single line); the default 4 KiB SCAN-002 per-line guard would
///   skip a whole evidence file before any pattern ran. A capsule is
///   bounded, locally produced content — not adversarial minified
///   input — so scanning the full line is safe.
///
/// # Coverage boundary
///
/// This scans the ten tracked evidence files. `manifest.json` is not
/// scanned directly: its fields are structurally non-free-text
/// (constant schema, commit SHAs, a `u64` seq window, file→SHA-256
/// digests), and its only string input — `producer.anvil_version` —
/// is already scanned via the README, which re-emits it. Any future
/// **free-text** field added to the manifest must be added to the
/// scanned set here.
///
/// # Known limitation (tracked: GITGOV-012)
///
/// With entropy disabled, free-text coverage is limited to
/// prefix-anchored secret shapes. Free-text exception prose gets the
/// entropy-enabled pass in [`scan_exception_prose_for_secrets`] —
/// the digest-density rationale for disabling entropy here does not
/// hold for prose (ADR-072 §3). `edda-context.json` remains an inert
/// `{}` placeholder; EDDA-SEAL owes it the same prose pass when it
/// wires real content.
fn scan_evidence_for_secrets(files: &[(&str, Vec<u8>)]) -> Result<(), CapsuleError> {
    let config = SecretCheckConfig {
        enable_entropy: false,
        max_line_bytes: usize::MAX,
        ..SecretCheckConfig::default()
    };
    for (name, bytes) in files {
        // Evidence is UTF-8 by construction (canonical JSON, NDJSON,
        // SARIF, Markdown — all serialised from Rust strings), so the
        // lossy path is unreachable. A future evidence type emitting
        // non-UTF-8 bytes must validate UTF-8 before this gate, since
        // U+FFFD substitution could otherwise split a secret token.
        let content = String::from_utf8_lossy(bytes);
        // No custom patterns: pass an empty compiled set so the
        // built-in `LazyLock` patterns are used with no per-call
        // recompilation (the legacy `scan_content` entry point would
        // re-run `compile_custom_patterns` each call).
        let findings =
            scan_content_with_compiled_patterns(&content, name, &config, &[], usize::MAX).0;
        if !findings.is_empty() {
            return Err(CapsuleError::SecretInEvidence {
                file: (*name).to_string(),
                count: findings.len(),
            });
        }
    }
    Ok(())
}

/// Entropy-enabled scan over the exception grants' free-text fields
/// (`reason`, `owner`, `created_by`) only. The whole-file scan above
/// keeps entropy off because evidence is digest-dense (hex shas would
/// false-positive constantly); prose has no such excuse, and a bare
/// high-entropy token pasted into a grant reason must not land in a
/// tracked capsule (ADR-072 §3, the gap the EXCEPT-009 wiring would
/// otherwise open).
///
/// `max_line_bytes` stays unbounded deliberately: the SCAN-002 guard
/// *skips* overlong lines, so on operator-editable prose a finite cap
/// is a bypass (pad the reason past the cap and the secret is never
/// scanned). Worst-case scan cost is already bounded upstream by the
/// store's 1 MiB read cap — the same trade `scan_evidence_for_secrets`
/// makes for canonical single-line JSON.
fn scan_exception_prose_for_secrets(
    exceptions: &crate::collect_exceptions::CollectedExceptions,
) -> Result<(), CapsuleError> {
    let mut prose = String::new();
    for ex in &exceptions.exceptions {
        for field in [
            Some(ex.reason.as_str()),
            ex.owner.as_deref(),
            ex.created_by.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            prose.push_str(field);
            prose.push('\n');
        }
    }
    if prose.trim().is_empty() {
        return Ok(());
    }
    let config = SecretCheckConfig {
        enable_entropy: true,
        max_line_bytes: usize::MAX,
        ..SecretCheckConfig::default()
    };
    let findings =
        scan_content_with_compiled_patterns(&prose, "exceptions.json", &config, &[], usize::MAX).0;
    if !findings.is_empty() {
        return Err(CapsuleError::SecretInEvidence {
            file: "exceptions.json".to_string(),
            count: findings.len(),
        });
    }
    Ok(())
}

/// Refuse a symlinked or non-empty existing output directory.
///
/// The symlink check (`lstat`, final component) keeps a
/// symlink-to-empty-dir from silently landing the capsule somewhere
/// other than the named path. Publication repeats this check after all
/// evidence has been written to a private sibling directory.
fn validate_out_dir(out_dir: &Path) -> Result<(), CapsuleError> {
    if let Ok(md) = std::fs::symlink_metadata(out_dir)
        && md.file_type().is_symlink()
    {
        return Err(out_dir_error(
            out_dir,
            "is a symlink; name the real destination",
        ));
    }
    match std::fs::read_dir(out_dir) {
        Ok(mut entries) => {
            if entries.next().is_some() {
                return Err(out_dir_error(
                    out_dir,
                    "is not empty; refusing to write a capsule into existing content",
                ));
            }
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(out_dir_error(out_dir, &format!("could not be read: {e}"))),
    }
}

#[cfg(unix)]
static STAGING_COUNTER: AtomicU64 = AtomicU64::new(0);

#[cfg(unix)]
#[derive(Debug)]
struct StagingDir {
    path: PathBuf,
    name: std::ffi::OsString,
    parent_fd: std::os::fd::OwnedFd,
    dir_fd: std::os::fd::OwnedFd,
    published: bool,
}

/// Create a private directory beside the final destination. On Unix,
/// the returned object pins both the parent and staging directory with
/// file descriptors so later writes cannot be redirected by replacing
/// the visible staging pathname.
#[cfg(unix)]
fn create_staging_dir(out_dir: &Path) -> Result<StagingDir, CapsuleError> {
    create_staging_dir_with_hook(out_dir, |_| {})
}

#[cfg(unix)]
fn create_staging_dir_with_hook(
    out_dir: &Path,
    mut after_create: impl FnMut(&Path),
) -> Result<StagingDir, CapsuleError> {
    use std::os::fd::AsFd;
    use std::time::{SystemTime, UNIX_EPOCH};

    use nix::errno::Errno;
    use nix::fcntl::{AtFlags, OFlag, open, openat};
    use nix::sys::stat::{Mode, fstatat, mkdirat};

    let parent = out_dir
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)
        .map_err(|e| out_dir_error(out_dir, &format!("parent could not be created: {e}")))?;
    let final_name = out_dir
        .file_name()
        .ok_or_else(|| out_dir_error(out_dir, "has no final path component"))?;
    let parent_fd = open(
        parent,
        OFlag::O_DIRECTORY | OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
        Mode::empty(),
    )
    .map_err(std::io::Error::from)
    .map_err(|e| out_dir_error(out_dir, &format!("parent could not be opened safely: {e}")))?;
    validate_staging_parent(&parent_fd, out_dir)?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());

    for _ in 0..128 {
        let sequence = STAGING_COUNTER.fetch_add(1, Ordering::Relaxed);
        let name = format!(
            ".{}.tmp-{}-{nanos}-{sequence}",
            final_name.to_string_lossy(),
            std::process::id()
        );
        match mkdirat(
            parent_fd.as_fd(),
            name.as_str(),
            Mode::from_bits_truncate(0o700),
        ) {
            Ok(()) => {
                let created = fstatat(
                    parent_fd.as_fd(),
                    name.as_str(),
                    AtFlags::AT_SYMLINK_NOFOLLOW,
                )
                .map_err(std::io::Error::from)
                .map_err(|e| {
                    out_dir_error(
                        out_dir,
                        &format!("private staging identity could not be captured: {e}"),
                    )
                })?;
                let candidate = parent.join(&name);
                after_create(&candidate);
                let dir_fd = match openat(
                    parent_fd.as_fd(),
                    name.as_str(),
                    OFlag::O_DIRECTORY | OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
                    Mode::empty(),
                ) {
                    Ok(fd) => fd,
                    Err(error) => {
                        let _ = nix::unistd::unlinkat(
                            parent_fd.as_fd(),
                            name.as_str(),
                            nix::unistd::UnlinkatFlags::RemoveDir,
                        );
                        return Err(out_dir_error(
                            out_dir,
                            &format!(
                                "private staging directory could not be pinned: {}",
                                std::io::Error::from(error)
                            ),
                        ));
                    }
                };
                let staging = StagingDir {
                    path: candidate,
                    name: name.into(),
                    parent_fd,
                    dir_fd,
                    published: false,
                };
                staging.validate_created_identity(created.st_dev, created.st_ino)?;
                return Ok(staging);
            }
            Err(Errno::EEXIST) => {}
            Err(error) => {
                return Err(out_dir_error(
                    out_dir,
                    &format!(
                        "private staging directory could not be created: {}",
                        std::io::Error::from(error)
                    ),
                ));
            }
        }
    }

    Err(out_dir_error(
        out_dir,
        "private staging directory name could not be allocated",
    ))
}

#[cfg(unix)]
fn validate_staging_parent(
    parent_fd: &std::os::fd::OwnedFd,
    out_dir: &Path,
) -> Result<(), CapsuleError> {
    use nix::sys::stat::fstat;
    use nix::unistd::geteuid;

    let stat = fstat(parent_fd)
        .map_err(std::io::Error::from)
        .map_err(|error| {
            out_dir_error(
                out_dir,
                &format!("staging parent metadata could not be read: {error}"),
            )
        })?;
    if stat.st_uid != geteuid().as_raw() {
        return Err(out_dir_error(
            out_dir,
            "staging parent is not owned by the current user",
        ));
    }
    if stat.st_mode & 0o022 != 0 {
        return Err(out_dir_error(
            out_dir,
            "staging parent is writable by another OS identity",
        ));
    }
    Ok(())
}

#[cfg(unix)]
impl StagingDir {
    fn validate_created_identity(
        &self,
        created_device: libc::dev_t,
        created_inode: libc::ino_t,
    ) -> Result<(), CapsuleError> {
        use nix::dir::Dir;
        use nix::sys::stat::{SFlag, fstat};
        use nix::unistd::{dup, geteuid};

        let stat = fstat(&self.dir_fd)
            .map_err(std::io::Error::from)
            .map_err(|error| {
                out_dir_error(
                    &self.path,
                    &format!("staging metadata could not be read: {error}"),
                )
            })?;
        let kind = SFlag::from_bits_truncate(stat.st_mode);
        if !kind.contains(SFlag::S_IFDIR)
            || stat.st_dev != created_device
            || stat.st_ino != created_inode
        {
            return Err(out_dir_error(
                &self.path,
                "opened staging directory is not the directory that was created",
            ));
        }
        if stat.st_uid != geteuid().as_raw() || stat.st_mode & 0o777 != 0o700 {
            return Err(out_dir_error(
                &self.path,
                "staging directory must be current-user-owned with mode 0700",
            ));
        }
        if !self.name_matches_fd(&self.name)? {
            return Err(out_dir_error(
                &self.path,
                "private staging directory was replaced while being opened",
            ));
        }

        let duplicate = dup(&self.dir_fd)
            .map_err(std::io::Error::from)
            .map_err(|error| {
                out_dir_error(
                    &self.path,
                    &format!("staging directory could not be inspected: {error}"),
                )
            })?;
        let mut directory = Dir::from_fd(duplicate)
            .map_err(std::io::Error::from)
            .map_err(|error| {
                out_dir_error(
                    &self.path,
                    &format!("staging directory could not be inspected: {error}"),
                )
            })?;
        for entry in directory.iter() {
            let entry = entry.map_err(std::io::Error::from).map_err(|error| {
                out_dir_error(
                    &self.path,
                    &format!("staging directory could not be inspected: {error}"),
                )
            })?;
            if !matches!(entry.file_name().to_bytes(), b"." | b"..") {
                return Err(out_dir_error(
                    &self.path,
                    "staging directory was not empty when opened",
                ));
            }
        }
        Ok(())
    }

    fn name_matches_fd(&self, name: &std::ffi::OsStr) -> Result<bool, CapsuleError> {
        use nix::errno::Errno;
        use nix::fcntl::AtFlags;
        use nix::sys::stat::{fstat, fstatat};

        let held = fstat(&self.dir_fd)
            .map_err(std::io::Error::from)
            .map_err(|e| out_dir_error(&self.path, &format!("staging identity failed: {e}")))?;
        match fstatat(&self.parent_fd, name, AtFlags::AT_SYMLINK_NOFOLLOW) {
            Ok(named) => Ok(held.st_dev == named.st_dev && held.st_ino == named.st_ino),
            Err(Errno::ENOENT) => Ok(false),
            Err(error) => Err(out_dir_error(
                &self.path,
                &format!(
                    "staging path could not be revalidated: {}",
                    std::io::Error::from(error)
                ),
            )),
        }
    }

    fn write_file(&self, name: &str, bytes: &[u8]) -> Result<(), CapsuleError> {
        use std::io::Write;

        use nix::fcntl::{OFlag, openat};
        use nix::sys::stat::Mode;

        let fd = openat(
            &self.dir_fd,
            name,
            OFlag::O_CREAT | OFlag::O_EXCL | OFlag::O_WRONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
            Mode::from_bits_truncate(0o600),
        )
        .map_err(std::io::Error::from)
        .map_err(|e| CapsuleError::Collect {
            path: name.to_string(),
            detail: format!("creating in pinned staging directory: {e}"),
        })?;
        let mut file = std::fs::File::from(fd);
        file.write_all(bytes).map_err(|e| CapsuleError::Collect {
            path: name.to_string(),
            detail: format!("writing: {e}"),
        })
    }

    fn publish(&mut self, out_dir: &Path) -> Result<(), CapsuleError> {
        use nix::errno::Errno;
        use nix::fcntl::{AtFlags, renameat};
        use nix::sys::stat::{SFlag, fstatat};
        use nix::unistd::{UnlinkatFlags, unlinkat};

        let expected_parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        let parent = out_dir
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let destination = out_dir
            .file_name()
            .ok_or_else(|| out_dir_error(out_dir, "has no final path component"))?;
        if parent != expected_parent {
            return Err(out_dir_error(
                out_dir,
                "does not share the pinned staging parent",
            ));
        }
        if !self.name_matches_fd(&self.name)? {
            return Err(out_dir_error(
                out_dir,
                "private staging directory was replaced before publication",
            ));
        }

        match fstatat(&self.parent_fd, destination, AtFlags::AT_SYMLINK_NOFOLLOW) {
            Ok(stat) => {
                let kind = SFlag::from_bits_truncate(stat.st_mode);
                if kind.contains(SFlag::S_IFLNK) {
                    return Err(out_dir_error(
                        out_dir,
                        "changed to a symlink before publication",
                    ));
                }
                if !kind.contains(SFlag::S_IFDIR) {
                    return Err(out_dir_error(
                        out_dir,
                        "changed to a non-directory before publication",
                    ));
                }
                unlinkat(&self.parent_fd, destination, UnlinkatFlags::RemoveDir)
                    .map_err(std::io::Error::from)
                    .map_err(|e| {
                        out_dir_error(
                            out_dir,
                            &format!("changed before publication and could not be removed: {e}"),
                        )
                    })?;
            }
            Err(Errno::ENOENT) => {}
            Err(error) => {
                return Err(out_dir_error(
                    out_dir,
                    &format!(
                        "could not be revalidated before publication: {}",
                        std::io::Error::from(error)
                    ),
                ));
            }
        }

        if !self.name_matches_fd(&self.name)? {
            return Err(out_dir_error(
                out_dir,
                "private staging directory was replaced before publication",
            ));
        }
        renameat(
            &self.parent_fd,
            self.name.as_os_str(),
            &self.parent_fd,
            destination,
        )
        .map_err(std::io::Error::from)
        .map_err(|e| out_dir_error(out_dir, &format!("could not be published atomically: {e}")))?;
        if !self.name_matches_fd(destination)? {
            let _ = remove_named_entry(&self.parent_fd, destination);
            return Err(out_dir_error(
                out_dir,
                "published directory identity did not match the written staging directory",
            ));
        }
        self.published = true;
        Ok(())
    }
}

#[cfg(unix)]
fn remove_named_entry(parent_fd: &std::os::fd::OwnedFd, name: &std::ffi::OsStr) -> nix::Result<()> {
    use nix::fcntl::AtFlags;
    use nix::sys::stat::{SFlag, fstatat};
    use nix::unistd::{UnlinkatFlags, unlinkat};

    let stat = fstatat(parent_fd, name, AtFlags::AT_SYMLINK_NOFOLLOW)?;
    let flags = if SFlag::from_bits_truncate(stat.st_mode).contains(SFlag::S_IFDIR) {
        UnlinkatFlags::RemoveDir
    } else {
        UnlinkatFlags::NoRemoveDir
    };
    unlinkat(parent_fd, name, flags)
}

#[cfg(unix)]
impl Drop for StagingDir {
    fn drop(&mut self) {
        use nix::unistd::{UnlinkatFlags, unlinkat};

        if self.published {
            return;
        }
        for name in crate::manifest::REQUIRED_FILES
            .iter()
            .copied()
            .chain(std::iter::once("manifest.json"))
        {
            let _ = unlinkat(&self.dir_fd, name, UnlinkatFlags::NoRemoveDir);
        }
        if self.name_matches_fd(&self.name).unwrap_or(false) {
            let _ = unlinkat(
                &self.parent_fd,
                self.name.as_os_str(),
                UnlinkatFlags::RemoveDir,
            );
        }
    }
}

fn out_dir_error(out_dir: &Path, detail: &str) -> CapsuleError {
    CapsuleError::Collect {
        path: out_dir.display().to_string(),
        detail: format!("output directory {detail}"),
    }
}

/// Exclusive-create write: a concurrent file at the same name (racing
/// creator, leftover content past the emptiness check) is an error,
/// never an overwrite.
/// Deterministic human-readable summary. No timestamps — the same
/// content must produce byte-identical capsules.
fn render_readme(content: &CapsuleContent) -> String {
    let policy = content
        .digests
        .policy
        .policy_file
        .as_ref()
        .map_or("absent", |file| file.path.as_str());
    let baseline = if content.digests.baseline.digest.is_some() {
        "present"
    } else {
        "absent"
    };
    let rules = content
        .digests
        .rules
        .rules_sha
        .as_deref()
        .unwrap_or("absent (no .anvil.* config)");
    let witness = witness_coverage(&content.witness);
    format!(
        "# anvil review capsule\n\n\
         Governance evidence for the commit range below, packaged per\n\
         ADR-074 (`anvil.capsule.v1`). Verify with `anvil capsule verify`;\n\
         every file's SHA-256 is recorded in `manifest.json`.\n\n\
         | Field | Value |\n\
         | ----- | ----- |\n\
         | Base | `{base}` |\n\
         | Head | `{head}` |\n\
         | Commits | {commits} |\n\
         | Policy | {policy} |\n\
         | Baseline | {baseline} |\n\
         | Rules identity | `{rules}` |\n\
         | Witness | {witness} |\n\
         | Producer | anvil {version} |\n\n\
         `verification.json` starts as a degraded placeholder — an\n\
         unverified capsule never claims `pass`. `witness.ndjson` carries\n\
         the verbatim full witness chain; an empty file means the repo\n\
         has no witness chain, not \"no findings\". `diagnostics.sarif` is\n\
         a SARIF 2.1.0 document; in v0 its `results[]` is empty because no\n\
         check pass is wired into capsule creation yet (GITGOV-009+) —\n\
         read it as \"no diagnostics collected\", not \"none found\".\n",
        base = content.commits.base,
        head = content.commits.head,
        commits = content.commits.commits.len(),
        version = content.producer.anvil_version,
    )
}

/// Deterministic one-line witness-coverage summary for the README.
///
/// Distinguishes the three honest states: an empty chain
/// (fresh-adoption repo), a chain present but with no line attesting a
/// range commit, and the `seq` window of the range's witnessed lines.
fn witness_coverage(witness: &CollectedWitness) -> String {
    match (witness.seq_start, witness.seq_end) {
        // Inclusive `[start, end]` window — spell it out so the bound is
        // not read as an exclusive `..` range, and collapse the
        // single-line case to avoid a confusing `seq 1 to 1`.
        (Some(start), Some(end)) if start == end => format!("seq {start}"),
        (Some(start), Some(end)) => format!("seq {start} to {end} (inclusive)"),
        _ if witness.ndjson.is_empty() => "absent (no witness chain)".to_string(),
        _ => "present, no range coverage".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::sha256_hex;
    use crate::collect::{COMMITS_SCHEMA, CommitEntry};
    use crate::collect_digests::{
        BASELINE_DIGEST_SCHEMA, BaselineDigest, POLICY_DIGEST_SCHEMA, PolicyDigest,
        RULES_DIGEST_SCHEMA, RulesDigest,
    };
    use crate::manifest::REQUIRED_FILES;
    use crate::verification::Verdict;

    fn empty_exceptions() -> CollectedExceptions {
        CollectedExceptions {
            exceptions: Vec::new(),
        }
    }

    fn sample_content() -> CapsuleContent {
        CapsuleContent {
            commits: CommitsDocument {
                schema: COMMITS_SCHEMA.to_string(),
                base: "1111111111111111111111111111111111111111".to_string(),
                head: "2222222222222222222222222222222222222222".to_string(),
                commits: vec![],
            },
            digests: CollectedDigests {
                policy: PolicyDigest {
                    schema: POLICY_DIGEST_SCHEMA.to_string(),
                    policy_file: None,
                    config_file: None,
                },
                rules: RulesDigest {
                    schema: RULES_DIGEST_SCHEMA.to_string(),
                    anvil_version: "0.0.0-test".to_string(),
                    opa_runtime_version: "0.10.0".to_string(),
                    rules: vec![],
                    config_sha: None,
                    rules_sha: None,
                },
                baseline: BaselineDigest {
                    schema: BASELINE_DIGEST_SCHEMA.to_string(),
                    cutoff_commit: None,
                    digest: None,
                },
            },
            witness: CollectedWitness::default(),
            diagnostics: crate::collect_diagnostics::collect_diagnostics(&[]).unwrap(),
            exceptions: empty_exceptions(),
            producer: Producer {
                anvil_version: "0.0.0-test".to_string(),
            },
        }
    }

    #[test]
    fn write_capsule_writes_every_required_file_plus_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("capsule");

        let manifest = write_capsule(&out, &sample_content()).unwrap();

        for name in REQUIRED_FILES {
            assert!(out.join(name).exists(), "{name} missing");
        }
        assert!(out.join("manifest.json").exists());
        assert!(manifest.missing_required().is_empty());
    }

    /// Every manifest digest must match the bytes on disk — the byte
    /// contract a verifier will enforce.
    #[test]
    fn write_capsule_manifest_digests_match_disk_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("capsule");

        let manifest = write_capsule(&out, &sample_content()).unwrap();

        for (name, digest) in &manifest.files {
            let bytes = std::fs::read(out.join(name)).unwrap();
            assert_eq!(&sha256_hex(&bytes), digest, "digest mismatch for {name}");
        }
        // And the manifest file itself round-trips.
        let manifest_bytes = std::fs::read(out.join("manifest.json")).unwrap();
        let parsed = CapsuleManifest::from_json_bytes(&manifest_bytes).unwrap();
        assert_eq!(parsed, manifest);
    }

    /// The unverified capsule carries a machine-readable degraded
    /// verdict, never silence (ADR-072 §4).
    #[test]
    fn write_capsule_verification_placeholder_is_degraded() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("capsule");

        write_capsule(&out, &sample_content()).unwrap();

        let bytes = std::fs::read(out.join("verification.json")).unwrap();
        let verification = CapsuleVerification::from_json_bytes(&bytes).unwrap();
        assert_eq!(verification.verdict, Verdict::Degraded);
        assert!(verification.checks.is_empty());
    }

    /// With no diagnostics, `diagnostics.sarif` is a complete SARIF
    /// document — an `anvil` run with empty `results[]` — not a 0-byte
    /// stream and not an empty `runs[]` (GITGOV-008 via the shared
    /// emitter).
    #[test]
    fn write_capsule_diagnostics_is_valid_empty_sarif_document() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("capsule");

        write_capsule(&out, &sample_content()).unwrap();

        let bytes = std::fs::read(out.join("diagnostics.sarif")).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["version"], "2.1.0");
        let runs = value["runs"].as_array().unwrap();
        assert_eq!(runs.len(), 1, "a complete run, not an empty runs[]");
        assert_eq!(runs[0]["tool"]["driver"]["name"], "anvil");
        assert!(runs[0]["results"].as_array().unwrap().is_empty());
    }

    /// The witness chain is embedded verbatim and the range pointers
    /// are recorded in the manifest (GITGOV-007).
    #[test]
    fn write_capsule_embeds_witness_chain_and_seq_window() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("capsule");

        let mut content = sample_content();
        content.witness = CollectedWitness {
            ndjson: b"{\"seq\":1}\n{\"seq\":2}\n".to_vec(),
            seq_start: Some(2),
            seq_end: Some(7),
        };

        let manifest = write_capsule(&out, &content).unwrap();

        // Bytes land verbatim, and the recorded digest matches them.
        let on_disk = std::fs::read(out.join("witness.ndjson")).unwrap();
        assert_eq!(on_disk, content.witness.ndjson);
        assert_eq!(sha256_hex(&on_disk), manifest.files["witness.ndjson"]);
        // The range window is carried on the manifest.
        assert_eq!(manifest.range.witness_seq_start, Some(2));
        assert_eq!(manifest.range.witness_seq_end, Some(7));
    }

    /// A fresh-adoption repo (no witness chain) keeps the
    /// present-but-empty discipline: an empty `witness.ndjson` and no
    /// range pointers (absent, never `null`).
    #[test]
    fn write_capsule_empty_witness_is_present_but_empty() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("capsule");

        let manifest = write_capsule(&out, &sample_content()).unwrap();

        let on_disk = std::fs::read(out.join("witness.ndjson")).unwrap();
        assert!(on_disk.is_empty());
        assert_eq!(manifest.range.witness_seq_start, None);
        assert_eq!(manifest.range.witness_seq_end, None);
    }

    #[test]
    fn write_capsule_is_deterministic_for_same_content() {
        let dir = tempfile::tempdir().unwrap();
        let out_a = dir.path().join("a");
        let out_b = dir.path().join("b");

        let a = write_capsule(&out_a, &sample_content()).unwrap();
        let b = write_capsule(&out_b, &sample_content()).unwrap();

        assert_eq!(a, b);
        assert_eq!(
            std::fs::read(out_a.join("manifest.json")).unwrap(),
            std::fs::read(out_b.join("manifest.json")).unwrap()
        );
    }

    #[test]
    fn write_capsule_refuses_non_empty_out_dir() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("capsule");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("keep.txt"), "existing").unwrap();

        let err = write_capsule(&out, &sample_content()).unwrap_err();

        assert!(err.to_string().contains("not empty"), "{err}");
        assert_eq!(
            std::fs::read_to_string(out.join("keep.txt")).unwrap(),
            "existing",
            "existing content untouched"
        );
    }

    /// A symlinked `--out` must not silently land the capsule at the
    /// symlink's target.
    #[cfg(unix)]
    #[test]
    fn write_capsule_refuses_symlinked_out_dir() {
        let target = tempfile::tempdir().unwrap();
        let staging = tempfile::tempdir().unwrap();
        let out = staging.path().join("capsule");
        std::os::unix::fs::symlink(target.path(), &out).unwrap();

        let err = write_capsule(&out, &sample_content()).unwrap_err();

        assert!(err.to_string().contains("symlink"), "{err}");
        assert!(
            !target.path().join("manifest.json").exists(),
            "nothing written through the symlink"
        );
    }

    #[cfg(unix)]
    #[test]
    fn publish_refuses_destination_swapped_to_symlink() {
        let parent = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        let out = parent.path().join("capsule");
        let mut staging = create_staging_dir(&out).unwrap();
        staging.write_file("manifest.json", b"staged").unwrap();
        std::os::unix::fs::symlink(target.path(), &out).unwrap();

        let err = staging.publish(&out).unwrap_err();

        assert!(err.to_string().contains("symlink"), "{err}");
        assert!(!target.path().join("manifest.json").exists());
        assert_eq!(
            std::fs::read_to_string(staging.path.join("manifest.json")).unwrap(),
            "staged"
        );
    }

    #[cfg(unix)]
    #[test]
    fn staging_path_swap_cannot_redirect_evidence_writes() {
        let parent = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let out = parent.path().join("capsule");
        let mut staging = create_staging_dir(&out).unwrap();
        let staging_path = staging.path.clone();
        let moved = parent.path().join("moved-staging");
        std::fs::rename(&staging_path, &moved).unwrap();
        std::os::unix::fs::symlink(outside.path(), &staging_path).unwrap();

        let result = staging.write_file("manifest.json", b"staged");

        assert!(
            result.is_err() || moved.join("manifest.json").exists(),
            "a write may fail closed or remain anchored to the created staging directory"
        );
        assert!(
            !outside.path().join("manifest.json").exists(),
            "the swapped staging symlink must never receive evidence"
        );
        assert!(
            staging.publish(&out).is_err(),
            "a replaced staging name must not be published"
        );
        assert!(
            !out.exists(),
            "publication failure must leave the destination absent"
        );
    }

    #[cfg(unix)]
    #[test]
    fn staging_directory_substitution_before_open_is_refused() {
        let parent = tempfile::tempdir().unwrap();
        let out = parent.path().join("capsule");
        let moved = parent.path().join("created-staging");

        let error = create_staging_dir_with_hook(&out, |candidate| {
            std::fs::rename(candidate, &moved).unwrap();
            std::fs::create_dir(candidate).unwrap();
        })
        .expect_err("a substituted ordinary directory must be rejected");

        assert!(
            error
                .to_string()
                .contains("is not the directory that was created"),
            "{error}"
        );
        assert!(
            std::fs::read_dir(&moved).unwrap().next().is_none(),
            "the originally created staging directory received no evidence"
        );
        assert!(
            !out.exists(),
            "a rejected staging substitution must not publish a capsule"
        );
    }

    #[test]
    fn write_capsule_accepts_existing_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("capsule");
        std::fs::create_dir_all(&out).unwrap();

        write_capsule(&out, &sample_content()).unwrap();

        assert!(out.join("manifest.json").exists());
    }

    /// A textbook AWS access key — structurally unambiguous and not
    /// suppressed by the `EXAMPLE` keyword for high-confidence patterns
    /// (anvil-checks #1800) — leaked as a committed file path. It flows
    /// verbatim into `commits.json`; `CommitEntry` carries no message
    /// field in v0, so a secret-shaped *path* is the realistic v0
    /// leak vector into capsule evidence.
    fn content_with_secret_in_a_changed_path() -> CapsuleContent {
        let mut content = sample_content();
        content.commits.commits.push(CommitEntry {
            sha: "3333333333333333333333333333333333333333".to_string(),
            tree: "4444444444444444444444444444444444444444".to_string(),
            parents: vec![],
            changed_paths: vec!["config/AKIAIOSFODNN7EXAMPLE.env".to_string()],
        });
        content
    }

    /// EXCEPT-009 / ADR-072 §3: a high-entropy token pasted into a
    /// grant's free-text reason fails capsule creation via the
    /// entropy-enabled prose pass (the whole-file scan keeps entropy
    /// off for digest-dense evidence and would miss it).
    #[test]
    fn write_capsule_secret_in_exception_reason_fails_creation() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("capsule");
        let mut content = sample_content();
        content
            .exceptions
            .exceptions
            .push(anvil_policy::exceptions::PolicyException {
                schema_version: "anvil.exception.v1".to_string(),
                id: "exc_prose_test".to_string(),
                policy_id: "AP-001".to_string(),
                file_pattern: String::new(),
                finding_hash: None,
                reason: "temp access, token: Zk9qX2tRb1B3N3lMc0RhVGdVaEplRnZCbU5wUXJTdA=="
                    .to_string(),
                owner: Some("team".to_string()),
                created_by: Some("alice".to_string()),
                created_at: chrono::Utc::now(),
                expires_at: None,
                revoked: None,
            });
        let err = write_capsule(&out, &content).expect_err("secret-shaped reason must refuse");
        assert!(
            matches!(
                err,
                CapsuleError::SecretInEvidence { ref file, .. } if file == "exceptions.json"
            ),
            "{err:?}",
        );
        assert!(!out.exists() || !out.join("manifest.json").exists());
    }

    /// GITGOV-012 / ADR-072 §3 scan-on-write: secret-shaped bytes bound
    /// for a tracked evidence file fail capsule creation, naming the
    /// offending file.
    #[test]
    fn write_capsule_tamper_secret_in_evidence_fails_creation() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("capsule");

        let err = write_capsule(&out, &content_with_secret_in_a_changed_path()).unwrap_err();

        assert!(
            matches!(&err, CapsuleError::SecretInEvidence { file, .. } if file == "commits.json"),
            "expected SecretInEvidence on commits.json, got {err:?}"
        );
    }

    /// The scan-on-write gate runs before any filesystem write, so a
    /// refused capsule leaves nothing on disk — secret-bearing evidence
    /// never reaches a tracked write (the GITGOV-012 invariant).
    #[test]
    fn write_capsule_tamper_secret_never_reaches_disk() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("capsule");

        let _ = write_capsule(&out, &content_with_secret_in_a_changed_path()).unwrap_err();

        assert!(
            !out.exists(),
            "capsule dir must not be created when a secret is refused"
        );
    }

    /// Clean, digest-dense evidence (real 40-hex SHAs, SHA-256 digests,
    /// base64 SARIF) must not false-positive — scan-on-write enforces
    /// identifiable secret shapes, not entropy guesses. This is the
    /// regression guard that the gate stays silent on legitimate
    /// capsules.
    #[test]
    fn write_capsule_tamper_clean_evidence_is_not_refused() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("capsule");

        // A normal commit with hex SHAs and a plain path — no secret.
        let mut content = sample_content();
        content.commits.commits.push(CommitEntry {
            sha: "5555555555555555555555555555555555555555".to_string(),
            tree: "6666666666666666666666666666666666666666".to_string(),
            parents: vec!["1111111111111111111111111111111111111111".to_string()],
            changed_paths: vec!["src/lib.rs".to_string()],
        });

        write_capsule(&out, &content).expect("clean evidence must write");
        assert!(out.join("manifest.json").exists());
    }
}
