use std::fs;
use std::path::Path;

use anvil_dashboard_server::{MAX_ARTEFACT_BYTES, Workspace, WorkspaceReadError};
use tempfile::tempdir;

#[test]
fn reads_regular_files_beneath_the_workspace() {
    let root = tempdir().expect("workspace");
    fs::create_dir(root.path().join(".anvil")).expect(".anvil");
    fs::write(
        root.path().join(".anvil/gates.json"),
        br#"{"status":"pass"}"#,
    )
    .expect("fixture");
    let workspace = Workspace::new(root.path()).expect("workspace boundary");

    let bytes = workspace
        .read(Path::new(".anvil/gates.json"))
        .expect("contained read");
    assert_eq!(bytes, br#"{"status":"pass"}"#);
}

#[test]
fn reads_paths_built_with_path_join() {
    // Regression: Windows WorkspaceAnchor is slash-only. Path::join uses the
    // platform separator (`\` on Windows), so Workspace::read must normalise
    // before the anchor call — otherwise plan module loads return empty Purpose.
    let root = tempdir().expect("workspace");
    fs::create_dir_all(root.path().join("plans").join("modules")).expect("modules");
    fs::write(
        root.path()
            .join("plans")
            .join("modules")
            .join("dash.aps.md"),
        b"## Purpose\n\nShip it.\n",
    )
    .expect("fixture");
    let workspace = Workspace::new(root.path()).expect("workspace boundary");

    let joined = Path::new("plans").join("modules").join("dash.aps.md");
    let bytes = workspace
        .read(&joined)
        .expect("Path::join multi-component read must succeed on every platform");
    assert_eq!(bytes, b"## Purpose\n\nShip it.\n");
}

#[test]
fn rejects_parent_and_absolute_paths() {
    let root = tempdir().expect("workspace");
    let workspace = Workspace::new(root.path()).expect("workspace boundary");

    assert!(matches!(
        workspace.read(Path::new("../outside.json")),
        Err(WorkspaceReadError::UnsafePath { .. })
    ));
    assert!(matches!(
        workspace.read(Path::new("/etc/passwd")),
        Err(WorkspaceReadError::UnsafePath { .. })
    ));
}

#[cfg(unix)]
#[test]
fn rejects_symlinked_files_and_directories() {
    use std::os::unix::fs::symlink;

    let root = tempdir().expect("workspace");
    let outside = tempdir().expect("outside");
    fs::write(outside.path().join("secret.json"), br#"{"secret":true}"#).expect("secret");
    fs::create_dir(root.path().join(".anvil")).expect(".anvil");
    symlink(
        outside.path().join("secret.json"),
        root.path().join(".anvil/link.json"),
    )
    .expect("file symlink");
    symlink(outside.path(), root.path().join("linked-dir")).expect("directory symlink");
    let workspace = Workspace::new(root.path()).expect("workspace boundary");

    assert!(matches!(
        workspace.read(Path::new(".anvil/link.json")),
        Err(WorkspaceReadError::Symlink { .. })
    ));
    assert!(matches!(
        workspace.read(Path::new("linked-dir/secret.json")),
        Err(WorkspaceReadError::Symlink { .. })
    ));
}

#[test]
fn rejects_missing_and_oversized_artefacts() {
    let root = tempdir().expect("workspace");
    fs::create_dir(root.path().join(".anvil")).expect(".anvil");
    fs::write(
        root.path().join(".anvil/oversized.json"),
        vec![b'x'; MAX_ARTEFACT_BYTES + 1],
    )
    .expect("oversized fixture");
    let workspace = Workspace::new(root.path()).expect("workspace boundary");

    assert!(matches!(
        workspace.read(Path::new(".anvil/missing.json")),
        Err(WorkspaceReadError::Missing { .. })
    ));
    assert!(matches!(
        workspace.read(Path::new(".anvil/oversized.json")),
        Err(WorkspaceReadError::TooLarge { .. })
    ));
}

#[test]
fn exposes_stable_structured_error_codes() {
    let unsafe_path = WorkspaceReadError::UnsafePath {
        path: "../outside.json".to_owned(),
    };
    let missing = WorkspaceReadError::Missing {
        path: ".anvil/missing.json".to_owned(),
    };
    let too_large = WorkspaceReadError::TooLarge {
        path: ".anvil/large.json".to_owned(),
        max_bytes: MAX_ARTEFACT_BYTES,
    };

    assert_eq!(unsafe_path.code(), "unsafe-artefact-path");
    assert_eq!(missing.code(), "artefact-not-found");
    assert_eq!(too_large.code(), "artefact-too-large");
}
