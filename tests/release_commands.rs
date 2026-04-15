use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

use stellar_forge::{AppContext, GlobalOptions, release_drift};

#[test]
fn release_status_reports_current_and_latest_history() {
    let root = init_rewards_project();
    seed_testnet_release_lockfile(&root);

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["release", "env", "export", "testnet"])
        .assert()
        .success();

    fs::create_dir_all(root.join("dist/history")).expect("history dir should be created");
    fs::write(
        root.join("dist/history/deploy.testnet.20260413T000000.000Z.json"),
        r#"{
  "environment": "testnet",
  "contracts": {
    "rewards": {
      "contract_id": "CROLLBACK123",
      "alias": "rewards",
      "wasm_hash": "rollbackbeef"
    }
  },
  "tokens": {
    "points": {
      "kind": "asset",
      "asset": "POINTS:GROLLBACK123",
      "sac_contract_id": "CSACROLLBACK123"
    }
  }
}"#,
    )
    .expect("history artifact should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "release", "status", "testnet"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "release.status");
    assert_eq!(json["network"], "testnet");
    assert_eq!(json["status"], "ok");
    assert_eq!(json["data"]["history_count"], 1);
    assert_eq!(json["data"]["current"]["kind"], "current");
    assert_eq!(json["data"]["latest_history"]["kind"], "history");
}

#[test]
fn release_diff_warns_when_selected_history_drifts_from_current() {
    let root = init_rewards_project();
    seed_testnet_release_lockfile(&root);

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["release", "env", "export", "testnet"])
        .assert()
        .success();

    fs::create_dir_all(root.join("dist/history")).expect("history dir should be created");
    fs::write(
        root.join("dist/history/deploy.testnet.20260413T000000.000Z.json"),
        r#"{
  "environment": "testnet",
  "contracts": {
    "rewards": {
      "contract_id": "CROLLBACK123",
      "alias": "rewards",
      "wasm_hash": "rollbackbeef"
    }
  },
  "tokens": {
    "points": {
      "kind": "asset",
      "asset": "POINTS:GROLLBACK123",
      "sac_contract_id": "CSACROLLBACK123"
    }
  }
}"#,
    )
    .expect("history artifact should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "release",
            "diff",
            "testnet",
            "--path",
            "dist/history/deploy.testnet.20260413T000000.000Z.json",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "release.diff");
    assert_eq!(json["status"], "warn");
    assert_eq!(json["network"], "testnet");
    assert_eq!(json["data"]["base"]["kind"], "current");
    assert_eq!(json["data"]["comparison"]["kind"], "selected");
    assert!(
        json["data"]["issues"]
            .as_array()
            .expect("issues should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|issue| issue.contains("contract `rewards`"))
    );
}

#[test]
fn release_drift_reports_current_expected_and_history_divergence() {
    let root = init_rewards_project();
    seed_testnet_release_lockfile(&root);

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["release", "env", "export", "testnet"])
        .assert()
        .success();

    fs::create_dir_all(root.join("dist/history")).expect("history dir should be created");
    fs::write(
        root.join("dist/history/deploy.testnet.20260413T000000.000Z.json"),
        r#"{
  "environment": "testnet",
  "contracts": {
    "rewards": {
      "contract_id": "CARCHIVED123",
      "alias": "rewards",
      "wasm_hash": "rollbackbeef"
    }
  },
  "tokens": {
    "points": {
      "kind": "asset",
      "asset": "POINTS:GOLD123",
      "sac_contract_id": "CSACOLD123"
    }
  }
}"#,
    )
    .expect("history artifact should be written");

    let context = test_context_with_dry_run(&root);
    let report = release_drift(&context, "testnet").expect("drift should succeed");

    assert_eq!(report.action, "release.drift");
    assert_eq!(report.network.as_deref(), Some("testnet"));
    assert_eq!(report.status, "warn");
    assert_eq!(
        report
            .data
            .as_ref()
            .and_then(|data| data.get("expected"))
            .and_then(|value| value.get("kind"))
            .and_then(Value::as_str),
        Some("expected")
    );
    assert_eq!(
        report
            .data
            .as_ref()
            .and_then(|data| data.get("current"))
            .and_then(|value| value.get("kind"))
            .and_then(Value::as_str),
        Some("current")
    );
    assert_eq!(
        report
            .data
            .as_ref()
            .and_then(|data| data.get("latest_history"))
            .and_then(|value| value.get("kind"))
            .and_then(Value::as_str),
        Some("history")
    );
    let history_issues = report
        .data
        .as_ref()
        .and_then(|data| data.get("drift"))
        .and_then(|value| value.get("latest_history_vs_expected"))
        .and_then(Value::as_array)
        .expect("history drift should be reported")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(
        history_issues
            .iter()
            .any(|issue| issue.contains("contract `rewards`"))
    );
    assert!(
        report
            .checks
            .iter()
            .any(|check| check.name == "release:testnet:history:drift")
    );
}

