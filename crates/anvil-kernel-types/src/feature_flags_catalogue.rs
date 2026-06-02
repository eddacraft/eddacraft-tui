//! Catalogue-sourced flag constants, generated at build time from the canonical
//! `flags/manifest.json` (FLAGCAT-004). See `build.rs`. The generated surface is
//! intentionally narrow — keys, default variants, variant-key constants, a
//! per-flag `definition()` builder, and `all::{KEYS, definitions}` — with no
//! runtime evaluation logic. FLAGCAT-005 cuts the CLI literal over to
//! `cli_licence_gate::definition()`.

include!(concat!(env!("OUT_DIR"), "/feature_flags_generated.rs"));

#[cfg(test)]
mod tests {
    use crate::FeatureFlagManifest;

    // The canonical manifest, embedded at compile time from the SAME absolute
    // path build.rs resolved (via the `FLAGCAT_MANIFEST_PATH` rustc-env it
    // emits) — so the test and the codegen can never read different files.
    const MANIFEST_JSON: &str = include_str!(env!("FLAGCAT_MANIFEST_PATH"));

    fn manifest() -> FeatureFlagManifest {
        serde_json::from_str(MANIFEST_JSON).expect("flags/manifest.json deserialises")
    }

    #[test]
    fn generated_keys_match_manifest_sorted() {
        let flags = manifest().flags;
        let mut expected: Vec<&str> = flags.iter().map(|f| f.key.as_str()).collect();
        expected.sort_unstable();
        assert_eq!(super::all::KEYS.to_vec(), expected);
    }

    #[test]
    fn generated_definitions_match_manifest() {
        // The generated definition() builders must reproduce the manifest
        // byte-for-byte (sorted by key). This is the safety net for the
        // build.rs literal emit — a wrong mapping fails here.
        let mut expected = manifest().flags;
        expected.sort_by(|a, b| a.key.cmp(&b.key));
        assert_eq!(super::all::definitions(), expected);
    }

    #[test]
    fn manifest_flags_declare_primary_group_and_round_trip() {
        // FLAGCAT-004 / Council: two guarantees in one pass — (a) `primary_group`
        // survives deserialisation (no field loss; it's `Option` on the struct),
        // and (b) every *manifest* flag actually declares one. `primary_group` is
        // optional on the type to match the TS base schema, so this is the
        // Rust-side invariant that the shipped manifest never omits it.
        for flag in manifest().flags {
            assert!(
                flag.primary_group.is_some(),
                "manifest flag {} is missing primaryGroup (or lost it on deserialisation)",
                flag.key
            );
        }
    }
}
