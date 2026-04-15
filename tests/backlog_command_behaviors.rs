use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

#[test]
fn wallet_smart_policy_apply_reads_file_and_builds_all_operations() {
    let root = init_rewards_project();

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["wallet", "smart", "create", "sentinel", "--mode", "ed25519"])
        .assert()
        .success();

    fs::write(
        root.join("policy.toml"),
        r#"source = "alice"
daily_limit = 1250
allow = ["treasury"]
revoke = ["issuer"]
build_only = true
"#,
    )
    .expect("policy file should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "smart",
            "policy",
            "apply",
            "sentinel",
            "--file",
            "policy.toml",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.smart.policy.apply");
    assert_eq!(json["data"]["source"], "alice");
    assert_eq!(json["data"]["build_only"], true);
    assert_eq!(json["data"]["daily_limit"], "1250");
    assert_eq!(json["data"]["operation_count"], 3);
    assert_eq!(json["data"]["allow"][0], "treasury");
    assert_eq!(json["data"]["revoke"][0], "issuer");
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
        command.contains(" allow ") && command.contains("--source-account alice")
    }));
    assert!(commands.iter().any(|command| {
        command.contains(" revoke ") && command.contains("--source-account alice")
    }));
}

#[test]
fn wallet_batch_resume_skips_completed_reported_entries() {
    let root = init_rewards_project();
    fs::write(
        root.join("payments.json"),
        r#"[
  { "to": "alice", "amount": "10" },
  { "to": "treasury", "amount": "5" },
  { "to": "issuer", "amount": "7" }
]
"#,
    )
    .expect("batch file should be written");
    fs::write(
        root.join("payments.report.json"),
        r#"{
  "action": "wallet.batch-pay",
  "data": {
    "payments": [
      { "index": 1, "to": "alice", "amount": "10", "asset": "XLM", "asset_source": "default" }
    ]
  }
}
"#,
    )
    .expect("batch report should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "batch-resume",
            "--from",
            "alice",
            "--asset",
            "XLM",
            "--file",
            "payments.json",
            "--report",
            "payments.report.json",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.batch-resume");
    assert_eq!(json["data"]["count"], 2);
    assert_eq!(json["data"]["resume"]["completed_from_report"][0], 1);
    let selected = json["data"]["resume"]["selected"]
        .as_array()
        .expect("selected should be an array")
        .iter()
        .filter_map(Value::as_u64)
        .collect::<Vec<_>>();
    assert_eq!(selected, vec![2, 3]);
    let preview = json["data"]["preview"]
        .as_array()
        .expect("preview should be an array");
    assert_eq!(preview.len(), 2);
    assert_eq!(preview[0]["index"], 2);
    assert_eq!(preview[1]["index"], 3);
}

#[test]
fn token_airdrop_reconcile_reports_missing_entries_against_report() {
    let root = init_rewards_project();
    fs::write(
        root.join("airdrop.csv"),
        "to,amount\nalice,10\ntreasury,20\n",
    )
    .expect("airdrop csv should be written");
    fs::write(
        root.join("airdrop.report.json"),
        r#"{
  "action": "token.airdrop",
  "data": {
    "payments": [
      { "index": 1, "to": "alice", "amount": "10", "asset": "points", "asset_source": "default" }
    ]
  }
}
"#,
    )
    .expect("airdrop report should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "token",
            "airdrop-reconcile",
            "points",
            "--file",
            "airdrop.csv",
            "--format",
            "csv",
            "--report",
            "airdrop.report.json",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "token.airdrop-reconcile");
    assert_eq!(json["status"], "warn");
    assert_eq!(json["data"]["token"], "points");
    assert_eq!(json["data"]["from"], "treasury");
    assert_eq!(json["data"]["reconcile"]["matched_indices"][0], 1);
    assert_eq!(
        json["data"]["reconcile"]["missing_entries"]
            .as_array()
            .expect("missing entries should be an array")
            .len(),
        1
    );
}

