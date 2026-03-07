pub mod check;
pub mod entropy;
pub mod git_scanner;
pub mod patterns;
pub mod scanner;
pub mod types;

pub use check::run_secret_check;
pub use entropy::{calculate_entropy, detect_high_entropy_strings};
pub use git_scanner::scan_git_history;
pub use patterns::{compile_secret_patterns, PatternMatcher, DEFAULT_ALLOWLIST, SECRET_PATTERNS};
pub use scanner::scan_content;
pub use types::{
    EntropyFinding, FindingType, SecretCheckConfig, SecretCheckResult, SecretFinding,
    SecretPatternDef,
};
