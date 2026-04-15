use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

#[test]
fn init_writes_expected_project_scaffold() {
    let temp = tempdir().expect("tempdir should be created");
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(temp.path())
        .args(["init", "demo", "--template", "fullstack"])
        .assert()
        .success();

    let root = temp.path().join("demo");
    let readme = read(root.join("README.md"));
    let reseed_script = read(root.join("scripts/reseed.mjs"));
    let release_script = read(root.join("scripts/release.mjs"));
    let doctor_script = read(root.join("scripts/doctor.mjs"));
    let worker = read(root.join("workers/events/ingest-events.mjs"));
    let web_package = read(root.join("apps/web/package.json"));
    let web_smoke_runner = read(root.join("apps/web/scripts/ui-smoke.mjs"));
    let web_browser_smoke_runner = read(root.join("apps/web/scripts/ui-browser-smoke.mjs"));
    assert!(root.join("stellarforge.toml").exists());
    assert!(root.join("stellarforge.lock.json").exists());
    assert!(root.join("apps/api/package.json").exists());
    assert!(root.join("apps/web/src/main.tsx").exists());
    assert!(root.join("apps/web/scripts/ui-smoke.mjs").exists());
    assert!(root.join("apps/web/scripts/ui-browser-smoke.mjs").exists());
    assert!(root.join("contracts/app/Cargo.toml").exists());
    assert!(root.join("contracts/app/rust-toolchain.toml").exists());
    assert!(root.join("scripts/reseed.mjs").exists());
    assert!(root.join("scripts/release.mjs").exists());
    assert!(root.join("scripts/doctor.mjs").exists());
    assert!(root.join("workers/events/ingest-events.mjs").exists());
    assert!(readme.contains("node scripts/doctor.mjs"));
    assert!(readme.contains("node scripts/release.mjs --plan"));
    assert!(readme.contains("node workers/events/ingest-events.mjs <resource> --status --once"));
    assert!(readme.contains("stellar forge project smoke"));
    assert!(readme.contains("stellar forge project smoke --browser"));
    assert!(readme.contains("pnpm --dir apps/web smoke:browser:build"));
    assert!(readme.contains("pnpm --dir apps/web smoke:browser:install"));
    assert!(readme.contains("pnpm --dir apps/web smoke:browser:run"));
    assert!(readme.contains("STELLAR_FORGE_BROWSER_SMOKE_PORT"));
    assert!(reseed_script.contains("dev', 'reseed"));
    assert!(release_script.contains("release', 'deploy"));
    assert!(release_script.contains("release', 'plan"));
    assert!(doctor_script.contains("['doctor'"));
    assert!(worker.contains("'events'"));
    assert!(worker.contains("'backfill'"));
    assert!(worker.contains("'status'"));
    assert!(worker.contains("'export'"));
    assert!(worker.contains("STELLAR_EVENTS_TOPICS"));
    assert!(web_package.contains("\"smoke:ui\""));
    assert!(web_package.contains("\"smoke:browser\""));
    assert!(web_package.contains("\"smoke:browser:build\""));
    assert!(web_package.contains("\"smoke:browser:install\""));
    assert!(web_package.contains("\"smoke:browser:run\""));
    assert!(web_package.contains("\"preview\""));
    assert!(web_smoke_runner.contains("vite build"));
    assert!(web_smoke_runner.contains("vite preview"));
    assert!(web_smoke_runner.contains("UI smoke passed"));
    assert!(web_browser_smoke_runner.contains("@playwright/test@1.59.1"));
    assert!(web_browser_smoke_runner.contains("Chromium revision"));
    assert!(web_browser_smoke_runner.contains("already present; skipping install"));
    assert!(web_browser_smoke_runner.contains("unknown browser smoke mode"));
    assert!(web_browser_smoke_runner.contains("STELLAR_FORGE_BROWSER_SMOKE_PORT"));
    assert!(web_browser_smoke_runner.contains("browser smoke passed"));
    if stellar_available() {
        assert!(root.join("Cargo.toml").exists());
        assert!(root.join("contracts/app/Makefile").exists());
        assert!(root.join("contracts/app/src/test.rs").exists());
    }
}

#[test]
fn init_install_rejects_unsupported_package_manager() {
    let temp = tempdir().expect("tempdir should be created");
    let bin = std::env::var_os("CARGO_BIN_EXE_stellar-forge")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| assert_cmd::cargo::cargo_bin("stellar-forge"));

    let output = Command::new(bin)
        .current_dir(temp.path())
        .args([
            "--json",
            "init",
            "demo",
            "--template",
            "fullstack",
            "--install",
            "--package-manager",
            "evil-pm",
        ])
        .output()
        .expect("command should run");

    assert!(!output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "init");
    assert_eq!(json["status"], "error");
    assert!(
        json["message"]
            .as_str()
            .expect("message should be present")
            .contains("unsupported `project.package_manager` value `evil-pm`")
    );
}

#[test]
fn init_with_install_and_git_runs_package_manager_and_git_init() {
    let temp = tempdir().expect("tempdir should be created");
    let log_path = temp.path().join("init-tools.log");
    let fake_bin = install_recording_fake_commands(temp.path(), &["pnpm", "git"]);

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(temp.path())
        .env("PATH", test_path(&fake_bin))
        .env("FAKE_LOG_PATH", &log_path)
        .args([
            "--json",
            "init",
            "demo",
            "--template",
            "fullstack",
            "--install",
            "--git",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "init");

    let invocations = read_invocation_log(&log_path);
    assert!(
        invocations
            .iter()
            .any(|line| line.starts_with("git|") && line.ends_with("|init"))
    );
    assert!(
        invocations
            .iter()
            .any(|line| line.starts_with("pnpm|") && line.contains("/apps/api|install"))
    );
    assert!(
        invocations
            .iter()
            .any(|line| line.starts_with("pnpm|") && line.contains("/apps/web|install"))
    );
}

#[test]
fn rewards_template_writes_domain_specific_contract_files() {
    let root = init_rewards_project();
    let manifest = read(root.join("stellarforge.toml"));
    let api_package = read(root.join("apps/api/package.json"));
    let lib_rs = read(root.join("contracts/rewards/src/lib.rs"));
    let test_rs = read(root.join("contracts/rewards/src/test.rs"));
    let toolchain = read(root.join("contracts/rewards/rust-toolchain.toml"));
    let api_server = read(root.join("apps/api/src/server.ts"));
    let api_health = read(root.join("apps/api/src/routes/health.ts"));
    let api_events = read(root.join("apps/api/src/routes/events.ts"));
    let api_store = read(root.join("apps/api/src/lib/events-store.ts"));
    let web_main = read(root.join("apps/web/src/main.tsx"));
    let web_state = read(root.join("apps/web/src/generated/stellar.ts"));
    let web_smoke_runner = read(root.join("apps/web/scripts/ui-smoke.mjs"));
    let web_browser_smoke_runner = read(root.join("apps/web/scripts/ui-browser-smoke.mjs"));

    assert!(manifest.contains("[contracts.rewards.init]"));
    assert!(manifest.contains("token = \"@token:points:sac\""));
    assert!(api_package.contains("\"events:ingest\""));
    assert!(api_package.contains("\"better-sqlite3\""));
    assert!(lib_rs.contains("pub fn award_points"));
    assert!(lib_rs.contains("pub fn spend_points"));
    assert!(test_rs.contains("rewards_flow_tracks_points"));
    assert!(toolchain.contains("wasm32v1-none"));
    assert!(api_server.contains("registerContractRoutes(app);"));
    assert!(api_server.contains("registerEventRoutes(app);"));
    assert!(api_server.contains("registerHealthRoutes(app);"));
    assert!(api_health.contains("manifest.project.version"));
    assert!(api_events.contains("/events/status"));
    assert!(api_events.contains("/events/cursors"));
    assert!(api_events.contains("getEventStatus"));
    assert!(api_events.contains("tracked_resources"));
    assert!(api_events.contains("retention_warning"));
    assert!(api_store.contains("insert or ignore into events"));
    assert!(api_store.contains("resolveEventWorkerConfig"));
    assert!(api_store.contains("syncCursorSnapshot"));
    assert!(web_main.contains("stellarState.project.name"));
    assert!(web_main.contains("stellarState.events"));
    assert!(web_main.contains("stellarState.deployment.contracts"));
    assert!(web_main.contains("stellarState.deployment.tokens"));
    assert!(web_main.contains("stellarState.network?.rpc_url"));
    assert!(web_main.contains("stellarState.wallets"));
    assert!(web_main.contains("stellarState.api?.enabled"));
    assert!(web_main.contains("stellar forge release deploy"));
    assert!(web_smoke_runner.contains("expectedMarkers"));
    assert!(web_smoke_runner.contains("project name"));
    assert!(web_browser_smoke_runner.contains("pageerror"));
    assert!(web_browser_smoke_runner.contains("generated frontend renders expected sections"));
    assert!(web_state.contains("\"environment\": \"testnet\""));
    assert!(web_state.contains("\"network\": {"));
    assert!(web_state.contains("\"api\": {"));
    assert!(web_state.contains("\"wallets\": {"));
    assert!(web_state.contains("\"events\": {"));
    assert!(web_state.contains("\"backend\": \"rpc-poller\""));
    assert!(web_state.contains("\"rewards\""));
}

#[test]
fn project_adopt_scaffold_imports_contracts_packages_and_environments() {
    let root = init_scaffold_like_project();
    let out_path = root.join("dist/project.adopt.json");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "project",
            "adopt",
            "scaffold",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "project.adopt.scaffold");
    assert_eq!(json["status"], "ok");
    assert_eq!(json["data"]["contracts"][0], "hello");
    assert_eq!(json["data"]["api"], false);
    assert_eq!(json["data"]["frontend"], false);
    assert_eq!(json["data"]["scaffold_frontend_detected"], true);
    let bindings = json["data"]["bindings"]["hello"]
        .as_array()
        .expect("bindings should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert_eq!(bindings, vec!["python", "typescript"]);
    let environments = json["data"]["environments"]
        .as_array()
        .expect("environments should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert_eq!(environments, vec!["local", "testnet"]);
    assert_eq!(json["data"]["deployments"]["local"], 1);
    assert_eq!(json["data"]["deployments"]["testnet"], 1);
    assert_eq!(
        json["data"]["out"],
        out_path.to_str().expect("out path should be valid UTF-8")
    );
    assert!(
        json["warnings"]
            .as_array()
            .expect("warnings should be an array")
            .iter()
            .any(|warning| warning
                .as_str()
                .is_some_and(|warning| warning.contains("project root")))
    );

    let manifest: toml::Value =
        toml::from_str(&read(root.join("stellarforge.toml"))).expect("manifest should parse");
    assert_eq!(manifest["project"]["package_manager"].as_str(), Some("npm"));
    assert_eq!(manifest["defaults"]["network"].as_str(), Some("testnet"));
    assert_eq!(
        manifest["contracts"]["hello"]["path"].as_str(),
        Some("contracts/hello")
    );
    assert_eq!(
        manifest["contracts"]["hello"]["alias"].as_str(),
        Some("hello-test")
    );
    let deploy_on = manifest["contracts"]["hello"]["deploy_on"]
        .as_array()
        .expect("deploy_on should be an array")
        .iter()
        .filter_map(toml::Value::as_str)
        .collect::<Vec<_>>();
    assert_eq!(deploy_on, vec!["local", "testnet"]);
    let manifest_bindings = manifest["contracts"]["hello"]["bindings"]
        .as_array()
        .expect("manifest bindings should be an array")
        .iter()
        .filter_map(toml::Value::as_str)
        .collect::<Vec<_>>();
    assert_eq!(manifest_bindings, vec!["python", "typescript"]);
    assert_eq!(
        manifest["networks"]["testnet"]["rpc_url"].as_str(),
        Some("https://rpc.example")
    );
    assert_eq!(
        manifest["networks"]["local"]["allow_http"].as_bool(),
        Some(true)
    );
    assert!(manifest.get("api").is_none());
    assert!(manifest.get("frontend").is_none());

    let lockfile: Value = serde_json::from_str(&read(root.join("stellarforge.lock.json")))
        .expect("lockfile should parse");
    assert_eq!(
        lockfile["environments"]["testnet"]["contracts"]["hello"]["contract_id"],
        "CHELLO123"
    );
    assert_eq!(
        lockfile["environments"]["testnet"]["contracts"]["hello"]["alias"],
        "hello-test"
    );
    assert_eq!(
        lockfile["environments"]["local"]["contracts"]["hello"]["contract_id"],
        "CLOCAL123"
    );
    let out_json: Value =
        serde_json::from_str(&read(&out_path)).expect("project adopt out should parse");
    assert_eq!(out_json["action"], "project.adopt.scaffold");
}

#[test]
fn project_info_reports_scaffold_compatibility_and_deployments() {
    let root = init_scaffold_like_project();
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["project", "adopt", "scaffold"])
        .assert()
        .success();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "project", "info"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "project.info");
    assert_eq!(json["network"], "testnet");
    assert!(
        json["message"]
            .as_str()
            .expect("message should be present")
            .contains("1 contracts")
    );
    assert_eq!(
        json["data"]["deployment"]["testnet"]["contracts"]["hello"]["contract_id"],
        "CHELLO123"
    );
    assert_eq!(json["data"]["compatibility"]["detected"], true);
    assert_eq!(json["data"]["compatibility"]["root_frontend"], true);
    assert_eq!(json["data"]["compatibility"]["managed_frontend"], false);
    let traces = json["data"]["compatibility"]["traces"]
        .as_array()
        .expect("traces should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(traces.contains(&"contracts"));
    assert!(traces.contains(&"packages"));
    assert!(traces.contains(&"environments"));
    assert!(traces.contains(&"root-frontend"));
    let compatibility_envs = json["data"]["compatibility"]["lockfile_environments"]
        .as_array()
        .expect("lockfile environments should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert_eq!(compatibility_envs, vec!["local", "testnet"]);
}

#[test]
fn project_info_and_validate_write_reports_to_out_paths() {
    let root = init_rewards_project();
    let info_out = root.join("dist/project.info.json");
    let validate_out = root.join("dist/project.validate.json");

    let info = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "project",
            "info",
            "--out",
            info_out.to_str().expect("info out should be valid UTF-8"),
        ])
        .output()
        .expect("project info should run");
    assert!(info.status.success());
    let info_json: Value =
        serde_json::from_slice(&info.stdout).expect("info stdout should be valid json");
    assert_eq!(info_json["action"], "project.info");
    assert_eq!(
        info_json["data"]["out"],
        info_out.to_str().expect("info out should be valid UTF-8")
    );
    let info_file: Value = serde_json::from_str(&read(&info_out)).expect("info out should parse");
    assert_eq!(info_file["action"], "project.info");

    let validate = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "project",
            "validate",
            "--out",
            validate_out
                .to_str()
                .expect("validate out should be valid UTF-8"),
        ])
        .output()
        .expect("project validate should run");
    assert!(validate.status.success());
    let validate_json: Value =
        serde_json::from_slice(&validate.stdout).expect("validate stdout should be valid json");
    assert_eq!(validate_json["action"], "project.validate");
    assert_eq!(
        validate_json["data"]["out"],
        validate_out
            .to_str()
            .expect("validate out should be valid UTF-8")
    );
    let validate_file: Value =
        serde_json::from_str(&read(&validate_out)).expect("validate out should parse");
    assert_eq!(validate_file["action"], "project.validate");
}

#[test]
fn project_validate_rejects_contract_token_without_matching_contract() {
    let root = init_contract_token_project();
    let manifest = read(root.join("stellarforge.toml"));
    let without_contract = manifest
        .split("[contracts.credits]")
        .next()
        .expect("manifest should contain contract section")
        .to_string();
    fs::write(root.join("stellarforge.toml"), without_contract)
        .expect("manifest should be updated");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["project", "validate"])
        .output()
        .expect("command should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be valid utf8");
    assert!(stderr.contains(
        "token `credits` is declared as a contract token but no matching contract `credits` exists in the manifest"
    ));
}

#[test]
fn init_rejects_unsafe_project_name() {
    let temp = tempdir().expect("tempdir should be created");
    let parent = temp.path().parent().expect("tempdir should have a parent");
    let escape_name = format!(
        "{}-escape",
        temp.path()
            .file_name()
            .expect("tempdir should have a final path segment")
            .to_string_lossy()
    );
    let unsafe_name = format!("../{escape_name}");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(temp.path())
        .args(["init", &unsafe_name])
        .output()
        .expect("command should run");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be valid utf8");
    assert!(stderr.contains("must be a single filesystem-safe name"));
    assert!(!parent.join(&escape_name).exists());
}

#[test]
fn wallet_create_rejects_unsafe_wallet_name() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "--dry-run", "wallet", "create", "../ops"])
        .output()
        .expect("command should run");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.create");
    assert_eq!(json["status"], "error");
    assert_eq!(json["data"]["error_code"], "input");
    assert!(
        json["message"]
            .as_str()
            .expect("message should be present")
            .contains("wallet name `../ops` must be a single filesystem-safe name")
    );
}

#[test]
fn token_create_rejects_unsafe_token_name() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "token",
            "create",
            "../points",
            "--issuer",
            "issuer",
            "--distribution",
            "treasury",
        ])
        .output()
        .expect("command should run");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "token.create");
    assert_eq!(json["status"], "error");
    assert_eq!(json["data"]["error_code"], "input");
    assert!(
        json["message"]
            .as_str()
            .expect("message should be present")
            .contains("token name `../points` must be a single filesystem-safe name")
    );
}

#[test]
fn project_validate_rejects_contract_paths_outside_project_root() {
    let root = init_rewards_project();
    let manifest = read(root.join("stellarforge.toml"));
    let escaped = manifest.replacen("path = \"contracts/rewards\"", "path = \"../escape\"", 1);
    fs::write(root.join("stellarforge.toml"), escaped).expect("manifest should be updated");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "project", "validate"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "project.validate");
    assert_eq!(json["status"], "error");
    assert_eq!(find_check(&json, "manifest")["status"], "error");
    assert!(
        find_check(&json, "manifest")["detail"]
            .as_str()
            .expect("manifest detail should be present")
            .contains("must stay inside the project root")
    );
}

#[test]
fn parse_errors_are_json_when_requested() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "token", "create"])
        .output()
        .expect("command should run");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "cli.parse");
    assert_eq!(json["status"], "error");
    assert_eq!(json["data"]["error_code"], "input");
    assert_eq!(json["data"]["exit_code"], 2);
    assert_eq!(json["data"]["kind"], "MissingRequiredArgument");
    assert!(
        json["message"]
            .as_str()
            .expect("message should be present")
            .contains("required arguments were not provided")
    );
    assert!(
        json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| value.as_str() == Some("stellar-forge --help"))
    );
}

#[test]
fn project_validate_json_reports_clean_project_summary() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "project", "validate"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "project.validate");
    assert_eq!(json["status"], "ok");
    assert_eq!(find_check(&json, "manifest")["status"], "ok");
    assert_eq!(find_check(&json, "env:example:consistency")["status"], "ok");
    assert_eq!(json["data"]["summary"]["error"], 0);
    assert!(
        json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| value.as_str() == Some("stellar forge doctor project"))
    );
}

#[test]
fn project_validate_json_reports_generated_file_drift_and_sync_hint() {
    let root = init_rewards_project();
    fs::write(root.join(".env.example"), "BROKEN=1\n").expect("env example should be overwritten");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "project", "validate"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "project.validate");
    assert_eq!(json["status"], "error");
    assert_eq!(
        find_check(&json, "env:example:consistency")["status"],
        "error"
    );
    assert!(
        find_check(&json, "env:example:consistency")["detail"]
            .as_str()
            .expect("consistency detail should be present")
            .contains("project sync")
    );
    assert!(
        json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| value.as_str() == Some("stellar forge project sync"))
    );
}

#[test]
fn project_validate_json_reports_event_scaffold_gaps() {
    let root = init_rewards_project();
    fs::remove_file(root.join("apps/api/src/routes/events.ts"))
        .expect("events route should be removable for the test");
    fs::write(root.join("apps/api/.env"), "STELLAR_EVENTS_TYPE=oops\n")
        .expect("api env should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "project", "validate"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "project.validate");
    assert_eq!(json["status"], "error");
    assert_eq!(find_check(&json, "api:events-route")["status"], "error");
    assert_eq!(find_check(&json, "events:config")["status"], "error");
    assert!(
        json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| value.as_str() == Some("stellar forge api events init"))
    );
}

