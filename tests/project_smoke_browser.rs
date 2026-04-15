use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::tempdir;

static NEXT_PORT: AtomicUsize = AtomicUsize::new(4179);

#[test]
fn project_smoke_browser_skips_playwright_install_when_chromium_is_cached() {
    if !node_available() {
        return;
    }

    let root = init_rewards_project();
    let fake_bin = install_fake_browser_smoke_tooling(&root);
    let pnpm_log = root.join("pnpm-browser-smoke.log");
    let vite_log = root.join("vite-browser-smoke.log");
    let port = next_port();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .env("FAKE_PNPM_LOG", &pnpm_log)
        .env("FAKE_VITE_LOG", &vite_log)
        .env(
            "FAKE_PLAYWRIGHT_LIST_OUTPUT",
            "chromium-1217\nchromium_headless_shell-1217\n",
        )
        .env("STELLAR_FORGE_BROWSER_SMOKE_PORT", port.to_string())
        .args(["--json", "project", "smoke", "--browser"])
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\n\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "project.smoke");
    assert_eq!(json["status"], "ok");
    assert_eq!(json["data"]["browser"], true);

    let pnpm_invocations = read_log(&pnpm_log);
    assert!(
        pnpm_invocations
            .iter()
            .any(|line| { line.contains("dlx @playwright/test@1.59.1 install --list chromium") })
    );
    assert!(
        !pnpm_invocations
            .iter()
            .any(|line| { line.contains("dlx @playwright/test@1.59.1 install chromium") })
    );
    assert!(pnpm_invocations.iter().any(|line| {
        line.contains("dlx @playwright/test@1.59.1 test ") && line.contains("--config ")
    }));

    let vite_invocations = read_log(&vite_log);
    assert!(vite_invocations.iter().any(|line| line == "build"));
    assert!(
        vite_invocations
            .iter()
            .any(|line| line.starts_with(&format!("preview --host 127.0.0.1 --port {port}")))
    );
    assert!(root.join("apps/web/dist/index.html").exists());
}

