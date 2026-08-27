//! GTAO-003 schedule hook: which save-time changes the CLI may follow up.
//!
//! The daemon never spawns `anvil check` or `anvil gate` and never links
//! tree-sitter (ADR-064 / ADR-127). After a `validate_paths` allow the CLI
//! process schedules a coalesced, changed-path `anvil check` subprocess.

use anvil_intercept_proto::protocol::ChangeKindWire;

/// Whether the CLI should schedule a cheap-catalogue follow-up for this change.
///
/// Deletes have no on-disk bytes to scan. Created, modified, and renamed
/// paths still exist (the new name, for a rename) and are eligible.
#[must_use]
pub fn should_schedule_followup(kind: &ChangeKindWire) -> bool {
    match kind {
        ChangeKindWire::Created | ChangeKindWire::Modified | ChangeKindWire::Renamed { .. } => true,
        ChangeKindWire::Deleted => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{ChangeKindWire, should_schedule_followup};

    #[test]
    fn created_modified_and_renamed_are_eligible() {
        assert!(should_schedule_followup(&ChangeKindWire::Created));
        assert!(should_schedule_followup(&ChangeKindWire::Modified));
        assert!(should_schedule_followup(&ChangeKindWire::Renamed {
            from: "src/old.rs".into(),
        }));
    }

    #[test]
    fn deleted_paths_are_not_eligible() {
        assert!(!should_schedule_followup(&ChangeKindWire::Deleted));
    }
}
