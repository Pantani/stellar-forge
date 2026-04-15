use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

use stellar_forge::{AppContext, GlobalOptions, release_diff, release_status};

#[test]
fn release_status_summarizes_current_and_latest_history_snapshots() {
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
  "updated_at": "2026-04-13T00:00:00Z",
  "contracts": {
    "rewards": {
      "contract_id": "COLD123"
    }
  },
  "tokens": {
    "points": {
      "asset": "POINTS:GOLD123"
    }
  }
}"#,
    )
    .expect("history artifact should be written");

    let context = test_context(&root);
    let report = release_status(&context, "testnet").expect("status should succeed");

    assert_eq!(report.action, "release.status");
    assert_eq!(report.status, "ok");
    assert_eq!(report.network.as_deref(), Some("testnet"));
    assert_eq!(
        report
            .data
            .as_ref()
            .and_then(|data| data.get("history_count"))
            .and_then(Value::as_u64),
        Some(1)
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
    assert!(
        report
            .checks
            .iter()
            .any(|check| check.name == "release:testnet:deploy-artifact")
    );
}

#[test]
fn release_diff_compares_against_an_explicit_history_artifact() {
    let root = init_rewards_project();
    seed_testnet_release_lockfile(&root);

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["release", "env", "export", "testnet"])
        .assert()
        .success();

    fs::create_dir_all(root.join("dist/history")).expect("history dir should be created");
    let history_path = root.join("dist/history/deploy.testnet.20260413T000000.000Z.json");
    fs::write(
        &history_path,
        r#"{
  "environment": "testnet",
  "updated_at": "2026-04-13T00:00:00Z",
  "contracts": {
    "rewards": {
      "contract_id": "COLD123"
    }
  },
  "tokens": {
    "points": {
      "asset": "POINTS:GOLD123"
    }
  }
}"#,
    )
    .expect("history artifact should be written");

    let context = test_context(&root);
    let report = release_diff(
        &context,
        "testnet",
        Some(Path::new(
            "dist/history/deploy.testnet.20260413T000000.000Z.json",
        )),
    )
    .expect("diff should succeed");

    assert_eq!(report.action, "release.diff");
    assert_eq!(report.status, "warn");
    assert_eq!(report.network.as_deref(), Some("testnet"));
    assert_eq!(
        report
            .data
            .as_ref()
            .and_then(|data| data.get("comparison"))
            .and_then(|value| value.get("kind"))
            .and_then(Value::as_str),
        Some("selected")
    );
    let issues = report
        .data
        .as_ref()
        .and_then(|data| data.get("issues"))
        .and_then(Value::as_array)
        .expect("issues should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(
        issues
            .iter()
            .any(|issue| issue.contains("contract `rewards` differs"))
    );
}

fn test_context(root: &Path) -> AppContext {
    AppContext::from_globals(&GlobalOptions {
        cwd: Some(root.to_path_buf()),
        manifest: None,
        network: None,
        identity: None,
        json: true,
        quiet: false,
        verbose: 0,
        dry_run: false,
        yes: false,
    })
    .expect("context should build")
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
