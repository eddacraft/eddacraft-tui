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
}
