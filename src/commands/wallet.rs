use super::*;

pub(super) fn wallet_command(
    context: &AppContext,
    command: WalletCommand,
) -> Result<CommandReport> {
    match command {
        WalletCommand::Create { name, fund } => wallet_create(context, &name, fund),
        WalletCommand::Ls => wallet_ls(context),
        WalletCommand::Address { name } => wallet_address(context, &name),
        WalletCommand::Fund { name_or_address } => wallet_fund(context, &name_or_address),
        WalletCommand::Balances { name_or_address } => wallet_balances(context, &name_or_address),
        WalletCommand::Trust { wallet, token } => wallet_trust(context, &wallet, &token),
        WalletCommand::Pay(args) => wallet_pay(context, &args),
        WalletCommand::Receive {
            wallet,
            sep7,
            qr,
            asset,
        } => wallet_receive(context, &wallet, sep7, qr, asset.as_deref()),
        WalletCommand::Sep7(args) => match args.command {
            WalletSep7Command::Payment(args) => wallet_pay_sep7(context, &args),
            WalletSep7Command::ContractCall(args) => wallet_sep7_contract_call(context, &args),
        },
        WalletCommand::Smart(args) => match args.command {
            WalletSmartCommand::Create { name, mode } => wallet_smart_create(context, &name, mode),
            WalletSmartCommand::Scaffold { name } => wallet_smart_scaffold(context, &name),
            WalletSmartCommand::Info { name } => wallet_smart_info(context, &name),
        },
    }
}

pub(super) fn wallet_trust(
    context: &AppContext,
    wallet: &str,
    token_name: &str,
) -> Result<CommandReport> {
    let mut report = CommandReport::new("wallet.trust");
    let manifest = load_manifest(context)?;
    ensure_named_wallets_are_materialized(&manifest, &[wallet])?;
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    let token = manifest
        .tokens
        .get(token_name)
        .ok_or_else(|| anyhow!("token `{token_name}` not found"))?;
    if token.kind == "contract" {
        bail!(
            "token `{token_name}` is a contract token and does not use classic trustlines; use `wallet pay` or a contract call instead"
        );
    }
    let wallet_identity =
        resolve_identity_name(Some(&manifest), wallet).unwrap_or_else(|| wallet.to_string());
    let asset = asset_string(context, &mut report, Some(&manifest), token)?;
    let args = vec![
        "tx".to_string(),
        "new".to_string(),
        "change-trust".to_string(),
        "--source-account".to_string(),
        wallet_identity.clone(),
        "--line".to_string(),
        asset.clone(),
        "--network".to_string(),
        env.clone(),
    ];
    context.run_command(&mut report, Some(&context.project_root()), "stellar", &args)?;
    report.network = Some(env);
    report.message = Some(format!(
        "trustline created for `{wallet}` -> `{token_name}`"
    ));
    report.data = Some(json!({
        "wallet": wallet,
        "identity": wallet_identity,
        "token": token_name,
        "asset": asset,
        "primitive": "change_trust",
    }));
    report.next = vec![format!("stellar forge wallet balances {wallet}")];
    Ok(report)
}

#[derive(Debug, Clone, Copy)]
struct SmartWalletScaffold<'a> {
    root: &'a Path,
    name: &'a str,
    mode: &'a str,
    onboarding_root: &'a Path,
    policy_contract: &'a str,
    policy_root: &'a Path,
    controller_identity: Option<&'a str>,
}

#[derive(Debug, Clone)]
struct PaymentPlan {
    primitive: String,
    reason: String,
    sep7_asset: Option<String>,
    args: Vec<String>,
}

#[derive(Debug, Clone)]
struct WalletAssetResolution {
    display: String,
    sep7_asset: Option<String>,
}

fn wallet_create(context: &AppContext, name: &str, fund: bool) -> Result<CommandReport> {
    let mut report = CommandReport::new("wallet.create");
    validate_single_path_segment("wallet name", name)?;
    let mut args = vec!["keys".to_string(), "generate".to_string(), name.to_string()];
    let mut manifest_synced = false;
    let mut network_name = None;
    if let Ok(manifest) = load_manifest(context) {
        if manifest
            .wallets
            .get(name)
            .is_some_and(|wallet| wallet.kind == "smart")
        {
            bail!(
                "wallet `{name}` already exists as a smart wallet; use `stellar forge wallet smart info {name}` or choose another classic wallet name"
            );
        }
        let env = manifest
            .active_network(context.globals.network.as_deref())?
            .0
            .to_string();
        args.push("--network".to_string());
        args.push(env.clone());
        if fund {
            args.push("--fund".to_string());
        }
        report.network = Some(env.clone());
        network_name = Some(env.clone());
        let mut updated_manifest = manifest;
        ensure_identity_manifest_entries(&mut updated_manifest, name);
        save_manifest(context, &mut report, &updated_manifest)?;
        manifest_synced = true;
    } else if fund {
        args.push("--fund".to_string());
    }
    context.run_command(&mut report, Some(&context.project_root()), "stellar", &args)?;
    report.message = Some(format!("wallet `{name}` created"));
    report.data = Some(json!({
        "wallet": name,
        "identity": name,
        "funded": fund,
        "manifest_synced": manifest_synced,
        "network": network_name,
    }));
    report.next = if fund {
        vec![
            format!("stellar forge wallet address {name}"),
            format!("stellar forge wallet balances {name}"),
        ]
    } else {
        vec![
            format!("stellar forge wallet fund {name}"),
            format!("stellar forge wallet address {name}"),
        ]
    };
    Ok(report)
}

