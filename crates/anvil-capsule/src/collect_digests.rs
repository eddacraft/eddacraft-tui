//! Policy/baseline/rules digest collector (GITGOV-006, ADR-074).
//!
//! Builds capsule `policy.json` / `rules.json` / `baseline.json` digests with
//! the same pipelines as the witness writer. Missing files → absent fields
//! (degraded); present-but-broken sources fail collection loudly.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::canonical::{canonical_json_bytes, sha256_hex};
use crate::errors::CapsuleError;

/// Schema identifier for `policy.json`.
pub const POLICY_DIGEST_SCHEMA: &str = "anvil.policy-digest.v1";

/// Schema identifier for `rules.json`.
pub const RULES_DIGEST_SCHEMA: &str = "anvil.rules-digest.v1";

/// Schema identifier for `baseline.json`.
pub const BASELINE_DIGEST_SCHEMA: &str = "anvil.baseline-digest.v1";

/// The effective policy file candidates, **in the order the L4
/// validator loads them** (`anvil-cli` `l4_validate::load_policy`):
/// `.yml` before `.yaml`. This is deliberately *not*
/// `anvil_config::DISCOVER_PRECEDENCE` (which prefers `.yaml`) —
/// "effective policy" means the file L4 actually loads, so the
/// capsule must resolve ties the same way the enforcement surface
/// does.
pub const POLICY_FILE_CANDIDATES: [&str; 4] = [
    "anvil/policy.yml",
    "anvil/policy.yaml",
    "anvil/policy.json",
    "anvil/policy.toml",
];

/// A governance source file and the digest of its canonical form.
///
/// `digest` is SHA-256 hex over `anvil_config::canonical_json_bytes`
/// of the parsed value — not the raw file bytes — so YAML / JSON /
/// TOML spellings of the same configuration collapse to the same
/// digest (the discipline `rules_sha`'s `config_sha` input already
/// uses).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileDigest {
    /// Repo-relative path, forward slashes (e.g. `anvil/policy.yml`
    /// or `.anvil.yaml`).
    pub path: String,
    /// SHA-256 hex digest of the file's canonical JSON form.
    pub digest: String,
}

/// `policy.json` — the effective policy identity
/// (`anvil.policy-digest.v1`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyDigest {
    /// Always [`POLICY_DIGEST_SCHEMA`]; gated on parse.
    pub schema: String,
    /// The effective `anvil/policy.*` file, when one exists. Missing
    /// (not `null`) when the repository has no policy file.
    ///
    /// The digest attests to the file's **content identity**, not its
    /// loadability: a file that parses as config but fails the L4
    /// loader's semantic validation (`anvil_l4::Policy::parse`) still
    /// digests cleanly here. The GITGOV-009 verifier treats a policy
    /// digest as corroborating evidence only alongside matching
    /// witness lines — which a repo with an unloadable policy cannot
    /// have produced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_file: Option<FileDigest>,
    /// The discovered `.anvil.*` config, when one exists. Its
    /// `digest` **is** the `config_sha` fed into the witnessed
    /// `rules_sha`. Missing (not `null`) when no config exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_file: Option<FileDigest>,
}

/// `rules.json` — the rule-set identity (`anvil.rules-digest.v1`).
///
/// Carries the *inputs* to `anvil_rules::rules_sha` alongside the
/// computed value, so a verifier can recompute the digest from the
/// document itself and cross-check it against witness lines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RulesDigest {
    /// Always [`RULES_DIGEST_SCHEMA`]; gated on parse.
    pub schema: String,
    /// The Anvil version input to `rules_sha`.
    pub anvil_version: String,
    /// The OPA runtime version input to `rules_sha`.
    pub opa_runtime_version: String,
    /// Resolved rule ids input to `rules_sha` (empty for v1 — the
    /// witness writer passes an empty set until the rule engine
    /// integration threads real ids through).
    pub rules: Vec<String>,
    /// `config_sha` over the discovered `.anvil.*` config. Missing
    /// (not `null`) when no config exists — the same conditions under
    /// which the witness line omits `rules_sha`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_sha: Option<String>,
    /// The `anvil_rules::rules_sha` value — the exact identity
    /// witnessed on the line. Missing (not `null`) when no config
    /// exists, matching the witness writer's `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rules_sha: Option<String>,
}

