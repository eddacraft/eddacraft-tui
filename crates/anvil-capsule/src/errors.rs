//! Error type shared by the capsule schema surface.

/// Errors from parsing, validating, or encoding capsule artefacts.
#[derive(Debug, thiserror::Error)]
pub enum CapsuleError {
    /// The document declared a schema this crate does not speak.
    /// Schema evolution is gate-by-version (ADR-074), never silent.
    #[error("unsupported schema: expected `{expected}`, found `{found}`")]
    SchemaMismatch {
        /// The schema identifier this crate supports.
        expected: &'static str,
        /// The schema identifier the document declared.
        found: String,
    },
    /// The document is not valid JSON for the declared schema.
    #[error("parse error: {0}")]
    Parse(String),
    /// The value could not be encoded (practically unreachable).
    #[error("serialisation error: {0}")]
    Serialise(String),
    /// A `git` invocation failed while collecting evidence — spawn
    /// failure, unresolvable ref, non-zero exit, or non-UTF-8 output
    /// (including changed paths, which are never lossily rewritten).
    #[error("git error: {0}")]
    Git(String),
    /// The prune surface refused an invocation or could not read the
    /// staging root (ADR-078).
    #[error("prune error: {0}")]
    Prune(String),
    /// A verification document's stored `verdict` disagrees with the
    /// worst-of derivation over its `checks` — a tampered or
    /// hand-assembled document trying to launder a verdict.
    #[error("inconsistent verdict: document claims `{claimed}`, checks derive `{derived}`")]
    InconsistentVerdict {
        /// The verdict the document stored.
        claimed: String,
        /// The verdict derived from the document's own checks.
        derived: String,
    },
    /// An evidence source exists on disk but could not be read or
    /// parsed during collection. This fails loudly — a capsule must
    /// not misrepresent present-but-broken governance state as
    /// absence. (A source that simply does not exist is **not** this
    /// error: it collects as an absent field, and absence is the
    /// verifier's `degraded` signal.)
    #[error("cannot collect `{path}`: {detail}")]
    Collect {
        /// Repo-relative path of the source that failed.
        path: String,
        /// What went wrong reading or parsing it. The wrapped error
        /// text may embed **absolute** filesystem paths — fine for
        /// local CLI output, but never embed this in a capsule
        /// artefact or transmit it.
        detail: String,
    },
    /// The rule identity inputs could not be combined into a
    /// `rules_sha` (invalid rule id or config digest).
    #[error("rules identity error: {0}")]
    RulesIdentity(String),
    /// Scan-on-write refused the capsule: secret-shaped content was
    /// found in evidence bound for a durable file (ADR-072 §3 — no
    /// secrets in durable Git evidence). Creation fails **before** any
    /// evidence file is written, so secret-bearing evidence never
    /// reaches a tracked write (GITGOV-012).
    #[error(
        "refusing to write capsule: {count} secret-shaped finding(s) in evidence file `{file}` \
         (ADR-072 §3: durable evidence must not contain secrets)"
    )]
    SecretInEvidence {
        /// Capsule-relative evidence file the finding(s) were in.
        file: String,
        /// Number of secret findings detected in that file.
        count: usize,
    },
}