#[test]
fn token_balance_and_wallet_read_surfaces_write_reports_to_out_paths() {
    let root = init_rewards_project();
    let fake_bin = install_fake_stellar(&root);

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args(["wallet", "create", "alice"])
        .assert()
        .success();

    let token_out = root.join("dist/token.balance.json");
    let token_balance = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args([
            "--json",
            "--dry-run",
            "token",
            "balance",
            "points",
            "--holder",
            "alice",
            "--out",
            token_out.to_str().expect("token out should be UTF-8"),
        ])
        .output()
        .expect("token balance should run");
    assert!(token_balance.status.success());
    let token_json: Value =
        serde_json::from_slice(&token_balance.stdout).expect("token stdout should be valid json");
    assert_eq!(token_json["action"], "token.balance");
    assert_eq!(token_json["data"]["out"], token_out.display().to_string());
    let token_file: Value =
        serde_json::from_str(&read(&token_out)).expect("token out should parse");
    assert_eq!(token_file["action"], "token.balance");

    let ls_out = root.join("dist/wallet.ls.json");
    let wallet_ls = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args([
            "--json",
            "wallet",
            "ls",
            "--out",
            ls_out.to_str().expect("wallet ls out should be UTF-8"),
        ])
        .output()
        .expect("wallet ls should run");
    assert!(wallet_ls.status.success());
    let wallet_ls_json: Value =
        serde_json::from_slice(&wallet_ls.stdout).expect("wallet ls stdout should be valid json");
    assert_eq!(wallet_ls_json["action"], "wallet.ls");
    assert_eq!(wallet_ls_json["data"]["out"], ls_out.display().to_string());
    let wallet_ls_file: Value =
        serde_json::from_str(&read(&ls_out)).expect("wallet ls out should parse");
    assert_eq!(wallet_ls_file["action"], "wallet.ls");

    let address_out = root.join("dist/wallet.address.json");
    let wallet_address = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args([
            "--json",
            "wallet",
            "address",
            "alice",
            "--out",
            address_out
                .to_str()
                .expect("wallet address out should be UTF-8"),
        ])
        .output()
        .expect("wallet address should run");
    assert!(wallet_address.status.success());
    let wallet_address_json: Value = serde_json::from_slice(&wallet_address.stdout)
        .expect("wallet address stdout should be valid json");
    assert_eq!(wallet_address_json["action"], "wallet.address");
    assert_eq!(
        wallet_address_json["data"]["out"],
        address_out.display().to_string()
    );
    let wallet_address_file: Value =
        serde_json::from_str(&read(&address_out)).expect("wallet address out should parse");
    assert_eq!(wallet_address_file["action"], "wallet.address");

    let balances_out = root.join("dist/wallet.balances.json");
    let wallet_balances = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "balances",
            "alice",
            "--out",
            balances_out
                .to_str()
                .expect("wallet balances out should be UTF-8"),
        ])
        .output()
        .expect("wallet balances should run");
    assert!(wallet_balances.status.success());
    let wallet_balances_json: Value = serde_json::from_slice(&wallet_balances.stdout)
        .expect("wallet balances stdout should be valid json");
    assert_eq!(wallet_balances_json["action"], "wallet.balances");
    assert_eq!(
        wallet_balances_json["data"]["out"],
        balances_out.display().to_string()
    );
    let wallet_balances_file: Value =
        serde_json::from_str(&read(&balances_out)).expect("wallet balances out should parse");
    assert_eq!(wallet_balances_file["action"], "wallet.balances");

    let receive_out = root.join("dist/wallet.receive.json");
    let wallet_receive = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args([
            "--json",
            "wallet",
            "receive",
            "alice",
            "--sep7",
            "--qr",
            "--asset",
            "XLM",
            "--out",
            receive_out
                .to_str()
                .expect("wallet receive out should be UTF-8"),
        ])
        .output()
        .expect("wallet receive should run");
    assert!(wallet_receive.status.success());
    let wallet_receive_json: Value = serde_json::from_slice(&wallet_receive.stdout)
        .expect("wallet receive stdout should be valid json");
    assert_eq!(wallet_receive_json["action"], "wallet.receive");
    assert_eq!(
        wallet_receive_json["data"]["out"],
        receive_out.display().to_string()
    );
    let wallet_receive_file: Value =
        serde_json::from_str(&read(&receive_out)).expect("wallet receive out should parse");
    assert_eq!(wallet_receive_file["action"], "wallet.receive");
}

