use super::*;
use crate::model::EnvironmentLock;

pub(super) fn release_command(
    context: &AppContext,
    command: ReleaseCommand,
) -> Result<CommandReport> {
    let out = release_command_output_path(&command);
    let mut report = match command {
        ReleaseCommand::Plan { env, .. } => release_plan(context, &env),
        ReleaseCommand::Deploy {
            env,
            confirm_mainnet,
            ..
        } => release_deploy(context, &env, confirm_mainnet),
        ReleaseCommand::Verify { env, .. } => release_verify(context, &env),
        ReleaseCommand::Status { env, .. } => release_status(context, &env),
        ReleaseCommand::Drift { env, .. } => release_drift(context, &env),
        ReleaseCommand::Diff { env, path, .. } => release_diff(context, &env, path.as_deref()),
        ReleaseCommand::History { env, .. } => release_history(context, &env),
        ReleaseCommand::Inspect { env, path, .. } => {
            release_inspect(context, &env, path.as_deref())
        }
        ReleaseCommand::Rollback { env, to, .. } => release_rollback(context, &env, to.as_deref()),
        ReleaseCommand::Prune(args) => release_prune(context, &args.env, args.keep),
        ReleaseCommand::Aliases(args) => match args.command {
            ReleaseAliasesCommand::Sync { env, .. } => release_aliases_sync(context, &env),
        },
        ReleaseCommand::Env(args) => match args.command {
            ReleaseEnvCommand::Export { env, .. } => release_env_export(context, &env),
        },
        ReleaseCommand::Registry(args) => match args.command {
            ReleaseRegistryCommand::Publish { contract, .. } => {
                release_registry_publish(context, &contract)
            }
            ReleaseRegistryCommand::Deploy { contract, .. } => {
                release_registry_deploy(context, &contract)
            }
        },
    }?;
    if let Some(path) = out.as_deref() {
        persist_report_output(context, &mut report, path)?;
    }
    Ok(report)
}

fn release_command_output_path(command: &ReleaseCommand) -> Option<PathBuf> {
    match command {
        ReleaseCommand::Plan { out, .. }
        | ReleaseCommand::Deploy { out, .. }
        | ReleaseCommand::Verify { out, .. }
        | ReleaseCommand::Status { out, .. }
        | ReleaseCommand::Drift { out, .. }
        | ReleaseCommand::Diff { out, .. }
        | ReleaseCommand::History { out, .. }
        | ReleaseCommand::Inspect { out, .. } => out.clone(),
        ReleaseCommand::Aliases(args) => match &args.command {
            ReleaseAliasesCommand::Sync { out, .. } => out.clone(),
        },
        ReleaseCommand::Env(args) => match &args.command {
            ReleaseEnvCommand::Export { out, .. } => out.clone(),
        },
        ReleaseCommand::Registry(args) => match &args.command {
            ReleaseRegistryCommand::Publish { out, .. } => out.clone(),
            ReleaseRegistryCommand::Deploy { out, .. } => out.clone(),
        },
        ReleaseCommand::Rollback { out, .. } => out.clone(),
        ReleaseCommand::Prune(args) => args.out.clone(),
    }
}

pub fn release_status(context: &AppContext, env: &str) -> Result<CommandReport> {
    let mut report = CommandReport::new("release.status");
    let manifest = load_manifest(context)?;
    if !manifest.networks.contains_key(env) {
        bail!("network `{env}` not found");
    }
    let lockfile = load_lockfile(context)?;
    let root = context.project_root();
    let current_path = release_artifact_path(&root, env);
    let current = if current_path.exists() {
        let (summary, warning) = release_artifact_summary(&current_path, "current")?;
        if let Some(warning) = warning {
            report.warnings.push(warning);
        }
        Some(summary)
    } else {
        None
    };
    let history_paths = release_history_artifacts(&root, env)?;
    let latest_history = if let Some(path) = history_paths.last() {
        let (summary, warning) = release_artifact_summary(path, "history")?;
        if let Some(warning) = warning {
            report.warnings.push(warning);
        }
        Some(summary)
    } else {
        None
    };

    report.checks.extend(release_state_checks(
        &root, &manifest, &lockfile, env, false,
    ));
    report.status = aggregate_status(&report.checks);
    report.network = Some(env.to_string());
    report.message = Some(format!("release status summarized for `{env}`"));
    report.next = vec![
        format!("stellar forge release diff {env}"),
        format!("stellar forge release prune {env}"),
    ];
    report.data = Some(json!({
        "current": current,
        "latest_history": latest_history,
        "history_count": history_paths.len(),
    }));
    Ok(report)
}

pub fn release_drift(context: &AppContext, env: &str) -> Result<CommandReport> {
    let mut report = CommandReport::new("release.drift");
    let manifest = load_manifest(context)?;
    if !manifest.networks.contains_key(env) {
        bail!("network `{env}` not found");
    }
    let lockfile = load_lockfile(context)?;
    let root = context.project_root();
    let expected_artifact = build_release_artifact(&manifest, &lockfile, env)?;
    let current_path = release_artifact_path(&root, env);
    let current = if current_path.exists() {
        let (summary, warning) = release_artifact_summary(&current_path, "current")?;
        if let Some(warning) = warning {
            report.warnings.push(warning);
        }
        Some((read_release_artifact_value(&current_path)?, summary))
    } else {
        None
    };
    let history_paths = release_history_artifacts(&root, env)?;
    let latest_history = if let Some(path) = history_paths.last() {
        let (summary, warning) = release_artifact_summary(path, "history")?;
        if let Some(warning) = warning {
            report.warnings.push(warning);
        }
        Some((read_release_artifact_value(path)?, summary))
    } else {
        None
    };

    report.checks.extend(release_state_checks(
        &root, &manifest, &lockfile, env, false,
    ));
    report.checks.extend(release_registry_artifact_checks(
        &root, &manifest, &lockfile, env, false,
    ));
    if let Some(environment) = lockfile.environments.get(env) {
        if !context.globals.dry_run && context.command_exists("stellar") {
            let probe_checks =
                probe_release_deployments(context, &mut report, &manifest, env, environment)?;
            report.checks.extend(probe_checks);
        } else if !environment.contracts.is_empty() || !environment.tokens.is_empty() {
            report.warnings.push(
                "skipped on-chain contract fetch probes; run without `--dry-run` on a machine with `stellar` configured to verify deployed IDs"
                    .to_string(),
            );
        }
    }
    if let Some((history_artifact, _)) = &latest_history {
        let history_issues = release_artifact_diff(&expected_artifact, history_artifact);
        report.checks.push(check(
            format!("release:{env}:history:drift"),
            if history_issues.is_empty() {
                "ok"
            } else {
                "warn"
            },
            Some(if history_issues.is_empty() {
                "latest archived release matches the current manifest and lockfile".to_string()
            } else {
                history_issues.join("; ")
            }),
        ));
    }

    let current_vs_expected = current
        .as_ref()
        .map(|(artifact, _)| release_artifact_diff(&expected_artifact, artifact))
        .unwrap_or_default();
    let latest_history_vs_expected = latest_history
        .as_ref()
        .map(|(artifact, _)| release_artifact_diff(&expected_artifact, artifact))
        .unwrap_or_default();
    let current_vs_latest_history = match (&current, &latest_history) {
        (Some((current_artifact, _)), Some((history_artifact, _))) => {
            release_artifact_diff(current_artifact, history_artifact)
        }
        _ => Vec::new(),
    };

    report.status = aggregate_status(&report.checks);
    report.network = Some(env.to_string());
    report.message = Some(format!("release drift summarized for `{env}`"));
    report.next = vec![
        format!("stellar forge release status {env}"),
        format!("stellar forge release diff {env}"),
        format!("stellar forge release history {env}"),
    ];
    report.data = Some(json!({
        "expected": release_artifact_summary_value(&current_path, &expected_artifact, "expected"),
        "current": current.as_ref().map(|(_, summary)| summary.clone()),
        "latest_history": latest_history.as_ref().map(|(_, summary)| summary.clone()),
        "history_count": history_paths.len(),
        "drift": {
            "current_vs_expected": current_vs_expected,
            "latest_history_vs_expected": latest_history_vs_expected,
            "current_vs_latest_history": current_vs_latest_history,
        }
    }));
    Ok(report)
}

pub fn release_diff(context: &AppContext, env: &str, path: Option<&Path>) -> Result<CommandReport> {
    let mut report = CommandReport::new("release.diff");
    let manifest = load_manifest(context)?;
    if !manifest.networks.contains_key(env) {
        bail!("network `{env}` not found");
    }
    let lockfile = load_lockfile(context)?;
    let root = context.project_root();
    let baseline_path = release_artifact_path(&root, env);
    let baseline_value = if baseline_path.exists() {
        Some(read_release_artifact_value(&baseline_path)?)
    } else {
        None
    };

    let comparison_path = match path {
        Some(path) if path.is_absolute() => path.to_path_buf(),
        Some(path) => root.join(path),
        None => release_history_artifacts(&root, env)?
            .last()
            .cloned()
            .unwrap_or_else(|| baseline_path.clone()),
    };
    if !comparison_path.exists() {
        bail!("release artifact `{}` not found", comparison_path.display());
    }
    let comparison_value = read_release_artifact_value(&comparison_path)?;
    if let Some(artifact_env) = comparison_value.get("environment").and_then(Value::as_str)
        && artifact_env != env
    {
        bail!("artifact environment `{artifact_env}` does not match requested environment `{env}`");
    }

    let (base_kind, base_value) = if let Some(base_value) = baseline_value {
        ("current", base_value)
    } else {
        (
            "expected",
            build_release_artifact(&manifest, &lockfile, env)?,
        )
    };
    let comparison_kind = if path.is_some() {
        "selected"
    } else if comparison_path == baseline_path {
        "current"
    } else {
        "history"
    };
    let issues = release_artifact_diff(&base_value, &comparison_value);
    let summary_path = if comparison_path.is_absolute() {
        comparison_path.clone()
    } else {
        root.join(&comparison_path)
    };
    let comparison_summary =
        release_artifact_summary_value(&summary_path, &comparison_value, comparison_kind);
    report.status = if issues.is_empty() {
        "ok".to_string()
    } else {
        "warn".to_string()
    };
    report.network = Some(env.to_string());
    report.message = Some(format!(
        "release diff compared `{}` with {}",
        baseline_path.display(),
        comparison_path.display()
    ));
    report.next = vec![
        format!("stellar forge release inspect {env}"),
        format!("stellar forge release prune {env}"),
    ];
    report.data = Some(json!({
        "base": release_artifact_summary_value(&baseline_path, &base_value, base_kind),
        "comparison": comparison_summary,
        "issues": issues,
    }));
    Ok(report)
}

