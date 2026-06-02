use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

mod support;

use support::init_rewards_project;

#[test]
fn project_smoke_dry_run_persists_browser_install_plan_without_package_manager() {
    let root = init_rewards_project();
    let out_path = root.join("dist/heavy/project.smoke.browser.json");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", "")
        .args([
            "--json",
            "--dry-run",
            "project",
            "smoke",
            "--install",
            "--browser",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("project smoke dry-run should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\n\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout_report: Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_smoke_report(&stdout_report, &out_path);
    assert!(
        stdout_report["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .any(|command| command.as_str() == Some("pnpm --dir apps/web install")),
        "dry-run should record package-manager install command"
    );
    assert!(
        stdout_report["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .any(|command| command
                .as_str()
                .is_some_and(|command| command.contains("pnpm --dir apps/web")
                    && command.contains("smoke:browser"))),
        "dry-run should record browser smoke command"
    );

    let out_report: Value =
        serde_json::from_str(&fs::read_to_string(&out_path).expect("out report should be written"))
            .expect("out report should parse as json");
    assert_smoke_report(&out_report, &out_path);
}

fn assert_smoke_report(report: &Value, out_path: &Path) {
    assert_eq!(report["action"], "project.smoke");
    assert_eq!(report["status"], "ok");
    assert_eq!(report["data"]["package_manager"], "pnpm");
    assert_eq!(report["data"]["install"], true);
    assert_eq!(report["data"]["browser"], true);
    assert_eq!(
        report["data"]["out"],
        out_path.to_str().expect("out path should be valid UTF-8")
    );
    assert!(
        report["artifacts"]
            .as_array()
            .expect("artifacts should be an array")
            .iter()
            .any(|artifact| artifact.as_str() == out_path.to_str()),
        "out report should be listed as an artifact"
    );
}
