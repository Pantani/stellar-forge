use super::*;
use crate::cli::{EventsExportArgs, EventsReplayArgs};

pub(super) fn events_command(
    context: &AppContext,
    command: EventsCommand,
) -> Result<CommandReport> {
    let out = events_command_output_path(&command);
    let mut report = match command {
        EventsCommand::Status(_) => events_status(context),
        EventsCommand::Export(args) => events_export(context, &args),
        EventsCommand::Replay(args) => events_replay(context, &args),
        EventsCommand::Watch(args) => events_watch(context, &args),
        EventsCommand::Ingest(args) => match args.command {
            EventsIngestCommand::Init(_) => events_ingest_init(context),
        },
        EventsCommand::Cursor(args) => match args.command {
            EventsCursorCommand::Ls(_) => events_cursor_ls(context),
            EventsCursorCommand::Reset { name, .. } => events_cursor_reset(context, &name),
        },
        EventsCommand::Backfill(args) => events_backfill(context, &args),
    }?;
    if let Some(path) = out.as_deref() {
        persist_report_output(context, &mut report, path)?;
    }
    Ok(report)
}

fn events_command_output_path(command: &EventsCommand) -> Option<PathBuf> {
    match command {
        EventsCommand::Status(args) => args.out.clone(),
        EventsCommand::Export(args) => args.out.clone(),
        EventsCommand::Replay(args) => args.out.clone(),
        EventsCommand::Watch(args) => args.out.clone(),
        EventsCommand::Ingest(args) => match &args.command {
            EventsIngestCommand::Init(args) => args.out.clone(),
        },
        EventsCommand::Cursor(args) => match &args.command {
            EventsCursorCommand::Ls(args) => args.out.clone(),
            EventsCursorCommand::Reset { out, .. } => out.clone(),
        },
        EventsCommand::Backfill(args) => args.out.clone(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventsExportFile {
    version: u32,
    exported_at: String,
    project_root: String,
    network: String,
    store: EventsExportStore,
    cursors: EventsExportSection<EventsExportCursorRow>,
    events: EventsExportSection<EventsExportEventRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventsExportStore {
    backend: String,
    database: String,
    db_path: String,
    schema_path: String,
    snapshot_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventsExportSection<T> {
    source: String,
    count: usize,
    rows: Vec<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventsExportCursorRow {
    name: String,
    resource_kind: String,
    resource_name: String,
    cursor: Option<String>,
    last_ledger: Option<i64>,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventsExportEventRow {
    external_id: String,
    cursor_name: String,
    cursor: Option<String>,
    resource_kind: String,
    resource_name: String,
    contract_id: String,
    event_type: String,
    topic: String,
    payload: String,
    tx_hash: Option<String>,
    ledger: Option<i64>,
    observed_at: String,
}

fn events_status(context: &AppContext) -> Result<CommandReport> {
    let mut report = CommandReport::new("events.status");
    let manifest = load_manifest(context)?;
    let root = context.project_root();
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    let paths = event_store_paths(&root);
    let env_values = load_event_env_values(&root, &paths.api_root);
    let backend = manifest
        .api
        .as_ref()
        .map(|api| api.events_backend.clone())
        .unwrap_or_else(|| "rpc-poller".to_string());
    let database = manifest
        .api
        .as_ref()
        .map(|api| api.database.clone())
        .unwrap_or_else(|| "sqlite".to_string());
    let worker_resources = parse_event_env_list(env_values.get("STELLAR_EVENTS_RESOURCES"));
    let tracked_resources = tracked_event_resources(&manifest, &worker_resources);
    let worker_topics = parse_event_env_list(env_values.get("STELLAR_EVENTS_TOPICS"));
    let worker_type = env_values
        .get("STELLAR_EVENTS_TYPE")
        .cloned()
        .unwrap_or_else(|| "all".to_string());
    let retention_days = env_values
        .get("STELLAR_EVENTS_RETENTION_DAYS")
        .and_then(|value| parse_event_env_u64(value))
        .or_else(|| (backend == "rpc-poller").then_some(7));

    report.checks.push(check(
        "events:api-root",
        if paths.api_root.exists() {
            "ok"
        } else {
            "warn"
        },
        Some(paths.api_root.display().to_string()),
    ));
    report.checks.push(check(
        "events:schema",
        if paths.schema_path.exists() {
            "ok"
        } else {
            "warn"
        },
        Some(paths.schema_path.display().to_string()),
    ));
    report.checks.push(check(
        "events:db",
        if paths.db_path.exists() || paths.snapshot_path.exists() {
            "ok"
        } else {
            "warn"
        },
        Some(paths.db_path.display().to_string()),
    ));
    report.checks.push(check(
        "events:snapshot",
        if paths.snapshot_path.exists() || paths.db_path.exists() {
            "ok"
        } else {
            "warn"
        },
        Some(paths.snapshot_path.display().to_string()),
    ));
    report.checks.push(check(
        "events:sqlite3",
        if context.command_exists("sqlite3") {
            "ok"
        } else {
            "warn"
        },
        Some(if context.command_exists("sqlite3") {
            "available".to_string()
        } else {
            "not installed".to_string()
        }),
    ));
    if let Some(config_check) = event_worker_config_check(&root, &manifest, true, "events:config") {
        report.checks.push(config_check);
    }

    let snapshot = load_event_cursors(&root)?;
    let sqlite_rows = match load_sqlite_event_cursors(context, &mut report, &root) {
        Ok(rows) => rows,
        Err(error) => {
            report
                .checks
                .push(check("events:cursors", "warn", Some(error.to_string())));
            report
                .warnings
                .push(format!("failed to inspect sqlite event cursors: {error}"));
            None
        }
    };

    let (cursor_source, cursors, cursor_names, cursor_count) = if let Some(rows) = sqlite_rows {
        let mut cursor_names = rows.iter().map(|row| row.name.clone()).collect::<Vec<_>>();
        cursor_names.sort();
        (
            "sqlite",
            cursor_rows_to_value(&rows)["cursors"].clone(),
            cursor_names,
            rows.len(),
        )
    } else {
        let mut cursor_names = snapshot
            .get("cursors")
            .and_then(Value::as_object)
            .map(|cursors| cursors.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        cursor_names.sort();
        (
            "snapshot",
            snapshot
                .get("cursors")
                .cloned()
                .unwrap_or_else(|| json!({})),
            cursor_names.clone(),
            cursor_names.len(),
        )
    };

    let (total_events, latest_ledger, latest_observed_at) =
        match load_event_store_summary(context, &mut report, &paths.db_path) {
            Ok(summary) => summary,
            Err(error) => {
                report
                    .checks
                    .push(check("events:store", "warn", Some(error.to_string())));
                report
                    .warnings
                    .push(format!("failed to inspect persisted event rows: {error}"));
                (0, None, None)
            }
        };

    report.status = aggregate_status(&report.checks);
    report.network = Some(env.clone());
    report.message = Some(format!("summarized event store status for `{env}`"));
    report.next = events_status_next(&paths, &tracked_resources);
    report.data = Some(json!({
        "backend": backend,
        "database": database,
        "db_path": paths.db_path.display().to_string(),
        "schema_path": paths.schema_path.display().to_string(),
        "snapshot_path": paths.snapshot_path.display().to_string(),
        "contracts": manifest.contracts.keys().cloned().collect::<Vec<_>>(),
        "tokens": manifest.tokens.keys().cloned().collect::<Vec<_>>(),
        "tracked_resources": tracked_resources,
        "worker": {
            "resources": worker_resources,
            "topics": worker_topics,
            "type": worker_type,
            "poll_interval_ms": env_values
                .get("STELLAR_EVENTS_POLL_INTERVAL_MS")
                .and_then(|value| parse_event_env_u64(value)),
            "batch_size": env_values
                .get("STELLAR_EVENTS_BATCH_SIZE")
                .and_then(|value| parse_event_env_u64(value)),
            "start_ledger": env_values
                .get("STELLAR_EVENTS_START_LEDGER")
                .and_then(|value| parse_event_env_u64(value)),
            "retention_days": retention_days,
        },
        "retention_days": retention_days,
        "retention_warning": retention_days.map(|days| format!(
            "RPC/event retention is short; backfill older than {days} day(s) requires your own archive or indexer."
        )),
        "source": cursor_source,
        "total_events": total_events,
        "latest_ledger": latest_ledger,
        "latest_observed_at": latest_observed_at,
        "cursor_count": cursor_count,
        "cursor_names": cursor_names,
        "cursors": cursors,
    }));
    Ok(report)
}

pub(super) fn events_export(
    context: &AppContext,
    args: &EventsExportArgs,
) -> Result<CommandReport> {
    let mut report = CommandReport::new("events.export");
    let manifest = load_manifest(context)?;
    let root = context.project_root();
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    let export_path = args
        .path
        .clone()
        .unwrap_or_else(|| root.join("dist").join(format!("events.{env}.json")));
    let paths = event_store_paths(&root);
    let backend = manifest
        .api
        .as_ref()
        .map(|api| api.events_backend.clone())
        .unwrap_or_else(|| "rpc-poller".to_string());
    let database = manifest
        .api
        .as_ref()
        .map(|api| api.database.clone())
        .unwrap_or_else(|| "sqlite".to_string());
    let (cursor_source, cursor_rows) = load_event_cursor_rows(context, &mut report, &root)?;
    let (event_source, event_rows) = load_event_rows(context, &mut report, &paths.db_path)?;
    let export = EventsExportFile {
        version: 1,
        exported_at: Utc::now().to_rfc3339(),
        project_root: root.display().to_string(),
        network: env.clone(),
        store: EventsExportStore {
            backend: backend.clone(),
            database: database.clone(),
            db_path: paths.db_path.display().to_string(),
            schema_path: paths.schema_path.display().to_string(),
            snapshot_path: paths.snapshot_path.display().to_string(),
        },
        cursors: EventsExportSection {
            source: cursor_source.clone(),
            count: cursor_rows.len(),
            rows: cursor_rows.iter().map(export_cursor_row).collect(),
        },
        events: EventsExportSection {
            source: event_source.clone(),
            count: event_rows.len(),
            rows: event_rows,
        },
    };

    if event_source == "unavailable" {
        report.warnings.push(
            "event rows could not be exported because `sqlite3` is unavailable; the cursor data was still captured"
                .to_string(),
        );
    }

    context.write_text(
        &mut report,
        &export_path,
        &serde_json::to_string_pretty(&export)?,
    )?;
    report.message = Some(format!(
        "exported {} cursor(s) and {} event row(s) to {}",
        export.cursors.count,
        export.events.count,
        export_path.display()
    ));
    report.network = Some(env);
    report.data = Some(json!({
        "version": export.version,
        "file": export_path.display().to_string(),
        "project_root": export.project_root,
        "store": export.store,
        "cursors": {
            "source": export.cursors.source,
            "count": export.cursors.count,
        },
        "events": {
            "source": export.events.source,
            "count": export.events.count,
        },
    }));
    Ok(report)
}

pub(super) fn events_replay(
    context: &AppContext,
    args: &EventsReplayArgs,
) -> Result<CommandReport> {
    let mut report = CommandReport::new("events.replay");
    let manifest = load_manifest(context)?;
    let root = context.project_root();
    let env = manifest
        .active_network(context.globals.network.as_deref())?
        .0
        .to_string();
    let paths = event_store_paths(&root);
    let replay_path = args
        .path
        .clone()
        .unwrap_or_else(|| root.join("dist").join(format!("events.{env}.json")));
    let raw = context.read_text(&replay_path)?;
    let export: EventsExportFile = serde_json::from_str(&raw)?;

    if export.version != 1 {
        bail!(
            "unsupported event export version {}; expected version 1",
            export.version
        );
    }

    let mut local_cursor_rows = load_local_event_cursor_rows(context, &mut report, &root)?;
    let imported_cursor_rows = export
        .cursors
        .rows
        .iter()
        .map(import_cursor_row)
        .collect::<Vec<_>>();
    local_cursor_rows = merge_event_cursor_rows(local_cursor_rows, imported_cursor_rows);

    if export.events.count > 0 && !context.command_exists("sqlite3") && !context.globals.dry_run {
        bail!(
            "`sqlite3` is required to replay {} event row(s) from {}",
            export.events.count,
            replay_path.display()
        );
    }

    let sqlite_available = context.command_exists("sqlite3") || context.globals.dry_run;
    let mut sqlite_rows = Vec::new();
    if sqlite_available {
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

        let replay_sql = build_events_replay_sql(&export.events.rows, &local_cursor_rows);
        if !replay_sql.trim().is_empty() {
            sqlite_exec(context, &mut report, &paths.db_path, &replay_sql)?;
        }
        sqlite_rows = load_sqlite_event_cursors(context, &mut report, &root)?.unwrap_or_default();
    } else if paths.db_path.exists() {
        report.warnings.push(format!(
            "event database exists at {} but `sqlite3` is unavailable, so replay will update only the cursor snapshot",
            paths.db_path.display()
        ));
    }

    let replay_cursor_source = if sqlite_rows.is_empty() {
        "snapshot"
    } else {
        "sqlite"
    };
    let snapshot_rows = if sqlite_rows.is_empty() {
        local_cursor_rows
    } else {
        sqlite_rows
    };
    write_cursor_snapshot(context, &mut report, &paths.snapshot_path, &snapshot_rows)?;
    sync_frontend_for_event_change(context, &mut report, &root, &manifest, &env)?;

    report.message = Some(format!(
        "replayed {} cursor(s) and {} event row(s) from {}",
        snapshot_rows.len(),
        export.events.count,
        replay_path.display()
    ));
    report.network = Some(env);
    report.data = Some(json!({
        "version": export.version,
        "file": replay_path.display().to_string(),
        "project_root": export.project_root,
        "store": export.store,
        "cursors": {
            "count": snapshot_rows.len(),
            "source": replay_cursor_source,
        },
        "events": {
            "count": export.events.count,
            "source": export.events.source,
        },
        "db_path": paths.db_path.display().to_string(),
        "snapshot_path": paths.snapshot_path.display().to_string(),
    }));
    Ok(report)
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

fn parse_event_env_list(value: Option<&String>) -> Vec<String> {
    value
        .map(String::as_str)
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_event_env_u64(value: &str) -> Option<u64> {
    value.trim().parse::<u64>().ok().filter(|value| *value > 0)
}

fn tracked_event_resources(manifest: &Manifest, filters: &[String]) -> Vec<String> {
    let declared = manifest
        .contracts
        .keys()
        .map(|name| format!("contract:{name}"))
        .chain(manifest.tokens.keys().map(|name| format!("token:{name}")))
        .collect::<Vec<_>>();
    if filters.is_empty() {
        return declared;
    }
    declared
        .into_iter()
        .filter(|resource| {
            let name = resource
                .split_once(':')
                .map(|(_, name)| name)
                .unwrap_or_default();
            filters
                .iter()
                .any(|filter| filter == resource || filter == name)
        })
        .collect()
}

fn load_event_store_summary(
    context: &AppContext,
    report: &mut CommandReport,
    db_path: &Path,
) -> Result<(i64, Option<i64>, Option<String>)> {
    #[derive(Debug, Deserialize)]
    struct EventStoreSummaryRow {
        total_events: Option<i64>,
        latest_ledger: Option<i64>,
        latest_observed_at: Option<String>,
    }

    if !db_path.exists() || !context.command_exists("sqlite3") {
        return Ok((0, None, None));
    }

    let rows = sqlite_query_json(
        context,
        report,
        db_path,
        "select count(*) as total_events, max(ledger) as latest_ledger, max(observed_at) as latest_observed_at from events;",
    )?;
    let summary = serde_json::from_value::<Vec<EventStoreSummaryRow>>(rows)?
        .into_iter()
        .next()
        .unwrap_or(EventStoreSummaryRow {
            total_events: Some(0),
            latest_ledger: None,
            latest_observed_at: None,
        });
    Ok((
        summary.total_events.unwrap_or_default(),
        summary.latest_ledger,
        summary.latest_observed_at,
    ))
}

fn events_status_next(paths: &EventStorePaths, tracked_resources: &[String]) -> Vec<String> {
    if !paths.api_root.exists() {
        return vec!["stellar forge events ingest init".to_string()];
    }

    let mut next = vec!["stellar forge events cursor ls".to_string()];
    if let Some(resource) = tracked_resources.first() {
        next.push(format!("stellar forge events backfill {resource}"));
    }
    next
}

fn load_event_cursor_rows(
    context: &AppContext,
    report: &mut CommandReport,
    root: &Path,
) -> Result<(String, Vec<EventCursorRow>)> {
    let snapshot = load_event_cursors(root)?;
    let snapshot_rows = snapshot_cursor_rows(&snapshot);
    let sqlite_rows = load_sqlite_event_cursors(context, report, root)?.unwrap_or_default();
    let source = match (sqlite_rows.is_empty(), snapshot_rows.is_empty()) {
        (false, false) => "sqlite+snapshot",
        (false, true) => "sqlite",
        (true, false) => "snapshot",
        (true, true) => "snapshot",
    }
    .to_string();

    Ok((source, merge_event_cursor_rows(sqlite_rows, snapshot_rows)))
}

fn load_local_event_cursor_rows(
    context: &AppContext,
    report: &mut CommandReport,
    root: &Path,
) -> Result<Vec<EventCursorRow>> {
    let snapshot = load_event_cursors(root)?;
    let snapshot_rows = snapshot_cursor_rows(&snapshot);
    let sqlite_rows = load_sqlite_event_cursors(context, report, root)?.unwrap_or_default();
    Ok(merge_event_cursor_rows(sqlite_rows, snapshot_rows))
}

fn load_event_rows(
    context: &AppContext,
    report: &mut CommandReport,
    db_path: &Path,
) -> Result<(String, Vec<EventsExportEventRow>)> {
    if !db_path.exists() || !context.command_exists("sqlite3") {
        return Ok(("unavailable".to_string(), Vec::new()));
    }

    let rows = sqlite_query_json(
        context,
        report,
        db_path,
        "select external_id, cursor_name, cursor, resource_kind, resource_name, contract_id, event_type, topic, payload, tx_hash, ledger, observed_at from events order by coalesce(ledger, 0) asc, external_id asc;",
    )?;
    let parsed = serde_json::from_value::<Vec<EventsExportEventRow>>(rows)?;
    Ok(("sqlite".to_string(), parsed))
}

fn snapshot_cursor_rows(snapshot: &Value) -> Vec<EventCursorRow> {
    let mut rows = snapshot
        .get("cursors")
        .and_then(Value::as_object)
        .map(|cursors| {
            cursors
                .keys()
                .filter_map(|name| snapshot_cursor_row(snapshot, name))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    rows.sort_by(|left, right| left.name.cmp(&right.name));
    rows
}

fn merge_event_cursor_rows(
    mut primary: Vec<EventCursorRow>,
    secondary: Vec<EventCursorRow>,
) -> Vec<EventCursorRow> {
    let mut merged = BTreeMap::new();
    for row in primary.drain(..) {
        merged.insert(row.name.clone(), row);
    }
    for row in secondary {
        let key = row.name.clone();
        match merged.get_mut(&key) {
            Some(current) if should_replace_event_cursor_row(current, &row) => {
                *current = row;
            }
            Some(_) => {}
            None => {
                merged.insert(key, row);
            }
        }
    }
    let mut rows = merged.into_values().collect::<Vec<_>>();
    rows.sort_by(|left, right| left.name.cmp(&right.name));
    rows
}

fn should_replace_event_cursor_row(current: &EventCursorRow, candidate: &EventCursorRow) -> bool {
    match (current.last_ledger, candidate.last_ledger) {
        (Some(left), Some(right)) if left != right => right > left,
        (None, Some(_)) => true,
        (Some(_), None) => false,
        _ => candidate.updated_at >= current.updated_at,
    }
}

fn export_cursor_row(row: &EventCursorRow) -> EventsExportCursorRow {
    EventsExportCursorRow {
        name: row.name.clone(),
        resource_kind: row.resource_kind.clone(),
        resource_name: row.resource_name.clone(),
        cursor: row.cursor.clone(),
        last_ledger: row.last_ledger,
        updated_at: row.updated_at.clone(),
    }
}

fn import_cursor_row(row: &EventsExportCursorRow) -> EventCursorRow {
    EventCursorRow {
        name: row.name.clone(),
        resource_kind: row.resource_kind.clone(),
        resource_name: row.resource_name.clone(),
        cursor: row.cursor.clone(),
        last_ledger: row.last_ledger,
        updated_at: row.updated_at.clone(),
    }
}

fn build_events_replay_sql(events: &[EventsExportEventRow], cursors: &[EventCursorRow]) -> String {
    let mut sql = String::from("begin;\n");
    for event in events {
        sql.push_str(&format!(
            "insert or ignore into events (external_id, cursor_name, cursor, resource_kind, resource_name, contract_id, event_type, topic, payload, tx_hash, ledger, observed_at) values ({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {});\n",
            sqlite_quote(&event.external_id),
            sqlite_quote(&event.cursor_name),
            sqlite_nullable_string(event.cursor.as_deref()),
            sqlite_quote(&event.resource_kind),
            sqlite_quote(&event.resource_name),
            sqlite_quote(&event.contract_id),
            sqlite_quote(&event.event_type),
            sqlite_quote(&event.topic),
            sqlite_quote(&event.payload),
            sqlite_nullable_string(event.tx_hash.as_deref()),
            sqlite_nullable_number(event.ledger),
            sqlite_quote(&event.observed_at),
        ));
    }
    for cursor in cursors {
        sql.push_str(&format!(
            "insert into cursors (name, resource_kind, resource_name, cursor, last_ledger, updated_at) values ({}, {}, {}, {}, {}, {}) on conflict(name) do update set resource_kind = excluded.resource_kind, resource_name = excluded.resource_name, cursor = excluded.cursor, last_ledger = excluded.last_ledger, updated_at = excluded.updated_at;\n",
            sqlite_quote(&cursor.name),
            sqlite_quote(&cursor.resource_kind),
            sqlite_quote(&cursor.resource_name),
            sqlite_nullable_string(cursor.cursor.as_deref()),
            sqlite_nullable_number(cursor.last_ledger),
            sqlite_quote(&cursor.updated_at),
        ));
    }
    sql.push_str("commit;\n");
    sql
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