pub fn release_prune(context: &AppContext, env: &str, keep: usize) -> Result<CommandReport> {
    let mut report = CommandReport::new("release.prune");
    let manifest = load_manifest(context)?;
    if !manifest.networks.contains_key(env) {
        bail!("network `{env}` not found");
    }
    let root = context.project_root();
    let history = release_history_artifacts(&root, env)?;
    let keep = keep.min(history.len());
    let prune_count = history.len().saturating_sub(keep);
    let prune = history
        .iter()
        .take(prune_count)
        .cloned()
        .collect::<Vec<_>>();
    let retain = history
        .iter()
        .skip(prune_count)
        .cloned()
        .collect::<Vec<_>>();

    for path in &prune {
        report.artifacts.push(path.display().to_string());
        if !context.globals.dry_run {
            fs::remove_file(path).with_context(|| {
                format!(
                    "failed to remove archived release artifact {}",
                    path.display()
                )
            })?;
        }
    }

    report.status = "ok".to_string();
    report.network = Some(env.to_string());
    report.message = Some(if prune.is_empty() {
        format!("no archived release artifacts pruned for `{env}`")
    } else {
        format!(
            "pruned {} archived release artifact(s) for `{env}`",
            prune.len()
        )
    });
    report.next = vec![format!("stellar forge release status {env}")];
    report.data = Some(json!({
        "keep": keep,
        "pruned": prune.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
        "retained": retain.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
    }));
    Ok(report)
}

pub(super) fn release_state_checks(
    root: &Path,
    manifest: &Manifest,
    lockfile: &Lockfile,
    env: &str,
    strict: bool,
) -> Vec<crate::runtime::CheckResult> {
    let (contracts, tokens, generate_env) = release_resources(manifest, env);
    let environment = lockfile.environments.get(env);
    let missing_status = if strict { "error" } else { "warn" };
    let mut checks = Vec::new();

    for contract in contracts {
        let deployment = environment.and_then(|environment| environment.contracts.get(&contract));
        let status = if deployment.is_some_and(|deployment| !deployment.contract_id.is_empty()) {
            "ok"
        } else {
            missing_status
        };
        let detail = deployment
            .map(|deployment| deployment.contract_id.clone())
            .filter(|value| !value.is_empty())
            .or_else(|| Some("missing deployment in lockfile".to_string()));
        checks.push(check(
            format!("release:{env}:contract:{contract}"),
            status,
            detail,
        ));
        if let (Some(config), Some(deployment)) = (
            manifest.contracts.get(&contract),
            deployment.filter(|deployment| !deployment.contract_id.is_empty()),
        ) && deployment.alias != config.alias
        {
            checks.push(check(
                format!("release:{env}:contract:{contract}:alias"),
                "warn",
                Some(format!(
                    "lockfile alias `{}` differs from manifest alias `{}`",
                    deployment.alias, config.alias
                )),
            ));
        }
    }

    for token_name in tokens {
        let Some(token) = manifest.tokens.get(&token_name) else {
            continue;
        };
        let deployment = environment.and_then(|environment| environment.tokens.get(&token_name));
        let (status, detail) = match deployment {
            None => (missing_status, "missing deployment in lockfile".to_string()),
            Some(deployment) if token.kind == "contract" && deployment.contract_id.is_empty() => (
                missing_status,
                "contract token is missing `contract_id` in lockfile".to_string(),
            ),
            Some(deployment) if token.kind != "contract" && deployment.asset.is_empty() => (
                missing_status,
                "asset token is missing `asset` in lockfile".to_string(),
            ),
            Some(deployment) if token.with_sac && deployment.sac_contract_id.is_empty() => (
                missing_status,
                "token is configured with `with_sac = true` but has no `sac_contract_id` in lockfile"
                    .to_string(),
            ),
            Some(deployment) => (
                "ok",
                if !deployment.sac_contract_id.is_empty() {
                    deployment.sac_contract_id.clone()
                } else if !deployment.contract_id.is_empty() {
                    deployment.contract_id.clone()
                } else {
                    deployment.asset.clone()
                },
            ),
        };
        checks.push(check(
            format!("release:{env}:token:{token_name}"),
            status,
            Some(detail),
        ));
    }

    if generate_env {
        let env_path = root.join(".env.generated");
        checks.push(path_check(
            format!("release:{env}:env-generated"),
            &env_path,
            missing_status,
        ));
        if env_path.exists() {
            checks.push(release_env_consistency_check(
                root, manifest, lockfile, env, strict,
            ));
        }
    }

    let artifact_path = release_artifact_path(root, env);
    checks.push(path_check(
        format!("release:{env}:deploy-artifact"),
        &artifact_path,
        missing_status,
    ));
    if artifact_path.exists() {
        checks.push(release_artifact_consistency_check(
            root, manifest, lockfile, env, strict,
        ));
    }

    checks
}

fn release_env_consistency_check(
    root: &Path,
    manifest: &Manifest,
    lockfile: &Lockfile,
    env: &str,
    strict: bool,
) -> crate::runtime::CheckResult {
    let label = format!("release:{env}:env-generated:consistency");
    let expected = match release_env_lines(manifest, lockfile, env) {
        Ok(lines) => lines
            .into_iter()
            .filter_map(|line| {
                line.split_once('=')
                    .map(|(key, value)| (key.to_string(), value.to_string()))
            })
            .collect::<BTreeMap<String, String>>(),
        Err(error) => {
            return check(
                label,
                if strict { "error" } else { "warn" },
                Some(error.to_string()),
            );
        }
    };
    let actual = fs::read_to_string(root.join(".env.generated"))
        .ok()
        .map(|contents| {
            parse_env_assignments(&contents)
                .into_iter()
                .collect::<BTreeMap<String, String>>()
        })
        .unwrap_or_default();
    let mut issues = Vec::new();
    for (key, value) in expected {
        match actual.get(&key) {
            Some(actual_value) if actual_value == &value => {}
            Some(actual_value) => issues.push(format!(
                "{key} expected `{value}` but found `{actual_value}`"
            )),
            None => issues.push(format!("missing {key}")),
        }
    }
    check(
        label,
        if issues.is_empty() {
            "ok"
        } else if strict {
            "error"
        } else {
            "warn"
        },
        Some(if issues.is_empty() {
            "env matches the current manifest and lockfile".to_string()
        } else {
            issues.join("; ")
        }),
    )
}

fn release_artifact_consistency_check(
    root: &Path,
    manifest: &Manifest,
    lockfile: &Lockfile,
    env: &str,
    strict: bool,
) -> crate::runtime::CheckResult {
    let label = format!("release:{env}:deploy-artifact:consistency");
    let expected = match build_release_artifact(manifest, lockfile, env) {
        Ok(value) => value,
        Err(error) => {
            return check(
                label,
                if strict { "error" } else { "warn" },
                Some(error.to_string()),
            );
        }
    };
    let actual = fs::read_to_string(release_artifact_path(root, env))
        .ok()
        .and_then(|contents| serde_json::from_str::<Value>(&contents).ok())
        .unwrap_or(Value::Null);
    let issues = release_artifact_diff(&expected, &actual);
    check(
        label,
        if issues.is_empty() {
            "ok"
        } else if strict {
            "error"
        } else {
            "warn"
        },
        Some(if issues.is_empty() {
            "deploy snapshot matches the current manifest and lockfile".to_string()
        } else {
            issues.join("; ")
        }),
    )
}

fn read_release_artifact_value(path: &Path) -> Result<Value> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read release artifact {}", path.display()))?;
    serde_json::from_str::<Value>(&raw)
        .with_context(|| format!("failed to parse release artifact {}", path.display()))
}

fn release_artifact_diff(expected: &Value, actual: &Value) -> Vec<String> {
    let mut issues = Vec::new();
    if actual.get("environment") != expected.get("environment") {
        issues.push("environment field differs from the active release target".to_string());
    }
    if actual.pointer("/network/rpc_url") != expected.pointer("/network/rpc_url") {
        issues.push("network.rpc_url differs from the manifest".to_string());
    }
    if actual.pointer("/network/horizon_url") != expected.pointer("/network/horizon_url") {
        issues.push("network.horizon_url differs from the manifest".to_string());
    }
    diff_named_release_entries(actual, expected, "contracts", "contract", &mut issues);
    diff_named_release_entries(actual, expected, "tokens", "token", &mut issues);
    issues
}

