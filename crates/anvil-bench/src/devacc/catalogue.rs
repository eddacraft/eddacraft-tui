//! Scenario catalogue loader (`benchmarks/devacc/catalogue.yaml`).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::report::SCHEMA_VERSION;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Catalogue {
    pub schema_version: String,
    pub arms: Vec<String>,
    pub classes: Vec<String>,
    pub scenarios: Vec<ScenarioDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScenarioDef {
    pub id: String,
    pub title: String,
    pub class: String,
    pub fixture: String,
    pub scale: String,
    pub arms: Vec<String>,
    pub tiers: Vec<String>,
    #[serde(default)]
    pub label: Option<String>,
    pub primary_metrics: Vec<String>,
    pub secondary_metrics: Vec<String>,
    pub gold: String,
    #[serde(default)]
    pub tier_a_scripts: BTreeMap<String, String>,
    #[serde(default)]
    pub notes: Option<String>,
}

pub fn load_catalogue(path: &Path) -> Result<Catalogue, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let cat: Catalogue =
        serde_yaml::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?;
    validate_catalogue(&cat)?;
    Ok(cat)
}

pub fn validate_catalogue(cat: &Catalogue) -> Result<(), String> {
    if cat.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "catalogue schema_version {} != expected {SCHEMA_VERSION}",
            cat.schema_version
        ));
    }
    if cat.scenarios.is_empty() {
        return Err("catalogue has no scenarios".into());
    }
    let mut ids = std::collections::BTreeSet::new();
    for sc in &cat.scenarios {
        if !sc.id.starts_with("DEVACC-SCN-") {
            return Err(format!("invalid scenario id: {}", sc.id));
        }
        if !ids.insert(sc.id.clone()) {
            return Err(format!("duplicate scenario id: {}", sc.id));
        }
        if sc.arms.is_empty() {
            return Err(format!("{} has no arms", sc.id));
        }
        if sc.tiers.is_empty() {
            return Err(format!("{} has no tiers", sc.id));
        }
        for arm in &sc.arms {
            if !cat.arms.iter().any(|a| a == arm) {
                return Err(format!("{} references unknown arm {arm}", sc.id));
            }
        }
        if sc.tiers.iter().any(|t| t == "A") && sc.tier_a_scripts.is_empty() {
            return Err(format!("{} has tier A but no tier_a_scripts", sc.id));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devacc::resolve_repo_root;

    #[test]
    fn devacc_catalogue_loads_and_validates() {
        let root = resolve_repo_root(None).expect("repo root");
        let path = root.join("benchmarks/devacc/catalogue.yaml");
        let cat = load_catalogue(&path).expect("load catalogue");
        assert_eq!(cat.schema_version, SCHEMA_VERSION);
        assert!(cat.scenarios.iter().any(|s| s.id == "DEVACC-SCN-01"));
        assert!(cat.scenarios.iter().any(|s| s.id == "DEVACC-SCN-40"));
        // Tier A coverage for the Ready-wave scenarios
        for id in [
            "DEVACC-SCN-01",
            "DEVACC-SCN-02",
            "DEVACC-SCN-04",
            "DEVACC-SCN-10",
            "DEVACC-SCN-30",
            "DEVACC-SCN-32",
        ] {
            let sc = cat.scenarios.iter().find(|s| s.id == id).unwrap();
            assert!(sc.tiers.iter().any(|t| t == "A"), "{id} missing tier A");
        }
    }
}
