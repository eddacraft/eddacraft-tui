//! SDT-004: behaviour contract for the vendored gitleaks tier-1 ruleset.
//!
//! Two things are proved here, and they are different claims.
//!
//! 1. **The blocker was real.** ADR-136 §1 describes converting upstream rules
//!    into `SecretPatternDef` form. `SecretPatternDef` cannot express
//!    confidence, so a rule routed through it compiles `high_confidence: false`
//!    and inherits the fuzzy false-positive stack — the `example`/`test`
//!    keyword allowlist and `looks_like_code`. The tests below show that stack
//!    eating textbook tier-1 credentials, using the real filters and (for the
//!    camelCase case) a real end-to-end scan.
//!
//! 2. **The vendored path is not subject to it, and has not weakened
//!    suppression.** The same credentials fire through the vendored path, carry
//!    the ruleset version in their provenance, report the credential span rather
//!    than the scaffolding around it — and are still suppressible by the
//!    shape-anchored and operator `custom_allowlist` tiers, with provenance
//!    recorded, exactly as a built-in would be (ADR-136 §1).
//!
//! Every credential here is a synthetic canary built at runtime from a `CANARY`
//! marker plus fixed filler, so no literal credential-shaped string appears in
//! this source file (the same construction rule the SDT-002 corpus uses).

use anvil_checks::secret::patterns::PatternMatcher;
use anvil_checks::secret::{
    AllowlistProvenance, SECRET_PATTERNS, SecretCheckConfig, SecretPatternDef,
    VENDORED_COMPILED_PATTERNS, VENDORED_RULESET, VENDORED_RULESET_VERSION,
    compile_custom_patterns, scan_content_with_stats,
};

// ---------------------------------------------------------------------------
// Canaries
// ---------------------------------------------------------------------------

/// A GitLab PAT whose *body* carries the word `EXAMPLE` — the tier-1 twin of
/// the canonical AWS `AKIAIOSFODNN7EXAMPLE` key from issue #1800.
fn gitlab_example_token() -> String {
    format!("glpat-{}{}", "EXAMPLEcanary", "1234567")
}

/// A Slack incoming-webhook URL. Structurally code-like by `looks_like_code`'s
/// own test (`^https?://`), which is what makes it a sharp case.
fn slack_webhook_url() -> String {
    format!(
        "https://hooks.slack.com/services/{}/{}/{}",
        "T00000000", "B00000000", "CANARYxK7mQ2pV9tR4wS6yE3"
    )
}

/// A Mailgun private API key.
fn mailgun_key() -> String {
    format!("key-{}", "0cafe0000cafe0000cafe0000cafe000")
}

fn vendored_pattern(id: &str) -> String {
    VENDORED_RULESET
        .rules
        .iter()
        .find(|rule| rule.id == id)
        .unwrap_or_else(|| panic!("{id} is not in the vendored tier-1 ruleset"))
        .pattern
        .clone()
}

fn scan(content: &str, path: &str, config: &SecretCheckConfig) -> (Vec<String>, Vec<String>) {
    let (findings, stats) = scan_content_with_stats(content, path, config);
    (
        findings.into_iter().map(|f| f.pattern_name).collect(),
        stats
            .suppressions
            .into_iter()
            .map(|s| s.rule_name)
            .collect(),
    )
}

// ---------------------------------------------------------------------------
// 1. The blocker: what `SecretPatternDef` would have done to these rules
// ---------------------------------------------------------------------------

#[test]
fn the_fuzzy_keyword_filter_would_eat_a_tier1_credential() {
    // This is the filter a `SecretPatternDef`-routed rule runs, on the exact
    // value a tier-1 rule matches. `is_allowlisted` is the low-confidence path;
    // `is_shape_or_custom_allowlisted` is the high-confidence one.
    let matcher = PatternMatcher::new(&[]);
    let token = gitlab_example_token();

    assert!(
        matcher.is_allowlisted(&token),
        "the keyword allowlist matches the `EXAMPLE` inside the token — this is the \
         suppression a SecretPatternDef-routed rule would inherit"
    );
    assert!(
        !matcher.is_shape_or_custom_allowlisted(&token),
        "the high-confidence path must not suppress a real-shape GitLab PAT"
    );
}

#[test]
fn looks_like_code_would_eat_a_slack_webhook_url() {
    let matcher = PatternMatcher::new(&[]);
    assert!(
        matcher.looks_like_code(&slack_webhook_url()),
        "a webhook URL is `^https?://`, which `looks_like_code` classifies as code; only \
         the high-confidence bypass keeps it a finding"
    );
}

