use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn release_history_lists_current_and_archived_artifacts() {
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

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "release", "history", "testnet"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "release.history");
    assert_eq!(json["network"], "testnet");
    assert!(
        json["data"]["current"]["path"]
            .as_str()
            .expect("current path should be present")
            .ends_with("dist/deploy.testnet.json")
    );
    assert_eq!(json["data"]["current"]["contracts"]["count"], 1);
    let history = json["data"]["history"]
        .as_array()
        .expect("history should be an array");
    assert_eq!(history.len(), 1);
    assert!(
        history[0]["path"]
            .as_str()
            .expect("history path should be present")
            .ends_with("dist/history/deploy.testnet.20260413T000000.000Z.json")
    );
    assert_eq!(history[0]["contracts"]["count"], 1);
    assert_eq!(history[0]["tokens"]["count"], 1);
}

#[test]
fn release_inspect_warns_when_artifact_differs_from_current_state() {
    let root = init_rewards_project();
    seed_testnet_release_lockfile(&root);

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["release", "env", "export", "testnet"])
        .assert()
        .success();

    fs::create_dir_all(root.join("dist/history")).expect("history dir should be created");
    let historical_path = root.join("dist/history/deploy.testnet.20260413T000000.000Z.json");
    fs::write(
        &historical_path,
        r#"{
  "environment": "testnet",
  "contracts": {
    "rewards": {
      "contract_id": "COLD123"
    }
  },
  "tokens": {
    "points": {
      "asset": "POINTS:GOLD123",
      "sac_contract_id": "CSACOLD123"
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
            "inspect",
            "testnet",
            "--path",
            "dist/history/deploy.testnet.20260413T000000.000Z.json",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "release.inspect");
    assert_eq!(json["status"], "warn");
    assert_eq!(json["network"], "testnet");
    assert_eq!(json["data"]["summary"]["kind"], "history");
    assert!(
        json["data"]["path"]
            .as_str()
            .expect("path should be present")
            .ends_with("dist/history/deploy.testnet.20260413T000000.000Z.json")
    );
    let issues = json["data"]["comparison"]["issues"]
        .as_array()
        .expect("comparison issues should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(
        issues
            .iter()
            .any(|issue| issue.contains("contract `rewards`"))
    );
}

#[test]
fn release_history_and_inspect_write_reports_to_out_paths() {
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
    let historical_artifact = root.join("dist/history/deploy.testnet.20260413T000000.000Z.json");
    fs::write(
        &historical_artifact,
        r#"{
  "environment": "testnet",
  "contracts": {
    "rewards": {
      "contract_id": "COLD123"
    }
  },
  "tokens": {
    "points": {
      "asset": "POINTS:GOLD123",
      "sac_contract_id": "CSACOLD123"
    }
  }
}"#,
    )
    .expect("history artifact should be written");

    let history_out = root.join("dist/release.history.json");
    let history = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", empty_bin.path())
        .args([
            "--json",
            "release",
            "history",
            "testnet",
            "--out",
            history_out.to_str().expect("history out should be UTF-8"),
        ])
        .output()
        .expect("history command should run");
    assert!(history.status.success());
    let history_json: Value =
        serde_json::from_slice(&history.stdout).expect("history stdout should be valid json");
    assert_eq!(history_json["action"], "release.history");
    assert_eq!(
        history_json["data"]["out"],
        history_out.to_str().expect("history out should be UTF-8")
    );
    let history_file: Value =
        serde_json::from_str(&read(&history_out)).expect("history out should parse");
    assert_eq!(history_file["action"], "release.history");

    let inspect_out = root.join("dist/release.inspect.json");
    let inspect = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", empty_bin.path())
        .args([
            "--json",
            "release",
            "inspect",
            "testnet",
            "--path",
            "dist/history/deploy.testnet.20260413T000000.000Z.json",
            "--out",
            inspect_out.to_str().expect("inspect out should be UTF-8"),
        ])
        .output()
        .expect("inspect command should run");
    assert!(inspect.status.success());
    let inspect_json: Value =
        serde_json::from_slice(&inspect.stdout).expect("inspect stdout should be valid json");
    assert_eq!(inspect_json["action"], "release.inspect");
    assert_eq!(
        inspect_json["data"]["out"],
        inspect_out.to_str().expect("inspect out should be UTF-8")
    );
    let inspect_file: Value =
        serde_json::from_str(&read(&inspect_out)).expect("inspect out should parse");
    assert_eq!(inspect_file["action"], "release.inspect");
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

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).expect("file should be readable")
}
