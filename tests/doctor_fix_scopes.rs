use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

#[test]
fn doctor_fix_scope_scripts_recreates_script_runners() {
    let root = init_rewards_project();
    fs::remove_dir_all(root.join("scripts")).ok();

    let json = run_doctor_fix(&root, "scripts");

    assert_eq!(json["action"], "doctor.fix");
    assert_eq!(json["data"]["scope"], "scripts");
    assert_repaired_contains(&json, &["scripts"]);
    assert_path_exists(&root.join("scripts/reseed.mjs"));
    assert_path_exists(&root.join("scripts/release.mjs"));
    assert_path_exists(&root.join("scripts/doctor.mjs"));
}

#[test]
fn doctor_fix_scope_api_recreates_api_scaffold() {
    let root = init_rewards_project();
    for path in [
        root.join("apps/api/package.json"),
        root.join("apps/api/openapi.json"),
        root.join("apps/api/src/routes/contracts.ts"),
        root.join("apps/api/src/services/contracts/rewards.ts"),
        root.join("apps/api/src/services/tokens/points.ts"),
    ] {
        fs::remove_file(path).ok();
    }

    let json = run_doctor_fix(&root, "api");

    assert_eq!(json["action"], "doctor.fix");
    assert_eq!(json["data"]["scope"], "api");
    assert_repaired_contains(&json, &["api"]);
    assert_path_exists(&root.join("apps/api/package.json"));
    assert_path_exists(&root.join("apps/api/openapi.json"));
    assert_path_exists(&root.join("apps/api/src/routes/contracts.ts"));
    assert_path_exists(&root.join("apps/api/src/services/contracts/rewards.ts"));
    assert_path_exists(&root.join("apps/api/src/services/tokens/points.ts"));
}

#[test]
fn doctor_fix_scope_frontend_recreates_frontend_scaffold() {
    let root = init_rewards_project();
    for path in [
        root.join("apps/web/package.json"),
        root.join("apps/web/index.html"),
        root.join("apps/web/src/main.tsx"),
        root.join("apps/web/scripts/ui-smoke.mjs"),
        root.join("apps/web/scripts/ui-browser-smoke.mjs"),
        root.join("apps/web/src/generated/stellar.ts"),
    ] {
        fs::remove_file(path).ok();
    }

    let json = run_doctor_fix(&root, "frontend");

    assert_eq!(json["action"], "doctor.fix");
    assert_eq!(json["data"]["scope"], "frontend");
    assert_repaired_contains(&json, &["frontend"]);
    assert_path_exists(&root.join("apps/web/package.json"));
    assert_path_exists(&root.join("apps/web/index.html"));
    assert_path_exists(&root.join("apps/web/src/main.tsx"));
    assert_path_exists(&root.join("apps/web/scripts/ui-smoke.mjs"));
    assert_path_exists(&root.join("apps/web/scripts/ui-browser-smoke.mjs"));
    assert_path_exists(&root.join("apps/web/src/generated/stellar.ts"));
}

#[test]
fn doctor_fix_scope_lockfile_restores_lockfile() {
    let root = init_rewards_project();
    fs::remove_file(root.join("stellarforge.lock.json")).ok();

    let json = run_doctor_fix(&root, "lockfile");

    assert_eq!(json["action"], "doctor.fix");
    assert_eq!(json["data"]["scope"], "lockfile");
    assert_repaired_contains(&json, &["lockfile"]);
    assert_path_exists(&root.join("stellarforge.lock.json"));
    let lockfile = read(root.join("stellarforge.lock.json"));
    assert!(lockfile.contains("\"version\": 1"));
}

#[test]
fn doctor_fix_scope_all_rebuilds_managed_artifacts() {
    let root = init_rewards_project();
    seed_testnet_release_lockfile(&root);

    for path in [
        root.join(".env.example"),
        root.join(".env.generated"),
        root.join("scripts"),
        root.join("apps/api/package.json"),
        root.join("apps/api/openapi.json"),
        root.join("apps/api/src/routes/contracts.ts"),
        root.join("apps/api/src/services/contracts/rewards.ts"),
        root.join("apps/api/src/services/tokens/points.ts"),
        root.join("apps/web/package.json"),
        root.join("apps/web/index.html"),
        root.join("apps/web/src/main.tsx"),
        root.join("apps/web/scripts/ui-smoke.mjs"),
        root.join("apps/web/scripts/ui-browser-smoke.mjs"),
        root.join("apps/web/src/generated/stellar.ts"),
        root.join("workers/events/ingest-events.mjs"),
        root.join("workers/events/cursors.json"),
        root.join("dist/deploy.testnet.json"),
    ] {
        if path.is_dir() {
            fs::remove_dir_all(path).ok();
        } else {
            fs::remove_file(path).ok();
        }
    }

    let json = run_doctor_fix(&root, "all");

    assert_eq!(json["action"], "doctor.fix");
    assert_eq!(json["data"]["scope"], "all");
    assert_repaired_contains(
        &json,
        &[
            "env_example",
            "scripts",
            "api",
            "lockfile",
            "events_worker",
            "events_cursor_snapshot",
            "frontend",
            "release_env",
            "release_artifact",
        ],
    );
    assert_path_exists(&root.join(".env.example"));
    assert_path_exists(&root.join("scripts/reseed.mjs"));
    assert_path_exists(&root.join("scripts/release.mjs"));
    assert_path_exists(&root.join("scripts/doctor.mjs"));
    assert_path_exists(&root.join("apps/api/package.json"));
    assert_path_exists(&root.join("apps/api/openapi.json"));
    assert_path_exists(&root.join("apps/api/src/routes/contracts.ts"));
    assert_path_exists(&root.join("apps/api/src/services/contracts/rewards.ts"));
    assert_path_exists(&root.join("apps/api/src/services/tokens/points.ts"));
    assert_path_exists(&root.join("apps/web/package.json"));
    assert_path_exists(&root.join("apps/web/index.html"));
    assert_path_exists(&root.join("apps/web/src/main.tsx"));
    assert_path_exists(&root.join("apps/web/scripts/ui-smoke.mjs"));
    assert_path_exists(&root.join("apps/web/scripts/ui-browser-smoke.mjs"));
    assert_path_exists(&root.join("apps/web/src/generated/stellar.ts"));
    assert_path_exists(&root.join("workers/events/ingest-events.mjs"));
    assert_path_exists(&root.join("workers/events/cursors.json"));
    assert_path_exists(&root.join(".env.generated"));
    assert_path_exists(&root.join("dist/deploy.testnet.json"));
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
        .args([
            "init",
            "demo",
            "--template",
            "rewards-loyalty",
            "--network",
            "testnet",
        ])
        .assert()
        .success();
    root
}

fn run_doctor_fix(root: &Path, scope: &str) -> Value {
    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(root)
        .args([
            "--json",
            "--network",
            "testnet",
            "doctor",
            "fix",
            "--scope",
            scope,
        ])
        .output()
        .expect("doctor fix should run");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("stdout should be valid json")
}

fn assert_repaired_contains(json: &Value, expected: &[&str]) {
    let repaired = json["data"]["repaired"]
        .as_array()
        .expect("repaired should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    for item in expected {
        assert!(
            repaired.contains(item),
            "expected repaired entries to contain `{item}`, got {repaired:?}"
        );
    }
}

fn assert_path_exists(path: &Path) {
    assert!(path.exists(), "expected {} to exist", path.display());
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
    fs::read_to_string(path).expect("file should exist")
}
