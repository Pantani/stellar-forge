use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn planned_commands_are_documented_in_readme_and_command_reference() {
    let root = workspace_root();
    let readme = read(root.join("README.md"));
    let reference = read(root.join("docs/command-reference.md"));
    let combined = format!("{readme}\n{reference}");

    for expected in [
        "docs/command-reference.md",
        "docs/manifest-reference.md",
        "docs/deployment-guide.md",
        "stellar forge project add contract escrow --template escrow --out dist/project.add.contract.json",
        "stellar forge dev snapshot save baseline --out dist/dev.snapshot.save.json",
        "stellar forge scenario test checkout --out dist/scenario.test.json",
        "stellar forge contract format rewards --check --out dist/contract.format.json",
        "stellar forge contract lint rewards --out dist/contract.lint.json",
        "stellar forge wallet smart provision checkout-passkey --address CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF --out dist/wallet.smart.provision.json",
        "stellar forge wallet smart policy sync checkout-passkey --out dist/wallet.smart.policy.sync.json",
        "stellar forge wallet smart policy diff checkout-passkey --out dist/wallet.smart.policy.diff.json",
        "stellar forge events backfill contract:rewards --count 200 --out dist/events.backfill.json",
        "stellar forge token airdrop-reconcile points --from treasury --file rewards.csv --format csv --report dist/airdrop.report.json --out dist/airdrop.reconcile.json",
        "stellar forge release drift testnet --out dist/release.drift.json",
        "stellar forge doctor fix --scope release",
    ] {
        assert!(
            combined.contains(expected),
            "expected documentation to mention `{expected}`"
        );
    }

    for expected in [
        "### `wallet smart provision`",
        "### `wallet smart policy sync`",
        "### `wallet smart policy diff`",
        "### `project info --out`",
        "### `project validate --out`",
        "### `project adopt scaffold --out`",
        "### `contract new --out`",
        "### `contract build --out`",
        "### `contract format`",
        "### `contract lint`",
        "### `contract deploy --out`",
        "### `contract call --out`",
        "### `contract bind --out`",
        "### `contract fetch`",
        "### `contract spec --out`",
        "### `contract ttl extend|restore --out`",
        "### `token create --out`",
        "### `token mint|burn|transfer --out`",
        "### `token trust|freeze|unfreeze|clawback --out`",
        "### `token sac id|deploy --out`",
        "### `token contract init --out`",
        "### `token balance --out`",
        "### `wallet create --out`",
        "### `wallet fund --out`",
        "### `wallet trust --out`",
        "### `wallet pay --out`",
        "### `wallet sep7 payment --out`",
        "### `wallet sep7 contract-call --out`",
        "### `wallet ls --out`",
        "### `wallet address --out`",
        "### `wallet balances --out`",
        "### `wallet receive --out`",
        "### `project sync --out`",
        "### `wallet smart create --out`",
        "### `wallet smart scaffold --out`",
        "### `wallet smart onboard --out`",
        "### `wallet smart policy info --out`",
        "### `wallet smart policy set-daily-limit|allow|revoke --out`",
        "### `wallet smart policy sync --out`",
        "### `wallet smart policy diff --out`",
        "### `project add contract --out`",
        "### `project add api --out`",
        "### `project add frontend --out`",
        "### `dev up`",
        "### `dev down`",
        "### `dev reset`",
        "### `dev reseed`",
        "### `dev snapshot save|load`",
        "### `dev fund <target>`",
        "### `dev events`",
        "### `dev watch`",
        "### `dev logs`",
        "### `scenario run`",
        "### `scenario test`",
        "### `api init --out`",
        "### `api generate contract --out`",
        "### `api generate token --out`",
        "### `api openapi export --out`",
        "### `api events init --out`",
        "### `api relayer init --out`",
        "### `dev up --out`",
        "### `dev down --out`",
        "### `dev reset --out`",
        "### `dev reseed --out`",
        "### `dev fund <target> --out`",
        "### `dev watch --out`",
        "### `dev events --out`",
        "### `dev logs --out`",
        "### `dev status --out`",
        "### `contract info --out`",
        "### `token info --out`",
        "### `wallet smart info --out`",
        "### `release drift <env>`",
        "### `release deploy <env> --out`",
        "### `wallet batch-resume`",
        "### `wallet batch-reconcile`",
        "### `wallet batch-report`",
        "### `token airdrop-reconcile`",
        "### `token airdrop-report`",
        "### `events watch --out`",
        "### `events ingest init --out`",
        "### `events status`",
        "### `events status --out`",
        "### `events cursor ls --out`",
        "### `events cursor reset --out`",
        "## Additional command surfaces",
        "### `wallet smart policy apply`",
        "### `project info --out`",
        "### `project validate --out`",
        "### `project adopt scaffold --out`",
        "### `contract new --out`",
        "### `contract build --out`",
        "### `contract format --out`",
        "### `contract lint --out`",
        "### `contract deploy --out`",
        "### `contract call --out`",
        "### `contract bind --out`",
        "### `contract fetch`",
        "### `contract spec --out`",
        "### `contract ttl extend|restore --out`",
        "### `token create --out`",
        "### `token mint|burn|transfer --out`",
        "### `token trust|freeze|unfreeze|clawback --out`",
        "### `token sac id|deploy --out`",
        "### `token contract init --out`",
        "### `token balance --out`",
        "### `wallet create --out`",
        "### `wallet fund --out`",
        "### `wallet trust --out`",
        "### `wallet pay --out`",
        "### `wallet sep7 payment --out`",
        "### `wallet sep7 contract-call --out`",
        "### `wallet ls --out`",
        "### `wallet address --out`",
        "### `wallet balances --out`",
        "### `wallet receive --out`",
        "### `project sync --out`",
        "### `wallet smart create --out`",
        "### `wallet smart scaffold --out`",
        "### `wallet smart onboard --out`",
        "### `wallet smart policy info --out`",
        "### `wallet smart policy set-daily-limit|allow|revoke --out`",
        "### `wallet smart policy simulate`",
        "### `wallet smart policy sync --out`",
        "### `wallet smart policy diff --out`",
        "### `dev snapshot save|load --out`",
        "### `scenario run|test --out`",
        "### `project add contract --out`",
        "### `project add api --out`",
        "### `project add frontend --out`",
        "### `dev up --out`",
        "### `dev down --out`",
        "### `dev reset --out`",
        "### `dev reseed --out`",
        "### `dev fund <target> --out`",
        "### `dev watch --out`",
        "### `dev events --out`",
        "### `dev logs --out`",
        "### `api init --out`",
        "### `api generate contract --out`",
        "### `api generate token --out`",
        "### `api openapi export --out`",
        "### `api events init --out`",
        "### `api relayer init --out`",
        "### `dev status --out`",
        "### `contract info --out`",
        "### `token info --out`",
        "### `wallet smart info --out`",
        "### `wallet batch-pay --out`",
        "### `wallet batch-validate|batch-preview|batch-summary --out`",
        "### `wallet batch-reconcile`",
        "### `wallet batch-report --out`",
        "### `wallet batch-reconcile --out`",
        "### `wallet batch-resume --out`",
        "### `token airdrop --out`",
        "### `token airdrop-validate|airdrop-preview|airdrop-summary --out`",
        "### `token airdrop-report --out`",
        "### `token airdrop-reconcile --out`",
        "### `token airdrop-resume`",
        "### `token airdrop-resume --out`",
        "### `release plan <env>`",
        "### `release deploy <env> --out`",
        "### `release verify <env>`",
        "### `release aliases sync <env>`",
        "### `release env export <env>`",
        "### `release registry publish <contract> --out`",
        "### `release registry deploy <contract> --out`",
        "### `release prune <env> --out`",
        "### `events export`",
        "### `events watch --out`",
        "### `events ingest init --out`",
        "### `events cursor ls --out`",
        "### `events cursor reset --out`",
        "### `events replay`",
        "### `doctor audit`",
        "### `doctor --out`",
        "### `doctor env`",
        "### `doctor deps`",
        "### `doctor project`",
        "### `doctor network <env>`",
        "### `doctor fix --scope`",
        "--address CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
        "--fund",
    ] {
        assert!(
            reference.contains(expected),
            "expected command reference to mention `{expected}`"
        );
    }

    assert!(
        !reference.contains("The next pass is expected to make `--path` optional"),
        "expected stale events replay note to be removed"
    );
}

