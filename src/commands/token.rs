use super::*;

pub(super) fn token_command(context: &AppContext, command: TokenCommand) -> Result<CommandReport> {
    match command {
        TokenCommand::Create(args) => token_create(context, &args),
        TokenCommand::Info { name } => token_info(context, &name),
        TokenCommand::Mint(args) => token_mint(context, &args),
        TokenCommand::Burn(args) => token_burn(context, &args),
        TokenCommand::Transfer(args) => token_transfer(context, &args),
        TokenCommand::Trust { name, wallet } => wallet::wallet_trust(context, &wallet, &name),
        TokenCommand::Freeze { name, holder } => {
            token_authorization(context, &name, &holder, false)
        }
        TokenCommand::Unfreeze { name, holder } => {
            token_authorization(context, &name, &holder, true)
        }
        TokenCommand::Clawback { name, from, amount } => {
            token_clawback(context, &name, &from, &amount)
        }
        TokenCommand::Sac(args) => match args.command {
            TokenSacCommand::Id { name } => token_sac_id(context, &name),
            TokenSacCommand::Deploy { name } => token_sac_deploy(context, &name),
        },
        TokenCommand::Contract(args) => match args.command {
            crate::cli::TokenContractCommand::Init { name } => token_contract_init(context, &name),
        },
        TokenCommand::Balance { name, holder } => token_balance(context, &name, holder.as_deref()),
    }
}

fn token_create(context: &AppContext, args: &TokenCreateArgs) -> Result<CommandReport> {
    let mut report = CommandReport::new("token.create");
    validate_single_path_segment("token name", &args.name)?;
    let mut manifest = load_manifest(context)?;
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    ensure_identity_manifest_entries(&mut manifest, &args.issuer);
    ensure_identity_manifest_entries(&mut manifest, &args.distribution);
    let code = args
        .code
        .clone()
        .unwrap_or_else(|| args.name.to_uppercase().replace('-', "_"));
    manifest.tokens.insert(
        args.name.clone(),
        TokenConfig {
            kind: args.mode.clone(),
            code: code.clone(),
            issuer: format!("@identity:{}", args.issuer),
            distribution: format!("@identity:{}", args.distribution),
            auth_required: args.auth_required,
            auth_revocable: args.auth_revocable,
            clawback_enabled: args.clawback_enabled,
            with_sac: args.with_sac,
            decimals: 7,
            metadata_name: args.metadata_name.clone(),
        },
    );
    if args.mode == "contract" {
        configure_contract_token_manifest(&mut manifest, &args.name)?;
    }
    save_manifest(context, &mut report, &manifest)?;
    if args.mode == "contract" {
        let template = "openzeppelin-token";
        let contract_report = contract_new(context, &args.name, template, false)?;
        report.commands.extend(contract_report.commands);
        report.artifacts.extend(contract_report.artifacts);
        deploy_contract_from_manifest(context, &mut report, &manifest, &args.name, &env)?;
        sync_contract_token_deployment(context, &mut report, &manifest, &args.name, &env)?;
        let lockfile = load_lockfile(context).unwrap_or_default();
        let languages = manifest
            .contracts
            .get(&args.name)
            .map(|contract| contract.bindings.clone())
            .unwrap_or_else(|| vec!["typescript".to_string()]);
        let outputs = generate_contract_bindings(
            context,
            &mut report,
            &manifest,
            &lockfile,
            &env,
            &args.name,
            &languages,
        )?;
        if args.initial_supply != "0" {
            token_mint_contract(
                context,
                &mut report,
                &manifest,
                &env,
                &TokenMoveArgs {
                    name: args.name.clone(),
                    to: args.distribution.clone(),
                    amount: args.initial_supply.clone(),
                    from: Some(args.issuer.clone()),
                },
            )?;
        }
        report.message = Some(format!(
            "contract token `{}` scaffolded, deployed, initialized, and bound",
            args.name
        ));
        report.network = Some(env.clone());
        report.data = Some(json!({
            "token": args.name,
            "mode": "contract",
            "bindings": outputs,
            "contract": manifest.contracts.get(&args.name),
        }));
        return Ok(report);
    }
    token_create_from_manifest(context, &mut report, &manifest, &args.name, &env)?;
    if args.initial_supply != "0" {
        let token = manifest
            .tokens
            .get(&args.name)
            .ok_or_else(|| anyhow!("token `{}` not found after manifest update", args.name))?;
        let issuer = resolve_identity_name(Some(&manifest), &token.issuer)
            .unwrap_or_else(|| "issuer".to_string());
        let distribution =
            resolve_address(context, &mut report, Some(&manifest), &token.distribution)?;
        let asset = asset_string(context, &mut report, Some(&manifest), token)?;
        context.run_command(
            &mut report,
            Some(&context.project_root()),
            "stellar",
            &[
                "tx".to_string(),
                "new".to_string(),
                "payment".to_string(),
                "--source-account".to_string(),
                issuer,
                "--destination".to_string(),
                distribution,
                "--asset".to_string(),
                asset,
                "--amount".to_string(),
                amount_to_stroops(&args.initial_supply, token.decimals)?,
                "--network".to_string(),
                env.clone(),
            ],
        )?;
    }
    report.message = Some(format!("asset token `{}` created", args.name));
    report.network = Some(env);
    Ok(report)
}