fn diff_named_release_entries(
    actual: &Value,
    expected: &Value,
    section: &str,
    singular: &str,
    issues: &mut Vec<String>,
) {
    let actual_entries = actual
        .get(section)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let expected_entries = expected
        .get(section)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    for (name, expected_entry) in &expected_entries {
        match actual_entries.get(name) {
            Some(actual_entry) if actual_entry == expected_entry => {}
            Some(_) => issues.push(format!(
                "{singular} `{name}` differs from the current lockfile"
            )),
            None => issues.push(format!("missing {singular} `{name}` in deploy snapshot")),
        }
    }
    for name in actual_entries.keys() {
        if !expected_entries.contains_key(name) {
            issues.push(format!("unexpected {singular} `{name}` in deploy snapshot"));
        }
    }
}

pub(super) fn probe_release_deployments(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: &Manifest,
    env: &str,
    environment: &crate::model::EnvironmentLock,
) -> Result<Vec<crate::runtime::CheckResult>> {
    let (contracts, tokens, _) = release_resources(manifest, env);
    let mut checks = Vec::new();
    for name in contracts {
        let deployment = environment.contracts.get(&name);
        let Some(contract_id) = deployment
            .map(|deployment| deployment.contract_id.as_str())
            .filter(|contract_id| !contract_id.is_empty())
        else {
            continue;
        };
        checks.push(probe_contract_fetch(
            context,
            report,
            env,
            &format!("network:{env}:contract:{name}"),
            contract_id,
        ));
    }
    for name in tokens {
        let Some(deployment) = environment.tokens.get(&name) else {
            continue;
        };
        if !deployment.sac_contract_id.is_empty() {
            checks.push(probe_contract_fetch(
                context,
                report,
                env,
                &format!("network:{env}:token:{name}:sac"),
                &deployment.sac_contract_id,
            ));
        } else if !deployment.contract_id.is_empty() {
            checks.push(probe_contract_fetch(
                context,
                report,
                env,
                &format!("network:{env}:token:{name}:contract"),
                &deployment.contract_id,
            ));
        }
    }
    Ok(checks)
}

fn probe_contract_fetch(
    context: &AppContext,
    report: &mut CommandReport,
    env: &str,
    label: &str,
    contract_id: &str,
) -> crate::runtime::CheckResult {
    let probe_file = std::env::temp_dir().join(format!(
        "stellar-forge-probe-{}-{}.wasm",
        env,
        label.replace(':', "-")
    ));
    let args = vec![
        "contract".to_string(),
        "fetch".to_string(),
        "--id".to_string(),
        contract_id.to_string(),
        "--out-file".to_string(),
        probe_file.display().to_string(),
        "--network".to_string(),
        env.to_string(),
    ];
    match context.run_command(report, Some(&context.project_root()), "stellar", &args) {
        Ok(_) => {
            let _ = fs::remove_file(&probe_file);
            check(label, "ok", Some(contract_id.to_string()))
        }
        Err(error) => check(
            label,
            "error",
            Some(format!(
                "{error}; the network may have been reset or the lockfile may be stale"
            )),
        ),
    }
}

fn release_plan(context: &AppContext, env: &str) -> Result<CommandReport> {
    let mut report = CommandReport::new("release.plan");
    let manifest = load_manifest(context)?;
    let root = context.project_root();
    let registry_cli = resolve_registry_cli(context);
    let errors = manifest.validate(&root);
    if errors.is_empty() {
        report
            .checks
            .push(check("manifest", "ok", Some("valid".to_string())));
    } else {
        for error in errors {
            report.checks.push(check("manifest", "error", Some(error)));
        }
    }
    if !manifest.networks.contains_key(env) {
        report.checks.push(check(
            "network",
            "error",
            Some(format!("network `{env}` is not defined in the manifest")),
        ));
        report.status = aggregate_status(&report.checks);
        report.network = Some(env.to_string());
        report.message = Some(format!("release plan blocked for `{env}`"));
        return Ok(report);
    }
    let lockfile = load_lockfile(context)?;
    let resources = release_resources(&manifest, env);
    let required_identities = release_required_identities(&manifest, context, env)?;
    let lockfile_changes = preview_release_lockfile_changes(&manifest, &lockfile, env);
    let registry_alternatives =
        preview_registry_release_alternatives(context, &manifest, &lockfile, env)?;
    report.commands = preview_release_commands(context, &manifest, &lockfile, env)?;
    if resources.2 {
        report
            .artifacts
            .push(root.join(".env.generated").display().to_string());
    }
    report
        .artifacts
        .push(release_artifact_path(&root, env).display().to_string());
    if !registry_alternatives.is_empty() {
        report
            .artifacts
            .push(registry_artifact_path(&root, env).display().to_string());
        report.warnings.push(format!(
            "registry metadata detected for {} contract(s); see `registry_alternatives` for an alternate deploy path",
            registry_alternatives.len()
        ));
        if !registry_cli.available {
            report.warnings.push(format!(
                "registry deploy preview resolves to `{}` but tooling is not ready locally; {}",
                registry_cli.label(),
                registry_cli.detail.as_str()
            ));
        }
    }
    report.status = aggregate_status(&report.checks);
    report.network = Some(env.to_string());
    report.message = Some(format!(
        "{} contracts and {} tokens planned for `{env}`",
        resources.0.len(),
        resources.1.len()
    ));
    report.data = Some(json!({
        "contracts": resources.0,
        "tokens": resources.1,
        "generate_env": resources.2,
        "required_identities": required_identities,
        "lockfile_changes": lockfile_changes,
        "registry_alternatives": registry_alternatives,
    }));
    Ok(report)
}

fn release_deploy(context: &AppContext, env: &str, confirm_mainnet: bool) -> Result<CommandReport> {
    let manifest = load_manifest(context)?;
    let network = release_network(&manifest, env)?;
    if network.kind == "pubnet" && !confirm_mainnet {
        bail!("mainnet deploy requires --confirm-mainnet");
    }
    let mut report = CommandReport::new("release.deploy");
    let (contracts, tokens, generate_env) = release_resources(&manifest, env);
    for token_name in &tokens {
        token::token_create_from_manifest(context, &mut report, &manifest, token_name, env)?;
    }
    for contract_name in &contracts {
        deploy_contract_from_manifest(context, &mut report, &manifest, contract_name, env)?;
    }
    if generate_env {
        let exported = release_env_export(context, env)?;
        report.artifacts.extend(exported.artifacts);
        report.commands.extend(exported.commands);
    } else {
        let lockfile = load_lockfile(context)?;
        write_release_artifact(context, &mut report, &manifest, &lockfile, env)?;
    }
    report.network = Some(env.to_string());
    report.message = Some(format!("release deployed to `{env}`"));
    report.data = Some(json!({
        "contracts": contracts,
        "tokens": tokens,
    }));
    Ok(report)
}

fn release_rollback(context: &AppContext, env: &str, to: Option<&Path>) -> Result<CommandReport> {
    let mut report = CommandReport::new("release.rollback");
    let manifest = load_manifest(context)?;
    if !manifest.networks.contains_key(env) {
        bail!("network `{env}` not found");
    }
    let root = context.project_root();
    let source_path = match to {
        Some(path) if path.is_absolute() => path.to_path_buf(),
        Some(path) => root.join(path),
        None => latest_release_history_artifact(&root, env)?,
    };
    let raw = context.read_text(&source_path)?;
    let artifact = serde_json::from_str::<Value>(&raw)
        .with_context(|| format!("failed to parse release artifact {}", source_path.display()))?;
    let restored = release_environment_from_artifact(&artifact, env)?;
    let contract_names = restored.contracts.keys().cloned().collect::<Vec<_>>();
    let token_names = restored.tokens.keys().cloned().collect::<Vec<_>>();

    let mut lockfile = load_lockfile(context)?;
    lockfile.environments.insert(env.to_string(), restored);

    let exported = release_env_export_with_lockfile(context, &manifest, &lockfile, env)?;
    report.commands.extend(exported.commands);
    report.artifacts.extend(exported.artifacts);
    report.warnings.extend(exported.warnings);
    save_lockfile(context, &mut report, &lockfile)?;
    report.warnings.push(
        "rollback restored local release metadata from a deploy snapshot; it does not revert on-chain state"
            .to_string(),
    );
    report.network = Some(env.to_string());
    report.message = Some(format!(
        "release metadata for `{env}` restored from {}",
        source_path.display()
    ));
    report.next = vec![
        format!("stellar forge release verify {env}"),
        format!("stellar forge release aliases sync {env}"),
    ];
    report.data = Some(json!({
        "source_artifact": source_path.display().to_string(),
        "contracts": contract_names,
        "tokens": token_names,
    }));
    Ok(report)
}

fn release_history(context: &AppContext, env: &str) -> Result<CommandReport> {
    let mut report = CommandReport::new("release.history");
    let manifest = load_manifest(context)?;
    if !manifest.networks.contains_key(env) {
        bail!("network `{env}` not found");
    }
    let root = context.project_root();
    let current_path = release_artifact_path(&root, env);
    let current = if current_path.exists() {
        let (summary, warning) = release_artifact_summary(&current_path, "current")?;
        if let Some(warning) = warning {
            report.warnings.push(warning);
        }
        Some(summary)
    } else {
        None
    };
    let history = release_history_artifacts(&root, env)?
        .into_iter()
        .rev()
        .map(|path| {
            let (summary, warning) = release_artifact_summary(&path, "history")?;
            if let Some(warning) = warning {
                report.warnings.push(warning);
            }
            Ok(summary)
        })
        .collect::<Result<Vec<_>>>()?;
    report.network = Some(env.to_string());
    report.status = if current.is_none() && history.is_empty() {
        "warn".to_string()
    } else {
        "ok".to_string()
    };
    report.message = Some(format!("release artifact history listed for `{env}`"));
    report.next = vec![
        format!("stellar forge release inspect {env}"),
        format!("stellar forge release rollback {env}"),
    ];
    report.data = Some(json!({
        "current": current,
        "history": history,
    }));
    Ok(report)
}

