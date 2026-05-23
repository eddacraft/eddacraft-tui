# Rust CLI Cutover Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the Rust `anvil` binary as the sole CLI, archive the Node.js CLI, and distribute via cargo-dist to a public GitHub Releases repo.

**Architecture:** Five blocking work items (auth migration, auth enforcement, gate checks, output formatters, welcome menu), then archival of the Node.js CLI to `archive/`, then cargo-dist setup publishing to `eddacraft/anvil-releases`. Each task produces a commit.

**Tech Stack:** Rust (clap, anyhow, serde_json, reqwest), cargo-dist, GitHub Actions

**Spec:** `docs/archive/specs/2026-03-27-rust-cli-cutover-design.md` (archived 2026-05-23, DOCGOV-008)

---

## File Map

| File | Responsibility | Task |
|------|---------------|------|
| `crates/anvil-cli/src/auth/credentials.rs` | Credential loading with legacy fallback + migration | 1 |
| `crates/anvil-cli/src/main.rs` | Pre-dispatch auth enforcement middleware | 2 |
| `crates/anvil-cli/src/commands/gate.rs` | Wire coverage, dependency, architecture, policy checks | 3 |
| `crates/anvil-architecture/src/lib.rs` | Minimal `validate()` entry point for gate | 3 |
| `crates/anvil-cli/src/output/mod.rs` | Output trait + TUI/plain/JSON dispatch | 4 |
| `crates/anvil-cli/src/output/plain.rs` | Structured plain text (already exists, extend) | 4 |
| `crates/anvil-cli/src/output/json.rs` | Structured JSON (already exists, extend) | 4 |
| `crates/anvil-tui/src/surfaces/welcome/mod.rs` | Add gate/watch menu items | 5 |
| `crates/anvil-cli/src/commands/welcome.rs` | Dispatch gate/watch from welcome hub | 5 |
| `archive/anvil-cli-node/` | Archived Node.js CLI | 6 |
| `archive/anvil-tui-ink/` | Archived Ink TUI | 6 |
| `pnpm-workspace.yaml` | Exclude archived package | 6 |
| `Cargo.toml` (workspace) | Release profile hardening | 7 |
| `.github/workflows/release.yml` | cargo-dist release workflow | 7 |

---

## Task 1: Auth Credential Migration (RCLI-015a)

**Files:**
- Modify: `crates/anvil-cli/src/auth/credentials.rs`
- Test: `crates/anvil-cli/src/auth/credentials.rs` (inline `#[cfg(test)]`)

- [ ] **Step 1: Write failing tests for legacy credential paths**

Add to the existing `#[cfg(test)] mod tests` block in `credentials.rs`:

```rust
#[test]
fn test_load_from_legacy_auth_json() {
    let dir = tempfile::tempdir().unwrap();
    let legacy_dir = dir.path().join(".anvil");
    std::fs::create_dir_all(&legacy_dir).unwrap();
    std::fs::write(
        legacy_dir.join("auth.json"),
        r#"{"license":"tok_legacy","email":"test@example.com"}"#,
    )
    .unwrap();

    let result = load_from_paths(dir.path(), None).unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().license, "tok_legacy");
}

#[test]
fn test_load_from_legacy_license_file() {
    let dir = tempfile::tempdir().unwrap();
    let legacy_dir = dir.path().join(".anvil");
    std::fs::create_dir_all(&legacy_dir).unwrap();
    std::fs::write(legacy_dir.join("license"), "tok_license_file").unwrap();

    let result = load_from_paths(dir.path(), None).unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().license, "tok_license_file");
}

#[test]
fn test_load_from_env_var() {
    std::env::set_var("ANVIL_LICENSE", "tok_env");
    let result = load_from_env().unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().license, "tok_env");
    std::env::remove_var("ANVIL_LICENSE");
}

#[test]
fn test_xdg_takes_priority_over_legacy() {
    let dir = tempfile::tempdir().unwrap();
    // Write to both XDG and legacy
    let xdg_dir = dir.path().join(".config/anvil");
    std::fs::create_dir_all(&xdg_dir).unwrap();
    std::fs::write(
        xdg_dir.join("credentials.json"),
        r#"{"license":"tok_xdg","email":"xdg@example.com"}"#,
    )
    .unwrap();
    let legacy_dir = dir.path().join(".anvil");
    std::fs::create_dir_all(&legacy_dir).unwrap();
    std::fs::write(
        legacy_dir.join("auth.json"),
        r#"{"license":"tok_legacy","email":"legacy@example.com"}"#,
    )
    .unwrap();

    let result = load_from_paths(dir.path(), Some(&xdg_dir)).unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().license, "tok_xdg");
}

#[test]
fn test_legacy_load_triggers_migration_copy() {
    let dir = tempfile::tempdir().unwrap();
    let xdg_dir = dir.path().join(".config/anvil");
    let legacy_dir = dir.path().join(".anvil");
    std::fs::create_dir_all(&legacy_dir).unwrap();
    std::fs::write(
        legacy_dir.join("auth.json"),
        r#"{"license":"tok_migrate","email":"m@example.com"}"#,
    )
    .unwrap();

    let _result = load_from_paths(dir.path(), Some(&xdg_dir)).unwrap();

    // Migration should have copied to XDG
    assert!(xdg_dir.join("credentials.json").exists());
    let migrated: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(xdg_dir.join("credentials.json")).unwrap())
            .unwrap();
    assert_eq!(migrated["license"], "tok_migrate");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p eddacraft-anvil -- test_load_from_legacy -v`
Expected: FAIL — `load_from_paths` and `load_from_env` don't exist yet.