/// `baseline.json` — the baseline identity
/// (`anvil.baseline-digest.v1`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineDigest {
    /// Always [`BASELINE_DIGEST_SCHEMA`]; gated on parse.
    pub schema: String,
    /// The baseline's `cutoff_commit` — the value the
    /// `GENESIS-BASELINED` witness line carries. Missing (not `null`)
    /// when there is no baseline or the baseline has no cutoff.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cutoff_commit: Option<String>,
    /// SHA-256 hex over `Baseline::to_canonical_bytes()`. Missing
    /// (not `null`) when `anvil/baseline.json` does not exist —
    /// presence of this field is what distinguishes "baseline with no
    /// cutoff" from "no baseline at all".
    ///
    /// This is a **normalised identity, not a file-integrity check**:
    /// the digest is over the re-canonicalised store (which e.g.
    /// always emits `"cutoff_commit":null` when unset), so it can
    /// differ from `sha256(file bytes)` for a baseline not written by
    /// `anvil_baseline::save` — say, a hand-edited file omitting that
    /// key. Two baselines with the same governance content always
    /// digest identically, whatever their byte spelling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
}

/// Identity of the collecting tool — the non-config inputs to
/// `anvil_rules::rules_sha`. The caller (the `capsule create` CLI,
/// GITGOV-004) supplies these so this crate does not bake in any one
/// binary's version constants.
///
/// These inputs are producer-asserted. Recomputing `rules_sha` from a
/// `rules.json` document proves internal consistency only; the trust
/// signal is agreement with witness lines written at enforcement time
/// (GITGOV-009), which additionally requires these values to match
/// what the hook binary used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolIdentity {
    /// The Anvil version performing collection.
    pub anvil_version: String,
    /// The OPA runtime version compiled into the collector.
    pub opa_runtime_version: String,
    /// Resolved rule ids (empty for v1, mirroring the witness writer).
    pub rules: Vec<String>,
}

/// The three digest documents a capsule writes as `policy.json`,
/// `rules.json`, and `baseline.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectedDigests {
    /// Document for `policy.json`.
    pub policy: PolicyDigest,
    /// Document for `rules.json`.
    pub rules: RulesDigest,
    /// Document for `baseline.json`.
    pub baseline: BaselineDigest,
}

/// Collect the governance-identity digests for the repository at
/// `repo_root`.
///
/// Missing sources (no policy file, no `.anvil.*` config, no
/// `anvil/baseline.json`) collect as absent fields — never errors —
/// per the ADR-074 present-but-empty discipline.
///
/// # Errors
///
/// [`CapsuleError::Collect`] when a source exists but cannot be read
/// or parsed; [`CapsuleError::RulesIdentity`] when the rule identity
/// inputs cannot be combined into a `rules_sha` (e.g. an invalid rule
/// id in [`ToolIdentity::rules`]).
pub fn collect_digests(
    repo_root: &Path,
    tool: &ToolIdentity,
) -> Result<CollectedDigests, CapsuleError> {
    let policy_file = collect_policy_file(repo_root)?;
    let config_file = collect_config_file(repo_root)?;

    // `rules_sha` mirrors the pre-commit witness writer
    // (`anvil-cli` `compute_pre_commit_rules_sha`): present exactly
    // when a `.anvil.*` config exists, computed by
    // `anvil_rules::rules_sha` over the same `config_sha`.
    let config_sha = config_file.as_ref().map(|f| f.digest.clone());
    let rules_sha = config_sha
        .as_ref()
        .map(|sha| {
            anvil_rules::rules_sha(
                tool.anvil_version.clone(),
                tool.opa_runtime_version.clone(),
                tool.rules.iter().map(String::as_str),
                sha.clone(),
            )
            .map_err(|e| CapsuleError::RulesIdentity(e.to_string()))
        })
        .transpose()?;

    let (cutoff_commit, baseline_digest) = collect_baseline(repo_root)?;

    Ok(CollectedDigests {
        policy: PolicyDigest {
            schema: POLICY_DIGEST_SCHEMA.to_string(),
            policy_file,
            config_file,
        },
        rules: RulesDigest {
            schema: RULES_DIGEST_SCHEMA.to_string(),
            anvil_version: tool.anvil_version.clone(),
            opa_runtime_version: tool.opa_runtime_version.clone(),
            rules: tool.rules.clone(),
            config_sha,
            rules_sha,
        },
        baseline: BaselineDigest {
            schema: BASELINE_DIGEST_SCHEMA.to_string(),
            cutoff_commit,
            digest: baseline_digest,
        },
    })
}

