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
}
