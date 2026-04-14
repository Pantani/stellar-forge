mod doctor;
mod events;
mod release;
mod token;
mod wallet;

use crate::cli::{
    ApiCommand, ApiEventsCommand, ApiGenerateTarget, ApiOpenapiCommand, ApiRelayerCommand, Cli,
    Command, ContractCallArgs, ContractCommand, ContractTtlMutationArgs, DevCommand, DoctorCommand,
    EventsBackfillArgs, EventsCommand, EventsCursorCommand, EventsIngestCommand, EventsWatchArgs,
    InitArgs, ProjectAddTarget, ProjectCommand, ReleaseAliasesCommand, ReleaseCommand,
    ReleaseEnvCommand, ReleaseRegistryCommand, SmartWalletMode, StorageDurability, TokenBurnArgs,
    TokenCommand, TokenCreateArgs, TokenMoveArgs, TokenSacCommand, WalletCommand, WalletPayArgs,
    WalletSep7Command, WalletSmartCommand,
};
use crate::model::{
    ApiConfig, ContractConfig, ContractDeployment, FrontendConfig, IdentityConfig, Lockfile,
    Manifest, ManifestRef, NetworkConfig, TokenConfig, WalletConfig, is_safe_name,
    parse_manifest_ref,
};
use crate::runtime::{AppContext, CommandReport, check, path_to_string, render_command};
use crate::templates;
use anyhow::{Context, Result, anyhow, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime};
use toml::{Value as TomlValue, value::Table as TomlTable};
use url::Url;
use walkdir::WalkDir;

pub fn execute(context: &AppContext, cli: Cli) -> Result<CommandReport> {
    match cli.command {
        Command::Init(args) => init_command(context, &args),
        Command::Project(args) => project_command(context, args.command),
        Command::Dev(args) => dev_command(context, args.command),
        Command::Contract(args) => contract_command(context, args.command),
        Command::Token(args) => token::token_command(context, args.command),
        Command::Wallet(args) => wallet::wallet_command(context, args.command),
        Command::Api(args) => api_command(context, args.command),
        Command::Events(args) => events::events_command(context, args.command),
        Command::Release(args) => release_command(context, args.command),
        Command::Doctor(args) => doctor_command(context, args.command),
    }
}

fn validate_single_path_segment(label: &str, value: &str) -> Result<()> {
    if is_safe_name(value) {
        Ok(())
    } else {
        bail!("{label} `{value}` must be a single filesystem-safe name")
    }
}

fn init_command(context: &AppContext, args: &InitArgs) -> Result<CommandReport> {
    let mut report = CommandReport::new("init");
    validate_single_path_segment("project name", &args.name)?;
    let root = context.cwd.join(&args.name);
    let manifest_path = root.join("stellarforge.toml");
    let lockfile_path = root.join("stellarforge.lock.json");
    if root.exists() && root.read_dir()?.next().is_some() {
        bail!("target directory `{}` is not empty", root.display());
    }

    let mut manifest = templates::scaffold_manifest(args);
    if args.no_api {
        manifest.api = None;
    }
    if args.contracts > manifest.contracts.len() {
        for index in manifest.contracts.len()..args.contracts {
            let name = format!("contract-{}", index + 1);
            manifest.contracts.insert(
                name.clone(),
                ContractConfig {
                    path: format!("contracts/{name}"),
                    alias: name.clone(),
                    template: "basic".to_string(),
                    bindings: vec!["typescript".to_string()],
                    deploy_on: vec!["local".to_string(), "testnet".to_string()],
                    init: None,
                },
            );
        }
    }

    context.ensure_dir(&mut report, &root)?;
    context.ensure_dir(&mut report, &root.join("contracts"))?;
    context.ensure_dir(&mut report, &root.join("packages"))?;
    context.ensure_dir(&mut report, &root.join("scripts"))?;
    context.ensure_dir(&mut report, &root.join("dist"))?;
    context.write_text(
        &mut report,
        &manifest_path,
        &toml::to_string_pretty(&manifest)?,
    )?;
    context.write_text(
        &mut report,
        &lockfile_path,
        &serde_json::to_string_pretty(&Lockfile::default())?,
    )?;
    context.write_text(
        &mut report,
        &root.join(".env.example"),
        &templates::env_example(&manifest),
    )?;
    context.write_text(&mut report, &root.join(".env.generated"), "")?;
    context.write_text(
        &mut report,
        &root.join(".gitignore"),
        templates::gitignore(),
    )?;
    context.write_text(
        &mut report,
        &root.join("README.md"),
        &templates::readme(&manifest),
    )?;
    context.write_text(
        &mut report,
        &root.join("scripts/reseed.mjs"),
        templates::project_reseed_script(),
    )?;
    context.write_text(
        &mut report,
        &root.join("scripts/release.mjs"),
        templates::project_release_script(),
    )?;
    context.write_text(
        &mut report,
        &root.join("scripts/doctor.mjs"),
        templates::project_doctor_script(),
    )?;

    scaffold_init_contracts(context, &mut report, &root, &manifest)?;
    if manifest.api.as_ref().is_some_and(|api| api.enabled) {
        sync_api_scaffold(context, &mut report, &root, &manifest)?;
    }
    if manifest
        .frontend
        .as_ref()
        .is_some_and(|frontend| frontend.enabled)
    {
        sync_frontend_scaffold(context, &mut report, &root, &manifest)?;
    }
    context.ensure_dir(&mut report, &root.join("workers/events"))?;
    context.write_text(
        &mut report,
        &root.join("workers/events/ingest-events.mjs"),
        templates::worker_stub(),
    )?;
    context.write_text(
        &mut report,
        &root.join("workers/events/cursors.json"),
        "{\n  \"cursors\": {}\n}\n",
    )?;

    if args.git && !args.no_git {
        let git_args = vec!["init".to_string()];
        context.run_command(&mut report, Some(&root), "git", &git_args)?;
    }
    if args.install && !args.no_install {
        for app_dir in ["apps/api", "apps/web"] {
            let package_json = root.join(app_dir).join("package.json");
            if package_json.exists() {
                let pm = manifest.project.package_manager.clone();
                context.run_command(
                    &mut report,
                    Some(&root.join(app_dir)),
                    &pm,
                    &["install".to_string()],
                )?;
            }
        }
    }

    report.message = Some(format!(
        "project `{}` scaffolded at {}",
        manifest.project.slug,
        root.display()
    ));
    report.network = Some(manifest.defaults.network.clone());
    report.next = vec![
        format!("cd {}", root.display()),
        "stellar forge doctor".to_string(),
        "stellar forge dev up".to_string(),
        format!("stellar forge release plan {}", manifest.defaults.network),
    ];
    report.data = Some(json!({
        "project": manifest.project,
        "contracts": manifest.contracts.keys().collect::<Vec<_>>(),
        "api": manifest.api.as_ref().is_some_and(|api| api.enabled),
        "frontend": manifest.frontend.as_ref().is_some_and(|frontend| frontend.enabled),
    }));
    Ok(report)
}

fn project_command(context: &AppContext, command: ProjectCommand) -> Result<CommandReport> {
    match command {
        ProjectCommand::Info => project_info(context),
        ProjectCommand::Sync => project_sync(context),
        ProjectCommand::Validate => project_validate(context),
        ProjectCommand::Add(args) => match args.target {
            ProjectAddTarget::Contract { name, template } => {
                contract_new(context, &name, &template, true)
            }
            ProjectAddTarget::Api => project_add_api(context),
            ProjectAddTarget::Frontend { framework } => project_add_frontend(context, &framework),
        },
        ProjectCommand::Adopt(args) => match args.source {
            crate::cli::ProjectAdoptSource::Scaffold => project_adopt_scaffold(context),
        },
    }
}

fn dev_command(context: &AppContext, command: DevCommand) -> Result<CommandReport> {
    match command {
        DevCommand::Up => dev_up(context),
        DevCommand::Down => dev_down(context),
        DevCommand::Status => dev_status(context),
        DevCommand::Reset => dev_reset(context),
        DevCommand::Reseed => dev_reseed(context),
        DevCommand::Fund { target } => dev_fund(context, &target),
        DevCommand::Watch { interval_ms, once } => dev_watch(context, interval_ms, once),
        DevCommand::Events { resource } => {
            let mut report = CommandReport::new("dev.events");
            report.message = Some(
                "use `stellar forge events watch` for resource-specific event streaming"
                    .to_string(),
            );
            report.data = Some(json!({ "resource": resource }));
            Ok(report)
        }
        DevCommand::Logs => dev_logs(context),
    }
}

fn contract_command(context: &AppContext, command: ContractCommand) -> Result<CommandReport> {
    match command {
        ContractCommand::New { name, template } => contract_new(context, &name, &template, false),
        ContractCommand::Build { name, optimize } => {
            contract_build(context, name.as_deref(), optimize)
        }
        ContractCommand::Deploy { name, env } => contract_deploy(context, &name, env.as_deref()),
        ContractCommand::Call(args) => contract_call(context, &args),
        ContractCommand::Bind { contract, langs } => contract_bind(context, &contract, &langs),
        ContractCommand::Info { contract } => contract_info(context, &contract),
        ContractCommand::Fetch { contract, out } => contract_fetch(context, &contract, out),
        ContractCommand::Ttl(args) => match args.command {
            crate::cli::ContractTtlCommand::Extend(args) => contract_ttl(context, &args, false),
            crate::cli::ContractTtlCommand::Restore(args) => contract_ttl(context, &args, true),
        },
        ContractCommand::Spec { contract } => contract_spec(context, &contract),
    }
}

fn api_command(context: &AppContext, command: ApiCommand) -> Result<CommandReport> {
    match command {
        ApiCommand::Init => api_init(context),
        ApiCommand::Generate(args) => match args.target {
            ApiGenerateTarget::Contract { name } => api_generate_contract(context, &name),
            ApiGenerateTarget::Token { name } => api_generate_token(context, &name),
        },
        ApiCommand::Openapi(args) => match args.command {
            ApiOpenapiCommand::Export => api_openapi_export(context),
        },
        ApiCommand::Events(args) => match args.command {
            ApiEventsCommand::Init => api_events_init(context),
        },
        ApiCommand::Relayer(args) => match args.command {
            ApiRelayerCommand::Init => api_relayer_init(context),
        },
    }
}

fn release_command(context: &AppContext, command: ReleaseCommand) -> Result<CommandReport> {
    release::release_command(context, command)
}

fn doctor_command(context: &AppContext, command: Option<DoctorCommand>) -> Result<CommandReport> {
    doctor::doctor_command(context, command)
}

fn project_info(context: &AppContext) -> Result<CommandReport> {
    let mut report = CommandReport::new("project.info");
    let manifest = load_manifest(context)?;
    let lockfile = load_lockfile(context)?;
    let root = context.project_root();
    let errors = manifest.validate(&root);
    if !errors.is_empty() {
        report.warnings.extend(errors);
    }
    let compatibility = match doctor::scaffold_compatibility_snapshot(&root, &manifest, &lockfile) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            report
                .warnings
                .push(format!("failed to inspect Scaffold compatibility: {error}"));
            None
        }
    };
    report.network = Some(manifest.defaults.network.clone());
    report.message = Some(format!(
        "{} contracts, {} tokens, {} wallets, {} networks",
        manifest.contracts.len(),
        manifest.tokens.len(),
        manifest.wallets.len(),
        manifest.networks.len(),
    ));
    report.data = Some(json!({
        "project": manifest.project,
        "defaults": manifest.defaults,
        "networks": manifest.networks,
        "contracts": manifest.contracts,
        "tokens": manifest.tokens,
        "wallets": manifest.wallets,
        "api": manifest.api,
        "frontend": manifest.frontend,
        "release": manifest.release,
        "deployment": lockfile.environments,
        "compatibility": compatibility,
    }));
    Ok(report)
}

fn project_sync(context: &AppContext) -> Result<CommandReport> {
    let mut report = CommandReport::new("project.sync");
    let manifest = load_manifest(context)?;
    let root = context.project_root();
    let mut synced_modules = vec!["env_example".to_string()];
    context.write_text(
        &mut report,
        &root.join(".env.example"),
        &templates::env_example(&manifest),
    )?;
    if manifest.api.as_ref().is_some_and(|api| api.enabled) {
        sync_api_scaffold(context, &mut report, &root, &manifest)?;
        synced_modules.push("api".to_string());
    }
    if manifest
        .frontend
        .as_ref()
        .is_some_and(|frontend| frontend.enabled)
    {
        sync_frontend_scaffold(context, &mut report, &root, &manifest)?;
        synced_modules.push("frontend".to_string());
    }
    report.message = Some(format!(
        "derived files synced from manifest ({})",
        synced_modules.join(", ")
    ));
    report.next = project_sync_next_steps(&manifest);
    report.data = Some(json!({
        "synced_modules": synced_modules,
        "paths": {
            "env_example": root.join(".env.example").display().to_string(),
            "api_root": manifest.api.as_ref().is_some_and(|api| api.enabled).then(|| {
                root.join("apps/api").display().to_string()
            }),
            "openapi": manifest.api.as_ref().is_some_and(|api| api.enabled).then(|| {
                root.join("apps/api/openapi.json").display().to_string()
            }),
            "frontend_root": manifest.frontend.as_ref().is_some_and(|frontend| frontend.enabled).then(|| {
                root.join("apps/web").display().to_string()
            }),
            "generated_state": manifest.frontend.as_ref().is_some_and(|frontend| frontend.enabled).then(|| {
                root.join("apps/web/src/generated/stellar.ts").display().to_string()
            }),
        },
    }));
    Ok(report)
}

fn project_validate(context: &AppContext) -> Result<CommandReport> {
    let report = doctor::project_validation_report(context)?;
    if report.status == "error" && !context.globals.json {
        bail!(doctor::project_validation_failure_message(&report));
    }
    Ok(report)
}

fn project_adopt_scaffold(context: &AppContext) -> Result<CommandReport> {
    let mut report = CommandReport::new("project.adopt.scaffold");
    let root = context.project_root();
    let manifest_path = context.manifest_path.clone();
    let mut manifest = if manifest_path.exists() {
        Manifest::load(&manifest_path)?
    } else {
        Manifest::default()
    };
    let project_name = project_name_from_root(&root);
    let detected_package_manager = detect_package_manager(&root);
    let package_manager = detected_package_manager
        .clone()
        .unwrap_or_else(|| manifest.project.package_manager.clone());
    let baseline = scaffold_adoption_baseline(
        &project_name,
        if package_manager.trim().is_empty() {
            "pnpm"
        } else {
            &package_manager
        },
    );
    let detected_contracts = detect_scaffold_contracts(&root)?;
    let detected_bindings = detect_scaffold_bindings(&root)?;
    let imported_environments = import_scaffold_environments(&root)?;
    let scaffold_frontend_detected = detect_scaffold_frontend(&root);

    if detected_contracts.is_empty()
        && detected_bindings.is_empty()
        && imported_environments.environments.is_empty()
    {
        bail!(
            "could not detect a Scaffold Stellar project layout in `{}`",
            root.display()
        );
    }

    if manifest.project.name.trim().is_empty() {
        manifest.project.name = baseline.project.name.clone();
    }
    if manifest.project.slug.trim().is_empty() {
        manifest.project.slug = baseline.project.slug.clone();
    }
    if manifest.project.version.trim().is_empty() {
        manifest.project.version = baseline.project.version.clone();
    }
    if let Some(detected_package_manager) = detected_package_manager {
        manifest.project.package_manager = detected_package_manager;
    } else if manifest.project.package_manager.trim().is_empty() {
        manifest.project.package_manager = baseline.project.package_manager.clone();
    }

    if manifest.defaults.output.trim().is_empty() {
        manifest.defaults.output = baseline.defaults.output.clone();
    }
    if manifest.networks.is_empty() {
        manifest.networks = baseline.networks.clone();
    }
    for (env_name, network) in &imported_environments.networks {
        manifest.networks.insert(env_name.clone(), network.clone());
    }
    if manifest.identities.is_empty() {
        manifest.identities = baseline.identities.clone();
    }
    if manifest.wallets.is_empty() {
        manifest.wallets = baseline.wallets.clone();
    }

    for (name, path) in detected_contracts {
        let bindings = detected_bindings.get(&name).cloned().unwrap_or_default();
        let deploy_on = imported_environments
            .contract_deploy_on
            .get(&name)
            .map(|environments| environments.iter().cloned().collect::<Vec<_>>())
            .unwrap_or_else(|| vec!["local".to_string(), "testnet".to_string()]);
        let alias = imported_environments
            .contract_aliases
            .get(&name)
            .cloned()
            .unwrap_or_else(|| name.clone());
        let entry = manifest
            .contracts
            .entry(name.clone())
            .or_insert_with(|| ContractConfig {
                path: path.clone(),
                alias: alias.clone(),
                template: "adopted".to_string(),
                bindings: bindings.clone(),
                deploy_on: deploy_on.clone(),
                init: None,
            });
        if entry.path.trim().is_empty() {
            entry.path = path;
        }
        if entry.alias.trim().is_empty() {
            entry.alias = alias;
        }
        if entry.template.trim().is_empty() {
            entry.template = "adopted".to_string();
        }
        if !bindings.is_empty() {
            entry.bindings = bindings;
        }
        if entry.deploy_on.is_empty() {
            entry.deploy_on = deploy_on;
        }
    }

    if root.join("apps/api").exists() {
        manifest.api.get_or_insert(ApiConfig {
            enabled: true,
            openapi: true,
            ..ApiConfig::default()
        });
    }
    if root.join("apps/web").exists() {
        manifest.frontend.get_or_insert(FrontendConfig {
            enabled: true,
            framework: "react-vite".to_string(),
        });
    }
    if scaffold_frontend_detected && !root.join("apps/web").exists() {
        report.warnings.push(
            "Scaffold frontend detected at the project root; preserved unmanaged outside `apps/web`"
                .to_string(),
        );
    }

    if manifest.defaults.network.trim().is_empty()
        || !manifest.networks.contains_key(&manifest.defaults.network)
    {
        manifest.defaults.network =
            preferred_network_name(&manifest.networks, &imported_environments.environments)
                .unwrap_or_else(|| baseline.defaults.network.clone());
    }
    if manifest.defaults.identity.trim().is_empty()
        || !manifest
            .identities
            .contains_key(&manifest.defaults.identity)
    {
        manifest.defaults.identity = if manifest
            .identities
            .contains_key(&baseline.defaults.identity)
        {
            baseline.defaults.identity.clone()
        } else {
            manifest
                .identities
                .keys()
                .next()
                .cloned()
                .unwrap_or_else(|| baseline.defaults.identity.clone())
        };
    }

    save_manifest(context, &mut report, &manifest)?;
    save_lockfile(context, &mut report, &imported_environments.lockfile)?;
    report.message =
        Some("existing Scaffold-like project adopted into stellarforge.toml".to_string());
    report.data = Some(json!({
        "contracts": manifest.contracts.keys().collect::<Vec<_>>(),
        "bindings": detected_bindings,
        "environments": imported_environments.environments,
        "deployments": imported_environments.deployment_counts,
        "api": manifest.api.as_ref().is_some_and(|api| api.enabled),
        "frontend": manifest.frontend.as_ref().is_some_and(|frontend| frontend.enabled),
        "scaffold_frontend_detected": scaffold_frontend_detected,
    }));
    Ok(report)
}

