use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng, distr::Alphanumeric};

/// Language distribution weights for synthetic repo generation.
#[derive(Debug, Clone)]
pub struct LanguageMix {
    pub typescript_weight: u32,
    pub javascript_weight: u32,
    pub rust_weight: u32,
    pub json_weight: u32,
}

impl Default for LanguageMix {
    fn default() -> Self {
        Self {
            typescript_weight: 5,
            javascript_weight: 3,
            rust_weight: 1,
            json_weight: 1,
        }
    }
}

impl LanguageMix {
    fn total_weight(&self) -> u32 {
        self.typescript_weight + self.javascript_weight + self.rust_weight + self.json_weight
    }

    fn extension_for(&self, roll: u32) -> &'static str {
        let mut threshold = self.typescript_weight;
        if roll < threshold {
            return ".ts";
        }
        threshold += self.javascript_weight;
        if roll < threshold {
            return ".js";
        }
        threshold += self.rust_weight;
        if roll < threshold {
            return ".rs";
        }
        let _ = threshold;
        ".json"
    }
}

/// Specification for a synthetic repository.
#[derive(Debug, Clone)]
pub struct RepoSpec {
    pub file_count: usize,
    pub max_depth: usize,
    pub lines_per_file: usize,
    pub language_mix: LanguageMix,
    /// Seed for deterministic repo generation. Defaults to 42.
    pub seed: u64,
}

impl Default for RepoSpec {
    fn default() -> Self {
        Self {
            file_count: 100,
            max_depth: 4,
            lines_per_file: 50,
            language_mix: LanguageMix::default(),
            seed: 42,
        }
    }
}

