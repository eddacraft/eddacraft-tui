use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TrustLevel {
    #[default]
    Unknown,
    Internal,
    Boundary,
    External,
    Privileged,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_unknown() {
        assert_eq!(TrustLevel::default(), TrustLevel::Unknown);
    }

    #[test]
    fn all_variants_are_distinct() {
        let variants = [
            TrustLevel::Unknown,
            TrustLevel::Internal,
            TrustLevel::Boundary,
            TrustLevel::External,
            TrustLevel::Privileged,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b, "variants at index {i} and {j} should differ");
                }
            }
        }
    }

    #[test]
    fn clone_produces_equal_value() {
        let original = TrustLevel::Boundary;
        let cloned = original;
        assert_eq!(original, cloned);
    }

    #[test]
    fn copy_semantics() {
        let a = TrustLevel::Privileged;
        let b = a; // Copy, not move
        assert_eq!(a, b);
    }

    #[test]
    fn debug_format_contains_variant_name() {
        assert_eq!(format!("{:?}", TrustLevel::Unknown), "Unknown");
        assert_eq!(format!("{:?}", TrustLevel::External), "External");
    }

    #[test]
    fn serde_round_trip_all_variants() {
        let variants = [
            TrustLevel::Unknown,
            TrustLevel::Internal,
            TrustLevel::Boundary,
            TrustLevel::External,
            TrustLevel::Privileged,
        ];
        for variant in &variants {
            let json = serde_json::to_string(variant).expect("serialise");
            let back: TrustLevel = serde_json::from_str(&json).expect("deserialise");
            assert_eq!(*variant, back);
        }
    }

    #[test]
    fn deserialise_from_known_json() {
        let level: TrustLevel = serde_json::from_str("\"Boundary\"").expect("deserialise");
        assert_eq!(level, TrustLevel::Boundary);
    }

    #[test]
    fn deserialise_invalid_variant_fails() {
        let result = serde_json::from_str::<TrustLevel>("\"Untrusted\"");
        assert!(result.is_err());
    }
}
