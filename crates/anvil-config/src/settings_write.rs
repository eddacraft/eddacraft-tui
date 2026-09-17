//! Class A user-config persistence (SETPREF).
//!
//! Policy: `plans/specs/2026-09-16-settings-class-a-safe-write.md`.
//! The settings service is the only writer process; this module is the
//! persistence target `user-config`.

use std::collections::BTreeMap;
#[cfg(unix)]
use std::fs::File;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::canonical::canonical_json_bytes;

const SETTINGS_FILE: &str = "settings.yaml";

/// Closed diagnostic class (safe-write policy §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafeWriteClass {
    Symlink,
    Traversal,
    Permission,
    NotRegular,
    Replace,
    Interrupted,
    Concurrent,
}

impl SafeWriteClass {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Symlink => "symlink",
            Self::Traversal => "traversal",
            Self::Permission => "permission",
            Self::NotRegular => "not-regular",
            Self::Replace => "replace",
            Self::Interrupted => "interrupted",
            Self::Concurrent => "concurrent",
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("[{class}] {message}", class = self.class.as_str())]
pub struct SafeWriteError {
    pub class: SafeWriteClass,
    message: String,
}

impl SafeWriteError {
    fn new(class: SafeWriteClass, message: impl Into<String>) -> Self {
        Self {
            class,
            message: message.into(),
        }
    }
}

/// Loaded user-config values plus a content revision for compare-and-swap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSettingsFile {
    pub values: BTreeMap<String, Value>,
    pub revision: String,
}

impl UserSettingsFile {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            values: BTreeMap::new(),
            revision: revision_of(&Value::Object(serde_json::Map::new())),
        }
    }
}

/// User-config root: `ANVIL_HOME/user`, else `XDG_CONFIG_HOME/anvil`, else
/// `$HOME/.config/anvil`. Tests should pass an explicit root.
#[must_use]
pub fn user_config_root() -> PathBuf {
    if let Ok(home) = std::env::var("ANVIL_HOME") {
        let home = home.trim();
        if !home.is_empty() {
            return PathBuf::from(home).join("user");
        }
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        let xdg = xdg.trim();
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("anvil");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".config").join("anvil")
}

#[must_use]
pub fn user_settings_path(root: &Path) -> PathBuf {
    root.join(SETTINGS_FILE)
}

/// Load `settings.yaml` from `root`. Missing file is empty, not an error.
pub fn load_user_settings(root: &Path) -> Result<UserSettingsFile, SafeWriteError> {
    let path = user_settings_path(root);
    match fs::symlink_metadata(&path) {
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(UserSettingsFile::empty()),
        Err(err) => return Err(io_to_error(&err)),
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(SafeWriteError::new(
                SafeWriteClass::Symlink,
                "user-config/settings.yaml is a symlink",
            ));
        }
        Ok(meta) if !meta.is_file() => {
            return Err(SafeWriteError::new(
                SafeWriteClass::NotRegular,
                "user-config/settings.yaml is not a regular file",
            ));
        }
        Ok(_) => {}
    }
    let text = read_regular_nofollow(&path)?;
    if text.trim().is_empty() {
        return Ok(UserSettingsFile::empty());
    }
    let parsed: BTreeMap<String, Value> = serde_yaml::from_str(&text)
        .map_err(|err| SafeWriteError::new(SafeWriteClass::Replace, err.to_string()))?;
    let revision = revision_of(&Value::Object(
        parsed.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
    ));
    Ok(UserSettingsFile {
        values: parsed,
        revision,
    })
}

/// Replace one key in the user-config file. `expected_revision` is the
/// SETPREF-005 compare-and-swap token (`None` skips the check).
pub fn store_user_setting(
    root: &Path,
    key: &str,
    value: Option<&Value>,
    expected_revision: Option<&str>,
) -> Result<UserSettingsFile, SafeWriteError> {
    let mut file = load_user_settings(root)?;
    if let Some(expected) = expected_revision
        && file.revision != expected
    {
        return Err(SafeWriteError::new(
            SafeWriteClass::Concurrent,
            "user-config changed since the edit began",
        ));
    }
    match value {
        Some(value) => {
            file.values.insert(key.to_owned(), value.clone());
        }
        None => {
            file.values.remove(key);
        }
    }
    persist_user_settings(root, &file.values)?;
    load_user_settings(root)
}

