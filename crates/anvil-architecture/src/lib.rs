// Anvil architecture enforcement — boundary definitions, import rules, drift detection.

/// Standard Anvil configuration directory name.
pub const ANVIL_DIR: &str = ".anvil";

pub mod baseline;
pub mod definition;
pub mod detection;
pub mod python_detection;
pub mod python_resolve;
pub mod rust_resolve;
pub mod types;
mod util;
pub mod validator;
pub mod yaml_parser;

// Re-export key items for ergonomic use.
pub use baseline::{
    CreateBaselineOptions, create_baseline, find_fixed_violations, find_new_violations,
    load_baseline, merge_violations, save_baseline,
};
pub use definition::{
    ArchitectureDefinition, ArchitectureDefinitionDiagnostic,
    ArchitectureDefinitionDiagnosticSeverity, ArchitectureTemplate, diagnose_definition,
    get_available_templates, validate_definition,
};
pub use detection::detect_rust_entry_points;
pub use python_detection::detect_python_entry_points;
pub use python_resolve::resolve_python_import;
pub use rust_resolve::resolve_rust_import;
pub use types::{
    ArchitectureBaseline, Boundary, BoundarySeverity, BoundaryViolation, EntryPoint, Layer,
    LayerAssignment, Layers, create_default_boundaries, create_default_layers, create_violation_id,
    is_existing_violation,
};
pub use util::read_to_string_capped;
pub use validator::{
    ImportEdge, ValidationResult, assign_layers, collect_source_files, validate,
    validate_with_edges, validate_with_files_and_edges,
};
pub use yaml_parser::{
    ARCHITECTURE_YAML_MAX_SIZE, architecture_yaml_exists, create_definition_from_template,
    get_template_defaults, merge_with_template, parse_architecture_definition,
    parse_architecture_definition_file, write_architecture_yaml,
};
