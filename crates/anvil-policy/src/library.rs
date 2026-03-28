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
            id: "AP-002".into(),
            name: "CommonJS require in TypeScript".into(),
            category: "lint".into(),
            enabled: true,
            description:
                "Detects require() calls in TypeScript files where ESM imports are preferred".into(),
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
            id: "AP-005".into(),
            name: "TODO/FIXME in production".into(),
            category: "quality".into(),
            enabled: true,
            description: "Detects TODO and FIXME comments that should be resolved before release"
                .into(),
            severity: "info".into(),
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
        PolicyEntry {
            id: "AP-008".into(),
            name: "Hardcoded secrets".into(),
            category: "security".into(),
            enabled: true,
            description: "Detects potential hardcoded secrets, API keys, and tokens in source code"
                .into(),
            severity: "error".into(),
        },
        PolicyEntry {
            id: "AP-009".into(),
            name: "Large file warning".into(),
            category: "scope".into(),
            enabled: true,
            description: "Flags files exceeding a configurable line-count threshold".into(),
            severity: "warning".into(),
        },
        PolicyEntry {
            id: "AP-010".into(),
            name: "Missing error handling in async".into(),
            category: "error-handling".into(),
            enabled: true,
            description: "Detects async functions and promises without proper error handling"
                .into(),
            severity: "warning".into(),
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

    #[test]
    fn builtin_policies_count() {
        assert_eq!(builtin_policies().len(), 10);
    }

    #[test]
    fn ids_are_sequential() {
        let policies = builtin_policies();
        for (i, p) in policies.iter().enumerate() {
            let expected = format!("AP-{:03}", i + 1);
            assert_eq!(p.id, expected, "policy at index {i} has unexpected ID");
        }
    }

    #[test]
    fn security_policy_is_error_severity() {
        let policies = builtin_policies();
        let secrets = policies.iter().find(|p| p.id == "AP-008").unwrap();
        assert_eq!(secrets.severity, "error");
        assert_eq!(secrets.category, "security");
    }
}
