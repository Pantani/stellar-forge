use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn wallet_batch_pay_reports_summary_and_preview() {
    let root = init_rewards_project();
    let out_path = root.join("dist/payouts.pay.json");
    fs::write(
        root.join("payments.json"),
        r#"[
  { "to": "bob", "amount": "10" },
  { "to": "alice", "amount": "5", "asset": "points" }
]
"#,
    )
    .expect("batch payment file should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "batch-pay",
            "--from",
            "alice",
            "--asset",
            "XLM",
            "--file",
            "payments.json",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json = assert_report_written(&output.stdout, "wallet.batch-pay", &out_path);
    assert_eq!(json["data"]["summary"]["kind"], "batch-pay");
    assert_eq!(json["data"]["summary"]["count"], 2);
    assert_eq!(json["data"]["summary"]["default_asset"], "XLM");
    assert_eq!(json["data"]["summary"]["explicit_assets"], 1);
    assert_eq!(json["data"]["summary"]["inferred_assets"], 1);
    assert_eq!(json["data"]["summary"]["unique_destinations"], 2);
    assert_eq!(json["data"]["summary"]["unique_assets"], 2);

    let preview = json["data"]["preview"]
        .as_array()
        .expect("preview should be an array");
    assert_eq!(preview.len(), 2);
    assert_eq!(preview[0]["asset_source"], "default");
    assert_eq!(preview[0]["asset"], "XLM");
    assert_eq!(preview[1]["asset_source"], "entry");
    assert_eq!(preview[1]["asset"], "points");
}

#[test]
fn token_airdrop_reports_summary_and_preview() {
    let root = init_rewards_project();
    let out_path = root.join("dist/airdrop.json");
    fs::write(root.join("airdrop.csv"), "to,amount\nalice,10\nbob,20\n")
        .expect("airdrop csv should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "token",
            "airdrop",
            "points",
            "--from",
            "treasury",
            "--file",
            "airdrop.csv",
            "--format",
            "csv",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json = assert_report_written(&output.stdout, "token.airdrop", &out_path);
    assert_eq!(json["data"]["token"], "points");
    assert_eq!(json["data"]["from"], "treasury");
    assert_eq!(json["data"]["summary"]["kind"], "batch-pay");
    assert_eq!(json["data"]["summary"]["count"], 2);
    assert_eq!(json["data"]["summary"]["default_asset"], "points");
    let preview = json["data"]["preview"]
        .as_array()
        .expect("preview should be an array");
    assert_eq!(preview.len(), 2);
    assert_eq!(preview[0]["asset_source"], "default");
    assert_eq!(preview[1]["asset_source"], "default");
}

#[test]
fn wallet_batch_validate_preview_and_summary_surface_new_actions() {
    let root = init_rewards_project();
    fs::write(
        root.join("payments.json"),
        r#"[{ "to": "bob", "amount": "10", "asset": "points" }]"#,
    )
    .expect("batch payment file should be written");

    let validate_out = root.join("dist/payouts.validate.json");
    let validate = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "wallet",
            "batch-validate",
            "--from",
            "alice",
            "--file",
            "payments.json",
            "--out",
            validate_out
                .to_str()
                .expect("validate out path should be valid UTF-8"),
        ])
        .output()
        .expect("validate command should run");
    assert!(validate.status.success());
    let validate_json =
        assert_report_written(&validate.stdout, "wallet.batch-validate", &validate_out);
    assert_eq!(validate_json["data"]["summary"]["kind"], "batch-validate");
    assert_eq!(validate_json["data"]["preview"][0]["asset"], "points");

    let preview_out = root.join("dist/payouts.preview.json");
    let preview = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "wallet",
            "batch-preview",
            "--from",
            "alice",
            "--file",
            "payments.json",
            "--out",
            preview_out
                .to_str()
                .expect("preview out path should be valid UTF-8"),
        ])
        .output()
        .expect("preview command should run");
    assert!(preview.status.success());
    let preview_json = assert_report_written(&preview.stdout, "wallet.batch-preview", &preview_out);
    assert_eq!(preview_json["data"]["summary"]["kind"], "batch-preview");
    assert_eq!(preview_json["data"]["preview"][0]["asset"], "points");

    let summary_out = root.join("dist/payouts.summary.json");
    let summary = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "wallet",
            "batch-summary",
            "--from",
            "alice",
            "--file",
            "payments.json",
            "--out",
            summary_out
                .to_str()
                .expect("summary out path should be valid UTF-8"),
        ])
        .output()
        .expect("summary command should run");
    assert!(summary.status.success());
    let summary_json = assert_report_written(&summary.stdout, "wallet.batch-summary", &summary_out);
    assert_eq!(summary_json["data"]["summary"]["kind"], "batch-summary");
    assert!(summary_json["data"]["preview"].is_null());
}