#[test]
fn a_secret_pattern_def_route_loses_a_camelcase_bound_credential() {
    // End-to-end, and isolated: the custom pattern below is the *shape* of the
    // vendored Mailgun rule with the provider keyword swapped for one no rule
    // in the catalogue knows, so nothing but this custom pattern can match and
    // the outcome is unambiguous.
    let template = vendored_pattern("mailgun-private-api-token");
    let custom_pattern = template.replace("mailgun", "canaryhost");
    assert_ne!(
        custom_pattern, template,
        "the substitution must actually apply, or this test proves nothing"
    );

    let config = SecretCheckConfig {
        custom_patterns: vec![SecretPatternDef {
            name: "canaryhost-private-api-token".to_string(),
            pattern: custom_pattern,
        }],
        ..SecretCheckConfig::default()
    };

    let content = format!("const canaryhostApiKey = '{}';\n", mailgun_key());

    // Non-vacuity control: the pattern must actually match the line, or the
    // assertion below would pass because nothing fired rather than because the
    // fuzzy stack ate it.
    let (compiled, errors) = compile_custom_patterns(&config.custom_patterns);
    assert!(errors.is_empty(), "custom pattern must compile: {errors:?}");
    assert!(
        compiled[0].first_match_range(content.trim_end()).is_some(),
        "the custom pattern must match this line for the test to mean anything"
    );

    let (found, suppressed) = scan(&content, "src/integrations/canaryhost.ts", &config);

    assert!(
        !found.contains(&"canaryhost-private-api-token".to_string()),
        "a SecretPatternDef-routed rule is low-confidence, so the whole match \
         (`canaryhostApiKey = 'key-…'`) trips `looks_like_code`'s mixed-case arm and the \
         credential is dropped. Findings were: {found:?} (suppressions: {suppressed:?})"
    );
}

// ---------------------------------------------------------------------------
// 2. The vendored path: the same credentials, detected
// ---------------------------------------------------------------------------

#[test]
fn a_vendored_rule_fires_on_a_credential_containing_example() {
    let config = SecretCheckConfig::default();
    // The word appears in the token *and* in the surrounding file, which is
    // exactly the shape that used to go quiet.
    let content = format!(
        "# example CI configuration for the example project\nGITLAB_TOKEN={}\n",
        gitlab_example_token()
    );
    let (found, _) = scan(&content, "ci/gitlab.conf", &config);
    assert!(
        found.contains(&"gitlab-pat".to_string()),
        "the vendored GitLab rule must fire despite `EXAMPLE`; got {found:?}"
    );
}

#[test]
fn a_vendored_rule_fires_on_a_camelcase_binding() {
    let config = SecretCheckConfig::default();
    let content = format!("const mailgunApiKey = '{}';\n", mailgun_key());
    let (found, _) = scan(&content, "src/integrations/mailgun.ts", &config);
    assert!(
        found.contains(&"mailgun-private-api-token".to_string()),
        "the vendored Mailgun rule must survive the camelCase binding that defeats the \
         fuzzy stack (CIB-363); got {found:?}"
    );
}

#[test]
fn a_vendored_rule_fires_on_a_code_shaped_webhook_url() {
    let config = SecretCheckConfig::default();
    let content = format!("const hook =\n  '{}';\n", slack_webhook_url());
    let (found, _) = scan(&content, "src/integrations/notify.ts", &config);
    assert!(
        found.contains(&"slack-webhook-url".to_string()),
        "got {found:?}"
    );
}

#[test]
fn a_vendored_finding_carries_the_ruleset_version_and_the_credential_span() {
    let config = SecretCheckConfig::default();
    let key = mailgun_key();
    let content = format!("const mailgunApiKey = '{key}';\n");
    let (findings, _) = scan_content_with_stats(&content, "src/mailgun.ts", &config);

    let finding = findings
        .iter()
        .find(|f| f.pattern_name == "mailgun-private-api-token")
        .expect("the vendored Mailgun rule fires");

    assert_eq!(
        finding.ruleset_version.as_deref(),
        Some(VENDORED_RULESET_VERSION.as_str()),
        "the work item requires the ruleset version in finding provenance"
    );

    // `secret_group` narrowing: the reported span is the credential, not the
    // provider-keyword gate upstream matches around it.
    let (start, end) = (
        finding.match_start.expect("span start"),
        finding.match_end.expect("span end"),
    );
    assert_eq!(
        &content[start..end],
        key,
        "the reported span must be the credential itself"
    );

    // And a built-in is not attributed to a vendored ruleset.
    let builtin_config = SecretCheckConfig::default();
    let builtin_content = "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE\n";
    let (builtin_findings, _) =
        scan_content_with_stats(builtin_content, "infra/deploy.conf", &builtin_config);
    let builtin = builtin_findings
        .iter()
        .find(|f| f.pattern_name == "AWS Key")
        .expect("the built-in AWS rule fires");
    assert!(builtin.ruleset_version.is_none());
}

// ---------------------------------------------------------------------------
// 3. Suppression is not weakened (ADR-136 §1)
// ---------------------------------------------------------------------------