#[test]
fn release_deploy_requires_confirm_mainnet_for_pubnet_kind_networks() {
    let root = init_rewards_project();
    let manifest = format!(
        "{}\n\n[networks.mainnet]\nkind = \"pubnet\"\nrpc_url = \"https://rpc-mainnet.example\"\nhorizon_url = \"https://horizon-mainnet.example\"\nnetwork_passphrase = \"Public Global Stellar Network ; September 2015\"\nallow_http = false\nfriendbot = false\n\n[release.mainnet]\ndeploy_contracts = []\ndeploy_tokens = []\ngenerate_env = false\n",
        read(root.join("stellarforge.toml"))
    );
    fs::write(root.join("stellarforge.toml"), manifest).expect("manifest should be updated");

    let blocked = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "--dry-run", "release", "deploy", "mainnet"])
        .output()
        .expect("blocked command should run");

    assert!(!blocked.status.success());
    assert_eq!(blocked.status.code(), Some(8));
    let blocked_json: Value =
        serde_json::from_slice(&blocked.stdout).expect("blocked stdout should be valid json");
    assert_eq!(blocked_json["action"], "release.deploy");
    assert_eq!(blocked_json["status"], "error");
    assert_eq!(blocked_json["data"]["error_code"], "unsafe");
    assert!(
        blocked_json["message"]
            .as_str()
            .expect("message should be present")
            .contains("mainnet deploy requires --confirm-mainnet")
    );
    assert!(
        blocked_json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| value.as_str()
                == Some("rerun with `stellar forge release deploy <env> --confirm-mainnet`"))
    );

    let confirmed = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "release",
            "deploy",
            "mainnet",
            "--confirm-mainnet",
        ])
        .output()
        .expect("confirmed command should run");

    assert!(confirmed.status.success());
    let confirmed_json: Value =
        serde_json::from_slice(&confirmed.stdout).expect("confirmed stdout should be valid json");
    assert_eq!(confirmed_json["action"], "release.deploy");
    assert_eq!(confirmed_json["network"], "mainnet");
    assert_eq!(confirmed_json["status"], "ok");
}

