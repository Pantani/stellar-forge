use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

use stellar_forge::{AppContext, GlobalOptions, release_prune};

#[test]
fn release_prune_keeps_only_the_newest_archived_artifact() {
    let root = init_rewards_project();
    seed_testnet_release_lockfile(&root);

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["release", "env", "export", "testnet"])
        .assert()
        .success();

    fs::create_dir_all(root.join("dist/history")).expect("history dir should be created");
    let oldest = root.join("dist/history/deploy.testnet.20260411T000000.000Z.json");
    let middle = root.join("dist/history/deploy.testnet.20260412T000000.000Z.json");
    let newest = root.join("dist/history/deploy.testnet.20260413T000000.000Z.json");
    fs::write(
        &oldest,
        r#"{
  "environment": "testnet",
  "updated_at": "2026-04-11T00:00:00Z",
  "contracts": {
    "rewards": {
      "contract_id": "COLD1"
    }
  },
  "tokens": {
    "points": {
      "asset": "POINTS:GOLD1"
    }
  }
}"#,
    )
    .expect("oldest history artifact should be written");
    fs::write(
        &middle,
        r#"{
  "environment": "testnet",
  "updated_at": "2026-04-12T00:00:00Z",
  "contracts": {
    "rewards": {
      "contract_id": "COLD2"
    }
  },
  "tokens": {
    "points": {
      "asset": "POINTS:GOLD2"
    }
  }
}"#,
    )
    .expect("middle history artifact should be written");
    fs::write(
        &newest,
        r#"{
  "environment": "testnet",
  "updated_at": "2026-04-13T00:00:00Z",
  "contracts": {
    "rewards": {
      "contract_id": "COLD3"
    }
  },
  "tokens": {
    "points": {
      "asset": "POINTS:GOLD3"
    }
  }
}"#,
    )
    .expect("newest history artifact should be written");

    let context = test_context(&root);
    let report = release_prune(&context, "testnet", 1).expect("prune should succeed");

    assert_eq!(report.action, "release.prune");
    assert_eq!(report.status, "ok");
    assert_eq!(report.network.as_deref(), Some("testnet"));
    let pruned = report
        .data
        .as_ref()
        .and_then(|data| data.get("pruned"))
        .and_then(Value::as_array)
        .expect("pruned should be an array");
    assert_eq!(pruned.len(), 2);
    assert!(!oldest.exists());
    assert!(!middle.exists());
    assert!(newest.exists());
    assert!(root.join("dist/deploy.testnet.json").exists());
}

#[test]
fn release_prune_writes_report_to_out_path() {
    let root = init_rewards_project();
    seed_testnet_release_lockfile(&root);

    fs::create_dir_all(root.join("dist/history")).expect("history dir should be created");
    let oldest = root.join("dist/history/deploy.testnet.20260411T000000.000Z.json");
    let newest = root.join("dist/history/deploy.testnet.20260413T000000.000Z.json");
    fs::write(
        &oldest,
        r#"{
  "environment": "testnet",
  "updated_at": "2026-04-11T00:00:00Z",
  "contracts": {
    "rewards": {
      "contract_id": "COLD1"
    }
  },
  "tokens": {
    "points": {
      "asset": "POINTS:GOLD1"
    }
  }
}"#,
    )
    .expect("oldest history artifact should be written");
    fs::write(
        &newest,
        r#"{
  "environment": "testnet",
  "updated_at": "2026-04-13T00:00:00Z",
  "contracts": {
    "rewards": {
      "contract_id": "COLD3"
    }
  },
  "tokens": {
    "points": {
      "asset": "POINTS:GOLD3"
    }
  }
}"#,
    )
    .expect("newest history artifact should be written");

    let out_path = root.join("dist/release.prune.json");
    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "release",
            "prune",
            "testnet",
            "--keep",
            "1",
            "--out",
            out_path.to_str().expect("out path should be utf-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout: Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(stdout["action"], "release.prune");
    assert_eq!(stdout["data"]["out"], out_path.display().to_string());
    assert!(out_path.exists());

    let saved: Value = serde_json::from_str(
        &fs::read_to_string(&out_path).expect("saved report should be readable"),
    )
    .expect("saved report should be valid json");
    assert_eq!(saved["action"], "release.prune");
    assert_eq!(saved["data"]["out"], out_path.display().to_string());
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
