//! Class A mutation through the settings service (SETPREF-001).

use std::path::Path;

use anvil_config::{UserSettingsFile, load_user_settings, store_user_setting};
use serde_json::Value;

use crate::catalogue::Catalogue;
use crate::service::{SettingsError, SettingsService};
use crate::types::{ConsequenceClass, PersistenceTarget, Scope, ValueType};

#[derive(Debug, Clone, PartialEq)]
pub enum ClassAOp {
    Set(Value),
    Toggle,
    Reset,
}

/// Apply a Class A edit to user-config. Surfaces must not open the file.
pub fn apply_class_a(
    catalogue: &Catalogue,
    key: &str,
    op: ClassAOp,
    scope: Scope,
    user_root: &Path,
    expected_revision: Option<&str>,
) -> Result<UserSettingsFile, SettingsError> {
    let entry = catalogue
        .get(key)
        .ok_or_else(|| SettingsError::TypeMismatch(key.to_owned()))?;
    if entry.consequence_class != ConsequenceClass::A {
        return Err(SettingsError::NotClassA(key.to_owned()));
    }
    if entry.writer_for(scope) != Some(PersistenceTarget::UserConfig) {
        return Err(SettingsError::UnsupportedScope {
            key: key.to_owned(),
            scope,
        });
    }
    let current = load_user_settings(user_root)?;
    let next = match op {
        ClassAOp::Reset => None,
        ClassAOp::Set(value) => {
            validate_type(entry.key.as_str(), &entry.value_type, &value)?;
            Some(value)
        }
        ClassAOp::Toggle => {
            let ValueType::Boolean = entry.value_type else {
                return Err(SettingsError::TypeMismatch(key.to_owned()));
            };
            let current_value = current
                .values
                .get(key)
                .cloned()
                .or_else(|| entry.default.clone())
                .unwrap_or(Value::Bool(false));
            let Some(flag) = current_value.as_bool() else {
                return Err(SettingsError::TypeMismatch(key.to_owned()));
            };
            Some(Value::Bool(!flag))
        }
    };
    Ok(store_user_setting(
        user_root,
        key,
        next.as_ref(),
        expected_revision,
    )?)
}

fn validate_type(key: &str, ty: &ValueType, value: &Value) -> Result<(), SettingsError> {
    let ok = match ty {
        ValueType::Boolean => value.as_bool().is_some(),
        ValueType::String => value.as_str().is_some(),
        ValueType::Integer => value.as_i64().is_some(),
        ValueType::Enum { allowed } => value
            .as_str()
            .is_some_and(|text| allowed.iter().any(|item| item == text)),
        ValueType::List => value.as_array().is_some(),
        ValueType::Map | ValueType::Set => value.as_object().is_some(),
    };
    if ok {
        Ok(())
    } else {
        Err(SettingsError::TypeMismatch(key.to_owned()))
    }
}

impl SettingsService {
    pub fn apply_class_a(
        &self,
        key: &str,
        op: ClassAOp,
        scope: Scope,
        user_root: &Path,
        expected_revision: Option<&str>,
    ) -> Result<UserSettingsFile, SettingsError> {
        apply_class_a(
            self.catalogue(),
            key,
            op,
            scope,
            user_root,
            expected_revision,
        )
    }
}

#[cfg(test)]
mod settings_write_class_a_tests {
    use super::*;
    use crate::seed::first_release_catalogue;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn settings_write_class_a_toggles_boolean() {
        let dir = tempdir().unwrap();
        let cat = first_release_catalogue().unwrap();
        let out = apply_class_a(
            &cat,
            "interface.compact",
            ClassAOp::Toggle,
            Scope::User,
            dir.path(),
            None,
        )
        .unwrap();
        assert_eq!(out.values["interface.compact"], json!(true));
    }

    #[test]
    fn settings_write_class_a_rejects_class_c() {
        let dir = tempdir().unwrap();
        let cat = first_release_catalogue().unwrap();
        let err = apply_class_a(
            &cat,
            "protection.enforcement.mode",
            ClassAOp::Set(json!("warn")),
            Scope::User,
            dir.path(),
            None,
        )
        .expect_err("class C");
        assert!(matches!(err, SettingsError::NotClassA(_)));
    }

    #[test]
    fn settings_scope_rejects_project_for_class_a() {
        let dir = tempdir().unwrap();
        let cat = first_release_catalogue().unwrap();
        let err = apply_class_a(
            &cat,
            "interface.hints",
            ClassAOp::Set(json!(false)),
            Scope::Project,
            dir.path(),
            None,
        )
        .expect_err("project");
        assert!(matches!(err, SettingsError::UnsupportedScope { .. }));
    }

    #[test]
    fn settings_reset_clears_user_declaration() {
        let dir = tempdir().unwrap();
        let cat = first_release_catalogue().unwrap();
        apply_class_a(
            &cat,
            "interface.hints",
            ClassAOp::Set(json!(false)),
            Scope::User,
            dir.path(),
            None,
        )
        .unwrap();
        let out = apply_class_a(
            &cat,
            "interface.hints",
            ClassAOp::Reset,
            Scope::User,
            dir.path(),
            None,
        )
        .unwrap();
        assert!(!out.values.contains_key("interface.hints"));
    }
}
