use super::*;

pub(super) fn events_command(
    context: &AppContext,
    command: EventsCommand,
) -> Result<CommandReport> {
    match command {
        EventsCommand::Watch(args) => events_watch(context, &args),
        EventsCommand::Ingest(args) => match args.command {
            EventsIngestCommand::Init => events_ingest_init(context),
        },
        EventsCommand::Cursor(args) => match args.command {
            EventsCursorCommand::Ls => events_cursor_ls(context),
            EventsCursorCommand::Reset { name } => events_cursor_reset(context, &name),
        },
        EventsCommand::Backfill(args) => events_backfill(context, &args),
    }
}

fn events_watch(context: &AppContext, args: &EventsWatchArgs) -> Result<CommandReport> {
    let mut report = CommandReport::new("events.watch");
    let manifest = load_manifest(context)?;
    let lockfile = load_lockfile(context)?;
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    let query = event_query_options(args.count, &args.cursor, args.start_ledger, &args.topics)?;
    let id = match args.kind.as_str() {
        "contract" => resolve_contract_id(&manifest, &lockfile, &env, &args.resource),
        "token" => {
            let token = lockfile
                .environments
                .get(&env)
                .and_then(|environment| environment.tokens.get(&args.resource))
                .ok_or_else(|| {
                    anyhow!(
                        "token `{}` has no materialized deployment in `{env}`",
                        args.resource
                    )
                })?;
            if token.sac_contract_id.is_empty() {
                bail!(
                    "token `{}` does not have a deployed SAC; use `stellar forge token sac deploy {}` first",
                    args.resource,
                    args.resource
                );
            }
            token.sac_contract_id.clone()
        }
        "account" => {
            let network = manifest
                .networks
                .get(&env)
                .ok_or_else(|| anyhow!("network `{env}` not found"))?;
            populate_account_watch_report(
                context,
                &mut report,
                &AccountWatchParams {
                    manifest: &manifest,
                    env: &env,
                    network,
                    resource: &args.resource,
                    query: &query,
                    topics: &args.topics,
                },
            )?;
            return Ok(report);
        }
        _ => bail!("unsupported event resource kind `{}`", args.kind),
    };
    let mut command_args = vec![
        "events".to_string(),
        "--network".to_string(),
        env.clone(),
        "--output".to_string(),
        if context.globals.json {
            "json".to_string()
        } else {
            "pretty".to_string()
        },
    ];
    append_event_query_args(&mut command_args, &id, &query, None);
    context.run_command(
        &mut report,
        Some(&context.project_root()),
        "stellar",
        &command_args,
    )?;
    report.message = Some(format!(
        "watching `{}` events for `{}`",
        args.kind, args.resource
    ));
    report.network = Some(env);
    report.data = Some(json!({
        "kind": args.kind,
        "resource": args.resource,
        "contract_id": id,
        "topics": args.topics,
        "resolved_topics": query.topics,
        "count": args.count,
        "cursor": args.cursor,
        "start_ledger": args.start_ledger,
    }));
    Ok(report)
}

fn events_ingest_init(context: &AppContext) -> Result<CommandReport> {
    let mut report = api_events_init(context)?;
    report.action = "events.ingest.init".to_string();
    Ok(report)
}

fn events_cursor_ls(context: &AppContext) -> Result<CommandReport> {
    let mut report = CommandReport::new("events.cursor.ls");
    let root = context.project_root();
    let paths = event_store_paths(&root);
    let value = if let Some(rows) = load_sqlite_event_cursors(context, &mut report, &root)? {
        report.message = Some(format!(
            "listed persisted event cursors from {}",
            paths.db_path.display()
        ));
        json!({
            "source": "sqlite",
            "db_path": paths.db_path.display().to_string(),
            "cursors": cursor_rows_to_value(&rows)["cursors"].clone(),
        })
    } else {
        let snapshot = load_event_cursors(&root)?;
        report.message = Some("listed persisted event cursors".to_string());
        json!({
            "source": "snapshot",
            "path": paths.snapshot_path.display().to_string(),
            "cursors": snapshot.get("cursors").cloned().unwrap_or_else(|| json!({})),
        })
    };
    report.data = Some(value);
    Ok(report)
}