fn token_info(context: &AppContext, name: &str) -> Result<CommandReport> {
    let mut report = CommandReport::new("token.info");
    let manifest = load_manifest(context)?;
    let lockfile = load_lockfile(context)?;
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    let token = manifest
        .tokens
        .get(name)
        .ok_or_else(|| anyhow!("token `{name}` not found"))?;
    let deployment = lockfile
        .environments
        .get(&env)
        .and_then(|environment| environment.tokens.get(name));
    report.message = Some(format!("token summary for `{name}`"));
    report.network = Some(env);
    report.data = Some(json!({
        "token": token,
        "deployment": deployment,
    }));
    Ok(report)
}

fn token_mint(context: &AppContext, args: &TokenMoveArgs) -> Result<CommandReport> {
    let manifest = load_manifest(context)?;
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    let token = manifest
        .tokens
        .get(&args.name)
        .ok_or_else(|| anyhow!("token `{}` not found", args.name))?;
    if token.kind == "contract" {
        let mut report = CommandReport::new("token.mint");
        token_mint_contract(context, &mut report, &manifest, &env, args)?;
        report.message = Some(format!("contract token `{}` minted", args.name));
        report.network = Some(env);
        return Ok(report);
    }
    let from = args.from.clone().unwrap_or_else(|| "issuer".to_string());
    token_transfer(
        context,
        &TokenMoveArgs {
            name: args.name.clone(),
            to: args.to.clone(),
            amount: args.amount.clone(),
            from: Some(from),
        },
    )
}

fn token_transfer(context: &AppContext, args: &TokenMoveArgs) -> Result<CommandReport> {
    wallet::wallet_pay(
        context,
        &WalletPayArgs {
            from: args.from.clone().unwrap_or_else(|| "treasury".to_string()),
            to: args.to.clone(),
            asset: args.name.clone(),
            amount: args.amount.clone(),
            sep7: false,
            build_only: false,
            relayer: false,
        },
    )
}