/// Find and digest the effective `anvil/policy.*` file, honouring
/// [`POLICY_FILE_CANDIDATES`] order. `Ok(None)` when no candidate
/// exists.
///
/// Two deliberate notes against the L4 loader this mirrors:
///
/// - `try_exists` (here) vs `exists` (L4): identical for files and
///   for broken symlinks (both traverse links; both report a broken
///   link as absent). The only divergence is a stat *error* (e.g.
///   permission-denied on `anvil/`), which L4 silently treats as
///   absent and the collector fails loudly — the safe direction for
///   evidence production.
/// - No symlink guard: matches `l4_validate::load_policy` and the
///   witness writer, which also follow symlinks. The baseline path is
///   hardened separately inside `anvil_baseline::load`. Hardening
///   policy reads must move both this and the L4 loader together, or
///   the digest stops describing what enforcement loads.
fn collect_policy_file(repo_root: &Path) -> Result<Option<FileDigest>, CapsuleError> {
    for rel in POLICY_FILE_CANDIDATES {
        let path = repo_root.join(rel);
        let exists = path.try_exists().map_err(|e| CapsuleError::Collect {
            path: rel.to_string(),
            detail: e.to_string(),
        })?;
        if exists {
            return Ok(Some(digest_config_shaped_file(&path, rel)?));
        }
    }
    Ok(None)
}

/// Find and digest the discovered `.anvil.*` config — the same
/// `anvil_config::discover(repo_root, ".anvil")` call the witness
/// writer makes, so the recorded digest is the witnessed `config_sha`
/// by construction. `Ok(None)` when no config exists.
fn collect_config_file(repo_root: &Path) -> Result<Option<FileDigest>, CapsuleError> {
    let discovered =
        anvil_config::discover(repo_root, ".anvil").map_err(|e| CapsuleError::Collect {
            path: ".anvil.*".to_string(),
            detail: e.to_string(),
        })?;
    let Some(found) = discovered else {
        return Ok(None);
    };
    let rel = format!(".anvil.{}", found.format.extension());
    Ok(Some(digest_config_shaped_file(&found.path, &rel)?))
}

/// Parse a config-shaped file (yaml/yml/json/toml) and digest its
/// canonical JSON form — `anvil_config::parse_file` →
/// `anvil_config::canonical_json_bytes` →
/// `anvil_rules::config_sha_from_canonical`, the exact pipeline that
/// produces the witnessed `config_sha`.
fn digest_config_shaped_file(path: &Path, rel: &str) -> Result<FileDigest, CapsuleError> {
    let value = anvil_config::parse_file(path).map_err(|e| CapsuleError::Collect {
        path: rel.to_string(),
        detail: e.to_string(),
    })?;
    let canonical =
        anvil_config::canonical_json_bytes(&value).map_err(|e| CapsuleError::Collect {
            path: rel.to_string(),
            detail: e.to_string(),
        })?;
    Ok(FileDigest {
        path: rel.to_string(),
        digest: anvil_rules::config_sha_from_canonical(&canonical),
    })
}

/// Load `anvil/baseline.json` and return `(cutoff_commit, digest)` —
/// both `None` when no baseline exists.
fn collect_baseline(repo_root: &Path) -> Result<(Option<String>, Option<String>), CapsuleError> {
    let baseline = anvil_baseline::load(repo_root).map_err(|e| CapsuleError::Collect {
        path: anvil_baseline::BASELINE_PATH.to_string(),
        detail: e.to_string(),
    })?;
    let Some(baseline) = baseline else {
        return Ok((None, None));
    };
    let canonical = baseline
        .to_canonical_bytes()
        .map_err(|e| CapsuleError::Collect {
            path: anvil_baseline::BASELINE_PATH.to_string(),
            detail: e.to_string(),
        })?;
    Ok((baseline.cutoff_commit.clone(), Some(sha256_hex(&canonical))))
}

impl PolicyDigest {
    /// Encode as canonical JSON bytes — the byte form written to
    /// `policy.json`.
    ///
    /// # Errors
    ///
    /// [`CapsuleError::Serialise`] if encoding fails (practically
    /// unreachable).
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, CapsuleError> {
        doc_canonical_bytes(self)
    }