#[test]
fn release_status_diff_and_drift_write_reports_to_out_paths() {
    let root = init_rewards_project();
    seed_testnet_release_lockfile(&root);
    let empty_bin = tempdir().expect("tempdir should be created");

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", empty_bin.path())
        .args(["release", "env", "export", "testnet"])
        .assert()
        .success();

    fs::create_dir_all(root.join("dist/history")).expect("history dir should be created");
    let history_path = root.join("dist/history/deploy.testnet.20260413T000000.000Z.json");
    fs::write(
        &history_path,
        r#"{
  "environment": "testnet",
  "contracts": {
    "rewards": {
      "contract_id": "CROLLBACK123",
      "alias": "rewards",
      "wasm_hash": "rollbackbeef"
    }
  },
  "tokens": {
    "points": {
      "kind": "asset",
      "asset": "POINTS:GROLLBACK123",
      "sac_contract_id": "CSACROLLBACK123"
    }
  }
}"#,
    )
    .expect("history artifact should be written");

    let status_out = root.join("dist/release.status.json");
    let status = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", empty_bin.path())
        .args([
            "--json",
            "release",
            "status",
            "testnet",
            "--out",
            status_out.to_str().expect("status out should be UTF-8"),
        ])
        .output()
        .expect("status command should run");
    assert!(status.status.success());
    let status_json: Value =
        serde_json::from_slice(&status.stdout).expect("status stdout should be valid json");
    assert_eq!(status_json["action"], "release.status");
    assert_eq!(
        status_json["data"]["out"],
        status_out.to_str().expect("status out should be UTF-8")
    );
    let status_file: Value =
        serde_json::from_str(&read(&status_out)).expect("status out should parse");
    assert_eq!(status_file["action"], "release.status");

    let drift_out = root.join("dist/release.drift.json");
    let drift = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", empty_bin.path())
        .args([
            "--json",
            "release",
            "drift",
            "testnet",
            "--out",
            drift_out.to_str().expect("drift out should be UTF-8"),
        ])
        .output()
        .expect("drift command should run");
    assert!(drift.status.success());
    let drift_json: Value =
        serde_json::from_slice(&drift.stdout).expect("drift stdout should be valid json");
    assert_eq!(drift_json["action"], "release.drift");
    assert_eq!(
        drift_json["data"]["out"],
        drift_out.to_str().expect("drift out should be UTF-8")
    );
    let drift_file: Value =
        serde_json::from_str(&read(&drift_out)).expect("drift out should parse");
    assert_eq!(drift_file["action"], "release.drift");

    let diff_out = root.join("dist/release.diff.json");
    let diff = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", empty_bin.path())
        .args([
            "--json",
            "release",
            "diff",
            "testnet",
            "--path",
            history_path.to_str().expect("history path should be UTF-8"),
            "--out",
            diff_out.to_str().expect("diff out should be UTF-8"),
        ])
        .output()
        .expect("diff command should run");
    assert!(diff.status.success());
    let diff_json: Value =
        serde_json::from_slice(&diff.stdout).expect("diff stdout should be valid json");
    assert_eq!(diff_json["action"], "release.diff");
    assert_eq!(
        diff_json["data"]["out"],
        diff_out.to_str().expect("diff out should be UTF-8")
    );
    let diff_file: Value = serde_json::from_str(&read(&diff_out)).expect("diff out should parse");
    assert_eq!(diff_file["action"], "release.diff");
}

#[test]
fn release_rollback_writes_report_to_out_path() {
    let root = init_rewards_project();
    seed_testnet_release_lockfile(&root);
    fs::create_dir_all(root.join("dist/history")).expect("history dir should be created");

    let history_path = root.join("dist/history/deploy.testnet.20260413T000000.000Z.json");
    fs::write(
        &history_path,
        r#"{
  "environment": "testnet",
  "contracts": {
    "rewards": {
      "contract_id": "CROLLBACK123",
      "alias": "rewards",
      "wasm_hash": "rollbackbeef"
    }
  },
  "tokens": {
    "points": {
      "kind": "asset",
      "asset": "POINTS:GROLLBACK123",
      "sac_contract_id": "CSACROLLBACK123"
    }
  }
}"#,
    )
    .expect("history artifact should be written");

    let out_path = root.join("dist/release.rollback.json");
    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "release",
            "rollback",
            "testnet",
            "--out",
            out_path.to_str().expect("rollback out should be UTF-8"),
        ])
        .output()
        .expect("rollback command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "release.rollback");
    assert_eq!(
        json["data"]["out"],
        out_path.to_str().expect("rollback out should be UTF-8")
    );
    let file: Value = serde_json::from_str(&read(&out_path)).expect("rollback out should parse");
    assert_eq!(file["action"], "release.rollback");
}

