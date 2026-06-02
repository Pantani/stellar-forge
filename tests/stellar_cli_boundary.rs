use serde_json::Value;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

mod support;

use support::{cargo_cli, init_minimal_contract_project, test_path};

#[test]
fn doctor_deps_requires_exact_forge_plugin_name_from_stellar_plugin_ls() {
    let root = init_minimal_contract_project();
    let (fake_bin, fake_log) = install_fake_stellar_with_plugins(&root, "forgery\n");

    let output = cargo_cli()
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .env("FAKE_STELLAR_LOG", &fake_log)
        .args(["--json", "doctor", "deps"])
        .output()
        .expect("doctor deps should run");

    assert!(
        output.status.success(),
        "doctor deps should complete offline\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "doctor.deps");

    let plugin_check = find_check(&json, "plugin detection");
    assert_eq!(plugin_check["status"], "warn");
    assert!(
        plugin_check["detail"]
            .as_str()
            .expect("plugin detail should be present")
            .contains("did not report `forge`")
    );
    assert!(
        json["commands"]
            .as_array()
            .expect("commands should be an array")
            .iter()
            .any(|command| command.as_str() == Some("stellar plugin ls"))
    );

    let invocations = fs::read_to_string(&fake_log).expect("fake stellar log should be readable");
    assert!(invocations.lines().any(|line| line == "plugin ls"));
}

fn install_fake_stellar_with_plugins(root: &Path, plugins: &str) -> (PathBuf, PathBuf) {
    let bin_dir = root.join(".test-bin");
    fs::create_dir_all(&bin_dir).expect("bin dir should be created");
    let log_path = root.join("fake-stellar.log");
    let script_path = bin_dir.join("stellar");
    let plugins = plugins.trim_end();
    let script = format!(
        r#"#!/bin/sh
if [ -n "$FAKE_STELLAR_LOG" ]; then
  printf '%s\n' "$*" >> "$FAKE_STELLAR_LOG"
fi
if [ "$1" = "registry" ] && [ "$2" = "--help" ]; then
  echo "stellar registry help"
  exit 0
fi
if [ "$1" = "plugin" ] && [ "$2" = "ls" ]; then
  cat <<'PLUGINS'
{plugins}
PLUGINS
  exit 0
fi
echo "unsupported fake stellar invocation: $@" >&2
exit 1
"#
    );
    fs::write(&script_path, script).expect("fake stellar should be written");
    #[cfg(unix)]
    fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))
        .expect("fake stellar should be executable");
    (bin_dir, log_path)
}

fn find_check<'a>(json: &'a Value, name: &str) -> &'a Value {
    json["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .find(|entry| entry["name"] == name)
        .unwrap_or_else(|| panic!("missing check `{name}`"))
}
