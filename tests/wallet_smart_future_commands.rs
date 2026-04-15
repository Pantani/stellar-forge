use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn wallet_smart_onboard_summarizes_paths_env_and_next_steps() {
    let root = init_rewards_project();

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["wallet", "smart", "scaffold", "guardian"])
        .assert()
        .success();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "wallet", "smart", "onboard", "guardian"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.smart.onboard");
    assert_eq!(json["data"]["wallet"]["name"], "guardian");
    assert_eq!(json["data"]["wallet"]["mode"], "passkey");
    assert_eq!(json["data"]["environment"]["name"], "testnet");
    assert_eq!(
        json["data"]["paths"]["onboarding_app"],
        "apps/smart-wallet/guardian"
    );
    assert_eq!(
        json["data"]["paths"]["onboarding_root"],
        canonical_display(&root.join("apps/smart-wallet/guardian"))
    );
    assert_eq!(
        json["data"]["paths"]["policy_contract_root"],
        canonical_display(&root.join("contracts/guardian-policy"))
    );
    assert_eq!(json["data"]["policy_contract"]["name"], "guardian-policy");
    assert_eq!(json["data"]["policy_contract"]["deployed"], false);
    assert_eq!(json["data"]["env"]["SMART_WALLET_MODE"], "passkey");
    assert_eq!(json["data"]["env"]["SMART_WALLET_NETWORK"], "testnet");
    assert_eq!(
        json["data"]["env"]["SMART_WALLET_POLICY_CONTRACT"],
        "guardian-policy"
    );

    let next = json["next"]
        .as_array()
        .expect("next should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(
        next.iter()
            .any(|step| step.contains("contract build guardian-policy"))
    );
    assert!(
        next.iter()
            .any(|step| step.contains("contract deploy guardian-policy --env testnet"))
    );
    assert!(
        next.iter()
            .any(|step| step.contains("pnpm") && step.contains("apps/smart-wallet/guardian"))
    );
    assert!(
        json["data"]["checklist"]
            .as_array()
            .expect("checklist should be an array")
            .iter()
            .any(|item| item
                .as_str()
                .is_some_and(|item| item.contains("WebAuthn ceremony")))
    );
}