fn release_inspect(context: &AppContext, env: &str, path: Option<&Path>) -> Result<CommandReport> {
    let mut report = CommandReport::new("release.inspect");
    let manifest = load_manifest(context)?;
    if !manifest.networks.contains_key(env) {
        bail!("network `{env}` not found");
    }
    let lockfile = load_lockfile(context)?;
    let root = context.project_root();
    let selected_path = match path {
        Some(path) if path.is_absolute() => path.to_path_buf(),
        Some(path) => root.join(path),
        None => release_artifact_path(&root, env),
    };
    if !selected_path.exists() {
        bail!("release artifact `{}` not found", selected_path.display());
    }
    let artifact = read_release_artifact_value(&selected_path)?;
    if let Some(artifact_env) = artifact.get("environment").and_then(Value::as_str)
        && artifact_env != env
    {
        bail!("artifact environment `{artifact_env}` does not match requested environment `{env}`");
    }
    let expected = build_release_artifact(&manifest, &lockfile, env)?;
    let issues = release_artifact_diff(&expected, &artifact);
    report.network = Some(env.to_string());
    report.status = if issues.is_empty() {
        "ok".to_string()
    } else {
        "warn".to_string()
    };
    report.message = Some(format!(
        "release artifact inspected for `{env}` from {}",
        selected_path.display()
    ));
    report.next = vec![
        format!("stellar forge release verify {env}"),
        format!(
            "stellar forge release rollback {env} --to {}",
            selected_path.display()
        ),
    ];
    report.data = Some(json!({
        "path": selected_path.display().to_string(),
        "artifact": artifact,
        "summary": release_artifact_summary_value(
            &selected_path,
            &artifact,
            if selected_path == release_artifact_path(&root, env) { "current" } else { "history" }
        ),
        "comparison": {
            "status": if issues.is_empty() { "ok" } else { "warn" },
            "issues": issues,
        },
    }));
    Ok(report)
}

pub(super) fn release_verify(context: &AppContext, env: &str) -> Result<CommandReport> {
    let mut report = CommandReport::new("release.verify");
    let manifest = load_manifest(context)?;
    if !manifest.networks.contains_key(env) {
        bail!("network `{env}` not found");
    }
    let lockfile = load_lockfile(context)?;
    report.checks.extend(release_state_checks(
        &context.project_root(),
        &manifest,
        &lockfile,
        env,
        true,
    ));
    report.checks.extend(release_registry_artifact_checks(
        &context.project_root(),
        &manifest,
        &lockfile,
        env,
        true,
    ));
    if let Some(events_check) = event_worker_config_check(
        &context.project_root(),
        &manifest,
        true,
        &format!("release:{env}:events:config"),
    ) {
        report.checks.push(events_check);
    }
    if let Some(environment) = lockfile.environments.get(env) {
        if !context.globals.dry_run && context.command_exists("stellar") {
            let probe_checks =
                probe_release_deployments(context, &mut report, &manifest, env, environment)?;
            report.checks.extend(probe_checks);
        } else if !environment.contracts.is_empty() || !environment.tokens.is_empty() {
            report.warnings.push(
                "skipped on-chain contract fetch probes; run without `--dry-run` on a machine with `stellar` configured to verify deployed IDs".to_string(),
            );
        }
    }
    report.status = aggregate_status(&report.checks);
    report.network = Some(env.to_string());
    report.message = Some("release verification completed".to_string());
    Ok(report)
}

fn release_aliases_sync(context: &AppContext, env: &str) -> Result<CommandReport> {
    let mut report = CommandReport::new("release.aliases.sync");
    let manifest = load_manifest(context)?;
    let lockfile = load_lockfile(context)?;
    let environment = lockfile
        .environments
        .get(env)
        .ok_or_else(|| anyhow!("no release state found for `{env}` in stellarforge.lock.json"))?;
    let (contracts, tokens, _) = release_resources(&manifest, env);
    let mut synced = Vec::new();
    let mut missing = Vec::new();

    for contract_name in contracts {
        let Some(contract) = manifest.contracts.get(&contract_name) else {
            continue;
        };
        let Some(contract_id) = environment
            .contracts
            .get(&contract_name)
            .map(|deployment| deployment.contract_id.clone())
            .filter(|contract_id| !contract_id.is_empty())
        else {
            missing.push(format!(
                "contract `{contract_name}` has no deployed contract_id"
            ));
            continue;
        };
        context.run_command(
            &mut report,
            Some(&context.project_root()),
            "stellar",
            &[
                "contract".to_string(),
                "alias".to_string(),
                "add".to_string(),
                "--overwrite".to_string(),
                "--id".to_string(),
                contract_id.clone(),
                contract.alias.clone(),
                "--network".to_string(),
                env.to_string(),
            ],
        )?;
        synced.push(json!({
            "resource": format!("contract:{contract_name}"),
            "alias": contract.alias,
            "contract_id": contract_id,
        }));
    }

    for token_name in tokens {
        let Some(token) = manifest.tokens.get(&token_name) else {
            continue;
        };
        if token.kind == "contract" || !token.with_sac {
            continue;
        }
        let Some(contract_id) = environment
            .tokens
            .get(&token_name)
            .map(|deployment| deployment.sac_contract_id.clone())
            .filter(|contract_id| !contract_id.is_empty())
        else {
            missing.push(format!(
                "token `{token_name}` has no deployed SAC contract_id"
            ));
            continue;
        };
        let alias = format!("{token_name}-sac");
        context.run_command(
            &mut report,
            Some(&context.project_root()),
            "stellar",
            &[
                "contract".to_string(),
                "alias".to_string(),
                "add".to_string(),
                "--overwrite".to_string(),
                "--id".to_string(),
                contract_id.clone(),
                alias.clone(),
                "--network".to_string(),
                env.to_string(),
            ],
        )?;
        synced.push(json!({
            "resource": format!("token:{token_name}:sac"),
            "alias": alias,
            "contract_id": contract_id,
        }));
    }

    if !missing.is_empty() {
        report.warnings.extend(missing.clone());
    }
    report.status = if missing.is_empty() {
        "ok".to_string()
    } else {
        "warn".to_string()
    };
    report.network = Some(env.to_string());
    report.message = Some(format!("release aliases synchronized for `{env}`"));
    report.data = Some(json!({
        "synced": synced,
        "missing": missing,
    }));
    Ok(report)
}

pub(super) fn release_env_export(context: &AppContext, env: &str) -> Result<CommandReport> {
    let manifest = load_manifest(context)?;
    let lockfile = load_lockfile(context)?;
    release_env_export_with_lockfile(context, &manifest, &lockfile, env)
}

fn release_env_export_with_lockfile(
    context: &AppContext,
    manifest: &Manifest,
    lockfile: &Lockfile,
    env: &str,
) -> Result<CommandReport> {
    let mut report = CommandReport::new("release.env.export");
    let env_lines = release_env_lines(manifest, lockfile, env)?;
    context.write_text(
        &mut report,
        &context.project_root().join(".env.generated"),
        &(env_lines.join("\n") + "\n"),
    )?;
    write_release_artifact(context, &mut report, manifest, lockfile, env)?;
    if manifest
        .frontend
        .as_ref()
        .is_some_and(|frontend| frontend.enabled)
    {
        sync_frontend_generated_state_with_lockfile(
            context,
            &mut report,
            &context.project_root(),
            manifest,
            lockfile,
            env,
        )?;
    }
    report.network = Some(env.to_string());
    report.message = Some(format!(
        "environment export and deploy snapshot written for `{env}`"
    ));
    Ok(report)
}

fn release_registry_publish(context: &AppContext, contract_name: &str) -> Result<CommandReport> {
    let mut report = CommandReport::new("release.registry.publish");
    let manifest = load_manifest(context)?;
    let env = release_active_env(&manifest, context)?;
    let registry_cli = resolve_registry_cli(context);
    let contract = manifest
        .contracts
        .get(contract_name)
        .ok_or_else(|| anyhow!("contract `{contract_name}` not found"))?;
    let build = contract_build(context, Some(contract_name), false)?;
    report.commands.extend(build.commands);
    report.artifacts.extend(build.artifacts);
    report.warnings.extend(build.warnings);

    let root = context.project_root();
    let contract_dir = root.join(&contract.path);
    let wasm_path = guess_registry_wasm_path(&root, &contract_dir, &env, contract_name);
    let wasm_name = registry_wasm_name(contract_name);
    let version = manifest.project.version.clone();
    let output = registry_cli.run(
        context,
        &mut report,
        Some(&root),
        &[
            "publish".to_string(),
            "--wasm".to_string(),
            path_to_string(&wasm_path)?,
            "--wasm-name".to_string(),
            wasm_name.clone(),
            "--binver".to_string(),
            version.clone(),
            "--network".to_string(),
            env.clone(),
        ],
    )?;
    let published_ref =
        normalized_command_output(&output).unwrap_or_else(|| format!("{wasm_name}@{version}"));
    write_registry_contract_artifact(
        context,
        &mut report,
        &manifest,
        &env,
        contract_name,
        json!({
            "wasm_name": wasm_name.clone(),
            "contract_name": contract.alias.clone(),
            "version": version.clone(),
            "wasm_path": wasm_path.display().to_string(),
            "wasm_hash": registry_wasm_hash(&wasm_path, context.globals.dry_run)?,
            "published_ref": published_ref,
            "published_at": Utc::now().to_rfc3339(),
        }),
    )?;
    report.network = Some(env.clone());
    report.message = Some(format!(
        "registry publish prepared for `{contract_name}` on `{env}`"
    ));
    report.data = Some(json!({
        "contract": contract_name,
        "environment": env,
        "registry_cli": registry_cli.label(),
        "wasm_name": registry_wasm_name(contract_name),
        "version": version,
        "wasm_path": wasm_path.display().to_string(),
        "artifact": registry_artifact_path(&root, &env).display().to_string(),
    }));
    Ok(report)
}

