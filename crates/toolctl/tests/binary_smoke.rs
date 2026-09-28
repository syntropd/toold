//! Smoke tests driving the real toolctl binary: dispatch plus completions.

use std::process::Command;

fn toolctl() -> Command {
    Command::new(env!("CARGO_BIN_EXE_toolctl"))
}

#[test]
fn test_completions_for_each_shell() {
    for shell in ["bash", "elvish", "fish", "powershell", "zsh"] {
        let out = toolctl().args(["completions", shell]).output().unwrap();
        assert!(out.status.success(), "completions {shell} failed");
        let script = String::from_utf8(out.stdout).unwrap();
        assert!(script.contains("toolctl"), "no toolctl mention for {shell}");
    }
}

#[test]
fn test_help_mentions_toolctl() {
    let out = toolctl().arg("--help").output().unwrap();
    assert!(out.status.success());
    let help = String::from_utf8(out.stdout).unwrap();
    assert!(help.contains("toolctl"), "unexpected help: {help}");
}
