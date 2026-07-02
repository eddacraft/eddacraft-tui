//! Real-binary tests for CIB-108: network-capable OPA built-ins are denied
//! during policy evaluation.
//!
//! Untrusted workspace policies (`.anvil/policies/*.rego`) must not be able
//! to call `http.send`, `net.lookup_ip_addr`, or `opa.runtime` from developer
//! or CI machines. `OpaExecutor` passes a restricted `--capabilities` profile
//! (derived from the installed binary) on every eval/test invocation.
//!
//! Skipped when `opa` is not on PATH and `ANVIL_OPA_PATH` is unset, mirroring
//! `opa_real_binary.rs`. Fixture rego uses `import rego.v1` so it parses on
//! both the 0.x line (>= 0.59) and the 1.x line pinned in CI.

use std::path::PathBuf;

use anvil_policy::loader::LoadedPolicy;
use anvil_policy::opa::{OpaExecutor, find_opa_binary};
use serde_json::json;

fn require_opa_or_skip(test_name: &str) -> Option<OpaExecutor> {
    let Some(binary) = find_opa_binary() else {
        eprintln!("[SKIP] {test_name}: opa not on PATH and ANVIL_OPA_PATH unset");
        return None;
    };
    let binary_str = binary.to_string_lossy();
    let executor = OpaExecutor::new(Some(&binary_str), Some(15_000));
    if !executor.is_available() {
        eprintln!("[SKIP] {test_name}: opa binary at {binary_str} reports unavailable");
        return None;
    }
    Some(executor)
}

fn base_input() -> serde_json::Value {
    json!({
        "plan": {
            "id": "plan-cib-108-rust",
            "hash": "h",
            "intent": "capabilities restriction",
            "schema_version": "0.1.0",
            "proposed_changes": [{"type": "file_create", "path": "src/a.ts"}],
        },
        "context": {
            "workspace_root": "/tmp",
            "timestamp": 0,
        },
    })
}

fn policy(name: &str, content: &str) -> LoadedPolicy {
    LoadedPolicy {
        name: name.to_string(),
        path: PathBuf::from(format!("{name}.rego")),
        content: content.to_string(),
        package: format!("anvil.policies.{name}"),
        has_tests: false,
        test_path: None,
        generated: false,
        source_hash: None,
    }
}

#[test]
fn denied_builtins_are_rejected_instead_of_executed() {
    let Some(executor) = require_opa_or_skip("denied_builtins_are_rejected_instead_of_executed")
    else {
        return;
    };

    let cases = [
        (
            "http.send",
            "exfil",
            "package anvil.policies.exfil\n\n\
             import rego.v1\n\n\
             violation contains msg if {\n\
             \x20 resp := http.send({\"method\": \"get\", \"url\": \"http://127.0.0.1:9/exfil\"})\n\
             \x20 msg := sprintf(\"leaked %v\", [resp.status_code])\n\
             }\n",
        ),
        (
            "net.lookup_ip_addr",
            "dns_probe",
            "package anvil.policies.dns_probe\n\n\
             import rego.v1\n\n\
             violation contains msg if {\n\
             \x20 addrs := net.lookup_ip_addr(\"example.com\")\n\
             \x20 msg := sprintf(\"resolved %v\", [addrs])\n\
             }\n",
        ),
        (
            "opa.runtime",
            "env_leak",
            "package anvil.policies.env_leak\n\n\
             import rego.v1\n\n\
             violation contains msg if {\n\
             \x20 rt := opa.runtime()\n\
             \x20 msg := sprintf(\"env %v\", [rt.env])\n\
             }\n",
        ),
    ];

    for (builtin, name, content) in cases {
        let result = executor
            .evaluate(&[policy(name, content)], &base_input())
            .expect("evaluate returns a result, not a hard error");

        assert!(
            !result.success,
            "policy using {builtin} must be rejected, got success"
        );
        assert!(
            result.violations.is_empty(),
            "denied policy must not produce violations, got {:?}",
            result.violations
        );
        let msg = result.error.as_deref().unwrap_or("");
        assert!(
            msg.contains(builtin),
            "error must name the denied built-in {builtin}, got {msg:?}"
        );
        assert!(
            msg.contains("not permitted"),
            "error must explain the built-in is not permitted, got {msg:?}"
        );
    }
}

#[test]
fn benign_policy_still_evaluates_with_capabilities() {
    let Some(executor) = require_opa_or_skip("benign_policy_still_evaluates_with_capabilities")
    else {
        return;
    };

    let benign = policy(
        "change_gate",
        "package anvil.policies.change_gate\n\n\
         import rego.v1\n\n\
         violation contains msg if {\n\
         \x20 count(input.plan.proposed_changes) > 0\n\
         \x20 msg := \"plan proposes changes\"\n\
         }\n",
    );

    let result = executor
        .evaluate(&[benign], &base_input())
        .expect("evaluate ok");

    assert!(
        result.error.is_none(),
        "unexpected error: {:?}",
        result.error
    );
    assert_eq!(
        result.violations.len(),
        1,
        "benign policy must still evaluate and fire, got {:?}",
        result.violations
    );
    assert_eq!(result.violations[0].message, "plan proposes changes");
    assert_eq!(result.violations[0].policy.as_deref(), Some("change_gate"));
}

#[test]
fn run_tests_rejects_denied_builtin() {
    let Some(executor) = require_opa_or_skip("run_tests_rejects_denied_builtin") else {
        return;
    };

    let dir = tempfile::TempDir::new().expect("temp dir");
    std::fs::write(
        dir.path().join("exfil_test.rego"),
        "package anvil.policies.exfil_test\n\n\
         import rego.v1\n\n\
         test_exfil if {\n\
         \x20 resp := http.send({\"method\": \"get\", \"url\": \"http://127.0.0.1:9/exfil\"})\n\
         \x20 resp.status_code == 200\n\
         }\n",
    )
    .expect("write test rego");

    let result = executor
        .run_tests(dir.path(), false)
        .expect("run_tests returns a result, not a hard error");

    assert_eq!(
        result.passed, 0,
        "an http.send test must not pass; details={:?}",
        result.details
    );
    assert!(
        !result.errors.is_empty(),
        "expected a compile/eval error naming the denied built-in; got passed={} failed={} details={:?}",
        result.passed,
        result.failed,
        result.details
    );
}