#[derive(Default)]
struct ScaffoldEnvironmentImport {
    environments: Vec<String>,
    deployment_counts: BTreeMap<String, usize>,
    contract_aliases: BTreeMap<String, String>,
    contract_deploy_on: BTreeMap<String, BTreeSet<String>>,
    networks: BTreeMap<String, NetworkConfig>,
    lockfile: Lockfile,
}

fn project_name_from_root(root: &Path) -> String {
    root.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("stellar-project")
        .to_string()
}

fn scaffold_adoption_baseline(project_name: &str, package_manager: &str) -> Manifest {
    let mut manifest = templates::scaffold_manifest(&InitArgs {
        name: project_name.to_string(),
        template: crate::cli::ProjectTemplate::MinimalContract,
        frontend: "react-vite".to_string(),
        api: false,
        no_api: true,
        wallet: "classic".to_string(),
        contracts: 0,
        network: "testnet".to_string(),
        package_manager: package_manager.to_string(),
        git: false,
        no_git: true,
        install: false,
        no_install: true,
    });
    manifest.contracts.clear();
    manifest.api = None;
    manifest.frontend = None;
    manifest
}

fn detect_package_manager(root: &Path) -> Option<String> {
    [
        ("pnpm-lock.yaml", "pnpm"),
        ("pnpm-workspace.yaml", "pnpm"),
        ("package-lock.json", "npm"),
        ("yarn.lock", "yarn"),
        ("bun.lockb", "bun"),
        ("bun.lock", "bun"),
    ]
    .into_iter()
    .find_map(|(file, manager)| root.join(file).exists().then(|| manager.to_string()))
}

fn package_manager_install_command(package_manager: &str, dir: &str) -> String {
    match package_manager {
        "npm" => format!("npm install --prefix {dir}"),
        "yarn" => format!("yarn --cwd {dir} install"),
        "bun" => format!("bun --cwd {dir} install"),
        _ => format!("pnpm --dir {dir} install"),
    }
}

fn package_manager_dev_command(package_manager: &str, dir: &str) -> String {
    match package_manager {
        "npm" => format!("npm run dev --prefix {dir}"),
        "yarn" => format!("yarn --cwd {dir} dev"),
        "bun" => format!("bun --cwd {dir} run dev"),
        _ => format!("pnpm --dir {dir} dev"),
    }
}

fn api_app_next_steps(package_manager: &str) -> Vec<String> {
    vec![
        package_manager_install_command(package_manager, "apps/api"),
        package_manager_dev_command(package_manager, "apps/api"),
    ]
}

fn frontend_app_next_steps(package_manager: &str) -> Vec<String> {
    vec![
        package_manager_install_command(package_manager, "apps/web"),
        package_manager_dev_command(package_manager, "apps/web"),
    ]
}

fn project_sync_next_steps(manifest: &Manifest) -> Vec<String> {
    let mut next = vec!["stellar forge project validate".to_string()];
    if manifest.api.as_ref().is_some_and(|api| api.enabled) {
        next.push(package_manager_dev_command(
            &manifest.project.package_manager,
            "apps/api",
        ));
    }
    if manifest
        .frontend
        .as_ref()
        .is_some_and(|frontend| frontend.enabled)
    {
        next.push(package_manager_dev_command(
            &manifest.project.package_manager,
            "apps/web",
        ));
    }
    next
}

fn ensure_api_enabled(manifest: &mut Manifest) {
    let mut api = manifest.api.clone().unwrap_or_default();
    api.enabled = true;
    api.openapi = true;
    manifest.api = Some(api);
}

fn detect_scaffold_contracts(root: &Path) -> Result<Vec<(String, String)>> {
    let contracts_root = root.join("contracts");
    let scan_root = if contracts_root.is_dir() {
        contracts_root
    } else {
        root.to_path_buf()
    };
    let inside_contracts_dir = scan_root != root;
    let mut contracts = Vec::new();
    for entry in fs::read_dir(&scan_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !entry.path().join("Cargo.toml").exists() {
            continue;
        }
        let path = if inside_contracts_dir {
            format!("contracts/{name}")
        } else {
            name.clone()
        };
        contracts.push((name, path));
    }
    contracts.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(contracts)
}

fn detect_scaffold_bindings(root: &Path) -> Result<BTreeMap<String, Vec<String>>> {
    let packages_root = root.join("packages");
    if !packages_root.is_dir() {
        return Ok(BTreeMap::new());
    }
    let mut bindings = BTreeMap::<String, BTreeSet<String>>::new();
    for entry in fs::read_dir(packages_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let directory_name = entry.file_name().to_string_lossy().to_string();
        if let Some((contract, language)) = parse_binding_package_directory(&directory_name) {
            bindings.entry(contract).or_default().insert(language);
        }
    }
    Ok(bindings
        .into_iter()
        .map(|(contract, languages)| (contract, languages.into_iter().collect()))
        .collect())
}

fn parse_binding_package_directory(directory_name: &str) -> Option<(String, String)> {
    for (suffix, language) in [
        ("-typescript", "typescript"),
        ("-ts", "typescript"),
        ("-python", "python"),
        ("-py", "python"),
        ("-java", "java"),
        ("-flutter", "flutter"),
        ("-swift", "swift"),
        ("-php", "php"),
        ("-rust", "rust"),
    ] {
        if let Some(contract) = directory_name.strip_suffix(suffix)
            && !contract.trim().is_empty()
        {
            return Some((contract.to_string(), language.to_string()));
        }
    }
    None
}

fn detect_scaffold_frontend(root: &Path) -> bool {
    !root.join("apps/web").exists()
        && root.join("package.json").exists()
        && (root.join("src").is_dir() || root.join("app").is_dir() || root.join("pages").is_dir())
}

fn import_scaffold_environments(root: &Path) -> Result<ScaffoldEnvironmentImport> {
    let environments_path = root.join("environments.toml");
    if !environments_path.exists() {
        return Ok(ScaffoldEnvironmentImport::default());
    }
    let raw = fs::read_to_string(&environments_path)?;
    let parsed: TomlValue = toml::from_str(&raw)?;
    let root_table = parsed
        .as_table()
        .ok_or_else(|| anyhow!("`environments.toml` must contain a top-level table"))?;
    let environment_tables = collect_environment_tables(root_table);
    let contract_tables = collect_named_tables(root_table.get("contracts"));
    let alias_tables = collect_alias_tables(root_table.get("aliases"));
    let mut environment_names = environment_tables.keys().cloned().collect::<BTreeSet<_>>();
    environment_names.extend(contract_tables.keys().cloned());
    environment_names.extend(alias_tables.keys().cloned());

    let mut import = ScaffoldEnvironmentImport::default();
    for env_name in environment_names {
        let env_table = environment_tables.get(&env_name);
        let mut network = default_network_for_environment(&env_name);
        if network.kind.trim().is_empty() {
            network.kind = env_name.clone();
        }
        if let Some(table) = env_table {
            apply_network_overrides(table, &mut network);
            if let Some(network_table) = table.get("network").and_then(TomlValue::as_table) {
                apply_network_overrides(network_table, &mut network);
            }
        }
        import.networks.insert(env_name.clone(), network);

        let mut aliases = alias_tables.get(&env_name).cloned().unwrap_or_default();
        if let Some(table) = env_table {
            for (contract, alias) in parse_alias_map(table.get("aliases")) {
                aliases.insert(contract, alias);
            }
        }
        let deployment_count = import_environment_contracts(
            &env_name,
            env_table.and_then(|table| table.get("contracts")),
            contract_tables.get(&env_name),
            &aliases,
            &mut import,
        );
        import.environments.push(env_name.clone());
        import.deployment_counts.insert(env_name, deployment_count);
    }
    Ok(import)
}

fn collect_environment_tables(root_table: &TomlTable) -> BTreeMap<String, TomlTable> {
    let mut environments = BTreeMap::new();
    if let Some(TomlValue::Table(env_table)) = root_table.get("environments") {
        for (name, value) in env_table {
            if let Some(table) = value.as_table() {
                environments.insert(name.clone(), table.clone());
            }
        }
    }
    for (name, value) in root_table {
        let Some(table) = value.as_table() else {
            continue;
        };
        if environments.contains_key(name) || !is_environment_table(name, table) {
            continue;
        }
        environments.insert(name.clone(), table.clone());
    }
    environments
}

fn collect_named_tables(value: Option<&TomlValue>) -> BTreeMap<String, TomlTable> {
    let mut tables = BTreeMap::new();
    let Some(TomlValue::Table(root)) = value else {
        return tables;
    };
    for (name, entry) in root {
        if let Some(table) = entry.as_table() {
            tables.insert(name.clone(), table.clone());
        }
    }
    tables
}

fn collect_alias_tables(value: Option<&TomlValue>) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut tables = BTreeMap::new();
    let Some(TomlValue::Table(root)) = value else {
        return tables;
    };
    for (name, entry) in root {
        tables.insert(name.clone(), parse_alias_map(Some(entry)));
    }
    tables
}

fn is_environment_table(name: &str, table: &TomlTable) -> bool {
    if matches!(name, "environments" | "aliases" | "contracts") {
        return false;
    }
    matches!(
        name,
        "local" | "testnet" | "futurenet" | "pubnet" | "mainnet"
    ) || table.contains_key("contracts")
        || table.contains_key("aliases")
        || table.contains_key("rpc_url")
        || table.contains_key("horizon_url")
        || table.contains_key("network_passphrase")
        || table.contains_key("allow_http")
        || table.contains_key("friendbot")
        || table.contains_key("network")
        || table.contains_key("kind")
}

fn apply_network_overrides(table: &TomlTable, network: &mut NetworkConfig) {
    if let Some(kind) = toml_string(table, &["kind", "network"]) {
        network.kind = kind;
    }
    if let Some(rpc_url) = toml_string(table, &["rpc_url"]) {
        network.rpc_url = rpc_url;
    }
    if let Some(horizon_url) = toml_string(table, &["horizon_url"]) {
        network.horizon_url = horizon_url;
    }
    if let Some(network_passphrase) = toml_string(table, &["network_passphrase", "passphrase"]) {
        network.network_passphrase = network_passphrase;
    }
    if let Some(allow_http) = toml_bool(table, "allow_http") {
        network.allow_http = allow_http;
    }
    if let Some(friendbot) = toml_bool(table, "friendbot") {
        network.friendbot = friendbot;
    }
}

fn default_network_for_environment(env_name: &str) -> NetworkConfig {
    let mut baseline = scaffold_adoption_baseline("stellar-project", "pnpm");
    baseline
        .networks
        .remove(env_name)
        .unwrap_or_else(|| NetworkConfig {
            kind: env_name.to_string(),
            ..NetworkConfig::default()
        })
}

fn parse_alias_map(value: Option<&TomlValue>) -> BTreeMap<String, String> {
    let mut aliases = BTreeMap::new();
    let Some(TomlValue::Table(table)) = value else {
        return aliases;
    };
    for (name, entry) in table {
        match entry {
            TomlValue::String(alias) if !alias.trim().is_empty() => {
                aliases.insert(name.clone(), alias.clone());
            }
            TomlValue::Table(data) => {
                if let Some(contract) = toml_string(data, &["contract", "target"]) {
                    aliases.insert(contract, name.clone());
                } else if let Some(alias) = toml_string(data, &["alias", "name"]) {
                    aliases.insert(name.clone(), alias);
                }
            }
            _ => {}
        }
    }
    aliases
}

fn import_environment_contracts(
    env_name: &str,
    env_contracts: Option<&TomlValue>,
    top_level_contracts: Option<&TomlTable>,
    aliases: &BTreeMap<String, String>,
    import: &mut ScaffoldEnvironmentImport,
) -> usize {
    let mut merged = BTreeMap::new();
    if let Some(TomlValue::Table(table)) = env_contracts {
        for (name, value) in table {
            merged.insert(name.clone(), value.clone());
        }
    }
    if let Some(table) = top_level_contracts {
        for (name, value) in table {
            merged.entry(name.clone()).or_insert_with(|| value.clone());
        }
    }

    let mut deployment_count = 0;
    for (contract_name, value) in merged {
        let explicit_alias = match value.as_table() {
            Some(table) => toml_string(table, &["alias", "name"])
                .or_else(|| aliases.get(&contract_name).cloned()),
            None => aliases.get(&contract_name).cloned(),
        };
        let alias = explicit_alias
            .clone()
            .unwrap_or_else(|| contract_name.clone());
        match import.contract_aliases.entry(contract_name.clone()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(alias.clone());
            }
            std::collections::btree_map::Entry::Occupied(mut entry)
                if explicit_alias.is_some() && entry.get() == &contract_name =>
            {
                entry.insert(alias.clone());
            }
            std::collections::btree_map::Entry::Occupied(_) => {}
        }
        import
            .contract_deploy_on
            .entry(contract_name.clone())
            .or_default()
            .insert(env_name.to_string());

        let deployment =
            match value {
                TomlValue::String(contract_id) if !contract_id.trim().is_empty() => {
                    Some(ContractDeployment {
                        contract_id,
                        alias,
                        wasm_hash: String::new(),
                        tx_hash: String::new(),
                        deployed_at: None,
                    })
                }
                TomlValue::Table(table) => toml_string(&table, &["contract_id", "id", "address"])
                    .map(|contract_id| ContractDeployment {
                        contract_id,
                        alias,
                        wasm_hash: toml_string(&table, &["wasm_hash"]).unwrap_or_default(),
                        tx_hash: toml_string(&table, &["tx_hash"]).unwrap_or_default(),
                        deployed_at: None,
                    }),
                _ => None,
            };
        if let Some(deployment) = deployment {
            import
                .lockfile
                .environment_mut(env_name)
                .contracts
                .insert(contract_name, deployment);
            deployment_count += 1;
        }
    }
    deployment_count
}

fn toml_string(table: &TomlTable, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| match table.get(*key) {
        Some(TomlValue::String(value)) if !value.trim().is_empty() => Some(value.clone()),
        _ => None,
    })
}

fn toml_bool(table: &TomlTable, key: &str) -> Option<bool> {
    match table.get(key) {
        Some(TomlValue::Boolean(value)) => Some(*value),
        _ => None,
    }
}

fn preferred_network_name(
    networks: &BTreeMap<String, NetworkConfig>,
    imported_environments: &[String],
) -> Option<String> {
    for preferred in ["testnet", "local", "futurenet", "pubnet", "mainnet"] {
        if networks.contains_key(preferred) {
            return Some(preferred.to_string());
        }
    }
    imported_environments
        .iter()
        .find(|name| networks.contains_key(*name))
        .cloned()
        .or_else(|| networks.keys().next().cloned())
}

fn project_add_frontend(context: &AppContext, framework: &str) -> Result<CommandReport> {
    let mut report = CommandReport::new("project.add.frontend");
    let mut manifest = load_manifest(context)?;
    manifest.frontend = Some(FrontendConfig {
        enabled: true,
        framework: framework.to_string(),
    });
    sync_frontend_scaffold(context, &mut report, &context.project_root(), &manifest)?;
    save_manifest(context, &mut report, &manifest)?;
    report.message = Some(format!("frontend `{framework}` added to the project"));
    let root = context.project_root();
    report.next = frontend_app_next_steps(&manifest.project.package_manager);
    report.data = Some(json!({
        "framework": framework,
        "frontend_root": root.join("apps/web").display().to_string(),
        "entrypoint": root.join("apps/web/src/main.tsx").display().to_string(),
        "generated_state": root.join("apps/web/src/generated/stellar.ts").display().to_string(),
    }));
    Ok(report)
}

fn project_add_api(context: &AppContext) -> Result<CommandReport> {
    let mut report = CommandReport::new("project.add.api");
    let mut manifest = load_manifest(context)?;
    ensure_api_enabled(&mut manifest);
    sync_api_scaffold(context, &mut report, &context.project_root(), &manifest)?;
    save_manifest(context, &mut report, &manifest)?;
    let root = context.project_root();
    report.message = Some("API scaffold added to the project".to_string());
    report.next = api_app_next_steps(&manifest.project.package_manager);
    report.data = Some(json!({
        "api_root": root.join("apps/api").display().to_string(),
        "openapi": root.join("apps/api/openapi.json").display().to_string(),
        "services": {
            "contracts": manifest.contracts.len(),
            "tokens": manifest.tokens.len(),
        },
        "events_backend": manifest
            .api
            .as_ref()
            .map(|api| api.events_backend.clone())
            .unwrap_or_else(|| "rpc-poller".to_string()),
    }));
    Ok(report)
}

fn event_worker_config_check(
    root: &Path,
    manifest: &Manifest,
    strict: bool,
    label: &str,
) -> Option<crate::runtime::CheckResult> {
    doctor::event_worker_config_check(root, manifest, strict, label)
}

fn doctor_network(context: &AppContext, env_name: Option<&str>) -> Result<CommandReport> {
    doctor::doctor_network(context, env_name)
}

fn release_state_checks(
    root: &Path,
    manifest: &Manifest,
    lockfile: &Lockfile,
    env: &str,
    strict: bool,
) -> Vec<crate::runtime::CheckResult> {
    release::release_state_checks(root, manifest, lockfile, env, strict)
}

fn probe_release_deployments(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: &Manifest,
    env: &str,
    environment: &crate::model::EnvironmentLock,
) -> Result<Vec<crate::runtime::CheckResult>> {
    release::probe_release_deployments(context, report, manifest, env, environment)
}

