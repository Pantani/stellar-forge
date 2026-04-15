use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn wallet_smart_policy_info_uses_active_identity_for_passkey_wallets() {
    let root = init_rewards_project();

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["wallet", "smart", "create", "guardian", "--mode", "passkey"])
        .assert()
        .success();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "smart",
            "policy",
            "info",
            "guardian",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.smart.policy.info");
    assert_eq!(json["network"], "testnet");
    assert_eq!(json["data"]["default_source"], "alice");
    assert_eq!(json["data"]["policy_contract"]["name"], "guardian-policy");
    assert_eq!(json["data"]["policy_contract"]["target"], "guardian-policy");
    assert_eq!(json["data"]["policy_contract"]["deployed"], false);
    let functions = json["data"]["functions"]
        .as_array()
        .expect("functions should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(functions.contains(&"set_daily_limit"));
    assert!(functions.contains(&"allow"));
    assert!(functions.contains(&"revoke"));
}

#[test]
fn wallet_smart_policy_set_daily_limit_build_only_uses_controller_identity() {
    let root = init_rewards_project();

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["wallet", "smart", "create", "sentinel", "--mode", "ed25519"])
        .assert()
        .success();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "smart",
            "policy",
            "set-daily-limit",
            "sentinel",
            "1250",
            "--build-only",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.smart.policy.set-daily-limit");
    assert_eq!(json["network"], "testnet");
    assert_eq!(json["data"]["source"], "sentinel-owner");
    assert_eq!(json["data"]["function"], "set_daily_limit");
    assert_eq!(json["data"]["build_only"], true);
    assert_eq!(json["data"]["daily_limit"], "1250");
    let commands = json["commands"]
        .as_array()
        .expect("commands should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(commands.iter().any(|command| {
        command.contains("stellar contract invoke")
            && command.contains("--id sentinel-policy")
            && command.contains("--source-account sentinel-owner")
            && command.contains("--send no")
            && command.contains("set_daily_limit")
            && command.contains("--daily_limit 1250")
    }));
}

#[test]
fn wallet_smart_policy_allow_and_revoke_resolve_addresses_in_dry_run() {
    let root = init_rewards_project();

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["wallet", "smart", "create", "guardian", "--mode", "passkey"])
        .assert()
        .success();

    let allow = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "smart",
            "policy",
            "allow",
            "guardian",
            "alice",
            "--build-only",
        ])
        .output()
        .expect("allow command should run");
    assert!(allow.status.success());
    let allow_json: Value =
        serde_json::from_slice(&allow.stdout).expect("allow stdout should be valid json");
    assert_eq!(allow_json["action"], "wallet.smart.policy.allow");
    assert_eq!(allow_json["data"]["address"], "<alice>");

    let revoke = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "smart",
            "policy",
            "revoke",
            "guardian",
            "alice",
            "--source",
            "alice",
            "--build-only",
        ])
        .output()
        .expect("revoke command should run");
    assert!(revoke.status.success());
    let revoke_json: Value =
        serde_json::from_slice(&revoke.stdout).expect("revoke stdout should be valid json");
    assert_eq!(revoke_json["action"], "wallet.smart.policy.revoke");
    assert_eq!(revoke_json["data"]["source"], "alice");
    assert_eq!(revoke_json["data"]["address"], "<alice>");
}

#[test]
fn wallet_smart_policy_simulate_previews_all_file_operations() {
    let root = init_rewards_project();

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["wallet", "smart", "create", "guardian", "--mode", "passkey"])
        .assert()
        .success();

    let policy_file = root.join("policy.toml");
    fs::write(
        &policy_file,
        r#"
daily_limit = "900"
allow = ["treasury"]
revoke = ["issuer"]
"#,
    )
    .expect("policy file should be writable");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "smart",
            "policy",
            "simulate",
            "guardian",
            "--file",
            policy_file.to_str().expect("policy file should be UTF-8"),
        ])
        .output()
        .expect("simulate command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.smart.policy.simulate");
    assert_eq!(json["data"]["simulated"], true);
    assert_eq!(json["data"]["build_only"], true);
    assert_eq!(json["data"]["source"], "alice");
    assert_eq!(json["data"]["operation_count"], 3);
    let commands = json["commands"]
        .as_array()
        .expect("commands should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(commands.iter().any(|command| {
        command.contains("set_daily_limit")
            && command.contains("--source-account alice")
            && command.contains("--send no")
    }));
    assert!(commands.iter().any(|command| {
        command.contains(" allow ")
            && command.contains("--source-account alice")
            && command.contains("--send no")
    }));
    assert!(commands.iter().any(|command| {
        command.contains(" revoke ")
            && command.contains("--source-account alice")
            && command.contains("--send no")
    }));
}

