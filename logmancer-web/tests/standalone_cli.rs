#![cfg(feature = "ssr")]

use std::process::Command;

#[test]
fn standalone_web_help_succeeds_and_invalid_options_fail_actionably() {
    let help = Command::new(env!("CARGO_BIN_EXE_logmancer-web"))
        .arg("--help")
        .output()
        .unwrap();

    assert!(help.status.success());
    assert!(String::from_utf8(help.stdout)
        .unwrap()
        .contains("--file-root"));

    let invalid = Command::new(env!("CARGO_BIN_EXE_logmancer-web"))
        .args(["--bind", "not-an-address"])
        .output()
        .unwrap();

    assert!(!invalid.status.success());
    assert!(String::from_utf8(invalid.stderr)
        .unwrap()
        .contains("--bind"));
}
