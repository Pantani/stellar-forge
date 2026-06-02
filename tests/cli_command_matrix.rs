use std::fs;

mod support;

use support::{
    append_manifest, cargo_cli, init_rewards_project, run_cli_json, run_cli_json_with_path,
};

const COMMAND_HELP_PATHS: &[&[&str]] = &[
    &[],
    &["init"],
    &["project"],
    &["project", "info"],
    &["project", "sync"],
    &["project", "validate"],
    &["project", "smoke"],
    &["project", "add"],
    &["project", "add", "contract"],
    &["project", "add", "api"],
    &["project", "add", "frontend"],
    &["project", "adopt"],
    &["project", "adopt", "scaffold"],
    &["dev"],
    &["dev", "up"],
    &["dev", "down"],
    &["dev", "status"],
    &["dev", "reset"],
    &["dev", "reseed"],
    &["dev", "snapshot"],
    &["dev", "snapshot", "save"],
    &["dev", "snapshot", "load"],
    &["dev", "fund"],
    &["dev", "watch"],
    &["dev", "events"],
    &["dev", "logs"],
    &["scenario"],
    &["scenario", "run"],
    &["scenario", "test"],
    &["contract"],
    &["contract", "new"],
    &["contract", "build"],
    &["contract", "format"],
    &["contract", "lint"],
    &["contract", "deploy"],
    &["contract", "call"],
    &["contract", "bind"],
    &["contract", "info"],
    &["contract", "fetch"],
    &["contract", "ttl"],
    &["contract", "ttl", "extend"],
    &["contract", "ttl", "restore"],
    &["contract", "spec"],
    &["token"],
    &["token", "create"],
    &["token", "info"],
    &["token", "airdrop"],
    &["token", "airdrop-reconcile"],
    &["token", "airdrop-resume"],
    &["token", "airdrop-report"],
    &["token", "airdrop-validate"],
    &["token", "airdrop-preview"],
    &["token", "airdrop-summary"],
    &["token", "mint"],
    &["token", "burn"],
    &["token", "transfer"],
    &["token", "trust"],
    &["token", "freeze"],
    &["token", "unfreeze"],
    &["token", "clawback"],
    &["token", "sac"],
    &["token", "sac", "id"],
    &["token", "sac", "deploy"],
    &["token", "contract"],
    &["token", "contract", "init"],
    &["token", "balance"],
    &["wallet"],
    &["wallet", "create"],
    &["wallet", "ls"],
    &["wallet", "address"],
    &["wallet", "fund"],
    &["wallet", "balances"],
    &["wallet", "trust"],
    &["wallet", "pay"],
    &["wallet", "batch-pay"],
    &["wallet", "batch-reconcile"],
    &["wallet", "batch-resume"],
    &["wallet", "batch-report"],
    &["wallet", "batch-validate"],
    &["wallet", "batch-preview"],
    &["wallet", "batch-summary"],
    &["wallet", "receive"],
    &["wallet", "sep7"],
    &["wallet", "sep7", "payment"],
    &["wallet", "sep7", "contract-call"],
    &["wallet", "smart"],
    &["wallet", "smart", "create"],
    &["wallet", "smart", "scaffold"],
    &["wallet", "smart", "info"],
    &["wallet", "smart", "onboard"],
    &["wallet", "smart", "provision"],
    &["wallet", "smart", "materialize"],
    &["wallet", "smart", "controller"],
    &["wallet", "smart", "controller", "rotate"],
    &["wallet", "smart", "policy"],
    &["wallet", "smart", "policy", "info"],
    &["wallet", "smart", "policy", "diff"],
    &["wallet", "smart", "policy", "sync"],
    &["wallet", "smart", "policy", "simulate"],
    &["wallet", "smart", "policy", "apply"],
    &["wallet", "smart", "policy", "set-daily-limit"],
    &["wallet", "smart", "policy", "allow"],
    &["wallet", "smart", "policy", "revoke"],
    &["api"],
    &["api", "init"],
    &["api", "generate"],
    &["api", "generate", "contract"],
    &["api", "generate", "token"],
    &["api", "openapi"],
    &["api", "openapi", "export"],
    &["api", "events"],
    &["api", "events", "init"],
    &["api", "relayer"],
    &["api", "relayer", "init"],
    &["events"],
    &["events", "status"],
    &["events", "export"],
    &["events", "replay"],
    &["events", "watch"],
    &["events", "ingest"],
    &["events", "ingest", "init"],
    &["events", "cursor"],
    &["events", "cursor", "ls"],
    &["events", "cursor", "reset"],
    &["events", "backfill"],
    &["release"],
    &["release", "plan"],
    &["release", "deploy"],
    &["release", "verify"],
    &["release", "status"],
    &["release", "drift"],
    &["release", "diff"],
    &["release", "history"],
    &["release", "inspect"],
    &["release", "rollback"],
    &["release", "prune"],
    &["release", "aliases"],
    &["release", "aliases", "sync"],
    &["release", "env"],
    &["release", "env", "export"],
    &["release", "registry"],
    &["release", "registry", "publish"],
    &["release", "registry", "deploy"],
    &["doctor"],
    &["doctor", "env"],
    &["doctor", "deps"],
    &["doctor", "audit"],
    &["doctor", "fix"],
    &["doctor", "network"],
    &["doctor", "project"],
];

