//! Fixture root resolution and content helpers for DEVACC.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Content keys used by guard / edit scripts (synthetic; never real secrets).
pub fn synthetic_content(key: &str) -> &'static str {
    match key {
        "secret_sample" => {
            // Synthetic near-miss only — not a real credential.
            "export const API_KEY = \"sk_live_EXAMPLE_NOT_REAL_000\";\n"
        }
        "boundary_violation" => {
            "import { insertOrder } from \"../store/orderStore.js\";\nexport function bad() { return insertOrder({ sku: \"x\", qty: 1, status: \"x\" }); }\n"
        }
        "clean_fix" => {
            "export function getOrderTotal(unitPrice: number, qty: number) {\n  return unitPrice * qty;\n}\n"
        }
        _ => "",
    }
}

pub fn fixture_root(repo: &Path, fixture_id: &str) -> PathBuf {
    repo.join("benchmarks/fixtures/devacc").join(fixture_id)
}

pub fn read_fixture_file(fixture: &Path, rel: &str) -> Result<String, String> {
    let path = fixture.join(rel);
    fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))
}

/// Walk a fixture directory (non-hidden) and return path names for `list_dir`.
pub fn list_relative(fixture: &Path, rel: &str) -> Result<Vec<String>, String> {
    let dir = if rel.is_empty() || rel == "." {
        fixture.to_path_buf()
    } else {
        fixture.join(rel)
    };
    let mut names = Vec::new();
    let rd = fs::read_dir(&dir).map_err(|e| format!("list_dir {}: {e}", dir.display()))?;
    for ent in rd {
        let ent = ent.map_err(|e| e.to_string())?;
        let name = ent.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        names.push(name);
    }
    names.sort();
    Ok(names)
}

/// Collect all source-like file contents under fixture for whole-tree estimates.
pub fn collect_source_files(fixture: &Path) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    walk(fixture, fixture, &mut out)?;
    Ok(out)
}

fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) -> Result<(), String> {
    for ent in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let ent = ent.map_err(|e| e.to_string())?;
        let path = ent.path();
        let name = ent.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name == "gold" || name == "node_modules" || name == "target" {
            continue;
        }
        if path.is_dir() {
            walk(root, &path, out)?;
        } else if is_source(&name) {
            let rel = path
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
            out.insert(rel, text);
        }
    }
    Ok(())
}

fn is_source(name: &str) -> bool {
    Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "ts" | "js" | "rs" | "md" | "yaml" | "yml" | "json"
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devacc::resolve_repo_root;

    #[test]
    fn devacc_fixtures_exist_and_load() {
        let root = resolve_repo_root(None).unwrap();
        for id in ["mini-ts-service", "mini-rs-lib", "mini-aps-plan"] {
            let f = fixture_root(&root, id);
            assert!(f.is_dir(), "missing fixture {id}");
            let files = collect_source_files(&f).unwrap();
            assert!(!files.is_empty(), "{id} has no source files");
        }
    }
}
