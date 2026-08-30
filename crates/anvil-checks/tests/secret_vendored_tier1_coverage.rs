//! SDT-004 follow-up: **every** vendored tier-1 rule fires on a well-formed
//! credential of its own shape.
//!
//! The tier shipped with 27 rules, of which the SDT-002 corpus exercises five
//! (GitLab PAT, Mailgun, Shopify, Slack webhook, Postman). The other 22 were
//! vendored on the strength of the converter's structural gates — id present,
//! at most one capture group, a literal provider prefix — none of which asks
//! the question that matters: *does the rule actually detect anything?*
//!
//! A regex can pass every structural gate, compile cleanly, sit in
//! `DEFAULT_COMPILED_PATTERNS`, and match nothing a real credential looks like.
//! That failure is silent by construction: a rule that never fires costs no
//! false positives and shows up in no report. It is exactly the class this
//! module exists to close, so it is closed here rather than assumed.
//!
//! Every canary is built at runtime from a marker plus fixed filler, so no
//! literal credential-shaped string appears in this source (the same
//! construction rule the SDT-002 corpus and `secret_vendored_tier1.rs` use).
//! Lengths are taken from each rule's own regex, so a canary that fails to fire
//! means the rule disagrees with the shape upstream documents.

use std::collections::BTreeSet;

use anvil_checks::secret::{SecretCheckConfig, VENDORED_RULESET, scan_content_with_stats};

/// `n` hex characters, deterministic and non-repeating enough to survive an
/// entropy filter, built rather than written out.
fn hex(n: usize) -> String {
    const CYCLE: &str = "0cafe0d1ced0ba5e0f00d1abe15b0bad";
    CYCLE.chars().cycle().take(n).collect()
}

/// `n` alphanumeric characters, for rules whose body is `[0-9a-zA-Z_-]`.
fn alnum(n: usize) -> String {
    const CYCLE: &str = "CanaryXK7mQ2pV9tR4wS6yE3zA8bN5cH";
    CYCLE.chars().cycle().take(n).collect()
}

/// `n` lowercase-alphanumeric characters, for `[a-z0-9]` bodies.
fn lower(n: usize) -> String {
    const CYCLE: &str = "canaryxk7mq2pv9tr4ws6ye3za8bn5ch";
    CYCLE.chars().cycle().take(n).collect()
}

/// The prefix-anchored families: the credential carries a literal provider
/// prefix and needs no surrounding context to be recognised.
fn prefix_anchored_canaries() -> Vec<(&'static str, String)> {
    vec![
        (
            "digitalocean-access-token",
            format!("DO_TOKEN={}{};", "doo_v1_", hex(64)),
        ),
        (
            "digitalocean-pat",
            format!("DO_TOKEN={}{};", "dop_v1_", hex(64)),
        ),
        (
            "digitalocean-refresh-token",
            format!("DO_REFRESH={}{};", "dor_v1_", hex(64)),
        ),
        (
            "gitlab-cicd-job-token",
            format!("CI_JOB_TOKEN={}{}_{}", "glcbt-", alnum(5), alnum(20)),
        ),
        (
            "gitlab-deploy-token",
            format!("DEPLOY_TOKEN={}{}", "gldt-", alnum(20)),
        ),
        (
            "gitlab-feature-flag-client-token",
            format!("FF_CLIENT_TOKEN={}{}", "glffct-", alnum(20)),
        ),
        (
            "gitlab-feed-token",
            format!("FEED_TOKEN={}{}", "glft-", alnum(20)),
        ),
        (
            "gitlab-incoming-mail-token",
            format!("MAIL_TOKEN={}{}", "glimt-", alnum(25)),
        ),
        (
            "gitlab-kubernetes-agent-token",
            format!("AGENT_TOKEN={}{}", "glagent-", alnum(50)),
        ),
        (
            "gitlab-oauth-app-secret",
            format!("OAUTH_SECRET={}{}", "gloas-", alnum(64)),
        ),
        (
            "gitlab-pat",
            format!("GITLAB_PAT={}{}", "glpat-", alnum(20)),
        ),
        (
            "gitlab-ptt",
            format!("PIPELINE_TRIGGER={}{}", "glptt-", hex(40)),
        ),
        (
            "gitlab-rrt",
            format!("RUNNER_REG={}{}", "GR1348941", alnum(20)),
        ),
        (
            "gitlab-runner-authentication-token",
            format!("RUNNER_AUTH={}{}", "glrt-", alnum(20)),
        ),
        (
            "gitlab-scim-token",
            format!("SCIM_TOKEN={}{}", "glsoat-", alnum(20)),
        ),
        (
            "gitlab-session-cookie",
            format!("Cookie: {}{}", "_gitlab_session=", lower(32)),
        ),
    ]
}

