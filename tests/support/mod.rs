#![allow(dead_code)]

use assert_cmd::prelude::*;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

pub fn init_rewards_project() -> PathBuf {
    init_project("rewards-loyalty", &[])
}

pub fn init_minimal_contract_project() -> PathBuf {
    init_project("minimal-contract", &["--no-api"])
}

pub fn install_fake_stellar(root: &Path) -> PathBuf {
    let bin_dir = root.join(".test-bin");
    fs::create_dir_all(&bin_dir).expect("bin dir should be created");
    let script_path = bin_dir.join("stellar");
    fs::write(
        &script_path,
        r#"#!/bin/sh
if [ "$1" = "keys" ] && [ "$2" = "generate" ]; then
  echo "generated $3"
  exit 0
fi
if [ "$1" = "keys" ] && [ "$2" = "public-key" ]; then
  echo "GFAKEPUBLICKEY"
  exit 0
fi
if [ "$1" = "keys" ] && [ "$2" = "ls" ]; then
  echo "alice GFAKEPUBLICKEY"
  exit 0
fi
echo "unsupported fake stellar invocation: $@" >&2
exit 1
"#,
    )
    .expect("fake stellar should be written");
    #[cfg(unix)]
    fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))
        .expect("fake stellar should be executable");
    bin_dir
}

pub fn test_path(fake_bin: &Path) -> String {
    format!(
        "{}:{}",
        fake_bin.display(),
        std::env::var("PATH").expect("PATH should exist")
    )
}

fn init_project(template: &str, extra_args: &[&str]) -> PathBuf {
    let temp = tempdir().expect("tempdir should be created");
    let kept = temp.keep();
    let root = kept.join("demo");
    let parent = root
        .parent()
        .expect("demo should have a parent")
        .to_path_buf();
    let mut command = Command::cargo_bin("stellar-forge").expect("binary should build");
    command
        .current_dir(parent)
        .args(["init", "demo", "--template", template])
        .args(extra_args)
        .assert()
        .success();
    root
}