fn token_burn(context: &AppContext, args: &TokenBurnArgs) -> Result<CommandReport> {
    let manifest = load_manifest(context)?;
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    let token = manifest
        .tokens
        .get(&args.name)
        .ok_or_else(|| anyhow!("token `{}` not found", args.name))?;
    if token.kind == "contract" {
        let mut report = CommandReport::new("token.burn");
        token_burn_contract(context, &mut report, &manifest, &env, args)?;
        report.message = Some(format!("contract token `{}` burned", args.name));
        report.network = Some(env);
        return Ok(report);
    }

    let holder = args.from.clone().unwrap_or_else(|| {
        resolve_identity_name(Some(&manifest), &token.distribution)
            .unwrap_or_else(|| "treasury".to_string())
    });
    let issuer = resolve_identity_name(Some(&manifest), &token.issuer)
        .unwrap_or_else(|| "issuer".to_string());
    let payment = wallet::wallet_pay(
        context,
        &WalletPayArgs {
            from: holder.clone(),
            to: issuer.clone(),
            asset: args.name.clone(),
            amount: args.amount.clone(),
            sep7: false,
            build_only: false,
            relayer: false,
        },
    )?;
    let mut report = CommandReport::new("token.burn");
    report.commands = payment.commands;
    report.artifacts = payment.artifacts;
    report.warnings = payment.warnings;
    report.network = payment.network;
    report.message = Some(format!(
        "classic asset `{}` burned by returning supply to `{}`",
        args.name, issuer
    ));
    report.data = Some(json!({
        "mode": "asset",
        "from": holder,
        "issuer": issuer,
        "amount": args.amount.clone(),
        "primitive": "payment",
    }));
    Ok(report)
}

fn token_contract_init(context: &AppContext, name: &str) -> Result<CommandReport> {
    let mut report = CommandReport::new("token.contract.init");
    let manifest = load_manifest(context)?;
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    run_contract_token_init(context, &mut report, &manifest, &env, name)?;
    report.message = Some(format!("contract token `{name}` initialized"));
    report.network = Some(env);
    Ok(report)
}

pub(super) fn contract_effective_init_config(
    manifest: &Manifest,
    name: &str,
) -> Result<Option<crate::model::ContractInitConfig>> {
    let Some(contract) = manifest.contracts.get(name) else {
        return Ok(None);
    };

    let Some(token) = manifest
        .tokens
        .get(name)
        .filter(|token| token.kind == "contract")
    else {
        return Ok(contract.init.clone());
    };

    let default_init = contract_token_init_config(manifest, name, Some(token))?;
    let mut init = contract.init.clone().unwrap_or_default();
    if init.fn_name.trim().is_empty() {
        init.fn_name = default_init.fn_name;
    }
    for (key, value) in default_init.args {
        match init.args.get_mut(&key) {
            Some(current) if !current.trim().is_empty() => {}
            Some(current) => *current = value,
            None => {
                init.args.insert(key, value);
            }
        }
    }
    if init.fn_name.trim().is_empty() {
        init.fn_name = "init".to_string();
    }
    Ok(Some(init))
}

fn configure_contract_token_manifest(manifest: &mut Manifest, name: &str) -> Result<()> {
    let token = manifest
        .tokens
        .get(name)
        .cloned()
        .ok_or_else(|| anyhow!("token `{name}` not found"))?;
    let init = contract_token_init_config(manifest, name, Some(&token))?;
    let entry = manifest
        .contracts
        .entry(name.to_string())
        .or_insert_with(|| ContractConfig {
            path: format!("contracts/{name}"),
            alias: name.to_string(),
            template: "openzeppelin-token".to_string(),
            bindings: vec!["typescript".to_string()],
            deploy_on: vec!["local".to_string(), "testnet".to_string()],
            init: None,
        });
    if entry.path.trim().is_empty() {
        entry.path = format!("contracts/{name}");
    }
    if entry.alias.trim().is_empty() {
        entry.alias = name.to_string();
    }
    entry.template = "openzeppelin-token".to_string();
    if entry.bindings.is_empty() {
        entry.bindings = vec!["typescript".to_string()];
    }
    if entry.deploy_on.is_empty() {
        entry.deploy_on = vec!["local".to_string(), "testnet".to_string()];
    }
    entry.init = Some(init);
    Ok(())
}

