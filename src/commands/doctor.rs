use super::*;

pub(super) fn doctor_command(
    context: &AppContext,
    command: Option<DoctorCommand>,
) -> Result<CommandReport> {
    match command {
        None => doctor_all(context),
        Some(DoctorCommand::Env) => doctor_env(context),
        Some(DoctorCommand::Deps) => doctor_deps(context),
        Some(DoctorCommand::Network { env }) => doctor_network(context, Some(&env)),
        Some(DoctorCommand::Project) => doctor_project(context),
    }
}

pub(super) fn scaffold_compatibility_snapshot(
    root: &Path,
    manifest: &Manifest,
    lockfile: &Lockfile,
) -> Result<Option<Value>> {
    let environments = import_scaffold_environments(root)?;
    let bindings = detect_scaffold_bindings(root)?;
    let root_frontend = detect_scaffold_frontend(root);
    let packages_dir = root.join("packages");
    let target_stellar = root.join("target/stellar");
    let traces = scaffold_trace_labels(root, &environments, &bindings, root_frontend);
    if traces.is_empty() {
        return Ok(None);
    }
    Ok(Some(json!({
        "detected": true,
        "traces": traces,
        "root_frontend": root_frontend,
        "managed_api": manifest.api.as_ref().is_some_and(|api| api.enabled),
        "managed_frontend": manifest.frontend.as_ref().is_some_and(|frontend| frontend.enabled),
        "packages_dir": packages_dir.exists(),
        "bindings": bindings,
        "environments": environments.environments,
        "deployments": environments.deployment_counts,
        "lockfile_environments": lockfile.environments.keys().cloned().collect::<Vec<_>>(),
        "target_stellar": target_stellar.exists(),
    })))
}

fn scaffold_trace_labels(
    root: &Path,
    environments: &ScaffoldEnvironmentImport,
    bindings: &BTreeMap<String, Vec<String>>,
    root_frontend: bool,
) -> Vec<String> {
    let mut traces = Vec::new();
    if root.join("contracts").is_dir() {
        traces.push("contracts".to_string());
    }
    if root.join("packages").is_dir() {
        traces.push("packages".to_string());
    }
    if !bindings.is_empty() {
        traces.push("bindings".to_string());
    }
    if !environments.environments.is_empty() {
        traces.push("environments".to_string());
    }
    if root_frontend {
        traces.push("root-frontend".to_string());
    }
    if root.join("target/stellar").is_dir() {
        traces.push("target-stellar".to_string());
    }
    traces
}

fn scaffold_compatibility_checks(
    root: &Path,
    manifest: &Manifest,
    lockfile: &Lockfile,
) -> Result<Vec<crate::runtime::CheckResult>> {
    let environments = import_scaffold_environments(root)?;
    let bindings = detect_scaffold_bindings(root)?;
    let root_frontend = detect_scaffold_frontend(root);
    let traces = scaffold_trace_labels(root, &environments, &bindings, root_frontend);
    if traces.is_empty() {
        return Ok(Vec::new());
    }

    let mut checks = vec![check(
        "compat:scaffold:layout",
        "ok",
        Some(traces.join(", ")),
    )];

    if !bindings.is_empty() || root.join("packages").is_dir() {
        let issues = scaffold_binding_issues(manifest, &bindings);
        checks.push(check(
            "compat:scaffold:packages",
            if issues.is_empty() { "ok" } else { "warn" },
            Some(if issues.is_empty() {
                format!(
                    "{} binding package(s) detected",
                    bindings.values().map(Vec::len).sum::<usize>()
                )
            } else {
                issues.join("; ")
            }),
        ));
    }

    if root_frontend {
        checks.push(check(
            "compat:scaffold:frontend-root",
            "ok",
            Some("root frontend detected and preserved outside managed `apps/web`".to_string()),
        ));
    }

    if root.join("target/stellar").is_dir() {
        checks.push(check(
            "compat:scaffold:target-stellar",
            "ok",
            Some(root.join("target/stellar").display().to_string()),
        ));
    }

    if root.join("environments.toml").exists() {
        let environment_issues = scaffold_environment_issues(manifest, &environments);
        checks.push(check(
            "compat:scaffold:environments",
            if environment_issues.is_empty() {
                "ok"
            } else {
                "warn"
            },
            Some(if environment_issues.is_empty() {
                environments.environments.join(", ")
            } else {
                environment_issues.join("; ")
            }),
        ));
        let deployment_issues = scaffold_deployment_issues(lockfile, &environments);
        checks.push(check(
            "compat:scaffold:deployments",
            if deployment_issues.is_empty() {
                "ok"
            } else {
                "warn"
            },
            Some(if deployment_issues.is_empty() {
                format!(
                    "{} imported deployment(s)",
                    environments
                        .deployment_counts
                        .values()
                        .copied()
                        .sum::<usize>()
                )
            } else {
                deployment_issues.join("; ")
            }),
        ));
    }

    Ok(checks)
}

