use super::config::PolicyEntry;

pub fn builtin_policies() -> Vec<PolicyEntry> {
    vec![
        PolicyEntry {
            id: "AP-001".into(),
            name: "Broad eslint-disable".into(),
            category: "lint".into(),
            enabled: true,
            description: "Detects broad /* eslint-disable */ comments".into(),
            severity: "warning".into(),
        },
        PolicyEntry {
            id: "AP-003".into(),
            name: "Explicit any type".into(),
            category: "type-safety".into(),
            enabled: true,
            description: "Detects explicit 'any' type usage".into(),
            severity: "warning".into(),
        },
        PolicyEntry {
            id: "AP-004".into(),
            name: "ts-ignore directive".into(),
            category: "type-safety".into(),
            enabled: true,
            description: "Detects @ts-ignore directives".into(),
            severity: "warning".into(),
        },
        PolicyEntry {
            id: "AP-006".into(),
            name: "Empty catch block".into(),
            category: "error-handling".into(),
            enabled: true,
            description: "Detects empty catch blocks".into(),
            severity: "warning".into(),
        },
        PolicyEntry {
            id: "AP-007".into(),
            name: "Console in production".into(),
            category: "logging".into(),
            enabled: false,
            description: "Detects console.log in production code".into(),
            severity: "info".into(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn builtin_ids_are_unique() {
        let policies = builtin_policies();
        let ids: HashSet<_> = policies.iter().map(|p| &p.id).collect();
        assert_eq!(ids.len(), policies.len(), "duplicate policy IDs found");
    }

    #[test]
    fn builtin_policies_not_empty() {
        assert!(!builtin_policies().is_empty());
    }
}