#[test]
fn project_add_api_generates_scaffold_and_reports_project_action() {
    let root = init_minimal_contract_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "project", "add", "api"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "project.add.api");
    assert_eq!(json["status"], "ok");
    assert_eq!(json["data"]["services"]["contracts"], 1);
    assert_eq!(json["data"]["services"]["tokens"], 0);
    assert!(
        json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| value.as_str() == Some("pnpm --dir apps/api dev"))
    );

    let manifest = read(root.join("stellarforge.toml"));
    let contract_service = read(root.join("apps/api/src/services/contracts/app.ts"));
    let openapi = read(root.join("apps/api/openapi.json"));

    assert!(manifest.contains("[api]"));
    assert!(manifest.contains("enabled = true"));
    assert!(manifest.contains("openapi = true"));
    assert!(root.join("apps/api/src/server.ts").exists());
    assert!(contract_service.contains("preview_endpoint: '/contracts/app/call/:fn'"));
    assert!(contract_service.contains("typescript_binding: \"packages/app-ts\""));
    assert!(openapi.contains("/contracts/app/call/{fn}"));
}

#[test]
fn project_add_frontend_generates_scaffold_and_reports_paths() {
    let root = init_minimal_contract_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "project",
            "add",
            "frontend",
            "--framework",
            "react-vite",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "project.add.frontend");
    assert_eq!(json["data"]["framework"], "react-vite");
    assert!(
        json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| value.as_str() == Some("pnpm --dir apps/web dev"))
    );

    let manifest = read(root.join("stellarforge.toml"));
    let web_package = read(root.join("apps/web/package.json"));
    let web_main = read(root.join("apps/web/src/main.tsx"));
    let generated_state = read(root.join("apps/web/src/generated/stellar.ts"));
    let smoke_runner = read(root.join("apps/web/scripts/ui-smoke.mjs"));
    let browser_smoke_runner = read(root.join("apps/web/scripts/ui-browser-smoke.mjs"));

    assert!(manifest.contains("[frontend]"));
    assert!(manifest.contains("framework = \"react-vite\""));
    assert!(root.join("apps/web/index.html").exists());
    assert!(root.join("apps/web/scripts/ui-smoke.mjs").exists());
    assert!(root.join("apps/web/scripts/ui-browser-smoke.mjs").exists());
    assert!(web_package.contains("\"smoke:ui\""));
    assert!(web_package.contains("\"smoke:browser\""));
    assert!(web_package.contains("\"smoke:browser:build\""));
    assert!(web_package.contains("\"smoke:browser:install\""));
    assert!(web_package.contains("\"smoke:browser:run\""));
    assert!(web_main.contains("stellarState.project.name"));
    assert!(generated_state.contains("stellarState"));
    assert!(smoke_runner.contains("stellar forge release "));
    assert!(browser_smoke_runner.contains("@playwright/test@1.59.1"));
    assert!(browser_smoke_runner.contains("already present; skipping install"));
    assert!(browser_smoke_runner.contains("STELLAR_FORGE_BROWSER_SMOKE_PORT"));
}

#[test]
fn project_add_surfaces_write_reports_to_out_paths() {
    let assert_report = |stdout: &[u8], action: &str, out_path: &Path| {
        let json: Value = serde_json::from_slice(stdout).expect("stdout should be valid json");
        assert_eq!(json["action"], action);
        assert_eq!(
            json["data"]["out"],
            out_path.to_str().expect("out path should be valid UTF-8")
        );

        let out_json: Value =
            serde_json::from_str(&read(out_path)).expect("out report should parse");
        assert_eq!(out_json["action"], action);
        assert_eq!(
            out_json["data"]["out"],
            out_path.to_str().expect("out path should be valid UTF-8")
        );
    };

    let api_root = init_minimal_contract_project();
    let api_out = api_root.join("dist/project.add.api.json");
    let api_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&api_root)
        .args([
            "--json",
            "project",
            "add",
            "api",
            "--out",
            api_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(api_output.status.success());
    assert_report(&api_output.stdout, "project.add.api", &api_out);

    let frontend_root = init_minimal_contract_project();
    let frontend_out = frontend_root.join("dist/project.add.frontend.json");
    let frontend_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&frontend_root)
        .args([
            "--json",
            "project",
            "add",
            "frontend",
            "--framework",
            "react-vite",
            "--out",
            frontend_out
                .to_str()
                .expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(frontend_output.status.success());
    assert_report(
        &frontend_output.stdout,
        "project.add.frontend",
        &frontend_out,
    );

    let contract_root = init_rewards_project();
    let contract_out = contract_root.join("dist/project.add.contract.json");
    let contract_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&contract_root)
        .env("PATH", "")
        .args([
            "--json",
            "project",
            "add",
            "contract",
            "escrow",
            "--template",
            "escrow",
            "--out",
            contract_out
                .to_str()
                .expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(contract_output.status.success());
    assert_report(
        &contract_output.stdout,
        "project.add.contract",
        &contract_out,
    );
}

#[test]
fn project_smoke_dry_run_reports_frontend_smoke_command() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "--dry-run", "project", "smoke"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "project.smoke");
    assert_eq!(json["status"], "ok");
    assert_eq!(json["data"]["package_manager"], "pnpm");
    assert_eq!(json["data"]["install"], false);
    assert!(
        json["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .any(|value| value.as_str() == Some("pnpm --dir apps/web 'smoke:ui'"))
    );
    assert!(
        json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| value.as_str() == Some("pnpm --dir apps/web dev"))
    );
}

#[test]
fn project_smoke_browser_dry_run_reports_frontend_browser_smoke_command() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "--dry-run", "project", "smoke", "--browser"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "project.smoke");
    assert_eq!(json["status"], "ok");
    assert_eq!(json["data"]["browser"], true);
    assert!(
        json["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .any(|value| value.as_str() == Some("pnpm --dir apps/web 'smoke:browser'"))
    );
    assert!(
        json["data"]["runner"]
            .as_str()
            .expect("runner should be present")
            .ends_with("apps/web/scripts/ui-browser-smoke.mjs")
    );
}

#[test]
fn project_smoke_dry_run_can_include_install_step() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "--dry-run", "project", "smoke", "--install"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "project.smoke");
    assert_eq!(json["data"]["install"], true);
    let commands = json["commands"]
        .as_array()
        .expect("commands should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(commands.contains(&"pnpm --dir apps/web install"));
    assert!(commands.contains(&"pnpm --dir apps/web 'smoke:ui'"));
}

#[test]
fn project_smoke_dry_run_writes_report_to_out_path() {
    let root = init_rewards_project();
    let out_path = root.join("dist/project.smoke.json");
    let fake_bin = install_fake_pnpm(&root);

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env(
            "PATH",
            format!(
                "{}:{}",
                fake_bin.display(),
                std::env::var("PATH").expect("PATH should exist")
            ),
        )
        .args([
            "--json",
            "project",
            "smoke",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout_json: Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(stdout_json["action"], "project.smoke");
    assert_eq!(
        stdout_json["data"]["out"],
        out_path.to_str().expect("out path should be valid UTF-8")
    );

    let file_json: Value =
        serde_json::from_str(&read(&out_path)).expect("out file should contain valid json");
    assert_eq!(file_json["action"], "project.smoke");
    assert_eq!(
        file_json["data"]["out"],
        out_path.to_str().expect("out path should be valid UTF-8")
    );
}

#[test]
fn project_smoke_runs_install_and_browser_runner() {
    let root = init_rewards_project();
    let log_path = root.join("project-smoke.log");
    let fake_bin = install_recording_fake_commands(&root, &["pnpm"]);

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .env("FAKE_LOG_PATH", &log_path)
        .args(["--json", "project", "smoke", "--install", "--browser"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "project.smoke");
    assert_eq!(json["data"]["install"], true);
    assert_eq!(json["data"]["browser"], true);

    let invocations = read_invocation_log(&log_path);
    assert!(
        invocations
            .iter()
            .any(|line| line.starts_with("pnpm|") && line.ends_with("|--dir apps/web install"))
    );
    assert!(invocations.iter().any(|line| {
        line.starts_with("pnpm|") && line.ends_with("|--dir apps/web smoke:browser")
    }));
}

#[test]
fn project_smoke_rejects_projects_without_frontend_scaffold() {
    let root = init_minimal_contract_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "project", "smoke"])
        .output()
        .expect("command should run");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(7));
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "project.smoke");
    assert_eq!(json["status"], "error");
    assert_eq!(json["data"]["error_code"], "manifest");
    assert!(
        json["message"]
            .as_str()
            .expect("message should be present")
            .contains("frontend scaffold is not enabled in the manifest")
    );
}

#[test]
fn api_generate_contract_enables_api_and_writes_service_module() {
    let root = init_minimal_contract_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "api", "generate", "contract", "app"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "api.generate.contract");
    assert_eq!(json["data"]["contract"], "app");
    assert_eq!(json["data"]["typescript_binding"], "packages/app-ts");
    assert!(
        json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| value.as_str() == Some("pnpm --dir apps/api dev"))
    );

    let manifest = read(root.join("stellarforge.toml"));
    let contract_service = read(root.join("apps/api/src/services/contracts/app.ts"));
    let routes = read(root.join("apps/api/src/routes/contracts.ts"));

    assert!(manifest.contains("[api]"));
    assert!(manifest.contains("enabled = true"));
    assert!(contract_service.contains("resourceDefinition()"));
    assert!(contract_service.contains("tx_endpoint: '/contracts/app/tx/:fn'"));
    assert!(routes.contains("/contracts/app/call/:fn"));
}

#[test]
fn api_routes_reference_generated_resource_service_modules() {
    let root = init_rewards_project();

    let contract_routes = read(root.join("apps/api/src/routes/contracts.ts"));
    let token_routes = read(root.join("apps/api/src/routes/tokens.ts"));

    assert!(contract_routes.contains("../services/contracts/rewards.js"));
    assert!(contract_routes.contains(".preview(params.fn, request.body)"));
    assert!(contract_routes.contains(".buildTx(params.fn, request.body)"));
    assert!(token_routes.contains("../services/tokens/points.js"));
    assert!(token_routes.contains(".metadata()"));
    assert!(token_routes.contains(".payment(request.body)"));
    assert!(token_routes.contains(".trust(request.body)"));
}

#[test]
fn api_generate_token_writes_service_module_and_builder_hints() {
    let root = init_rewards_project();
    fs::write(
        root.join("apps/api/src/services/tokens/points.ts"),
        "BROKEN\n",
    )
    .expect("token service should be overwritten for the test");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "api", "generate", "token", "points"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "api.generate.token");
    assert_eq!(json["data"]["token"], "points");
    assert_eq!(json["data"]["with_sac"], true);
    assert!(
        json["data"]["builders"]
            .as_array()
            .expect("builders should be an array")
            .iter()
            .any(|value| value.as_str() == Some("sac_transfer"))
    );

    let token_service = read(root.join("apps/api/src/services/tokens/points.ts"));
    let openapi = read(root.join("apps/api/openapi.json"));

    assert!(token_service.contains("\"sac_transfer\""));
    assert!(token_service.contains("/tokens/points/payment"));
    assert!(token_service.contains("stellar forge wallet trust ${wallet} points"));
    assert!(openapi.contains("/tokens/points/payment"));
}

#[test]
fn api_init_and_generate_surfaces_write_reports_to_out_paths() {
    let assert_report = |stdout: &[u8], action: &str, out_path: &Path| {
        let json: Value = serde_json::from_slice(stdout).expect("stdout should be valid json");
        assert_eq!(json["action"], action);
        assert_eq!(
            json["data"]["out"],
            out_path.to_str().expect("out path should be valid UTF-8")
        );

        let out_json: Value =
            serde_json::from_str(&read(out_path)).expect("out report should parse");
        assert_eq!(out_json["action"], action);
        assert_eq!(
            out_json["data"]["out"],
            out_path.to_str().expect("out path should be valid UTF-8")
        );
    };

    let init_root = init_minimal_contract_project();
    let init_out = init_root.join("dist/api.init.json");
    let init_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&init_root)
        .args([
            "--json",
            "api",
            "init",
            "--out",
            init_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(init_output.status.success());
    assert_report(&init_output.stdout, "api.init", &init_out);

    let contract_root = init_minimal_contract_project();
    let contract_out = contract_root.join("dist/api.generate.contract.json");
    let contract_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&contract_root)
        .args([
            "--json",
            "api",
            "generate",
            "contract",
            "app",
            "--out",
            contract_out
                .to_str()
                .expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(contract_output.status.success());
    assert_report(
        &contract_output.stdout,
        "api.generate.contract",
        &contract_out,
    );

    let token_root = init_rewards_project();
    let token_out = token_root.join("dist/api.generate.token.json");
    let token_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&token_root)
        .args([
            "--json",
            "api",
            "generate",
            "token",
            "points",
            "--out",
            token_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(token_output.status.success());
    assert_report(&token_output.stdout, "api.generate.token", &token_out);
}

#[test]
fn project_add_contract_refreshes_api_and_frontend_derivatives() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", "")
        .args([
            "--json",
            "project",
            "add",
            "contract",
            "escrow",
            "--template",
            "escrow",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "project.add.contract");
    assert_eq!(json["data"]["contract"], "escrow");
    assert!(
        json["data"]["synced_modules"]
            .as_array()
            .expect("synced_modules should be an array")
            .iter()
            .any(|value| value.as_str() == Some("api"))
    );
    assert!(
        json["data"]["synced_modules"]
            .as_array()
            .expect("synced_modules should be an array")
            .iter()
            .any(|value| value.as_str() == Some("frontend"))
    );
    assert!(
        json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| value.as_str() == Some("stellar forge contract build escrow"))
    );

    let manifest = read(root.join("stellarforge.toml"));
    let contract_lib = read(root.join("contracts/escrow/src/lib.rs"));
    let contract_service = read(root.join("apps/api/src/services/contracts/escrow.ts"));
    let contract_routes = read(root.join("apps/api/src/routes/contracts.ts"));
    let openapi = read(root.join("apps/api/openapi.json"));
    let generated_state = read(root.join("apps/web/src/generated/stellar.ts"));

    assert!(manifest.contains("[contracts.escrow]"));
    assert!(manifest.contains("template = \"escrow\""));
    assert!(contract_lib.contains("pub fn init"));
    assert!(contract_lib.contains("pub fn release"));
    assert!(contract_lib.contains("pub fn is_released"));
    assert!(contract_service.contains("preview_endpoint: '/contracts/escrow/call/:fn'"));
    assert!(contract_routes.contains("../services/contracts/escrow.js"));
    assert!(openapi.contains("/contracts/escrow/call/{fn}"));
    assert!(generated_state.contains("\"escrow\""));
}

#[test]
fn project_sync_restores_derived_api_frontend_files_and_reports_modules() {
    let root = init_rewards_project();
    let out_path = root.join("dist/project.sync.json");
    fs::write(root.join(".env.example"), "BROKEN=1\n").expect("env example should be overwritten");
    fs::write(
        root.join("apps/api/src/services/contracts/rewards.ts"),
        "BROKEN\n",
    )
    .expect("contract service should be overwritten");
    fs::write(root.join("apps/api/openapi.json"), "{}\n").expect("openapi should be overwritten");
    fs::write(root.join("apps/web/src/generated/stellar.ts"), "BROKEN\n")
        .expect("frontend state should be overwritten");
    fs::write(root.join("apps/web/scripts/ui-smoke.mjs"), "BROKEN\n")
        .expect("frontend smoke runner should be overwritten");
    fs::write(
        root.join("apps/web/scripts/ui-browser-smoke.mjs"),
        "BROKEN\n",
    )
    .expect("frontend browser smoke runner should be overwritten");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "project",
            "sync",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "project.sync");
    let synced_modules = json["data"]["synced_modules"]
        .as_array()
        .expect("synced_modules should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(synced_modules.contains(&"env_example"));
    assert!(synced_modules.contains(&"api"));
    assert!(synced_modules.contains(&"frontend"));
    assert_eq!(
        json["data"]["out"],
        out_path.to_str().expect("out path should be valid UTF-8")
    );
    assert!(
        json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| value.as_str() == Some("stellar forge project validate"))
    );
    let out_json: Value =
        serde_json::from_str(&read(&out_path)).expect("project sync out should parse");
    assert_eq!(out_json["action"], "project.sync");

    let env_example = read(root.join(".env.example"));
    let contract_service = read(root.join("apps/api/src/services/contracts/rewards.ts"));
    let openapi = read(root.join("apps/api/openapi.json"));
    let generated_state = read(root.join("apps/web/src/generated/stellar.ts"));
    let smoke_runner = read(root.join("apps/web/scripts/ui-smoke.mjs"));
    let browser_smoke_runner = read(root.join("apps/web/scripts/ui-browser-smoke.mjs"));

    assert!(env_example.contains("STELLAR_NETWORK=testnet"));
    assert!(contract_service.contains("/contracts/rewards/call/:fn"));
    assert!(openapi.contains("/contracts/rewards/call/{fn}"));
    assert!(generated_state.contains("stellarState"));
    assert!(smoke_runner.contains("UI smoke passed"));
    assert!(browser_smoke_runner.contains("browser smoke passed"));
}

#[test]
fn api_openapi_export_rewrites_document_and_reports_path_count() {
    let root = init_rewards_project();
    fs::write(root.join("apps/api/openapi.json"), "{}\n").expect("openapi should be overwritten");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "api", "openapi", "export"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "api.openapi.export");
    assert!(
        json["data"]["path_count"]
            .as_u64()
            .expect("path_count should be numeric")
            > 0
    );
    assert!(
        json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| value.as_str() == Some("pnpm --dir apps/api dev"))
    );

    let openapi = read(root.join("apps/api/openapi.json"));
    assert!(openapi.contains("/contracts/rewards/call/{fn}"));
    assert!(openapi.contains("/tokens/points/payment"));
}

#[test]
fn api_openapi_events_and_relayer_surfaces_write_reports_to_out_paths() {
    let assert_report = |stdout: &[u8], action: &str, out_path: &Path| {
        let json: Value = serde_json::from_slice(stdout).expect("stdout should be valid json");
        assert_eq!(json["action"], action);
        assert_eq!(
            json["data"]["out"],
            out_path.to_str().expect("out path should be valid UTF-8")
        );

        let out_json: Value =
            serde_json::from_str(&read(out_path)).expect("out report should parse");
        assert_eq!(out_json["action"], action);
        assert_eq!(
            out_json["data"]["out"],
            out_path.to_str().expect("out path should be valid UTF-8")
        );
    };

    let openapi_root = init_rewards_project();
    let openapi_out = openapi_root.join("dist/api.openapi.json");
    let openapi_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&openapi_root)
        .args([
            "--json",
            "api",
            "openapi",
            "export",
            "--out",
            openapi_out
                .to_str()
                .expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(openapi_output.status.success());
    assert_report(&openapi_output.stdout, "api.openapi.export", &openapi_out);

    let events_root = init_rewards_project();
    let events_out = events_root.join("dist/api.events.init.json");
    let events_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&events_root)
        .args([
            "--json",
            "api",
            "events",
            "init",
            "--out",
            events_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(events_output.status.success());
    assert_report(&events_output.stdout, "api.events.init", &events_out);

    let relayer_root = init_rewards_project();
    let relayer_out = relayer_root.join("dist/api.relayer.init.json");
    let relayer_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&relayer_root)
        .args([
            "--json",
            "api",
            "relayer",
            "init",
            "--out",
            relayer_out
                .to_str()
                .expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(relayer_output.status.success());
    assert_report(&relayer_output.stdout, "api.relayer.init", &relayer_out);
}

#[test]
fn doctor_project_reports_scaffold_compatibility_drift() {
    let root = init_scaffold_like_project();
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["project", "adopt", "scaffold"])
        .assert()
        .success();

    fs::create_dir_all(root.join("packages/ghost-ts"))
        .expect("drift binding directory should be created");
    fs::write(
        root.join("environments.toml"),
        r#"[testnet]
rpc_url = "https://rpc.changed.example"
horizon_url = "https://horizon.example"
network_passphrase = "Test SDF Network ; September 2015"
friendbot = true

[testnet.contracts.hello]
contract_id = "CHELLO999"
alias = "hello-test"
wasm_hash = "beef-updated"

[local]
rpc_url = "http://localhost:8000/rpc"
horizon_url = "http://localhost:8000"
network_passphrase = "Standalone Network ; February 2017"
allow_http = true

[local.contracts.hello]
id = "CLOCAL123"
"#,
    )
    .expect("environments.toml should be overwritten");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "doctor", "project"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "doctor.project");
    assert_eq!(find_check(&json, "compat:scaffold:layout")["status"], "ok");
    assert_eq!(
        find_check(&json, "compat:scaffold:packages")["status"],
        "warn"
    );
    assert!(
        find_check(&json, "compat:scaffold:packages")["detail"]
            .as_str()
            .expect("package drift detail should be present")
            .contains("ghost")
    );
    assert_eq!(
        find_check(&json, "compat:scaffold:environments")["status"],
        "warn"
    );
    assert!(
        find_check(&json, "compat:scaffold:environments")["detail"]
            .as_str()
            .expect("environment drift detail should be present")
            .contains("rpc_url")
    );
    assert_eq!(
        find_check(&json, "compat:scaffold:deployments")["status"],
        "warn"
    );
    assert!(
        find_check(&json, "compat:scaffold:deployments")["detail"]
            .as_str()
            .expect("deployment drift detail should be present")
            .contains("contract_id differs")
    );
    assert_eq!(
        find_check(&json, "compat:scaffold:frontend-root")["status"],
        "ok"
    );
}

#[test]
fn dev_watch_once_dry_run_refreshes_build_and_bindings() {
    let root = init_rewards_project();
    let out_path = root.join("dist/dev.watch.json");
    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("STELLAR_FORGE_REGISTRY_MODE", "dedicated")
        .args([
            "--json",
            "--dry-run",
            "--network",
            "local",
            "dev",
            "watch",
            "--once",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "dev.watch");
    assert_eq!(
        json["data"]["out"],
        out_path.to_str().expect("out path should be valid UTF-8")
    );
    assert_eq!(json["data"]["mode"], "once");
    assert_eq!(json["data"]["contracts"][0]["name"], "rewards");
    assert_eq!(json["data"]["contracts"][0]["binding_status"], "generated");
    let out_json: Value =
        serde_json::from_str(&read(&out_path)).expect("dev watch out should parse");
    assert_eq!(out_json["action"], "dev.watch");

    let commands = json["commands"]
        .as_array()
        .expect("commands should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let artifacts = json["artifacts"]
        .as_array()
        .expect("artifacts should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(
        commands
            .iter()
            .any(|command| command.contains("stellar contract build"))
    );
    assert!(commands.iter().any(|command| {
        command.contains("stellar contract bindings typescript")
            && command.contains("--wasm")
            && command.contains("target/wasm32v1-none/release/rewards.wasm")
    }));
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact.ends_with("apps/api/src/server.ts"))
    );
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact.ends_with("apps/api/src/lib/manifest.ts"))
    );
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact.ends_with("apps/api/src/routes/events.ts"))
    );
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact.ends_with("apps/web/src/generated/stellar.ts"))
    );
}