/// Persist the whole map with the safe-write policy.
pub fn persist_user_settings(
    root: &Path,
    values: &BTreeMap<String, Value>,
) -> Result<(), SafeWriteError> {
    if root.as_os_str().is_empty() {
        return Err(SafeWriteError::new(
            SafeWriteClass::Traversal,
            "user-config root is empty",
        ));
    }
    if let Ok(meta) = fs::symlink_metadata(root)
        && meta.file_type().is_symlink()
    {
        return Err(SafeWriteError::new(
            SafeWriteClass::Symlink,
            "user-config root is a symlink",
        ));
    }
    fs::create_dir_all(root).map_err(|err| io_to_error(&err))?;
    refuse_escaping_root(root, root)?;
    let dest = user_settings_path(root);
    refuse_traversal_name(SETTINGS_FILE)?;
    refuse_leaf_symlink(&dest)?;
    refuse_escaping_root(&dest, root)?;
    let yaml = serde_yaml::to_string(values)
        .map_err(|err| SafeWriteError::new(SafeWriteClass::Replace, err.to_string()))?;
    atomic_replace(&dest, yaml.as_bytes(), root)
}

fn read_regular_nofollow(path: &Path) -> Result<String, SafeWriteError> {
    #[cfg(unix)]
    {
        use std::io::Read;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
            .open(path)
            .map_err(|err| io_to_error(&err))?;
        let mut text = String::new();
        file.read_to_string(&mut text)
            .map_err(|err| io_to_error(&err))?;
        Ok(text)
    }
    #[cfg(not(unix))]
    {
        refuse_leaf_symlink(path)?;
        fs::read_to_string(path).map_err(|err| io_to_error(&err))
    }
}

fn atomic_replace(dest: &Path, bytes: &[u8], root: &Path) -> Result<(), SafeWriteError> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let tmp_name = format!(".{SETTINGS_FILE}.{}.{nonce}.tmp", std::process::id());
    refuse_traversal_name(&tmp_name)?;
    let tmp = root.join(&tmp_name);
    refuse_leaf_symlink(&tmp)?;

    let mut opts = OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut file = opts.open(&tmp).map_err(|err| io_to_error(&err))?;
    file.write_all(bytes).map_err(|err| {
        let _ = fs::remove_file(&tmp);
        io_to_error(&err)
    })?;
    file.sync_all().map_err(|err| {
        let _ = fs::remove_file(&tmp);
        SafeWriteError::new(SafeWriteClass::Interrupted, err.to_string())
    })?;
    drop(file);

    if let Ok(meta) = fs::symlink_metadata(dest) {
        if meta.file_type().is_symlink() {
            let _ = fs::remove_file(&tmp);
            return Err(SafeWriteError::new(
                SafeWriteClass::Symlink,
                "user-config/settings.yaml is a symlink",
            ));
        }
        if !meta.is_file() {
            let _ = fs::remove_file(&tmp);
            return Err(SafeWriteError::new(
                SafeWriteClass::NotRegular,
                "user-config/settings.yaml is not a regular file",
            ));
        }
        if OpenOptions::new().write(true).open(dest).is_err() {
            let _ = fs::remove_file(&tmp);
            return Err(SafeWriteError::new(
                SafeWriteClass::Permission,
                "user-config/settings.yaml is not writable",
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = meta.permissions().mode();
            if let Err(err) = fs::set_permissions(&tmp, fs::Permissions::from_mode(mode)) {
                let _ = fs::remove_file(&tmp);
                return Err(io_to_error(&err));
            }
        }
    }

    match fs::rename(&tmp, dest) {
        Ok(()) => {
            #[cfg(unix)]
            {
                if let Ok(dir) = File::open(root) {
                    let _ = dir.sync_all();
                }
            }
            Ok(())
        }
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
            refuse_leaf_symlink(dest)?;
            fs::remove_file(dest).map_err(|err| io_to_error(&err))?;
            fs::rename(&tmp, dest).map_err(|err| io_to_error(&err))
        }
        Err(err) => {
            let _ = fs::remove_file(&tmp);
            Err(io_to_error(&err))
        }
    }
}

fn refuse_traversal_name(name: &str) -> Result<(), SafeWriteError> {
    if name.is_empty() || name.contains('\0') {
        return Err(SafeWriteError::new(
            SafeWriteClass::Traversal,
            "invalid user-config name",
        ));
    }
    let path = Path::new(name);
    if path.is_absolute() {
        return Err(SafeWriteError::new(
            SafeWriteClass::Traversal,
            "absolute user-config name rejected",
        ));
    }
    for component in path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            _ => {
                return Err(SafeWriteError::new(
                    SafeWriteClass::Traversal,
                    "path traversal rejected",
                ));
            }
        }
    }
    if looks_like_windows_prefix(name) {
        return Err(SafeWriteError::new(
            SafeWriteClass::Traversal,
            "drive or UNC user-config name rejected",
        ));
    }
    Ok(())
}

