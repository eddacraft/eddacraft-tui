use semver::Version;
use thiserror::Error;

/// Errors returned by [`RequiredAnvilVersion`] parsing and
/// comparison.
#[derive(Debug, Error)]
pub enum VersionFloorError {
    /// The `required_anvil_version` field in the policy file was not
    /// a valid semver string (e.g. missing `MAJOR.MINOR.PATCH`).
    #[error("required_anvil_version is not a valid semver: {raw:?} ({source})")]
    InvalidFloor {
        raw: String,
        #[source]
        source: semver::Error,
    },
    /// The running anvil version could not be parsed as semver.
    /// Should be unreachable for distributed binaries (the workspace
    /// version is checked at build time) but the typed error keeps
    /// the API safe.
    #[error("current anvil version is not a valid semver: {raw:?} ({source})")]
    InvalidCurrent {
        raw: String,
        #[source]
        source: semver::Error,
    },
}

/// A parsed `required_anvil_version` floor from a policy file.
///
/// Comparison follows standard semver precedence (1.0.0 < 1.0.1 <
/// 1.1.0 < 2.0.0; prerelease versions are lower than their
/// corresponding release, so `0.7.0-beta < 0.7.0`).
///
/// Callers should pass their own `env!("CARGO_PKG_VERSION")` for the
/// `current` argument so the comparison reflects the actual running
/// binary rather than a transitive crate version that can diverge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredAnvilVersion(Version);

impl RequiredAnvilVersion {
    /// Parse a `required_anvil_version` string from a policy file.
    pub fn parse(raw: impl AsRef<str>) -> Result<Self, VersionFloorError> {
        let raw = raw.as_ref();
        Version::parse(raw)
            .map(Self)
            .map_err(|source| VersionFloorError::InvalidFloor {
                raw: raw.to_string(),
                source,
            })
    }

    /// True if `current` is greater than or equal to this floor.
    /// Both versions are parsed; a parse error on either is bubbled
    /// up.
    pub fn satisfied_by(&self, current: impl AsRef<str>) -> Result<bool, VersionFloorError> {
        let current = current.as_ref();
        let parsed_current =
            Version::parse(current).map_err(|source| VersionFloorError::InvalidCurrent {
                raw: current.to_string(),
                source,
            })?;
        Ok(parsed_current >= self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_release_semver() {
        let v = RequiredAnvilVersion::parse("0.7.0").unwrap();
        // Equality through `satisfied_by` is the only public way to
        // probe parsed state, so test it both directions.
        assert!(v.satisfied_by("0.7.0").unwrap());
        assert!(!v.satisfied_by("0.6.99").unwrap());
    }

    #[test]
    fn parse_accepts_prerelease_semver() {
        let v = RequiredAnvilVersion::parse("0.7.0-beta").unwrap();
        assert!(v.satisfied_by("0.7.0-beta").unwrap());
    }

    #[test]
    fn parse_rejects_non_semver() {
        let err = RequiredAnvilVersion::parse("v0.7").unwrap_err();
        match err {
            VersionFloorError::InvalidFloor { raw, .. } => {
                assert_eq!(raw, "v0.7");
            }
            VersionFloorError::InvalidCurrent { .. } => {
                panic!("expected InvalidFloor, got InvalidCurrent")
            }
        }
    }

    #[test]
    fn parse_rejects_semver_range_syntax() {
        // CIB-029: docs once taught `required_anvil_version: ">=0.6.0"`,
        // but `parse` accepts only exact semver — the floor semantics
        // live in `satisfied_by`. Range operators must be rejected so the
        // documented contract and the parser stay aligned.
        for range in [">=0.6.0", "^0.6.0", "~0.6.0", "0.6.*", ">0.6.0"] {
            let err = RequiredAnvilVersion::parse(range).unwrap_err();
            assert!(
                matches!(err, VersionFloorError::InvalidFloor { .. }),
                "expected InvalidFloor for range syntax {range:?}",
            );
        }
    }

    #[test]
    fn parse_rejects_empty_string() {
        let err = RequiredAnvilVersion::parse("").unwrap_err();
        assert!(matches!(err, VersionFloorError::InvalidFloor { .. }));
    }

    #[test]
    fn floor_satisfied_by_equal_version() {
        let v = RequiredAnvilVersion::parse("0.7.0").unwrap();
        assert!(v.satisfied_by("0.7.0").unwrap());
    }

    #[test]
    fn floor_satisfied_by_newer_version() {
        let v = RequiredAnvilVersion::parse("0.7.0").unwrap();
        assert!(v.satisfied_by("0.7.1").unwrap());
        assert!(v.satisfied_by("0.8.0").unwrap());
        assert!(v.satisfied_by("1.0.0").unwrap());
    }

    #[test]
    fn floor_not_satisfied_by_older_version() {
        let v = RequiredAnvilVersion::parse("0.7.0").unwrap();
        assert!(!v.satisfied_by("0.6.9").unwrap());
        assert!(!v.satisfied_by("0.6.2-beta").unwrap());
    }

    #[test]
    fn prerelease_is_lower_than_release() {
        // Standard semver: 0.7.0-beta < 0.7.0
        let floor = RequiredAnvilVersion::parse("0.7.0").unwrap();
        assert!(!floor.satisfied_by("0.7.0-beta").unwrap());

        let prerelease_floor = RequiredAnvilVersion::parse("0.7.0-beta").unwrap();
        assert!(prerelease_floor.satisfied_by("0.7.0").unwrap());
        assert!(prerelease_floor.satisfied_by("0.7.0-beta").unwrap());
    }

    #[test]
    fn invalid_current_version_surfaces_typed_error() {
        let v = RequiredAnvilVersion::parse("0.7.0").unwrap();
        let err = v.satisfied_by("not-a-version").unwrap_err();
        match err {
            VersionFloorError::InvalidCurrent { raw, .. } => {
                assert_eq!(raw, "not-a-version");
            }
            VersionFloorError::InvalidFloor { .. } => {
                panic!("expected InvalidCurrent, got InvalidFloor")
            }
        }
    }
}