fn wallet_ls(context: &AppContext) -> Result<CommandReport> {
    let mut report = CommandReport::new("wallet.ls");
    let manifest = load_manifest(context).ok();
    let output = context.run_command(
        &mut report,
        Some(&context.project_root()),
        "stellar",
        &["keys".to_string(), "ls".to_string(), "-l".to_string()],
    )?;
    report.message = Some("listed Stellar identities".to_string());
    report.data = Some(json!({
        "stellar_cli_output": output,
        "identities": output
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>(),
        "declared_wallets": manifest
            .as_ref()
            .map(|manifest| {
                manifest
                    .wallets
                    .iter()
                    .map(|(name, wallet)| {
                        json!({
                            "name": name,
                            "kind": wallet.kind,
                            "identity": wallet_runtime_identity(wallet),
                            "controller_identity": wallet_controller_identity_value(wallet),
                            "mode": wallet.mode.clone(),
                            "onboarding_app": wallet.onboarding_app.clone(),
                            "policy_contract": wallet.policy_contract.clone(),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
    }));
    Ok(report)
}

fn smart_wallet_name_for_input(manifest: &Manifest, input: &str) -> Option<String> {
    let wallet_name = match parse_manifest_ref(input) {
        Some(ManifestRef::Wallet(wallet)) => wallet,
        _ if manifest.wallets.contains_key(input) => input.to_string(),
        _ => return None,
    };
    manifest
        .wallets
        .get(&wallet_name)
        .filter(|wallet| wallet.kind == "smart")
        .map(|_| wallet_name)
}

fn ensure_not_unmaterialized_smart_wallet(manifest: &Manifest, input: &str) -> Result<()> {
    if let Some(name) = smart_wallet_name_for_input(manifest, input) {
        bail!(
            "smart wallet `{name}` does not resolve to a classic account yet; use `stellar forge wallet smart info {name}` for onboarding details"
        );
    }
    Ok(())
}

pub(super) fn ensure_named_wallets_are_materialized(
    manifest: &Manifest,
    inputs: &[&str],
) -> Result<()> {
    for input in inputs {
        if !input.trim().is_empty() {
            ensure_not_unmaterialized_smart_wallet(manifest, input)?;
        }
    }
    Ok(())
}

fn wallet_address(context: &AppContext, name: &str) -> Result<CommandReport> {
    let mut report = CommandReport::new("wallet.address");
    let manifest = load_manifest(context).ok();
    if let Some(manifest) = manifest.as_ref() {
        ensure_not_unmaterialized_smart_wallet(manifest, name)?;
    }
    let resolved_identity = manifest
        .as_ref()
        .and_then(|manifest| resolve_identity_name(Some(manifest), name));
    let address = resolve_address(context, &mut report, manifest.as_ref(), name)?;
    report.message = Some(format!("address for `{name}` resolved"));
    report.data = Some(json!({
        "input": name,
        "identity": resolved_identity,
        "address": address,
        "wallet_kind": manifest
            .as_ref()
            .and_then(|manifest| manifest.wallets.get(name))
            .map(|wallet| wallet.kind.clone()),
    }));
    Ok(report)
}

fn wallet_fund(context: &AppContext, target: &str) -> Result<CommandReport> {
    if let Ok(manifest) = load_manifest(context) {
        ensure_not_unmaterialized_smart_wallet(&manifest, target)?;
    }
    let mut report = dev_fund(context, target)?;
    report.action = "wallet.fund".to_string();
    if let Some(data) = report.data.as_mut().and_then(Value::as_object_mut) {
        data.insert("target".to_string(), json!(target));
    }
    report.next = vec![format!("stellar forge wallet balances {target}")];
    Ok(report)
}

pub(super) fn wallet_balances(context: &AppContext, target: &str) -> Result<CommandReport> {
    let mut report = CommandReport::new("wallet.balances");
    let manifest = load_manifest(context)?;
    ensure_not_unmaterialized_smart_wallet(&manifest, target)?;
    let lockfile = load_lockfile(context)?;
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    let network = manifest
        .networks
        .get(&env)
        .ok_or_else(|| anyhow!("network `{env}` missing"))?;
    let address = resolve_address(context, &mut report, Some(&manifest), target)?;
    let mut url = Url::parse(&network.horizon_url)?;
    url.path_segments_mut()
        .map_err(|_| anyhow!("invalid horizon URL"))?
        .extend(["accounts", address.as_str()]);
    report.commands.push(format!("GET {url}"));
    let response = if context.globals.dry_run {
        json!({ "balances": [] })
    } else {
        context.get_json(&url)?
    };
    let balances = response
        .get("balances")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let project_tokens = collect_project_token_balances(
        context,
        &mut report,
        &manifest,
        &lockfile,
        &env,
        &address,
        balances.as_array().map(Vec::as_slice).unwrap_or(&[]),
    )?;
    report.network = Some(env);
    report.message = Some(format!("balances fetched for `{target}`"));
    report.data = Some(json!({
        "address": address,
        "balances": balances,
        "project_tokens": project_tokens,
    }));
    Ok(report)
}

fn collect_project_token_balances(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: &Manifest,
    lockfile: &Lockfile,
    env: &str,
    address: &str,
    balances: &[Value],
) -> Result<Vec<Value>> {
    let source = manifest
        .active_identity(context.globals.identity.as_deref())
        .unwrap_or(&manifest.defaults.identity)
        .to_string();
    manifest
        .tokens
        .iter()
        .map(|(name, token)| {
            let deployment = lockfile
                .environments
                .get(env)
                .and_then(|environment| environment.tokens.get(name));
            let classic_asset = if token.kind == "contract" {
                None
            } else {
                Some(asset_string(context, report, Some(manifest), token)?)
            };
            let classic_balance = classic_asset
                .as_ref()
                .and_then(|asset| find_classic_balance_for_asset(balances, asset));
            let sac_contract_id = deployment
                .map(|deployment| deployment.sac_contract_id.clone())
                .filter(|value| !value.is_empty())
                .or_else(|| {
                    if context.globals.dry_run && token.with_sac {
                        Some(format!("{name}-sac"))
                    } else {
                        None
                    }
                });
            let contract_id = deployment
                .map(|deployment| deployment.contract_id.clone())
                .filter(|value| !value.is_empty())
                .or_else(|| {
                    if context.globals.dry_run && token.kind == "contract" {
                        Some(name.to_string())
                    } else {
                        None
                    }
                });
            let sac_balance = if let Some(contract_id) = sac_contract_id.as_deref() {
                query_standard_token_balance(
                    context,
                    report,
                    env,
                    contract_id,
                    address,
                    &source,
                    &format!("{name} SAC"),
                )?
            } else {
                None
            };
            let contract_balance = if let Some(contract_id) = contract_id.as_deref() {
                query_standard_token_balance(
                    context,
                    report,
                    env,
                    contract_id,
                    address,
                    &source,
                    &format!("{name} contract token"),
                )?
            } else {
                None
            };
            Ok(json!({
                "name": name,
                "kind": token.kind,
                "code": if token.code.is_empty() {
                    "XLM".to_string()
                } else {
                    token.code.clone()
                },
                "classic_asset": classic_asset,
                "classic_balance": classic_balance,
                "sac_contract_id": sac_contract_id,
                "sac_balance": sac_balance,
                "contract_id": contract_id,
                "contract_balance": contract_balance,
            }))
        })
        .collect()
}

fn find_classic_balance_for_asset(balances: &[Value], asset: &str) -> Option<Value> {
    if matches!(asset, "XLM" | "native") {
        return balances
            .iter()
            .find(|entry| entry.get("asset_type").and_then(Value::as_str) == Some("native"))
            .cloned();
    }
    let (code, issuer) = asset.split_once(':')?;
    balances
        .iter()
        .find(|entry| {
            entry.get("asset_code").and_then(Value::as_str) == Some(code)
                && entry.get("asset_issuer").and_then(Value::as_str) == Some(issuer)
        })
        .cloned()
}

pub(super) fn wallet_pay(context: &AppContext, args: &WalletPayArgs) -> Result<CommandReport> {
    let mut report = CommandReport::new("wallet.pay");
    let manifest = load_manifest(context)?;
    let lockfile = load_lockfile(context)?;
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    let plan = payment_plan(context, &mut report, &manifest, &lockfile, &env, args)?;
    report.network = Some(env.clone());
    if args.sep7 {
        return finalize_sep7_payment(report, &plan, args);
    }
    if args.relayer {
        return wallet_pay_relayer(context, report, &manifest, &env, &plan, args);
    }
    let mut command_args = plan.args.clone();
    if args.build_only && !command_args.contains(&"--build-only".to_string()) {
        command_args.push("--build-only".to_string());
    }
    let output = context.run_command(
        &mut report,
        Some(&context.project_root()),
        "stellar",
        &command_args,
    )?;
    report.message = Some(plan.reason);
    report.data = if output.is_empty() {
        Some(json!({ "primitive": plan.primitive, "plan_only": true }))
    } else {
        Some(json!({ "primitive": plan.primitive, "result": output }))
    };
    Ok(report)
}

fn wallet_pay_sep7(context: &AppContext, args: &WalletPayArgs) -> Result<CommandReport> {
    wallet_pay(
        context,
        &WalletPayArgs {
            sep7: true,
            ..args.clone()
        },
    )
}

fn wallet_pay_relayer(
    context: &AppContext,
    mut report: CommandReport,
    manifest: &Manifest,
    env: &str,
    plan: &PaymentPlan,
    args: &WalletPayArgs,
) -> Result<CommandReport> {
    let mut build_args = plan.args.clone();
    if !build_args.iter().any(|argument| argument == "--build-only") {
        build_args.push("--build-only".to_string());
    }
    let xdr = context.run_command(
        &mut report,
        Some(&context.project_root()),
        "stellar",
        &build_args,
    )?;
    let relay_endpoint = resolve_relayer_submit_url(context, manifest)?;
    report.commands.push(format!("POST {relay_endpoint}"));

    let xdr_value = if xdr.is_empty() {
        Value::Null
    } else {
        Value::String(xdr.clone())
    };
    let request_body = json!({
        "network": env,
        "primitive": plan.primitive,
        "xdr": xdr_value,
        "payment": {
            "from": args.from,
            "to": args.to,
            "asset": args.asset,
            "amount": args.amount,
        },
    });

    if context.globals.dry_run {
        report.message =
            Some("planned a relayed payment submission through the local API proxy".to_string());
        report.data = Some(json!({
            "primitive": plan.primitive,
            "relay_endpoint": relay_endpoint.as_str(),
            "request": request_body,
            "plan_only": true,
        }));
        return Ok(report);
    }

    if args.build_only {
        report.message = Some("built a relayer-ready transaction XDR".to_string());
        report.data = Some(json!({
            "primitive": plan.primitive,
            "relay_endpoint": relay_endpoint.as_str(),
            "xdr": xdr,
            "request": request_body,
            "build_only": true,
        }));
        return Ok(report);
    }

    let result = context.post_json(&relay_endpoint, &request_body)?;
    report.message = Some("submitted payment to the configured relayer proxy".to_string());
    report.data = Some(json!({
        "primitive": plan.primitive,
        "relay_endpoint": relay_endpoint.as_str(),
        "request": request_body,
        "result": result,
    }));
    Ok(report)
}

fn wallet_receive(
    context: &AppContext,
    wallet: &str,
    sep7: bool,
    qr: bool,
    asset: Option<&str>,
) -> Result<CommandReport> {
    let mut report = CommandReport::new("wallet.receive");
    let manifest = load_manifest(context).ok();
    if let Some(manifest) = manifest.as_ref() {
        ensure_not_unmaterialized_smart_wallet(manifest, wallet)?;
    }
    let lockfile = load_lockfile(context).ok();
    let address = resolve_address(context, &mut report, manifest.as_ref(), wallet)?;
    let resolved_asset = asset
        .map(|asset| {
            resolve_wallet_asset(
                context,
                &mut report,
                manifest.as_ref(),
                lockfile.as_ref(),
                asset,
            )
        })
        .transpose()?;
    let uri = if sep7 || qr {
        if let Some(resolution) = resolved_asset.as_ref() {
            if let Some(classic_asset) = resolution.sep7_asset.as_ref() {
                Some(build_pay_uri(&address, classic_asset, None))
            } else {
                report.warnings.push(format!(
                    "asset `{}` is not representable as a SEP-7 payment URI; sharing the raw address instead",
                    resolution.display
                ));
                None
            }
        } else {
            Some(build_pay_uri(&address, "XLM", None))
        }
    } else {
        None
    };
    let qr_payload = if qr {
        uri.clone().or_else(|| Some(address.clone()))
    } else {
        None
    };
    report.message = Some(format!("receive details for `{wallet}`"));
    report.data = Some(json!({
        "address": address,
        "recommended_asset": resolved_asset
            .as_ref()
            .map(|resolution| resolution.display.clone())
            .unwrap_or_else(|| "XLM".to_string()),
        "sep7_uri": if sep7 { uri.clone() } else { None },
        "qr_payload": qr_payload,
        "qr_hint": if qr {
            Some("render `qr_payload` as the QR contents")
        } else {
            None::<&str>
        },
    }));
    Ok(report)
}

fn wallet_sep7_contract_call(
    context: &AppContext,
    args: &ContractCallArgs,
) -> Result<CommandReport> {
    let mut report = contract_call(
        context,
        &ContractCallArgs {
            build_only: true,
            send: "no".to_string(),
            ..args.clone()
        },
    )?;
    report.action = "wallet.sep7.contract-call".to_string();
    report.message =
        Some("generated a build-only contract invocation for wallet handoff".to_string());
    if let Some(data) = &report.data
        && let Some(result) = data.get("result").and_then(Value::as_str)
    {
        report.data = Some(json!({
            "sep7_uri": format!("web+stellar:tx?xdr={}", urlencoding(result)),
            "xdr": result,
        }));
    }
    Ok(report)
}

fn smart_wallet_mode_name(mode: SmartWalletMode) -> &'static str {
    match mode {
        SmartWalletMode::Ed25519 => "ed25519",
        SmartWalletMode::Passkey => "passkey",
    }
}

fn smart_wallet_controller_identity(
    name: &str,
    mode: &str,
    existing: Option<&WalletConfig>,
) -> Option<String> {
    if mode != "ed25519" {
        return None;
    }
    existing
        .and_then(wallet_controller_identity_value)
        .or_else(|| Some(format!("{name}-owner")))
}

fn smart_wallet_paths(root: &Path, name: &str) -> (String, PathBuf, String, String, PathBuf) {
    let onboarding_relative = format!("apps/smart-wallet/{name}");
    let onboarding_root = root.join(&onboarding_relative);
    let policy_contract = format!("{name}-policy");
    let policy_relative = format!("contracts/{policy_contract}");
    let policy_root = root.join(&policy_relative);
    (
        onboarding_relative,
        onboarding_root,
        policy_contract,
        policy_relative,
        policy_root,
    )
}

fn ensure_smart_wallet_controller_entries(
    manifest: &mut Manifest,
    controller_identity: &str,
) -> Result<()> {
    if let Some(existing) = manifest.wallets.get(controller_identity)
        && existing.kind != "classic"
    {
        bail!("controller identity `{controller_identity}` already exists as a smart wallet");
    }
    ensure_identity_manifest_entries(manifest, controller_identity);
    Ok(())
}

fn upsert_smart_wallet_manifest_entries(
    manifest: &mut Manifest,
    name: &str,
    mode: &str,
    onboarding_relative: &str,
    policy_contract: &str,
    policy_relative: &str,
    controller_identity: Option<&str>,
) {
    manifest.wallets.insert(
        name.to_string(),
        WalletConfig {
            kind: "smart".to_string(),
            identity: String::new(),
            controller_identity: controller_identity.map(str::to_string),
            mode: Some(mode.to_string()),
            onboarding_app: Some(onboarding_relative.to_string()),
            policy_contract: Some(policy_contract.to_string()),
        },
    );
    manifest
        .contracts
        .entry(policy_contract.to_string())
        .or_insert(ContractConfig {
            path: policy_relative.to_string(),
            alias: policy_contract.to_string(),
            template: "passkey-wallet-policy".to_string(),
            bindings: vec!["typescript".to_string()],
            deploy_on: vec!["local".to_string(), "testnet".to_string()],
            init: None,
        });
}

fn write_smart_wallet_scaffold_files(
    context: &AppContext,
    report: &mut CommandReport,
    scaffold: &SmartWalletScaffold<'_>,
) -> Result<()> {
    if !scaffold.policy_root.exists() || context.globals.dry_run {
        write_contract_stub(
            context,
            report,
            scaffold.root,
            scaffold.policy_contract,
            "passkey-wallet-policy",
        )?;
    }

    context.ensure_dir(report, &scaffold.onboarding_root.join("src"))?;
    write_text_if_missing(
        context,
        report,
        &scaffold.onboarding_root.join("README.md"),
        &templates::smart_wallet_readme(
            scaffold.name,
            scaffold.mode,
            scaffold.policy_contract,
            scaffold.controller_identity,
        ),
    )?;
    write_text_if_missing(
        context,
        report,
        &scaffold.onboarding_root.join(".env.example"),
        &templates::smart_wallet_env_example(
            scaffold.name,
            scaffold.mode,
            scaffold.policy_contract,
            scaffold.controller_identity,
        ),
    )?;
    write_text_if_missing(
        context,
        report,
        &scaffold.onboarding_root.join("package.json"),
        &templates::smart_wallet_package_json(scaffold.name),
    )?;
    write_text_if_missing(
        context,
        report,
        &scaffold.onboarding_root.join("tsconfig.json"),
        templates::smart_wallet_tsconfig(),
    )?;
    write_text_if_missing(
        context,
        report,
        &scaffold.onboarding_root.join("index.html"),
        templates::smart_wallet_index_html(),
    )?;
    write_text_if_missing(
        context,
        report,
        &scaffold.onboarding_root.join("src/main.ts"),
        &templates::smart_wallet_main_ts(
            scaffold.name,
            scaffold.mode,
            scaffold.policy_contract,
            scaffold.controller_identity,
        ),
    )?;
    Ok(())
}

fn smart_wallet_create_next_steps(
    package_manager: &str,
    onboarding_relative: &str,
    policy_contract: &str,
    env: &str,
    controller_identity: Option<&str>,
) -> Vec<String> {
    let mut next = Vec::new();
    if let Some(controller_identity) = controller_identity {
        next.push(format!("stellar forge wallet fund {controller_identity}"));
        next.push(format!(
            "stellar forge wallet address {controller_identity}"
        ));
    }
    next.extend(smart_wallet_next_steps(
        package_manager,
        onboarding_relative,
        policy_contract,
        env,
    ));
    next
}

fn wallet_smart_create(
    context: &AppContext,
    name: &str,
    mode: SmartWalletMode,
) -> Result<CommandReport> {
    let mut report = CommandReport::new("wallet.smart.create");
    validate_single_path_segment("smart wallet name", name)?;
    let root = context.project_root();
    let mut manifest = load_manifest(context)?;
    if let Some(existing) = manifest.wallets.get(name)
        && existing.kind != "smart"
    {
        bail!("wallet `{name}` already exists as a classic wallet");
    }

    let mode_name = smart_wallet_mode_name(mode);
    let existing_wallet = manifest.wallets.get(name).cloned();
    let controller_identity =
        smart_wallet_controller_identity(name, mode_name, existing_wallet.as_ref());
    let (onboarding_relative, onboarding_root, policy_contract, policy_relative, policy_root) =
        smart_wallet_paths(&root, name);

    if let Some(controller_identity) = controller_identity.as_deref() {
        ensure_smart_wallet_controller_entries(&mut manifest, controller_identity)?;
    }
    upsert_smart_wallet_manifest_entries(
        &mut manifest,
        name,
        mode_name,
        &onboarding_relative,
        &policy_contract,
        &policy_relative,
        controller_identity.as_deref(),
    );
    write_smart_wallet_scaffold_files(
        context,
        &mut report,
        &SmartWalletScaffold {
            root: &root,
            name,
            mode: mode_name,
            onboarding_root: &onboarding_root,
            policy_contract: &policy_contract,
            policy_root: &policy_root,
            controller_identity: controller_identity.as_deref(),
        },
    )?;
    save_manifest(context, &mut report, &manifest)?;

    if let Some(controller_identity) = controller_identity.as_deref() {
        ensure_identity_exists(
            context,
            &mut report,
            &manifest,
            controller_identity,
            &manifest.defaults.network,
            false,
        )?;
        report.warnings.push(format!(
            "controller identity `{controller_identity}` was prepared for wallet ownership; the smart wallet contract account still needs provisioning"
        ));
    } else {
        report.warnings.push(
            "passkey mode still needs a browser-based ceremony and contract-account provisioning flow"
                .to_string(),
        );
    }

    report.message = Some(format!(
        "smart wallet `{name}` prepared in `{mode_name}` mode"
    ));
    report.next = smart_wallet_create_next_steps(
        &manifest.project.package_manager,
        &onboarding_relative,
        &policy_contract,
        &manifest.defaults.network,
        controller_identity.as_deref(),
    );
    report.data = Some(json!({
        "wallet": name,
        "mode": mode_name,
        "controller_identity": controller_identity,
        "controller_generated": mode_name == "ed25519",
        "onboarding_app": onboarding_relative,
        "policy_contract": policy_contract,
        "paths": {
            "onboarding": onboarding_root.display().to_string(),
            "policy_contract": policy_root.display().to_string(),
        },
    }));
    Ok(report)
}

fn wallet_smart_scaffold(context: &AppContext, name: &str) -> Result<CommandReport> {
    let mut report = CommandReport::new("wallet.smart.scaffold");
    validate_single_path_segment("smart wallet name", name)?;
    let root = context.project_root();
    let mut manifest = load_manifest(context)?;
    if let Some(existing) = manifest.wallets.get(name)
        && existing.kind != "smart"
    {
        bail!("wallet `{name}` already exists as a classic wallet");
    }

    let (onboarding_relative, onboarding_root, policy_contract, policy_relative, policy_root) =
        smart_wallet_paths(&root, name);
    upsert_smart_wallet_manifest_entries(
        &mut manifest,
        name,
        "passkey",
        &onboarding_relative,
        &policy_contract,
        &policy_relative,
        None,
    );
    write_smart_wallet_scaffold_files(
        context,
        &mut report,
        &SmartWalletScaffold {
            root: &root,
            name,
            mode: "passkey",
            onboarding_root: &onboarding_root,
            policy_contract: &policy_contract,
            policy_root: &policy_root,
            controller_identity: None,
        },
    )?;
    save_manifest(context, &mut report, &manifest)?;

    let default_env = manifest.defaults.network.clone();
    report.message = Some(format!(
        "smart wallet onboarding scaffold created at {}",
        onboarding_root.display()
    ));
    report.next = smart_wallet_next_steps(
        &manifest.project.package_manager,
        &onboarding_relative,
        &policy_contract,
        &default_env,
    );
    report.data = Some(json!({
        "wallet": name,
        "mode": "passkey",
        "onboarding_app": onboarding_relative,
        "policy_contract": policy_contract,
        "paths": {
            "onboarding": onboarding_root.display().to_string(),
            "policy_contract": policy_root.display().to_string(),
        },
    }));
    Ok(report)
}

fn wallet_smart_info(context: &AppContext, name: &str) -> Result<CommandReport> {
    let mut report = CommandReport::new("wallet.smart.info");
    let root = context.project_root();
    let manifest = load_manifest(context)?;
    let wallet = manifest.wallets.get(name).cloned();
    let controller_identity = wallet.as_ref().and_then(wallet_controller_identity_value);
    let onboarding_relative = wallet
        .as_ref()
        .and_then(|wallet| wallet.onboarding_app.clone())
        .unwrap_or_else(|| format!("apps/smart-wallet/{name}"));
    let onboarding_root = root.join(&onboarding_relative);
    let policy_contract = wallet
        .as_ref()
        .and_then(|wallet| wallet.policy_contract.clone())
        .unwrap_or_else(|| format!("{name}-policy"));
    let policy_root = root.join("contracts").join(&policy_contract);
    report.message = Some(format!("smart wallet scaffold info for `{name}`"));
    report.next = smart_wallet_next_steps(
        &manifest.project.package_manager,
        &onboarding_relative,
        &policy_contract,
        &manifest.defaults.network,
    );
    report.data = Some(json!({
        "wallet": wallet,
        "controller_identity": controller_identity,
        "onboarding": {
            "path": onboarding_root.display().to_string(),
            "exists": onboarding_root.exists(),
        },
        "policy_contract": {
            "name": policy_contract,
            "path": policy_root.display().to_string(),
            "exists": policy_root.exists(),
        },
    }));
    Ok(report)
}

fn payment_plan(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: &Manifest,
    lockfile: &Lockfile,
    env: &str,
    args: &WalletPayArgs,
) -> Result<PaymentPlan> {
    ensure_named_wallets_are_materialized(manifest, &[args.from.as_str(), args.to.as_str()])?;
    let from_identity =
        resolve_identity_name(Some(manifest), &args.from).unwrap_or_else(|| args.from.clone());
    let from_address = resolve_address(context, report, Some(manifest), &args.from)?;
    let to_address = resolve_address(context, report, Some(manifest), &args.to)?;
    let from_is_contract = is_contract_address(&from_address);
    let to_is_contract = is_contract_address(&to_address);
    let token = manifest.tokens.get(&args.asset);
    if token.is_none() && !matches!(args.asset.as_str(), "XLM" | "native") {
        bail!(
            "asset or token `{}` is not declared in the manifest",
            args.asset
        );
    }

    if matches!(args.asset.as_str(), "XLM" | "native") {
        return Ok(PaymentPlan {
            primitive: "payment".to_string(),
            reason: "used classic payment because the transfer is native XLM".to_string(),
            sep7_asset: Some("XLM".to_string()),
            args: vec![
                "tx".to_string(),
                "new".to_string(),
                "payment".to_string(),
                "--source-account".to_string(),
                from_identity,
                "--destination".to_string(),
                to_address,
                "--amount".to_string(),
                amount_to_stroops(&args.amount, 7)?,
                "--network".to_string(),
                env.to_string(),
            ],
        });
    }

    let Some(token) = token else {
        bail!(
            "asset or token `{}` is not declared in the manifest",
            args.asset
        );
    };
    if token.kind == "contract" {
        let contract_id = lockfile
            .environments
            .get(env)
            .and_then(|environment| environment.tokens.get(&args.asset))
            .map(|token| token.contract_id.clone())
            .filter(|contract_id| !contract_id.is_empty())
            .unwrap_or_else(|| args.asset.clone());
        return Ok(PaymentPlan {
            primitive: "contract.transfer".to_string(),
            reason: "used contract transfer because the token is declared as a contract token"
                .to_string(),
            sep7_asset: None,
            args: vec![
                "contract".to_string(),
                "invoke".to_string(),
                "--id".to_string(),
                contract_id,
                "--source-account".to_string(),
                from_identity,
                "--network".to_string(),
                env.to_string(),
                "--send".to_string(),
                if args.build_only { "no" } else { "yes" }.to_string(),
                "--".to_string(),
                "transfer".to_string(),
                "--from".to_string(),
                from_address,
                "--to".to_string(),
                to_address,
                "--amount".to_string(),
                amount_to_stroops(&args.amount, token.decimals)?,
            ],
        });
    }

    if !from_is_contract && !to_is_contract {
        let asset = asset_string(context, report, Some(manifest), token)?;
        return Ok(PaymentPlan {
            primitive: "payment".to_string(),
            reason:
                "used classic payment because this is an asset transfer between classic accounts"
                    .to_string(),
            sep7_asset: Some(asset.clone()),
            args: vec![
                "tx".to_string(),
                "new".to_string(),
                "payment".to_string(),
                "--source-account".to_string(),
                from_identity,
                "--destination".to_string(),
                to_address,
                "--asset".to_string(),
                asset,
                "--amount".to_string(),
                amount_to_stroops(&args.amount, token.decimals)?,
                "--network".to_string(),
                env.to_string(),
            ],
        });
    }

    let sac_id = lockfile
        .environments
        .get(env)
        .and_then(|environment| environment.tokens.get(&args.asset))
        .map(|token| token.sac_contract_id.clone())
        .filter(|contract_id| !contract_id.is_empty())
        .ok_or_else(|| anyhow!("asset `{}` needs a SAC for contract-address transfers; run `stellar forge token sac deploy {}` first", args.asset, args.asset))?;
    Ok(PaymentPlan {
        primitive: "sac.transfer".to_string(),
        reason: "used SAC transfer because at least one side of the transfer is a contract address"
            .to_string(),
        sep7_asset: None,
        args: vec![
            "contract".to_string(),
            "invoke".to_string(),
            "--id".to_string(),
            sac_id,
            "--source-account".to_string(),
            from_identity,
            "--network".to_string(),
            env.to_string(),
            "--send".to_string(),
            if args.build_only { "no" } else { "yes" }.to_string(),
            "--".to_string(),
            "transfer".to_string(),
            "--from".to_string(),
            from_address,
            "--to".to_string(),
            to_address,
            "--amount".to_string(),
            amount_to_stroops(&args.amount, token.decimals)?,
        ],
    })
}

fn finalize_sep7_payment(
    mut report: CommandReport,
    plan: &PaymentPlan,
    args: &WalletPayArgs,
) -> Result<CommandReport> {
    if plan.primitive == "payment"
        && !plan.args.iter().any(|arg| arg == "native")
        && let Some(asset) = plan.sep7_asset.as_deref()
    {
        let uri = build_pay_uri(&args.to, asset, Some(&args.amount));
        report.message = Some("generated a SEP-7 payment URI".to_string());
        report.data = Some(json!({
            "primitive": plan.primitive,
            "sep7_uri": uri,
        }));
        return Ok(report);
    }
    report.status = "warn".to_string();
    report.message = Some("SEP-7 handoff for contract and SAC flows requires build-only XDR; run the same command without --dry-run to materialize it".to_string());
    report.data = Some(json!({
        "primitive": plan.primitive,
        "commands": plan.args,
    }));
    Ok(report)
}

fn resolve_relayer_submit_url(context: &AppContext, manifest: &Manifest) -> Result<Url> {
    let root = context.project_root();
    let api_root = root.join("apps/api");
    let env = load_event_env_values(&root, &api_root);
    let configured_base = env
        .get("STELLAR_FORGE_API_URL")
        .or_else(|| env.get("PUBLIC_STELLAR_API_URL"))
        .cloned();
    if configured_base.is_none() && !manifest.api.as_ref().is_some_and(|api| api.relayer) {
        bail!(
            "relayer submission expects the generated API proxy; run `stellar forge api relayer init` or set STELLAR_FORGE_API_URL"
        );
    }
    let base = configured_base.unwrap_or_else(|| {
        format!(
            "http://127.0.0.1:{}",
            env.get("PORT").map(String::as_str).unwrap_or("3000")
        )
    });
    let mut url = Url::parse(&base).with_context(|| format!("invalid API base URL `{base}`"))?;
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| anyhow!("invalid API base URL `{base}`"))?;
        segments.push("relayer");
        segments.push("submit");
    }
    Ok(url)
}

fn resolve_wallet_asset(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: Option<&Manifest>,
    lockfile: Option<&Lockfile>,
    asset: &str,
) -> Result<WalletAssetResolution> {
    if matches!(asset, "XLM" | "native") {
        return Ok(WalletAssetResolution {
            display: "XLM".to_string(),
            sep7_asset: Some("XLM".to_string()),
        });
    }
    if let Some(manifest) = manifest
        && let Some(token) = manifest.tokens.get(asset)
    {
        if token.kind == "contract" {
            let contract_id = lockfile
                .and_then(|lockfile| {
                    manifest
                        .active_network(context.globals.network.as_deref())
                        .ok()
                        .and_then(|(env, _)| {
                            lockfile
                                .environments
                                .get(env)
                                .and_then(|environment| environment.tokens.get(asset))
                                .map(|deployment| deployment.contract_id.clone())
                        })
                })
                .filter(|value| !value.is_empty());
            return Ok(WalletAssetResolution {
                display: contract_id.unwrap_or_else(|| asset.to_string()),
                sep7_asset: None,
            });
        }
        let resolved = asset_string(context, report, Some(manifest), token)?;
        return Ok(WalletAssetResolution {
            display: resolved.clone(),
            sep7_asset: Some(resolved),
        });
    }
    Ok(WalletAssetResolution {
        display: asset.to_string(),
        sep7_asset: Some(asset.to_string()),
    })
}

fn build_pay_uri(destination: &str, asset: &str, amount: Option<&str>) -> String {
    let mut uri = String::from("web+stellar:pay");
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    serializer.append_pair("destination", destination);
    if let Some(amount) = amount {
        serializer.append_pair("amount", amount);
    }
    if matches!(asset, "XLM" | "native") {
        serializer.append_pair("asset_code", "XLM");
    } else {
        let (asset_code, asset_issuer) = asset.split_once(':').unwrap_or((asset, ""));
        serializer.append_pair("asset_code", asset_code);
        if !asset_issuer.is_empty() {
            serializer.append_pair("asset_issuer", asset_issuer);
        }
    }
    uri.push('?');
    uri.push_str(&serializer.finish());
    uri
}

fn urlencoding(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}