fn release_registry_deploy(context: &AppContext, contract_name: &str) -> Result<CommandReport> {
    let mut report = CommandReport::new("release.registry.deploy");
    let manifest = load_manifest(context)?;
    let env = release_active_env(&manifest, context)?;
    let registry_cli = resolve_registry_cli(context);
    let contract = manifest
        .contracts
        .get(contract_name)
        .ok_or_else(|| anyhow!("contract `{contract_name}` not found"))?;
    let root = context.project_root();
    let contract_dir = root.join(&contract.path);
    let wasm_path = guess_registry_wasm_path(&root, &contract_dir, &env, contract_name);
    let registry_entry = load_registry_contract_artifact(&root, &env, contract_name);
    let wasm_name = registry_entry
        .as_ref()
        .and_then(|entry| entry.get("wasm_name"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| registry_wasm_name(contract_name));
    let version = registry_entry
        .as_ref()
        .and_then(|entry| entry.get("version"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| manifest.project.version.clone());
    if registry_entry.is_none() {
        report.warnings.push(format!(
            "no registry artifact found for `{contract_name}` in `{env}`; using manifest defaults"
        ));
    }

    let output = registry_cli.run(
        context,
        &mut report,
        Some(&root),
        &[
            "deploy".to_string(),
            "--contract-name".to_string(),
            contract.alias.clone(),
            "--wasm-name".to_string(),
            wasm_name.clone(),
            "--version".to_string(),
            version.clone(),
            "--network".to_string(),
            env.clone(),
        ],
    )?;
    registry_cli.run(
        context,
        &mut report,
        Some(&root),
        &["install".to_string(), contract.alias.clone()],
    )?;

    let contract_id = normalized_command_output(&output).unwrap_or_else(|| contract.alias.clone());
    let wasm_hash = registry_wasm_hash(&wasm_path, context.globals.dry_run)?;
    let mut lockfile = load_lockfile(context)?;
    let environment = lockfile.environment_mut(&env);
    environment.contracts.insert(
        contract_name.to_string(),
        ContractDeployment {
            contract_id: contract_id.clone(),
            alias: contract.alias.clone(),
            wasm_hash: wasm_hash.clone(),
            tx_hash: String::new(),
            deployed_at: Some(Utc::now()),
        },
    );
    save_lockfile(context, &mut report, &lockfile)?;

    if let Some(init) = token::contract_effective_init_config(&manifest, contract_name)? {
        let identity = manifest
            .active_identity(context.globals.identity.as_deref())
            .unwrap_or(&manifest.defaults.identity)
            .to_string();
        ensure_identity_exists(context, &mut report, &manifest, &identity, &env, true)?;
        let mut call_args = vec![
            "contract".to_string(),
            "invoke".to_string(),
            "--id".to_string(),
            contract_id.clone(),
            "--source-account".to_string(),
            identity,
            "--network".to_string(),
            env.clone(),
            "--send".to_string(),
            "yes".to_string(),
            "--".to_string(),
            if init.fn_name.is_empty() {
                "init".to_string()
            } else {
                init.fn_name.clone()
            },
        ];
        for (key, value) in &init.args {
            call_args.push(format!("--{key}"));
            call_args.push(resolve_argument_value(
                context,
                &mut report,
                Some(&manifest),
                &env,
                Some(&lockfile),
                value,
            )?);
        }
        context.run_command(&mut report, Some(&root), "stellar", &call_args)?;
    }

    write_registry_contract_artifact(
        context,
        &mut report,
        &manifest,
        &env,
        contract_name,
        json!({
            "wasm_name": wasm_name.clone(),
            "contract_name": contract.alias.clone(),
            "version": version.clone(),
            "wasm_path": wasm_path.display().to_string(),
            "wasm_hash": wasm_hash.clone(),
            "contract_id": contract_id.clone(),
            "installed_alias": manifest
                .contracts
                .get(contract_name)
                .map(|contract| contract.alias.clone())
                .unwrap_or_else(|| contract_name.to_string()),
            "deployed_at": Utc::now().to_rfc3339(),
        }),
    )?;

    let (_, _, generate_env) = release_resources(&manifest, &env);
    if generate_env {
        let exported = release_env_export(context, &env)?;
        report.commands.extend(exported.commands);
        report.artifacts.extend(exported.artifacts);
        report.warnings.extend(exported.warnings);
    } else {
        write_release_artifact(context, &mut report, &manifest, &lockfile, &env)?;
    }

    report.network = Some(env.clone());
    report.message = Some(format!(
        "registry deploy completed for `{contract_name}` on `{env}`"
    ));
    report.data = Some(json!({
        "contract": contract_name,
        "environment": env,
        "registry_cli": registry_cli.label(),
        "wasm_name": wasm_name,
        "version": version,
        "alias": manifest
            .contracts
            .get(contract_name)
            .map(|contract| contract.alias.clone())
            .unwrap_or_else(|| contract_name.to_string()),
        "contract_id": contract_id,
        "artifact": registry_artifact_path(&root, &env).display().to_string(),
    }));
    Ok(report)
}

pub(super) fn release_env_lines(
    manifest: &Manifest,
    lockfile: &Lockfile,
    env: &str,
) -> Result<Vec<String>> {
    let network = manifest
        .networks
        .get(env)
        .ok_or_else(|| anyhow!("network `{env}` not found"))?;
    let environment = lockfile.environments.get(env).cloned().unwrap_or_default();
    let mut env_lines = vec![
        format!("PUBLIC_STELLAR_NETWORK={env}"),
        format!("PUBLIC_STELLAR_RPC_URL={}", network.rpc_url),
    ];
    for (name, contract) in environment.contracts {
        if !contract.contract_id.is_empty() {
            env_lines.push(format!(
                "PUBLIC_{}_CONTRACT_ID={}",
                shouty(&name),
                contract.contract_id
            ));
        }
    }
    for (name, token) in environment.tokens {
        if !token.asset.is_empty() {
            env_lines.push(format!("PUBLIC_{}_ASSET={}", shouty(&name), token.asset));
        }
        if !token.sac_contract_id.is_empty() {
            env_lines.push(format!(
                "PUBLIC_{}_SAC_ID={}",
                shouty(&name),
                token.sac_contract_id
            ));
        }
        if !token.contract_id.is_empty() {
            env_lines.push(format!(
                "PUBLIC_{}_TOKEN_ID={}",
                shouty(&name),
                token.contract_id
            ));
        }
    }
    Ok(env_lines)
}

fn release_registry_artifact_checks(
    root: &Path,
    manifest: &Manifest,
    lockfile: &Lockfile,
    env: &str,
    strict: bool,
) -> Vec<crate::runtime::CheckResult> {
    let path = registry_artifact_path(root, env);
    if !path.exists() {
        return Vec::new();
    }
    let artifact = load_registry_artifact(root, env, Some(manifest));
    let contracts = artifact
        .get("contracts")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let (release_contracts, _, _) = release_resources(manifest, env);
    let mut checks = vec![check(
        format!("release:{env}:registry:artifact"),
        "ok",
        Some(path.display().to_string()),
    )];
    for contract_name in release_contracts {
        let Some(entry) = contracts.get(&contract_name) else {
            continue;
        };
        let issues = registry_contract_artifact_issues(lockfile, env, &contract_name, entry);
        checks.push(check(
            format!("release:{env}:registry:artifact:{contract_name}"),
            if issues.is_empty() {
                "ok"
            } else if strict {
                "error"
            } else {
                "warn"
            },
            Some(if issues.is_empty() {
                format!("registry artifact and lockfile align for `{contract_name}`")
            } else {
                issues.join("; ")
            }),
        ));
    }
    checks
}

fn registry_contract_artifact_issues(
    lockfile: &Lockfile,
    env: &str,
    contract_name: &str,
    entry: &Value,
) -> Vec<String> {
    let Some(entry_object) = entry.as_object() else {
        return vec!["registry artifact entry is not a JSON object".to_string()];
    };
    let artifact_contract_id = entry_object
        .get("contract_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let artifact_wasm_hash = entry_object
        .get("wasm_hash")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let artifact_alias = entry_object
        .get("installed_alias")
        .and_then(Value::as_str)
        .or_else(|| entry_object.get("contract_name").and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let deployment = lockfile
        .environments
        .get(env)
        .and_then(|environment| environment.contracts.get(contract_name));

    let mut issues = Vec::new();
    if let Some(contract_id) = artifact_contract_id {
        let Some(current) = deployment else {
            issues.push(
                "registry artifact has `contract_id` but lockfile has no deployment".to_string(),
            );
            return issues;
        };
        if contract_id != current.contract_id {
            issues.push(format!(
                "registry contract_id `{contract_id}` differs from lockfile `{}`",
                current.contract_id
            ));
        }
    }
    if let Some(wasm_hash) = artifact_wasm_hash
        && let Some(current) = deployment
        && !current.wasm_hash.is_empty()
        && wasm_hash != current.wasm_hash
    {
        issues.push(format!(
            "registry wasm_hash `{wasm_hash}` differs from lockfile `{}`",
            current.wasm_hash
        ));
    }
    if let Some(alias) = artifact_alias
        && let Some(current) = deployment
        && !current.alias.is_empty()
        && alias != current.alias
    {
        issues.push(format!(
            "registry alias `{alias}` differs from lockfile `{}`",
            current.alias
        ));
    }
    issues
}

pub(super) fn write_release_artifact(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: &Manifest,
    lockfile: &Lockfile,
    env: &str,
) -> Result<()> {
    let artifact = build_release_artifact(manifest, lockfile, env)?;
    let rendered = serde_json::to_string_pretty(&artifact)?;
    archive_existing_release_artifact(context, report, &context.project_root(), env, &rendered)?;
    context.write_text(
        report,
        &release_artifact_path(&context.project_root(), env),
        &rendered,
    )
}

pub(super) fn release_artifact_path(root: &Path, env: &str) -> PathBuf {
    root.join("dist").join(format!("deploy.{env}.json"))
}

fn release_history_dir(root: &Path) -> PathBuf {
    root.join("dist").join("history")
}

fn release_history_artifact_path(root: &Path, env: &str) -> PathBuf {
    let timestamp = Utc::now().format("%Y%m%dT%H%M%S%.9fZ");
    release_history_dir(root).join(format!("deploy.{env}.{timestamp}.json"))
}

fn archive_existing_release_artifact(
    context: &AppContext,
    report: &mut CommandReport,
    root: &Path,
    env: &str,
    new_contents: &str,
) -> Result<Option<PathBuf>> {
    let path = release_artifact_path(root, env);
    let Ok(existing) = fs::read_to_string(&path) else {
        return Ok(None);
    };
    if existing == new_contents {
        return Ok(None);
    }
    let archive_path = release_history_artifact_path(root, env);
    context.write_text(report, &archive_path, &existing)?;
    Ok(Some(archive_path))
}

fn latest_release_history_artifact(root: &Path, env: &str) -> Result<PathBuf> {
    let mut candidates = release_history_artifacts(root, env)?;
    candidates.pop().ok_or_else(|| {
        anyhow!(
            "no release history found for `{env}`; run another release or pass `--to <artifact>`"
        )
    })
}

fn release_history_artifacts(root: &Path, env: &str) -> Result<Vec<PathBuf>> {
    let history_dir = release_history_dir(root);
    let prefix = format!("deploy.{env}.");
    let entries = match fs::read_dir(&history_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "failed to read release history directory {}",
                    history_dir.display()
                )
            });
        }
    };
    let mut candidates = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| name.starts_with(&prefix) && name.ends_with(".json"))
        })
        .collect::<Vec<_>>();
    candidates.sort();
    Ok(candidates)
}