#[test]
fn help_matrix_reaches_every_public_command_path() {
    for path in COMMAND_HELP_PATHS {
        let output = cargo_cli()
            .args(*path)
            .arg("--help")
            .output()
            .unwrap_or_else(|error| panic!("help command should run for {path:?}: {error}"));

        assert!(
            output.status.success(),
            "help failed for `{}`\nstdout:\n{}\nstderr:\n{}",
            command_label(path),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("Usage:"),
            "help output should include Usage for `{}`",
            command_label(path)
        );
    }
}

#[test]
fn init_command_builds_workspace_from_compiled_binary() {
    let parent = tempfile::tempdir().expect("tempdir should be created");

    let report = run_cli_json(
        parent.path(),
        &[
            "--json",
            "init",
            "matrix-demo",
            "--template",
            "minimal-contract",
            "--no-api",
        ],
    );

    assert_eq!(report["action"], "init");
    assert_eq!(report["status"], "ok");
    let root = parent.path().join("matrix-demo");
    assert!(root.join("stellarforge.toml").exists());
    assert!(root.join("stellarforge.lock.json").exists());
    assert!(root.join("contracts/app/src/lib.rs").exists());
    assert!(!root.join("apps/api/package.json").exists());
}

#[test]
fn offline_command_matrix_executes_representative_json_reports() {
    let root = init_rewards_project();
    let clean_cases = [
        CommandCase::new(
            "project info",
            &["--json", "project", "info"],
            "project.info",
        ),
        CommandCase::new(
            "project validate",
            &["--json", "project", "validate"],
            "project.validate",
        ),
    ];

    for case in clean_cases {
        let report = run_cli_json(&root, case.args);
        assert_eq!(report["action"], case.expected_action, "{}", case.name);
        assert_ne!(report["status"], "error", "{}", case.name);
    }

    append_checkout_scenario(&root);

    let cases = [
        CommandCase::new(
            "dev events",
            &["--json", "dev", "events", "rewards"],
            "dev.events",
        ),
        CommandCase::new(
            "scenario test",
            &["--json", "scenario", "test", "checkout"],
            "scenario.test",
        ),
        CommandCase::new(
            "contract info",
            &["--json", "contract", "info", "rewards"],
            "contract.info",
        ),
        CommandCase::new(
            "token info",
            &["--json", "token", "info", "points"],
            "token.info",
        ),
        CommandCase::new(
            "wallet receive",
            &["--json", "wallet", "receive", "alice", "--sep7"],
            "wallet.receive",
        ),
        CommandCase::new(
            "api openapi export",
            &["--json", "api", "openapi", "export"],
            "api.openapi.export",
        ),
        CommandCase::new(
            "events cursor ls",
            &["--json", "events", "cursor", "ls"],
            "events.cursor.ls",
        ),
        CommandCase::new(
            "release plan",
            &["--json", "--dry-run", "release", "plan", "testnet"],
            "release.plan",
        ),
        CommandCase::new("doctor env", &["--json", "doctor", "env"], "doctor.env"),
    ];

    for case in cases {
        let report = run_cli_json(&root, case.args);
        assert_eq!(report["action"], case.expected_action, "{}", case.name);
        assert_ne!(report["status"], "error", "{}", case.name);
    }
}