fn dev_up(context: &AppContext) -> Result<CommandReport> {
    let mut report = CommandReport::new("dev.up");
    let manifest = load_manifest(context)?;
    let network = manifest
        .networks
        .get("local")
        .ok_or_else(|| anyhow!("local network is not configured in the manifest"))?;
    if network.kind != "local" {
        bail!("`dev up` expects networks.local.kind = \"local\"");
    }
    let args = vec![
        "container".to_string(),
        "start".to_string(),
        "local".to_string(),
    ];
    context.run_command(&mut report, Some(&context.project_root()), "stellar", &args)?;
    if !context.globals.dry_run {
        wait_for_local_network(context, network)?;
    }
    context.write_text(
        &mut report,
        &context.project_root().join(".env.generated"),
        &format!(
            "STELLAR_NETWORK=local\nSTELLAR_RPC_URL={}\nSTELLAR_HORIZON_URL={}\n",
            network.rpc_url, network.horizon_url
        ),
    )?;
    report.network = Some("local".to_string());
    report.message = Some("local Stellar quickstart started".to_string());
    report.next = vec![
        "stellar forge dev status".to_string(),
        "stellar forge dev reseed".to_string(),
    ];
    Ok(report)
}

fn dev_down(context: &AppContext) -> Result<CommandReport> {
    let mut report = CommandReport::new("dev.down");
    context.run_command(
        &mut report,
        Some(&context.project_root()),
        "stellar",
        &[
            "container".to_string(),
            "stop".to_string(),
            "local".to_string(),
        ],
    )?;
    report.message = Some("local Stellar quickstart stopped".to_string());
    report.network = Some("local".to_string());
    Ok(report)
}

fn dev_status(context: &AppContext) -> Result<CommandReport> {
    let mut report = doctor_network(context, Some("local"))?;
    report.action = "dev.status".to_string();
    report.message = Some("local network status checked".to_string());
    Ok(report)
}

fn dev_reset(context: &AppContext) -> Result<CommandReport> {
    let mut report = CommandReport::new("dev.reset");
    context.run_command(
        &mut report,
        Some(&context.project_root()),
        "stellar",
        &[
            "container".to_string(),
            "stop".to_string(),
            "local".to_string(),
        ],
    )?;
    context.run_command(
        &mut report,
        Some(&context.project_root()),
        "stellar",
        &[
            "container".to_string(),
            "start".to_string(),
            "local".to_string(),
        ],
    )?;
    report.message = Some("local network restarted".to_string());
    report.network = Some("local".to_string());
    Ok(report)
}

fn dev_reseed(context: &AppContext) -> Result<CommandReport> {
    let mut report = CommandReport::new("dev.reseed");
    let manifest = load_manifest(context)?;
    let (network_name, _) = manifest.active_network(context.globals.network.as_deref())?;
    report.network = Some(network_name.to_string());

    for identity in manifest.identities.keys() {
        let sub = ensure_identity_exists(
            context,
            &mut report,
            &manifest,
            identity,
            network_name,
            true,
        )?;
        if sub {
            report
                .warnings
                .push(format!("identity `{identity}` was created or funded"));
        }
    }
    for token_name in manifest.tokens.keys() {
        token::token_create_from_manifest(
            context,
            &mut report,
            &manifest,
            token_name,
            network_name,
        )?;
    }
    for contract_name in manifest.contracts.keys() {
        deploy_contract_from_manifest(
            context,
            &mut report,
            &manifest,
            contract_name,
            network_name,
        )?;
    }
    let event_state_reset =
        events::clear_event_state_for_env(context, &mut report, &manifest, network_name)?;
    let (_, _, generate_env) = release_resources(&manifest, network_name);
    let should_export_env = generate_env
        || manifest.api.as_ref().is_some_and(|api| api.enabled)
        || manifest
            .frontend
            .as_ref()
            .is_some_and(|frontend| frontend.enabled);
    if should_export_env {
        let exported = release_env_export(context, network_name)?;
        report.commands.extend(exported.commands);
        report.artifacts.extend(exported.artifacts);
        report.warnings.extend(exported.warnings);
    }
    if !context.globals.dry_run {
        let verify = release_verify(context, network_name)?;
        report.checks.extend(verify.checks);
        let network = doctor_network(context, Some(network_name))?;
        report.checks.extend(network.checks);
        report.status = aggregate_status(&report.checks);
    }
    report.message = Some(if context.globals.dry_run {
        "planned a full project reseed, event-state reset, and environment refresh".to_string()
    } else {
        "declared identities, tokens, and contracts were rehydrated; event state and generated env were refreshed".to_string()
    });
    report.data = Some(json!({
        "identities": manifest.identities.keys().cloned().collect::<Vec<_>>(),
        "tokens": manifest.tokens.keys().cloned().collect::<Vec<_>>(),
        "contracts": manifest.contracts.keys().cloned().collect::<Vec<_>>(),
        "event_state_reset": event_state_reset,
        "env_exported": should_export_env,
        "verification_ran": !context.globals.dry_run,
    }));
    report.next = vec![
        format!("stellar forge release verify {network_name}"),
        format!("stellar forge doctor network {network_name}"),
    ];
    Ok(report)
}

fn dev_fund(context: &AppContext, target: &str) -> Result<CommandReport> {
    let mut report = CommandReport::new("dev.fund");
    let manifest = load_manifest(context)?;
    let (network_name, network) = manifest.active_network(context.globals.network.as_deref())?;
    report.network = Some(network_name.to_string());
    if network.kind == "pubnet" {
        bail!("funding is refused on pubnet");
    }
    let address = resolve_address(context, &mut report, Some(&manifest), target)?;
    if network.kind == "local" {
        fund_local_address(context, &mut report, network, &address)?;
        report.message = Some(format!("local root account funded `{address}`"));
        report.data = Some(json!({ "address": address }));
        return Ok(report);
    }
    let url = friendbot_url(network_name, network, &address)?;
    if !context.globals.dry_run {
        context
            .get_json(&url)
            .with_context(|| format!("friendbot request failed for `{address}`"))?;
    }
    report.commands.push(format!("GET {url}"));
    report.message = Some(format!("funding request sent for `{address}`"));
    report.data = Some(json!({ "address": address }));
    Ok(report)
}

#[derive(Debug, Clone)]
struct DevWatchTarget {
    name: String,
    fingerprint: Option<SystemTime>,
}

fn dev_watch(context: &AppContext, interval_ms: u64, once: bool) -> Result<CommandReport> {
    let mut report = CommandReport::new("dev.watch");
    if context.globals.json && !once {
        bail!("`dev watch --json` requires `--once` because watch mode is a long-running stream");
    }

    let manifest = load_manifest(context)?;
    let (network_name, _) = manifest.active_network(context.globals.network.as_deref())?;
    let mut targets = watch_targets(context, &manifest)?;
    if targets.is_empty() {
        report.status = "warn".to_string();
        report.message =
            Some("no contracts declared in the manifest; nothing to watch".to_string());
        return Ok(report);
    }

    report.network = Some(network_name.to_string());
    if manifest.api.as_ref().is_some_and(|api| api.enabled) {
        report.warnings.push(
            format!(
                "API scaffold will be refreshed on contract changes; keep `{}` running in `apps/api` for automatic restarts",
                manifest.project.package_manager
            ),
        );
    }
    if manifest
        .frontend
        .as_ref()
        .is_some_and(|frontend| frontend.enabled)
    {
        report.warnings.push(
            format!(
                "frontend generated state will be refreshed on contract changes; keep `{}` running in `apps/web` for HMR",
                manifest.project.package_manager
            ),
        );
    }

    let refreshed = refresh_watch_targets(
        context,
        &mut report,
        &manifest,
        targets.iter().map(|target| target.name.as_str()).collect(),
    )?;
    refresh_watch_api_scaffold(context, &mut report, &manifest)?;
    refresh_watch_frontend_scaffold(context, &mut report, &manifest, network_name)?;

    if once || context.globals.dry_run {
        report.message = Some(format!(
            "watch bootstrap completed for {} contract(s)",
            refreshed.len()
        ));
        report.data = Some(json!({
            "mode": "once",
            "interval_ms": interval_ms,
            "contracts": refreshed,
        }));
        return Ok(report);
    }

    if !context.globals.quiet {
        println!(
            "watching {} contract(s); polling every {}ms",
            targets.len(),
            interval_ms.max(250)
        );
    }

    loop {
        sleep(Duration::from_millis(interval_ms.max(250)));
        let changed = changed_watch_targets(context, &manifest, &mut targets)?;
        if changed.is_empty() {
            continue;
        }

        let refreshed = refresh_watch_targets(context, &mut report, &manifest, changed)?;
        refresh_watch_api_scaffold(context, &mut report, &manifest)?;
        refresh_watch_frontend_scaffold(context, &mut report, &manifest, network_name)?;
        if context.globals.json {
            println!(
                "{}",
                serde_json::to_string(&json!({
                    "action": "dev.watch.refresh",
                    "contracts": refreshed,
                }))?
            );
        } else if !context.globals.quiet {
            for item in refreshed {
                println!(
                    "refreshed `{}`",
                    item["name"].as_str().unwrap_or("contract")
                );
            }
        }
    }
}

fn dev_logs(context: &AppContext) -> Result<CommandReport> {
    let mut report = CommandReport::new("dev.logs");
    context.run_command(
        &mut report,
        Some(&context.project_root()),
        "stellar",
        &[
            "container".to_string(),
            "logs".to_string(),
            "local".to_string(),
        ],
    )?;
    report.message = Some("local quickstart logs streamed".to_string());
    Ok(report)
}

fn refresh_watch_api_scaffold(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: &Manifest,
) -> Result<()> {
    if !manifest.api.as_ref().is_some_and(|api| api.enabled) {
        return Ok(());
    }
    sync_api_scaffold(context, report, &context.project_root(), manifest)
}

fn refresh_watch_frontend_scaffold(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: &Manifest,
    env: &str,
) -> Result<()> {
    if !manifest
        .frontend
        .as_ref()
        .is_some_and(|frontend| frontend.enabled)
    {
        return Ok(());
    }
    sync_frontend_generated_state(context, report, &context.project_root(), manifest, env)
}

fn watch_targets(context: &AppContext, manifest: &Manifest) -> Result<Vec<DevWatchTarget>> {
    manifest
        .contracts
        .iter()
        .map(|(name, contract)| {
            let contract_dir = context.project_root().join(&contract.path);
            Ok(DevWatchTarget {
                name: name.clone(),
                fingerprint: contract_watch_fingerprint(&contract_dir)?,
            })
        })
        .collect()
}

fn changed_watch_targets<'a>(
    context: &AppContext,
    manifest: &'a Manifest,
    targets: &'a mut [DevWatchTarget],
) -> Result<Vec<&'a str>> {
    let mut changed = Vec::new();
    for target in targets {
        let contract = manifest
            .contracts
            .get(&target.name)
            .ok_or_else(|| anyhow!("contract `{}` not found", target.name))?;
        let contract_dir = context.project_root().join(&contract.path);
        let next = contract_watch_fingerprint(&contract_dir)?;
        if next > target.fingerprint {
            target.fingerprint = next;
            changed.push(target.name.as_str());
        }
    }
    Ok(changed)
}

fn refresh_watch_targets(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: &Manifest,
    contract_names: Vec<&str>,
) -> Result<Vec<Value>> {
    let mut refreshed = Vec::new();
    for contract_name in contract_names {
        let contract = manifest
            .contracts
            .get(contract_name)
            .ok_or_else(|| anyhow!("contract `{contract_name}` not found"))?;
        let build = contract_build(context, Some(contract_name), false)?;
        report.commands.extend(build.commands);
        report.artifacts.extend(build.artifacts);
        report.warnings.extend(build.warnings);

        let mut binding_status = "skipped".to_string();
        if !contract.bindings.is_empty() {
            match contract_bind(context, contract_name, &contract.bindings) {
                Ok(bind) => {
                    report.commands.extend(bind.commands);
                    report.artifacts.extend(bind.artifacts);
                    report.warnings.extend(bind.warnings);
                    binding_status = "generated".to_string();
                }
                Err(error) => {
                    report.warnings.push(format!(
                        "bindings for `{contract_name}` were skipped during watch refresh: {error}"
                    ));
                    binding_status = "warning".to_string();
                }
            }
        }

        refreshed.push(json!({
            "name": contract_name,
            "bindings": contract.bindings.clone(),
            "binding_status": binding_status,
        }));
    }
    Ok(refreshed)
}

fn contract_watch_fingerprint(contract_dir: &Path) -> Result<Option<SystemTime>> {
    if !contract_dir.exists() {
        return Ok(None);
    }

    let mut latest: Option<SystemTime> = None;
    for file in ["Cargo.toml", "rust-toolchain.toml", "README.md"] {
        let candidate = contract_dir.join(file);
        if let Ok(metadata) = fs::metadata(candidate) {
            let modified = metadata.modified()?;
            latest = Some(latest.map_or(modified, |current| current.max(modified)));
        }
    }

    for entry in WalkDir::new(contract_dir)
        .into_iter()
        .filter_entry(|entry| entry.file_name() != "target")
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let modified = entry.metadata()?.modified()?;
        latest = Some(latest.map_or(modified, |current| current.max(modified)));
    }

    Ok(latest)
}

fn contract_new(
    context: &AppContext,
    name: &str,
    template: &str,
    project_add: bool,
) -> Result<CommandReport> {
    let mut report = CommandReport::new(if project_add {
        "project.add.contract"
    } else {
        "contract.new"
    });
    validate_single_path_segment("contract name", name)?;
    let root = context.project_root();
    let mut manifest = if context.manifest_path.exists() {
        load_manifest(context)?
    } else {
        Manifest::default()
    };
    manifest
        .contracts
        .entry(name.to_string())
        .or_insert(ContractConfig {
            path: format!("contracts/{name}"),
            alias: name.to_string(),
            template: template.to_string(),
            bindings: vec!["typescript".to_string()],
            deploy_on: vec!["local".to_string(), "testnet".to_string()],
            init: None,
        });
    let contract_dir = root.join("contracts").join(name);
    if context.command_exists("stellar") && !context.globals.dry_run {
        context.run_command(
            &mut report,
            Some(&root),
            "stellar",
            &[
                "contract".to_string(),
                "init".to_string(),
                path_to_string(&contract_dir)?,
                "--name".to_string(),
                name.to_string(),
            ],
        )?;
    } else {
        write_contract_stub(context, &mut report, &root, name, template)?;
        report
            .warnings
            .push("stellar CLI not available, wrote a local contract stub instead".to_string());
    }
    save_manifest(context, &mut report, &manifest)?;
    let mut synced_modules = Vec::new();
    if manifest.api.as_ref().is_some_and(|api| api.enabled) {
        sync_api_scaffold(context, &mut report, &root, &manifest)?;
        synced_modules.push("api".to_string());
    }
    if manifest
        .frontend
        .as_ref()
        .is_some_and(|frontend| frontend.enabled)
    {
        sync_frontend_scaffold(context, &mut report, &root, &manifest)?;
        synced_modules.push("frontend".to_string());
    }
    report.message = Some(if synced_modules.is_empty() {
        format!("contract `{name}` added")
    } else {
        format!(
            "contract `{name}` added and derived modules refreshed ({})",
            synced_modules.join(", ")
        )
    });
    report.next = vec![format!("stellar forge contract build {name}")];
    report.data = Some(json!({
        "contract": name,
        "template": template,
        "path": contract_dir.display().to_string(),
        "synced_modules": synced_modules,
        "api_service": manifest.api.as_ref().is_some_and(|api| api.enabled).then(|| {
            root.join("apps/api/src/services/contracts")
                .join(format!("{name}.ts"))
                .display()
                .to_string()
        }),
        "openapi": manifest.api.as_ref().is_some_and(|api| api.enabled).then(|| {
            root.join("apps/api/openapi.json").display().to_string()
        }),
        "frontend_state": manifest.frontend.as_ref().is_some_and(|frontend| frontend.enabled).then(|| {
            root.join("apps/web/src/generated/stellar.ts").display().to_string()
        }),
    }));
    Ok(report)
}

fn contract_build(
    context: &AppContext,
    name: Option<&str>,
    optimize: bool,
) -> Result<CommandReport> {
    let mut report = CommandReport::new("contract.build");
    let manifest = load_manifest(context)?;
    let mut built = Vec::new();
    let contracts: Vec<String> = match name {
        Some(name) => vec![name.to_string()],
        None => manifest.contracts.keys().cloned().collect(),
    };
    if contracts.is_empty() {
        bail!("no contracts defined in the manifest");
    }
    for contract_name in contracts {
        let contract = manifest
            .contracts
            .get(&contract_name)
            .ok_or_else(|| anyhow!("contract `{contract_name}` not found"))?;
        let contract_dir = context.project_root().join(&contract.path);
        let mut args = vec!["contract".to_string(), "build".to_string()];
        if optimize {
            args.push("--optimize".to_string());
        }
        context.run_command(&mut report, Some(&contract_dir), "stellar", &args)?;
        let wasm_path = guess_wasm_path(&contract_dir, &contract_name);
        if wasm_path.exists() && !context.globals.dry_run {
            let bytes = fs::read(&wasm_path)?;
            let hash = hex_digest(&bytes);
            built.push(json!({
                "name": contract_name,
                "wasm": wasm_path.display().to_string(),
                "wasm_hash": hash,
            }));
        } else {
            built.push(json!({
                "name": contract_name,
                "wasm": wasm_path.display().to_string(),
            }));
        }
    }
    report.message = Some("contracts built via the Stellar CLI".to_string());
    report.data = Some(json!({ "contracts": built }));
    Ok(report)
}

fn contract_deploy(
    context: &AppContext,
    name: &str,
    env_override: Option<&str>,
) -> Result<CommandReport> {
    let mut report = CommandReport::new("contract.deploy");
    let manifest = load_manifest(context)?;
    let env = env_override
        .map(ToOwned::to_owned)
        .or_else(|| context.globals.network.clone())
        .unwrap_or_else(|| manifest.defaults.network.clone());
    deploy_contract_from_manifest(context, &mut report, &manifest, name, &env)?;
    report.message = Some(format!("contract `{name}` deployed"));
    report.network = Some(env);
    Ok(report)
}

