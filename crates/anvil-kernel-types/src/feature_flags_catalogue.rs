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

    // The canonical manifest, embedded at compile time for the equivalence test.
    const MANIFEST_JSON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../flags/manifest.json"
    ));

    fn manifest() -> FeatureFlagManifest {
        serde_json::from_str(MANIFEST_JSON).expect("flags/manifest.json deserialises")
    }

    #[test]
    fn generated_keys_match_manifest_sorted() {
        let mut expected: Vec<String> = manifest().flags.iter().map(|f| f.key.clone()).collect();
        expected.sort();
        assert_eq!(super::all::KEYS, expected.as_slice());
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
    fn manifest_round_trip_preserves_primary_group() {
        // FLAGCAT-004 / Council: deserialising the live manifest must not drop
        // the gating-model fields — every flag carries a primaryGroup.
        for flag in manifest().flags {
            assert!(
                flag.primary_group.is_some(),
                "flag {} lost its primaryGroup on deserialisation",
                flag.key
            );
        }
    }
}