#[test]
fn events_export_and_replay_roundtrip_store_state() {
    if !sqlite_available() {
        return;
    }

    let source_root = init_minimal_contract_project();
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&source_root)
        .args(["api", "events", "init"])
        .assert()
        .success();
    seed_sqlite_cursor(
        &source_root,
        "testnet:contract:app",
        "contract",
        "app",
        Some("ledger:321"),
        Some(321),
    );
    seed_sqlite_event(&source_root, "evt-1", "testnet:contract:app");

    let export_path = source_root.join("events-export.json");
    let export_out_path = source_root.join("dist/events.export.report.json");
    let export = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&source_root)
        .args([
            "--json",
            "events",
            "export",
            "--path",
            export_path
                .to_str()
                .expect("export path should be valid UTF-8"),
            "--out",
            export_out_path
                .to_str()
                .expect("export out path should be valid UTF-8"),
        ])
        .output()
        .expect("export command should run");
    assert!(export.status.success());
    let export_json: Value =
        serde_json::from_slice(&export.stdout).expect("export stdout should be valid json");
    assert_eq!(export_json["action"], "events.export");
    assert_eq!(export_json["data"]["cursors"]["count"], 1);
    assert_eq!(export_json["data"]["events"]["count"], 1);
    assert_eq!(
        export_json["data"]["out"],
        export_out_path.display().to_string()
    );
    let exported_file: Value =
        serde_json::from_str(&read(&export_path)).expect("export file should parse");
    assert_eq!(exported_file["version"], 1);
    assert_eq!(exported_file["events"]["count"], 1);
    let export_report: Value =
        serde_json::from_str(&read(&export_out_path)).expect("export report should parse");
    assert_eq!(export_report["action"], "events.export");

    let target_root = init_minimal_contract_project();
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&target_root)
        .args(["api", "events", "init"])
        .assert()
        .success();

    let replay_out_path = target_root.join("dist/events.replay.report.json");
    let replay = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&target_root)
        .args([
            "--json",
            "events",
            "replay",
            "--path",
            export_path
                .to_str()
                .expect("export path should be valid UTF-8"),
            "--out",
            replay_out_path
                .to_str()
                .expect("replay out path should be valid UTF-8"),
        ])
        .output()
        .expect("replay command should run");
    assert!(replay.status.success());
    let replay_json: Value =
        serde_json::from_slice(&replay.stdout).expect("replay stdout should be valid json");
    assert_eq!(replay_json["action"], "events.replay");
    assert_eq!(replay_json["data"]["events"]["count"], 1);
    assert_eq!(
        replay_json["data"]["out"],
        replay_out_path.display().to_string()
    );
    let cursor_snapshot = read(target_root.join("workers/events/cursors.json"));
    assert!(cursor_snapshot.contains("testnet:contract:app"));
    assert_eq!(sqlite_event_count(&target_root), 1);
    let replay_report: Value =
        serde_json::from_str(&read(&replay_out_path)).expect("replay report should parse");
    assert_eq!(replay_report["action"], "events.replay");
}

#[test]
fn events_backfill_dry_run_writes_report_to_out_path() {
    let root = init_minimal_contract_project();
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["api", "events", "init"])
        .assert()
        .success();

    let out_path = root.join("dist/events.backfill.report.json");
    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "events",
            "backfill",
            "app",
            "--count",
            "1",
            "--out",
            out_path
                .to_str()
                .expect("backfill out path should be UTF-8"),
        ])
        .output()
        .expect("backfill command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "events.backfill");
    assert_eq!(json["data"]["out"], out_path.display().to_string());
    let file_json: Value =
        serde_json::from_str(&read(&out_path)).expect("backfill report should parse");
    assert_eq!(file_json["action"], "events.backfill");
}

