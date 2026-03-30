// Anvil architecture enforcement — boundary definitions, import rules, drift detection.

pub mod baseline;
pub mod definition;
pub mod types;
pub mod validator;
pub mod yaml_parser;

// Re-export key items for ergonomic use.
pub use baseline::{
    CreateBaselineOptions, create_baseline, find_fixed_violations, find_new_violations,
    load_baseline, merge_violations, save_baseline,
};
pub use definition::{
    ArchitectureDefinition, ArchitectureTemplate, get_available_templates, validate_definition,
};
pub use types::{
    ArchitectureBaseline, Boundary, BoundaryViolation, EntryPoint, Layer, LayerAssignment, Layers,
    create_default_boundaries, create_default_layers, create_violation_id, is_existing_violation,
};
pub use validator::{ImportEdge, ValidationResult, validate, validate_with_edges};
pub use yaml_parser::{
    architecture_yaml_exists, create_definition_from_template, get_template_defaults,
    merge_with_template, parse_architecture_definition, write_architecture_yaml,
};
