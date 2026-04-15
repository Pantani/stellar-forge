use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn dev_snapshot_save_and_load_restore_local_state_files() {
    let root = init_rewards_project();
    let lockfile_path = root.join("stellarforge.lock.json");
    let env_generated_path = root.join(".env.generated");
    let deploy_path = root.join("dist/deploy.testnet.json");
    let cursor_path = root.join("workers/events/cursors.json");

    fs::write(
        &lockfile_path,
        r#"{
  "version": 1,
  "environments": {
    "testnet": {
      "contracts": {
        "rewards": {
          "contract_id": "CSNAPSHOT123",
          "alias": "rewards",
          "wasm_hash": "abc123"
        }
      }
    }
  }
}"#,
    )
    .expect("lockfile should be writable");
    fs::write(
        &env_generated_path,
        "STELLAR_NETWORK=testnet\nREWARDS_CONTRACT_ID=CSNAPSHOT123\n",
    )
    .expect(".env.generated should be writable");
    fs::create_dir_all(root.join("dist")).expect("dist dir should exist");
    fs::write(
        &deploy_path,
        r#"{
  "environment": "testnet",
  "contracts": {
    "rewards": {
      "contract_id": "CSNAPSHOT123",
      "alias": "rewards"
    }
  }
}"#,
    )
    .expect("deploy artifact should be writable");
    fs::write(
        &cursor_path,
        r#"{
  "cursors": {
    "testnet:contract:rewards": {
      "resource_kind": "contract",
      "resource_name": "rewards",
      "cursor": "123-0",
      "last_ledger": 123,
      "updated_at": "2026-04-14T12:00:00Z"
    }
  }
}"#,
    )
    .expect("cursor snapshot should be writable");

    let save_out = root.join("dist/dev.snapshot.save.json");
    let save = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "dev",
            "snapshot",
            "save",
            "baseline",
            "--out",
            save_out.to_str().expect("save out should be UTF-8"),
        ])
        .output()
        .expect("save command should run");

    assert!(save.status.success());
    let save_json: Value =
        serde_json::from_slice(&save.stdout).expect("save stdout should be valid json");
    assert_eq!(save_json["action"], "dev.snapshot.save");
    assert_eq!(save_json["data"]["name"], "baseline");

    let snapshot_path = root.join("dist/snapshots/dev.testnet.baseline.json");
    assert!(snapshot_path.exists(), "snapshot file should be created");

    fs::write(
        &lockfile_path,
        r#"{
  "version": 1,
  "environments": {
    "testnet": {
      "contracts": {
        "rewards": {
          "contract_id": "CSNAPSHOT999",
          "alias": "rewards",
          "wasm_hash": "def456"
        }
      }
    }
  }
}"#,
    )
    .expect("lockfile should be rewritable before second save");
    fs::write(
        &env_generated_path,
        "STELLAR_NETWORK=testnet\nREWARDS_CONTRACT_ID=CSNAPSHOT999\n",
    )
    .expect("env should be rewritable before second save");

    let second_save = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "dev", "snapshot", "save", "baseline"])
        .output()
        .expect("second save command should run");
    assert!(second_save.status.success());
    let second_save_json: Value =
        serde_json::from_slice(&second_save.stdout).expect("second save stdout should be json");
    let archived_previous = second_save_json["data"]["archived_previous"]
        .as_str()
        .expect("archive path should be present");
    assert!(
        archived_previous.contains("dist/snapshots/history/dev.testnet.baseline."),
        "archive path should point at snapshot history"
    );
    let archived_contents =
        fs::read_to_string(archived_previous).expect("archived snapshot should be readable");
    assert!(
        archived_contents.contains("CSNAPSHOT123"),
        "archive should contain the previous snapshot contents"
    );

    fs::write(
        &lockfile_path,
        "{\n  \"version\": 1,\n  \"environments\": {}\n}\n",
    )
    .expect("lockfile should be mutable");
    fs::write(
        &env_generated_path,
        "STELLAR_NETWORK=testnet\nREWARDS_CONTRACT_ID=CBROKEN\n",
    )
    .expect(".env.generated should be mutable");
    fs::write(
        &deploy_path,
        "{\n  \"environment\": \"testnet\",\n  \"contracts\": {}\n}\n",
    )
    .expect("deploy artifact should be mutable");
    fs::write(&cursor_path, "{\n  \"cursors\": {}\n}\n").expect("cursors should be mutable");
    fs::remove_file(&snapshot_path).expect("current snapshot should be removable");

    let load_out = root.join("dist/dev.snapshot.load.json");
    let load = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "dev",
            "snapshot",
            "load",
            "baseline",
            "--out",
            load_out.to_str().expect("load out should be UTF-8"),
        ])
        .output()
        .expect("load command should run");

    assert!(load.status.success());
    let load_json: Value =
        serde_json::from_slice(&load.stdout).expect("load stdout should be valid json");
    assert_eq!(load_json["action"], "dev.snapshot.load");
    assert_eq!(load_json["data"]["name"], "baseline");
    assert_eq!(load_json["data"]["source"], "history");
    assert!(
        fs::read_to_string(&lockfile_path)
            .expect("restored lockfile should be readable")
            .contains("CSNAPSHOT123")
    );
    assert!(
        fs::read_to_string(&env_generated_path)
            .expect("restored env should be readable")
            .contains("CSNAPSHOT123")
    );
    assert!(
        fs::read_to_string(&deploy_path)
            .expect("restored deploy artifact should be readable")
            .contains("CSNAPSHOT123")
    );
    assert!(
        fs::read_to_string(&cursor_path)
            .expect("restored cursors should be readable")
            .contains("testnet:contract:rewards")
    );
}

#[test]
fn dev_snapshot_load_with_explicit_missing_path_does_not_fallback_to_history() {
    let root = init_rewards_project();
    let missing_path = root.join("dist/custom.snapshot.json");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "dev",
            "snapshot",
            "load",
            "baseline",
            "--path",
            missing_path
                .to_str()
                .expect("missing snapshot path should be UTF-8"),
        ])
        .output()
        .expect("explicit load command should run");

    assert!(!output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "dev.snapshot.load");
    assert_eq!(json["status"], "error");
    assert!(
        json["message"]
            .as_str()
            .expect("error message should be present")
            .contains("failed to read")
    );
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