#[test]
fn readme_is_oriented_around_project_and_docs() {
    let root = workspace_root();
    let readme = read(root.join("README.md"));

    for expected in [
        "Rust CLI for manifest-driven Stellar workspaces",
        "## Documentation",
        "docs/command-reference.md",
        "docs/manifest-reference.md",
        "docs/deployment-guide.md",
        "## Quick Start",
        "stellar forge init hello-stellar --template fullstack --network testnet",
        "stellar forge doctor",
        "stellar forge project validate",
        "stellar forge dev up",
        "stellar forge --dry-run release plan testnet",
        "## Core Files",
        "`stellarforge.toml`",
        "`stellarforge.lock.json`",
        "## Generated Workspace Layout",
        "## How To Work With This Repository",
        "CONTRIBUTING.md",
        "SUPPORT.md",
        "SECURITY.md",
    ] {
        assert!(
            readme.contains(expected),
            "expected README to surface `{expected}`"
        );
    }
}

#[test]
fn planned_command_contract_spells_out_expected_json_shape() {
    let root = init_rewards_project();
    let fake_bin = install_fake_stellar(&root);

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args([
            "wallet",
            "smart",
            "create",
            "checkout-passkey",
            "--mode",
            "ed25519",
        ])
        .assert()
        .success();

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args(["wallet", "create", "checkout-ops"])
        .assert()
        .success();

    let provision = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args([
            "--json",
            "wallet",
            "smart",
            "provision",
            "checkout-passkey",
            "--address",
            "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            "--fund",
        ])
        .output()
        .expect("provision command should run");

    assert!(provision.status.success());
    let provision_json: Value =
        serde_json::from_slice(&provision.stdout).expect("provision stdout should be valid json");
    assert_eq!(provision_json["action"], "wallet.smart.provision");
    assert_eq!(provision_json["data"]["wallet"], "checkout-passkey");
    assert_eq!(
        provision_json["data"]["contract_id"],
        "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"
    );
    assert_eq!(provision_json["data"]["controller_funded"], true);

    seed_policy_deployment(&root);

    let sync = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args([
            "--json",
            "wallet",
            "smart",
            "policy",
            "sync",
            "checkout-passkey",
        ])
        .output()
        .expect("sync command should run");
    assert!(sync.status.success());
    let sync_json: Value =
        serde_json::from_slice(&sync.stdout).expect("sync stdout should be valid json");
    assert_eq!(sync_json["action"], "wallet.smart.policy.sync");
    assert_eq!(sync_json["data"]["wallet"], "checkout-passkey");
    assert_eq!(sync_json["data"]["controller_identity"], "checkout-ops");
    assert_eq!(sync_json["data"]["synced"], true);

    let diff = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args([
            "--json",
            "wallet",
            "smart",
            "policy",
            "diff",
            "checkout-passkey",
        ])
        .output()
        .expect("diff command should run");
    assert!(diff.status.success());
    let diff_json: Value =
        serde_json::from_slice(&diff.stdout).expect("diff stdout should be valid json");
    assert_eq!(diff_json["action"], "wallet.smart.policy.diff");
    assert_eq!(diff_json["data"]["wallet"]["name"], "checkout-passkey");
    assert_eq!(
        diff_json["data"]["observed"]["admin_address"],
        "GCHECKOUTOPS"
    );
    assert_eq!(diff_json["data"]["observed"]["daily_limit"], "1250");

    let drift = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .args(["--json", "--dry-run", "release", "drift", "testnet"])
        .output()
        .expect("drift command should run");
    assert!(drift.status.success());
    let drift_json: Value =
        serde_json::from_slice(&drift.stdout).expect("drift stdout should be valid json");
    assert_eq!(drift_json["action"], "release.drift");
    assert_eq!(drift_json["network"], "testnet");
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: PathBuf) -> String {
    fs::read_to_string(path).expect("documentation should be readable")
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
        .args(["init", "demo", "--template", "rewards-loyalty"])
        .assert()
        .success();
    root
}

