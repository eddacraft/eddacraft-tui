//! Installed `--help` alignment for GH #3921.
//!
//! Pins high-risk phrases so root, start, architecture, config, gate,
//! intercept, and MCP help matches current command behaviour.

use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

fn help(args: &[&str]) -> String {
    let out = Command::new(ANVIL_BIN)
        .args(args)
        .arg("--help")
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .output()
        .unwrap_or_else(|err| panic!("invoke anvil {} --help: {err}", args.join(" ")));
    assert!(
        out.status.success(),
        "anvil {} --help failed: stderr={}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn when_to_use_section(help: &str) -> &str {
    let start = help
        .find("WHEN TO USE:")
        .unwrap_or_else(|| panic!("help is missing WHEN TO USE footer:\n{help}"));
    let rest = &help[start..];
    let end = rest.find("COMMON FLAGS:").unwrap_or_else(|| {
        panic!("help is missing COMMON FLAGS footer after WHEN TO USE:\n{help}")
    });
    &rest[..end]
}

#[test]
fn root_help_start_summary_names_canonical_config_not_anvilrc() {
    let stdout = help(&[]);
    assert!(
        !stdout.contains("Writes `.anvilrc`"),
        "root help must not say start writes .anvilrc:\n{stdout}"
    );
    assert!(
        stdout.contains("Writes `.anvil.yaml`") || stdout.contains("Writes `.anvil.<ext>`"),
        "root start summary must name the canonical config file:\n{stdout}"
    );
}

#[test]
fn architecture_help_does_not_claim_watch_reads_only_standalone() {
    let stdout = help(&["architecture"]);
    let when = when_to_use_section(&stdout);
    assert!(
        !when.contains("still reads the standalone file, not the section"),
        "architecture help must not say watch is standalone-only:\n{stdout}"
    );
    assert!(
        when.contains("inline") || when.contains("architecture.source"),
        "architecture help must mention inline or delegated resolution:\n{stdout}"
    );
}

#[test]
fn config_set_help_documents_rule_and_mode_values() {
    let stdout = help(&["config", "set"]);
    assert!(
        stdout.contains("public-api-expansion")
            && stdout.contains("new-dependency-introduction")
            && stdout.contains("cross-layer-violation")
            && stdout.contains("privilege-expansion"),
        "config set help must list accepted rules:\n{stdout}"
    );
    assert!(
        stdout.contains("off") && stdout.contains("warn") && stdout.contains("enforce"),
        "config set help must list accepted modes:\n{stdout}"
    );
    assert!(
        stdout.contains("RULE") && stdout.contains("MODE"),
        "config set help must name <RULE> and <MODE>:\n{stdout}"
    );
}

#[test]
fn config_convert_help_is_about_format_not_rule_modes() {
    let stdout = help(&["config", "convert"]);
    let when = when_to_use_section(&stdout).to_ascii_lowercase();
    assert!(
        !when.contains("rule mode"),
        "config convert WHEN TO USE must not inherit rule-mode guidance:\n{stdout}"
    );
    assert!(
        stdout.contains(".anvil.") || stdout.to_ascii_lowercase().contains("format"),
        "config convert help must describe format conversion:\n{stdout}"
    );
}

#[test]
fn gate_help_explains_architecture_import_boundaries_alias() {
    let stdout = help(&["gate"]);
    assert!(
        stdout.contains("import-boundaries"),
        "gate help must name the canonical check:\n{stdout}"
    );
    assert!(
        stdout.contains("architecture"),
        "gate help must name the architecture alias:\n{stdout}"
    );
    assert!(
        stdout.contains("alias"),
        "gate help must explain the architecture mapping:\n{stdout}"
    );
}

#[test]
fn intercept_start_help_marks_foreground_required() {
    let stdout = help(&["intercept", "start"]);
    assert!(
        stdout.contains("--foreground"),
        "intercept start help must name --foreground:\n{stdout}"
    );
    assert!(
        !stdout.contains("[--foreground]"),
        "intercept start help must not present --foreground as optional:\n{stdout}"
    );
    assert!(
        stdout.to_ascii_lowercase().contains("required"),
        "intercept start help must say --foreground is required:\n{stdout}"
    );
}

#[test]
fn mcp_serve_help_describes_stdio_server_not_client_config() {
    let stdout = help(&["mcp", "serve"]);
    let lower = stdout.to_ascii_lowercase();
    assert!(
        lower.contains("stdio") || lower.contains("stdin"),
        "mcp serve help must describe stdio serving:\n{stdout}"
    );
    assert!(
        !lower.contains("preview") && !lower.contains("verify the existing client"),
        "mcp serve help must not describe install preview/verify:\n{stdout}"
    );
    let when = when_to_use_section(&stdout).to_ascii_lowercase();
    assert!(
        !when.contains("install") || when.contains("stdio"),
        "mcp serve WHEN TO USE must describe serving, not client install:\n{stdout}"
    );
}