fn contract_token_init_config(
    manifest: &Manifest,
    name: &str,
    token_hint: Option<&TokenConfig>,
) -> Result<crate::model::ContractInitConfig> {
    let token = token_hint
        .cloned()
        .or_else(|| manifest.tokens.get(name).cloned())
        .ok_or_else(|| anyhow!("token `{name}` not found"))?;
    let admin = resolve_identity_name(Some(manifest), &token.issuer)
        .unwrap_or_else(|| "issuer".to_string());
    let display_name = token
        .metadata_name
        .clone()
        .unwrap_or_else(|| title_case_token_name(name));
    Ok(crate::model::ContractInitConfig {
        fn_name: "init".to_string(),
        args: BTreeMap::from([
            ("admin".to_string(), format!("@identity:{admin}")),
            ("name".to_string(), display_name),
            (
                "symbol".to_string(),
                if token.code.trim().is_empty() {
                    name.to_uppercase().replace('-', "_")
                } else {
                    token.code.clone()
                },
            ),
            ("decimals".to_string(), token.decimals.to_string()),
        ]),
    })
}

fn title_case_token_name(name: &str) -> String {
    name.split(['-', '_'])
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => format!(
                    "{}{}",
                    first.to_ascii_uppercase(),
                    chars.as_str().to_ascii_lowercase()
                ),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn contract_token_contract_id(
    manifest: &Manifest,
    lockfile: &Lockfile,
    env: &str,
    name: &str,
) -> String {
    lockfile
        .environments
        .get(env)
        .and_then(|environment| environment.tokens.get(name))
        .map(|deployment| deployment.contract_id.clone())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| resolve_contract_id(manifest, lockfile, env, name))
}

fn run_contract_token_init(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: &Manifest,
    env: &str,
    name: &str,
) -> Result<()> {
    let token = manifest
        .tokens
        .get(name)
        .ok_or_else(|| anyhow!("token `{name}` not found"))?;
    if token.kind != "contract" {
        bail!("token `{name}` is not declared as a contract token");
    }
    let init = contract_effective_init_config(manifest, name)?
        .unwrap_or(contract_token_init_config(manifest, name, Some(token))?);
    let lockfile = load_lockfile(context).unwrap_or_default();
    let source = resolve_identity_name(Some(manifest), &token.issuer)
        .unwrap_or_else(|| "issuer".to_string());
    let contract_id = contract_token_contract_id(manifest, &lockfile, env, name);
    let mut args = vec![
        "contract".to_string(),
        "invoke".to_string(),
        "--id".to_string(),
        contract_id.clone(),
        "--source-account".to_string(),
        source.clone(),
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
        args.push(resolve_argument_value(
            context,
            report,
            Some(manifest),
            env,
            Some(&lockfile),
            value,
        )?);
    }
    let output = context.run_command(report, Some(&context.project_root()), "stellar", &args)?;
    report.data = Some(json!({
        "token": name,
        "contract_id": contract_id,
        "source": source,
        "init": init,
        "result": if output.is_empty() { Value::Null } else { Value::String(output) },
    }));
    Ok(())
}

