use serde::{Deserialize, Serialize};

use crate::config::PolicyConfig;
use crate::library::builtin_policies;

/// Pre-configured policy profile that enables a curated set of built-in policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    /// Only essential policies — type-safety and error-handling.
    Minimal,
    /// Balanced set suitable for most projects.
    Standard,
    /// All built-in policies enabled at their highest severity.
    Strict,
    /// User-managed — no built-in policies are pre-configured.
    Custom,
}

impl Profile {
    /// All known profile variants (excluding Custom).
    pub const ALL: &[Profile] = &[Profile::Minimal, Profile::Standard, Profile::Strict];

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Standard => "standard",
            Self::Strict => "strict",
            Self::Custom => "custom",
        }
    }

    #[must_use]
    pub fn description(self) -> &'static str {
        match self {
            Self::Minimal => "Essential policies only — type-safety and error-handling",
            Self::Standard => "Balanced set suitable for most projects",
            Self::Strict => "All built-in policies enabled at highest severity",
            Self::Custom => "User-managed policy configuration",
        }
    }
}

impl std::fmt::Display for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl std::str::FromStr for Profile {
    type Err = ProfileError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "minimal" => Ok(Self::Minimal),
            "standard" => Ok(Self::Standard),
            "strict" => Ok(Self::Strict),
            "custom" => Ok(Self::Custom),
            _ => Err(ProfileError::UnknownProfile(s.to_string())),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("unknown profile: {0}")]
    UnknownProfile(String),
}

/// Metadata about a profile for listing/display purposes.
#[derive(Debug, Clone, Serialize)]
pub struct ProfileInfo {
    pub name: String,
    pub description: String,
    pub policy_count: usize,
    pub enabled_ids: Vec<String>,
}

/// IDs enabled under the minimal profile.
const MINIMAL_IDS: &[&str] = &["AP-003", "AP-004", "AP-006"];

/// IDs enabled under the standard profile (minimal + lint + quality + scope).
const STANDARD_IDS: &[&str] = &[
    "AP-001", "AP-002", "AP-003", "AP-004", "AP-005", "AP-006", "AP-008", "AP-010",
];

fn is_enabled_for(profile: Profile, id: &str) -> bool {
    match profile {
        Profile::Minimal => MINIMAL_IDS.contains(&id),
        Profile::Standard => STANDARD_IDS.contains(&id),
        Profile::Strict => true,
        Profile::Custom => false,
    }
}

/// Returns a `PolicyConfig` pre-populated for the given profile.
///
/// Policies not included in the profile are present but disabled.
/// Under `Strict`, all severities are upgraded to `"error"`.
#[must_use]
pub fn get_profile(profile: Profile) -> PolicyConfig {
    let policies = builtin_policies()
        .into_iter()
        .map(|mut p| {
            p.enabled = is_enabled_for(profile, &p.id);
            if profile == Profile::Strict && p.enabled {
                p.severity = "error".to_string();
            }
            p
        })
        .collect();

    PolicyConfig { policies }
}

/// Returns metadata about all non-custom profiles.
#[must_use]
pub fn list_profiles() -> Vec<ProfileInfo> {
    Profile::ALL
        .iter()
        .map(|&profile| {
            let config = get_profile(profile);
            let enabled_ids: Vec<String> = config
                .policies
                .iter()
                .filter(|p| p.enabled)
                .map(|p| p.id.clone())
                .collect();
            ProfileInfo {
                name: profile.name().to_string(),
                description: profile.description().to_string(),
                policy_count: enabled_ids.len(),
                enabled_ids,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn minimal_enables_subset() {
        let config = get_profile(Profile::Minimal);
        let enabled: Vec<_> = config.policies.iter().filter(|p| p.enabled).collect();
        assert_eq!(enabled.len(), MINIMAL_IDS.len());
        for p in &enabled {
            assert!(MINIMAL_IDS.contains(&p.id.as_str()), "unexpected: {}", p.id);
        }
    }

    #[test]
    fn standard_is_superset_of_minimal() {
        let min: HashSet<_> = MINIMAL_IDS.iter().copied().collect();
        let std: HashSet<_> = STANDARD_IDS.iter().copied().collect();
        assert!(min.is_subset(&std));
    }

    #[test]
    fn strict_enables_everything() {
        let config = get_profile(Profile::Strict);
        assert!(config.policies.iter().all(|p| p.enabled));
    }

    #[test]
    fn strict_upgrades_severity() {
        let config = get_profile(Profile::Strict);
        for p in &config.policies {
            assert_eq!(p.severity, "error", "policy {} not upgraded", p.id);
        }
    }

    #[test]
    fn custom_enables_nothing() {
        let config = get_profile(Profile::Custom);
        assert!(config.policies.iter().all(|p| !p.enabled));
    }

    #[test]
    fn list_profiles_returns_three() {
        let profiles = list_profiles();
        assert_eq!(profiles.len(), 3);
        let names: Vec<_> = profiles.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"minimal"));
        assert!(names.contains(&"standard"));
        assert!(names.contains(&"strict"));
    }

    #[test]
    fn profile_roundtrip_from_str() {
        for &p in Profile::ALL {
            let parsed: Profile = p.name().parse().unwrap();
            assert_eq!(parsed, p);
        }
    }

    #[test]
    fn unknown_profile_is_err() {
        assert!("bogus".parse::<Profile>().is_err());
    }

    #[test]
    fn profile_display() {
        assert_eq!(Profile::Standard.to_string(), "standard");
    }
}