#[test]
fn dev_basic_command_surfaces_write_reports_to_out_paths() {
    let assert_report = |stdout: &[u8], action: &str, out_path: &Path| {
        let json: Value = serde_json::from_slice(stdout).expect("stdout should be valid json");
        assert_eq!(json["action"], action);
        assert_eq!(
            json["data"]["out"],
            out_path.to_str().expect("out path should be valid UTF-8")
        );
        let out_json: Value =
            serde_json::from_str(&read(out_path)).expect("out report should parse");
        assert_eq!(out_json["action"], action);
        json
    };

    let root = init_rewards_project();

    let up_out = root.join("dist/dev.up.json");
    let up_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "dev",
            "up",
            "--out",
            up_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");
    assert!(up_output.status.success());
    let up_json = assert_report(&up_output.stdout, "dev.up", &up_out);
    assert_eq!(up_json["network"], "local");

    let down_out = root.join("dist/dev.down.json");
    let down_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "dev",
            "down",
            "--out",
            down_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");
    assert!(down_output.status.success());
    let down_json = assert_report(&down_output.stdout, "dev.down", &down_out);
    assert_eq!(down_json["network"], "local");

    let reset_out = root.join("dist/dev.reset.json");
    let reset_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "dev",
            "reset",
            "--out",
            reset_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");
    assert!(reset_output.status.success());
    let reset_json = assert_report(&reset_output.stdout, "dev.reset", &reset_out);
    assert_eq!(reset_json["network"], "local");

    let fund_out = root.join("dist/dev.fund.json");
    let fund_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "dev",
            "fund",
            "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            "--out",
            fund_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");
    assert!(fund_output.status.success());
    let fund_json = assert_report(&fund_output.stdout, "dev.fund", &fund_out);
    assert_eq!(
        fund_json["data"]["address"],
        "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"
    );

    let events_out = root.join("dist/dev.events.json");
    let events_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "dev",
            "events",
            "rewards",
            "--out",
            events_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");
    assert!(events_output.status.success());
    let events_json = assert_report(&events_output.stdout, "dev.events", &events_out);
    assert_eq!(events_json["data"]["resource"], "rewards");

    let logs_out = root.join("dist/dev.logs.json");
    let logs_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "dev",
            "logs",
            "--out",
            logs_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");
    assert!(logs_output.status.success());
    assert_report(&logs_output.stdout, "dev.logs", &logs_out);
}

#[test]
fn api_events_init_enables_api_and_generates_event_scaffold() {
    let temp = tempdir().expect("tempdir should be created");
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(temp.path())
        .args(["init", "demo", "--template", "minimal-contract"])
        .assert()
        .success();

    let root = temp.path().join("demo");
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["api", "events", "init"])
        .assert()
        .success();

    let manifest = read(root.join("stellarforge.toml"));
    let api_package = read(root.join("apps/api/package.json"));
    let api_server = read(root.join("apps/api/src/server.ts"));
    let api_health = read(root.join("apps/api/src/routes/health.ts"));
    let api_events = read(root.join("apps/api/src/routes/events.ts"));
    let api_wallets = read(root.join("apps/api/src/routes/wallets.ts"));
    let api_contracts = read(root.join("apps/api/src/routes/contracts.ts"));
    let api_tokens = read(root.join("apps/api/src/routes/tokens.ts"));
    let api_config = read(root.join("apps/api/src/lib/config.ts"));
    let api_errors = read(root.join("apps/api/src/lib/errors.ts"));
    let api_rpc = read(root.join("apps/api/src/services/rpc.ts"));
    let api_store = read(root.join("apps/api/src/lib/events-store.ts"));
    let worker = read(root.join("apps/api/src/workers/ingest-events.ts"));
    let schema = read(root.join("apps/api/db/schema.sql"));
    let env_example = read(root.join("apps/api/.env.example"));

    assert!(manifest.contains("[api]"));
    assert!(manifest.contains("enabled = true"));
    assert!(api_package.contains("\"events:ingest\""));
    assert!(api_server.contains("registerEventRoutes(app);"));
    assert!(api_server.contains("registerHealthRoutes(app);"));
    assert!(api_server.contains("registerWalletRoutes(app);"));
    assert!(api_health.contains("app.get('/health'"));
    assert!(api_health.contains("app.get('/ready'"));
    assert!(api_health.contains("app.get('/version'"));
    assert!(api_events.contains("/events/status"));
    assert!(api_events.contains("cursor_names"));
    assert!(api_events.contains("tracked_resources"));
    assert!(api_events.contains("retention_warning"));
    assert!(api_wallets.contains("app.get('/wallets'"));
    assert!(api_wallets.contains("app.get('/wallets/:name'"));
    assert!(api_contracts.contains("/contracts/app/call/:fn"));
    assert!(api_contracts.contains("/contracts/app/tx/:fn"));
    assert!(api_contracts.contains("../services/contracts/app.js"));
    assert!(api_contracts.contains(".preview(params.fn, request.body)"));
    assert!(api_tokens.contains("export function registerTokenRoutes"));
    assert!(api_config.contains("RELAYER_SUBMIT_PATH"));
    assert!(api_errors.contains("HttpError"));
    assert!(api_rpc.contains("contractPreviewTemplate"));
    assert!(api_rpc.contains("tokenPaymentTemplate"));
    assert!(api_store.contains("create table if not exists cursors"));
    assert!(api_store.contains("resolveEventWorkerConfig"));
    assert!(worker.contains("process.env.STELLAR_BIN ?? 'stellar'"));
    assert!(worker.contains("normalizeTopicFilter"));
    assert!(worker.contains("resolveEventWorkerConfig"));
    assert!(worker.contains("syncCursorSnapshot"));
    assert!(worker.contains("insertEvent(normalized, db)"));
    assert!(env_example.contains("PORT=3000"));
    assert!(env_example.contains("STELLAR_EVENTS_RESOURCES="));
    assert!(env_example.contains("STELLAR_EVENTS_TOPICS="));
    assert!(env_example.contains("STELLAR_EVENTS_TYPE=all"));
    assert!(schema.contains("last_ledger integer"));
    assert!(schema.contains("external_id text not null unique"));
}

#[test]
fn events_ingest_init_writes_report_to_out_path() {
    let temp = tempdir().expect("tempdir should be created");
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(temp.path())
        .args(["init", "demo", "--template", "minimal-contract"])
        .assert()
        .success();

    let root = temp.path().join("demo");
    let out_path = root.join("dist/events.ingest.init.json");
    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "events",
            "ingest",
            "init",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "events.ingest.init");
    assert_eq!(
        json["data"]["out"],
        out_path.to_str().expect("out path should be valid UTF-8")
    );
    let out_json: Value =
        serde_json::from_str(&read(&out_path)).expect("events ingest init out should parse");
    assert_eq!(out_json["action"], "events.ingest.init");
}

#[test]
fn api_relayer_init_scaffolds_proxy_and_wallet_pay_relayer_dry_run_targets_it() {
    let temp = tempdir().expect("tempdir should be created");
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(temp.path())
        .args(["init", "demo", "--template", "minimal-contract"])
        .assert()
        .success();

    let root = temp.path().join("demo");
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["api", "relayer", "init"])
        .assert()
        .success();

    let manifest = read(root.join("stellarforge.toml"));
    let api_server = read(root.join("apps/api/src/server.ts"));
    let relayer_route = read(root.join("apps/api/src/routes/relayer.ts"));
    let relayer_service = read(root.join("apps/api/src/services/relayer.ts"));
    let env_example = read(root.join("apps/api/.env.example"));
    let openapi = read(root.join("apps/api/openapi.json"));

    assert!(manifest.contains("relayer = true"));
    assert!(api_server.contains("registerRelayerRoutes(app);"));
    assert!(relayer_route.contains("/relayer/submit"));
    assert!(relayer_route.contains("/relayer/status"));
    assert!(relayer_service.contains("submitSponsoredTransaction"));
    assert!(env_example.contains("RELAYER_BASE_URL="));
    assert!(env_example.contains("RELAYER_API_KEY="));
    assert!(env_example.contains("RELAYER_SUBMIT_PATH=/transactions"));
    assert!(openapi.contains("/relayer/submit"));

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "pay",
            "--from",
            "alice",
            "--to",
            "alice",
            "--asset",
            "XLM",
            "--amount",
            "1",
            "--relayer",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.pay");
    assert_eq!(
        json["data"]["relay_endpoint"],
        "http://127.0.0.1:3000/relayer/submit"
    );
    assert_eq!(json["data"]["primitive"], "payment");
    assert!(
        json["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|command| command.contains("stellar tx new payment")
                && command.contains("--build-only"))
    );
    assert!(
        json["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|command| command == "POST http://127.0.0.1:3000/relayer/submit")
    );
}

#[test]
fn wallet_batch_pay_dry_run_reads_json_entries() {
    let root = init_rewards_project();
    fs::write(
        root.join("payments.json"),
        r#"[
  { "to": "bob", "amount": "10" },
  { "to": "alice", "amount": "5" }
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
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.batch-pay");
    assert_eq!(json["data"]["count"], 2);
    assert_eq!(json["data"]["format"], "json");
    let payments = json["data"]["payments"]
        .as_array()
        .expect("payments should be an array");
    assert_eq!(payments[0]["to"], "bob");
    assert_eq!(payments[1]["amount"], "5");
    let commands = json["commands"]
        .as_array()
        .expect("commands should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert_eq!(
        commands
            .iter()
            .filter(|command| command.contains("stellar tx new payment"))
            .count(),
        2
    );
    assert!(commands.iter().any(|command| command.contains("bob")));
    assert!(commands.iter().any(|command| command.contains("alice")));
}

#[test]
fn token_airdrop_dry_run_reads_csv_entries() {
    let root = init_rewards_project();
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
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "token.airdrop");
    assert_eq!(json["data"]["token"], "points");
    assert_eq!(json["data"]["count"], 2);
    assert_eq!(json["data"]["format"], "csv");
    let commands = json["commands"]
        .as_array()
        .expect("commands should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert_eq!(
        commands
            .iter()
            .filter(|command| command.contains("stellar tx new payment"))
            .count(),
        2
    );
    assert!(commands.iter().any(|command| command.contains("POINTS:")));
}

#[test]
fn events_cursor_commands_prefer_sqlite_store_when_available() {
    if !sqlite_available() {
        return;
    }

    let temp = tempdir().expect("tempdir should be created");
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(temp.path())
        .args(["init", "demo", "--template", "minimal-contract"])
        .assert()
        .success();

    let root = temp.path().join("demo");
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["api", "events", "init"])
        .assert()
        .success();

    seed_sqlite_cursor(
        &root,
        "testnet:contract:app",
        "contract",
        "app",
        Some("ledger:321"),
        Some(321),
    );
    fs::write(
        root.join("workers/events/cursors.json"),
        "{\n  \"cursors\": {\n    \"testnet:contract:app\": \"stale\"\n  }\n}\n",
    )
    .expect("snapshot should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "events", "cursor", "ls"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "events.cursor.ls");
    assert_eq!(json["data"]["source"], "sqlite");
    assert_eq!(
        json["data"]["cursors"]["testnet:contract:app"]["cursor"],
        "ledger:321"
    );

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["events", "cursor", "reset", "testnet:contract:app"])
        .assert()
        .success();

    let snapshot = read(root.join("workers/events/cursors.json"));
    assert!(!snapshot.contains("testnet:contract:app"));

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "events", "cursor", "ls"])
        .output()
        .expect("command should run");
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["data"]["source"], "sqlite");
    assert!(
        json["data"]["cursors"]
            .get("testnet:contract:app")
            .is_none()
    );
}

#[test]
fn events_cursor_ls_writes_report_to_out_path() {
    if !sqlite_available() {
        return;
    }

    let temp = tempdir().expect("tempdir should be created");
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(temp.path())
        .args(["init", "demo", "--template", "minimal-contract"])
        .assert()
        .success();

    let root = temp.path().join("demo");
    let out_path = root.join("dist/events.cursor.json");
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["api", "events", "init"])
        .assert()
        .success();
    seed_sqlite_cursor(
        &root,
        "testnet:contract:app",
        "contract",
        "app",
        Some("ledger:321"),
        Some(321),
    );

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "events",
            "cursor",
            "ls",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "events.cursor.ls");
    assert_eq!(
        json["data"]["out"],
        out_path.to_str().expect("out path should be valid UTF-8")
    );
    let out_json: Value =
        serde_json::from_str(&read(&out_path)).expect("events cursor out should parse");
    assert_eq!(out_json["action"], "events.cursor.ls");
}

#[test]
fn events_cursor_reset_writes_report_to_out_path() {
    if !sqlite_available() {
        return;
    }

    let temp = tempdir().expect("tempdir should be created");
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(temp.path())
        .args(["init", "demo", "--template", "minimal-contract"])
        .assert()
        .success();

    let root = temp.path().join("demo");
    let out_path = root.join("dist/events.cursor.reset.json");
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["api", "events", "init"])
        .assert()
        .success();
    seed_sqlite_cursor(
        &root,
        "testnet:contract:app",
        "contract",
        "app",
        Some("ledger:321"),
        Some(321),
    );

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "events",
            "cursor",
            "reset",
            "testnet:contract:app",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "events.cursor.reset");
    assert_eq!(
        json["data"]["out"],
        out_path.to_str().expect("out path should be valid UTF-8")
    );
    let out_json: Value =
        serde_json::from_str(&read(&out_path)).expect("events cursor reset out should parse");
    assert_eq!(out_json["action"], "events.cursor.reset");
}

#[test]
fn events_status_reports_local_event_store_summary() {
    if !sqlite_available() {
        return;
    }

    let temp = tempdir().expect("tempdir should be created");
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(temp.path())
        .args(["init", "demo", "--template", "minimal-contract"])
        .assert()
        .success();

    let root = temp.path().join("demo");
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["api", "events", "init"])
        .assert()
        .success();

    seed_sqlite_cursor(
        &root,
        "testnet:contract:app",
        "contract",
        "app",
        Some("ledger:321"),
        Some(321),
    );

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "events", "status"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "events.status");
    assert_eq!(json["status"], "ok");
    assert_eq!(json["data"]["total_events"], 0);
    assert_eq!(json["data"]["cursor_count"], 1);
    assert_eq!(json["data"]["latest_ledger"], Value::Null);
}

#[test]
fn events_status_writes_report_to_out_path() {
    if !sqlite_available() {
        return;
    }

    let temp = tempdir().expect("tempdir should be created");
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(temp.path())
        .args(["init", "demo", "--template", "minimal-contract"])
        .assert()
        .success();

    let root = temp.path().join("demo");
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["api", "events", "init"])
        .assert()
        .success();

    let out_path = root.join("dist/events.status.json");
    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "events",
            "status",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout_json: Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(stdout_json["action"], "events.status");
    assert_eq!(
        stdout_json["data"]["out"],
        out_path.to_str().expect("out path should be valid UTF-8")
    );
    let file_json: Value = serde_json::from_str(&read(&out_path)).expect("out file should parse");
    assert_eq!(file_json["action"], "events.status");
}

#[test]
fn events_backfill_dry_run_plans_fetch_and_sqlite_persist() {
    let root = init_rewards_project();
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["api", "events", "init"])
        .assert()
        .success();
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
      "tokens": {}
    }
  }
}"#,
    )
    .expect("lockfile should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "events",
            "backfill",
            "rewards",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "events.backfill");
    assert_eq!(json["data"]["resource"]["kind"], "contract");
    assert_eq!(json["data"]["resource"]["name"], "rewards");
    assert_eq!(json["data"]["resource"]["contract_id"], "CREWARDS123");
    let commands = json["commands"]
        .as_array()
        .expect("commands should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(commands.iter().any(|command| {
        command.contains("stellar events")
            && command.contains("--output json")
            && command.contains("--id CREWARDS123")
            && command.contains("--network testnet")
    }));
    assert!(
        commands
            .iter()
            .any(|command| command.contains("sqlite3") && command.contains("events.sqlite"))
    );
}

#[test]
fn events_watch_dry_run_forwards_count_cursor_and_topics() {
    let root = init_rewards_project();
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
      "tokens": {}
    }
  }
}"#,
    )
    .expect("lockfile should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "events",
            "watch",
            "contract",
            "rewards",
            "--count",
            "25",
            "--cursor",
            "ledger:55",
            "--topic",
            "COUNTER,*",
            "--topic",
            "mint,**",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "events.watch");
    assert_eq!(json["data"]["kind"], "contract");
    assert_eq!(json["data"]["resource"], "rewards");
    assert_eq!(json["data"]["contract_id"], "CREWARDS123");
    assert_eq!(json["data"]["count"], 25);
    assert_eq!(json["data"]["cursor"], "ledger:55");
    let commands = json["commands"]
        .as_array()
        .expect("commands should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(commands.iter().any(|command| {
        command.contains("stellar events")
            && command.contains("--id CREWARDS123")
            && command.contains("--count 25")
            && command.contains("--cursor 'ledger:55'")
            && command.contains("--topic 'AAAADwAAAAdDT1VOVEVSAA==,*'")
            && command.contains("--topic 'AAAADwAAAARtaW50,**'")
    }));
}

#[test]
fn events_backfill_dry_run_accepts_explicit_start_ledger_and_topics() {
    let root = init_rewards_project();
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["api", "events", "init"])
        .assert()
        .success();
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
      "tokens": {}
    }
  }
}"#,
    )
    .expect("lockfile should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "events",
            "backfill",
            "rewards",
            "--count",
            "50",
            "--start-ledger",
            "12345",
            "--topic",
            "COUNTER,*",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "events.backfill");
    assert_eq!(json["data"]["count"], 50);
    assert_eq!(json["data"]["start_ledger"], 12345);
    let commands = json["commands"]
        .as_array()
        .expect("commands should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(commands.iter().any(|command| {
        command.contains("stellar events")
            && command.contains("--count 50")
            && command.contains("--start-ledger 12345")
            && command.contains("--topic 'AAAADwAAAAdDT1VOVEVSAA==,*'")
    }));
}