#[test]
fn wallet_batch_report_surfaces_preview_and_summary() {
    let root = init_rewards_project();
    let out_path = root.join("dist/payouts.report.json");
    fs::write(
        root.join("payments.json"),
        r#"[
  { "to": "bob", "amount": "10" },
  { "to": "alice", "amount": "5", "asset": "points" },
  { "to": "carol", "amount": "7" }
]
"#,
    )
    .expect("batch payment file should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "batch-report",
            "--from",
            "alice",
            "--asset",
            "XLM",
            "--file",
            "payments.json",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json = assert_report_written(&output.stdout, "wallet.batch-report", &out_path);
    assert_eq!(json["data"]["summary"]["kind"], "batch-report");
    assert_eq!(json["data"]["summary"]["count"], 3);
    assert_eq!(json["data"]["summary"]["default_asset"], "XLM");
    let preview = json["data"]["preview"]
        .as_array()
        .expect("preview should be an array");
    assert_eq!(preview.len(), 3);
    assert_eq!(preview[0]["asset"], "XLM");
    assert_eq!(preview[1]["asset"], "points");
}

#[test]
fn wallet_batch_reconcile_matches_reported_preview_rows() {
    let root = init_rewards_project();
    let out_path = root.join("dist/payouts.reconcile.json");
    fs::write(
        root.join("payments.json"),
        r#"[
  { "to": "bob", "amount": "10" },
  { "to": "alice", "amount": "5", "asset": "points" }
]
"#,
    )
    .expect("batch payment file should be written");
    fs::write(
        root.join("payments.report.json"),
        r#"{
  "action": "wallet.batch-report",
  "data": {
    "preview": [
      { "index": 1, "to": "bob", "amount": "10", "asset": "XLM", "asset_source": "default" },
      { "index": 2, "to": "alice", "amount": "5", "asset": "points", "asset_source": "entry" }
    ]
  }
}
"#,
    )
    .expect("batch report should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "wallet",
            "batch-reconcile",
            "--from",
            "alice",
            "--asset",
            "XLM",
            "--file",
            "payments.json",
            "--report",
            "payments.report.json",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json = assert_report_written(&output.stdout, "wallet.batch-reconcile", &out_path);
    assert_eq!(json["data"]["reconcile"]["matched_indices"][0], 1);
    assert_eq!(json["data"]["reconcile"]["matched_indices"][1], 2);
    assert!(
        json["data"]["reconcile"]["missing_entries"]
            .as_array()
            .expect("missing entries should be an array")
            .is_empty()
    );
    assert!(
        json["data"]["reconcile"]["unexpected_entries"]
            .as_array()
            .expect("unexpected entries should be an array")
            .is_empty()
    );
}

#[test]
fn wallet_batch_resume_writes_report_to_out_path() {
    let root = init_rewards_project();
    let out_path = root.join("dist/payouts.resume.json");
    fs::write(
        root.join("payments.json"),
        r#"[
  { "to": "alice", "amount": "10" },
  { "to": "treasury", "amount": "5" },
  { "to": "issuer", "amount": "7" }
]
"#,
    )
    .expect("batch file should be written");
    fs::write(
        root.join("payments.report.json"),
        r#"{
  "action": "wallet.batch-pay",
  "data": {
    "payments": [
      { "index": 1, "to": "alice", "amount": "10", "asset": "XLM", "asset_source": "default" }
    ]
  }
}
"#,
    )
    .expect("batch report should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "batch-resume",
            "--from",
            "alice",
            "--asset",
            "XLM",
            "--file",
            "payments.json",
            "--report",
            "payments.report.json",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json = assert_report_written(&output.stdout, "wallet.batch-resume", &out_path);
    assert_eq!(json["data"]["count"], 2);
    assert_eq!(json["data"]["resume"]["completed_from_report"][0], 1);
    let selected = json["data"]["resume"]["selected"]
        .as_array()
        .expect("selected should be an array")
        .iter()
        .filter_map(Value::as_u64)
        .collect::<Vec<_>>();
    assert_eq!(selected, vec![2, 3]);
}