#[test]
fn wallet_smart_controller_rotate_updates_manifest_and_scaffold_files() {
    let root = init_rewards_project();
    let fake_bin = install_fake_stellar(&root);

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args(["wallet", "smart", "create", "sentinel", "--mode", "ed25519"])
        .assert()
        .success();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args([
            "--json",
            "wallet",
            "smart",
            "controller",
            "rotate",
            "sentinel",
            "sentinel-ops",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.smart.controller.rotate");
    assert_eq!(json["data"]["wallet"], "sentinel");
    assert_eq!(json["data"]["controller_identity"], "sentinel-ops");
    assert_eq!(json["data"]["controller_created"], true);
    assert_eq!(
        json["data"]["paths"]["onboarding"],
        canonical_display(&root.join("apps/smart-wallet/sentinel"))
    );
    assert!(
        json["warnings"]
            .as_array()
            .expect("warnings should be an array")
            .iter()
            .any(|warning| warning.as_str().is_some_and(
                |warning| warning.contains("previous controller identity `sentinel-owner`")
            ))
    );
    assert!(
        json["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|command| {
                command.contains("stellar keys generate sentinel-ops")
                    && command.contains("--network testnet")
            })
    );

    let manifest = fs::read_to_string(root.join("stellarforge.toml"))
        .expect("manifest should be readable after rotation");
    let env = fs::read_to_string(root.join("apps/smart-wallet/sentinel/.env.example"))
        .expect(".env.example should be readable after rotation");
    let main_ts = fs::read_to_string(root.join("apps/smart-wallet/sentinel/src/main.ts"))
        .expect("main.ts should be readable after rotation");

    assert!(manifest.contains("controller_identity = \"sentinel-ops\""));
    assert!(env.contains("SMART_WALLET_CONTROLLER_IDENTITY=sentinel-ops"));
    assert!(env.contains("SMART_WALLET_NETWORK=testnet"));
    assert!(main_ts.contains("sentinel-ops"));
    assert!(main_ts.contains("controller-signing"));
    assert!(main_ts.contains("wallet address sentinel-ops"));
    assert!(main_ts.contains("wallet smart policy sync sentinel"));
}

#[test]
fn wallet_smart_materialize_guarantees_scaffold_and_controller_identity() {
    let root = init_rewards_project();
    let fake_bin = install_fake_stellar(&root);

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args(["wallet", "smart", "create", "sentinel", "--mode", "ed25519"])
        .assert()
        .success();

    fs::remove_dir_all(root.join("apps/smart-wallet/sentinel"))
        .expect("onboarding app should be removable before materialize");
    fs::remove_dir_all(root.join("contracts/sentinel-policy"))
        .expect("policy contract should be removable before materialize");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args([
            "--json",
            "wallet",
            "smart",
            "materialize",
            "sentinel",
            "--no-policy-deploy",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.smart.materialize");
    assert_eq!(json["data"]["wallet"], "sentinel");
    assert_eq!(json["data"]["controller_identity"], "sentinel-owner");
    assert_eq!(json["data"]["policy_contract"]["name"], "sentinel-policy");
    assert_eq!(json["data"]["policy_contract"]["deployed"], false);
    assert_eq!(json["data"]["policy_contract"]["deployed_now"], false);
    assert_eq!(
        json["data"]["paths"]["onboarding"],
        canonical_display(&root.join("apps/smart-wallet/sentinel"))
    );
    assert_eq!(
        json["data"]["paths"]["policy_contract"],
        canonical_display(&root.join("contracts/sentinel-policy"))
    );

    let env = fs::read_to_string(root.join("apps/smart-wallet/sentinel/.env.example"))
        .expect(".env.example should be recreated after materialize");
    let main_ts = fs::read_to_string(root.join("apps/smart-wallet/sentinel/src/main.ts"))
        .expect("main.ts should be recreated after materialize");
    let policy_lib = fs::read_to_string(root.join("contracts/sentinel-policy/src/lib.rs"))
        .expect("policy contract should be recreated after materialize");

    assert!(env.contains("SMART_WALLET_CONTROLLER_IDENTITY=sentinel-owner"));
    assert!(env.contains("SMART_WALLET_NETWORK=testnet"));
    assert!(main_ts.contains("sentinel-owner"));
    assert!(main_ts.contains("Copy env block"));
    assert!(main_ts.contains("wallet fund sentinel-owner"));
    assert!(policy_lib.contains("set_daily_limit"));

    let next = json["next"]
        .as_array()
        .expect("next should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(
        next.iter()
            .any(|step| step.contains("wallet smart onboard sentinel"))
    );
    assert!(next.iter().any(|step| step.contains("contract deploy")));
    assert!(
        json["warnings"]
            .as_array()
            .expect("warnings should be an array")
            .iter()
            .any(|warning| warning
                .as_str()
                .is_some_and(|warning| warning.contains("policy contract deploy skipped")))
    );
}

#[test]
fn wallet_smart_read_surfaces_write_reports_to_out_paths() {
    let root = init_rewards_project();
    let fake_bin = install_fake_stellar(&root);

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args(["wallet", "smart", "create", "sentinel", "--mode", "ed25519"])
        .assert()
        .success();

    let onboard_out = root.join("dist/wallet.smart.onboard.json");
    let onboard = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args([
            "--json",
            "wallet",
            "smart",
            "onboard",
            "sentinel",
            "--out",
            onboard_out.to_str().expect("onboard out should be UTF-8"),
        ])
        .output()
        .expect("command should run");
    assert!(onboard.status.success());
    let onboard_json: Value =
        serde_json::from_slice(&onboard.stdout).expect("onboard stdout should be valid json");
    assert_eq!(onboard_json["action"], "wallet.smart.onboard");
    assert_eq!(
        onboard_json["data"]["out"],
        onboard_out.display().to_string()
    );
    let onboard_file: Value =
        serde_json::from_str(&fs::read_to_string(&onboard_out).expect("onboard out should parse"))
            .expect("onboard out should parse as json");
    assert_eq!(onboard_file["action"], "wallet.smart.onboard");

    let policy_info_out = root.join("dist/wallet.smart.policy.info.json");
    let policy_info = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args([
            "--json",
            "wallet",
            "smart",
            "policy",
            "info",
            "sentinel",
            "--out",
            policy_info_out
                .to_str()
                .expect("policy info out should be UTF-8"),
        ])
        .output()
        .expect("command should run");
    assert!(policy_info.status.success());
    let policy_info_json: Value = serde_json::from_slice(&policy_info.stdout)
        .expect("policy info stdout should be valid json");
    assert_eq!(policy_info_json["action"], "wallet.smart.policy.info");
    assert_eq!(
        policy_info_json["data"]["out"],
        policy_info_out.display().to_string()
    );
    let policy_info_file: Value = serde_json::from_str(
        &fs::read_to_string(&policy_info_out).expect("policy info out should parse"),
    )
    .expect("policy info out should parse as json");
    assert_eq!(policy_info_file["action"], "wallet.smart.policy.info");

    let policy_diff_out = root.join("dist/wallet.smart.policy.diff.json");
    let policy_diff = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "smart",
            "policy",
            "diff",
            "sentinel",
            "--out",
            policy_diff_out
                .to_str()
                .expect("policy diff out should be UTF-8"),
        ])
        .output()
        .expect("command should run");
    assert!(policy_diff.status.success());
    let policy_diff_json: Value = serde_json::from_slice(&policy_diff.stdout)
        .expect("policy diff stdout should be valid json");
    assert_eq!(policy_diff_json["action"], "wallet.smart.policy.diff");
    assert_eq!(
        policy_diff_json["data"]["out"],
        policy_diff_out.display().to_string()
    );
    let policy_diff_file: Value = serde_json::from_str(
        &fs::read_to_string(&policy_diff_out).expect("policy diff out should parse"),
    )
    .expect("policy diff out should parse as json");
    assert_eq!(policy_diff_file["action"], "wallet.smart.policy.diff");

    let policy_sync_out = root.join("dist/wallet.smart.policy.sync.json");
    let policy_sync = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "smart",
            "policy",
            "sync",
            "sentinel",
            "--out",
            policy_sync_out
                .to_str()
                .expect("policy sync out should be UTF-8"),
        ])
        .output()
        .expect("command should run");
    assert!(policy_sync.status.success());
    let policy_sync_json: Value = serde_json::from_slice(&policy_sync.stdout)
        .expect("policy sync stdout should be valid json");
    assert_eq!(policy_sync_json["action"], "wallet.smart.policy.sync");
    assert_eq!(
        policy_sync_json["data"]["out"],
        policy_sync_out.display().to_string()
    );
    let policy_sync_file: Value = serde_json::from_str(
        &fs::read_to_string(&policy_sync_out).expect("policy sync out should parse"),
    )
    .expect("policy sync out should parse as json");
    assert_eq!(policy_sync_file["action"], "wallet.smart.policy.sync");
}