#[test]
fn events_watch_rejects_non_terminal_deep_wildcard_topic() {
    let root = init_rewards_project();
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
      "tokens": {}
    }
  }
}"#,
    )
    .expect("lockfile should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "events",
            "watch",
            "contract",
            "rewards",
            "--topic",
            "mint,**,tail",
        ])
        .output()
        .expect("command should run");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "events.watch");
    assert_eq!(json["status"], "error");
    assert_eq!(json["data"]["error_code"], "input");
    assert_eq!(json["data"]["exit_code"], 2);
    assert!(
        json["message"]
            .as_str()
            .expect("message should be present")
            .contains("must be the last segment")
    );
    assert!(
        json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| value.as_str() == Some("move `**` to the final topic segment"))
    );
}

#[test]
fn events_watch_rejects_cursor_and_start_ledger_together() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "events",
            "watch",
            "contract",
            "rewards",
            "--cursor",
            "ledger:55",
            "--start-ledger",
            "12345",
        ])
        .output()
        .expect("command should run");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "events.watch");
    assert_eq!(json["status"], "error");
    assert_eq!(json["data"]["error_code"], "input");
    assert!(
        json["message"]
            .as_str()
            .expect("message should be present")
            .contains("use either `--cursor` or `--start-ledger`, not both")
    );
}

#[test]
fn wallet_pay_dry_run_uses_payment_for_classic_accounts() {
    let root = init_rewards_project();
    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "pay",
            "--from",
            "treasury",
            "--to",
            "alice",
            "--asset",
            "points",
            "--amount",
            "100",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["data"]["primitive"], "payment");
    assert!(
        json["commands"][0]
            .as_str()
            .expect("command should be string")
            .contains("tx new payment")
    );
}

#[test]
fn contract_spec_dry_run_reports_alias_bindings_and_paths() {
    let root = init_rewards_project();
    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "--dry-run", "contract", "spec", "rewards"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "contract.spec");
    assert_eq!(json["data"]["contract"]["alias"], "rewards");
    assert_eq!(json["data"]["contract"]["bindings"][0], "typescript");
    assert_eq!(json["data"]["contract"]["init"]["fn"], "init");
    assert_eq!(json["data"]["contract"]["effective_init"]["fn"], "init");
    assert!(
        json["data"]["paths"]["contract_dir"]
            .as_str()
            .expect("contract_dir should be a string")
            .ends_with("/contracts/rewards")
    );
    assert!(
        json["data"]["paths"]["wasm"]
            .as_str()
            .expect("wasm path should be a string")
            .contains("rewards.wasm")
    );
}

#[test]
fn contract_spec_dry_run_writes_report_to_out_path() {
    let root = init_rewards_project();
    let out_path = root.join("dist/contract.spec.json");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "contract",
            "spec",
            "rewards",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "contract.spec");
    assert_eq!(
        json["data"]["out"],
        out_path.to_str().expect("out path should be valid UTF-8")
    );
    let out_json: Value =
        serde_json::from_str(&read(&out_path)).expect("contract spec out should parse");
    assert_eq!(out_json["action"], "contract.spec");
}

#[test]
fn contract_info_dry_run_uses_stellar_contract_info_subcommands() {
    let root = init_contract_token_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "contract",
            "info",
            "credits",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "contract.info");
    assert_eq!(json["data"]["deployment"]["contract_id"], "CCREDIT123");
    assert_eq!(json["data"]["info_source"]["kind"], "contract_id");
    let commands = json["commands"]
        .as_array()
        .expect("commands should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(commands.iter().any(|command| {
        command.contains("stellar contract info interface")
            && command.contains("--contract-id CCREDIT123")
            && command.contains("--network testnet")
            && command.contains("--output rust")
    }));
    assert!(commands.iter().any(|command| {
        command.contains("stellar contract info meta")
            && command.contains("--contract-id CCREDIT123")
            && command.contains("--output json-formatted")
    }));
    assert!(commands.iter().any(|command| {
        command.contains("stellar contract info env-meta")
            && command.contains("--contract-id CCREDIT123")
            && command.contains("--output json-formatted")
    }));
    assert!(commands.iter().any(|command| {
        command.contains("stellar contract info build")
            && command.contains("--contract-id CCREDIT123")
            && command.contains("--network testnet")
    }));
    assert!(
        commands
            .iter()
            .all(|command| !command.contains("contract inspect"))
    );
}

#[test]
fn contract_info_dry_run_writes_report_to_out_path() {
    let root = init_contract_token_project();
    let out_path = root.join("dist/contract.info.json");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "contract",
            "info",
            "credits",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "contract.info");
    assert_eq!(
        json["data"]["out"],
        out_path.to_str().expect("out path should be valid UTF-8")
    );
    let out_json: Value =
        serde_json::from_str(&read(&out_path)).expect("contract info out should parse");
    assert_eq!(out_json["action"], "contract.info");
}

#[test]
fn contract_format_dry_run_reports_cargo_fmt_command() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "contract",
            "format",
            "rewards",
            "--check",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "contract.format");
    assert_eq!(json["data"]["mode"], "check");
    assert_eq!(json["data"]["contracts"][0]["name"], "rewards");
    assert!(
        json["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|command| command.contains("cargo fmt --all --check"))
    );
    assert!(
        json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| value.as_str() == Some("stellar forge contract format"))
    );
}

#[test]
fn token_info_writes_report_to_out_path() {
    let root = init_rewards_project();
    let out_path = root.join("dist/token.info.json");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "token",
            "info",
            "points",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "token.info");
    assert_eq!(
        json["data"]["out"],
        out_path.to_str().expect("out path should be valid UTF-8")
    );
    let out_json: Value =
        serde_json::from_str(&read(&out_path)).expect("token info out should parse");
    assert_eq!(out_json["action"], "token.info");
}

#[test]
fn contract_fetch_dry_run_resolves_contract_id_and_default_artifact_path() {
    let root = init_rewards_project();
    seed_testnet_release_lockfile(&root);

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "contract",
            "fetch",
            "rewards",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "contract.fetch");
    assert_eq!(json["data"]["contract"], "rewards");
    assert_eq!(json["data"]["contract_id"], "CREWARDS123");
    assert!(
        json["data"]["output"]
            .as_str()
            .expect("output should be a string")
            .ends_with("dist/contracts/rewards.testnet.wasm")
    );
    assert!(
        json["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|command| command.contains("stellar contract fetch")
                && command.contains("--id CREWARDS123")
                && command.contains("--out-file")
                && command.contains("rewards.testnet.wasm")
                && command.contains("--network testnet"))
    );
}

#[test]
fn contract_fetch_dry_run_respects_custom_output_path() {
    let root = init_rewards_project();
    seed_testnet_release_lockfile(&root);
    let out_path = root.join("tmp/custom/rewards.wasm");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "contract",
            "fetch",
            "rewards",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "contract.fetch");
    assert_eq!(
        json["data"]["output"],
        out_path.to_str().expect("out path should be valid UTF-8")
    );
    assert!(json["data"].get("out").is_none());
    assert!(
        json["artifacts"]
            .as_array()
            .expect("artifacts should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|artifact| artifact == out_path.to_str().expect("out path should be valid UTF-8"))
    );
    assert!(
        json["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|command| command.contains("stellar contract fetch")
                && command.contains("--out-file")
                && command.contains("tmp/custom/rewards.wasm"))
    );
}

#[test]
fn contract_command_surfaces_write_reports_to_out_paths() {
    let assert_report = |stdout: &[u8], action: &str, out_path: &Path| {
        let json: Value = serde_json::from_slice(stdout).expect("stdout should be valid json");
        assert_eq!(json["action"], action);
        assert_eq!(
            json["data"]["out"],
            out_path.to_str().expect("out path should be valid UTF-8")
        );

        let out_json: Value =
            serde_json::from_str(&read(out_path)).expect("out report should parse");
        assert_eq!(out_json["action"], action);
        assert_eq!(
            out_json["data"]["out"],
            out_path.to_str().expect("out path should be valid UTF-8")
        );
    };

    let format_root = init_rewards_project();
    let format_out = format_root.join("dist/contract.format.json");
    let format_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&format_root)
        .args([
            "--json",
            "--dry-run",
            "contract",
            "format",
            "rewards",
            "--check",
            "--out",
            format_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(format_output.status.success());
    assert_report(&format_output.stdout, "contract.format", &format_out);

    let build_root = init_rewards_project();
    let build_out = build_root.join("dist/contract.build.json");
    let build_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&build_root)
        .args([
            "--json",
            "--dry-run",
            "contract",
            "build",
            "rewards",
            "--out",
            build_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(build_output.status.success());
    assert_report(&build_output.stdout, "contract.build", &build_out);

    let deploy_root = init_rewards_project();
    let deploy_out = deploy_root.join("dist/contract.deploy.json");
    let deploy_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&deploy_root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "contract",
            "deploy",
            "rewards",
            "--out",
            deploy_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(deploy_output.status.success());
    assert_report(&deploy_output.stdout, "contract.deploy", &deploy_out);

    let call_root = init_rewards_project();
    seed_testnet_release_lockfile(&call_root);
    let call_out = call_root.join("dist/contract.call.json");
    let call_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&call_root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "contract",
            "call",
            "rewards",
            "increment",
            "--build-only",
            "--out",
            call_out.to_str().expect("out path should be valid UTF-8"),
            "--",
            "--amount",
            "5",
        ])
        .output()
        .expect("command should run");

    assert!(call_output.status.success());
    assert_report(&call_output.stdout, "contract.call", &call_out);

    let bind_root = init_rewards_project();
    seed_testnet_release_lockfile(&bind_root);
    let bind_out = bind_root.join("dist/contract.bind.json");
    let bind_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&bind_root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "contract",
            "bind",
            "rewards",
            "--lang",
            "typescript",
            "--out",
            bind_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(bind_output.status.success());
    assert_report(&bind_output.stdout, "contract.bind", &bind_out);
}

#[test]
fn contract_lint_runs_cargo_clippy_and_writes_report_to_out_path() {
    let root = init_rewards_project();
    let log_path = root.join("contract-lint.log");
    let fake_bin = install_recording_fake_commands(&root, &["cargo"]);
    let out_path = root.join("dist/contract.lint.json");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("HOME", &root)
        .env("PATH", test_path(&fake_bin))
        .env("FAKE_LOG_PATH", &log_path)
        .args([
            "--json",
            "contract",
            "lint",
            "rewards",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "contract.lint");
    assert_eq!(
        json["data"]["out"],
        out_path.to_str().expect("out path should be valid UTF-8")
    );
    assert_eq!(json["data"]["contracts"][0]["name"], "rewards");
    assert_eq!(
        json["data"]["profile"],
        "clippy --all-targets --all-features -- -D warnings"
    );

    let out_json: Value =
        serde_json::from_str(&read(&out_path)).expect("contract lint out should parse");
    assert_eq!(out_json["action"], "contract.lint");

    let invocations = read_invocation_log(&log_path);
    assert!(invocations.iter().any(|line| {
        line.starts_with("cargo|")
            && line.contains("/contracts/rewards|")
            && line.ends_with("clippy --all-targets --all-features -- -D warnings")
    }));
}

#[test]
fn contract_ttl_extend_dry_run_maps_spec_command_to_stellar_extend() {
    let root = init_rewards_project();
    seed_testnet_release_lockfile(&root);

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "contract",
            "ttl",
            "extend",
            "rewards",
            "--ledgers",
            "1024",
            "--key",
            "Points",
            "--ttl-ledger-only",
            "--build-only",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "contract.ttl.extend");
    assert_eq!(json["data"]["contract"], "rewards");
    assert_eq!(json["data"]["contract_id"], "CREWARDS123");
    assert_eq!(json["data"]["ledgers_to_extend"], 1024);
    let commands = json["commands"]
        .as_array()
        .expect("commands should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(commands.iter().any(|command| {
        command.contains("stellar contract extend")
            && command.contains("--id CREWARDS123")
            && command.contains("--ledgers-to-extend 1024")
            && command.contains("--source-account alice")
            && command.contains("--key Points")
            && command.contains("--durability persistent")
            && command.contains("--ttl-ledger-only")
            && command.contains("--build-only")
            && command.contains("--network testnet")
    }));
}

#[test]
fn contract_ttl_restore_dry_run_maps_spec_command_to_stellar_restore() {
    let root = init_rewards_project();
    seed_testnet_release_lockfile(&root);

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "contract",
            "ttl",
            "restore",
            "rewards",
            "--ledgers",
            "2048",
            "--durability",
            "temporary",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "contract.ttl.restore");
    assert_eq!(json["data"]["mode"], "restore");
    assert_eq!(json["data"]["durability"], "temporary");
    let commands = json["commands"]
        .as_array()
        .expect("commands should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(commands.iter().any(|command| {
        command.contains("stellar contract restore")
            && command.contains("--id CREWARDS123")
            && command.contains("--ledgers-to-extend 2048")
            && command.contains("--source-account alice")
            && command.contains("--durability temporary")
            && command.contains("--network testnet")
    }));
}

#[test]
fn contract_ttl_surfaces_write_reports_to_out_paths() {
    let assert_report = |stdout: &[u8], action: &str, out_path: &Path| {
        let json: Value = serde_json::from_slice(stdout).expect("stdout should be valid json");
        assert_eq!(json["action"], action);
        assert_eq!(
            json["data"]["out"],
            out_path.to_str().expect("out path should be valid UTF-8")
        );

        let out_json: Value =
            serde_json::from_str(&read(out_path)).expect("out report should parse");
        assert_eq!(out_json["action"], action);
        assert_eq!(
            out_json["data"]["out"],
            out_path.to_str().expect("out path should be valid UTF-8")
        );
    };

    let extend_root = init_rewards_project();
    seed_testnet_release_lockfile(&extend_root);
    let extend_out = extend_root.join("dist/contract.ttl.extend.json");
    let extend_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&extend_root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "contract",
            "ttl",
            "extend",
            "rewards",
            "--out",
            extend_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(extend_output.status.success());
    assert_report(&extend_output.stdout, "contract.ttl.extend", &extend_out);

    let restore_root = init_rewards_project();
    seed_testnet_release_lockfile(&restore_root);
    let restore_out = restore_root.join("dist/contract.ttl.restore.json");
    let restore_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&restore_root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "contract",
            "ttl",
            "restore",
            "rewards",
            "--out",
            restore_out
                .to_str()
                .expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(restore_output.status.success());
    assert_report(&restore_output.stdout, "contract.ttl.restore", &restore_out);
}

#[test]
fn wallet_receive_dry_run_resolves_token_asset_for_sep7_and_qr() {
    let root = init_rewards_project();
    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "receive",
            "alice",
            "--asset",
            "points",
            "--sep7",
            "--qr",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    let sep7 = json["data"]["sep7_uri"]
        .as_str()
        .expect("sep7 uri should be present");
    assert_eq!(json["data"]["recommended_asset"], "POINTS:<issuer>");
    assert!(sep7.contains("destination=%3Calice%3E"));
    assert!(sep7.contains("asset_code=POINTS"));
    assert!(sep7.contains("asset_issuer=%3Cissuer%3E"));
    assert_eq!(json["data"]["qr_payload"], json["data"]["sep7_uri"]);
}

#[test]
fn wallet_receive_dry_run_writes_report_to_out_path() {
    let root = init_rewards_project();
    let out_path = root.join("dist/wallet.receive.json");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "receive",
            "alice",
            "--asset",
            "points",
            "--sep7",
            "--qr",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.receive");
    assert_eq!(
        json["data"]["out"],
        out_path.to_str().expect("out path should be valid UTF-8")
    );
    let out_json: Value =
        serde_json::from_str(&read(&out_path)).expect("wallet receive out should parse");
    assert_eq!(out_json["action"], "wallet.receive");
}

#[test]
fn wallet_sep7_payment_resolves_declared_token_asset() {
    let root = init_rewards_project();
    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "sep7",
            "payment",
            "--from",
            "treasury",
            "--to",
            "alice",
            "--asset",
            "points",
            "--amount",
            "10",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    let sep7 = json["data"]["sep7_uri"]
        .as_str()
        .expect("sep7 uri should be present");
    assert!(sep7.contains("amount=10"));
    assert!(sep7.contains("asset_code=POINTS"));
    assert!(sep7.contains("asset_issuer=%3Cissuer%3E"));
}

#[test]
fn wallet_create_records_manifest_entry_and_reports_next_steps() {
    let root = init_rewards_project();
    let fake_bin = install_fake_stellar(&root);

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env(
            "PATH",
            format!(
                "{}:{}",
                fake_bin.display(),
                std::env::var("PATH").expect("PATH should exist")
            ),
        )
        .args(["--json", "wallet", "create", "bob", "--fund"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.create");
    assert_eq!(json["data"]["wallet"], "bob");
    assert_eq!(json["data"]["identity"], "bob");
    assert_eq!(json["data"]["funded"], true);
    assert_eq!(json["data"]["manifest_synced"], true);
    assert_eq!(json["data"]["network"], "testnet");
    assert!(
        json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| value.as_str() == Some("stellar forge wallet balances bob"))
    );
    assert!(
        json["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|command| {
                command.contains("stellar keys generate bob")
                    && command.contains("--network testnet")
                    && command.contains("--fund")
            })
    );

    let manifest = read(root.join("stellarforge.toml"));
    assert!(manifest.contains("[identities.bob]"));
    assert!(manifest.contains("[wallets.bob]"));
    assert!(manifest.contains("identity = \"bob\""));
}

#[test]
fn wallet_ls_dry_run_reports_declared_wallet_inventory() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "--dry-run", "wallet", "ls"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.ls");
    let declared = json["data"]["declared_wallets"]
        .as_array()
        .expect("declared_wallets should be an array");
    assert!(declared.iter().any(|wallet| wallet["name"] == "alice"));
    assert!(declared.iter().any(|wallet| wallet["name"] == "issuer"));
    assert!(
        json["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|command| command.contains("stellar keys ls -l"))
    );
}

#[test]
fn wallet_ls_dry_run_writes_report_to_out_path() {
    let root = init_rewards_project();
    let out_path = root.join("dist/wallet.ls.json");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "ls",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.ls");
    assert_eq!(
        json["data"]["out"],
        out_path.to_str().expect("out path should be valid UTF-8")
    );
    let out_json: Value =
        serde_json::from_str(&read(&out_path)).expect("wallet ls out should parse");
    assert_eq!(out_json["action"], "wallet.ls");
}

#[test]
fn wallet_address_dry_run_resolves_identity_placeholder() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "--dry-run", "wallet", "address", "treasury"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.address");
    assert_eq!(json["data"]["input"], "treasury");
    assert_eq!(json["data"]["identity"], "treasury");
    assert_eq!(json["data"]["address"], "<treasury>");
    assert_eq!(json["data"]["wallet_kind"], "classic");
}

#[test]
fn wallet_address_dry_run_writes_report_to_out_path() {
    let root = init_rewards_project();
    let out_path = root.join("dist/wallet.address.json");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "address",
            "treasury",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.address");
    assert_eq!(
        json["data"]["out"],
        out_path.to_str().expect("out path should be valid UTF-8")
    );
    let out_json: Value =
        serde_json::from_str(&read(&out_path)).expect("wallet address out should parse");
    assert_eq!(out_json["action"], "wallet.address");
}

#[test]
fn wallet_fund_dry_run_uses_wallet_action_and_friendbot_plan() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "--dry-run", "wallet", "fund", "alice"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.fund");
    assert_eq!(json["network"], "testnet");
    assert_eq!(json["data"]["target"], "alice");
    assert_eq!(json["data"]["address"], "<alice>");
    assert!(
        json["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|command| {
                command.contains("GET https://friendbot.stellar.org")
                    && command.contains("addr=%3Calice%3E")
            })
    );
    assert!(
        json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| value.as_str() == Some("stellar forge wallet balances alice"))
    );
}

#[test]
fn wallet_trust_dry_run_uses_declared_asset_and_reports_builder() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "--dry-run", "wallet", "trust", "alice", "points"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.trust");
    assert_eq!(json["network"], "testnet");
    assert_eq!(json["data"]["wallet"], "alice");
    assert_eq!(json["data"]["identity"], "alice");
    assert_eq!(json["data"]["token"], "points");
    assert_eq!(json["data"]["asset"], "POINTS:<issuer>");
    assert_eq!(json["data"]["primitive"], "change_trust");
    assert!(
        json["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|command| {
                command.contains("stellar tx new change-trust")
                    && command.contains("--source-account alice")
                    && command.contains("POINTS:<issuer>")
            })
    );
}

