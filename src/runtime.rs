use crate::cli::GlobalOptions;
use anyhow::{Context, Result, anyhow, bail};
use reqwest::blocking::Client;
use serde::Serialize;
use serde_json::Value;
use shell_escape::escape;
use std::borrow::Cow;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use url::Url;

#[derive(Debug, Clone)]
pub struct AppContext {
    pub cwd: PathBuf,
    pub manifest_path: PathBuf,
    pub globals: GlobalOptions,
    client: Client,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    pub name: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandReport {
    pub status: String,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checks: Vec<CheckResult>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub next: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl CommandReport {
    pub fn new(action: impl Into<String>) -> Self {
        Self {
            status: "ok".to_string(),
            action: action.into(),
            message: None,
            network: None,
            warnings: Vec::new(),
            checks: Vec::new(),
            commands: Vec::new(),
            artifacts: Vec::new(),
            next: Vec::new(),
            data: None,
        }
    }
}

impl AppContext {
    pub fn from_globals(globals: &GlobalOptions) -> Result<Self> {
        let cwd = match &globals.cwd {
            Some(cwd) => cwd.clone(),
            None => std::env::current_dir().context("failed to resolve current directory")?,
        };
        let manifest_path = match &globals.manifest {
            Some(path) if path.is_absolute() => path.clone(),
            Some(path) => cwd.join(path),
            None => cwd.join("stellarforge.toml"),
        };
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .context("failed to build HTTP client")?;
        Ok(Self {
            cwd,
            manifest_path,
            globals: globals.clone(),
            client,
        })
    }

    pub fn project_root(&self) -> PathBuf {
        self.manifest_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.cwd.clone())
    }

    pub fn render(&self, report: &CommandReport) -> String {
        render_report(report, self.globals.json)
    }

    pub fn ensure_dir(&self, report: &mut CommandReport, path: &Path) -> Result<()> {
        report.artifacts.push(path.display().to_string());
        if self.globals.dry_run {
            return Ok(());
        }
        fs::create_dir_all(path)
            .with_context(|| format!("failed to create directory {}", path.display()))?;
        Ok(())
    }

    pub fn write_text(
        &self,
        report: &mut CommandReport,
        path: &Path,
        contents: &str,
    ) -> Result<()> {
        report.artifacts.push(path.display().to_string());
        if self.globals.dry_run {
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create parent directory {}", parent.display())
            })?;
        }
        fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    pub fn read_text(&self, path: &Path) -> Result<String> {
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))
    }

    pub fn run_command(
        &self,
        report: &mut CommandReport,
        cwd: Option<&Path>,
        program: &str,
        args: &[String],
    ) -> Result<String> {
        let rendered = render_command(program, args);
        report.commands.push(rendered.clone());
        if self.globals.dry_run {
            return Ok(String::new());
        }
        let preferred_path = preferred_path_env();
        let resolved_program = preferred_path.as_ref().and_then(|path| {
            which::which_in(program, Some(path), cwd.unwrap_or(self.cwd.as_path())).ok()
        });
        let mut command = Command::new(resolved_program.unwrap_or_else(|| PathBuf::from(program)));
        command.args(args);
        if let Some(dir) = cwd {
            command.current_dir(dir);
        }
        if let Some(path) = &preferred_path {
            command.env("PATH", path);
        }
        let output = command
            .output()
            .with_context(|| format!("failed to run `{rendered}`"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let detail = if !stderr.is_empty() {
                stderr
            } else if !stdout.is_empty() {
                stdout
            } else {
                format!("process exited with status {}", output.status)
            };
            bail!("command `{rendered}` failed: {detail}");
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub fn command_exists(&self, name: &str) -> bool {
        preferred_path_env()
            .as_ref()
            .map(|path| which::which_in(name, Some(path), self.cwd.as_path()).is_ok())
            .unwrap_or_else(|| which::which(name).is_ok())
    }

    pub fn command_succeeds(&self, cwd: Option<&Path>, program: &str, args: &[&str]) -> bool {
        let preferred_path = preferred_path_env();
        let resolved_program = preferred_path.as_ref().and_then(|path| {
            which::which_in(program, Some(path), cwd.unwrap_or(self.cwd.as_path())).ok()
        });
        let mut command = Command::new(resolved_program.unwrap_or_else(|| PathBuf::from(program)));
        command.args(args);
        if let Some(dir) = cwd {
            command.current_dir(dir);
        }
        if let Some(path) = &preferred_path {
            command.env("PATH", path);
        }
        command
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    pub fn get_json(&self, url: &Url) -> Result<Value> {
        self.client
            .get(url.clone())
            .send()
            .with_context(|| format!("failed to GET {url}"))?
            .error_for_status()
            .with_context(|| format!("request to {url} failed"))?
            .json()
            .with_context(|| format!("invalid JSON from {url}"))
    }

    pub fn post_json(&self, url: &Url, body: &Value) -> Result<Value> {
        self.client
            .post(url.clone())
            .json(body)
            .send()
            .with_context(|| format!("failed to POST {url}"))?
            .error_for_status()
            .with_context(|| format!("request to {url} failed"))?
            .json()
            .with_context(|| format!("invalid JSON from {url}"))
    }
}

pub(crate) fn render_report(report: &CommandReport, json: bool) -> String {
    if json {
        match serde_json::to_string_pretty(report) {
            Ok(rendered) => rendered,
            Err(error) => format!(
                "{{\n  \"action\": {:?},\n  \"status\": \"error\",\n  \"message\": {:?}\n}}",
                report.action,
                format!("failed to serialize report: {error}")
            ),
        }
    } else {
        render_human(report)
    }
}

pub fn render_command(program: &str, args: &[String]) -> String {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(escape(Cow::Borrowed(program)).to_string());
    for arg in args {
        parts.push(escape(Cow::Borrowed(arg.as_str())).to_string());
    }
    parts.join(" ")
}

pub fn check(
    name: impl Into<String>,
    status: impl Into<String>,
    detail: impl Into<Option<String>>,
) -> CheckResult {
    CheckResult {
        name: name.into(),
        status: status.into(),
        detail: detail.into(),
    }
}

pub fn path_to_string(path: &Path) -> Result<String> {
    path.to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("path is not valid UTF-8: {}", path.display()))
}

fn preferred_path_env() -> Option<OsString> {
    let existing = env::var_os("PATH")?;
    let Some(home) = env::var_os("HOME") else {
        return Some(existing);
    };
    let cargo_bin = PathBuf::from(home).join(".cargo").join("bin");
    if !cargo_bin.is_dir() {
        return Some(existing);
    }

    let mut paths = vec![cargo_bin.clone()];
    paths.extend(env::split_paths(&existing).filter(|path| path != &cargo_bin));
    env::join_paths(paths).ok()
}

fn render_human(report: &CommandReport) -> String {
    let mut lines = Vec::new();
    lines.push(format!("{} [{}]", report.action, report.status));
    if let Some(message) = &report.message {
        lines.push(message.clone());
    }
    if let Some(network) = &report.network {
        lines.push(format!("network: {network}"));
    }
    if !report.checks.is_empty() {
        lines.push(String::new());
        lines.push("checks:".to_string());
        for check in &report.checks {
            let mut line = format!("- {}: {}", check.status, check.name);
            if let Some(detail) = &check.detail {
                line.push_str(&format!(" ({detail})"));
            }
            lines.push(line);
        }
    }
    if !report.warnings.is_empty() {
        lines.push(String::new());
        lines.push("warnings:".to_string());
        for warning in &report.warnings {
            lines.push(format!("- {warning}"));
        }
    }
    if !report.commands.is_empty() {
        lines.push(String::new());
        lines.push("commands:".to_string());
        for command in &report.commands {
            lines.push(format!("- {command}"));
        }
    }
    if !report.artifacts.is_empty() {
        lines.push(String::new());
        lines.push("artifacts:".to_string());
        for artifact in &report.artifacts {
            lines.push(format!("- {artifact}"));
        }
    }
    if !report.next.is_empty() {
        lines.push(String::new());
        lines.push("next:".to_string());
        for next in &report.next {
            lines.push(format!("- {next}"));
        }
    }
    if let Some(data) = &report.data {
        lines.push(String::new());
        lines.push("data:".to_string());
        match serde_json::to_string_pretty(data) {
            Ok(rendered) => lines.push(rendered),
            Err(error) => lines.push(format!("<failed to serialize data: {error}>")),
        }
    }
    lines.join("\n")
}