fn token_mint_contract(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: &Manifest,
    env: &str,
    args: &TokenMoveArgs,
) -> Result<()> {
    let mut guarded_inputs = vec![args.to.as_str()];
    if let Some(from) = args.from.as_deref() {
        guarded_inputs.push(from);
    }
    wallet::ensure_named_wallets_are_materialized(manifest, &guarded_inputs)?;
    let token = manifest
        .tokens
        .get(&args.name)
        .ok_or_else(|| anyhow!("token `{}` not found", args.name))?;
    let lockfile = load_lockfile(context).unwrap_or_default();
    let source = args
        .from
        .clone()
        .or_else(|| resolve_identity_name(Some(manifest), &token.issuer))
        .unwrap_or_else(|| "issuer".to_string());
    let to = resolve_address(context, report, Some(manifest), &args.to)?;
    let contract_id = contract_token_contract_id(manifest, &lockfile, env, &args.name);
    let output = context.run_command(
        report,
        Some(&context.project_root()),
        "stellar",
        &[
            "contract".to_string(),
            "invoke".to_string(),
            "--id".to_string(),
            contract_id.clone(),
            "--source-account".to_string(),
            source.clone(),
            "--network".to_string(),
            env.to_string(),
            "--send".to_string(),
            "yes".to_string(),
            "--".to_string(),
            "mint".to_string(),
            "--to".to_string(),
            to.clone(),
            "--amount".to_string(),
            amount_to_stroops(&args.amount, token.decimals)?,
        ],
    )?;
    report.data = Some(json!({
        "token": args.name.clone(),
        "mode": "contract",
        "contract_id": contract_id,
        "source": source,
        "to": to,
        "amount": args.amount.clone(),
        "result": if output.is_empty() { Value::Null } else { Value::String(output) },
    }));
    Ok(())
}

fn token_burn_contract(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: &Manifest,
    env: &str,
    args: &TokenBurnArgs,
) -> Result<()> {
    if let Some(from) = args.from.as_deref() {
        wallet::ensure_named_wallets_are_materialized(manifest, &[from])?;
    }
    let token = manifest
        .tokens
        .get(&args.name)
        .ok_or_else(|| anyhow!("token `{}` not found", args.name))?;
    let lockfile = load_lockfile(context).unwrap_or_default();
    let from = args
        .from
        .clone()
        .or_else(|| resolve_identity_name(Some(manifest), &token.distribution))
        .unwrap_or_else(|| "treasury".to_string());
    let from_address = resolve_address(context, report, Some(manifest), &from)?;
    let contract_id = contract_token_contract_id(manifest, &lockfile, env, &args.name);
    let output = context.run_command(
        report,
        Some(&context.project_root()),
        "stellar",
        &[
            "contract".to_string(),
            "invoke".to_string(),
            "--id".to_string(),
            contract_id.clone(),
            "--source-account".to_string(),
            from.clone(),
            "--network".to_string(),
            env.to_string(),
            "--send".to_string(),
            "yes".to_string(),
            "--".to_string(),
            "burn".to_string(),
            "--from".to_string(),
            from_address.clone(),
            "--amount".to_string(),
            amount_to_stroops(&args.amount, token.decimals)?,
        ],
    )?;
    report.data = Some(json!({
        "token": args.name.clone(),
        "mode": "contract",
        "contract_id": contract_id,
        "from": from_address,
        "amount": args.amount.clone(),
        "result": if output.is_empty() { Value::Null } else { Value::String(output) },
    }));
    Ok(())
}

fn token_authorization(
    context: &AppContext,
    name: &str,
    holder: &str,
    authorize: bool,
) -> Result<CommandReport> {
    let mut report = CommandReport::new(if authorize {
        "token.unfreeze"
    } else {
        "token.freeze"
    });
    let manifest = load_manifest(context)?;
    wallet::ensure_named_wallets_are_materialized(&manifest, &[holder])?;
    let token = manifest
        .tokens
        .get(name)
        .ok_or_else(|| anyhow!("token `{name}` not found"))?;
    if token.kind == "contract" {
        bail!(
            "token `{name}` is a contract token; freeze/unfreeze needs a contract-specific admin call"
        );
    }
    if !token.auth_required || !token.auth_revocable {
        bail!(
            "token `{name}` does not support freeze/unfreeze via trustline authorization; set `auth_required = true` and `auth_revocable = true` in the manifest"
        );
    }
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    let issuer = resolve_identity_name(Some(&manifest), &token.issuer)
        .unwrap_or_else(|| "issuer".to_string());
    let holder_address = resolve_address(context, &mut report, Some(&manifest), holder)?;
    let asset = asset_string(context, &mut report, Some(&manifest), token)?;
    let mut args = vec![
        "tx".to_string(),
        "new".to_string(),
        "set-trustline-flags".to_string(),
        "--source-account".to_string(),
        issuer.clone(),
        "--trustor".to_string(),
        holder_address,
        "--asset".to_string(),
        asset,
        "--network".to_string(),
        env.clone(),
    ];
    args.push(if authorize {
        "--set-authorize".to_string()
    } else {
        "--clear-authorize".to_string()
    });
    context.run_command(&mut report, Some(&context.project_root()), "stellar", &args)?;
    report.message = Some(format!("authorization updated for holder `{holder}`"));
    report.network = Some(env);
    Ok(report)
}