fn scaffold_binding_issues(
    manifest: &Manifest,
    bindings: &BTreeMap<String, Vec<String>>,
) -> Vec<String> {
    let mut issues = Vec::new();
    for (contract_name, languages) in bindings {
        let Some(contract) = manifest.contracts.get(contract_name) else {
            issues.push(format!(
                "binding package(s) found for undeclared contract `{contract_name}`"
            ));
            continue;
        };
        for language in languages {
            if !contract.bindings.iter().any(|binding| binding == language) {
                issues.push(format!(
                    "manifest contract `{contract_name}` is missing binding `{language}`"
                ));
            }
        }
    }
    issues
}

fn scaffold_environment_issues(
    manifest: &Manifest,
    environments: &ScaffoldEnvironmentImport,
) -> Vec<String> {
    let mut issues = Vec::new();
    for (env_name, imported) in &environments.networks {
        let Some(current) = manifest.networks.get(env_name) else {
            issues.push(format!(
                "manifest is missing imported environment `{env_name}`"
            ));
            continue;
        };
        if imported.rpc_url != current.rpc_url {
            issues.push(format!("environment `{env_name}` rpc_url differs"));
        }
        if imported.horizon_url != current.horizon_url {
            issues.push(format!("environment `{env_name}` horizon_url differs"));
        }
        if imported.network_passphrase != current.network_passphrase {
            issues.push(format!(
                "environment `{env_name}` network_passphrase differs"
            ));
        }
        if imported.allow_http != current.allow_http {
            issues.push(format!("environment `{env_name}` allow_http differs"));
        }
        if imported.friendbot != current.friendbot {
            issues.push(format!("environment `{env_name}` friendbot differs"));
        }
    }
    issues
}

fn scaffold_deployment_issues(
    lockfile: &Lockfile,
    environments: &ScaffoldEnvironmentImport,
) -> Vec<String> {
    let mut issues = Vec::new();
    for (env_name, imported_environment) in &environments.lockfile.environments {
        let Some(current_environment) = lockfile.environments.get(env_name) else {
            issues.push(format!(
                "lockfile is missing imported environment `{env_name}`"
            ));
            continue;
        };
        for (contract_name, imported_deployment) in &imported_environment.contracts {
            let Some(current_deployment) = current_environment.contracts.get(contract_name) else {
                issues.push(format!(
                    "lockfile is missing imported deployment `{env_name}:{contract_name}`"
                ));
                continue;
            };
            if imported_deployment.contract_id != current_deployment.contract_id {
                issues.push(format!(
                    "deployment `{env_name}:{contract_name}` contract_id differs"
                ));
            }
            if imported_deployment.alias != current_deployment.alias {
                issues.push(format!(
                    "deployment `{env_name}:{contract_name}` alias differs"
                ));
            }
            if !imported_deployment.wasm_hash.is_empty()
                && imported_deployment.wasm_hash != current_deployment.wasm_hash
            {
                issues.push(format!(
                    "deployment `{env_name}:{contract_name}` wasm_hash differs"
                ));
            }
        }
    }
    issues
}

fn doctor_all(context: &AppContext) -> Result<CommandReport> {
    let mut report = CommandReport::new("doctor");
    let deps = doctor_deps(context)?;
    let env = doctor_env(context)?;
    report.checks.extend(deps.checks);
    report.checks.extend(env.checks);
    if context.manifest_path.exists() {
        let project = doctor_project(context)?;
        report.checks.extend(project.checks);
        let network = doctor_network(context, None)?;
        report.checks.extend(network.checks);
    } else {
        report.checks.push(check(
            "manifest",
            "warn",
            Some("stellarforge.toml not found; run `stellar forge init` first".to_string()),
        ));
    }
    report.status = aggregate_status(&report.checks);
    report.message = Some("environment and project diagnostics completed".to_string());
    Ok(report)
}

