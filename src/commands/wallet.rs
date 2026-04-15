use super::*;
use crate::cli::{
    WalletBatchPayArgs, WalletBatchReconcileArgs, WalletBatchResumeArgs,
    WalletSmartControllerCommand, WalletSmartPolicyCommand,
};
use csv::{ReaderBuilder, Trim};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn wallet_command(
    context: &AppContext,
    command: WalletCommand,
) -> Result<CommandReport> {
    let out = wallet_command_output_path(&command);
    let mut report = match command {
        WalletCommand::Create { name, fund, .. } => wallet_create(context, &name, fund),
        WalletCommand::Ls(_) => wallet_ls(context),
        WalletCommand::Address(args) => wallet_address(context, &args.name),
        WalletCommand::Fund {
            name_or_address, ..
        } => wallet_fund(context, &name_or_address),
        WalletCommand::Balances(args) => wallet_balances(context, &args.name_or_address),
        WalletCommand::Trust { wallet, token, .. } => wallet_trust(context, &wallet, &token),
        WalletCommand::Pay(args) => wallet_pay(context, &args),
        WalletCommand::BatchPay(args) => wallet_batch_pay(context, &args),
        WalletCommand::BatchReconcile(args) => wallet_batch_reconcile_command(context, &args),
        WalletCommand::BatchResume(args) => wallet_batch_resume(context, &args),
        WalletCommand::BatchReport(args) => wallet_batch_report(context, &args),
        WalletCommand::BatchValidate(args) => wallet_batch_validate(context, &args),
        WalletCommand::BatchPreview(args) => wallet_batch_preview(context, &args),
        WalletCommand::BatchSummary(args) => wallet_batch_summary(context, &args),
        WalletCommand::Receive(args) => wallet_receive(
            context,
            &args.wallet,
            args.sep7,
            args.qr,
            args.asset.as_deref(),
        ),
        WalletCommand::Sep7(args) => match args.command {
            WalletSep7Command::Payment(args) => wallet_pay_sep7(context, &args),
            WalletSep7Command::ContractCall(args) => wallet_sep7_contract_call(context, &args),
        },
        WalletCommand::Smart(args) => match args.command {
            WalletSmartCommand::Create { name, mode, .. } => {
                wallet_smart_create(context, &name, mode)
            }
            WalletSmartCommand::Scaffold { name, .. } => wallet_smart_scaffold(context, &name),
            WalletSmartCommand::Info { name, .. } => wallet_smart_info(context, &name),
            WalletSmartCommand::Onboard(args) => wallet_smart_onboard(context, &args.name),
            WalletSmartCommand::Provision {
                name,
                address,
                out: _,
                fund,
            } => wallet_smart_provision(context, &name, address.as_deref(), fund),
            WalletSmartCommand::Materialize {
                name,
                out: _,
                fund,
                no_policy_deploy,
            } => wallet_smart_materialize(context, &name, fund, no_policy_deploy),
            WalletSmartCommand::Controller(args) => match args.command {
                WalletSmartControllerCommand::Rotate {
                    name,
                    identity,
                    out: _,
                    fund,
                } => wallet_smart_controller_rotate(context, &name, &identity, fund),
            },
            WalletSmartCommand::Policy(args) => match args.command {
                WalletSmartPolicyCommand::Info(args) => {
                    wallet_smart_policy_info(context, &args.name)
                }
                WalletSmartPolicyCommand::Diff(args) => {
                    wallet_smart_policy_diff(context, &args.name)
                }
                WalletSmartPolicyCommand::Sync(args) => {
                    wallet_smart_policy_sync(context, &args.name)
                }
                WalletSmartPolicyCommand::Simulate {
                    name,
                    file,
                    source,
                    out: _,
                } => wallet_smart_policy_simulate(context, &name, &file, source.as_deref()),
                WalletSmartPolicyCommand::Apply {
                    name,
                    file,
                    source,
                    out: _,
                    build_only,
                } => {
                    wallet_smart_policy_apply(context, &name, &file, source.as_deref(), build_only)
                }
                WalletSmartPolicyCommand::SetDailyLimit {
                    name,
                    amount,
                    source,
                    build_only,
                    out: _,
                } => wallet_smart_policy_set_daily_limit(
                    context,
                    &name,
                    &amount,
                    source.as_deref(),
                    build_only,
                ),
                WalletSmartPolicyCommand::Allow {
                    name,
                    address,
                    source,
                    build_only,
                    out: _,
                } => wallet_smart_policy_access_update(
                    context,
                    &name,
                    "allow",
                    &address,
                    source.as_deref(),
                    build_only,
                ),
                WalletSmartPolicyCommand::Revoke {
                    name,
                    address,
                    source,
                    build_only,
                    out: _,
                } => wallet_smart_policy_access_update(
                    context,
                    &name,
                    "revoke",
                    &address,
                    source.as_deref(),
                    build_only,
                ),
            },
        },
    }?;
    if let Some(path) = out.as_deref() {
        persist_report_output(context, &mut report, path)?;
    }
    Ok(report)
}