fn contract_call(context: &AppContext, args: &ContractCallArgs) -> Result<CommandReport> {
    let mut report = CommandReport::new("contract.call");
    let manifest = load_manifest(context)?;
    let lockfile = load_lockfile(context)?;
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    let contract_id = resolve_contract_id(&manifest, &lockfile, &env, &args.contract);
    let source = manifest
        .active_identity(context.globals.identity.as_deref())
        .unwrap_or(&manifest.defaults.identity)
        .to_string();
    let mut command_args = vec![
        "contract".to_string(),
        "invoke".to_string(),
        "--id".to_string(),
        contract_id,
        "--source-account".to_string(),
        source.clone(),
        "--network".to_string(),
        env.clone(),
        "--send".to_string(),
        args.send.clone(),
    ];
    if args.build_only {
        command_args.push("--build-only".to_string());
    }
    command_args.push("--".to_string());
    command_args.push(args.function.clone());
    command_args.extend(args.args.clone());
    let output = context.run_command(
        &mut report,
        Some(&context.project_root()),
        "stellar",
        &command_args,
    )?;
    report.message = Some(format!(
        "invoked `{}` on `{}`",
        args.function, args.contract
    ));
    report.network = Some(env);
    if !output.is_empty() {
        report.data = Some(json!({ "result": output, "source": source }));
    }
    Ok(report)
}

fn contract_bind(
    context: &AppContext,
    contract_name: &str,
    langs: &[String],
) -> Result<CommandReport> {
    let mut report = CommandReport::new("contract.bind");
    let manifest = load_manifest(context)?;
    let lockfile = load_lockfile(context)?;
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    let outputs = generate_contract_bindings(
        context,
        &mut report,
        &manifest,
        &lockfile,
        &env,
        contract_name,
        langs,
    )?;
    report.message = Some(format!("bindings generated for `{contract_name}`"));
    report.network = Some(env);
    report.data = Some(json!({ "outputs": outputs }));
    Ok(report)
}

fn generate_contract_bindings(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: &Manifest,
    lockfile: &Lockfile,
    env: &str,
    contract_name: &str,
    langs: &[String],
) -> Result<Vec<String>> {
    let contract = manifest
        .contracts
        .get(contract_name)
        .ok_or_else(|| anyhow!("contract `{contract_name}` not found"))?;
    let contract_dir = context.project_root().join(&contract.path);
    let deployed_contract_id = lockfile
        .environments
        .get(env)
        .and_then(|environment| environment.contracts.get(contract_name))
        .map(|deployment| deployment.contract_id.clone())
        .filter(|contract_id| !contract_id.is_empty());
    let mut wasm_path = guess_wasm_path(&contract_dir, contract_name);
    if deployed_contract_id.is_none() && !context.globals.dry_run && !wasm_path.exists() {
        context.run_command(
            report,
            Some(&contract_dir),
            "stellar",
            &["contract".to_string(), "build".to_string()],
        )?;
        wasm_path = guess_wasm_path(&contract_dir, contract_name);
    }
    if deployed_contract_id.is_none() && !context.globals.dry_run && !wasm_path.exists() {
        bail!(
            "contract `{contract_name}` has no deployed id in `{env}` and no local wasm artifact; run `stellar forge contract build {contract_name}` or deploy it first"
        );
    }
    let languages = if langs.is_empty() {
        vec!["typescript".to_string()]
    } else {
        langs.to_vec()
    };
    let mut outputs = Vec::new();
    for lang in languages {
        let output_dir = context.project_root().join("packages").join(format!(
            "{contract_name}-{}",
            lang.replace("typescript", "ts")
        ));
        let mut args = vec![
            "contract".to_string(),
            "bindings".to_string(),
            lang.clone(),
            if deployed_contract_id.is_some() {
                "--contract-id".to_string()
            } else {
                "--wasm".to_string()
            },
            if let Some(contract_id) = &deployed_contract_id {
                contract_id.clone()
            } else {
                path_to_string(&wasm_path)?
            },
            "--output-dir".to_string(),
            path_to_string(&output_dir)?,
            "--overwrite".to_string(),
        ];
        if deployed_contract_id.is_some() {
            args.push("--network".to_string());
            args.push(env.to_string());
        }
        context.run_command(report, Some(&context.project_root()), "stellar", &args)?;
        outputs.push(output_dir.display().to_string());
    }
    Ok(outputs)
}

fn contract_info(context: &AppContext, contract_name: &str) -> Result<CommandReport> {
    contract_summary(context, contract_name, "contract.info")
}

fn contract_fetch(
    context: &AppContext,
    contract_name: &str,
    out_override: Option<PathBuf>,
) -> Result<CommandReport> {
    let mut report = CommandReport::new("contract.fetch");
    let manifest = load_manifest(context)?;
    let lockfile = load_lockfile(context)?;
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    let contract_id = resolve_contract_id(&manifest, &lockfile, &env, contract_name);
    let output_path = out_override.unwrap_or_else(|| {
        contract_fetch_output_path(&context.project_root(), &env, contract_name)
    });
    if let Some(parent) = output_path.parent() {
        context.ensure_dir(&mut report, parent)?;
    }
    report.artifacts.push(output_path.display().to_string());
    let args = vec![
        "contract".to_string(),
        "fetch".to_string(),
        "--id".to_string(),
        contract_id.clone(),
        "--out-file".to_string(),
        path_to_string(&output_path)?,
        "--network".to_string(),
        env.clone(),
    ];
    let output =
        context.run_command(&mut report, Some(&context.project_root()), "stellar", &args)?;
    report.message = Some(format!("Wasm fetched for `{contract_name}`"));
    report.network = Some(env);
    let mut data = json!({
        "contract": contract_name,
        "contract_id": contract_id,
        "output": output_path.display().to_string(),
    });
    if !context.globals.dry_run && output_path.exists() {
        let wasm = fs::read(&output_path)?;
        data["wasm"] = json!({
            "bytes": wasm.len(),
            "sha256": hex_digest(&wasm),
        });
    }
    if !output.is_empty() {
        data["result"] = Value::String(output);
    }
    report.data = Some(data);
    Ok(report)
}

fn contract_ttl(
    context: &AppContext,
    args: &ContractTtlMutationArgs,
    restore: bool,
) -> Result<CommandReport> {
    let mut report = CommandReport::new(if restore {
        "contract.ttl.restore"
    } else {
        "contract.ttl.extend"
    });
    let manifest = load_manifest(context)?;
    let lockfile = load_lockfile(context)?;
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    let contract_id = resolve_contract_id(&manifest, &lockfile, &env, &args.contract);
    let source = manifest
        .active_identity(context.globals.identity.as_deref())
        .unwrap_or(&manifest.defaults.identity)
        .to_string();
    let mut command_args = vec![
        "contract".to_string(),
        if restore {
            "restore".to_string()
        } else {
            "extend".to_string()
        },
        "--id".to_string(),
        contract_id.clone(),
        "--ledgers-to-extend".to_string(),
        args.ledgers.to_string(),
        "--source-account".to_string(),
        source.clone(),
        "--durability".to_string(),
        contract_ttl_durability(args.durability).to_string(),
        "--network".to_string(),
        env.clone(),
    ];
    if let Some(key) = &args.key {
        command_args.push("--key".to_string());
        command_args.push(key.clone());
    }
    if args.ttl_ledger_only {
        command_args.push("--ttl-ledger-only".to_string());
    }
    if args.build_only {
        command_args.push("--build-only".to_string());
    }
    let output = context.run_command(
        &mut report,
        Some(&context.project_root()),
        "stellar",
        &command_args,
    )?;
    report.message = Some(format!(
        "contract TTL {} prepared for `{}`",
        if restore { "restore" } else { "extension" },
        args.contract
    ));
    report.network = Some(env);
    let mut data = json!({
        "contract": args.contract,
        "contract_id": contract_id,
        "source": source,
        "ledgers_to_extend": args.ledgers,
        "durability": contract_ttl_durability(args.durability),
        "key": args.key,
        "mode": if restore { "restore" } else { "extend" },
    });
    if !output.is_empty() {
        data["result"] = Value::String(output);
    }
    report.data = Some(data);
    Ok(report)
}

fn contract_spec(context: &AppContext, contract_name: &str) -> Result<CommandReport> {
    contract_summary(context, contract_name, "contract.spec")
}

fn contract_summary(
    context: &AppContext,
    contract_name: &str,
    action: &str,
) -> Result<CommandReport> {
    let mut report = CommandReport::new(action);
    let manifest = load_manifest(context)?;
    let lockfile = load_lockfile(context)?;
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    let contract = manifest
        .contracts
        .get(contract_name)
        .ok_or_else(|| anyhow!("contract `{contract_name}` not found"))?;
    let contract_dir = context.project_root().join(&contract.path);
    let wasm_path = guess_wasm_path(&contract_dir, contract_name);
    let binding_outputs = contract
        .bindings
        .iter()
        .map(|lang| {
            context
                .project_root()
                .join("packages")
                .join(format!(
                    "{contract_name}-{}",
                    lang.replace("typescript", "ts")
                ))
                .display()
                .to_string()
        })
        .collect::<Vec<_>>();
    let deployment = lockfile
        .environments
        .get(&env)
        .and_then(|environment| environment.contracts.get(contract_name))
        .cloned();
    let effective_init = token::contract_effective_init_config(&manifest, contract_name)?;
    let mut data = json!({
        "contract": {
            "name": contract_name,
            "path": contract.path,
            "alias": contract.alias,
            "template": contract.template,
            "bindings": contract.bindings,
            "deploy_on": contract.deploy_on,
            "init": contract.init,
            "effective_init": effective_init,
        },
        "paths": {
            "contract_dir": contract_dir.display().to_string(),
            "wasm": wasm_path.display().to_string(),
            "bindings": binding_outputs,
        },
        "wasm": {
            "exists": wasm_path.exists(),
            "sha256": if wasm_path.exists() && !context.globals.dry_run {
                Some(hex_digest(&fs::read(&wasm_path)?))
            } else {
                None::<String>
            },
        },
    });
    if let Some((target_args, info_source)) =
        contract_info_target(&wasm_path, deployment.as_ref(), &env)?
    {
        data["info_source"] = info_source;
        let mut info = serde_json::Map::new();
        for (subcommand, output_format) in [
            ("interface", Some("rust")),
            ("meta", Some("json-formatted")),
            ("env-meta", Some("json-formatted")),
        ] {
            let output = run_contract_info_subcommand(
                context,
                &mut report,
                &contract_dir,
                subcommand,
                &target_args,
                output_format,
            )?;
            if !output.is_empty() {
                info.insert(subcommand.replace('-', "_"), Value::String(output));
            }
        }
        if action == "contract.info" {
            let output = run_contract_info_subcommand(
                context,
                &mut report,
                &contract_dir,
                "build",
                &target_args,
                None,
            )?;
            if !output.is_empty() {
                info.insert("build".to_string(), Value::String(output));
            }
        }
        if !info.is_empty() {
            data["info"] = Value::Object(info);
        }
    }
    data["deployment"] = serde_json::to_value(deployment)?;
    report.data = Some(data);
    report.message = Some(format!(
        "{} summary for `{contract_name}`",
        if action == "contract.info" {
            "info"
        } else {
            "spec"
        }
    ));
    report.network = Some(env);
    Ok(report)
}

fn contract_info_target(
    wasm_path: &Path,
    deployment: Option<&ContractDeployment>,
    env: &str,
) -> Result<Option<(Vec<String>, Value)>> {
    if wasm_path.exists() {
        return Ok(Some((
            vec!["--wasm".to_string(), path_to_string(wasm_path)?],
            json!({
                "kind": "wasm",
                "path": wasm_path.display().to_string(),
            }),
        )));
    }
    if let Some(contract_id) = deployment
        .map(|deployment| deployment.contract_id.clone())
        .filter(|contract_id| !contract_id.is_empty())
    {
        return Ok(Some((
            vec![
                "--contract-id".to_string(),
                contract_id.clone(),
                "--network".to_string(),
                env.to_string(),
            ],
            json!({
                "kind": "contract_id",
                "contract_id": contract_id,
                "network": env,
            }),
        )));
    }
    Ok(None)
}

fn run_contract_info_subcommand(
    context: &AppContext,
    report: &mut CommandReport,
    cwd: &Path,
    subcommand: &str,
    target_args: &[String],
    output_format: Option<&str>,
) -> Result<String> {
    let mut args = vec![
        "contract".to_string(),
        "info".to_string(),
        subcommand.to_string(),
    ];
    args.extend(target_args.iter().cloned());
    if let Some(output_format) = output_format {
        args.push("--output".to_string());
        args.push(output_format.to_string());
    }
    context.run_command(report, Some(cwd), "stellar", &args)
}

fn wallet_runtime_identity(wallet: &WalletConfig) -> Option<String> {
    if wallet.kind == "classic" && !wallet.identity.is_empty() {
        Some(wallet.identity.clone())
    } else {
        None
    }
}

fn wallet_controller_identity_value(wallet: &WalletConfig) -> Option<String> {
    wallet
        .controller_identity
        .clone()
        .filter(|identity| !identity.trim().is_empty())
        .or_else(|| {
            if wallet.kind == "smart" && !wallet.identity.trim().is_empty() {
                Some(wallet.identity.clone())
            } else {
                None
            }
        })
}

fn query_standard_token_balance(
    context: &AppContext,
    report: &mut CommandReport,
    env: &str,
    contract_id: &str,
    address: &str,
    source: &str,
    label: &str,
) -> Result<Option<String>> {
    let args = vec![
        "contract".to_string(),
        "invoke".to_string(),
        "--id".to_string(),
        contract_id.to_string(),
        "--source-account".to_string(),
        source.to_string(),
        "--network".to_string(),
        env.to_string(),
        "--send".to_string(),
        "no".to_string(),
        "--".to_string(),
        "balance".to_string(),
        "--id".to_string(),
        address.to_string(),
    ];
    match context.run_command(report, Some(&context.project_root()), "stellar", &args) {
        Ok(output) if !output.is_empty() => Ok(Some(output)),
        Ok(_) => Ok(None),
        Err(error) => {
            report.warnings.push(format!(
                "could not query {label} balance via `stellar contract invoke`: {error}"
            ));
            Ok(None)
        }
    }
}

fn api_init(context: &AppContext) -> Result<CommandReport> {
    let mut report = CommandReport::new("api.init");
    let mut manifest = load_manifest(context)?;
    ensure_api_enabled(&mut manifest);
    sync_api_scaffold(context, &mut report, &context.project_root(), &manifest)?;
    save_manifest(context, &mut report, &manifest)?;
    report.message = Some("API scaffold created".to_string());
    let root = context.project_root();
    report.next = api_app_next_steps(&manifest.project.package_manager);
    report.data = Some(json!({
        "api_root": root.join("apps/api").display().to_string(),
        "openapi": root.join("apps/api/openapi.json").display().to_string(),
        "contracts": manifest.contracts.len(),
        "tokens": manifest.tokens.len(),
        "events_backend": manifest
            .api
            .as_ref()
            .map(|api| api.events_backend.clone())
            .unwrap_or_else(|| "rpc-poller".to_string()),
    }));
    Ok(report)
}

fn api_generate_contract(context: &AppContext, name: &str) -> Result<CommandReport> {
    let mut report = CommandReport::new("api.generate.contract");
    let mut manifest = load_manifest(context)?;
    let contract = manifest
        .contracts
        .get(name)
        .cloned()
        .ok_or_else(|| anyhow!("contract `{name}` is not declared in the manifest"))?;
    ensure_api_enabled(&mut manifest);
    sync_api_scaffold(context, &mut report, &context.project_root(), &manifest)?;
    save_manifest(context, &mut report, &manifest)?;
    report.message = Some(format!("contract API routes generated for `{name}`"));
    let root = context.project_root();
    report.next = api_app_next_steps(&manifest.project.package_manager);
    report.data = Some(json!({
        "contract": name,
        "service_path": root
            .join("apps/api/src/services/contracts")
            .join(format!("{name}.ts"))
            .display()
            .to_string(),
        "route_path": root.join("apps/api/src/routes/contracts.ts").display().to_string(),
        "openapi_path": root.join("apps/api/openapi.json").display().to_string(),
        "bindings": contract.bindings.clone(),
        "typescript_binding": contract
            .bindings
            .iter()
            .any(|binding| matches!(binding.as_str(), "typescript" | "javascript"))
            .then(|| format!("packages/{name}-ts")),
    }));
    Ok(report)
}

fn api_generate_token(context: &AppContext, name: &str) -> Result<CommandReport> {
    let mut report = CommandReport::new("api.generate.token");
    let mut manifest = load_manifest(context)?;
    let token = manifest
        .tokens
        .get(name)
        .cloned()
        .ok_or_else(|| anyhow!("token `{name}` is not declared in the manifest"))?;
    ensure_api_enabled(&mut manifest);
    sync_api_scaffold(context, &mut report, &context.project_root(), &manifest)?;
    save_manifest(context, &mut report, &manifest)?;
    report.message = Some(format!("token API routes generated for `{name}`"));
    let root = context.project_root();
    let builders = if token.kind == "contract" {
        vec!["balance", "payment", "mint"]
    } else if token.with_sac {
        vec!["balance", "payment", "mint", "trust", "sac_transfer"]
    } else {
        vec!["balance", "payment", "mint", "trust"]
    };
    report.next = api_app_next_steps(&manifest.project.package_manager);
    report.data = Some(json!({
        "token": name,
        "kind": token.kind.clone(),
        "with_sac": token.with_sac,
        "service_path": root
            .join("apps/api/src/services/tokens")
            .join(format!("{name}.ts"))
            .display()
            .to_string(),
        "route_path": root.join("apps/api/src/routes/tokens.ts").display().to_string(),
        "openapi_path": root.join("apps/api/openapi.json").display().to_string(),
        "builders": builders,
    }));
    Ok(report)
}