#[test]
fn project_smoke_browser_installs_playwright_when_cache_is_cold() {
    if !node_available() {
        return;
    }

    let root = init_rewards_project();
    let fake_bin = install_fake_browser_smoke_tooling(&root);
    let pnpm_log = root.join("pnpm-browser-smoke.log");
    let port = next_port();

    let output = Command::cargo_bin("stellar-forge")
        .expect("binary should build")
        .current_dir(&root)
        .env("PATH", test_path(&fake_bin))
        .env("FAKE_PNPM_LOG", &pnpm_log)
        .env("FAKE_PLAYWRIGHT_LIST_OUTPUT", "chromium-1000\n")
        .env("STELLAR_FORGE_BROWSER_SMOKE_PORT", port.to_string())
        .args(["--json", "project", "smoke", "--browser"])
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\n\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    assert_eq!(json["action"], "project.smoke");
    assert_eq!(json["status"], "ok");

    let pnpm_invocations = read_log(&pnpm_log);
    assert!(
        pnpm_invocations
            .iter()
            .any(|line| { line.contains("dlx @playwright/test@1.59.1 install --list chromium") })
    );
    assert!(
        pnpm_invocations
            .iter()
            .any(|line| { line.contains("dlx @playwright/test@1.59.1 install chromium") })
    );
    assert!(pnpm_invocations.iter().any(|line| {
        line.contains("dlx @playwright/test@1.59.1 test ") && line.contains("--config ")
    }));
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

fn install_fake_browser_smoke_tooling(root: &Path) -> PathBuf {
    install_fake_vite(root);
    install_fake_pnpm(root)
}

fn install_fake_vite(root: &Path) {
    let vite_bin = root.join("apps/web/node_modules/vite/bin");
    fs::create_dir_all(&vite_bin).expect("vite bin dir should be created");
    let vite_path = vite_bin.join("vite.js");
    fs::write(
        &vite_path,
        r#"#!/usr/bin/env node
const fs = require('fs');
const http = require('http');
const path = require('path');

const args = process.argv.slice(2);
const appRoot = path.resolve(__dirname, '..', '..', '..');
const distDir = path.join(appRoot, 'dist');
const indexPath = path.join(distDir, 'index.html');

if (process.env.FAKE_VITE_LOG) {
  fs.appendFileSync(process.env.FAKE_VITE_LOG, `${args.join(' ')}\n`);
}

if (args[0] === 'build') {
  fs.mkdirSync(distDir, { recursive: true });
  fs.writeFileSync(
    indexPath,
    '<!doctype html><html><body><main><h1>demo</h1><div>RPC</div><div>Queue</div><div>Runtime</div><div>Events</div><div>Contracts</div><div>Tokens</div></main></body></html>',
  );
  process.exit(0);
}

if (args[0] === 'preview') {
  const host = args[args.indexOf('--host') + 1];
  const port = Number(args[args.indexOf('--port') + 1]);
  const server = http.createServer((_req, res) => {
    res.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
    res.end(fs.readFileSync(indexPath, 'utf8'));
  });
  server.listen(port, host, () => {});
  const shutdown = () => server.close(() => process.exit(0));
  process.on('SIGTERM', shutdown);
  process.on('SIGINT', shutdown);
  return;
}

console.error(`unsupported fake vite invocation: ${args.join(' ')}`);
process.exit(1);
"#,
    )
    .expect("fake vite should be written");
    #[cfg(unix)]
    fs::set_permissions(&vite_path, fs::Permissions::from_mode(0o755))
        .expect("fake vite should be executable");
}

fn install_fake_pnpm(root: &Path) -> PathBuf {
    let bin_dir = root.join(".test-bin-browser");
    fs::create_dir_all(&bin_dir).expect("browser test bin dir should be created");
    let pnpm_path = bin_dir.join("pnpm");
    fs::write(
        &pnpm_path,
        r#"#!/usr/bin/env node
const fs = require('fs');
const http = require('http');
const path = require('path');
const { spawnSync } = require('child_process');

const args = process.argv.slice(2);
const logPath = process.env.FAKE_PNPM_LOG;
if (logPath) {
  fs.appendFileSync(logPath, `${args.join(' ')}\n`);
}

if (args[0] === '--dir' && args[2] === 'smoke:browser') {
  const target = path.resolve(process.cwd(), args[1]);
  const result = spawnSync(process.execPath, [path.join(target, 'scripts', 'ui-browser-smoke.mjs')], {
    cwd: target,
    env: { ...process.env, STELLAR_FORGE_PACKAGE_MANAGER: 'pnpm' },
    stdio: 'inherit',
  });
  process.exit(result.status ?? 1);
}

if (args[0] === 'dlx') {
  const sub = args.slice(2);
  if (sub[0] === 'install' && sub[1] === '--list' && sub[2] === 'chromium') {
    process.stdout.write(process.env.FAKE_PLAYWRIGHT_LIST_OUTPUT || '');
    process.exit(0);
  }
  if (sub[0] === 'install' && sub.includes('chromium')) {
    process.exit(0);
  }
  if (sub[0] === 'test') {
    const configIndex = sub.indexOf('--config');
    if (configIndex === -1 || configIndex + 1 >= sub.length) {
      console.error('missing --config for fake playwright test');
      process.exit(1);
    }
    const config = fs.readFileSync(sub[configIndex + 1], 'utf8');
    const match = config.match(/baseURL:\s*['"]([^'"]+)['"]/);
    if (!match) {
      console.error('missing baseURL in fake playwright config');
      process.exit(1);
    }
    http
      .get(match[1], (response) => {
        response.resume();
        process.exit(response.statusCode === 200 ? 0 : 1);
      })
      .on('error', (error) => {
        console.error(error.message);
        process.exit(1);
      });
    return;
  }
}

console.error(`unsupported fake pnpm invocation: ${args.join(' ')}`);
process.exit(1);
"#,
    )
    .expect("fake pnpm should be written");
    #[cfg(unix)]
    fs::set_permissions(&pnpm_path, fs::Permissions::from_mode(0o755))
        .expect("fake pnpm should be executable");
    bin_dir
}

fn read_log(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .expect("log file should be readable")
        .lines()
        .map(str::to_string)
        .collect()
}

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn test_path(fake_bin: &Path) -> String {
    format!(
        "{}:{}",
        fake_bin.display(),
        std::env::var("PATH").expect("PATH should exist")
    )
}

fn next_port() -> usize {
    NEXT_PORT.fetch_add(1, Ordering::Relaxed)
}