#[test]
fn release_plan_writes_report_to_out_path() {
    let root = init_rewards_project();
    seed_testnet_release_lockfile(&root);
    let empty_bin = tempdir().expect("tempdir should be created");

    let plan_out = root.join("dist/release.plan.json");
    let plan = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", empty_bin.path())
        .args([
            "--json",
            "release",
            "plan",
            "testnet",
            "--out",
            plan_out.to_str().expect("plan out should be UTF-8"),
        ])
        .output()
        .expect("plan command should run");
    assert!(plan.status.success());
    let plan_json: Value =
        serde_json::from_slice(&plan.stdout).expect("plan stdout should be valid json");
    assert_eq!(plan_json["action"], "release.plan");
    assert_eq!(
        plan_json["data"]["out"],
        plan_out.to_str().expect("plan out should be UTF-8")
    );
    let plan_file: Value = serde_json::from_str(&read(&plan_out)).expect("plan out should parse");
    assert_eq!(plan_file["action"], "release.plan");

    let export_out = root.join("dist/release.env.json");
    let export = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", empty_bin.path())
        .args([
            "--json",
            "release",
            "env",
            "export",
            "testnet",
            "--out",
            export_out.to_str().expect("export out should be UTF-8"),
        ])
        .output()
        .expect("env export command should run");
    assert!(export.status.success());
    let export_json: Value =
        serde_json::from_slice(&export.stdout).expect("export stdout should be valid json");
    assert_eq!(export_json["action"], "release.env.export");
    let export_file: Value =
        serde_json::from_str(&read(&export_out)).expect("export out should parse");
    assert_eq!(export_file["action"], "release.env.export");

    let verify_out = root.join("dist/release.verify.json");
    let verify = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", empty_bin.path())
        .args([
            "--json",
            "release",
            "verify",
            "testnet",
            "--out",
            verify_out.to_str().expect("verify out should be UTF-8"),
        ])
        .output()
        .expect("verify command should run");
    assert!(verify.status.success());
    let verify_json: Value =
        serde_json::from_slice(&verify.stdout).expect("verify stdout should be valid json");
    assert_eq!(verify_json["action"], "release.verify");
    let verify_file: Value =
        serde_json::from_str(&read(&verify_out)).expect("verify out should parse");
    assert_eq!(verify_file["action"], "release.verify");
}

#[test]
fn release_aliases_sync_invokes_stellar_alias_add_for_contracts_and_sac_tokens() {
    let root = init_rewards_project();
    seed_testnet_release_lockfile(&root);
    let fake_bin = install_fake_stellar_alias_logger(&root);
    let log_path = root.join("dist/fake-stellar.log");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("FAKE_STELLAR_LOG", &log_path)
        .env("PATH", test_path(&fake_bin))
        .args(["--json", "release", "aliases", "sync", "testnet"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "release.aliases.sync");
    assert_eq!(json["network"], "testnet");
    assert_eq!(json["data"]["synced"].as_array().map(Vec::len), Some(2));

    let invocations = read(&log_path);
    assert!(
        invocations
            .contains("contract alias add --overwrite --id CREWARDS123 rewards --network testnet")
    );
    assert!(
        invocations
            .contains("contract alias add --overwrite --id CSAC123 points-sac --network testnet")
    );
}

#[test]
fn release_rollback_rejects_incomplete_artifact_without_mutating_lockfile() {
    let root = init_rewards_project();
    seed_testnet_release_lockfile(&root);
    let original_lockfile = read(root.join("stellarforge.lock.json"));
    let artifact_path = root.join("dist/history/deploy.testnet.incomplete.json");
    fs::create_dir_all(
        artifact_path
            .parent()
            .expect("artifact should have a parent"),
    )
    .expect("history dir should be created");
    fs::write(
        &artifact_path,
        r#"{
  "environment": "testnet",
  "tokens": {}
}"#,
    )
    .expect("incomplete artifact should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "release",
            "rollback",
            "testnet",
            "--to",
            artifact_path
                .to_str()
                .expect("artifact path should be UTF-8"),
        ])
        .output()
        .expect("rollback command should run");

    assert!(!output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "release.rollback");
    assert_eq!(json["status"], "error");
    assert!(
        json["message"]
            .as_str()
            .expect("message should be present")
            .contains("missing `contracts`")
    );
    assert_eq!(read(root.join("stellarforge.lock.json")), original_lockfile);
}