fn wallet_command_output_path(command: &WalletCommand) -> Option<PathBuf> {
    match command {
        WalletCommand::Create { out, .. } => out.clone(),
        WalletCommand::BatchPay(args)
        | WalletCommand::BatchReport(args)
        | WalletCommand::BatchValidate(args)
        | WalletCommand::BatchPreview(args)
        | WalletCommand::BatchSummary(args) => args.out.clone(),
        WalletCommand::BatchResume(args) => args.out.clone(),
        WalletCommand::BatchReconcile(args) => args.out.clone(),
        WalletCommand::Ls(args) => args.out.clone(),
        WalletCommand::Address(args) => args.out.clone(),
        WalletCommand::Fund { out, .. } => out.clone(),
        WalletCommand::Balances(args) => args.out.clone(),
        WalletCommand::Trust { out, .. } => out.clone(),
        WalletCommand::Pay(args) => args.out.clone(),
        WalletCommand::Receive(args) => args.out.clone(),
        WalletCommand::Sep7(args) => match &args.command {
            WalletSep7Command::Payment(args) => args.out.clone(),
            WalletSep7Command::ContractCall(args) => args.out.clone(),
        },
        WalletCommand::Smart(args) => match &args.command {
            WalletSmartCommand::Create { out, .. } => out.clone(),
            WalletSmartCommand::Scaffold { out, .. } => out.clone(),
            WalletSmartCommand::Info { out, .. } => out.clone(),
            WalletSmartCommand::Onboard(args) => args.out.clone(),
            WalletSmartCommand::Provision { out, .. } => out.clone(),
            WalletSmartCommand::Materialize { out, .. } => out.clone(),
            WalletSmartCommand::Controller(args) => match &args.command {
                WalletSmartControllerCommand::Rotate { out, .. } => out.clone(),
            },
            WalletSmartCommand::Policy(args) => match &args.command {
                WalletSmartPolicyCommand::Info(args)
                | WalletSmartPolicyCommand::Diff(args)
                | WalletSmartPolicyCommand::Sync(args) => args.out.clone(),
                WalletSmartPolicyCommand::Simulate { out, .. } => out.clone(),
                WalletSmartPolicyCommand::Apply { out, .. } => out.clone(),
                WalletSmartPolicyCommand::SetDailyLimit { out, .. } => out.clone(),
                WalletSmartPolicyCommand::Allow { out, .. } => out.clone(),
                WalletSmartPolicyCommand::Revoke { out, .. } => out.clone(),
            },
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
    network_name: &'a str,
    rpc_url: &'a str,
    onboarding_root: &'a Path,
    policy_contract: &'a str,
    policy_root: &'a Path,
    controller_identity: Option<&'a str>,
    contract_id: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
struct SmartWalletManifestUpdate<'a> {
    name: &'a str,
    mode: &'a str,
    env: &'a str,
    onboarding_relative: &'a str,
    policy_contract: &'a str,
    policy_relative: &'a str,
    controller_identity: Option<&'a str>,
    contract_id: Option<&'a str>,
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

#[derive(Debug, Clone, Copy)]
enum BatchPaymentFormat {
    Csv,
    Json,
}

#[derive(Debug, Clone, Deserialize)]
struct BatchPaymentEntry {
    to: String,
    amount: String,
    #[serde(default)]
    asset: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct BatchPaymentCsvEntry {
    to: String,
    amount: String,
    #[serde(default)]
    asset: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BatchPaymentPreviewEntry {
    index: usize,
    to: String,
    amount: String,
    asset: String,
    asset_source: String,
}

#[derive(Debug, Clone, Serialize)]
struct BatchPaymentSummary {
    kind: &'static str,
    file: String,
    format: &'static str,
    count: usize,
    default_asset: Option<String>,
    explicit_assets: usize,
    inferred_assets: usize,
    unique_destinations: usize,
    unique_assets: usize,
}

#[derive(Debug, Clone)]
struct BatchPaymentPlan {
    entries: Vec<BatchPaymentEntry>,
    summary: BatchPaymentSummary,
    preview: Vec<BatchPaymentPreviewEntry>,
}

#[derive(Debug, Clone)]
struct SmartWalletPolicyDetails {
    wallet_name: String,
    wallet: WalletConfig,
    controller_identity: Option<String>,
    policy_contract: String,
    policy_root: PathBuf,
    onboarding_relative: String,
    deployment: Option<ContractDeployment>,
}

#[derive(Debug, Clone)]
struct BatchExecutionReport {
    action: String,
    executed: bool,
    entries: Vec<BatchPaymentPreviewEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct SmartWalletPolicyApplyFile {
    #[serde(default)]
    daily_limit: Option<SmartWalletPolicyValue>,
    #[serde(default)]
    allow: Vec<String>,
    #[serde(default)]
    revoke: Vec<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    build_only: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum SmartWalletPolicyValue {
    String(String),
    Signed(i64),
    Unsigned(u64),
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

pub(super) fn wallet_batch_pay(
    context: &AppContext,
    args: &WalletBatchPayArgs,
) -> Result<CommandReport> {
    let format = batch_payment_format(&args.file, args.format.as_deref())?;
    let plan = load_batch_payment_plan(&args.file, format, args.asset.as_deref())?;
    let mut report = CommandReport::new("wallet.batch-pay");

    let mut payments = Vec::new();
    for (index, entry) in plan.entries.iter().enumerate() {
        let (asset, asset_source) =
            resolve_batch_payment_asset(entry, args.asset.as_deref(), index)?;
        let child = wallet_pay(
            context,
            &WalletPayArgs {
                from: args.from.clone(),
                to: entry.to.clone(),
                asset: asset.clone(),
                amount: entry.amount.clone(),
                sep7: args.sep7,
                build_only: args.build_only,
                relayer: args.relayer,
                out: None,
            },
        )?;
        report.commands.extend(child.commands);
        report.artifacts.extend(child.artifacts);
        report.warnings.extend(child.warnings);
        payments.push(json!({
            "index": index + 1,
            "to": entry.to.clone(),
            "amount": entry.amount.clone(),
            "asset": asset,
            "asset_source": asset_source,
            "result": child.data,
        }));
    }

    report.message = Some(format!(
        "processed {} batch payment(s) from `{}`",
        payments.len(),
        args.from
    ));
    report.data = Some(json!({
        "summary": plan.summary,
        "preview": plan.preview,
        "count": payments.len(),
        "from": args.from,
        "file": args.file.display().to_string(),
        "format": match format {
            BatchPaymentFormat::Csv => "csv",
            BatchPaymentFormat::Json => "json",
        },
        "default_asset": args.asset,
        "payments": payments,
    }));
    report.next = vec![format!("stellar forge wallet balances {}", args.from)];
    Ok(report)
}

pub(crate) fn wallet_batch_validate(
    context: &AppContext,
    args: &WalletBatchPayArgs,
) -> Result<CommandReport> {
    batch_payment_report(context, args, BatchPaymentMode::Validate)
}

pub(crate) fn wallet_batch_report(
    context: &AppContext,
    args: &WalletBatchPayArgs,
) -> Result<CommandReport> {
    batch_payment_report(context, args, BatchPaymentMode::Report)
}

pub(crate) fn wallet_batch_reconcile_command(
    _context: &AppContext,
    args: &WalletBatchReconcileArgs,
) -> Result<CommandReport> {
    let batch_args = WalletBatchPayArgs {
        from: args.from.clone(),
        file: args.file.clone(),
        out: None,
        asset: args.asset.clone(),
        format: args.format.clone(),
        sep7: false,
        build_only: false,
        relayer: false,
    };
    wallet_batch_reconcile(&batch_args, &args.report)
}

pub(crate) fn wallet_batch_resume(
    context: &AppContext,
    args: &WalletBatchResumeArgs,
) -> Result<CommandReport> {
    let format = batch_payment_format(&args.file, args.format.as_deref())?;
    let plan = load_batch_payment_plan(&args.file, format, args.asset.as_deref())?;
    let completed_from_report = args
        .report
        .as_ref()
        .map(|path| load_batch_execution_report(path))
        .transpose()?;
    let completed_indices = completed_from_report
        .as_ref()
        .map(|report| {
            report
                .entries
                .iter()
                .map(|entry| entry.index)
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let explicit_skip = args
        .skip
        .iter()
        .copied()
        .map(validate_batch_resume_index)
        .collect::<Result<BTreeSet<_>>>()?;
    let start_at = args
        .start_at
        .map(validate_batch_resume_index)
        .transpose()?
        .unwrap_or(1);
    let selected = plan
        .preview
        .iter()
        .filter(|entry| entry.index >= start_at)
        .filter(|entry| !completed_indices.contains(&entry.index))
        .filter(|entry| !explicit_skip.contains(&entry.index))
        .cloned()
        .collect::<Vec<_>>();

    let mut report = CommandReport::new("wallet.batch-resume");
    if let Some(execution_report) = completed_from_report.as_ref()
        && !execution_report.executed
    {
        report.warnings.push(format!(
            "report `{}` did not include executed payment rows; resume selection only used the recorded indexes",
            args.report
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default()
        ));
    }

    if selected.is_empty() {
        report.status = "warn".to_string();
        report.message = Some("no pending batch payment entries remained to resume".to_string());
        report.data = Some(json!({
            "summary": plan.summary,
            "from": args.from,
            "file": args.file.display().to_string(),
            "format": match format {
                BatchPaymentFormat::Csv => "csv",
                BatchPaymentFormat::Json => "json",
            },
            "default_asset": args.asset,
            "resume": {
                "start_at": start_at,
                "skipped": explicit_skip.into_iter().collect::<Vec<_>>(),
                "completed_from_report": completed_indices.into_iter().collect::<Vec<_>>(),
                "remaining": 0,
            },
        }));
        return Ok(report);
    }

    let selected_indices = selected.iter().map(|entry| entry.index).collect::<Vec<_>>();

    let mut payments = Vec::new();
    for preview in &selected {
        let entry = plan
            .entries
            .get(preview.index - 1)
            .ok_or_else(|| anyhow!("failed to locate resumed batch entry {}", preview.index))?;
        let (asset, asset_source) =
            resolve_batch_payment_asset(entry, args.asset.as_deref(), preview.index - 1)?;
        let child = wallet_pay(
            context,
            &WalletPayArgs {
                from: args.from.clone(),
                to: entry.to.clone(),
                asset: asset.clone(),
                amount: entry.amount.clone(),
                sep7: args.sep7,
                build_only: args.build_only,
                relayer: args.relayer,
                out: None,
            },
        )?;
        report.commands.extend(child.commands);
        report.artifacts.extend(child.artifacts);
        report.warnings.extend(child.warnings);
        payments.push(json!({
            "index": preview.index,
            "to": entry.to.clone(),
            "amount": entry.amount.clone(),
            "asset": asset,
            "asset_source": asset_source,
            "result": child.data,
        }));
    }

    report.message = Some(format!(
        "resumed {} batch payment(s) from `{}`",
        payments.len(),
        args.from
    ));
    report.data = Some(json!({
        "summary": plan.summary,
        "preview": selected,
        "count": payments.len(),
        "from": args.from,
        "file": args.file.display().to_string(),
        "format": match format {
            BatchPaymentFormat::Csv => "csv",
            BatchPaymentFormat::Json => "json",
        },
        "default_asset": args.asset,
        "payments": payments,
        "analysis": batch_payment_analysis_for_preview(&plan.preview),
        "resume": {
            "start_at": start_at,
            "selected": selected_indices,
            "skipped": explicit_skip.into_iter().collect::<Vec<_>>(),
            "completed_from_report": completed_indices.into_iter().collect::<Vec<_>>(),
            "report": args.report.as_ref().map(|path| path.display().to_string()),
        },
    }));
    report.next = vec![format!("stellar forge wallet balances {}", args.from)];
    Ok(report)
}

pub(crate) fn wallet_batch_reconcile(
    args: &WalletBatchPayArgs,
    report_path: &Path,
) -> Result<CommandReport> {
    let format = batch_payment_format(&args.file, args.format.as_deref())?;
    let plan = load_batch_payment_plan(&args.file, format, args.asset.as_deref())?;
    let execution_report = load_batch_execution_report(report_path)?;
    let reported_lookup = execution_report
        .entries
        .iter()
        .map(|entry| (entry.index, entry))
        .collect::<BTreeMap<_, _>>();

    let mut matched = Vec::new();
    let mut missing = Vec::new();
    let mut unexpected = Vec::new();
    let mut mismatches = Vec::new();

    for expected in &plan.preview {
        match reported_lookup.get(&expected.index) {
            Some(actual)
                if expected.to == actual.to
                    && expected.amount == actual.amount
                    && expected.asset == actual.asset =>
            {
                matched.push(expected.index);
            }
            Some(actual) => mismatches.push(json!({
                "index": expected.index,
                "expected": expected,
                "reported": actual,
            })),
            None => missing.push(expected.clone()),
        }
    }

    for actual in &execution_report.entries {
        if !plan.preview.iter().any(|entry| entry.index == actual.index) {
            unexpected.push(json!(actual));
        }
    }

    let mut report = CommandReport::new("wallet.batch-reconcile");
    if !execution_report.executed {
        report.warnings.push(format!(
            "report `{}` did not include executed payments; reconciliation used preview rows only",
            report_path.display()
        ));
    }
    report.status = if missing.is_empty() && unexpected.is_empty() && mismatches.is_empty() {
        "ok".to_string()
    } else {
        "warn".to_string()
    };
    report.message = Some(format!(
        "reconciled {} of {} batch payment entries",
        matched.len(),
        plan.summary.count
    ));
    report.data = Some(json!({
        "summary": plan.summary,
        "from": args.from,
        "file": args.file.display().to_string(),
        "report": {
            "path": report_path.display().to_string(),
            "action": execution_report.action,
            "executed": execution_report.executed,
            "entry_count": execution_report.entries.len(),
        },
        "preview": plan.preview,
        "reported_entries": execution_report.entries,
        "reconcile": {
            "matched_indices": matched,
            "missing_entries": missing,
            "unexpected_entries": unexpected,
            "mismatches": mismatches,
        },
    }));
    if report.status == "warn" {
        report.next = vec![format!(
            "stellar forge wallet batch-resume --from {} --file {} --report {}",
            args.from,
            args.file.display(),
            report_path.display()
        )];
    }
    Ok(report)
}

pub(crate) fn wallet_batch_preview(
    context: &AppContext,
    args: &WalletBatchPayArgs,
) -> Result<CommandReport> {
    batch_payment_report(context, args, BatchPaymentMode::Preview)
}

pub(crate) fn wallet_batch_summary(
    context: &AppContext,
    args: &WalletBatchPayArgs,
) -> Result<CommandReport> {
    batch_payment_report(context, args, BatchPaymentMode::Summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BatchPaymentMode {
    Report,
    Validate,
    Preview,
    Summary,
}

fn batch_payment_report(
    _context: &AppContext,
    args: &WalletBatchPayArgs,
    mode: BatchPaymentMode,
) -> Result<CommandReport> {
    let format = batch_payment_format(&args.file, args.format.as_deref())?;
    let mut plan = load_batch_payment_plan(&args.file, format, args.asset.as_deref())?;
    let action = match mode {
        BatchPaymentMode::Report => "wallet.batch-report",
        BatchPaymentMode::Validate => "wallet.batch-validate",
        BatchPaymentMode::Preview => "wallet.batch-preview",
        BatchPaymentMode::Summary => "wallet.batch-summary",
    };
    plan.summary.kind = match mode {
        BatchPaymentMode::Report => "batch-report",
        BatchPaymentMode::Validate => "batch-validate",
        BatchPaymentMode::Preview => "batch-preview",
        BatchPaymentMode::Summary => "batch-summary",
    };
    let mut report = CommandReport::new(action);
    report.message = Some(match mode {
        BatchPaymentMode::Report => {
            format!("reported {} batch payment entries", plan.summary.count)
        }
        BatchPaymentMode::Validate => {
            format!("validated {} batch payment entries", plan.summary.count)
        }
        BatchPaymentMode::Preview => {
            format!("previewed {} batch payment entries", plan.summary.count)
        }
        BatchPaymentMode::Summary => {
            format!("summarized {} batch payment entries", plan.summary.count)
        }
    });
    report.data = Some(match mode {
        BatchPaymentMode::Summary => json!({
            "summary": plan.summary,
            "from": args.from,
            "file": args.file.display().to_string(),
            "default_asset": args.asset,
        }),
        BatchPaymentMode::Report => json!({
            "summary": plan.summary,
            "preview": plan.preview,
            "from": args.from,
            "file": args.file.display().to_string(),
            "format": match format {
                BatchPaymentFormat::Csv => "csv",
                BatchPaymentFormat::Json => "json",
            },
            "default_asset": args.asset,
            "analysis": batch_payment_analysis(&plan),
        }),
        _ => json!({
            "summary": plan.summary,
            "preview": plan.preview,
            "from": args.from,
            "file": args.file.display().to_string(),
            "format": match format {
                BatchPaymentFormat::Csv => "csv",
                BatchPaymentFormat::Json => "json",
            },
            "default_asset": args.asset,
        }),
    });
    Ok(report)
}

fn batch_payment_analysis(plan: &BatchPaymentPlan) -> Value {
    batch_payment_analysis_for_preview(&plan.preview)
}

fn batch_payment_analysis_for_preview(preview: &[BatchPaymentPreviewEntry]) -> Value {
    let destination_counts =
        batch_payment_value_counts(preview.iter().map(|entry| entry.to.as_str()), "destination");
    let asset_counts =
        batch_payment_value_counts(preview.iter().map(|entry| entry.asset.as_str()), "asset");
    json!({
        "destinations": destination_counts.clone(),
        "assets": asset_counts,
        "duplicate_destinations": destination_counts
            .into_iter()
            .filter(|value| value["count"].as_u64().unwrap_or_default() > 1)
            .collect::<Vec<_>>(),
    })
}

fn batch_payment_value_counts<'a>(
    values: impl Iterator<Item = &'a str>,
    label: &'static str,
) -> Vec<Value> {
    let mut counts = BTreeMap::new();
    for value in values {
        *counts.entry(value.to_string()).or_insert(0usize) += 1;
    }
    counts
        .into_iter()
        .map(|(value, count)| json!({ label: value, "count": count }))
        .collect()
}

fn validate_batch_resume_index(index: usize) -> Result<usize> {
    if index == 0 {
        bail!("batch resume indexes are 1-based and must be greater than zero");
    }
    Ok(index)
}

fn load_batch_execution_report(path: &Path) -> Result<BatchExecutionReport> {
    let value = serde_json::from_str::<Value>(
        &fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?,
    )
    .with_context(|| format!("failed to parse batch report {}", path.display()))?;
    let action = value
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let data = value
        .get("data")
        .ok_or_else(|| anyhow!("batch report `{}` is missing `data`", path.display()))?;
    let payments = data
        .get("payments")
        .and_then(Value::as_array)
        .map(|entries| batch_execution_report_entries(entries))
        .transpose()?;
    let preview = data
        .get("preview")
        .and_then(Value::as_array)
        .map(|entries| batch_execution_report_entries(entries))
        .transpose()?;
    let (executed, entries) = match (payments, preview) {
        (Some(entries), _) => (true, entries),
        (None, Some(entries)) => (false, entries),
        (None, None) => {
            bail!(
                "batch report `{}` did not contain `data.payments` or `data.preview`",
                path.display()
            )
        }
    };
    Ok(BatchExecutionReport {
        action,
        executed,
        entries,
    })
}

fn batch_execution_report_entries(entries: &[Value]) -> Result<Vec<BatchPaymentPreviewEntry>> {
    entries
        .iter()
        .map(|entry| {
            Ok(BatchPaymentPreviewEntry {
                index: entry
                    .get("index")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| anyhow!("batch execution entry is missing `index`"))?
                    as usize,
                to: entry
                    .get("to")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("batch execution entry is missing `to`"))?
                    .to_string(),
                amount: entry
                    .get("amount")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("batch execution entry is missing `amount`"))?
                    .to_string(),
                asset: entry
                    .get("asset")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("batch execution entry is missing `asset`"))?
                    .to_string(),
                asset_source: entry
                    .get("asset_source")
                    .and_then(Value::as_str)
                    .unwrap_or("report")
                    .to_string(),
            })
        })
        .collect()
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
    if let Some(name) = smart_wallet_name_for_input(manifest, input)
        && manifest
            .wallets
            .get(&name)
            .and_then(wallet_smart_contract_id_value)
            .is_none()
    {
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
    update: SmartWalletManifestUpdate<'_>,
) {
    manifest.wallets.insert(
        update.name.to_string(),
        WalletConfig {
            kind: "smart".to_string(),
            identity: update.contract_id.unwrap_or_default().to_string(),
            controller_identity: update.controller_identity.map(str::to_string),
            mode: Some(update.mode.to_string()),
            onboarding_app: Some(update.onboarding_relative.to_string()),
            policy_contract: Some(update.policy_contract.to_string()),
        },
    );
    manifest
        .contracts
        .entry(update.policy_contract.to_string())
        .and_modify(|contract| {
            if !contract
                .deploy_on
                .iter()
                .any(|network| network == update.env)
            {
                contract.deploy_on.push(update.env.to_string());
            }
        })
        .or_insert_with(|| {
            let mut deploy_on = vec!["local".to_string(), "testnet".to_string()];
            if !deploy_on.iter().any(|network| network == update.env) {
                deploy_on.push(update.env.to_string());
            }
            ContractConfig {
                path: update.policy_relative.to_string(),
                alias: update.policy_contract.to_string(),
                template: "passkey-wallet-policy".to_string(),
                bindings: vec!["typescript".to_string()],
                deploy_on,
                init: None,
            }
        });
}

fn write_smart_wallet_scaffold_files(
    context: &AppContext,
    report: &mut CommandReport,
    scaffold: &SmartWalletScaffold<'_>,
) -> Result<()> {
    write_smart_wallet_scaffold_files_with_mode(context, report, scaffold, false)
}

fn write_smart_wallet_scaffold_files_with_mode(
    context: &AppContext,
    report: &mut CommandReport,
    scaffold: &SmartWalletScaffold<'_>,
    refresh_dynamic_files: bool,
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
    let readme = templates::smart_wallet_readme(
        scaffold.name,
        scaffold.mode,
        scaffold.network_name,
        scaffold.policy_contract,
        scaffold.controller_identity,
    );
    let env_example = templates::smart_wallet_env_example(
        scaffold.name,
        scaffold.mode,
        scaffold.network_name,
        scaffold.policy_contract,
        scaffold.controller_identity,
        scaffold.contract_id,
        scaffold.rpc_url,
    );
    let main_ts = templates::smart_wallet_main_ts(
        scaffold.name,
        scaffold.mode,
        scaffold.network_name,
        scaffold.policy_contract,
        scaffold.controller_identity,
        scaffold.contract_id,
        scaffold.rpc_url,
    );
    if refresh_dynamic_files {
        context.write_text(report, &scaffold.onboarding_root.join("README.md"), &readme)?;
        context.write_text(
            report,
            &scaffold.onboarding_root.join(".env.example"),
            &env_example,
        )?;
    } else {
        write_text_if_missing(
            context,
            report,
            &scaffold.onboarding_root.join("README.md"),
            &readme,
        )?;
        write_text_if_missing(
            context,
            report,
            &scaffold.onboarding_root.join(".env.example"),
            &env_example,
        )?;
    }
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
    if refresh_dynamic_files {
        context.write_text(
            report,
            &scaffold.onboarding_root.join("src/main.ts"),
            &main_ts,
        )?;
    } else {
        write_text_if_missing(
            context,
            report,
            &scaffold.onboarding_root.join("src/main.ts"),
            &main_ts,
        )?;
    }
    Ok(())
}

fn smart_wallet_policy_root(root: &Path, manifest: &Manifest, policy_contract: &str) -> PathBuf {
    manifest
        .contracts
        .get(policy_contract)
        .map(|contract| root.join(&contract.path))
        .unwrap_or_else(|| root.join("contracts").join(policy_contract))
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

fn smart_wallet_effective_mode(wallet: &WalletConfig) -> String {
    wallet
        .mode
        .clone()
        .filter(|mode| !mode.trim().is_empty())
        .unwrap_or_else(|| {
            if wallet_controller_identity_value(wallet).is_some() {
                "ed25519".to_string()
            } else {
                "passkey".to_string()
            }
        })
}

fn smart_wallet_configured_paths(
    root: &Path,
    manifest: &Manifest,
    name: &str,
    wallet: &WalletConfig,
) -> (String, PathBuf, String, String, PathBuf) {
    let onboarding_relative = wallet
        .onboarding_app
        .clone()
        .unwrap_or_else(|| format!("apps/smart-wallet/{name}"));
    let onboarding_root = root.join(&onboarding_relative);
    let policy_contract = wallet
        .policy_contract
        .clone()
        .unwrap_or_else(|| format!("{name}-policy"));
    let policy_relative = manifest
        .contracts
        .get(&policy_contract)
        .map(|contract| contract.path.clone())
        .unwrap_or_else(|| format!("contracts/{policy_contract}"));
    let policy_root = root.join(&policy_relative);
    (
        onboarding_relative,
        onboarding_root,
        policy_contract,
        policy_relative,
        policy_root,
    )
}

fn smart_wallet_env_contract_id(context: &AppContext, onboarding_root: &Path) -> Option<String> {
    let env_path = onboarding_root.join(".env.example");
    context.read_text(&env_path).ok().and_then(|contents| {
        parse_env_assignments(&contents)
            .into_iter()
            .find(|(key, value)| {
                key == "SMART_WALLET_CONTRACT_ID" && looks_like_account(value.trim())
            })
            .map(|(_, value)| value)
    })
}

fn smart_wallet_registered_contract_id(
    context: &AppContext,
    onboarding_root: &Path,
    wallet: &WalletConfig,
) -> Option<String> {
    wallet_smart_contract_id_value(wallet)
        .or_else(|| smart_wallet_env_contract_id(context, onboarding_root))
}

fn normalized_wallet_command_output(output: &str) -> Option<String> {
    output
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}

fn find_identity_name_by_address(
    context: &AppContext,
    manifest: &Manifest,
    address: &str,
) -> Option<String> {
    for identity in manifest.identities.keys() {
        if let Ok(resolved) = resolve_address(
            context,
            &mut CommandReport::new("wallet.smart.policy.resolve"),
            Some(manifest),
            identity,
        ) && resolved == address
        {
            return Some(identity.clone());
        }
    }
    None
}

#[derive(Debug, Clone, Serialize)]
struct SmartWalletPolicyObservedState {
    source: String,
    source_address: String,
    target: String,
    deployed: bool,
    admin_address: Option<String>,
    daily_limit: Option<String>,
}

fn smart_wallet_policy_observed_state(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: &Manifest,
    details: &SmartWalletPolicyDetails,
    env: &str,
) -> Result<Option<SmartWalletPolicyObservedState>> {
    let source = smart_wallet_policy_source(manifest, details, None)?;
    let source_address = resolve_address(
        context,
        &mut CommandReport::new("wallet.smart.policy.resolve"),
        Some(manifest),
        &source,
    )?;
    let deployed = details
        .deployment
        .as_ref()
        .is_some_and(|deployment| !deployment.contract_id.is_empty());
    let target = smart_wallet_policy_target(details);
    if !deployed {
        return Ok(Some(SmartWalletPolicyObservedState {
            source,
            source_address,
            target,
            deployed,
            admin_address: None,
            daily_limit: None,
        }));
    }
    if context.globals.dry_run || !context.command_exists("stellar") {
        return Ok(None);
    }

    let mut query = |fn_name: &str| -> Result<Option<String>> {
        let output = context.run_command(
            report,
            Some(&context.project_root()),
            "stellar",
            &[
                "contract".to_string(),
                "invoke".to_string(),
                "--id".to_string(),
                target.clone(),
                "--source-account".to_string(),
                source.clone(),
                "--network".to_string(),
                env.to_string(),
                "--send".to_string(),
                "no".to_string(),
                "--".to_string(),
                fn_name.to_string(),
            ],
        )?;
        Ok(normalized_wallet_command_output(&output))
    };
    let admin_address = query("admin")?;
    let daily_limit = query("daily_limit")?;

    Ok(Some(SmartWalletPolicyObservedState {
        source,
        source_address,
        target,
        deployed,
        admin_address,
        daily_limit,
    }))
}

fn smart_wallet_onboarding_env_values(
    manifest: &Manifest,
    env: &str,
    network: &crate::model::NetworkConfig,
    details: &SmartWalletPolicyDetails,
) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    values.insert("SMART_WALLET_NAME".to_string(), details.wallet_name.clone());
    values.insert(
        "SMART_WALLET_MODE".to_string(),
        smart_wallet_effective_mode(&details.wallet),
    );
    values.insert("SMART_WALLET_NETWORK".to_string(), env.to_string());
    values.insert(
        "SMART_WALLET_POLICY_CONTRACT".to_string(),
        details.policy_contract.clone(),
    );
    if let Some(controller_identity) = details.controller_identity.as_deref() {
        values.insert(
            "SMART_WALLET_CONTROLLER_IDENTITY".to_string(),
            controller_identity.to_string(),
        );
    }
    values.insert(
        "SMART_WALLET_CONTRACT_ID".to_string(),
        wallet_smart_contract_id_value(&details.wallet).unwrap_or_default(),
    );
    values.insert("SMART_WALLET_RPC_URL".to_string(), network.rpc_url.clone());
    if let Ok(active_identity) = manifest.active_identity(None) {
        values.insert(
            "SMART_WALLET_ACTIVE_IDENTITY".to_string(),
            active_identity.to_string(),
        );
    }
    values
}

fn smart_wallet_onboarding_checklist(
    env: &str,
    details: &SmartWalletPolicyDetails,
    policy_deployed: bool,
) -> Vec<String> {
    let mut steps = Vec::new();
    if let Some(controller_identity) = details.controller_identity.as_deref() {
        steps.push(format!(
            "Generate or verify the controller identity `{controller_identity}` for `{env}`."
        ));
        steps.push(format!(
            "Fund `{controller_identity}` on `{env}` before signing provisioning transactions."
        ));
    }
    if policy_deployed {
        steps.push(format!(
            "Use `stellar forge wallet smart policy info {}` to inspect the deployed policy target.",
            details.wallet_name
        ));
    } else {
        steps.push(format!(
            "Build and deploy the `{}` policy contract for `{env}`.",
            details.policy_contract
        ));
    }
    if let Some(contract_id) = wallet_smart_contract_id_value(&details.wallet) {
        steps.push(format!(
            "Confirm `SMART_WALLET_CONTRACT_ID={contract_id}` inside `{}` before handing the app to operators.",
            details.onboarding_relative
        ));
    } else {
        steps.push(format!(
            "Set `SMART_WALLET_CONTRACT_ID` inside `{}` once the contract account is provisioned.",
            details.onboarding_relative
        ));
    }
    steps.push(
        if smart_wallet_effective_mode(&details.wallet) == "ed25519" {
            "Use the generated onboarding console as the operator runbook for controller-signing and provisioning."
                .to_string()
        } else {
            "Use the generated onboarding console to guide the browser WebAuthn ceremony and provisioning flow."
                .to_string()
        },
    );
    steps
}

fn smart_wallet_onboarding_next_steps(
    package_manager: &str,
    env: &str,
    details: &SmartWalletPolicyDetails,
    policy_deployed: bool,
) -> Vec<String> {
    let mut next = Vec::new();
    if let Some(controller_identity) = details.controller_identity.as_deref() {
        next.push(format!("stellar forge wallet fund {controller_identity}"));
        next.push(format!(
            "stellar forge wallet address {controller_identity}"
        ));
    }
    if policy_deployed {
        next.push(format!(
            "stellar forge wallet smart policy info {}",
            details.wallet_name
        ));
    } else {
        next.push(format!(
            "stellar forge contract build {}",
            details.policy_contract
        ));
        next.push(format!(
            "stellar forge contract deploy {} --env {env}",
            details.policy_contract
        ));
    }
    next.push(package_manager_install_command(
        package_manager,
        &details.onboarding_relative,
    ));
    next.push(package_manager_dev_command(
        package_manager,
        &details.onboarding_relative,
    ));
    if wallet_smart_contract_id_value(&details.wallet).is_some() {
        next.push(format!(
            "stellar forge wallet balances {}",
            details.wallet_name
        ));
    }
    next
}

fn wallet_smart_onboard(context: &AppContext, name: &str) -> Result<CommandReport> {
    let mut report = CommandReport::new("wallet.smart.onboard");
    let root = context.project_root();
    let manifest = load_manifest(context)?;
    let lockfile = load_lockfile(context)?;
    let (env, network) = manifest.active_network(context.globals.network.as_deref())?;
    let details = smart_wallet_policy_details(&root, &manifest, &lockfile, env, name)?;
    let onboarding_root = root.join(&details.onboarding_relative);
    let env_path = onboarding_root.join(".env.example");
    let env_values = if env_path.exists() {
        parse_env_assignments(&context.read_text(&env_path)?)
            .into_iter()
            .collect::<BTreeMap<String, String>>()
    } else {
        smart_wallet_onboarding_env_values(&manifest, env, network, &details)
    };
    let policy_deployed = details
        .deployment
        .as_ref()
        .is_some_and(|deployment| !deployment.contract_id.is_empty());

    if !onboarding_root.exists() {
        report.warnings.push(format!(
            "onboarding scaffold `{}` is missing; run `stellar forge wallet smart materialize {}`",
            onboarding_root.display(),
            details.wallet_name
        ));
    }
    if !policy_deployed {
        report.warnings.push(format!(
            "policy contract `{}` is not deployed yet for `{env}`",
            details.policy_contract
        ));
    }
    if smart_wallet_effective_mode(&details.wallet) == "passkey" {
        report.warnings.push(
            "passkey onboarding still requires a browser ceremony and contract-account provisioning flow"
                .to_string(),
        );
    }

    report.network = Some(env.to_string());
    report.message = Some(format!(
        "smart wallet onboarding guide prepared for `{}`",
        details.wallet_name
    ));
    report.next = smart_wallet_onboarding_next_steps(
        &manifest.project.package_manager,
        env,
        &details,
        policy_deployed,
    );
    report.data = Some(json!({
        "wallet": {
            "name": details.wallet_name,
            "mode": smart_wallet_effective_mode(&details.wallet),
            "controller_identity": details.controller_identity,
            "contract_id": wallet_smart_contract_id_value(&details.wallet),
        },
        "environment": {
            "name": env,
            "rpc_url": network.rpc_url,
            "horizon_url": network.horizon_url,
        },
        "paths": {
            "onboarding_app": details.onboarding_relative,
            "onboarding_root": onboarding_root.display().to_string(),
            "env_example": env_path.display().to_string(),
            "policy_contract_root": details.policy_root.display().to_string(),
        },
        "policy_contract": {
            "name": details.policy_contract,
            "deployed": policy_deployed,
            "target": smart_wallet_policy_target(&details),
        },
        "env": env_values,
        "checklist": smart_wallet_onboarding_checklist(env, &details, policy_deployed),
    }));
    Ok(report)
}

fn wallet_smart_provision(
    context: &AppContext,
    name: &str,
    address: Option<&str>,
    fund: bool,
) -> Result<CommandReport> {
    let mut report = CommandReport::new("wallet.smart.provision");
    validate_single_path_segment("smart wallet name", name)?;
    let root = context.project_root();
    let mut manifest = load_manifest(context)?;
    let (env_name, network) = manifest.active_network(context.globals.network.as_deref())?;
    let env = env_name.to_string();
    let rpc_url = network.rpc_url.clone();
    let wallet = manifest
        .wallets
        .get(name)
        .cloned()
        .ok_or_else(|| anyhow!("smart wallet `{name}` not found"))?;
    if wallet.kind != "smart" {
        bail!("wallet `{name}` is not a smart wallet");
    }

    let mode = smart_wallet_effective_mode(&wallet);
    let controller_identity = smart_wallet_controller_identity(name, &mode, Some(&wallet))
        .or_else(|| wallet_controller_identity_value(&wallet));
    if let Some(controller_identity) = controller_identity.as_deref() {
        ensure_smart_wallet_controller_entries(&mut manifest, controller_identity)?;
    }
    let (onboarding_relative, onboarding_root, policy_contract, policy_relative, policy_root) =
        smart_wallet_configured_paths(&root, &manifest, name, &wallet);
    let previous_contract_id =
        smart_wallet_registered_contract_id(context, &onboarding_root, &wallet);
    let contract_id = address
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or(previous_contract_id.clone())
        .or_else(|| smart_wallet_env_contract_id(context, &onboarding_root))
        .ok_or_else(|| {
            anyhow!(
                "smart wallet `{name}` still needs a contract id; pass `--address <contract-id>` or set `SMART_WALLET_CONTRACT_ID` in `{}`",
                onboarding_root.join(".env.example").display()
            )
        })?;
    if !is_contract_address(&contract_id) {
        bail!(
            "smart wallet contract id `{contract_id}` must be a contract address starting with `C`"
        );
    }

    upsert_smart_wallet_manifest_entries(
        &mut manifest,
        SmartWalletManifestUpdate {
            name,
            mode: &mode,
            env: &env,
            onboarding_relative: &onboarding_relative,
            policy_contract: &policy_contract,
            policy_relative: &policy_relative,
            controller_identity: controller_identity.as_deref(),
            contract_id: Some(&contract_id),
        },
    );
    let scaffold = SmartWalletScaffold {
        root: &root,
        name,
        mode: &mode,
        network_name: &env,
        rpc_url: &rpc_url,
        onboarding_root: &onboarding_root,
        policy_contract: &policy_contract,
        policy_root: &policy_root,
        controller_identity: controller_identity.as_deref(),
        contract_id: Some(&contract_id),
    };
    write_smart_wallet_scaffold_files_with_mode(context, &mut report, &scaffold, true)?;
    save_manifest(context, &mut report, &manifest)?;

    let controller_created = if let Some(controller_identity) = controller_identity.as_deref() {
        ensure_identity_exists(
            context,
            &mut report,
            &manifest,
            controller_identity,
            &env,
            fund,
        )?
    } else {
        false
    };
    let lockfile = load_lockfile(context)?;
    let details = smart_wallet_policy_details(&root, &manifest, &lockfile, &env, name)?;
    let policy_deployed = details
        .deployment
        .as_ref()
        .is_some_and(|deployment| !deployment.contract_id.is_empty());
    if !policy_deployed {
        report.warnings.push(format!(
            "policy contract `{policy_contract}` is not deployed yet for `{env}`"
        ));
    }
    if previous_contract_id
        .as_deref()
        .is_some_and(|previous| previous != contract_id)
    {
        report.warnings.push(format!(
            "updated smart wallet `{name}` contract id from `{}` to `{contract_id}`",
            previous_contract_id.clone().unwrap_or_default()
        ));
    }
    report.warnings.push(
        "provision records the smart wallet contract id locally; the on-chain contract account must already exist"
            .to_string(),
    );
    report.network = Some(env.to_string());
    report.message = Some(format!(
        "smart wallet `{name}` provisioned locally for `{env}`"
    ));
    report.next = vec![
        format!("stellar forge wallet balances {name}"),
        format!("stellar forge wallet smart policy diff {name}"),
        format!("stellar forge wallet smart policy info {name}"),
    ];
    report.data = Some(json!({
        "wallet": name,
        "mode": mode,
        "contract_id": contract_id,
        "previous_contract_id": previous_contract_id,
        "controller_identity": controller_identity,
        "controller_created": controller_created,
        "controller_funded": controller_identity.is_some() && fund,
        "onboarding_app": onboarding_relative,
        "policy_contract": {
            "name": policy_contract,
            "target": smart_wallet_policy_target(&details),
            "deployed": policy_deployed,
        },
        "paths": {
            "onboarding": onboarding_root.display().to_string(),
            "policy_contract": policy_root.display().to_string(),
            "env_example": onboarding_root.join(".env.example").display().to_string(),
        },
    }));
    Ok(report)
}

fn wallet_smart_materialize(
    context: &AppContext,
    name: &str,
    fund: bool,
    no_policy_deploy: bool,
) -> Result<CommandReport> {
    let mut report = CommandReport::new("wallet.smart.materialize");
    validate_single_path_segment("smart wallet name", name)?;
    let root = context.project_root();
    let mut manifest = load_manifest(context)?;
    let (env_name, network) = manifest.active_network(context.globals.network.as_deref())?;
    let env = env_name.to_string();
    let rpc_url = network.rpc_url.clone();
    let wallet = manifest
        .wallets
        .get(name)
        .cloned()
        .ok_or_else(|| anyhow!("smart wallet `{name}` not found"))?;
    if wallet.kind != "smart" {
        bail!("wallet `{name}` is not a smart wallet");
    }
    let mode = smart_wallet_effective_mode(&wallet);
    let controller_identity = smart_wallet_controller_identity(name, &mode, Some(&wallet))
        .or_else(|| wallet_controller_identity_value(&wallet));
    if let Some(controller_identity) = controller_identity.as_deref() {
        ensure_smart_wallet_controller_entries(&mut manifest, controller_identity)?;
    }
    let contract_id = wallet_smart_contract_id_value(&wallet);
    let (onboarding_relative, onboarding_root, policy_contract, policy_relative, policy_root) =
        smart_wallet_configured_paths(&root, &manifest, name, &wallet);
    upsert_smart_wallet_manifest_entries(
        &mut manifest,
        SmartWalletManifestUpdate {
            name,
            mode: &mode,
            env: &env,
            onboarding_relative: &onboarding_relative,
            policy_contract: &policy_contract,
            policy_relative: &policy_relative,
            controller_identity: controller_identity.as_deref(),
            contract_id: contract_id.as_deref(),
        },
    );
    let scaffold = SmartWalletScaffold {
        root: &root,
        name,
        mode: &mode,
        network_name: &env,
        rpc_url: &rpc_url,
        onboarding_root: &onboarding_root,
        policy_contract: &policy_contract,
        policy_root: &policy_root,
        controller_identity: controller_identity.as_deref(),
        contract_id: contract_id.as_deref(),
    };
    write_smart_wallet_scaffold_files_with_mode(context, &mut report, &scaffold, true)?;
    save_manifest(context, &mut report, &manifest)?;

    let controller_created = if let Some(controller_identity) = controller_identity.as_deref() {
        ensure_identity_exists(
            context,
            &mut report,
            &manifest,
            controller_identity,
            &env,
            fund,
        )?
    } else {
        false
    };

    let existing_deployment = load_lockfile(context).ok().and_then(|lockfile| {
        lockfile
            .environments
            .get(&env)
            .and_then(|environment| environment.contracts.get(&policy_contract))
            .cloned()
    });
    let mut policy_deployed_now = false;
    if no_policy_deploy {
        report.warnings.push(format!(
            "policy contract deploy skipped for `{}`; rerun without `--no-policy-deploy` to materialize on-chain metadata",
            policy_contract
        ));
    } else if existing_deployment
        .as_ref()
        .is_some_and(|deployment| !deployment.contract_id.is_empty())
    {
        report.warnings.push(format!(
            "policy contract `{policy_contract}` is already deployed for `{env}`"
        ));
    } else {
        deploy_contract_from_manifest(context, &mut report, &manifest, &policy_contract, &env)?;
        policy_deployed_now = true;
    }

    let lockfile = load_lockfile(context)?;
    let details = smart_wallet_policy_details(&root, &manifest, &lockfile, &env, name)?;
    let policy_deployed = details
        .deployment
        .as_ref()
        .is_some_and(|deployment| !deployment.contract_id.is_empty());
    report.network = Some(env.to_string());
    report.message = Some(format!("smart wallet `{name}` materialized for `{env}`"));
    report.next = smart_wallet_onboarding_next_steps(
        &manifest.project.package_manager,
        &env,
        &details,
        policy_deployed,
    );
    report
        .next
        .insert(0, format!("stellar forge wallet smart onboard {name}"));
    report.data = Some(json!({
        "wallet": name,
        "mode": mode,
        "controller_identity": controller_identity,
        "controller_created": controller_created,
        "controller_funded": controller_identity.is_some() && fund,
        "onboarding_app": onboarding_relative,
        "policy_contract": {
            "name": policy_contract,
            "target": smart_wallet_policy_target(&details),
            "deployed": policy_deployed,
            "deployed_now": policy_deployed_now,
        },
        "paths": {
            "onboarding": onboarding_root.display().to_string(),
            "policy_contract": policy_root.display().to_string(),
        },
    }));
    report.warnings.push(
        "smart wallet contract-account provisioning still needs to happen through your onboarding flow"
            .to_string(),
    );
    Ok(report)
}

fn wallet_smart_controller_rotate(
    context: &AppContext,
    name: &str,
    identity: &str,
    fund: bool,
) -> Result<CommandReport> {
    let mut report = CommandReport::new("wallet.smart.controller.rotate");
    validate_single_path_segment("smart wallet name", name)?;
    validate_single_path_segment("controller identity", identity)?;
    let root = context.project_root();
    let mut manifest = load_manifest(context)?;
    let (env_name, network) = manifest.active_network(context.globals.network.as_deref())?;
    let env = env_name.to_string();
    let rpc_url = network.rpc_url.clone();
    let wallet = manifest
        .wallets
        .get(name)
        .cloned()
        .ok_or_else(|| anyhow!("smart wallet `{name}` not found"))?;
    if wallet.kind != "smart" {
        bail!("wallet `{name}` is not a smart wallet");
    }
    let previous_controller = wallet_controller_identity_value(&wallet);
    let mode = smart_wallet_effective_mode(&wallet);
    let contract_id = wallet_smart_contract_id_value(&wallet);
    let (onboarding_relative, onboarding_root, policy_contract, policy_relative, policy_root) =
        smart_wallet_configured_paths(&root, &manifest, name, &wallet);
    ensure_smart_wallet_controller_entries(&mut manifest, identity)?;
    upsert_smart_wallet_manifest_entries(
        &mut manifest,
        SmartWalletManifestUpdate {
            name,
            mode: &mode,
            env: &env,
            onboarding_relative: &onboarding_relative,
            policy_contract: &policy_contract,
            policy_relative: &policy_relative,
            controller_identity: Some(identity),
            contract_id: contract_id.as_deref(),
        },
    );
    let scaffold = SmartWalletScaffold {
        root: &root,
        name,
        mode: &mode,
        network_name: &env,
        rpc_url: &rpc_url,
        onboarding_root: &onboarding_root,
        policy_contract: &policy_contract,
        policy_root: &policy_root,
        controller_identity: Some(identity),
        contract_id: contract_id.as_deref(),
    };
    write_smart_wallet_scaffold_files_with_mode(context, &mut report, &scaffold, true)?;
    save_manifest(context, &mut report, &manifest)?;
    let controller_created =
        ensure_identity_exists(context, &mut report, &manifest, identity, &env, fund)?;
    if let Some(previous_controller) = previous_controller.as_deref()
        && previous_controller != identity
    {
        report.warnings.push(format!(
            "previous controller identity `{previous_controller}` remains declared locally; review whether it should keep access"
        ));
    }

    report.network = Some(env.to_string());
    report.message = Some(format!(
        "controller identity rotated for smart wallet `{name}`"
    ));
    report.next = vec![
        format!("stellar forge wallet smart onboard {name}"),
        format!("stellar forge wallet smart materialize {name}"),
        format!("stellar forge wallet smart policy info {name}"),
    ];
    report.data = Some(json!({
        "wallet": name,
        "mode": mode,
        "previous_controller_identity": previous_controller,
        "controller_identity": identity,
        "controller_created": controller_created,
        "controller_funded": fund,
        "onboarding_app": onboarding_relative,
        "policy_contract": policy_contract,
        "paths": {
            "onboarding": onboarding_root.display().to_string(),
            "policy_contract": policy_root.display().to_string(),
        },
    }));
    Ok(report)
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
    let (env_name, network) = manifest.active_network(context.globals.network.as_deref())?;
    let env = env_name.to_string();
    let rpc_url = network.rpc_url.clone();
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
        SmartWalletManifestUpdate {
            name,
            mode: mode_name,
            env: &env,
            onboarding_relative: &onboarding_relative,
            policy_contract: &policy_contract,
            policy_relative: &policy_relative,
            controller_identity: controller_identity.as_deref(),
            contract_id: None,
        },
    );
    write_smart_wallet_scaffold_files(
        context,
        &mut report,
        &SmartWalletScaffold {
            root: &root,
            name,
            mode: mode_name,
            network_name: &env,
            rpc_url: &rpc_url,
            onboarding_root: &onboarding_root,
            policy_contract: &policy_contract,
            policy_root: &policy_root,
            controller_identity: controller_identity.as_deref(),
            contract_id: None,
        },
    )?;
    save_manifest(context, &mut report, &manifest)?;

    if let Some(controller_identity) = controller_identity.as_deref() {
        ensure_identity_exists(
            context,
            &mut report,
            &manifest,
            controller_identity,
            &env,
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
        &env,
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
    let (env_name, network) = manifest.active_network(context.globals.network.as_deref())?;
    let env = env_name.to_string();
    let rpc_url = network.rpc_url.clone();
    if let Some(existing) = manifest.wallets.get(name)
        && existing.kind != "smart"
    {
        bail!("wallet `{name}` already exists as a classic wallet");
    }

    let (onboarding_relative, onboarding_root, policy_contract, policy_relative, policy_root) =
        smart_wallet_paths(&root, name);
    upsert_smart_wallet_manifest_entries(
        &mut manifest,
        SmartWalletManifestUpdate {
            name,
            mode: "passkey",
            env: &env,
            onboarding_relative: &onboarding_relative,
            policy_contract: &policy_contract,
            policy_relative: &policy_relative,
            controller_identity: None,
            contract_id: None,
        },
    );
    write_smart_wallet_scaffold_files(
        context,
        &mut report,
        &SmartWalletScaffold {
            root: &root,
            name,
            mode: "passkey",
            network_name: &env,
            rpc_url: &rpc_url,
            onboarding_root: &onboarding_root,
            policy_contract: &policy_contract,
            policy_root: &policy_root,
            controller_identity: None,
            contract_id: None,
        },
    )?;
    save_manifest(context, &mut report, &manifest)?;

    report.message = Some(format!(
        "smart wallet onboarding scaffold created at {}",
        onboarding_root.display()
    ));
    report.next = smart_wallet_next_steps(
        &manifest.project.package_manager,
        &onboarding_relative,
        &policy_contract,
        &env,
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
    let policy_root = smart_wallet_policy_root(&root, &manifest, &policy_contract);
    let contract_id = wallet
        .as_ref()
        .and_then(|wallet| smart_wallet_registered_contract_id(context, &onboarding_root, wallet));
    let materialized = contract_id.is_some();
    report.message = Some(format!("smart wallet scaffold info for `{name}`"));
    report.next = if contract_id.is_some() {
        vec![
            format!("stellar forge wallet balances {name}"),
            format!("stellar forge wallet smart policy diff {name}"),
            format!("stellar forge wallet smart policy info {name}"),
        ]
    } else {
        let mut next = vec![
            format!("stellar forge wallet smart provision {name} --address <contract-id>"),
            format!("stellar forge wallet smart onboard {name}"),
            format!("stellar forge wallet smart materialize {name}"),
        ];
        next.extend(smart_wallet_next_steps(
            &manifest.project.package_manager,
            &onboarding_relative,
            &policy_contract,
            &manifest.defaults.network,
        ));
        next
    };
    report.data = Some(json!({
        "wallet": wallet,
        "controller_identity": controller_identity,
        "contract_id": contract_id,
        "materialized": materialized,
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

fn wallet_smart_policy_info(context: &AppContext, name: &str) -> Result<CommandReport> {
    let mut report = CommandReport::new("wallet.smart.policy.info");
    let manifest = load_manifest(context)?;
    let lockfile = load_lockfile(context)?;
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    let details =
        smart_wallet_policy_details(&context.project_root(), &manifest, &lockfile, &env, name)?;
    let default_source = smart_wallet_policy_source(&manifest, &details, None)?;
    let target_id = smart_wallet_policy_target(&details);
    let deployed = details
        .deployment
        .as_ref()
        .is_some_and(|deployment| !deployment.contract_id.is_empty());
    if !deployed {
        report.warnings.push(format!(
            "policy contract `{}` is not deployed in `{env}` yet; deploy it before sending policy mutations",
            details.policy_contract
        ));
    }
    report.network = Some(env.clone());
    report.message = Some(format!("smart wallet policy info for `{name}`"));
    report.next = vec![
        format!("stellar forge wallet smart info {name}"),
        format!("stellar forge wallet smart policy diff {name}"),
        format!("stellar forge wallet smart policy sync {name}"),
        format!("stellar forge wallet smart policy set-daily-limit {name} 1000 --build-only"),
        format!(
            "stellar forge wallet smart policy allow {name} {} --build-only",
            default_source
        ),
    ];
    report.data = Some(json!({
        "wallet": {
            "name": details.wallet_name,
            "mode": details.wallet.mode.clone(),
            "controller_identity": details.controller_identity,
            "onboarding_app": details.onboarding_relative,
        },
        "policy_contract": {
            "name": details.policy_contract,
            "path": details.policy_root.display().to_string(),
            "target": target_id,
            "deployed": deployed,
            "deployment": details.deployment,
        },
        "default_source": default_source,
        "functions": ["admin", "daily_limit", "is_allowed", "set_daily_limit", "allow", "revoke"],
    }));
    Ok(report)
}

fn wallet_smart_policy_diff(context: &AppContext, name: &str) -> Result<CommandReport> {
    let mut report = CommandReport::new("wallet.smart.policy.diff");
    let manifest = load_manifest(context)?;
    let lockfile = load_lockfile(context)?;
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    let details =
        smart_wallet_policy_details(&context.project_root(), &manifest, &lockfile, &env, name)?;
    let local_controller_identity = details.controller_identity.clone();
    let local_controller_address = local_controller_identity
        .as_deref()
        .map(|identity| {
            resolve_address(
                context,
                &mut CommandReport::new("wallet.smart.policy.resolve"),
                Some(&manifest),
                identity,
            )
        })
        .transpose()?;
    let contract_id = smart_wallet_registered_contract_id(
        context,
        &context.project_root().join(&details.onboarding_relative),
        &details.wallet,
    );
    let observed =
        smart_wallet_policy_observed_state(context, &mut report, &manifest, &details, &env)?;
    let mut issues = Vec::new();
    if let Some(observed) = observed.as_ref() {
        if !observed.deployed {
            issues.push(format!(
                "policy contract `{}` is not deployed in `{env}` yet",
                details.policy_contract
            ));
        }
        match (
            local_controller_address.as_deref(),
            observed.admin_address.as_deref(),
        ) {
            (Some(expected), Some(actual)) if expected != actual => issues.push(format!(
                "controller drift: local controller `{}` resolves to `{expected}` but on-chain admin is `{actual}`",
                local_controller_identity.as_deref().unwrap_or_default()
            )),
            (Some(_), None) if observed.deployed => issues.push(
                "could not read the on-chain admin value from the deployed policy".to_string(),
            ),
            _ => {}
        }
    } else if context.globals.dry_run || !context.command_exists("stellar") {
        report.warnings.push(
            "skipped on-chain policy probes; rerun without `--dry-run` on a machine with `stellar` configured"
                .to_string(),
        );
    }
    report.status = if issues.is_empty() { "ok" } else { "warn" }.to_string();
    report.network = Some(env.clone());
    report.message = Some(format!("smart wallet policy drift analyzed for `{name}`"));
    report.next = vec![
        format!("stellar forge wallet smart policy sync {name}"),
        format!("stellar forge wallet smart policy info {name}"),
        format!("stellar forge wallet smart info {name}"),
    ];
    report.data = Some(json!({
        "wallet": {
            "name": details.wallet_name,
            "mode": details.wallet.mode.clone(),
            "controller_identity": local_controller_identity,
            "controller_address": local_controller_address,
            "contract_id": contract_id,
        },
        "policy_contract": {
            "name": details.policy_contract,
            "target": smart_wallet_policy_target(&details),
            "deployed": details
                .deployment
                .as_ref()
                .is_some_and(|deployment| !deployment.contract_id.is_empty()),
            "deployment": details.deployment,
        },
        "observed": observed,
        "drift": {
            "controller_aligned": issues.is_empty(),
            "issues": issues,
        },
    }));
    Ok(report)
}

fn wallet_smart_policy_sync(context: &AppContext, name: &str) -> Result<CommandReport> {
    let mut report = CommandReport::new("wallet.smart.policy.sync");
    let root = context.project_root();
    let mut manifest = load_manifest(context)?;
    let lockfile = load_lockfile(context)?;
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    let rpc_url = manifest
        .networks
        .get(&env)
        .map(|network| network.rpc_url.clone())
        .unwrap_or_default();
    let details = smart_wallet_policy_details(&root, &manifest, &lockfile, &env, name)?;
    let previous_controller_identity = details.controller_identity.clone();
    let observed =
        smart_wallet_policy_observed_state(context, &mut report, &manifest, &details, &env)?;

    let Some(observed) = observed else {
        report.status = "warn".to_string();
        report.network = Some(env.clone());
        report.message = Some(format!(
            "smart wallet policy sync needs a live policy probe for `{name}`"
        ));
        report.warnings.push(
            "on-chain policy reads were skipped; rerun without `--dry-run` on a machine with `stellar` configured"
                .to_string(),
        );
        report.next = vec![
            format!("stellar forge wallet smart policy diff {name}"),
            format!("stellar forge wallet smart policy info {name}"),
        ];
        report.data = Some(json!({
            "wallet": name,
            "previous_controller_identity": previous_controller_identity,
            "synced": false,
        }));
        return Ok(report);
    };

    if !observed.deployed {
        report.status = "warn".to_string();
        report.network = Some(env.clone());
        report.message = Some(format!(
            "smart wallet policy sync skipped for undeployed `{name}`"
        ));
        report.warnings.push(format!(
            "policy contract `{}` is not deployed in `{env}` yet",
            details.policy_contract
        ));
        report.next = vec![
            format!(
                "stellar forge contract deploy {} --env {env}",
                details.policy_contract
            ),
            format!("stellar forge wallet smart policy diff {name}"),
        ];
        report.data = Some(json!({
            "wallet": name,
            "previous_controller_identity": previous_controller_identity,
            "observed": observed,
            "synced": false,
        }));
        return Ok(report);
    }

    let matched_identity = observed
        .admin_address
        .as_deref()
        .and_then(|address| find_identity_name_by_address(context, &manifest, address));
    let current_wallet = manifest
        .wallets
        .get(name)
        .cloned()
        .ok_or_else(|| anyhow!("smart wallet `{name}` not found"))?;
    let mode = smart_wallet_effective_mode(&current_wallet);
    let (onboarding_relative, onboarding_root, policy_contract, policy_relative, policy_root) =
        smart_wallet_configured_paths(&root, &manifest, name, &current_wallet);
    let contract_id =
        smart_wallet_registered_contract_id(context, &onboarding_root, &current_wallet);

    let mut synced = false;
    if let Some(controller_identity) = matched_identity.as_deref() {
        ensure_smart_wallet_controller_entries(&mut manifest, controller_identity)?;
        if previous_controller_identity.as_deref() != Some(controller_identity) {
            upsert_smart_wallet_manifest_entries(
                &mut manifest,
                SmartWalletManifestUpdate {
                    name,
                    mode: &mode,
                    env: &env,
                    onboarding_relative: &onboarding_relative,
                    policy_contract: &policy_contract,
                    policy_relative: &policy_relative,
                    controller_identity: Some(controller_identity),
                    contract_id: contract_id.as_deref(),
                },
            );
            let scaffold = SmartWalletScaffold {
                root: &root,
                name,
                mode: &mode,
                network_name: &env,
                rpc_url: &rpc_url,
                onboarding_root: &onboarding_root,
                policy_contract: &policy_contract,
                policy_root: &policy_root,
                controller_identity: Some(controller_identity),
                contract_id: contract_id.as_deref(),
            };
            write_smart_wallet_scaffold_files_with_mode(context, &mut report, &scaffold, true)?;
            save_manifest(context, &mut report, &manifest)?;
            synced = true;
        }
    } else if let Some(admin_address) = observed.admin_address.as_deref() {
        report.warnings.push(format!(
            "on-chain admin `{admin_address}` does not match any declared local identity; controller metadata was left unchanged"
        ));
    } else {
        report.warnings.push(
            "could not determine the on-chain admin from the deployed policy; controller metadata was left unchanged"
                .to_string(),
        );
    }

    report.status = if report.warnings.is_empty() {
        "ok"
    } else {
        "warn"
    }
    .to_string();
    report.network = Some(env.clone());
    report.message = Some(if synced {
        format!("smart wallet policy metadata synchronized for `{name}`")
    } else {
        format!("smart wallet policy metadata already aligned for `{name}`")
    });
    report.next = vec![
        format!("stellar forge wallet smart policy diff {name}"),
        format!("stellar forge wallet smart policy info {name}"),
        format!("stellar forge wallet smart info {name}"),
    ];
    report.data = Some(json!({
        "wallet": name,
        "previous_controller_identity": previous_controller_identity,
        "controller_identity": matched_identity,
        "observed": observed,
        "synced": synced,
        "contract_id": contract_id,
    }));
    Ok(report)
}

fn wallet_smart_policy_apply(
    context: &AppContext,
    name: &str,
    file: &Path,
    source: Option<&str>,
    build_only: bool,
) -> Result<CommandReport> {
    let mut report = CommandReport::new("wallet.smart.policy.apply");
    let policy = load_smart_wallet_policy_apply_file(file)?;
    let effective_source = source.or(policy.source.as_deref());
    let effective_build_only = build_only || policy.build_only.unwrap_or(false);
    let mut operations = Vec::new();
    let mut resolved_source = effective_source.map(ToOwned::to_owned);

    if let Some(daily_limit) = policy.daily_limit.as_ref() {
        let normalized_amount = smart_wallet_policy_value_string(daily_limit)?;
        let child = wallet_smart_policy_set_daily_limit(
            context,
            name,
            &normalized_amount,
            effective_source,
            effective_build_only,
        )?;
        resolved_source = resolved_source.or_else(|| smart_wallet_policy_child_source(&child));
        operations.push(json!({
            "type": "set_daily_limit",
            "daily_limit": normalized_amount,
            "result": merge_smart_wallet_policy_child(&mut report, child),
        }));
    }

    for address in &policy.allow {
        let child = wallet_smart_policy_access_update(
            context,
            name,
            "allow",
            address,
            effective_source,
            effective_build_only,
        )?;
        resolved_source = resolved_source.or_else(|| smart_wallet_policy_child_source(&child));
        operations.push(json!({
            "type": "allow",
            "address": address,
            "result": merge_smart_wallet_policy_child(&mut report, child),
        }));
    }

    for address in &policy.revoke {
        let child = wallet_smart_policy_access_update(
            context,
            name,
            "revoke",
            address,
            effective_source,
            effective_build_only,
        )?;
        resolved_source = resolved_source.or_else(|| smart_wallet_policy_child_source(&child));
        operations.push(json!({
            "type": "revoke",
            "address": address,
            "result": merge_smart_wallet_policy_child(&mut report, child),
        }));
    }

    if operations.is_empty() {
        bail!(
            "policy file `{}` did not declare any `daily_limit`, `allow`, or `revoke` operations",
            file.display()
        );
    }

    report.message = Some(format!(
        "smart wallet policy {} from `{}` for `{name}`",
        if effective_build_only {
            "prepared"
        } else {
            "applied"
        },
        file.display()
    ));
    report.next = vec![
        format!("stellar forge wallet smart policy info {name}"),
        format!("stellar forge wallet smart policy diff {name}"),
    ];
    report.data = Some(json!({
        "wallet": name,
        "file": file.display().to_string(),
        "source": resolved_source,
        "build_only": effective_build_only,
        "daily_limit": policy
            .daily_limit
            .as_ref()
            .map(smart_wallet_policy_value_string)
            .transpose()?,
        "allow": policy.allow,
        "revoke": policy.revoke,
        "operation_count": operations.len(),
        "operations": operations,
    }));
    Ok(report)
}

fn wallet_smart_policy_simulate(
    context: &AppContext,
    name: &str,
    file: &Path,
    source: Option<&str>,
) -> Result<CommandReport> {
    let mut report = wallet_smart_policy_apply(context, name, file, source, true)?;
    report.action = "wallet.smart.policy.simulate".to_string();
    report.message = Some(format!(
        "smart wallet policy simulated from `{}` for `{name}`",
        file.display()
    ));
    if let Some(data) = report.data.as_mut().and_then(Value::as_object_mut) {
        data.insert("simulated".to_string(), json!(true));
    }
    Ok(report)
}

fn merge_smart_wallet_policy_child(
    report: &mut CommandReport,
    child: CommandReport,
) -> Option<Value> {
    report.commands.extend(child.commands);
    report.artifacts.extend(child.artifacts);
    report.warnings.extend(child.warnings);
    if child.network.is_some() {
        report.network = child.network;
    }
    child.data
}

fn smart_wallet_policy_child_source(child: &CommandReport) -> Option<String> {
    child
        .data
        .as_ref()
        .and_then(|data| data.get("source"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn wallet_smart_policy_set_daily_limit(
    context: &AppContext,
    name: &str,
    amount: &str,
    source: Option<&str>,
    build_only: bool,
) -> Result<CommandReport> {
    let normalized_amount = amount
        .parse::<i128>()
        .with_context(|| format!("invalid policy daily limit `{amount}`"))?
        .to_string();
    let mut report = wallet_smart_policy_invoke(
        context,
        name,
        "set_daily_limit",
        &[("daily_limit".to_string(), normalized_amount.clone())],
        source,
        build_only,
    )?;
    report.action = "wallet.smart.policy.set-daily-limit".to_string();
    report.message = Some(format!(
        "smart wallet policy daily limit {} for `{name}`",
        if build_only { "prepared" } else { "updated" }
    ));
    if let Some(data) = report.data.as_mut().and_then(Value::as_object_mut) {
        data.insert("daily_limit".to_string(), json!(normalized_amount));
    }
    Ok(report)
}

fn wallet_smart_policy_access_update(
    context: &AppContext,
    name: &str,
    operation: &str,
    address: &str,
    source: Option<&str>,
    build_only: bool,
) -> Result<CommandReport> {
    let manifest = load_manifest(context)?;
    let resolved_address = resolve_address(
        context,
        &mut CommandReport::new("wallet.smart.policy.resolve"),
        Some(&manifest),
        address,
    )?;
    let mut report = wallet_smart_policy_invoke(
        context,
        name,
        operation,
        &[("address".to_string(), resolved_address.clone())],
        source,
        build_only,
    )?;
    report.action = format!("wallet.smart.policy.{operation}");
    report.message = Some(format!(
        "smart wallet policy `{operation}` {} for `{name}`",
        if build_only { "prepared" } else { "updated" }
    ));
    if let Some(data) = report.data.as_mut().and_then(Value::as_object_mut) {
        data.insert("address".to_string(), json!(resolved_address));
    }
    Ok(report)
}

fn wallet_smart_policy_invoke(
    context: &AppContext,
    name: &str,
    fn_name: &str,
    arguments: &[(String, String)],
    source: Option<&str>,
    build_only: bool,
) -> Result<CommandReport> {
    let mut report = CommandReport::new(format!("wallet.smart.policy.{fn_name}"));
    let manifest = load_manifest(context)?;
    let lockfile = load_lockfile(context)?;
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    let details =
        smart_wallet_policy_details(&context.project_root(), &manifest, &lockfile, &env, name)?;
    let source_identity = smart_wallet_policy_source(&manifest, &details, source)?;
    let target = smart_wallet_policy_target(&details);
    let deployed = details
        .deployment
        .as_ref()
        .is_some_and(|deployment| !deployment.contract_id.is_empty());
    if !deployed && !build_only && !context.globals.dry_run {
        bail!(
            "policy contract `{}` is not deployed in `{env}`; deploy it before sending `{fn_name}`",
            details.policy_contract
        );
    }
    if !deployed {
        report.warnings.push(format!(
            "policy contract `{}` is not deployed in `{env}` yet; using `{}` for preview/build-only output",
            details.policy_contract, target
        ));
    }
    ensure_identity_exists(
        context,
        &mut report,
        &manifest,
        &source_identity,
        &env,
        false,
    )?;
    let mut command_args = vec![
        "contract".to_string(),
        "invoke".to_string(),
        "--id".to_string(),
        target.clone(),
        "--source-account".to_string(),
        source_identity.clone(),
        "--network".to_string(),
        env.clone(),
        "--send".to_string(),
        if build_only { "no" } else { "yes" }.to_string(),
        "--".to_string(),
        fn_name.to_string(),
    ];
    for (key, value) in arguments {
        command_args.push(format!("--{key}"));
        command_args.push(value.clone());
    }
    let output = context.run_command(
        &mut report,
        Some(&context.project_root()),
        "stellar",
        &command_args,
    )?;
    report.network = Some(env.clone());
    report.data = Some(json!({
        "wallet": name,
        "function": fn_name,
        "source": source_identity,
        "policy_contract": details.policy_contract,
        "target": target,
        "build_only": build_only,
        "deployment": details.deployment,
        "arguments": arguments
            .iter()
            .map(|(key, value)| json!({ "name": key, "value": value }))
            .collect::<Vec<_>>(),
        "result": if output.is_empty() { Value::Null } else { Value::String(output) },
    }));
    report.next = vec![format!("stellar forge wallet smart policy info {name}")];
    Ok(report)
}

fn smart_wallet_policy_details(
    root: &Path,
    manifest: &Manifest,
    lockfile: &Lockfile,
    env: &str,
    name: &str,
) -> Result<SmartWalletPolicyDetails> {
    let wallet = manifest
        .wallets
        .get(name)
        .cloned()
        .ok_or_else(|| anyhow!("smart wallet `{name}` not found"))?;
    if wallet.kind != "smart" {
        bail!("wallet `{name}` is not a smart wallet");
    }
    let (onboarding_relative, _, policy_contract, _, policy_root) =
        smart_wallet_configured_paths(root, manifest, name, &wallet);
    Ok(SmartWalletPolicyDetails {
        wallet_name: name.to_string(),
        controller_identity: wallet.controller_identity.clone(),
        policy_root,
        onboarding_relative,
        deployment: lockfile
            .environments
            .get(env)
            .and_then(|environment| environment.contracts.get(&policy_contract))
            .cloned(),
        policy_contract,
        wallet,
    })
}

fn smart_wallet_policy_source(
    manifest: &Manifest,
    details: &SmartWalletPolicyDetails,
    override_source: Option<&str>,
) -> Result<String> {
    if let Some(source) = override_source {
        let resolved =
            resolve_identity_name(Some(manifest), source).unwrap_or_else(|| source.to_string());
        validate_single_path_segment("identity or wallet name", &resolved)?;
        return Ok(resolved);
    }
    if let Some(controller_identity) = details.controller_identity.as_deref() {
        validate_single_path_segment("identity or wallet name", controller_identity)?;
        return Ok(controller_identity.to_string());
    }
    let active = manifest
        .active_identity(None)
        .unwrap_or(&manifest.defaults.identity)
        .to_string();
    validate_single_path_segment("identity or wallet name", &active)?;
    Ok(active)
}

fn smart_wallet_policy_target(details: &SmartWalletPolicyDetails) -> String {
    details
        .deployment
        .as_ref()
        .map(|deployment| deployment.contract_id.clone())
        .filter(|contract_id| !contract_id.is_empty())
        .unwrap_or_else(|| details.policy_contract.clone())
}

fn load_smart_wallet_policy_apply_file(path: &Path) -> Result<SmartWalletPolicyApplyFile> {
    let contents =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("toml"))
    {
        toml::from_str(&contents)
            .with_context(|| format!("failed to parse policy file {}", path.display()))
    } else {
        serde_json::from_str(&contents)
            .with_context(|| format!("failed to parse policy file {}", path.display()))
    }
}

fn smart_wallet_policy_value_string(value: &SmartWalletPolicyValue) -> Result<String> {
    match value {
        SmartWalletPolicyValue::String(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                bail!("policy daily limit cannot be empty");
            }
            Ok(trimmed.to_string())
        }
        SmartWalletPolicyValue::Signed(value) => Ok(value.to_string()),
        SmartWalletPolicyValue::Unsigned(value) => Ok(value.to_string()),
    }
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

fn batch_payment_format(path: &Path, explicit: Option<&str>) -> Result<BatchPaymentFormat> {
    let normalized = explicit
        .map(|value| value.trim().to_ascii_lowercase())
        .or_else(|| {
            path.extension()
                .and_then(|value| value.to_str())
                .map(|value| value.trim().to_ascii_lowercase())
        })
        .unwrap_or_else(|| "json".to_string());
    match normalized.as_str() {
        "csv" => Ok(BatchPaymentFormat::Csv),
        "json" => Ok(BatchPaymentFormat::Json),
        _ => bail!("unsupported batch payment format `{normalized}`; use `json` or `csv`"),
    }
}

fn load_batch_payment_entries(
    path: &Path,
    format: BatchPaymentFormat,
) -> Result<Vec<BatchPaymentEntry>> {
    match format {
        BatchPaymentFormat::Csv => load_csv_batch_payment_entries(path),
        BatchPaymentFormat::Json => load_json_batch_payment_entries(path),
    }
}

fn load_json_batch_payment_entries(path: &Path) -> Result<Vec<BatchPaymentEntry>> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read batch payment file {}", path.display()))?;
    let value = serde_json::from_str::<Value>(&raw)
        .with_context(|| format!("invalid JSON in {}", path.display()))?;
    let entries_value = value
        .get("payments")
        .cloned()
        .unwrap_or_else(|| value.clone());
    let entries = serde_json::from_value::<Vec<BatchPaymentEntry>>(entries_value)
        .with_context(|| format!("expected an array of payments in {}", path.display()))?;
    validate_batch_payment_entries(entries)
}

fn load_csv_batch_payment_entries(path: &Path) -> Result<Vec<BatchPaymentEntry>> {
    let mut reader = ReaderBuilder::new()
        .trim(Trim::All)
        .from_path(path)
        .with_context(|| format!("failed to read CSV batch payment file {}", path.display()))?;
    let entries = reader
        .deserialize::<BatchPaymentCsvEntry>()
        .map(|row| {
            row.map(|entry| BatchPaymentEntry {
                to: entry.to,
                amount: entry.amount,
                asset: if entry.asset.trim().is_empty() {
                    None
                } else {
                    Some(entry.asset)
                },
            })
        })
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("invalid CSV batch payment data in {}", path.display()))?;
    validate_batch_payment_entries(entries)
}

fn validate_batch_payment_entries(
    entries: Vec<BatchPaymentEntry>,
) -> Result<Vec<BatchPaymentEntry>> {
    let mut normalized = Vec::with_capacity(entries.len());
    for (index, mut entry) in entries.into_iter().enumerate() {
        entry.to = entry.to.trim().to_string();
        entry.amount = entry.amount.trim().to_string();
        entry.asset = entry
            .asset
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        if entry.to.is_empty() {
            bail!("batch payment entry {} is missing `to`", index + 1);
        }
        if entry.amount.is_empty() {
            bail!("batch payment entry {} is missing `amount`", index + 1);
        }
        normalized.push(entry);
    }
    Ok(normalized)
}

fn load_batch_payment_plan(
    path: &Path,
    format: BatchPaymentFormat,
    default_asset: Option<&str>,
) -> Result<BatchPaymentPlan> {
    let entries = load_batch_payment_entries(path, format)?;
    if entries.is_empty() {
        bail!(
            "batch payment file `{}` did not contain any entries",
            path.display()
        );
    }

    let default_asset = default_asset
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let mut preview = Vec::with_capacity(entries.len());
    let mut unique_destinations = BTreeSet::new();
    let mut unique_assets = BTreeSet::new();
    let mut explicit_assets = 0;
    let mut inferred_assets = 0;

    for (index, entry) in entries.iter().enumerate() {
        let (asset, asset_source) =
            resolve_batch_payment_asset(entry, default_asset.as_deref(), index)?;
        if asset_source == "entry" {
            explicit_assets += 1;
        } else {
            inferred_assets += 1;
        }
        unique_destinations.insert(entry.to.clone());
        unique_assets.insert(asset.clone());
        preview.push(BatchPaymentPreviewEntry {
            index: index + 1,
            to: entry.to.clone(),
            amount: entry.amount.clone(),
            asset,
            asset_source: asset_source.to_string(),
        });
    }

    Ok(BatchPaymentPlan {
        summary: BatchPaymentSummary {
            kind: "batch-pay",
            file: path.display().to_string(),
            format: match format {
                BatchPaymentFormat::Csv => "csv",
                BatchPaymentFormat::Json => "json",
            },
            count: entries.len(),
            default_asset,
            explicit_assets,
            inferred_assets,
            unique_destinations: unique_destinations.len(),
            unique_assets: unique_assets.len(),
        },
        entries,
        preview,
    })
}

fn resolve_batch_payment_asset(
    entry: &BatchPaymentEntry,
    default_asset: Option<&str>,
    index: usize,
) -> Result<(String, &'static str)> {
    if let Some(asset) = entry
        .asset
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok((asset.to_string(), "entry"));
    }
    if let Some(default_asset) = default_asset {
        return Ok((default_asset.to_string(), "default"));
    }
    bail!(
        "batch payment entry {} is missing `asset`; pass `--asset` or include an `asset` field",
        index + 1
    )
}