/// Shopify, Postman and the Slack webhook URL — also prefix-anchored, kept
/// separate only to keep each table readable.
fn service_canaries() -> Vec<(&'static str, String)> {
    vec![
        (
            "postman-api-token",
            format!("POSTMAN_KEY={}{}-{};", "PMAK-", hex(24), hex(34)),
        ),
        (
            "shopify-access-token",
            format!("SHOPIFY_TOKEN={}{}", "shpat_", hex(32)),
        ),
        (
            "shopify-custom-access-token",
            format!("SHOPIFY_CUSTOM={}{}", "shpca_", hex(32)),
        ),
        (
            "shopify-private-app-access-token",
            format!("SHOPIFY_PRIVATE={}{}", "shppa_", hex(32)),
        ),
        (
            "shopify-shared-secret",
            format!("SHOPIFY_SECRET={}{}", "shpss_", hex(32)),
        ),
        (
            "slack-webhook-url",
            format!("SLACK_HOOK=https://hooks.slack.com/services/{}", alnum(48)),
        ),
    ]
}

/// **Keyword-gated rules.** These five are not prefix-anchored alone: upstream
/// wraps each in a `[\w.-]{0,50}?(?:mailgun|newrelic|...)` gate, so the rule
/// fires only when the provider name sits within ~50 characters *before* the
/// credential. Measured 2026-08-30: a well-formed `NRAK-` key on a line with no
/// `newrelic` token falls through to the entropy backstop instead, and a bare
/// Mailgun `key-` falls to the built-in `API Key` rule.
///
/// That is upstream's design, vendored verbatim as ADR-136 requires. It is
/// grouped separately here because it bounds what tier 1 delivers: 22 of the 27
/// rules match the credential anywhere, and these five need the context too.
fn keyword_gated_canaries() -> Vec<(&'static str, String)> {
    vec![
        (
            "mailgun-private-api-token",
            format!("mailgun_api_key={}{};", "key-", hex(32)),
        ),
        (
            "mailgun-pub-key",
            format!("mailgun_pub_key={}{};", "pubkey-", hex(32)),
        ),
        (
            "new-relic-browser-api-token",
            format!("newrelic_browser_token={}{};", "NRJS-", hex(19)),
        ),
        (
            "new-relic-insert-key",
            format!("newrelic_insert_key={}{};", "NRII-", lower(32)),
        ),
        (
            "new-relic-user-api-key",
            format!("newrelic_user_key={}{};", "NRAK-", lower(27)),
        ),
    ]
}

/// One well-formed canary line per vendored rule id.
///
/// The line shape matters as much as the token: several upstream rules require
/// a trailing delimiter, and the Mailgun and New Relic families require a
/// provider-keyword gate before the credential. A canary without those is not a
/// well-formed instance of the rule, and proving nothing is worse than proving
/// little.
fn canaries() -> Vec<(&'static str, String)> {
    let mut all = prefix_anchored_canaries();
    all.extend(service_canaries());
    all.extend(keyword_gated_canaries());
    all
}

fn fired_rules(content: &str, path: &str) -> Vec<String> {
    let (findings, _) = scan_content_with_stats(content, path, &SecretCheckConfig::default());
    findings.into_iter().map(|f| f.pattern_name).collect()
}

/// The completeness half: a rule added to the tier without a canary here fails
/// this test rather than shipping unproved.
#[test]
fn every_vendored_rule_has_a_canary() {
    let covered: BTreeSet<&str> = canaries().iter().map(|(id, _)| *id).collect();
    let shipped: BTreeSet<&str> = VENDORED_RULESET
        .rules
        .iter()
        .map(|rule| rule.id.as_str())
        .collect();
    let unproved: Vec<&&str> = shipped.difference(&covered).collect();
    assert!(
        unproved.is_empty(),
        "vendored rules with no canary proving they fire: {unproved:?}. \
         A rule that never matches costs no false positives and appears in no \
         report, so nothing else in the suite would notice."
    );
    let stale: Vec<&&str> = covered.difference(&shipped).collect();
    assert!(
        stale.is_empty(),
        "canaries for rules no longer in the tier: {stale:?}"
    );
}

/// The behaviour half: each rule fires, end to end, through the real scanner —
/// not against its own regex in isolation, so the false-positive filter stack
/// is in the path exactly as it is in production.
#[test]
fn every_vendored_rule_fires_on_a_well_formed_credential() {
    let mut silent = Vec::new();
    for (id, line) in canaries() {
        let content = format!("# fixture\n{line}\n");
        let fired = fired_rules(&content, "ci/providers.conf");
        if !fired.iter().any(|name| name == id) {
            silent.push(format!("{id} (fired instead: {fired:?})"));
        }
    }
    assert!(
        silent.is_empty(),
        "vendored rules that did not fire on a well-formed credential of their \
         own documented shape:\n  {}",
        silent.join("\n  ")
    );
}
