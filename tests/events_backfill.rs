use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

#[test]
fn events_backfill_persists_rows_and_cursor_snapshot_statefully() {
    if !sqlite_available() {
        return;
    }

    let root = init_backfill_project();
    let fake_bin = install_fake_stellar(&root);
    let fake_log = root.join("fake-stellar.log");

    let first = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .env("FAKE_LOG_PATH", &fake_log)
        .args(["--json", "events", "backfill", "app", "--count", "2"])
        .output()
        .expect("first backfill should run");

    assert!(first.status.success());
    let first_json: Value =
        serde_json::from_slice(&first.stdout).expect("first stdout should be valid json");
    assert_eq!(first_json["action"], "events.backfill");
    assert_eq!(first_json["status"], "ok");
    assert_eq!(first_json["data"]["event_count"], 2);
    assert_eq!(first_json["data"]["latest_ledger"], 202);
    assert_eq!(first_json["data"]["cursor_name"], "testnet:contract:app");
    assert_eq!(first_json["data"]["cursor"], Value::Null);

    assert_eq!(
        sqlite_scalar_i64(&root, "select count(*) as count from events;"),
        2
    );
    assert_eq!(
        sqlite_scalar_i64(&root, "select count(*) as count from cursors;"),
        1
    );
    assert_eq!(
        sqlite_json(
            &root,
            "select name, resource_kind, resource_name, cursor, last_ledger from cursors;"
        )
        .as_array()
        .expect("cursor rows should be an array")[0]["cursor"]
            .as_str(),
        Some("ledger:202")
    );

    let snapshot_path = root.join("workers/events/cursors.json");
    let first_snapshot: Value =
        serde_json::from_str(&read(&snapshot_path)).expect("snapshot should parse");
    assert_eq!(
        first_snapshot["cursors"]["testnet:contract:app"]["cursor"],
        "ledger:202"
    );
    assert_eq!(
        first_snapshot["cursors"]["testnet:contract:app"]["last_ledger"],
        202
    );

    sqlite_exec(
        &root,
        "delete from cursors where name = 'testnet:contract:app';",
    );
    assert_eq!(
        sqlite_scalar_i64(&root, "select count(*) as count from cursors;"),
        0
    );

    let second = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .env("FAKE_LOG_PATH", &fake_log)
        .args(["--json", "events", "backfill", "app", "--count", "1"])
        .output()
        .expect("second backfill should run");

    assert!(second.status.success());
    let second_json: Value =
        serde_json::from_slice(&second.stdout).expect("second stdout should be valid json");
    assert_eq!(second_json["action"], "events.backfill");
    assert_eq!(second_json["status"], "ok");
    assert_eq!(second_json["data"]["event_count"], 1);
    assert_eq!(second_json["data"]["latest_ledger"], 203);
    assert_eq!(second_json["data"]["cursor"].as_str(), Some("ledger:202"));

    assert_eq!(
        sqlite_scalar_i64(&root, "select count(*) as count from events;"),
        3
    );
    assert_eq!(
        sqlite_scalar_i64(&root, "select count(*) as count from cursors;"),
        1
    );
    assert_eq!(
        sqlite_json(
            &root,
            "select name, resource_kind, resource_name, cursor, last_ledger from cursors;"
        )
        .as_array()
        .expect("cursor rows should be an array")[0]["cursor"]
            .as_str(),
        Some("ledger:203")
    );

    let second_snapshot: Value =
        serde_json::from_str(&read(&snapshot_path)).expect("snapshot should parse");
    assert_eq!(
        second_snapshot["cursors"]["testnet:contract:app"]["cursor"],
        "ledger:203"
    );
    assert_eq!(
        second_snapshot["cursors"]["testnet:contract:app"]["last_ledger"],
        203
    );

    let invocations = read_invocation_log(&fake_log);
    assert_eq!(invocations.len(), 2);
    assert!(invocations[0].contains("events --output json --network testnet"));
    assert!(invocations[0].contains("--count 2"));
    assert!(!invocations[0].contains("--cursor"));
    assert!(invocations[1].contains("events --output json --network testnet"));
    assert!(invocations[1].contains("--count 1"));
    assert!(invocations[1].contains("--cursor ledger:202"));
}

fn init_backfill_project() -> PathBuf {
    let temp = tempdir().expect("tempdir should be created");
    let kept = temp.keep();
    let fake_bin = install_fake_stellar(&kept);
    let root = kept.join("demo");
    let parent = root
        .parent()
        .expect("demo should have a parent")
        .to_path_buf();

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&parent)
        .env("PATH", test_path(&fake_bin))
        .args(["init", "demo", "--template", "minimal-contract", "--no-api"])
        .assert()
        .success();

    Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .args(["api", "events", "init"])
        .assert()
        .success();

    root
}

fn install_fake_stellar(root: &Path) -> PathBuf {
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

if [ "$1" = "events" ]; then
  prev=""
  cursor=""
  for arg in "$@"; do
    if [ "$prev" = "--cursor" ]; then
      cursor="$arg"
    fi
    prev="$arg"
  done

  if [ -n "$FAKE_LOG_PATH" ]; then
    printf '%s\n' "$*" >> "$FAKE_LOG_PATH"
  fi

  if [ "$cursor" = "ledger:202" ]; then
    cat <<'JSON'
[
  {
    "id": "evt-3",
    "cursor": "ledger:203",
    "type": "contract",
    "topic": ["contract", "rewards", "updated"],
    "payload": {"amount": "30"},
    "txHash": "TX3",
    "ledger": 203,
    "ledgerClosedAt": "2026-04-14T00:00:03Z"
  }
]
JSON
  else
    cat <<'JSON'
[
  {
    "id": "evt-1",
    "cursor": "ledger:201",
    "type": "contract",
    "topic": ["contract", "rewards", "minted"],
    "payload": {"amount": "10"},
    "txHash": "TX1",
    "ledger": 201,
    "ledgerClosedAt": "2026-04-14T00:00:01Z"
  },
  {
    "id": "evt-2",
    "cursor": "ledger:202",
    "type": "contract",
    "topic": ["contract", "rewards", "minted"],
    "payload": {"amount": "20"},
    "txHash": "TX2",
    "ledger": 202,
    "ledgerClosedAt": "2026-04-14T00:00:02Z"
  }
]
JSON
  fi
  exit 0
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

fn sqlite_exec(root: &Path, sql: &str) {
    let output = Command::new("sqlite3")
        .current_dir(root)
        .arg(root.join("apps/api/db/events.sqlite"))
        .arg(sql)
        .output()
        .expect("sqlite3 should run");
    assert!(
        output.status.success(),
        "sqlite3 exec should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn sqlite_json(root: &Path, sql: &str) -> Value {
    let output = Command::new("sqlite3")
        .current_dir(root)
        .arg(root.join("apps/api/db/events.sqlite"))
        .arg("-json")
        .arg(sql)
        .output()
        .expect("sqlite3 should run");
    assert!(
        output.status.success(),
        "sqlite query should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("sqlite stdout should be json")
}

fn sqlite_scalar_i64(root: &Path, sql: &str) -> i64 {
    let rows = sqlite_json(root, sql);
    rows.as_array()
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("count"))
        .and_then(Value::as_i64)
        .expect("count should be present")
}

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).expect("file should exist")
}

fn read_invocation_log(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .expect("invocation log should be readable")
        .lines()
        .map(str::to_string)
        .collect()
}

fn sqlite_available() -> bool {
    Command::new("sqlite3")
        .arg("-version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