#[test]
fn doctor_audit_and_fix_scope_release_report_new_actions() {
    let root = init_rewards_project();
    seed_testnet_release_lockfile(&root);
    fs::write(root.join(".env.generated"), "BROKEN=1\n")
        .expect("env.generated should be writable for the test");
    let empty_bin = tempdir().expect("tempdir should be created");
    let audit_out = root.join("dist/doctor.audit.json");
    let fix_out = root.join("dist/doctor.fix.json");

    let audit = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", empty_bin.path())
        .args([
            "--json",
            "doctor",
            "audit",
            "--out",
            audit_out.to_str().expect("audit out should be valid UTF-8"),
        ])
        .output()
        .expect("doctor audit should run");
    assert!(audit.status.success());
    let audit_json: Value =
        serde_json::from_slice(&audit.stdout).expect("audit stdout should be valid json");
    assert_eq!(audit_json["action"], "doctor.audit");
    let audit_file: Value =
        serde_json::from_str(&read(&audit_out)).expect("audit out should parse");
    assert_eq!(audit_file["action"], "doctor.audit");
    assert!(audit_out.exists());
    assert!(
        audit_json["checks"]
            .as_array()
            .expect("checks should be an array")
            .iter()
            .any(|check| check["name"] == "manifest")
    );

    let fix = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", empty_bin.path())
        .args([
            "--json",
            "doctor",
            "fix",
            "--scope",
            "release",
            "--out",
            fix_out.to_str().expect("fix out should be valid UTF-8"),
        ])
        .output()
        .expect("doctor fix should run");
    assert!(fix.status.success());
    let fix_json: Value =
        serde_json::from_slice(&fix.stdout).expect("fix stdout should be valid json");
    assert_eq!(fix_json["action"], "doctor.fix");
    assert_eq!(fix_json["data"]["scope"], "release");
    let fix_file: Value = serde_json::from_str(&read(&fix_out)).expect("fix out should parse");
    assert_eq!(fix_file["action"], "doctor.fix");
    assert!(fix_out.exists());
    let env_generated = read(root.join(".env.generated"));
    assert!(env_generated.contains("PUBLIC_REWARDS_CONTRACT_ID=CREWARDS123"));
    assert!(root.join("dist/deploy.testnet.json").exists());
}

#[test]
fn doctor_root_writes_report_to_out_path() {
    let root = init_rewards_project();
    let out_path = root.join("dist/doctor.json");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "doctor",
            "--out",
            out_path.to_str().expect("out path should be UTF-8"),
        ])
        .output()
        .expect("doctor root should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "doctor");
    assert!(out_path.exists());
    let file_json: Value = serde_json::from_str(&read(&out_path)).expect("out file should parse");
    assert_eq!(file_json["action"], "doctor");
}

#[test]
fn doctor_fix_scope_events_repairs_only_event_store_artifacts() {
    if !sqlite_available() {
        return;
    }

    let root = init_rewards_project();
    seed_sqlite_cursor(
        &root,
        "testnet:contract:rewards",
        "contract",
        "rewards",
        Some("ledger:321"),
        Some(321),
    );
    fs::remove_file(root.join("workers/events/cursors.json")).ok();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "doctor", "fix", "--scope", "events"])
        .output()
        .expect("doctor fix should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "doctor.fix");
    assert_eq!(json["data"]["scope"], "events");
    let repaired = json["data"]["repaired"]
        .as_array()
        .expect("repaired should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(repaired.contains(&"events_cursor_snapshot"));
    assert!(root.join("workers/events/cursors.json").exists());
}

#[test]
fn doctor_env_deps_project_and_network_write_reports_to_out_paths() {
    let root = init_rewards_project();
    let env_out = root.join("dist/doctor.env.json");
    let deps_out = root.join("dist/doctor.deps.json");
    let project_out = root.join("dist/doctor.project.json");
    let network_out = root.join("dist/doctor.network.json");

    let env = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "doctor",
            "env",
            "--out",
            env_out.to_str().expect("env out should be UTF-8"),
        ])
        .output()
        .expect("doctor env should run");
    assert!(env.status.success());
    let env_json: Value =
        serde_json::from_slice(&env.stdout).expect("env stdout should be valid json");
    assert_eq!(env_json["action"], "doctor.env");
    let env_file: Value = serde_json::from_str(&read(&env_out)).expect("env out should parse");
    assert_eq!(env_file["action"], "doctor.env");

    let deps = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "doctor",
            "deps",
            "--out",
            deps_out.to_str().expect("deps out should be UTF-8"),
        ])
        .output()
        .expect("doctor deps should run");
    assert!(deps.status.success());
    let deps_json: Value =
        serde_json::from_slice(&deps.stdout).expect("deps stdout should be valid json");
    assert_eq!(deps_json["action"], "doctor.deps");
    let deps_file: Value = serde_json::from_str(&read(&deps_out)).expect("deps out should parse");
    assert_eq!(deps_file["action"], "doctor.deps");

    let project = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "doctor",
            "project",
            "--out",
            project_out.to_str().expect("project out should be UTF-8"),
        ])
        .output()
        .expect("doctor project should run");
    assert!(project.status.success());
    let project_json: Value =
        serde_json::from_slice(&project.stdout).expect("project stdout should be valid json");
    assert_eq!(project_json["action"], "doctor.project");
    let project_file: Value =
        serde_json::from_str(&read(&project_out)).expect("project out should parse");
    assert_eq!(project_file["action"], "doctor.project");

    let network = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "doctor",
            "network",
            "local",
            "--out",
            network_out.to_str().expect("network out should be UTF-8"),
        ])
        .output()
        .expect("doctor network should run");
    assert!(network.status.success());
    let network_json: Value =
        serde_json::from_slice(&network.stdout).expect("network stdout should be valid json");
    assert_eq!(network_json["action"], "doctor.network");
    let network_file: Value =
        serde_json::from_str(&read(&network_out)).expect("network out should parse");
    assert_eq!(network_file["action"], "doctor.network");
}