#[test]
fn wallet_smart_mutating_commands_write_reports_to_out_paths() {
    let root = init_rewards_project();
    let fake_bin = install_fake_stellar(&root);

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args([
            "wallet",
            "smart",
            "create",
            "provisioned",
            "--mode",
            "ed25519",
        ])
        .assert()
        .success();

    let provision_out = root.join("dist/wallet.smart.provision.json");
    let provision = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args([
            "--json",
            "wallet",
            "smart",
            "provision",
            "provisioned",
            "--address",
            "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            "--fund",
            "--out",
            provision_out
                .to_str()
                .expect("provision out should be UTF-8"),
        ])
        .output()
        .expect("provision command should run");
    assert!(provision.status.success());
    let provision_json: Value =
        serde_json::from_slice(&provision.stdout).expect("provision stdout should be valid json");
    assert_eq!(provision_json["action"], "wallet.smart.provision");
    assert_eq!(
        provision_json["data"]["out"],
        provision_out.display().to_string()
    );
    let provision_file: Value = serde_json::from_str(
        &fs::read_to_string(&provision_out).expect("provision out should parse"),
    )
    .expect("provision out should parse as json");
    assert_eq!(provision_file["action"], "wallet.smart.provision");

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args([
            "wallet",
            "smart",
            "create",
            "materialized",
            "--mode",
            "ed25519",
        ])
        .assert()
        .success();
    fs::remove_dir_all(root.join("apps/smart-wallet/materialized"))
        .expect("materialized app should be removable before materialize");
    fs::remove_dir_all(root.join("contracts/materialized-policy"))
        .expect("materialized policy should be removable before materialize");

    let materialize_out = root.join("dist/wallet.smart.materialize.json");
    let materialize = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args([
            "--json",
            "wallet",
            "smart",
            "materialize",
            "materialized",
            "--no-policy-deploy",
            "--out",
            materialize_out
                .to_str()
                .expect("materialize out should be UTF-8"),
        ])
        .output()
        .expect("materialize command should run");
    assert!(materialize.status.success());
    let materialize_json: Value = serde_json::from_slice(&materialize.stdout)
        .expect("materialize stdout should be valid json");
    assert_eq!(materialize_json["action"], "wallet.smart.materialize");
    assert_eq!(
        materialize_json["data"]["out"],
        materialize_out.display().to_string()
    );
    let materialize_file: Value = serde_json::from_str(
        &fs::read_to_string(&materialize_out).expect("materialize out should parse"),
    )
    .expect("materialize out should parse as json");
    assert_eq!(materialize_file["action"], "wallet.smart.materialize");

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args(["wallet", "smart", "create", "rotating", "--mode", "ed25519"])
        .assert()
        .success();

    let rotate_out = root.join("dist/wallet.smart.controller.rotate.json");
    let rotate = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args([
            "--json",
            "wallet",
            "smart",
            "controller",
            "rotate",
            "rotating",
            "rotating-ops",
            "--out",
            rotate_out.to_str().expect("rotate out should be UTF-8"),
        ])
        .output()
        .expect("rotate command should run");
    assert!(rotate.status.success());
    let rotate_json: Value =
        serde_json::from_slice(&rotate.stdout).expect("rotate stdout should be valid json");
    assert_eq!(rotate_json["action"], "wallet.smart.controller.rotate");
    assert_eq!(rotate_json["data"]["out"], rotate_out.display().to_string());
    let rotate_file: Value =
        serde_json::from_str(&fs::read_to_string(&rotate_out).expect("rotate out should parse"))
            .expect("rotate out should parse as json");
    assert_eq!(rotate_file["action"], "wallet.smart.controller.rotate");
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

fn canonical_display(path: &Path) -> String {
    fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}