fn token_clawback(
    context: &AppContext,
    name: &str,
    from: &str,
    amount: &str,
) -> Result<CommandReport> {
    let mut report = CommandReport::new("token.clawback");
    let manifest = load_manifest(context)?;
    wallet::ensure_named_wallets_are_materialized(&manifest, &[from])?;
    let token = manifest
        .tokens
        .get(name)
        .ok_or_else(|| anyhow!("token `{name}` not found"))?;
    if token.kind == "contract" {
        bail!("token `{name}` is a contract token; clawback needs a contract-specific admin call");
    }
    if !token.clawback_enabled {
        bail!("token `{name}` does not have `clawback_enabled = true` in the manifest");
    }
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    let issuer = resolve_identity_name(Some(&manifest), &token.issuer)
        .unwrap_or_else(|| "issuer".to_string());
    let from_address = resolve_address(context, &mut report, Some(&manifest), from)?;
    let asset = asset_string(context, &mut report, Some(&manifest), token)?;
    let stroops = amount_to_stroops(amount, token.decimals)?;
    let args = vec![
        "tx".to_string(),
        "new".to_string(),
        "clawback".to_string(),
        "--source-account".to_string(),
        issuer,
        "--from".to_string(),
        from_address,
        "--asset".to_string(),
        asset,
        "--amount".to_string(),
        stroops,
        "--network".to_string(),
        env.clone(),
    ];
    context.run_command(&mut report, Some(&context.project_root()), "stellar", &args)?;
    report.message = Some(format!("clawback executed for `{name}`"));
    report.network = Some(env);
    Ok(report)
}

fn token_sac_id(context: &AppContext, name: &str) -> Result<CommandReport> {
    let mut report = CommandReport::new("token.sac.id");
    let manifest = load_manifest(context)?;
    let token = manifest
        .tokens
        .get(name)
        .ok_or_else(|| anyhow!("token `{name}` not found"))?;
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    let asset = asset_string(context, &mut report, Some(&manifest), token)?;
    let args = vec![
        "contract".to_string(),
        "asset".to_string(),
        "id".to_string(),
        "--asset".to_string(),
        asset,
        "--network".to_string(),
        env.clone(),
    ];
    let output =
        context.run_command(&mut report, Some(&context.project_root()), "stellar", &args)?;
    report.message = Some(format!("resolved SAC id for `{name}`"));
    report.network = Some(env);
    report.data = Some(json!({ "sac_contract_id": output }));
    Ok(report)
}

fn token_sac_deploy(context: &AppContext, name: &str) -> Result<CommandReport> {
    let mut report = CommandReport::new("token.sac.deploy");
    let manifest = load_manifest(context)?;
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    deploy_sac_for_token(context, &mut report, &manifest, name, &env)?;
    report.message = Some(format!("SAC deployed for `{name}`"));
    report.network = Some(env);
    Ok(report)
}