    /// Parse and schema-gate a `policy.json` document.
    ///
    /// # Errors
    ///
    /// [`CapsuleError::SchemaMismatch`] for a foreign schema version;
    /// [`CapsuleError::Parse`] for malformed JSON or unknown fields.
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, CapsuleError> {
        crate::schema_gate(bytes, POLICY_DIGEST_SCHEMA)?;
        serde_json::from_slice(bytes).map_err(|e| CapsuleError::Parse(e.to_string()))
    }
}

impl RulesDigest {
    /// Encode as canonical JSON bytes — the byte form written to
    /// `rules.json`.
    ///
    /// # Errors
    ///
    /// [`CapsuleError::Serialise`] if encoding fails (practically
    /// unreachable).
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, CapsuleError> {
        doc_canonical_bytes(self)
    }

    /// Parse and schema-gate a `rules.json` document.
    ///
    /// # Errors
    ///
    /// [`CapsuleError::SchemaMismatch`] for a foreign schema version;
    /// [`CapsuleError::Parse`] for malformed JSON or unknown fields.
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, CapsuleError> {
        crate::schema_gate(bytes, RULES_DIGEST_SCHEMA)?;
        serde_json::from_slice(bytes).map_err(|e| CapsuleError::Parse(e.to_string()))
    }
}

impl BaselineDigest {
    /// Encode as canonical JSON bytes — the byte form written to
    /// `baseline.json`.
    ///
    /// # Errors
    ///
    /// [`CapsuleError::Serialise`] if encoding fails (practically
    /// unreachable).
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, CapsuleError> {
        doc_canonical_bytes(self)
    }

    /// Parse and schema-gate a `baseline.json` document.
    ///
    /// # Errors
    ///
    /// [`CapsuleError::SchemaMismatch`] for a foreign schema version;
    /// [`CapsuleError::Parse`] for malformed JSON or unknown fields.
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, CapsuleError> {
        crate::schema_gate(bytes, BASELINE_DIGEST_SCHEMA)?;
        serde_json::from_slice(bytes).map_err(|e| CapsuleError::Parse(e.to_string()))
    }
}

