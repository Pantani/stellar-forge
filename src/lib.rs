mod cli;
mod commands;
mod model;
mod runtime;
mod templates;

use anyhow::{Error, Result};
use clap::{Parser, error::ErrorKind};
use cli::Cli;
use commands::execute;
use runtime::{AppContext, CommandReport, render_report};
use serde_json::json;
use std::ffi::{OsStr, OsString};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone)]
pub struct RunOutput {
    pub rendered: String,
    pub exit_code: i32,
    pub stream: OutputStream,
}

#[derive(Debug, Clone, Copy)]
struct ErrorClass {
    code: &'static str,
    exit_code: i32,
}

pub fn run_cli<I, T>(args: I) -> RunOutput
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<OsString>>();
    let json_requested = args.iter().any(|arg| arg == OsStr::new("--json"));

    match Cli::try_parse_from(args) {
        Ok(cli) => run_parsed(cli),
        Err(error) => render_parse_error(error, json_requested),
    }
}

pub fn run_from<I, T>(args: I) -> Result<String>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::parse_from(args);
    let context = AppContext::from_globals(&cli.global)?;
    let report = execute(&context, cli)?;
    Ok(context.render(&report))
}

fn run_parsed(cli: Cli) -> RunOutput {
    let json_mode = cli.global.json;
    let action = cli.command.action_hint();

    match AppContext::from_globals(&cli.global) {
        Ok(context) => match execute(&context, cli) {
            Ok(report) => RunOutput {
                rendered: context.render(&report),
                exit_code: 0,
                stream: OutputStream::Stdout,
            },
            Err(error) => render_runtime_error(action, json_mode, &error),
        },
        Err(error) => render_runtime_error(action, json_mode, &error),
    }
}

fn render_parse_error(error: clap::Error, json_requested: bool) -> RunOutput {
    let kind = error.kind();
    if matches!(kind, ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) {
        return RunOutput {
            rendered: error.to_string(),
            exit_code: 0,
            stream: OutputStream::Stdout,
        };
    }

    if json_requested {
        let message = error.to_string();
        let report = error_report(
            "cli.parse",
            &message,
            ErrorClass {
                code: "input",
                exit_code: 2,
            },
            vec!["stellar-forge --help".to_string()],
            Some(json!({
                "kind": format!("{kind:?}"),
            })),
        );
        return RunOutput {
            rendered: render_report(&report, true),
            exit_code: 2,
            stream: OutputStream::Stdout,
        };
    }

    RunOutput {
        rendered: error.to_string(),
        exit_code: 2,
        stream: OutputStream::Stderr,
    }
}

fn render_runtime_error(action: &str, json_mode: bool, error: &Error) -> RunOutput {
    let report = runtime_error_report(action, error);
    RunOutput {
        rendered: render_report(&report, json_mode),
        exit_code: report
            .data
            .as_ref()
            .and_then(|data| data.get("exit_code"))
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(1) as i32,
        stream: if json_mode {
            OutputStream::Stdout
        } else {
            OutputStream::Stderr
        },
    }
}

fn runtime_error_report(action: &str, error: &Error) -> CommandReport {
    let class = classify_error(error);
    let message = format!("{error:#}");
    let next = suggest_next_steps(action, &message);
    error_report(action, &message, class, next, None)
}

fn error_report(
    action: &str,
    message: &str,
    class: ErrorClass,
    next: Vec<String>,
    extra_data: Option<serde_json::Value>,
) -> CommandReport {
    let mut report = CommandReport::new(action);
    report.status = "error".to_string();
    report.message = Some(message.to_string());
    report.next = next;

    let causes = split_causes(message);
    let mut data = json!({
        "error_code": class.code,
        "exit_code": class.exit_code,
        "causes": causes,
    });
    if let Some(extra) = extra_data
        && let (Some(target), Some(source)) = (data.as_object_mut(), extra.as_object())
    {
        for (key, value) in source {
            target.insert(key.clone(), value.clone());
        }
    }
    report.data = Some(data);
    report
}

fn classify_error(error: &Error) -> ErrorClass {
    let message = format!("{error:#}").to_ascii_lowercase();

    if contains_any(
        &message,
        &[
            "filesystem-safe name",
            "clawback_enabled = true",
            "must be the last segment",
            "use `events backfill account",
            "unsupported event resource prefix",
            "was not recognized; use a contract name",
            "does not use classic trustlines",
            "does not resolve to a classic account yet",
            "already exists as a smart wallet",
            "already exists as a classic wallet",
        ],
    ) {
        return ErrorClass {
            code: "input",
            exit_code: 2,
        };
    }

    if contains_any(
        &message,
        &[
            "confirm_mainnet",
            "funding is refused on pubnet",
            "unsafe",
            "refused on pubnet",
        ],
    ) {
        return ErrorClass {
            code: "unsafe",
            exit_code: 8,
        };
    }

    if contains_any(
        &message,
        &[
            "contract wrapper",
            "needs a sac",
            "materialized deployment",
            "token sac deploy",
            "no release state",
            "no deployed contract id",
            "lockfile",
        ],
    ) {
        return ErrorClass {
            code: "state",
            exit_code: 9,
        };
    }

    if contains_any(
        &message,
        &[
            "stellarforge.toml",
            "manifest",
            "not defined",
            "references missing",
            "must stay inside the project root",
            "declared as a contract token",
        ],
    ) {
        return ErrorClass {
            code: "manifest",
            exit_code: 7,
        };
    }

    if contains_any(
        &message,
        &[
            "wasm artifact",
            "cargo build",
            "failed to build",
            "cargo metadata",
        ],
    ) {
        return ErrorClass {
            code: "build",
            exit_code: 6,
        };
    }

    if contains_any(
        &message,
        &[
            "contract invoke",
            "transaction",
            "simulation",
            "soroban",
            "tx failed",
        ],
    ) {
        return ErrorClass {
            code: "chain",
            exit_code: 5,
        };
    }

    if contains_any(
        &message,
        &[
            "failed to post",
            "failed to get",
            "request to ",
            "friendbot",
            "invalid json from",
            "rpc request failed",
            "horizon",
            "network request",
        ],
    ) {
        return ErrorClass {
            code: "network",
            exit_code: 4,
        };
    }

    if contains_any(
        &message,
        &[
            "requires the `",
            "not installed",
            "failed to run `stellar",
            "failed to run `docker",
            "failed to build http client",
            "no such file or directory",
        ],
    ) {
        return ErrorClass {
            code: "dependency",
            exit_code: 3,
        };
    }

    if contains_any(
        &message,
        &[
            "must be the last segment",
            "not found",
            "required",
            "invalid",
            "missing",
            "refused",
        ],
    ) {
        return ErrorClass {
            code: "input",
            exit_code: 2,
        };
    }

    ErrorClass {
        code: "unknown",
        exit_code: 1,
    }
}