fn release_network<'a>(manifest: &'a Manifest, env: &str) -> Result<&'a NetworkConfig> {
    manifest
        .networks
        .get(env)
        .ok_or_else(|| anyhow!("network `{env}` not found"))
}

fn release_environment_from_artifact(artifact: &Value, env: &str) -> Result<EnvironmentLock> {
    if let Some(artifact_env) = artifact.get("environment").and_then(Value::as_str)
        && artifact_env != env
    {
        bail!("artifact environment `{artifact_env}` does not match requested environment `{env}`");
    }
    let contracts = artifact
        .get("contracts")
        .cloned()
        .ok_or_else(|| anyhow!("release artifact is missing `contracts`"))?;
    let tokens = artifact
        .get("tokens")
        .cloned()
        .ok_or_else(|| anyhow!("release artifact is missing `tokens`"))?;
    Ok(EnvironmentLock {
        contracts: serde_json::from_value(contracts)
            .context("release artifact contracts are invalid")?,
        tokens: serde_json::from_value(tokens).context("release artifact tokens are invalid")?,
    })
}

fn release_artifact_summary(path: &Path, kind: &str) -> Result<(Value, Option<String>)> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read release artifact {}", path.display()))?;
    let artifact = serde_json::from_str::<Value>(&raw)
        .with_context(|| format!("failed to parse release artifact {}", path.display()))?;
    Ok((release_artifact_summary_value(path, &artifact, kind), None))
}

fn release_artifact_summary_value(path: &Path, artifact: &Value, kind: &str) -> Value {
    let contracts = artifact
        .get("contracts")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let tokens = artifact
        .get("tokens")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    json!({
        "kind": kind,
        "path": path.display().to_string(),
        "project": artifact.pointer("/project/slug").or_else(|| artifact.get("project")).cloned().unwrap_or(Value::Null),
        "environment": artifact.get("environment").cloned().unwrap_or(Value::Null),
        "updated_at": artifact.get("updated_at").cloned().unwrap_or(Value::Null),
        "contracts": {
            "count": contracts.len(),
            "names": contracts.keys().cloned().collect::<Vec<_>>(),
        },
        "tokens": {
            "count": tokens.len(),
            "names": tokens.keys().cloned().collect::<Vec<_>>(),
        },
    })
}

#[derive(Clone, Debug)]
pub(super) struct RegistryCli {
    program: String,
    prefix: Vec<String>,
    pub(super) available: bool,
    pub(super) detail: String,
}

impl RegistryCli {
    fn label(&self) -> String {
        render_command(&self.program, &self.prefix)
    }

    fn render(&self, args: &[String]) -> String {
        let mut full_args = self.prefix.clone();
        full_args.extend(args.iter().cloned());
        render_command(&self.program, &full_args)
    }

    fn run(
        &self,
        context: &AppContext,
        report: &mut CommandReport,
        cwd: Option<&Path>,
        args: &[String],
    ) -> Result<String> {
        if !self.available && !context.globals.dry_run {
            bail!("{}", self.detail);
        }
        let mut full_args = self.prefix.clone();
        full_args.extend(args.iter().cloned());
        context.run_command(report, cwd, &self.program, &full_args)
    }
}

fn registry_artifact_path(root: &Path, env: &str) -> PathBuf {
    root.join("dist").join(format!("registry.{env}.json"))
}

pub(super) fn resolve_registry_cli(context: &AppContext) -> RegistryCli {
    match env::var("STELLAR_FORGE_REGISTRY_MODE")
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("stellar") | Some("subcommand") | Some("stellar-subcommand") => {
            registry_cli_for_stellar(context, true)
        }
        Some("dedicated") | Some("binary") | Some("stellar-registry") => {
            registry_cli_for_dedicated_binary(context, true)
        }
        _ => {
            if stellar_registry_subcommand_available(context) {
                registry_cli_for_stellar(context, false)
            } else {
                registry_cli_for_dedicated_binary(context, false)
            }
        }
    }
}

fn registry_cli_for_stellar(context: &AppContext, forced: bool) -> RegistryCli {
    let available = stellar_registry_subcommand_available(context);
    let detail = if available {
        "using `stellar registry` from the installed `stellar` CLI".to_string()
    } else if forced {
        "`STELLAR_FORGE_REGISTRY_MODE=stellar` requires a `stellar` CLI that exposes the `registry` subcommand".to_string()
    } else if context.command_exists("stellar") {
        "`stellar` is installed, but it does not expose the `registry` subcommand".to_string()
    } else {
        "`stellar` is not installed".to_string()
    };
    RegistryCli {
        program: "stellar".to_string(),
        prefix: vec!["registry".to_string()],
        available,
        detail,
    }
}

fn registry_cli_for_dedicated_binary(context: &AppContext, forced: bool) -> RegistryCli {
    let available = context.command_exists("stellar-registry");
    let detail = if available {
        if forced {
            "using dedicated `stellar-registry` because `STELLAR_FORGE_REGISTRY_MODE=dedicated` is set".to_string()
        } else {
            "using dedicated `stellar-registry` because `stellar registry` is unavailable"
                .to_string()
        }
    } else if forced {
        "`STELLAR_FORGE_REGISTRY_MODE=dedicated` requires the `stellar-registry` binary in PATH"
            .to_string()
    } else if context.command_exists("stellar") {
        "`stellar registry` is unavailable in the installed `stellar` CLI; install `stellar-registry` to run registry workflows".to_string()
    } else {
        "install `stellar-registry` to run registry workflows".to_string()
    };
    RegistryCli {
        program: "stellar-registry".to_string(),
        prefix: Vec::new(),
        available,
        detail,
    }
}

fn stellar_registry_subcommand_available(context: &AppContext) -> bool {
    context.command_succeeds(None, "stellar", &["registry", "--help"])
}

pub(super) fn project_has_registry_artifacts(root: &Path) -> bool {
    let Ok(entries) = fs::read_dir(root.join("dist")) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        name.starts_with("registry.") && name.ends_with(".json")
    })
}

fn release_active_env(manifest: &Manifest, context: &AppContext) -> Result<String> {
    Ok(manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string())
}

fn registry_wasm_name(contract_name: &str) -> String {
    contract_name.to_string()
}

fn guess_registry_wasm_path(root: &Path, contract_dir: &Path, env: &str, name: &str) -> PathBuf {
    let registry_root = root.join("target").join("stellar").join(env);
    let mut candidate_names = vec![name.to_string(), name.replace('-', "_")];
    candidate_names.dedup();
    for candidate_name in candidate_names {
        let candidate = registry_root.join(format!("{candidate_name}.wasm"));
        if candidate.exists() {
            return candidate;
        }
    }
    guess_wasm_path(contract_dir, name)
}

fn normalized_command_output(output: &str) -> Option<String> {
    output
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}

fn registry_wasm_hash(path: &Path, dry_run: bool) -> Result<String> {
    if dry_run || !path.exists() {
        return Ok(String::new());
    }
    Ok(hex_digest(&fs::read(path)?))
}

fn load_registry_contract_artifact(root: &Path, env: &str, contract_name: &str) -> Option<Value> {
    let artifact = load_registry_artifact(root, env, None);
    artifact.get("contracts")?.get(contract_name).cloned()
}

fn load_registry_artifact(root: &Path, env: &str, manifest: Option<&Manifest>) -> Value {
    let path = registry_artifact_path(root, env);
    let mut artifact = fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| default_registry_artifact(env, manifest));
    if let Some(object) = artifact.as_object_mut() {
        object.insert("version".to_string(), json!(1));
        object.insert("environment".to_string(), json!(env));
        if let Some(manifest) = manifest {
            object.insert("project".to_string(), json!(manifest.project.slug.clone()));
        }
    }
    artifact
}

