//! Integration tests for the secret detection module.
//!
//! These tests exercise the public API (`anvil_checks::secret::*`) with
//! realistic file content that mirrors what developers actually commit.

use anvil_checks::secret::{SecretCheckConfig, calculate_entropy, run_secret_check, scan_content};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn default_config() -> SecretCheckConfig {
    SecretCheckConfig::default()
}

fn config_with_entropy(threshold: f64) -> SecretCheckConfig {
    SecretCheckConfig {
        enable_entropy: true,
        entropy_threshold: threshold,
        ..SecretCheckConfig::default()
    }
}

fn config_without_entropy() -> SecretCheckConfig {
    SecretCheckConfig {
        enable_entropy: false,
        ..SecretCheckConfig::default()
    }
}

fn temp_dir(label: &str) -> std::path::PathBuf {
    let unique = format!(
        "anvil-checks-integ-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    );
    let path = std::env::temp_dir().join(unique);
    let _ = std::fs::create_dir_all(&path);
    path
}

// ---------------------------------------------------------------------------
// scan_content — realistic TypeScript / config files
// ---------------------------------------------------------------------------

#[test]
fn detects_aws_secret_key_in_config_module() {
    // Before #1800, the bare `AKIA…` pattern was suppressed by the
    // `looks_like_code` filter and by the `example` keyword allowlist
    // — only the assignment-anchored AWS Secret Key path survived. The
    // bare `AKIA…` shape is now high-confidence and surfaces on its
    // own; this test still asserts the secret-access-key shape because
    // that's what the production-snippet fixture contains.
    let content = r"
import { S3Client } from '@aws-sdk/client-s3';

const client = new S3Client({
  region: 'eu-west-2',
  credentials: {
    aws_secret_access_key: 'abcdabcdabcdabcdabcdabcdabcdabcdabcdabcd',
  },
});

export default client;
";

    let findings = scan_content(content, "src/infra/s3.ts", &default_config());
    assert!(
        findings.iter().any(|f| f.pattern_name == "AWS Secret Key"),
        "should detect the AWS secret access key"
    );
}

#[test]
fn detects_github_token_in_env_file() {
    let token = format!("ghp_{}", "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0");
    let content = format!("# CI credentials\nGITHUB_TOKEN={token}\nNODE_ENV=production\n");

    let findings = scan_content(&content, ".env.production", &default_config());
    assert!(
        findings.iter().any(|f| f.pattern_name == "GitHub Token"),
        "should detect the GitHub personal access token"
    );
}

#[test]
fn detects_stripe_live_key_among_normal_code() {
    let stripe_key = format!("sk_live_{}", "1234567890abcdefghijABCD");
    let content = format!(
        r"
import Stripe from 'stripe';

// Initialise Stripe with production key
const stripe = new Stripe('{stripe_key}', {{
  apiVersion: '2024-06-20',
}});

export async function createPaymentIntent(amount: number) {{
  return stripe.paymentIntents.create({{ amount, currency: 'gbp' }});
}}
"
    );

    let findings = scan_content(&content, "src/payments/stripe.ts", &default_config());
    assert!(
        findings.iter().any(|f| f.pattern_name == "Stripe Key"),
        "should detect the Stripe live secret key"
    );
    // The redacted output must not leak the full key
    for finding in &findings {
        assert!(
            !finding.redacted_match.contains(&stripe_key),
            "redacted_match must not contain the full key"
        );
    }
}

#[test]
fn detects_generic_password_in_config() {
    // Exercises the Generic Secret pattern. The keyword-driven Generic
    // Secret rule remains low-confidence (#1800) so the existing FP
    // filters keep guarding it.
    let content = "password='super-s3cret-value!'\n";

    let findings = scan_content(content, "config/db.ts", &default_config());
    assert!(
        findings.iter().any(|f| f.pattern_name == "Generic Secret"),
        "should detect the password assignment, got: {:?}",
        findings.iter().map(|f| &f.pattern_name).collect::<Vec<_>>()
    );
}

#[test]
fn detects_private_key_header() {
    let content = "\
-----BEGIN RSA PRIVATE KEY-----\n\
MIIEpAIBAAKCAQEA0Z3VS5JJcds3xfn/ygWe\n\
-----END RSA PRIVATE KEY-----\n";

    let findings = scan_content(content, "certs/server.pem", &default_config());
    assert!(
        findings.iter().any(|f| f.pattern_name == "Private Key"),
        "should detect RSA private key header"
    );
}

#[test]
fn detects_sendgrid_api_key() {
    // SendGrid keys have a distinctive SG.xxx.yyy format. As a
    // high-confidence shape (#1800) they bypass the `looks_like_code`
    // filter, the same as the AWS / GitHub / Stripe shapes.
    let content =
        "const key = 'SG.1234567890123456789012.1234567890123456789012345678901234567890123';\n";

    let findings = scan_content(content, "src/email/config.ts", &default_config());
    assert!(
        findings
            .iter()
            .any(|f| f.pattern_name == "SendGrid API Key"),
        "should detect SendGrid API key, got patterns: {:?}",
        findings.iter().map(|f| &f.pattern_name).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// False-positive avoidance
// ---------------------------------------------------------------------------

#[test]
fn does_not_flag_placeholder_values() {
    let content = r"
const config = {
  apiKey: 'placeholder-api-key-for-testing',
  secret: 'example-secret-value',
  token: 'test-token-for-local-dev',
};
";

    let findings = scan_content(content, "src/config.ts", &config_without_entropy());
    assert!(
        findings.is_empty(),
        "placeholder / example / test values should be allowlisted, got {} findings",
        findings.len()
    );
}

#[test]
fn does_not_flag_code_identifiers_as_entropy() {
    let content = r"
import { readFileSync } from 'node:fs';
const serviceEndpoint = 'https://api.eddacraft.dev/v2/gate';
function buildPayloadForArchitecture(id: string): string {
  return `${id}-processed`;
}
";

    let findings = scan_content(content, "src/utils.ts", &config_with_entropy(3.0));
    assert!(
        findings.is_empty(),
        "code-like identifiers and URLs should not trigger entropy findings"
    );
}

#[test]
fn does_not_flag_credit_card_digits_inside_https_url_path() {
    // CIB-323: facebook reel-shaped paths carry a 16-digit id that matches
    // the Credit Card regex and can be Luhn-valid. Path context is not a card.
    // Stripe test PAN groups (already a fixture in this crate) — assembled
    // at runtime so the source never contains a 16-digit run.
    let digits = ["4242"; 4].join("");
    let config = config_without_entropy();

    let url = format!("const reel = 'https://www.facebook.com/reel/{digits}';\n");
    let url_findings = scan_content(&url, "src/share.ts", &config);
    assert!(
        !url_findings.iter().any(|f| f.pattern_name == "Credit Card"),
        "16-digit run in an https URL path must not flag as Credit Card, got: {:?}",
        url_findings
            .iter()
            .map(|f| &f.pattern_name)
            .collect::<Vec<_>>(),
    );

    let standalone = format!("const card = '{digits}';\n");
    let card_findings = scan_content(&standalone, "src/payments.ts", &config);
    assert!(
        card_findings
            .iter()
            .any(|f| f.pattern_name == "Credit Card"),
        "standalone 16-digit card-shaped token must still flag"
    );
}

#[test]
fn does_not_flag_hex_hashes_via_shape_allowlist() {
    // CLAWP-063: exercise the entropy path AND the hex-shape allowlist
    // (`^[a-f0-9]{64}$`) this test protects. Two subtleties the prior
    // form missed: (1) the hash must sit in a quoted/assignment position
    // or the entropy extractor never captures it as a candidate (a bare
    // `sha512-<hash>` token is invisible to the scanner, so scoring it was
    // vacuous regardless of threshold); (2) suppression must be proven to
    // come from the shape allowlist, not from filename/extension handling.
    //
    // Uses a regular source filename, NOT a lockfile name: lockfiles now get a
    // URL-credential-only scan with no entropy pass (GH #2584), so a lockfile
    // filename would make this test vacuous. Lockfile hex-hash silence is
    // covered separately in `secret::check` / `welcome` tests.
    let hex64 = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
    let entropy_count = |body: &str| -> usize {
        scan_content(body, "src/integrity.ts", &config_with_entropy(3.0))
            .iter()
            .filter(|f| f.pattern_name == "High Entropy String")
            .count()
    };

    // The 64-char hex hash is extracted (quoted assignment) and must be
    // suppressed by the shape allowlist.
    assert_eq!(
        entropy_count(&format!("const integrity = '{hex64}'\n")),
        0,
        "a 64-char hex hash must be allowlisted even with entropy enabled"
    );

    // Control (non-vacuity): the SAME length, SAME position, SAME
    // filename — only the leading char is non-hex, so it fails the
    // `^[a-f0-9]{64}$` shape allowlist. It MUST fire, proving the clean
    // result above is the allowlist discriminating on shape, not the
    // scanner being silent (extension-skip / non-extraction).
    let not_hex = format!("z{}", &hex64[1..]);
    assert!(
        entropy_count(&format!("const integrity = '{not_hex}'\n")) >= 1,
        "a same-length non-hex high-entropy value must fire (else this test is vacuous)"
    );
}

// ---------------------------------------------------------------------------
// Entropy detection
// ---------------------------------------------------------------------------

#[test]
fn entropy_catches_high_entropy_unquoted_assignment() {
    // A realistic token that is not matched by known patterns
    let content = "const sessionKey = 'Qm9kR3p4VnNNdkxaWlhTamtCdQ==';\n";

    let findings = scan_content(content, "src/session.ts", &config_with_entropy(3.5));
    assert!(
        !findings.is_empty(),
        "high-entropy base64-like string should be caught"
    );
}

#[test]
fn entropy_is_zero_for_repeated_characters() {
    assert!((calculate_entropy("aaaaaaaaaaaaaaaa") - 0.0).abs() < f64::EPSILON);
}

#[test]
fn entropy_scales_with_character_variety() {
    let low = calculate_entropy("aabbccdd");
    let high = calculate_entropy("9xY7qW2vK8mN4pR6");
    assert!(high > low, "more varied string should have higher entropy");
}

// ---------------------------------------------------------------------------
// Mixed content — pattern and entropy together
// ---------------------------------------------------------------------------

#[test]
fn mixed_file_finds_both_pattern_and_entropy_findings() {
    // Use a Stripe live key for the production-config narrative.
    // (Stripe test keys are now also high-confidence per #1800 — the
    // "test" keyword no longer suppresses them.)
    let stripe_key = format!("sk_live_{}", "1234567890abcdefghijABCD");
    let content = format!(
        r"
// Config for production environment
export const STRIPE_KEY = '{stripe_key}';

// Random session nonce (not a known pattern)
export const NONCE = 'Qm9kR3p4VnNNdkxaWlhTamtCdQ==';

export const APP_NAME = 'Anvil';
"
    );

    let findings = scan_content(&content, "src/config.ts", &config_with_entropy(3.5));
    let has_pattern = findings
        .iter()
        .any(|f| f.finding_type == anvil_checks::secret::FindingType::Pattern);
    let has_entropy = findings
        .iter()
        .any(|f| f.finding_type == anvil_checks::secret::FindingType::Entropy);

    assert!(has_pattern, "should find the Stripe live key via pattern");
    assert!(has_entropy, "should find the nonce via entropy");
}

#[test]
fn entropy_findings_are_suppressed_on_lines_already_matched_by_pattern() {
    // Use a Stripe live key for the production-config narrative.
    // (Stripe test keys are now also high-confidence per #1800 — the
    // "test" keyword no longer suppresses them.)
    let stripe_key = format!("sk_live_{}", "1234567890abcdefghijABCD");
    let content = format!("const key = '{stripe_key}';\n");

    let findings = scan_content(&content, "src/keys.ts", &config_with_entropy(2.0));
    // Even with a very low entropy threshold, the line already matched by the
    // Stripe pattern should not produce a duplicate entropy finding.
    let entropy_on_stripe_line: Vec<_> = findings
        .iter()
        .filter(|f| f.finding_type == anvil_checks::secret::FindingType::Entropy && f.line == 1)
        .collect();
    assert!(
        entropy_on_stripe_line.is_empty(),
        "entropy findings should be suppressed when a pattern already matched the line"
    );
}

// ---------------------------------------------------------------------------
// run_secret_check — file-level orchestration
// ---------------------------------------------------------------------------

#[test]
fn run_secret_check_on_clean_files_passes() {
    let dir = temp_dir("clean-check");
    let f1 = dir.join("app.ts");
    let f2 = dir.join("utils.ts");
    std::fs::write(&f1, "export const VERSION = '1.0.0';").unwrap();
    std::fs::write(
        &f2,
        "export function add(a: number, b: number) { return a + b; }",
    )
    .unwrap();

    let f1s = f1.to_string_lossy().to_string();
    let f2s = f2.to_string_lossy().to_string();
    let files = [f1s.as_str(), f2s.as_str()];
    let result = run_secret_check(&files, &default_config(), None);

    assert!(result.passed);
    assert_eq!(result.score, 100);
    assert!(result.message.contains("No secrets detected"));

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn run_secret_check_scores_degrade_with_findings() {
    let dir = temp_dir("score-degrade");
    let f = dir.join("leaked.ts");
    // Use an aws_secret_access_key assignment plus a Stripe key.
    // (The bare AKIA shape is high-confidence post-#1800 and would
    // also fire on its own; this fixture exercises the assignment
    // path because that's how secret leaks usually appear in code.)
    let aws_secret = "abcdabcdabcdabcdabcdabcdabcdabcdabcdabcd";
    let stripe_key = format!("sk_live_{}", "1234567890abcdefghijABCD");
    let content =
        format!("aws_secret_access_key='{aws_secret}';\nconst secret = '{stripe_key}';\n");
    std::fs::write(&f, content).unwrap();

    let fs = f.to_string_lossy().to_string();
    let files = [fs.as_str()];
    let result = run_secret_check(&files, &default_config(), None);

    assert!(!result.passed);
    assert!(result.score < 100, "score should be penalised");
    assert!(
        result
            .findings
            .iter()
            .any(|f| f.pattern_name == "AWS Secret Key"),
        "should detect the AWS secret access key"
    );
    assert!(
        result
            .findings
            .iter()
            .any(|f| f.pattern_name == "Stripe Key"),
        "should detect the Stripe live key"
    );
    assert!(result.findings.len() >= 2, "should find at least 2 secrets");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn run_secret_check_skips_binary_extensions() {
    let dir = temp_dir("skip-ext");
    let f = dir.join("icon.png");
    // Even if we write secret-like content into a .png, it should be skipped
    std::fs::write(&f, "api_key='AKIAABCDEFGHIJKLMNOP'").unwrap();

    let fs = f.to_string_lossy().to_string();
    let files = [fs.as_str()];
    let result = run_secret_check(&files, &default_config(), None);

    assert!(result.passed);
    assert_eq!(result.findings.len(), 0);

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn run_secret_check_normalises_paths_relative_to_workspace() {
    let dir = temp_dir("normalise");
    let f = dir.join("src").join("config.ts");
    let _ = std::fs::create_dir_all(f.parent().unwrap());
    // Build the test secret at runtime so it never appears as a string literal
    // and suppress logging of detection results (CodeQL cwe-312).
    let test_secret = format!("{}_{}", "synthetic", "8charVal");
    std::fs::write(&f, format!("password='{test_secret}'")).unwrap();

    let fs = f.to_string_lossy().to_string();
    let files = [fs.as_str()];
    let ws = dir.to_string_lossy().to_string();
    let result = run_secret_check(&files, &default_config(), Some(&ws));

    assert!(!result.passed);
    // Validate path normalisation without logging finding details
    let file_path = &result.findings[0].file;
    assert!(
        file_path.starts_with('/'),
        "file path should be normalised to start with /"
    );
    assert!(
        file_path.contains("src/config.ts"),
        "path should include relative components"
    );

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn custom_patterns_extend_detection() {
    let config = SecretCheckConfig {
        enable_entropy: false,
        custom_patterns: vec![anvil_checks::secret::SecretPatternDef {
            name: "Internal API Token".to_string(),
            pattern: r"anvil_tok_[a-zA-Z0-9]{20,}".to_string(),
        }],
        ..SecretCheckConfig::default()
    };

    let content = "const token = 'anvil_tok_abcdefghij1234567890';";
    let findings = scan_content(content, "src/auth.ts", &config);

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].pattern_name, "Internal API Token");
}

#[test]
fn custom_allowlist_suppresses_known_safe_values() {
    let config = SecretCheckConfig {
        enable_entropy: false,
        custom_allowlist: vec!["sk_test_.*".to_string()],
        ..SecretCheckConfig::default()
    };

    let stripe_key = format!("sk_test_{}", "1234567890abcdefghijABCD");
    let content = format!("const key = '{stripe_key}';");
    let findings = scan_content(&content, "src/test-config.ts", &config);

    assert!(
        findings.is_empty(),
        "custom allowlist should suppress known test keys"
    );
}

// ---------------------------------------------------------------------------
// Redaction
// ---------------------------------------------------------------------------

#[test]
fn findings_never_contain_raw_secrets() {
    let stripe_key = format!("sk_live_{}", "1234567890abcdefghijABCD");
    let github_token = format!("ghp_{}", "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0");
    let content = format!("const stripe = '{stripe_key}';\nconst gh = '{github_token}';\n");

    let findings = scan_content(&content, "src/secrets.ts", &default_config());
    assert!(
        findings.len() >= 2,
        "expected at least 2 findings, got {}: {:?}",
        findings.len(),
        findings.iter().map(|f| &f.pattern_name).collect::<Vec<_>>()
    );

    for finding in &findings {
        // Verify [REDACTED] placeholder is present
        assert!(
            finding.redacted_line.contains("[REDACTED]"),
            "redacted_line should contain [REDACTED] placeholder"
        );
        // Verify the raw secret tokens are NOT present in redacted output
        assert!(
            !finding.redacted_line.contains(stripe_key.as_str()),
            "redacted_line must not contain the raw Stripe key"
        );
        assert!(
            !finding.redacted_line.contains(github_token.as_str()),
            "redacted_line must not contain the raw GitHub token"
        );
    }
}