fn doctor_env(context: &AppContext) -> Result<CommandReport> {
    let mut report = CommandReport::new("doctor.env");
    report
        .checks
        .push(check("cwd", "ok", Some(context.cwd.display().to_string())));
    report.checks.push(check(
        "manifest_path",
        if context.manifest_path.exists() {
            "ok"
        } else {
            "warn"
        },
        Some(context.manifest_path.display().to_string()),
    ));
    report.checks.push(check(
        "output_mode",
        "ok",
        Some(
            if context.globals.json {
                "json"
            } else {
                "human"
            }
            .to_string(),
        ),
    ));
    report.checks.push(check(
        "project_root",
        "ok",
        Some(context.project_root().display().to_string()),
    ));
    if context.manifest_path.exists() {
        let manifest = load_manifest(context)?;
        let (network_name, _) = manifest.active_network(context.globals.network.as_deref())?;
        let active_identity = manifest
            .active_identity(context.globals.identity.as_deref())
            .unwrap_or(&manifest.defaults.identity)
            .to_string();
        report.checks.push(check(
            "active_network",
            "ok",
            Some(network_name.to_string()),
        ));
        report
            .checks
            .push(check("active_identity", "ok", Some(active_identity)));
        report.checks.push(check(
            "package_manager",
            "ok",
            Some(manifest.project.package_manager.clone()),
        ));
        report.checks.push(check(
            "api",
            if manifest.api.as_ref().is_some_and(|api| api.enabled) {
                "ok"
            } else {
                "warn"
            },
            Some(if manifest.api.as_ref().is_some_and(|api| api.enabled) {
                "enabled".to_string()
            } else {
                "disabled".to_string()
            }),
        ));
        report.checks.push(check(
            "frontend",
            if manifest
                .frontend
                .as_ref()
                .is_some_and(|frontend| frontend.enabled)
            {
                "ok"
            } else {
                "warn"
            },
            Some(
                manifest
                    .frontend
                    .as_ref()
                    .map(|frontend| frontend.framework.clone())
                    .unwrap_or_else(|| "disabled".to_string()),
            ),
        ));
    }
    report.status = aggregate_status(&report.checks);
    report.message = Some("runtime environment inspected".to_string());
    Ok(report)
}

fn doctor_deps(context: &AppContext) -> Result<CommandReport> {
    let mut report = CommandReport::new("doctor.deps");
    let manifest = if context.manifest_path.exists() {
        Some(load_manifest(context)?)
    } else {
        None
    };
    let requires_node = manifest.as_ref().is_some_and(|manifest| {
        manifest.api.as_ref().is_some_and(|api| api.enabled)
            || manifest
                .frontend
                .as_ref()
                .is_some_and(|frontend| frontend.enabled)
    });
    let requires_rust = manifest
        .as_ref()
        .map(|manifest| !manifest.contracts.is_empty())
        .unwrap_or(true);
    let requires_docker = manifest.as_ref().is_none_or(|manifest| {
        manifest
            .networks
            .values()
            .any(|network| network.kind == "local")
    });
    let requires_sqlite = manifest.as_ref().is_some_and(|manifest| {
        manifest.api.as_ref().is_some_and(|api| api.enabled)
            && manifest
                .api
                .as_ref()
                .is_none_or(|api| api.database == "sqlite")
    });
    for (dependency, required, detail) in [
        (
            "stellar",
            true,
            "required for contract, wallet, and release orchestration",
        ),
        (
            "docker",
            requires_docker,
            if requires_docker {
                "required for local network workflows"
            } else {
                "optional until you use a local network workflow"
            },
        ),
        (
            "cargo",
            requires_rust,
            if requires_rust {
                "required because the project declares Rust contracts"
            } else {
                "optional until you add Rust contracts"
            },
        ),
        (
            "rustc",
            requires_rust,
            if requires_rust {
                "required because the project declares Rust contracts"
            } else {
                "optional until you add Rust contracts"
            },
        ),
        (
            "node",
            requires_node,
            if requires_node {
                "required by API/frontend scaffolds"
            } else {
                "optional until you enable API or frontend scaffolds"
            },
        ),
        (
            "pnpm",
            requires_node,
            if requires_node {
                "required by API/frontend scaffolds"
            } else {
                "optional until you enable API or frontend scaffolds"
            },
        ),
        (
            "sqlite3",
            requires_sqlite,
            if requires_sqlite {
                "required for persisted event backfills and cursor maintenance"
            } else {
                "optional until you persist event data with sqlite"
            },
        ),
    ] {
        let available = context.command_exists(dependency);
        let status = if available {
            "ok"
        } else if required {
            "error"
        } else {
            "warn"
        };
        report
            .checks
            .push(check(dependency, status, Some(detail.to_string())));
    }
    let registry_cli = resolve_registry_cli(context);
    let registry_artifacts_present = project_has_registry_artifacts(&context.project_root());
    let registry_detail = if registry_artifacts_present {
        format!(
            "registry metadata detected in `dist`; {}",
            registry_cli.detail.as_str()
        )
    } else {
        format!(
            "optional until you use `release registry publish|deploy`; {}",
            registry_cli.detail.as_str()
        )
    };
    report.checks.push(check(
        "registry tooling",
        if registry_cli.available { "ok" } else { "warn" },
        Some(registry_detail),
    ));
    if context.command_exists("stellar") {
        if context.globals.dry_run {
            report.commands.push("stellar plugin ls".to_string());
            report.checks.push(check(
                "plugin detection",
                "warn",
                Some("skipped in --dry-run; run `stellar plugin ls` to verify that `stellar-forge` is visible as the `forge` plugin".to_string()),
            ));
        } else {
            let output = context.run_command(
                &mut report,
                None,
                "stellar",
                &["plugin".to_string(), "ls".to_string()],
            )?;
            let status = if output.contains("forge") {
                "ok"
            } else {
                "warn"
            };
            report.checks.push(check(
                "plugin detection",
                status,
                Some(if status == "ok" {
                    "`stellar plugin ls` includes `forge`".to_string()
                } else {
                    "`stellar plugin ls` did not report `forge`; ensure the `stellar-forge` binary is available on PATH".to_string()
                }),
            ));
        }
    }
    report.status = aggregate_status(&report.checks);
    report.message = Some("dependency checks completed".to_string());
    Ok(report)
}

