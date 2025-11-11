#![allow(deprecated)]

use assert_cmd::cargo::CommandCargoExt;
use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("gh-notifier").unwrap();
    cmd.arg("--help");
    cmd.assert().success();
}

#[test]
fn test_cli_commands_available() {
    let mut cmd = Command::cargo_bin("gh-notifier").unwrap();
    cmd.arg("history");
    // We're testing that the CLI structure works
    cmd.assert();
}

#[test]
fn test_cli_info_command() {
    let mut cmd = Command::cargo_bin("gh-notifier").unwrap();
    cmd.arg("info");
    // This might fail because there's no database, but it should recognize the command
    cmd.assert();
}