fn api_openapi_export(context: &AppContext) -> Result<CommandReport> {
    let mut report = CommandReport::new("api.openapi.export");
    let mut manifest = load_manifest(context)?;
    ensure_api_enabled(&mut manifest);
    sync_api_scaffold(context, &mut report, &context.project_root(), &manifest)?;
    save_manifest(context, &mut report, &manifest)?;
    let openapi = build_openapi(&manifest);
    let path_count = openapi
        .get("paths")
        .and_then(Value::as_object)
        .map(|paths| paths.len())
        .unwrap_or(0);
    report.message = Some("OpenAPI document exported to apps/api/openapi.json".to_string());
    let root = context.project_root();
    report.next = api_app_next_steps(&manifest.project.package_manager);
    report.data = Some(json!({
        "openapi_path": root.join("apps/api/openapi.json").display().to_string(),
        "path_count": path_count,
    }));
    Ok(report)
}

fn api_events_init(context: &AppContext) -> Result<CommandReport> {
    let mut report = CommandReport::new("api.events.init");
    let mut manifest = load_manifest(context)?;
    let mut api = manifest.api.unwrap_or_default();
    api.enabled = true;
    api.openapi = true;
    manifest.api = Some(api);
    sync_api_scaffold(context, &mut report, &context.project_root(), &manifest)?;
    save_manifest(context, &mut report, &manifest)?;
    report.message = Some("event ingestion worker initialized".to_string());
    Ok(report)
}

fn api_relayer_init(context: &AppContext) -> Result<CommandReport> {
    let mut report = CommandReport::new("api.relayer.init");
    let mut manifest = load_manifest(context)?;
    let mut api = manifest.api.unwrap_or_default();
    api.enabled = true;
    api.relayer = true;
    api.openapi = true;
    manifest.api = Some(api);
    sync_api_scaffold(context, &mut report, &context.project_root(), &manifest)?;
    let env_path = context.project_root().join("apps/api/.env.example");
    let mut env = if env_path.exists() {
        context.read_text(&env_path)?
    } else {
        String::new()
    };
    for line in [
        "RELAYER_BASE_URL=",
        "RELAYER_API_KEY=",
        "RELAYER_SUBMIT_PATH=/transactions",
    ] {
        if !env.contains(line) {
            env.push_str(line);
            env.push('\n');
        }
    }
    context.write_text(&mut report, &env_path, &env)?;
    save_manifest(context, &mut report, &manifest)?;
    report.message = Some("relayer integration scaffolded for the API".to_string());
    Ok(report)
}

fn release_verify(context: &AppContext, env: &str) -> Result<CommandReport> {
    release::release_verify(context, env)
}

fn release_env_export(context: &AppContext, env: &str) -> Result<CommandReport> {
    release::release_env_export(context, env)
}

fn contract_fetch_output_path(root: &Path, env: &str, contract_name: &str) -> PathBuf {
    root.join("dist").join("contracts").join(format!(
        "{}.{}.wasm",
        artifact_token(contract_name),
        env
    ))
}

fn artifact_token(input: &str) -> String {
    input
        .chars()
        .map(|char| {
            if char.is_ascii_alphanumeric() || matches!(char, '-' | '_') {
                char
            } else {
                '-'
            }
        })
        .collect()
}

fn contract_ttl_durability(durability: StorageDurability) -> &'static str {
    match durability {
        StorageDurability::Persistent => "persistent",
        StorageDurability::Temporary => "temporary",
    }
}

fn resolve_registry_cli(context: &AppContext) -> release::RegistryCli {
    release::resolve_registry_cli(context)
}

fn project_has_registry_artifacts(root: &Path) -> bool {
    release::project_has_registry_artifacts(root)
}

fn render_api_manifest_module(manifest: &Manifest) -> Result<String> {
    doctor::render_api_manifest_module(manifest)
}

fn save_manifest(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: &Manifest,
) -> Result<()> {
    context.write_text(
        report,
        &context.manifest_path,
        &toml::to_string_pretty(manifest)?,
    )
}

fn save_lockfile(
    context: &AppContext,
    report: &mut CommandReport,
    lockfile: &Lockfile,
) -> Result<()> {
    context.write_text(
        report,
        &context.project_root().join("stellarforge.lock.json"),
        &serde_json::to_string_pretty(lockfile)?,
    )
}

fn load_manifest(context: &AppContext) -> Result<Manifest> {
    Manifest::load(&context.manifest_path)
}

fn load_lockfile(context: &AppContext) -> Result<Lockfile> {
    Lockfile::load(&context.project_root().join("stellarforge.lock.json"))
}

fn sync_api_scaffold(
    context: &AppContext,
    report: &mut CommandReport,
    root: &Path,
    manifest: &Manifest,
) -> Result<()> {
    let api_root = root.join("apps/api");
    let relayer_enabled = manifest.api.as_ref().is_some_and(|api| api.relayer);
    context.ensure_dir(report, &api_root.join("src/routes"))?;
    context.ensure_dir(report, &api_root.join("src/services"))?;
    context.ensure_dir(report, &api_root.join("src/services/contracts"))?;
    context.ensure_dir(report, &api_root.join("src/services/events"))?;
    context.ensure_dir(report, &api_root.join("src/services/tokens"))?;
    context.ensure_dir(report, &api_root.join("src/workers"))?;
    context.ensure_dir(report, &api_root.join("src/lib"))?;
    context.ensure_dir(report, &api_root.join("db"))?;
    context.write_text(
        report,
        &api_root.join("package.json"),
        templates::api_package_json(),
    )?;
    context.write_text(
        report,
        &api_root.join("tsconfig.json"),
        templates::api_tsconfig(),
    )?;
    context.write_text(
        report,
        &api_root.join(".env.example"),
        &templates::api_env_example(manifest),
    )?;
    context.write_text(
        report,
        &api_root.join("src/server.ts"),
        &templates::api_server(manifest),
    )?;
    context.write_text(
        report,
        &api_root.join("src/routes/health.ts"),
        templates::api_health_routes(),
    )?;
    context.write_text(
        report,
        &api_root.join("src/routes/contracts.ts"),
        &render_contract_routes(manifest),
    )?;
    context.write_text(
        report,
        &api_root.join("src/routes/events.ts"),
        &render_event_routes(manifest),
    )?;
    context.write_text(
        report,
        &api_root.join("src/routes/relayer.ts"),
        templates::api_relayer_routes(),
    )?;
    context.write_text(
        report,
        &api_root.join("src/routes/tokens.ts"),
        &render_token_routes(manifest),
    )?;
    context.write_text(
        report,
        &api_root.join("src/routes/wallets.ts"),
        templates::api_wallet_routes(),
    )?;
    context.write_text(
        report,
        &api_root.join("src/services/rpc.ts"),
        templates::api_rpc_service(),
    )?;
    context.write_text(
        report,
        &api_root.join("src/services/relayer.ts"),
        templates::api_relayer_service(),
    )?;
    for (name, contract) in &manifest.contracts {
        context.write_text(
            report,
            &api_root
                .join("src/services/contracts")
                .join(format!("{name}.ts")),
            &templates::api_contract_resource_service(name, contract, relayer_enabled),
        )?;
    }
    for (name, token) in &manifest.tokens {
        context.write_text(
            report,
            &api_root
                .join("src/services/tokens")
                .join(format!("{name}.ts")),
            &templates::api_token_resource_service(name, token),
        )?;
    }
    context.write_text(
        report,
        &api_root.join("src/lib/config.ts"),
        templates::api_config(),
    )?;
    context.write_text(
        report,
        &api_root.join("src/lib/errors.ts"),
        templates::api_errors(),
    )?;
    context.write_text(
        report,
        &api_root.join("src/lib/events-store.ts"),
        templates::api_events_store(),
    )?;
    context.write_text(
        report,
        &api_root.join("src/lib/manifest.ts"),
        &render_api_manifest_module(manifest)?,
    )?;
    context.write_text(
        report,
        &api_root.join("src/workers/ingest-events.ts"),
        templates::api_events_worker(),
    )?;
    context.write_text(
        report,
        &api_root.join("db/schema.sql"),
        templates::api_events_schema(),
    )?;
    context.write_text(
        report,
        &api_root.join("openapi.json"),
        &serde_json::to_string_pretty(&build_openapi(manifest))?,
    )?;
    Ok(())
}

fn sync_frontend_scaffold(
    context: &AppContext,
    report: &mut CommandReport,
    root: &Path,
    manifest: &Manifest,
) -> Result<()> {
    let web_root = root.join("apps/web");
    context.ensure_dir(report, &web_root.join("src"))?;
    context.ensure_dir(report, &web_root.join("src/generated"))?;
    context.write_text(
        report,
        &web_root.join("package.json"),
        templates::web_package_json(),
    )?;
    context.write_text(
        report,
        &web_root.join("index.html"),
        templates::web_index_html(),
    )?;
    context.write_text(
        report,
        &web_root.join("src/main.tsx"),
        &templates::web_main(manifest),
    )?;
    sync_frontend_generated_state(context, report, root, manifest, &manifest.defaults.network)?;
    Ok(())
}

fn sync_frontend_generated_state(
    context: &AppContext,
    report: &mut CommandReport,
    root: &Path,
    manifest: &Manifest,
    env: &str,
) -> Result<()> {
    let web_root = root.join("apps/web");
    context.ensure_dir(report, &web_root.join("src/generated"))?;
    let lockfile = Lockfile::load(&root.join("stellarforge.lock.json"))?;
    let event_cursors = load_event_cursors(root)?;
    context.write_text(
        report,
        &web_root.join("src/generated/stellar.ts"),
        &templates::web_generated_state(manifest, &lockfile, &event_cursors, env),
    )
}

fn load_event_cursors(root: &Path) -> Result<Value> {
    let cursor_path = root.join("workers/events/cursors.json");
    if !cursor_path.exists() {
        return Ok(json!({ "cursors": {} }));
    }

    let value = serde_json::from_str::<Value>(
        &fs::read_to_string(&cursor_path)
            .with_context(|| format!("failed to read {}", cursor_path.display()))?,
    )?;

    if value.get("cursors").is_none() {
        return Ok(json!({ "cursors": {} }));
    }

    Ok(value)
}

#[derive(Debug, Clone, Deserialize)]
struct EventCursorRow {
    name: String,
    resource_kind: String,
    resource_name: String,
    cursor: Option<String>,
    last_ledger: Option<i64>,
    updated_at: String,
}

#[derive(Debug, Clone)]
struct EventStorePaths {
    api_root: PathBuf,
    db_path: PathBuf,
    schema_path: PathBuf,
    snapshot_path: PathBuf,
}

#[derive(Debug, Clone)]
struct ResolvedEventResource {
    kind: String,
    name: String,
    contract_id: String,
}

#[derive(Debug, Clone)]
struct BackfillEventRow {
    external_id: String,
    cursor: Option<String>,
    contract_id: String,
    event_type: String,
    topic: String,
    payload: String,
    tx_hash: Option<String>,
    ledger: Option<i64>,
    observed_at: String,
}

#[derive(Debug, Clone)]
struct EventQueryOptions {
    count: Option<u32>,
    cursor: Option<String>,
    start_ledger: Option<u64>,
    topics: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct AccountWatchParams<'a> {
    manifest: &'a Manifest,
    env: &'a str,
    network: &'a NetworkConfig,
    resource: &'a str,
    query: &'a EventQueryOptions,
    topics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct AccountBackfillParams<'a> {
    manifest: &'a Manifest,
    root: &'a Path,
    env: &'a str,
    resource: &'a str,
    query: &'a EventQueryOptions,
    topics: &'a [String],
}

fn sync_frontend_for_event_change(
    context: &AppContext,
    report: &mut CommandReport,
    root: &Path,
    manifest: &Manifest,
    env: &str,
) -> Result<()> {
    if manifest
        .frontend
        .as_ref()
        .is_some_and(|frontend| frontend.enabled)
    {
        sync_frontend_generated_state(context, report, root, manifest, env)?;
    }
    Ok(())
}

fn populate_account_watch_report(
    context: &AppContext,
    report: &mut CommandReport,
    params: &AccountWatchParams<'_>,
) -> Result<()> {
    let resolved_address =
        resolve_account_history_address(context, report, params.manifest, params.resource)?;
    let endpoint = horizon_account_payments_url(
        params.network,
        &resolved_address,
        params.query.count.or(Some(10)),
        params.query.cursor.as_deref(),
        "desc",
    )?;
    report.network = Some(params.env.to_string());
    report.commands.push(format!("GET {endpoint}"));
    append_account_history_warnings(report, params.query, params.topics);
    report.next = vec![format!("stellar forge wallet balances {}", params.resource)];

    let records = if context.globals.dry_run {
        Vec::new()
    } else {
        let value = context.get_json(&endpoint)?;
        extract_event_rows(&value)
            .into_iter()
            .map(|event| account_payment_summary(event, &resolved_address))
            .collect::<Vec<_>>()
    };

    report.message = Some(if context.globals.dry_run {
        format!(
            "planned an account payment lookup for `{}` via Horizon",
            params.resource
        )
    } else {
        format!(
            "fetched {} recent account payment record(s) for `{}`",
            records.len(),
            params.resource
        )
    });
    report.data = Some(json!({
        "kind": "account",
        "resource": params.resource,
        "resolved_address": resolved_address,
        "source": "horizon",
        "stream": "account_payments",
        "endpoint": endpoint.as_str(),
        "topics": params.topics,
        "resolved_topics": params.query.topics,
        "count": params.query.count.or(Some(10)),
        "cursor": params.query.cursor,
        "start_ledger": params.query.start_ledger,
        "records": records,
    }));
    Ok(())
}

fn populate_account_backfill_report(
    context: &AppContext,
    report: &mut CommandReport,
    params: &AccountBackfillParams<'_>,
) -> Result<()> {
    let network = params
        .manifest
        .networks
        .get(params.env)
        .ok_or_else(|| anyhow!("network `{}` not found", params.env))?;
    let resolved_address =
        resolve_account_history_address(context, report, params.manifest, params.resource)?;
    let cursor_name = format!("{}:account:{}", params.env, params.resource);
    let paths = event_store_paths(params.root);

    if !paths.api_root.exists() {
        bail!(
            "event backfill needs the API scaffold; run `stellar forge events ingest init` first"
        );
    }

    if !context.command_exists("sqlite3") && !context.globals.dry_run {
        bail!(
            "`sqlite3` is required for `events backfill` because the command persists imported events"
        );
    }

    context.ensure_dir(report, &paths.api_root.join("db"))?;
    if !paths.schema_path.exists() {
        report.warnings.push(format!(
            "event schema was missing and has been recreated at {}",
            paths.schema_path.display()
        ));
        context.write_text(report, &paths.schema_path, templates::api_events_schema())?;
    }
    sqlite_exec(
        context,
        report,
        &paths.db_path,
        templates::api_events_schema(),
    )?;

    let current_snapshot = load_event_cursors(params.root)?;
    let current_cursor = load_sqlite_event_cursor(context, report, params.root, &cursor_name)?
        .or_else(|| snapshot_cursor_row(&current_snapshot, &cursor_name))
        .unwrap_or_else(|| EventCursorRow {
            name: cursor_name.clone(),
            resource_kind: "account".to_string(),
            resource_name: params.resource.to_string(),
            cursor: None,
            last_ledger: None,
            updated_at: Utc::now().to_rfc3339(),
        });
    let effective_cursor = params.query.cursor.clone().or_else(|| {
        current_cursor
            .cursor
            .as_ref()
            .filter(|cursor| !cursor.is_empty())
            .cloned()
    });
    let endpoint = horizon_account_payments_url(
        network,
        &resolved_address,
        params.query.count.or(Some(200)),
        effective_cursor.as_deref(),
        "asc",
    )?;

    report.network = Some(params.env.to_string());
    report.commands.push(format!("GET {endpoint}"));
    append_account_history_warnings(report, params.query, params.topics);
    report.warnings.push("account payment backfill uses Horizon translation for classic account history, not Soroban RPC event retention".to_string());

    if context.globals.dry_run {
        report.message = Some(format!(
            "planned an account payment backfill for `{}` into {}",
            params.resource,
            paths.db_path.display()
        ));
        report.data = Some(json!({
            "resource": {
                "kind": "account",
                "name": params.resource,
                "contract_id": resolved_address,
            },
            "cursor_name": cursor_name,
            "source": "horizon",
            "stream": "account_payments",
            "endpoint": endpoint.as_str(),
            "topics": params.topics,
            "resolved_topics": params.query.topics,
            "count": params.query.count.or(Some(200)),
            "cursor": effective_cursor,
            "start_ledger": params.query.start_ledger,
            "db_path": paths.db_path.display().to_string(),
            "schema_path": paths.schema_path.display().to_string(),
            "snapshot_path": paths.snapshot_path.display().to_string(),
        }));
        return Ok(());
    }

    let raw_value = context.get_json(&endpoint)?;
    let events = extract_event_rows(&raw_value)
        .into_iter()
        .map(|event| normalize_account_backfill_event(event, params.resource, &resolved_address))
        .collect::<Vec<_>>();

    if events.is_empty() {
        report.status = "warn".to_string();
        report.message = Some(format!(
            "no account payment history found for `{}`",
            params.resource
        ));
        report.data = Some(json!({
            "resource": {
                "kind": "account",
                "name": params.resource,
                "contract_id": resolved_address,
            },
            "cursor_name": cursor_name,
            "source": "horizon",
            "stream": "account_payments",
            "event_count": 0,
        }));
        return Ok(());
    }

    let Some(last_event) = events.last().cloned() else {
        return Ok(());
    };
    let sql = build_backfill_sql(
        &events,
        &EventCursorRow {
            name: cursor_name.clone(),
            resource_kind: "account".to_string(),
            resource_name: params.resource.to_string(),
            cursor: last_event.cursor.clone(),
            last_ledger: last_event.ledger,
            updated_at: Utc::now().to_rfc3339(),
        },
    );
    sqlite_exec(context, report, &paths.db_path, &sql)?;

    let rows = load_sqlite_event_cursors(context, report, params.root)?.unwrap_or_default();
    write_cursor_snapshot(context, report, &paths.snapshot_path, &rows)?;
    sync_frontend_for_event_change(context, report, params.root, params.manifest, params.env)?;

    report.message = Some(format!(
        "imported {} account payment record(s) for `{}`",
        events.len(),
        params.resource
    ));
    report.data = Some(json!({
        "resource": {
            "kind": "account",
            "name": params.resource,
            "contract_id": resolved_address,
        },
        "cursor_name": cursor_name,
        "source": "horizon",
        "stream": "account_payments",
        "event_count": events.len(),
        "latest_ledger": last_event.ledger,
        "endpoint": endpoint.as_str(),
        "topics": params.topics,
        "resolved_topics": params.query.topics,
        "count": params.query.count.or(Some(200)),
        "cursor": effective_cursor,
        "start_ledger": params.query.start_ledger,
        "db_path": paths.db_path.display().to_string(),
        "snapshot_path": paths.snapshot_path.display().to_string(),
    }));
    Ok(())
}