fn doctor_project(context: &AppContext) -> Result<CommandReport> {
    let mut report = CommandReport::new("doctor.project");
    if !context.manifest_path.exists() {
        report.checks.push(check(
            "manifest",
            "error",
            Some("stellarforge.toml not found".to_string()),
        ));
        report.status = "error".to_string();
        return Ok(report);
    }
    let manifest = load_manifest(context)?;
    let errors = manifest.validate(&context.project_root());
    if errors.is_empty() {
        report
            .checks
            .push(check("manifest", "ok", Some("valid".to_string())));
    } else {
        for error in errors {
            report.checks.push(check("manifest", "error", Some(error)));
        }
    }
    append_project_scaffold_checks(&mut report, &context.project_root(), &manifest);
    append_contract_token_checks(&mut report, &manifest);
    if let Some(events_check) =
        event_worker_config_check(&context.project_root(), &manifest, false, "events:config")
    {
        report.checks.push(events_check);
    }
    let lockfile = load_lockfile(context)?;
    report.checks.push(check(
        "lockfile",
        if context
            .project_root()
            .join("stellarforge.lock.json")
            .exists()
        {
            "ok"
        } else {
            "warn"
        },
        Some(format!("{} environments", lockfile.environments.len())),
    ));
    if let Ok((env_name, _)) = manifest.active_network(context.globals.network.as_deref()) {
        report.checks.extend(release_state_checks(
            &context.project_root(),
            &manifest,
            &lockfile,
            env_name,
            false,
        ));
    }
    report
        .checks
        .extend(stale_lockfile_checks(&manifest, &lockfile));
    match scaffold_compatibility_checks(&context.project_root(), &manifest, &lockfile) {
        Ok(checks) => report.checks.extend(checks),
        Err(error) => {
            report
                .checks
                .push(check("compat:scaffold", "error", Some(error.to_string())))
        }
    }
    report.status = aggregate_status(&report.checks);
    report.message = Some("project integrity checked".to_string());
    Ok(report)
}

pub(super) fn doctor_network(
    context: &AppContext,
    env_name: Option<&str>,
) -> Result<CommandReport> {
    let mut report = CommandReport::new("doctor.network");
    let manifest = load_manifest(context)?;
    let (name, network) =
        manifest.active_network(env_name.or(context.globals.network.as_deref()))?;
    report.network = Some(name.to_string());
    if let Ok(rpc_url) = Url::parse(&network.rpc_url) {
        report.commands.push(format!("POST {rpc_url}"));
        if context.globals.dry_run {
            report.checks.push(check(
                "rpc",
                "warn",
                Some(format!("skipped in --dry-run: {}", network.rpc_url)),
            ));
        } else {
            let response = context.post_json(
                &rpc_url,
                &json!({"jsonrpc":"2.0","id":"health","method":"getHealth"}),
            );
            match response {
                Ok(_) => report
                    .checks
                    .push(check("rpc", "ok", Some(network.rpc_url.clone()))),
                Err(error) => report
                    .checks
                    .push(check("rpc", "warn", Some(error.to_string()))),
            }
        }
        if network.kind == "local" {
            report.checks.push(check(
                "rpc-host",
                if rpc_url.host_str().is_some_and(is_loopback_host) {
                    "ok"
                } else {
                    "warn"
                },
                Some(rpc_url.host_str().unwrap_or("<missing-host>").to_string()),
            ));
        }
    }
    if let Ok(horizon_url) = Url::parse(&network.horizon_url) {
        report.commands.push(format!("GET {horizon_url}"));
        if context.globals.dry_run {
            report.checks.push(check(
                "horizon",
                "warn",
                Some(format!("skipped in --dry-run: {}", network.horizon_url)),
            ));
        } else {
            let response = context.get_json(&horizon_url);
            match response {
                Ok(_) => {
                    report
                        .checks
                        .push(check("horizon", "ok", Some(network.horizon_url.clone())))
                }
                Err(error) => report
                    .checks
                    .push(check("horizon", "warn", Some(error.to_string()))),
            }
        }
        if network.kind == "local" {
            report.checks.push(check(
                "horizon-host",
                if horizon_url.host_str().is_some_and(is_loopback_host) {
                    "ok"
                } else {
                    "warn"
                },
                Some(
                    horizon_url
                        .host_str()
                        .unwrap_or("<missing-host>")
                        .to_string(),
                ),
            ));
        }
    }
    let lockfile = load_lockfile(context)?;
    if let Some(environment) = lockfile.environments.get(name) {
        let deployed_resources = environment.contracts.len() + environment.tokens.len();
        if matches!(network.kind.as_str(), "testnet" | "futurenet") && deployed_resources > 0 {
            report.checks.push(check(
                "reset-risk",
                "warn",
                Some(format!(
                    "`{}` can reset and wipe deployed state; run `stellar forge dev reseed --network {name}` if lockfile IDs stop resolving",
                    network.kind
                )),
            ));
        }
        if !context.globals.dry_run && context.command_exists("stellar") {
            let probe_checks =
                probe_release_deployments(context, &mut report, &manifest, name, environment)?;
            report.checks.extend(probe_checks);
        } else if deployed_resources > 0 {
            report.warnings.push(
                "skipped deployed-resource network probes; run without `--dry-run` on a machine with `stellar` configured to verify lockfile IDs against the network".to_string(),
            );
        }
    }
    report.status = aggregate_status(&report.checks);
    report.message = Some("network endpoints probed".to_string());
    Ok(report)
}