fn default_registry_artifact(env: &str, manifest: Option<&Manifest>) -> Value {
    let mut artifact = serde_json::Map::new();
    artifact.insert("version".to_string(), json!(1));
    artifact.insert("environment".to_string(), json!(env));
    artifact.insert("contracts".to_string(), json!({}));
    if let Some(manifest) = manifest {
        artifact.insert("project".to_string(), json!(manifest.project.slug.clone()));
    }
    Value::Object(artifact)
}

fn write_registry_contract_artifact(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: &Manifest,
    env: &str,
    contract_name: &str,
    entry: Value,
) -> Result<()> {
    let root = context.project_root();
    let mut artifact = load_registry_artifact(&root, env, Some(manifest));
    let entry_object = entry
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow!("registry artifact entry must be a JSON object"))?;
    let artifact_object = artifact
        .as_object_mut()
        .ok_or_else(|| anyhow!("registry artifact root must be a JSON object"))?;
    artifact_object.insert("updated_at".to_string(), json!(Utc::now().to_rfc3339()));
    if let Some(network) = manifest.networks.get(env) {
        artifact_object.insert("network".to_string(), json!(network));
    }
    let contracts_value = artifact_object
        .entry("contracts".to_string())
        .or_insert_with(|| json!({}));
    let contracts_object = contracts_value
        .as_object_mut()
        .ok_or_else(|| anyhow!("registry artifact `contracts` must be a JSON object"))?;
    let mut merged = contracts_object
        .get(contract_name)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for (key, value) in entry_object {
        merged.insert(key, value);
    }
    merged.insert("name".to_string(), json!(contract_name));
    contracts_object.insert(contract_name.to_string(), Value::Object(merged));
    context.write_text(
        report,
        &registry_artifact_path(&root, env),
        &serde_json::to_string_pretty(&artifact)?,
    )
}

pub(super) fn build_release_artifact(
    manifest: &Manifest,
    lockfile: &Lockfile,
    env: &str,
) -> Result<Value> {
    let network = manifest
        .networks
        .get(env)
        .ok_or_else(|| anyhow!("network `{env}` not found"))?;
    let environment = lockfile.environments.get(env).cloned().unwrap_or_default();
    let (contracts, tokens, generate_env) = release_resources(manifest, env);
    let contract_entries = contracts
        .into_iter()
        .map(|name| {
            (
                name.clone(),
                json!(
                    environment
                        .contracts
                        .get(&name)
                        .cloned()
                        .unwrap_or_default()
                ),
            )
        })
        .collect::<serde_json::Map<String, Value>>();
    let token_entries = tokens
        .into_iter()
        .map(|name| {
            (
                name.clone(),
                json!(environment.tokens.get(&name).cloned().unwrap_or_default()),
            )
        })
        .collect::<serde_json::Map<String, Value>>();

    Ok(json!({
        "project": {
            "name": manifest.project.name,
            "slug": manifest.project.slug,
            "version": manifest.project.version,
        },
        "environment": env,
        "generate_env": generate_env,
        "network": {
            "kind": network.kind,
            "rpc_url": network.rpc_url,
            "horizon_url": network.horizon_url,
            "network_passphrase": network.network_passphrase,
            "allow_http": network.allow_http,
            "friendbot": network.friendbot,
        },
        "contracts": contract_entries,
        "tokens": token_entries,
    }))
}

fn release_required_identities(
    manifest: &Manifest,
    context: &AppContext,
    env: &str,
) -> Result<Vec<String>> {
    let (contracts, tokens, _) = release_resources(manifest, env);
    let mut identities = BTreeSet::new();
    identities.insert(
        manifest
            .active_identity(context.globals.identity.as_deref())?
            .to_string(),
    );

    for token_name in tokens {
        let Some(token) = manifest.tokens.get(&token_name) else {
            continue;
        };
        if token.kind != "contract" {
            if let Some(identity) = release_identity_name(manifest, &token.issuer) {
                identities.insert(identity);
            }
            if let Some(identity) = release_identity_name(manifest, &token.distribution) {
                identities.insert(identity);
            }
        }
    }

    for contract_name in contracts {
        let Some(init) = token::contract_effective_init_config(manifest, &contract_name)? else {
            continue;
        };
        for value in init.args.values() {
            if let Some(identity) = release_identity_name(manifest, value) {
                identities.insert(identity);
            }
        }
    }

    Ok(identities.into_iter().collect())
}

fn release_identity_name(manifest: &Manifest, input: &str) -> Option<String> {
    if let Some(reference) = parse_manifest_ref(input) {
        return match reference {
            ManifestRef::Identity(identity) => Some(identity),
            ManifestRef::Wallet(wallet) => manifest
                .wallets
                .get(&wallet)
                .and_then(wallet_runtime_identity),
            _ => None,
        };
    }
    if manifest.identities.contains_key(input) {
        return Some(input.to_string());
    }
    manifest
        .wallets
        .get(input)
        .and_then(wallet_runtime_identity)
}

fn preview_release_lockfile_changes(
    manifest: &Manifest,
    lockfile: &Lockfile,
    env: &str,
) -> Vec<Value> {
    let (contracts, tokens, _) = release_resources(manifest, env);
    let environment = lockfile.environments.get(env);
    let mut changes = Vec::new();

    for contract_name in contracts {
        let deployment =
            environment.and_then(|environment| environment.contracts.get(&contract_name));
        changes.push(json!({
            "resource": format!("contract:{contract_name}"),
            "action": if deployment.is_some_and(|deployment| !deployment.contract_id.is_empty()) { "update" } else { "create" },
            "fields": ["contract_id", "alias", "wasm_hash", "deployed_at"],
        }));
    }

    for token_name in tokens {
        let Some(token) = manifest.tokens.get(&token_name) else {
            continue;
        };
        let deployment = environment.and_then(|environment| environment.tokens.get(&token_name));
        let mut fields = if token.kind == "contract" {
            vec!["contract_id"]
        } else {
            vec!["asset", "issuer_identity", "distribution_identity"]
        };
        if token.with_sac {
            fields.push("sac_contract_id");
        }
        let action = if token.kind == "contract" {
            if deployment.is_some_and(|deployment| !deployment.contract_id.is_empty()) {
                "update"
            } else {
                "create"
            }
        } else if deployment.is_some() {
            "update"
        } else {
            "create"
        };
        changes.push(json!({
            "resource": format!("token:{token_name}"),
            "action": action,
            "fields": fields,
        }));
    }

    changes
}

fn preview_release_commands(
    context: &AppContext,
    manifest: &Manifest,
    lockfile: &Lockfile,
    env: &str,
) -> Result<Vec<String>> {
    let (contracts, tokens, _) = release_resources(manifest, env);
    let mut commands = Vec::new();
    for token_name in &tokens {
        commands.extend(preview_token_create_commands(
            context, manifest, lockfile, env, token_name,
        )?);
    }
    for contract_name in &contracts {
        commands.extend(preview_contract_deploy_commands(
            context,
            manifest,
            lockfile,
            env,
            contract_name,
        )?);
    }
    Ok(commands)
}

fn preview_contract_deploy_commands(
    context: &AppContext,
    manifest: &Manifest,
    lockfile: &Lockfile,
    env: &str,
    contract_name: &str,
) -> Result<Vec<String>> {
    let contract = manifest
        .contracts
        .get(contract_name)
        .ok_or_else(|| anyhow!("contract `{contract_name}` not found"))?;
    let source_identity = manifest
        .active_identity(context.globals.identity.as_deref())?
        .to_string();
    let contract_dir = context.project_root().join(&contract.path);
    let wasm_path = guess_wasm_path(&contract_dir, contract_name);
    let mut commands = vec![render_command(
        "stellar",
        &["contract".to_string(), "build".to_string()],
    )];
    commands.push(render_command(
        "stellar",
        &[
            "contract".to_string(),
            "deploy".to_string(),
            "--wasm".to_string(),
            path_to_string(&wasm_path)?,
            "--source-account".to_string(),
            source_identity.clone(),
            "--network".to_string(),
            env.to_string(),
            "--alias".to_string(),
            contract.alias.clone(),
        ],
    ));
    if let Some(init_command) = preview_release_init_command(
        manifest,
        lockfile,
        env,
        contract_name,
        &contract.alias,
        &source_identity,
    )? {
        commands.push(init_command);
    }
    Ok(commands)
}

fn preview_registry_release_alternatives(
    context: &AppContext,
    manifest: &Manifest,
    lockfile: &Lockfile,
    env: &str,
) -> Result<Vec<Value>> {
    let root = context.project_root();
    let registry_path = registry_artifact_path(&root, env);
    if !registry_path.exists() {
        return Ok(Vec::new());
    }
    let (contracts, _, _) = release_resources(manifest, env);
    let mut alternatives = Vec::new();
    for contract_name in contracts {
        let Some(contract) = manifest.contracts.get(&contract_name) else {
            continue;
        };
        let Some(entry) = load_registry_contract_artifact(&root, env, &contract_name) else {
            continue;
        };
        let wasm_name = entry
            .get("wasm_name")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| registry_wasm_name(&contract_name));
        let version = entry
            .get("version")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| manifest.project.version.clone());
        let commands = preview_registry_deploy_commands(
            context,
            manifest,
            lockfile,
            &RegistryDeployPreview {
                env,
                contract_name: &contract_name,
                alias: &contract.alias,
                wasm_name: &wasm_name,
                version: &version,
            },
        )?;
        alternatives.push(json!({
            "contract": contract_name,
            "alias": contract.alias,
            "wasm_name": wasm_name,
            "version": version,
            "artifact": registry_path.display().to_string(),
            "commands": commands,
        }));
    }
    Ok(alternatives)
}