fn init_rewards_project() -> PathBuf {
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

fn init_minimal_contract_project() -> PathBuf {
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
        .args(["init", "demo", "--template", "minimal-contract", "--no-api"])
        .assert()
        .success();
    root
}

fn install_fake_stellar(root: &Path) -> PathBuf {
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

fn test_path(fake_bin: &Path) -> String {
    format!(
        "{}:{}",
        fake_bin.display(),
        std::env::var("PATH").expect("PATH should exist")
    )
}

fn seed_testnet_release_lockfile(root: &Path) {
    fs::write(
        root.join("stellarforge.lock.json"),
        r#"{
  "version": 1,
  "environments": {
    "testnet": {
      "contracts": {
        "rewards": {
          "contract_id": "CREWARDS123",
          "alias": "rewards",
          "wasm_hash": "deadbeef",
          "tx_hash": "",
          "deployed_at": "2026-04-14T00:00:00Z"
        }
      },
      "tokens": {
        "points": {
          "kind": "asset",
          "asset": "POINTS:GISSUER123",
          "issuer_identity": "issuer",
          "distribution_identity": "treasury",
          "sac_contract_id": "CSAC123",
          "contract_id": ""
        }
      }
    }
  }
}"#,
    )
    .expect("lockfile should be written");
}

fn seed_sqlite_cursor(
    root: &Path,
    name: &str,
    resource_kind: &str,
    resource_name: &str,
    cursor: Option<&str>,
    last_ledger: Option<i64>,
) {
    let db_path = root.join("apps/api/db/events.sqlite");
    let schema = read(root.join("apps/api/db/schema.sql"));
    let cursor_sql = cursor
        .map(|cursor| format!("'{}'", cursor))
        .unwrap_or_else(|| "null".to_string());
    let ledger_sql = last_ledger
        .map(|ledger| ledger.to_string())
        .unwrap_or_else(|| "null".to_string());
    let sql = format!(
        "{schema}
insert into cursors (name, resource_kind, resource_name, cursor, last_ledger, updated_at)
values ('{name}', '{resource_kind}', '{resource_name}', {cursor_sql}, {ledger_sql}, '2026-04-14T00:00:00Z');
"
    );
    let output = Command::new("sqlite3")
        .current_dir(root)
        .arg(db_path)
        .arg(sql)
        .output()
        .expect("sqlite3 should run");
    assert!(
        output.status.success(),
        "sqlite3 seed should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn seed_sqlite_event(root: &Path, external_id: &str, cursor_name: &str) {
    let db_path = root.join("apps/api/db/events.sqlite");
    let sql = format!(
        "insert into events (external_id, cursor_name, cursor, resource_kind, resource_name, contract_id, event_type, topic, payload, tx_hash, ledger, observed_at) values ('{external_id}', '{cursor_name}', 'ledger:321', 'contract', 'app', 'CAPP123', 'contract', '[]', '{{}}', 'TXHASH123', 321, '2026-04-14T00:00:00Z');"
    );
    let output = Command::new("sqlite3")
        .current_dir(root)
        .arg(db_path)
        .arg(sql)
        .output()
        .expect("sqlite3 should run");
    assert!(
        output.status.success(),
        "sqlite3 event seed should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn sqlite_event_count(root: &Path) -> i64 {
    let output = Command::new("sqlite3")
        .current_dir(root)
        .arg(root.join("apps/api/db/events.sqlite"))
        .arg("-json")
        .arg("select count(*) as count from events;")
        .output()
        .expect("sqlite3 should run");
    assert!(
        output.status.success(),
        "sqlite query should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rows: Value = serde_json::from_slice(&output.stdout).expect("sqlite stdout should be json");
    rows.as_array()
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("count"))
        .and_then(Value::as_i64)
        .expect("count should be present")
}

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).expect("file should exist")
}

fn sqlite_available() -> bool {
    Command::new("sqlite3")
        .arg("-version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