fn append_project_scaffold_checks(report: &mut CommandReport, root: &Path, manifest: &Manifest) {
    let required_paths = vec![
        ("scripts:release", root.join("scripts/release.mjs"), "error"),
        ("scripts:doctor", root.join("scripts/doctor.mjs"), "error"),
        ("scripts:reseed", root.join("scripts/reseed.mjs"), "error"),
        (
            "events:ingest-worker",
            root.join("workers/events/ingest-events.mjs"),
            "error",
        ),
        (
            "events:cursor-snapshot",
            root.join("workers/events/cursors.json"),
            "error",
        ),
    ];
    for (label, path, missing_status) in required_paths {
        report.checks.push(path_check(label, &path, missing_status));
    }
    for (name, contract) in &manifest.contracts {
        let contract_root = root.join(&contract.path);
        for (label, path) in [
            (format!("contract:{name}:path"), contract_root.clone()),
            (
                format!("contract:{name}:cargo"),
                contract_root.join("Cargo.toml"),
            ),
            (
                format!("contract:{name}:src"),
                contract_root.join("src/lib.rs"),
            ),
            (
                format!("contract:{name}:toolchain"),
                contract_root.join("rust-toolchain.toml"),
            ),
        ] {
            report.checks.push(path_check(label, &path, "error"));
        }
    }
    if manifest.api.as_ref().is_some_and(|api| api.enabled) {
        for (label, path) in [
            ("api:env-example", root.join("apps/api/.env.example")),
            ("api:package", root.join("apps/api/package.json")),
            ("api:tsconfig", root.join("apps/api/tsconfig.json")),
            ("api:openapi", root.join("apps/api/openapi.json")),
            ("api:server", root.join("apps/api/src/server.ts")),
            (
                "api:health-route",
                root.join("apps/api/src/routes/health.ts"),
            ),
            (
                "api:contracts-route",
                root.join("apps/api/src/routes/contracts.ts"),
            ),
            (
                "api:events-route",
                root.join("apps/api/src/routes/events.ts"),
            ),
            (
                "api:tokens-route",
                root.join("apps/api/src/routes/tokens.ts"),
            ),
            (
                "api:wallets-route",
                root.join("apps/api/src/routes/wallets.ts"),
            ),
            (
                "api:event-store",
                root.join("apps/api/src/lib/events-store.ts"),
            ),
            ("api:config", root.join("apps/api/src/lib/config.ts")),
            ("api:errors", root.join("apps/api/src/lib/errors.ts")),
            (
                "api:manifest-lib",
                root.join("apps/api/src/lib/manifest.ts"),
            ),
            ("api:rpc-service", root.join("apps/api/src/services/rpc.ts")),
            (
                "api:event-worker",
                root.join("apps/api/src/workers/ingest-events.ts"),
            ),
            ("api:event-schema", root.join("apps/api/db/schema.sql")),
        ] {
            report.checks.push(path_check(label, &path, "error"));
        }
        if manifest.api.as_ref().is_some_and(|api| api.relayer) {
            for (label, path) in [
                (
                    "api:relayer-route",
                    root.join("apps/api/src/routes/relayer.ts"),
                ),
                (
                    "api:relayer-service",
                    root.join("apps/api/src/services/relayer.ts"),
                ),
            ] {
                report.checks.push(path_check(label, &path, "error"));
            }
        }
    }
    if manifest
        .frontend
        .as_ref()
        .is_some_and(|frontend| frontend.enabled)
    {
        for (label, path) in [
            ("frontend:package", root.join("apps/web/package.json")),
            ("frontend:index", root.join("apps/web/index.html")),
            ("frontend:entry", root.join("apps/web/src/main.tsx")),
            (
                "frontend:ui-smoke-runner",
                root.join("apps/web/scripts/ui-smoke.mjs"),
            ),
            (
                "frontend:generated-state",
                root.join("apps/web/src/generated/stellar.ts"),
            ),
        ] {
            report.checks.push(path_check(label, &path, "error"));
        }
    }
}