#[test]
fn token_airdrop_report_reuses_batch_report_shape() {
    let root = init_rewards_project();
    let out_path = root.join("dist/airdrop.report.json");
    fs::write(root.join("airdrop.csv"), "to,amount\nalice,10\nbob,20\n")
        .expect("airdrop csv should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "token",
            "airdrop-report",
            "points",
            "--from",
            "treasury",
            "--file",
            "airdrop.csv",
            "--format",
            "csv",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json = assert_report_written(&output.stdout, "token.airdrop-report", &out_path);
    assert_eq!(json["data"]["token"], "points");
    assert_eq!(json["data"]["from"], "treasury");
    assert_eq!(json["data"]["summary"]["kind"], "batch-report");
    assert_eq!(json["data"]["summary"]["count"], 2);
    assert_eq!(json["data"]["summary"]["default_asset"], "points");
    let preview = json["data"]["preview"]
        .as_array()
        .expect("preview should be an array");
    assert_eq!(preview.len(), 2);
    assert_eq!(preview[0]["asset"], "points");
    assert_eq!(preview[1]["asset"], "points");
}

#[test]
fn token_airdrop_validate_rewrites_action_and_preserves_token_context() {
    let root = init_rewards_project();
    let out_path = root.join("dist/airdrop.validate.json");
    fs::write(root.join("airdrop.csv"), "to,amount\nalice,10\n")
        .expect("airdrop csv should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "token",
            "airdrop-validate",
            "points",
            "--file",
            "airdrop.csv",
            "--format",
            "csv",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json = assert_report_written(&output.stdout, "token.airdrop-validate", &out_path);
    assert_eq!(json["data"]["token"], "points");
    assert_eq!(json["data"]["from"], "treasury");
    assert_eq!(json["data"]["summary"]["kind"], "batch-validate");
}

#[test]
fn token_airdrop_preview_summary_reconcile_and_resume_write_reports_to_out_paths() {
    let root = init_rewards_project();
    fs::write(
        root.join("airdrop.csv"),
        "to,amount\nalice,10\ntreasury,20\n",
    )
    .expect("airdrop csv should be written");
    fs::write(
        root.join("airdrop.report.json"),
        r#"{
  "action": "token.airdrop",
  "data": {
    "payments": [
      { "index": 1, "to": "alice", "amount": "10", "asset": "points", "asset_source": "default" }
    ]
  }
}
"#,
    )
    .expect("airdrop report should be written");

    let preview_out = root.join("dist/airdrop.preview.json");
    let preview = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "token",
            "airdrop-preview",
            "points",
            "--from",
            "treasury",
            "--file",
            "airdrop.csv",
            "--format",
            "csv",
            "--out",
            preview_out
                .to_str()
                .expect("preview out path should be valid UTF-8"),
        ])
        .output()
        .expect("preview command should run");
    assert!(preview.status.success());
    let preview_json =
        assert_report_written(&preview.stdout, "token.airdrop-preview", &preview_out);
    assert_eq!(preview_json["data"]["summary"]["kind"], "batch-preview");

    let summary_out = root.join("dist/airdrop.summary.json");
    let summary = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "token",
            "airdrop-summary",
            "points",
            "--file",
            "airdrop.csv",
            "--format",
            "csv",
            "--out",
            summary_out
                .to_str()
                .expect("summary out path should be valid UTF-8"),
        ])
        .output()
        .expect("summary command should run");
    assert!(summary.status.success());
    let summary_json =
        assert_report_written(&summary.stdout, "token.airdrop-summary", &summary_out);
    assert_eq!(summary_json["data"]["summary"]["kind"], "batch-summary");
    assert!(summary_json["data"]["preview"].is_null());

    let reconcile_out = root.join("dist/airdrop.reconcile.json");
    let reconcile = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "token",
            "airdrop-reconcile",
            "points",
            "--from",
            "treasury",
            "--file",
            "airdrop.csv",
            "--format",
            "csv",
            "--report",
            "airdrop.report.json",
            "--out",
            reconcile_out
                .to_str()
                .expect("reconcile out path should be valid UTF-8"),
        ])
        .output()
        .expect("reconcile command should run");
    assert!(reconcile.status.success());
    let reconcile_json =
        assert_report_written(&reconcile.stdout, "token.airdrop-reconcile", &reconcile_out);
    assert_eq!(reconcile_json["data"]["token"], "points");
    assert_eq!(reconcile_json["data"]["reconcile"]["matched_indices"][0], 1);

    let resume_out = root.join("dist/airdrop.resume.json");
    let resume = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "token",
            "airdrop-resume",
            "points",
            "--from",
            "treasury",
            "--file",
            "airdrop.csv",
            "--format",
            "csv",
            "--report",
            "airdrop.report.json",
            "--out",
            resume_out
                .to_str()
                .expect("resume out path should be valid UTF-8"),
        ])
        .output()
        .expect("resume command should run");
    assert!(resume.status.success());
    let resume_json = assert_report_written(&resume.stdout, "token.airdrop-resume", &resume_out);
    assert_eq!(resume_json["data"]["token"], "points");
    assert_eq!(resume_json["data"]["count"], 1);
}

#[test]
fn wallet_batch_pay_rejects_missing_asset_without_default() {
    let root = init_rewards_project();
    fs::write(
        root.join("payments.json"),
        r#"[{ "to": "bob", "amount": "10" }]"#,
    )
    .expect("batch payment file should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "wallet",
            "batch-pay",
            "--from",
            "alice",
            "--file",
            "payments.json",
        ])
        .output()
        .expect("command should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing `asset`"));
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

fn assert_report_written(stdout: &[u8], action: &str, out_path: &Path) -> Value {
    let json: Value = serde_json::from_slice(stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], action);
    assert_eq!(json["data"]["out"], out_path.display().to_string());
    let out_json: Value = serde_json::from_str(
        &fs::read_to_string(out_path).expect("report output should be written"),
    )
    .expect("report output should parse");
    assert_eq!(out_json["action"], action);
    assert_eq!(out_json["data"]["out"], out_path.display().to_string());
    json
}