fn events_cursor_reset(context: &AppContext, name: &str) -> Result<CommandReport> {
    let mut report = CommandReport::new("events.cursor.reset");
    let manifest = load_manifest(context)?;
    let root = context.project_root();
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    let paths = event_store_paths(&root);
    let sqlite_rows = if paths.db_path.exists() {
        if !context.command_exists("sqlite3") && !context.globals.dry_run {
            bail!(
                "`sqlite3` is required to reset persisted event cursors in {}; install it or remove the database first",
                paths.db_path.display()
            );
        }
        sqlite_exec(
            context,
            &mut report,
            &paths.db_path,
            &format!("delete from cursors where name = {};", sqlite_quote(name)),
        )?;
        load_sqlite_event_cursors(context, &mut report, &root)?.unwrap_or_default()
    } else {
        Vec::new()
    };

    let mut snapshot = load_event_cursors(&root)?;
    if let Some(cursors) = snapshot.get_mut("cursors").and_then(Value::as_object_mut) {
        cursors.remove(name);
    }

    if !sqlite_rows.is_empty() || paths.db_path.exists() {
        write_cursor_snapshot(context, &mut report, &paths.snapshot_path, &sqlite_rows)?;
    } else {
        context.write_text(
            &mut report,
            &paths.snapshot_path,
            &serde_json::to_string_pretty(&snapshot)?,
        )?;
    }

    sync_frontend_for_event_change(context, &mut report, &root, &manifest, &env)?;
    report.message = Some(format!("cursor `{name}` cleared"));
    report.data = Some(json!({
        "source": if paths.db_path.exists() { "sqlite" } else { "snapshot" },
        "name": name,
        "db_path": paths.db_path.display().to_string(),
        "snapshot_path": paths.snapshot_path.display().to_string(),
    }));
    Ok(report)
}

pub(super) fn clear_event_state_for_env(
    context: &AppContext,
    report: &mut CommandReport,
    manifest: &Manifest,
    env: &str,
) -> Result<bool> {
    let root = context.project_root();
    let paths = event_store_paths(&root);
    let prefix = format!("{env}:");
    let like_pattern = format!("{env}:%");
    let mut cleared = false;

    if paths.db_path.exists() {
        if !context.command_exists("sqlite3") && !context.globals.dry_run {
            report.warnings.push(format!(
                "event database exists at {} but `sqlite3` is unavailable, so event rows for `{env}` could not be cleared automatically",
                paths.db_path.display()
            ));
        } else {
            sqlite_exec(
                context,
                report,
                &paths.db_path,
                &format!(
                    "delete from events where cursor_name like {}; delete from cursors where name like {};",
                    sqlite_quote(&like_pattern),
                    sqlite_quote(&like_pattern),
                ),
            )?;
            cleared = true;
        }
    }

    let mut snapshot = load_event_cursors(&root)?;
    if let Some(cursors) = snapshot.get_mut("cursors").and_then(Value::as_object_mut) {
        let before = cursors.len();
        cursors.retain(|name, _| !name.starts_with(&prefix));
        cleared |= cursors.len() != before;
    }

    if context.globals.dry_run || !paths.db_path.exists() || !context.command_exists("sqlite3") {
        context.write_text(
            report,
            &paths.snapshot_path,
            &serde_json::to_string_pretty(&snapshot)?,
        )?;
    } else {
        let rows = load_sqlite_event_cursors(context, report, &root)?.unwrap_or_default();
        write_cursor_snapshot(context, report, &paths.snapshot_path, &rows)?;
    }

    sync_frontend_for_event_change(context, report, &root, manifest, env)?;
    Ok(cleared)
}