#[test]
fn fake_stellar_boundary_executes_external_cli_commands_offline() {
    let root = init_rewards_project();
    let fake_bin = support::install_fake_stellar(&root);

    let report = run_cli_json_with_path(&root, &["--json", "wallet", "ls"], &fake_bin);

    assert_eq!(report["action"], "wallet.ls");
    assert_eq!(report["status"], "ok");
}

#[test]
fn out_report_matrix_persists_json_artifacts_for_multiple_command_families() {
    let root = init_rewards_project();
    let cases = [
        OutCase::new(
            &[
                "--json",
                "project",
                "validate",
                "--out",
                "dist/matrix/project.validate.json",
            ],
            "project.validate",
            "dist/matrix/project.validate.json",
        ),
        OutCase::new(
            &[
                "--json",
                "api",
                "openapi",
                "export",
                "--out",
                "dist/matrix/api.openapi.json",
            ],
            "api.openapi.export",
            "dist/matrix/api.openapi.json",
        ),
        OutCase::new(
            &[
                "--json",
                "--dry-run",
                "release",
                "plan",
                "testnet",
                "--out",
                "dist/matrix/release.plan.json",
            ],
            "release.plan",
            "dist/matrix/release.plan.json",
        ),
        OutCase::new(
            &[
                "--json",
                "doctor",
                "env",
                "--out",
                "dist/matrix/doctor.env.json",
            ],
            "doctor.env",
            "dist/matrix/doctor.env.json",
        ),
    ];

    for case in cases {
        let stdout_report = run_cli_json(&root, case.args);
        assert_eq!(stdout_report["action"], case.expected_action);

        let out_path = root.join(case.out_path);
        let file_report: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&out_path).unwrap_or_else(|error| {
                panic!("{} should be readable: {error}", out_path.display())
            }))
            .unwrap_or_else(|error| panic!("{} should parse as json: {error}", out_path.display()));
        assert_eq!(file_report["action"], case.expected_action);
        assert_eq!(file_report["status"], stdout_report["status"]);
    }
}

struct CommandCase {
    name: &'static str,
    args: &'static [&'static str],
    expected_action: &'static str,
}

impl CommandCase {
    const fn new(
        name: &'static str,
        args: &'static [&'static str],
        expected_action: &'static str,
    ) -> Self {
        Self {
            name,
            args,
            expected_action,
        }
    }
}

struct OutCase {
    args: &'static [&'static str],
    expected_action: &'static str,
    out_path: &'static str,
}

impl OutCase {
    const fn new(
        args: &'static [&'static str],
        expected_action: &'static str,
        out_path: &'static str,
    ) -> Self {
        Self {
            args,
            expected_action,
            out_path,
        }
    }
}

fn append_checkout_scenario(root: &std::path::Path) {
    append_manifest(
        root,
        r#"

[scenarios.checkout]
description = "Offline command matrix checkout"
network = "testnet"
identity = "alice"

[[scenarios.checkout.steps]]
action = "release.plan"
env = "testnet"

[[scenarios.checkout.assertions]]
assertion = "status"
status = "ok"

[[scenarios.checkout.assertions]]
assertion = "step"
step = 1
status = "ok"
command_contains = ["stellar contract deploy"]
"#,
    );
}

fn command_label(path: &[&str]) -> String {
    if path.is_empty() {
        "stellar-forge".to_string()
    } else {
        format!("stellar-forge {}", path.join(" "))
    }
}
