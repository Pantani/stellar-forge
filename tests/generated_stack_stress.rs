use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

mod support;

use support::cargo_cli;

#[test]
fn frontend_template_matrix_syncs_generated_state_and_uses_fake_browser_smoke_runner() {
    let temp = tempfile::tempdir().expect("tempdir should be created");
    let empty_path = temp.path().join(".empty-path");
    fs::create_dir_all(&empty_path).expect("empty PATH dir should be created");

    for case in [
        TemplateCase {
            name: "full",
            template: "fullstack",
            expected_state: &["\"app\"", "\"contracts\"", "\"frontend\""],
        },
        TemplateCase {
            name: "issuer",
            template: "issuer-wallet",
            expected_state: &["\"points\"", "\"issuer\"", "\"treasury\""],
        },
        TemplateCase {
            name: "merchant",
            template: "merchant-checkout",
            expected_state: &["\"points\"", "\"clawback_enabled\": true", "\"wallets\""],
        },
    ] {
        let root = init_template(temp.path(), &empty_path, case.name, case.template);
        let generated_state = root.join("apps/web/src/generated/stellar.ts");
        let browser_runner = root.join("apps/web/scripts/ui-browser-smoke.mjs");

        fs::remove_file(&generated_state).expect("generated state should be removable");
        fs::remove_file(&browser_runner).expect("browser smoke runner should be removable");

        let sync = cargo_cli()
            .current_dir(&root)
            .env("PATH", &empty_path)
            .args(["--json", "project", "sync"])
            .output()
            .expect("project sync should run");
        assert!(
            sync.status.success(),
            "project sync failed for `{}`\nstdout:\n{}\nstderr:\n{}",
            case.template,
            String::from_utf8_lossy(&sync.stdout),
            String::from_utf8_lossy(&sync.stderr)
        );
        let sync_json: Value =
            serde_json::from_slice(&sync.stdout).expect("sync stdout should be valid json");
        assert_eq!(sync_json["action"], "project.sync");
        assert_eq!(sync_json["status"], "ok");
        assert_synced_modules(&sync_json, &["env_example", "api", "frontend"]);

        let state = read(&generated_state);
        for expected in case.expected_state {
            assert!(
                state.contains(expected),
                "expected `{}` generated state to contain `{expected}`",
                case.template
            );
        }
        assert!(read(&browser_runner).contains("browser smoke passed"));

        let fake_bin = install_fake_pnpm(&root);
        let smoke_log = root.join("fake-pnpm.log");
        let smoke = cargo_cli()
            .current_dir(&root)
            .env("PATH", fake_bin)
            .env("FAKE_PNPM_LOG", &smoke_log)
            .args(["--json", "project", "smoke", "--browser"])
            .output()
            .expect("project smoke --browser should run");
        assert!(
            smoke.status.success(),
            "project smoke --browser failed for `{}`\nstdout:\n{}\nstderr:\n{}",
            case.template,
            String::from_utf8_lossy(&smoke.stdout),
            String::from_utf8_lossy(&smoke.stderr)
        );
        let smoke_json: Value =
            serde_json::from_slice(&smoke.stdout).expect("smoke stdout should be valid json");
        assert_eq!(smoke_json["action"], "project.smoke");
        assert_eq!(smoke_json["status"], "ok");
        assert_eq!(smoke_json["data"]["browser"], true);
        let expected_runner = fs::canonicalize(&browser_runner)
            .expect("browser runner should canonicalize")
            .display()
            .to_string();
        assert_eq!(
            smoke_json["data"]["runner"].as_str(),
            Some(expected_runner.as_str())
        );

        let smoke_log = read(&smoke_log);
        assert_eq!(smoke_log, "--dir apps/web smoke:browser\n");
    }
}

struct TemplateCase {
    name: &'static str,
    template: &'static str,
    expected_state: &'static [&'static str],
}

fn init_template(parent: &Path, empty_path: &Path, name: &str, template: &str) -> PathBuf {
    cargo_cli()
        .current_dir(parent)
        .env("PATH", empty_path)
        .args(["init", name, "--template", template])
        .assert()
        .success();
    parent.join(name)
}

fn install_fake_pnpm(root: &Path) -> PathBuf {
    let bin_dir = root.join(".test-bin-generated-stack");
    fs::create_dir_all(&bin_dir).expect("fake pnpm bin dir should be created");
    let pnpm_path = bin_dir.join("pnpm");
    fs::write(
        &pnpm_path,
        r#"#!/bin/sh
printf '%s\n' "$*" >> "$FAKE_PNPM_LOG"
if [ "$1" = "--dir" ] && [ "$2" = "apps/web" ] && [ "$3" = "smoke:browser" ]; then
  exit 0
fi
echo "unsupported fake pnpm invocation: $*" >&2
exit 1
"#,
    )
    .expect("fake pnpm should be written");
    #[cfg(unix)]
    fs::set_permissions(&pnpm_path, fs::Permissions::from_mode(0o755))
        .expect("fake pnpm should be executable");
    bin_dir
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

fn read(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("{} should be readable: {error}", path.display()))
}
