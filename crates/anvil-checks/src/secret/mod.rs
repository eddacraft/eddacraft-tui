pub mod check;
pub(crate) mod context;
pub mod entropy;
pub mod git_scanner;
pub mod patterns;
pub mod scanner;
pub(crate) mod source;
pub mod types;
pub mod vendored;

pub use check::{MAX_FILE_SIZE, is_secret_scannable, run_secret_check};
pub use entropy::{
    calculate_entropy, detect_high_entropy_strings, detect_high_entropy_strings_with_limit,
};
pub use git_scanner::scan_git_history;
pub use patterns::{
    BUILTIN_COMPILED_PATTERNS, CompiledPattern, DEFAULT_ALLOWLIST, DEFAULT_COMPILED_PATTERNS,
    PatternMatcher, SECRET_PATTERNS, compile_custom_patterns, compile_secret_patterns,
};
pub use scanner::{
    ScanStats, scan_content, scan_content_with_compiled_patterns, scan_content_with_limit,
    scan_content_with_limit_and_stats, scan_content_with_pattern_errors_and_stats,
    scan_content_with_stats, scan_file_with_compiled_patterns, scan_lockfile_url_credentials,
};
pub use types::{
    AllowlistProvenance, EntropyFinding, FindingType, SecretCheckConfig, SecretCheckResult,
    SecretFinding, SecretPatternDef, Suppression, TokenShape,
};
pub use vendored::{
    VENDORED_COMPILED_PATTERNS, VENDORED_RULESET, VENDORED_RULESET_VERSION, VendoredRule,
    VendoredRuleset, vendored_patterns_for, vendored_prefix_gate_is_active,
};