fn resolve_account_history_address(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: &Manifest,
    resource: &str,
) -> Result<String> {
    let resolved = resolve_address(context, report, Some(manifest), resource)?;
    if !resolved.starts_with('<') && !is_horizon_account_id(&resolved) {
        bail!(
            "account payment history requires a classic or muxed Stellar address; got `{resolved}`"
        );
    }
    Ok(resolved)
}

fn horizon_account_payments_url(
    network: &NetworkConfig,
    address: &str,
    count: Option<u32>,
    cursor: Option<&str>,
    order: &str,
) -> Result<Url> {
    let mut url = Url::parse(&network.horizon_url)?;
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| anyhow!("invalid Horizon URL `{}`", network.horizon_url))?;
        segments.push("accounts");
        segments.push(address);
        segments.push("payments");
    }
    let mut pairs = url.query_pairs_mut();
    if let Some(count) = count {
        pairs.append_pair("limit", &count.to_string());
    }
    if let Some(cursor) = cursor {
        pairs.append_pair("cursor", cursor);
    }
    pairs.append_pair("order", order);
    drop(pairs);
    Ok(url)
}

fn append_account_history_warnings(
    report: &mut CommandReport,
    query: &EventQueryOptions,
    topics: &[String],
) {
    if !topics.is_empty() {
        report.warnings.push(
            "topic filters only apply to Soroban contract events; Horizon account payment lookups ignore them"
                .to_string(),
        );
    }
    if query.start_ledger.is_some() {
        report.warnings.push(
            "`--start-ledger` applies to Soroban RPC events; Horizon account payment lookups use cursors instead"
                .to_string(),
        );
    }
}

fn account_event_resource_name(manifest: &Manifest, resource: &str) -> Option<String> {
    if let Some((kind, name)) = resource.split_once(':') {
        return match kind {
            "account" => Some(name.to_string()),
            _ => None,
        };
    }
    if manifest.contracts.contains_key(resource)
        || manifest.tokens.contains_key(resource)
        || is_contract_address(resource)
    {
        return None;
    }
    if manifest.wallets.contains_key(resource)
        || manifest.identities.contains_key(resource)
        || is_horizon_account_id(resource)
    {
        return Some(resource.to_string());
    }
    None
}

fn event_query_options(
    count: Option<u32>,
    cursor: &Option<String>,
    start_ledger: Option<u64>,
    topics: &[String],
) -> Result<EventQueryOptions> {
    if cursor.is_some() && start_ledger.is_some() {
        bail!("use either `--cursor` or `--start-ledger`, not both");
    }
    Ok(EventQueryOptions {
        count,
        cursor: cursor.clone(),
        start_ledger,
        topics: topics
            .iter()
            .map(|topic| normalize_event_topic_filter(topic))
            .collect::<Result<Vec<_>>>()?,
    })
}

fn normalize_event_topic_filter(filter: &str) -> Result<String> {
    let segments = filter.split(',').map(str::trim).collect::<Vec<_>>();
    if segments.is_empty() || segments.iter().any(|segment| segment.is_empty()) {
        bail!(
            "topic filter `{filter}` is invalid; use 1-4 comma-separated segments such as `COUNTER,*`"
        );
    }
    if segments.len() > 5 {
        bail!(
            "topic filter `{filter}` has too many segments; use at most 4 segments plus an optional trailing `**`"
        );
    }
    if let Some(index) = segments.iter().position(|segment| *segment == "**")
        && index + 1 != segments.len()
    {
        bail!(
            "topic filter `{filter}` uses `**` before the end; the flexible wildcard must be the last segment"
        );
    }
    segments
        .iter()
        .map(|segment| normalize_event_topic_segment(segment))
        .collect::<Result<Vec<_>>>()
        .map(|segments| segments.join(","))
}

fn normalize_event_topic_segment(segment: &str) -> Result<String> {
    if matches!(segment, "*" | "**") {
        return Ok(segment.to_string());
    }
    if looks_like_xdr_topic_segment(segment) {
        return Ok(segment.to_string());
    }

    if let Some(value) = segment
        .strip_prefix("sym:")
        .or_else(|| segment.strip_prefix("symbol:"))
    {
        return encode_scval_symbol(value);
    }
    if let Some(value) = segment
        .strip_prefix("str:")
        .or_else(|| segment.strip_prefix("string:"))
    {
        return Ok(encode_scval_string(value));
    }
    if let Some(value) = segment.strip_prefix("bool:") {
        return encode_scval_bool(value);
    }
    if let Some(value) = segment.strip_prefix("u32:") {
        return encode_scval_u32(value);
    }
    if let Some(value) = segment.strip_prefix("i32:") {
        return encode_scval_i32(value);
    }
    if let Some(value) = segment.strip_prefix("u64:") {
        return encode_scval_u64(value);
    }
    if let Some(value) = segment.strip_prefix("i64:") {
        return encode_scval_i64(value);
    }

    if looks_like_symbol_segment(segment) {
        return encode_scval_symbol(segment);
    }

    Ok(encode_scval_string(segment))
}

fn looks_like_xdr_topic_segment(segment: &str) -> bool {
    if segment.len() < 8 || !segment.len().is_multiple_of(4) {
        return false;
    }
    BASE64
        .decode(segment)
        .map(|bytes| bytes.len() >= 8 && bytes.len() % 4 == 0)
        .unwrap_or(false)
}

fn looks_like_symbol_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment.len() <= 32
        && segment
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn encode_scval_bool(raw: &str) -> Result<String> {
    match raw {
        "true" => Ok(encode_scval_with_payload(0, &[0, 0, 0, 1])),
        "false" => Ok(encode_scval_with_payload(0, &[0, 0, 0, 0])),
        _ => bail!("invalid bool topic segment `{raw}`; use `bool:true` or `bool:false`"),
    }
}

fn encode_scval_u32(raw: &str) -> Result<String> {
    let value = raw
        .parse::<u32>()
        .with_context(|| format!("invalid u32 topic segment `{raw}`"))?;
    Ok(encode_scval_with_payload(3, &value.to_be_bytes()))
}

fn encode_scval_i32(raw: &str) -> Result<String> {
    let value = raw
        .parse::<i32>()
        .with_context(|| format!("invalid i32 topic segment `{raw}`"))?;
    Ok(encode_scval_with_payload(4, &value.to_be_bytes()))
}

fn encode_scval_u64(raw: &str) -> Result<String> {
    let value = raw
        .parse::<u64>()
        .with_context(|| format!("invalid u64 topic segment `{raw}`"))?;
    Ok(encode_scval_with_payload(5, &value.to_be_bytes()))
}

fn encode_scval_i64(raw: &str) -> Result<String> {
    let value = raw
        .parse::<i64>()
        .with_context(|| format!("invalid i64 topic segment `{raw}`"))?;
    Ok(encode_scval_with_payload(6, &value.to_be_bytes()))
}

fn encode_scval_symbol(raw: &str) -> Result<String> {
    if !looks_like_symbol_segment(raw) {
        bail!(
            "invalid symbol topic segment `{raw}`; symbols must be ASCII alphanumeric/underscore and up to 32 characters"
        );
    }
    Ok(encode_scval_string_like(15, raw))
}

fn encode_scval_string(raw: &str) -> String {
    encode_scval_string_like(14, raw)
}

fn encode_scval_string_like(tag: u32, raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut payload = Vec::with_capacity(4 + bytes.len() + 3);
    payload.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    payload.extend_from_slice(bytes);
    while payload.len() % 4 != 0 {
        payload.push(0);
    }
    encode_scval_with_payload(tag, &payload)
}

fn encode_scval_with_payload(tag: u32, payload: &[u8]) -> String {
    let mut bytes = Vec::with_capacity(4 + payload.len());
    bytes.extend_from_slice(&tag.to_be_bytes());
    bytes.extend_from_slice(payload);
    BASE64.encode(bytes)
}

fn append_event_query_args(
    args: &mut Vec<String>,
    contract_id: &str,
    query: &EventQueryOptions,
    default_count: Option<u32>,
) {
    args.push("--id".to_string());
    args.push(contract_id.to_string());

    if let Some(count) = query.count.or(default_count) {
        args.push("--count".to_string());
        args.push(count.to_string());
    }

    if let Some(cursor) = query.cursor.as_ref() {
        args.push("--cursor".to_string());
        args.push(cursor.clone());
    } else if let Some(start_ledger) = query.start_ledger {
        args.push("--start-ledger".to_string());
        args.push(start_ledger.to_string());
    }

    for topic in &query.topics {
        args.push("--topic".to_string());
        args.push(topic.clone());
    }
}

fn event_store_paths(root: &Path) -> EventStorePaths {
    let api_root = root.join("apps").join("api");
    let env = load_event_env_values(root, &api_root);
    let db_path = resolve_api_relative(
        &api_root,
        env.get("STELLAR_EVENTS_DB_PATH")
            .map(String::as_str)
            .unwrap_or("./db/events.sqlite"),
    );
    let schema_path = resolve_api_relative(
        &api_root,
        env.get("STELLAR_EVENTS_SCHEMA_PATH")
            .map(String::as_str)
            .unwrap_or("./db/schema.sql"),
    );
    let snapshot_path = resolve_api_relative(
        &api_root,
        env.get("STELLAR_EVENTS_CURSOR_FILE")
            .map(String::as_str)
            .unwrap_or("../../workers/events/cursors.json"),
    );
    EventStorePaths {
        api_root,
        db_path,
        schema_path,
        snapshot_path,
    }
}

fn load_event_env_values(root: &Path, api_root: &Path) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    for path in [
        api_root.join(".env"),
        api_root.join(".env.local"),
        root.join(".env"),
        root.join(".env.local"),
        root.join(".env.generated"),
    ] {
        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };
        for (key, value) in parse_env_assignments(&contents) {
            values.entry(key).or_insert(value);
        }
    }
    values
}

fn parse_env_assignments(contents: &str) -> Vec<(String, String)> {
    contents
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            let normalized = trimmed.strip_prefix("export ").unwrap_or(trimmed);
            let (key, raw_value) = normalized.split_once('=')?;
            let value = raw_value
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            Some((key.trim().to_string(), value))
        })
        .collect()
}

fn resolve_api_relative(api_root: &Path, candidate: &str) -> PathBuf {
    let path = PathBuf::from(candidate);
    if path.is_absolute() {
        path
    } else {
        api_root.join(path)
    }
}

fn sqlite_query_json(
    context: &AppContext,
    report: &mut CommandReport,
    db_path: &Path,
    sql: &str,
) -> Result<Value> {
    let stdout = context.run_command(
        report,
        Some(&context.project_root()),
        "sqlite3",
        &[
            path_to_string(db_path)?,
            "-json".to_string(),
            sql.to_string(),
        ],
    )?;
    if stdout.trim().is_empty() {
        return Ok(json!([]));
    }
    Ok(serde_json::from_str(&stdout)?)
}

fn sqlite_exec(
    context: &AppContext,
    report: &mut CommandReport,
    db_path: &Path,
    sql: &str,
) -> Result<()> {
    context.run_command(
        report,
        Some(&context.project_root()),
        "sqlite3",
        &[path_to_string(db_path)?, sql.to_string()],
    )?;
    Ok(())
}

fn load_sqlite_event_cursors(
    context: &AppContext,
    report: &mut CommandReport,
    root: &Path,
) -> Result<Option<Vec<EventCursorRow>>> {
    let paths = event_store_paths(root);
    if !paths.db_path.exists() || !context.command_exists("sqlite3") {
        return Ok(None);
    }
    let rows = sqlite_query_json(
        context,
        report,
        &paths.db_path,
        "select name, resource_kind, resource_name, cursor, last_ledger, updated_at from cursors order by name asc;",
    )?;
    Ok(Some(serde_json::from_value(rows)?))
}

fn load_sqlite_event_cursor(
    context: &AppContext,
    report: &mut CommandReport,
    root: &Path,
    name: &str,
) -> Result<Option<EventCursorRow>> {
    let paths = event_store_paths(root);
    if !paths.db_path.exists() || !context.command_exists("sqlite3") {
        return Ok(None);
    }
    let rows = sqlite_query_json(
        context,
        report,
        &paths.db_path,
        &format!(
            "select name, resource_kind, resource_name, cursor, last_ledger, updated_at from cursors where name = {};",
            sqlite_quote(name)
        ),
    )?;
    let mut parsed = serde_json::from_value::<Vec<EventCursorRow>>(rows)?;
    Ok(parsed.pop())
}

fn cursor_rows_to_value(rows: &[EventCursorRow]) -> Value {
    let cursors = rows
        .iter()
        .map(|row| {
            (
                row.name.clone(),
                json!({
                    "resource_kind": row.resource_kind,
                    "resource_name": row.resource_name,
                    "cursor": row.cursor,
                    "last_ledger": row.last_ledger,
                    "updated_at": row.updated_at,
                }),
            )
        })
        .collect::<serde_json::Map<String, Value>>();
    json!({ "cursors": cursors })
}

fn write_cursor_snapshot(
    context: &AppContext,
    report: &mut CommandReport,
    snapshot_path: &Path,
    rows: &[EventCursorRow],
) -> Result<()> {
    context.write_text(
        report,
        snapshot_path,
        &serde_json::to_string_pretty(&cursor_rows_to_value(rows))?,
    )
}

fn snapshot_cursor_row(snapshot: &Value, name: &str) -> Option<EventCursorRow> {
    let entry = snapshot.get("cursors")?.get(name)?;
    if let Some(cursor) = entry.as_str() {
        return Some(EventCursorRow {
            name: name.to_string(),
            resource_kind: "unknown".to_string(),
            resource_name: name.to_string(),
            cursor: Some(cursor.to_string()),
            last_ledger: None,
            updated_at: Utc::now().to_rfc3339(),
        });
    }
    let object = entry.as_object()?;
    Some(EventCursorRow {
        name: name.to_string(),
        resource_kind: object
            .get("resource_kind")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        resource_name: object
            .get("resource_name")
            .and_then(Value::as_str)
            .unwrap_or(name)
            .to_string(),
        cursor: object
            .get("cursor")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        last_ledger: object.get("last_ledger").and_then(Value::as_i64),
        updated_at: object
            .get("updated_at")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
    })
}

fn resolve_event_resource(
    manifest: &Manifest,
    lockfile: &Lockfile,
    env: &str,
    resource: &str,
) -> Result<ResolvedEventResource> {
    if let Some((kind, name)) = resource.split_once(':') {
        return match kind {
            "contract" => Ok(ResolvedEventResource {
                kind: "contract".to_string(),
                name: name.to_string(),
                contract_id: resolve_contract_id(manifest, lockfile, env, name),
            }),
            "token" => {
                let deployment = lockfile
                    .environments
                    .get(env)
                    .and_then(|environment| environment.tokens.get(name))
                    .ok_or_else(|| {
                        anyhow!("token `{name}` has no materialized deployment in `{env}`")
                    })?;
                let contract_id = if !deployment.sac_contract_id.is_empty() {
                    deployment.sac_contract_id.clone()
                } else if !deployment.contract_id.is_empty() {
                    deployment.contract_id.clone()
                } else {
                    bail!(
                        "token `{name}` does not have a contract wrapper in `{env}`; deploy a SAC or token contract first"
                    );
                };
                Ok(ResolvedEventResource {
                    kind: "token".to_string(),
                    name: name.to_string(),
                    contract_id,
                })
            }
            "account" => bail!(
                "use `events backfill account:<name>` for Horizon-backed account payment history"
            ),
            _ => bail!("unsupported event resource prefix `{kind}`"),
        };
    }

    if manifest.contracts.contains_key(resource) || is_contract_address(resource) {
        return Ok(ResolvedEventResource {
            kind: "contract".to_string(),
            name: resource.to_string(),
            contract_id: resolve_contract_id(manifest, lockfile, env, resource),
        });
    }

    if manifest.tokens.contains_key(resource) {
        let deployment = lockfile
            .environments
            .get(env)
            .and_then(|environment| environment.tokens.get(resource))
            .ok_or_else(|| {
                anyhow!("token `{resource}` has no materialized deployment in `{env}`")
            })?;
        let contract_id = if !deployment.sac_contract_id.is_empty() {
            deployment.sac_contract_id.clone()
        } else if !deployment.contract_id.is_empty() {
            deployment.contract_id.clone()
        } else {
            bail!(
                "token `{resource}` does not have a contract wrapper in `{env}`; deploy a SAC or token contract first"
            );
        };
        return Ok(ResolvedEventResource {
            kind: "token".to_string(),
            name: resource.to_string(),
            contract_id,
        });
    }

    bail!(
        "resource `{resource}` was not recognized; use a contract name, token name, or an explicit prefix like `contract:{resource}`"
    )
}

fn extract_event_rows(value: &Value) -> Vec<&Value> {
    if let Some(rows) = value.as_array() {
        return rows.iter().collect();
    }
    if let Some(rows) = value.get("events").and_then(Value::as_array) {
        return rows.iter().collect();
    }
    if let Some(rows) = value.get("records").and_then(Value::as_array) {
        return rows.iter().collect();
    }
    if let Some(rows) = value
        .get("_embedded")
        .and_then(|embedded| embedded.get("records"))
        .and_then(Value::as_array)
    {
        return rows.iter().collect();
    }
    if let Some(rows) = value
        .get("result")
        .and_then(|result| result.get("events"))
        .and_then(Value::as_array)
    {
        return rows.iter().collect();
    }
    if let Some(rows) = value
        .get("result")
        .and_then(|result| result.get("records"))
        .and_then(Value::as_array)
    {
        return rows.iter().collect();
    }
    Vec::new()
}

fn json_string<T: Serialize>(value: &T, fallback: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| fallback.to_string())
}