fn install_fake_stellar(root: &std::path::Path) -> PathBuf {
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
if [ "$1" = "keys" ] && [ "$2" = "public-key" ]; then
  case "$3" in
    alice) echo "GALICEPUBLIC" ; exit 0 ;;
    issuer) echo "GISSUERPUBLIC" ; exit 0 ;;
    treasury) echo "GTREASURYPUBLIC" ; exit 0 ;;
    checkout-passkey-owner) echo "GCHECKOUTOWNER" ; exit 0 ;;
    checkout-ops) echo "GCHECKOUTOPS" ; exit 0 ;;
  esac
  echo "missing key $3" >&2
  exit 1
fi
if [ "$1" = "contract" ] && [ "$2" = "invoke" ]; then
  case " $@ " in
    *" admin "*) echo "GCHECKOUTOPS" ; exit 0 ;;
    *" daily_limit "*) echo "1250" ; exit 0 ;;
  esac
  echo "invoked" ; exit 0
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

fn test_path(fake_bin: &std::path::Path) -> String {
    format!(
        "{}:{}",
        fake_bin.display(),
        std::env::var("PATH").expect("PATH should exist")
    )
}

fn seed_policy_deployment(root: &std::path::Path) {
    fs::write(
        root.join("stellarforge.lock.json"),
        r#"{
  "version": 1,
  "environments": {
    "testnet": {
      "contracts": {
        "checkout-passkey-policy": {
          "contract_id": "CPOLICY123",
          "alias": "checkout-passkey-policy",
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
}
