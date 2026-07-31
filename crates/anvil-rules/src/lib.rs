//! Rule-set hashing and version-floor primitives (MLP-012).
//!
//! `rules_sha` pins the rule version a witness line was produced under;
//! version-floor checks reject too-old rule packs.

mod input;
mod version;

pub use input::{
    OPA_RUNTIME_VERSION, RulesShaError, RulesShaInput, config_sha_from_canonical, rules_sha,
};
pub use version::{RequiredAnvilVersion, VersionFloorError};