fn preview_registry_deploy_commands(
    context: &AppContext,
    manifest: &Manifest,
    lockfile: &Lockfile,
    preview: &RegistryDeployPreview<'_>,
) -> Result<Vec<String>> {
    let registry_cli = resolve_registry_cli(context);
    let source_identity = manifest
        .active_identity(context.globals.identity.as_deref())?
        .to_string();
    let mut commands = vec![registry_cli.render(&[
        "deploy".to_string(),
        "--contract-name".to_string(),
        preview.alias.to_string(),
        "--wasm-name".to_string(),
        preview.wasm_name.to_string(),
        "--version".to_string(),
        preview.version.to_string(),
        "--network".to_string(),
        preview.env.to_string(),
    ])];
    commands.push(registry_cli.render(&["install".to_string(), preview.alias.to_string()]));
    if let Some(init_command) = preview_release_init_command(
        manifest,
        lockfile,
        preview.env,
        preview.contract_name,
        preview.alias,
        &source_identity,
    )? {
        commands.push(init_command);
    }
    Ok(commands)
}

fn preview_release_init_command(
    manifest: &Manifest,
    lockfile: &Lockfile,
    env: &str,
    contract_name: &str,
    fallback_target: &str,
    source_identity: &str,
) -> Result<Option<String>> {
    let Some(init) = token::contract_effective_init_config(manifest, contract_name)? else {
        return Ok(None);
    };
    let mut args = vec![
        "contract".to_string(),
        "invoke".to_string(),
        "--id".to_string(),
        preview_release_target_id(lockfile, env, contract_name, fallback_target),
        "--source-account".to_string(),
        source_identity.to_string(),
        "--network".to_string(),
        env.to_string(),
        "--send".to_string(),
        "yes".to_string(),
        "--".to_string(),
        if init.fn_name.is_empty() {
            "init".to_string()
        } else {
            init.fn_name.clone()
        },
    ];
    for (key, value) in &init.args {
        args.push(format!("--{key}"));
        args.push(preview_release_argument_value(
            manifest, lockfile, env, value,
        ));
    }
    Ok(Some(render_command("stellar", &args)))
}

fn preview_release_target_id(
    lockfile: &Lockfile,
    env: &str,
    contract_name: &str,
    fallback_target: &str,
) -> String {
    lockfile
        .environments
        .get(env)
        .and_then(|environment| environment.contracts.get(contract_name))
        .map(|deployment| deployment.contract_id.clone())
        .filter(|contract_id| !contract_id.is_empty())
        .unwrap_or_else(|| fallback_target.to_string())
}

fn preview_token_create_commands(
    _context: &AppContext,
    manifest: &Manifest,
    lockfile: &Lockfile,
    env: &str,
    token_name: &str,
) -> Result<Vec<String>> {
    let token = manifest
        .tokens
        .get(token_name)
        .ok_or_else(|| anyhow!("token `{token_name}` not found"))?;
    if token.kind == "contract" {
        return Ok(Vec::new());
    }

    let issuer =
        release_identity_name(manifest, &token.issuer).unwrap_or_else(|| "issuer".to_string());
    let distribution = release_identity_name(manifest, &token.distribution)
        .unwrap_or_else(|| "treasury".to_string());
    let asset = preview_asset_string(manifest, token);
    let mut commands = Vec::new();

    if token.auth_required || token.auth_revocable || token.clawback_enabled {
        let mut args = vec![
            "tx".to_string(),
            "new".to_string(),
            "set-options".to_string(),
            "--source-account".to_string(),
            issuer.clone(),
            "--network".to_string(),
            env.to_string(),
        ];
        if token.auth_required {
            args.push("--set-required".to_string());
        }
        if token.auth_revocable {
            args.push("--set-revocable".to_string());
        }
        if token.clawback_enabled {
            args.push("--set-clawback-enabled".to_string());
        }
        commands.push(render_command("stellar", &args));
    }

    commands.push(render_command(
        "stellar",
        &[
            "tx".to_string(),
            "new".to_string(),
            "change-trust".to_string(),
            "--source-account".to_string(),
            distribution.clone(),
            "--line".to_string(),
            asset.clone(),
            "--network".to_string(),
            env.to_string(),
        ],
    ));

    if token.auth_required {
        commands.push(render_command(
            "stellar",
            &[
                "tx".to_string(),
                "new".to_string(),
                "set-trustline-flags".to_string(),
                "--source-account".to_string(),
                issuer.clone(),
                "--trustor".to_string(),
                preview_address_display(manifest, &token.distribution),
                "--asset".to_string(),
                asset.clone(),
                "--set-authorize".to_string(),
                "--network".to_string(),
                env.to_string(),
            ],
        ));
    }

    if token.with_sac {
        let sac_contract_id = lockfile
            .environments
            .get(env)
            .and_then(|environment| environment.tokens.get(token_name))
            .map(|deployment| deployment.sac_contract_id.clone())
            .filter(|contract_id| !contract_id.is_empty())
            .unwrap_or_else(|| format!("{token_name}-sac"));
        let _ = sac_contract_id;
        commands.push(render_command(
            "stellar",
            &[
                "contract".to_string(),
                "asset".to_string(),
                "deploy".to_string(),
                "--asset".to_string(),
                asset,
                "--source-account".to_string(),
                issuer,
                "--alias".to_string(),
                format!("{token_name}-sac"),
                "--network".to_string(),
                env.to_string(),
            ],
        ));
    }

    Ok(commands)
}

fn preview_release_argument_value(
    manifest: &Manifest,
    lockfile: &Lockfile,
    env: &str,
    value: &str,
) -> String {
    match parse_manifest_ref(value) {
        Some(ManifestRef::Identity(identity)) => format!("<{identity}>"),
        Some(ManifestRef::Wallet(wallet)) => manifest
            .wallets
            .get(&wallet)
            .and_then(wallet_runtime_identity)
            .map(|identity| format!("<{identity}>"))
            .unwrap_or_else(|| format!("<{wallet}>")),
        Some(ManifestRef::TokenSac(token)) => lockfile
            .environments
            .get(env)
            .and_then(|environment| environment.tokens.get(&token))
            .map(|deployment| deployment.sac_contract_id.clone())
            .filter(|contract_id| !contract_id.is_empty())
            .unwrap_or_else(|| format!("{token}-sac")),
        Some(ManifestRef::Contract(contract)) => lockfile
            .environments
            .get(env)
            .and_then(|environment| environment.contracts.get(&contract))
            .map(|deployment| deployment.contract_id.clone())
            .filter(|contract_id| !contract_id.is_empty())
            .or_else(|| {
                manifest
                    .contracts
                    .get(&contract)
                    .map(|contract| contract.alias.clone())
            })
            .unwrap_or(contract),
        _ => value.to_string(),
    }
}

fn preview_address_display(manifest: &Manifest, value: &str) -> String {
    if looks_like_account(value) {
        return value.to_string();
    }
    release_identity_name(manifest, value)
        .map(|identity| format!("<{identity}>"))
        .unwrap_or_else(|| value.to_string())
}

fn preview_asset_string(manifest: &Manifest, token: &TokenConfig) -> String {
    if token.code == "XLM" {
        return "native".to_string();
    }
    format!(
        "{}:{}",
        token.code,
        preview_address_display(manifest, &token.issuer)
    )
}

#[derive(Debug, Clone, Copy)]
struct RegistryDeployPreview<'a> {
    env: &'a str,
    contract_name: &'a str,
    alias: &'a str,
    wasm_name: &'a str,
    version: &'a str,
}

pub(super) fn release_resources(
    manifest: &Manifest,
    env: &str,
) -> (Vec<String>, Vec<String>, bool) {
    if let Some(release) = manifest.release.get(env) {
        return (
            release.deploy_contracts.clone(),
            release.deploy_tokens.clone(),
            release.generate_env,
        );
    }
    (
        manifest.contracts.keys().cloned().collect(),
        manifest.tokens.keys().cloned().collect(),
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::release_resources;
    use crate::model::{ContractConfig, Manifest, ReleaseConfig, TokenConfig};
    use std::collections::BTreeMap;

    #[test]
    fn release_resources_falls_back_to_all_declared_entries() {
        let manifest = Manifest {
            contracts: BTreeMap::from([
                ("app".to_string(), ContractConfig::default()),
                ("escrow".to_string(), ContractConfig::default()),
            ]),
            tokens: BTreeMap::from([
                ("credits".to_string(), TokenConfig::default()),
                ("points".to_string(), TokenConfig::default()),
            ]),
            ..Manifest::default()
        };

        let (contracts, tokens, generate_env) = release_resources(&manifest, "testnet");
        assert_eq!(contracts, vec!["app".to_string(), "escrow".to_string()]);
        assert_eq!(tokens, vec!["credits".to_string(), "points".to_string()]);
        assert!(generate_env);
    }

    #[test]
    fn release_resources_respects_release_override() {
        let manifest = Manifest {
            contracts: BTreeMap::from([
                ("app".to_string(), ContractConfig::default()),
                ("escrow".to_string(), ContractConfig::default()),
            ]),
            tokens: BTreeMap::from([
                ("credits".to_string(), TokenConfig::default()),
                ("points".to_string(), TokenConfig::default()),
            ]),
            release: BTreeMap::from([(
                "testnet".to_string(),
                ReleaseConfig {
                    deploy_contracts: vec!["escrow".to_string()],
                    deploy_tokens: vec!["points".to_string()],
                    generate_env: false,
                },
            )]),
            ..Manifest::default()
        };

        let (contracts, tokens, generate_env) = release_resources(&manifest, "testnet");
        assert_eq!(contracts, vec!["escrow".to_string()]);
        assert_eq!(tokens, vec!["points".to_string()]);
        assert!(!generate_env);
    }
}
