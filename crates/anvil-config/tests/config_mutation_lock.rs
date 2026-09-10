use std::fs;

#[test]
fn linked_worktrees_share_the_git_common_dir_lock() {
    let repo = tempfile::tempdir().expect("repo");
    let common = tempfile::tempdir().expect("common git dir");
    let admin = common.path().join("worktrees/member");
    fs::create_dir_all(&admin).expect("worktree admin dir");
    fs::write(admin.join("commondir"), "../..\n").expect("commondir");
    fs::write(
        admin.join("gitdir"),
        format!("{}\n", repo.path().join(".git").display()),
    )
    .expect("gitdir backlink");
    fs::write(
        repo.path().join(".git"),
        format!("gitdir: {}\n", admin.display()),
    )
    .expect("gitfile");

    let path = anvil_config::mutation_lock_path(repo.path()).expect("lock path");

    assert_eq!(
        path,
        common.path().join("anvil/config-mutation.lock"),
        "linked worktrees must coordinate through the common Git directory"
    );
}

#[test]
fn planted_gitfile_cannot_redirect_lock_outside_the_repository() {
    let repo = tempfile::tempdir().expect("repo");
    let outside = tempfile::tempdir().expect("outside");
    fs::write(
        repo.path().join(".git"),
        format!("gitdir: {}\n", outside.path().display()),
    )
    .expect("gitfile");

    let error = anvil_config::mutation_lock_path(repo.path()).expect_err("unbound gitfile");

    assert!(error.to_string().contains("invalid Git metadata"));
    assert!(!outside.path().join("anvil").exists());
}

#[test]
fn non_git_existing_file_mutation_has_no_lock_authority() {
    let repo = tempfile::tempdir().expect("repo");

    let error = anvil_config::mutation_lock_path(repo.path()).expect_err("not a Git repo");

    assert!(
        error.to_string().contains("Git common directory"),
        "unexpected error: {error}"
    );
}
