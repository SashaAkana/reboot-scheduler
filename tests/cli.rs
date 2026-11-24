use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_reboot-scheduler");

#[test]
fn dry_run_exits_without_waiting() {
    let out = Command::new(BIN).arg("--dry-run").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains(" 01:"));
    assert!(stdout.contains("no reboot requested"));
}

#[test]
fn version_matches_package() {
    let out = Command::new(BIN).arg("--version").output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn unknown_argument_fails_closed() {
    let out = Command::new(BIN).arg("--dryrun").output().unwrap();
    assert_eq!(out.status.code(), Some(2));
}