fn normalize_backfill_event(
    event: &Value,
    resource: &ResolvedEventResource,
    _cursor_name: &str,
) -> BackfillEventRow {
    let external_id = event
        .get("id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            format!(
                "{}:{}:{}:{}:{}",
                resource.kind,
                resource.name,
                event_field_string(event, &["txHash", "tx_hash"]).unwrap_or("unknown"),
                event_field_number(event, &["ledger", "ledgerSequence", "ledger_sequence"])
                    .unwrap_or(0),
                event_field_string(
                    event,
                    &["cursor", "pagingToken", "paging_token", "pagingTokenId"]
                )
                .unwrap_or("tail")
            )
        });
    let topic = json_string(&event_topic_value(event), "[]");
    let payload = json_string(&event_payload_value(event), "{}");
    BackfillEventRow {
        external_id,
        cursor: event_field_string(
            event,
            &[
                "cursor",
                "pagingToken",
                "paging_token",
                "pagingTokenId",
                "id",
            ],
        )
        .map(ToOwned::to_owned),
        contract_id: event_field_string(event, &["contractId", "contract_id"])
            .unwrap_or(&resource.contract_id)
            .to_string(),
        event_type: event_field_string(event, &["type", "eventType", "event_type"])
            .unwrap_or("contract")
            .to_string(),
        topic,
        payload,
        tx_hash: event_field_string(event, &["txHash", "tx_hash"]).map(ToOwned::to_owned),
        ledger: event_field_number(event, &["ledger", "ledgerSequence", "ledger_sequence"]),
        observed_at: event_field_string(event, &["ledgerClosedAt", "ledger_closed_at"])
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| Utc::now().to_rfc3339()),
    }
}

fn normalize_account_backfill_event(
    event: &Value,
    resource_name: &str,
    resolved_address: &str,
) -> BackfillEventRow {
    let kind = event_field_string(event, &["type"]).unwrap_or("payment");
    let external_id = event
        .get("id")
        .and_then(Value::as_str)
        .or_else(|| event_field_string(event, &["paging_token", "pagingToken"]))
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            format!(
                "account:{resource_name}:{kind}:{}",
                event_field_string(event, &["paging_token", "pagingToken"]).unwrap_or("tail")
            )
        });
    let payload = json_string(event, "{}");
    let topic = json_string(&json!(["account", "payment", kind]), "[]");

    BackfillEventRow {
        external_id,
        cursor: event_field_string(event, &["paging_token", "pagingToken", "id"])
            .map(ToOwned::to_owned),
        contract_id: resolved_address.to_string(),
        event_type: kind.to_string(),
        topic,
        payload,
        tx_hash: event_field_string(event, &["transaction_hash", "transactionHash"])
            .map(ToOwned::to_owned),
        ledger: event_field_number(event, &["ledger", "ledger_attr", "ledger_sequence"]),
        observed_at: event_field_string(event, &["created_at", "ledger_closed_at"])
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| Utc::now().to_rfc3339()),
    }
}

fn account_payment_summary(event: &Value, account: &str) -> Value {
    let kind = event_field_string(event, &["type"]).unwrap_or("payment");
    let from = event_field_string(event, &["from", "source_account", "funder"]);
    let to = event_field_string(event, &["to", "account", "into"]);
    let amount = event_field_string(
        event,
        &[
            "amount",
            "starting_balance",
            "source_amount",
            "destination_amount",
        ],
    );
    json!({
        "id": event.get("id").and_then(Value::as_str),
        "cursor": event_field_string(event, &["paging_token", "pagingToken", "id"]),
        "type": kind,
        "account": account,
        "from": from,
        "to": to,
        "asset": horizon_payment_asset(event),
        "amount": amount,
        "tx_hash": event_field_string(event, &["transaction_hash", "transactionHash"]),
        "ledger": event_field_number(event, &["ledger", "ledger_attr", "ledger_sequence"]),
        "observed_at": event_field_string(event, &["created_at", "ledger_closed_at"]),
    })
}

fn horizon_payment_asset(event: &Value) -> String {
    match event_field_string(event, &["asset_type"]) {
        Some("native") | None
            if matches!(
                event_field_string(event, &["type"]),
                Some("create_account") | Some("account_created")
            ) =>
        {
            "XLM".to_string()
        }
        Some("native") => "XLM".to_string(),
        _ => {
            let code = event_field_string(event, &["asset_code"]);
            let issuer = event_field_string(event, &["asset_issuer"]);
            match (code, issuer) {
                (Some(code), Some(issuer)) => format!("{code}:{issuer}"),
                (Some(code), None) => code.to_string(),
                _ => "unknown".to_string(),
            }
        }
    }
}

fn event_topic_value(event: &Value) -> &Value {
    event
        .get("topic")
        .or_else(|| event.get("topics"))
        .unwrap_or(&Value::Null)
}

fn event_payload_value(event: &Value) -> &Value {
    event
        .get("value")
        .or_else(|| event.get("data"))
        .or_else(|| event.get("payload"))
        .or_else(|| event.get("body"))
        .unwrap_or(&Value::Null)
}

fn event_field_string<'a>(event: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| event.get(key).and_then(Value::as_str))
}

fn event_field_number(event: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter().find_map(|key| {
        event.get(key).and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_str().and_then(|raw| raw.parse::<i64>().ok()))
        })
    })
}

fn build_backfill_sql(events: &[BackfillEventRow], cursor: &EventCursorRow) -> String {
    let mut sql = String::from("begin;\n");
    for event in events {
        sql.push_str(&format!(
            "insert or ignore into events (external_id, cursor_name, cursor, resource_kind, resource_name, contract_id, event_type, topic, payload, tx_hash, ledger, observed_at) values ({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {});\n",
            sqlite_quote(&event.external_id),
            sqlite_quote(&cursor.name),
            sqlite_nullable_string(event.cursor.as_deref()),
            sqlite_quote(&cursor.resource_kind),
            sqlite_quote(&cursor.resource_name),
            sqlite_quote(&event.contract_id),
            sqlite_quote(&event.event_type),
            sqlite_quote(&event.topic),
            sqlite_quote(&event.payload),
            sqlite_nullable_string(event.tx_hash.as_deref()),
            sqlite_nullable_number(event.ledger),
            sqlite_quote(&event.observed_at),
        ));
    }
    sql.push_str(&format!(
        "insert into cursors (name, resource_kind, resource_name, cursor, last_ledger, updated_at) values ({}, {}, {}, {}, {}, {}) on conflict(name) do update set resource_kind = excluded.resource_kind, resource_name = excluded.resource_name, cursor = excluded.cursor, last_ledger = excluded.last_ledger, updated_at = excluded.updated_at;\ncommit;\n",
        sqlite_quote(&cursor.name),
        sqlite_quote(&cursor.resource_kind),
        sqlite_quote(&cursor.resource_name),
        sqlite_nullable_string(cursor.cursor.as_deref()),
        sqlite_nullable_number(cursor.last_ledger),
        sqlite_quote(&cursor.updated_at),
    ));
    sql
}

fn sqlite_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn sqlite_nullable_string(value: Option<&str>) -> String {
    value
        .map(sqlite_quote)
        .unwrap_or_else(|| "null".to_string())
}

fn sqlite_nullable_number(value: Option<i64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn scaffold_init_contracts(
    context: &AppContext,
    report: &mut CommandReport,
    root: &Path,
    manifest: &Manifest,
) -> Result<()> {
    if manifest.contracts.is_empty() {
        return Ok(());
    }

    if context.command_exists("stellar") && !context.globals.dry_run {
        let mut fell_back = false;
        for (name, contract) in &manifest.contracts {
            let args = vec![
                "contract".to_string(),
                "init".to_string(),
                path_to_string(root)?,
                "--name".to_string(),
                name.clone(),
            ];
            if let Err(error) = context.run_command(report, Some(root), "stellar", &args) {
                report.warnings.push(format!(
                    "official contract scaffold failed for `{name}` (template `{}`): {error}",
                    contract.template
                ));
                fell_back = true;
                break;
            }
            apply_contract_template_overrides(
                context,
                report,
                &root.join("contracts").join(name),
                name,
                &contract.template,
            )?;
            write_contract_toolchain(context, report, &root.join("contracts").join(name))?;
        }
        if !fell_back {
            return Ok(());
        }
    }

    for (name, contract) in &manifest.contracts {
        write_contract_stub(context, report, root, name, &contract.template)?;
    }
    Ok(())
}

fn write_contract_stub(
    context: &AppContext,
    report: &mut CommandReport,
    root: &Path,
    name: &str,
    template: &str,
) -> Result<()> {
    let contract_root = root.join("contracts").join(name);
    context.ensure_dir(report, &contract_root.join("src"))?;
    write_contract_toolchain(context, report, &contract_root)?;
    if apply_contract_template_overrides(context, report, &contract_root, name, template)? {
        context.write_text(
            report,
            &contract_root.join("Cargo.toml"),
            &format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n[lib]\ncrate-type = [\"lib\", \"cdylib\"]\ndoctest = false\n\n[dependencies]\nsoroban-sdk = \"25\"\n\n[dev-dependencies]\nsoroban-sdk = {{ version = \"25\", features = [\"testutils\"] }}\n"
            ),
        )?;
        return Ok(());
    }
    context.write_text(
        report,
        &contract_root.join("Cargo.toml"),
        &format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\ncrate-type = [\"cdylib\"]\n\n[dependencies]\n"
        ),
    )?;
    context.write_text(
        report,
        &contract_root.join("src/lib.rs"),
        &format!(
            "#![allow(dead_code)]\n\npub fn template_name() -> &'static str {{\n    \"{template}\"\n}}\n"
        ),
    )?;
    context.write_text(
        report,
        &contract_root.join("README.md"),
        &format!("# {name}\n\nScaffolded contract placeholder for template `{template}`.\n"),
    )?;
    Ok(())
}

fn apply_contract_template_overrides(
    context: &AppContext,
    report: &mut CommandReport,
    contract_root: &Path,
    name: &str,
    template: &str,
) -> Result<bool> {
    let Some(files) = templates::contract_template_files(template, name) else {
        return Ok(false);
    };
    context.ensure_dir(report, &contract_root.join("src"))?;
    context.write_text(report, &contract_root.join("src/lib.rs"), &files.lib_rs)?;
    if let Some(test_rs) = files.test_rs {
        context.write_text(report, &contract_root.join("src/test.rs"), &test_rs)?;
    }
    context.write_text(report, &contract_root.join("README.md"), &files.readme)?;
    report.warnings.push(format!(
        "contract `{name}` received template-specific source overrides for `{template}`"
    ));
    Ok(true)
}

fn write_contract_toolchain(
    context: &AppContext,
    report: &mut CommandReport,
    contract_root: &Path,
) -> Result<()> {
    context.write_text(
        report,
        &contract_root.join("rust-toolchain.toml"),
        templates::contract_rust_toolchain(),
    )
}

fn write_text_if_missing(
    context: &AppContext,
    report: &mut CommandReport,
    path: &Path,
    contents: &str,
) -> Result<()> {
    report.artifacts.push(path.display().to_string());
    if context.globals.dry_run {
        return Ok(());
    }
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create parent directory {}", parent.display()))?;
    }
    fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn smart_wallet_next_steps(
    package_manager: &str,
    onboarding_relative: &str,
    policy_contract: &str,
    env: &str,
) -> Vec<String> {
    vec![
        format!("stellar forge contract build {policy_contract}"),
        format!("stellar forge contract deploy {policy_contract} --env {env}"),
        package_manager_install_command(package_manager, onboarding_relative),
        package_manager_dev_command(package_manager, onboarding_relative),
    ]
}

fn deploy_contract_from_manifest(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: &Manifest,
    name: &str,
    env: &str,
) -> Result<()> {
    let contract = manifest
        .contracts
        .get(name)
        .ok_or_else(|| anyhow!("contract `{name}` not found"))?;
    let identity = manifest
        .active_identity(context.globals.identity.as_deref())
        .unwrap_or(&manifest.defaults.identity)
        .to_string();
    ensure_identity_exists(context, report, manifest, &identity, env, true)?;
    let contract_dir = context.project_root().join(&contract.path);
    let mut wasm_path = guess_wasm_path(&contract_dir, name);
    if !context.globals.dry_run && !wasm_path.exists() {
        context.run_command(
            report,
            Some(&contract_dir),
            "stellar",
            &["contract".to_string(), "build".to_string()],
        )?;
        wasm_path = guess_wasm_path(&contract_dir, name);
    }
    let args = vec![
        "contract".to_string(),
        "deploy".to_string(),
        "--wasm".to_string(),
        path_to_string(&wasm_path)?,
        "--source-account".to_string(),
        identity.clone(),
        "--network".to_string(),
        env.to_string(),
        "--alias".to_string(),
        contract.alias.clone(),
    ];
    let output = context.run_command(report, Some(&context.project_root()), "stellar", &args)?;
    let mut lockfile = load_lockfile(context)?;
    let environment = lockfile.environment_mut(env);
    environment.contracts.insert(
        name.to_string(),
        ContractDeployment {
            contract_id: if output.is_empty() {
                contract.alias.clone()
            } else {
                output.clone()
            },
            alias: contract.alias.clone(),
            wasm_hash: if wasm_path.exists() && !context.globals.dry_run {
                hex_digest(&fs::read(&wasm_path)?)
            } else {
                String::new()
            },
            tx_hash: String::new(),
            deployed_at: Some(Utc::now()),
        },
    );
    save_lockfile(context, report, &lockfile)?;
    if let Some(init) = token::contract_effective_init_config(manifest, name)? {
        let contract_id = resolve_contract_id(manifest, &lockfile, env, name);
        let mut call_args = vec![
            "contract".to_string(),
            "invoke".to_string(),
            "--id".to_string(),
            contract_id,
            "--source-account".to_string(),
            identity,
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
            call_args.push(format!("--{key}"));
            call_args.push(resolve_argument_value(
                context,
                report,
                Some(manifest),
                env,
                Some(&lockfile),
                value,
            )?);
        }
        context.run_command(report, Some(&context.project_root()), "stellar", &call_args)?;
    }
    Ok(())
}

fn ensure_identity_manifest_entries(manifest: &mut Manifest, name: &str) {
    if looks_like_account(name) {
        return;
    }
    manifest
        .identities
        .entry(name.to_string())
        .or_insert(IdentityConfig {
            source: "stellar-cli".to_string(),
            name: name.to_string(),
        });
    manifest
        .wallets
        .entry(name.to_string())
        .or_insert(WalletConfig {
            kind: "classic".to_string(),
            identity: name.to_string(),
            controller_identity: None,
            mode: None,
            onboarding_app: None,
            policy_contract: None,
        });
}

fn ensure_identity_exists(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: &Manifest,
    name: &str,
    env: &str,
    fund: bool,
) -> Result<bool> {
    if looks_like_account(name) || context.globals.dry_run {
        return Ok(false);
    }
    let network = manifest
        .networks
        .get(env)
        .ok_or_else(|| anyhow!("network `{env}` not defined"))?;
    let exists = context.run_command(
        &mut CommandReport::new("identity.resolve"),
        Some(&context.project_root()),
        "stellar",
        &[
            "keys".to_string(),
            "public-key".to_string(),
            name.to_string(),
        ],
    );
    if exists.is_ok() {
        if fund {
            if network.kind == "local" {
                let address = resolve_address(context, report, Some(manifest), name)?;
                fund_local_address(context, report, network, &address)?;
            } else if network.friendbot {
                let address = resolve_address(context, report, Some(manifest), name)?;
                let url = friendbot_url(env, network, &address)?;
                if !context.globals.dry_run {
                    let _ = context.get_json(&url);
                }
                report.commands.push(format!("GET {url}"));
            }
        }
        return Ok(false);
    }
    let mut args = vec![
        "keys".to_string(),
        "generate".to_string(),
        name.to_string(),
        "--network".to_string(),
        env.to_string(),
    ];
    if fund && network.kind != "local" {
        args.push("--fund".to_string());
    }
    context.run_command(report, Some(&context.project_root()), "stellar", &args)?;
    if fund && network.kind == "local" {
        let address = resolve_address(context, report, Some(manifest), name)?;
        fund_local_address(context, report, network, &address)?;
    }
    Ok(true)
}

fn wait_for_local_network(
    context: &AppContext,
    network: &crate::model::NetworkConfig,
) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let rpc_ready = Url::parse(&network.rpc_url)
            .ok()
            .and_then(|rpc_url| {
                context
                    .post_json(
                        &rpc_url,
                        &json!({"jsonrpc":"2.0","id":"health","method":"getHealth"}),
                    )
                    .ok()
            })
            .is_some();

        let horizon_ready = local_root_account_public_key(context)
            .ok()
            .and_then(|root| local_account_exists(context, network, &root).ok())
            .unwrap_or(false);

        if rpc_ready && horizon_ready {
            return Ok(());
        }

        if Instant::now() >= deadline {
            bail!("local network did not become ready within 30 seconds");
        }

        sleep(Duration::from_secs(1));
    }
}

fn fund_local_address(
    context: &AppContext,
    report: &mut CommandReport,
    network: &crate::model::NetworkConfig,
    address: &str,
) -> Result<()> {
    let root_secret = local_root_account_secret(context)?;
    let args = if local_account_exists(context, network, address)? {
        vec![
            "tx".to_string(),
            "new".to_string(),
            "payment".to_string(),
            "--source-account".to_string(),
            root_secret,
            "--destination".to_string(),
            address.to_string(),
            "--amount".to_string(),
            "100000000".to_string(),
            "--network".to_string(),
            "local".to_string(),
        ]
    } else {
        vec![
            "tx".to_string(),
            "new".to_string(),
            "create-account".to_string(),
            "--source-account".to_string(),
            root_secret,
            "--destination".to_string(),
            address.to_string(),
            "--starting-balance".to_string(),
            "100000000".to_string(),
            "--network".to_string(),
            "local".to_string(),
        ]
    };
    context.run_command(report, Some(&context.project_root()), "stellar", &args)?;
    Ok(())
}

fn local_account_exists(
    context: &AppContext,
    network: &crate::model::NetworkConfig,
    address: &str,
) -> Result<bool> {
    let mut url = Url::parse(&network.horizon_url)?;
    url.path_segments_mut()
        .map_err(|_| anyhow!("invalid horizon URL"))?
        .extend(["accounts", address]);
    match context.get_json(&url) {
        Ok(_) => Ok(true),
        Err(error) => {
            let message = error.to_string();
            if message.contains("404") || message.contains("Not Found") {
                Ok(false)
            } else {
                Err(error)
            }
        }
    }
}

