//! Developer Acceleration (DEVACC) task-level benchmark suite.
//!
//! Measures assistant-facing token efficiency and success with Anvil on vs off.
//! Distinct from RLB (CPU/RSS) and GCTX-031 (payload micro-bench).

pub mod catalogue;
pub mod fixture;
pub mod report;
pub mod runner_a;
pub mod runner_b;

pub use catalogue::{Catalogue, ScenarioDef, load_catalogue};
pub use report::{DevaccReport, SCHEMA_VERSION};
pub use runner_a::{RunTierAOptions, run_tier_a};
pub use runner_b::{RunTierBOptions, run_tier_b};

use std::path::{Path, PathBuf};

/// Resolve the repository root that contains `benchmarks/devacc/catalogue.yaml`.
pub fn resolve_repo_root(explicit: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(p) = explicit {
        let cat = p.join("benchmarks/devacc/catalogue.yaml");
        if cat.is_file() {
            return Ok(p.to_path_buf());
        }
        return Err(format!(
            "catalogue not found at {}",
            cat.display()
        ));
    }

    let mut dir = std::env::current_dir().map_err(|e| e.to_string())?;
    loop {
        let cat = dir.join("benchmarks/devacc/catalogue.yaml");
        if cat.is_file() {
            return Ok(dir);
        }
        if !dir.pop() {
            break;
        }
    }
    Err(
        "could not locate benchmarks/devacc/catalogue.yaml from cwd or ancestors"
            .to_string(),
    )
}
