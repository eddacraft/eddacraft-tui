//! Settings semantic outcomes on the global CLI exit-code registry
//! (SETCON-009 / ADR-132).

pub use anvil_settings::exit_codes::{EXIT_REDACTION_ERROR, SettingsOutcome, code_for};

#[cfg(test)]
mod exit_codes_tests {
    use super::*;

    #[test]
    fn exit_codes_match_global_registry_and_cli_constants() {
        assert_eq!(code_for(SettingsOutcome::Success), crate::EXIT_OK);
        assert_eq!(code_for(SettingsOutcome::InternalError), crate::EXIT_ERROR);
        assert_eq!(
            code_for(SettingsOutcome::CheckFailed),
            crate::EXIT_GATE_FAIL
        );
        assert_eq!(code_for(SettingsOutcome::UsageError), crate::EXIT_GATE_FAIL);
        assert_eq!(
            code_for(SettingsOutcome::AccessError),
            crate::EXIT_AUTH_REQUIRED
        );
        assert_eq!(
            code_for(SettingsOutcome::ResolutionError),
            crate::EXIT_CONFIG_ERROR
        );
        assert_eq!(
            code_for(SettingsOutcome::RedactionError),
            crate::EXIT_REDACTION_ERROR
        );
        assert_eq!(crate::EXIT_REDACTION_ERROR, 8);
        assert_eq!(EXIT_REDACTION_ERROR, crate::EXIT_REDACTION_ERROR);
    }

    #[test]
    fn exit_codes_help_documents_redaction() {
        use clap::CommandFactory as _;
        let mut cmd = crate::Cli::command();
        let help = cmd.render_long_help().to_string();
        assert!(
            help.contains("EXIT CODES:"),
            "global registry must stay in --help"
        );
        assert!(
            help.contains("8  Redaction failure"),
            "redaction_error must be documented: {help}"
        );
    }
}