#[test]
fn an_operator_allowlist_still_suppresses_a_vendored_rule_with_provenance() {
    let token = gitlab_example_token();
    let config = SecretCheckConfig {
        custom_allowlist: vec![token.clone()],
        ..SecretCheckConfig::default()
    };
    let content = format!("GITLAB_TOKEN={token}\n");
    let (findings, stats) = scan_content_with_stats(&content, "ci/gitlab.conf", &config);

    assert!(
        !findings.iter().any(|f| f.pattern_name == "gitlab-pat"),
        "an operator opt-out must apply to vendored rules exactly as to built-ins"
    );
    let suppression = stats
        .suppressions
        .iter()
        .find(|s| s.rule_name == "gitlab-pat")
        .expect("the suppression must be recorded, never silent");
    assert_eq!(
        suppression.provenance,
        AllowlistProvenance::Custom {
            pattern: token.clone()
        },
        "the operator must be able to trace the suppression back to their own entry"
    );
    assert!(
        !suppression.redacted_match.contains(&token),
        "a recorded suppression must not echo the raw value"
    );
}

#[test]
fn the_shape_allowlist_still_applies_to_vendored_rules() {
    // A Shopify token whose body is a 32-character hex run is also an MD5-shaped
    // value; the shape tier is the one that survives the high-confidence bypass,
    // and it must keep working for vendored rules too.
    let matcher = PatternMatcher::new(&[]);
    assert!(
        matcher.is_shape_or_custom_allowlisted("abcdef0123456789abcdef0123456789"),
        "the shape tier is unchanged"
    );
    assert!(
        VENDORED_COMPILED_PATTERNS
            .iter()
            .all(|pattern| pattern.high_confidence),
        "tier 1 is high-confidence, which is precisely why the shape tier must still apply"
    );
}

// ---------------------------------------------------------------------------
// 4. The vendored data must not poison the repository it ships in
// ---------------------------------------------------------------------------

#[test]
fn vendored_metadata_stays_clean_under_the_scanner() {
    // The SDT-002 corpus learned this the hard way: files that anvil's own gate
    // scans must not carry credential-shaped values. The vendored directory is
    // committed source, so it is scanned — including by the very rules it
    // defines.
    let root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/secret/vendor/gitleaks");
    let config = SecretCheckConfig::default();
    for name in ["tier1.json", "PIN.toml", "tier1-rules.txt", "PROVENANCE.md"] {
        let path = root.join(name);
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("{name} unreadable at {}: {err}", path.display()));
        let (findings, _) = scan_content_with_stats(&content, name, &config);
        let reported: Vec<String> = findings
            .iter()
            .map(|f| format!("{}:{} {}", name, f.line, f.pattern_name))
            .collect();
        assert!(
            findings.is_empty(),
            "{name} is committed source and is scanned by anvil's own gate; it must carry no \
             credential-shaped values. Reported: {reported:?}"
        );
    }
}

/// The offline half of ADR-136 §5's drift contract.
///
/// `refresh-gitleaks-ruleset.sh --check` is the authoritative check, but it
/// needs the network, so CI runs it only on the paths that could drift. These
/// assertions need nothing but the tree and therefore run on every test
/// invocation: they catch a `tier1.json` edited out of step with its pin or its
/// membership list, which is the realistic hand-edit.
#[test]
fn vendored_data_agrees_with_its_pin_and_membership_list() {
    let root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/secret/vendor/gitleaks");
    let pin = std::fs::read_to_string(root.join("PIN.toml")).expect("PIN.toml");
    let value_of = |key: &str| -> String {
        pin.lines()
            .find_map(|line| line.strip_prefix(&format!("{key} = \"")))
            .and_then(|rest| rest.strip_suffix('"'))
            .unwrap_or_else(|| panic!("PIN.toml has no {key}"))
            .to_string()
    };

    assert_eq!(
        VENDORED_RULESET.version,
        value_of("tag"),
        "tier1.json records a different upstream tag than PIN.toml"
    );
    assert_eq!(
        VENDORED_RULESET.commit,
        value_of("commit"),
        "tier1.json records a different upstream commit than PIN.toml"
    );
    assert!(
        VENDORED_RULESET_VERSION.contains(&VENDORED_RULESET.version),
        "the provenance string must name the pinned tag"
    );

    let membership =
        std::fs::read_to_string(root.join("tier1-rules.txt")).expect("tier1-rules.txt");
    let declared: Vec<&str> = membership
        .lines()
        .map(|line| line.split('#').next().unwrap_or("").trim())
        .filter(|line| !line.is_empty())
        .collect();
    let mut declared_sorted = declared.clone();
    declared_sorted.sort_unstable();
    let vendored: Vec<&str> = VENDORED_RULESET
        .rules
        .iter()
        .map(|rule| rule.id.as_str())
        .collect();
    assert_eq!(
        vendored, declared_sorted,
        "tier1.json's rule set must be exactly the membership list, sorted — a mismatch means \
         the data was hand-edited or the refresh was not re-run"
    );
}

#[test]
fn no_vendored_rule_shadows_a_builtin_name() {
    // A duplicate name would make findings ambiguous and would break the
    // corpus's per-rule accounting.
    for vendored in VENDORED_COMPILED_PATTERNS.iter() {
        assert!(
            !SECRET_PATTERNS
                .iter()
                .any(|builtin| builtin.name == vendored.name),
            "vendored rule {} collides with a built-in rule name",
            vendored.name
        );
    }
}