#[test]
fn wallet_smart_policy_simulate_accepts_json_policy_files() {
    let root = init_rewards_project();

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["wallet", "smart", "create", "guardian", "--mode", "passkey"])
        .assert()
        .success();

    let policy_file = root.join("policy.json");
    fs::write(
        &policy_file,
        r#"{
  "daily_limit": "700",
  "allow": ["treasury"],
  "revoke": ["issuer"]
}"#,
    )
    .expect("json policy file should be writable");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "smart",
            "policy",
            "simulate",
            "guardian",
            "--file",
            policy_file.to_str().expect("policy file should be UTF-8"),
        ])
        .output()
        .expect("simulate command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.smart.policy.simulate");
    assert_eq!(json["data"]["daily_limit"], "700");
    assert_eq!(json["data"]["allow"][0], "treasury");
    assert_eq!(json["data"]["revoke"][0], "issuer");
}

#[test]
fn wallet_smart_policy_apply_writes_report_to_out_path() {
    let root = init_rewards_project();
    let fake_bin = install_fake_stellar(&root);

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args(["wallet", "smart", "create", "guardian", "--mode", "ed25519"])
        .assert()
        .success();

    let policy_file = root.join("policy.toml");
    fs::write(
        &policy_file,
        r#"
daily_limit = "1000"
allow = ["treasury"]
revoke = ["issuer"]
"#,
    )
    .expect("policy file should be writable");

    let out_path = root.join("dist/wallet.smart.policy.apply.json");
    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "smart",
            "policy",
            "apply",
            "guardian",
            "--file",
            policy_file.to_str().expect("policy file should be UTF-8"),
            "--build-only",
            "--out",
            out_path.to_str().expect("out path should be UTF-8"),
        ])
        .output()
        .expect("apply command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.smart.policy.apply");
    assert_eq!(json["data"]["out"], out_path.display().to_string());

    let file_json: Value = serde_json::from_str(&fs::read_to_string(&out_path).expect("out file"))
        .expect("out file should parse as json");
    assert_eq!(file_json["action"], "wallet.smart.policy.apply");
    assert_eq!(file_json["data"]["out"], out_path.display().to_string());
}

#[test]
fn wallet_smart_info_uses_manifest_policy_contract_path() {
    let root = init_rewards_project();

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["wallet", "smart", "create", "guardian", "--mode", "passkey"])
        .assert()
        .success();

    let original_policy_root = root.join("contracts/guardian-policy");
    let custom_policy_root = root.join("contracts/policies/guardian-policy");
    fs::create_dir_all(
        custom_policy_root
            .parent()
            .expect("custom policy path should have a parent"),
    )
    .expect("custom policy parent should be created");
    fs::rename(&original_policy_root, &custom_policy_root)
        .expect("policy scaffold should be moved");

    let manifest =
        fs::read_to_string(root.join("stellarforge.toml")).expect("manifest should load");
    let updated = manifest.replace(
        "path = \"contracts/guardian-policy\"",
        "path = \"contracts/policies/guardian-policy\"",
    );
    fs::write(root.join("stellarforge.toml"), updated).expect("manifest should be updated");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "wallet", "smart", "info", "guardian"])
        .output()
        .expect("smart info should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.smart.info");
    assert_eq!(
        json["data"]["policy_contract"]["path"],
        fs::canonicalize(&custom_policy_root)
            .expect("custom policy path should canonicalize")
            .display()
            .to_string()
    );
    assert_eq!(json["data"]["policy_contract"]["exists"], true);
}

fn init_rewards_project() -> std::path::PathBuf {
    let temp = tempdir().expect("tempdir should be created");
    let kept = temp.keep();
    let root = kept.join("demo");
    let parent = root
        .parent()
        .expect("demo should have a parent")
        .to_path_buf();
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(parent)
        .args(["init", "demo", "--template", "rewards-loyalty"])
        .assert()
        .success();
    root
}

fn install_fake_stellar(root: &Path) -> std::path::PathBuf {
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
  case "$3" in
    alice|issuer|treasury)
      echo "GFAKEPUBLICKEY"
      exit 0
      ;;
  esac
  echo "missing key $3" >&2
  exit 1
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

fn test_path(fake_bin: &Path) -> String {
    format!(
        "{}:{}",
        fake_bin.display(),
        std::env::var("PATH").expect("PATH should exist")
    )
}