fn append_contract_token_checks(report: &mut CommandReport, manifest: &Manifest) {
    for (name, token) in &manifest.tokens {
        if token.kind != "contract" {
            continue;
        }
        let Some(contract) = manifest.contracts.get(name) else {
            report.checks.push(check(
                format!("token:{name}:contract"),
                "error",
                Some(format!(
                    "token `{name}` is declared as a contract token but no matching contract `{name}` exists in the manifest"
                )),
            ));
            continue;
        };

        report.checks.push(check(
            format!("token:{name}:contract"),
            "ok",
            Some(contract.path.clone()),
        ));

        let template_status =
            if contract.template.trim().is_empty() || contract.template == "openzeppelin-token" {
                "ok"
            } else {
                "warn"
            };
        let template_detail = if contract.template.trim().is_empty() {
            "template not declared; contract-token helpers assume the OpenZeppelin token ABI"
                .to_string()
        } else if contract.template == "openzeppelin-token" {
            "openzeppelin-token".to_string()
        } else {
            format!(
                "template `{}` may not match the contract-token helper ABI",
                contract.template
            )
        };
        report.checks.push(check(
            format!("token:{name}:template"),
            template_status,
            Some(template_detail),
        ));

        match token::contract_effective_init_config(manifest, name) {
            Ok(Some(init)) => {
                let required = ["admin", "name", "symbol", "decimals"];
                let missing = required
                    .into_iter()
                    .filter(|field| {
                        init.args
                            .get(*field)
                            .is_none_or(|value| value.trim().is_empty())
                    })
                    .collect::<Vec<_>>();
                let defaulted = required
                    .into_iter()
                    .filter(|field| {
                        contract
                            .init
                            .as_ref()
                            .and_then(|init| init.args.get(*field))
                            .is_none_or(|value| value.trim().is_empty())
                    })
                    .collect::<Vec<_>>();
                let arg_names = init.args.keys().cloned().collect::<Vec<_>>().join(", ");
                let mut detail = format!(
                    "fn={}, args={}",
                    init.fn_name,
                    if arg_names.is_empty() {
                        "<none>"
                    } else {
                        &arg_names
                    }
                );
                if !defaulted.is_empty() {
                    detail.push_str(&format!(
                        "; derived defaults applied for {}",
                        defaulted.join(", ")
                    ));
                }
                if !missing.is_empty() {
                    detail.push_str(&format!("; missing {}", missing.join(", ")));
                }
                report.checks.push(check(
                    format!("token:{name}:init"),
                    if missing.is_empty() { "ok" } else { "error" },
                    Some(detail),
                ));
            }
            Ok(None) => report.checks.push(check(
                format!("token:{name}:init"),
                "error",
                Some("no init configuration is available".to_string()),
            )),
            Err(error) => report.checks.push(check(
                format!("token:{name}:init"),
                "error",
                Some(error.to_string()),
            )),
        }
    }
}

pub(super) fn project_validation_report(context: &AppContext) -> Result<CommandReport> {
    let mut report = CommandReport::new("project.validate");
    if !context.manifest_path.exists() {
        report.checks.push(check(
            "manifest",
            "error",
            Some("stellarforge.toml not found".to_string()),
        ));
        report.message = Some("project validation failed".to_string());
        report.status = "error".to_string();
        report.next = vec!["stellar forge init <name>".to_string()];
        return Ok(report);
    }

    let manifest = load_manifest(context)?;
    let root = context.project_root();
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

    append_project_scaffold_checks(&mut report, &root, &manifest);
    append_contract_token_checks(&mut report, &manifest);
    append_project_generated_file_checks(context, &mut report, &root, &manifest)?;
    if let Some(events_check) = event_worker_config_check(&root, &manifest, true, "events:config") {
        report.checks.push(events_check);
    }

    report.checks.push(check(
        "lockfile",
        if root.join("stellarforge.lock.json").exists() {
            "ok"
        } else {
            "error"
        },
        Some(root.join("stellarforge.lock.json").display().to_string()),
    ));

    let lockfile = load_lockfile(context)?;
    report
        .checks
        .extend(stale_lockfile_checks(&manifest, &lockfile));
    match scaffold_compatibility_checks(&root, &manifest, &lockfile) {
        Ok(checks) => report.checks.extend(checks),
        Err(error) => {
            report
                .checks
                .push(check("compat:scaffold", "error", Some(error.to_string())))
        }
    }

    report.status = aggregate_status(&report.checks);
    report.message = Some(if report.status == "error" {
        "project validation failed".to_string()
    } else if report.status == "warn" {
        "project validation found warnings".to_string()
    } else {
        "manifest and local project structure are consistent".to_string()
    });
    report.network = Some(manifest.defaults.network.clone());
    report.next = project_validation_next_steps(&report, &manifest);
    report.data = Some(project_validation_data(&report, &manifest));
    Ok(report)
}

