pub mod check;
pub mod patterns;
pub mod registry_loader;
pub mod scanner;
pub mod types;

pub use registry_loader::{
    CompiledPattern, CompiledRegistry, Detection, FamilyEntry, LoadRegistryOptions,
    LoadRegistryResult, compiled_to_antipattern, load_compiled_registry, load_registry_patterns,
    reset_registry_cache,
};

pub use check::run_antipattern_check;
pub use patterns::{
    PATTERNS, get_default_patterns, get_enabled_patterns, get_pattern, get_pattern_ids,
    is_valid_pattern_id,
};
pub use scanner::{ScanOptions, ScanResult, scan_file, scan_files};
pub use types::{
    AntiPattern, AntiPatternCategory, AntipatternCheckConfig, AntipatternCheckResult, Confidence,
    Location, Suppression, SuppressionScope, Warning, WarningCategory, WarningResult,
    WarningSeverity, WarningSummary, count_by_severity, create_warning_fingerprint,
    create_warning_result, validate_warning_result_consistency,
};