#[test]
fn release_rollback_does_not_mutate_lockfile_when_export_fails() {
    let root = init_rewards_project();
    seed_testnet_release_lockfile(&root);
    let original_lockfile = read(root.join("stellarforge.lock.json"));
    let artifact_path = root.join("dist/history/deploy.testnet.20260413T000000.000Z.json");
    fs::create_dir_all(
        artifact_path
            .parent()
            .expect("artifact should have a parent"),
    )
    .expect("history dir should be created");
    fs::write(
        &artifact_path,
        r#"{
  "environment": "testnet",
  "contracts": {
    "rewards": {
      "contract_id": "CROLLBACK123",
      "alias": "rewards",
      "wasm_hash": "rollbackbeef"
    }
  },
  "tokens": {
    "points": {
      "kind": "asset",
      "asset": "POINTS:GROLLBACK123",
      "sac_contract_id": "CSACROLLBACK123"
    }
  }
}"#,
    )
    .expect("history artifact should be written");
    fs::remove_file(root.join(".env.generated")).expect("env.generated should be removable");
    fs::create_dir(root.join(".env.generated")).expect("env.generated directory should be created");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "release",
            "rollback",
            "testnet",
            "--to",
            artifact_path
                .to_str()
                .expect("artifact path should be UTF-8"),
        ])
        .output()
        .expect("rollback command should run");

    assert!(!output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "release.rollback");
    assert_eq!(json["status"], "error");
    assert!(
        json["message"]
            .as_str()
            .expect("message should be present")
            .contains(".env.generated")
    );
    assert_eq!(read(root.join("stellarforge.lock.json")), original_lockfile);
}

#[test]
fn release_prune_removes_old_history_and_keeps_latest() {
    let root = init_rewards_project();
    fs::create_dir_all(root.join("dist/history")).expect("history dir should be created");
    let old_path = root.join("dist/history/deploy.testnet.20260412T000000.000Z.json");
    let new_path = root.join("dist/history/deploy.testnet.20260413T000000.000Z.json");
    fs::write(
        &old_path,
        r#"{ "environment": "testnet", "contracts": {}, "tokens": {} }"#,
    )
    .expect("old artifact should be written");
    fs::write(
        &new_path,
        r#"{ "environment": "testnet", "contracts": {}, "tokens": {} }"#,
    )
    .expect("new artifact should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "release", "prune", "testnet", "--keep", "1"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "release.prune");
    assert_eq!(json["network"], "testnet");
    assert_eq!(json["data"]["pruned"].as_array().map(Vec::len), Some(1));
    assert_eq!(json["data"]["retained"].as_array().map(Vec::len), Some(1));
    assert!(!old_path.exists());
    assert!(new_path.exists());
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

fn install_fake_stellar_alias_logger(root: &Path) -> std::path::PathBuf {
    let bin_dir = root.join(".test-bin");
    fs::create_dir_all(&bin_dir).expect("bin dir should be created");
    let script_path = bin_dir.join("stellar");
    fs::write(
        &script_path,
        r#"#!/bin/sh
printf '%s\n' "$*" >> "$FAKE_STELLAR_LOG"
if [ "$1" = "contract" ] && [ "$2" = "alias" ] && [ "$3" = "add" ]; then
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

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).expect("file should be readable")
}

fn test_path(fake_bin: &Path) -> String {
    format!(
        "{}:{}",
        fake_bin.display(),
        std::env::var("PATH").expect("PATH should exist")
    )
}

fn test_context_with_dry_run(root: &Path) -> AppContext {
    test_context_with_dry_run_internal(root, true)
}

fn test_context_with_dry_run_internal(root: &Path, dry_run: bool) -> AppContext {
    AppContext::from_globals(&GlobalOptions {
        cwd: Some(root.to_path_buf()),
        manifest: None,
        network: None,
        identity: None,
        json: true,
        quiet: false,
        verbose: 0,
        dry_run,
        yes: false,
    })
    .expect("context should build")
}
