use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

mod support;

use support::init_rewards_project;

#[test]
fn project_sync_dry_run_reports_artifacts_without_touching_drifted_files() {
    let root = init_rewards_project();
    let env_path = root.join(".env.example");
    let openapi_path = root.join("apps/api/openapi.json");
    let generated_state_path = root.join("apps/web/src/generated/stellar.ts");

    fs::write(&env_path, "BROKEN=1\n").expect("env example should be made stale");
    fs::remove_file(&openapi_path).expect("openapi should be removable for the test");
    let generated_state_before = read(&generated_state_path);

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "--dry-run", "project", "sync"])
        .output()
        .expect("project sync dry-run should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "project.sync");
    assert_eq!(json["status"], "ok");
    assert_synced_modules(&json, &["env_example", "api", "frontend"]);
    assert_artifact(&json, &env_path);
    assert_artifact(&json, &openapi_path);
    assert_artifact(&json, &generated_state_path);

    assert_eq!(read(&env_path), "BROKEN=1\n");
    assert!(!openapi_path.exists());
    assert_eq!(read(&generated_state_path), generated_state_before);
}

#[test]
fn project_sync_recreates_missing_generated_scaffold_and_returns_to_clean_validation() {
    let root = init_rewards_project();
    let removed_paths = [
        root.join("apps/api/src/routes/contracts.ts"),
        root.join("apps/api/src/services/tokens/points.ts"),
        root.join("apps/api/openapi.json"),
        root.join("apps/web/src/main.tsx"),
        root.join("apps/web/scripts/ui-smoke.mjs"),
    ];
    for path in &removed_paths {
        fs::remove_file(path).expect("generated file should be removable for the test");
    }
    fs::remove_dir_all(root.join("apps/web/src/generated"))
        .expect("generated state directory should be removable for the test");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "project", "sync"])
        .output()
        .expect("project sync should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "project.sync");
    assert_synced_modules(&json, &["env_example", "api", "frontend"]);
    let generated_state_path = root.join("apps/web/src/generated/stellar.ts");
    let expected_generated_state = report_path(&generated_state_path);
    assert_eq!(
        json["data"]["paths"]["generated_state"].as_str(),
        Some(expected_generated_state.as_str())
    );

    assert!(read(root.join("apps/api/src/routes/contracts.ts")).contains("registerContractRoutes"));
    assert!(read(root.join("apps/api/src/services/tokens/points.ts")).contains("metadata"));
    assert!(read(root.join("apps/api/openapi.json")).contains("/tokens/points"));
    assert!(read(root.join("apps/web/src/main.tsx")).contains("stellarState"));
    assert!(read(root.join("apps/web/scripts/ui-smoke.mjs")).contains("UI smoke passed"));
    assert!(read(root.join("apps/web/src/generated/stellar.ts")).contains("stellarState"));

    let validate = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "project", "validate"])
        .output()
        .expect("project validate should run");
    assert!(validate.status.success());
    let validate_json: Value =
        serde_json::from_slice(&validate.stdout).expect("validate stdout should be valid json");
    assert_eq!(validate_json["action"], "project.validate");
    assert_eq!(validate_json["status"], "ok");
}

fn assert_synced_modules(json: &Value, expected: &[&str]) {
    let synced = json["data"]["synced_modules"]
        .as_array()
        .expect("synced_modules should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    for module in expected {
        assert!(
            synced.contains(module),
            "expected synced_modules to include {module}; got {synced:?}"
        );
    }
}

fn assert_artifact(json: &Value, path: &Path) {
    let artifact = report_path(path);
    assert!(
        json["artifacts"]
            .as_array()
            .expect("artifacts should be an array")
            .iter()
            .any(|value| value.as_str() == Some(artifact.as_str())),
        "expected artifacts to include {artifact}"
    );
}

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).expect("file should exist")
}

fn report_path(path: &Path) -> String {
    if path.exists() {
        return fs::canonicalize(path)
            .expect("existing report path should canonicalize")
            .display()
            .to_string();
    }
    let parent = path.parent().expect("report path should have a parent");
    fs::canonicalize(parent)
        .expect("report path parent should canonicalize")
        .join(
            path.file_name()
                .expect("report path should have a file name"),
        )
        .display()
        .to_string()
}
