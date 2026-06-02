use serde_json::Value;
use std::fs;

mod support;

use support::init_minimal_contract_project;

#[test]
fn project_info_out_persists_same_artifact_contract_as_stdout() {
    let root = init_minimal_contract_project();
    let out_path = root.join("dist/project.info.json");
    let out = out_path.to_str().expect("out path should be valid UTF-8");

    let output = support::cargo_cli()
        .current_dir(&root)
        .args(["--json", "project", "info", "--out", out])
        .output()
        .expect("project info should run");

    assert!(output.status.success());
    let stdout: Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");
    assert_eq!(stdout["action"], "project.info");
    assert_eq!(stdout["status"], "ok");
    assert_eq!(stdout["data"]["out"], out_path.display().to_string());
    assert_report_artifact(&stdout, &out_path.display().to_string());

    let saved: Value = serde_json::from_str(
        &fs::read_to_string(&out_path).expect("persisted report should be readable"),
    )
    .expect("persisted report should be valid JSON");
    assert_eq!(saved["action"], stdout["action"]);
    assert_eq!(saved["status"], stdout["status"]);
    assert_eq!(saved["data"]["out"], stdout["data"]["out"]);
    assert_eq!(saved["artifacts"], stdout["artifacts"]);
    assert_report_artifact(&saved, &out_path.display().to_string());
}

fn assert_report_artifact(report: &Value, expected: &str) {
    let artifacts = report["artifacts"]
        .as_array()
        .expect("artifacts should be an array");
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact.as_str() == Some(expected)),
        "expected artifacts to include {expected}; got {artifacts:?}"
    );
}