#[test]
fn wallet_trust_rejects_contract_tokens() {
    let root = init_contract_token_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "--dry-run", "wallet", "trust", "alice", "credits"])
        .output()
        .expect("command should run");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.trust");
    assert_eq!(json["status"], "error");
    assert_eq!(json["data"]["error_code"], "input");
    assert_eq!(json["data"]["exit_code"], 2);
    assert!(
        json["message"]
            .as_str()
            .expect("message should be present")
            .contains("does not use classic trustlines")
    );
    assert!(
        json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| {
                value.as_str()
                    == Some("use `stellar forge wallet pay ...` or a contract call for this token")
            })
    );
}

#[test]
fn wallet_smart_scaffold_generates_onboarding_app_and_policy_contract() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "wallet", "smart", "scaffold", "guardian"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.smart.scaffold");
    assert_eq!(json["data"]["wallet"], "guardian");
    assert_eq!(json["data"]["mode"], "passkey");
    assert_eq!(json["data"]["policy_contract"], "guardian-policy");
    assert_eq!(json["data"]["onboarding_app"], "apps/smart-wallet/guardian");
    assert!(
        json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| value.as_str() == Some("stellar forge contract build guardian-policy"))
    );

    let manifest = read(root.join("stellarforge.toml"));
    let readme = read(root.join("apps/smart-wallet/guardian/README.md"));
    let env = read(root.join("apps/smart-wallet/guardian/.env.example"));
    let package_json = read(root.join("apps/smart-wallet/guardian/package.json"));
    let main_ts = read(root.join("apps/smart-wallet/guardian/src/main.ts"));
    let policy_lib = read(root.join("contracts/guardian-policy/src/lib.rs"));
    let policy_test = read(root.join("contracts/guardian-policy/src/test.rs"));

    assert!(manifest.contains("[wallets.guardian]"));
    assert!(manifest.contains("kind = \"smart\""));
    assert!(manifest.contains("mode = \"passkey\""));
    assert!(manifest.contains("onboarding_app = \"apps/smart-wallet/guardian\""));
    assert!(manifest.contains("policy_contract = \"guardian-policy\""));
    assert!(manifest.contains("[contracts.guardian-policy]"));
    assert!(manifest.contains("template = \"passkey-wallet-policy\""));
    assert!(readme.contains("guardian-policy"));
    assert!(readme.contains("copy-ready checklist"));
    assert!(env.contains("SMART_WALLET_MODE=passkey"));
    assert!(env.contains("SMART_WALLET_NETWORK=testnet"));
    assert!(env.contains("SMART_WALLET_POLICY_CONTRACT=guardian-policy"));
    assert!(env.contains("SMART_WALLET_RPC_URL=https://soroban-testnet.stellar.org"));
    assert!(package_json.contains("\"vite\""));
    assert!(main_ts.contains("guardian-policy"));
    assert!(main_ts.contains("Copy env block"));
    assert!(main_ts.contains("wallet smart provision guardian --address <contract-id>"));
    assert!(main_ts.contains("contract deploy guardian-policy --env testnet"));
    assert!(main_ts.contains("\"network\": \"testnet\""));
    assert!(main_ts.contains("wallet smart policy diff guardian"));
    assert!(policy_lib.contains("set_daily_limit"));
    assert!(policy_lib.contains("require_admin"));
    assert!(policy_test.contains("policy_template_tracks_admin_limit_and_allow_list"));
}

#[test]
fn wallet_smart_scaffold_respects_selected_network_in_generated_files() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--network",
            "futurenet",
            "--json",
            "wallet",
            "smart",
            "scaffold",
            "guardian",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.smart.scaffold");

    let manifest = read(root.join("stellarforge.toml"));
    let manifest_value: toml::Value =
        toml::from_str(&manifest).expect("manifest should parse after scaffold");
    let env = read(root.join("apps/smart-wallet/guardian/.env.example"));
    let main_ts = read(root.join("apps/smart-wallet/guardian/src/main.ts"));

    let deploy_on = manifest_value["contracts"]["guardian-policy"]["deploy_on"]
        .as_array()
        .expect("deploy_on should be an array")
        .iter()
        .filter_map(toml::Value::as_str)
        .collect::<Vec<_>>();
    assert_eq!(deploy_on, vec!["local", "testnet", "futurenet"]);
    assert!(env.contains("SMART_WALLET_NETWORK=futurenet"));
    assert!(env.contains("SMART_WALLET_RPC_URL=https://rpc-futurenet.stellar.org"));
    assert!(main_ts.contains("\"network\": \"futurenet\""));
    assert!(main_ts.contains("contract deploy guardian-policy --env futurenet"));
}

#[test]
fn wallet_smart_info_reports_manifest_and_generated_paths() {
    let root = init_rewards_project();
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["wallet", "smart", "scaffold", "guardian"])
        .assert()
        .success();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "wallet", "smart", "info", "guardian"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.smart.info");
    assert_eq!(json["data"]["wallet"]["kind"], "smart");
    assert_eq!(json["data"]["wallet"]["mode"], "passkey");
    assert_eq!(
        json["data"]["wallet"]["onboarding_app"],
        "apps/smart-wallet/guardian"
    );
    assert_eq!(json["data"]["wallet"]["policy_contract"], "guardian-policy");
    assert_eq!(json["data"]["onboarding"]["exists"], true);
    assert_eq!(json["data"]["policy_contract"]["name"], "guardian-policy");
    assert_eq!(json["data"]["policy_contract"]["exists"], true);
    assert!(
        json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| {
                value.as_str()
                    == Some("stellar forge contract deploy guardian-policy --env testnet")
            })
    );
}

#[test]
fn wallet_smart_info_writes_report_to_out_path() {
    let root = init_rewards_project();
    let out_path = root.join("dist/wallet.smart.info.json");
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["wallet", "smart", "scaffold", "guardian"])
        .assert()
        .success();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "wallet",
            "smart",
            "info",
            "guardian",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.smart.info");
    assert_eq!(
        json["data"]["out"],
        out_path.to_str().expect("out path should be valid UTF-8")
    );
    let out_json: Value =
        serde_json::from_str(&read(&out_path)).expect("wallet smart info out should parse");
    assert_eq!(out_json["action"], "wallet.smart.info");
}

#[test]
fn wallet_smart_policy_info_reports_source_and_deployment_state() {
    let root = init_rewards_project();
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["wallet", "smart", "scaffold", "guardian"])
        .assert()
        .success();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "wallet", "smart", "policy", "info", "guardian"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.smart.policy.info");
    assert_eq!(json["data"]["wallet"]["name"], "guardian");
    assert_eq!(json["data"]["default_source"], "alice");
    assert_eq!(json["data"]["policy_contract"]["name"], "guardian-policy");
    assert_eq!(json["data"]["policy_contract"]["deployed"], false);
    assert_eq!(json["data"]["policy_contract"]["target"], "guardian-policy");
}

#[test]
fn wallet_smart_policy_set_daily_limit_dry_run_uses_controller_identity() {
    let root = init_rewards_project();
    let fake_bin = install_fake_stellar(&root);

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env(
            "PATH",
            format!(
                "{}:{}",
                fake_bin.display(),
                std::env::var("PATH").expect("PATH should exist")
            ),
        )
        .args(["wallet", "smart", "create", "sentinel", "--mode", "ed25519"])
        .assert()
        .success();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "smart",
            "policy",
            "set-daily-limit",
            "sentinel",
            "1250",
            "--build-only",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.smart.policy.set-daily-limit");
    assert_eq!(json["data"]["source"], "sentinel-owner");
    assert_eq!(json["data"]["daily_limit"], "1250");
    assert!(
        json["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|command| {
                command.contains("stellar contract invoke")
                    && command.contains("--id sentinel-policy")
                    && command.contains("--source-account sentinel-owner")
                    && command.contains("--send no")
                    && command.contains("set_daily_limit")
                    && command.contains("--daily_limit 1250")
            })
    );
}

#[test]
fn wallet_smart_create_ed25519_generates_controller_identity_and_scaffold() {
    let root = init_rewards_project();
    let fake_bin = install_fake_stellar(&root);

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env(
            "PATH",
            format!(
                "{}:{}",
                fake_bin.display(),
                std::env::var("PATH").expect("PATH should exist")
            ),
        )
        .args([
            "--json", "wallet", "smart", "create", "sentinel", "--mode", "ed25519",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.smart.create");
    assert_eq!(json["data"]["wallet"], "sentinel");
    assert_eq!(json["data"]["mode"], "ed25519");
    assert_eq!(json["data"]["controller_identity"], "sentinel-owner");
    assert_eq!(json["data"]["controller_generated"], true);
    assert!(
        json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| value.as_str() == Some("stellar forge wallet fund sentinel-owner"))
    );
    assert!(
        json["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|command| {
                command.contains("stellar keys generate sentinel-owner")
                    && command.contains("--network testnet")
            })
    );

    let manifest = read(root.join("stellarforge.toml"));
    let env = read(root.join("apps/smart-wallet/sentinel/.env.example"));
    let main_ts = read(root.join("apps/smart-wallet/sentinel/src/main.ts"));

    assert!(manifest.contains("[wallets.sentinel]"));
    assert!(manifest.contains("kind = \"smart\""));
    assert!(manifest.contains("mode = \"ed25519\""));
    assert!(manifest.contains("controller_identity = \"sentinel-owner\""));
    assert!(manifest.contains("[identities.sentinel-owner]"));
    assert!(manifest.contains("[wallets.sentinel-owner]"));
    assert!(env.contains("SMART_WALLET_MODE=ed25519"));
    assert!(env.contains("SMART_WALLET_NETWORK=testnet"));
    assert!(env.contains("SMART_WALLET_CONTROLLER_IDENTITY=sentinel-owner"));
    assert!(env.contains("SMART_WALLET_RPC_URL=https://soroban-testnet.stellar.org"));
    assert!(main_ts.contains("sentinel-owner"));
    assert!(main_ts.contains("controller-signing"));
    assert!(main_ts.contains("Copy command"));
    assert!(main_ts.contains("wallet fund sentinel-owner"));
    assert!(main_ts.contains("\"network\": \"testnet\""));
    assert!(main_ts.contains("wallet smart policy sync sentinel"));
}

#[test]
fn wallet_smart_create_passkey_preserves_browser_onboarding_flow() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json", "wallet", "smart", "create", "guardian", "--mode", "passkey",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.smart.create");
    assert_eq!(json["data"]["wallet"], "guardian");
    assert_eq!(json["data"]["mode"], "passkey");
    assert_eq!(json["data"]["controller_identity"], Value::Null);
    assert_eq!(json["data"]["controller_generated"], false);
    let commands = json["commands"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect::<Vec<_>>();
    assert!(
        commands
            .iter()
            .all(|command| !command.contains("stellar keys generate"))
    );

    let manifest = read(root.join("stellarforge.toml"));
    let env = read(root.join("apps/smart-wallet/guardian/.env.example"));
    let main_ts = read(root.join("apps/smart-wallet/guardian/src/main.ts"));

    assert!(manifest.contains("[wallets.guardian]"));
    assert!(manifest.contains("mode = \"passkey\""));
    assert!(env.contains("SMART_WALLET_MODE=passkey"));
    assert!(env.contains("SMART_WALLET_NETWORK=testnet"));
    assert!(!env.contains("SMART_WALLET_CONTROLLER_IDENTITY"));
    assert!(main_ts.contains("Passkey onboarding console"));
    assert!(main_ts.contains("Copy env block"));
    assert!(main_ts.contains("wallet smart provision guardian --address <contract-id>"));
    assert!(main_ts.contains("\"network\": \"testnet\""));
    assert!(main_ts.contains("WebAuthn ceremony"));
}

#[test]
fn wallet_address_rejects_smart_wallet_even_with_controller_identity() {
    let root = init_rewards_project();
    let fake_bin = install_fake_stellar(&root);

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env(
            "PATH",
            format!(
                "{}:{}",
                fake_bin.display(),
                std::env::var("PATH").expect("PATH should exist")
            ),
        )
        .args(["wallet", "smart", "create", "sentinel", "--mode", "ed25519"])
        .assert()
        .success();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["wallet", "address", "sentinel"])
        .output()
        .expect("command should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be valid utf8");
    assert!(stderr.contains("does not resolve to a classic account yet"));
}

#[test]
fn wallet_address_rejects_unsafe_identity_name() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "--dry-run", "wallet", "address", "../oops"])
        .output()
        .expect("command should run");

    assert!(!output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.address");
    assert_eq!(json["status"], "error");
    assert!(
        json["message"]
            .as_str()
            .expect("message should be present")
            .contains("identity or wallet name `../oops` must be a single filesystem-safe name")
    );
}

#[test]
fn wallet_balances_rejects_smart_wallet_even_with_controller_identity() {
    let root = init_rewards_project();
    let fake_bin = install_fake_stellar(&root);

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env(
            "PATH",
            format!(
                "{}:{}",
                fake_bin.display(),
                std::env::var("PATH").expect("PATH should exist")
            ),
        )
        .args(["wallet", "smart", "create", "sentinel", "--mode", "ed25519"])
        .assert()
        .success();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["wallet", "balances", "sentinel"])
        .output()
        .expect("command should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be valid utf8");
    assert!(stderr.contains("does not resolve to a classic account yet"));
}

#[test]
fn wallet_balances_dry_run_writes_report_to_out_path() {
    let root = init_rewards_project();
    let out_path = root.join("dist/wallet.balances.json");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "wallet",
            "balances",
            "alice",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.balances");
    assert_eq!(
        json["data"]["out"],
        out_path.to_str().expect("out path should be valid UTF-8")
    );
    let out_json: Value =
        serde_json::from_str(&read(&out_path)).expect("wallet balances out should parse");
    assert_eq!(out_json["action"], "wallet.balances");
}

#[test]
fn wallet_pay_rejects_smart_wallet_even_with_controller_identity() {
    let root = init_rewards_project();
    let fake_bin = install_fake_stellar(&root);

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env(
            "PATH",
            format!(
                "{}:{}",
                fake_bin.display(),
                std::env::var("PATH").expect("PATH should exist")
            ),
        )
        .args(["wallet", "smart", "create", "sentinel", "--mode", "ed25519"])
        .assert()
        .success();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "pay",
            "--from",
            "treasury",
            "--to",
            "sentinel",
            "--asset",
            "points",
            "--amount",
            "1",
        ])
        .output()
        .expect("command should run");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.pay");
    assert_eq!(json["status"], "error");
    assert_eq!(json["data"]["error_code"], "input");
    assert!(
        json["message"]
            .as_str()
            .expect("message should be present")
            .contains("does not resolve to a classic account yet")
    );
    assert!(
        json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| value.as_str() == Some("stellar forge wallet smart info sentinel"))
    );
}

#[test]
fn wallet_trust_rejects_smart_wallet_even_with_controller_identity() {
    let root = init_rewards_project();
    let fake_bin = install_fake_stellar(&root);

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env(
            "PATH",
            format!(
                "{}:{}",
                fake_bin.display(),
                std::env::var("PATH").expect("PATH should exist")
            ),
        )
        .args(["wallet", "smart", "create", "sentinel", "--mode", "ed25519"])
        .assert()
        .success();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "trust",
            "sentinel",
            "points",
        ])
        .output()
        .expect("command should run");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.trust");
    assert_eq!(json["status"], "error");
    assert_eq!(json["data"]["error_code"], "input");
    assert!(
        json["message"]
            .as_str()
            .expect("message should be present")
            .contains("does not resolve to a classic account yet")
    );
    assert!(
        json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| value.as_str() == Some("stellar forge wallet smart info sentinel"))
    );
}

#[test]
fn wallet_create_rejects_existing_smart_wallet_name() {
    let root = init_rewards_project();
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["wallet", "smart", "scaffold", "guardian"])
        .assert()
        .success();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "--dry-run", "wallet", "create", "guardian"])
        .output()
        .expect("command should run");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.create");
    assert_eq!(json["status"], "error");
    assert_eq!(json["data"]["error_code"], "input");
    assert!(
        json["message"]
            .as_str()
            .expect("message should be present")
            .contains("already exists as a smart wallet")
    );
    assert!(
        json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| value.as_str() == Some("stellar forge wallet smart info guardian"))
    );
}

#[test]
fn events_watch_account_dry_run_uses_horizon_payments() {
    let root = init_rewards_project();
    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "events",
            "watch",
            "account",
            "alice",
            "--count",
            "5",
            "--cursor",
            "now",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "events.watch");
    assert_eq!(json["data"]["kind"], "account");
    assert_eq!(json["data"]["source"], "horizon");
    assert_eq!(json["data"]["stream"], "account_payments");
    assert_eq!(json["data"]["resolved_address"], "<alice>");
    assert!(
        json["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|command| {
                command.contains(
                    "GET https://horizon-testnet.stellar.org/accounts/%3Calice%3E/payments",
                ) && command.contains("limit=5")
                    && command.contains("cursor=now")
                    && command.contains("order=desc")
            })
    );
}

#[test]
fn events_watch_writes_report_to_out_path() {
    let root = init_rewards_project();
    seed_testnet_release_lockfile(&root);
    let out_path = root.join("dist/events.watch.json");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "events",
            "watch",
            "contract",
            "rewards",
            "--count",
            "5",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "events.watch");
    assert_eq!(
        json["data"]["out"],
        out_path.to_str().expect("out path should be valid UTF-8")
    );
    let out_json: Value =
        serde_json::from_str(&read(&out_path)).expect("events watch out should parse");
    assert_eq!(out_json["action"], "events.watch");
}

#[test]
fn events_backfill_account_dry_run_plans_horizon_payment_backfill() {
    let root = init_rewards_project();
    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "events",
            "backfill",
            "account:alice",
            "--count",
            "25",
            "--cursor",
            "ledger:55",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "events.backfill");
    assert_eq!(json["data"]["resource"]["kind"], "account");
    assert_eq!(json["data"]["source"], "horizon");
    assert_eq!(json["data"]["stream"], "account_payments");
    assert_eq!(json["data"]["cursor_name"], "testnet:account:alice");
    assert!(
        json["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|command| {
                command.contains(
                    "GET https://horizon-testnet.stellar.org/accounts/%3Calice%3E/payments",
                ) && command.contains("limit=25")
                    && command.contains("cursor=ledger%3A55")
                    && command.contains("order=asc")
            })
    );
}

#[test]
fn wallet_pay_contract_destination_requires_sac_materialization() {
    let root = init_rewards_project();
    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "pay",
            "--from",
            "treasury",
            "--to",
            "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "--asset",
            "points",
            "--amount",
            "1",
        ])
        .output()
        .expect("command should run");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(9));
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "wallet.pay");
    assert_eq!(json["status"], "error");
    assert_eq!(json["data"]["error_code"], "state");
    assert_eq!(json["data"]["exit_code"], 9);
    assert!(
        json["message"]
            .as_str()
            .expect("message should be present")
            .contains("needs a SAC")
    );
    assert!(
        json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| value.as_str() == Some("stellar forge token sac deploy points"))
    );
}

#[test]
fn token_balance_dry_run_reports_declared_token_targets() {
    let root = init_rewards_project();
    fs::write(
        root.join("stellarforge.lock.json"),
        r#"{
  "version": 1,
  "environments": {
    "testnet": {
      "contracts": {},
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

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "token",
            "balance",
            "points",
            "--holder",
            "alice",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["data"]["token"]["name"], "points");
    assert_eq!(json["data"]["token"]["classic_asset"], "POINTS:<issuer>");
    assert_eq!(json["data"]["token"]["sac_contract_id"], "CSAC123");
    assert!(
        json["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|command| {
                command.contains("GET https://horizon-testnet.stellar.org/accounts/%3Calice%3E")
            })
    );
    assert!(
        json["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|command| {
                command.contains("stellar contract invoke")
                    && command.contains("--id CSAC123")
                    && command.contains("balance")
            })
    );
}

#[test]
fn token_burn_dry_run_returns_classic_asset_to_issuer() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "token",
            "burn",
            "points",
            "--amount",
            "5",
            "--from",
            "treasury",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "token.burn");
    assert_eq!(json["data"]["mode"], "asset");
    assert_eq!(json["data"]["primitive"], "payment");
    assert!(
        json["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|command| command.contains("stellar tx new payment")
                && command.contains("--source-account treasury")
                && command.contains("--destination '<issuer>'")
                && command.contains("POINTS:<issuer>")
                && command.contains("--amount 50000000"))
    );
}

#[test]
fn token_contract_init_dry_run_uses_manifest_defaults() {
    let root = init_contract_token_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "token",
            "contract",
            "init",
            "credits",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "token.contract.init");
    assert_eq!(json["data"]["token"], "credits");
    assert_eq!(json["data"]["contract_id"], "CCREDIT123");
    assert!(
        json["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|command| command.contains("stellar contract invoke")
                && command.contains("--id CCREDIT123")
                && command.contains("--source-account issuer")
                && command.contains("--admin '<issuer>'")
                && command.contains("--name 'Store Credit'")
                && command.contains("--symbol CREDIT")
                && command.contains("--decimals 7"))
    );
}