fn append_project_generated_file_checks(
    context: &AppContext,
    report: &mut CommandReport,
    root: &Path,
    manifest: &Manifest,
) -> Result<()> {
    report.checks.push(generated_file_consistency_check(
        context,
        "env:example:consistency",
        &root.join(".env.example"),
        &templates::env_example(manifest),
    ));

    if manifest.api.as_ref().is_some_and(|api| api.enabled) {
        report.checks.push(generated_file_consistency_check(
            context,
            "api:openapi:consistency",
            &root.join("apps/api/openapi.json"),
            &serde_json::to_string_pretty(&build_openapi(manifest))?,
        ));
        report.checks.push(generated_file_consistency_check(
            context,
            "api:manifest-lib:consistency",
            &root.join("apps/api/src/lib/manifest.ts"),
            &render_api_manifest_module(manifest)?,
        ));
    }

    if manifest
        .frontend
        .as_ref()
        .is_some_and(|frontend| frontend.enabled)
    {
        let env_name = manifest
            .active_network(context.globals.network.as_deref())?
            .0
            .to_string();
        let lockfile = load_lockfile(context)?;
        let event_cursors = load_event_cursors(root)?;
        report.checks.push(generated_file_consistency_check(
            context,
            "frontend:generated-state:consistency",
            &root.join("apps/web/src/generated/stellar.ts"),
            &templates::web_generated_state(manifest, &lockfile, &event_cursors, &env_name),
        ));
        report.checks.push(generated_file_consistency_check(
            context,
            "frontend:ui-smoke-runner:consistency",
            &root.join("apps/web/scripts/ui-smoke.mjs"),
            templates::web_ui_smoke_runner(),
        ));
    }

    Ok(())
}

fn generated_file_consistency_check(
    context: &AppContext,
    label: &str,
    path: &Path,
    expected: &str,
) -> crate::runtime::CheckResult {
    if !path.exists() {
        return check(label, "error", Some(format!("missing {}", path.display())));
    }

    match context.read_text(path) {
        Ok(actual) if actual == expected => check(label, "ok", Some(path.display().to_string())),
        Ok(_) => check(
            label,
            "error",
            Some(format!(
                "{} differs from the generated output; run `stellar forge project sync`",
                path.display()
            )),
        ),
        Err(error) => check(label, "error", Some(error.to_string())),
    }
}

pub(super) fn render_api_manifest_module(manifest: &Manifest) -> Result<String> {
    Ok(format!(
        "export const manifest = {} as const;\n",
        serde_json::to_string_pretty(manifest)?
    ))
}

fn project_validation_next_steps(report: &CommandReport, manifest: &Manifest) -> Vec<String> {
    let mut next = Vec::new();
    let has_generated_drift = report
        .checks
        .iter()
        .any(|check| check.name.ends_with(":consistency") && check.status == "error");
    let has_event_worker_gap = report.checks.iter().any(|check| {
        matches!(
            check.name.as_str(),
            "api:event-worker" | "api:event-schema" | "api:events-route"
        ) && check.status == "error"
    });

    if has_generated_drift || report.status != "ok" {
        next.push("stellar forge project sync".to_string());
    }
    if has_event_worker_gap {
        next.push("stellar forge api events init".to_string());
    }
    next.push("stellar forge doctor project".to_string());
    if let Ok((env_name, _)) = manifest.active_network(None) {
        next.push(format!("stellar forge release plan {env_name}"));
    }
    next
}

fn project_validation_data(report: &CommandReport, manifest: &Manifest) -> Value {
    let failing_checks = report
        .checks
        .iter()
        .filter(|check| check.status == "error")
        .map(|check| check.name.clone())
        .collect::<Vec<_>>();
    let warning_checks = report
        .checks
        .iter()
        .filter(|check| check.status == "warn")
        .map(|check| check.name.clone())
        .collect::<Vec<_>>();
    json!({
        "project": manifest.project.slug,
        "contracts": manifest.contracts.len(),
        "tokens": manifest.tokens.len(),
        "wallets": manifest.wallets.len(),
        "summary": {
            "ok": report.checks.iter().filter(|check| check.status == "ok").count(),
            "warn": warning_checks.len(),
            "error": failing_checks.len(),
        },
        "failing_checks": failing_checks,
        "warning_checks": warning_checks,
    })
}