fn local_root_account_public_key(context: &AppContext) -> Result<String> {
    context.run_command(
        &mut CommandReport::new("local.root-account.public"),
        Some(&context.project_root()),
        "stellar",
        &[
            "network".to_string(),
            "root-account".to_string(),
            "public-key".to_string(),
            "--network".to_string(),
            "local".to_string(),
        ],
    )
}

fn local_root_account_secret(context: &AppContext) -> Result<String> {
    context.run_command(
        &mut CommandReport::new("local.root-account.secret"),
        Some(&context.project_root()),
        "stellar",
        &[
            "network".to_string(),
            "root-account".to_string(),
            "secret".to_string(),
            "--network".to_string(),
            "local".to_string(),
        ],
    )
}

fn build_openapi(manifest: &Manifest) -> Value {
    let contract_paths = manifest
        .contracts
        .keys()
        .flat_map(|alias| {
            let mut paths = vec![
                (
                    format!("/contracts/{alias}/call/{{fn}}"),
                    json!({
                        "post": {
                            "summary": format!("Preview contract call for `{alias}`"),
                            "responses": {
                                "200": { "description": "Simulation payload or tx plan" }
                            }
                        }
                    }),
                ),
                (
                    format!("/contracts/{alias}/tx/{{fn}}"),
                    json!({
                        "post": {
                            "summary": format!("Build a transaction for `{alias}`"),
                            "responses": {
                                "200": { "description": "Build-only transaction details" }
                            }
                        }
                    }),
                ),
            ];
            if manifest.api.as_ref().is_some_and(|api| api.relayer) {
                paths.push((
                    format!("/contracts/{alias}/send/{{fn}}"),
                    json!({
                        "post": {
                            "summary": format!("Submit `{alias}` through the relayer proxy"),
                            "responses": {
                                "200": { "description": "Relayer submission result" }
                            }
                        }
                    }),
                ));
            }
            paths
        })
        .collect::<serde_json::Map<String, Value>>();
    let token_paths = manifest
        .tokens
        .keys()
        .flat_map(|alias| {
            vec![
                (
                    format!("/tokens/{alias}"),
                    json!({
                        "get": {
                            "summary": format!("Metadata for token `{alias}`"),
                            "responses": {
                                "200": { "description": "Token definition and runtime metadata" }
                            }
                        }
                    }),
                ),
                (
                    format!("/tokens/{alias}/balances/{{holder}}"),
                    json!({
                        "get": {
                            "summary": format!("Balance lookup for token `{alias}`"),
                            "responses": {
                                "200": { "description": "Balance lookup strategy and holder" }
                            }
                        }
                    }),
                ),
                (
                    format!("/tokens/{alias}/trust"),
                    json!({
                        "post": {
                            "summary": format!("Build trustline setup for `{alias}`"),
                            "responses": {
                                "200": { "description": "Trustline builder response" }
                            }
                        }
                    }),
                ),
                (
                    format!("/tokens/{alias}/payment"),
                    json!({
                        "post": {
                            "summary": format!("Build transfer plan for `{alias}`"),
                            "responses": {
                                "200": { "description": "Payment plan or built tx" }
                            }
                        }
                    }),
                ),
                (
                    format!("/tokens/{alias}/mint"),
                    json!({
                        "post": {
                            "summary": format!("Build mint flow for `{alias}`"),
                            "responses": {
                                "200": { "description": "Mint builder response" }
                            }
                        }
                    }),
                ),
            ]
        })
        .collect::<serde_json::Map<String, Value>>();
    let mut paths = contract_paths;
    paths.extend(token_paths);
    paths.extend(
        [
            (
                "/health".to_string(),
                json!({
                    "get": {
                        "summary": "Health probe",
                        "responses": {
                            "200": { "description": "Service health" }
                        }
                    }
                }),
            ),
            (
                "/ready".to_string(),
                json!({
                    "get": {
                        "summary": "Readiness probe",
                        "responses": {
                            "200": { "description": "Service readiness" }
                        }
                    }
                }),
            ),
            (
                "/version".to_string(),
                json!({
                    "get": {
                        "summary": "Version probe",
                        "responses": {
                            "200": { "description": "Project version" }
                        }
                    }
                }),
            ),
            (
                "/wallets".to_string(),
                json!({
                    "get": {
                        "summary": "List wallets declared in the manifest",
                        "responses": {
                            "200": { "description": "Wallet inventory" }
                        }
                    }
                }),
            ),
            (
                "/wallets/{name}".to_string(),
                json!({
                    "get": {
                        "summary": "Inspect a wallet declared in the manifest",
                        "responses": {
                            "200": { "description": "Wallet configuration entry" }
                        }
                    }
                }),
            ),
            (
                "/events/status".to_string(),
                json!({
                    "get": {
                        "summary": "Inspect event ingestion status and tracked resources",
                        "responses": {
                            "200": { "description": "Current backend, resource list, and cursor summary" }
                        }
                    }
                }),
            ),
            (
                "/events/cursors".to_string(),
                json!({
                    "get": {
                        "summary": "List persisted event cursors",
                        "responses": {
                            "200": { "description": "Named cursors persisted by the ingest worker" }
                        }
                    }
                }),
            ),
        ]
        .into_iter()
        .collect::<serde_json::Map<String, Value>>(),
    );
    if manifest.api.as_ref().is_some_and(|api| api.relayer) {
        paths.extend(
            [
                (
                    "/relayer/status".to_string(),
                    json!({
                        "get": {
                            "summary": "Inspect relayer proxy configuration",
                            "responses": {
                                "200": { "description": "Relayer readiness" }
                            }
                        }
                    }),
                ),
                (
                    "/relayer/submit".to_string(),
                    json!({
                        "post": {
                            "summary": "Submit a sponsored transaction through the configured relayer",
                            "responses": {
                                "200": { "description": "Proxy submission result" }
                            }
                        }
                    }),
                ),
            ]
            .into_iter()
            .collect::<serde_json::Map<String, Value>>(),
        );
    }
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": format!("{} API", manifest.project.name),
            "version": manifest.project.version,
        },
        "paths": paths,
    })
}

fn render_contract_routes(manifest: &Manifest) -> String {
    let mut lines = vec!["import type { FastifyInstance } from 'fastify';".to_string()];
    let contracts = manifest.contracts.keys().cloned().collect::<Vec<_>>();
    for (index, name) in contracts.iter().enumerate() {
        lines.push(format!(
            "import * as contractService{index} from '../services/contracts/{name}.js';"
        ));
    }
    lines.push(String::new());
    lines.push("export function registerContractRoutes(app: FastifyInstance) {".to_string());
    for (index, name) in contracts.iter().enumerate() {
        lines.push(format!(
            "  app.post('/contracts/{name}/call/:fn', async (request) => {{"
        ));
        lines.push("    const params = request.params as { fn: string };".to_string());
        lines.push(format!(
            "    return contractService{index}.preview(params.fn, request.body);"
        ));
        lines.push("  });".to_string());
        lines.push(String::new());
        lines.push(format!(
            "  app.post('/contracts/{name}/tx/:fn', async (request) => {{"
        ));
        lines.push("    const params = request.params as { fn: string };".to_string());
        lines.push(format!(
            "    return contractService{index}.buildTx(params.fn, request.body);"
        ));
        lines.push("  });".to_string());
        if manifest.api.as_ref().is_some_and(|api| api.relayer) {
            lines.push(String::new());
            lines.push(format!(
                "  app.post('/contracts/{name}/send/:fn', async (request) => {{"
            ));
            lines.push("    const params = request.params as { fn: string };".to_string());
            lines.push(format!(
                "    return contractService{index}.send(params.fn, request.body);"
            ));
            lines.push("  });".to_string());
        }
        lines.push(String::new());
    }
    lines.push("}".to_string());
    lines.join("\n")
}

fn render_token_routes(manifest: &Manifest) -> String {
    let mut lines = vec!["import type { FastifyInstance } from 'fastify';".to_string()];
    let tokens = manifest.tokens.keys().cloned().collect::<Vec<_>>();
    for (index, name) in tokens.iter().enumerate() {
        lines.push(format!(
            "import * as tokenService{index} from '../services/tokens/{name}.js';"
        ));
    }
    lines.push(String::new());
    lines.push("export function registerTokenRoutes(app: FastifyInstance) {".to_string());
    for (index, name) in tokens.iter().enumerate() {
        lines.push(format!(
            "  app.get('/tokens/{name}', async () => tokenService{index}.metadata());"
        ));
        lines.push(format!(
            "  app.get('/tokens/{name}/balances/:holder', async (request) => {{"
        ));
        lines.push("    const params = request.params as { holder: string };".to_string());
        lines.push(format!(
            "    return tokenService{index}.balance(params.holder);"
        ));
        lines.push("  });".to_string());
        lines.push(format!(
            "  app.post('/tokens/{name}/trust', async (request) => tokenService{index}.trust(request.body));"
        ));
        lines.push(format!(
            "  app.post('/tokens/{name}/payment', async (request) => tokenService{index}.payment(request.body));"
        ));
        lines.push(format!(
            "  app.post('/tokens/{name}/mint', async (request) => tokenService{index}.mint(request.body));"
        ));
        lines.push(String::new());
    }
    lines.push("}".to_string());
    lines.join("\n")
}

fn render_event_routes(manifest: &Manifest) -> String {
    let backend = manifest
        .api
        .as_ref()
        .map(|api| api.events_backend.as_str())
        .unwrap_or("rpc-poller");
    let database = manifest
        .api
        .as_ref()
        .map(|api| api.database.as_str())
        .unwrap_or("sqlite");
    format!(
        "import type {{ FastifyInstance }} from 'fastify';\nimport {{ getEventStatus, listEventCursors, resolveEventPaths, resolveEventWorkerConfig }} from '../lib/events-store.js';\nimport {{ manifest }} from '../lib/manifest.js';\n\nfunction trackedResources(filters: string[]) {{\n  const declared = [\n    ...Object.keys(manifest.contracts).map((name) => `contract:${{name}}`),\n    ...Object.keys(manifest.tokens).map((name) => `token:${{name}}`),\n  ];\n  if (filters.length === 0) {{\n    return declared;\n  }}\n  return declared.filter((resource) => {{\n    const [, name] = resource.split(':');\n    return filters.includes(resource) || (name ? filters.includes(name) : false);\n  }});\n}}\n\nexport function registerEventRoutes(app: FastifyInstance) {{\n  app.get('/events/status', async () => {{\n    const status = getEventStatus();\n    const worker = resolveEventWorkerConfig();\n    const activeBackend = manifest.api?.events_backend ?? '{backend}';\n    const retentionDays = worker.retention_days ?? (activeBackend === 'rpc-poller' ? 7 : null);\n    return {{\n      backend: activeBackend,\n      database: manifest.api?.database ?? '{database}',\n      db_path: resolveEventPaths().dbPath,\n      contracts: Object.keys(manifest.contracts),\n      tokens: Object.keys(manifest.tokens),\n      tracked_resources: trackedResources(worker.resources),\n      worker,\n      retention_days: retentionDays,\n      retention_warning: retentionDays === null\n        ? null\n        : `RPC/event retention is short; backfill older than ${{retentionDays}} day(s) requires your own archive or indexer.`,\n      ...status,\n      cursor_names: listEventCursors().map((cursor) => cursor.name),\n    }};\n  }});\n\n  app.get('/events/cursors', async () => ({{\n    db_path: resolveEventPaths().dbPath,\n    cursors: listEventCursors(),\n  }}));\n}}\n"
    )
}

fn resolve_contract_id(manifest: &Manifest, lockfile: &Lockfile, env: &str, name: &str) -> String {
    if is_contract_address(name) {
        return name.to_string();
    }
    lockfile
        .environments
        .get(env)
        .and_then(|environment| environment.contracts.get(name))
        .map(|deployment| deployment.contract_id.clone())
        .filter(|contract_id| !contract_id.is_empty())
        .or_else(|| {
            manifest
                .contracts
                .get(name)
                .map(|contract| contract.alias.clone())
        })
        .unwrap_or_else(|| name.to_string())
}

fn resolve_argument_value(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: Option<&Manifest>,
    env: &str,
    lockfile_hint: Option<&Lockfile>,
    value: &str,
) -> Result<String> {
    let owned_lockfile;
    let lockfile = if let Some(lockfile) = lockfile_hint {
        lockfile
    } else {
        owned_lockfile = load_lockfile(context)?;
        &owned_lockfile
    };
    match parse_manifest_ref(value) {
        Some(ManifestRef::Identity(identity)) => {
            resolve_address(context, report, manifest, &identity)
        }
        Some(ManifestRef::Wallet(wallet)) => resolve_address(context, report, manifest, &wallet),
        Some(ManifestRef::TokenSac(token)) => {
            if let Some(id) = lockfile
                .environments
                .get(env)
                .and_then(|environment| environment.tokens.get(&token))
                .map(|token| token.sac_contract_id.clone())
                .filter(|id| !id.is_empty())
            {
                return Ok(id);
            }
            if context.globals.dry_run
                && manifest
                    .and_then(|manifest| manifest.tokens.get(&token))
                    .is_some_and(|token| token.with_sac)
            {
                return Ok(format!("{token}-sac"));
            }
            Err(anyhow!(
                "token `{token}` does not have a deployed SAC in `{env}`"
            ))
        }
        Some(ManifestRef::Contract(contract)) => {
            let manifest = manifest.ok_or_else(|| {
                anyhow!("contract references require a loaded manifest in this context")
            })?;
            Ok(resolve_contract_id(manifest, lockfile, env, &contract))
        }
        _ => Ok(value.to_string()),
    }
}

fn resolve_address(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: Option<&Manifest>,
    value: &str,
) -> Result<String> {
    if looks_like_account(value) {
        return Ok(value.to_string());
    }
    let logical = resolve_identity_name(manifest, value).unwrap_or_else(|| value.to_string());
    if context.globals.dry_run || !context.command_exists("stellar") {
        return Ok(format!("<{logical}>"));
    }
    context.run_command(
        report,
        Some(&context.project_root()),
        "stellar",
        &["keys".to_string(), "public-key".to_string(), logical],
    )
}

fn asset_string(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: Option<&Manifest>,
    token: &TokenConfig,
) -> Result<String> {
    if token.code == "XLM" {
        return Ok("native".to_string());
    }
    let issuer = resolve_address(context, report, manifest, &token.issuer)?;
    Ok(format!("{}:{}", token.code, issuer))
}

fn resolve_identity_name(manifest: Option<&Manifest>, input: &str) -> Option<String> {
    if let Some(reference) = parse_manifest_ref(input) {
        match reference {
            ManifestRef::Identity(identity) => return Some(identity),
            ManifestRef::Wallet(wallet) => {
                let manifest = manifest?;
                return manifest
                    .wallets
                    .get(&wallet)
                    .and_then(wallet_runtime_identity);
            }
            _ => {}
        }
    }
    if let Some(manifest) = manifest {
        if manifest.identities.contains_key(input) {
            return Some(input.to_string());
        }
        if let Some(wallet) = manifest.wallets.get(input)
            && let Some(identity) = wallet_runtime_identity(wallet)
        {
            return Some(identity);
        }
    }
    None
}

fn friendbot_url(
    env_name: &str,
    network: &crate::model::NetworkConfig,
    address: &str,
) -> Result<Url> {
    let base = match network.kind.as_str() {
        "local" => format!("{}/friendbot", network.horizon_url.trim_end_matches('/')),
        "testnet" => "https://friendbot.stellar.org".to_string(),
        "futurenet" => "https://friendbot-futurenet.stellar.org".to_string(),
        _ => bail!("friendbot is not available for `{env_name}`"),
    };
    let mut url = Url::parse(&base)?;
    url.query_pairs_mut().append_pair("addr", address);
    Ok(url)
}

fn guess_wasm_path(contract_dir: &Path, name: &str) -> PathBuf {
    let underscored = name.replace('-', "_");
    let fallback = contract_dir
        .join("target")
        .join("wasm32v1-none")
        .join("release")
        .join(format!("{underscored}.wasm"));
    for ancestor in contract_dir.ancestors() {
        let candidate = ancestor
            .join("target")
            .join("wasm32v1-none")
            .join("release")
            .join(format!("{underscored}.wasm"));
        if candidate.exists() {
            return candidate;
        }
    }
    WalkDir::new(contract_dir)
        .into_iter()
        .filter_map(Result::ok)
        .find(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "wasm")
        })
        .map(|entry| entry.path().to_path_buf())
        .unwrap_or(fallback)
}

fn aggregate_status(checks: &[crate::runtime::CheckResult]) -> String {
    if checks.iter().any(|check| check.status == "error") {
        "error".to_string()
    } else if checks.iter().any(|check| check.status == "warn") {
        "warn".to_string()
    } else {
        "ok".to_string()
    }
}

fn release_resources(manifest: &Manifest, env: &str) -> (Vec<String>, Vec<String>, bool) {
    release::release_resources(manifest, env)
}

fn amount_to_stroops(input: &str, decimals: u32) -> Result<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        bail!("amount cannot be empty");
    }
    let negative = trimmed.starts_with('-');
    if negative {
        bail!("negative amounts are not supported");
    }
    let parts: Vec<&str> = trimmed.split('.').collect();
    if parts.len() > 2 {
        bail!("invalid decimal amount `{input}`");
    }
    let whole = parts[0];
    let fraction = parts.get(1).copied().unwrap_or("");
    if fraction.len() > decimals as usize {
        bail!("amount `{input}` has more than {decimals} decimal places");
    }
    let mut normalized = format!("{whole}{fraction}");
    normalized.push_str(&"0".repeat(decimals as usize - fraction.len()));
    let normalized = normalized.trim_start_matches('0');
    Ok(if normalized.is_empty() {
        "0".to_string()
    } else {
        normalized.to_string()
    })
}

fn is_horizon_account_id(value: &str) -> bool {
    matches!(value.chars().next(), Some('G' | 'M'))
}

fn looks_like_account(value: &str) -> bool {
    matches!(value.chars().next(), Some('G' | 'M' | 'C'))
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1" | "0.0.0.0")
}

fn is_contract_address(value: &str) -> bool {
    value.starts_with('C')
}

fn shouty(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            'a'..='z' | '0'..='9' => character.to_ascii_uppercase(),
            'A'..='Z' => character,
            _ => '_',
        })
        .collect()
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}