#[test]
fn token_contract_init_dry_run_fills_missing_contract_token_defaults() {
    let root = init_contract_token_project();
    let manifest = read(root.join("stellarforge.toml")).replace(
        r#"[contracts.credits.init]
fn = "init"
admin = "@identity:issuer"
name = "Store Credit"
symbol = "CREDIT"
decimals = "7"
"#,
        r#"[contracts.credits.init]
symbol = "CREDIT"
"#,
    );
    fs::write(root.join("stellarforge.toml"), manifest).expect("manifest should be updated");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "token",
            "contract",
            "init",
            "credits",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "token.contract.init");
    assert!(
        json["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|command| command.contains("stellar contract invoke")
                && command.contains("--id CCREDIT123")
                && command.contains(" -- init")
                && command.contains("--admin '<issuer>'")
                && command.contains("--name 'Store Credit'")
                && command.contains("--symbol CREDIT")
                && command.contains("--decimals 7"))
    );
}

#[test]
fn token_mint_contract_dry_run_uses_contract_invoke() {
    let root = init_contract_token_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "token",
            "mint",
            "credits",
            "--to",
            "alice",
            "--amount",
            "10",
            "--from",
            "issuer",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "token.mint");
    assert_eq!(json["data"]["mode"], "contract");
    assert!(
        json["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|command| command.contains("stellar contract invoke")
                && command.contains("--id CCREDIT123")
                && command.contains("--source-account issuer")
                && command.contains(" mint ")
                && command.contains("--to '<alice>'")
                && command.contains("--amount 100000000"))
    );
}

#[test]
fn token_burn_contract_dry_run_uses_contract_invoke() {
    let root = init_contract_token_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "token",
            "burn",
            "credits",
            "--amount",
            "3",
            "--from",
            "treasury",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "token.burn");
    assert_eq!(json["data"]["mode"], "contract");
    assert!(
        json["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|command| command.contains("stellar contract invoke")
                && command.contains("--id CCREDIT123")
                && command.contains("--source-account treasury")
                && command.contains(" burn ")
                && command.contains("--from '<treasury>'")
                && command.contains("--amount 30000000"))
    );
}

#[test]
fn token_create_contract_dry_run_plans_init_bindings_and_initial_mint() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "token",
            "create",
            "credits",
            "--mode",
            "contract",
            "--metadata-name",
            "Store Credit",
            "--initial-supply",
            "25",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "token.create");
    assert_eq!(json["data"]["mode"], "contract");
    assert_eq!(json["data"]["contract"]["template"], "openzeppelin-token");
    let commands = json["commands"]
        .as_array()
        .expect("commands should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(commands.iter().any(|command| {
        command.contains("stellar contract deploy")
            && command.contains("--alias credits")
            && command.contains("credits.wasm")
    }));
    assert!(commands.iter().any(|command| {
        command.contains("stellar contract invoke")
            && command.contains("--id credits")
            && command.contains("--source-account alice")
            && command.contains("--admin '<issuer>'")
            && command.contains("--name 'Store Credit'")
            && command.contains("--symbol CREDITS")
            && command.contains("--decimals 7")
    }));
    assert!(commands.iter().any(|command| {
        command.contains("stellar contract bindings typescript")
            && command.contains("--wasm")
            && command.contains("credits.wasm")
            && command.contains("packages/credits-ts")
    }));
    assert!(commands.iter().any(|command| {
        command.contains("stellar contract invoke")
            && command.contains("--id credits")
            && command.contains("--source-account issuer")
            && command.contains(" mint ")
            && command.contains("--to '<treasury>'")
            && command.contains("--amount 250000000")
    }));
}

#[test]
fn token_clawback_rejects_when_manifest_disables_it() {
    let root = init_rewards_project();
    let manifest_path = root.join("stellarforge.toml");
    let manifest = read(&manifest_path);
    fs::write(
        &manifest_path,
        manifest.replace("clawback_enabled = true", "clawback_enabled = false"),
    )
    .expect("manifest should be rewritten");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "token",
            "clawback",
            "points",
            "alice",
            "1",
        ])
        .output()
        .expect("command should run");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "token.clawback");
    assert_eq!(json["status"], "error");
    assert_eq!(json["data"]["error_code"], "input");
    assert_eq!(json["data"]["exit_code"], 2);
    assert!(
        json["message"]
            .as_str()
            .expect("message should be present")
            .contains("clawback_enabled = true")
    );
    assert!(
        json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| {
                value.as_str() == Some("enable `clawback_enabled = true` in `stellarforge.toml`")
            })
    );
}

#[test]
fn release_env_export_materializes_env_file_from_lockfile() {
    let root = init_rewards_project();
    fs::write(
        root.join("workers/events/cursors.json"),
        r#"{
  "cursors": {
    "rewards-feed": "ledger:42"
  }
}"#,
    )
    .expect("event cursors should be written");
    seed_testnet_release_lockfile(&root);

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["release", "env", "export", "testnet"])
        .assert()
        .success();

    let env_contents =
        fs::read_to_string(root.join(".env.generated")).expect(".env.generated should exist");
    let deploy_artifact = read(root.join("dist/deploy.testnet.json"));
    let web_state = read(root.join("apps/web/src/generated/stellar.ts"));
    assert!(env_contents.contains("PUBLIC_REWARDS_CONTRACT_ID=CREWARDS123"));
    assert!(env_contents.contains("PUBLIC_POINTS_ASSET=POINTS:GISSUER123"));
    assert!(env_contents.contains("PUBLIC_POINTS_SAC_ID=CSAC123"));
    assert!(deploy_artifact.contains("\"environment\": \"testnet\""));
    assert!(deploy_artifact.contains("\"rpc_url\": \"https://soroban-testnet.stellar.org\""));
    assert!(deploy_artifact.contains("\"contract_id\": \"CREWARDS123\""));
    assert!(deploy_artifact.contains("\"asset\": \"POINTS:GISSUER123\""));
    assert!(deploy_artifact.contains("\"sac_contract_id\": \"CSAC123\""));
    assert!(web_state.contains("\"environment\": \"testnet\""));
    assert!(web_state.contains("\"rpc_url\": \"https://soroban-testnet.stellar.org\""));
    assert!(web_state.contains("\"api\": {"));
    assert!(web_state.contains("\"wallets\": {"));
    assert!(web_state.contains("\"events\": {"));
    assert!(web_state.contains("\"rewards-feed\""));
    assert!(web_state.contains("\"ledger:42\""));
    assert!(web_state.contains("\"contract_id\": \"CREWARDS123\""));
    assert!(web_state.contains("\"sac_contract_id\": \"CSAC123\""));
}

#[test]
fn release_plan_reports_required_identities_commands_and_lockfile_changes() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "--dry-run", "release", "plan", "testnet"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "release.plan");
    assert_eq!(json["status"], "ok");
    assert_eq!(find_check(&json, "manifest")["status"], "ok");

    let identities = json["data"]["required_identities"]
        .as_array()
        .expect("required identities should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(identities.contains(&"alice"));
    assert!(identities.contains(&"issuer"));
    assert!(identities.contains(&"treasury"));

    let commands = json["commands"]
        .as_array()
        .expect("commands should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(
        commands
            .iter()
            .any(|command| command.contains("stellar tx new change-trust"))
    );
    assert!(commands.iter().any(|command| {
        command.contains("stellar contract asset deploy") && command.contains("--alias points-sac")
    }));
    assert!(commands.iter().any(|command| {
        command.contains("stellar contract deploy") && command.contains("--alias rewards")
    }));

    let changes = json["data"]["lockfile_changes"]
        .as_array()
        .expect("lockfile changes should be an array");
    assert!(
        changes.iter().any(|entry| {
            entry["resource"] == "contract:rewards" && entry["action"] == "create"
        })
    );
    assert!(
        changes
            .iter()
            .any(|entry| entry["resource"] == "token:points" && entry["action"] == "create")
    );
    assert!(
        json["artifacts"]
            .as_array()
            .expect("artifacts should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|artifact| artifact.ends_with(".env.generated"))
    );
    assert!(
        json["artifacts"]
            .as_array()
            .expect("artifacts should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|artifact| artifact.ends_with("dist/deploy.testnet.json"))
    );
}

#[test]
fn release_aliases_sync_dry_run_plans_contract_and_sac_aliases() {
    let root = init_rewards_project();
    seed_testnet_release_lockfile(&root);

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "release",
            "aliases",
            "sync",
            "testnet",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "release.aliases.sync");
    assert_eq!(json["status"], "ok");
    let commands = json["commands"]
        .as_array()
        .expect("commands should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(commands.iter().any(|command| {
        command.contains("stellar contract alias add")
            && command.contains("CREWARDS123")
            && command.contains("rewards")
    }));
    assert!(commands.iter().any(|command| {
        command.contains("stellar contract alias add")
            && command.contains("CSAC123")
            && command.contains("points-sac")
    }));
    assert_eq!(
        json["data"]["synced"]
            .as_array()
            .expect("synced aliases should be an array")
            .len(),
        2
    );
}

#[test]
fn release_deploy_and_registry_surfaces_write_reports_to_out_paths() {
    let assert_report = |stdout: &[u8], action: &str, out_path: &Path| {
        let json: Value = serde_json::from_slice(stdout).expect("stdout should be valid json");
        assert_eq!(json["action"], action);
        assert_eq!(
            json["data"]["out"],
            out_path.to_str().expect("out path should be valid UTF-8")
        );

        let out_json: Value =
            serde_json::from_str(&read(out_path)).expect("out report should parse");
        assert_eq!(out_json["action"], action);
        assert_eq!(
            out_json["data"]["out"],
            out_path.to_str().expect("out path should be valid UTF-8")
        );
    };

    let deploy_root = init_rewards_project();
    let deploy_out = deploy_root.join("dist/release.deploy.json");
    let deploy_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&deploy_root)
        .args([
            "--json",
            "--dry-run",
            "release",
            "deploy",
            "testnet",
            "--out",
            deploy_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(deploy_output.status.success());
    assert_report(&deploy_output.stdout, "release.deploy", &deploy_out);

    let publish_root = init_rewards_project();
    let publish_out = publish_root.join("dist/release.registry.publish.json");
    let publish_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&publish_root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "release",
            "registry",
            "publish",
            "rewards",
            "--out",
            publish_out
                .to_str()
                .expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(publish_output.status.success());
    assert_report(
        &publish_output.stdout,
        "release.registry.publish",
        &publish_out,
    );

    let registry_root = init_rewards_project();
    fs::write(
        registry_root.join("dist/registry.testnet.json"),
        r#"{
  "version": 1,
  "environment": "testnet",
  "contracts": {
    "rewards": {
      "wasm_name": "rewards-registry",
      "version": "1.2.3"
    }
  }
}"#,
    )
    .expect("registry artifact should be written");
    let registry_out = registry_root.join("dist/release.registry.deploy.json");
    let registry_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&registry_root)
        .env("STELLAR_FORGE_REGISTRY_MODE", "dedicated")
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "release",
            "registry",
            "deploy",
            "rewards",
            "--out",
            registry_out
                .to_str()
                .expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(registry_output.status.success());
    assert_report(
        &registry_output.stdout,
        "release.registry.deploy",
        &registry_out,
    );
}

#[test]
fn release_registry_publish_dry_run_plans_build_and_registry_publish() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "release",
            "registry",
            "publish",
            "rewards",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "release.registry.publish");
    assert_eq!(json["network"], "testnet");
    assert_eq!(json["data"]["contract"], "rewards");
    assert_eq!(json["data"]["wasm_name"], "rewards");
    assert_eq!(json["data"]["version"], "0.1.0");
    let commands = json["commands"]
        .as_array()
        .expect("commands should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(
        commands
            .iter()
            .any(|command| command.contains("stellar contract build"))
    );
    assert!(commands.iter().any(|command| {
        command.contains("stellar-registry publish")
            && command.contains("--wasm-name rewards")
            && command.contains("--binver 0.1.0")
            && command.contains("--network testnet")
    }));
    assert!(
        json["artifacts"]
            .as_array()
            .expect("artifacts should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|artifact| artifact.ends_with("dist/registry.testnet.json"))
    );
}

#[test]
fn release_registry_deploy_dry_run_uses_registry_metadata_and_plans_install_and_init() {
    let root = init_rewards_project();
    fs::write(
        root.join("dist/registry.testnet.json"),
        r#"{
  "version": 1,
  "environment": "testnet",
  "contracts": {
    "rewards": {
      "wasm_name": "rewards-registry",
      "version": "1.2.3"
    }
  }
}"#,
    )
    .expect("registry artifact should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("STELLAR_FORGE_REGISTRY_MODE", "dedicated")
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "release",
            "registry",
            "deploy",
            "rewards",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "release.registry.deploy");
    assert_eq!(json["network"], "testnet");
    assert_eq!(json["data"]["contract"], "rewards");
    assert_eq!(json["data"]["wasm_name"], "rewards-registry");
    let commands = json["commands"]
        .as_array()
        .expect("commands should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(commands.iter().any(|command| {
        command.contains("stellar-registry deploy")
            && command.contains("--contract-name rewards")
            && command.contains("--wasm-name rewards-registry")
            && command.contains("--version 1.2.3")
            && command.contains("--network testnet")
    }));
    assert!(
        commands
            .iter()
            .any(|command| command.contains("stellar-registry install rewards"))
    );
    assert!(commands.iter().any(|command| {
        command.contains("stellar contract invoke")
            && command.contains("--id rewards")
            && command.contains("init")
    }));
    assert!(
        json["artifacts"]
            .as_array()
            .expect("artifacts should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|artifact| artifact.ends_with("dist/registry.testnet.json"))
    );
    assert!(
        json["artifacts"]
            .as_array()
            .expect("artifacts should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|artifact| artifact.ends_with("stellarforge.lock.json"))
    );
}

#[test]
fn release_plan_reports_registry_alternative_when_artifact_exists() {
    let root = init_rewards_project();
    fs::write(
        root.join("dist/registry.testnet.json"),
        r#"{
  "version": 1,
  "environment": "testnet",
  "contracts": {
    "rewards": {
      "wasm_name": "rewards-registry",
      "version": "1.2.3"
    }
  }
}"#,
    )
    .expect("registry artifact should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("STELLAR_FORGE_REGISTRY_MODE", "dedicated")
        .args(["--json", "--dry-run", "release", "plan", "testnet"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "release.plan");
    let alternatives = json["data"]["registry_alternatives"]
        .as_array()
        .expect("registry alternatives should be an array");
    assert_eq!(alternatives.len(), 1);
    assert_eq!(alternatives[0]["contract"], "rewards");
    assert_eq!(alternatives[0]["wasm_name"], "rewards-registry");
    assert_eq!(alternatives[0]["version"], "1.2.3");
    let alt_commands = alternatives[0]["commands"]
        .as_array()
        .expect("registry alternative commands should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(alt_commands.iter().any(|command| {
        command.contains("stellar-registry deploy")
            && command.contains("--contract-name rewards")
            && command.contains("--wasm-name rewards-registry")
            && command.contains("--version 1.2.3")
    }));
    assert!(
        json["warnings"]
            .as_array()
            .expect("warnings should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|warning| warning.contains("registry metadata detected"))
    );
    assert!(
        json["artifacts"]
            .as_array()
            .expect("artifacts should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|artifact| artifact.ends_with("dist/registry.testnet.json"))
    );
}

#[test]
fn release_verify_reports_registry_artifact_drift() {
    let root = init_rewards_project();
    seed_testnet_release_lockfile(&root);
    fs::write(
        root.join("dist/registry.testnet.json"),
        r#"{
  "version": 1,
  "environment": "testnet",
  "contracts": {
    "rewards": {
      "contract_id": "CSTALE123",
      "wasm_hash": "badcafe",
      "installed_alias": "rewards"
    }
  }
}"#,
    )
    .expect("registry artifact should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "--dry-run", "release", "verify", "testnet"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "release.verify");
    assert_eq!(
        find_check(&json, "release:testnet:registry:artifact")["status"],
        "ok"
    );
    assert_eq!(
        find_check(&json, "release:testnet:registry:artifact:rewards")["status"],
        "error"
    );
    let detail = find_check(&json, "release:testnet:registry:artifact:rewards")["detail"]
        .as_str()
        .expect("registry drift detail should be present");
    assert!(detail.contains("contract_id"));
    assert!(detail.contains("wasm_hash"));
}

#[test]
fn release_verify_reports_local_drift_in_env_artifact_and_event_config() {
    let root = init_rewards_project();
    seed_testnet_release_lockfile(&root);

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["release", "env", "export", "testnet"])
        .assert()
        .success();

    fs::write(
        root.join(".env.generated"),
        "PUBLIC_STELLAR_NETWORK=testnet\nPUBLIC_STELLAR_RPC_URL=https://soroban-testnet.stellar.org\nPUBLIC_REWARDS_CONTRACT_ID=CSTALE123\nPUBLIC_POINTS_ASSET=POINTS:GISSUER123\nPUBLIC_POINTS_SAC_ID=CSAC123\n",
    )
    .expect("env file should be overwritten");
    fs::write(
        root.join("dist/deploy.testnet.json"),
        r#"{
  "environment": "testnet",
  "network": {
    "rpc_url": "https://soroban-testnet.stellar.org",
    "horizon_url": "https://horizon-testnet.stellar.org"
  },
  "contracts": {
    "rewards": {
      "contract_id": "CSTALE123"
    }
  },
  "tokens": {
    "points": {
      "asset": "POINTS:GISSUER123",
      "sac_contract_id": "CSAC123"
    }
  }
}"#,
    )
    .expect("deploy artifact should be overwritten");
    fs::write(
        root.join("apps/api/.env"),
        "STELLAR_EVENTS_BATCH_SIZE=0\nSTELLAR_EVENTS_TYPE=weird\nSTELLAR_EVENTS_RESOURCES=contract:missing\n",
    )
    .expect("api env should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "--dry-run", "release", "verify", "testnet"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "release.verify");
    assert_eq!(json["status"], "error");
    assert_eq!(
        find_check(&json, "release:testnet:env-generated:consistency")["status"],
        "error"
    );
    assert!(
        find_check(&json, "release:testnet:env-generated:consistency")["detail"]
            .as_str()
            .expect("env consistency detail should be present")
            .contains("PUBLIC_REWARDS_CONTRACT_ID")
    );
    assert_eq!(
        find_check(&json, "release:testnet:deploy-artifact:consistency")["status"],
        "error"
    );
    assert!(
        find_check(&json, "release:testnet:deploy-artifact:consistency")["detail"]
            .as_str()
            .expect("artifact consistency detail should be present")
            .contains("contract `rewards`")
    );
    assert_eq!(
        find_check(&json, "release:testnet:events:config")["status"],
        "error"
    );
    assert!(
        find_check(&json, "release:testnet:events:config")["detail"]
            .as_str()
            .expect("events config detail should be present")
            .contains("STELLAR_EVENTS_BATCH_SIZE")
    );
}

#[test]
fn release_rollback_restores_lockfile_from_latest_history_artifact() {
    let root = init_rewards_project();
    seed_testnet_release_lockfile(&root);

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
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
      "contract_id": "CROLLBACK123",
      "alias": "rewards",
      "wasm_hash": "rollbackbeef",
      "tx_hash": "",
      "deployed_at": "2026-04-13T00:00:00Z"
    }
  },
  "tokens": {
    "points": {
      "kind": "asset",
      "asset": "POINTS:GROLLBACK123",
      "issuer_identity": "issuer",
      "distribution_identity": "treasury",
      "sac_contract_id": "CSACROLLBACK123",
      "contract_id": ""
    }
  }
}"#,
    )
    .expect("historical artifact should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "release", "rollback", "testnet"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "release.rollback");
    assert_eq!(json["network"], "testnet");
    assert!(
        json["data"]["source_artifact"]
            .as_str()
            .expect("source artifact should be present")
            .ends_with("dist/history/deploy.testnet.20260413T000000.000Z.json")
    );

    let lockfile: Value = serde_json::from_str(&read(root.join("stellarforge.lock.json")))
        .expect("lockfile should parse");
    assert_eq!(
        lockfile["environments"]["testnet"]["contracts"]["rewards"]["contract_id"],
        "CROLLBACK123"
    );
    assert_eq!(
        lockfile["environments"]["testnet"]["tokens"]["points"]["asset"],
        "POINTS:GROLLBACK123"
    );

    let env_generated = read(root.join(".env.generated"));
    assert!(env_generated.contains("PUBLIC_REWARDS_CONTRACT_ID=CROLLBACK123"));
    assert!(env_generated.contains("PUBLIC_POINTS_ASSET=POINTS:GROLLBACK123"));

    let deploy_artifact = read(root.join("dist/deploy.testnet.json"));
    assert!(deploy_artifact.contains("CROLLBACK123"));
    assert!(deploy_artifact.contains("CSACROLLBACK123"));

    let archived_current = fs::read_dir(root.join("dist/history"))
        .expect("history dir should be readable")
        .filter_map(Result::ok)
        .map(|entry| read(entry.path()))
        .any(|contents| contents.contains("CREWARDS123"));
    assert!(archived_current);
}

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
    let historical_artifact = root.join("dist/history/deploy.testnet.20260413T000000.000Z.json");
    fs::write(
        &historical_artifact,
        r#"{
  "environment": "testnet",
  "contracts": {
    "rewards": {
      "contract_id": "CROLLBACK123",
      "alias": "rewards",
      "wasm_hash": "rollbackbeef",
      "tx_hash": "",
      "deployed_at": "2026-04-13T00:00:00Z"
    }
  },
  "tokens": {
    "points": {
      "kind": "asset",
      "asset": "POINTS:GROLLBACK123",
      "issuer_identity": "issuer",
      "distribution_identity": "treasury",
      "sac_contract_id": "CSACROLLBACK123",
      "contract_id": ""
    }
  }
}"#,
    )
    .expect("historical artifact should be written");

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
    assert_eq!(json["data"]["current"]["kind"], "current");
    assert!(
        json["data"]["current"]["path"]
            .as_str()
            .expect("current path should exist")
            .ends_with("dist/deploy.testnet.json")
    );
    assert_eq!(json["data"]["history"].as_array().map(Vec::len), Some(1));
    assert_eq!(json["data"]["history"][0]["kind"], "history");
    assert_eq!(json["data"]["history"][0]["contracts"]["count"], 1);
    assert_eq!(json["data"]["history"][0]["tokens"]["count"], 1);
}