pub(super) fn project_validation_failure_message(report: &CommandReport) -> String {
    let mut lines = vec!["project validation failed:".to_string()];
    for check in report.checks.iter().filter(|check| check.status == "error") {
        match &check.detail {
            Some(detail) => lines.push(format!("- {}: {}", check.name, detail)),
            None => lines.push(format!("- {}", check.name)),
        }
    }
    if !report.next.is_empty() {
        lines.push(String::new());
        lines.push("next steps:".to_string());
        for next in &report.next {
            lines.push(format!("- {next}"));
        }
    }
    lines.join("\n")
}

pub(super) fn event_worker_config_check(
    root: &Path,
    manifest: &Manifest,
    strict: bool,
    label: &str,
) -> Option<crate::runtime::CheckResult> {
    if !manifest.api.as_ref().is_some_and(|api| api.enabled) {
        return None;
    }

    let api_root = root.join("apps/api");
    let env = load_event_env_values(root, &api_root);
    let paths = event_store_paths(root);
    let mut issues = Vec::new();

    for key in [
        "STELLAR_EVENTS_BATCH_SIZE",
        "STELLAR_EVENTS_POLL_INTERVAL_MS",
        "STELLAR_EVENTS_START_LEDGER",
        "STELLAR_EVENTS_RETENTION_DAYS",
    ] {
        if let Some(value) = env.get(key)
            && !value.trim().is_empty()
            && value
                .parse::<u64>()
                .ok()
                .filter(|value| *value > 0)
                .is_none()
        {
            issues.push(format!("{key} must be a positive integer"));
        }
    }

    if let Some(value) = env.get("STELLAR_EVENTS_TYPE")
        && !value.trim().is_empty()
        && !matches!(value.trim(), "all" | "contract" | "system")
    {
        issues
            .push("STELLAR_EVENTS_TYPE must be one of `all`, `contract`, or `system`".to_string());
    }

    if let Some(value) = env.get("STELLAR_EVENTS_RESOURCES") {
        let unknown = parse_env_list(value, &[',', '\n'])
            .into_iter()
            .find(|resource| !event_resource_exists(manifest, resource));
        if let Some(resource) = unknown {
            issues.push(format!(
                "STELLAR_EVENTS_RESOURCES references undeclared resource `{resource}`"
            ));
        }
    }

    Some(check(
        label,
        if issues.is_empty() {
            "ok"
        } else if strict {
            "error"
        } else {
            "warn"
        },
        Some(if issues.is_empty() {
            format!(
                "db={}, schema={}, cursor_snapshot={}",
                paths.db_path.display(),
                paths.schema_path.display(),
                paths.snapshot_path.display()
            )
        } else {
            issues.join("; ")
        }),
    ))
}

fn parse_env_list(value: &str, separators: &[char]) -> Vec<String> {
    value
        .split(|character| separators.contains(&character) || character == '\r')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn event_resource_exists(manifest: &Manifest, resource: &str) -> bool {
    if let Some((kind, name)) = resource.split_once(':') {
        return match kind {
            "contract" => manifest.contracts.contains_key(name) || is_contract_address(name),
            "token" => manifest.tokens.contains_key(name),
            "account" => {
                manifest.wallets.contains_key(name)
                    || manifest.identities.contains_key(name)
                    || looks_like_account(name)
            }
            _ => false,
        };
    }

    manifest.contracts.contains_key(resource)
        || manifest.tokens.contains_key(resource)
        || manifest.wallets.contains_key(resource)
        || manifest.identities.contains_key(resource)
        || is_contract_address(resource)
        || looks_like_account(resource)
}

fn path_check(
    label: impl Into<String>,
    path: &Path,
    missing_status: &str,
) -> crate::runtime::CheckResult {
    check(
        label.into(),
        if path.exists() { "ok" } else { missing_status },
        Some(path.display().to_string()),
    )
}

fn stale_lockfile_checks(
    manifest: &Manifest,
    lockfile: &Lockfile,
) -> Vec<crate::runtime::CheckResult> {
    let mut checks = Vec::new();
    for (env, environment) in &lockfile.environments {
        for name in environment.contracts.keys() {
            if !manifest.contracts.contains_key(name) {
                checks.push(check(
                    format!("lockfile:{env}:contract:{name}"),
                    "warn",
                    Some("present in lockfile but missing from manifest".to_string()),
                ));
            }
        }
        for name in environment.tokens.keys() {
            if !manifest.tokens.contains_key(name) {
                checks.push(check(
                    format!("lockfile:{env}:token:{name}"),
                    "warn",
                    Some("present in lockfile but missing from manifest".to_string()),
                ));
            }
        }
    }
    checks
}
