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

    for expected in [
        "New Commands",
        "stellar forge wallet smart provision checkout-passkey --address CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
        "stellar forge wallet smart provision checkout-passkey --fund",
        "stellar forge wallet smart policy sync checkout-passkey",
        "stellar forge wallet smart policy diff checkout-passkey",
        "stellar forge release drift testnet",
        "stellar forge project info --out dist/project.info.json",
        "stellar forge project validate --out dist/project.validate.json",
        "stellar forge project sync --out dist/project.sync.json",
        "stellar forge project adopt scaffold --out dist/project.adopt.json",
        "stellar forge project add contract escrow --template escrow --out dist/project.add.contract.json",
        "stellar forge project add api --out dist/project.add.api.json",
        "stellar forge project add frontend --framework react-vite --out dist/project.add.frontend.json",
        "stellar forge dev up",
        "stellar forge dev reset",
        "stellar forge dev fund alice",
        "stellar forge dev watch --once",
        "stellar forge dev events rewards",
        "stellar forge dev logs",
        "stellar forge dev up --out dist/dev.up.json",
        "stellar forge dev down --out dist/dev.down.json",
        "stellar forge dev reset --out dist/dev.reset.json",
        "stellar forge dev reseed --out dist/dev.reseed.json",
        "stellar forge dev snapshot save baseline --out dist/dev.snapshot.save.json",
        "stellar forge dev snapshot load baseline --out dist/dev.snapshot.load.json",
        "stellar forge --network testnet --dry-run dev fund GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF --out dist/dev.fund.json",
        "stellar forge --dry-run --network local dev watch --once --out dist/dev.watch.json",
        "stellar forge dev events rewards --out dist/dev.events.json",
        "stellar forge --dry-run dev logs --out dist/dev.logs.json",
        "stellar forge dev status --out dist/dev.status.json",
        "stellar forge scenario run checkout --out dist/scenario.run.json",
        "stellar forge scenario test checkout --out dist/scenario.test.json",
        "stellar forge contract new escrow --template escrow --out dist/contract.new.json",
        "stellar forge contract build rewards --out dist/contract.build.json",
        "stellar forge contract format rewards --check --out dist/contract.format.json",
        "stellar forge contract lint rewards --out dist/contract.lint.json",
        "stellar forge contract deploy rewards --out dist/contract.deploy.json",
        "stellar forge contract call rewards award_points --out dist/contract.call.json -- --member alice --amount 25",
        "stellar forge contract bind rewards --lang typescript --out dist/contract.bind.json",
        "stellar forge contract info credits --out dist/contract.info.json",
        "stellar forge contract fetch escrow",
        "stellar forge contract fetch rewards --out ./tmp/rewards.wasm",
        "stellar forge contract spec rewards --out dist/contract.spec.json",
        "stellar forge contract ttl extend rewards --out dist/contract.ttl.extend.json",
        "stellar forge contract ttl restore rewards --out dist/contract.ttl.restore.json",
        "stellar forge --dry-run --network testnet token create credits --mode contract --metadata-name \"Store Credit\" --initial-supply 25 --out dist/token.create.json",
        "stellar forge --dry-run --network testnet token mint credits --to alice --amount 10 --from issuer --out dist/token.mint.json",
        "stellar forge token burn points --amount 5 --from treasury --out dist/token.burn.json",
        "stellar forge token transfer points --to alice --amount 10 --from treasury --out dist/token.transfer.json",
        "stellar forge token trust points alice --out dist/token.trust.json",
        "stellar forge token freeze points alice --out dist/token.freeze.json",
        "stellar forge token unfreeze points alice --out dist/token.unfreeze.json",
        "stellar forge token clawback points alice 1 --out dist/token.clawback.json",
        "stellar forge --network testnet token sac id points --out dist/token.sac.id.json",
        "stellar forge --network testnet token sac deploy points --out dist/token.sac.deploy.json",
        "stellar forge --network testnet token contract init credits --out dist/token.contract.init.json",
        "stellar forge token info points --out dist/token.info.json",
        "stellar forge token balance points --holder alice --out dist/token.balance.json",
        "stellar forge wallet create bob --fund --out dist/wallet.create.json",
        "stellar forge wallet fund alice --out dist/wallet.fund.json",
        "stellar forge wallet trust alice points --out dist/wallet.trust.json",
        "stellar forge wallet pay --from treasury --to alice --asset points --amount 10 --out dist/wallet.pay.json",
        "stellar forge wallet sep7 payment --from treasury --to alice --asset points --amount 10 --out dist/wallet.sep7.payment.json",
        "stellar forge --network testnet wallet sep7 contract-call rewards award_points --out dist/wallet.sep7.contract-call.json -- --member alice --amount 25",
        "stellar forge wallet ls --out dist/wallet.ls.json",
        "stellar forge wallet address alice --out dist/wallet.address.json",
        "stellar forge wallet balances alice --out dist/wallet.balances.json",
        "stellar forge wallet receive alice --sep7 --asset points --out dist/wallet.receive.json",
        "stellar forge wallet smart create sentinel --mode ed25519 --out dist/wallet.smart.create.json",
        "stellar forge wallet smart scaffold guardian --out dist/wallet.smart.scaffold.json",
        "stellar forge wallet smart info guardian --out dist/wallet.smart.info.json",
        "stellar forge wallet smart onboard checkout-passkey --out dist/wallet.smart.onboard.json",
        "stellar forge wallet smart provision checkout-passkey --address CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF --out dist/wallet.smart.provision.json",
        "stellar forge wallet smart materialize checkout-passkey --out dist/wallet.smart.materialize.json",
        "stellar forge wallet smart controller rotate checkout-passkey alice --out dist/wallet.smart.controller.rotate.json",
        "stellar forge wallet smart policy info guardian --out dist/wallet.smart.policy.info.json",
        "stellar forge wallet smart policy set-daily-limit sentinel 1250 --build-only --out dist/wallet.smart.policy.set-daily-limit.json",
        "stellar forge wallet smart policy allow sentinel alice --build-only --out dist/wallet.smart.policy.allow.json",
        "stellar forge wallet smart policy revoke sentinel alice --build-only --out dist/wallet.smart.policy.revoke.json",
        "stellar forge wallet smart policy sync checkout-passkey --out dist/wallet.smart.policy.sync.json",
        "stellar forge wallet smart policy diff checkout-passkey --out dist/wallet.smart.policy.diff.json",
        "stellar forge wallet smart policy apply checkout-passkey --file policy.toml --out dist/wallet.smart.policy.apply.json",
        "stellar forge wallet smart policy simulate checkout-passkey --file policy.toml --out dist/wallet.smart.policy.simulate.json",
        "stellar forge api init --out dist/api.init.json",
        "stellar forge api generate contract rewards --out dist/api.generate.contract.json",
        "stellar forge api generate token points --out dist/api.generate.token.json",
        "stellar forge api openapi export --out dist/api.openapi.json",
        "stellar forge api events init --out dist/api.events.init.json",
        "stellar forge api relayer init --out dist/api.relayer.init.json",
        "stellar forge events export --path dist/events.json --out dist/events.export.json",
        "stellar forge events replay --path dist/events.json --out dist/events.replay.json",
        "stellar forge events watch contract rewards --out dist/events.watch.json",
        "stellar forge events ingest init --out dist/events.ingest.init.json",
        "stellar forge events backfill contract:rewards --count 200 --out dist/events.backfill.json",
        "stellar forge events status --out dist/events.status.json",
        "stellar forge events cursor ls --out dist/events.cursor.json",
        "stellar forge events cursor reset testnet:contract:rewards --out dist/events.cursor.reset.json",
        "stellar forge doctor --out dist/doctor.json",
        "stellar forge wallet batch-report --from treasury --asset XLM --file payouts.json",
        "stellar forge token airdrop-report points --from treasury --file rewards.csv --format csv",
        "stellar forge events status",
        "stellar forge wallet batch-reconcile --from treasury --asset XLM --file payouts.json --report dist/payouts.report.json",
        "stellar forge wallet batch-pay --from treasury --asset XLM --file payouts.json --out dist/payouts.pay.json",
        "stellar forge wallet batch-validate --from treasury --asset points --file payouts.json --out dist/payouts.validate.json",
        "stellar forge wallet batch-preview --from treasury --asset points --file payouts.csv --format csv --out dist/payouts.preview.json",
        "stellar forge wallet batch-summary --from treasury --asset points --file payouts.json --out dist/payouts.summary.json",
        "stellar forge wallet batch-reconcile --from treasury --asset XLM --file payouts.json --report dist/payouts.report.json --out dist/payouts.reconcile.json",
        "stellar forge wallet batch-resume --from treasury --asset XLM --file payouts.json --report dist/payouts.report.json --out dist/payouts.resume.json",
        "stellar forge token airdrop points --from treasury --file rewards.csv --format csv --out dist/airdrop.json",
        "stellar forge token airdrop-validate points --file rewards.csv --format csv --out dist/airdrop.validate.json",
        "stellar forge token airdrop-preview points --from treasury --file rewards.json --out dist/airdrop.preview.json",
        "stellar forge token airdrop-summary points --file rewards.csv --format csv --out dist/airdrop.summary.json",
        "stellar forge token airdrop-reconcile points --from treasury --file rewards.csv --format csv --report dist/airdrop.report.json --out dist/airdrop.reconcile.json",
        "stellar forge token airdrop-resume points --from treasury --file rewards.csv --format csv --report dist/airdrop.report.json --out dist/airdrop.resume.json",
        "stellar forge wallet smart policy apply checkout-passkey --file policy.toml",
        "stellar forge release plan testnet --out dist/release.plan.json",
        "stellar forge release deploy testnet --out dist/release.deploy.json",
        "stellar forge release verify testnet --out dist/release.verify.json",
        "stellar forge release aliases sync testnet --out dist/release.aliases.json",
        "stellar forge release env export testnet --out dist/release.env.json",
        "stellar forge --network testnet release registry publish rewards --out dist/release.registry.publish.json",
        "stellar forge --network testnet release registry deploy rewards --out dist/release.registry.deploy.json",
        "stellar forge wallet batch-report --from treasury --asset XLM --file payouts.json --out dist/payouts.report.json",
        "stellar forge token airdrop-report points --from treasury --file rewards.csv --format csv --out dist/airdrop.report.json",
        "stellar forge wallet batch-reconcile --from treasury --asset XLM --file payouts.json --report dist/payouts.report.json",
        "stellar forge token airdrop-resume points --from treasury --file rewards.csv --format csv --report dist/airdrop.report.json",
        "stellar forge project smoke --out dist/project.smoke.json",
        "stellar forge doctor audit --out dist/doctor.audit.json",
        "stellar forge doctor fix --out dist/doctor.fix.json",
        "stellar forge doctor env --out dist/doctor.env.json",
        "stellar forge doctor deps --out dist/doctor.deps.json",
        "stellar forge doctor project --out dist/doctor.project.json",
        "stellar forge doctor network local --out dist/doctor.network.json",
        "stellar forge release status testnet --out dist/release.status.json",
        "stellar forge release drift testnet --out dist/release.drift.json",
        "stellar forge release diff testnet --out dist/release.diff.json",
        "stellar forge release history testnet --out dist/release.history.json",
        "stellar forge release inspect testnet --out dist/release.inspect.json",
        "stellar forge release rollback testnet --out dist/release.rollback.json",
        "stellar forge release prune testnet --keep 3 --out dist/release.prune.json",
        "stellar forge events export",
        "stellar forge events replay",
        "stellar forge doctor audit",
        "stellar forge doctor fix --scope events",
        "stellar forge doctor fix --scope release",
    ] {
        assert!(
            readme.contains(expected) || reference.contains(expected),
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
fn readme_surfaces_operational_examples() {
    let root = workspace_root();
    let readme = read(root.join("README.md"));

    for expected in [
        "stellar forge wallet sep7 payment --from alice --to bob --asset points --amount 25",
        "stellar forge wallet smart create guardian --mode ed25519",
        "stellar forge wallet smart policy allow guardian alice --build-only",
        "stellar forge wallet smart policy revoke guardian alice --build-only",
        "stellar forge token freeze points alice",
        "stellar forge token clawback points alice 10",
        "stellar forge token sac id points",
        "stellar forge api init",
        "stellar forge api generate contract rewards",
        "stellar forge api openapi export",
        "stellar forge api relayer init",
        "stellar forge events export --path dist/events.json",
        "stellar forge events replay --path dist/events.json",
        "stellar forge events ingest init",
        "stellar forge events cursor reset testnet:contract:rewards",
        "stellar forge doctor env",
        "stellar forge doctor deps",
        "stellar forge doctor project",
        "stellar forge doctor network local",
        "stellar forge doctor fix --scope release",
        "stellar forge --network testnet release registry publish rewards",
        "stellar forge --network testnet release registry deploy rewards",
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