#[test]
fn release_inspect_summarizes_selected_artifact() {
    let root = init_rewards_project();
    fs::create_dir_all(root.join("dist/history")).expect("history dir should be created");
    let historical_artifact = root.join("dist/history/deploy.testnet.20260413T000000.000Z.json");
    fs::write(
        &historical_artifact,
        r#"{
  "environment": "testnet",
  "contracts": {
    "rewards": {
      "contract_id": "CROLLBACK123",
      "alias": "rewards",
      "wasm_hash": "rollbackbeef",
      "tx_hash": "",
      "deployed_at": "2026-04-13T00:00:00Z"
    }
  },
  "tokens": {
    "points": {
      "kind": "asset",
      "asset": "POINTS:GROLLBACK123",
      "issuer_identity": "issuer",
      "distribution_identity": "treasury",
      "sac_contract_id": "CSACROLLBACK123",
      "contract_id": ""
    }
  }
}"#,
    )
    .expect("historical artifact should be written");

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
    assert_eq!(json["network"], "testnet");
    assert!(
        json["data"]["path"]
            .as_str()
            .expect("artifact path should exist")
            .ends_with("dist/history/deploy.testnet.20260413T000000.000Z.json")
    );
    assert_eq!(json["data"]["summary"]["kind"], "history");
    assert_eq!(json["data"]["summary"]["contracts"]["count"], 1);
    assert_eq!(json["data"]["summary"]["tokens"]["count"], 1);
    assert_eq!(json["data"]["comparison"]["status"], "warn");
    assert!(
        json["data"]["comparison"]["issues"]
            .as_array()
            .expect("comparison issues should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|issue| issue.contains("contract `rewards`"))
    );
}

#[test]
fn release_drift_reports_current_history_and_expected_mismatch() {
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
  "contracts": {
    "rewards": {
      "contract_id": "CARCHIVED123",
      "alias": "rewards",
      "wasm_hash": "rollbackbeef"
    }
  },
  "tokens": {
    "points": {
      "kind": "asset",
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
        .args(["--json", "--dry-run", "release", "drift", "testnet"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "release.drift");
    assert_eq!(json["network"], "testnet");
    assert_eq!(json["status"], "warn");
    assert_eq!(json["data"]["expected"]["kind"], "expected");
    assert_eq!(json["data"]["current"]["kind"], "current");
    assert_eq!(json["data"]["latest_history"]["kind"], "history");
    assert!(
        find_check(&json, "release:testnet:history:drift")["detail"]
            .as_str()
            .expect("history drift detail should be present")
            .contains("contract `rewards`")
    );
}

#[test]
fn doctor_project_reports_scaffold_and_release_gaps() {
    let root = init_rewards_project();
    fs::remove_file(root.join("apps/api/src/routes/events.ts"))
        .expect("events route should be removable for the test");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "doctor", "project"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "doctor.project");
    assert_eq!(json["status"], "error");
    assert_eq!(find_check(&json, "api:events-route")["status"], "error");
    assert_eq!(
        find_check(&json, "release:testnet:contract:rewards")["status"],
        "warn"
    );
    assert_eq!(
        find_check(&json, "release:testnet:token:points")["status"],
        "warn"
    );
    assert_eq!(
        find_check(&json, "release:testnet:env-generated")["status"],
        "ok"
    );
}

#[test]
fn doctor_project_reports_contract_openapi_and_events_config_drift() {
    let root = init_rewards_project();
    fs::remove_file(root.join("contracts/rewards/src/lib.rs"))
        .expect("contract source should be removable for the test");
    fs::remove_file(root.join("apps/api/openapi.json"))
        .expect("openapi should be removable for the test");
    fs::remove_file(root.join("apps/web/index.html"))
        .expect("frontend index should be removable for the test");
    fs::remove_file(root.join("apps/web/scripts/ui-smoke.mjs"))
        .expect("frontend smoke runner should be removable for the test");
    fs::remove_file(root.join("apps/web/scripts/ui-browser-smoke.mjs"))
        .expect("frontend browser smoke runner should be removable for the test");
    fs::write(
        root.join("apps/api/.env"),
        "STELLAR_EVENTS_RESOURCES=contract:missing\n",
    )
    .expect("api env should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "doctor", "project"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "doctor.project");
    assert_eq!(json["status"], "error");
    assert_eq!(find_check(&json, "contract:rewards:src")["status"], "error");
    assert_eq!(find_check(&json, "api:openapi")["status"], "error");
    assert_eq!(find_check(&json, "frontend:index")["status"], "error");
    assert_eq!(
        find_check(&json, "frontend:ui-smoke-runner")["status"],
        "error"
    );
    assert_eq!(
        find_check(&json, "frontend:ui-browser-smoke-runner")["status"],
        "error"
    );
    assert_eq!(find_check(&json, "events:config")["status"], "warn");
    assert!(
        find_check(&json, "events:config")["detail"]
            .as_str()
            .expect("events config detail should be present")
            .contains("contract:missing")
    );
}

#[test]
fn doctor_fix_restores_managed_files_and_reexports_release_state() {
    let root = init_rewards_project();
    seed_testnet_release_lockfile(&root);
    fs::remove_file(root.join("scripts/doctor.mjs"))
        .expect("doctor script should be removable for the test");
    fs::remove_file(root.join("workers/events/ingest-events.mjs"))
        .expect("worker should be removable for the test");
    fs::remove_file(root.join("apps/api/openapi.json"))
        .expect("openapi should be removable for the test");
    fs::remove_file(root.join("apps/web/src/generated/stellar.ts"))
        .expect("generated state should be removable for the test");
    fs::write(root.join(".env.generated"), "BROKEN=1\n")
        .expect("env.generated should be writable for the test");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "doctor", "fix"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "doctor.fix");
    assert_eq!(json["status"], "ok");
    assert!(read(root.join("scripts/doctor.mjs")).contains("['doctor'"));
    assert!(read(root.join("workers/events/ingest-events.mjs")).contains("'events'"));
    assert!(read(root.join("apps/api/openapi.json")).contains("/contracts/rewards/call/{fn}"));
    assert!(read(root.join("apps/web/src/generated/stellar.ts")).contains("CREWARDS123"));
    let env_generated = read(root.join(".env.generated"));
    assert!(env_generated.contains("PUBLIC_REWARDS_CONTRACT_ID=CREWARDS123"));
    assert!(env_generated.contains("PUBLIC_POINTS_SAC_ID=CSAC123"));
    let repaired = json["data"]["repaired"]
        .as_array()
        .expect("repaired should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(repaired.contains(&"scripts"));
    assert!(repaired.contains(&"release_artifact"));
    assert!(root.join("dist/deploy.testnet.json").exists());
}

#[test]
fn doctor_env_reports_manifest_context() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "doctor", "env"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    let project_root = find_check(&json, "project_root")["detail"]
        .as_str()
        .expect("project_root detail should be present");
    assert_eq!(json["action"], "doctor.env");
    assert_eq!(json["status"], "ok");
    assert_eq!(
        Path::new(project_root)
            .canonicalize()
            .expect("project root should canonicalize"),
        root.canonicalize().expect("root should canonicalize")
    );
    assert_eq!(find_check(&json, "active_network")["detail"], "testnet");
    assert_eq!(find_check(&json, "active_identity")["detail"], "alice");
    assert_eq!(find_check(&json, "package_manager")["detail"], "pnpm");
    assert_eq!(find_check(&json, "api")["status"], "ok");
    assert_eq!(find_check(&json, "frontend")["status"], "ok");
}

#[test]
fn doctor_deps_treats_node_tooling_as_optional_without_api_or_frontend() {
    let temp = tempdir().expect("tempdir should be created");
    let root = temp.path().join("demo");
    fs::create_dir_all(&root).expect("project root should be created");
    fs::write(
        root.join("stellarforge.toml"),
        r#"[project]
name = "demo"
slug = "demo"
version = "0.1.0"
package_manager = "pnpm"

[defaults]
network = "testnet"
identity = "alice"

[contracts.app]
path = "contracts/app"
alias = "app"
template = "default"
bindings = ["typescript"]
deploy_on = ["testnet"]
"#,
    )
    .expect("manifest should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "--dry-run", "doctor", "deps"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "doctor.deps");
    assert_ne!(find_check(&json, "node")["status"], "error");
    assert_ne!(find_check(&json, "pnpm")["status"], "error");
    assert_eq!(
        find_check(&json, "node")["detail"],
        "optional until you enable API or frontend scaffolds"
    );
    assert_eq!(
        find_check(&json, "pnpm")["detail"],
        "optional until you enable API or frontend scaffolds"
    );
    assert_eq!(
        find_check(&json, "cargo")["detail"],
        "required because the project declares Rust contracts"
    );
    if stellar_available() {
        assert_eq!(find_check(&json, "plugin detection")["status"], "warn");
    }
    assert_ne!(find_check(&json, "registry tooling")["status"], "error");
    assert!(
        find_check(&json, "registry tooling")["detail"]
            .as_str()
            .expect("registry tooling detail should exist")
            .contains("release registry")
    );
}

#[test]
fn doctor_deps_warns_when_registry_artifact_exists_without_registry_tooling() {
    let root = init_rewards_project();
    fs::write(
        root.join("dist/registry.testnet.json"),
        r#"{
  "version": 1,
  "environment": "testnet",
  "contracts": {
    "rewards": {
      "wasm_name": "rewards-registry",
      "version": "1.2.3"
    }
  }
}"#,
    )
    .expect("registry artifact should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env_remove("STELLAR_FORGE_REGISTRY_MODE")
        .args(["--json", "--dry-run", "doctor", "deps"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "doctor.deps");
    assert_eq!(find_check(&json, "registry tooling")["status"], "warn");
    let detail = find_check(&json, "registry tooling")["detail"]
        .as_str()
        .expect("registry tooling detail should be present");
    assert!(detail.contains("registry metadata detected"));
    assert!(detail.contains("stellar-registry"));
}

#[test]
fn doctor_network_dry_run_plans_endpoint_probes() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "--dry-run", "doctor", "network", "testnet"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "doctor.network");
    assert_eq!(json["network"], "testnet");
    let commands = json["commands"]
        .as_array()
        .expect("commands should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(
        commands
            .iter()
            .any(|command| command.contains("POST https://soroban-testnet.stellar.org"))
    );
    assert!(
        commands
            .iter()
            .any(|command| command.contains("GET https://horizon-testnet.stellar.org"))
    );
    assert_eq!(find_check(&json, "rpc")["status"], "warn");
    assert_eq!(find_check(&json, "horizon")["status"], "warn");
    assert!(
        find_check(&json, "rpc")["detail"]
            .as_str()
            .expect("rpc detail should be present")
            .contains("skipped in --dry-run")
    );
    assert!(
        find_check(&json, "horizon")["detail"]
            .as_str()
            .expect("horizon detail should be present")
            .contains("skipped in --dry-run")
    );
}

#[test]
fn dev_status_dry_run_writes_report_to_out_path() {
    let root = init_rewards_project();
    let out_path = root.join("dist/dev.status.json");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "dev",
            "status",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "dev.status");
    assert_eq!(
        json["data"]["out"],
        out_path.to_str().expect("out path should be valid UTF-8")
    );
    let out_json: Value =
        serde_json::from_str(&read(&out_path)).expect("dev status out should parse");
    assert_eq!(out_json["action"], "dev.status");
}

#[test]
fn dev_reseed_dry_run_resets_event_state_and_exports_env() {
    let root = init_rewards_project();
    let out_path = root.join("dist/dev.reseed.json");
    fs::write(
        root.join("workers/events/cursors.json"),
        r#"{
  "cursors": {
    "testnet:contract:rewards": {
      "cursor": "ledger:55",
      "last_ledger": 55,
      "updated_at": "2026-04-14T00:00:00Z"
    },
    "local:contract:rewards": {
      "cursor": "ledger:12",
      "last_ledger": 12,
      "updated_at": "2026-04-14T00:00:00Z"
    }
  }
}"#,
    )
    .expect("event snapshot should be written");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "dev",
            "reseed",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "dev.reseed");
    assert_eq!(
        json["data"]["out"],
        out_path.to_str().expect("out path should be valid UTF-8")
    );
    assert_eq!(json["data"]["event_state_reset"], true);
    assert_eq!(json["data"]["env_exported"], true);
    assert_eq!(json["data"]["verification_ran"], false);
    let commands = json["commands"]
        .as_array()
        .expect("commands should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let artifacts = json["artifacts"]
        .as_array()
        .expect("artifacts should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(
        commands
            .iter()
            .any(|command| command.contains("stellar tx new change-trust"))
    );
    assert!(commands.iter().any(|command| {
        command.contains("stellar contract asset deploy") && command.contains("--alias points-sac")
    }));
    assert!(commands.iter().any(|command| {
        command.contains("stellar contract deploy") && command.contains("--alias rewards")
    }));
    assert!(commands.iter().any(|command| {
        command.contains("stellar contract invoke") && command.contains("init")
    }));
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact.ends_with(".env.generated"))
    );
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact.ends_with("workers/events/cursors.json"))
    );
    assert!(
        json["next"]
            .as_array()
            .expect("next should be an array")
            .iter()
            .any(|value| value.as_str() == Some("stellar forge release verify testnet"))
    );
    let out_json: Value =
        serde_json::from_str(&read(&out_path)).expect("dev reseed out should parse");
    assert_eq!(out_json["action"], "dev.reseed");
}