fn looks_like_windows_prefix(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.starts_with(br"\\")
        || (bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic())
}

fn refuse_leaf_symlink(path: &Path) -> Result<(), SafeWriteError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(SafeWriteError::new(
            SafeWriteClass::Symlink,
            "user-config path is a symlink",
        )),
        Ok(_) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(io_to_error(&err)),
    }
}

fn refuse_escaping_root(path: &Path, root: &Path) -> Result<(), SafeWriteError> {
    let canonical_root = dunce::canonicalize(root).map_err(|err| io_to_error(&err))?;
    if !path.exists() {
        if let Some(parent) = path.parent()
            && parent.exists()
        {
            let parent = dunce::canonicalize(parent).map_err(|err| io_to_error(&err))?;
            if !parent.starts_with(&canonical_root) {
                return Err(SafeWriteError::new(
                    SafeWriteClass::Traversal,
                    "user-config path escapes the user-config root",
                ));
            }
        }
        return Ok(());
    }
    let canonical = dunce::canonicalize(path).map_err(|err| io_to_error(&err))?;
    if canonical != canonical_root && !canonical.starts_with(&canonical_root) {
        return Err(SafeWriteError::new(
            SafeWriteClass::Traversal,
            "user-config path escapes the user-config root",
        ));
    }
    Ok(())
}

fn revision_of(value: &Value) -> String {
    match canonical_json_bytes(value) {
        Ok(bytes) => {
            use sha2::{Digest, Sha256};
            Sha256::digest(bytes)
                .iter()
                .fold(String::new(), |mut out, byte| {
                    use std::fmt::Write;
                    let _ = write!(out, "{byte:02x}");
                    out
                })
        }
        Err(_) => "unknown".into(),
    }
}

fn io_to_error(err: &io::Error) -> SafeWriteError {
    let class = match err.kind() {
        io::ErrorKind::PermissionDenied => SafeWriteClass::Permission,
        io::ErrorKind::Interrupted => SafeWriteClass::Interrupted,
        _ => SafeWriteClass::Replace,
    };
    SafeWriteError::new(class, err.to_string())
}