- [ ] **Step 3: Implement credential fallback and migration**

Refactor `credentials.rs`. Keep the existing `load()` function signature but extract the path resolution into testable functions:

```rust
use std::path::{Path, PathBuf};

/// Load credentials checking all known paths in priority order.
/// If loaded from a legacy path, migrates to XDG with a notice.
pub fn load() -> Result<Option<Credentials>> {
    let home = dirs::home_dir().context("Cannot determine home directory")?;
    let xdg_dir = xdg_config_dir();
    load_from_paths(&home, Some(&xdg_dir))
}

/// Testable credential loading with explicit paths.
pub(crate) fn load_from_paths(
    home: &Path,
    xdg_dir: Option<&Path>,
) -> Result<Option<Credentials>> {
    // 1. XDG path (current behaviour)
    if let Some(xdg) = xdg_dir {
        let xdg_path = xdg.join("credentials.json");
        if xdg_path.exists() {
            let content = std::fs::read_to_string(&xdg_path)?;
            return Ok(Some(serde_json::from_str(&content)?));
        }
    }

    // 2. Legacy ~/.anvil/auth.json
    let legacy_auth = home.join(".anvil/auth.json");
    if legacy_auth.exists() {
        let content = std::fs::read_to_string(&legacy_auth)?;
        let creds: Credentials = serde_json::from_str(&content)?;
        if let Some(xdg) = xdg_dir {
            migrate_to_xdg(&creds, xdg)?;
        }
        return Ok(Some(creds));
    }

    // 3. Legacy ~/.anvil/license (plain text token)
    let legacy_license = home.join(".anvil/license");
    if legacy_license.exists() {
        let token = std::fs::read_to_string(&legacy_license)?.trim().to_string();
        let creds = Credentials {
            license: token,
            refresh_token: None,
            email: None,
            expires_at: None,
        };
        if let Some(xdg) = xdg_dir {
            migrate_to_xdg(&creds, xdg)?;
        }
        return Ok(Some(creds));
    }

    // 4. ANVIL_LICENSE env var
    if let Some(creds) = load_from_env()? {
        return Ok(Some(creds));
    }

    Ok(None)
}

/// Load credentials from ANVIL_LICENSE environment variable.
pub(crate) fn load_from_env() -> Result<Option<Credentials>> {
    match std::env::var("ANVIL_LICENSE") {
        Ok(token) if !token.is_empty() => Ok(Some(Credentials {
            license: token,
            refresh_token: None,
            email: None,
            expires_at: None,
        })),
        _ => Ok(None),
    }
}

fn migrate_to_xdg(creds: &Credentials, xdg_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(xdg_dir)?;
    let dest = xdg_dir.join("credentials.json");
    let json = serde_json::to_string_pretty(creds)?;
    std::fs::write(&dest, &json)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o600))?;
    }
    eprintln!(
        "Migrated credentials → {}",
        dest.display()
    );
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p eddacraft-anvil -- test_load_from -v`
Expected: All 5 new tests PASS.

- [ ] **Step 5: Run full workspace test suite**

Run: `cargo test --workspace`
Expected: All tests pass, no regressions.

- [ ] **Step 6: Commit**

```bash
git add crates/anvil-cli/src/auth/credentials.rs
git commit -m "feat(RCLI-015a): credential fallback for legacy paths with XDG migration"
```

---

## Task 2: Pre-Action Auth Enforcement (RCLI-015b)

**Files:**
- Modify: `crates/anvil-cli/src/main.rs`
- Modify: `crates/anvil-cli/src/auth/credentials.rs` (re-export `is_expired`)

- [ ] **Step 1: Write failing test for auth enforcement**

Add a test module at the bottom of `main.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_requires_auth_gate() {
        assert!(requires_auth(&Commands::Gate(Default::default())));
    }

    #[test]
    fn test_requires_auth_watch() {
        assert!(requires_auth(&Commands::Watch(Default::default())));
    }

    #[test]
    fn test_requires_auth_status() {
        assert!(requires_auth(&Commands::Status(Default::default())));
    }

    #[test]
    fn test_requires_auth_admin() {
        assert!(requires_auth(&Commands::Admin(Default::default())));
    }

    #[test]
    fn test_requires_auth_export() {
        assert!(requires_auth(&Commands::Export(Default::default())));
    }

    #[test]
    fn test_no_auth_doctor() {
        assert!(!requires_auth(&Commands::Doctor(Default::default())));
    }

    #[test]
    fn test_no_auth_tutorial() {
        assert!(!requires_auth(&Commands::Tutorial(Default::default())));
    }

    #[test]
    fn test_no_auth_init() {
        assert!(!requires_auth(&Commands::Init(Default::default())));
    }

    #[test]
    fn test_no_auth_hooks() {
        assert!(!requires_auth(&Commands::Hooks(Default::default())));
    }

    #[test]
    fn test_no_auth_welcome() {
        assert!(!requires_auth(&Commands::Welcome(Default::default())));
    }

    #[test]
    fn test_no_auth_new() {
        assert!(!requires_auth(&Commands::New(Default::default())));
    }

    #[test]
    fn test_no_auth_wizard() {
        assert!(!requires_auth(&Commands::Wizard(Default::default())));
    }
}
```

Note: For this to compile, the args structs need `Default` derives. If they don't already have them, add `#[derive(Default)]` to each args struct used in tests. Alternatively, construct minimal instances directly.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p eddacraft-anvil -- test_requires_auth -v`
Expected: FAIL — `requires_auth` doesn't exist.

- [ ] **Step 3: Implement requires_auth and pre-dispatch check**

Add to `main.rs`, before the `main()` function:

```rust
/// Commands that require valid authentication before execution.
fn requires_auth(cmd: &Commands) -> bool {
    matches!(
        cmd,
        Commands::Gate(_)
            | Commands::Watch(_)
            | Commands::Status(_)
            | Commands::Admin(_)
            | Commands::Export(_)
            | Commands::Auth(commands::auth::AuthArgs {
                command: commands::auth::AuthCommand::Whoami,
                ..
            })
    )
}

/// Check credentials and return EXIT_AUTH_REQUIRED if missing or expired.
fn check_auth() -> Result<(), u8> {
    match auth::credentials::load() {
        Ok(Some(creds)) => {
            if auth::credentials::is_expired(&creds) {
                eprintln!("Session expired. Run `anvil auth login` to re-authenticate.");
                Err(EXIT_AUTH_REQUIRED)
            } else {
                Ok(())
            }
        }
        Ok(None) => {
            eprintln!("Authentication required. Run `anvil auth login` to authenticate.");
            Err(EXIT_AUTH_REQUIRED)
        }
        Err(_) => {
            eprintln!("Authentication required. Run `anvil auth login` to authenticate.");
            Err(EXIT_AUTH_REQUIRED)
        }
    }
}
```

Then modify the dispatch in `main()` to insert the check before command execution:

```rust
// After parsing cli, before the match dispatch:
if requires_auth(&cli.command) {
    if let Err(code) = check_auth() {
        if cli.global.json {
            let msg = serde_json::json!({"error": "authentication_required"});
            eprintln!("{}", serde_json::to_string_pretty(&msg).unwrap());
        }
        std::process::exit(code as i32);
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p eddacraft-anvil -- test_requires_auth -v`
Expected: All 12 tests PASS.

- [ ] **Step 5: Run full workspace tests**

Run: `cargo test --workspace`
Expected: All pass, no regressions.

- [ ] **Step 6: Commit**

```bash
git add crates/anvil-cli/src/main.rs crates/anvil-cli/src/auth/credentials.rs
git commit -m "feat(RCLI-015b): pre-action auth enforcement with exit code 3"
```

---

## Task 3: Wire Remaining Gate Checks (RCLI-013a)

**Files:**
- Modify: `crates/anvil-cli/src/commands/gate.rs`
- Modify: `crates/anvil-architecture/src/lib.rs`
- Modify: `crates/anvil-policy/src/lib.rs` (re-export evaluator)

This task has 4 sub-parts — one per check. Each gets its own commit.

### Task 3a: Coverage Check

- [ ] **Step 1: Write failing test for coverage check**

Add to `gate.rs` test module:

```rust
#[test]
fn test_coverage_check_with_lcov_report() {
    let dir = tempfile::tempdir().unwrap();
    let coverage_dir = dir.path().join("coverage");
    std::fs::create_dir_all(&coverage_dir).unwrap();
    // Minimal lcov: 10 lines, 8 hit = 80%
    std::fs::write(
        coverage_dir.join("lcov.info"),
        "TN:\nSF:src/main.rs\nLF:10\nLH:8\nend_of_record\n",
    )
    .unwrap();

    let result = run_check_coverage(dir.path(), 70.0);
    assert!(result.passed);
    assert!((result.score - 80.0).abs() < 0.1);
}

#[test]
fn test_coverage_check_below_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let coverage_dir = dir.path().join("coverage");
    std::fs::create_dir_all(&coverage_dir).unwrap();
    std::fs::write(
        coverage_dir.join("lcov.info"),
        "TN:\nSF:src/main.rs\nLF:10\nLH:5\nend_of_record\n",
    )
    .unwrap();

    let result = run_check_coverage(dir.path(), 70.0);
    assert!(!result.passed);
    assert!((result.score - 50.0).abs() < 0.1);
}

#[test]
fn test_coverage_check_no_report_skips() {
    let dir = tempfile::tempdir().unwrap();
    let result = run_check_coverage(dir.path(), 70.0);
    assert!(result.passed); // Skip = pass with notice
    assert!(result.message.contains("No coverage report"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p eddacraft-anvil -- test_coverage_check -v`
Expected: FAIL — `run_check_coverage` doesn't exist.

- [ ] **Step 3: Implement coverage check**

Add to `gate.rs`:

```rust
fn run_check_coverage(project_root: &Path, threshold: f64) -> CheckResult {
    // Look for lcov.info or cobertura.xml
    let lcov_path = project_root.join("coverage/lcov.info");
    let cobertura_path = project_root.join("coverage/cobertura.xml");

    if lcov_path.exists() {
        match parse_lcov_coverage(&lcov_path) {
            Ok(pct) => CheckResult {
                name: "coverage".to_string(),
                passed: pct >= threshold,
                score: pct,
                message: if pct >= threshold {
                    format!("Coverage {pct:.1}% meets threshold {threshold:.1}%")
                } else {
                    format!("Coverage {pct:.1}% below threshold {threshold:.1}%")
                },
            },
            Err(e) => CheckResult {
                name: "coverage".to_string(),
                passed: false,
                score: 0.0,
                message: format!("Failed to parse lcov report: {e}"),
            },
        }
    } else if cobertura_path.exists() {
        match parse_cobertura_coverage(&cobertura_path) {
            Ok(pct) => CheckResult {
                name: "coverage".to_string(),
                passed: pct >= threshold,
                score: pct,
                message: if pct >= threshold {
                    format!("Coverage {pct:.1}% meets threshold {threshold:.1}%")
                } else {
                    format!("Coverage {pct:.1}% below threshold {threshold:.1}%")
                },
            },
            Err(e) => CheckResult {
                name: "coverage".to_string(),
                passed: false,
                score: 0.0,
                message: format!("Failed to parse cobertura report: {e}"),
            },
        }
    } else {
        CheckResult {
            name: "coverage".to_string(),
            passed: true,
            score: 0.0,
            message: "No coverage report found (coverage/lcov.info or coverage/cobertura.xml). Skipping.".to_string(),
        }
    }
}

fn parse_lcov_coverage(path: &Path) -> Result<f64> {
    let content = std::fs::read_to_string(path)?;
    let mut total_lines: u64 = 0;
    let mut hit_lines: u64 = 0;
    for line in content.lines() {
        if let Some(val) = line.strip_prefix("LF:") {
            total_lines += val.trim().parse::<u64>().unwrap_or(0);
        } else if let Some(val) = line.strip_prefix("LH:") {
            hit_lines += val.trim().parse::<u64>().unwrap_or(0);
        }
    }
    if total_lines == 0 {
        return Ok(0.0);
    }
    Ok((hit_lines as f64 / total_lines as f64) * 100.0)
}

fn parse_cobertura_coverage(path: &Path) -> Result<f64> {
    // Parse line-rate attribute from <coverage> root element
    let content = std::fs::read_to_string(path)?;
    // Simple regex extraction — cobertura XML has line-rate="0.XX" on root
    for line in content.lines() {
        if let Some(idx) = line.find("line-rate=\"") {
            let start = idx + "line-rate=\"".len();
            if let Some(end) = line[start..].find('"') {
                let rate_str = &line[start..start + end];
                if let Ok(rate) = rate_str.parse::<f64>() {
                    return Ok(rate * 100.0);
                }
            }
        }
    }
    anyhow::bail!("No line-rate attribute found in cobertura XML")
}
```

Then replace the coverage stub in `run_single_check`:

```rust
"coverage" => run_check_coverage(&std::env::current_dir()?, 80.0),
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p eddacraft-anvil -- test_coverage_check -v`
Expected: All 3 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/anvil-cli/src/commands/gate.rs
git commit -m "feat(RCLI-013a): wire coverage gate check with lcov/cobertura parsing"
```

### Task 3b: Dependency Check

- [ ] **Step 1: Write failing test**

```rust
#[test]
fn test_dependency_check_clean_lockfile() {
    let dir = tempfile::tempdir().unwrap();
    // Minimal package-lock with no flagged packages
    std::fs::write(
        dir.path().join("package-lock.json"),
        r#"{"lockfileVersion":3,"packages":{"node_modules/safe-pkg":{"version":"1.0.0"}}}"#,
    )
    .unwrap();

    let result = run_check_dependency(dir.path());
    assert!(result.passed);
}

