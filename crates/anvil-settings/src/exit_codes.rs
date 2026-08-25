//! Settings semantic outcomes mapped onto Anvil's global CLI exit registry
//! (SETCON-009 / ADR-132). Existing command exits are not renumbered.

/// Settings semantic outcomes from spec §7.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsOutcome {
    Success,
    CheckFailed,
    UsageError,
    ResolutionError,
    AccessError,
    RedactionError,
    InternalError,
}

pub const EXIT_OK: u8 = 0;
pub const EXIT_ERROR: u8 = 1;
pub const EXIT_GATE_FAIL: u8 = 2;
pub const EXIT_AUTH_REQUIRED: u8 = 3;
pub const EXIT_CONFIG_ERROR: u8 = 4;
/// Fail-closed redaction; no settings payload. Claims reserved code 8.
pub const EXIT_REDACTION_ERROR: u8 = 8;

/// Map a settings semantic outcome onto the global numeric registry.
#[must_use]
pub const fn code_for(outcome: SettingsOutcome) -> u8 {
    match outcome {
        SettingsOutcome::Success => EXIT_OK,
        SettingsOutcome::InternalError => EXIT_ERROR,
        SettingsOutcome::CheckFailed | SettingsOutcome::UsageError => EXIT_GATE_FAIL,
        SettingsOutcome::AccessError => EXIT_AUTH_REQUIRED,
        SettingsOutcome::ResolutionError => EXIT_CONFIG_ERROR,
        SettingsOutcome::RedactionError => EXIT_REDACTION_ERROR,
    }
}

#[cfg(test)]
mod exit_codes_crate_tests {
    use super::*;

    #[test]
    fn exit_codes_mapping_is_stable() {
        assert_eq!(code_for(SettingsOutcome::Success), 0);
        assert_eq!(code_for(SettingsOutcome::InternalError), 1);
        assert_eq!(code_for(SettingsOutcome::CheckFailed), 2);
        assert_eq!(code_for(SettingsOutcome::UsageError), 2);
        assert_eq!(code_for(SettingsOutcome::AccessError), 3);
        assert_eq!(code_for(SettingsOutcome::ResolutionError), 4);
        assert_eq!(code_for(SettingsOutcome::RedactionError), 8);
    }
}
