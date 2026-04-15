use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn scenario_test_previews_declared_steps_in_order() {
    let root = init_rewards_project();
    append_to_manifest(
        &root,
        r#"
[scenarios.checkout]
description = "Checkout rehearsal"
network = "testnet"
identity = "alice"

[[scenarios.checkout.steps]]
action = "project.validate"

[[scenarios.checkout.steps]]
action = "contract.call"
contract = "rewards"
function = "award_points"
build_only = true
args = ["--member", "alice", "--amount", "25"]

[[scenarios.checkout.steps]]
action = "wallet.pay"
from = "treasury"
to = "alice"
asset = "points"
amount = "10"
build_only = true

[[scenarios.checkout.steps]]
action = "release.plan"

[[scenarios.checkout.assertions]]
assertion = "step"
step = 2
status = "ok"
command_contains = ["stellar contract invoke", "award_points"]

[[scenarios.checkout.assertions]]
assertion = "step"
step = 4
command_contains = ["stellar contract deploy", "stellar contract asset deploy"]
"#,
    );

    let out_path = root.join("dist/scenario.test.json");
    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "scenario",
            "test",
            "checkout",
            "--out",
            out_path.to_str().expect("out path should be UTF-8"),
        ])
        .output()
        .expect("scenario test should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "scenario.test");
    assert_eq!(json["data"]["name"], "checkout");
    assert_eq!(json["data"]["mode"], "test");
    assert_eq!(json["data"]["step_count"], 4);
    assert_eq!(json["data"]["identity"], "alice");
    assert_eq!(json["data"]["steps"][0]["action"], "project.validate");
    assert_eq!(json["data"]["steps"][1]["action"], "contract.call");
    assert_eq!(json["data"]["steps"][2]["action"], "wallet.pay");
    assert_eq!(json["data"]["steps"][3]["action"], "release.plan");
    assert_eq!(json["data"]["assertions"][0]["status"], "ok");
    assert_eq!(json["data"]["assertions"][1]["status"], "ok");
    let commands = json["commands"]
        .as_array()
        .expect("commands should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(commands.iter().any(|command| {
        command.contains("award_points") && command.contains("stellar contract invoke")
    }));
    assert!(
        commands
            .iter()
            .any(|command| command.contains("stellar contract invoke"))
    );
    assert!(
        commands
            .iter()
            .any(|command| command.contains("stellar tx") || command.contains("payment"))
    );

    let file_json: Value =
        serde_json::from_str(&fs::read_to_string(&out_path).expect("out file should exist"))
            .expect("out file should parse");
    assert_eq!(file_json["action"], "scenario.test");
}

#[test]
fn scenario_test_reports_failed_assertions_as_error_status() {
    let root = init_rewards_project();
    append_to_manifest(
        &root,
        r#"
[scenarios.assertion-failure]
description = "Intentional assertion mismatch"

[[scenarios.assertion-failure.steps]]
action = "project.sync"

[[scenarios.assertion-failure.assertions]]
assertion = "step"
step = 1
warning_contains = ["this warning does not exist"]
"#,
    );

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "scenario", "test", "assertion-failure"])
        .output()
        .expect("scenario test should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "scenario.test");
    assert_eq!(json["status"], "error");
    assert_eq!(json["data"]["assertions"][0]["status"], "error");
    assert!(
        json["data"]["assertions"][0]["issues"][0]
            .as_str()
            .expect("assertion issue should be present")
            .contains("this warning does not exist")
    );
    assert!(
        json["checks"]
            .as_array()
            .expect("checks should be an array")
            .iter()
            .any(|check| {
                check["name"]
                    .as_str()
                    .is_some_and(|name| name.contains("scenario:assertion-failure:assertion:1"))
                    && check["status"].as_str() == Some("error")
            })
    );
}

#[test]
fn scenario_run_executes_non_chain_workspace_steps() {
    let root = init_rewards_project();
    append_to_manifest(
        &root,
        r#"
[scenarios.refresh]
description = "Refresh derived workspace files"

[[scenarios.refresh.steps]]
action = "project.sync"

[[scenarios.refresh.steps]]
action = "project.validate"
"#,
    );

    let out_path = root.join("dist/scenario.run.json");
    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "scenario",
            "run",
            "refresh",
            "--out",
            out_path.to_str().expect("out path should be UTF-8"),
        ])
        .output()
        .expect("scenario run should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "scenario.run");
    assert_eq!(json["data"]["name"], "refresh");
    assert_eq!(json["data"]["mode"], "run");
    assert_eq!(json["data"]["step_count"], 2);
    assert!(root.join(".env.example").exists());

    let file_json: Value =
        serde_json::from_str(&fs::read_to_string(&out_path).expect("out file should exist"))
            .expect("out file should parse");
    assert_eq!(file_json["action"], "scenario.run");
}

fn append_to_manifest(root: &std::path::Path, contents: &str) {
    let manifest_path = root.join("stellarforge.toml");
    let mut manifest = fs::read_to_string(&manifest_path).expect("manifest should be readable");
    manifest.push_str(contents);
    fs::write(manifest_path, manifest).expect("manifest should be writable");
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