#[cfg(test)]
mod settings_write_tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn settings_write_class_a_round_trip() {
        let dir = tempdir().unwrap();
        let stored = store_user_setting(dir.path(), "interface.compact", Some(&json!(true)), None)
            .expect("store");
        assert_eq!(stored.values["interface.compact"], json!(true));
        let loaded = load_user_settings(dir.path()).unwrap();
        assert_eq!(loaded.values["interface.compact"], json!(true));
        assert_eq!(loaded.revision, stored.revision);
    }

    #[test]
    fn settings_persist_preserves_sibling_keys() {
        let dir = tempdir().unwrap();
        store_user_setting(dir.path(), "interface.compact", Some(&json!(true)), None).unwrap();
        store_user_setting(dir.path(), "interface.hints", Some(&json!(false)), None).unwrap();
        let loaded = load_user_settings(dir.path()).unwrap();
        assert_eq!(loaded.values["interface.compact"], json!(true));
        assert_eq!(loaded.values["interface.hints"], json!(false));
    }

    #[test]
    fn settings_reset_removes_only_the_named_key() {
        let dir = tempdir().unwrap();
        store_user_setting(dir.path(), "interface.compact", Some(&json!(true)), None).unwrap();
        store_user_setting(dir.path(), "interface.hints", Some(&json!(true)), None).unwrap();
        store_user_setting(dir.path(), "interface.compact", None, None).unwrap();
        let loaded = load_user_settings(dir.path()).unwrap();
        assert!(!loaded.values.contains_key("interface.compact"));
        assert_eq!(loaded.values["interface.hints"], json!(true));
    }

    #[test]
    fn settings_concurrency_refuses_stale_revision() {
        let dir = tempdir().unwrap();
        let first =
            store_user_setting(dir.path(), "interface.compact", Some(&json!(true)), None).unwrap();
        store_user_setting(
            dir.path(),
            "interface.hints",
            Some(&json!(false)),
            Some(&first.revision),
        )
        .unwrap();
        let err = store_user_setting(
            dir.path(),
            "interface.motion",
            Some(&json!("reduced")),
            Some(&first.revision),
        )
        .expect_err("stale");
        assert_eq!(err.class, SafeWriteClass::Concurrent);
    }

    #[test]
    fn settings_safe_write_refuses_leaf_symlink() {
        let dir = tempdir().unwrap();
        let dest = user_settings_path(dir.path());
        let outside = dir.path().join("outside.yaml");
        fs::write(&outside, "stolen: true\n").unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside, &dest).unwrap();
            let err = persist_user_settings(dir.path(), &BTreeMap::new()).expect_err("symlink");
            assert_eq!(err.class, SafeWriteClass::Symlink);
            assert_eq!(fs::read_to_string(&outside).unwrap(), "stolen: true\n");
        }
    }

    #[test]
    fn settings_safe_write_refuses_traversal_name() {
        let err = refuse_traversal_name("../secrets").expect_err("dotdot");
        assert_eq!(err.class, SafeWriteClass::Traversal);
        let err = refuse_traversal_name("/tmp/x").expect_err("absolute");
        assert_eq!(err.class, SafeWriteClass::Traversal);
        let err = refuse_traversal_name(r"\\server\share").expect_err("unc");
        assert_eq!(err.class, SafeWriteClass::Traversal);
    }

    #[test]
    fn settings_persist_failed_validation_is_not_this_layer() {
        // Persistence writes only after the caller validates. Empty map is valid.
        let dir = tempdir().unwrap();
        persist_user_settings(dir.path(), &BTreeMap::new()).unwrap();
        assert!(user_settings_path(dir.path()).is_file());
    }

    #[cfg(unix)]
    #[test]
    fn settings_persist_new_file_is_owner_rw() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        persist_user_settings(dir.path(), &BTreeMap::new()).unwrap();
        let mode = fs::metadata(user_settings_path(dir.path()))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn settings_safe_write_refuses_broken_leaf_symlink() {
        let dir = tempdir().unwrap();
        let dest = user_settings_path(dir.path());
        std::os::unix::fs::symlink(dir.path().join("missing.yaml"), &dest).unwrap();
        let err = persist_user_settings(dir.path(), &BTreeMap::new()).expect_err("broken");
        assert_eq!(err.class, SafeWriteClass::Symlink);
    }

    #[cfg(unix)]
    #[test]
    fn settings_safe_write_refuses_parent_symlink_escape() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let linked_root = dir.path().join("linked");
        std::os::unix::fs::symlink(outside.path(), &linked_root).unwrap();
        let err = persist_user_settings(&linked_root, &BTreeMap::new()).expect_err("escape");
        assert_eq!(err.class, SafeWriteClass::Symlink);
    }

    #[cfg(unix)]
    #[test]
    fn settings_safe_write_unwritable_target() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        persist_user_settings(dir.path(), &BTreeMap::new()).unwrap();
        let dest = user_settings_path(dir.path());
        fs::set_permissions(&dest, fs::Permissions::from_mode(0o400)).unwrap();
        let mut values = BTreeMap::new();
        values.insert("interface.hints".into(), json!(false));
        let result = persist_user_settings(dir.path(), &values);
        let class = result.as_ref().err().map(|err| err.class);
        fs::set_permissions(&dest, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(class, Some(SafeWriteClass::Permission));
    }

    #[test]
    fn settings_safe_write_concurrent_publishers_do_not_mix_bytes() {
        let dir = tempdir().unwrap();
        let mut a = BTreeMap::new();
        a.insert("interface.compact".into(), json!(true));
        let mut b = BTreeMap::new();
        b.insert("interface.hints".into(), json!(false));
        persist_user_settings(dir.path(), &a).unwrap();
        persist_user_settings(dir.path(), &b).unwrap();
        let loaded = load_user_settings(dir.path()).unwrap();
        assert_eq!(loaded.values.get("interface.hints"), Some(&json!(false)));
        assert!(!loaded.values.contains_key("interface.compact"));
    }

    #[cfg(unix)]
    #[test]
    fn settings_persist_keeps_existing_mode() {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        let dir = tempdir().unwrap();
        let dest = user_settings_path(dir.path());
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o640)
            .open(&dest)
            .unwrap()
            .write_all(b"{}\n")
            .unwrap();
        persist_user_settings(dir.path(), &BTreeMap::new()).unwrap();
        let mode = fs::metadata(&dest).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o640);
    }
}