/// Shared canonical-bytes encoder for the digest documents — the same
/// `Value`-roundtrip discipline `CapsuleManifest::to_canonical_bytes`
/// uses.
fn doc_canonical_bytes<T: Serialize>(doc: &T) -> Result<Vec<u8>, CapsuleError> {
    let value = serde_json::to_value(doc).map_err(|e| CapsuleError::Serialise(e.to_string()))?;
    canonical_json_bytes(&value).map_err(|e| CapsuleError::Serialise(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn tool() -> ToolIdentity {
        ToolIdentity {
            anvil_version: "0.7.4-beta".to_string(),
            opa_runtime_version: "opa-runtime-0.0.0".to_string(),
            rules: vec![],
        }
    }

    /// A repo with no governance files collects present-but-empty
    /// documents: schema set, every evidence field absent. Absence is
    /// the verifier's `degraded` signal, never a collection error.
    #[test]
    fn empty_repo_collects_present_but_empty_docs() {
        let dir = TempDir::new().unwrap();
        let collected = collect_digests(dir.path(), &tool()).unwrap();

        assert_eq!(collected.policy.schema, POLICY_DIGEST_SCHEMA);
        assert!(collected.policy.policy_file.is_none());
        assert!(collected.policy.config_file.is_none());

        assert_eq!(collected.rules.schema, RULES_DIGEST_SCHEMA);
        assert_eq!(collected.rules.anvil_version, "0.7.4-beta");
        assert_eq!(collected.rules.opa_runtime_version, "opa-runtime-0.0.0");
        assert!(collected.rules.rules.is_empty());
        assert!(collected.rules.config_sha.is_none());
        assert!(collected.rules.rules_sha.is_none());

        assert_eq!(collected.baseline.schema, BASELINE_DIGEST_SCHEMA);
        assert!(collected.baseline.cutoff_commit.is_none());
        assert!(collected.baseline.digest.is_none());
    }

    /// The witnessed-identity anchor: `config_sha` and `rules_sha`
    /// must equal what the pre-commit witness writer would compute
    /// for the same repo — same discovery, same canonicalisation,
    /// same digest functions (`anvil-cli` `hook.rs`
    /// `compute_pre_commit_rules_sha`).
    #[test]
    fn config_sha_and_rules_sha_match_witnessed_identity_by_construction() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".anvil.yaml"), "languages:\n  - rust\n").unwrap();

        let collected = collect_digests(dir.path(), &tool()).unwrap();

        let value = anvil_config::parse_file(&dir.path().join(".anvil.yaml")).unwrap();
        let canonical = anvil_config::canonical_json_bytes(&value).unwrap();
        let expected_config_sha = anvil_rules::config_sha_from_canonical(&canonical);
        let expected_rules_sha = anvil_rules::rules_sha(
            "0.7.4-beta",
            "opa-runtime-0.0.0",
            std::iter::empty::<&str>(),
            expected_config_sha.clone(),
        )
        .unwrap();

        assert_eq!(
            collected.rules.config_sha.as_deref(),
            Some(expected_config_sha.as_str())
        );
        assert_eq!(
            collected.rules.rules_sha.as_deref(),
            Some(expected_rules_sha.as_str())
        );

        // policy.json carries the same digest for the same file, so
        // the two documents cannot drift apart.
        let config_file = collected.policy.config_file.unwrap();
        assert_eq!(config_file.path, ".anvil.yaml");
        assert_eq!(config_file.digest, expected_config_sha);
    }

    /// Policy digests are over the canonical JSON of the parsed
    /// value, so YAML and JSON spellings of the same policy collapse
    /// to the same digest.
    #[test]
    fn policy_file_digest_is_stable_across_formats() {
        let yml_repo = TempDir::new().unwrap();
        fs::create_dir_all(yml_repo.path().join("anvil")).unwrap();
        fs::write(
            yml_repo.path().join("anvil/policy.yml"),
            "witness:\n  require: l4_or_l3\n",
        )
        .unwrap();

        let json_repo = TempDir::new().unwrap();
        fs::create_dir_all(json_repo.path().join("anvil")).unwrap();
        fs::write(
            json_repo.path().join("anvil/policy.json"),
            r#"{"witness":{"require":"l4_or_l3"}}"#,
        )
        .unwrap();

        let from_yml = collect_digests(yml_repo.path(), &tool()).unwrap();
        let from_json = collect_digests(json_repo.path(), &tool()).unwrap();

        let yml_file = from_yml.policy.policy_file.unwrap();
        let json_file = from_json.policy.policy_file.unwrap();
        assert_eq!(yml_file.path, "anvil/policy.yml");
        assert_eq!(json_file.path, "anvil/policy.json");
        assert_eq!(yml_file.digest, json_file.digest);
    }

    /// Tie-break order is the L4 loader's (`.yml` first), not
    /// `anvil_config::DISCOVER_PRECEDENCE` (`.yaml` first): the
    /// capsule must describe the policy file enforcement actually
    /// loads.
    #[test]
    fn policy_file_candidate_order_matches_l4_loader() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("anvil")).unwrap();
        fs::write(dir.path().join("anvil/policy.yml"), "witness: {}\n").unwrap();
        fs::write(
            dir.path().join("anvil/policy.yaml"),
            "witness:\n  require: l4_or_l3\n",
        )
        .unwrap();

        let collected = collect_digests(dir.path(), &tool()).unwrap();
        assert_eq!(
            collected.policy.policy_file.unwrap().path,
            "anvil/policy.yml"
        );
    }

    /// Baseline identity comes from the same store the
    /// `GENESIS-BASELINED` witness line is seeded from: cutoff
    /// matches, digest is over `Baseline::to_canonical_bytes()`.
    #[test]
    fn baseline_digest_and_cutoff_match_the_store() {
        let dir = TempDir::new().unwrap();
        let mut baseline = anvil_baseline::Baseline::new(
            anvil_baseline::BaselineMetadata {
                created_at: "2026-06-08T00:00:00Z".to_string(),
                created_by_version: "0.7.4-beta".to_string(),
                project_uuid: "01997e4a-1b2c-7345-8901-abcdef123456".to_string(),
            },
            vec![],
        );
        baseline.cutoff_commit = Some("a3b2ea4ecafef00da3b2ea4ecafef00da3b2ea4e".to_string());
        // Expected digest comes from the in-memory store *before*
        // save, so a save/load byte-identity divergence would fail
        // this test rather than hide inside a post-load recompute.
        let expected_digest = crate::canonical::sha256_hex(&baseline.to_canonical_bytes().unwrap());
        anvil_baseline::save(dir.path(), &baseline).unwrap();

        let collected = collect_digests(dir.path(), &tool()).unwrap();

        assert_eq!(
            collected.baseline.cutoff_commit.as_deref(),
            Some("a3b2ea4ecafef00da3b2ea4ecafef00da3b2ea4e")
        );
        assert_eq!(
            collected.baseline.digest.as_deref(),
            Some(expected_digest.as_str())
        );
        // And the round-trip agrees: load + re-canonicalise matches.
        let loaded = anvil_baseline::load(dir.path()).unwrap().unwrap();
        assert_eq!(
            crate::canonical::sha256_hex(&loaded.to_canonical_bytes().unwrap()),
            expected_digest
        );
    }

    /// A baseline with no cutoff still digests: field presence keeps
    /// "baseline with no cutoff" distinguishable from "no baseline".
    #[test]
    fn baseline_without_cutoff_still_digests() {
        let dir = TempDir::new().unwrap();
        let baseline = anvil_baseline::Baseline::new(
            anvil_baseline::BaselineMetadata {
                created_at: "2026-06-08T00:00:00Z".to_string(),
                created_by_version: "0.7.4-beta".to_string(),
                project_uuid: "01997e4a-1b2c-7345-8901-abcdef123456".to_string(),
            },
            vec![],
        );
        anvil_baseline::save(dir.path(), &baseline).unwrap();

        let collected = collect_digests(dir.path(), &tool()).unwrap();
        assert!(collected.baseline.cutoff_commit.is_none());
        assert!(collected.baseline.digest.is_some());
    }

    /// A policy file that exists but cannot be parsed fails
    /// collection loudly — a capsule must not misrepresent
    /// present-but-broken governance state as absence.
    #[test]
    fn unparseable_policy_file_fails_collection() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("anvil")).unwrap();
        fs::write(dir.path().join("anvil/policy.json"), "{").unwrap();

        let err = collect_digests(dir.path(), &tool()).unwrap_err();
        assert!(matches!(err, CapsuleError::Collect { .. }), "got: {err:?}");
    }

    /// Same loud-failure rule for a present-but-broken `.anvil.*`
    /// config — the witness writer's collapse-to-`None` is a
    /// save-time noise-discipline choice (ADR-038 §D-1) that does NOT
    /// apply to evidence production.
    #[test]
    fn unparseable_config_file_fails_collection() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".anvil.json"), "{").unwrap();

        let err = collect_digests(dir.path(), &tool()).unwrap_err();
        assert!(matches!(err, CapsuleError::Collect { .. }), "got: {err:?}");
    }

    /// A broken `anvil/baseline.json` fails collection rather than
    /// collecting as "no baseline".
    #[test]
    fn unparseable_baseline_fails_collection() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("anvil")).unwrap();
        fs::write(dir.path().join("anvil/baseline.json"), "not json").unwrap();

        let err = collect_digests(dir.path(), &tool()).unwrap_err();
        assert!(matches!(err, CapsuleError::Collect { .. }), "got: {err:?}");
    }

    /// An invalid rule id surfaces as a rules-identity error, not a
    /// silent omission.
    #[test]
    fn invalid_rule_id_fails_collection() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".anvil.yaml"), "languages:\n  - rust\n").unwrap();

        // `anvil_rules` rejects empty / non-ASCII rule ids.
        let bad_tool = ToolIdentity {
            rules: vec![String::new()],
            ..tool()
        };
        let err = collect_digests(dir.path(), &bad_tool).unwrap_err();
        assert!(
            matches!(err, CapsuleError::RulesIdentity(_)),
            "got: {err:?}"
        );
    }

    /// All three documents round-trip through their canonical bytes,
    /// and re-encoding the parse is byte-identical.
    #[test]
    fn docs_round_trip_through_canonical_bytes() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".anvil.yaml"), "languages:\n  - rust\n").unwrap();
        fs::create_dir_all(dir.path().join("anvil")).unwrap();
        fs::write(
            dir.path().join("anvil/policy.yml"),
            "witness:\n  require: l4_or_l3\n",
        )
        .unwrap();
        let collected = collect_digests(dir.path(), &tool()).unwrap();

        let bytes = collected.policy.to_canonical_bytes().unwrap();
        let parsed = PolicyDigest::from_json_bytes(&bytes).unwrap();
        assert_eq!(parsed, collected.policy);
        assert_eq!(parsed.to_canonical_bytes().unwrap(), bytes);

        let bytes = collected.rules.to_canonical_bytes().unwrap();
        let parsed = RulesDigest::from_json_bytes(&bytes).unwrap();
        assert_eq!(parsed, collected.rules);
        assert_eq!(parsed.to_canonical_bytes().unwrap(), bytes);

        let bytes = collected.baseline.to_canonical_bytes().unwrap();
        let parsed = BaselineDigest::from_json_bytes(&bytes).unwrap();
        assert_eq!(parsed, collected.baseline);
        assert_eq!(parsed.to_canonical_bytes().unwrap(), bytes);
    }

    /// Each document rejects a foreign schema version with
    /// `SchemaMismatch` — probe before parse, like the manifest.
    #[test]
    fn docs_reject_unknown_schema_versions() {
        let policy = br#"{"schema":"anvil.policy-digest.v999"}"#;
        assert!(matches!(
            PolicyDigest::from_json_bytes(policy).unwrap_err(),
            CapsuleError::SchemaMismatch { .. }
        ));
        let rules = br#"{"schema":"anvil.rules-digest.v999"}"#;
        assert!(matches!(
            RulesDigest::from_json_bytes(rules).unwrap_err(),
            CapsuleError::SchemaMismatch { .. }
        ));
        let baseline = br#"{"schema":"anvil.baseline-digest.v999"}"#;
        assert!(matches!(
            BaselineDigest::from_json_bytes(baseline).unwrap_err(),
            CapsuleError::SchemaMismatch { .. }
        ));
    }

    /// Closed schemas: unknown fields are a parse error, because a
    /// field the parser ignored is content the digest discipline
    /// cannot vouch for.
    #[test]
    fn docs_reject_unknown_fields() {
        let raw = br#"{"schema":"anvil.baseline-digest.v1","smuggled":true}"#;
        assert!(matches!(
            BaselineDigest::from_json_bytes(raw).unwrap_err(),
            CapsuleError::Parse(_)
        ));
    }

    /// Absent evidence serialises as missing keys, never `null` — the
    /// canonical discipline `WitnessLine` and the manifest use.
    #[test]
    fn empty_docs_omit_absent_fields_not_null() {
        let dir = TempDir::new().unwrap();
        let collected = collect_digests(dir.path(), &tool()).unwrap();

        let policy = String::from_utf8(collected.policy.to_canonical_bytes().unwrap()).unwrap();
        assert!(!policy.contains("null"), "policy.json: {policy}");
        assert!(!policy.contains("policy_file"));

        let rules = String::from_utf8(collected.rules.to_canonical_bytes().unwrap()).unwrap();
        assert!(!rules.contains("null"), "rules.json: {rules}");
        assert!(!rules.contains("rules_sha"));

        let baseline = String::from_utf8(collected.baseline.to_canonical_bytes().unwrap()).unwrap();
        assert!(!baseline.contains("null"), "baseline.json: {baseline}");
        assert!(!baseline.contains("cutoff_commit"));
    }

    /// Golden pin: the exact canonical encoding is the digest
    /// contract. A diff here is a schema-epoch event, not a refactor.
    #[test]
    fn rules_digest_canonical_bytes_golden() {
        let doc = RulesDigest {
            schema: RULES_DIGEST_SCHEMA.to_string(),
            anvil_version: "0.7.4-beta".to_string(),
            opa_runtime_version: "opa-runtime-0.0.0".to_string(),
            rules: vec!["AP-001".to_string()],
            config_sha: Some(
                "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a".to_string(),
            ),
            rules_sha: Some(
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
            ),
        };
        assert_eq!(
            std::str::from_utf8(&doc.to_canonical_bytes().unwrap()).unwrap(),
            concat!(
                r#"{"anvil_version":"0.7.4-beta","#,
                r#""config_sha":"44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a","#,
                r#""opa_runtime_version":"opa-runtime-0.0.0","#,
                r#""rules":["AP-001"],"#,
                r#""rules_sha":"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","#,
                r#""schema":"anvil.rules-digest.v1"}"#
            )
        );
    }

    /// Golden pin for `policy.json` — same schema-epoch contract as
    /// the rules golden.
    #[test]
    fn policy_digest_canonical_bytes_golden() {
        let doc = PolicyDigest {
            schema: POLICY_DIGEST_SCHEMA.to_string(),
            policy_file: Some(FileDigest {
                path: "anvil/policy.yml".to_string(),
                digest: "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
                    .to_string(),
            }),
            config_file: Some(FileDigest {
                path: ".anvil.yaml".to_string(),
                digest: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                    .to_string(),
            }),
        };
        assert_eq!(
            std::str::from_utf8(&doc.to_canonical_bytes().unwrap()).unwrap(),
            concat!(
                r#"{"config_file":{"digest":"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","path":".anvil.yaml"},"#,
                r#""policy_file":{"digest":"44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a","path":"anvil/policy.yml"},"#,
                r#""schema":"anvil.policy-digest.v1"}"#
            )
        );
    }

    /// Golden pin for `baseline.json` — same schema-epoch contract as
    /// the rules golden.
    #[test]
    fn baseline_digest_canonical_bytes_golden() {
        let doc = BaselineDigest {
            schema: BASELINE_DIGEST_SCHEMA.to_string(),
            cutoff_commit: Some("a3b2ea4ecafef00da3b2ea4ecafef00da3b2ea4e".to_string()),
            digest: Some(
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
            ),
        };
        assert_eq!(
            std::str::from_utf8(&doc.to_canonical_bytes().unwrap()).unwrap(),
            concat!(
                r#"{"cutoff_commit":"a3b2ea4ecafef00da3b2ea4ecafef00da3b2ea4e","#,
                r#""digest":"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","#,
                r#""schema":"anvil.baseline-digest.v1"}"#
            )
        );
    }

    /// `.anvil.*` tie-break follows `anvil_config::DISCOVER_PRECEDENCE`
    /// (`.yaml` beats `.yml`) — the same `discover` walk the witness
    /// writer performs, pinned here so a refactor away from
    /// `discover()` cannot silently change which file the capsule's
    /// `config_sha` describes. (Deliberately the opposite tie-break to
    /// the policy file, which follows the L4 loader — see
    /// [`POLICY_FILE_CANDIDATES`].)
    #[test]
    fn config_discovery_tie_break_matches_witness_writer() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".anvil.yaml"), "languages:\n  - rust\n").unwrap();
        fs::write(dir.path().join(".anvil.yml"), "languages:\n  - go\n").unwrap();

        let collected = collect_digests(dir.path(), &tool()).unwrap();
        assert_eq!(collected.policy.config_file.unwrap().path, ".anvil.yaml");
    }

    /// Caller-supplied rule ids thread through to both the recorded
    /// `rules` list (verbatim, caller order) and the `rules_sha`
    /// computation (which sorts/dedupes internally, like the witness
    /// writer's input type).
    #[test]
    fn tool_rules_propagate_into_doc_and_rules_sha() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".anvil.yaml"), "languages:\n  - rust\n").unwrap();

        let tool_with_rules = ToolIdentity {
            rules: vec!["AP-002".to_string(), "AP-001".to_string()],
            ..tool()
        };
        let collected = collect_digests(dir.path(), &tool_with_rules).unwrap();

        assert_eq!(collected.rules.rules, vec!["AP-002", "AP-001"]);
        let expected = anvil_rules::rules_sha(
            "0.7.4-beta",
            "opa-runtime-0.0.0",
            ["AP-002", "AP-001"],
            collected.rules.config_sha.clone().unwrap(),
        )
        .unwrap();
        assert_eq!(
            collected.rules.rules_sha.as_deref(),
            Some(expected.as_str())
        );
    }

    /// A stat *error* on a policy candidate (as opposed to a clean
    /// "absent") fails collection loudly — the L4 loader would
    /// silently skip the candidate, but evidence production must not
    /// guess.
    #[cfg(unix)]
    #[test]
    fn stat_error_on_policy_candidate_fails_collection_loudly() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new().unwrap();
        let anvil = dir.path().join("anvil");
        fs::create_dir_all(&anvil).unwrap();
        fs::write(anvil.join("policy.yml"), "witness: {}\n").unwrap();

        // Make `anvil/` untraversable so stat on candidates errors.
        fs::set_permissions(&anvil, fs::Permissions::from_mode(0o000)).unwrap();
        // Root (some CI containers) ignores mode bits — detect by
        // probing whether the chmod actually blocks traversal.
        let blocked = fs::metadata(anvil.join("policy.yml")).is_err();
        let result = collect_digests(dir.path(), &tool());
        // Restore before TempDir cleanup.
        fs::set_permissions(&anvil, fs::Permissions::from_mode(0o755)).unwrap();

        if !blocked {
            return; // running with CAP_DAC_OVERRIDE; nothing to assert
        }
        assert!(matches!(result.unwrap_err(), CapsuleError::Collect { .. }));
    }
}