impl RepoSpec {
    #[must_use]
    pub fn small() -> Self {
        Self {
            file_count: 50,
            max_depth: 2,
            lines_per_file: 30,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn medium() -> Self {
        Self {
            file_count: 500,
            max_depth: 5,
            lines_per_file: 100,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn large() -> Self {
        Self {
            file_count: 5_000,
            max_depth: 8,
            lines_per_file: 200,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn stress() -> Self {
        Self {
            file_count: 20_000,
            max_depth: 10,
            lines_per_file: 300,
            ..Self::default()
        }
    }
}

/// A generated synthetic repo on disk. The directory is cleaned up on drop
/// unless `into_path` is called.
pub struct SyntheticRepo {
    root: PathBuf,
    file_count: usize,
    own: bool,
}

impl SyntheticRepo {
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub fn file_count(&self) -> usize {
        self.file_count
    }

    /// Take ownership of the path -- caller is responsible for cleanup.
    #[must_use]
    pub fn into_path(mut self) -> PathBuf {
        self.own = false;
        self.root.clone()
    }
}

impl Drop for SyntheticRepo {
    fn drop(&mut self) {
        if self.own {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

/// Generate a synthetic repository matching the given spec.
///
/// Files are distributed across a directory tree with deterministic structure
/// seeded by `spec.seed`, using the language mix to determine file extensions.
///
/// Creates `base_dir/synthetic-repo` exclusively. If that path already exists,
/// returns [`std::io::ErrorKind::AlreadyExists`] without modifying it, so
/// [`SyntheticRepo`]'s drop cleanup cannot delete pre-existing content.
pub fn generate_repo(spec: &RepoSpec, base_dir: &Path) -> std::io::Result<SyntheticRepo> {
    let root = base_dir.join("synthetic-repo");
    // Ensure the parent exists, then create the leaf exclusively so we never
    // claim ownership of a directory that already contained user data.
    fs::create_dir_all(base_dir)?;
    match fs::create_dir(&root) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!(
                    "refusing to generate into existing directory: {}",
                    root.display()
                ),
            ));
        }
        Err(err) => return Err(err),
    }

    let generated = (|| -> std::io::Result<SyntheticRepo> {
        let mut rng = StdRng::seed_from_u64(spec.seed);
        let total_weight = spec.language_mix.total_weight();

        for i in 0..spec.file_count {
            let depth = if spec.max_depth == 0 {
                0
            } else {
                rng.random_range(0..=spec.max_depth)
            };

            let mut dir = root.clone();
            for d in 0..depth {
                let segment = format!("d{d}_{}", i % (d + 3).max(1));
                dir.push(segment);
            }
            fs::create_dir_all(&dir)?;

            let roll = rng.random_range(0..total_weight);
            let ext = spec.language_mix.extension_for(roll);

            let name_suffix: String = (0..6).map(|_| rng.sample(Alphanumeric) as char).collect();
            let filename = format!("file_{i}_{name_suffix}{ext}");

            let content = generate_file_content(ext, spec.lines_per_file, i);
            fs::write(dir.join(filename), content)?;
        }

        Ok(SyntheticRepo {
            root: root.clone(),
            file_count: spec.file_count,
            own: true,
        })
    })();

    match generated {
        Ok(repo) => Ok(repo),
        Err(err) => {
            // We created the directory; remove the partial tree rather than
            // leaving an orphan that a later call would refuse to reuse.
            let _ = fs::remove_dir_all(&root);
            Err(err)
        }
    }
}

fn generate_file_content(ext: &str, lines: usize, seed: usize) -> String {
    let mut buf = String::with_capacity(lines * 60);

    match ext {
        ".js" => generate_javascript(&mut buf, lines, seed),
        ".rs" => generate_rust_source(&mut buf, lines, seed),
        ".json" => generate_json(&mut buf, lines, seed),
        // .ts and anything else
        _ => generate_typescript(&mut buf, lines, seed),
    }

    buf
}

fn generate_typescript(buf: &mut String, lines: usize, seed: usize) {
    buf.push_str("import { readFileSync } from 'node:fs';\n");
    for i in 1..lines {
        let idx = seed.wrapping_mul(31).wrapping_add(i);
        match i % 8 {
            0 => {
                let _ = writeln!(buf, "type T{idx} = {{ id: string; value: number }};");
            }
            1 => {
                let _ = writeln!(buf, "const c{idx} = {idx};");
            }
            2 => {
                let _ = writeln!(
                    buf,
                    "function f{idx}(x: number): number {{ return x + {idx}; }}"
                );
            }
            3 => {
                let _ = writeln!(
                    buf,
                    "export class S{idx} {{ run(): void {{ /* noop */ }} }}"
                );
            }
            4 => {
                let _ = writeln!(buf, "const route{idx} = `/api/v1/{idx}`;");
            }
            5 => {
                let _ = writeln!(buf, "if (c{idx}) {{ void readFileSync('package.json'); }}");
            }
            6 => {
                let _ = writeln!(buf, "export const r{idx} = f{idx}({idx});");
            }
            _ => {
                let _ = writeln!(
                    buf,
                    "export function compute{idx}(a: number, b: number): number {{ return a + b + {idx}; }}"
                );
            }
        }
    }
}

fn generate_javascript(buf: &mut String, lines: usize, seed: usize) {
    buf.push_str("'use strict';\n");
    for i in 1..lines {
        let idx = seed.wrapping_mul(17).wrapping_add(i);
        match i % 6 {
            0 => {
                let _ = writeln!(buf, "const v{idx} = {idx};");
            }
            1 => {
                let _ = writeln!(buf, "function f{idx}(x) {{ return x + {idx}; }}");
            }
            2 => {
                let _ = writeln!(buf, "module.exports.f{idx} = f{idx};");
            }
            3 => {
                let _ = writeln!(buf, "const arr{idx} = Array({idx} % 100).fill(0);");
            }
            4 => {
                let _ = writeln!(
                    buf,
                    "class C{idx} {{ constructor() {{ this.id = {idx}; }} }}"
                );
            }
            _ => {
                let _ = writeln!(buf, "// line {idx}");
            }
        }
    }
}

fn generate_rust_source(buf: &mut String, lines: usize, seed: usize) {
    buf.push_str("#![allow(dead_code)]\n");
    for i in 1..lines {
        let idx = seed.wrapping_mul(23).wrapping_add(i);
        match i % 5 {
            0 => {
                let _ = writeln!(buf, "fn f{idx}(x: u64) -> u64 {{ x + {idx} }}");
            }
            1 => {
                let _ = writeln!(buf, "struct S{idx} {{ value: u64 }}");
            }
            2 => {
                let _ = writeln!(buf, "const C{idx}: u64 = {idx};");
            }
            3 => {
                let _ = writeln!(
                    buf,
                    "impl S{idx} {{ fn new() -> Self {{ Self {{ value: {idx} }} }} }}"
                );
            }
            _ => {
                let _ = writeln!(buf, "// comment {idx}");
            }
        }
    }
}

fn generate_json(buf: &mut String, lines: usize, seed: usize) {
    buf.push_str("{\n");
    let body_lines = lines.saturating_sub(2);
    for i in 0..body_lines {
        let idx = seed.wrapping_mul(7).wrapping_add(i);
        let comma = if i < body_lines.saturating_sub(1) {
            ","
        } else {
            ""
        };
        let _ = writeln!(buf, "  \"key_{idx}\": {idx}{comma}");
    }
    buf.push_str("}\n");
}

/// Count all files under a directory recursively.
pub fn count_files(root: &Path) -> usize {
    let mut count = 0;
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if let Ok(ft) = entry.file_type() {
                if ft.is_dir() {
                    stack.push(entry.path());
                } else if ft.is_file() {
                    count += 1;
                }
            }
        }
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_correct_file_count() {
        let dir = tempfile::tempdir().unwrap();
        let spec = RepoSpec {
            file_count: 20,
            max_depth: 2,
            lines_per_file: 10,
            ..RepoSpec::default()
        };

        let repo = generate_repo(&spec, dir.path()).unwrap();
        assert_eq!(repo.file_count(), 20);
        assert_eq!(count_files(repo.root()), 20);
    }

    #[test]
    fn generates_mixed_extensions() {
        let dir = tempfile::tempdir().unwrap();
        let spec = RepoSpec {
            file_count: 100,
            max_depth: 3,
            lines_per_file: 10,
            ..RepoSpec::default()
        };

        let repo = generate_repo(&spec, dir.path()).unwrap();
        let extensions = collect_extensions(repo.root());

        assert!(extensions.contains("ts"), "expected at least one .ts file");
        assert!(extensions.contains("js"), "expected at least one .js file");
    }

    fn collect_extensions(root: &Path) -> std::collections::HashSet<String> {
        let mut exts = std::collections::HashSet::new();
        let mut stack = vec![root.to_path_buf()];

        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(&dir).unwrap().flatten() {
                let ft = entry.file_type().unwrap();
                if ft.is_dir() {
                    stack.push(entry.path());
                } else if let Some(ext) = entry.path().extension() {
                    exts.insert(ext.to_string_lossy().to_string());
                }
            }
        }
        exts
    }

    #[test]
    fn cleanup_on_drop() {
        let dir = tempfile::tempdir().unwrap();
        let root_path;
        {
            let repo = generate_repo(
                &RepoSpec {
                    file_count: 5,
                    max_depth: 1,
                    lines_per_file: 5,
                    ..RepoSpec::default()
                },
                dir.path(),
            )
            .unwrap();
            root_path = repo.root().to_path_buf();
            assert!(root_path.exists());
        }
        assert!(
            !root_path.exists(),
            "directory should be cleaned up on drop"
        );
    }

    #[test]
    fn into_path_prevents_cleanup() {
        let dir = tempfile::tempdir().unwrap();
        let path = {
            let repo = generate_repo(
                &RepoSpec {
                    file_count: 3,
                    max_depth: 1,
                    lines_per_file: 5,
                    ..RepoSpec::default()
                },
                dir.path(),
            )
            .unwrap();
            repo.into_path()
        };
        assert!(path.exists(), "into_path should prevent cleanup");
        let _ = fs::remove_dir_all(&path);
    }

    #[test]
    fn rejects_preexisting_synthetic_repo_without_cleanup() {
        let dir = tempfile::tempdir().unwrap();
        let preexisting = dir.path().join("synthetic-repo");
        fs::create_dir_all(&preexisting).unwrap();
        let sentinel = preexisting.join("important.txt");
        fs::write(&sentinel, "do-not-delete").unwrap();

        let result = generate_repo(
            &RepoSpec {
                file_count: 3,
                max_depth: 1,
                lines_per_file: 5,
                ..RepoSpec::default()
            },
            dir.path(),
        );
        let err = match result {
            Ok(_repo) => panic!("must refuse an existing synthetic-repo directory"),
            Err(err) => err,
        };
        assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);

        assert_eq!(
            fs::read_to_string(&sentinel).unwrap(),
            "do-not-delete",
            "sentinel must remain unmodified after rejected generation"
        );
        assert!(
            preexisting.is_dir(),
            "pre-existing directory must remain after rejection"
        );
    }

    #[test]
    fn presets_have_increasing_sizes() {
        let small = RepoSpec::small();
        let medium = RepoSpec::medium();
        let large = RepoSpec::large();
        let stress = RepoSpec::stress();

        assert!(small.file_count < medium.file_count);
        assert!(medium.file_count < large.file_count);
        assert!(large.file_count < stress.file_count);
    }
}