fn events_backfill(context: &AppContext, args: &EventsBackfillArgs) -> Result<CommandReport> {
    let mut report = CommandReport::new("events.backfill");
    let manifest = load_manifest(context)?;
    let lockfile = load_lockfile(context)?;
    let root = context.project_root();
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    let query = event_query_options(args.count, &args.cursor, args.start_ledger, &args.topics)?;
    if let Some(account_name) = account_event_resource_name(&manifest, &args.resource) {
        populate_account_backfill_report(
            context,
            &mut report,
            &AccountBackfillParams {
                manifest: &manifest,
                root: &root,
                env: &env,
                resource: &account_name,
                query: &query,
                topics: &args.topics,
            },
        )?;
        return Ok(report);
    }
    let resolved = resolve_event_resource(&manifest, &lockfile, &env, &args.resource)?;
    let cursor_name = format!("{}:{}:{}", env, resolved.kind, resolved.name);
    let paths = event_store_paths(&root);

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

    context.ensure_dir(&mut report, &paths.api_root.join("db"))?;
    if !paths.schema_path.exists() {
        report.warnings.push(format!(
            "event schema was missing and has been recreated at {}",
            paths.schema_path.display()
        ));
        context.write_text(
            &mut report,
            &paths.schema_path,
            templates::api_events_schema(),
        )?;
    }

    sqlite_exec(
        context,
        &mut report,
        &paths.db_path,
        templates::api_events_schema(),
    )?;

    let current_snapshot = load_event_cursors(&root)?;
    let current_cursor = load_sqlite_event_cursor(context, &mut report, &root, &cursor_name)?
        .or_else(|| snapshot_cursor_row(&current_snapshot, &cursor_name))
        .unwrap_or_else(|| EventCursorRow {
            name: cursor_name.clone(),
            resource_kind: resolved.kind.clone(),
            resource_name: resolved.name.clone(),
            cursor: None,
            last_ledger: None,
            updated_at: Utc::now().to_rfc3339(),
        });

    let mut command_args = vec![
        "events".to_string(),
        "--output".to_string(),
        "json".to_string(),
        "--network".to_string(),
        env.clone(),
    ];
    let fallback_cursor = current_cursor
        .cursor
        .as_ref()
        .filter(|cursor| !cursor.is_empty())
        .cloned();
    let fallback_start_ledger = if query.cursor.is_none() && query.start_ledger.is_none() {
        current_cursor
            .last_ledger
            .and_then(|ledger| u64::try_from(ledger).ok())
    } else {
        None
    };
    let effective_query = EventQueryOptions {
        count: query.count,
        cursor: query.cursor.clone().or(fallback_cursor),
        start_ledger: query.start_ledger.or(fallback_start_ledger),
        topics: query.topics.clone(),
    };
    append_event_query_args(
        &mut command_args,
        &resolved.contract_id,
        &effective_query,
        Some(200),
    );

    if context.globals.dry_run {
        context.run_command(&mut report, Some(&root), "stellar", &command_args)?;
        report.message = Some(format!(
            "planned a bounded RPC backfill for `{}` into {}",
            resolved.name,
            paths.db_path.display()
        ));
        report.warnings.push("backfill on public RPC is retention-bound; use this to seed recent history, not as a long-term archive".to_string());
        report.data = Some(json!({
            "resource": {
                "kind": resolved.kind,
                "name": resolved.name,
                "contract_id": resolved.contract_id,
            },
            "cursor_name": cursor_name,
            "topics": args.topics,
            "resolved_topics": effective_query.topics,
            "count": effective_query.count.or(Some(200)),
            "cursor": effective_query.cursor,
            "start_ledger": effective_query.start_ledger,
            "db_path": paths.db_path.display().to_string(),
            "schema_path": paths.schema_path.display().to_string(),
            "snapshot_path": paths.snapshot_path.display().to_string(),
        }));
        return Ok(report);
    }

    let raw = context.run_command(&mut report, Some(&root), "stellar", &command_args)?;
    let raw_value = if raw.trim().is_empty() {
        json!([])
    } else {
        serde_json::from_str::<Value>(&raw)?
    };
    let events = extract_event_rows(&raw_value)
        .into_iter()
        .map(|event| normalize_backfill_event(event, &resolved, &cursor_name))
        .collect::<Vec<_>>();

    if events.is_empty() {
        report.status = "warn".to_string();
        report.message = Some(format!(
            "no events found for `{}` inside the current RPC retention window",
            resolved.name
        ));
        report.warnings.push(
            "public RPC history is short-lived; older ledgers may no longer be queryable"
                .to_string(),
        );
        report.data = Some(json!({
            "resource": {
                "kind": resolved.kind,
                "name": resolved.name,
                "contract_id": resolved.contract_id,
            },
            "event_count": 0,
        }));
        return Ok(report);
    }

    let Some(last_event) = events.last().cloned() else {
        return Ok(report);
    };
    let sql = build_backfill_sql(
        &events,
        &EventCursorRow {
            name: cursor_name.clone(),
            resource_kind: resolved.kind.clone(),
            resource_name: resolved.name.clone(),
            cursor: last_event.cursor.clone(),
            last_ledger: last_event.ledger,
            updated_at: Utc::now().to_rfc3339(),
        },
    );
    sqlite_exec(context, &mut report, &paths.db_path, &sql)?;

    let rows = load_sqlite_event_cursors(context, &mut report, &root)?.unwrap_or_default();
    write_cursor_snapshot(context, &mut report, &paths.snapshot_path, &rows)?;
    sync_frontend_for_event_change(context, &mut report, &root, &manifest, &env)?;

    report.message = Some(format!(
        "imported {} recent event(s) for `{}`",
        events.len(),
        resolved.name
    ));
    report.warnings.push("backfill on public RPC is retention-bound; persist this database if you need history beyond the provider window".to_string());
    report.data = Some(json!({
        "resource": {
            "kind": resolved.kind,
            "name": resolved.name,
            "contract_id": resolved.contract_id,
        },
        "cursor_name": cursor_name,
        "event_count": events.len(),
        "latest_ledger": last_event.ledger,
        "topics": args.topics,
        "resolved_topics": effective_query.topics,
        "count": effective_query.count.or(Some(200)),
        "cursor": effective_query.cursor,
        "start_ledger": effective_query.start_ledger,
        "db_path": paths.db_path.display().to_string(),
        "snapshot_path": paths.snapshot_path.display().to_string(),
    }));
    Ok(report)
}