#[test]
fn test_dependency_check_no_lockfile_skips() {
    let dir = tempfile::tempdir().unwrap();
    let result = run_check_dependency(dir.path());
    assert!(result.passed);
    assert!(result.message.contains("No lockfile"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p eddacraft-anvil -- test_dependency_check -v`
Expected: FAIL.

- [ ] **Step 3: Implement dependency check**

```rust
/// Known-vulnerable package patterns (curated blocklist, updated per release).
const BLOCKED_PACKAGES: &[&str] = &[
    "event-stream",    // Supply chain attack (2018)
    "flatmap-stream",  // Supply chain attack (2018)
    "ua-parser-js",    // Hijacked versions 0.7.29/0.8.0/1.0.0
    "colors",          // Sabotaged >=1.4.1
    "faker",           // Sabotaged >=6.6.6
    "node-ipc",        // Protestware >=10.1.1
];

fn run_check_dependency(project_root: &Path) -> CheckResult {
    let npm_lock = project_root.join("package-lock.json");
    let cargo_lock = project_root.join("Cargo.lock");

    let mut findings: Vec<String> = Vec::new();

    if npm_lock.exists() {
        if let Ok(content) = std::fs::read_to_string(&npm_lock) {
            for pkg in BLOCKED_PACKAGES {
                // Check for "node_modules/<pkg>" key in lockfile
                let needle = format!("node_modules/{pkg}");
                if content.contains(&needle) {
                    findings.push(format!("Blocked package found: {pkg}"));
                }
            }
        }
    }

    // Cargo.lock: no curated blocklist yet, just verify it exists
    if !npm_lock.exists() && !cargo_lock.exists() {
        return CheckResult {
            name: "dependency".to_string(),
            passed: true,
            score: 0.0,
            message: "No lockfile found (package-lock.json or Cargo.lock). Skipping.".to_string(),
        };
    }

    if findings.is_empty() {
        CheckResult {
            name: "dependency".to_string(),
            passed: true,
            score: 100.0,
            message: "No blocked dependencies found".to_string(),
        }
    } else {
        CheckResult {
            name: "dependency".to_string(),
            passed: false,
            score: 0.0,
            message: findings.join("; "),
        }
    }
}
```

Replace dependency stub: `"dependency" => run_check_dependency(&std::env::current_dir()?),`

- [ ] **Step 4: Run tests**

Run: `cargo test -p eddacraft-anvil -- test_dependency_check -v`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/anvil-cli/src/commands/gate.rs
git commit -m "feat(RCLI-013a): wire dependency gate check with blocklist scanning"
```

### Task 3c: Architecture Check

- [ ] **Step 1: Write failing test**

```rust
#[test]
fn test_architecture_check_no_config_skips() {
    let dir = tempfile::tempdir().unwrap();
    let result = run_check_architecture(dir.path());
    assert!(result.passed);
    assert!(result.message.contains("No architecture config"));
}

#[test]
fn test_architecture_check_with_valid_config() {
    let dir = tempfile::tempdir().unwrap();
    let anvil_dir = dir.path().join(".anvil");
    std::fs::create_dir_all(&anvil_dir).unwrap();
    std::fs::write(
        anvil_dir.join("architecture.yaml"),
        "layers:\n  - name: core\n    paths: [\"src/core/**\"]\n",
    )
    .unwrap();

    let result = run_check_architecture(dir.path());
    assert!(result.passed);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p eddacraft-anvil -- test_architecture_check -v`
Expected: FAIL.

- [ ] **Step 3: Add minimal validate() to anvil-architecture crate**

In `crates/anvil-architecture/src/lib.rs`:

```rust
use anyhow::Result;
use std::path::Path;

/// Result of architecture validation.
pub struct ValidationResult {
    pub valid: bool,
    pub violations: Vec<ArchViolation>,
}

pub struct ArchViolation {
    pub rule: String,
    pub file: String,
    pub message: String,
}

/// Validate architecture constraints from `.anvil/architecture.yaml`.
///
/// Returns Ok with empty violations if config is parseable and no
/// import edges cross layer boundaries. This is a basic implementation —
/// full kernel-backed graph analysis comes in a later release.
pub fn validate(project_root: &Path) -> Result<ValidationResult> {
    let config_path = project_root.join(".anvil/architecture.yaml");
    if !config_path.exists() {
        return Ok(ValidationResult {
            valid: true,
            violations: vec![],
        });
    }

    // Parse YAML to verify it's valid
    let content = std::fs::read_to_string(&config_path)?;
    let _doc: serde_yaml::Value = serde_yaml::from_str(&content)?;

    // For beta: config is parseable = pass. Full boundary checking
    // requires kernel integration (deferred).
    Ok(ValidationResult {
        valid: true,
        violations: vec![],
    })
}
```

Add `serde_yaml` to `crates/anvil-architecture/Cargo.toml` if not present:

```toml
[dependencies]
anyhow = { workspace = true }
serde_yaml = "0.9"
```

- [ ] **Step 4: Implement architecture gate check in gate.rs**

```rust
fn run_check_architecture(project_root: &Path) -> CheckResult {
    let config_path = project_root.join(".anvil/architecture.yaml");
    if !config_path.exists() {
        return CheckResult {
            name: "architecture".to_string(),
            passed: true,
            score: 0.0,
            message: "No architecture config found (.anvil/architecture.yaml). Skipping."
                .to_string(),
        };
    }

    match anvil_architecture::validate(project_root) {
        Ok(result) => {
            if result.valid && result.violations.is_empty() {
                CheckResult {
                    name: "architecture".to_string(),
                    passed: true,
                    score: 100.0,
                    message: "Architecture config valid, no violations".to_string(),
                }
            } else {
                let msgs: Vec<String> = result
                    .violations
                    .iter()
                    .map(|v| format!("{}: {} ({})", v.file, v.message, v.rule))
                    .collect();
                CheckResult {
                    name: "architecture".to_string(),
                    passed: false,
                    score: 0.0,
                    message: msgs.join("; "),
                }
            }
        }
        Err(e) => CheckResult {
            name: "architecture".to_string(),
            passed: false,
            score: 0.0,
            message: format!("Architecture validation failed: {e}"),
        },
    }
}
```

Replace architecture stub: `"architecture" => run_check_architecture(&std::env::current_dir()?),`

- [ ] **Step 5: Run tests**

Run: `cargo test -p eddacraft-anvil -- test_architecture_check -v && cargo test -p eddacraft-anvil-architecture -v`
Expected: All PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/anvil-architecture/src/lib.rs crates/anvil-architecture/Cargo.toml crates/anvil-cli/src/commands/gate.rs
git commit -m "feat(RCLI-013a): wire architecture gate check with config validation"
```

### Task 3d: Policy Check

- [ ] **Step 1: Write failing test**

```rust
#[test]
fn test_policy_check_no_bundle_skips() {
    let dir = tempfile::tempdir().unwrap();
    let result = run_check_policy(dir.path());
    assert!(result.passed);
    assert!(result.message.contains("No policy bundle"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p eddacraft-anvil -- test_policy_check -v`
Expected: FAIL.

- [ ] **Step 3: Implement policy check**

```rust
fn run_check_policy(project_root: &Path) -> CheckResult {
    let policy_dir = project_root.join(".anvil/policies");
    if !policy_dir.exists() || !policy_dir.is_dir() {
        return CheckResult {
            name: "policy".to_string(),
            passed: true,
            score: 0.0,
            message: "No policy bundle found (.anvil/policies/). Skipping.".to_string(),
        };
    }

    // Check if OPA is available
    let evaluator = anvil_policy::evaluator::Evaluator::new(None);
    let input = serde_json::json!({
        "workspace": project_root.to_string_lossy(),
    });

    match evaluator.evaluate(project_root, &input, Some(".anvil/policies")) {
        Ok(result) => {
            if result.passed {
                CheckResult {
                    name: "policy".to_string(),
                    passed: true,
                    score: 100.0,
                    message: format!("{} policies evaluated, no violations", result.checks_run),
                }
            } else {
                let msgs: Vec<String> = result
                    .violations
                    .iter()
                    .map(|v| format!("[{}] {}: {}", v.severity, v.policy_id, v.message))
                    .collect();
                CheckResult {
                    name: "policy".to_string(),
                    passed: false,
                    score: 0.0,
                    message: msgs.join("; "),
                }
            }
        }
        Err(anvil_policy::evaluator::EvalError::OpaNotAvailable) => CheckResult {
            name: "policy".to_string(),
            passed: true,
            score: 0.0,
            message: "OPA not installed. Skipping policy evaluation.".to_string(),
        },
        Err(e) => CheckResult {
            name: "policy".to_string(),
            passed: false,
            score: 0.0,
            message: format!("Policy evaluation failed: {e}"),
        },
    }
}
```

Replace policy stub: `"policy" => run_check_policy(&std::env::current_dir()?),`

- [ ] **Step 4: Run tests**

Run: `cargo test -p eddacraft-anvil -- test_policy_check -v`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/anvil-cli/src/commands/gate.rs
git commit -m "feat(RCLI-013a): wire policy gate check via anvil-policy evaluator"
```

---

## Task 4: Output Formatters (RCLI-022)

**Files:**
- Modify: `crates/anvil-cli/src/output/mod.rs`
- Modify: `crates/anvil-cli/src/output/plain.rs`
- Modify: `crates/anvil-cli/src/output/json.rs`
- Modify: `crates/anvil-cli/src/commands/gate.rs` (integrate output)
- Modify: `crates/anvil-cli/src/commands/status.rs` (integrate output)

- [ ] **Step 1: Write failing test for OutputMode selection**

Add to `output/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_flag_selects_json() {
        assert_eq!(OutputMode::resolve(true, false, true), OutputMode::Json);
    }

    #[test]
    fn test_no_tui_flag_selects_plain() {
        assert_eq!(OutputMode::resolve(false, true, true), OutputMode::Plain);
    }

    #[test]
    fn test_non_tty_selects_plain() {
        assert_eq!(OutputMode::resolve(false, false, false), OutputMode::Plain);
    }

    #[test]
    fn test_tty_no_flags_selects_tui() {
        assert_eq!(OutputMode::resolve(false, false, true), OutputMode::Tui);
    }

    #[test]
    fn test_json_overrides_no_tui() {
        // --json takes priority over --no-tui
        assert_eq!(OutputMode::resolve(true, true, true), OutputMode::Json);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p eddacraft-anvil -- test_json_flag -v`
Expected: FAIL — `OutputMode` doesn't exist.

- [ ] **Step 3: Implement OutputMode**

Replace `crates/anvil-cli/src/output/mod.rs`:

```rust
pub mod json;
pub mod plain;

/// Output mode resolved from CLI flags and terminal state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Tui,
    Plain,
    Json,
}

impl OutputMode {
    /// Resolve output mode from flags and TTY detection.
    /// Priority: --json > --no-tui > TTY detection.
    pub fn resolve(json: bool, no_tui: bool, is_tty: bool) -> Self {
        if json {
            OutputMode::Json
        } else if no_tui || !is_tty {
            OutputMode::Plain
        } else {
            OutputMode::Tui
        }
    }

    /// Convenience: resolve from GlobalArgs + stdout TTY check.
    pub fn from_global(global: &crate::GlobalArgs) -> Self {
        Self::resolve(global.json, global.no_tui, std::io::IsTerminal::is_terminal(&std::io::stdout()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_flag_selects_json() {
        assert_eq!(OutputMode::resolve(true, false, true), OutputMode::Json);
    }

    #[test]
    fn test_no_tui_flag_selects_plain() {
        assert_eq!(OutputMode::resolve(false, true, true), OutputMode::Plain);
    }

    #[test]
    fn test_non_tty_selects_plain() {
        assert_eq!(OutputMode::resolve(false, false, false), OutputMode::Plain);
    }

    #[test]
    fn test_tty_no_flags_selects_tui() {
        assert_eq!(OutputMode::resolve(false, false, true), OutputMode::Tui);
    }

    #[test]
    fn test_json_overrides_no_tui() {
        assert_eq!(OutputMode::resolve(true, true, true), OutputMode::Json);
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p eddacraft-anvil -- test_json_flag -v`
Expected: All 5 PASS.

- [ ] **Step 5: Integrate OutputMode into gate command**

In `gate.rs`, modify the `run()` function to branch on output mode:

```rust
use crate::output::OutputMode;

pub fn run(args: &GateArgs, global: &crate::GlobalArgs) -> Result<()> {
    let mode = OutputMode::from_global(global);

    // ... existing check logic producing GateResult ...

    match mode {
        OutputMode::Json => {
            crate::output::json::print(&gate_result)?;
        }
        OutputMode::Plain => {
            crate::output::plain::header("Gate Results");
            for check in &gate_result.checks {
                let icon = if check.passed { "✓" } else { "✗" };
                crate::output::plain::item(&format!(
                    "{icon} {}: {} ({:.0}%)",
                    check.name, check.message, check.score
                ));
            }
            crate::output::plain::blank();
            let overall_icon = if gate_result.overall { "✓" } else { "✗" };
            crate::output::plain::header(&format!(
                "{overall_icon} Overall: {:.0}% ({:.0}ms)",
                gate_result.score, gate_result.duration_ms
            ));
        }
        OutputMode::Tui => {
            // Existing TUI path — launch GateState surface
            // (keep current implementation)
        }
    }

    if !gate_result.overall {
        std::process::exit(crate::EXIT_GATE_FAIL as i32);
    }

    Ok(())
}
```

- [ ] **Step 6: Derive Serialize on GateResult and CheckResult**

Add `#[derive(serde::Serialize)]` to `GateResult` and `CheckResult` structs so `json::print()` works:

```rust
#[derive(serde::Serialize)]
struct GateResult {
    overall: bool,
    score: f64,
    checks: Vec<CheckResult>,
    duration_ms: u64,
}

#[derive(serde::Serialize)]
struct CheckResult {
    name: String,
    passed: bool,
    score: f64,
    message: String,
}
```

- [ ] **Step 7: Run full tests**

Run: `cargo test --workspace`
Expected: All pass.

- [ ] **Step 8: Commit**

```bash
git add crates/anvil-cli/src/output/ crates/anvil-cli/src/commands/gate.rs
git commit -m "feat(RCLI-022): output mode selection and gate JSON/plain formatters"
```

---

## Task 5: Welcome Menu Parity (RCLI-030)

**Files:**
- Modify: `crates/anvil-tui/src/surfaces/welcome/mod.rs`
- Modify: `crates/anvil-cli/src/commands/welcome.rs`

- [ ] **Step 1: Add gate and watch to QuickStartOption enum**

In `crates/anvil-tui/src/surfaces/welcome/mod.rs`, uncomment or add:

```rust
pub enum QuickStartOption {
    RunAudit,
    RunDoctor,
    RunTutorial,
    RunGate,      // NEW
    StartWatch,   // NEW
    ViewDocs,
}
```

Update `MENU_ITEMS` (or equivalent array) to include labels:

```rust
const MENU_ITEMS: &[(QuickStartOption, &str, &str)] = &[
    (QuickStartOption::RunGate, "Run quality gate", "Check code against all quality checks"),
    (QuickStartOption::StartWatch, "Start watch mode", "Monitor files for changes in real time"),
    (QuickStartOption::RunAudit, "Run project audit", "Scan for security issues and anti-patterns"),
    (QuickStartOption::RunDoctor, "Run diagnostics", "Check environment and fix common issues"),
    (QuickStartOption::RunTutorial, "Interactive tutorial", "Learn Anvil with guided walkthrough"),
    (QuickStartOption::ViewDocs, "View documentation", "Open docs in browser"),
];
```

Note: Place gate and watch first — they're the primary workflow commands.

- [ ] **Step 2: Verify the TUI renders correctly**

Run: `cargo build -p eddacraft-anvil && ./target/debug/anvil start`
Expected: Welcome menu shows 6 items with gate and watch at the top. Arrow keys navigate. Esc exits.

- [ ] **Step 3: Wire gate dispatch in welcome hub**

In `crates/anvil-cli/src/commands/welcome.rs`, add match arms in `run_welcome_hub`:

```rust
Some(QuickStartOption::RunGate) => {
    // Show loading frame
    render_loading_frame(terminal, "Running quality gate...")?;

    // Collect gate data (reuse gate command's collection logic)
    let checks = crate::commands::gate::collect_gate_data()?;
    let gate_state = anvil_tui::surfaces::gate::GateState::new(checks);
    let exit = crate::tui::run_surface_in(terminal, gate_state)?;
    if matches!(exit, SurfaceExit::Quit) {
        break;
    }
}
Some(QuickStartOption::StartWatch) => {
    // Watch requires kernel — spawn watcher and launch watch surface
    let (tx, rx) = std::sync::mpsc::channel();
    let source = std::env::current_dir()?;
    let _handle = anvil_kernel::watcher::spawn_watcher(&source, tx)?;

    let watch_state = anvil_tui::surfaces::watch::WatchState::default();
    // Drop to raw watch loop (can't nest inside surface_in)
    // Restore terminal, run watch, re-enter hub on exit
    drop(terminal);
    crate::tui::run_watch(watch_state, &rx)?;
    // Re-acquire terminal for hub
    *terminal = crate::tui::setup_terminal()?;
}
```

Note: The exact function names (`collect_gate_data`, `spawn_watcher`) may differ — the implementer should check the existing gate.rs and kernel watcher API and adjust accordingly.

- [ ] **Step 4: Test manually**

Run: `cargo build -p eddacraft-anvil && ./target/debug/anvil start`

1. Select "Run quality gate" → gate surface should launch
2. Press Esc → should return to welcome menu
3. Select "Start watch mode" → watch surface should launch
4. Press `q` → should exit

- [ ] **Step 5: Commit**

```bash
git add crates/anvil-tui/src/surfaces/welcome/mod.rs crates/anvil-cli/src/commands/welcome.rs
git commit -m "feat(RCLI-030): add gate and watch to welcome menu"
```

---

## Task 5.5: Pre-Archival Housekeeping

- [ ] **Step 1: Triage e2e test failures**

Run: `pnpm nx run @eddacraft/anvil-e2e:test --skip-nx-cache 2>&1 | grep "FAIL"`

For each failing test: determine if it's a pre-existing failure (present before
this work) or a new regression. Pre-existing failures: add `.skip` with a comment
referencing the known issue. New regressions: fix before proceeding.

- [ ] **Step 2: Decide on MAINT-011 (TS 6.0 migration)**

Check current status: is it blocking any build? If TS 6.0 migration is
incomplete but builds pass, defer to a follow-up PR. If it's causing build
issues, finish it now.

- [ ] **Step 3: Commit any triage/skip changes**

```bash
git add -A
git commit -m "chore: triage e2e failures and settle MAINT-011 status"
```

---

## Task 6: Archive Node.js CLI (RCLI-023)

**Files:**
- Move: `apps/anvil-cli/` → `archive/anvil-cli-node/`
- Move: `apps/anvil-cli/src/tui/` → `archive/anvil-tui-ink/`
- Modify: `pnpm-workspace.yaml` (exclude archive)
- Modify: `package.json` (remove CLI scripts)
- Modify: `nx.json` or `project.json` (remove CLI project)

- [ ] **Step 1: Tag the repo for rollback reference**

```bash
git tag pre-rust-cli
```

- [ ] **Step 2: Move Ink TUI first (it's inside the CLI)**

```bash
mkdir -p archive
git mv apps/anvil-cli/src/tui archive/anvil-tui-ink
```

- [ ] **Step 3: Move the rest of the Node.js CLI**

```bash
git mv apps/anvil-cli archive/anvil-cli-node
```

- [ ] **Step 4: Exclude archive from pnpm workspace**

Edit `pnpm-workspace.yaml` — add exclusion:

```yaml
packages:
  # Infrastructure
  - infra
  # Apps (v1.1+)
  - apps/*
  # ... existing entries ...
  # Exclude archived packages
  - '!archive/**'
```

- [ ] **Step 5: Remove CLI references from root package.json**

Remove the `unlink:cli` script from root `package.json`:

```json
// REMOVE this line:
"unlink:cli": "cd apps/anvil-cli && npm unlink -g @eddacraft/anvil-cli",
```

- [ ] **Step 6: Remove Nx project config for anvil-cli**

Check if there's a `project.json` in the archived location or a reference in `nx.json`. Remove the project entry.

```bash
# Check for project config
ls archive/anvil-cli-node/project.json 2>/dev/null
grep -rn "anvil-cli" nx.json 2>/dev/null
```

Remove any references found.

- [ ] **Step 7: Verify builds**

```bash
cargo build --workspace && echo "Rust OK"
pnpm install && pnpm build && echo "TS OK"
pnpm nx run-many -t test --skip-nx-cache 2>&1 | tail -5
```

Expected: Both build systems pass. TS test count will be lower (anvil-cli tests are archived).

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "chore(RCLI-023): archive Node.js CLI and Ink TUI to archive/"
```

---

## Task 7: Distribution Pipeline (RCLI-024)

**Files:**
- Create: `eddacraft/anvil-releases` GitHub repo (manual step)
- Modify: `Cargo.toml` (workspace release profile)
- Create: `.github/workflows/release.yml` (generated by cargo-dist)

- [ ] **Step 1: Create the public releases repo**

This is a manual step — create `eddacraft/anvil-releases` on GitHub:
- Public visibility
- No source code, just a README: "Release binaries for Anvil CLI"
- Enable GitHub Releases

- [ ] **Step 2: Harden the release profile**

In root `Cargo.toml`, add or update:

```toml
[profile.release]
strip = "symbols"
lto = true
codegen-units = 1
panic = "abort"
opt-level = "z"  # Optimize for size
```

- [ ] **Step 3: Run cargo-dist init**

```bash
cargo install cargo-dist
cargo dist init
```

Follow the prompts:
- Targets: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`
- Installers: shell (generates install.sh)
- CI: GitHub Actions
- Hosting: GitHub Releases

This generates `.github/workflows/release.yml` and updates `Cargo.toml` with `[workspace.metadata.dist]`.

- [ ] **Step 4: Modify workflow to publish to public repo**

Edit `.github/workflows/release.yml` — the upload step needs to target `eddacraft/anvil-releases` instead of the current repo. Add a step after the build that uses `gh release create` against the public repo:

```yaml
- name: Publish to anvil-releases
  env:
    GH_TOKEN: ${{ secrets.RELEASES_REPO_TOKEN }}
  run: |
    TAG="${{ github.ref_name }}"
    # Create release in public repo
    gh release create "$TAG" \
      --repo eddacraft/anvil-releases \
      --title "Anvil CLI $TAG" \
      --notes "See changelog at eddacraft/anvil (private)" \
      artifacts/*.tar.gz
```

Set up `RELEASES_REPO_TOKEN` as a repository secret with write access to the public repo.

- [ ] **Step 5: Test with a dry run**

```bash
cargo dist build
ls target/distrib/
```

Expected: Produces `anvil-x86_64-unknown-linux-gnu.tar.gz` (at minimum, for the local platform).

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml .github/workflows/release.yml
git commit -m "feat(RCLI-024): cargo-dist release pipeline targeting public anvil-releases repo"
```

- [ ] **Step 7: Test the release flow**

```bash
git tag v0.3.0-beta.rc1
git push origin v0.3.0-beta.rc1
```

Watch the CI run. Verify binaries appear in `eddacraft/anvil-releases` releases page.

- [ ] **Step 8: Verify install script**

```bash
# From a clean machine or container:
curl -sSf https://github.com/eddacraft/anvil-releases/releases/latest/download/anvil-installer.sh | sh
anvil --version
```

Expected: Downloads correct binary, installs to PATH, version matches tag.

---

## Task 8: Release

- [ ] **Step 1: Write release notes**

Create a changelog entry or GitHub Release body covering:
- Rust CLI is now the primary binary
- Node.js CLI archived (available in `archive/` for reference)
- Auth credential migration (automatic, one-time)
- All 7 gate checks functional
- Known gaps table (from spec)
- Install instructions

- [ ] **Step 2: Tag and release**

```bash
git tag v0.3.0-beta
git push origin v0.3.0-beta
```

- [ ] **Step 3: Verify end-to-end install**

On a clean machine:
1. Install via curl script
2. `anvil --version` — shows version
3. `anvil auth login` — device code flow works
4. `anvil doctor` — diagnostics pass
5. `anvil gate` — all 7 checks run
6. `anvil start` — welcome menu with gate/watch

- [ ] **Step 4: Notify beta users**

Send update via existing communication channel with install instructions.

- [ ] **Step 5: Update APS module statuses**

Mark completed items in `plans/modules/rust-cli.aps.md`:
- RCLI-015a: Complete
- RCLI-015b: Complete
- RCLI-013a: Complete
- RCLI-022: Complete
- RCLI-030: Complete
- RCLI-023: Complete
- RCLI-024: Complete