fn token_balance(context: &AppContext, name: &str, holder: Option<&str>) -> Result<CommandReport> {
    let holder = holder.unwrap_or("alice");
    let balances = wallet::wallet_balances(context, holder)?;
    let mut report = CommandReport::new("token.balance");
    report.commands = balances.commands;
    report.artifacts = balances.artifacts;
    report.warnings = balances.warnings;
    report.network = balances.network;
    report.message = Some(format!("filtered balances for `{name}` on `{holder}`"));
    if let Some(data) = balances.data {
        let filtered = data
            .get("project_tokens")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter(|item| item.get("name").and_then(Value::as_str) == Some(name))
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        report.data = Some(json!({
            "holder": holder,
            "address": data.get("address").cloned().unwrap_or(Value::Null),
            "token": filtered.first().cloned().unwrap_or(Value::Null),
            "project_tokens": filtered,
        }));
    }
    Ok(report)
}

pub(super) fn token_create_from_manifest(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: &Manifest,
    name: &str,
    env: &str,
) -> Result<()> {
    let token = manifest
        .tokens
        .get(name)
        .ok_or_else(|| anyhow!("token `{name}` not found"))?;
    if token.kind == "contract" {
        if !manifest.contracts.contains_key(name) {
            bail!(
                "token `{name}` is declared as a contract token but no matching contract `{name}` exists in the manifest"
            );
        }
        let deployed = load_lockfile(context)?
            .environments
            .get(env)
            .and_then(|environment| environment.contracts.get(name))
            .map(|deployment| deployment.contract_id.clone())
            .filter(|contract_id| !contract_id.is_empty());
        if deployed.is_none() {
            deploy_contract_from_manifest(context, report, manifest, name, env)?;
        }
        sync_contract_token_deployment(context, report, manifest, name, env)?;
        return Ok(());
    }
    let issuer_name = resolve_identity_name(Some(manifest), &token.issuer)
        .unwrap_or_else(|| "issuer".to_string());
    let distribution_name = resolve_identity_name(Some(manifest), &token.distribution)
        .unwrap_or_else(|| "treasury".to_string());
    ensure_identity_exists(context, report, manifest, &issuer_name, env, true)?;
    ensure_identity_exists(context, report, manifest, &distribution_name, env, true)?;
    let issuer_address = resolve_address(context, report, Some(manifest), &token.issuer)?;
    let asset = format!("{}:{}", token.code, issuer_address);
    if token.auth_required || token.auth_revocable || token.clawback_enabled {
        let mut args = vec![
            "tx".to_string(),
            "new".to_string(),
            "set-options".to_string(),
            "--source-account".to_string(),
            issuer_name.clone(),
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
        context.run_command(report, Some(&context.project_root()), "stellar", &args)?;
    }
    context.run_command(
        report,
        Some(&context.project_root()),
        "stellar",
        &[
            "tx".to_string(),
            "new".to_string(),
            "change-trust".to_string(),
            "--source-account".to_string(),
            distribution_name.clone(),
            "--line".to_string(),
            asset.clone(),
            "--network".to_string(),
            env.to_string(),
        ],
    )?;
    if token.auth_required {
        let distribution_address =
            resolve_address(context, report, Some(manifest), &token.distribution)?;
        context.run_command(
            report,
            Some(&context.project_root()),
            "stellar",
            &[
                "tx".to_string(),
                "new".to_string(),
                "set-trustline-flags".to_string(),
                "--source-account".to_string(),
                issuer_name.clone(),
                "--trustor".to_string(),
                distribution_address,
                "--asset".to_string(),
                asset.clone(),
                "--set-authorize".to_string(),
                "--network".to_string(),
                env.to_string(),
            ],
        )?;
    }
    let mut lockfile = load_lockfile(context)?;
    let environment = lockfile.environment_mut(env);
    let mut deployment = environment.tokens.get(name).cloned().unwrap_or_default();
    deployment.kind = token.kind.clone();
    deployment.asset = asset.clone();
    deployment.issuer_identity = issuer_name.clone();
    deployment.distribution_identity = distribution_name.clone();
    environment.tokens.insert(name.to_string(), deployment);
    save_lockfile(context, report, &lockfile)?;
    if token.with_sac {
        deploy_sac_for_token(context, report, manifest, name, env)?;
    }
    Ok(())
}

fn sync_contract_token_deployment(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: &Manifest,
    name: &str,
    env: &str,
) -> Result<()> {
    let token = manifest
        .tokens
        .get(name)
        .ok_or_else(|| anyhow!("token `{name}` not found"))?;
    let mut lockfile = load_lockfile(context).unwrap_or_default();
    let fallback_contract_id = if context.globals.dry_run {
        Some(resolve_contract_id(manifest, &lockfile, env, name))
    } else {
        None
    };
    let existing_contract_id = lockfile
        .environments
        .get(env)
        .and_then(|environment| environment.contracts.get(name))
        .map(|deployment| deployment.contract_id.clone())
        .or(fallback_contract_id)
        .filter(|contract_id| !contract_id.is_empty())
        .ok_or_else(|| anyhow!("contract token `{name}` has no deployed contract id in `{env}`"))?;
    let environment = lockfile.environment_mut(env);
    let mut deployment = environment.tokens.get(name).cloned().unwrap_or_default();
    deployment.kind = token.kind.clone();
    deployment.contract_id = existing_contract_id;
    environment.tokens.insert(name.to_string(), deployment);
    save_lockfile(context, report, &lockfile)
}

fn deploy_sac_for_token(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: &Manifest,
    name: &str,
    env: &str,
) -> Result<()> {
    let mut lockfile = load_lockfile(context)?;
    let environment = lockfile.environment_mut(env);
    if environment
        .tokens
        .get(name)
        .is_some_and(|deployment| !deployment.sac_contract_id.is_empty())
    {
        return Ok(());
    }
    let token = manifest
        .tokens
        .get(name)
        .ok_or_else(|| anyhow!("token `{name}` not found"))?;
    let issuer = resolve_identity_name(Some(manifest), &token.issuer)
        .unwrap_or_else(|| "issuer".to_string());
    let asset = asset_string(context, report, Some(manifest), token)?;
    let alias = format!("{name}-sac");
    let deploy_args = vec![
        "contract".to_string(),
        "asset".to_string(),
        "deploy".to_string(),
        "--asset".to_string(),
        asset.clone(),
        "--source-account".to_string(),
        issuer,
        "--alias".to_string(),
        alias.clone(),
        "--network".to_string(),
        env.to_string(),
    ];
    let output = match context.run_command(
        report,
        Some(&context.project_root()),
        "stellar",
        &deploy_args,
    ) {
        Ok(output) => output,
        Err(error) => {
            let message = error.to_string();
            if !message.contains("ExistingValue") && !message.contains("contract already exists") {
                return Err(error);
            }
            let contract_id = context.run_command(
                report,
                Some(&context.project_root()),
                "stellar",
                &[
                    "contract".to_string(),
                    "id".to_string(),
                    "asset".to_string(),
                    "--asset".to_string(),
                    asset,
                    "--network".to_string(),
                    env.to_string(),
                ],
            )?;
            if let Err(alias_error) = context.run_command(
                report,
                Some(&context.project_root()),
                "stellar",
                &[
                    "contract".to_string(),
                    "alias".to_string(),
                    "add".to_string(),
                    "--id".to_string(),
                    contract_id.clone(),
                    "--alias".to_string(),
                    alias,
                    "--network".to_string(),
                    env.to_string(),
                ],
            ) {
                report.warnings.push(format!(
                    "failed to refresh SAC alias after detecting an existing contract: {alias_error}"
                ));
            }
            contract_id
        }
    };
    let mut deployment = environment.tokens.get(name).cloned().unwrap_or_default();
    deployment.sac_contract_id = if output.is_empty() {
        format!("{name}-sac")
    } else {
        output
    };
    deployment.kind = token.kind.clone();
    environment.tokens.insert(name.to_string(), deployment);
    save_lockfile(context, report, &lockfile)?;
    Ok(())
}
