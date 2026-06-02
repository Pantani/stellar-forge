#![allow(dead_code)]

use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::tempdir;

pub fn init_rewards_project() -> PathBuf {
    init_project("rewards-loyalty", &[])
}

pub fn init_minimal_contract_project() -> PathBuf {
    init_project("minimal-contract", &["--no-api"])
}

pub fn cargo_cli() -> Command {
    Command::cargo_bin("stellar-forge").expect("binary should build")
}

pub fn run_cli_json(cwd: &Path, args: &[&str]) -> Value {
    let output = cargo_cli()
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("stellar-forge command should run: {args:?}: {error}"));
    output_json(args, &output)
}

pub fn run_cli_json_with_path(cwd: &Path, args: &[&str], fake_bin: &Path) -> Value {
    let output = cargo_cli()
        .current_dir(cwd)
        .env("PATH", test_path(fake_bin))
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("stellar-forge command should run: {args:?}: {error}"));
    output_json(args, &output)
}

pub fn append_manifest(root: &Path, contents: &str) {
    let manifest = root.join("stellarforge.toml");
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(&manifest)
        .unwrap_or_else(|error| panic!("{} should open for append: {error}", manifest.display()));
    file.write_all(contents.as_bytes())
        .unwrap_or_else(|error| panic!("{} should be appended: {error}", manifest.display()));
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
    let mut command = cargo_cli();
    command
        .current_dir(parent)
        .args(["init", "demo", "--template", template])
        .args(extra_args)
        .assert()
        .success();
    root
}

fn output_json(args: &[&str], output: &Output) -> Value {
    assert!(
        output.status.success(),
        "stellar-forge command failed: {args:?}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout should be valid json for {args:?}: {error}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}