fn suggest_next_steps(action: &str, message: &str) -> Vec<String> {
    let lower = message.to_ascii_lowercase();

    if action == "cli.parse" {
        return vec!["stellar-forge --help".to_string()];
    }

    if lower.contains("contract wrapper")
        || lower.contains("needs a sac")
        || lower.contains("token sac deploy")
    {
        if let Some(token) = first_backticked(message) {
            return vec![format!("stellar forge token sac deploy {token}")];
        }
        return vec!["stellar forge token sac deploy <token>".to_string()];
    }

    if lower.contains("clawback_enabled = true") {
        return vec!["enable `clawback_enabled = true` in `stellarforge.toml`".to_string()];
    }

    if lower.contains("does not use classic trustlines") {
        return vec![
            "use `stellar forge wallet pay ...` or a contract call for this token".to_string(),
        ];
    }

    if lower.contains("does not resolve to a classic account yet") {
        if let Some(wallet) = first_backticked(message) {
            return vec![format!("stellar forge wallet smart info {wallet}")];
        }
        return vec!["stellar forge wallet smart info <name>".to_string()];
    }

    if lower.contains("already exists as a smart wallet") {
        if let Some(wallet) = first_backticked(message) {
            return vec![format!("stellar forge wallet smart info {wallet}")];
        }
        return vec!["stellar forge wallet smart info <name>".to_string()];
    }

    if lower.contains("must be the last segment") {
        return vec!["move `**` to the final topic segment".to_string()];
    }

    if lower.contains("stellarforge.toml not found") {
        return vec!["stellar forge init <name>".to_string()];
    }

    if lower.contains("confirm_mainnet") {
        return vec![
            "rerun with `stellar forge release deploy <env> --confirm-mainnet`".to_string(),
        ];
    }

    Vec::new()
}

fn split_causes(message: &str) -> Vec<String> {
    message
        .split(": ")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn first_backticked(message: &str) -> Option<&str> {
    let start = message.find('`')?;
    let rest = &message[start + 1..];
    let end = rest.find('`')?;
    Some(&rest[..end])
}

fn contains_any(message: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| message.contains(pattern))
}

#[cfg(test)]
mod tests {
    use super::{
        classify_error, first_backticked, runtime_error_report, split_causes, suggest_next_steps,
    };
    use anyhow::anyhow;

    #[test]
    fn classify_error_prefers_network_signals_over_generic_invalid_text() {
        let error = anyhow!("invalid JSON from https://rpc.example");
        let class = classify_error(&error);

        assert_eq!(class.code, "network");
        assert_eq!(class.exit_code, 4);
    }

    #[test]
    fn runtime_error_report_includes_state_classification_causes_and_next_steps() {
        let error = anyhow!("wallet payment failed: token `points` needs a sac: lockfile missing");
        let report = runtime_error_report("wallet.pay", &error);
        let data = report.data.expect("report should include structured data");

        assert_eq!(report.status, "error");
        assert_eq!(
            report.next,
            vec!["stellar forge token sac deploy points".to_string()]
        );
        assert_eq!(data["error_code"], "state");
        assert_eq!(data["exit_code"], 9);
        assert_eq!(
            data["causes"]
                .as_array()
                .expect("causes should be an array")
                .iter()
                .filter_map(|value| value.as_str())
                .collect::<Vec<_>>(),
            vec![
                "wallet payment failed",
                "token `points` needs a sac",
                "lockfile missing",
            ]
        );
    }

    #[test]
    fn suggest_next_steps_uses_backticked_wallet_name_for_smart_wallet_conflicts() {
        let next = suggest_next_steps(
            "wallet.create",
            "wallet `vault` already exists as a smart wallet",
        );

        assert_eq!(
            next,
            vec!["stellar forge wallet smart info vault".to_string()]
        );
    }

    #[test]
    fn split_causes_and_first_backticked_extract_structured_parts() {
        let message = "outer context: inner `value`: leaf detail";

        assert_eq!(
            split_causes(message),
            vec![
                "outer context".to_string(),
                "inner `value`".to_string(),
                "leaf detail".to_string(),
            ]
        );
        assert_eq!(first_backticked(message), Some("value"));
        assert_eq!(first_backticked("no marker here"), None);
    }
}