#[test]
fn contract_new_writes_report_to_out_path() {
    let root = init_rewards_project();
    let out_path = root.join("dist/contract.new.json");

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args([
            "--json",
            "contract",
            "new",
            "escrow",
            "--template",
            "escrow",
            "--out",
            out_path.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "contract.new");
    assert_eq!(
        json["data"]["out"],
        out_path.to_str().expect("out path should be valid UTF-8")
    );
    let out_json: Value =
        serde_json::from_str(&read(&out_path)).expect("contract new out should parse");
    assert_eq!(out_json["action"], "contract.new");
}

#[test]
fn contract_new_basic_without_stellar_generates_starter_contract_scaffold() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", "")
        .args(["--json", "contract", "new", "hello", "--template", "basic"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "contract.new");
    assert!(
        json["warnings"]
            .as_array()
            .expect("warnings should be an array")
            .iter()
            .any(|value| value
                .as_str()
                .is_some_and(|warning| warning.contains("local contract scaffold")))
    );

    let manifest = read(root.join("stellarforge.toml"));
    let cargo_toml = read(root.join("contracts/hello/Cargo.toml"));
    let lib_rs = read(root.join("contracts/hello/src/lib.rs"));
    let test_rs = read(root.join("contracts/hello/src/test.rs"));
    let readme = read(root.join("contracts/hello/README.md"));
    let openapi = read(root.join("apps/api/openapi.json"));
    let generated_state = read(root.join("apps/web/src/generated/stellar.ts"));

    assert!(manifest.contains("[contracts.hello]"));
    assert!(manifest.contains("template = \"basic\""));
    assert!(cargo_toml.contains("soroban-sdk = \"25\""));
    assert!(cargo_toml.contains("testutils"));
    assert!(lib_rs.contains("pub fn init"));
    assert!(lib_rs.contains("pub fn increment"));
    assert!(lib_rs.contains("pub fn set_count"));
    assert!(lib_rs.contains("mod test;"));
    assert!(test_rs.contains("starter_contract_tracks_admin_and_counter"));
    assert!(readme.contains("Starter Soroban contract scaffold"));
    assert!(openapi.contains("/contracts/hello/call/{fn}"));
    assert!(generated_state.contains("\"hello\""));
}

#[test]
fn contract_new_unknown_template_without_stellar_uses_starter_fallback() {
    let root = init_rewards_project();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", "")
        .args([
            "--json",
            "contract",
            "new",
            "mystery",
            "--template",
            "space-market",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "contract.new");
    assert!(
        json["warnings"]
            .as_array()
            .expect("warnings should be an array")
            .iter()
            .any(|value| value
                .as_str()
                .is_some_and(|warning| warning.contains("starter fallback scaffold")))
    );

    let lib_rs = read(root.join("contracts/mystery/src/lib.rs"));
    let test_rs = read(root.join("contracts/mystery/src/test.rs"));
    let readme = read(root.join("contracts/mystery/README.md"));

    assert!(lib_rs.contains("pub fn increment"));
    assert!(test_rs.contains("starter_contract_tracks_admin_and_counter"));
    assert!(readme.contains("template `space-market`"));
    assert!(!readme.contains("Scaffolded contract placeholder"));
}

#[test]
fn token_contract_command_surfaces_write_reports_to_out_paths() {
    let assert_report = |stdout: &[u8], action: &str, out_path: &Path| {
        let json: Value = serde_json::from_slice(stdout).expect("stdout should be valid json");
        assert_eq!(json["action"], action);
        assert_eq!(
            json["data"]["out"],
            out_path.to_str().expect("out path should be valid UTF-8")
        );
        let out_json: Value =
            serde_json::from_str(&read(out_path)).expect("out report should parse");
        assert_eq!(out_json["action"], action);
        assert_eq!(
            out_json["data"]["out"],
            out_path.to_str().expect("out path should be valid UTF-8")
        );
    };

    let create_root = init_rewards_project();
    let create_out = create_root.join("dist/token.create.json");
    let create_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&create_root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "token",
            "create",
            "credits",
            "--mode",
            "contract",
            "--metadata-name",
            "Store Credit",
            "--initial-supply",
            "25",
            "--out",
            create_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(create_output.status.success());
    assert_report(&create_output.stdout, "token.create", &create_out);

    let mint_root = init_contract_token_project();
    let mint_out = mint_root.join("dist/token.mint.json");
    let mint_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&mint_root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "token",
            "mint",
            "credits",
            "--to",
            "alice",
            "--amount",
            "10",
            "--from",
            "issuer",
            "--out",
            mint_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(mint_output.status.success());
    assert_report(&mint_output.stdout, "token.mint", &mint_out);

    let init_root = init_contract_token_project();
    let init_out = init_root.join("dist/token.contract.init.json");
    let init_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&init_root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "token",
            "contract",
            "init",
            "credits",
            "--out",
            init_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(init_output.status.success());
    assert_report(&init_output.stdout, "token.contract.init", &init_out);
}

#[test]
fn token_classic_command_surfaces_write_reports_to_out_paths() {
    let assert_report = |stdout: &[u8], action: &str, out_path: &Path| {
        let json: Value = serde_json::from_slice(stdout).expect("stdout should be valid json");
        assert_eq!(json["action"], action);
        assert_eq!(
            json["data"]["out"],
            out_path.to_str().expect("out path should be valid UTF-8")
        );
        let out_json: Value =
            serde_json::from_str(&read(out_path)).expect("out report should parse");
        assert_eq!(out_json["action"], action);
        assert_eq!(
            out_json["data"]["out"],
            out_path.to_str().expect("out path should be valid UTF-8")
        );
    };

    let transfer_root = init_rewards_project();
    let transfer_out = transfer_root.join("dist/token.transfer.json");
    let transfer_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&transfer_root)
        .args([
            "--json",
            "--dry-run",
            "token",
            "transfer",
            "points",
            "--to",
            "alice",
            "--amount",
            "10",
            "--from",
            "treasury",
            "--out",
            transfer_out
                .to_str()
                .expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(transfer_output.status.success());
    assert_report(&transfer_output.stdout, "wallet.pay", &transfer_out);

    let trust_root = init_rewards_project();
    let trust_out = trust_root.join("dist/token.trust.json");
    let trust_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&trust_root)
        .args([
            "--json",
            "--dry-run",
            "token",
            "trust",
            "points",
            "alice",
            "--out",
            trust_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(trust_output.status.success());
    assert_report(&trust_output.stdout, "wallet.trust", &trust_out);

    let freeze_root = init_rewards_project();
    let freeze_out = freeze_root.join("dist/token.freeze.json");
    let freeze_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&freeze_root)
        .args([
            "--json",
            "--dry-run",
            "token",
            "freeze",
            "points",
            "alice",
            "--out",
            freeze_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(freeze_output.status.success());
    assert_report(&freeze_output.stdout, "token.freeze", &freeze_out);

    let unfreeze_root = init_rewards_project();
    let unfreeze_out = unfreeze_root.join("dist/token.unfreeze.json");
    let unfreeze_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&unfreeze_root)
        .args([
            "--json",
            "--dry-run",
            "token",
            "unfreeze",
            "points",
            "alice",
            "--out",
            unfreeze_out
                .to_str()
                .expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(unfreeze_output.status.success());
    assert_report(&unfreeze_output.stdout, "token.unfreeze", &unfreeze_out);

    let unsupported_manifest = read(unfreeze_root.join("stellarforge.toml")).replace(
        "auth_required = true\nauth_revocable = true",
        "auth_required = false\nauth_revocable = false",
    );
    fs::write(
        unfreeze_root.join("stellarforge.toml"),
        unsupported_manifest,
    )
    .expect("manifest should be updated");
    let unsupported = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&unfreeze_root)
        .args(["--json", "--dry-run", "token", "freeze", "points", "alice"])
        .output()
        .expect("unsupported freeze command should run");

    assert!(!unsupported.status.success());
    assert_eq!(unsupported.status.code(), Some(2));
    let unsupported_json: Value =
        serde_json::from_slice(&unsupported.stdout).expect("stdout should be valid json");
    assert_eq!(unsupported_json["action"], "token.freeze");
    assert_eq!(unsupported_json["data"]["error_code"], "input");
    assert!(
        unsupported_json["message"]
            .as_str()
            .expect("message should be present")
            .contains("does not support freeze/unfreeze")
    );

    let burn_root = init_rewards_project();
    let burn_out = burn_root.join("dist/token.burn.json");
    let burn_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&burn_root)
        .args([
            "--json",
            "--dry-run",
            "token",
            "burn",
            "points",
            "--amount",
            "5",
            "--from",
            "treasury",
            "--out",
            burn_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(burn_output.status.success());
    assert_report(&burn_output.stdout, "token.burn", &burn_out);

    let clawback_root = init_rewards_project();
    let clawback_out = clawback_root.join("dist/token.clawback.json");
    let clawback_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&clawback_root)
        .args([
            "--json",
            "--dry-run",
            "token",
            "clawback",
            "points",
            "alice",
            "1",
            "--out",
            clawback_out
                .to_str()
                .expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(clawback_output.status.success());
    assert_report(&clawback_output.stdout, "token.clawback", &clawback_out);

    let sac_id_root = init_rewards_project();
    let sac_id_out = sac_id_root.join("dist/token.sac.id.json");
    let sac_id_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&sac_id_root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "token",
            "sac",
            "id",
            "points",
            "--out",
            sac_id_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(sac_id_output.status.success());
    assert_report(&sac_id_output.stdout, "token.sac.id", &sac_id_out);

    let sac_deploy_root = init_rewards_project();
    let sac_deploy_out = sac_deploy_root.join("dist/token.sac.deploy.json");
    let sac_deploy_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&sac_deploy_root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "token",
            "sac",
            "deploy",
            "points",
            "--out",
            sac_deploy_out
                .to_str()
                .expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(sac_deploy_output.status.success());
    assert_report(
        &sac_deploy_output.stdout,
        "token.sac.deploy",
        &sac_deploy_out,
    );
}

#[test]
fn wallet_command_surfaces_write_reports_to_out_paths() {
    let assert_report = |stdout: &[u8], action: &str, out_path: &Path| {
        let json: Value = serde_json::from_slice(stdout).expect("stdout should be valid json");
        assert_eq!(json["action"], action);
        assert_eq!(
            json["data"]["out"],
            out_path.to_str().expect("out path should be valid UTF-8")
        );
        let out_json: Value =
            serde_json::from_str(&read(out_path)).expect("out report should parse");
        assert_eq!(out_json["action"], action);
        assert_eq!(
            out_json["data"]["out"],
            out_path.to_str().expect("out path should be valid UTF-8")
        );
    };

    let create_root = init_rewards_project();
    let create_out = create_root.join("dist/wallet.create.json");
    let fake_bin = install_fake_stellar(&create_root);
    let create_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&create_root)
        .env("PATH", test_path(&fake_bin))
        .args([
            "--json",
            "wallet",
            "create",
            "bob",
            "--fund",
            "--out",
            create_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(create_output.status.success());
    assert_report(&create_output.stdout, "wallet.create", &create_out);

    let fund_root = init_rewards_project();
    let fund_out = fund_root.join("dist/wallet.fund.json");
    let fund_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&fund_root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "fund",
            "alice",
            "--out",
            fund_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(fund_output.status.success());
    assert_report(&fund_output.stdout, "wallet.fund", &fund_out);

    let trust_root = init_rewards_project();
    let trust_out = trust_root.join("dist/wallet.trust.json");
    let trust_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&trust_root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "trust",
            "alice",
            "points",
            "--out",
            trust_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(trust_output.status.success());
    assert_report(&trust_output.stdout, "wallet.trust", &trust_out);

    let pay_root = init_rewards_project();
    let pay_out = pay_root.join("dist/wallet.pay.json");
    let pay_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&pay_root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "pay",
            "--from",
            "treasury",
            "--to",
            "alice",
            "--asset",
            "points",
            "--amount",
            "10",
            "--out",
            pay_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(pay_output.status.success());
    assert_report(&pay_output.stdout, "wallet.pay", &pay_out);

    let sep7_payment_root = init_rewards_project();
    let sep7_payment_out = sep7_payment_root.join("dist/wallet.sep7.payment.json");
    let sep7_payment_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&sep7_payment_root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "sep7",
            "payment",
            "--from",
            "treasury",
            "--to",
            "alice",
            "--asset",
            "points",
            "--amount",
            "10",
            "--out",
            sep7_payment_out
                .to_str()
                .expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(sep7_payment_output.status.success());
    assert_report(&sep7_payment_output.stdout, "wallet.pay", &sep7_payment_out);

    let sep7_contract_root = init_rewards_project();
    seed_testnet_release_lockfile(&sep7_contract_root);
    let sep7_contract_out = sep7_contract_root.join("dist/wallet.sep7.contract-call.json");
    let sep7_contract_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&sep7_contract_root)
        .args([
            "--json",
            "--dry-run",
            "--network",
            "testnet",
            "wallet",
            "sep7",
            "contract-call",
            "rewards",
            "award_points",
            "--out",
            sep7_contract_out
                .to_str()
                .expect("out path should be valid UTF-8"),
            "--",
            "--member",
            "alice",
            "--amount",
            "25",
        ])
        .output()
        .expect("command should run");

    assert!(sep7_contract_output.status.success());
    assert_report(
        &sep7_contract_output.stdout,
        "wallet.sep7.contract-call",
        &sep7_contract_out,
    );
}

#[test]
fn wallet_smart_mutation_surfaces_write_reports_to_out_paths() {
    let assert_report = |stdout: &[u8], action: &str, out_path: &Path| {
        let json: Value = serde_json::from_slice(stdout).expect("stdout should be valid json");
        assert_eq!(json["action"], action);
        assert_eq!(
            json["data"]["out"],
            out_path.to_str().expect("out path should be valid UTF-8")
        );
        let out_json: Value =
            serde_json::from_str(&read(out_path)).expect("out report should parse");
        assert_eq!(out_json["action"], action);
        assert_eq!(
            out_json["data"]["out"],
            out_path.to_str().expect("out path should be valid UTF-8")
        );
    };

    let create_root = init_rewards_project();
    let create_fake_bin = install_fake_stellar(&create_root);
    let create_out = create_root.join("dist/wallet.smart.create.json");
    let create_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&create_root)
        .env("PATH", test_path(&create_fake_bin))
        .args([
            "--json",
            "wallet",
            "smart",
            "create",
            "sentinel",
            "--mode",
            "ed25519",
            "--out",
            create_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(create_output.status.success());
    assert_report(&create_output.stdout, "wallet.smart.create", &create_out);

    let scaffold_root = init_rewards_project();
    let scaffold_out = scaffold_root.join("dist/wallet.smart.scaffold.json");
    let scaffold_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&scaffold_root)
        .args([
            "--json",
            "wallet",
            "smart",
            "scaffold",
            "guardian",
            "--out",
            scaffold_out
                .to_str()
                .expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(scaffold_output.status.success());
    assert_report(
        &scaffold_output.stdout,
        "wallet.smart.scaffold",
        &scaffold_out,
    );

    let policy_root = init_rewards_project();
    let policy_fake_bin = install_fake_stellar(&policy_root);
    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&policy_root)
        .env("PATH", test_path(&policy_fake_bin))
        .args(["wallet", "smart", "create", "sentinel", "--mode", "ed25519"])
        .assert()
        .success();

    let daily_limit_out = policy_root.join("dist/wallet.smart.policy.set-daily-limit.json");
    let daily_limit_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&policy_root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "smart",
            "policy",
            "set-daily-limit",
            "sentinel",
            "1250",
            "--build-only",
            "--out",
            daily_limit_out
                .to_str()
                .expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(daily_limit_output.status.success());
    assert_report(
        &daily_limit_output.stdout,
        "wallet.smart.policy.set-daily-limit",
        &daily_limit_out,
    );

    let allow_out = policy_root.join("dist/wallet.smart.policy.allow.json");
    let allow_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&policy_root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "smart",
            "policy",
            "allow",
            "sentinel",
            "alice",
            "--build-only",
            "--out",
            allow_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(allow_output.status.success());
    assert_report(
        &allow_output.stdout,
        "wallet.smart.policy.allow",
        &allow_out,
    );

    let revoke_out = policy_root.join("dist/wallet.smart.policy.revoke.json");
    let revoke_output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&policy_root)
        .args([
            "--json",
            "--dry-run",
            "wallet",
            "smart",
            "policy",
            "revoke",
            "sentinel",
            "alice",
            "--build-only",
            "--out",
            revoke_out.to_str().expect("out path should be valid UTF-8"),
        ])
        .output()
        .expect("command should run");

    assert!(revoke_output.status.success());
    assert_report(
        &revoke_output.stdout,
        "wallet.smart.policy.revoke",
        &revoke_out,
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

fn init_minimal_contract_project() -> std::path::PathBuf {
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
        .args(["init", "demo", "--template", "minimal-contract", "--no-api"])
        .assert()
        .success();
    root
}

fn install_fake_stellar(root: &Path) -> std::path::PathBuf {
    let bin_dir = root.join(".test-bin");
    fs::create_dir_all(&bin_dir).expect("bin dir should be created");
    let script_path = bin_dir.join("stellar");
    fs::write(
        &script_path,
        r#"#!/bin/sh
if [ "$1" = "keys" ] && [ "$2" = "generate" ]; then
  echo "generated $3"
  exit 0
fi
if [ "$1" = "keys" ] && [ "$2" = "ls" ]; then
  printf "alice\nissuer\ntreasury\n"
  exit 0
fi
if [ "$1" = "keys" ] && [ "$2" = "public-key" ]; then
  case "$3" in
    alice|issuer|treasury)
      echo "GFAKEPUBLICKEY"
      exit 0
      ;;
  esac
  echo "missing key $3" >&2
  exit 1
fi
echo "unsupported fake stellar invocation: $@" >&2
exit 1
"#,
    )
    .expect("fake stellar should be written");
    #[cfg(unix)]
    fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))
        .expect("fake stellar should be executable");
    bin_dir
}

fn test_path(fake_bin: &Path) -> String {
    format!(
        "{}:{}",
        fake_bin.display(),
        std::env::var("PATH").expect("PATH should exist")
    )
}

fn init_contract_token_project() -> std::path::PathBuf {
    let temp = tempdir().expect("tempdir should be created");
    let kept = temp.keep();
    let root = kept.join("demo");
    fs::create_dir_all(root.join("contracts/credits/src"))
        .expect("contract source directory should be created");
    fs::write(
        root.join("contracts/credits/src/lib.rs"),
        "#![allow(dead_code)]\n",
    )
    .expect("contract source should be written");
    fs::write(
        root.join("stellarforge.toml"),
        r#"[project]
name = "demo"
slug = "demo"
version = "0.1.0"
package_manager = "pnpm"

[defaults]
network = "testnet"
identity = "alice"

[networks.testnet]
kind = "testnet"
rpc_url = "https://soroban-testnet.stellar.org"
horizon_url = "https://horizon-testnet.stellar.org"
network_passphrase = "Test SDF Network ; September 2015"
friendbot = true

[identities.issuer]
source = "stellar-cli"
name = "issuer"

[identities.treasury]
source = "stellar-cli"
name = "treasury"

[identities.alice]
source = "stellar-cli"
name = "alice"

[wallets.issuer]
kind = "classic"
identity = "issuer"

[wallets.treasury]
kind = "classic"
identity = "treasury"

[wallets.alice]
kind = "classic"
identity = "alice"

[tokens.credits]
kind = "contract"
code = "CREDIT"
issuer = "@identity:issuer"
distribution = "@identity:treasury"
decimals = 7
metadata_name = "Store Credit"

[contracts.credits]
path = "contracts/credits"
alias = "credits"
template = "openzeppelin-token"
bindings = ["typescript"]
deploy_on = ["testnet"]

[contracts.credits.init]
fn = "init"
admin = "@identity:issuer"
name = "Store Credit"
symbol = "CREDIT"
decimals = "7"
"#,
    )
    .expect("manifest should be written");
    fs::write(
        root.join("stellarforge.lock.json"),
        r#"{
  "version": 1,
  "environments": {
    "testnet": {
      "contracts": {
        "credits": {
          "contract_id": "CCREDIT123",
          "alias": "credits",
          "wasm_hash": "feedbeef",
          "tx_hash": "",
          "deployed_at": "2026-04-14T00:00:00Z"
        }
      },
      "tokens": {
        "credits": {
          "kind": "contract",
          "asset": "",
          "issuer_identity": "issuer",
          "distribution_identity": "treasury",
          "sac_contract_id": "",
          "contract_id": "CCREDIT123"
        }
      }
    }
  }
}"#,
    )
    .expect("lockfile should be written");
    root
}

fn init_scaffold_like_project() -> std::path::PathBuf {
    let temp = tempdir().expect("tempdir should be created");
    let kept = temp.keep();
    let root = kept.join("demo");
    fs::create_dir_all(root.join("contracts/hello/src"))
        .expect("contract source directory should be created");
    fs::create_dir_all(root.join("packages/hello-ts"))
        .expect("typescript bindings directory should be created");
    fs::create_dir_all(root.join("packages/hello-python"))
        .expect("python bindings directory should be created");
    fs::create_dir_all(root.join("src")).expect("root frontend source directory should be created");
    fs::write(
        root.join("contracts/hello/Cargo.toml"),
        r#"[package]
name = "hello"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]
"#,
    )
    .expect("contract Cargo.toml should be written");
    fs::write(
        root.join("contracts/hello/src/lib.rs"),
        "pub fn hello() {}\n",
    )
    .expect("contract source should be written");
    fs::write(root.join("package.json"), "{\"name\":\"demo\"}\n")
        .expect("package.json should be written");
    fs::write(root.join("package-lock.json"), "{}\n").expect("package-lock.json should be written");
    fs::write(root.join("src/main.tsx"), "console.log('hello');\n")
        .expect("root frontend entry should be written");
    fs::write(
        root.join("environments.toml"),
        r#"[testnet]
rpc_url = "https://rpc.example"
horizon_url = "https://horizon.example"
network_passphrase = "Test SDF Network ; September 2015"
friendbot = true

[testnet.contracts.hello]
contract_id = "CHELLO123"
alias = "hello-test"
wasm_hash = "beef"

[local]
rpc_url = "http://localhost:8000/rpc"
horizon_url = "http://localhost:8000"
network_passphrase = "Standalone Network ; February 2017"
allow_http = true

[local.contracts.hello]
id = "CLOCAL123"
"#,
    )
    .expect("environments.toml should be written");
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
    fs::read_to_string(path).expect("file should exist")
}

fn install_fake_pnpm(root: &Path) -> PathBuf {
    let bin_dir = root.join(".test-bin-pnpm");
    fs::create_dir_all(&bin_dir).expect("pnpm bin dir should be created");
    let script_path = bin_dir.join("pnpm");
    fs::write(
        &script_path,
        r#"#!/bin/sh
exit 0
"#,
    )
    .expect("fake pnpm should be written");
    #[cfg(unix)]
    fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))
        .expect("fake pnpm should be executable");
    bin_dir
}

fn install_recording_fake_commands(root: &Path, names: &[&str]) -> PathBuf {
    let bin_dir = root.join(".test-bin-recording");
    fs::create_dir_all(&bin_dir).expect("recording bin dir should be created");
    for name in names {
        let script = format!(
            "#!/bin/sh\nif [ -n \"$FAKE_LOG_PATH\" ]; then\n  printf '%s|%s|%s\\n' '{name}' \"$PWD\" \"$*\" >> \"$FAKE_LOG_PATH\"\nfi\nexit 0\n"
        );
        let script_path = bin_dir.join(name);
        fs::write(&script_path, &script).expect("recording fake command should be written");
        #[cfg(unix)]
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))
            .expect("recording fake command should be executable");

        if *name == "cargo" {
            let cargo_home_bin = root.join(".cargo").join("bin");
            fs::create_dir_all(&cargo_home_bin).expect("home cargo bin dir should be created");
            let cargo_home_script = cargo_home_bin.join(name);
            fs::write(&cargo_home_script, &script)
                .expect("home cargo fake command should be written");
            #[cfg(unix)]
            fs::set_permissions(&cargo_home_script, fs::Permissions::from_mode(0o755))
                .expect("home cargo fake command should be executable");
        }
    }
    bin_dir
}

fn read_invocation_log(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .expect("invocation log should be readable")
        .lines()
        .map(str::to_string)
        .collect()
}

fn stellar_available() -> bool {
    Command::new("stellar")
        .arg("version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn sqlite_available() -> bool {
    Command::new("sqlite3")
        .arg("-version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn seed_sqlite_cursor(
    root: &Path,
    name: &str,
    resource_kind: &str,
    resource_name: &str,
    cursor: Option<&str>,
    last_ledger: Option<i64>,
) {
    let db_path = root.join("apps/api/db/events.sqlite");
    let schema = read(root.join("apps/api/db/schema.sql"));
    let cursor_sql = cursor
        .map(|cursor| format!("'{}'", cursor))
        .unwrap_or_else(|| "null".to_string());
    let ledger_sql = last_ledger
        .map(|ledger| ledger.to_string())
        .unwrap_or_else(|| "null".to_string());
    let sql = format!(
        "{schema}
insert into cursors (name, resource_kind, resource_name, cursor, last_ledger, updated_at)
values ('{name}', '{resource_kind}', '{resource_name}', {cursor_sql}, {ledger_sql}, '2026-04-14T00:00:00Z');
"
    );
    let output = Command::new("sqlite3")
        .current_dir(root)
        .arg(db_path)
        .arg(sql)
        .output()
        .expect("sqlite3 should run");
    assert!(
        output.status.success(),
        "sqlite3 seed should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn find_check<'a>(json: &'a Value, name: &str) -> &'a Value {
    json["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .find(|entry| entry["name"] == name)
        .unwrap_or_else(|| panic!("missing check `{name}`"))
}
