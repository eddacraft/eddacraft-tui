pub mod check;
pub mod patterns;
pub mod scanner;
pub mod types;

pub use check::run_antipattern_check;
pub use patterns::{
    get_default_patterns, get_enabled_patterns, get_pattern, get_pattern_ids, is_valid_pattern_id,
    PATTERNS,
};
pub use scanner::{scan_file, scan_files, ScanOptions, ScanResult};
pub use types::{
    count_by_severity, create_warning_fingerprint, create_warning_result,
    validate_warning_result_consistency, AntiPattern, AntiPatternCategory, AntipatternCheckConfig,
    AntipatternCheckResult, Confidence, Location, Suppression, SuppressionScope, Warning,
    WarningCategory, WarningResult, WarningSeverity, WarningSummary,
};
