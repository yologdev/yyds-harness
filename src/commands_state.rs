//! Shell subcommands for the yoagent-state-backed shadow log.

use crate::format::*;
use crate::state::{EvalResult, HarnessPatch};
use rusqlite::{params, Connection};
use serde_json::Value;
use std::collections::VecDeque;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub(crate) fn default_events_path() -> PathBuf {
    let (config, _) = crate::config::load_deepseek_config_file();
    config
        .get("state_events")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".yoyo/state/events.jsonl"))
}

pub(crate) fn default_store_path(events_path: &Path) -> PathBuf {
    let (config, _) = crate::config::load_deepseek_config_file();
    config
        .get("state_store")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::state::sqlite_projection_path(events_path))
}

pub fn handle_state_subcommand(args: &[String]) {
    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("help");
    match sub {
        "init" => handle_init(),
        "tail" => {
            let limit = args
                .iter()
                .position(|a| a == "--limit")
                .and_then(|i| args.get(i + 1))
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(50);
            let json = args.iter().any(|a| a == "--json");
            handle_tail(limit, json);
        }
        "trace" => {
            let Some(id) = args.get(3) else {
                eprintln!("{YELLOW}  Usage: yoyo state trace <run-id|trace-id>{RESET}");
                return;
            };
            handle_trace(id);
        }
        "lifecycle" => handle_lifecycle(&args[3..]),
        "project" => handle_project(&args[3..]),
        "migrate" => handle_migrate(),
        "recover" => handle_recover(&args[3..]),
        "retention" => handle_retention(&args[3..]),
        "memory" => crate::commands_state_memory::handle_memory(&args[3..]),
        "journal" => handle_journal(&args[3..]),
        "export" => {
            let Some(path) = args.get(3) else {
                eprintln!("{YELLOW}  Usage: yoyo state export <path>{RESET}");
                return;
            };
            handle_export(path);
        }
        "import" => {
            let Some(path) = args.get(3) else {
                eprintln!("{YELLOW}  Usage: yoyo state import <path> [--replace]{RESET}");
                return;
            };
            handle_import(path, args.iter().any(|arg| arg == "--replace"));
        }
        "graph" => crate::commands_state_graph::handle_graph_subcommand(args),
        "failures" => handle_failures(&args[3..]),
        "crashes" => crate::commands_state_crashes::handle_crashes(&args[3..]),
        "cache" => handle_cache(&args[3..]),
        "policies" => handle_policies(&args[3..]),
        "fixes" => handle_fixes(&args[3..]),
        "rollbacks" => handle_rollbacks(&args[3..]),
        "evals" => handle_evals(&args[3..]),
        "patches" => handle_patches(&args[3..]),
        "why" => {
            let summary = args.iter().any(|a| a == "--summary");
            let limit = args
                .iter()
                .position(|a| a == "--limit")
                .and_then(|i| args.get(i + 1))
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(200);
            // No explicit id? Show summary only (--summary without id path)
            let id_candidate = args
                .get(3)
                .filter(|a| !a.starts_with("--") && !a.chars().all(|c| c.is_ascii_digit()));
            if id_candidate.is_none() {
                handle_state_summary(args, limit);
                return;
            }
            let id = id_candidate.unwrap();
            handle_why(id, summary, limit);
        }
        "lineage" => {
            let Some(id) = args.get(3) else {
                eprintln!("{YELLOW}  Usage: yoyo state lineage <event-id|patch-id|commit>{RESET}");
                return;
            };
            handle_lineage(id);
        }
        "doctor" => handle_doctor(),
        _ => print_usage(),
    }
}

fn handle_init() {
    let path = default_events_path();
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("{RED}  failed to create {}: {e}{RESET}", parent.display());
            return;
        }
    }
    if let Err(e) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        eprintln!("{RED}  failed to initialize {}: {e}{RESET}", path.display());
        return;
    }
    println!("{GREEN}  initialized state log:{RESET} {}", path.display());
}

fn handle_doctor() {
    println!("{BOLD}State Doctor{RESET}");

    let events_path = default_events_path();
    let store_path = default_store_path(&events_path);

    // --- Events: count, runs, failures, type distribution ---
    let (total_events, runs, run_failures, type_counts, recent_failures) =
        match read_events(&events_path) {
            Ok(events) => {
                let total = events.len();
                let mut runs_count = 0u64;
                let mut failures = 0u64;
                let mut types: BTreeMap<String, u64> = BTreeMap::new();
                let mut fail_list: Vec<(String, String)> = Vec::new();

                for ev in &events {
                    let typ = ev.get("type").and_then(|v| v.as_str()).unwrap_or("unknown");
                    *types.entry(typ.to_string()).or_default() += 1;

                    match typ {
                        "RunStarted" => runs_count += 1,
                        "RunCompleted" => {
                            let status = ev.get("status").and_then(|v| v.as_str()).unwrap_or("");
                            if status == "failed" {
                                failures += 1;
                                let ts = ev.get("ts").and_then(|v| v.as_str()).unwrap_or("?");
                                let run_id =
                                    ev.get("run_id").and_then(|v| v.as_str()).unwrap_or("?");
                                fail_list.push((ts.to_string(), run_id.to_string()));
                            }
                        }
                        _ => {}
                    }
                }

                // Sort by ts descending, keep last 5
                fail_list.sort_by(|a, b| b.0.cmp(&a.0));
                fail_list.truncate(5);

                (total, runs_count, failures, types, fail_list)
            }
            Err(e) => {
                eprintln!(
                    "{RED}  Events: error reading {} — {e}{RESET}",
                    events_path.display()
                );
                (0, 0, 0, BTreeMap::new(), Vec::new())
            }
        };

    let events_color = if total_events > 0 { &GREEN } else { &YELLOW };
    println!(
        "  {events_color}Events:{RESET}    {total_events} total ({runs} runs, {run_failures} failures)"
    );

    // --- Store (SQLite) ---
    let store_line = if store_path.exists() {
        match Connection::open(&store_path) {
            Ok(conn) => {
                let integrity = conn
                    .pragma_query_value(None, "integrity_check", |row| row.get::<_, String>(0))
                    .unwrap_or_else(|_| "error".to_string());
                if integrity != "ok" {
                    format!("{RED}  Store:     SQLite — integrity FAIL: {integrity}{RESET}")
                } else {
                    let version = conn
                        .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                        .unwrap_or(-1);
                    let current = i64::from(crate::state::STATE_SQLITE_SCHEMA_VERSION);
                    if version == current {
                        format!("{GREEN}  Store:     SQLite v{version} — integrity OK{RESET}")
                    } else {
                        format!(
                            "{YELLOW}  Store:     SQLite v{version} (current v{current}) — integrity OK{RESET}"
                        )
                    }
                }
            }
            Err(e) => {
                format!("{RED}  Store:     cannot open: {e}{RESET}")
            }
        }
    } else {
        format!(
            "{YELLOW}  Store:     not found at {}{RESET}",
            store_path.display()
        )
    };
    println!("{store_line}");

    // --- Disk usage ---
    let events_size = std::fs::metadata(&events_path)
        .map(|m| m.len())
        .unwrap_or(0);
    let store_size = std::fs::metadata(&store_path).map(|m| m.len()).unwrap_or(0);
    println!(
        "  {CYAN}Disk:{RESET}      events={}, store={}",
        human_bytes(events_size),
        human_bytes(store_size),
    );

    // --- Schema version ---
    let schema_current = crate::state::STATE_SQLITE_SCHEMA_VERSION;
    let schema_status = if store_path.exists() {
        match Connection::open(&store_path) {
            Ok(conn) => {
                let ver = conn
                    .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                    .unwrap_or(-1);
                if ver == i64::from(schema_current) {
                    format!("{GREEN}  Schema:    version {ver} (current){RESET}")
                } else {
                    format!("{YELLOW}  Schema:    version {ver} (current={schema_current}){RESET}")
                }
            }
            Err(_) => format!("{YELLOW}  Schema:    unable to read{RESET}"),
        }
    } else {
        format!("{YELLOW}  Schema:    no store{RESET}")
    };
    println!("{schema_status}");

    // --- Event type distribution (grouped) ---
    let grouped = group_event_types(&type_counts);
    if !grouped.is_empty() {
        let parts: Vec<String> = grouped
            .iter()
            .map(|(name, count)| format!("{name}={count}"))
            .collect();
        println!("  {CYAN}Types:{RESET}     {}", parts.join(", "));
    }

    // --- Config validation ---
    let events_dir_exists = events_path.parent().map(|p| p.exists()).unwrap_or(false);
    let events_rw = std::fs::OpenOptions::new()
        .read(true)
        .open(&events_path)
        .is_ok();
    let store_dir_exists = store_path.parent().map(|p| p.exists()).unwrap_or(false);
    let store_rw = if store_path.exists() {
        std::fs::OpenOptions::new()
            .read(true)
            .open(&store_path)
            .is_ok()
    } else {
        // Store might not exist yet — check parent dir is writable
        store_path
            .parent()
            .map(|p| {
                std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(p.join(".doctor_test_write"))
                    .map(|f| {
                        drop(f);
                        let _ = std::fs::remove_file(p.join(".doctor_test_write"));
                    })
                    .is_ok()
            })
            .unwrap_or(false)
    };

    let config_ok = events_dir_exists && events_rw && store_dir_exists && store_rw;
    let mut config_issues: Vec<&str> = Vec::new();
    if !events_dir_exists {
        config_issues.push("events dir missing");
    }
    if !events_rw {
        config_issues.push("events not readable");
    }
    if !store_dir_exists {
        config_issues.push("store dir missing");
    }
    if !store_rw {
        config_issues.push("store not writable");
    }

    let config_line = if config_ok {
        format!("{GREEN}  Config:    paths OK{RESET}")
    } else {
        format!(
            "{YELLOW}  Config:    issues: {}{RESET}",
            config_issues.join(", ")
        )
    };
    println!("{config_line}");

    // --- Recent failures ---
    if !recent_failures.is_empty() {
        println!("{BOLD}  Recent failures:{RESET}");
        for (ts, run_id) in &recent_failures {
            println!("{RED}    {ts}  {run_id}{RESET}");
        }
    }

    // --- Overall health ---
    let all_ok = total_events > 0
        && store_path.exists()
        && config_ok
        && run_failures == 0
        && store_line.contains("integrity OK");
    let health_line = if all_ok {
        format!("{GREEN}  Health:    ✓ All checks passed{RESET}")
    } else if total_events == 0 || !store_path.exists() || !config_ok {
        format!("{RED}  Health:    ✗ Issues found — see above{RESET}")
    } else {
        format!("{YELLOW}  Health:    ⚠ Warnings — see above{RESET}")
    };
    println!("{health_line}");

    // --- Stale data warnings ---
    for warning in stale_data_warnings(total_events, events_size, store_size) {
        println!("{warning}");
    }
}

/// Group raw JSON event type strings into display categories.
fn group_event_types(raw: &BTreeMap<String, u64>) -> Vec<(String, u64)> {
    let mut grouped: BTreeMap<String, u64> = BTreeMap::new();

    let merge = |map: &mut BTreeMap<String, u64>, key: &str, src_key: &str| {
        if let Some(v) = raw.get(src_key) {
            *map.entry(key.to_string()).or_default() += v;
        }
    };

    merge(&mut grouped, "Run", "RunStarted");
    merge(&mut grouped, "Run", "RunCompleted");
    merge(&mut grouped, "ToolCall", "ToolCallStarted");
    merge(&mut grouped, "ToolCall", "ToolCallCompleted");
    merge(&mut grouped, "Command", "CommandStarted");
    merge(&mut grouped, "Command", "CommandCompleted");
    merge(&mut grouped, "Model", "ModelCallStarted");
    merge(&mut grouped, "Model", "ModelCallCompleted");
    merge(&mut grouped, "File", "FileRead");
    merge(&mut grouped, "File", "FileEdited");
    merge(&mut grouped, "Test", "TestStarted");
    merge(&mut grouped, "Test", "TestCompleted");
    merge(&mut grouped, "Cache", "CacheMetricsRecorded");

    // Remaining types: keep as-is
    let known: BTreeSet<&str> = [
        "RunStarted",
        "RunCompleted",
        "ToolCallStarted",
        "ToolCallCompleted",
        "CommandStarted",
        "CommandCompleted",
        "ModelCallStarted",
        "ModelCallCompleted",
        "FileRead",
        "FileEdited",
        "TestStarted",
        "TestCompleted",
        "CacheMetricsRecorded",
    ]
    .iter()
    .copied()
    .collect();

    for (key, count) in raw {
        if !known.contains(key.as_str()) {
            grouped.insert(key.clone(), *count);
        }
    }

    // Sort by count descending
    let mut result: Vec<(String, u64)> = grouped.into_iter().collect();
    result.sort_by_key(|b| std::cmp::Reverse(b.1));
    result
}

fn human_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.1}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{}KB", bytes / KB)
    } else {
        format!("{}B", bytes)
    }
}

const STALE_DATA_THRESHOLD_BYTES: u64 = 5 * 1024 * 1024;

/// Return actionable recommendations when state files exist but contain no
/// events — stale data from prior runs that can be cleaned up.
fn stale_data_warnings(total_events: usize, events_size: u64, store_size: u64) -> Vec<String> {
    let mut warnings = Vec::new();
    if total_events == 0 {
        if events_size >= STALE_DATA_THRESHOLD_BYTES {
            warnings.push(format!(
                "{YELLOW}  Stale event data from prior runs detected ({}). Run `yyds state retention --prune` to clean up.{RESET}",
                human_bytes(events_size)
            ));
        }
        if store_size >= STALE_DATA_THRESHOLD_BYTES {
            warnings.push(format!(
                "{YELLOW}  Stale SQLite store from prior runs detected ({}). Run `yyds state retention --prune` to clean up.{RESET}",
                human_bytes(store_size)
            ));
        }
    }
    warnings
}

fn handle_tail(limit: usize, json: bool) {
    let path = default_events_path();
    let Ok(lines) = read_tail(&path, limit) else {
        eprintln!("{YELLOW}  no state log found at {}{RESET}", path.display());
        return;
    };
    if json {
        for line in &lines {
            println!("{line}");
        }
    } else {
        for line in lines.iter() {
            print_event_line(line);
        }
    }
    if limit > 0 && lines.len() >= limit {
        println!();
        println!("{DIM}(showing last {limit}, use --limit 0 for all){RESET}");
    }
}

fn handle_trace(id: &str) {
    let path = default_events_path();
    let Ok(events) = read_events(&path) else {
        eprintln!("{YELLOW}  no state log found at {}{RESET}", path.display());
        return;
    };
    match build_trace_report(&events, id) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_lifecycle(args: &[String]) {
    let limit = flag_value(args, "--limit")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let json_output = args.iter().any(|arg| arg == "--json");
    let path = default_events_path();
    let Ok(events) = read_limited_events(&path, limit) else {
        eprintln!("{YELLOW}  no state log found at {}{RESET}", path.display());
        return;
    };
    let payload = build_state_lifecycle_json(&events);
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!("{}", format_state_lifecycle_report(&payload));
    }
}

fn handle_project(args: &[String]) {
    let rebuild = args.iter().any(|arg| arg == "--rebuild");
    if !rebuild {
        eprintln!("{YELLOW}  Usage: yoyo state project --rebuild{RESET}");
        return;
    }
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    match crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path) {
        Ok(report) => {
            println!("State projection rebuilt");
            println!("  events:        {}", report.events);
            println!("  patches:       {}", report.patches);
            println!("  evals:         {}", report.evals);
            println!("  failures:      {}", report.failures);
            println!("  hypotheses:   {}", report.hypotheses);
            println!("  decisions:    {}", report.decisions);
            println!("  cache metrics: {}", report.cache_metrics);
            println!("  relations:     {}", report.relations);
            println!("  sqlite:        {}", sqlite_path.display());
        }
        Err(e) => eprintln!("{RED}  failed to rebuild state projection: {e}{RESET}"),
    }
}

fn handle_migrate() {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    match crate::state::migrate_sqlite_projection(&sqlite_path) {
        Ok(report) => {
            println!("State projection migrated");
            println!("  from version: {}", report.from_version);
            println!("  to version:   {}", report.to_version);
            if report.applied_versions.is_empty() {
                println!("  applied:      none");
            } else {
                let versions = report
                    .applied_versions
                    .iter()
                    .map(|version| version.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("  applied:      {versions}");
            }
            println!("  sqlite:       {}", sqlite_path.display());
        }
        Err(e) => eprintln!("{RED}  failed to migrate state projection: {e}{RESET}"),
    }
}

fn handle_recover(args: &[String]) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    let replace = args.iter().any(|arg| arg == "--replace");
    let output_path = args
        .iter()
        .position(|arg| arg == "--output")
        .and_then(|idx| args.get(idx + 1))
        .map(PathBuf::from);

    match recover_events(&events_path, &sqlite_path, output_path.as_deref(), replace) {
        Ok(report) => {
            println!("State events recovered");
            println!("  valid events:  {}", report.valid_events);
            println!("  invalid lines: {}", report.invalid_lines.len());
            println!("  output:        {}", report.output_path.display());
            if let Some(backup_path) = report.backup_path {
                println!("  backup:        {}", backup_path.display());
            }
            if report.replaced {
                println!("  mode:          replace");
                println!("  sqlite:        {}", sqlite_path.display());
                println!("  projected:     {}", report.projected_events);
            } else {
                println!("  mode:          recovery-file");
            }
            for issue in report.invalid_lines.iter().take(5) {
                println!(
                    "  skipped line {}: {} ({})",
                    issue.line_number, issue.error, issue.raw_preview
                );
            }
            if report.invalid_lines.len() > 5 {
                println!(
                    "  skipped:       {} more invalid lines",
                    report.invalid_lines.len() - 5
                );
            }
        }
        Err(e) => eprintln!("{RED}  failed to recover state events: {e}{RESET}"),
    }
}

fn handle_retention(args: &[String]) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    let days = args
        .iter()
        .position(|arg| arg == "--days")
        .and_then(|idx| args.get(idx + 1))
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(30);
    let archive_path = args
        .iter()
        .position(|arg| arg == "--archive")
        .and_then(|idx| args.get(idx + 1))
        .map(PathBuf::from);
    let prune = args.iter().any(|arg| arg == "--prune");

    match apply_retention_policy(
        &events_path,
        &sqlite_path,
        days,
        archive_path.as_deref(),
        prune,
        current_time_ms(),
    ) {
        Ok(report) => {
            println!("State retention report");
            println!("  policy:        keep {} days", report.keep_days);
            println!("  cutoff_ms:     {}", report.cutoff_ms);
            println!("  total events:  {}", report.total_events);
            println!("  kept events:   {}", report.kept_events);
            println!("  old events:    {}", report.archived_events);
            if let Some(path) = report.archive_path {
                println!("  archive:       {}", path.display());
            }
            if let Some(path) = report.backup_path {
                println!("  backup:        {}", path.display());
            }
            if report.pruned {
                println!("  mode:          prune");
                println!("  sqlite:        {}", sqlite_path.display());
                println!("  projected:     {}", report.projected_events);
            } else {
                println!("  mode:          report");
            }
        }
        Err(e) => eprintln!("{RED}  failed to apply state retention policy: {e}{RESET}"),
    }
}

fn handle_journal(args: &[String]) {
    if args.first().map(|arg| arg.as_str()) != Some("generate") {
        eprintln!("{YELLOW}  Usage: yoyo state journal generate [--output PATH]{RESET}");
        return;
    }
    let events_path = default_events_path();
    let output_path = args
        .iter()
        .position(|arg| arg == "--output")
        .and_then(|idx| args.get(idx + 1))
        .map(PathBuf::from);
    let Ok(events) = read_events(&events_path) else {
        eprintln!(
            "{YELLOW}  no state log found at {}{RESET}",
            events_path.display()
        );
        return;
    };
    match build_state_journal_draft(&events) {
        Ok(report) => {
            if let Some(path) = output_path {
                match write_text_artifact(&path, &report, "state journal draft") {
                    Ok(()) => println!("State journal draft written\n  output: {}", path.display()),
                    Err(e) => eprintln!("{RED}  failed to write state journal draft: {e}{RESET}"),
                }
            } else {
                println!("{report}");
            }
        }
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_export(path: &str) {
    let events_path = default_events_path();
    match export_events(&events_path, Path::new(path)) {
        Ok(count) => {
            println!("State events exported");
            println!("  events: {}", count);
            println!("  from:   {}", events_path.display());
            println!("  to:     {path}");
        }
        Err(e) => eprintln!("{RED}  failed to export state events: {e}{RESET}"),
    }
}

fn handle_import(path: &str, replace: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    match import_events(Path::new(path), &events_path, &sqlite_path, replace) {
        Ok(report) => {
            println!("State events imported");
            println!("  imported: {}", report.imported_events);
            println!(
                "  mode:     {}",
                if report.replaced { "replace" } else { "append" }
            );
            println!("  events:   {}", events_path.display());
            println!("  sqlite:   {}", sqlite_path.display());
            println!("  projected events: {}", report.projection.events);
        }
        Err(e) => eprintln!("{RED}  failed to import state events: {e}{RESET}"),
    }
}

fn handle_policies(args: &[String]) {
    if args.first().map(|arg| arg.as_str()) != Some("--recent") && !args.is_empty() {
        eprintln!("{YELLOW}  Usage: yoyo state policies --recent [--limit N]{RESET}");
        return;
    }
    let limit = flag_value(args, "--limit")
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(12);
    let path = default_events_path();
    let Ok(events) = read_events(&path) else {
        eprintln!("{YELLOW}  no state log found at {}{RESET}", path.display());
        return;
    };
    match build_policy_report(&events, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_fixes(args: &[String]) {
    if args.first().map(|arg| arg.as_str()) != Some("--recent") && !args.is_empty() {
        eprintln!("{YELLOW}  Usage: yoyo state fixes --recent [--class CLASS] [--limit N]{RESET}");
        return;
    }
    let limit = flag_value(args, "--limit")
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(12);
    let class_filter = flag_value(args, "--class").map(String::as_str);
    let path = default_events_path();
    let Ok(events) = read_events(&path) else {
        eprintln!("{YELLOW}  no state log found at {}{RESET}", path.display());
        return;
    };
    match build_failure_fix_report(&events, class_filter, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_rollbacks(args: &[String]) {
    if args.iter().any(|arg| {
        !matches!(arg.as_str(), "--recent" | "--json" | "--limit")
            && !arg.chars().all(|ch| ch.is_ascii_digit())
    }) {
        eprintln!("{YELLOW}  Usage: yoyo state rollbacks --recent [--limit N] [--json]{RESET}");
        return;
    }
    let json_output = args.iter().any(|arg| arg == "--json");
    let limit = flag_value(args, "--limit")
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(12);
    let path = default_events_path();
    let Ok(events) = read_events(&path) else {
        eprintln!("{YELLOW}  no state log found at {}{RESET}", path.display());
        return;
    };
    if json_output {
        match build_rollback_payload(&events, limit) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
    } else {
        match build_rollback_report(&events, limit) {
            Ok(report) => println!("{report}"),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
    }
}

fn handle_failures(args: &[String]) {
    if !args.is_empty() && args.first().map(|arg| arg.as_str()) != Some("--recent") {
        eprintln!("{YELLOW}  Usage: yoyo state failures --recent [--limit N]{RESET}");
        return;
    }
    let limit = flag_value(args, "--limit")
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(12);
    let path = default_events_path();
    let Ok(events) = read_tail_events(&path, 0) else {
        eprintln!("{YELLOW}  no state log found at {}{RESET}", path.display());
        return;
    };
    if events.is_empty() {
        eprintln!(
            "{YELLOW}  no parseable events found at {}{RESET}",
            path.display()
        );
        return;
    }
    match build_recent_failure_report(&events, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_cache(args: &[String]) {
    if !args.is_empty() && args.first().map(|arg| arg.as_str()) != Some("--recent") {
        eprintln!("{YELLOW}  Usage: yoyo state cache --recent [--limit N]{RESET}");
        return;
    }
    let limit = flag_value(args, "--limit")
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(12);
    let path = default_events_path();
    let Ok(events) = read_tail_events(&path, 0) else {
        eprintln!("{YELLOW}  no state log found at {}{RESET}", path.display());
        return;
    };
    if events.is_empty() {
        eprintln!(
            "{YELLOW}  no parseable events found at {}{RESET}",
            path.display()
        );
        return;
    }
    match build_cache_recent_report(&events, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_evals(args: &[String]) {
    let path = default_events_path();
    let Ok(events) = read_events(&path) else {
        eprintln!("{YELLOW}  no state log found at {}{RESET}", path.display());
        return;
    };
    let harness_version = args
        .iter()
        .position(|a| a == "--harness-version")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str());
    let patch_id = args
        .iter()
        .position(|a| a == "--patch-id")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str());

    match build_eval_report(&events, harness_version, patch_id) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_patches(args: &[String]) {
    let path = default_events_path();
    let Ok(events) = read_events(&path) else {
        eprintln!("{YELLOW}  no state log found at {}{RESET}", path.display());
        return;
    };

    if args.first().map(|s| s.as_str()) == Some("show") {
        let Some(id) = args.get(1) else {
            eprintln!("{YELLOW}  Usage: yoyo state patches show <patch-id>{RESET}");
            return;
        };
        match build_patch_show_report(&events, id) {
            Ok(report) => println!("{report}"),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }

    let status_filter = args
        .iter()
        .position(|a| a == "--status")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str());
    match build_patch_list_report(&events, status_filter) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_why(id: &str, show_summary: bool, limit: usize) {
    let path = default_events_path();
    let Ok(events) = read_tail_events(&path, limit) else {
        eprintln!(
            "{YELLOW}  No state events file found at {}{RESET}",
            path.display()
        );
        eprintln!(
            "{YELLOW}  This is a cold start: no evolution sessions have recorded state yet.{RESET}"
        );
        eprintln!("{DIM}  To enable state recording, run: yoyo state init{RESET}");
        eprintln!("{DIM}  Once sessions complete, run: yoyo state why last-failure{RESET}");
        return;
    };
    if show_summary {
        println!("{}", build_state_summary(&events));
        println!();
    }
    match build_why_report(&events, id) {
        Ok(report) => println!("{report}"),
        Err(e) => {
            let mut msg = format!("{YELLOW}  {e}{RESET}");
            if limit > 0 && events.len() >= limit {
                msg.push_str(&format!(
                    "\n{DIM}(note: only the most recent {limit} events were scanned; the target may be further back — retry with --limit 0){RESET}"
                ));
            }
            eprintln!("{msg}");

            // Fallback: when no failure found for last-failure, scan for
            // incomplete runs (RunStarted without RunCompleted) and offer
            // diagnostic next-steps so cold-start users get a breadcrumb trail.
            if id == "last-failure" {
                let incomplete = find_incomplete_runs(&events);
                if !incomplete.is_empty() {
                    let s = if incomplete.len() == 1 { "" } else { "s" };
                    eprintln!(
                        "\n{YELLOW}  However, {} incomplete run{} detected (started but not completed):{RESET}",
                        incomplete.len(),
                        s,
                    );
                    let now_ms = current_time_ms() as u64;
                    for (run_id, ts_ms) in &incomplete {
                        let ago = if *ts_ms > 0 && now_ms > *ts_ms {
                            format_duration(std::time::Duration::from_secs(
                                (now_ms - *ts_ms) / 1000,
                            ))
                        } else {
                            "?".to_string()
                        };
                        eprintln!(
                            "{DIM}    {run_id} — started {} ago, no RunCompleted event{RESET}",
                            ago
                        );
                    }
                    eprintln!(
                        "{DIM}  Run: yyds state trace <run-id> for details, or yyds state crashes for crash analysis.{RESET}"
                    );
                }
            }
        }
    }
    if limit > 0 {
        let all_count = read_events(&path).map(|e| e.len()).unwrap_or(events.len());
        if events.len() >= limit {
            println!();
            println!("{DIM}(searched last {limit} events of {all_count} total, use --limit 0 for full scan){RESET}");
        } else if events.len() < all_count {
            println!();
            println!("{DIM}(searched {all_count} events){RESET}");
        }
    }
}

/// Find runs that started (RunStarted) but never completed (no RunCompleted event).
/// Returns a list of (run_id, timestamp_ms) sorted by most recent first.
fn find_incomplete_runs(events: &[Value]) -> Vec<(String, u64)> {
    let completed: BTreeSet<&str> = events
        .iter()
        .filter(|e| event_string(e, "event_type") == Some("RunCompleted"))
        .filter_map(|e| event_string(e, "run_id"))
        .collect();
    let mut incomplete: BTreeMap<String, u64> = BTreeMap::new();
    for e in events {
        let event_type = event_string(e, "event_type");
        if event_type == Some("RunStarted") {
            if let Some(run_id) = event_string(e, "run_id") {
                if !completed.contains(run_id) {
                    let ts = event_timestamp(e);
                    // Keep the most recent (largest) timestamp for each run_id
                    incomplete
                        .entry(run_id.to_string())
                        .and_modify(|existing| {
                            if ts > *existing {
                                *existing = ts;
                            }
                        })
                        .or_insert(ts);
                }
            }
        }
    }
    let mut result: Vec<(String, u64)> = incomplete.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1));
    result
}

fn handle_state_summary(args: &[String], limit: usize) {
    let path = default_events_path();
    let show_tail = args.iter().any(|a| a == "--tail");
    let Ok(events) = read_tail_events(&path, limit) else {
        eprintln!("{YELLOW}  no state log found at {}{RESET}", path.display());
        return;
    };
    println!("{}", build_state_summary(&events));
    if limit > 0 {
        let all_count = read_events(&path).map(|e| e.len()).unwrap_or(events.len());
        if events.len() >= limit {
            println!();
            println!("{DIM}(summary from last {limit} events of {all_count} total, use --limit 0 for full scan){RESET}");
        }
    }
    if show_tail && !events.is_empty() {
        println!();
        println!("Most recent events:");
        for event in events.iter().rev().take(5) {
            let ts = event_timestamp_ms(event)
                .map(format_timestamp_ms)
                .unwrap_or_else(|| "?".to_string());
            let et = event_string(event, "event_type").unwrap_or("?");
            let eid = event_string(event, "event_id").unwrap_or("?");
            println!("  {ts}  {et}  {eid}");
        }
    }
}

fn handle_lineage(id: &str) {
    let path = default_events_path();
    let Ok(events) = read_events(&path) else {
        eprintln!("{YELLOW}  no state log found at {}{RESET}", path.display());
        return;
    };
    match build_lineage_report(&events, id) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn read_tail(path: &Path, limit: usize) -> Result<Vec<String>, std::io::Error> {
    let raw = std::fs::read_to_string(path)?;
    let mut lines = VecDeque::new();
    for line in raw.lines() {
        lines.push_back(line.to_string());
        if lines.len() > limit {
            lines.pop_front();
        }
    }
    Ok(lines.into_iter().collect())
}

pub(crate) fn read_events(path: &Path) -> Result<Vec<Value>, std::io::Error> {
    crate::state::read_compatibility_events(path).map_err(std::io::Error::other)
}

/// Lenient event reader: parses all lines as JSON, skipping malformed ones.
/// Only returns an error if the file genuinely can't be read (missing, perms).
fn read_events_lenient(path: &Path) -> Result<Vec<Value>, std::io::Error> {
    let raw = std::fs::read_to_string(path)?;
    let events: Vec<Value> = raw
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let normalized = crate::state::compatibility_event_json_line(line).ok()?;
            serde_json::from_str::<Value>(&normalized).ok()
        })
        .collect();
    Ok(events)
}

fn read_limited_events(path: &Path, limit: usize) -> Result<Vec<Value>, std::io::Error> {
    let mut events = read_events(path)?;
    if limit > 0 && events.len() > limit {
        let keep_from = events.len() - limit;
        events = events.split_off(keep_from);
    }
    Ok(events)
}

/// Read only the last `limit` events from the state log.
/// Returns fewer than `limit` events if the log is smaller.
/// When `limit == 0`, reads all events with lenient parsing (skips malformed lines).
fn read_tail_events(path: &Path, limit: usize) -> Result<Vec<Value>, std::io::Error> {
    if limit == 0 {
        return read_events_lenient(path);
    }
    let raw_lines = read_tail(path, limit)?;
    let mut events = Vec::with_capacity(raw_lines.len());
    for line in &raw_lines {
        if let Ok(value) = serde_json::from_str::<Value>(line) {
            events.push(value);
        }
    }
    Ok(events)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ImportReport {
    imported_events: usize,
    replaced: bool,
    projection: crate::state::ProjectionReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecoveryIssue {
    line_number: usize,
    error: String,
    raw_preview: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecoveryReport {
    valid_events: usize,
    invalid_lines: Vec<RecoveryIssue>,
    output_path: PathBuf,
    backup_path: Option<PathBuf>,
    replaced: bool,
    projected_events: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RetentionReport {
    keep_days: u64,
    cutoff_ms: i64,
    total_events: usize,
    kept_events: usize,
    archived_events: usize,
    archive_path: Option<PathBuf>,
    backup_path: Option<PathBuf>,
    pruned: bool,
    projected_events: usize,
}

fn export_events(events_path: &Path, export_path: &Path) -> Result<usize, String> {
    let raw = std::fs::read_to_string(events_path)
        .map_err(|e| format!("read state events '{}': {e}", events_path.display()))?;
    let lines = validate_event_jsonl(&raw)?;
    if let Some(parent) = export_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create export directory '{}': {e}", parent.display()))?;
    }
    let mut normalized = lines.join("\n");
    if !normalized.is_empty() {
        normalized.push('\n');
    }
    std::fs::write(export_path, normalized)
        .map_err(|e| format!("write state export '{}': {e}", export_path.display()))?;
    Ok(lines.len())
}

fn import_events(
    import_path: &Path,
    events_path: &Path,
    sqlite_path: &Path,
    replace: bool,
) -> Result<ImportReport, String> {
    let raw = std::fs::read_to_string(import_path)
        .map_err(|e| format!("read state import '{}': {e}", import_path.display()))?;
    let lines = validate_event_jsonl(&raw)?;
    if let Some(parent) = events_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create state directory '{}': {e}", parent.display()))?;
    }
    let mut normalized = lines.join("\n");
    if !normalized.is_empty() {
        normalized.push('\n');
    }
    if replace {
        std::fs::write(events_path, normalized)
            .map_err(|e| format!("replace state events '{}': {e}", events_path.display()))?;
    } else {
        let mut existing = std::fs::read_to_string(events_path).unwrap_or_default();
        if !existing.is_empty() && !existing.ends_with('\n') {
            existing.push('\n');
        }
        existing.push_str(&normalized);
        std::fs::write(events_path, existing)
            .map_err(|e| format!("append state events '{}': {e}", events_path.display()))?;
    }

    let projection = crate::state::rebuild_sqlite_projection(events_path, sqlite_path)?;
    Ok(ImportReport {
        imported_events: lines.len(),
        replaced: replace,
        projection,
    })
}

fn apply_retention_policy(
    events_path: &Path,
    sqlite_path: &Path,
    keep_days: u64,
    archive_path: Option<&Path>,
    prune: bool,
    now_ms: i64,
) -> Result<RetentionReport, String> {
    let raw = std::fs::read_to_string(events_path)
        .map_err(|e| format!("read state events '{}': {e}", events_path.display()))?;
    let cutoff_ms = retention_cutoff_ms(now_ms, keep_days);
    let mut normalized_lines = Vec::new();
    let mut kept_lines = Vec::new();
    let mut archived_lines = Vec::new();
    for (idx, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let normalized = crate::state::normalize_event_json_line(trimmed)
            .map_err(|e| format!("parse event line {} before retention: {e}", idx + 1))?;
        let value: Value = serde_json::from_str(&normalized)
            .map_err(|e| format!("decode normalized event line {}: {e}", idx + 1))?;
        let timestamp_ms = event_timestamp_ms(&value).unwrap_or(0);
        if timestamp_ms < cutoff_ms {
            archived_lines.push(normalized.clone());
        } else {
            kept_lines.push(normalized.clone());
        }
        normalized_lines.push(normalized);
    }

    let archive_path = archive_path
        .map(PathBuf::from)
        .or_else(|| prune.then(|| default_retention_archive_path(events_path, cutoff_ms)));
    if let Some(path) = archive_path.as_ref() {
        write_normalized_jsonl(path, &archived_lines, "state retention archive")?;
    }

    let backup_path = if prune {
        let backup = retention_backup_path(events_path);
        write_normalized_jsonl(&backup, &normalized_lines, "state retention backup")?;
        write_normalized_jsonl(events_path, &kept_lines, "pruned state events")?;
        Some(backup)
    } else {
        None
    };

    let projected_events = if prune {
        crate::state::rebuild_sqlite_projection(events_path, sqlite_path)?.events
    } else {
        0
    };

    Ok(RetentionReport {
        keep_days,
        cutoff_ms,
        total_events: kept_lines.len() + archived_lines.len(),
        kept_events: kept_lines.len(),
        archived_events: archived_lines.len(),
        archive_path,
        backup_path,
        pruned: prune,
        projected_events,
    })
}

fn recover_events(
    events_path: &Path,
    sqlite_path: &Path,
    output_path: Option<&Path>,
    replace: bool,
) -> Result<RecoveryReport, String> {
    let raw = std::fs::read_to_string(events_path)
        .map_err(|e| format!("read state events '{}': {e}", events_path.display()))?;
    let mut valid_lines = Vec::new();
    let mut invalid_lines = Vec::new();
    for (idx, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match crate::state::normalize_event_json_line(trimmed) {
            Ok(normalized) => valid_lines.push(normalized),
            Err(error) => invalid_lines.push(RecoveryIssue {
                line_number: idx + 1,
                error,
                raw_preview: preview_line(trimmed, 120),
            }),
        }
    }

    let out_path = output_path
        .map(PathBuf::from)
        .unwrap_or_else(|| recovered_events_path(events_path));
    let target_path = if replace { events_path } else { &out_path };
    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create recovery directory '{}': {e}", parent.display()))?;
    }

    let mut normalized = valid_lines.join("\n");
    if !normalized.is_empty() {
        normalized.push('\n');
    }

    let backup_path = if replace {
        let backup = backup_events_path(events_path);
        write_normalized_jsonl(&backup, &valid_lines, "state recovery backup")?;
        std::fs::write(events_path, normalized)
            .map_err(|e| format!("replace state events '{}': {e}", events_path.display()))?;
        Some(backup)
    } else {
        std::fs::write(&out_path, normalized)
            .map_err(|e| format!("write recovered events '{}': {e}", out_path.display()))?;
        None
    };

    let projected_events = if replace {
        crate::state::rebuild_sqlite_projection(events_path, sqlite_path)?.events
    } else {
        0
    };

    Ok(RecoveryReport {
        valid_events: valid_lines.len(),
        invalid_lines,
        output_path: target_path.to_path_buf(),
        backup_path,
        replaced: replace,
        projected_events,
    })
}

fn validate_event_jsonl(raw: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for (idx, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let normalized = crate::state::normalize_event_json_line(trimmed)
            .map_err(|e| format!("parse event line {}: {e}", idx + 1))?;
        out.push(normalized);
    }
    Ok(out)
}

fn write_normalized_jsonl(path: &Path, lines: &[String], label: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create {label} directory '{}': {e}", parent.display()))?;
    }
    let mut normalized = lines.join("\n");
    if !normalized.is_empty() {
        normalized.push('\n');
    }
    std::fs::write(path, normalized).map_err(|e| format!("write {label} '{}': {e}", path.display()))
}

pub(crate) fn write_text_artifact(path: &Path, text: &str, label: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create {label} directory '{}': {e}", parent.display()))?;
    }
    std::fs::write(path, text).map_err(|e| format!("write {label} '{}': {e}", path.display()))
}

pub(crate) fn event_timestamp_ms(event: &Value) -> Option<i64> {
    event
        .get("timestamp_ms")
        .or_else(|| event.get("ts_ms"))
        .and_then(|v| v.as_i64())
}

fn retention_cutoff_ms(now_ms: i64, keep_days: u64) -> i64 {
    const DAY_MS: i64 = 86_400_000;
    let retention_ms = keep_days.min((i64::MAX / DAY_MS) as u64) as i64 * DAY_MS;
    now_ms.saturating_sub(retention_ms)
}

fn recovered_events_path(events_path: &Path) -> PathBuf {
    let file_name = events_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("events.jsonl");
    events_path.with_file_name(format!("{file_name}.recovered"))
}

fn backup_events_path(events_path: &Path) -> PathBuf {
    let file_name = events_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("events.jsonl");
    events_path.with_file_name(format!("{file_name}.bak"))
}

fn default_retention_archive_path(events_path: &Path, cutoff_ms: i64) -> PathBuf {
    let file_name = events_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("events.jsonl");
    events_path.with_file_name(format!("{file_name}.archive-before-{cutoff_ms}.jsonl"))
}

fn retention_backup_path(events_path: &Path) -> PathBuf {
    let file_name = events_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("events.jsonl");
    events_path.with_file_name(format!("{file_name}.retention.bak"))
}

pub(crate) fn current_time_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

pub(crate) fn format_timestamp_ms(ts_ms: i64) -> String {
    let secs = ts_ms / 1000;
    let d = std::time::Duration::from_secs(secs.max(0) as u64);
    let since = std::time::UNIX_EPOCH
        .checked_add(d)
        .unwrap_or(std::time::UNIX_EPOCH);
    // Format as ISO 8601-like: YYYY-MM-DD HH:MM:SS
    let total_secs = since
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = total_secs / 86400;
    let time_secs = total_secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let secs = time_secs % 60;
    // Convert days since epoch to date (approximate, using civil calendar calculation)
    let (y, m, d) = days_to_ymd(days as i64);
    format!("{y:04}-{m:02}-{d:02} {hours:02}:{minutes:02}:{secs:02}")
}

fn days_to_ymd(days: i64) -> (i64, i64, i64) {
    // Algorithm from Howard Hinnant
    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as i64, d as i64)
}

pub(crate) fn flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a String> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|idx| args.get(idx + 1))
}

pub(crate) fn preview_line(line: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for ch in line.chars().take(max_chars) {
        out.push(ch);
    }
    if line.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

fn print_event_line(line: &str) {
    match event_line_value(line) {
        Ok(value) => println!("{}", format_event_value(&value)),
        Err(_) => println!("{line}"),
    }
}

fn event_line_value(line: &str) -> Result<Value, String> {
    crate::state::compatibility_event_json_line(line)
        .and_then(|line| serde_json::from_str::<Value>(&line).map_err(|e| e.to_string()))
        .or_else(|_| serde_json::from_str::<Value>(line).map_err(|e| e.to_string()))
}

fn format_event_value(event: &Value) -> String {
    let ts = event
        .get("timestamp_ms")
        .or_else(|| event.get("ts_ms"))
        .and_then(|v| v.as_u64())
        .map(|v| v.to_string())
        .unwrap_or_else(|| "?".to_string());
    let kind = event
        .get("event_type")
        .or_else(|| event.get("kind"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");
    let event_id = event
        .get("event_id")
        .or_else(|| event.get("id"))
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let run_id = event.get("run_id").and_then(|v| v.as_str()).unwrap_or("-");
    let payload = event.get("payload").unwrap_or(&Value::Null);
    let summary = event_payload_summary(kind, payload);
    format!("{ts}  {kind:<22} {event_id}  run={run_id}  {summary}")
}

fn event_payload_summary(kind: &str, payload: &Value) -> String {
    match kind {
        "RunStarted" => payload_str(payload, "task")
            .map(|task| format!("task={}", preview_line(task, 100)))
            .unwrap_or_else(|| compact_payload_summary(payload)),
        "RunCompleted" => payload_str(payload, "status")
            .map(|status| format!("status={status}"))
            .unwrap_or_else(|| compact_payload_summary(payload)),
        "ModelCallStarted" => payload_str(payload, "model")
            .map(|model| format!("model={model}"))
            .unwrap_or_else(|| compact_payload_summary(payload)),
        "ModelCallCompleted" => {
            let model = payload_str(payload, "model").unwrap_or("-");
            let input = payload_u64(payload, "input_tokens").unwrap_or(0);
            let output = payload_u64(payload, "output_tokens").unwrap_or(0);
            let cache_read = payload_u64(payload, "cache_read_tokens").unwrap_or(0);
            let cache_write = payload_u64(payload, "cache_write_tokens").unwrap_or(0);
            format!(
                "model={model} tokens=in:{input} out:{output} cache_read:{cache_read} cache_write:{cache_write}"
            )
        }
        "ToolCallStarted" => {
            let tool = payload_str(payload, "tool_name").unwrap_or("-");
            let tool_call_id = payload_str(payload, "tool_call_id").unwrap_or("-");
            let args = payload.get("args").unwrap_or(&Value::Null);
            format!(
                "tool={tool} tool_call={tool_call_id} {}",
                tool_arg_summary(args)
            )
            .trim_end()
            .to_string()
        }
        "ToolCallCompleted" => {
            let tool = payload_str(payload, "tool_name").unwrap_or("-");
            let tool_call_id = payload_str(payload, "tool_call_id").unwrap_or("-");
            let status = if payload_bool(payload, "is_error").unwrap_or(false) {
                "error"
            } else {
                "ok"
            };
            let preview = payload_str(payload, "result_preview")
                .map(|value| format!(" result={}", preview_line(value, 120)))
                .unwrap_or_default();
            format!("tool={tool} tool_call={tool_call_id} status={status}{preview}")
        }
        "FileRead" => payload_str(payload, "path")
            .map(|path| format!("path={path}"))
            .unwrap_or_else(|| compact_payload_summary(payload)),
        "FileEdited" => {
            let path = payload_str(payload, "path")
                .or_else(|| payload_str(payload, "file_path"))
                .unwrap_or("-");
            let kind = payload_str(payload, "edit_kind")
                .or_else(|| payload_str(payload, "source"))
                .unwrap_or("-");
            let old_lines = payload_u64(payload, "old_line_count").unwrap_or(0);
            let new_lines = payload_u64(payload, "new_line_count").unwrap_or(0);
            let diff = payload_str(payload, "diff_preview")
                .map(|value| format!(" diff={}", preview_line(value, 120).replace('\n', " | ")))
                .unwrap_or_default();
            format!("path={path} edit={kind} lines={old_lines}->{new_lines}{diff}")
        }
        "CommandStarted" => payload_str(payload, "command")
            .map(|command| format!("command={}", preview_line(command, 140)))
            .unwrap_or_else(|| compact_payload_summary(payload)),
        "CommandCompleted" => {
            let status = if payload_bool(payload, "is_error").unwrap_or(false) {
                "error"
            } else {
                "ok"
            };
            let preview = payload_str(payload, "result_preview")
                .map(|value| format!(" result={}", preview_line(value, 120)))
                .unwrap_or_default();
            format!("status={status}{preview}")
        }
        "TestStarted" => {
            let kind = payload_str(payload, "test_kind").unwrap_or("-");
            let command = payload_str(payload, "command").unwrap_or("-");
            format!("test={kind} command={}", preview_line(command, 120))
        }
        "TestCompleted" => {
            let kind = payload_str(payload, "test_kind").unwrap_or("-");
            let passed = payload_bool(payload, "passed").unwrap_or(false);
            let preview = payload_str(payload, "result_preview")
                .map(|value| format!(" result={}", preview_line(value, 120)))
                .unwrap_or_default();
            format!("test={kind} passed={passed}{preview}")
        }
        "FailureObserved" | "ToolSchemaFailure" | "JsonOutputFailure" => {
            let taxonomy = classify_failure_payload(kind, payload);
            let source = payload_str(payload, "source")
                .or_else(|| payload_str(payload, "operation"))
                .unwrap_or("-");
            let preview = payload_str(payload, "error_preview")
                .or_else(|| payload_str(payload, "error"))
                .or_else(|| payload_str(payload, "summary"))
                .unwrap_or("-");
            format!(
                "source={source} class={} owner={} retryable={} error={}",
                taxonomy.class,
                taxonomy.owner,
                taxonomy.retryable,
                preview_line(preview, 120)
            )
        }
        "CacheMetricsRecorded" => {
            let model = payload_str(payload, "model").unwrap_or("-");
            let hit = payload_u64(payload, "cache_hit_tokens")
                .or_else(|| payload_u64(payload, "prompt_cache_hit_tokens"))
                .unwrap_or(0);
            let miss = payload_u64(payload, "cache_miss_tokens")
                .or_else(|| payload_u64(payload, "prompt_cache_miss_tokens"))
                .unwrap_or(0);
            let ratio = payload_f64(payload, "cache_hit_ratio").unwrap_or(0.0);
            format!("model={model} cache_hit={hit} cache_miss={miss} ratio={ratio:.2}")
        }
        "ContextBuilt" => {
            let policy = payload_str(payload, "context_policy").unwrap_or("-");
            let layout = payload_str(payload, "prompt_layout").unwrap_or("-");
            let tokens = payload_u64(payload, "estimated_tokens").unwrap_or(0);
            let blocks = context_included_block_names(payload);
            let instruction_files = payload_string_array(payload, "include_instruction_files");
            format!(
                "policy={policy} layout={layout} tokens={tokens} instructions=[{}] blocks=[{}]",
                instruction_files.join(", "),
                blocks.join(", ")
            )
        }
        "PatchProposed" | "PatchApplied" | "PatchEvaluated" | "PatchPromoted" | "PatchRejected" => {
            let patch = payload_str(payload, "patch_id").unwrap_or("-");
            let status = payload_str(payload, "status")
                .or_else(|| payload_str(payload, "decision"))
                .unwrap_or(kind);
            let reason = payload_str(payload, "reason")
                .or_else(|| payload_str(payload, "intent"))
                .unwrap_or("-");
            format!(
                "patch={patch} status={status} reason={}",
                preview_line(reason, 120)
            )
        }
        "DecisionRecorded" => {
            let decision = payload_str(payload, "decision")
                .or_else(|| payload_str(payload, "decision_type"))
                .unwrap_or("-");
            let reason = payload_str(payload, "reason")
                .or_else(|| payload_str(payload, "rationale"))
                .unwrap_or("-");
            format!("decision={decision} reason={}", preview_line(reason, 120))
        }
        _ => compact_payload_summary(payload),
    }
}

fn compact_payload_summary(payload: &Value) -> String {
    let payload_text = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());
    format!("payload={}", preview_line(&payload_text, 180))
}

fn tool_arg_summary(args: &Value) -> String {
    let path = payload_str(args, "path")
        .map(|path| format!("path={path}"))
        .unwrap_or_default();
    let command = payload_str(args, "command")
        .map(|command| format!("command={}", preview_line(command, 120)))
        .unwrap_or_default();
    let omitted = ["content_omitted", "text_omitted"]
        .iter()
        .find_map(|key| {
            payload_bool(args, key)
                .filter(|omitted| *omitted)
                .map(|_| (*key).to_string())
        })
        .unwrap_or_default();
    [path, command, omitted]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FailureTaxonomy {
    class: &'static str,
    owner: &'static str,
    retryable: bool,
}

fn format_failure_event_value(event: &Value) -> String {
    let taxonomy = classify_failure_event(event);
    format!(
        "{}  class={} owner={} retryable={}",
        format_event_value(event),
        taxonomy.class,
        taxonomy.owner,
        taxonomy.retryable
    )
}

fn build_trace_report(events: &[Value], id: &str) -> Result<String, String> {
    let mut matched = events
        .iter()
        .filter(|event| {
            event_string(event, "run_id")
                .map(|run_id| run_id == id)
                .unwrap_or(false)
                || event_string(event, "trace_id")
                    .map(|trace_id| trace_id == id)
                    .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    if matched.is_empty() {
        return Err(format!("no state trace found for '{id}'"));
    }
    matched.sort_by_key(|event| {
        (
            event_timestamp(event),
            event_string(event, "event_id").unwrap_or("").to_string(),
        )
    });

    let run_id = matched
        .iter()
        .find_map(|event| event_string(event, "run_id"))
        .unwrap_or("-");
    let trace_id = matched
        .iter()
        .find_map(|event| event_string(event, "trace_id"))
        .unwrap_or("-");
    let first_ms = matched
        .first()
        .map(|event| event_timestamp(event))
        .unwrap_or(0);
    let last_ms = matched
        .last()
        .map(|event| event_timestamp(event))
        .unwrap_or(0);

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut terminal_status = "started".to_string();
    for event in &matched {
        let kind = event_string(event, "event_type")
            .or_else(|| event_string(event, "kind"))
            .unwrap_or("Unknown");
        *counts.entry(kind.to_string()).or_default() += 1;
        if kind == "RunCompleted" {
            terminal_status = event
                .get("payload")
                .and_then(|payload| payload_str(payload, "status"))
                .unwrap_or("completed")
                .to_string();
        }
    }

    let mut out = String::new();
    out.push_str(&format!("State trace: {id}\n"));
    out.push_str(&format!("run:   {run_id}\n"));
    out.push_str(&format!("trace: {trace_id}\n"));
    out.push_str(&format!("status: {terminal_status}\n"));
    out.push_str(&format!("events: {}\n", matched.len()));
    out.push_str(&format!("window: {first_ms}..{last_ms}\n"));
    out.push_str("counts:\n");
    for (kind, count) in counts {
        out.push_str(&format!("  {kind}: {count}\n"));
    }
    out.push_str("\nTimeline:\n");
    for event in matched {
        out.push_str("  ");
        out.push_str(&format_event_value(event));
        out.push('\n');
    }
    Ok(out.trim_end().to_string())
}

fn build_recent_failure_report(events: &[Value], limit: usize) -> Result<String, String> {
    let recent = events
        .iter()
        .rev()
        .filter(|event| {
            event_string(event, "event_type")
                .map(is_failure_event_type)
                .unwrap_or(false)
        })
        .take(limit)
        .collect::<Vec<_>>();

    if recent.is_empty() {
        return Err("no state failures found".to_string());
    }

    let mut class_counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut retryable = 0usize;
    let mut rows = Vec::new();
    for event in &recent {
        let taxonomy = classify_failure_event(event);
        *class_counts.entry(taxonomy.class).or_default() += 1;
        if taxonomy.retryable {
            retryable += 1;
        }
        rows.push(format_failure_event_value(event));
    }

    let classes = class_counts
        .iter()
        .map(|(class, count)| format!("{class}={count}"))
        .collect::<Vec<_>>()
        .join(", ");
    let mut out = String::new();
    out.push_str("State failures\n");
    out.push_str(&format!("  recent events: {}\n", recent.len()));
    out.push_str(&format!("  retryable:     {retryable}\n"));
    out.push_str(&format!("  classes:       {classes}\n"));
    out.push_str("\nRecent failure events\n");
    for row in rows {
        out.push_str("  ");
        out.push_str(&row);
        out.push('\n');
    }
    Ok(out.trim_end().to_string())
}

fn classify_failure_event(event: &Value) -> FailureTaxonomy {
    let kind = event
        .get("event_type")
        .or_else(|| event.get("kind"))
        .and_then(|v| v.as_str())
        .unwrap_or("FailureObserved");
    let payload = event.get("payload").unwrap_or(&Value::Null);
    match kind {
        "JsonOutputFailure" => taxonomy("json_output", "model_output", true),
        "ToolSchemaFailure" => taxonomy("tool_schema", "model_output", true),
        _ if payload_mentions(payload, &["context"])
            && payload_mentions(payload, &["missing", "miss", "omitted"]) =>
        {
            taxonomy("context_miss", "harness", true)
        }
        _ if payload_mentions(
            payload,
            &["permission denied", "forbidden", "approval denied"],
        ) =>
        {
            taxonomy("permission", "user_or_policy", false)
        }
        _ if payload_mentions(
            payload,
            &[
                "timeout",
                "timed out",
                "rate limit",
                "429",
                "503",
                "502",
                "network",
                "connection reset",
            ],
        ) =>
        {
            taxonomy("transport", "provider_or_network", true)
        }
        _ if payload_has_source(
            payload,
            &[
                "eval_gate",
                "eval_fixture_task",
                "eval_fixture_agent_attempt",
            ],
        ) =>
        {
            taxonomy("eval", "harness_eval", false)
        }
        _ if payload_has_source(payload, &["harness_patch_apply", "harness_patch_rollback"]) => {
            taxonomy("harness_patch", "harness", false)
        }
        _ if payload_mentions(payload, &["fim", "deepseek_fim"]) => {
            taxonomy("fim", "harness", true)
        }
        _ if payload_has_source(payload, &["tool"]) || payload.get("tool_name").is_some() => {
            taxonomy("tool_execution", "tool", true)
        }
        _ if payload_has_source(payload, &["input_rejected"]) => {
            taxonomy("input_rejected", "user_or_policy", false)
        }
        _ => taxonomy("unknown", "harness", false),
    }
}

fn classify_failure_payload(kind: &str, payload: &Value) -> FailureTaxonomy {
    classify_failure_event(&serde_json::json!({
        "event_type": kind,
        "payload": payload,
    }))
}

fn taxonomy(class: &'static str, owner: &'static str, retryable: bool) -> FailureTaxonomy {
    FailureTaxonomy {
        class,
        owner,
        retryable,
    }
}

fn payload_has_source(payload: &Value, sources: &[&str]) -> bool {
    ["source", "operation", "tool_name", "mode", "task"]
        .iter()
        .filter_map(|key| payload.get(*key).and_then(Value::as_str))
        .any(|value| {
            let normalized = value.to_ascii_lowercase();
            sources.iter().any(|source| normalized.contains(source))
        })
}

fn payload_mentions(payload: &Value, needles: &[&str]) -> bool {
    let text = failure_payload_text(payload);
    needles.iter().any(|needle| text.contains(needle))
}

fn failure_payload_text(payload: &Value) -> String {
    let mut text = String::new();
    collect_payload_text(payload, &mut text);
    text.to_ascii_lowercase()
}

fn collect_payload_text(value: &Value, out: &mut String) {
    match value {
        Value::String(text) => {
            out.push(' ');
            out.push_str(text);
        }
        Value::Array(items) => {
            for item in items {
                collect_payload_text(item, out);
            }
        }
        Value::Object(object) => {
            for value in object.values() {
                collect_payload_text(value, out);
            }
        }
        _ => {}
    }
}

fn build_policy_report(events: &[Value], limit: usize) -> Result<String, String> {
    let mut rows = Vec::new();
    for event in events.iter().rev() {
        let event_type = event_string(event, "event_type").unwrap_or("Unknown");
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        match event_type {
            "ContextBuilt" => {
                let policy = payload_str(&payload, "context_policy").unwrap_or("-");
                let layout = payload_scalar_label(&payload, "layout_version");
                let stable = payload_string_array(&payload, "stable_prefix_blocks");
                let dynamic = payload_string_array(&payload, "dynamic_suffix_blocks");
                let included = context_included_block_names(&payload);
                let instruction_files = payload_string_array(&payload, "include_instruction_files");
                rows.push(format!(
                    "  {:<18} {:<14} context policy={} layout={} stable={} dynamic={} instructions=[{}] included=[{}]",
                    event_string(event, "event_id").unwrap_or("?"),
                    event_string(event, "run_id").unwrap_or("-"),
                    policy,
                    layout,
                    stable.len(),
                    dynamic.len(),
                    instruction_files
                        .into_iter()
                        .take(6)
                        .collect::<Vec<_>>()
                        .join(", "),
                    included.into_iter().take(6).collect::<Vec<_>>().join(", ")
                ));
            }
            "ToolSchemaFailure" => {
                let schema = payload_str(&payload, "schema_name")
                    .or_else(|| payload_str(&payload, "tool_name"))
                    .unwrap_or("-");
                let schema_version = payload_scalar_label(&payload, "schema_version");
                let action = payload_str(&payload, "repair_action").unwrap_or("-");
                rows.push(format!(
                    "  {:<18} {:<14} schema name={} version={} valid={} repair_action={}",
                    event_string(event, "event_id").unwrap_or("?"),
                    event_string(event, "run_id").unwrap_or("-"),
                    schema,
                    schema_version,
                    payload
                        .get("valid")
                        .and_then(|value| value.as_bool())
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    action
                ));
            }
            _ => {}
        }
        if rows.len() >= limit {
            break;
        }
    }

    if rows.is_empty() {
        return Err("no context policy or tool schema events found".to_string());
    }

    let mut out = String::new();
    out.push_str("State policies\n");
    for row in rows {
        out.push_str(&row);
        out.push('\n');
    }
    Ok(out.trim_end().to_string())
}

fn build_failure_fix_report(
    events: &[Value],
    class_filter: Option<&str>,
    limit: usize,
) -> Result<String, String> {
    let mut failures_by_id = BTreeMap::new();
    let mut patch_proposals = BTreeMap::new();
    for event in events {
        let event_type = event_string(event, "event_type").unwrap_or("Unknown");
        if is_failure_event_type(event_type) {
            if let Some(event_id) = event_string(event, "event_id") {
                failures_by_id.insert(event_id.to_string(), event);
            }
        }
        if event_type == "PatchProposed" {
            if let Some(patch_id) = event.get("payload").and_then(extract_patch_id) {
                patch_proposals.insert(patch_id.to_string(), event);
            }
        }
    }

    let mut rows = Vec::new();
    for event in events.iter().rev() {
        if event_string(event, "event_type") != Some("PatchPromoted") {
            continue;
        }
        let payload = event.get("payload").unwrap_or(&Value::Null);
        let Some(patch_id) = extract_patch_id(payload) else {
            continue;
        };
        let proposal_payload = patch_proposals
            .get(patch_id)
            .and_then(|event| event.get("payload"))
            .unwrap_or(payload);
        let evidence_event_ids = {
            let direct = evidence_ids(payload);
            if direct.is_empty() {
                evidence_ids(proposal_payload)
            } else {
                direct
            }
        };

        let mut classes = BTreeSet::new();
        let mut evidence_preview = Vec::new();
        for evidence_id in &evidence_event_ids {
            let Some(failure) = failures_by_id.get(evidence_id) else {
                continue;
            };
            let taxonomy = classify_failure_event(failure);
            classes.insert(taxonomy.class.to_string());
            if evidence_preview.len() < 3 {
                let signal = failure
                    .get("payload")
                    .map(failure_signal)
                    .unwrap_or_else(|| "-".to_string());
                evidence_preview.push(format!("{evidence_id}: {}", preview_line(&signal, 80)));
            }
        }

        if classes.is_empty() {
            continue;
        }
        if let Some(filter) = class_filter {
            if !classes.iter().any(|class| class == filter) {
                continue;
            }
        }

        let intent = payload_str(proposal_payload, "intent")
            .or_else(|| payload_str(payload, "intent"))
            .or_else(|| payload_str(payload, "reason"))
            .unwrap_or("promoted harness patch");
        let reason = payload_str(payload, "reason")
            .or_else(|| payload_str(payload, "rationale"))
            .unwrap_or("-");
        let evals = patch_outcome_evidence(events, patch_id)
            .into_iter()
            .filter(|row| row.starts_with("eval "))
            .take(2)
            .collect::<Vec<_>>();
        rows.push(format!(
            "  {:<24} classes=[{}] patch={} kind={} risk={} intent={} promoted_reason={} evidence=[{}]{}",
            event_string(event, "event_id").unwrap_or("?"),
            classes.into_iter().collect::<Vec<_>>().join(", "),
            patch_id,
            payload_str(proposal_payload, "kind").unwrap_or("-"),
            payload_str(proposal_payload, "risk_level").unwrap_or("-"),
            preview_line(intent, 80),
            preview_line(reason, 80),
            evidence_preview.join("; "),
            if evals.is_empty() {
                String::new()
            } else {
                format!(" evals=[{}]", evals.join("; "))
            }
        ));

        if rows.len() >= limit {
            break;
        }
    }

    if rows.is_empty() {
        return Err(match class_filter {
            Some(class) => format!("no promoted patch fixes found for failure class '{class}'"),
            None => "no promoted patch fixes found".to_string(),
        });
    }

    let mut out = String::new();
    out.push_str("State failure fixes\n");
    for row in rows {
        out.push_str(&row);
        out.push('\n');
    }
    Ok(out.trim_end().to_string())
}

#[derive(Debug, Clone, PartialEq)]
enum RollbackReportRow {
    Candidate {
        harness_version: String,
        patch_id: String,
        status: String,
        suite: String,
        eval_id: String,
        score: Option<f64>,
        kind: String,
        risk: String,
        base_git_commit: String,
        rollback_plan: Vec<String>,
        already_reverted: Option<String>,
    },
    Revert {
        status: String,
        patch_id: String,
        reason: String,
        reverted_commit: String,
        event_id: String,
    },
}

fn build_rollback_rows(events: &[Value], limit: usize) -> Result<Vec<RollbackReportRow>, String> {
    let patches = patch_summaries(events);
    let mut proposal_payloads = BTreeMap::<String, &Value>::new();
    let mut reverted_patch_events = BTreeMap::<String, &Value>::new();
    for event in events {
        let event_type = event_string(event, "event_type").unwrap_or("Unknown");
        let Some(payload) = event.get("payload") else {
            continue;
        };
        let Some(patch_id) = extract_patch_id(payload) else {
            continue;
        };
        match event_type {
            "PatchProposed" => {
                proposal_payloads.insert(patch_id.to_string(), payload);
            }
            "RevertPerformed" => {
                reverted_patch_events.insert(patch_id.to_string(), event);
            }
            _ => {}
        }
    }

    let mut rows = Vec::new();
    let mut seen_failed_patch_ids = BTreeSet::new();
    for event in events.iter().rev() {
        let event_type = event_string(event, "event_type").unwrap_or("Unknown");
        let Some(payload) = event.get("payload") else {
            continue;
        };
        match event_type {
            "RevertPerformed" => {
                let Some(patch_id) = extract_patch_id(payload) else {
                    continue;
                };
                rows.push(RollbackReportRow::Revert {
                    status: payload_scalar_label(payload, "status"),
                    patch_id: patch_id.to_string(),
                    reason: payload_str(payload, "reason").unwrap_or("-").to_string(),
                    reverted_commit: payload_str(payload, "reverted_commit")
                        .unwrap_or("-")
                        .to_string(),
                    event_id: event_string(event, "event_id").unwrap_or("?").to_string(),
                });
            }
            _ if is_eval_event(event, payload) && eval_failed(payload) => {
                let patch_id = payload_str(payload, "patch_id").unwrap_or("-");
                if patch_id == "-" || !seen_failed_patch_ids.insert(patch_id.to_string()) {
                    continue;
                }
                let summary = patches.get(patch_id);
                let proposal = proposal_payloads.get(patch_id).copied().unwrap_or(payload);
                let rollback_plan = payload_string_array(proposal, "rollback_plan");
                let reverted = reverted_patch_events
                    .get(patch_id)
                    .and_then(|event| event_string(event, "event_id"))
                    .map(str::to_string);
                rows.push(RollbackReportRow::Candidate {
                    harness_version: payload_str(payload, "harness_version")
                        .unwrap_or("-")
                        .to_string(),
                    patch_id: patch_id.to_string(),
                    status: eval_status_label(payload).to_string(),
                    suite: payload_str(payload, "suite").unwrap_or("-").to_string(),
                    eval_id: payload_str(payload, "eval_id").unwrap_or("-").to_string(),
                    score: payload.get("score").and_then(|value| value.as_f64()),
                    kind: summary
                        .map(|summary| summary.kind.clone())
                        .unwrap_or_else(|| "-".to_string()),
                    risk: summary
                        .map(|summary| summary.risk_level.clone())
                        .unwrap_or_else(|| "-".to_string()),
                    base_git_commit: summary
                        .map(|summary| summary.base_git_commit.clone())
                        .unwrap_or_else(|| "-".to_string()),
                    rollback_plan,
                    already_reverted: reverted,
                });
            }
            _ => {}
        }
        if rows.len() >= limit {
            break;
        }
    }

    if rows.is_empty() {
        return Err("no rollback candidates or revert events found".to_string());
    }

    Ok(rows)
}

fn build_rollback_report(events: &[Value], limit: usize) -> Result<String, String> {
    let rows = build_rollback_rows(events, limit)?;
    let mut out = String::new();
    out.push_str("State rollback candidates\n");
    for row in rows {
        out.push_str(&format_rollback_row(&row));
        out.push('\n');
    }
    Ok(out.trim_end().to_string())
}

fn format_rollback_row(row: &RollbackReportRow) -> String {
    match row {
        RollbackReportRow::Candidate {
            harness_version,
            patch_id,
            status,
            suite,
            eval_id,
            score,
            kind,
            risk,
            base_git_commit,
            rollback_plan,
            already_reverted,
        } => {
            let rollback_plan = if rollback_plan.is_empty() {
                "-".to_string()
            } else {
                rollback_plan.join("; ")
            };
            format!(
                "  candidate rollback harness={harness_version} patch={patch_id} status={status} suite={suite} eval={eval_id} score={} kind={kind} risk={risk} base={base_git_commit} rollback_plan=[{rollback_plan}] already_reverted={}",
                score
                    .map(|value| format!("{value:.3}"))
                    .unwrap_or_else(|| "-".to_string()),
                already_reverted.as_deref().unwrap_or("-")
            )
        }
        RollbackReportRow::Revert {
            status,
            patch_id,
            reason,
            reverted_commit,
            event_id,
        } => format!(
            "  {status:<20} reverted patch={patch_id} reason={} reverted_commit={reverted_commit} event={event_id}",
            preview_line(reason, 90)
        ),
    }
}

fn build_rollback_payload(events: &[Value], limit: usize) -> Result<Value, String> {
    let rows = build_rollback_rows(events, limit)?;
    let candidate_count = rows
        .iter()
        .filter(|row| matches!(row, RollbackReportRow::Candidate { .. }))
        .count();
    let revert_count = rows
        .iter()
        .filter(|row| matches!(row, RollbackReportRow::Revert { .. }))
        .count();
    Ok(serde_json::json!({
        "diagnostic": "state_rollbacks",
        "limit": limit,
        "row_count": rows.len(),
        "candidate_count": candidate_count,
        "revert_count": revert_count,
        "rows": rows.iter().map(rollback_row_payload).collect::<Vec<_>>(),
    }))
}

fn rollback_row_payload(row: &RollbackReportRow) -> Value {
    match row {
        RollbackReportRow::Candidate {
            harness_version,
            patch_id,
            status,
            suite,
            eval_id,
            score,
            kind,
            risk,
            base_git_commit,
            rollback_plan,
            already_reverted,
        } => serde_json::json!({
            "kind": "candidate",
            "harness_version": harness_version,
            "patch_id": patch_id,
            "status": status,
            "suite": suite,
            "eval_id": eval_id,
            "score": score,
            "patch_kind": kind,
            "risk": risk,
            "base_git_commit": base_git_commit,
            "rollback_plan": rollback_plan,
            "already_reverted": already_reverted,
        }),
        RollbackReportRow::Revert {
            status,
            patch_id,
            reason,
            reverted_commit,
            event_id,
        } => serde_json::json!({
            "kind": "revert",
            "status": status,
            "patch_id": patch_id,
            "reason": reason,
            "reverted_commit": reverted_commit,
            "event_id": event_id,
        }),
    }
}

fn eval_failed(payload: &Value) -> bool {
    payload
        .get("passed")
        .and_then(|value| value.as_bool())
        .map(|passed| !passed)
        .unwrap_or_else(|| {
            payload
                .get("status")
                .and_then(|value| value.as_str())
                .map(|status| {
                    matches!(
                        normalize_status(status).as_str(),
                        "failed" | "failure" | "regressed"
                    )
                })
                .unwrap_or(false)
        })
}

fn eval_status_label(payload: &Value) -> String {
    payload
        .get("status")
        .and_then(|value| value.as_str())
        .map(|status| status.to_string())
        .or_else(|| {
            payload
                .get("passed")
                .and_then(|value| value.as_bool())
                .map(|passed| if passed { "passed" } else { "failed" }.to_string())
        })
        .unwrap_or_else(|| "-".to_string())
}

fn payload_scalar_label(payload: &Value, key: &str) -> String {
    payload
        .get(key)
        .and_then(|value| {
            value
                .as_str()
                .map(|text| text.to_string())
                .or_else(|| value.as_u64().map(|number| number.to_string()))
                .or_else(|| value.as_i64().map(|number| number.to_string()))
        })
        .unwrap_or_else(|| "-".to_string())
}

fn payload_string_array(payload: &Value, key: &str) -> Vec<String> {
    payload
        .get(key)
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(|text| text.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn count_json_array_items(raw: &str) -> usize {
    serde_json::from_str::<Value>(raw)
        .ok()
        .and_then(|value| value.as_array().map(Vec::len))
        .unwrap_or_default()
}

fn context_included_block_names(payload: &Value) -> Vec<String> {
    payload
        .get("included_blocks")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    item.as_str().map(|text| text.to_string()).or_else(|| {
                        item.get("name")
                            .and_then(|value| value.as_str())
                            .map(|text| text.to_string())
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn build_state_summary(events: &[Value]) -> String {
    if events.is_empty() {
        return "\
State: empty (no events recorded yet)

  Diagnostic paths:
    yyds state doctor        — full health check (events, store, projections)
    yyds state crashes       — check for startup crashes
    yyds state init          — explicit initialization (auto-initialized on first run)
    yyds state tail --limit 5  — see most recent events"
            .to_string();
    }

    let total = events.len();

    let run_completed_count = events
        .iter()
        .filter(|e| event_string(e, "event_type") == Some("RunCompleted"))
        .count();
    let run_started_count = events
        .iter()
        .filter(|e| event_string(e, "event_type") == Some("RunStarted"))
        .count();

    let mut event_types: BTreeMap<&str, usize> = BTreeMap::new();
    for e in events {
        if let Some(et) = event_string(e, "event_type") {
            *event_types.entry(et).or_insert(0) += 1;
        }
    }

    let failure_count = events
        .iter()
        .filter(|e| {
            event_string(e, "event_type")
                .map(is_failure_event_type)
                .unwrap_or(false)
        })
        .count();

    let first_ts = events.iter().filter_map(event_timestamp_ms).min();
    let last_ts = events.iter().filter_map(event_timestamp_ms).max();

    let mut out = String::new();
    out.push_str("State summary:\n");
    out.push_str(&format!("  events: {} total\n", total));
    out.push_str(&format!(
        "  runs: {} started, {} completed\n",
        run_started_count, run_completed_count
    ));
    out.push_str(&format!("  failures: {} recorded\n", failure_count));

    if let (Some(first), Some(last)) = (first_ts, last_ts) {
        out.push_str(&format!(
            "  range: {} to {}\n",
            format_timestamp_ms(first),
            format_timestamp_ms(last)
        ));
    } else {
        out.push_str("  range: (no timestamps)\n");
    }

    out.push_str("  event types seen:\n");
    for (et, count) in &event_types {
        out.push_str(&format!("    {et}: {count}\n"));
    }

    out.trim_end().to_string()
}

#[derive(Debug, Clone)]
struct OpenModelCall {
    run_id: String,
    started_event_id: String,
    model: String,
}

fn build_state_lifecycle_json(events: &[Value]) -> Value {
    let mut run_started = BTreeSet::<String>::new();
    let mut run_completed = BTreeSet::<String>::new();
    let mut model_started = 0usize;
    let mut model_completed = 0usize;
    let mut open_model_calls = BTreeMap::<String, OpenModelCall>::new();
    let mut unmatched_model_completions = Vec::<Value>::new();
    let mut last_event_by_run = BTreeMap::<String, Value>::new();

    for event in events {
        let event_type = event_string(event, "event_type").unwrap_or("Unknown");
        let run_id = event_string(event, "run_id").unwrap_or("").to_string();
        match event_type {
            "RunStarted" if !run_id.is_empty() => {
                run_started.insert(run_id.clone());
            }
            "RunCompleted" if !run_id.is_empty() => {
                run_completed.insert(run_id.clone());
            }
            "ModelCallStarted" => {
                model_started += 1;
                let event_id = event_string(event, "event_id").unwrap_or("").to_string();
                let key = model_lifecycle_key(event);
                let payload = event.get("payload").unwrap_or(&Value::Null);
                open_model_calls.insert(
                    key,
                    OpenModelCall {
                        run_id: run_id.clone(),
                        started_event_id: event_id,
                        model: payload_str(payload, "model").unwrap_or("-").to_string(),
                    },
                );
            }
            "ModelCallCompleted" => {
                model_completed += 1;
                let key = model_lifecycle_key(event);
                if open_model_calls.remove(&key).is_none() {
                    unmatched_model_completions.push(model_completion_summary(event));
                }
            }
            _ => {}
        }
        if !run_id.is_empty() {
            last_event_by_run.insert(run_id, event_summary_json(event));
        }
    }

    let incomplete_runs = run_started
        .difference(&run_completed)
        .map(|run_id| {
            serde_json::json!({
                "run_id": run_id,
                "last_event": last_event_by_run.get(run_id).cloned().unwrap_or(Value::Null),
            })
        })
        .collect::<Vec<_>>();
    let incomplete_model_calls = open_model_calls
        .values()
        .map(|open| {
            serde_json::json!({
                "run_id": if open.run_id.is_empty() { Value::Null } else { Value::String(open.run_id.clone()) },
                "started_event_id": open.started_event_id,
                "model": open.model,
                "last_event": last_event_by_run.get(&open.run_id).cloned().unwrap_or(Value::Null),
            })
        })
        .collect::<Vec<_>>();
    let incomplete_run_count = incomplete_runs.len();
    let incomplete_model_call_count = incomplete_model_calls.len();
    let unmatched_model_completion_count = unmatched_model_completions.len();
    let balanced = incomplete_run_count == 0
        && incomplete_model_call_count == 0
        && unmatched_model_completion_count == 0;

    serde_json::json!({
        "schema_version": 1,
        "source": "state_lifecycle",
        "events_considered": events.len(),
        "runs": {
            "started": run_started.len(),
            "completed": run_completed.len(),
            "incomplete": incomplete_run_count,
            "incomplete_runs": incomplete_runs,
        },
        "model_calls": {
            "started": model_started,
            "completed": model_completed,
            "incomplete": incomplete_model_call_count,
            "incomplete_runs": incomplete_model_calls,
            "unmatched_completed": unmatched_model_completion_count,
            "unmatched_completed_runs": unmatched_model_completions,
        },
        "balanced": balanced,
    })
}

fn format_state_lifecycle_report(payload: &Value) -> String {
    let runs = payload.get("runs").unwrap_or(&Value::Null);
    let model_calls = payload.get("model_calls").unwrap_or(&Value::Null);
    let mut out = String::new();
    out.push_str("State lifecycle:\n");
    out.push_str(&format!(
        "  events considered: {}\n",
        payload_u64(payload, "events_considered").unwrap_or(0)
    ));
    out.push_str(&format!(
        "  runs: {} started, {} completed, {} incomplete\n",
        payload_u64(runs, "started").unwrap_or(0),
        payload_u64(runs, "completed").unwrap_or(0),
        payload_u64(runs, "incomplete").unwrap_or(0)
    ));
    out.push_str(&format!(
        "  model calls: {} started, {} completed, {} incomplete, {} unmatched completed\n",
        payload_u64(model_calls, "started").unwrap_or(0),
        payload_u64(model_calls, "completed").unwrap_or(0),
        payload_u64(model_calls, "incomplete").unwrap_or(0),
        payload_u64(model_calls, "unmatched_completed").unwrap_or(0)
    ));
    if let Some(items) = model_calls
        .get("incomplete_runs")
        .and_then(|value| value.as_array())
        .filter(|items| !items.is_empty())
    {
        out.push_str("  incomplete model calls:\n");
        for item in items.iter().take(10) {
            let run_id = payload_str(item, "run_id").unwrap_or("-");
            let model = payload_str(item, "model").unwrap_or("-");
            let last = item
                .get("last_event")
                .map(compact_lifecycle_event)
                .unwrap_or_else(|| "last_event=-".to_string());
            out.push_str(&format!("    run={run_id} model={model} {last}\n"));
        }
    }
    out.trim_end().to_string()
}

fn model_lifecycle_key(event: &Value) -> String {
    event_string(event, "run_id")
        .filter(|run_id| !run_id.is_empty())
        .or_else(|| event_string(event, "event_id"))
        .unwrap_or("-")
        .to_string()
}

fn model_completion_summary(event: &Value) -> Value {
    let payload = event.get("payload").unwrap_or(&Value::Null);
    serde_json::json!({
        "run_id": event_string(event, "run_id"),
        "event_id": event_string(event, "event_id"),
        "model": payload_str(payload, "model"),
        "status": payload_str(payload, "status").unwrap_or("completed"),
        "error_detail": payload_str(payload, "error_detail"),
    })
}

fn event_summary_json(event: &Value) -> Value {
    let payload = event.get("payload").unwrap_or(&Value::Null);
    serde_json::json!({
        "event_id": event_string(event, "event_id"),
        "event_type": event_string(event, "event_type").unwrap_or("Unknown"),
        "timestamp_ms": event_timestamp_ms(event),
        "tool_name": payload_str(payload, "tool_name"),
        "path": payload_str(payload, "path").or_else(|| payload_str(payload, "file_path")),
        "command": payload_str(payload, "command"),
        "status": payload_str(payload, "status"),
        "error_detail": payload_str(payload, "error_detail").or_else(|| payload_str(payload, "error")),
    })
}

fn compact_lifecycle_event(event: &Value) -> String {
    let kind = payload_str(event, "event_type").unwrap_or("Unknown");
    let mut parts = vec![format!("last_event={kind}")];
    if let Some(path) = payload_str(event, "path") {
        parts.push(format!("path={path}"));
    }
    if let Some(tool_name) = payload_str(event, "tool_name") {
        parts.push(format!("tool={tool_name}"));
    }
    if let Some(command) = payload_str(event, "command") {
        parts.push(format!("command={}", preview_line(command, 80)));
    }
    if let Some(status) = payload_str(event, "status") {
        parts.push(format!("status={status}"));
    }
    if let Some(error_detail) = payload_str(event, "error_detail") {
        parts.push(format!("error={}", preview_line(error_detail, 80)));
    }
    parts.join(" ")
}

fn build_why_report(events: &[Value], id: &str) -> Result<String, String> {
    let Some(target) = find_target_event(events, id) else {
        let mut err = String::new();
        err.push_str(&format!("no state event found for '{id}'\n"));
        err.push('\n');
        err.push_str(&build_state_summary(events));
        err.push('\n');
        err.push('\n');
        // Add actionable guidance
        let run_completed_count = events
            .iter()
            .filter(|e| event_string(e, "event_type") == Some("RunCompleted"))
            .count();
        let failure_count = events
            .iter()
            .filter(|e| {
                event_string(e, "event_type")
                    .map(is_failure_event_type)
                    .unwrap_or(false)
            })
            .count();
        let run_started = events
            .iter()
            .any(|e| event_string(e, "event_type") == Some("RunStarted"));

        if events.is_empty() || (run_completed_count == 0 && !run_started) {
            err.push_str("State recording is active but no sessions have completed yet.\n");
            err.push_str(
                "Diagnostics become available after 2\u{2013}3 completed evolution sessions.",
            );
            if id == "last-failure" {
                err.push_str("\n\nTry 'yoyo state tail --limit 5' to inspect recent events.");
            }
        } else if run_completed_count == 0 && run_started {
            err.push_str("A session is currently in progress.\n");
            err.push_str(
                "Diagnostics and failure data become available after the session completes.",
            );
            if id == "last-failure" {
                // Show incomplete run IDs and timestamps for actionable diagnostics
                let completed_ids: std::collections::HashSet<&str> = events
                    .iter()
                    .filter(|e| event_string(e, "event_type") == Some("RunCompleted"))
                    .filter_map(|e| event_string(e, "run_id"))
                    .collect();
                let incomplete_runs: Vec<&Value> = events
                    .iter()
                    .filter(|e| event_string(e, "event_type") == Some("RunStarted"))
                    .filter(|e| {
                        let rid = event_string(e, "run_id");
                        rid.is_none_or(|r| !completed_ids.contains(r))
                    })
                    .take(5)
                    .collect();
                if !incomplete_runs.is_empty() {
                    err.push_str("\n\nIncomplete run(s):");
                    for ev in incomplete_runs {
                        let rid = event_string(ev, "run_id").unwrap_or("?");
                        let ts = event_timestamp_ms(ev)
                            .map(format_timestamp_ms)
                            .unwrap_or_else(|| "unknown".to_string());
                        err.push_str(&format!("\n  run={rid}  started={ts}"));
                    }
                }
                err.push_str("\n\nTry 'yoyo state tail --limit 5' to follow live events.");
            }
        } else if failure_count == 0 {
            err.push_str(&format!(
                "{} successful session{} recorded. No failure data to diagnose.",
                run_completed_count,
                if run_completed_count == 1 { "" } else { "s" }
            ));
            if id == "last-failure" {
                err.push_str(
                    "\n\nTry 'yoyo state crashes --limit 10' for crashed or incomplete sessions, or 'yoyo state why last-crash' to diagnose the latest crash.",
                );
            }
        } else {
            err.push_str(&format!(
                "{} failure{} recorded but not enough to cluster into patterns.\nCheck back after 2\u{2013}3 more sessions.",
                failure_count,
                if failure_count == 1 { "" } else { "s" }
            ));
        }
        return Err(err);
    };

    let event_id = event_string(target, "event_id").unwrap_or("?");
    let event_type = event_string(target, "event_type").unwrap_or("Unknown");
    let run_id = event_string(target, "run_id").unwrap_or("-");
    let trace_id = event_string(target, "trace_id").unwrap_or("-");
    let payload = target.get("payload").cloned().unwrap_or(Value::Null);

    let mut out = String::new();
    out.push_str(&format!("State why: {id}\n"));
    out.push_str(&format!("event: {event_type} {event_id}\n"));
    out.push_str(&format!("run:   {run_id}\n"));
    out.push_str(&format!("trace: {trace_id}\n"));
    out.push_str(&format!(
        "payload: {}\n",
        serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
    ));

    if is_failure_event_type(event_type) {
        append_failure_explanation(&mut out, events, target, &payload);
    } else if matches!(event_type, "CommitCreated" | "RevertPerformed") {
        append_commit_explanation(&mut out, events, target, &payload);
    } else if event_type == "DecisionRecorded"
        && payload_str(&payload, "decision_type") == Some("release_gate")
    {
        append_release_gate_explanation(&mut out, event_id, &payload);
    } else if event_type == "DecisionRecorded"
        && payload_str(&payload, "decision_type") == Some("deepseek_json_output_check")
    {
        append_json_output_check_explanation(&mut out, event_id, &payload);
    } else if event_type == "DecisionRecorded"
        && payload_str(&payload, "decision_type") == Some("deepseek_strict_tool_call_check")
    {
        append_strict_tool_call_check_explanation(&mut out, event_id, &payload);
    } else if event_type == "DecisionRecorded"
        && payload_str(&payload, "decision_type") == Some("deepseek_transport_policy_check")
    {
        append_transport_policy_check_explanation(&mut out, event_id, &payload);
    } else if event_type == "DecisionRecorded"
        && payload_str(&payload, "decision_type") == Some("deepseek_thinking_protocol_check")
    {
        append_thinking_protocol_check_explanation(&mut out, event_id, &payload);
    } else if event_type == "DecisionRecorded"
        && payload_str(&payload, "decision_type") == Some("deepseek_streaming_protocol_check")
    {
        append_streaming_protocol_check_explanation(&mut out, event_id, &payload);
    } else if event_type == "DecisionRecorded" && payload.get("promotion_decision").is_some() {
        append_promotion_gate_explanation(&mut out, event_id, &payload);
    }

    let parents = parent_ids(target);
    if !parents.is_empty() {
        out.push_str("\nParents:\n");
        for parent_id in parents {
            match find_target_event(events, &parent_id) {
                Some(parent) => out.push_str(&format!("  {}\n", format_event_value(parent))),
                None => out.push_str(&format!("  {parent_id} (missing)\n")),
            }
        }
    }

    let evidence_ids = why_evidence_ids(&payload);
    if !evidence_ids.is_empty() {
        out.push_str("\nEvidence:\n");
        for evidence_id in evidence_ids {
            match find_target_event(events, &evidence_id) {
                Some(evidence) => out.push_str(&format!("  {}\n", format_event_value(evidence))),
                None => out.push_str(&format!("  {evidence_id} (missing)\n")),
            }
        }
    }

    let related = related_events(events, target);
    if !related.is_empty() {
        out.push_str("\nRelated timeline:\n");
        for event in related {
            out.push_str(&format!("  {}\n", format_event_value(event)));
        }
    }

    Ok(out.trim_end().to_string())
}

fn append_release_gate_explanation(out: &mut String, event_id: &str, payload: &Value) {
    let decision = payload_str(payload, "decision").unwrap_or("-");
    let reason = payload_str(payload, "reason")
        .or_else(|| payload_str(payload, "rationale"))
        .unwrap_or("-");
    let suite = payload_str(payload, "suite").unwrap_or("-");
    let max_age = payload_u64(payload, "max_age_hours")
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let last_eval = payload_str(payload, "last_eval_id").unwrap_or("-");
    let last_eval_status = payload_str(payload, "last_eval_status").unwrap_or("-");
    let last_eval_dirty = payload_bool_label(payload, "last_eval_git_dirty");
    let last_eval_fixture_tasks = payload_u64(payload, "last_eval_fixture_task_count")
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let last_eval_fixture_commands = payload_u64(payload, "last_eval_fixture_command_count")
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let last_eval_fixture_risk_labels =
        payload_u64_count_map(payload, "last_eval_fixture_risk_labels");
    let last_eval_model_route_tasks = payload_u64_count_map(payload, "last_eval_model_route_tasks");
    let min_fixture_risk_labels = payload_u64_count_map(payload, "min_fixture_risk_labels");
    let last_eval_mutation_scope_failures_value =
        payload_u64(payload, "last_eval_mutation_scope_failures");
    let last_eval_mutation_scope_failures = last_eval_mutation_scope_failures_value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let last_eval_unexpected_changed_files_value =
        payload_u64(payload, "last_eval_unexpected_changed_files");
    let last_eval_unexpected_changed_files = last_eval_unexpected_changed_files_value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let min_fixture_tasks_value = payload_u64(payload, "min_fixture_task_count");
    let min_fixture_commands_value = payload_u64(payload, "min_fixture_command_count");
    let min_fixture_tasks = min_fixture_tasks_value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let min_fixture_commands = min_fixture_commands_value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let fixture_breadth_satisfied = payload_bool_label(payload, "fixture_breadth_satisfied");
    let fixture_risk_satisfied = payload_bool_label(payload, "fixture_risk_satisfied");
    let stale = payload_bool_label(payload, "stale");
    let replay_failures = payload_u64(payload, "replay_failures_after_eval").unwrap_or(0);
    let replay_command = payload_str(payload, "replay_command");
    let missing_gates = payload_string_array(payload, "missing_required_gates");
    let require_protocol = payload_bool_label(payload, "require_protocol");
    let protocol_eval = payload_str(payload, "protocol_eval_id").unwrap_or("-");
    let protocol_status = payload_str(payload, "protocol_eval_status").unwrap_or("-");
    let protocol_dirty = payload_bool_label(payload, "protocol_eval_git_dirty");
    let protocol_stale = payload_bool_label(payload, "protocol_stale");
    let protocol_older = payload_bool_label(payload, "protocol_older_than_eval");
    let protocol_counts = format_protocol_check_metric_evidence(payload)
        .map(|counts| format!(" {counts}"))
        .unwrap_or_default();
    let source_audit = payload_bool(payload, "source_provenance_passed")
        .map(|passed| if passed { "passed" } else { "blocked" })
        .unwrap_or("-");
    let source_findings = payload_u64(payload, "source_provenance_findings").unwrap_or(0);
    let source_scan = payload_str(payload, "source_provenance_scan_source").unwrap_or("-");
    let source_scanned = payload_u64(payload, "source_provenance_scanned_files").unwrap_or(0);
    let source_skipped = payload_u64(payload, "source_provenance_skipped_files").unwrap_or(0);
    let source_finding_summaries =
        payload_string_array(payload, "source_provenance_finding_summaries");

    out.push_str("\nExplanation:\n");
    out.push_str(&format!("  release decision: {decision}\n"));
    out.push_str(&format!("  reason: {reason}\n"));
    out.push_str(&format!("  suite: {suite} max_age_hours={max_age}\n"));
    out.push_str(&format!(
        "  latest eval: id={last_eval} status={last_eval_status} dirty={last_eval_dirty} stale={stale} fixture_tasks={last_eval_fixture_tasks} fixture_commands={last_eval_fixture_commands}\n"
    ));
    if !last_eval_fixture_risk_labels.is_empty() {
        out.push_str(&format!(
            "  fixture risks: {}\n",
            format_u64_count_map(&last_eval_fixture_risk_labels)
        ));
    }
    if !last_eval_model_route_tasks.is_empty() {
        out.push_str(&format!(
            "  model routes: {}\n",
            format_u64_count_map(&last_eval_model_route_tasks)
        ));
    }
    if !min_fixture_risk_labels.is_empty() {
        out.push_str(&format!(
            "  fixture risk gate: min={} satisfied={fixture_risk_satisfied}\n",
            format_u64_count_map(&min_fixture_risk_labels)
        ));
    }
    out.push_str(&format!(
        "  fixture agent scope: failures={last_eval_mutation_scope_failures} unexpected_files={last_eval_unexpected_changed_files}\n"
    ));
    out.push_str(&format!(
        "  fixture breadth gate: min_tasks={min_fixture_tasks} min_commands={min_fixture_commands} satisfied={fixture_breadth_satisfied}\n"
    ));
    out.push_str(&format!(
        "  protocol gate: required={require_protocol} id={protocol_eval} status={protocol_status} dirty={protocol_dirty} stale={protocol_stale} older_than_suite={protocol_older}{protocol_counts}\n"
    ));
    if !missing_gates.is_empty() {
        out.push_str(&format!("  missing gates: {}\n", missing_gates.join(", ")));
    }
    out.push_str(&format!(
        "  replay failures: {replay_failures}{}\n",
        replay_command
            .map(|command| format!(" command={command}"))
            .unwrap_or_default()
    ));
    out.push_str(&format!(
        "  source audit: {source_audit} findings={source_findings} source={source_scan} scanned={source_scanned} skipped={source_skipped}\n"
    ));
    for finding in source_finding_summaries.iter().take(5) {
        out.push_str(&format!("    source finding: {finding}\n"));
    }

    out.push_str("  next actions:\n");
    if replay_failures > 0 {
        out.push_str(&format!(
            "    replay state failures: {}\n",
            replay_command.unwrap_or("yoyo eval replay --from-state --limit 5")
        ));
        out.push_str(&format!(
            "    inspect replay failure signals: yoyo state graph signals {event_id} --depth 2\n"
        ));
        out.push_str(&format!(
            "    inspect replay failure evidence: yoyo state graph failures {event_id} --depth 2\n"
        ));
        out.push_str("    review recent state failures: yoyo state failures --recent\n");
    }
    if !missing_gates.is_empty() {
        if last_eval != "-" {
            out.push_str(&format!(
                "    inspect required-gate eval: yoyo eval report {last_eval}\n"
            ));
        }
        out.push_str(&format!(
            "    inspect required-gate signals: yoyo state graph signals {event_id} --depth 2\n"
        ));
        out.push_str(&format!(
            "    inspect required-gate evidence: yoyo state graph evidence {event_id} --depth 2\n"
        ));
        out.push_str(&format!(
            "    inspect required-gate decision: yoyo state graph decisions {event_id} --depth 2\n"
        ));
        out.push_str(&format!(
            "    rerun missing gates: {}\n",
            missing_gates.join(" && ")
        ));
        if suite != "-" {
            out.push_str(&format!(
                "    rerun required-gate suite eval: yoyo eval run --suite {suite}\n"
            ));
        }
    }
    if source_audit == "blocked" {
        out.push_str(&format!(
            "    inspect source audit policy: yoyo state graph policies {event_id} --depth 2\n"
        ));
        out.push_str(&format!(
            "    inspect source audit evidence: yoyo state graph evidence {event_id} --depth 2\n"
        ));
        out.push_str(&format!(
            "    inspect source audit signals: yoyo state graph signals {event_id} --depth 2\n"
        ));
        out.push_str(&format!(
            "    inspect source audit impact: yoyo state graph impact {event_id} --depth 2\n"
        ));
        out.push_str(&format!(
            "    inspect source audit decision: yoyo state graph decisions {event_id} --depth 2\n"
        ));
        out.push_str(&format!(
            "    rerun source audit release gate: yoyo eval release-gate --suite {suite} --max-age-hours {max_age}\n"
        ));
    }
    let release_fixture_agent_scope_blocked = release_fixture_agent_scope_blocked(reason)
        || last_eval_mutation_scope_failures_value.unwrap_or_default() > 0
        || last_eval_unexpected_changed_files_value.unwrap_or_default() > 0;
    if release_fixture_agent_scope_blocked {
        if last_eval != "-" {
            out.push_str(&format!(
                "    inspect fixture agent scope eval: yoyo eval report {last_eval}\n"
            ));
        }
        out.push_str(&format!(
            "    inspect fixture agent scope signals: yoyo state graph signals {event_id} --depth 2\n"
        ));
        out.push_str(&format!(
            "    inspect fixture agent changed files: yoyo state graph files {event_id} --depth 2\n"
        ));
        out.push_str(&format!(
            "    inspect fixture agent scope evidence: yoyo state graph evidence {event_id} --depth 2\n"
        ));
        if suite != "-" {
            out.push_str(&format!(
                "    rerun fixture suite eval: yoyo eval run --suite {suite}\n"
            ));
        }
    }
    let release_fixture_coverage_blocked = release_fixture_coverage_blocked(reason)
        || fixture_breadth_satisfied == "no"
        || fixture_risk_satisfied == "no";
    if release_fixture_coverage_blocked {
        if last_eval != "-" {
            out.push_str(&format!(
                "    inspect fixture coverage eval: yoyo eval report {last_eval}\n"
            ));
        }
        out.push_str(&format!(
            "    inspect fixture coverage signals: yoyo state graph signals {event_id} --depth 2\n"
        ));
        out.push_str(&format!(
            "    inspect fixture coverage evidence: yoyo state graph evidence {event_id} --depth 2\n"
        ));
        out.push_str(&format!(
            "    inspect fixture coverage evals: yoyo state graph evals {event_id} --depth 2\n"
        ));
        if suite != "-" {
            out.push_str(&format!(
                "    rerun fixture coverage suite: yoyo eval run --suite {suite}\n"
            ));
        }
    }
    let release_failed_eval_blocked =
        release_failed_eval_blocked(reason) || release_eval_status_failed(last_eval_status);
    if release_failed_eval_blocked {
        if last_eval != "-" {
            out.push_str(&format!(
                "    inspect failed suite eval: yoyo eval report {last_eval}\n"
            ));
        }
        out.push_str(&format!(
            "    inspect failed release eval signals: yoyo state graph signals {event_id} --depth 2\n"
        ));
        out.push_str(&format!(
            "    inspect failed release eval evidence: yoyo state graph evals {event_id} --depth 2\n"
        ));
        if suite != "-" {
            out.push_str(&format!(
                "    rerun suite eval: yoyo eval run --suite {suite}\n"
            ));
        }
    }
    let release_dirty_eval_blocked = release_dirty_worktree_blocked(reason)
        || last_eval_dirty == "yes"
        || protocol_dirty == "yes";
    if release_dirty_eval_blocked {
        if last_eval != "-" && last_eval_dirty == "yes" {
            out.push_str(&format!(
                "    inspect dirty suite eval: yoyo eval report {last_eval}\n"
            ));
        }
        if protocol_eval != "-" && protocol_dirty == "yes" {
            out.push_str(&format!(
                "    inspect dirty protocol eval: yoyo eval report {protocol_eval}\n"
            ));
        }
        out.push_str(&format!(
            "    inspect dirty release eval signals: yoyo state graph signals {event_id} --depth 2\n"
        ));
        out.push_str(&format!(
            "    inspect dirty release eval evidence: yoyo state graph evals {event_id} --depth 2\n"
        ));
        if suite != "-" && last_eval_dirty == "yes" {
            out.push_str(&format!(
                "    rerun clean suite eval: yoyo eval run --suite {suite}\n"
            ));
        }
        if protocol_dirty == "yes" {
            out.push_str(
                "    rerun clean protocol eval: yoyo eval run --suite protocol-deepseek\n",
            );
        }
    }
    let release_stale_eval_blocked = release_stale_eval_blocked(reason) || stale == "yes";
    if release_stale_eval_blocked {
        if last_eval != "-" {
            out.push_str(&format!(
                "    inspect stale suite eval: yoyo eval report {last_eval}\n"
            ));
        }
        out.push_str(&format!(
            "    inspect stale release eval signals: yoyo state graph signals {event_id} --depth 2\n"
        ));
        out.push_str(&format!(
            "    inspect stale release eval evidence: yoyo state graph evals {event_id} --depth 2\n"
        ));
        if suite != "-" {
            out.push_str(&format!(
                "    rerun fresh suite eval: yoyo eval run --suite {suite}\n"
            ));
        }
    }
    if require_protocol == "yes"
        && (reason.contains("protocol eval")
            || protocol_eval == "-"
            || protocol_dirty == "yes"
            || protocol_stale == "yes"
            || protocol_older == "yes")
    {
        if protocol_eval != "-" {
            out.push_str(&format!(
                "    inspect protocol evidence: yoyo state graph evals {event_id} --depth 2\n"
            ));
            out.push_str(&format!(
                "    inspect protocol signals: yoyo state graph signals {event_id} --depth 2\n"
            ));
        }
        if protocol_dirty != "yes" {
            out.push_str("    rerun protocol eval: yoyo eval run --suite protocol-deepseek\n");
        }
    }
    let min_fixture_task_flag = min_fixture_tasks_value
        .map(|value| format!(" --min-fixture-tasks {value}"))
        .unwrap_or_default();
    let min_fixture_command_flag = min_fixture_commands_value
        .map(|value| format!(" --min-fixture-commands {value}"))
        .unwrap_or_default();
    let min_fixture_risk_flags = min_fixture_risk_labels
        .iter()
        .map(|(label, value)| format!(" --min-fixture-{label}-risk {value}"))
        .collect::<String>();
    out.push_str(&format!(
        "    rerun release gate: yoyo eval release-gate --suite {suite} --max-age-hours {max_age}{min_fixture_task_flag}{min_fixture_command_flag}{min_fixture_risk_flags}\n"
    ));
}

fn release_dirty_worktree_blocked(reason: &str) -> bool {
    let normalized = reason.to_ascii_lowercase();
    normalized.contains("dirty worktree")
}

fn release_stale_eval_blocked(reason: &str) -> bool {
    let normalized = reason.to_ascii_lowercase();
    normalized.contains("older than max age")
}

fn release_fixture_agent_scope_blocked(reason: &str) -> bool {
    let normalized = reason.to_ascii_lowercase();
    normalized.contains("fixture agent mutation-scope")
}

fn release_fixture_coverage_blocked(reason: &str) -> bool {
    let normalized = reason.to_ascii_lowercase();
    normalized.contains("fixture suite breadth") || normalized.contains("fixture risk coverage")
}

fn release_failed_eval_blocked(reason: &str) -> bool {
    reason.eq_ignore_ascii_case("latest eval did not pass")
}

fn release_eval_status_failed(status: &str) -> bool {
    matches!(
        status.to_ascii_lowercase().as_str(),
        "failed" | "fail" | "failure"
    )
}

fn payload_bool_label(payload: &Value, key: &str) -> &'static str {
    match payload_bool(payload, key) {
        Some(true) => "yes",
        Some(false) => "no",
        None => "-",
    }
}

fn append_json_output_check_explanation(out: &mut String, event_id: &str, payload: &Value) {
    let decision = payload_str(payload, "decision").unwrap_or("-");
    let check = payload_str(payload, "check").unwrap_or("-");
    let schema = payload_str(payload, "schema_name").unwrap_or("-");
    let attempts = payload_u64(payload, "attempt_count")
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let retry_used = payload_bool_label(payload, "retry_used");
    let statuses = payload_string_array(payload, "attempt_statuses");

    out.push_str("\nExplanation:\n");
    out.push_str(&format!(
        "  JSON output check: {check} decision={decision}\n"
    ));
    out.push_str(&format!(
        "  schema: {schema} attempts={attempts} retry_used={retry_used}\n"
    ));
    if !statuses.is_empty() {
        out.push_str(&format!("  attempt statuses: {}\n", statuses.join(", ")));
    }
    out.push_str("  next actions:\n");
    out.push_str(&format!(
        "    inspect JSON output evidence: yoyo state graph evidence {event_id} --depth 2\n"
    ));
    if schema != "-" {
        out.push_str(&format!(
            "    inspect schema lineage: yoyo state graph policies {event_id} --depth 2\n"
        ));
    }
    out.push_str(
        "    rerun JSON protocol check: yoyo deepseek json-check --input '{\"ok\":true}' --json\n",
    );
}

fn append_strict_tool_call_check_explanation(out: &mut String, event_id: &str, payload: &Value) {
    let decision = payload_str(payload, "decision").unwrap_or("-");
    let check = payload_str(payload, "check").unwrap_or("-");
    let model = payload_str(payload, "model").unwrap_or("-");
    let thinking = payload_str(payload, "thinking").unwrap_or("-");
    let effort = payload_str(payload, "reasoning_effort").unwrap_or("-");
    let schema_count = payload_u64(payload, "schema_count")
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let selected_tool_count = payload_u64(payload, "selected_tool_count")
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let schema_names = payload_string_array(payload, "schema_names");
    let selected_tool_names = payload_string_array(payload, "selected_tool_names");

    out.push_str("\nExplanation:\n");
    out.push_str(&format!(
        "  strict tool-call check: {check} decision={decision}\n"
    ));
    out.push_str(&format!(
        "  request policy: model={model} thinking={thinking} effort={effort}\n"
    ));
    out.push_str(&format!(
        "  schemas: total={schema_count} selected={selected_tool_count}\n"
    ));
    if !selected_tool_names.is_empty() {
        out.push_str(&format!(
            "  selected tools: {}\n",
            selected_tool_names.join(", ")
        ));
    }
    if !schema_names.is_empty() {
        out.push_str(&format!(
            "  validated schemas: {}\n",
            schema_names
                .iter()
                .take(8)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    out.push_str("  next actions:\n");
    out.push_str(&format!(
        "    inspect strict tool-call evidence: yoyo state graph evidence {event_id} --depth 2\n"
    ));
    out.push_str(&format!(
        "    inspect strict schema lineage: yoyo state graph policies {event_id} --depth 2\n"
    ));
    out.push_str(
        "    rerun strict tool-call protocol check: yoyo deepseek test-tool-call --record --json\n",
    );
}

fn append_transport_policy_check_explanation(out: &mut String, event_id: &str, payload: &Value) {
    let decision = payload_str(payload, "decision").unwrap_or("-");
    let check = payload_str(payload, "check").unwrap_or("-");
    let transport_class = payload_str(payload, "transport_class").unwrap_or("-");
    let status = payload_u64(payload, "status")
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let attempt = payload_u64(payload, "attempt")
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let max_retries = payload_u64(payload, "max_retries")
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let retryable = payload_bool_label(payload, "retryable");
    let backoff = payload_u64(payload, "next_backoff_ms")
        .map(|value| format!("{value}ms"))
        .unwrap_or_else(|| "-".to_string());
    let reason = payload_str(payload, "reason").unwrap_or("-");

    out.push_str("\nExplanation:\n");
    out.push_str(&format!(
        "  transport policy check: {check} decision={decision}\n"
    ));
    out.push_str(&format!(
        "  class: {transport_class} status={status} retryable={retryable} attempt={attempt}/{max_retries} backoff={backoff}\n"
    ));
    out.push_str(&format!("  reason: {reason}\n"));
    out.push_str("  next actions:\n");
    out.push_str(&format!(
        "    inspect transport evidence: yoyo state graph evidence {event_id} --depth 2\n"
    ));
    out.push_str(&format!(
        "    inspect transport policy lineage: yoyo state graph policies {event_id} --depth 2\n"
    ));
    out.push_str(
        "    rerun transport protocol check: yoyo deepseek transport-check --status 429 --error 'rate limit' --record --json\n",
    );
}

fn append_thinking_protocol_check_explanation(out: &mut String, event_id: &str, payload: &Value) {
    let decision = payload_str(payload, "decision").unwrap_or("-");
    let check = payload_str(payload, "check").unwrap_or("-");
    let diagnostic_source = payload_str(payload, "diagnostic_source").unwrap_or("-");
    let probe = payload.get("probe").unwrap_or(&Value::Null);
    let messages = probe
        .get("message_count")
        .and_then(Value::as_u64)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let assistant_turns = probe
        .get("assistant_tool_call_turns")
        .and_then(Value::as_u64)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let reasoning_present = probe
        .get("assistant_tool_call_turns_with_reasoning_content")
        .and_then(Value::as_u64)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let reasoning_missing = probe
        .get("assistant_tool_call_turns_missing_reasoning_content")
        .and_then(Value::as_u64)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let tool_results = probe
        .get("tool_result_turns")
        .and_then(Value::as_u64)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());

    out.push_str("\nExplanation:\n");
    out.push_str(&format!(
        "  thinking protocol check: {check} decision={decision}\n"
    ));
    out.push_str(&format!(
        "  probe: source={diagnostic_source} messages={messages} assistant_tool_calls={assistant_turns} reasoning_present={reasoning_present} reasoning_missing={reasoning_missing} tool_results={tool_results}\n"
    ));
    out.push_str("  next actions:\n");
    out.push_str(&format!(
        "    inspect thinking evidence: yoyo state graph evidence {event_id} --depth 2\n"
    ));
    out.push_str(&format!(
        "    inspect thinking protocol policy: yoyo state graph policies {event_id} --depth 2\n"
    ));
    out.push_str(
        "    rerun thinking protocol check: yoyo deepseek test-thinking --record --json\n",
    );
}

fn append_streaming_protocol_check_explanation(out: &mut String, event_id: &str, payload: &Value) {
    let decision = payload_str(payload, "decision").unwrap_or("-");
    let check = payload_str(payload, "check").unwrap_or("-");
    let finish_reason = payload_str(payload, "finish_reason").unwrap_or("-");
    let content_chars = payload_u64(payload, "content_chars")
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let reasoning_content_chars = payload_u64(payload, "reasoning_content_chars")
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let tool_call_count = payload_u64(payload, "tool_call_count")
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let input_tokens = payload_u64(payload, "input_tokens")
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let output_tokens = payload_u64(payload, "output_tokens")
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let cache_hit_tokens = payload_u64(payload, "cache_hit_tokens")
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let cache_miss_tokens = payload_u64(payload, "cache_miss_tokens")
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());

    out.push_str("\nExplanation:\n");
    out.push_str(&format!(
        "  streaming protocol check: {check} decision={decision}\n"
    ));
    out.push_str(&format!(
        "  stream: finish={finish_reason} content_chars={content_chars} reasoning_chars={reasoning_content_chars} tool_calls={tool_call_count}\n"
    ));
    out.push_str(&format!(
        "  usage: input={input_tokens} output={output_tokens} cache_hit={cache_hit_tokens} cache_miss={cache_miss_tokens}\n"
    ));
    out.push_str("  next actions:\n");
    out.push_str(&format!(
        "    inspect streaming evidence: yoyo state graph evidence {event_id} --depth 2\n"
    ));
    out.push_str(&format!(
        "    inspect streaming protocol policy: yoyo state graph policies {event_id} --depth 2\n"
    ));
    out.push_str(
        "    rerun streaming protocol check: yoyo deepseek stream-check --record --json\n",
    );
}

fn append_promotion_gate_explanation(out: &mut String, event_id: &str, payload: &Value) {
    let decision_type = payload_str(payload, "decision_type").unwrap_or("-");
    let decision = payload_str(payload, "decision").unwrap_or("-");
    let reason = payload_str(payload, "reason")
        .or_else(|| payload_str(payload, "rationale"))
        .unwrap_or("-");
    let patch_id = payload_str(payload, "patch_id").unwrap_or("-");
    let eval_id = payload_str(payload, "eval_id").unwrap_or("-");
    let promotion = payload.get("promotion_decision").unwrap_or(&Value::Null);
    let eligible = promotion
        .get("eligible")
        .and_then(Value::as_bool)
        .map(|eligible| if eligible { "yes" } else { "no" })
        .unwrap_or("-");
    let criterion = promotion
        .get("criterion")
        .and_then(Value::as_str)
        .unwrap_or("-");
    let promotion_reason = promotion
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or("-");
    let baseline_eval = promotion
        .get("baseline_eval_id")
        .and_then(Value::as_str)
        .unwrap_or("-");
    let candidate_eval = promotion
        .get("candidate_eval_id")
        .and_then(Value::as_str)
        .unwrap_or("-");
    let protocol_eval = promotion
        .get("protocol_eval_id")
        .and_then(Value::as_str)
        .unwrap_or("-");
    let promotion_suite = promotion
        .get("suite")
        .and_then(Value::as_str)
        .unwrap_or("-");

    out.push_str("\nExplanation:\n");
    out.push_str(&format!(
        "  promotion decision: type={decision_type} decision={decision} eligible={eligible} criterion={criterion}\n"
    ));
    out.push_str(&format!("  reason: {reason}\n"));
    if promotion_reason != "-" && promotion_reason != reason {
        out.push_str(&format!("  promotion reason: {promotion_reason}\n"));
    }
    out.push_str(&format!(
        "  patch: {patch_id} eval={eval_id} baseline={baseline_eval} candidate={candidate_eval} protocol={protocol_eval}\n"
    ));
    if let Some(metrics) = format_promotion_metric_evidence(payload) {
        out.push_str(&format!("  {metrics}\n"));
    }
    if let Some(safety) = format_promotion_safety_gate_summary(payload) {
        out.push_str(&format!("  {safety}\n"));
    }
    let safety_gate = payload.get("safety_gate").unwrap_or(&Value::Null);
    let safety_reason = safety_gate
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or("-");
    let safety_requires_human = safety_gate
        .get("requires_human_approval")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    out.push_str("  next actions:\n");
    if protocol_eval != "-" {
        out.push_str(&format!(
            "    inspect protocol lineage: yoyo state graph signals {event_id} --depth 2\n"
        ));
        out.push_str(&format!(
            "    inspect protocol eval evidence: yoyo state graph evals {event_id} --depth 2\n"
        ));
    }
    if promotion_fixture_suite_blocked(reason, promotion_reason) {
        if baseline_eval != "-" && candidate_eval != "-" {
            out.push_str(&format!(
                "    compare fixture evals: yoyo eval compare {baseline_eval} {candidate_eval}\n"
            ));
        }
        out.push_str(&format!(
            "    inspect promotion decision: yoyo state graph decisions {event_id} --depth 2\n"
        ));
        out.push_str(&format!(
            "    inspect promotion fixture evidence: yoyo state graph evidence {event_id} --depth 2\n"
        ));
    }
    if promotion_dirty_worktree_blocked(reason, promotion_reason) {
        let dirty_scope = promotion_dirty_worktree_scope(reason, promotion_reason);
        if candidate_eval != "-" && dirty_scope_mentions(&dirty_scope, "candidate") {
            out.push_str(&format!(
                "    inspect dirty candidate eval: yoyo eval report {candidate_eval}\n"
            ));
        }
        if baseline_eval != "-" && dirty_scope_mentions(&dirty_scope, "baseline") {
            out.push_str(&format!(
                "    inspect dirty baseline eval: yoyo eval report {baseline_eval}\n"
            ));
        }
        if protocol_eval != "-" && dirty_scope_mentions(&dirty_scope, "protocol") {
            out.push_str(&format!(
                "    inspect dirty protocol eval: yoyo eval report {protocol_eval}\n"
            ));
        }
        out.push_str(&format!(
            "    inspect dirty eval signals: yoyo state graph signals {event_id} --depth 2\n"
        ));
        out.push_str(&format!(
            "    inspect dirty eval evidence: yoyo state graph evals {event_id} --depth 2\n"
        ));
        if promotion_suite != "-"
            && (dirty_scope_mentions(&dirty_scope, "candidate")
                || dirty_scope_mentions(&dirty_scope, "baseline"))
        {
            out.push_str(&format!(
                "    rerun clean fixture eval: yoyo eval run --suite {promotion_suite}\n"
            ));
        }
    }
    if promotion_required_gate_blocked(reason, promotion_reason) {
        let gate_scope = promotion_required_gate_scope(reason, promotion_reason);
        if candidate_eval != "-" && gate_scope_mentions(&gate_scope, "candidate") {
            out.push_str(&format!(
                "    inspect candidate gate evidence: yoyo eval report {candidate_eval}\n"
            ));
        }
        if baseline_eval != "-" && gate_scope_mentions(&gate_scope, "baseline") {
            out.push_str(&format!(
                "    inspect baseline gate evidence: yoyo eval report {baseline_eval}\n"
            ));
        }
        let missing_gates = promotion_missing_required_gates(reason, promotion_reason);
        if !missing_gates.is_empty() {
            out.push_str(&format!(
                "    rerun missing promotion gates: {}\n",
                missing_gates.join(" && ")
            ));
        }
        out.push_str(&format!(
            "    inspect promotion gate evidence: yoyo state graph evals {event_id} --depth 2\n"
        ));
        out.push_str(&format!(
            "    inspect promotion decision: yoyo state graph decisions {event_id} --depth 2\n"
        ));
        if promotion_suite != "-" {
            out.push_str(&format!(
                "    rerun promotion fixture eval: yoyo eval run --suite {promotion_suite}\n"
            ));
        }
    }
    if promotion_budget_gate_blocked(reason, promotion_reason) {
        if baseline_eval != "-" && candidate_eval != "-" {
            out.push_str(&format!(
                "    compare budget evidence: yoyo eval compare {baseline_eval} {candidate_eval}\n"
            ));
        }
        if baseline_eval != "-" {
            out.push_str(&format!(
                "    inspect baseline budget eval: yoyo eval report {baseline_eval}\n"
            ));
        }
        if candidate_eval != "-" {
            out.push_str(&format!(
                "    inspect candidate budget eval: yoyo eval report {candidate_eval}\n"
            ));
        }
        out.push_str(&format!(
            "    inspect promotion budget decision: yoyo state graph decisions {event_id} --depth 2\n"
        ));
        out.push_str(&format!(
            "    inspect promotion budget evals: yoyo state graph evals {event_id} --depth 2\n"
        ));
        if promotion_suite != "-" {
            out.push_str(&format!(
                "    rerun budget fixture eval: yoyo eval run --suite {promotion_suite}\n"
            ));
        }
    }
    if promotion_harness_quality_blocked(reason, promotion_reason) {
        if baseline_eval != "-" && candidate_eval != "-" {
            out.push_str(&format!(
                "    compare harness quality: yoyo eval compare {baseline_eval} {candidate_eval}\n"
            ));
        }
        if candidate_eval != "-" {
            out.push_str(&format!(
                "    inspect candidate quality eval: yoyo eval report {candidate_eval}\n"
            ));
        }
        out.push_str(&format!(
            "    inspect promotion quality decision: yoyo state graph decisions {event_id} --depth 2\n"
        ));
        out.push_str(&format!(
            "    inspect promotion quality evals: yoyo state graph evals {event_id} --depth 2\n"
        ));
        if promotion_suite != "-" {
            out.push_str(&format!(
                "    rerun quality fixture eval: yoyo eval run --suite {promotion_suite}\n"
            ));
        }
    }
    if promotion_rollback_plan_blocked(reason, promotion_reason, safety_reason) {
        if patch_id != "-" {
            out.push_str(&format!(
                "    inspect rollback plan: yoyo state patches show {patch_id}\n"
            ));
            out.push_str(&format!(
                "    propose rollback-safe patch update: yoyo evolve harness propose --from-state {event_id} --rollback-plan <step>\n"
            ));
        }
        out.push_str(&format!(
            "    inspect promotion safety decision: yoyo state graph decisions {event_id} --depth 2\n"
        ));
    }
    if promotion_human_approval_blocked(
        reason,
        promotion_reason,
        safety_reason,
        safety_requires_human,
    ) {
        if patch_id != "-" {
            out.push_str(&format!(
                "    inspect promotion approval scope: yoyo state patches show {patch_id}\n"
            ));
            out.push_str(&format!(
                "    request promotion approval: yoyo evolve harness approve {patch_id} --reason <text>\n"
            ));
        }
        out.push_str(&format!(
            "    inspect promotion approval evidence: yoyo state graph decisions {event_id} --depth 2\n"
        ));
    }
    if promotion_reason.contains("protocol eval") || reason.contains("protocol eval") {
        out.push_str("    rerun protocol eval: yoyo eval run --suite protocol-deepseek\n");
    }
    if patch_id != "-" {
        out.push_str(&format!(
            "    review patch lifecycle: yoyo state lineage {patch_id}\n"
        ));
    }
}

fn promotion_fixture_suite_blocked(reason: &str, promotion_reason: &str) -> bool {
    [reason, promotion_reason].iter().any(|text| {
        let normalized = text.to_ascii_lowercase();
        normalized.contains("fixture")
            && (normalized.contains("suite")
                || normalized.contains("breadth")
                || normalized.contains("risk-label")
                || normalized.contains("risk label"))
    })
}

fn promotion_dirty_worktree_blocked(reason: &str, promotion_reason: &str) -> bool {
    [reason, promotion_reason].iter().any(|text| {
        let normalized = text.to_ascii_lowercase();
        normalized.contains("dirty worktree")
            && (normalized.contains("eval") || normalized.contains("reproducibility"))
    })
}

fn promotion_dirty_worktree_scope(reason: &str, promotion_reason: &str) -> String {
    format!("{reason} {promotion_reason}").to_ascii_lowercase()
}

fn dirty_scope_mentions(scope: &str, label: &str) -> bool {
    scope.contains(label)
        || (!scope.contains("candidate")
            && !scope.contains("baseline")
            && !scope.contains("protocol"))
}

fn promotion_required_gate_blocked(reason: &str, promotion_reason: &str) -> bool {
    [reason, promotion_reason].iter().any(|text| {
        let normalized = text.to_ascii_lowercase();
        normalized.contains("missing required gate evidence")
    })
}

fn promotion_required_gate_scope(reason: &str, promotion_reason: &str) -> String {
    format!("{reason} {promotion_reason}").to_ascii_lowercase()
}

fn gate_scope_mentions(scope: &str, label: &str) -> bool {
    scope.contains(label) || (!scope.contains("candidate") && !scope.contains("baseline"))
}

fn promotion_missing_required_gates(reason: &str, promotion_reason: &str) -> Vec<String> {
    [reason, promotion_reason]
        .iter()
        .filter_map(|text| text.split_once("missing required gate evidence: "))
        .flat_map(|(_, gates)| gates.split(','))
        .map(str::trim)
        .filter(|gate| !gate.is_empty())
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn promotion_budget_gate_blocked(reason: &str, promotion_reason: &str) -> bool {
    [reason, promotion_reason].iter().any(|text| {
        let normalized = text.to_ascii_lowercase();
        normalized.contains("budget gate")
            || normalized.contains("token_total increases")
            || normalized.contains("cost_usd increases")
            || normalized.contains("token cost")
    })
}

fn promotion_harness_quality_blocked(reason: &str, promotion_reason: &str) -> bool {
    [reason, promotion_reason].iter().any(|text| {
        let normalized = text.to_ascii_lowercase();
        normalized.contains("regresses harness quality gate")
    })
}

fn promotion_rollback_plan_blocked(
    reason: &str,
    promotion_reason: &str,
    safety_reason: &str,
) -> bool {
    [reason, promotion_reason, safety_reason]
        .iter()
        .any(|text| {
            let normalized = text.to_ascii_lowercase();
            normalized.contains("rollback plan") && normalized.contains("no")
        })
}

fn promotion_human_approval_blocked(
    reason: &str,
    promotion_reason: &str,
    safety_reason: &str,
    safety_requires_human: bool,
) -> bool {
    safety_requires_human
        && [reason, promotion_reason, safety_reason]
            .iter()
            .any(|text| {
                let normalized = text.to_ascii_lowercase();
                normalized.contains("humanapprovalreceived")
                    || (normalized.contains("human approval") && normalized.contains("promotion"))
                    || normalized.contains("fresh human approval")
            })
}

fn append_commit_explanation(out: &mut String, events: &[Value], target: &Value, payload: &Value) {
    let event_type = event_string(target, "event_type").unwrap_or("Unknown");
    let commit = payload_str(payload, "commit")
        .or_else(|| payload_str(payload, "commit_id"))
        .unwrap_or("-");
    let branch = payload_str(payload, "branch").unwrap_or("-");
    let files = payload_string_array(payload, "files");

    out.push_str("\nExplanation:\n");
    match event_type {
        "CommitCreated" => {
            out.push_str(&format!("  commit: {commit}\n"));
            out.push_str(&format!("  branch: {branch}\n"));
            if !files.is_empty() {
                out.push_str(&format!("  modified files: {}\n", files.join(", ")));
            }
            let reverts = commit_revert_events(events, commit);
            if !reverts.is_empty() {
                out.push_str("  later reverts:\n");
                for event in reverts {
                    let payload = event.get("payload").unwrap_or(&Value::Null);
                    out.push_str(&format!(
                        "    {} reason={} revert_commit={}\n",
                        format_event_value(event),
                        preview_line(payload_str(payload, "reason").unwrap_or("-"), 120),
                        payload_str(payload, "commit")
                            .or_else(|| payload_str(payload, "commit_id"))
                            .unwrap_or("-")
                    ));
                }
            }
        }
        "RevertPerformed" => {
            let reverted = payload_str(payload, "reverted_commit").unwrap_or("-");
            out.push_str(&format!("  revert commit: {commit}\n"));
            out.push_str(&format!("  reverted commit: {reverted}\n"));
            out.push_str(&format!("  branch: {branch}\n"));
            if let Some(original) = find_commit_event(events, reverted) {
                out.push_str(&format!("  original: {}\n", format_event_value(original)));
            }
            if !files.is_empty() {
                out.push_str(&format!("  affected files: {}\n", files.join(", ")));
            }
        }
        _ => {}
    }
}

fn append_failure_explanation(out: &mut String, events: &[Value], target: &Value, payload: &Value) {
    let target_event_id = event_string(target, "event_id").unwrap_or("");
    let target_run_id = event_string(target, "run_id");
    let source = payload
        .get("source")
        .or_else(|| payload.get("operation"))
        .and_then(|value| value.as_str())
        .unwrap_or("unknown");
    let error = payload
        .get("error_preview")
        .or_else(|| payload.get("error"))
        .and_then(|value| value.as_str())
        .unwrap_or("-");

    out.push_str("\nExplanation:\n");
    out.push_str(&format!("  failure source: {source}\n"));
    out.push_str(&format!("  failure signal: {error}\n"));

    if let Some(context) = latest_context_for_failure(events, target) {
        out.push_str(&format!(
            "  active context: {}\n",
            format_context_summary(context)
        ));
    }

    let similar_failures = similar_historical_failures(events, target, payload);
    if !similar_failures.is_empty() {
        out.push_str("  similar failures:\n");
        for event in &similar_failures {
            out.push_str(&format!(
                "    {}\n",
                format_failure_similarity_summary(event)
            ));
        }
    }

    let hypotheses = failure_hypotheses(events, target_event_id, target_run_id);
    if !hypotheses.is_empty() {
        out.push_str("  hypotheses:\n");
        for event in hypotheses {
            out.push_str(&format!("    {}\n", format_hypothesis_summary(event)));
        }
    }

    let similar_failure_ids = similar_failures
        .iter()
        .filter_map(|event| event_string(event, "event_id").map(|id| id.to_string()))
        .collect::<Vec<_>>();
    let patches = failure_patch_candidates(events, target_event_id, &similar_failure_ids);
    if !patches.is_empty() {
        out.push_str("  candidate patches:\n");
        for event in &patches {
            out.push_str(&format!("    {}\n", format_patch_candidate_summary(event)));
            let patch_id = event
                .get("payload")
                .and_then(extract_patch_id)
                .unwrap_or("");
            for evidence in patch_outcome_evidence(events, patch_id) {
                out.push_str(&format!("      {}\n", evidence));
            }
        }
    }

    append_failure_next_actions(out, events, target, &patches);
}

fn append_failure_next_actions(
    out: &mut String,
    events: &[Value],
    target: &Value,
    patches: &[&Value],
) {
    let event_id = event_string(target, "event_id").unwrap_or("last-failure");
    let trace_id = event_string(target, "trace_id").unwrap_or("-");
    let taxonomy = classify_failure_event(target);
    let mut actions = BTreeSet::new();

    if trace_id != "-" && !trace_id.is_empty() {
        actions.insert(format!("inspect trace: yoyo state trace {trace_id}"));
    }
    actions.insert("replay related failures: yoyo eval replay --from-state --limit 5".to_string());

    if patches.is_empty() {
        let kind = match taxonomy.class {
            "context_miss" => "context_policy",
            "transport" => "transport",
            "tool_schema" => "tool_schema",
            "permission" => "permission_policy",
            "fim" => "repair_policy",
            _ => "repair_policy",
        };
        actions.insert(format!(
            "propose harness patch: yoyo evolve harness propose --from-state {event_id} --kind {kind}"
        ));
    } else {
        for patch in patches.iter().take(3) {
            let Some(patch_id) = patch.get("payload").and_then(extract_patch_id) else {
                continue;
            };
            actions.insert(format!("inspect patch: yoyo state patches show {patch_id}"));
            match latest_patch_eval(events, patch_id) {
                Some((eval_id, status)) if status == "passed" => {
                    actions.insert(format!(
                        "compare/promote patch: yoyo evolve harness promote {patch_id} --baseline-eval <baseline-eval> --candidate-eval {eval_id}"
                    ));
                }
                Some((_eval_id, status)) if status == "failed" || status == "error" => {
                    actions.insert(format!(
                        "review rollback: yoyo evolve harness rollback {patch_id} --reason <text>"
                    ));
                }
                Some((eval_id, _status)) => {
                    actions.insert(format!("inspect eval: yoyo eval report {eval_id}"));
                }
                None => {
                    actions.insert(format!(
                        "evaluate patch: yoyo evolve harness eval {patch_id}"
                    ));
                }
            }
        }
    }

    if actions.is_empty() {
        return;
    }
    out.push_str("  next actions:\n");
    for action in actions.into_iter().take(6) {
        out.push_str(&format!("    {action}\n"));
    }
}

fn latest_patch_eval(events: &[Value], patch_id: &str) -> Option<(String, String)> {
    events.iter().rev().find_map(|event| {
        let payload = event.get("payload")?;
        if extract_patch_id(payload) != Some(patch_id) || !is_eval_event(event, payload) {
            return None;
        }
        let eval_id = payload_str(payload, "eval_id")?.to_string();
        let status = payload_str(payload, "status")
            .or_else(|| {
                payload
                    .get("passed")
                    .and_then(Value::as_bool)
                    .map(|passed| if passed { "passed" } else { "failed" })
            })
            .unwrap_or("unknown")
            .to_ascii_lowercase();
        Some((eval_id, status))
    })
}

fn latest_context_for_failure<'a>(events: &'a [Value], target: &Value) -> Option<&'a Value> {
    let run_id = event_string(target, "run_id")?;
    let target_ts = event_timestamp(target);
    events.iter().rev().find(|event| {
        event_string(event, "event_type") == Some("ContextBuilt")
            && event_string(event, "run_id") == Some(run_id)
            && event_timestamp(event) <= target_ts
    })
}

fn format_context_summary(event: &Value) -> String {
    let payload = event.get("payload").unwrap_or(&Value::Null);
    let policy = payload
        .get("context_policy")
        .and_then(|value| value.as_str())
        .unwrap_or("-");
    let layout = payload
        .get("layout_version")
        .and_then(|value| {
            value
                .as_str()
                .map(|text| text.to_string())
                .or_else(|| value.as_u64().map(|number| number.to_string()))
        })
        .unwrap_or_else(|| "-".to_string());
    let blocks = context_included_block_names(payload)
        .into_iter()
        .take(8)
        .collect::<Vec<_>>()
        .join(", ");
    let instruction_files = payload_string_array(payload, "include_instruction_files")
        .into_iter()
        .take(6)
        .collect::<Vec<_>>()
        .join(", ");
    format!("policy={policy} layout={layout} instructions=[{instruction_files}] blocks=[{blocks}]")
}

fn failure_hypotheses<'a>(
    events: &'a [Value],
    failure_event_id: &str,
    run_id: Option<&str>,
) -> Vec<&'a Value> {
    events
        .iter()
        .filter(|event| {
            event_string(event, "event_type") == Some("HypothesisCreated")
                && event
                    .get("payload")
                    .map(|payload| {
                        payload
                            .get("failure_event_id")
                            .and_then(|value| value.as_str())
                            .map(|id| id == failure_event_id)
                            .unwrap_or(false)
                            || evidence_ids(payload)
                                .iter()
                                .any(|id| id == failure_event_id)
                            || run_id
                                .map(|run_id| event_string(event, "run_id") == Some(run_id))
                                .unwrap_or(false)
                    })
                    .unwrap_or(false)
        })
        .take(5)
        .collect()
}

fn similar_historical_failures<'a>(
    events: &'a [Value],
    target: &Value,
    payload: &Value,
) -> Vec<&'a Value> {
    let target_event_id = event_string(target, "event_id").unwrap_or("");
    let target_ts = event_timestamp(target);
    let target_source = failure_source(payload);
    let target_tokens = failure_signature_tokens(payload);
    let mut scored = Vec::new();

    for event in events {
        let event_id = event_string(event, "event_id").unwrap_or("");
        if event_id == target_event_id {
            continue;
        }
        let kind = event_string(event, "event_type").unwrap_or("");
        if !is_failure_event_type(kind) || event_timestamp(event) > target_ts {
            continue;
        }
        let candidate_payload = event.get("payload").unwrap_or(&Value::Null);
        let mut score = 0usize;
        if failure_source(candidate_payload) == target_source {
            score += 2;
        }
        let candidate_tokens = failure_signature_tokens(candidate_payload);
        score += target_tokens
            .intersection(&candidate_tokens)
            .filter(|token| token.len() >= 4)
            .count();
        if score > 0 {
            scored.push((score, event_timestamp(event), event));
        }
    }

    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
    scored
        .into_iter()
        .take(3)
        .map(|(_, _, event)| event)
        .collect()
}

fn format_hypothesis_summary(event: &Value) -> String {
    let payload = event.get("payload").unwrap_or(&Value::Null);
    let id = payload
        .get("hypothesis_id")
        .and_then(|value| value.as_str())
        .or_else(|| event_string(event, "event_id"))
        .unwrap_or("-");
    let summary = payload
        .get("summary")
        .or_else(|| payload.get("hypothesis"))
        .and_then(|value| value.as_str())
        .unwrap_or("-");
    let confidence = payload
        .get("confidence")
        .and_then(|value| value.as_f64())
        .map(|value| format!(" confidence={value:.2}"))
        .unwrap_or_default();
    format!("{id}: {summary}{confidence}")
}

fn failure_patch_candidates<'a>(
    events: &'a [Value],
    failure_event_id: &str,
    similar_failure_ids: &[String],
) -> Vec<&'a Value> {
    let mut seen = BTreeSet::new();
    let mut matches = Vec::new();
    events
        .iter()
        .filter(|event| {
            event_string(event, "event_type") == Some("PatchProposed")
                && event
                    .get("payload")
                    .map(|payload| {
                        let evidence = evidence_ids(payload);
                        evidence.iter().any(|id| id == failure_event_id)
                            || evidence
                                .iter()
                                .any(|id| similar_failure_ids.iter().any(|similar| similar == id))
                    })
                    .unwrap_or(false)
        })
        .filter(|event| {
            let key = event
                .get("payload")
                .and_then(extract_patch_id)
                .or_else(|| event_string(event, "event_id"))
                .unwrap_or("");
            seen.insert(key.to_string())
        })
        .take(5)
        .for_each(|event| matches.push(event));
    matches
}

fn format_patch_candidate_summary(event: &Value) -> String {
    let payload = event.get("payload").unwrap_or(&Value::Null);
    let patch_id = extract_patch_id(payload).unwrap_or("-");
    let kind = payload
        .get("kind")
        .and_then(|value| value.as_str())
        .unwrap_or("-");
    let risk = payload
        .get("risk_level")
        .and_then(|value| value.as_str())
        .unwrap_or("-");
    let intent = payload
        .get("intent")
        .and_then(|value| value.as_str())
        .unwrap_or("-");
    format!("{patch_id}: kind={kind} risk={risk} intent={intent}")
}

fn patch_outcome_evidence(events: &[Value], patch_id: &str) -> Vec<String> {
    if patch_id.is_empty() {
        return Vec::new();
    }
    let mut rows = Vec::new();
    for event in events {
        let Some(payload) = event.get("payload") else {
            continue;
        };
        if extract_patch_id(payload) != Some(patch_id) {
            continue;
        }
        let event_type = event_string(event, "event_type").unwrap_or("Unknown");
        match event_type {
            "PatchEvaluated" | "TestCompleted" if is_eval_event(event, payload) => {
                let eval_id = payload_str(payload, "eval_id").unwrap_or("-");
                let suite = payload_str(payload, "suite").unwrap_or("-");
                let status = payload_str(payload, "status")
                    .or_else(|| {
                        payload
                            .get("passed")
                            .and_then(|value| value.as_bool())
                            .map(|passed| if passed { "passed" } else { "failed" })
                    })
                    .unwrap_or("-");
                let score = payload
                    .get("score")
                    .and_then(|value| value.as_f64())
                    .map(|value| format!(" score={value:.3}"))
                    .unwrap_or_default();
                rows.push(format!(
                    "eval {eval_id}: suite={suite} status={status}{score}"
                ));
            }
            "PatchPromoted" | "PatchRejected" | "HumanApprovalRequested" => {
                let status = event_type_patch_status(event_type).unwrap_or(event_type);
                let reason = payload_str(payload, "reason")
                    .or_else(|| payload_str(payload, "rationale"))
                    .unwrap_or("-");
                rows.push(format!("decision: {status} reason={reason}"));
                if let Some(safety) = format_promotion_safety_gate_summary(payload) {
                    rows.push(safety);
                }
            }
            "DecisionRecorded" => {
                if let Some(summary) = format_promotion_decision_summary(payload) {
                    rows.push(summary);
                    if let Some(metrics) = format_promotion_metric_evidence(payload) {
                        rows.push(metrics);
                    }
                    if let Some(safety) = format_promotion_safety_gate_summary(payload) {
                        rows.push(safety);
                    }
                } else {
                    let decision = payload_str(payload, "decision").unwrap_or("-");
                    let reason = payload_str(payload, "reason")
                        .or_else(|| payload_str(payload, "rationale"))
                        .unwrap_or("-");
                    rows.push(format!("decision: {decision} reason={reason}"));
                }
            }
            _ => {}
        }
    }
    rows.into_iter().rev().take(8).collect()
}

fn format_promotion_safety_gate_summary(payload: &Value) -> Option<String> {
    let gate = payload.get("safety_gate")?;
    let allowed = gate
        .get("allowed")
        .and_then(Value::as_bool)
        .map(|allowed| allowed.to_string())
        .unwrap_or_else(|| "-".to_string());
    let required = gate
        .get("requires_human_approval")
        .and_then(Value::as_bool)
        .map(|required| required.to_string())
        .unwrap_or_else(|| "-".to_string());
    let reason = gate.get("reason").and_then(Value::as_str).unwrap_or("-");
    let approvals = payload_string_array(payload, "approval_event_ids");
    let approval_summary = if approvals.is_empty() {
        "-".to_string()
    } else {
        approvals.join(",")
    };
    Some(format!(
        "safety: allowed={allowed} human_approval_required={required} approvals={approval_summary} reason={}",
        preview_line(reason, 140)
    ))
}

fn format_promotion_decision_summary(payload: &Value) -> Option<String> {
    let decision = payload.get("promotion_decision")?;
    let eligible = decision
        .get("eligible")
        .and_then(Value::as_bool)
        .map(|eligible| if eligible { "eligible" } else { "not_eligible" })
        .or_else(|| payload_str(payload, "decision"))
        .unwrap_or("-");
    let criterion = decision
        .get("criterion")
        .and_then(Value::as_str)
        .unwrap_or("-");
    let baseline = decision
        .get("baseline_eval_id")
        .and_then(Value::as_str)
        .unwrap_or("-");
    let candidate = decision
        .get("candidate_eval_id")
        .and_then(Value::as_str)
        .unwrap_or("-");
    let protocol = decision
        .get("protocol_eval_id")
        .and_then(Value::as_str)
        .unwrap_or("-");
    let reason = decision
        .get("reason")
        .and_then(Value::as_str)
        .or_else(|| payload_str(payload, "rationale"))
        .or_else(|| payload_str(payload, "reason"))
        .unwrap_or("-");
    Some(format!(
        "promotion: {eligible} criterion={criterion} baseline={baseline} candidate={candidate} protocol={protocol} reason={}",
        preview_line(reason, 140)
    ))
}

fn format_promotion_metric_evidence(payload: &Value) -> Option<String> {
    let evidence = payload.get("promotion_decision")?.get("metric_evidence")?;
    let mut parts = Vec::new();

    if let Some(token_total) = evidence.get("token_total") {
        if let Some(part) = format_metric_evidence_part("token_total", token_total) {
            parts.push(part);
        }
    }
    if let Some(fixture_suite) = evidence.get("fixture_suite") {
        if let Some(part) = format_promotion_fixture_suite_evidence(fixture_suite) {
            parts.push(part);
        }
    }
    if let Some(model_routes) = evidence.get("model_route_tasks") {
        if let Some(part) = format_promotion_model_route_evidence(model_routes) {
            parts.push(part);
        }
    }
    if let Some(protocol_eval) = evidence.get("protocol_eval") {
        if let Some(part) = format_promotion_protocol_eval_evidence(protocol_eval) {
            parts.push(part);
        }
    }

    if let Some(metrics) = evidence.get("metrics").and_then(Value::as_array) {
        let preferred = [
            "cost_usd",
            "cost_per_successful_task_usd",
            "input_tokens",
            "output_tokens",
            "cache_hit_ratio",
            "model_calls",
            "tool_calls",
            "failures",
            "context_misses",
            "rollback_count",
        ];
        for name in preferred {
            if let Some(metric) = metrics.iter().find(|metric| {
                metric
                    .get("metric")
                    .and_then(Value::as_str)
                    .map(|candidate| candidate == name)
                    .unwrap_or(false)
            }) {
                if let Some(part) = format_metric_evidence_part(name, metric) {
                    parts.push(part);
                }
            }
            if parts.len() >= 4 {
                break;
            }
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(format!("promotion metrics: {}", parts.join(", ")))
    }
}

fn format_promotion_protocol_eval_evidence(value: &Value) -> Option<String> {
    let eval_id = value.get("eval_id").and_then(Value::as_str)?;
    let status = value.get("status").and_then(Value::as_str).unwrap_or("-");
    let dirty = value
        .get("git_dirty")
        .and_then(Value::as_bool)
        .map(bool_word)
        .unwrap_or("-");
    let created_at = value
        .get("created_at_ms")
        .and_then(Value::as_u64)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let checks = format_protocol_check_metric_evidence(value)
        .map(|checks| format!(" {checks}"))
        .unwrap_or_default();
    Some(format!(
        "protocol_eval id={eval_id} status={status} dirty={dirty} created_at_ms={created_at}{checks}"
    ))
}

fn format_protocol_check_metric_evidence(value: &Value) -> Option<String> {
    let checks = value
        .get("protocol_checks")
        .or_else(|| value.get("protocol_check_counts"))?;
    let total = payload_value_u64(checks.get("total")?)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let passes = checks
        .get("passes")
        .and_then(payload_value_u64)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    Some(format!(
        "checks={passes}/{total} strict={} thinking={} stream={} json={} transport={}",
        protocol_check_count_label(checks, "strict"),
        protocol_check_count_label(checks, "thinking"),
        protocol_check_count_label(checks, "stream"),
        protocol_check_count_label(checks, "json"),
        protocol_check_count_label(checks, "transport")
    ))
}

fn release_protocol_check_count(payload: &Value, key: &str) -> Option<u64> {
    payload
        .get("protocol_check_counts")
        .and_then(|counts| counts.get(key))
        .and_then(payload_value_u64)
}

fn protocol_check_count_label(checks: &Value, key: &str) -> String {
    checks
        .get(key)
        .and_then(payload_value_u64)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn bool_word(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn format_promotion_model_route_evidence(value: &Value) -> Option<String> {
    let baseline = value
        .get("baseline")
        .map(value_u64_count_map)
        .unwrap_or_default();
    let candidate = value
        .get("candidate")
        .map(value_u64_count_map)
        .unwrap_or_default();
    if baseline.is_empty() && candidate.is_empty() {
        return None;
    }
    Some(format!(
        "model_routes [{}] -> [{}]",
        format_u64_count_map(&baseline),
        format_u64_count_map(&candidate)
    ))
}

fn format_promotion_fixture_suite_evidence(value: &Value) -> Option<String> {
    let baseline = value.get("baseline")?;
    let candidate = value.get("candidate")?;
    let task_part = format_fixture_suite_metric("tasks", baseline, candidate, "task_count");
    let command_part =
        format_fixture_suite_metric("commands", baseline, candidate, "command_count");
    let risk_part = format_fixture_suite_risk_labels(baseline, candidate);
    let parts = [task_part, command_part, risk_part]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    if parts.is_empty() {
        None
    } else {
        Some(format!("fixture_suite {}", parts.join(" ")))
    }
}

fn format_fixture_suite_metric(
    label: &str,
    baseline: &Value,
    candidate: &Value,
    key: &str,
) -> Option<String> {
    let baseline_value = baseline.get(key).and_then(payload_value_u64);
    let candidate_value = candidate.get(key).and_then(payload_value_u64);
    if baseline_value.is_none() && candidate_value.is_none() {
        return None;
    }
    Some(format!(
        "{label}={} -> {}",
        baseline_value
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        candidate_value
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string())
    ))
}

fn format_fixture_suite_risk_labels(baseline: &Value, candidate: &Value) -> Option<String> {
    let baseline_risks = value_u64_count_map(baseline.get("risk_labels")?);
    let candidate_risks = value_u64_count_map(candidate.get("risk_labels")?);
    if baseline_risks.is_empty() && candidate_risks.is_empty() {
        return None;
    }
    Some(format!(
        "risks=[{}] -> [{}]",
        format_u64_count_map(&baseline_risks),
        format_u64_count_map(&candidate_risks)
    ))
}

fn format_metric_evidence_part(name: &str, value: &Value) -> Option<String> {
    let baseline = value.get("baseline")?;
    let candidate = value.get("candidate")?;
    if baseline.is_null() || candidate.is_null() {
        return None;
    }
    let delta = value.get("delta").filter(|delta| !delta.is_null());
    Some(format!(
        "{}={} ({} -> {})",
        name,
        format_metric_delta_value(delta),
        format_metric_value(baseline),
        format_metric_value(candidate)
    ))
}

fn format_metric_delta_value(value: Option<&Value>) -> String {
    let Some(value) = value else {
        return "-".to_string();
    };
    if let Some(number) = value.as_i64() {
        return format!("{number:+}");
    }
    if let Some(number) = value.as_u64() {
        return format!("+{number}");
    }
    if let Some(number) = value.as_f64() {
        return format!("{number:+.6}");
    }
    value.to_string()
}

fn format_metric_value(value: &Value) -> String {
    if let Some(number) = value.as_i64() {
        return number.to_string();
    }
    if let Some(number) = value.as_u64() {
        return number.to_string();
    }
    if let Some(number) = value.as_f64() {
        return format!("{number:.6}");
    }
    value.to_string()
}

fn format_failure_similarity_summary(event: &Value) -> String {
    let payload = event.get("payload").unwrap_or(&Value::Null);
    let event_id = event_string(event, "event_id").unwrap_or("-");
    let source = failure_source(payload);
    let signal = failure_signal(payload);
    format!(
        "{event_id}: source={source} signal={}",
        preview_line(&signal, 120)
    )
}

fn failure_source(payload: &Value) -> String {
    payload_str(payload, "source")
        .or_else(|| payload_str(payload, "operation"))
        .unwrap_or("unknown")
        .to_string()
}

fn failure_signal(payload: &Value) -> String {
    payload_str(payload, "error_preview")
        .or_else(|| payload_str(payload, "error"))
        .or_else(|| payload_str(payload, "summary"))
        .or_else(|| payload_str(payload, "operation"))
        .unwrap_or("failure recorded")
        .to_string()
}

fn failure_signature_tokens(payload: &Value) -> BTreeSet<String> {
    failure_signal(payload)
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '.')
        .map(|token| token.trim().to_ascii_lowercase())
        .filter(|token| token.len() >= 3)
        .collect()
}

fn build_lineage_report(events: &[Value], id: &str) -> Result<String, String> {
    let target = find_target_event(events, id);
    let patch_id = target
        .and_then(|event| event.get("payload"))
        .and_then(extract_patch_id)
        .unwrap_or(id);
    let commit_ids = target
        .and_then(|event| event.get("payload"))
        .map(commit_ids_from_payload)
        .unwrap_or_else(|| vec![id.to_string()]);

    let related: Vec<&Value> = events
        .iter()
        .filter(|event| lineage_matches(event, id, patch_id, &commit_ids))
        .collect();

    if related.is_empty() {
        return Err(format!("no lineage found for '{id}'"));
    }

    let mut out = String::new();
    out.push_str(&format!("State lineage: {id}\n"));
    for event in related {
        out.push_str(&format!("  {}\n", format_event_value(event)));
    }
    Ok(out.trim_end().to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GraphRelationStep {
    depth: usize,
    relation: crate::state::StateRelation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GraphPathStep {
    depth: usize,
    from_id: String,
    to_id: String,
    relation: crate::state::StateRelation,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct GraphSummary {
    id: String,
    depth: usize,
    node_count: usize,
    relation_count: usize,
    depth_counts: BTreeMap<usize, usize>,
    relation_counts: BTreeMap<String, usize>,
    destination_kind_counts: BTreeMap<String, usize>,
    node_kind_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GraphCluster {
    seed: String,
    seed_relation: String,
    nodes: BTreeSet<String>,
    relation_counts: BTreeMap<String, usize>,
    kind_counts: BTreeMap<String, usize>,
}

impl GraphCluster {
    fn relation_count(&self) -> usize {
        self.relation_counts.values().sum()
    }
}

pub(crate) fn build_graph_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
) -> Result<String, String> {
    let steps = query_graph_steps(sqlite_path, id, depth)?;
    if steps.is_empty() {
        return Err(format!("no graph relations found for '{id}'"));
    }
    let mut out = String::new();
    out.push_str(&format!("State graph: {id} depth={}\n", depth.min(4)));
    for step in steps {
        let relation = step.relation;
        let src_kind = infer_graph_node_kind(&relation.src_id);
        out.push_str(&format!(
            "  d{} {} -[{}]-> {} ({} -> {})\n",
            step.depth,
            relation.src_id,
            relation.relation,
            relation.dst_id,
            src_kind,
            relation.dst_kind
        ));
    }
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_summary_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let steps = query_graph_steps(sqlite_path, id, max_depth)?;
    if steps.is_empty() {
        return Err(format!("no graph relations found for '{id}'"));
    }
    let summary = summarize_graph_steps(id, max_depth, &steps);

    let mut out = String::new();
    out.push_str(&format!(
        "State graph summary: {} depth={}\n",
        summary.id, summary.depth
    ));
    out.push_str(&format!("  nodes:       {}\n", summary.node_count));
    out.push_str(&format!("  relations:   {}\n", summary.relation_count));
    out.push_str(&format!(
        "  by depth:    {}\n",
        format_count_map(&summary.depth_counts)
    ));
    out.push_str(&format!(
        "  by relation: {}\n",
        format_count_map(&summary.relation_counts)
    ));
    out.push_str(&format!(
        "  by dst kind: {}\n",
        format_count_map(&summary.destination_kind_counts)
    ));
    out.push_str(&format!(
        "  by node kind: {}\n",
        format_count_map(&summary.node_kind_counts)
    ));
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_summary_payload(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
) -> Result<Value, String> {
    let max_depth = depth.clamp(1, 4);
    let steps = query_graph_steps(sqlite_path, id, max_depth)?;
    if steps.is_empty() {
        return Err(format!("no graph relations found for '{id}'"));
    }
    let summary = summarize_graph_steps(id, max_depth, &steps);
    Ok(serde_json::json!({
        "diagnostic": "state_graph_summary",
        "id": summary.id,
        "depth": summary.depth,
        "node_count": summary.node_count,
        "relation_count": summary.relation_count,
        "by_depth": summary.depth_counts,
        "by_relation": summary.relation_counts,
        "by_destination_kind": summary.destination_kind_counts,
        "by_node_kind": summary.node_kind_counts,
    }))
}

fn summarize_graph_steps(id: &str, max_depth: usize, steps: &[GraphRelationStep]) -> GraphSummary {
    let mut nodes = BTreeSet::new();
    let mut node_kinds = BTreeMap::<String, String>::new();
    let mut summary = GraphSummary {
        id: id.to_string(),
        depth: max_depth,
        relation_count: steps.len(),
        ..GraphSummary::default()
    };
    for step in steps {
        nodes.insert(step.relation.src_id.clone());
        nodes.insert(step.relation.dst_id.clone());
        record_graph_node_kind_hint(
            &mut node_kinds,
            &step.relation.src_id,
            &infer_graph_node_kind(&step.relation.src_id),
        );
        record_graph_node_kind_hint(
            &mut node_kinds,
            &step.relation.dst_id,
            &step.relation.dst_kind,
        );
        *summary
            .relation_counts
            .entry(step.relation.relation.clone())
            .or_default() += 1;
        *summary.depth_counts.entry(step.depth).or_default() += 1;
        *summary
            .destination_kind_counts
            .entry(step.relation.dst_kind.clone())
            .or_default() += 1;
    }
    summary.node_count = nodes.len();
    for node in &nodes {
        let kind = node_kinds
            .get(node)
            .cloned()
            .unwrap_or_else(|| infer_graph_node_kind(node));
        *summary.node_kind_counts.entry(kind).or_default() += 1;
    }
    summary
}

fn record_graph_node_kind_hint(kinds: &mut BTreeMap<String, String>, id: &str, kind: &str) {
    let inferred = if kind == "unknown" {
        infer_graph_node_kind(id)
    } else {
        kind.to_string()
    };
    kinds
        .entry(id.to_string())
        .and_modify(|existing| {
            if existing == "unknown" && inferred != "unknown" {
                *existing = inferred.clone();
            }
        })
        .or_insert(inferred);
}

pub(crate) fn build_graph_clusters_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let steps = query_graph_steps(sqlite_path, id, max_depth)?;
    if steps.is_empty() {
        return Err(format!("no graph relations found for '{id}'"));
    }
    let clusters = build_graph_clusters(id, &steps);
    if clusters.is_empty() {
        return Err(format!("no graph clusters found for '{id}'"));
    }

    let mut out = String::new();
    out.push_str(&format!("State graph clusters: {id} depth={max_depth}\n"));
    out.push_str(&format!("  clusters: {}\n", clusters.len()));
    for cluster in clusters {
        let nodes = cluster.nodes.iter().cloned().collect::<Vec<_>>();
        out.push_str(&format!(
            "  {:<32} via {:<14} nodes={} relations={} kinds={} relation_counts={}\n",
            cluster.seed,
            cluster.seed_relation,
            cluster.nodes.len(),
            cluster.relation_count(),
            format_count_map(&cluster.kind_counts),
            format_top_relation_counts(&cluster.relation_counts, 8)
        ));
        out.push_str(&format!("    nodes: {}\n", compact_id_list(&nodes, 6)));
    }
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_clusters_payload(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
) -> Result<Value, String> {
    let max_depth = depth.clamp(1, 4);
    let steps = query_graph_steps(sqlite_path, id, max_depth)?;
    if steps.is_empty() {
        return Err(format!("no graph relations found for '{id}'"));
    }
    let clusters = build_graph_clusters(id, &steps);
    if clusters.is_empty() {
        return Err(format!("no graph clusters found for '{id}'"));
    }

    Ok(serde_json::json!({
        "diagnostic": "state_graph_clusters",
        "id": id,
        "depth": max_depth,
        "cluster_count": clusters.len(),
        "clusters": clusters
            .iter()
            .map(|cluster| serde_json::json!({
                "seed": &cluster.seed,
                "seed_relation": &cluster.seed_relation,
                "node_count": cluster.nodes.len(),
                "relation_count": cluster.relation_count(),
                "kind_counts": &cluster.kind_counts,
                "relation_counts": &cluster.relation_counts,
                "nodes": cluster.nodes.iter().cloned().collect::<Vec<_>>(),
            }))
            .collect::<Vec<_>>(),
    }))
}

pub(crate) fn build_graph_clusters(
    root_id: &str,
    steps: &[GraphRelationStep],
) -> Vec<GraphCluster> {
    let mut adjacency = BTreeMap::<String, BTreeSet<String>>::new();
    let mut node_kinds = BTreeMap::<String, String>::new();
    for step in steps {
        adjacency
            .entry(step.relation.src_id.clone())
            .or_default()
            .insert(step.relation.dst_id.clone());
        adjacency
            .entry(step.relation.dst_id.clone())
            .or_default()
            .insert(step.relation.src_id.clone());
        record_graph_node_kind_hint(
            &mut node_kinds,
            &step.relation.src_id,
            &infer_graph_node_kind(&step.relation.src_id),
        );
        record_graph_node_kind_hint(
            &mut node_kinds,
            &step.relation.dst_id,
            &step.relation.dst_kind,
        );
    }

    let mut seeds = BTreeMap::<String, String>::new();
    for step in steps.iter().filter(|step| step.depth == 1) {
        let relation = &step.relation;
        let seed = if relation.src_id == root_id {
            Some(relation.dst_id.as_str())
        } else if relation.dst_id == root_id {
            Some(relation.src_id.as_str())
        } else {
            None
        };
        if let Some(seed) = seed {
            seeds
                .entry(seed.to_string())
                .or_insert_with(|| relation.relation.clone());
        }
    }

    let mut seeds = seeds.into_iter().collect::<Vec<_>>();
    seeds.sort_by(|(left_seed, _), (right_seed, _)| {
        graph_cluster_seed_priority(left_seed)
            .cmp(&graph_cluster_seed_priority(right_seed))
            .then_with(|| left_seed.cmp(right_seed))
    });

    let mut seen_node_sets = BTreeSet::new();
    let mut clusters = Vec::new();
    for (seed, seed_relation) in seeds {
        let nodes = collect_graph_cluster_nodes(root_id, &seed, &adjacency);
        if nodes.is_empty() {
            continue;
        }
        let node_key = nodes.iter().cloned().collect::<Vec<_>>().join("\n");
        if !seen_node_sets.insert(node_key) {
            continue;
        }

        let mut relation_counts = BTreeMap::<String, usize>::new();
        let mut kind_counts = BTreeMap::<String, usize>::new();
        for node in &nodes {
            let kind = node_kinds
                .get(node)
                .cloned()
                .unwrap_or_else(|| infer_graph_node_kind(node));
            *kind_counts.entry(kind).or_default() += 1;
        }
        for step in steps {
            let relation = &step.relation;
            let inside_edge = nodes.contains(&relation.src_id) && nodes.contains(&relation.dst_id);
            let root_edge = (relation.src_id == root_id && nodes.contains(&relation.dst_id))
                || (relation.dst_id == root_id && nodes.contains(&relation.src_id));
            if inside_edge || root_edge {
                *relation_counts
                    .entry(relation.relation.clone())
                    .or_default() += 1;
            }
        }

        clusters.push(GraphCluster {
            seed,
            seed_relation,
            nodes,
            relation_counts,
            kind_counts,
        });
    }

    clusters.sort_by(|a, b| {
        b.relation_count()
            .cmp(&a.relation_count())
            .then_with(|| b.nodes.len().cmp(&a.nodes.len()))
            .then_with(|| a.seed.cmp(&b.seed))
    });
    clusters.truncate(20);
    clusters
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct GraphImpact {
    nodes: BTreeMap<String, String>,
    relation_counts: BTreeMap<String, usize>,
    patches: BTreeSet<String>,
    evals: BTreeSet<String>,
    decisions: BTreeSet<String>,
    files: BTreeSet<String>,
    policies: BTreeSet<String>,
    evidence_nodes: BTreeSet<String>,
    positive_signals: BTreeMap<String, usize>,
    risk_signals: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GraphSignalEdge {
    depth: usize,
    src_id: String,
    relation: String,
    dst_id: String,
    dst_kind: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct GraphSignals {
    positive_counts: BTreeMap<String, usize>,
    risk_counts: BTreeMap<String, usize>,
    positive_edges: Vec<GraphSignalEdge>,
    risk_edges: Vec<GraphSignalEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GraphTimelineStep {
    depth: usize,
    timestamp_ms: i64,
    event_id: String,
    event_type: String,
    src_id: String,
    relation: String,
    dst_id: String,
    dst_kind: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct GraphEvalMetadata {
    eval_id: String,
    patch_id: Option<String>,
    suite: Option<String>,
    status: Option<String>,
    score: Option<f64>,
    fixture_task_count: Option<u64>,
    fixture_command_count: Option<u64>,
    deepseek_protocol_checks: Option<u64>,
    deepseek_protocol_passes: Option<u64>,
    deepseek_strict_tool_call_checks: Option<u64>,
    deepseek_thinking_protocol_checks: Option<u64>,
    deepseek_streaming_protocol_checks: Option<u64>,
    deepseek_json_output_checks: Option<u64>,
    deepseek_transport_policy_checks: Option<u64>,
    model_route_tasks: BTreeMap<String, u64>,
    fixture_agent_changed_files: Vec<String>,
    fixture_agent_unexpected_files: Vec<String>,
    last_event_id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct GraphPatchMetadata {
    patch_id: String,
    status: Option<String>,
    kind: Option<String>,
    risk_level: Option<String>,
    base_harness_version: Option<String>,
    state_version: Option<u64>,
    base_git_commit: Option<String>,
    rollback_plan_steps: usize,
    last_event_id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct GraphDecisionMetadata {
    decision_id: String,
    decision_type: Option<String>,
    decision: Option<String>,
    status: Option<String>,
    patch_id: Option<String>,
    eval_id: Option<String>,
    event_id: String,
    suite: Option<String>,
    reason: Option<String>,
    missing_required_gates: usize,
    replay_failures_after_eval: Option<u64>,
    last_eval_fixture_task_count: Option<u64>,
    last_eval_fixture_command_count: Option<u64>,
    last_eval_fixture_risk_labels: BTreeMap<String, u64>,
    last_eval_model_route_tasks: BTreeMap<String, u64>,
    last_eval_mutation_scope_failures: Option<u64>,
    last_eval_unexpected_changed_files: Option<u64>,
    min_fixture_task_count: Option<u64>,
    min_fixture_command_count: Option<u64>,
    min_fixture_risk_labels: BTreeMap<String, u64>,
    fixture_breadth_satisfied: Option<bool>,
    fixture_risk_satisfied: Option<bool>,
    require_protocol: Option<bool>,
    protocol_eval_status: Option<String>,
    protocol_eval_git_dirty: Option<bool>,
    protocol_check_total: Option<u64>,
    protocol_check_passes: Option<u64>,
    protocol_check_strict: Option<u64>,
    protocol_check_thinking: Option<u64>,
    protocol_check_stream: Option<u64>,
    protocol_check_json: Option<u64>,
    protocol_check_transport: Option<u64>,
    source_provenance_passed: Option<bool>,
    source_provenance_findings: Option<u64>,
    source_provenance_scan_source: Option<String>,
    promotion_eligible: Option<bool>,
    promotion_criterion: Option<String>,
    promotion_reason: Option<String>,
    promotion_baseline_eval_id: Option<String>,
    promotion_candidate_eval_id: Option<String>,
    promotion_protocol_eval_id: Option<String>,
    promotion_fixture_baseline_task_count: Option<u64>,
    promotion_fixture_candidate_task_count: Option<u64>,
    promotion_fixture_baseline_command_count: Option<u64>,
    promotion_fixture_candidate_command_count: Option<u64>,
    promotion_fixture_baseline_risk_labels: BTreeMap<String, u64>,
    promotion_fixture_candidate_risk_labels: BTreeMap<String, u64>,
    promotion_model_route_baseline: BTreeMap<String, u64>,
    promotion_model_route_candidate: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct GraphHypothesisMetadata {
    hypothesis_id: String,
    event_id: String,
    failure_event_id: Option<String>,
    summary: Option<String>,
    confidence: Option<f64>,
    status: Option<String>,
    run_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct GraphVersionMetadata {
    event_id: String,
    event_type: String,
    harness_version: Option<String>,
    base_harness_version: Option<String>,
    patch_id: Option<String>,
    eval_id: Option<String>,
    suite: Option<String>,
    status: Option<String>,
    run_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct GraphRunMetadata {
    event_id: String,
    event_type: String,
    run_id: Option<String>,
    trace_id: Option<String>,
    task_ids: Vec<String>,
    status: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct GraphArtifactMetadata {
    event_id: String,
    event_type: String,
    artifact_uri: String,
    eval_id: Option<String>,
    patch_id: Option<String>,
    suite: Option<String>,
    status: Option<String>,
    repro_mode: Option<String>,
    agent_command_source: Option<String>,
    replay_command: Option<String>,
    git_dirty: Option<bool>,
    command_count: usize,
    fixture_task_count: Option<u64>,
    fixture_command_count: Option<u64>,
    run_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct GraphModelMetadata {
    event_id: String,
    event_type: String,
    model_call_id: String,
    model: Option<String>,
    route_task: Option<String>,
    thinking: Option<String>,
    reasoning_effort: Option<String>,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_read_tokens: Option<u64>,
    cache_write_tokens: Option<u64>,
    run_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct GraphToolMetadata {
    event_id: String,
    event_type: String,
    tool_call_id: String,
    tool_name: Option<String>,
    status: Option<String>,
    result_preview: Option<String>,
    args_preview: Option<String>,
    run_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct GraphCommandMetadata {
    event_id: String,
    event_type: String,
    command: Option<String>,
    status: Option<String>,
    result_preview: Option<String>,
    run_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct GraphTestMetadata {
    event_id: String,
    event_type: String,
    test_kind: Option<String>,
    command: Option<String>,
    status: Option<String>,
    result_preview: Option<String>,
    run_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct GraphCommitMetadata {
    event_id: String,
    event_type: String,
    commit: Option<String>,
    reverted_commit: Option<String>,
    branch: Option<String>,
    message: Option<String>,
    reason: Option<String>,
    files: Vec<String>,
    run_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct GraphMemoryMetadata {
    event_id: String,
    event_type: String,
    candidate_id: String,
    status: Option<String>,
    source: Option<String>,
    summary: Option<String>,
    reason: Option<String>,
    proposed_event_id: Option<String>,
    evidence_event_ids: Vec<String>,
    run_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct GraphIssueMetadata {
    event_id: String,
    issue_id: String,
    patch_id: Option<String>,
    intake_source: Option<String>,
    intake_kind: Option<String>,
    summary: Option<String>,
    details: Option<String>,
    kind: Option<String>,
    risk_level: Option<String>,
    status: Option<String>,
    run_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct GraphCacheMetadata {
    event_id: String,
    model: Option<String>,
    prompt_cache_hit_tokens: Option<i64>,
    prompt_cache_miss_tokens: Option<i64>,
    cache_hit_ratio: Option<f64>,
    timestamp_ms: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct GraphFailureMetadata {
    event_id: String,
    event_type: String,
    class: String,
    owner: String,
    retryable: bool,
    source: Option<String>,
    error_preview: Option<String>,
    run_id: Option<String>,
    timestamp_ms: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct GraphPolicyMetadata {
    event_id: String,
    event_type: String,
    run_id: Option<String>,
    context_policy: Option<String>,
    prompt_layout: Option<String>,
    prompt_version: Option<String>,
    schema_name: Option<String>,
    schema_version: Option<String>,
    source_provenance_scan_source: Option<String>,
    source_provenance_passed: Option<bool>,
    source_provenance_findings: Option<u64>,
    stable_blocks: usize,
    dynamic_blocks: usize,
    included_blocks: Vec<String>,
    instruction_files: Vec<String>,
}

pub(crate) fn build_graph_impact_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let steps = query_graph_steps(sqlite_path, id, max_depth)?;
    if steps.is_empty() {
        return Err(format!("no graph relations found for '{id}'"));
    }
    let impact = summarize_graph_impact(&steps);

    let mut out = String::new();
    out.push_str(&format!("State graph impact: {id} depth={max_depth}\n"));
    out.push_str(&format!("  nodes:       {}\n", impact.nodes.len()));
    out.push_str(&format!("  relations:   {}\n", steps.len()));
    out.push_str(&format!(
        "  patches:     {}\n",
        format_graph_impact_ids(&impact.patches, 6)
    ));
    out.push_str(&format!(
        "  evals:       {}\n",
        format_graph_impact_ids(&impact.evals, 6)
    ));
    out.push_str(&format!(
        "  decisions:   {}\n",
        format_graph_impact_ids(&impact.decisions, 6)
    ));
    out.push_str(&format!(
        "  files:       {}\n",
        format_graph_impact_ids(&impact.files, 6)
    ));
    out.push_str(&format!(
        "  policies:    {}\n",
        format_graph_impact_ids(&impact.policies, 6)
    ));
    out.push_str(&format!(
        "  evidence:    {}\n",
        format_graph_impact_ids(&impact.evidence_nodes, 6)
    ));
    out.push_str(&format!(
        "  positives:   {}\n",
        format_count_map(&impact.positive_signals)
    ));
    out.push_str(&format!(
        "  risks:       {}\n",
        format_count_map(&impact.risk_signals)
    ));
    out.push_str(&format!(
        "  relations:   {}\n",
        format_top_relation_counts(&impact.relation_counts, 10)
    ));
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_impact_payload(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
) -> Result<Value, String> {
    let max_depth = depth.clamp(1, 4);
    let steps = query_graph_steps(sqlite_path, id, max_depth)?;
    if steps.is_empty() {
        return Err(format!("no graph relations found for '{id}'"));
    }
    let impact = summarize_graph_impact(&steps);

    Ok(serde_json::json!({
        "diagnostic": "state_graph_impact",
        "id": id,
        "depth": max_depth,
        "node_count": impact.nodes.len(),
        "relation_count": steps.len(),
        "nodes": &impact.nodes,
        "patches": impact.patches.iter().cloned().collect::<Vec<_>>(),
        "evals": impact.evals.iter().cloned().collect::<Vec<_>>(),
        "decisions": impact.decisions.iter().cloned().collect::<Vec<_>>(),
        "files": impact.files.iter().cloned().collect::<Vec<_>>(),
        "policies": impact.policies.iter().cloned().collect::<Vec<_>>(),
        "evidence": impact.evidence_nodes.iter().cloned().collect::<Vec<_>>(),
        "positive_signals": &impact.positive_signals,
        "risk_signals": &impact.risk_signals,
        "relations": &impact.relation_counts,
    }))
}

pub(crate) fn build_graph_signals_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let steps = query_graph_steps(sqlite_path, id, max_depth)?;
    if steps.is_empty() {
        return Err(format!("no graph relations found for '{id}'"));
    }
    let signals = summarize_graph_signals(&steps);
    if signals.positive_edges.is_empty() && signals.risk_edges.is_empty() {
        return Err(format!(
            "no graph signals found for '{id}' within depth {max_depth}"
        ));
    }

    let mut out = String::new();
    out.push_str(&format!("State graph signals: {id} depth={max_depth}\n"));
    out.push_str(&format!(
        "  positives: {}\n",
        format_count_map(&signals.positive_counts)
    ));
    out.push_str(&format!(
        "  risks:     {}\n",
        format_count_map(&signals.risk_counts)
    ));
    append_graph_signal_rows(&mut out, "positive paths", &signals.positive_edges, 5);
    append_graph_signal_rows(&mut out, "risk paths", &signals.risk_edges, 5);
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_signals_payload(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
) -> Result<Value, String> {
    let max_depth = depth.clamp(1, 4);
    let steps = query_graph_steps(sqlite_path, id, max_depth)?;
    if steps.is_empty() {
        return Err(format!("no graph relations found for '{id}'"));
    }
    let signals = summarize_graph_signals(&steps);
    if signals.positive_edges.is_empty() && signals.risk_edges.is_empty() {
        return Err(format!(
            "no graph signals found for '{id}' within depth {max_depth}"
        ));
    }

    Ok(serde_json::json!({
        "diagnostic": "state_graph_signals",
        "id": id,
        "depth": max_depth,
        "positive_counts": &signals.positive_counts,
        "risk_counts": &signals.risk_counts,
        "positive_paths": graph_signal_edges_payload(&signals.positive_edges, 20),
        "risk_paths": graph_signal_edges_payload(&signals.risk_edges, 20),
    }))
}

fn graph_signal_edges_payload(edges: &[GraphSignalEdge], limit: usize) -> Vec<Value> {
    edges
        .iter()
        .take(limit)
        .map(|edge| {
            serde_json::json!({
                "depth": edge.depth,
                "src_id": &edge.src_id,
                "relation": &edge.relation,
                "dst_id": &edge.dst_id,
                "dst_kind": &edge.dst_kind,
            })
        })
        .collect()
}

pub(crate) fn build_graph_timeline_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let steps = query_graph_timeline(sqlite_path, id, max_depth, limit)?;
    if steps.is_empty() {
        return Err(format!("no graph relations found for '{id}'"));
    }

    let mut out = String::new();
    out.push_str(&format!(
        "State graph timeline: {id} depth={max_depth} limit={limit}\n"
    ));
    for step in steps {
        out.push_str(&format!(
            "  t={} d{} {} {} -[{}]-> {} ({}) via {}\n",
            step.timestamp_ms,
            step.depth,
            step.event_type,
            step.src_id,
            step.relation,
            step.dst_id,
            step.dst_kind,
            step.event_id
        ));
    }
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_timeline_payload(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<Value, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let steps = query_graph_timeline(sqlite_path, id, max_depth, limit)?;
    if steps.is_empty() {
        return Err(format!("no graph relations found for '{id}'"));
    }

    Ok(serde_json::json!({
        "diagnostic": "state_graph_timeline",
        "id": id,
        "depth": max_depth,
        "limit": limit,
        "relation_count": steps.len(),
        "timeline": graph_timeline_steps_payload(&steps),
    }))
}

pub(crate) fn build_graph_evidence_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_evidence_relation(&step.relation))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph evidence relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut relation_counts = BTreeMap::<String, usize>::new();
    for step in &steps {
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }

    let mut out = String::new();
    out.push_str(&format!(
        "State graph evidence: {id} depth={max_depth} limit={limit}\n"
    ));
    out.push_str(&format!(
        "  by relation: {}\n",
        format_count_map(&relation_counts)
    ));
    for step in steps {
        out.push_str(&format!(
            "  t={} d{} {} {} -[{}]-> {} ({}) via {}\n",
            step.timestamp_ms,
            step.depth,
            step.event_type,
            step.src_id,
            step.relation,
            step.dst_id,
            step.dst_kind,
            step.event_id
        ));
    }
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_evidence_payload(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<Value, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_evidence_relation(&step.relation))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph evidence relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut relation_counts = BTreeMap::<String, usize>::new();
    for step in &steps {
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let evidence = graph_timeline_steps_payload(&steps);

    Ok(serde_json::json!({
        "diagnostic": "state_graph_evidence",
        "id": id,
        "depth": max_depth,
        "limit": limit,
        "evidence_count": steps.len(),
        "relations": relation_counts,
        "evidence": evidence,
    }))
}

fn graph_timeline_steps_payload(steps: &[GraphTimelineStep]) -> Vec<Value> {
    steps
        .iter()
        .map(|step| {
            serde_json::json!({
                "timestamp_ms": step.timestamp_ms,
                "depth": step.depth,
                "event_id": &step.event_id,
                "event_type": &step.event_type,
                "src_id": &step.src_id,
                "relation": &step.relation,
                "dst_id": &step.dst_id,
                "dst_kind": &step.dst_kind,
            })
        })
        .collect()
}

pub(crate) fn build_graph_files_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(graph_timeline_step_mentions_file)
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph file relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut files = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    for step in &steps {
        if let Some(file) = graph_timeline_step_file_id(step) {
            files.insert(file);
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let files = files.into_iter().collect::<Vec<_>>();

    let mut out = String::new();
    out.push_str(&format!(
        "State graph files: {id} depth={max_depth} limit={limit}\n"
    ));
    out.push_str(&format!("  files:       {}\n", compact_id_list(&files, 8)));
    out.push_str(&format!(
        "  by relation: {}\n",
        format_count_map(&relation_counts)
    ));
    for step in steps {
        out.push_str(&format!(
            "  t={} d{} {} {} -[{}]-> {} ({}) via {}\n",
            step.timestamp_ms,
            step.depth,
            step.event_type,
            step.src_id,
            step.relation,
            step.dst_id,
            step.dst_kind,
            step.event_id
        ));
    }
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_files_payload(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<Value, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(graph_timeline_step_mentions_file)
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph file relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut files = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    for step in &steps {
        if let Some(file) = graph_timeline_step_file_id(step) {
            files.insert(file);
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let files = files.into_iter().collect::<Vec<_>>();
    let file_relations = graph_timeline_steps_payload(&steps);

    Ok(serde_json::json!({
        "diagnostic": "state_graph_files",
        "id": id,
        "depth": max_depth,
        "limit": limit,
        "file_count": files.len(),
        "relation_count": steps.len(),
        "files": files,
        "relations": relation_counts,
        "file_relations": file_relations,
    }))
}

pub(crate) fn build_graph_evals_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(graph_timeline_step_mentions_eval)
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph eval relations found for '{id}' within depth {max_depth}"
        ));
    }

    let metadata = query_graph_eval_metadata(sqlite_path)?;
    let mut evals = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    for step in &steps {
        if let Some(eval_id) = graph_timeline_step_eval_key(step)
            .as_deref()
            .and_then(|key| metadata.get(key))
            .map(|metadata| metadata.eval_id.clone())
            .or_else(|| graph_timeline_step_eval_key(step))
        {
            evals.insert(eval_id);
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let evals = evals.into_iter().collect::<Vec<_>>();

    let mut out = String::new();
    out.push_str(&format!(
        "State graph evals: {id} depth={max_depth} limit={limit}\n"
    ));
    out.push_str(&format!("  evals:       {}\n", compact_id_list(&evals, 8)));
    out.push_str(&format!(
        "  by relation: {}\n",
        format_count_map(&relation_counts)
    ));
    for step in steps {
        let detail = graph_timeline_step_eval_key(&step)
            .as_deref()
            .and_then(|eval_id| metadata.get(eval_id))
            .map(format_graph_eval_metadata)
            .unwrap_or_else(|| "suite=- status=- score=- patch=-".to_string());
        out.push_str(&format!(
            "  t={} d{} {} {} -[{}]-> {} ({}) {} via {}\n",
            step.timestamp_ms,
            step.depth,
            step.event_type,
            step.src_id,
            step.relation,
            step.dst_id,
            step.dst_kind,
            detail,
            step.event_id
        ));
    }
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_evals_payload(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<Value, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(graph_timeline_step_mentions_eval)
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph eval relations found for '{id}' within depth {max_depth}"
        ));
    }

    let metadata = query_graph_eval_metadata(sqlite_path)?;
    let mut evals = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    for step in &steps {
        if let Some(eval_id) = graph_eval_id_for_timeline_step(step, &metadata) {
            evals.insert(eval_id);
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let evals = evals.into_iter().collect::<Vec<_>>();
    let eval_metadata = evals
        .iter()
        .filter_map(|eval_id| {
            metadata
                .get(eval_id)
                .map(|metadata| (eval_id.clone(), graph_eval_metadata_payload(metadata)))
        })
        .collect::<BTreeMap<_, _>>();
    let eval_relations = graph_eval_relations_payload(&steps, &metadata);

    Ok(serde_json::json!({
        "diagnostic": "state_graph_evals",
        "id": id,
        "depth": max_depth,
        "limit": limit,
        "eval_count": evals.len(),
        "relation_count": steps.len(),
        "evals": evals,
        "relations": relation_counts,
        "eval_metadata": eval_metadata,
        "eval_relations": eval_relations,
    }))
}

fn graph_eval_id_for_timeline_step(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphEvalMetadata>,
) -> Option<String> {
    let key = graph_timeline_step_eval_key(step)?;
    metadata
        .get(&key)
        .map(|metadata| metadata.eval_id.clone())
        .or(Some(key))
}

fn graph_eval_relations_payload(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphEvalMetadata>,
) -> Vec<Value> {
    steps
        .iter()
        .map(|step| {
            let eval_id = graph_eval_id_for_timeline_step(step, metadata);
            serde_json::json!({
                "timestamp_ms": step.timestamp_ms,
                "depth": step.depth,
                "event_id": &step.event_id,
                "event_type": &step.event_type,
                "src_id": &step.src_id,
                "relation": &step.relation,
                "dst_id": &step.dst_id,
                "dst_kind": &step.dst_kind,
                "eval_id": eval_id,
            })
        })
        .collect()
}

fn graph_eval_metadata_payload(metadata: &GraphEvalMetadata) -> Value {
    serde_json::json!({
        "eval_id": &metadata.eval_id,
        "patch_id": &metadata.patch_id,
        "suite": &metadata.suite,
        "status": &metadata.status,
        "score": metadata.score,
        "fixture_task_count": metadata.fixture_task_count,
        "fixture_command_count": metadata.fixture_command_count,
        "deepseek_protocol_checks": metadata.deepseek_protocol_checks,
        "deepseek_protocol_passes": metadata.deepseek_protocol_passes,
        "deepseek_strict_tool_call_checks": metadata.deepseek_strict_tool_call_checks,
        "deepseek_thinking_protocol_checks": metadata.deepseek_thinking_protocol_checks,
        "deepseek_streaming_protocol_checks": metadata.deepseek_streaming_protocol_checks,
        "deepseek_json_output_checks": metadata.deepseek_json_output_checks,
        "deepseek_transport_policy_checks": metadata.deepseek_transport_policy_checks,
        "model_route_tasks": &metadata.model_route_tasks,
        "fixture_agent_changed_files": &metadata.fixture_agent_changed_files,
        "fixture_agent_unexpected_files": &metadata.fixture_agent_unexpected_files,
        "last_event_id": &metadata.last_event_id,
    })
}

pub(crate) fn build_graph_patches_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(graph_timeline_step_mentions_patch)
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph patch relations found for '{id}' within depth {max_depth}"
        ));
    }

    let metadata = query_graph_patch_metadata(sqlite_path)?;
    let mut patches = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    for step in &steps {
        if let Some(key) = graph_timeline_step_patch_key(step) {
            let patch_id = metadata
                .get(&key)
                .map(|metadata| metadata.patch_id.clone())
                .or_else(|| (infer_graph_node_kind(&key) == "patch").then_some(key));
            if let Some(patch_id) = patch_id {
                patches.insert(patch_id);
            }
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let patches = patches.into_iter().collect::<Vec<_>>();

    let mut out = String::new();
    out.push_str(&format!(
        "State graph patches: {id} depth={max_depth} limit={limit}\n"
    ));
    out.push_str(&format!("  patches:    {}\n", compact_id_list(&patches, 8)));
    out.push_str(&format!(
        "  by relation: {}\n",
        format_count_map(&relation_counts)
    ));
    for step in steps {
        let detail = graph_timeline_step_patch_key(&step)
            .as_deref()
            .and_then(|patch_id| metadata.get(patch_id))
            .map(format_graph_patch_metadata)
            .unwrap_or_else(|| {
                "status=- kind=- risk=- harness=- state=- base=- rollback_steps=0".to_string()
            });
        out.push_str(&format!(
            "  t={} d{} {} {} -[{}]-> {} ({}) {} via {}\n",
            step.timestamp_ms,
            step.depth,
            step.event_type,
            step.src_id,
            step.relation,
            step.dst_id,
            step.dst_kind,
            detail,
            step.event_id
        ));
    }
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_patches_payload(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<Value, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(graph_timeline_step_mentions_patch)
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph patch relations found for '{id}' within depth {max_depth}"
        ));
    }

    let metadata = query_graph_patch_metadata(sqlite_path)?;
    let mut patches = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    for step in &steps {
        if let Some(patch_id) = graph_patch_id_for_timeline_step(step, &metadata) {
            patches.insert(patch_id);
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let patches = patches.into_iter().collect::<Vec<_>>();
    let patch_metadata = patches
        .iter()
        .filter_map(|patch_id| {
            metadata
                .get(patch_id)
                .map(|metadata| (patch_id.clone(), graph_patch_metadata_payload(metadata)))
        })
        .collect::<BTreeMap<_, _>>();
    let patch_relations = graph_patch_relations_payload(&steps, &metadata);

    Ok(serde_json::json!({
        "diagnostic": "state_graph_patches",
        "id": id,
        "depth": max_depth,
        "limit": limit,
        "patch_count": patches.len(),
        "relation_count": steps.len(),
        "patches": patches,
        "relations": relation_counts,
        "patch_metadata": patch_metadata,
        "patch_relations": patch_relations,
    }))
}

fn graph_patch_id_for_timeline_step(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphPatchMetadata>,
) -> Option<String> {
    let key = graph_timeline_step_patch_key(step)?;
    metadata
        .get(&key)
        .map(|metadata| metadata.patch_id.clone())
        .or_else(|| (infer_graph_node_kind(&key) == "patch").then_some(key))
}

fn graph_patch_relations_payload(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphPatchMetadata>,
) -> Vec<Value> {
    steps
        .iter()
        .map(|step| {
            let patch_id = graph_patch_id_for_timeline_step(step, metadata);
            serde_json::json!({
                "timestamp_ms": step.timestamp_ms,
                "depth": step.depth,
                "event_id": &step.event_id,
                "event_type": &step.event_type,
                "src_id": &step.src_id,
                "relation": &step.relation,
                "dst_id": &step.dst_id,
                "dst_kind": &step.dst_kind,
                "patch_id": patch_id,
            })
        })
        .collect()
}

fn graph_patch_metadata_payload(metadata: &GraphPatchMetadata) -> Value {
    serde_json::json!({
        "patch_id": &metadata.patch_id,
        "status": &metadata.status,
        "kind": &metadata.kind,
        "risk_level": &metadata.risk_level,
        "base_harness_version": &metadata.base_harness_version,
        "state_version": metadata.state_version,
        "base_git_commit": &metadata.base_git_commit,
        "rollback_plan_steps": metadata.rollback_plan_steps,
        "last_event_id": &metadata.last_event_id,
    })
}

pub(crate) fn build_graph_decisions_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(graph_timeline_step_mentions_decision)
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph decision relations found for '{id}' within depth {max_depth}"
        ));
    }

    let metadata = query_graph_decision_metadata(sqlite_path)?;
    let mut decisions = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    for step in &steps {
        if let Some(key) = graph_timeline_step_decision_key(step) {
            let decision_id = metadata
                .get(&key)
                .map(|metadata| metadata.decision_id.clone())
                .or_else(|| (infer_graph_node_kind(&key) == "decision").then_some(key));
            if let Some(decision_id) = decision_id {
                decisions.insert(decision_id);
            }
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let decisions = decisions.into_iter().collect::<Vec<_>>();

    let mut out = String::new();
    out.push_str(&format!(
        "State graph decisions: {id} depth={max_depth} limit={limit}\n"
    ));
    out.push_str(&format!(
        "  decisions:  {}\n",
        compact_id_list(&decisions, 8)
    ));
    out.push_str(&format!(
        "  by relation: {}\n",
        format_count_map(&relation_counts)
    ));
    for step in steps {
        let detail = graph_timeline_step_decision_key(&step)
            .as_deref()
            .and_then(|decision_id| metadata.get(decision_id))
            .map(format_graph_decision_metadata)
            .unwrap_or_else(|| "type=- decision=- status=- patch=- eval=-".to_string());
        out.push_str(&format!(
            "  t={} d{} {} {} -[{}]-> {} ({}) {} via {}\n",
            step.timestamp_ms,
            step.depth,
            step.event_type,
            step.src_id,
            step.relation,
            step.dst_id,
            step.dst_kind,
            detail,
            step.event_id
        ));
    }
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_decisions_payload(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<Value, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(graph_timeline_step_mentions_decision)
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph decision relations found for '{id}' within depth {max_depth}"
        ));
    }

    let metadata = query_graph_decision_metadata(sqlite_path)?;
    let mut decisions = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    for step in &steps {
        if let Some(decision_id) = graph_decision_id_for_timeline_step(step, &metadata) {
            decisions.insert(decision_id);
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let decisions = decisions.into_iter().collect::<Vec<_>>();
    let decision_metadata = decisions
        .iter()
        .filter_map(|decision_id| {
            metadata.get(decision_id).map(|metadata| {
                (
                    decision_id.clone(),
                    graph_decision_metadata_payload(metadata),
                )
            })
        })
        .collect::<BTreeMap<_, _>>();
    let decision_relations = graph_decision_relations_payload(&steps, &metadata);

    Ok(serde_json::json!({
        "diagnostic": "state_graph_decisions",
        "id": id,
        "depth": max_depth,
        "limit": limit,
        "decision_count": decisions.len(),
        "relation_count": steps.len(),
        "decisions": decisions,
        "relations": relation_counts,
        "decision_metadata": decision_metadata,
        "decision_relations": decision_relations,
    }))
}

fn graph_decision_id_for_timeline_step(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphDecisionMetadata>,
) -> Option<String> {
    let key = graph_timeline_step_decision_key(step)?;
    metadata
        .get(&key)
        .map(|metadata| metadata.decision_id.clone())
        .or_else(|| (infer_graph_node_kind(&key) == "decision").then_some(key))
}

fn graph_decision_relations_payload(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphDecisionMetadata>,
) -> Vec<Value> {
    steps
        .iter()
        .map(|step| {
            let decision_id = graph_decision_id_for_timeline_step(step, metadata);
            serde_json::json!({
                "timestamp_ms": step.timestamp_ms,
                "depth": step.depth,
                "event_id": &step.event_id,
                "event_type": &step.event_type,
                "src_id": &step.src_id,
                "relation": &step.relation,
                "dst_id": &step.dst_id,
                "dst_kind": &step.dst_kind,
                "decision_id": decision_id,
            })
        })
        .collect()
}

fn graph_decision_metadata_payload(metadata: &GraphDecisionMetadata) -> Value {
    let mut out = serde_json::Map::new();
    out.insert(
        "decision_id".to_string(),
        Value::from(metadata.decision_id.clone()),
    );
    out.insert(
        "decision_type".to_string(),
        serde_json::to_value(&metadata.decision_type).unwrap_or(Value::Null),
    );
    out.insert(
        "decision".to_string(),
        serde_json::to_value(&metadata.decision).unwrap_or(Value::Null),
    );
    out.insert(
        "status".to_string(),
        serde_json::to_value(&metadata.status).unwrap_or(Value::Null),
    );
    out.insert(
        "patch_id".to_string(),
        serde_json::to_value(&metadata.patch_id).unwrap_or(Value::Null),
    );
    out.insert(
        "eval_id".to_string(),
        serde_json::to_value(&metadata.eval_id).unwrap_or(Value::Null),
    );
    out.insert(
        "event_id".to_string(),
        Value::from(metadata.event_id.clone()),
    );
    out.insert(
        "suite".to_string(),
        serde_json::to_value(&metadata.suite).unwrap_or(Value::Null),
    );
    out.insert(
        "reason".to_string(),
        serde_json::to_value(&metadata.reason).unwrap_or(Value::Null),
    );
    out.insert(
        "missing_required_gates".to_string(),
        Value::from(metadata.missing_required_gates),
    );
    out.insert(
        "replay_failures_after_eval".to_string(),
        serde_json::to_value(metadata.replay_failures_after_eval).unwrap_or(Value::Null),
    );
    out.insert(
        "last_eval_fixture_task_count".to_string(),
        serde_json::to_value(metadata.last_eval_fixture_task_count).unwrap_or(Value::Null),
    );
    out.insert(
        "last_eval_fixture_command_count".to_string(),
        serde_json::to_value(metadata.last_eval_fixture_command_count).unwrap_or(Value::Null),
    );
    out.insert(
        "last_eval_fixture_risk_labels".to_string(),
        serde_json::to_value(&metadata.last_eval_fixture_risk_labels).unwrap_or(Value::Null),
    );
    out.insert(
        "last_eval_model_route_tasks".to_string(),
        serde_json::to_value(&metadata.last_eval_model_route_tasks).unwrap_or(Value::Null),
    );
    out.insert(
        "last_eval_mutation_scope_failures".to_string(),
        serde_json::to_value(metadata.last_eval_mutation_scope_failures).unwrap_or(Value::Null),
    );
    out.insert(
        "last_eval_unexpected_changed_files".to_string(),
        serde_json::to_value(metadata.last_eval_unexpected_changed_files).unwrap_or(Value::Null),
    );
    out.insert(
        "min_fixture_task_count".to_string(),
        serde_json::to_value(metadata.min_fixture_task_count).unwrap_or(Value::Null),
    );
    out.insert(
        "min_fixture_command_count".to_string(),
        serde_json::to_value(metadata.min_fixture_command_count).unwrap_or(Value::Null),
    );
    out.insert(
        "min_fixture_risk_labels".to_string(),
        serde_json::to_value(&metadata.min_fixture_risk_labels).unwrap_or(Value::Null),
    );
    out.insert(
        "fixture_breadth_satisfied".to_string(),
        serde_json::to_value(metadata.fixture_breadth_satisfied).unwrap_or(Value::Null),
    );
    out.insert(
        "fixture_risk_satisfied".to_string(),
        serde_json::to_value(metadata.fixture_risk_satisfied).unwrap_or(Value::Null),
    );
    out.insert(
        "require_protocol".to_string(),
        serde_json::to_value(metadata.require_protocol).unwrap_or(Value::Null),
    );
    out.insert(
        "protocol_eval_status".to_string(),
        serde_json::to_value(&metadata.protocol_eval_status).unwrap_or(Value::Null),
    );
    out.insert(
        "protocol_eval_git_dirty".to_string(),
        serde_json::to_value(metadata.protocol_eval_git_dirty).unwrap_or(Value::Null),
    );
    out.insert(
        "protocol_check_total".to_string(),
        serde_json::to_value(metadata.protocol_check_total).unwrap_or(Value::Null),
    );
    out.insert(
        "protocol_check_passes".to_string(),
        serde_json::to_value(metadata.protocol_check_passes).unwrap_or(Value::Null),
    );
    out.insert(
        "protocol_check_strict".to_string(),
        serde_json::to_value(metadata.protocol_check_strict).unwrap_or(Value::Null),
    );
    out.insert(
        "protocol_check_thinking".to_string(),
        serde_json::to_value(metadata.protocol_check_thinking).unwrap_or(Value::Null),
    );
    out.insert(
        "protocol_check_stream".to_string(),
        serde_json::to_value(metadata.protocol_check_stream).unwrap_or(Value::Null),
    );
    out.insert(
        "protocol_check_json".to_string(),
        serde_json::to_value(metadata.protocol_check_json).unwrap_or(Value::Null),
    );
    out.insert(
        "protocol_check_transport".to_string(),
        serde_json::to_value(metadata.protocol_check_transport).unwrap_or(Value::Null),
    );
    out.insert(
        "source_provenance_passed".to_string(),
        serde_json::to_value(metadata.source_provenance_passed).unwrap_or(Value::Null),
    );
    out.insert(
        "source_provenance_findings".to_string(),
        serde_json::to_value(metadata.source_provenance_findings).unwrap_or(Value::Null),
    );
    out.insert(
        "source_provenance_scan_source".to_string(),
        serde_json::to_value(&metadata.source_provenance_scan_source).unwrap_or(Value::Null),
    );
    out.insert(
        "promotion_eligible".to_string(),
        serde_json::to_value(metadata.promotion_eligible).unwrap_or(Value::Null),
    );
    out.insert(
        "promotion_criterion".to_string(),
        serde_json::to_value(&metadata.promotion_criterion).unwrap_or(Value::Null),
    );
    out.insert(
        "promotion_reason".to_string(),
        serde_json::to_value(&metadata.promotion_reason).unwrap_or(Value::Null),
    );
    out.insert(
        "promotion_baseline_eval_id".to_string(),
        serde_json::to_value(&metadata.promotion_baseline_eval_id).unwrap_or(Value::Null),
    );
    out.insert(
        "promotion_candidate_eval_id".to_string(),
        serde_json::to_value(&metadata.promotion_candidate_eval_id).unwrap_or(Value::Null),
    );
    out.insert(
        "promotion_protocol_eval_id".to_string(),
        serde_json::to_value(&metadata.promotion_protocol_eval_id).unwrap_or(Value::Null),
    );
    out.insert(
        "promotion_fixture_baseline_task_count".to_string(),
        serde_json::to_value(metadata.promotion_fixture_baseline_task_count).unwrap_or(Value::Null),
    );
    out.insert(
        "promotion_fixture_candidate_task_count".to_string(),
        serde_json::to_value(metadata.promotion_fixture_candidate_task_count)
            .unwrap_or(Value::Null),
    );
    out.insert(
        "promotion_fixture_baseline_command_count".to_string(),
        serde_json::to_value(metadata.promotion_fixture_baseline_command_count)
            .unwrap_or(Value::Null),
    );
    out.insert(
        "promotion_fixture_candidate_command_count".to_string(),
        serde_json::to_value(metadata.promotion_fixture_candidate_command_count)
            .unwrap_or(Value::Null),
    );
    out.insert(
        "promotion_fixture_baseline_risk_labels".to_string(),
        serde_json::to_value(&metadata.promotion_fixture_baseline_risk_labels)
            .unwrap_or(Value::Null),
    );
    out.insert(
        "promotion_fixture_candidate_risk_labels".to_string(),
        serde_json::to_value(&metadata.promotion_fixture_candidate_risk_labels)
            .unwrap_or(Value::Null),
    );
    out.insert(
        "promotion_model_route_baseline".to_string(),
        serde_json::to_value(&metadata.promotion_model_route_baseline).unwrap_or(Value::Null),
    );
    out.insert(
        "promotion_model_route_candidate".to_string(),
        serde_json::to_value(&metadata.promotion_model_route_candidate).unwrap_or(Value::Null),
    );
    Value::Object(out)
}

pub(crate) fn build_graph_hypotheses_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_hypothesis_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_hypothesis(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph hypothesis relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut hypotheses = BTreeSet::new();
    let mut failures = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut status_counts = BTreeMap::<String, usize>::new();
    let mut supporting_events = BTreeSet::new();
    let mut contradicting_events = BTreeSet::new();
    let mut counted_hypotheses = BTreeSet::new();
    for step in &steps {
        if let Some(key) = graph_timeline_step_hypothesis_key(step, &metadata) {
            if let Some(hypothesis) = metadata.get(&key) {
                hypotheses.insert(hypothesis.hypothesis_id.clone());
                if counted_hypotheses.insert(hypothesis.hypothesis_id.clone()) {
                    if let Some(status) = hypothesis.status.as_deref() {
                        *status_counts.entry(status.to_string()).or_default() += 1;
                    }
                    if let Some(failure_id) = hypothesis.failure_event_id.as_deref() {
                        failures.insert(failure_id.to_string());
                    }
                }
            }
        } else if step.dst_kind == "hypothesis" {
            hypotheses.insert(step.dst_id.clone());
        }
        match step.relation.as_str() {
            "supports" => {
                supporting_events.insert(step.src_id.clone());
            }
            "contradicts" => {
                contradicting_events.insert(step.src_id.clone());
            }
            "caused_by" | "explains" => {
                if infer_graph_node_kind(&step.src_id) == "event" {
                    failures.insert(step.src_id.clone());
                }
                if infer_graph_node_kind(&step.dst_id) == "event" {
                    failures.insert(step.dst_id.clone());
                }
            }
            _ => {}
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let hypotheses = hypotheses.into_iter().collect::<Vec<_>>();
    let failures = failures.into_iter().collect::<Vec<_>>();
    let supporting_events = supporting_events.into_iter().collect::<Vec<_>>();
    let contradicting_events = contradicting_events.into_iter().collect::<Vec<_>>();

    let mut out = String::new();
    out.push_str(&format!(
        "State graph hypotheses: {id} depth={max_depth} limit={limit}\n"
    ));
    out.push_str(&format!(
        "  hypotheses: {}\n",
        compact_id_list(&hypotheses, 8)
    ));
    out.push_str(&format!(
        "  failures:   {}\n",
        compact_id_list(&failures, 8)
    ));
    out.push_str(&format!(
        "  statuses:   {}\n",
        format_count_map(&status_counts)
    ));
    out.push_str(&format!(
        "  supports:   {}\n",
        compact_id_list(&supporting_events, 8)
    ));
    out.push_str(&format!(
        "  contradicts: {}\n",
        compact_id_list(&contradicting_events, 8)
    ));
    out.push_str(&format!(
        "  by relation: {}\n",
        format_count_map(&relation_counts)
    ));
    for step in steps {
        let detail = graph_timeline_step_hypothesis_key(&step, &metadata)
            .as_deref()
            .and_then(|key| metadata.get(key))
            .map(format_graph_hypothesis_metadata)
            .unwrap_or_else(|| {
                "hypothesis=- failure=- confidence=- status=- summary=- run=-".to_string()
            });
        out.push_str(&format!(
            "  t={} d{} {} {} -[{}]-> {} ({}) {} via {}\n",
            step.timestamp_ms,
            step.depth,
            step.event_type,
            step.src_id,
            step.relation,
            step.dst_id,
            step.dst_kind,
            detail,
            step.event_id
        ));
    }
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_hypotheses_payload(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<Value, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_hypothesis_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_hypothesis(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph hypothesis relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut hypotheses = BTreeSet::new();
    let mut failures = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut status_counts = BTreeMap::<String, usize>::new();
    let mut supporting_events = BTreeSet::new();
    let mut contradicting_events = BTreeSet::new();
    let mut counted_hypotheses = BTreeSet::new();
    for step in &steps {
        if let Some(hypothesis) = graph_hypothesis_for_timeline_step(step, &metadata) {
            hypotheses.insert(hypothesis.hypothesis_id.clone());
            if counted_hypotheses.insert(hypothesis.hypothesis_id.clone()) {
                if let Some(status) = hypothesis.status.as_deref() {
                    *status_counts.entry(status.to_string()).or_default() += 1;
                }
                if let Some(failure_id) = hypothesis.failure_event_id.as_deref() {
                    failures.insert(failure_id.to_string());
                }
            }
        } else if step.dst_kind == "hypothesis" {
            hypotheses.insert(step.dst_id.clone());
        }
        match step.relation.as_str() {
            "supports" => {
                supporting_events.insert(step.src_id.clone());
            }
            "contradicts" => {
                contradicting_events.insert(step.src_id.clone());
            }
            "caused_by" | "explains" => {
                if infer_graph_node_kind(&step.src_id) == "event" {
                    failures.insert(step.src_id.clone());
                }
                if infer_graph_node_kind(&step.dst_id) == "event" {
                    failures.insert(step.dst_id.clone());
                }
            }
            _ => {}
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let hypotheses = hypotheses.into_iter().collect::<Vec<_>>();
    let failures = failures.into_iter().collect::<Vec<_>>();
    let supporting_events = supporting_events.into_iter().collect::<Vec<_>>();
    let contradicting_events = contradicting_events.into_iter().collect::<Vec<_>>();
    let hypothesis_metadata = hypotheses
        .iter()
        .filter_map(|hypothesis_id| {
            metadata.get(hypothesis_id).map(|metadata| {
                (
                    hypothesis_id.clone(),
                    graph_hypothesis_metadata_payload(metadata),
                )
            })
        })
        .collect::<BTreeMap<_, _>>();
    let hypothesis_relations = graph_hypothesis_relations_payload(&steps, &metadata);

    Ok(serde_json::json!({
        "diagnostic": "state_graph_hypotheses",
        "id": id,
        "depth": max_depth,
        "limit": limit,
        "hypothesis_count": hypotheses.len(),
        "failure_count": failures.len(),
        "relation_count": steps.len(),
        "hypotheses": hypotheses,
        "failures": failures,
        "statuses": status_counts,
        "supporting_events": supporting_events,
        "contradicting_events": contradicting_events,
        "relations": relation_counts,
        "hypothesis_metadata": hypothesis_metadata,
        "hypothesis_relations": hypothesis_relations,
    }))
}

fn graph_hypothesis_for_timeline_step<'a>(
    step: &GraphTimelineStep,
    metadata: &'a BTreeMap<String, GraphHypothesisMetadata>,
) -> Option<&'a GraphHypothesisMetadata> {
    graph_timeline_step_hypothesis_key(step, metadata).and_then(|key| metadata.get(&key))
}

fn graph_hypothesis_relations_payload(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphHypothesisMetadata>,
) -> Vec<Value> {
    steps
        .iter()
        .map(|step| {
            let hypothesis_id = graph_hypothesis_for_timeline_step(step, metadata)
                .map(|hypothesis| hypothesis.hypothesis_id.clone())
                .or_else(|| (step.dst_kind == "hypothesis").then_some(step.dst_id.clone()))
                .or_else(|| {
                    (infer_graph_node_kind(&step.src_id) == "hypothesis")
                        .then_some(step.src_id.clone())
                });
            serde_json::json!({
                "timestamp_ms": step.timestamp_ms,
                "depth": step.depth,
                "event_id": &step.event_id,
                "event_type": &step.event_type,
                "src_id": &step.src_id,
                "relation": &step.relation,
                "dst_id": &step.dst_id,
                "dst_kind": &step.dst_kind,
                "hypothesis_id": hypothesis_id,
            })
        })
        .collect()
}

fn graph_hypothesis_metadata_payload(metadata: &GraphHypothesisMetadata) -> Value {
    serde_json::json!({
        "hypothesis_id": &metadata.hypothesis_id,
        "event_id": &metadata.event_id,
        "failure_event_id": &metadata.failure_event_id,
        "summary": &metadata.summary,
        "confidence": metadata.confidence,
        "status": &metadata.status,
        "run_id": &metadata.run_id,
    })
}

pub(crate) fn build_graph_versions_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_version_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_version(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph harness version relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut versions = BTreeSet::new();
    let mut base_versions = BTreeSet::new();
    let mut patches = BTreeSet::new();
    let mut evals = BTreeSet::new();
    let mut statuses = BTreeMap::<String, usize>::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut counted_events = BTreeSet::new();
    for step in &steps {
        if let Some(key) = graph_timeline_step_version_key(step, &metadata) {
            if let Some(version) = metadata.get(&key) {
                if let Some(harness_version) = version.harness_version.as_deref() {
                    versions.insert(harness_version.to_string());
                }
                if let Some(base_harness_version) = version.base_harness_version.as_deref() {
                    base_versions.insert(base_harness_version.to_string());
                    versions.insert(base_harness_version.to_string());
                }
                if let Some(patch_id) = version.patch_id.as_deref() {
                    patches.insert(patch_id.to_string());
                }
                if let Some(eval_id) = version.eval_id.as_deref() {
                    evals.insert(eval_id.to_string());
                }
                if counted_events.insert(version.event_id.clone()) {
                    if let Some(status) = version.status.as_deref() {
                        *statuses.entry(status.to_string()).or_default() += 1;
                    }
                }
            }
        } else if step.dst_kind == "harness_version" {
            versions.insert(step.dst_id.clone());
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let versions = versions.into_iter().collect::<Vec<_>>();
    let base_versions = base_versions.into_iter().collect::<Vec<_>>();
    let patches = patches.into_iter().collect::<Vec<_>>();
    let evals = evals.into_iter().collect::<Vec<_>>();

    let mut out = String::new();
    out.push_str(&format!(
        "State graph versions: {id} depth={max_depth} limit={limit}\n"
    ));
    out.push_str(&format!(
        "  versions:   {}\n",
        compact_id_list(&versions, 8)
    ));
    out.push_str(&format!(
        "  bases:      {}\n",
        compact_id_list(&base_versions, 8)
    ));
    out.push_str(&format!("  patches:    {}\n", compact_id_list(&patches, 8)));
    out.push_str(&format!("  evals:      {}\n", compact_id_list(&evals, 8)));
    out.push_str(&format!("  statuses:   {}\n", format_count_map(&statuses)));
    out.push_str(&format!(
        "  by relation: {}\n",
        format_count_map(&relation_counts)
    ));
    for step in steps {
        let detail = graph_timeline_step_version_key(&step, &metadata)
            .as_deref()
            .and_then(|key| metadata.get(key))
            .map(format_graph_version_metadata)
            .unwrap_or_else(|| {
                "harness=- base=- patch=- eval=- suite=- status=- run=- source_event=-".to_string()
            });
        out.push_str(&format!(
            "  t={} d{} {} {} -[{}]-> {} ({}) {} via {}\n",
            step.timestamp_ms,
            step.depth,
            step.event_type,
            step.src_id,
            step.relation,
            step.dst_id,
            step.dst_kind,
            detail,
            step.event_id
        ));
    }
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_versions_payload(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<Value, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_version_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_version(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph harness version relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut versions = BTreeSet::new();
    let mut base_versions = BTreeSet::new();
    let mut patches = BTreeSet::new();
    let mut evals = BTreeSet::new();
    let mut statuses = BTreeMap::<String, usize>::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut counted_events = BTreeSet::new();
    for step in &steps {
        if let Some(version) = graph_version_for_timeline_step(step, &metadata) {
            if let Some(harness_version) = version.harness_version.as_deref() {
                versions.insert(harness_version.to_string());
            }
            if let Some(base_harness_version) = version.base_harness_version.as_deref() {
                base_versions.insert(base_harness_version.to_string());
                versions.insert(base_harness_version.to_string());
            }
            if let Some(patch_id) = version.patch_id.as_deref() {
                patches.insert(patch_id.to_string());
            }
            if let Some(eval_id) = version.eval_id.as_deref() {
                evals.insert(eval_id.to_string());
            }
            if counted_events.insert(version.event_id.clone()) {
                if let Some(status) = version.status.as_deref() {
                    *statuses.entry(status.to_string()).or_default() += 1;
                }
            }
        } else if step.dst_kind == "harness_version" {
            versions.insert(step.dst_id.clone());
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let versions = versions.into_iter().collect::<Vec<_>>();
    let base_versions = base_versions.into_iter().collect::<Vec<_>>();
    let patches = patches.into_iter().collect::<Vec<_>>();
    let evals = evals.into_iter().collect::<Vec<_>>();
    let version_metadata = version_metadata_payload_map(&steps, &metadata);
    let version_relations = graph_version_relations_payload(&steps, &metadata);

    Ok(serde_json::json!({
        "diagnostic": "state_graph_versions",
        "id": id,
        "depth": max_depth,
        "limit": limit,
        "version_count": versions.len(),
        "base_version_count": base_versions.len(),
        "patch_count": patches.len(),
        "eval_count": evals.len(),
        "relation_count": steps.len(),
        "versions": versions,
        "base_versions": base_versions,
        "patches": patches,
        "evals": evals,
        "statuses": statuses,
        "relations": relation_counts,
        "version_metadata": version_metadata,
        "version_relations": version_relations,
    }))
}

fn graph_version_for_timeline_step<'a>(
    step: &GraphTimelineStep,
    metadata: &'a BTreeMap<String, GraphVersionMetadata>,
) -> Option<&'a GraphVersionMetadata> {
    graph_timeline_step_version_key(step, metadata).and_then(|key| metadata.get(&key))
}

fn version_metadata_payload_map(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphVersionMetadata>,
) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    for step in steps {
        if let Some(version) = graph_version_for_timeline_step(step, metadata) {
            out.entry(version.event_id.clone())
                .or_insert_with(|| graph_version_metadata_payload(version));
        }
    }
    out
}

fn graph_version_relations_payload(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphVersionMetadata>,
) -> Vec<Value> {
    steps
        .iter()
        .map(|step| {
            let version_event_id = graph_version_for_timeline_step(step, metadata)
                .map(|version| version.event_id.clone());
            serde_json::json!({
                "timestamp_ms": step.timestamp_ms,
                "depth": step.depth,
                "event_id": &step.event_id,
                "event_type": &step.event_type,
                "src_id": &step.src_id,
                "relation": &step.relation,
                "dst_id": &step.dst_id,
                "dst_kind": &step.dst_kind,
                "version_event_id": version_event_id,
            })
        })
        .collect()
}

fn graph_version_metadata_payload(metadata: &GraphVersionMetadata) -> Value {
    serde_json::json!({
        "event_id": &metadata.event_id,
        "event_type": &metadata.event_type,
        "harness_version": &metadata.harness_version,
        "base_harness_version": &metadata.base_harness_version,
        "patch_id": &metadata.patch_id,
        "eval_id": &metadata.eval_id,
        "suite": &metadata.suite,
        "status": &metadata.status,
        "run_id": &metadata.run_id,
    })
}

pub(crate) fn build_graph_runs_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_run_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_run(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph run, trace, or task relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut runs = BTreeSet::new();
    let mut traces = BTreeSet::new();
    let mut tasks = BTreeSet::new();
    let mut statuses = BTreeMap::<String, usize>::new();
    let mut event_types = BTreeMap::<String, usize>::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut counted_events = BTreeSet::new();
    for step in &steps {
        if let Some(key) = graph_timeline_step_run_key(step, &metadata) {
            if let Some(run) = metadata.get(&key) {
                if let Some(run_id) = run.run_id.as_deref() {
                    runs.insert(run_id.to_string());
                }
                if let Some(trace_id) = run.trace_id.as_deref() {
                    traces.insert(trace_id.to_string());
                }
                for task_id in &run.task_ids {
                    tasks.insert(task_id.clone());
                }
                if counted_events.insert(run.event_id.clone()) {
                    *event_types.entry(run.event_type.clone()).or_default() += 1;
                    if let Some(status) = run.status.as_deref() {
                        *statuses.entry(status.to_string()).or_default() += 1;
                    }
                }
            }
        }
        match step.dst_kind.as_str() {
            "run" => {
                runs.insert(step.dst_id.clone());
            }
            "trace" => {
                traces.insert(step.dst_id.clone());
            }
            "task" => {
                tasks.insert(step.dst_id.clone());
            }
            _ => {}
        }
        if infer_graph_node_kind(&step.src_id) == "run" {
            runs.insert(step.src_id.clone());
        } else if infer_graph_node_kind(&step.src_id) == "trace" {
            traces.insert(step.src_id.clone());
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let runs = runs.into_iter().collect::<Vec<_>>();
    let traces = traces.into_iter().collect::<Vec<_>>();
    let tasks = tasks.into_iter().collect::<Vec<_>>();

    let mut out = String::new();
    out.push_str(&format!(
        "State graph runs: {id} depth={max_depth} limit={limit}\n"
    ));
    out.push_str(&format!("  runs:       {}\n", compact_id_list(&runs, 8)));
    out.push_str(&format!("  traces:     {}\n", compact_id_list(&traces, 8)));
    out.push_str(&format!("  tasks:      {}\n", compact_id_list(&tasks, 8)));
    out.push_str(&format!("  statuses:   {}\n", format_count_map(&statuses)));
    out.push_str(&format!(
        "  event types: {}\n",
        format_count_map(&event_types)
    ));
    out.push_str(&format!(
        "  by relation: {}\n",
        format_count_map(&relation_counts)
    ));
    for step in steps {
        let detail = graph_timeline_step_run_key(&step, &metadata)
            .as_deref()
            .and_then(|key| metadata.get(key))
            .map(format_graph_run_metadata)
            .unwrap_or_else(|| "run=- trace=- tasks=[] status=- source_event=-".to_string());
        out.push_str(&format!(
            "  t={} d{} {} {} -[{}]-> {} ({}) {} via {}\n",
            step.timestamp_ms,
            step.depth,
            step.event_type,
            step.src_id,
            step.relation,
            step.dst_id,
            step.dst_kind,
            detail,
            step.event_id
        ));
    }
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_runs_payload(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<Value, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_run_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_run(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph run, trace, or task relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut runs = BTreeSet::new();
    let mut traces = BTreeSet::new();
    let mut tasks = BTreeSet::new();
    let mut statuses = BTreeMap::<String, usize>::new();
    let mut event_types = BTreeMap::<String, usize>::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut counted_events = BTreeSet::new();
    for step in &steps {
        if let Some(run) = graph_run_for_timeline_step(step, &metadata) {
            if let Some(run_id) = run.run_id.as_deref() {
                runs.insert(run_id.to_string());
            }
            if let Some(trace_id) = run.trace_id.as_deref() {
                traces.insert(trace_id.to_string());
            }
            for task_id in &run.task_ids {
                tasks.insert(task_id.clone());
            }
            if counted_events.insert(run.event_id.clone()) {
                *event_types.entry(run.event_type.clone()).or_default() += 1;
                if let Some(status) = run.status.as_deref() {
                    *statuses.entry(status.to_string()).or_default() += 1;
                }
            }
        }
        match step.dst_kind.as_str() {
            "run" => {
                runs.insert(step.dst_id.clone());
            }
            "trace" => {
                traces.insert(step.dst_id.clone());
            }
            "task" => {
                tasks.insert(step.dst_id.clone());
            }
            _ => {}
        }
        if infer_graph_node_kind(&step.src_id) == "run" {
            runs.insert(step.src_id.clone());
        } else if infer_graph_node_kind(&step.src_id) == "trace" {
            traces.insert(step.src_id.clone());
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let runs = runs.into_iter().collect::<Vec<_>>();
    let traces = traces.into_iter().collect::<Vec<_>>();
    let tasks = tasks.into_iter().collect::<Vec<_>>();
    let run_metadata = run_metadata_payload_map(&steps, &metadata);
    let run_relations = graph_run_relations_payload(&steps, &metadata);

    Ok(serde_json::json!({
        "diagnostic": "state_graph_runs",
        "id": id,
        "depth": max_depth,
        "limit": limit,
        "run_count": runs.len(),
        "trace_count": traces.len(),
        "task_count": tasks.len(),
        "relation_count": steps.len(),
        "runs": runs,
        "traces": traces,
        "tasks": tasks,
        "statuses": statuses,
        "event_types": event_types,
        "relations": relation_counts,
        "run_metadata": run_metadata,
        "run_relations": run_relations,
    }))
}

fn graph_run_for_timeline_step<'a>(
    step: &GraphTimelineStep,
    metadata: &'a BTreeMap<String, GraphRunMetadata>,
) -> Option<&'a GraphRunMetadata> {
    graph_timeline_step_run_key(step, metadata).and_then(|key| metadata.get(&key))
}

fn run_metadata_payload_map(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphRunMetadata>,
) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    for step in steps {
        if let Some(run) = graph_run_for_timeline_step(step, metadata) {
            out.entry(run.event_id.clone())
                .or_insert_with(|| graph_run_metadata_payload(run));
        }
    }
    out
}

fn graph_run_relations_payload(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphRunMetadata>,
) -> Vec<Value> {
    steps
        .iter()
        .map(|step| {
            let run_event_id =
                graph_run_for_timeline_step(step, metadata).map(|run| run.event_id.clone());
            serde_json::json!({
                "timestamp_ms": step.timestamp_ms,
                "depth": step.depth,
                "event_id": &step.event_id,
                "event_type": &step.event_type,
                "src_id": &step.src_id,
                "relation": &step.relation,
                "dst_id": &step.dst_id,
                "dst_kind": &step.dst_kind,
                "run_event_id": run_event_id,
            })
        })
        .collect()
}

fn graph_run_metadata_payload(metadata: &GraphRunMetadata) -> Value {
    serde_json::json!({
        "event_id": &metadata.event_id,
        "event_type": &metadata.event_type,
        "run_id": &metadata.run_id,
        "trace_id": &metadata.trace_id,
        "task_ids": &metadata.task_ids,
        "status": &metadata.status,
    })
}

pub(crate) fn build_graph_artifacts_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_artifact_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_artifact(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph artifact relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut artifacts = BTreeSet::new();
    let mut evals = BTreeSet::new();
    let mut patches = BTreeSet::new();
    let mut statuses = BTreeMap::<String, usize>::new();
    let mut repro_modes = BTreeMap::<String, usize>::new();
    let mut agent_sources = BTreeMap::<String, usize>::new();
    let mut dirty_counts = BTreeMap::<String, usize>::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut counted_events = BTreeSet::new();
    for step in &steps {
        if let Some(key) = graph_timeline_step_artifact_key(step, &metadata) {
            if let Some(artifact) = metadata.get(&key) {
                artifacts.insert(artifact.artifact_uri.clone());
                if let Some(eval_id) = artifact.eval_id.as_deref() {
                    evals.insert(eval_id.to_string());
                }
                if let Some(patch_id) = artifact.patch_id.as_deref() {
                    patches.insert(patch_id.to_string());
                }
                if counted_events.insert(artifact.event_id.clone()) {
                    if let Some(status) = artifact.status.as_deref() {
                        *statuses.entry(status.to_string()).or_default() += 1;
                    }
                    if let Some(mode) = artifact.repro_mode.as_deref() {
                        *repro_modes.entry(mode.to_string()).or_default() += 1;
                    }
                    if let Some(source) = artifact.agent_command_source.as_deref() {
                        *agent_sources.entry(source.to_string()).or_default() += 1;
                    }
                    if let Some(dirty) = artifact.git_dirty {
                        let label = if dirty { "dirty" } else { "clean" };
                        *dirty_counts.entry(label.to_string()).or_default() += 1;
                    }
                }
            }
        }
        if step.dst_kind == "artifact" {
            artifacts.insert(step.dst_id.clone());
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let artifacts = artifacts.into_iter().collect::<Vec<_>>();
    let evals = evals.into_iter().collect::<Vec<_>>();
    let patches = patches.into_iter().collect::<Vec<_>>();

    let mut out = String::new();
    out.push_str(&format!(
        "State graph artifacts: {id} depth={max_depth} limit={limit}\n"
    ));
    out.push_str(&format!(
        "  artifacts:  {}\n",
        compact_id_list(&artifacts, 6)
    ));
    out.push_str(&format!("  evals:      {}\n", compact_id_list(&evals, 8)));
    out.push_str(&format!("  patches:    {}\n", compact_id_list(&patches, 8)));
    out.push_str(&format!("  statuses:   {}\n", format_count_map(&statuses)));
    out.push_str(&format!(
        "  repro modes: {}\n",
        format_count_map(&repro_modes)
    ));
    out.push_str(&format!(
        "  agent sources: {}\n",
        format_count_map(&agent_sources)
    ));
    out.push_str(&format!(
        "  dirtiness:  {}\n",
        format_count_map(&dirty_counts)
    ));
    out.push_str(&format!(
        "  by relation: {}\n",
        format_count_map(&relation_counts)
    ));
    for step in steps {
        let detail = graph_timeline_step_artifact_key(&step, &metadata)
            .as_deref()
            .and_then(|key| metadata.get(key))
            .map(format_graph_artifact_metadata)
            .unwrap_or_else(|| {
                "artifact=- eval=- patch=- suite=- status=- repro=- agent_source=- dirty=- commands=0 fixture_tasks=- fixture_commands=- run=- source_event=-".to_string()
            });
        out.push_str(&format!(
            "  t={} d{} {} {} -[{}]-> {} ({}) {} via {}\n",
            step.timestamp_ms,
            step.depth,
            step.event_type,
            step.src_id,
            step.relation,
            step.dst_id,
            step.dst_kind,
            detail,
            step.event_id
        ));
    }
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_artifacts_payload(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<Value, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_artifact_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_artifact(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph artifact relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut artifacts = BTreeSet::new();
    let mut evals = BTreeSet::new();
    let mut patches = BTreeSet::new();
    let mut statuses = BTreeMap::<String, usize>::new();
    let mut repro_modes = BTreeMap::<String, usize>::new();
    let mut agent_sources = BTreeMap::<String, usize>::new();
    let mut dirty_counts = BTreeMap::<String, usize>::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut counted_events = BTreeSet::new();
    for step in &steps {
        if let Some(artifact) = graph_artifact_for_timeline_step(step, &metadata) {
            artifacts.insert(artifact.artifact_uri.clone());
            if let Some(eval_id) = artifact.eval_id.as_deref() {
                evals.insert(eval_id.to_string());
            }
            if let Some(patch_id) = artifact.patch_id.as_deref() {
                patches.insert(patch_id.to_string());
            }
            if counted_events.insert(artifact.event_id.clone()) {
                if let Some(status) = artifact.status.as_deref() {
                    *statuses.entry(status.to_string()).or_default() += 1;
                }
                if let Some(mode) = artifact.repro_mode.as_deref() {
                    *repro_modes.entry(mode.to_string()).or_default() += 1;
                }
                if let Some(source) = artifact.agent_command_source.as_deref() {
                    *agent_sources.entry(source.to_string()).or_default() += 1;
                }
                if let Some(dirty) = artifact.git_dirty {
                    let label = if dirty { "dirty" } else { "clean" };
                    *dirty_counts.entry(label.to_string()).or_default() += 1;
                }
            }
        }
        if step.dst_kind == "artifact" {
            artifacts.insert(step.dst_id.clone());
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let artifacts = artifacts.into_iter().collect::<Vec<_>>();
    let evals = evals.into_iter().collect::<Vec<_>>();
    let patches = patches.into_iter().collect::<Vec<_>>();
    let artifact_metadata = artifact_metadata_payload_map(&steps, &metadata);
    let artifact_relations = graph_artifact_relations_payload(&steps, &metadata);

    Ok(serde_json::json!({
        "diagnostic": "state_graph_artifacts",
        "id": id,
        "depth": max_depth,
        "limit": limit,
        "artifact_count": artifacts.len(),
        "eval_count": evals.len(),
        "patch_count": patches.len(),
        "relation_count": steps.len(),
        "artifacts": artifacts,
        "evals": evals,
        "patches": patches,
        "statuses": statuses,
        "repro_modes": repro_modes,
        "agent_sources": agent_sources,
        "dirtiness": dirty_counts,
        "relations": relation_counts,
        "artifact_metadata": artifact_metadata,
        "artifact_relations": artifact_relations,
    }))
}

fn graph_artifact_for_timeline_step<'a>(
    step: &GraphTimelineStep,
    metadata: &'a BTreeMap<String, GraphArtifactMetadata>,
) -> Option<&'a GraphArtifactMetadata> {
    graph_timeline_step_artifact_key(step, metadata).and_then(|key| metadata.get(&key))
}

fn artifact_metadata_payload_map(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphArtifactMetadata>,
) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    for step in steps {
        if let Some(artifact) = graph_artifact_for_timeline_step(step, metadata) {
            out.entry(artifact.artifact_uri.clone())
                .or_insert_with(|| graph_artifact_metadata_payload(artifact));
        }
    }
    out
}

fn graph_artifact_relations_payload(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphArtifactMetadata>,
) -> Vec<Value> {
    steps
        .iter()
        .map(|step| {
            let artifact_uri = graph_artifact_for_timeline_step(step, metadata)
                .map(|artifact| artifact.artifact_uri.clone());
            serde_json::json!({
                "timestamp_ms": step.timestamp_ms,
                "depth": step.depth,
                "event_id": &step.event_id,
                "event_type": &step.event_type,
                "src_id": &step.src_id,
                "relation": &step.relation,
                "dst_id": &step.dst_id,
                "dst_kind": &step.dst_kind,
                "artifact_uri": artifact_uri,
            })
        })
        .collect()
}

fn graph_artifact_metadata_payload(metadata: &GraphArtifactMetadata) -> Value {
    serde_json::json!({
        "event_id": &metadata.event_id,
        "event_type": &metadata.event_type,
        "artifact_uri": &metadata.artifact_uri,
        "eval_id": &metadata.eval_id,
        "patch_id": &metadata.patch_id,
        "suite": &metadata.suite,
        "status": &metadata.status,
        "repro_mode": &metadata.repro_mode,
        "agent_command_source": &metadata.agent_command_source,
        "replay_command": &metadata.replay_command,
        "git_dirty": metadata.git_dirty,
        "command_count": metadata.command_count,
        "fixture_task_count": metadata.fixture_task_count,
        "fixture_command_count": metadata.fixture_command_count,
        "run_id": &metadata.run_id,
    })
}

pub(crate) fn build_graph_models_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_model_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_model(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph model relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut model_calls = BTreeSet::new();
    let mut models = BTreeSet::new();
    let mut route_tasks = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut input_tokens = 0u64;
    let mut output_tokens = 0u64;
    let mut cache_read_tokens = 0u64;
    let mut cache_write_tokens = 0u64;
    let mut counted_events = BTreeSet::new();
    for step in &steps {
        if let Some(key) = graph_timeline_step_model_key(step, &metadata) {
            if let Some(model_call) = metadata.get(&key) {
                model_calls.insert(model_call.model_call_id.clone());
                if let Some(model) = model_call.model.as_deref() {
                    models.insert(model.to_string());
                }
                if let Some(route_task) = model_call.route_task.as_deref() {
                    route_tasks.insert(route_task.to_string());
                }
                if model_call.event_type == "ModelCallCompleted"
                    && counted_events.insert(model_call.event_id.clone())
                {
                    input_tokens += model_call.input_tokens.unwrap_or(0);
                    output_tokens += model_call.output_tokens.unwrap_or(0);
                    cache_read_tokens += model_call.cache_read_tokens.unwrap_or(0);
                    cache_write_tokens += model_call.cache_write_tokens.unwrap_or(0);
                }
            }
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let model_calls = model_calls.into_iter().collect::<Vec<_>>();
    let models = models.into_iter().collect::<Vec<_>>();
    let route_tasks = route_tasks.into_iter().collect::<Vec<_>>();

    let mut out = String::new();
    out.push_str(&format!(
        "State graph models: {id} depth={max_depth} limit={limit}\n"
    ));
    out.push_str(&format!(
        "  calls:      {}\n",
        compact_id_list(&model_calls, 8)
    ));
    out.push_str(&format!("  models:     {}\n", compact_id_list(&models, 8)));
    out.push_str(&format!(
        "  route tasks: {}\n",
        compact_id_list(&route_tasks, 8)
    ));
    out.push_str(&format!(
        "  tokens:     in={} out={} cache_read={} cache_write={}\n",
        input_tokens, output_tokens, cache_read_tokens, cache_write_tokens
    ));
    out.push_str(&format!(
        "  by relation: {}\n",
        format_count_map(&relation_counts)
    ));
    for step in steps {
        let detail = graph_timeline_step_model_key(&step, &metadata)
            .as_deref()
            .and_then(|key| metadata.get(key))
            .map(format_graph_model_metadata)
            .unwrap_or_else(|| {
                "call=- model=- route=- thinking=- effort=- tokens=in:- out:- cache_read:- cache_write:- run=-"
                    .to_string()
            });
        out.push_str(&format!(
            "  t={} d{} {} {} -[{}]-> {} ({}) {} via {}\n",
            step.timestamp_ms,
            step.depth,
            step.event_type,
            step.src_id,
            step.relation,
            step.dst_id,
            step.dst_kind,
            detail,
            step.event_id
        ));
    }
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_models_payload(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<Value, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_model_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_model(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph model relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut model_calls = BTreeSet::new();
    let mut models = BTreeSet::new();
    let mut route_tasks = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut input_tokens = 0u64;
    let mut output_tokens = 0u64;
    let mut cache_read_tokens = 0u64;
    let mut cache_write_tokens = 0u64;
    let mut counted_events = BTreeSet::new();
    for step in &steps {
        if let Some(model_call) = graph_model_for_timeline_step(step, &metadata) {
            model_calls.insert(model_call.model_call_id.clone());
            if let Some(model) = model_call.model.as_deref() {
                models.insert(model.to_string());
            }
            if let Some(route_task) = model_call.route_task.as_deref() {
                route_tasks.insert(route_task.to_string());
            }
            if model_call.event_type == "ModelCallCompleted"
                && counted_events.insert(model_call.event_id.clone())
            {
                input_tokens += model_call.input_tokens.unwrap_or(0);
                output_tokens += model_call.output_tokens.unwrap_or(0);
                cache_read_tokens += model_call.cache_read_tokens.unwrap_or(0);
                cache_write_tokens += model_call.cache_write_tokens.unwrap_or(0);
            }
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let model_calls = model_calls.into_iter().collect::<Vec<_>>();
    let models = models.into_iter().collect::<Vec<_>>();
    let route_tasks = route_tasks.into_iter().collect::<Vec<_>>();
    let model_metadata = model_metadata_payload_map(&steps, &metadata);
    let model_relations = graph_model_relations_payload(&steps, &metadata);

    Ok(serde_json::json!({
        "diagnostic": "state_graph_models",
        "id": id,
        "depth": max_depth,
        "limit": limit,
        "model_call_count": model_calls.len(),
        "model_count": models.len(),
        "route_task_count": route_tasks.len(),
        "relation_count": steps.len(),
        "model_calls": model_calls,
        "models": models,
        "route_tasks": route_tasks,
        "tokens": {
            "input": input_tokens,
            "output": output_tokens,
            "cache_read": cache_read_tokens,
            "cache_write": cache_write_tokens,
        },
        "relations": relation_counts,
        "model_metadata": model_metadata,
        "model_relations": model_relations,
    }))
}

fn graph_model_for_timeline_step<'a>(
    step: &GraphTimelineStep,
    metadata: &'a BTreeMap<String, GraphModelMetadata>,
) -> Option<&'a GraphModelMetadata> {
    graph_timeline_step_model_key(step, metadata).and_then(|key| metadata.get(&key))
}

fn model_metadata_payload_map(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphModelMetadata>,
) -> BTreeMap<String, Value> {
    let mut selected = BTreeMap::<String, GraphModelMetadata>::new();
    for step in steps {
        if let Some(model_call) = graph_model_for_timeline_step(step, metadata) {
            selected
                .entry(model_call.model_call_id.clone())
                .and_modify(|existing| {
                    if existing.event_type != "ModelCallCompleted"
                        && model_call.event_type == "ModelCallCompleted"
                    {
                        *existing = model_call.clone();
                    }
                })
                .or_insert_with(|| model_call.clone());
        }
    }
    selected
        .into_iter()
        .map(|(call_id, metadata)| (call_id, graph_model_metadata_payload(&metadata)))
        .collect()
}

fn graph_model_relations_payload(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphModelMetadata>,
) -> Vec<Value> {
    steps
        .iter()
        .map(|step| {
            let model_call_id = graph_model_for_timeline_step(step, metadata)
                .map(|call| call.model_call_id.clone());
            serde_json::json!({
                "timestamp_ms": step.timestamp_ms,
                "depth": step.depth,
                "event_id": &step.event_id,
                "event_type": &step.event_type,
                "src_id": &step.src_id,
                "relation": &step.relation,
                "dst_id": &step.dst_id,
                "dst_kind": &step.dst_kind,
                "model_call_id": model_call_id,
            })
        })
        .collect()
}

fn graph_model_metadata_payload(metadata: &GraphModelMetadata) -> Value {
    serde_json::json!({
        "event_id": &metadata.event_id,
        "event_type": &metadata.event_type,
        "model_call_id": &metadata.model_call_id,
        "model": &metadata.model,
        "route_task": &metadata.route_task,
        "thinking": &metadata.thinking,
        "reasoning_effort": &metadata.reasoning_effort,
        "input_tokens": metadata.input_tokens,
        "output_tokens": metadata.output_tokens,
        "cache_read_tokens": metadata.cache_read_tokens,
        "cache_write_tokens": metadata.cache_write_tokens,
        "run_id": &metadata.run_id,
    })
}

pub(crate) fn build_graph_tools_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_tool_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_tool(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph tool relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut tool_calls = BTreeSet::new();
    let mut tools = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut status_counts = BTreeMap::<String, usize>::new();
    let mut counted_calls = BTreeSet::new();
    for step in &steps {
        if let Some(key) = graph_timeline_step_tool_key(step, &metadata) {
            if let Some(tool_call) = metadata.get(&key) {
                tool_calls.insert(tool_call.tool_call_id.clone());
                if let Some(tool_name) = tool_call.tool_name.as_deref() {
                    tools.insert(tool_name.to_string());
                }
                if tool_call.event_type == "ToolCallCompleted"
                    && counted_calls.insert(tool_call.tool_call_id.clone())
                {
                    *status_counts
                        .entry(
                            tool_call
                                .status
                                .clone()
                                .unwrap_or_else(|| "unknown".to_string()),
                        )
                        .or_default() += 1;
                }
            }
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let tool_calls = tool_calls.into_iter().collect::<Vec<_>>();
    let tools = tools.into_iter().collect::<Vec<_>>();

    let mut out = String::new();
    out.push_str(&format!(
        "State graph tools: {id} depth={max_depth} limit={limit}\n"
    ));
    out.push_str(&format!(
        "  calls:      {}\n",
        compact_id_list(&tool_calls, 8)
    ));
    out.push_str(&format!("  tools:      {}\n", compact_id_list(&tools, 8)));
    out.push_str(&format!(
        "  statuses:   {}\n",
        format_count_map(&status_counts)
    ));
    out.push_str(&format!(
        "  by relation: {}\n",
        format_count_map(&relation_counts)
    ));
    for step in steps {
        let detail = graph_timeline_step_tool_key(&step, &metadata)
            .as_deref()
            .and_then(|key| metadata.get(key))
            .map(format_graph_tool_metadata)
            .unwrap_or_else(|| "call=- tool=- status=- args=- result=- run=-".to_string());
        out.push_str(&format!(
            "  t={} d{} {} {} -[{}]-> {} ({}) {} via {}\n",
            step.timestamp_ms,
            step.depth,
            step.event_type,
            step.src_id,
            step.relation,
            step.dst_id,
            step.dst_kind,
            detail,
            step.event_id
        ));
    }
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_tools_payload(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<Value, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_tool_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_tool(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph tool relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut tool_calls = BTreeSet::new();
    let mut tools = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut status_counts = BTreeMap::<String, usize>::new();
    let mut counted_calls = BTreeSet::new();
    for step in &steps {
        if let Some(tool_call) = graph_tool_for_timeline_step(step, &metadata) {
            tool_calls.insert(tool_call.tool_call_id.clone());
            if let Some(tool_name) = tool_call.tool_name.as_deref() {
                tools.insert(tool_name.to_string());
            }
            if tool_call.event_type == "ToolCallCompleted"
                && counted_calls.insert(tool_call.tool_call_id.clone())
            {
                *status_counts
                    .entry(
                        tool_call
                            .status
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string()),
                    )
                    .or_default() += 1;
            }
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let tool_calls = tool_calls.into_iter().collect::<Vec<_>>();
    let tools = tools.into_iter().collect::<Vec<_>>();
    let tool_metadata = tool_metadata_payload_map(&steps, &metadata);
    let tool_relations = graph_tool_relations_payload(&steps, &metadata);

    Ok(serde_json::json!({
        "diagnostic": "state_graph_tools",
        "id": id,
        "depth": max_depth,
        "limit": limit,
        "tool_call_count": tool_calls.len(),
        "tool_count": tools.len(),
        "relation_count": steps.len(),
        "tool_calls": tool_calls,
        "tools": tools,
        "statuses": status_counts,
        "relations": relation_counts,
        "tool_metadata": tool_metadata,
        "tool_relations": tool_relations,
    }))
}

fn graph_tool_for_timeline_step<'a>(
    step: &GraphTimelineStep,
    metadata: &'a BTreeMap<String, GraphToolMetadata>,
) -> Option<&'a GraphToolMetadata> {
    graph_timeline_step_tool_key(step, metadata).and_then(|key| metadata.get(&key))
}

fn tool_metadata_payload_map(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphToolMetadata>,
) -> BTreeMap<String, Value> {
    let mut selected = BTreeMap::<String, GraphToolMetadata>::new();
    for step in steps {
        if let Some(tool_call) = graph_tool_for_timeline_step(step, metadata) {
            selected
                .entry(tool_call.tool_call_id.clone())
                .and_modify(|existing| {
                    if existing.event_type != "ToolCallCompleted"
                        && tool_call.event_type == "ToolCallCompleted"
                    {
                        *existing = tool_call.clone();
                    }
                })
                .or_insert_with(|| tool_call.clone());
        }
    }
    selected
        .into_iter()
        .map(|(call_id, metadata)| (call_id, graph_tool_metadata_payload(&metadata)))
        .collect()
}

fn graph_tool_relations_payload(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphToolMetadata>,
) -> Vec<Value> {
    steps
        .iter()
        .map(|step| {
            let tool_call_id =
                graph_tool_for_timeline_step(step, metadata).map(|call| call.tool_call_id.clone());
            serde_json::json!({
                "timestamp_ms": step.timestamp_ms,
                "depth": step.depth,
                "event_id": &step.event_id,
                "event_type": &step.event_type,
                "src_id": &step.src_id,
                "relation": &step.relation,
                "dst_id": &step.dst_id,
                "dst_kind": &step.dst_kind,
                "tool_call_id": tool_call_id,
            })
        })
        .collect()
}

fn graph_tool_metadata_payload(metadata: &GraphToolMetadata) -> Value {
    serde_json::json!({
        "event_id": &metadata.event_id,
        "event_type": &metadata.event_type,
        "tool_call_id": &metadata.tool_call_id,
        "tool_name": &metadata.tool_name,
        "status": &metadata.status,
        "args_preview": &metadata.args_preview,
        "result_preview": &metadata.result_preview,
        "run_id": &metadata.run_id,
    })
}

pub(crate) fn build_graph_commands_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_command_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_command(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph command events found for '{id}' within depth {max_depth}"
        ));
    }

    let mut events = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut status_counts = BTreeMap::<String, usize>::new();
    let mut counted_status_events = BTreeSet::new();
    for step in &steps {
        if let Some(key) = graph_timeline_step_command_key(step, &metadata) {
            if let Some(command) = metadata.get(&key) {
                events.insert(command.event_id.clone());
                if command.event_type == "CommandCompleted"
                    && counted_status_events.insert(command.event_id.clone())
                {
                    *status_counts
                        .entry(
                            command
                                .status
                                .clone()
                                .unwrap_or_else(|| "unknown".to_string()),
                        )
                        .or_default() += 1;
                }
            }
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let events = events.into_iter().collect::<Vec<_>>();

    let mut out = String::new();
    out.push_str(&format!(
        "State graph commands: {id} depth={max_depth} limit={limit}\n"
    ));
    out.push_str(&format!("  events:     {}\n", compact_id_list(&events, 8)));
    out.push_str(&format!(
        "  statuses:   {}\n",
        format_count_map(&status_counts)
    ));
    out.push_str(&format!(
        "  by relation: {}\n",
        format_count_map(&relation_counts)
    ));
    for step in steps {
        let detail = graph_timeline_step_command_key(&step, &metadata)
            .as_deref()
            .and_then(|key| metadata.get(key))
            .map(format_graph_command_metadata)
            .unwrap_or_else(|| "command=- status=- result=- run=-".to_string());
        out.push_str(&format!(
            "  t={} d{} {} {} -[{}]-> {} ({}) {} via {}\n",
            step.timestamp_ms,
            step.depth,
            step.event_type,
            step.src_id,
            step.relation,
            step.dst_id,
            step.dst_kind,
            detail,
            step.event_id
        ));
    }
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_commands_payload(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<Value, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_command_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_command(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph command events found for '{id}' within depth {max_depth}"
        ));
    }

    let mut events = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut status_counts = BTreeMap::<String, usize>::new();
    let mut counted_status_events = BTreeSet::new();
    for step in &steps {
        if let Some(command) = graph_command_for_timeline_step(step, &metadata) {
            events.insert(command.event_id.clone());
            if command.event_type == "CommandCompleted"
                && counted_status_events.insert(command.event_id.clone())
            {
                *status_counts
                    .entry(
                        command
                            .status
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string()),
                    )
                    .or_default() += 1;
            }
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let command_events = events.into_iter().collect::<Vec<_>>();
    let command_metadata = command_metadata_payload_map(&steps, &metadata);
    let command_relations = graph_command_relations_payload(&steps, &metadata);

    Ok(serde_json::json!({
        "diagnostic": "state_graph_commands",
        "id": id,
        "depth": max_depth,
        "limit": limit,
        "command_event_count": command_events.len(),
        "relation_count": steps.len(),
        "command_events": command_events,
        "statuses": status_counts,
        "relations": relation_counts,
        "command_metadata": command_metadata,
        "command_relations": command_relations,
    }))
}

fn graph_command_for_timeline_step<'a>(
    step: &GraphTimelineStep,
    metadata: &'a BTreeMap<String, GraphCommandMetadata>,
) -> Option<&'a GraphCommandMetadata> {
    graph_timeline_step_command_key(step, metadata).and_then(|key| metadata.get(&key))
}

fn command_metadata_payload_map(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphCommandMetadata>,
) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    for step in steps {
        if let Some(command) = graph_command_for_timeline_step(step, metadata) {
            out.entry(command.event_id.clone())
                .or_insert_with(|| graph_command_metadata_payload(command));
        }
    }
    out
}

fn graph_command_relations_payload(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphCommandMetadata>,
) -> Vec<Value> {
    steps
        .iter()
        .map(|step| {
            let command_event_id = graph_command_for_timeline_step(step, metadata)
                .map(|command| command.event_id.clone());
            serde_json::json!({
                "timestamp_ms": step.timestamp_ms,
                "depth": step.depth,
                "event_id": &step.event_id,
                "event_type": &step.event_type,
                "src_id": &step.src_id,
                "relation": &step.relation,
                "dst_id": &step.dst_id,
                "dst_kind": &step.dst_kind,
                "command_event_id": command_event_id,
            })
        })
        .collect()
}

fn graph_command_metadata_payload(metadata: &GraphCommandMetadata) -> Value {
    serde_json::json!({
        "event_id": &metadata.event_id,
        "event_type": &metadata.event_type,
        "command": &metadata.command,
        "status": &metadata.status,
        "result_preview": &metadata.result_preview,
        "run_id": &metadata.run_id,
    })
}

pub(crate) fn build_graph_tests_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_test_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_test(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph test events found for '{id}' within depth {max_depth}"
        ));
    }

    let mut events = BTreeSet::new();
    let mut test_kinds = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut status_counts = BTreeMap::<String, usize>::new();
    let mut counted_status_events = BTreeSet::new();
    for step in &steps {
        if let Some(key) = graph_timeline_step_test_key(step, &metadata) {
            if let Some(test) = metadata.get(&key) {
                events.insert(test.event_id.clone());
                if let Some(test_kind) = test.test_kind.as_deref() {
                    test_kinds.insert(test_kind.to_string());
                }
                if test.event_type == "TestCompleted"
                    && counted_status_events.insert(test.event_id.clone())
                {
                    *status_counts
                        .entry(test.status.clone().unwrap_or_else(|| "unknown".to_string()))
                        .or_default() += 1;
                }
            }
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let events = events.into_iter().collect::<Vec<_>>();
    let test_kinds = test_kinds.into_iter().collect::<Vec<_>>();

    let mut out = String::new();
    out.push_str(&format!(
        "State graph tests: {id} depth={max_depth} limit={limit}\n"
    ));
    out.push_str(&format!("  events:     {}\n", compact_id_list(&events, 8)));
    out.push_str(&format!(
        "  kinds:      {}\n",
        compact_id_list(&test_kinds, 8)
    ));
    out.push_str(&format!(
        "  statuses:   {}\n",
        format_count_map(&status_counts)
    ));
    out.push_str(&format!(
        "  by relation: {}\n",
        format_count_map(&relation_counts)
    ));
    for step in steps {
        let detail = graph_timeline_step_test_key(&step, &metadata)
            .as_deref()
            .and_then(|key| metadata.get(key))
            .map(format_graph_test_metadata)
            .unwrap_or_else(|| "test=- command=- status=- result=- run=-".to_string());
        out.push_str(&format!(
            "  t={} d{} {} {} -[{}]-> {} ({}) {} via {}\n",
            step.timestamp_ms,
            step.depth,
            step.event_type,
            step.src_id,
            step.relation,
            step.dst_id,
            step.dst_kind,
            detail,
            step.event_id
        ));
    }
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_tests_payload(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<Value, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_test_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_test(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph test events found for '{id}' within depth {max_depth}"
        ));
    }

    let mut events = BTreeSet::new();
    let mut test_kinds = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut status_counts = BTreeMap::<String, usize>::new();
    let mut counted_status_events = BTreeSet::new();
    for step in &steps {
        if let Some(test) = graph_test_for_timeline_step(step, &metadata) {
            events.insert(test.event_id.clone());
            if let Some(test_kind) = test.test_kind.as_deref() {
                test_kinds.insert(test_kind.to_string());
            }
            if test.event_type == "TestCompleted"
                && counted_status_events.insert(test.event_id.clone())
            {
                *status_counts
                    .entry(test.status.clone().unwrap_or_else(|| "unknown".to_string()))
                    .or_default() += 1;
            }
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let test_events = events.into_iter().collect::<Vec<_>>();
    let test_kinds = test_kinds.into_iter().collect::<Vec<_>>();
    let test_metadata = test_metadata_payload_map(&steps, &metadata);
    let test_relations = graph_test_relations_payload(&steps, &metadata);

    Ok(serde_json::json!({
        "diagnostic": "state_graph_tests",
        "id": id,
        "depth": max_depth,
        "limit": limit,
        "test_event_count": test_events.len(),
        "test_kind_count": test_kinds.len(),
        "relation_count": steps.len(),
        "test_events": test_events,
        "test_kinds": test_kinds,
        "statuses": status_counts,
        "relations": relation_counts,
        "test_metadata": test_metadata,
        "test_relations": test_relations,
    }))
}

fn graph_test_for_timeline_step<'a>(
    step: &GraphTimelineStep,
    metadata: &'a BTreeMap<String, GraphTestMetadata>,
) -> Option<&'a GraphTestMetadata> {
    graph_timeline_step_test_key(step, metadata).and_then(|key| metadata.get(&key))
}

fn test_metadata_payload_map(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphTestMetadata>,
) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    for step in steps {
        if let Some(test) = graph_test_for_timeline_step(step, metadata) {
            out.entry(test.event_id.clone())
                .or_insert_with(|| graph_test_metadata_payload(test));
        }
    }
    out
}

fn graph_test_relations_payload(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphTestMetadata>,
) -> Vec<Value> {
    steps
        .iter()
        .map(|step| {
            let test_event_id =
                graph_test_for_timeline_step(step, metadata).map(|test| test.event_id.clone());
            serde_json::json!({
                "timestamp_ms": step.timestamp_ms,
                "depth": step.depth,
                "event_id": &step.event_id,
                "event_type": &step.event_type,
                "src_id": &step.src_id,
                "relation": &step.relation,
                "dst_id": &step.dst_id,
                "dst_kind": &step.dst_kind,
                "test_event_id": test_event_id,
            })
        })
        .collect()
}

fn graph_test_metadata_payload(metadata: &GraphTestMetadata) -> Value {
    serde_json::json!({
        "event_id": &metadata.event_id,
        "event_type": &metadata.event_type,
        "test_kind": &metadata.test_kind,
        "command": &metadata.command,
        "status": &metadata.status,
        "result_preview": &metadata.result_preview,
        "run_id": &metadata.run_id,
    })
}

pub(crate) fn build_graph_commits_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_commit_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_commit(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph commit relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut events = BTreeSet::new();
    let mut commits = BTreeSet::new();
    let mut reverted = BTreeSet::new();
    let mut branches = BTreeSet::new();
    let mut files = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut counted_events = BTreeSet::new();
    for step in &steps {
        if let Some(key) = graph_timeline_step_commit_key(step, &metadata) {
            if let Some(commit) = metadata.get(&key) {
                if counted_events.insert(commit.event_id.clone()) {
                    events.insert(commit.event_id.clone());
                    if let Some(commit_id) = commit.commit.as_deref() {
                        commits.insert(commit_id.to_string());
                    }
                    if let Some(reverted_commit) = commit.reverted_commit.as_deref() {
                        reverted.insert(reverted_commit.to_string());
                    }
                    if let Some(branch) = commit.branch.as_deref() {
                        branches.insert(branch.to_string());
                    }
                    files.extend(commit.files.iter().cloned());
                }
            }
        } else if step.dst_kind == "commit" {
            commits.insert(step.dst_id.clone());
        } else if step.dst_kind == "branch" {
            branches.insert(step.dst_id.clone());
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let events = events.into_iter().collect::<Vec<_>>();
    let commits = commits.into_iter().collect::<Vec<_>>();
    let reverted = reverted.into_iter().collect::<Vec<_>>();
    let branches = branches.into_iter().collect::<Vec<_>>();
    let files = files.into_iter().collect::<Vec<_>>();

    let mut out = String::new();
    out.push_str(&format!(
        "State graph commits: {id} depth={max_depth} limit={limit}\n"
    ));
    out.push_str(&format!("  events:     {}\n", compact_id_list(&events, 8)));
    out.push_str(&format!("  commits:    {}\n", compact_id_list(&commits, 8)));
    out.push_str(&format!(
        "  reverted:   {}\n",
        compact_id_list(&reverted, 8)
    ));
    out.push_str(&format!(
        "  branches:   {}\n",
        compact_id_list(&branches, 8)
    ));
    out.push_str(&format!("  files:      {}\n", compact_id_list(&files, 8)));
    out.push_str(&format!(
        "  by relation: {}\n",
        format_count_map(&relation_counts)
    ));
    for step in steps {
        let detail = graph_timeline_step_commit_key(&step, &metadata)
            .as_deref()
            .and_then(|key| metadata.get(key))
            .map(format_graph_commit_metadata)
            .unwrap_or_else(|| {
                "commit=- reverted=- branch=- files=[] message=- reason=- run=-".to_string()
            });
        out.push_str(&format!(
            "  t={} d{} {} {} -[{}]-> {} ({}) {} via {}\n",
            step.timestamp_ms,
            step.depth,
            step.event_type,
            step.src_id,
            step.relation,
            step.dst_id,
            step.dst_kind,
            detail,
            step.event_id
        ));
    }
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_commits_payload(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<Value, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_commit_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_commit(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph commit relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut events = BTreeSet::new();
    let mut commits = BTreeSet::new();
    let mut reverted = BTreeSet::new();
    let mut branches = BTreeSet::new();
    let mut files = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut counted_events = BTreeSet::new();
    for step in &steps {
        if let Some(commit) = graph_commit_for_timeline_step(step, &metadata) {
            if counted_events.insert(commit.event_id.clone()) {
                events.insert(commit.event_id.clone());
                if let Some(commit_id) = commit.commit.as_deref() {
                    commits.insert(commit_id.to_string());
                }
                if let Some(reverted_commit) = commit.reverted_commit.as_deref() {
                    reverted.insert(reverted_commit.to_string());
                }
                if let Some(branch) = commit.branch.as_deref() {
                    branches.insert(branch.to_string());
                }
                files.extend(commit.files.iter().cloned());
            }
        } else if step.dst_kind == "commit" {
            commits.insert(step.dst_id.clone());
        } else if step.dst_kind == "branch" {
            branches.insert(step.dst_id.clone());
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let commit_events = events.into_iter().collect::<Vec<_>>();
    let commits = commits.into_iter().collect::<Vec<_>>();
    let reverted_commits = reverted.into_iter().collect::<Vec<_>>();
    let branches = branches.into_iter().collect::<Vec<_>>();
    let files = files.into_iter().collect::<Vec<_>>();
    let commit_metadata = commit_metadata_payload_map(&steps, &metadata);
    let commit_relations = graph_commit_relations_payload(&steps, &metadata);

    Ok(serde_json::json!({
        "diagnostic": "state_graph_commits",
        "id": id,
        "depth": max_depth,
        "limit": limit,
        "commit_event_count": commit_events.len(),
        "commit_count": commits.len(),
        "reverted_commit_count": reverted_commits.len(),
        "branch_count": branches.len(),
        "file_count": files.len(),
        "relation_count": steps.len(),
        "commit_events": commit_events,
        "commits": commits,
        "reverted_commits": reverted_commits,
        "branches": branches,
        "files": files,
        "relations": relation_counts,
        "commit_metadata": commit_metadata,
        "commit_relations": commit_relations,
    }))
}

fn graph_commit_for_timeline_step<'a>(
    step: &GraphTimelineStep,
    metadata: &'a BTreeMap<String, GraphCommitMetadata>,
) -> Option<&'a GraphCommitMetadata> {
    graph_timeline_step_commit_key(step, metadata).and_then(|key| metadata.get(&key))
}

fn commit_metadata_payload_map(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphCommitMetadata>,
) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    for step in steps {
        if let Some(commit) = graph_commit_for_timeline_step(step, metadata) {
            out.entry(commit.event_id.clone())
                .or_insert_with(|| graph_commit_metadata_payload(commit));
        }
    }
    out
}

fn graph_commit_relations_payload(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphCommitMetadata>,
) -> Vec<Value> {
    steps
        .iter()
        .map(|step| {
            let commit_event_id = graph_commit_for_timeline_step(step, metadata)
                .map(|commit| commit.event_id.clone());
            serde_json::json!({
                "timestamp_ms": step.timestamp_ms,
                "depth": step.depth,
                "event_id": &step.event_id,
                "event_type": &step.event_type,
                "src_id": &step.src_id,
                "relation": &step.relation,
                "dst_id": &step.dst_id,
                "dst_kind": &step.dst_kind,
                "commit_event_id": commit_event_id,
            })
        })
        .collect()
}

fn graph_commit_metadata_payload(metadata: &GraphCommitMetadata) -> Value {
    serde_json::json!({
        "event_id": &metadata.event_id,
        "event_type": &metadata.event_type,
        "commit": &metadata.commit,
        "reverted_commit": &metadata.reverted_commit,
        "branch": &metadata.branch,
        "message": &metadata.message,
        "reason": &metadata.reason,
        "files": &metadata.files,
        "run_id": &metadata.run_id,
    })
}

pub(crate) fn build_graph_memories_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_memory_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_memory(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph memory relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut candidates = BTreeSet::new();
    let mut events = BTreeSet::new();
    let mut evidence = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut status_counts = BTreeMap::<String, usize>::new();
    let mut source_counts = BTreeMap::<String, usize>::new();
    let mut counted_events = BTreeSet::new();
    for step in &steps {
        if let Some(key) = graph_timeline_step_memory_key(step, &metadata) {
            if let Some(memory) = metadata.get(&key) {
                candidates.insert(memory.candidate_id.clone());
                if counted_events.insert(memory.event_id.clone()) {
                    events.insert(memory.event_id.clone());
                    *status_counts
                        .entry(
                            memory
                                .status
                                .clone()
                                .unwrap_or_else(|| "unknown".to_string()),
                        )
                        .or_default() += 1;
                    if let Some(source) = memory.source.as_deref() {
                        *source_counts.entry(source.to_string()).or_default() += 1;
                    }
                    evidence.extend(memory.evidence_event_ids.iter().cloned());
                }
            }
        } else if step.dst_kind == "memory" {
            candidates.insert(step.dst_id.clone());
        }
        if graph_evidence_relation(&step.relation) || step.relation == "derived_from" {
            if infer_graph_node_kind(&step.dst_id) == "event" {
                evidence.insert(step.dst_id.clone());
            }
            if infer_graph_node_kind(&step.src_id) == "event" {
                evidence.insert(step.src_id.clone());
            }
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let candidates = candidates.into_iter().collect::<Vec<_>>();
    let events = events.into_iter().collect::<Vec<_>>();
    let evidence = evidence.into_iter().collect::<Vec<_>>();

    let mut out = String::new();
    out.push_str(&format!(
        "State graph memories: {id} depth={max_depth} limit={limit}\n"
    ));
    out.push_str(&format!(
        "  candidates: {}\n",
        compact_id_list(&candidates, 8)
    ));
    out.push_str(&format!("  events:     {}\n", compact_id_list(&events, 8)));
    out.push_str(&format!(
        "  statuses:   {}\n",
        format_count_map(&status_counts)
    ));
    out.push_str(&format!(
        "  sources:    {}\n",
        format_count_map(&source_counts)
    ));
    out.push_str(&format!(
        "  evidence:   {}\n",
        compact_id_list(&evidence, 8)
    ));
    out.push_str(&format!(
        "  by relation: {}\n",
        format_count_map(&relation_counts)
    ));
    for step in steps {
        let detail = graph_timeline_step_memory_key(&step, &metadata)
            .as_deref()
            .and_then(|key| metadata.get(key))
            .map(format_graph_memory_metadata)
            .unwrap_or_else(|| {
                "candidate=- status=- source=- summary=- reason=- proposed=- evidence=[] run=-"
                    .to_string()
            });
        out.push_str(&format!(
            "  t={} d{} {} {} -[{}]-> {} ({}) {} via {}\n",
            step.timestamp_ms,
            step.depth,
            step.event_type,
            step.src_id,
            step.relation,
            step.dst_id,
            step.dst_kind,
            detail,
            step.event_id
        ));
    }
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_memories_payload(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<Value, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_memory_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_memory(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph memory relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut candidates = BTreeSet::new();
    let mut events = BTreeSet::new();
    let mut evidence = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut status_counts = BTreeMap::<String, usize>::new();
    let mut source_counts = BTreeMap::<String, usize>::new();
    let mut counted_events = BTreeSet::new();
    for step in &steps {
        if let Some(memory) = graph_memory_for_timeline_step(step, &metadata) {
            candidates.insert(memory.candidate_id.clone());
            if counted_events.insert(memory.event_id.clone()) {
                events.insert(memory.event_id.clone());
                *status_counts
                    .entry(
                        memory
                            .status
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string()),
                    )
                    .or_default() += 1;
                if let Some(source) = memory.source.as_deref() {
                    *source_counts.entry(source.to_string()).or_default() += 1;
                }
                evidence.extend(memory.evidence_event_ids.iter().cloned());
            }
        } else if step.dst_kind == "memory" {
            candidates.insert(step.dst_id.clone());
        }
        if graph_evidence_relation(&step.relation) || step.relation == "derived_from" {
            if infer_graph_node_kind(&step.dst_id) == "event" {
                evidence.insert(step.dst_id.clone());
            }
            if infer_graph_node_kind(&step.src_id) == "event" {
                evidence.insert(step.src_id.clone());
            }
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let memory_candidates = candidates.into_iter().collect::<Vec<_>>();
    let memory_events = events.into_iter().collect::<Vec<_>>();
    let evidence_events = evidence.into_iter().collect::<Vec<_>>();
    let memory_metadata = memory_metadata_payload_map(&steps, &metadata);
    let memory_relations = graph_memory_relations_payload(&steps, &metadata);

    Ok(serde_json::json!({
        "diagnostic": "state_graph_memories",
        "id": id,
        "depth": max_depth,
        "limit": limit,
        "memory_candidate_count": memory_candidates.len(),
        "memory_event_count": memory_events.len(),
        "evidence_event_count": evidence_events.len(),
        "relation_count": steps.len(),
        "memory_candidates": memory_candidates,
        "memory_events": memory_events,
        "evidence_events": evidence_events,
        "statuses": status_counts,
        "sources": source_counts,
        "relations": relation_counts,
        "memory_metadata": memory_metadata,
        "memory_relations": memory_relations,
    }))
}

fn graph_memory_for_timeline_step<'a>(
    step: &GraphTimelineStep,
    metadata: &'a BTreeMap<String, GraphMemoryMetadata>,
) -> Option<&'a GraphMemoryMetadata> {
    graph_timeline_step_memory_key(step, metadata).and_then(|key| metadata.get(&key))
}

fn memory_metadata_payload_map(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphMemoryMetadata>,
) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    for step in steps {
        if let Some(memory) = graph_memory_for_timeline_step(step, metadata) {
            out.entry(memory.event_id.clone())
                .or_insert_with(|| graph_memory_metadata_payload(memory));
        }
    }
    out
}

fn graph_memory_relations_payload(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphMemoryMetadata>,
) -> Vec<Value> {
    steps
        .iter()
        .map(|step| {
            let memory_event_id = graph_memory_for_timeline_step(step, metadata)
                .map(|memory| memory.event_id.clone());
            serde_json::json!({
                "timestamp_ms": step.timestamp_ms,
                "depth": step.depth,
                "event_id": &step.event_id,
                "event_type": &step.event_type,
                "src_id": &step.src_id,
                "relation": &step.relation,
                "dst_id": &step.dst_id,
                "dst_kind": &step.dst_kind,
                "memory_event_id": memory_event_id,
            })
        })
        .collect()
}

fn graph_memory_metadata_payload(metadata: &GraphMemoryMetadata) -> Value {
    serde_json::json!({
        "event_id": &metadata.event_id,
        "event_type": &metadata.event_type,
        "candidate_id": &metadata.candidate_id,
        "status": &metadata.status,
        "source": &metadata.source,
        "summary": &metadata.summary,
        "reason": &metadata.reason,
        "proposed_event_id": &metadata.proposed_event_id,
        "evidence_event_ids": &metadata.evidence_event_ids,
        "run_id": &metadata.run_id,
    })
}

pub(crate) fn build_graph_issues_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_issue_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_issue(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph issue relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut issues = BTreeSet::new();
    let mut patches = BTreeSet::new();
    let mut events = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut source_counts = BTreeMap::<String, usize>::new();
    let mut kind_counts = BTreeMap::<String, usize>::new();
    let mut counted_events = BTreeSet::new();
    for step in &steps {
        if let Some(key) = graph_timeline_step_issue_key(step, &metadata) {
            if let Some(issue) = metadata.get(&key) {
                issues.insert(issue.issue_id.clone());
                if let Some(patch_id) = issue.patch_id.as_deref() {
                    patches.insert(patch_id.to_string());
                }
                if counted_events.insert(issue.event_id.clone()) {
                    events.insert(issue.event_id.clone());
                    if let Some(source) = issue.intake_source.as_deref() {
                        *source_counts.entry(source.to_string()).or_default() += 1;
                    }
                    if let Some(kind) = issue.kind.as_deref() {
                        *kind_counts.entry(kind.to_string()).or_default() += 1;
                    }
                }
            }
        } else if step.dst_kind == "issue" {
            issues.insert(step.dst_id.clone());
        } else if step.dst_kind == "patch" || infer_graph_node_kind(&step.src_id) == "patch" {
            if let Some(patch_id) = graph_timeline_step_patch_key(step) {
                patches.insert(patch_id);
            }
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let issues = issues.into_iter().collect::<Vec<_>>();
    let patches = patches.into_iter().collect::<Vec<_>>();
    let events = events.into_iter().collect::<Vec<_>>();

    let mut out = String::new();
    out.push_str(&format!(
        "State graph issues: {id} depth={max_depth} limit={limit}\n"
    ));
    out.push_str(&format!("  issues:     {}\n", compact_id_list(&issues, 8)));
    out.push_str(&format!("  patches:    {}\n", compact_id_list(&patches, 8)));
    out.push_str(&format!("  events:     {}\n", compact_id_list(&events, 8)));
    out.push_str(&format!(
        "  sources:    {}\n",
        format_count_map(&source_counts)
    ));
    out.push_str(&format!(
        "  kinds:      {}\n",
        format_count_map(&kind_counts)
    ));
    out.push_str(&format!(
        "  by relation: {}\n",
        format_count_map(&relation_counts)
    ));
    for step in steps {
        let detail = graph_timeline_step_issue_key(&step, &metadata)
            .as_deref()
            .and_then(|key| metadata.get(key))
            .map(format_graph_issue_metadata)
            .unwrap_or_else(|| {
                "issue=- patch=- source=- kind=- patch_kind=- risk=- status=- summary=- details=- run=-"
                    .to_string()
            });
        out.push_str(&format!(
            "  t={} d{} {} {} -[{}]-> {} ({}) {} via {}\n",
            step.timestamp_ms,
            step.depth,
            step.event_type,
            step.src_id,
            step.relation,
            step.dst_id,
            step.dst_kind,
            detail,
            step.event_id
        ));
    }
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_issues_payload(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<Value, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_issue_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_issue(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph issue relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut issues = BTreeSet::new();
    let mut patches = BTreeSet::new();
    let mut events = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut source_counts = BTreeMap::<String, usize>::new();
    let mut kind_counts = BTreeMap::<String, usize>::new();
    let mut counted_events = BTreeSet::new();
    for step in &steps {
        if let Some(issue) = graph_issue_for_timeline_step(step, &metadata) {
            issues.insert(issue.issue_id.clone());
            if let Some(patch_id) = issue.patch_id.as_deref() {
                patches.insert(patch_id.to_string());
            }
            if counted_events.insert(issue.event_id.clone()) {
                events.insert(issue.event_id.clone());
                if let Some(source) = issue.intake_source.as_deref() {
                    *source_counts.entry(source.to_string()).or_default() += 1;
                }
                if let Some(kind) = issue.kind.as_deref() {
                    *kind_counts.entry(kind.to_string()).or_default() += 1;
                }
            }
        } else if step.dst_kind == "issue" {
            issues.insert(step.dst_id.clone());
        } else if step.dst_kind == "patch" || infer_graph_node_kind(&step.src_id) == "patch" {
            if let Some(patch_id) = graph_timeline_step_patch_key(step) {
                patches.insert(patch_id);
            }
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let issue_ids = issues.into_iter().collect::<Vec<_>>();
    let patch_ids = patches.into_iter().collect::<Vec<_>>();
    let issue_events = events.into_iter().collect::<Vec<_>>();
    let issue_metadata = issue_metadata_payload_map(&steps, &metadata);
    let issue_relations = graph_issue_relations_payload(&steps, &metadata);

    Ok(serde_json::json!({
        "diagnostic": "state_graph_issues",
        "id": id,
        "depth": max_depth,
        "limit": limit,
        "issue_count": issue_ids.len(),
        "patch_count": patch_ids.len(),
        "issue_event_count": issue_events.len(),
        "relation_count": steps.len(),
        "issues": issue_ids,
        "patches": patch_ids,
        "issue_events": issue_events,
        "sources": source_counts,
        "kinds": kind_counts,
        "relations": relation_counts,
        "issue_metadata": issue_metadata,
        "issue_relations": issue_relations,
    }))
}

fn graph_issue_for_timeline_step<'a>(
    step: &GraphTimelineStep,
    metadata: &'a BTreeMap<String, GraphIssueMetadata>,
) -> Option<&'a GraphIssueMetadata> {
    graph_timeline_step_issue_key(step, metadata).and_then(|key| metadata.get(&key))
}

fn issue_metadata_payload_map(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphIssueMetadata>,
) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    for step in steps {
        if let Some(issue) = graph_issue_for_timeline_step(step, metadata) {
            out.entry(issue.event_id.clone())
                .or_insert_with(|| graph_issue_metadata_payload(issue));
        }
    }
    out
}

fn graph_issue_relations_payload(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphIssueMetadata>,
) -> Vec<Value> {
    steps
        .iter()
        .map(|step| {
            let issue_event_id =
                graph_issue_for_timeline_step(step, metadata).map(|issue| issue.event_id.clone());
            serde_json::json!({
                "timestamp_ms": step.timestamp_ms,
                "depth": step.depth,
                "event_id": &step.event_id,
                "event_type": &step.event_type,
                "src_id": &step.src_id,
                "relation": &step.relation,
                "dst_id": &step.dst_id,
                "dst_kind": &step.dst_kind,
                "issue_event_id": issue_event_id,
            })
        })
        .collect()
}

fn graph_issue_metadata_payload(metadata: &GraphIssueMetadata) -> Value {
    serde_json::json!({
        "event_id": &metadata.event_id,
        "issue_id": &metadata.issue_id,
        "patch_id": &metadata.patch_id,
        "intake_source": &metadata.intake_source,
        "intake_kind": &metadata.intake_kind,
        "summary": &metadata.summary,
        "details": &metadata.details,
        "kind": &metadata.kind,
        "risk_level": &metadata.risk_level,
        "status": &metadata.status,
        "run_id": &metadata.run_id,
    })
}

pub(crate) fn build_graph_cache_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_cache_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_cache(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph cache metrics found for '{id}' within depth {max_depth}"
        ));
    }

    let mut events = BTreeSet::new();
    let mut models = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut total_hit = 0i64;
    let mut total_miss = 0i64;
    for step in &steps {
        if let Some(cache_id) = graph_timeline_step_cache_key(step, &metadata) {
            if events.insert(cache_id.clone()) {
                if let Some(cache) = metadata.get(&cache_id) {
                    if let Some(model) = cache.model.as_deref() {
                        models.insert(model.to_string());
                    }
                    total_hit += cache.prompt_cache_hit_tokens.unwrap_or(0);
                    total_miss += cache.prompt_cache_miss_tokens.unwrap_or(0);
                }
            }
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let events = events.into_iter().collect::<Vec<_>>();
    let models = models.into_iter().collect::<Vec<_>>();
    let total = total_hit + total_miss;
    let ratio = if total > 0 {
        total_hit as f64 / total as f64
    } else {
        0.0
    };

    let mut out = String::new();
    out.push_str(&format!(
        "State graph cache: {id} depth={max_depth} limit={limit}\n"
    ));
    out.push_str(&format!("  events:     {}\n", compact_id_list(&events, 8)));
    out.push_str(&format!("  models:     {}\n", compact_id_list(&models, 8)));
    out.push_str(&format!(
        "  totals:     hit={} miss={} ratio={ratio:.3}\n",
        total_hit, total_miss
    ));
    out.push_str(&format!(
        "  by relation: {}\n",
        format_count_map(&relation_counts)
    ));
    for step in steps {
        let detail = graph_timeline_step_cache_key(&step, &metadata)
            .as_deref()
            .and_then(|cache_id| metadata.get(cache_id))
            .map(format_graph_cache_metadata)
            .unwrap_or_else(|| "model=- hit=- miss=- ratio=- cache_t=-".to_string());
        out.push_str(&format!(
            "  t={} d{} {} {} -[{}]-> {} ({}) {} via {}\n",
            step.timestamp_ms,
            step.depth,
            step.event_type,
            step.src_id,
            step.relation,
            step.dst_id,
            step.dst_kind,
            detail,
            step.event_id
        ));
    }
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_cache_payload(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<Value, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_cache_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_cache(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph cache metrics found for '{id}' within depth {max_depth}"
        ));
    }

    let mut events = BTreeSet::new();
    let mut models = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut total_hit = 0i64;
    let mut total_miss = 0i64;
    for step in &steps {
        if let Some(cache) = graph_cache_for_timeline_step(step, &metadata) {
            if events.insert(cache.event_id.clone()) {
                if let Some(model) = cache.model.as_deref() {
                    models.insert(model.to_string());
                }
                total_hit += cache.prompt_cache_hit_tokens.unwrap_or(0);
                total_miss += cache.prompt_cache_miss_tokens.unwrap_or(0);
            }
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let cache_events = events.into_iter().collect::<Vec<_>>();
    let models = models.into_iter().collect::<Vec<_>>();
    let total = total_hit + total_miss;
    let ratio = if total > 0 {
        total_hit as f64 / total as f64
    } else {
        0.0
    };

    Ok(serde_json::json!({
        "diagnostic": "state_graph_cache",
        "id": id,
        "depth": max_depth,
        "limit": limit,
        "cache_event_count": cache_events.len(),
        "model_count": models.len(),
        "relation_count": steps.len(),
        "cache_events": cache_events,
        "models": models,
        "totals": {
            "hit": total_hit,
            "miss": total_miss,
            "ratio": ratio,
        },
        "relations": relation_counts,
        "cache_metadata": graph_cache_metadata_payload_map(&steps, &metadata),
        "cache_relations": graph_cache_relations_payload(&steps, &metadata),
    }))
}

pub(crate) fn build_graph_failures_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_failure_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_failure(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph failure relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut failures = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut class_counts = BTreeMap::<String, usize>::new();
    let mut retryable = 0usize;
    for step in &steps {
        if let Some(failure_id) = graph_timeline_step_failure_key(step, &metadata) {
            if failures.insert(failure_id.clone()) {
                if let Some(failure) = metadata.get(&failure_id) {
                    *class_counts.entry(failure.class.clone()).or_default() += 1;
                    if failure.retryable {
                        retryable += 1;
                    }
                }
            }
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let failures = failures.into_iter().collect::<Vec<_>>();

    let mut out = String::new();
    out.push_str(&format!(
        "State graph failures: {id} depth={max_depth} limit={limit}\n"
    ));
    out.push_str(&format!(
        "  failures:   {}\n",
        compact_id_list(&failures, 8)
    ));
    out.push_str(&format!("  retryable:  {retryable}\n"));
    out.push_str(&format!(
        "  classes:    {}\n",
        format_count_map(&class_counts)
    ));
    out.push_str(&format!(
        "  by relation: {}\n",
        format_count_map(&relation_counts)
    ));
    for step in steps {
        let detail = graph_timeline_step_failure_key(&step, &metadata)
            .as_deref()
            .and_then(|failure_id| metadata.get(failure_id))
            .map(format_graph_failure_metadata)
            .unwrap_or_else(|| {
                "class=- owner=- retryable=false source=- signal=- run=-".to_string()
            });
        out.push_str(&format!(
            "  t={} d{} {} {} -[{}]-> {} ({}) {} via {}\n",
            step.timestamp_ms,
            step.depth,
            step.event_type,
            step.src_id,
            step.relation,
            step.dst_id,
            step.dst_kind,
            detail,
            step.event_id
        ));
    }
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_failures_payload(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<Value, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let metadata = query_graph_failure_metadata(sqlite_path)?;
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(|step| graph_timeline_step_mentions_failure(step, &metadata))
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph failure relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut failures = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    let mut class_counts = BTreeMap::<String, usize>::new();
    let mut retryable = 0usize;
    for step in &steps {
        if let Some(failure) = graph_failure_for_timeline_step(step, &metadata) {
            if failures.insert(failure.event_id.clone()) {
                *class_counts.entry(failure.class.clone()).or_default() += 1;
                if failure.retryable {
                    retryable += 1;
                }
            }
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let failures = failures.into_iter().collect::<Vec<_>>();

    Ok(serde_json::json!({
        "diagnostic": "state_graph_failures",
        "id": id,
        "depth": max_depth,
        "limit": limit,
        "failure_count": failures.len(),
        "retryable_count": retryable,
        "relation_count": steps.len(),
        "failures": failures,
        "classes": class_counts,
        "relations": relation_counts,
        "failure_metadata": graph_failure_metadata_payload_map(&steps, &metadata),
        "failure_relations": graph_failure_relations_payload(&steps, &metadata),
    }))
}

pub(crate) fn build_graph_policies_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(graph_timeline_step_mentions_policy)
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph policy relations found for '{id}' within depth {max_depth}"
        ));
    }

    let metadata = query_graph_policy_metadata(sqlite_path)?;
    let mut policies = BTreeSet::new();
    let mut schemas = BTreeSet::new();
    let mut prompts = BTreeSet::new();
    let mut blocks = BTreeSet::new();
    let mut instruction_files = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    for step in &steps {
        if infer_graph_node_kind(&step.src_id) == "policy" {
            policies.insert(step.src_id.clone());
        }
        if step.relation == "supports_strict_tool_call_check" {
            schemas.insert(step.src_id.clone());
        }
        match step.dst_kind.as_str() {
            "context_policy" | "policy" => {
                policies.insert(step.dst_id.clone());
            }
            "tool_schema" | "tool_schema_version" => {
                schemas.insert(step.dst_id.clone());
            }
            "prompt_layout" | "prompt_version" => {
                prompts.insert(step.dst_id.clone());
            }
            "context_block" => {
                blocks.insert(step.dst_id.clone());
            }
            "instruction_file" => {
                instruction_files.insert(step.dst_id.clone());
            }
            _ => {}
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let policies = policies.into_iter().collect::<Vec<_>>();
    let schemas = schemas.into_iter().collect::<Vec<_>>();
    let prompts = prompts.into_iter().collect::<Vec<_>>();
    let blocks = blocks.into_iter().collect::<Vec<_>>();
    let instruction_files = instruction_files.into_iter().collect::<Vec<_>>();

    let mut out = String::new();
    out.push_str(&format!(
        "State graph policies: {id} depth={max_depth} limit={limit}\n"
    ));
    out.push_str(&format!(
        "  policies:   {}\n",
        compact_id_list(&policies, 8)
    ));
    out.push_str(&format!("  schemas:    {}\n", compact_id_list(&schemas, 8)));
    out.push_str(&format!("  prompts:    {}\n", compact_id_list(&prompts, 8)));
    out.push_str(&format!("  blocks:     {}\n", compact_id_list(&blocks, 8)));
    out.push_str(&format!(
        "  instruction files: {}\n",
        compact_id_list(&instruction_files, 8)
    ));
    out.push_str(&format!(
        "  by relation: {}\n",
        format_count_map(&relation_counts)
    ));
    for step in steps {
        let detail = metadata
            .get(&step.event_id)
            .map(format_graph_policy_metadata)
            .unwrap_or_else(|| {
                "policy=- layout=- prompt=- schema=- version=- source_audit=- source_scan=- source_findings=- blocks=0/0 instructions=[] included=[]".to_string()
            });
        out.push_str(&format!(
            "  t={} d{} {} {} -[{}]-> {} ({}) {} via {}\n",
            step.timestamp_ms,
            step.depth,
            step.event_type,
            step.src_id,
            step.relation,
            step.dst_id,
            step.dst_kind,
            detail,
            step.event_id
        ));
    }
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_policies_payload(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<Value, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(graph_timeline_step_mentions_policy)
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph policy relations found for '{id}' within depth {max_depth}"
        ));
    }

    let metadata = query_graph_policy_metadata(sqlite_path)?;
    let mut policies = BTreeSet::new();
    let mut schemas = BTreeSet::new();
    let mut prompts = BTreeSet::new();
    let mut blocks = BTreeSet::new();
    let mut instruction_files = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    for step in &steps {
        if infer_graph_node_kind(&step.src_id) == "policy" {
            policies.insert(step.src_id.clone());
        }
        if step.relation == "supports_strict_tool_call_check" {
            schemas.insert(step.src_id.clone());
        }
        match step.dst_kind.as_str() {
            "context_policy" | "policy" => {
                policies.insert(step.dst_id.clone());
            }
            "tool_schema" | "tool_schema_version" => {
                schemas.insert(step.dst_id.clone());
            }
            "prompt_layout" | "prompt_version" => {
                prompts.insert(step.dst_id.clone());
            }
            "context_block" => {
                blocks.insert(step.dst_id.clone());
            }
            "instruction_file" => {
                instruction_files.insert(step.dst_id.clone());
            }
            _ => {}
        }
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let policies = policies.into_iter().collect::<Vec<_>>();
    let schemas = schemas.into_iter().collect::<Vec<_>>();
    let prompts = prompts.into_iter().collect::<Vec<_>>();
    let blocks = blocks.into_iter().collect::<Vec<_>>();
    let instruction_files = instruction_files.into_iter().collect::<Vec<_>>();

    Ok(serde_json::json!({
        "diagnostic": "state_graph_policies",
        "id": id,
        "depth": max_depth,
        "limit": limit,
        "policy_count": policies.len(),
        "schema_count": schemas.len(),
        "prompt_count": prompts.len(),
        "block_count": blocks.len(),
        "instruction_file_count": instruction_files.len(),
        "relation_count": steps.len(),
        "policies": policies,
        "schemas": schemas,
        "prompts": prompts,
        "blocks": blocks,
        "instruction_files": instruction_files,
        "relations": relation_counts,
        "policy_metadata": graph_policy_metadata_payload_map(&steps, &metadata),
        "policy_relations": graph_policy_relations_payload(&steps, &metadata),
    }))
}

pub(crate) fn build_graph_protocol_report(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(graph_timeline_step_mentions_protocol)
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph protocol relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut checks = BTreeSet::new();
    let mut evals = BTreeSet::new();
    let mut decisions = BTreeSet::new();
    let mut events = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    for step in &steps {
        collect_protocol_check_id(&step.src_id, &mut checks);
        collect_protocol_check_id(&step.dst_id, &mut checks);
        let src_kind = infer_graph_node_kind(&step.src_id);
        if src_kind == "eval" {
            evals.insert(step.src_id.clone());
        }
        if step.dst_kind == "eval" {
            evals.insert(step.dst_id.clone());
        }
        if src_kind == "decision" {
            decisions.insert(step.src_id.clone());
        }
        if step.dst_kind == "decision" {
            decisions.insert(step.dst_id.clone());
        }
        if src_kind == "event" {
            events.insert(step.src_id.clone());
        }
        if step.dst_kind == "event" {
            events.insert(step.dst_id.clone());
        }
        events.insert(step.event_id.clone());
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let checks = checks.into_iter().collect::<Vec<_>>();
    let evals = evals.into_iter().collect::<Vec<_>>();
    let decisions = decisions.into_iter().collect::<Vec<_>>();
    let events = events.into_iter().collect::<Vec<_>>();

    let mut out = String::new();
    out.push_str(&format!(
        "State graph protocol: {id} depth={max_depth} limit={limit}\n"
    ));
    out.push_str(&format!("  checks:    {}\n", compact_id_list(&checks, 8)));
    out.push_str(&format!("  evals:     {}\n", compact_id_list(&evals, 8)));
    out.push_str(&format!(
        "  decisions: {}\n",
        compact_id_list(&decisions, 8)
    ));
    out.push_str(&format!("  events:    {}\n", compact_id_list(&events, 8)));
    out.push_str(&format!(
        "  by relation: {}\n",
        format_count_map(&relation_counts)
    ));
    for step in steps {
        out.push_str(&format!(
            "  t={} d{} {} {} -[{}]-> {} ({}) via {}\n",
            step.timestamp_ms,
            step.depth,
            step.event_type,
            step.src_id,
            step.relation,
            step.dst_id,
            step.dst_kind,
            step.event_id
        ));
    }
    Ok(out.trim_end().to_string())
}

pub(crate) fn build_graph_protocol_payload(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<Value, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let steps = query_graph_timeline(sqlite_path, id, max_depth, 200)?
        .into_iter()
        .filter(graph_timeline_step_mentions_protocol)
        .take(limit)
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err(format!(
            "no graph protocol relations found for '{id}' within depth {max_depth}"
        ));
    }

    let mut checks = BTreeSet::new();
    let mut evals = BTreeSet::new();
    let mut decisions = BTreeSet::new();
    let mut events = BTreeSet::new();
    let mut relation_counts = BTreeMap::<String, usize>::new();
    for step in &steps {
        collect_protocol_check_id(&step.src_id, &mut checks);
        collect_protocol_check_id(&step.dst_id, &mut checks);
        let src_kind = infer_graph_node_kind(&step.src_id);
        if src_kind == "eval" {
            evals.insert(step.src_id.clone());
        }
        if step.dst_kind == "eval" {
            evals.insert(step.dst_id.clone());
        }
        if src_kind == "decision" {
            decisions.insert(step.src_id.clone());
        }
        if step.dst_kind == "decision" {
            decisions.insert(step.dst_id.clone());
        }
        if src_kind == "event" {
            events.insert(step.src_id.clone());
        }
        if step.dst_kind == "event" {
            events.insert(step.dst_id.clone());
        }
        events.insert(step.event_id.clone());
        *relation_counts.entry(step.relation.clone()).or_default() += 1;
    }
    let checks = checks.into_iter().collect::<Vec<_>>();
    let evals = evals.into_iter().collect::<Vec<_>>();
    let decisions = decisions.into_iter().collect::<Vec<_>>();
    let events = events.into_iter().collect::<Vec<_>>();

    Ok(serde_json::json!({
        "diagnostic": "state_graph_protocol",
        "id": id,
        "depth": max_depth,
        "limit": limit,
        "check_count": checks.len(),
        "eval_count": evals.len(),
        "decision_count": decisions.len(),
        "event_count": events.len(),
        "relation_count": steps.len(),
        "checks": checks,
        "evals": evals,
        "decisions": decisions,
        "events": events,
        "relations": relation_counts,
        "protocol_relations": graph_protocol_relations_payload(&steps),
    }))
}

fn graph_timeline_step_mentions_protocol(step: &GraphTimelineStep) -> bool {
    step.src_id.starts_with("deepseek_protocol_check:")
        || step.dst_id.starts_with("deepseek_protocol_check:")
        || matches!(
            step.relation.as_str(),
            "covers_protocol_check"
                | "supports_strict_tool_call_check"
                | "supports_transport_policy_check"
                | "supports_thinking_protocol_check"
                | "supports_streaming_protocol_check"
                | "supports_json_output_check"
                | "uses_thinking_protocol_policy"
                | "uses_streaming_protocol_policy"
                | "uses_transport_policy"
        )
}

fn collect_protocol_check_id(id: &str, out: &mut BTreeSet<String>) {
    if id.starts_with("deepseek_protocol_check:") {
        out.insert(id.to_string());
    }
}

fn graph_protocol_relations_payload(steps: &[GraphTimelineStep]) -> Vec<Value> {
    steps
        .iter()
        .map(|step| {
            serde_json::json!({
                "event_id": &step.event_id,
                "event_type": &step.event_type,
                "depth": step.depth,
                "timestamp_ms": step.timestamp_ms,
                "src_id": &step.src_id,
                "relation": &step.relation,
                "dst_id": &step.dst_id,
                "dst_kind": &step.dst_kind,
            })
        })
        .collect()
}

fn graph_timeline_step_mentions_file(step: &GraphTimelineStep) -> bool {
    step.dst_kind == "file" || infer_graph_node_kind(&step.src_id) == "file"
}

fn graph_timeline_step_file_id(step: &GraphTimelineStep) -> Option<String> {
    if step.dst_kind == "file" {
        Some(step.dst_id.clone())
    } else if infer_graph_node_kind(&step.src_id) == "file" {
        Some(step.src_id.clone())
    } else {
        None
    }
}

fn graph_timeline_step_mentions_eval(step: &GraphTimelineStep) -> bool {
    step.dst_kind == "eval"
        || infer_graph_node_kind(&step.src_id) == "eval"
        || step.event_type == "PatchEvaluated"
}

fn graph_timeline_step_eval_key(step: &GraphTimelineStep) -> Option<String> {
    if step.dst_kind == "eval" {
        Some(step.dst_id.clone())
    } else if infer_graph_node_kind(&step.src_id) == "eval" {
        Some(step.src_id.clone())
    } else if step.event_type == "PatchEvaluated" {
        Some(step.event_id.clone())
    } else {
        None
    }
}

fn format_graph_eval_metadata(metadata: &GraphEvalMetadata) -> String {
    let score = metadata
        .score
        .map(|score| format!("{score:.3}"))
        .unwrap_or_else(|| "-".to_string());
    let fixture = match (metadata.fixture_task_count, metadata.fixture_command_count) {
        (Some(tasks), Some(commands)) => {
            format!(" fixture_tasks={tasks} fixture_commands={commands}")
        }
        (Some(tasks), None) => format!(" fixture_tasks={tasks}"),
        (None, Some(commands)) => format!(" fixture_commands={commands}"),
        (None, None) => String::new(),
    };
    let protocol =
        metadata
            .deepseek_protocol_checks
            .filter(|checks| *checks > 0)
            .map(|checks| {
                format!(
                " protocol_checks={} protocol_passes={} strict={} thinking={} stream={} json={} transport={}",
                checks,
                metadata.deepseek_protocol_passes.unwrap_or_default(),
                metadata.deepseek_strict_tool_call_checks.unwrap_or_default(),
                metadata.deepseek_thinking_protocol_checks.unwrap_or_default(),
                metadata.deepseek_streaming_protocol_checks.unwrap_or_default(),
                metadata.deepseek_json_output_checks.unwrap_or_default(),
                metadata.deepseek_transport_policy_checks.unwrap_or_default()
            )
            })
            .unwrap_or_default();
    let agent_changes = if metadata.fixture_agent_changed_files.is_empty() {
        String::new()
    } else {
        format!(
            " agent_changes={} [{}]",
            metadata.fixture_agent_changed_files.len(),
            compact_id_list(&metadata.fixture_agent_changed_files, 4)
        )
    };
    let unexpected_agent_changes = if metadata.fixture_agent_unexpected_files.is_empty() {
        String::new()
    } else {
        format!(
            " unexpected_agent_changes={} [{}]",
            metadata.fixture_agent_unexpected_files.len(),
            compact_id_list(&metadata.fixture_agent_unexpected_files, 4)
        )
    };
    let model_routes = if metadata.model_route_tasks.is_empty() {
        String::new()
    } else {
        format!(
            " model_routes=[{}]",
            format_u64_count_map(&metadata.model_route_tasks)
        )
    };
    format!(
        "suite={} status={} score={} patch={}{}{}{}{}{}",
        metadata.suite.as_deref().unwrap_or("-"),
        metadata.status.as_deref().unwrap_or("-"),
        score,
        metadata.patch_id.as_deref().unwrap_or("-"),
        fixture,
        model_routes,
        agent_changes,
        unexpected_agent_changes,
        protocol
    )
}

fn graph_timeline_step_mentions_patch(step: &GraphTimelineStep) -> bool {
    step.dst_kind == "patch"
        || infer_graph_node_kind(&step.src_id) == "patch"
        || matches!(
            step.event_type.as_str(),
            "PatchApplied" | "PatchEvaluated" | "PatchPromoted" | "PatchRejected"
        )
}

fn graph_timeline_step_patch_key(step: &GraphTimelineStep) -> Option<String> {
    if step.dst_kind == "patch" {
        Some(step.dst_id.clone())
    } else if infer_graph_node_kind(&step.src_id) == "patch" {
        Some(step.src_id.clone())
    } else if matches!(
        step.event_type.as_str(),
        "PatchApplied" | "PatchEvaluated" | "PatchPromoted" | "PatchRejected"
    ) {
        Some(step.event_id.clone())
    } else {
        None
    }
}

fn format_graph_patch_metadata(metadata: &GraphPatchMetadata) -> String {
    format!(
        "status={} kind={} risk={} harness={} state={} base={} rollback_steps={}",
        metadata.status.as_deref().unwrap_or("-"),
        metadata.kind.as_deref().unwrap_or("-"),
        metadata.risk_level.as_deref().unwrap_or("-"),
        metadata.base_harness_version.as_deref().unwrap_or("-"),
        metadata
            .state_version
            .map(|version| version.to_string())
            .unwrap_or_else(|| "-".to_string()),
        metadata.base_git_commit.as_deref().unwrap_or("-"),
        metadata.rollback_plan_steps
    )
}

fn graph_timeline_step_mentions_decision(step: &GraphTimelineStep) -> bool {
    step.dst_kind == "decision"
        || infer_graph_node_kind(&step.src_id) == "decision"
        || step.event_type == "DecisionRecorded"
}

fn graph_timeline_step_decision_key(step: &GraphTimelineStep) -> Option<String> {
    if step.dst_kind == "decision" {
        Some(step.dst_id.clone())
    } else if infer_graph_node_kind(&step.src_id) == "decision" {
        Some(step.src_id.clone())
    } else if step.event_type == "DecisionRecorded" {
        Some(step.event_id.clone())
    } else {
        None
    }
}

fn format_graph_decision_metadata(metadata: &GraphDecisionMetadata) -> String {
    let mut detail = format!(
        "type={} decision={} status={} patch={} eval={}",
        metadata.decision_type.as_deref().unwrap_or("-"),
        metadata.decision.as_deref().unwrap_or("-"),
        metadata.status.as_deref().unwrap_or("-"),
        metadata.patch_id.as_deref().unwrap_or("-"),
        metadata.eval_id.as_deref().unwrap_or("-")
    );
    if metadata.decision_type.as_deref() == Some("release_gate") {
        let source_audit = metadata
            .source_provenance_passed
            .map(|passed| if passed { "passed" } else { "blocked" })
            .unwrap_or("-");
        let require_protocol = metadata
            .require_protocol
            .map(|required| if required { "yes" } else { "no" })
            .unwrap_or("-");
        detail.push_str(&format!(
            " suite={} missing_gates={} replay_failures={} fixture_tasks={} fixture_commands={} fixture_risks=[{}] model_routes=[{}] mutation_scope_failures={} unexpected_files={} min_fixture_tasks={} min_fixture_commands={} min_fixture_risks=[{}] fixture_breadth_ok={} fixture_risk_ok={} protocol_required={} protocol_status={} protocol_dirty={} protocol_checks={}/{} protocol_strict={} protocol_thinking={} protocol_stream={} protocol_json={} protocol_transport={} source_audit={} source_findings={} source_scan={} reason={}",
            metadata.suite.as_deref().unwrap_or("-"),
            metadata.missing_required_gates,
            metadata.replay_failures_after_eval.unwrap_or(0),
            metadata
                .last_eval_fixture_task_count
                .map(|count| count.to_string())
                .unwrap_or_else(|| "-".to_string()),
            metadata
                .last_eval_fixture_command_count
                .map(|count| count.to_string())
                .unwrap_or_else(|| "-".to_string()),
            format_u64_count_map(&metadata.last_eval_fixture_risk_labels),
            format_u64_count_map(&metadata.last_eval_model_route_tasks),
            metadata
                .last_eval_mutation_scope_failures
                .map(|count| count.to_string())
                .unwrap_or_else(|| "-".to_string()),
            metadata
                .last_eval_unexpected_changed_files
                .map(|count| count.to_string())
                .unwrap_or_else(|| "-".to_string()),
            metadata
                .min_fixture_task_count
                .map(|count| count.to_string())
                .unwrap_or_else(|| "-".to_string()),
            metadata
                .min_fixture_command_count
                .map(|count| count.to_string())
                .unwrap_or_else(|| "-".to_string()),
            format_u64_count_map(&metadata.min_fixture_risk_labels),
            metadata
                .fixture_breadth_satisfied
                .map(|satisfied| if satisfied { "yes" } else { "no" })
                .unwrap_or("-"),
            metadata
                .fixture_risk_satisfied
                .map(|satisfied| if satisfied { "yes" } else { "no" })
                .unwrap_or("-"),
            require_protocol,
            metadata.protocol_eval_status.as_deref().unwrap_or("-"),
            metadata
                .protocol_eval_git_dirty
                .map(|dirty| if dirty { "yes" } else { "no" })
                .unwrap_or("-"),
            metadata
                .protocol_check_passes
                .map(|count| count.to_string())
                .unwrap_or_else(|| "-".to_string()),
            metadata
                .protocol_check_total
                .map(|count| count.to_string())
                .unwrap_or_else(|| "-".to_string()),
            metadata
                .protocol_check_strict
                .map(|count| count.to_string())
                .unwrap_or_else(|| "-".to_string()),
            metadata
                .protocol_check_thinking
                .map(|count| count.to_string())
                .unwrap_or_else(|| "-".to_string()),
            metadata
                .protocol_check_stream
                .map(|count| count.to_string())
                .unwrap_or_else(|| "-".to_string()),
            metadata
                .protocol_check_json
                .map(|count| count.to_string())
                .unwrap_or_else(|| "-".to_string()),
            metadata
                .protocol_check_transport
                .map(|count| count.to_string())
                .unwrap_or_else(|| "-".to_string()),
            source_audit,
            metadata.source_provenance_findings.unwrap_or(0),
            metadata.source_provenance_scan_source.as_deref().unwrap_or("-"),
            preview_line(metadata.reason.as_deref().unwrap_or("-"), 80)
        ));
    }
    if metadata.decision_type.as_deref() == Some("harness_patch_promotion") {
        detail.push_str(&format!(
            " promotion_eligible={} criterion={} baseline={} candidate={} protocol={} fixture_tasks={}->{} fixture_commands={}->{} fixture_risks=[{}]->[{}] model_routes=[{}]->[{}] promotion_reason={}",
            metadata
                .promotion_eligible
                .map(|eligible| if eligible { "yes" } else { "no" })
                .unwrap_or("-"),
            metadata.promotion_criterion.as_deref().unwrap_or("-"),
            metadata
                .promotion_baseline_eval_id
                .as_deref()
                .unwrap_or("-"),
            metadata
                .promotion_candidate_eval_id
                .as_deref()
                .unwrap_or("-"),
            metadata
                .promotion_protocol_eval_id
                .as_deref()
                .unwrap_or("-"),
            metadata
                .promotion_fixture_baseline_task_count
                .map(|count| count.to_string())
                .unwrap_or_else(|| "-".to_string()),
            metadata
                .promotion_fixture_candidate_task_count
                .map(|count| count.to_string())
                .unwrap_or_else(|| "-".to_string()),
            metadata
                .promotion_fixture_baseline_command_count
                .map(|count| count.to_string())
                .unwrap_or_else(|| "-".to_string()),
            metadata
                .promotion_fixture_candidate_command_count
                .map(|count| count.to_string())
                .unwrap_or_else(|| "-".to_string()),
            format_u64_count_map(&metadata.promotion_fixture_baseline_risk_labels),
            format_u64_count_map(&metadata.promotion_fixture_candidate_risk_labels),
            format_u64_count_map(&metadata.promotion_model_route_baseline),
            format_u64_count_map(&metadata.promotion_model_route_candidate),
            preview_line(
                metadata
                    .promotion_reason
                    .as_deref()
                    .or(metadata.reason.as_deref())
                    .unwrap_or("-"),
                80
            )
        ));
    }
    detail
}

fn graph_timeline_step_mentions_hypothesis(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphHypothesisMetadata>,
) -> bool {
    graph_timeline_step_hypothesis_key(step, metadata).is_some()
        || step.dst_kind == "hypothesis"
        || infer_graph_node_kind(&step.src_id) == "hypothesis"
        || matches!(
            step.relation.as_str(),
            "records_hypothesis" | "caused_by" | "supports" | "contradicts" | "explains"
        )
}

fn graph_timeline_step_hypothesis_key(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphHypothesisMetadata>,
) -> Option<String> {
    if metadata.contains_key(&step.event_id) {
        Some(step.event_id.clone())
    } else if metadata.contains_key(&step.src_id) {
        Some(step.src_id.clone())
    } else if metadata.contains_key(&step.dst_id) {
        Some(step.dst_id.clone())
    } else {
        None
    }
}

fn format_graph_hypothesis_metadata(metadata: &GraphHypothesisMetadata) -> String {
    let confidence = metadata
        .confidence
        .map(|confidence| format!("{confidence:.3}"))
        .unwrap_or_else(|| "-".to_string());
    format!(
        "hypothesis={} failure={} confidence={} status={} summary={} run={} source_event={}",
        metadata.hypothesis_id,
        metadata.failure_event_id.as_deref().unwrap_or("-"),
        confidence,
        metadata.status.as_deref().unwrap_or("-"),
        metadata
            .summary
            .as_deref()
            .map(|summary| preview_line(summary, 80))
            .unwrap_or_else(|| "-".to_string()),
        metadata.run_id.as_deref().unwrap_or("-"),
        metadata.event_id
    )
}

fn graph_timeline_step_mentions_version(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphVersionMetadata>,
) -> bool {
    graph_timeline_step_version_key(step, metadata).is_some()
        || step.dst_kind == "harness_version"
        || matches!(
            step.relation.as_str(),
            "uses_harness_version" | "based_on_harness_version"
        )
}

fn graph_timeline_step_version_key(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphVersionMetadata>,
) -> Option<String> {
    if metadata.contains_key(&step.event_id) {
        Some(step.event_id.clone())
    } else if metadata.contains_key(&step.src_id) {
        Some(step.src_id.clone())
    } else if metadata.contains_key(&step.dst_id) {
        Some(step.dst_id.clone())
    } else {
        None
    }
}

fn format_graph_version_metadata(metadata: &GraphVersionMetadata) -> String {
    format!(
        "harness={} base={} patch={} eval={} suite={} status={} run={} source_event={}:{}",
        metadata.harness_version.as_deref().unwrap_or("-"),
        metadata.base_harness_version.as_deref().unwrap_or("-"),
        metadata.patch_id.as_deref().unwrap_or("-"),
        metadata.eval_id.as_deref().unwrap_or("-"),
        metadata.suite.as_deref().unwrap_or("-"),
        metadata.status.as_deref().unwrap_or("-"),
        metadata.run_id.as_deref().unwrap_or("-"),
        metadata.event_id,
        metadata.event_type
    )
}

fn graph_timeline_step_mentions_run(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphRunMetadata>,
) -> bool {
    matches!(step.dst_kind.as_str(), "run" | "trace" | "task")
        || matches!(
            infer_graph_node_kind(&step.src_id).as_str(),
            "run" | "trace"
        )
        || matches!(
            step.relation.as_str(),
            "observed_in" | "traced_by" | "records_task"
        )
        || (step.relation == "tested_by" && metadata.contains_key(&step.src_id))
}

fn graph_timeline_step_run_key(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphRunMetadata>,
) -> Option<String> {
    if metadata.contains_key(&step.event_id) {
        Some(step.event_id.clone())
    } else if metadata.contains_key(&step.src_id) {
        Some(step.src_id.clone())
    } else if metadata.contains_key(&step.dst_id) {
        Some(step.dst_id.clone())
    } else {
        None
    }
}

fn format_graph_run_metadata(metadata: &GraphRunMetadata) -> String {
    format!(
        "run={} trace={} tasks=[{}] status={} source_event={}:{}",
        metadata.run_id.as_deref().unwrap_or("-"),
        metadata.trace_id.as_deref().unwrap_or("-"),
        metadata.task_ids.join(","),
        metadata.status.as_deref().unwrap_or("-"),
        metadata.event_id,
        metadata.event_type
    )
}

fn graph_timeline_step_mentions_artifact(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphArtifactMetadata>,
) -> bool {
    step.dst_kind == "artifact"
        || infer_graph_node_kind(&step.src_id) == "artifact"
        || matches!(
            step.relation.as_str(),
            "has_artifact" | "references_artifact"
        )
        || metadata.contains_key(&step.dst_id)
}

fn graph_timeline_step_artifact_key(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphArtifactMetadata>,
) -> Option<String> {
    if metadata.contains_key(&step.event_id) {
        Some(step.event_id.clone())
    } else if metadata.contains_key(&step.src_id) {
        Some(step.src_id.clone())
    } else if metadata.contains_key(&step.dst_id) {
        Some(step.dst_id.clone())
    } else {
        None
    }
}

fn format_graph_artifact_metadata(metadata: &GraphArtifactMetadata) -> String {
    let dirty = metadata
        .git_dirty
        .map(|dirty| if dirty { "yes" } else { "no" })
        .unwrap_or("-");
    format!(
        "artifact={} eval={} patch={} suite={} status={} repro={} agent_source={} dirty={} commands={} fixture_tasks={} fixture_commands={} replay={} run={} source_event={}:{}",
        metadata.artifact_uri,
        metadata.eval_id.as_deref().unwrap_or("-"),
        metadata.patch_id.as_deref().unwrap_or("-"),
        metadata.suite.as_deref().unwrap_or("-"),
        metadata.status.as_deref().unwrap_or("-"),
        metadata.repro_mode.as_deref().unwrap_or("-"),
        metadata.agent_command_source.as_deref().unwrap_or("-"),
        dirty,
        metadata.command_count,
        metadata
            .fixture_task_count
            .map(|count| count.to_string())
            .unwrap_or_else(|| "-".to_string()),
        metadata
            .fixture_command_count
            .map(|count| count.to_string())
            .unwrap_or_else(|| "-".to_string()),
        metadata.replay_command.as_deref().unwrap_or("-"),
        metadata.run_id.as_deref().unwrap_or("-"),
        metadata.event_id,
        metadata.event_type
    )
}

fn graph_timeline_step_mentions_model(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphModelMetadata>,
) -> bool {
    graph_timeline_step_model_key(step, metadata).is_some()
        || matches!(step.dst_kind.as_str(), "model" | "model_call")
        || matches!(step.relation.as_str(), "records_model_call" | "uses_model")
}

fn graph_timeline_step_model_key(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphModelMetadata>,
) -> Option<String> {
    if metadata.contains_key(&step.event_id) {
        Some(step.event_id.clone())
    } else if metadata.contains_key(&step.src_id) {
        Some(step.src_id.clone())
    } else if metadata.contains_key(&step.dst_id) {
        Some(step.dst_id.clone())
    } else {
        None
    }
}

fn format_graph_model_metadata(metadata: &GraphModelMetadata) -> String {
    format!(
        "call={} model={} route={} thinking={} effort={} tokens=in:{} out:{} cache_read:{} cache_write:{} run={}",
        metadata.model_call_id,
        metadata.model.as_deref().unwrap_or("-"),
        metadata.route_task.as_deref().unwrap_or("-"),
        metadata.thinking.as_deref().unwrap_or("-"),
        metadata.reasoning_effort.as_deref().unwrap_or("-"),
        metadata
            .input_tokens
            .map(|tokens| tokens.to_string())
            .unwrap_or_else(|| "-".to_string()),
        metadata
            .output_tokens
            .map(|tokens| tokens.to_string())
            .unwrap_or_else(|| "-".to_string()),
        metadata
            .cache_read_tokens
            .map(|tokens| tokens.to_string())
            .unwrap_or_else(|| "-".to_string()),
        metadata
            .cache_write_tokens
            .map(|tokens| tokens.to_string())
            .unwrap_or_else(|| "-".to_string()),
        metadata.run_id.as_deref().unwrap_or("-")
    )
}

fn graph_timeline_step_mentions_tool(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphToolMetadata>,
) -> bool {
    graph_timeline_step_tool_key(step, metadata).is_some()
        || matches!(step.dst_kind.as_str(), "tool" | "tool_call")
        || matches!(step.relation.as_str(), "records_tool_call" | "invokes_tool")
}

fn graph_timeline_step_tool_key(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphToolMetadata>,
) -> Option<String> {
    if metadata.contains_key(&step.event_id) {
        Some(step.event_id.clone())
    } else if metadata.contains_key(&step.src_id) {
        Some(step.src_id.clone())
    } else if metadata.contains_key(&step.dst_id) {
        Some(step.dst_id.clone())
    } else {
        None
    }
}

fn format_graph_tool_metadata(metadata: &GraphToolMetadata) -> String {
    format!(
        "call={} tool={} status={} args={} result={} run={}",
        metadata.tool_call_id,
        metadata.tool_name.as_deref().unwrap_or("-"),
        metadata.status.as_deref().unwrap_or("-"),
        metadata.args_preview.as_deref().unwrap_or("-"),
        metadata.result_preview.as_deref().unwrap_or("-"),
        metadata.run_id.as_deref().unwrap_or("-")
    )
}

fn graph_timeline_step_mentions_command(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphCommandMetadata>,
) -> bool {
    graph_timeline_step_command_key(step, metadata).is_some()
}

fn graph_timeline_step_command_key(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphCommandMetadata>,
) -> Option<String> {
    if metadata.contains_key(&step.event_id) {
        Some(step.event_id.clone())
    } else if metadata.contains_key(&step.src_id) {
        Some(step.src_id.clone())
    } else if metadata.contains_key(&step.dst_id) {
        Some(step.dst_id.clone())
    } else {
        None
    }
}

fn format_graph_command_metadata(metadata: &GraphCommandMetadata) -> String {
    format!(
        "command={} status={} result={} run={}",
        metadata.command.as_deref().unwrap_or("-"),
        metadata.status.as_deref().unwrap_or("-"),
        metadata.result_preview.as_deref().unwrap_or("-"),
        metadata.run_id.as_deref().unwrap_or("-")
    )
}

fn graph_timeline_step_mentions_test(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphTestMetadata>,
) -> bool {
    graph_timeline_step_test_key(step, metadata).is_some()
}

fn graph_timeline_step_test_key(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphTestMetadata>,
) -> Option<String> {
    if metadata.contains_key(&step.event_id) {
        Some(step.event_id.clone())
    } else if metadata.contains_key(&step.src_id) {
        Some(step.src_id.clone())
    } else if metadata.contains_key(&step.dst_id) {
        Some(step.dst_id.clone())
    } else {
        None
    }
}

fn format_graph_test_metadata(metadata: &GraphTestMetadata) -> String {
    format!(
        "test={} command={} status={} result={} run={}",
        metadata.test_kind.as_deref().unwrap_or("-"),
        metadata.command.as_deref().unwrap_or("-"),
        metadata.status.as_deref().unwrap_or("-"),
        metadata.result_preview.as_deref().unwrap_or("-"),
        metadata.run_id.as_deref().unwrap_or("-")
    )
}

fn graph_timeline_step_mentions_commit(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphCommitMetadata>,
) -> bool {
    graph_timeline_step_commit_key(step, metadata).is_some()
        || matches!(step.dst_kind.as_str(), "commit" | "branch")
        || matches!(
            step.relation.as_str(),
            "records_commit" | "reverted_commit" | "on_branch"
        )
}

fn graph_timeline_step_commit_key(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphCommitMetadata>,
) -> Option<String> {
    if metadata.contains_key(&step.event_id) {
        Some(step.event_id.clone())
    } else if metadata.contains_key(&step.src_id) {
        Some(step.src_id.clone())
    } else if metadata.contains_key(&step.dst_id) {
        Some(step.dst_id.clone())
    } else {
        None
    }
}

fn format_graph_commit_metadata(metadata: &GraphCommitMetadata) -> String {
    format!(
        "commit={} reverted={} branch={} files=[{}] message={} reason={} run={} source={}:{}",
        metadata.commit.as_deref().unwrap_or("-"),
        metadata.reverted_commit.as_deref().unwrap_or("-"),
        metadata.branch.as_deref().unwrap_or("-"),
        metadata
            .files
            .iter()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .join(", "),
        metadata.message.as_deref().unwrap_or("-"),
        metadata.reason.as_deref().unwrap_or("-"),
        metadata.run_id.as_deref().unwrap_or("-"),
        metadata.event_id,
        metadata.event_type
    )
}

fn graph_timeline_step_mentions_memory(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphMemoryMetadata>,
) -> bool {
    graph_timeline_step_memory_key(step, metadata).is_some()
        || step.dst_kind == "memory"
        || matches!(
            step.relation.as_str(),
            "records_memory" | "proposes_memory" | "promoted_memory" | "rejected_memory"
        )
}

fn graph_timeline_step_memory_key(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphMemoryMetadata>,
) -> Option<String> {
    if metadata.contains_key(&step.event_id) {
        Some(step.event_id.clone())
    } else if metadata.contains_key(&step.src_id) {
        Some(step.src_id.clone())
    } else if metadata.contains_key(&step.dst_id) {
        Some(step.dst_id.clone())
    } else {
        None
    }
}

fn format_graph_memory_metadata(metadata: &GraphMemoryMetadata) -> String {
    format!(
        "candidate={} status={} source={} summary={} reason={} proposed={} evidence=[{}] run={} source_event={}:{}",
        metadata.candidate_id,
        metadata.status.as_deref().unwrap_or("-"),
        metadata.source.as_deref().unwrap_or("-"),
        metadata.summary.as_deref().unwrap_or("-"),
        metadata.reason.as_deref().unwrap_or("-"),
        metadata.proposed_event_id.as_deref().unwrap_or("-"),
        metadata
            .evidence_event_ids
            .iter()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .join(", "),
        metadata.run_id.as_deref().unwrap_or("-"),
        metadata.event_id,
        metadata.event_type
    )
}

fn graph_timeline_step_mentions_issue(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphIssueMetadata>,
) -> bool {
    graph_timeline_step_issue_key(step, metadata).is_some()
        || step.dst_kind == "issue"
        || matches!(step.relation.as_str(), "records_issue" | "addresses_patch")
}

fn graph_timeline_step_issue_key(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphIssueMetadata>,
) -> Option<String> {
    if metadata.contains_key(&step.event_id) {
        Some(step.event_id.clone())
    } else if metadata.contains_key(&step.src_id) {
        Some(step.src_id.clone())
    } else if metadata.contains_key(&step.dst_id) {
        Some(step.dst_id.clone())
    } else {
        None
    }
}

fn format_graph_issue_metadata(metadata: &GraphIssueMetadata) -> String {
    format!(
        "issue={} patch={} source={} kind={} patch_kind={} risk={} status={} summary={} details={} run={} source_event={}",
        metadata.issue_id,
        metadata.patch_id.as_deref().unwrap_or("-"),
        metadata.intake_source.as_deref().unwrap_or("-"),
        metadata.intake_kind.as_deref().unwrap_or("-"),
        metadata.kind.as_deref().unwrap_or("-"),
        metadata.risk_level.as_deref().unwrap_or("-"),
        metadata.status.as_deref().unwrap_or("-"),
        metadata.summary.as_deref().unwrap_or("-"),
        metadata.details.as_deref().unwrap_or("-"),
        metadata.run_id.as_deref().unwrap_or("-"),
        metadata.event_id
    )
}

fn graph_timeline_step_mentions_cache(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphCacheMetadata>,
) -> bool {
    graph_timeline_step_cache_key(step, metadata).is_some()
}

fn graph_cache_for_timeline_step<'a>(
    step: &GraphTimelineStep,
    metadata: &'a BTreeMap<String, GraphCacheMetadata>,
) -> Option<&'a GraphCacheMetadata> {
    graph_timeline_step_cache_key(step, metadata).and_then(|cache_id| metadata.get(&cache_id))
}

fn graph_timeline_step_cache_key(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphCacheMetadata>,
) -> Option<String> {
    if metadata.contains_key(&step.src_id) {
        Some(step.src_id.clone())
    } else if metadata.contains_key(&step.dst_id) {
        Some(step.dst_id.clone())
    } else if metadata.contains_key(&step.event_id) {
        Some(step.event_id.clone())
    } else {
        None
    }
}

fn graph_cache_metadata_payload_map(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphCacheMetadata>,
) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    for step in steps {
        if let Some(cache) = graph_cache_for_timeline_step(step, metadata) {
            out.entry(cache.event_id.clone())
                .or_insert_with(|| graph_cache_metadata_payload(cache));
        }
    }
    out
}

fn graph_cache_relations_payload(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphCacheMetadata>,
) -> Vec<Value> {
    steps
        .iter()
        .filter_map(|step| {
            graph_cache_for_timeline_step(step, metadata).map(|cache| {
                serde_json::json!({
                    "event_id": &step.event_id,
                    "event_type": &step.event_type,
                    "depth": step.depth,
                    "timestamp_ms": step.timestamp_ms,
                    "src_id": &step.src_id,
                    "relation": &step.relation,
                    "dst_id": &step.dst_id,
                    "dst_kind": &step.dst_kind,
                    "cache_event_id": &cache.event_id,
                })
            })
        })
        .collect()
}

fn graph_cache_metadata_payload(metadata: &GraphCacheMetadata) -> Value {
    serde_json::json!({
        "event_id": &metadata.event_id,
        "model": &metadata.model,
        "prompt_cache_hit_tokens": metadata.prompt_cache_hit_tokens,
        "prompt_cache_miss_tokens": metadata.prompt_cache_miss_tokens,
        "cache_hit_ratio": metadata.cache_hit_ratio,
        "timestamp_ms": metadata.timestamp_ms,
    })
}

fn format_graph_cache_metadata(metadata: &GraphCacheMetadata) -> String {
    let hit = metadata
        .prompt_cache_hit_tokens
        .map(|tokens| tokens.to_string())
        .unwrap_or_else(|| "-".to_string());
    let miss = metadata
        .prompt_cache_miss_tokens
        .map(|tokens| tokens.to_string())
        .unwrap_or_else(|| "-".to_string());
    let ratio = metadata
        .cache_hit_ratio
        .map(|ratio| format!("{ratio:.3}"))
        .unwrap_or_else(|| "-".to_string());
    format!(
        "cache={} model={} hit={} miss={} ratio={} cache_t={}",
        metadata.event_id,
        metadata.model.as_deref().unwrap_or("-"),
        hit,
        miss,
        ratio,
        metadata.timestamp_ms
    )
}

fn graph_timeline_step_mentions_failure(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphFailureMetadata>,
) -> bool {
    graph_timeline_step_failure_key(step, metadata).is_some()
}

fn graph_failure_for_timeline_step<'a>(
    step: &GraphTimelineStep,
    metadata: &'a BTreeMap<String, GraphFailureMetadata>,
) -> Option<&'a GraphFailureMetadata> {
    graph_timeline_step_failure_key(step, metadata).and_then(|failure_id| metadata.get(&failure_id))
}

fn graph_timeline_step_failure_key(
    step: &GraphTimelineStep,
    metadata: &BTreeMap<String, GraphFailureMetadata>,
) -> Option<String> {
    if metadata.contains_key(&step.src_id) {
        Some(step.src_id.clone())
    } else if metadata.contains_key(&step.dst_id) {
        Some(step.dst_id.clone())
    } else if metadata.contains_key(&step.event_id) {
        Some(step.event_id.clone())
    } else {
        None
    }
}

fn graph_failure_metadata_payload_map(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphFailureMetadata>,
) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    for step in steps {
        if let Some(failure) = graph_failure_for_timeline_step(step, metadata) {
            out.entry(failure.event_id.clone())
                .or_insert_with(|| graph_failure_metadata_payload(failure));
        }
    }
    out
}

fn graph_failure_relations_payload(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphFailureMetadata>,
) -> Vec<Value> {
    steps
        .iter()
        .filter_map(|step| {
            graph_failure_for_timeline_step(step, metadata).map(|failure| {
                serde_json::json!({
                    "event_id": &step.event_id,
                    "event_type": &step.event_type,
                    "depth": step.depth,
                    "timestamp_ms": step.timestamp_ms,
                    "src_id": &step.src_id,
                    "relation": &step.relation,
                    "dst_id": &step.dst_id,
                    "dst_kind": &step.dst_kind,
                    "failure_event_id": &failure.event_id,
                })
            })
        })
        .collect()
}

fn graph_failure_metadata_payload(metadata: &GraphFailureMetadata) -> Value {
    serde_json::json!({
        "event_id": &metadata.event_id,
        "event_type": &metadata.event_type,
        "class": &metadata.class,
        "owner": &metadata.owner,
        "retryable": metadata.retryable,
        "source": &metadata.source,
        "error_preview": &metadata.error_preview,
        "run_id": &metadata.run_id,
        "timestamp_ms": metadata.timestamp_ms,
    })
}

fn format_graph_failure_metadata(metadata: &GraphFailureMetadata) -> String {
    format!(
        "failure={} class={} owner={} retryable={} source={} signal={} run={} failure_t={} failure_type={}",
        metadata.event_id,
        metadata.class,
        metadata.owner,
        metadata.retryable,
        metadata.source.as_deref().unwrap_or("-"),
        preview_line(metadata.error_preview.as_deref().unwrap_or("-"), 80),
        metadata.run_id.as_deref().unwrap_or("-"),
        metadata.timestamp_ms,
        metadata.event_type
    )
}

fn graph_timeline_step_mentions_policy(step: &GraphTimelineStep) -> bool {
    graph_policy_relation(&step.relation)
        || infer_graph_node_kind(&step.src_id) == "policy"
        || matches!(
            step.dst_kind.as_str(),
            "policy"
                | "context_policy"
                | "prompt_layout"
                | "prompt_version"
                | "context_block"
                | "instruction_file"
                | "tool_schema"
                | "tool_schema_version"
        )
}

fn graph_policy_relation(relation: &str) -> bool {
    matches!(
        relation,
        "uses_context_policy"
            | "uses_prompt_layout"
            | "uses_prompt"
            | "uses_context_block"
            | "uses_instruction_file"
            | "uses_schema"
            | "uses_schema_version"
            | "passed_source_provenance_audit"
            | "blocked_by_source_provenance_audit"
            | "used_source_provenance_scan"
            | "supports_source_provenance_audit"
            | "supports_strict_tool_call_check"
            | "uses_transport_policy"
            | "supports_transport_policy_check"
            | "uses_thinking_protocol_policy"
            | "supports_thinking_protocol_check"
            | "uses_streaming_protocol_policy"
            | "supports_streaming_protocol_check"
            | "supports_json_output_check"
    )
}

fn graph_policy_metadata_payload_map(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphPolicyMetadata>,
) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    for step in steps {
        if let Some(policy) = metadata.get(&step.event_id) {
            out.entry(policy.event_id.clone())
                .or_insert_with(|| graph_policy_metadata_payload(policy));
        }
    }
    out
}

fn graph_policy_relations_payload(
    steps: &[GraphTimelineStep],
    metadata: &BTreeMap<String, GraphPolicyMetadata>,
) -> Vec<Value> {
    steps
        .iter()
        .map(|step| {
            serde_json::json!({
                "event_id": &step.event_id,
                "event_type": &step.event_type,
                "depth": step.depth,
                "timestamp_ms": step.timestamp_ms,
                "src_id": &step.src_id,
                "relation": &step.relation,
                "dst_id": &step.dst_id,
                "dst_kind": &step.dst_kind,
                "metadata_event_id": metadata.get(&step.event_id).map(|policy| &policy.event_id),
            })
        })
        .collect()
}

fn graph_policy_metadata_payload(metadata: &GraphPolicyMetadata) -> Value {
    serde_json::json!({
        "event_id": &metadata.event_id,
        "event_type": &metadata.event_type,
        "run_id": &metadata.run_id,
        "context_policy": &metadata.context_policy,
        "prompt_layout": &metadata.prompt_layout,
        "prompt_version": &metadata.prompt_version,
        "schema_name": &metadata.schema_name,
        "schema_version": &metadata.schema_version,
        "source_provenance_scan_source": &metadata.source_provenance_scan_source,
        "source_provenance_passed": metadata.source_provenance_passed,
        "source_provenance_findings": metadata.source_provenance_findings,
        "stable_blocks": metadata.stable_blocks,
        "dynamic_blocks": metadata.dynamic_blocks,
        "included_blocks": &metadata.included_blocks,
        "instruction_files": &metadata.instruction_files,
    })
}

fn format_graph_policy_metadata(metadata: &GraphPolicyMetadata) -> String {
    format!(
        "policy={} layout={} prompt={} schema={} version={} source_audit={} source_scan={} source_findings={} blocks={}/{} instructions=[{}] included=[{}] run={} source={}:{}",
        metadata.context_policy.as_deref().unwrap_or("-"),
        metadata.prompt_layout.as_deref().unwrap_or("-"),
        metadata.prompt_version.as_deref().unwrap_or("-"),
        metadata.schema_name.as_deref().unwrap_or("-"),
        metadata.schema_version.as_deref().unwrap_or("-"),
        metadata
            .source_provenance_passed
            .map(|passed| if passed { "passed" } else { "blocked" })
            .unwrap_or("-"),
        metadata
            .source_provenance_scan_source
            .as_deref()
            .unwrap_or("-"),
        metadata
            .source_provenance_findings
            .map(|findings| findings.to_string())
            .unwrap_or_else(|| "-".to_string()),
        metadata.stable_blocks,
        metadata.dynamic_blocks,
        metadata
            .instruction_files
            .iter()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .join(", "),
        metadata
            .included_blocks
            .iter()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .join(", "),
        metadata.run_id.as_deref().unwrap_or("-"),
        metadata.event_id,
        metadata.event_type
    )
}

fn summarize_graph_signals(steps: &[GraphRelationStep]) -> GraphSignals {
    let mut signals = GraphSignals::default();
    for step in steps {
        let relation = &step.relation;
        let edge = GraphSignalEdge {
            depth: step.depth,
            src_id: relation.src_id.clone(),
            relation: relation.relation.clone(),
            dst_id: relation.dst_id.clone(),
            dst_kind: relation.dst_kind.clone(),
        };
        if graph_positive_relation(&relation.relation) {
            *signals
                .positive_counts
                .entry(relation.relation.clone())
                .or_default() += 1;
            signals.positive_edges.push(edge.clone());
        }
        if graph_risk_relation(&relation.relation) {
            *signals
                .risk_counts
                .entry(relation.relation.clone())
                .or_default() += 1;
            signals.risk_edges.push(edge);
        }
    }
    signals.positive_edges.sort_by(signal_edge_sort);
    signals.risk_edges.sort_by(signal_edge_sort);
    signals
}

fn signal_edge_sort(left: &GraphSignalEdge, right: &GraphSignalEdge) -> std::cmp::Ordering {
    left.depth
        .cmp(&right.depth)
        .then_with(|| left.relation.cmp(&right.relation))
        .then_with(|| left.src_id.cmp(&right.src_id))
        .then_with(|| left.dst_id.cmp(&right.dst_id))
}

fn append_graph_signal_rows(
    out: &mut String,
    label: &str,
    edges: &[GraphSignalEdge],
    limit: usize,
) {
    if edges.is_empty() {
        return;
    }
    out.push_str(&format!("  {label}:\n"));
    for edge in edges.iter().take(limit) {
        out.push_str(&format!(
            "    d{} {} -[{}]-> {} ({})\n",
            edge.depth, edge.src_id, edge.relation, edge.dst_id, edge.dst_kind
        ));
    }
}

fn summarize_graph_impact(steps: &[GraphRelationStep]) -> GraphImpact {
    let mut impact = GraphImpact::default();
    for step in steps {
        let relation = &step.relation;
        let src_kind = graph_relation_source_kind(relation);
        record_graph_impact_node(&mut impact, &relation.src_id, &src_kind);
        record_graph_impact_node(&mut impact, &relation.dst_id, &relation.dst_kind);
        *impact
            .relation_counts
            .entry(relation.relation.clone())
            .or_default() += 1;

        if graph_positive_relation(&relation.relation) {
            *impact
                .positive_signals
                .entry(relation.relation.clone())
                .or_default() += 1;
        }
        if graph_risk_relation(&relation.relation) {
            *impact
                .risk_signals
                .entry(relation.relation.clone())
                .or_default() += 1;
        }
        if graph_evidence_relation(&relation.relation) {
            if graph_impact_evidence_node(&relation.src_id, &src_kind) {
                impact.evidence_nodes.insert(relation.src_id.clone());
            }
            if graph_impact_evidence_node(&relation.dst_id, &relation.dst_kind) {
                impact.evidence_nodes.insert(relation.dst_id.clone());
            }
        }
    }
    impact
}

fn graph_relation_source_kind(relation: &crate::state::StateRelation) -> String {
    match relation.relation.as_str() {
        "supports_strict_tool_call_check" | "supports_json_output_check" => {
            "tool_schema".to_string()
        }
        "supports_transport_policy_check" => "evidence".to_string(),
        "supports_thinking_protocol_check" => "evidence".to_string(),
        "supports_streaming_protocol_check" => "evidence".to_string(),
        _ => infer_graph_node_kind(&relation.src_id),
    }
}

fn graph_impact_evidence_node(id: &str, kind: &str) -> bool {
    matches!(kind, "event" | "evidence")
        || matches!(infer_graph_node_kind(id).as_str(), "event" | "evidence")
}

fn record_graph_impact_node(impact: &mut GraphImpact, id: &str, kind: &str) {
    let inferred = if kind == "unknown" {
        infer_graph_node_kind(id)
    } else {
        kind.to_string()
    };
    impact
        .nodes
        .entry(id.to_string())
        .or_insert_with(|| inferred.clone());
    match inferred.as_str() {
        "patch" => {
            impact.patches.insert(id.to_string());
        }
        "eval" => {
            impact.evals.insert(id.to_string());
        }
        "decision" => {
            impact.decisions.insert(id.to_string());
        }
        "file" => {
            impact.files.insert(id.to_string());
        }
        "policy"
        | "context_policy"
        | "prompt_layout"
        | "prompt_version"
        | "context_block"
        | "tool_schema"
        | "tool_schema_version" => {
            impact.policies.insert(id.to_string());
        }
        _ => {}
    }
}

fn graph_evidence_relation(relation: &str) -> bool {
    matches!(
        relation,
        "addresses"
            | "evaluated_failure"
            | "explains"
            | "supports"
            | "contradicts"
            | "missing_required_gate"
            | "fixture_breadth_below_minimum"
            | "fixture_risk_coverage_below_minimum"
            | "fixture_agent_mutation_scope_block"
            | "promotion_fixture_risk_mismatch"
            | "has_source_provenance_finding"
            | "agent_attempt_changed_file"
            | "agent_attempt_unexpected_file"
            | "covers_protocol_check"
            | "passed_source_provenance_audit"
            | "blocked_by_source_provenance_audit"
            | "requires_protocol_eval"
            | "supports_source_provenance_audit"
            | "supports_strict_tool_call_check"
            | "supports_transport_policy_check"
            | "supports_thinking_protocol_check"
            | "supports_streaming_protocol_check"
            | "supports_json_output_check"
    )
}

fn graph_positive_relation(relation: &str) -> bool {
    matches!(
        relation,
        "validated_by"
            | "promoted_by"
            | "approved_by"
            | "passed_source_provenance_audit"
            | "supports_source_provenance_audit"
            | "supports_release_gate"
            | "supports_promotion"
            | "covers_protocol_check"
            | "supports_strict_tool_call_check"
            | "supports_transport_policy_check"
            | "supports_thinking_protocol_check"
            | "supports_streaming_protocol_check"
            | "supports_json_output_check"
    )
}

fn graph_risk_relation(relation: &str) -> bool {
    matches!(
        relation,
        "rejected_by"
            | "reverted_by"
            | "stale_due_to"
            | "blocked_by_dirty_eval"
            | "dirty_eval_blocks_release_gate"
            | "dirty_eval_blocks_promotion"
            | "older_than_candidate_eval"
            | "blocked_by_stale_protocol_eval"
            | "missing_required_gate"
            | "fixture_breadth_below_minimum"
            | "fixture_risk_coverage_below_minimum"
            | "fixture_agent_mutation_scope_block"
            | "promotion_fixture_risk_mismatch"
            | "blocks_promotion"
            | "blocked_by_source_provenance_audit"
            | "blocks_release_gate"
            | "contradicts"
            | "agent_attempt_unexpected_file"
            | "evaluated_failure"
    )
}

fn format_graph_impact_ids(ids: &BTreeSet<String>, limit: usize) -> String {
    if ids.is_empty() {
        return "-".to_string();
    }
    let values = ids.iter().cloned().collect::<Vec<_>>();
    compact_id_list(&values, limit)
}

fn graph_cluster_seed_priority(seed: &str) -> usize {
    match infer_graph_node_kind(seed).as_str() {
        "patch" => 0,
        "eval" => 1,
        "decision" | "hypothesis" => 2,
        "file" | "issue" => 3,
        "event" | "run" | "trace" => 4,
        _ => 5,
    }
}

fn collect_graph_cluster_nodes(
    root_id: &str,
    seed: &str,
    adjacency: &BTreeMap<String, BTreeSet<String>>,
) -> BTreeSet<String> {
    let mut nodes = BTreeSet::new();
    let mut queue = VecDeque::from([seed.to_string()]);
    while let Some(node) = queue.pop_front() {
        if node == root_id || !nodes.insert(node.clone()) {
            continue;
        }
        if nodes.len() >= 100 {
            break;
        }
        if let Some(neighbors) = adjacency.get(&node) {
            for neighbor in neighbors {
                if neighbor != root_id && !nodes.contains(neighbor) {
                    queue.push_back(neighbor.clone());
                }
            }
        }
    }
    nodes
}

pub(crate) fn build_graph_path_report(
    sqlite_path: &Path,
    from: &str,
    to: &str,
    depth: usize,
) -> Result<String, String> {
    let max_depth = depth.clamp(1, 4);
    let steps = query_graph_path(sqlite_path, from, to, depth)?;
    if from == to {
        return Ok(format!(
            "State graph path: {from} -> {to} depth=0\n  same node"
        ));
    }
    if steps.is_empty() {
        return Err(format!(
            "no graph path found from '{from}' to '{to}' within depth {max_depth}"
        ));
    }
    let mut out = String::new();
    out.push_str(&format!(
        "State graph path: {from} -> {to} depth={}/{}\n",
        steps.len(),
        max_depth
    ));
    for step in steps {
        let relation = step.relation;
        let src_kind = infer_graph_node_kind(&relation.src_id);
        if relation.src_id == step.from_id && relation.dst_id == step.to_id {
            out.push_str(&format!(
                "  d{} {} -[{}]-> {} ({} -> {})\n",
                step.depth,
                step.from_id,
                relation.relation,
                step.to_id,
                src_kind,
                relation.dst_kind
            ));
        } else {
            out.push_str(&format!(
                "  d{} {} <-[{}]- {} ({} -> {})\n",
                step.depth,
                step.from_id,
                relation.relation,
                step.to_id,
                src_kind,
                relation.dst_kind
            ));
        }
    }
    Ok(out.trim_end().to_string())
}

fn query_graph_steps(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
) -> Result<Vec<GraphRelationStep>, String> {
    let max_depth = depth.clamp(1, 4);
    let mut queue = VecDeque::from([(id.to_string(), 0usize)]);
    let mut visited_nodes = BTreeSet::from([id.to_string()]);
    let mut seen_relations = BTreeSet::new();
    let mut steps = Vec::new();

    while let Some((node, node_depth)) = queue.pop_front() {
        if node_depth >= max_depth {
            continue;
        }
        let relation_depth = node_depth + 1;
        for relation in crate::state::query_sqlite_relations(sqlite_path, &node)? {
            let key = (
                relation.src_id.clone(),
                relation.relation.clone(),
                relation.dst_id.clone(),
            );
            if seen_relations.insert(key) {
                steps.push(GraphRelationStep {
                    depth: relation_depth,
                    relation: relation.clone(),
                });
            }
            for neighbor in [&relation.src_id, &relation.dst_id] {
                if visited_nodes.insert(neighbor.clone()) {
                    queue.push_back((neighbor.clone(), relation_depth));
                }
            }
        }
    }

    steps.sort_by(|a, b| {
        a.depth
            .cmp(&b.depth)
            .then_with(|| a.relation.relation.cmp(&b.relation.relation))
            .then_with(|| a.relation.src_id.cmp(&b.relation.src_id))
            .then_with(|| a.relation.dst_id.cmp(&b.relation.dst_id))
    });
    steps.truncate(200);
    Ok(steps)
}

fn query_graph_eval_metadata(
    sqlite_path: &Path,
) -> Result<BTreeMap<String, GraphEvalMetadata>, String> {
    let conn = Connection::open(sqlite_path)
        .map_err(|e| format!("open sqlite projection '{}': {e}", sqlite_path.display()))?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT eval_id, patch_id, suite, status, score, last_event_id, payload_json
            FROM eval_results
            ORDER BY eval_id
            LIMIT 10000
            "#,
        )
        .map_err(|e| format!("prepare state graph eval metadata query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            let payload_json = row.get::<_, String>(6)?;
            let payload = serde_json::from_str::<Value>(&payload_json).unwrap_or(Value::Null);
            let fixture_suite = payload
                .get("metrics")
                .and_then(|metrics| metrics.get("fixture_suite"))
                .or_else(|| payload.get("fixture_suite"));
            let state_metrics = payload
                .get("metrics")
                .and_then(|metrics| metrics.get("state_metrics"));
            Ok(GraphEvalMetadata {
                eval_id: row.get(0)?,
                patch_id: row.get(1)?,
                suite: row.get(2)?,
                status: row.get(3)?,
                score: row.get(4)?,
                fixture_task_count: fixture_suite
                    .and_then(|suite| payload_u64(suite, "task_count")),
                fixture_command_count: fixture_suite
                    .and_then(|suite| payload_u64(suite, "command_count")),
                deepseek_protocol_checks: state_metrics
                    .and_then(|metrics| payload_u64(metrics, "deepseek_protocol_checks")),
                deepseek_protocol_passes: state_metrics
                    .and_then(|metrics| payload_u64(metrics, "deepseek_protocol_passes")),
                deepseek_strict_tool_call_checks: state_metrics
                    .and_then(|metrics| payload_u64(metrics, "deepseek_strict_tool_call_checks")),
                deepseek_thinking_protocol_checks: state_metrics
                    .and_then(|metrics| payload_u64(metrics, "deepseek_thinking_protocol_checks")),
                deepseek_streaming_protocol_checks: state_metrics
                    .and_then(|metrics| payload_u64(metrics, "deepseek_streaming_protocol_checks")),
                deepseek_json_output_checks: state_metrics
                    .and_then(|metrics| payload_u64(metrics, "deepseek_json_output_checks")),
                deepseek_transport_policy_checks: state_metrics
                    .and_then(|metrics| payload_u64(metrics, "deepseek_transport_policy_checks")),
                model_route_tasks: state_metrics
                    .and_then(|metrics| metrics.get("model_route_tasks"))
                    .map(value_u64_count_map)
                    .unwrap_or_default(),
                fixture_agent_changed_files: graph_payload_fixture_agent_changed_files(&payload),
                fixture_agent_unexpected_files: graph_payload_fixture_agent_unexpected_files(
                    &payload,
                ),
                last_event_id: row.get(5)?,
            })
        })
        .map_err(|e| format!("query state graph eval metadata: {e}"))?;

    let mut out = BTreeMap::new();
    for row in rows {
        let metadata = row.map_err(|e| format!("read state graph eval metadata row: {e}"))?;
        let last_event_id = metadata.last_event_id.clone();
        out.insert(metadata.eval_id.clone(), metadata.clone());
        out.insert(last_event_id, metadata);
    }
    Ok(out)
}

fn query_graph_patch_metadata(
    sqlite_path: &Path,
) -> Result<BTreeMap<String, GraphPatchMetadata>, String> {
    let conn = Connection::open(sqlite_path)
        .map_err(|e| format!("open sqlite projection '{}': {e}", sqlite_path.display()))?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT patch_id, status, kind, risk_level, base_harness_version, state_version,
                   base_git_commit, rollback_plan_json, last_event_id
            FROM harness_patches
            ORDER BY patch_id
            LIMIT 10000
            "#,
        )
        .map_err(|e| format!("prepare state graph patch metadata query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(GraphPatchMetadata {
                patch_id: row.get(0)?,
                status: row.get(1)?,
                kind: row.get(2)?,
                risk_level: row.get(3)?,
                base_harness_version: row.get(4)?,
                state_version: row
                    .get::<_, Option<i64>>(5)?
                    .and_then(|version| u64::try_from(version).ok()),
                base_git_commit: row.get(6)?,
                rollback_plan_steps: row
                    .get::<_, Option<String>>(7)?
                    .as_deref()
                    .map(count_json_array_items)
                    .unwrap_or_default(),
                last_event_id: row.get(8)?,
            })
        })
        .map_err(|e| format!("query state graph patch metadata: {e}"))?;

    let mut out = BTreeMap::new();
    for row in rows {
        let metadata = row.map_err(|e| format!("read state graph patch metadata row: {e}"))?;
        let last_event_id = metadata.last_event_id.clone();
        out.insert(metadata.patch_id.clone(), metadata.clone());
        out.insert(last_event_id, metadata);
    }
    Ok(out)
}

fn query_graph_decision_metadata(
    sqlite_path: &Path,
) -> Result<BTreeMap<String, GraphDecisionMetadata>, String> {
    let conn = Connection::open(sqlite_path)
        .map_err(|e| format!("open sqlite projection '{}': {e}", sqlite_path.display()))?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT decision_id, decision_type, decision, status, patch_id, eval_id, event_id,
                   payload_json
            FROM decisions
            ORDER BY decision_id
            LIMIT 10000
            "#,
        )
        .map_err(|e| format!("prepare state graph decision metadata query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            let payload_json = row.get::<_, String>(7)?;
            let payload = serde_json::from_str::<Value>(&payload_json).unwrap_or(Value::Null);
            let promotion_decision = payload.get("promotion_decision");
            let promotion_fixture_suite = promotion_decision
                .and_then(|decision| decision.get("metric_evidence"))
                .and_then(|evidence| evidence.get("fixture_suite"));
            let promotion_fixture_baseline =
                promotion_fixture_suite.and_then(|suite| suite.get("baseline"));
            let promotion_fixture_candidate =
                promotion_fixture_suite.and_then(|suite| suite.get("candidate"));
            let promotion_model_route_tasks = promotion_decision
                .and_then(|decision| decision.get("metric_evidence"))
                .and_then(|evidence| evidence.get("model_route_tasks"));
            let promotion_model_route_baseline =
                promotion_model_route_tasks.and_then(|routes| routes.get("baseline"));
            let promotion_model_route_candidate =
                promotion_model_route_tasks.and_then(|routes| routes.get("candidate"));
            Ok(GraphDecisionMetadata {
                decision_id: row.get(0)?,
                decision_type: row.get(1)?,
                decision: row.get(2)?,
                status: row.get(3)?,
                patch_id: row.get(4)?,
                eval_id: row.get(5)?,
                event_id: row.get(6)?,
                suite: payload_str(&payload, "suite").map(str::to_string),
                reason: payload_str(&payload, "reason")
                    .or_else(|| payload_str(&payload, "rationale"))
                    .map(str::to_string),
                missing_required_gates: payload_string_array(&payload, "missing_required_gates")
                    .len(),
                replay_failures_after_eval: payload_u64(&payload, "replay_failures_after_eval"),
                last_eval_fixture_task_count: payload_u64(&payload, "last_eval_fixture_task_count"),
                last_eval_fixture_command_count: payload_u64(
                    &payload,
                    "last_eval_fixture_command_count",
                ),
                last_eval_fixture_risk_labels: payload_u64_count_map(
                    &payload,
                    "last_eval_fixture_risk_labels",
                ),
                last_eval_model_route_tasks: payload_u64_count_map(
                    &payload,
                    "last_eval_model_route_tasks",
                ),
                last_eval_mutation_scope_failures: payload_u64(
                    &payload,
                    "last_eval_mutation_scope_failures",
                ),
                last_eval_unexpected_changed_files: payload_u64(
                    &payload,
                    "last_eval_unexpected_changed_files",
                ),
                min_fixture_task_count: payload_u64(&payload, "min_fixture_task_count"),
                min_fixture_command_count: payload_u64(&payload, "min_fixture_command_count"),
                min_fixture_risk_labels: payload_u64_count_map(&payload, "min_fixture_risk_labels"),
                fixture_breadth_satisfied: payload_bool(&payload, "fixture_breadth_satisfied"),
                fixture_risk_satisfied: payload_bool(&payload, "fixture_risk_satisfied"),
                require_protocol: payload_bool(&payload, "require_protocol"),
                protocol_eval_status: payload_str(&payload, "protocol_eval_status")
                    .map(str::to_string),
                protocol_eval_git_dirty: payload_bool(&payload, "protocol_eval_git_dirty"),
                protocol_check_total: release_protocol_check_count(&payload, "total"),
                protocol_check_passes: release_protocol_check_count(&payload, "passes"),
                protocol_check_strict: release_protocol_check_count(&payload, "strict"),
                protocol_check_thinking: release_protocol_check_count(&payload, "thinking"),
                protocol_check_stream: release_protocol_check_count(&payload, "stream"),
                protocol_check_json: release_protocol_check_count(&payload, "json"),
                protocol_check_transport: release_protocol_check_count(&payload, "transport"),
                source_provenance_passed: payload_bool(&payload, "source_provenance_passed"),
                source_provenance_findings: payload_u64(&payload, "source_provenance_findings"),
                source_provenance_scan_source: payload_str(
                    &payload,
                    "source_provenance_scan_source",
                )
                .map(str::to_string),
                promotion_eligible: promotion_decision
                    .and_then(|decision| decision.get("eligible"))
                    .and_then(Value::as_bool),
                promotion_criterion: promotion_decision
                    .and_then(|decision| decision.get("criterion"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
                promotion_reason: promotion_decision
                    .and_then(|decision| decision.get("reason"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
                promotion_baseline_eval_id: promotion_decision
                    .and_then(|decision| decision.get("baseline_eval_id"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
                promotion_candidate_eval_id: promotion_decision
                    .and_then(|decision| decision.get("candidate_eval_id"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
                promotion_protocol_eval_id: promotion_decision
                    .and_then(|decision| decision.get("protocol_eval_id"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
                promotion_fixture_baseline_task_count: promotion_fixture_baseline
                    .and_then(|fixture| fixture.get("task_count"))
                    .and_then(payload_value_u64),
                promotion_fixture_candidate_task_count: promotion_fixture_candidate
                    .and_then(|fixture| fixture.get("task_count"))
                    .and_then(payload_value_u64),
                promotion_fixture_baseline_command_count: promotion_fixture_baseline
                    .and_then(|fixture| fixture.get("command_count"))
                    .and_then(payload_value_u64),
                promotion_fixture_candidate_command_count: promotion_fixture_candidate
                    .and_then(|fixture| fixture.get("command_count"))
                    .and_then(payload_value_u64),
                promotion_fixture_baseline_risk_labels: promotion_fixture_baseline
                    .and_then(|fixture| fixture.get("risk_labels"))
                    .map(value_u64_count_map)
                    .unwrap_or_default(),
                promotion_fixture_candidate_risk_labels: promotion_fixture_candidate
                    .and_then(|fixture| fixture.get("risk_labels"))
                    .map(value_u64_count_map)
                    .unwrap_or_default(),
                promotion_model_route_baseline: promotion_model_route_baseline
                    .map(value_u64_count_map)
                    .unwrap_or_default(),
                promotion_model_route_candidate: promotion_model_route_candidate
                    .map(value_u64_count_map)
                    .unwrap_or_default(),
            })
        })
        .map_err(|e| format!("query state graph decision metadata: {e}"))?;

    let mut out = BTreeMap::new();
    for row in rows {
        let metadata = row.map_err(|e| format!("read state graph decision metadata row: {e}"))?;
        let event_id = metadata.event_id.clone();
        out.insert(metadata.decision_id.clone(), metadata.clone());
        out.insert(event_id, metadata);
    }
    Ok(out)
}

fn query_graph_hypothesis_metadata(
    sqlite_path: &Path,
) -> Result<BTreeMap<String, GraphHypothesisMetadata>, String> {
    let conn = Connection::open(sqlite_path)
        .map_err(|e| format!("open sqlite projection '{}': {e}", sqlite_path.display()))?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT hypothesis_id, event_id, failure_event_id, summary, confidence, status, run_id
            FROM hypotheses
            ORDER BY hypothesis_id
            LIMIT 10000
            "#,
        )
        .map_err(|e| format!("prepare state graph hypothesis metadata query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(GraphHypothesisMetadata {
                hypothesis_id: row.get(0)?,
                event_id: row.get(1)?,
                failure_event_id: row.get(2)?,
                summary: row.get(3)?,
                confidence: row.get(4)?,
                status: row.get(5)?,
                run_id: row.get(6)?,
            })
        })
        .map_err(|e| format!("query state graph hypothesis metadata: {e}"))?;

    let mut out = BTreeMap::new();
    for row in rows {
        let metadata = row.map_err(|e| format!("read state graph hypothesis metadata row: {e}"))?;
        let event_id = metadata.event_id.clone();
        let failure_event_id = metadata.failure_event_id.clone();
        out.insert(metadata.hypothesis_id.clone(), metadata.clone());
        out.insert(event_id, metadata.clone());
        if let Some(failure_event_id) = failure_event_id {
            out.entry(failure_event_id).or_insert(metadata);
        }
    }
    Ok(out)
}

fn query_graph_version_metadata(
    sqlite_path: &Path,
) -> Result<BTreeMap<String, GraphVersionMetadata>, String> {
    let conn = Connection::open(sqlite_path)
        .map_err(|e| format!("open sqlite projection '{}': {e}", sqlite_path.display()))?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT event_id, event_type, run_id, payload_json
            FROM state_events
            WHERE payload_json LIKE '%harness_version%'
               OR payload_json LIKE '%base_harness_version%'
            ORDER BY timestamp_ms, event_id
            LIMIT 10000
            "#,
        )
        .map_err(|e| format!("prepare state graph version metadata query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| format!("query state graph version metadata: {e}"))?;

    let mut out = BTreeMap::new();
    for row in rows {
        let (event_id, event_type, run_id, payload_json) =
            row.map_err(|e| format!("read state graph version metadata row: {e}"))?;
        let payload = serde_json::from_str::<Value>(&payload_json).unwrap_or(Value::Null);
        let harness_version = payload_str(&payload, "harness_version").map(str::to_string);
        let base_harness_version =
            payload_str(&payload, "base_harness_version").map(str::to_string);
        if harness_version.is_none() && base_harness_version.is_none() {
            continue;
        }
        let metadata = GraphVersionMetadata {
            event_id: event_id.clone(),
            event_type,
            harness_version: harness_version.clone(),
            base_harness_version: base_harness_version.clone(),
            patch_id: payload_str(&payload, "patch_id").map(str::to_string),
            eval_id: payload_str(&payload, "eval_id").map(str::to_string),
            suite: payload_str(&payload, "suite").map(str::to_string),
            status: payload_str(&payload, "status").map(str::to_string),
            run_id,
        };
        out.insert(event_id, metadata.clone());
        if let Some(harness_version) = harness_version {
            out.entry(harness_version)
                .or_insert_with(|| metadata.clone());
        }
        if let Some(base_harness_version) = base_harness_version {
            out.entry(base_harness_version)
                .or_insert_with(|| metadata.clone());
        }
        if let Some(patch_id) = metadata.patch_id.as_deref() {
            out.entry(patch_id.to_string())
                .or_insert_with(|| metadata.clone());
        }
        if let Some(eval_id) = metadata.eval_id.as_deref() {
            out.entry(eval_id.to_string()).or_insert(metadata);
        }
    }
    Ok(out)
}

fn query_graph_run_metadata(
    sqlite_path: &Path,
) -> Result<BTreeMap<String, GraphRunMetadata>, String> {
    let conn = Connection::open(sqlite_path)
        .map_err(|e| format!("open sqlite projection '{}': {e}", sqlite_path.display()))?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT event_id, event_type, run_id, trace_id, payload_json
            FROM state_events
            WHERE run_id IS NOT NULL
               OR trace_id != ''
               OR payload_json LIKE '%task_id%'
               OR payload_json LIKE '%task_ids%'
            ORDER BY timestamp_ms, event_id
            LIMIT 10000
            "#,
        )
        .map_err(|e| format!("prepare state graph run metadata query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(|e| format!("query state graph run metadata: {e}"))?;

    let mut out = BTreeMap::new();
    for row in rows {
        let (event_id, event_type, run_id, trace_id, payload_json) =
            row.map_err(|e| format!("read state graph run metadata row: {e}"))?;
        let payload = serde_json::from_str::<Value>(&payload_json).unwrap_or(Value::Null);
        let task_ids = graph_payload_task_ids(&payload);
        let metadata = GraphRunMetadata {
            event_id: event_id.clone(),
            event_type,
            run_id: run_id.clone(),
            trace_id: trace_id.clone().filter(|trace_id| !trace_id.is_empty()),
            task_ids: task_ids.clone(),
            status: graph_run_status(&payload),
        };
        out.insert(event_id, metadata.clone());
        if let Some(run_id) = run_id {
            out.entry(run_id).or_insert_with(|| metadata.clone());
        }
        if let Some(trace_id) = trace_id.filter(|trace_id| !trace_id.is_empty()) {
            out.entry(trace_id).or_insert_with(|| metadata.clone());
        }
        for task_id in task_ids {
            out.entry(task_id).or_insert_with(|| metadata.clone());
        }
    }
    Ok(out)
}

fn query_graph_artifact_metadata(
    sqlite_path: &Path,
) -> Result<BTreeMap<String, GraphArtifactMetadata>, String> {
    let conn = Connection::open(sqlite_path)
        .map_err(|e| format!("open sqlite projection '{}': {e}", sqlite_path.display()))?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT event_id, event_type, run_id, payload_json
            FROM state_events
            WHERE payload_json LIKE '%artifact_uri%'
               OR payload_json LIKE '%artifacts%'
            ORDER BY timestamp_ms, event_id
            LIMIT 10000
            "#,
        )
        .map_err(|e| format!("prepare state graph artifact metadata query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| format!("query state graph artifact metadata: {e}"))?;

    let mut out = BTreeMap::new();
    for row in rows {
        let (event_id, event_type, run_id, payload_json) =
            row.map_err(|e| format!("read state graph artifact metadata row: {e}"))?;
        let payload = serde_json::from_str::<Value>(&payload_json).unwrap_or(Value::Null);
        let artifact_uris = graph_payload_artifact_uris(&payload);
        if artifact_uris.is_empty() {
            continue;
        }
        let reproducibility = graph_reproducibility_payload(&payload);
        let fixture_suite = payload
            .get("metrics")
            .and_then(|metrics| metrics.get("fixture_suite"))
            .or_else(|| payload.get("fixture_suite"));
        for artifact_uri in artifact_uris {
            let metadata = GraphArtifactMetadata {
                event_id: event_id.clone(),
                event_type: event_type.clone(),
                artifact_uri: artifact_uri.clone(),
                eval_id: payload_str(&payload, "eval_id").map(str::to_string),
                patch_id: payload_str(&payload, "patch_id").map(str::to_string),
                suite: payload_str(&payload, "suite").map(str::to_string),
                status: payload_str(&payload, "status").map(str::to_string),
                repro_mode: reproducibility
                    .and_then(|manifest| payload_str(manifest, "mode"))
                    .map(str::to_string),
                agent_command_source: reproducibility
                    .and_then(|manifest| payload_str(manifest, "agent_command_source"))
                    .map(str::to_string),
                replay_command: reproducibility
                    .and_then(|manifest| payload_str(manifest, "replay_command"))
                    .map(|command| preview_line(command, 120)),
                git_dirty: reproducibility.and_then(|manifest| payload_bool(manifest, "git_dirty")),
                command_count: reproducibility
                    .and_then(|manifest| manifest.get("commands"))
                    .and_then(Value::as_array)
                    .map(Vec::len)
                    .unwrap_or_default(),
                fixture_task_count: fixture_suite
                    .and_then(|suite| payload_u64(suite, "task_count")),
                fixture_command_count: fixture_suite
                    .and_then(|suite| payload_u64(suite, "command_count")),
                run_id: run_id.clone(),
            };
            out.insert(event_id.clone(), metadata.clone());
            out.insert(artifact_uri, metadata.clone());
            if let Some(eval_id) = metadata.eval_id.as_deref() {
                out.entry(eval_id.to_string())
                    .or_insert_with(|| metadata.clone());
            }
            if let Some(patch_id) = metadata.patch_id.as_deref() {
                out.entry(patch_id.to_string()).or_insert(metadata);
            }
        }
    }
    Ok(out)
}

fn query_graph_model_metadata(
    sqlite_path: &Path,
) -> Result<BTreeMap<String, GraphModelMetadata>, String> {
    let conn = Connection::open(sqlite_path)
        .map_err(|e| format!("open sqlite projection '{}': {e}", sqlite_path.display()))?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT event_id, event_type, run_id, payload_json
            FROM state_events
            WHERE event_type IN ('ModelCallStarted', 'ModelCallCompleted')
            ORDER BY timestamp_ms, event_id
            LIMIT 10000
            "#,
        )
        .map_err(|e| format!("prepare state graph model metadata query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| format!("query state graph model metadata: {e}"))?;

    let mut out = BTreeMap::new();
    for row in rows {
        let (event_id, event_type, run_id, payload_json) =
            row.map_err(|e| format!("read state graph model metadata row: {e}"))?;
        let payload = serde_json::from_str::<Value>(&payload_json).unwrap_or(Value::Null);
        let model_call_id = payload_str(&payload, "model_call_id")
            .unwrap_or(&event_id)
            .to_string();
        let metadata = GraphModelMetadata {
            event_id: event_id.clone(),
            event_type,
            model_call_id: model_call_id.clone(),
            model: payload_str(&payload, "model").map(str::to_string),
            route_task: graph_model_route_task(&payload),
            thinking: graph_model_payload_label(&payload, "thinking"),
            reasoning_effort: graph_model_payload_label(&payload, "reasoning_effort"),
            input_tokens: payload_u64(&payload, "input_tokens"),
            output_tokens: payload_u64(&payload, "output_tokens"),
            cache_read_tokens: payload_u64(&payload, "cache_read_tokens"),
            cache_write_tokens: payload_u64(&payload, "cache_write_tokens"),
            run_id,
        };
        out.insert(event_id, metadata.clone());
        out.insert(model_call_id, metadata);
    }
    Ok(out)
}

fn graph_model_route_task(payload: &Value) -> Option<String> {
    [
        "route_task",
        "task",
        "deepseek_route_task",
        "routing_task",
        "model_route_task",
    ]
    .iter()
    .filter_map(|key| payload.get(*key).and_then(Value::as_str))
    .map(|text| text.trim().replace([' ', '-'], "_").to_ascii_lowercase())
    .find(|text| !text.is_empty())
}

fn query_graph_tool_metadata(
    sqlite_path: &Path,
) -> Result<BTreeMap<String, GraphToolMetadata>, String> {
    let conn = Connection::open(sqlite_path)
        .map_err(|e| format!("open sqlite projection '{}': {e}", sqlite_path.display()))?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT event_id, event_type, run_id, payload_json
            FROM state_events
            WHERE event_type IN ('ToolCallStarted', 'ToolCallCompleted')
            ORDER BY timestamp_ms, event_id
            LIMIT 10000
            "#,
        )
        .map_err(|e| format!("prepare state graph tool metadata query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| format!("query state graph tool metadata: {e}"))?;

    let mut out = BTreeMap::new();
    for row in rows {
        let (event_id, event_type, run_id, payload_json) =
            row.map_err(|e| format!("read state graph tool metadata row: {e}"))?;
        let payload = serde_json::from_str::<Value>(&payload_json).unwrap_or(Value::Null);
        let tool_call_id = payload_str(&payload, "tool_call_id")
            .unwrap_or(&event_id)
            .to_string();
        let metadata = GraphToolMetadata {
            event_id: event_id.clone(),
            event_type,
            tool_call_id: tool_call_id.clone(),
            tool_name: payload_str(&payload, "tool_name").map(str::to_string),
            status: graph_tool_status(&payload),
            result_preview: payload_str(&payload, "result_preview")
                .or_else(|| payload_str(&payload, "error_preview"))
                .map(|text| preview_line(text, 80)),
            args_preview: graph_tool_args_preview(&payload),
            run_id,
        };
        out.insert(event_id, metadata.clone());
        out.insert(tool_call_id, metadata);
    }
    Ok(out)
}

fn query_graph_command_metadata(
    sqlite_path: &Path,
) -> Result<BTreeMap<String, GraphCommandMetadata>, String> {
    let conn = Connection::open(sqlite_path)
        .map_err(|e| format!("open sqlite projection '{}': {e}", sqlite_path.display()))?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT event_id, event_type, run_id, payload_json
            FROM state_events
            WHERE event_type IN ('CommandStarted', 'CommandCompleted')
            ORDER BY timestamp_ms, event_id
            LIMIT 10000
            "#,
        )
        .map_err(|e| format!("prepare state graph command metadata query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| format!("query state graph command metadata: {e}"))?;

    let mut out = BTreeMap::new();
    for row in rows {
        let (event_id, event_type, run_id, payload_json) =
            row.map_err(|e| format!("read state graph command metadata row: {e}"))?;
        let payload = serde_json::from_str::<Value>(&payload_json).unwrap_or(Value::Null);
        out.insert(
            event_id.clone(),
            GraphCommandMetadata {
                event_id,
                event_type,
                command: payload_str(&payload, "command").map(|command| preview_line(command, 120)),
                status: graph_command_status(&payload),
                result_preview: payload_str(&payload, "result_preview")
                    .or_else(|| payload_str(&payload, "error_preview"))
                    .map(|text| preview_line(text, 80)),
                run_id,
            },
        );
    }
    Ok(out)
}

fn query_graph_test_metadata(
    sqlite_path: &Path,
) -> Result<BTreeMap<String, GraphTestMetadata>, String> {
    let conn = Connection::open(sqlite_path)
        .map_err(|e| format!("open sqlite projection '{}': {e}", sqlite_path.display()))?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT event_id, event_type, run_id, payload_json
            FROM state_events
            WHERE event_type IN ('TestStarted', 'TestCompleted')
            ORDER BY timestamp_ms, event_id
            LIMIT 10000
            "#,
        )
        .map_err(|e| format!("prepare state graph test metadata query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| format!("query state graph test metadata: {e}"))?;

    let mut out = BTreeMap::new();
    for row in rows {
        let (event_id, event_type, run_id, payload_json) =
            row.map_err(|e| format!("read state graph test metadata row: {e}"))?;
        let payload = serde_json::from_str::<Value>(&payload_json).unwrap_or(Value::Null);
        out.insert(
            event_id.clone(),
            GraphTestMetadata {
                event_id,
                event_type,
                test_kind: payload_str(&payload, "test_kind").map(str::to_string),
                command: payload_str(&payload, "command").map(|command| preview_line(command, 120)),
                status: graph_test_status(&payload),
                result_preview: payload_str(&payload, "result_preview")
                    .or_else(|| payload_str(&payload, "error_preview"))
                    .map(|text| preview_line(text, 80)),
                run_id,
            },
        );
    }
    Ok(out)
}

fn query_graph_commit_metadata(
    sqlite_path: &Path,
) -> Result<BTreeMap<String, GraphCommitMetadata>, String> {
    let conn = Connection::open(sqlite_path)
        .map_err(|e| format!("open sqlite projection '{}': {e}", sqlite_path.display()))?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT event_id, event_type, run_id, payload_json
            FROM state_events
            WHERE event_type IN ('CommitCreated', 'RevertPerformed')
            ORDER BY timestamp_ms, event_id
            LIMIT 10000
            "#,
        )
        .map_err(|e| format!("prepare state graph commit metadata query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| format!("query state graph commit metadata: {e}"))?;

    let mut out = BTreeMap::new();
    for row in rows {
        let (event_id, event_type, run_id, payload_json) =
            row.map_err(|e| format!("read state graph commit metadata row: {e}"))?;
        let payload = serde_json::from_str::<Value>(&payload_json).unwrap_or(Value::Null);
        let commit = payload_str(&payload, "commit")
            .or_else(|| payload_str(&payload, "commit_id"))
            .map(str::to_string);
        let metadata = GraphCommitMetadata {
            event_id: event_id.clone(),
            event_type,
            commit: commit.clone(),
            reverted_commit: payload_str(&payload, "reverted_commit").map(str::to_string),
            branch: payload_str(&payload, "branch").map(str::to_string),
            message: payload_str(&payload, "message")
                .or_else(|| payload_str(&payload, "summary"))
                .map(|message| preview_line(message, 80)),
            reason: payload_str(&payload, "reason").map(|reason| preview_line(reason, 80)),
            files: graph_commit_payload_files(&payload),
            run_id,
        };
        out.insert(event_id, metadata.clone());
        if let Some(commit) = metadata.commit.as_deref() {
            out.insert(commit.to_string(), metadata.clone());
        }
        if let Some(reverted_commit) = metadata.reverted_commit.as_deref() {
            out.entry(reverted_commit.to_string())
                .or_insert_with(|| metadata.clone());
        }
    }
    Ok(out)
}

fn graph_commit_payload_files(payload: &Value) -> Vec<String> {
    let mut files = BTreeSet::new();
    for key in ["files", "modified_files", "paths", "file_paths"] {
        files.extend(payload_string_array(payload, key));
    }
    if let Some(path) = payload_str(payload, "path") {
        files.insert(path.to_string());
    }
    files.into_iter().collect()
}

fn query_graph_memory_metadata(
    sqlite_path: &Path,
) -> Result<BTreeMap<String, GraphMemoryMetadata>, String> {
    let conn = Connection::open(sqlite_path)
        .map_err(|e| format!("open sqlite projection '{}': {e}", sqlite_path.display()))?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT event_id, event_type, run_id, payload_json
            FROM state_events
            WHERE event_type IN ('MemoryProposed', 'MemoryPromoted', 'MemoryRejected')
            ORDER BY timestamp_ms, event_id
            LIMIT 10000
            "#,
        )
        .map_err(|e| format!("prepare state graph memory metadata query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| format!("query state graph memory metadata: {e}"))?;

    let mut out = BTreeMap::new();
    for row in rows {
        let (event_id, event_type, run_id, payload_json) =
            row.map_err(|e| format!("read state graph memory metadata row: {e}"))?;
        let payload = serde_json::from_str::<Value>(&payload_json).unwrap_or(Value::Null);
        let Some(candidate_id) = payload_str(&payload, "candidate_id") else {
            continue;
        };
        let metadata = GraphMemoryMetadata {
            event_id: event_id.clone(),
            event_type,
            candidate_id: candidate_id.to_string(),
            status: payload_str(&payload, "status").map(str::to_string),
            source: payload_str(&payload, "source").map(str::to_string),
            summary: payload_str(&payload, "summary").map(|summary| preview_line(summary, 80)),
            reason: payload_str(&payload, "reason").map(|reason| preview_line(reason, 80)),
            proposed_event_id: payload_str(&payload, "proposed_event_id").map(str::to_string),
            evidence_event_ids: payload_string_array(&payload, "evidence_event_ids"),
            run_id,
        };
        out.insert(event_id, metadata.clone());
        out.insert(candidate_id.to_string(), metadata);
    }
    Ok(out)
}

fn query_graph_issue_metadata(
    sqlite_path: &Path,
) -> Result<BTreeMap<String, GraphIssueMetadata>, String> {
    let conn = Connection::open(sqlite_path)
        .map_err(|e| format!("open sqlite projection '{}': {e}", sqlite_path.display()))?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT event_id, run_id, payload_json
            FROM state_events
            WHERE event_type = 'PatchProposed'
            ORDER BY timestamp_ms, event_id
            LIMIT 10000
            "#,
        )
        .map_err(|e| format!("prepare state graph issue metadata query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| format!("query state graph issue metadata: {e}"))?;

    let mut out = BTreeMap::new();
    for row in rows {
        let (event_id, run_id, payload_json) =
            row.map_err(|e| format!("read state graph issue metadata row: {e}"))?;
        let payload = serde_json::from_str::<Value>(&payload_json).unwrap_or(Value::Null);
        if payload_str(&payload, "intake_kind") != Some("issue") {
            continue;
        }
        let patch_id = payload_str(&payload, "patch_id").map(str::to_string);
        let issue_id = payload_str(&payload, "issue_id")
            .map(str::to_string)
            .or_else(|| {
                patch_id
                    .as_ref()
                    .map(|patch_id| format!("issue:{patch_id}"))
            })
            .unwrap_or_else(|| format!("issue:{event_id}"));
        let metadata = GraphIssueMetadata {
            event_id: event_id.clone(),
            issue_id: issue_id.clone(),
            patch_id,
            intake_source: payload_str(&payload, "intake_source").map(str::to_string),
            intake_kind: payload_str(&payload, "intake_kind").map(str::to_string),
            summary: payload_str(&payload, "intake_summary")
                .or_else(|| payload_str(&payload, "intent"))
                .map(|summary| preview_line(summary, 80)),
            details: payload_str(&payload, "intake_details")
                .map(|details| preview_line(details, 80)),
            kind: payload_str(&payload, "kind").map(str::to_string),
            risk_level: payload_str(&payload, "risk_level").map(str::to_string),
            status: payload_str(&payload, "status").map(str::to_string),
            run_id,
        };
        out.insert(event_id, metadata.clone());
        out.insert(issue_id, metadata.clone());
        if let Some(patch_id) = metadata.patch_id.as_deref() {
            out.insert(patch_id.to_string(), metadata);
        }
    }
    Ok(out)
}

fn graph_payload_task_ids(payload: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(task_id) = payload_str(payload, "task_id") {
        push_unique_string(&mut out, task_id);
    }
    for task_id in payload_string_array(payload, "task_ids") {
        push_unique_string(&mut out, &task_id);
    }
    graph_collect_fixture_task_ids(payload, &mut out);
    if let Some(metrics) = payload.get("metrics") {
        graph_collect_fixture_task_ids(metrics, &mut out);
    }
    out
}

fn graph_collect_fixture_task_ids(value: &Value, out: &mut Vec<String>) {
    for key in ["fixture_tasks", "fixture_results"] {
        if let Some(items) = value.get(key).and_then(|items| items.as_array()) {
            for item in items {
                if let Some(task_id) = payload_str(item, "task_id") {
                    push_unique_string(out, task_id);
                }
            }
        }
    }
}

fn graph_payload_fixture_agent_changed_files(payload: &Value) -> Vec<String> {
    let mut out = Vec::new();
    graph_collect_fixture_agent_files(payload, "changed_files", &mut out);
    if let Some(metrics) = payload.get("metrics") {
        graph_collect_fixture_agent_files(metrics, "changed_files", &mut out);
    }
    out
}

fn graph_payload_fixture_agent_unexpected_files(payload: &Value) -> Vec<String> {
    let mut out = Vec::new();
    graph_collect_fixture_agent_files(payload, "unexpected_changed_files", &mut out);
    if let Some(metrics) = payload.get("metrics") {
        graph_collect_fixture_agent_files(metrics, "unexpected_changed_files", &mut out);
    }
    out
}

fn graph_collect_fixture_agent_files(value: &Value, key: &str, out: &mut Vec<String>) {
    let Some(attempts) = value
        .get("fixture_agent_attempts")
        .and_then(|items| items.as_array())
    else {
        return;
    };
    for attempt in attempts {
        let Some(files) = attempt.get(key).and_then(|items| items.as_array()) else {
            continue;
        };
        for file in files {
            if let Some(path) = file.as_str() {
                push_unique_string(out, path);
            }
        }
    }
}

fn push_unique_string(out: &mut Vec<String>, value: &str) {
    let value = value.trim();
    if !value.is_empty() && !out.iter().any(|existing| existing == value) {
        out.push(value.to_string());
    }
}

fn graph_payload_artifact_uris(payload: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(uri) = payload_str(payload, "artifact_uri") {
        push_unique_string(&mut out, uri);
    }
    if let Some(metrics) = payload.get("metrics") {
        if let Some(uri) = payload_str(metrics, "artifact_uri") {
            push_unique_string(&mut out, uri);
        }
        if let Some(items) = metrics.get("artifacts").and_then(Value::as_array) {
            for item in items {
                if let Some(uri) = payload_str(item, "uri") {
                    push_unique_string(&mut out, uri);
                }
            }
        }
    }
    out
}

fn graph_reproducibility_payload(payload: &Value) -> Option<&Value> {
    payload.get("reproducibility").or_else(|| {
        payload
            .get("metrics")
            .and_then(|metrics| metrics.get("reproducibility"))
    })
}

fn graph_run_status(payload: &Value) -> Option<String> {
    payload_str(payload, "status")
        .map(str::to_string)
        .or_else(|| {
            payload_bool(payload, "is_error").map(|is_error| {
                if is_error {
                    "error".to_string()
                } else {
                    "ok".to_string()
                }
            })
        })
        .or_else(|| {
            payload_bool(payload, "passed").map(|passed| {
                if passed {
                    "passed".to_string()
                } else {
                    "failed".to_string()
                }
            })
        })
}

fn graph_test_status(payload: &Value) -> Option<String> {
    payload_str(payload, "status")
        .map(str::to_string)
        .or_else(|| {
            payload_bool(payload, "passed").map(|passed| {
                if passed {
                    "passed".to_string()
                } else {
                    "failed".to_string()
                }
            })
        })
}

fn graph_command_status(payload: &Value) -> Option<String> {
    payload_str(payload, "status")
        .map(str::to_string)
        .or_else(|| {
            payload_bool(payload, "is_error").map(|is_error| {
                if is_error {
                    "error".to_string()
                } else {
                    "ok".to_string()
                }
            })
        })
        .or_else(|| {
            payload_bool(payload, "passed").map(|passed| {
                if passed {
                    "passed".to_string()
                } else {
                    "failed".to_string()
                }
            })
        })
}

fn graph_tool_status(payload: &Value) -> Option<String> {
    payload_str(payload, "status")
        .map(str::to_string)
        .or_else(|| {
            payload_bool(payload, "is_error").map(|is_error| {
                if is_error {
                    "error".to_string()
                } else {
                    "ok".to_string()
                }
            })
        })
        .or_else(|| payload_bool(payload, "ok").map(|ok| if ok { "ok" } else { "error" }.into()))
}

fn graph_tool_args_preview(payload: &Value) -> Option<String> {
    payload
        .get("args")
        .or_else(|| payload.get("arguments"))
        .map(tool_arg_summary)
        .filter(|summary| !summary.is_empty())
}

fn graph_model_payload_label(payload: &Value, key: &str) -> Option<String> {
    let value = payload.get(key)?;
    value
        .as_str()
        .map(|text| text.to_string())
        .or_else(|| value.as_bool().map(|value| value.to_string()))
        .or_else(|| {
            value
                .get("type")
                .and_then(Value::as_str)
                .map(|text| text.to_string())
        })
}

fn query_graph_cache_metadata(
    sqlite_path: &Path,
) -> Result<BTreeMap<String, GraphCacheMetadata>, String> {
    let conn = Connection::open(sqlite_path)
        .map_err(|e| format!("open sqlite projection '{}': {e}", sqlite_path.display()))?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT event_id, model, prompt_cache_hit_tokens, prompt_cache_miss_tokens,
                   cache_hit_ratio, timestamp_ms
            FROM cache_metrics
            ORDER BY timestamp_ms, event_id
            LIMIT 10000
            "#,
        )
        .map_err(|e| format!("prepare state graph cache metadata query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(GraphCacheMetadata {
                event_id: row.get(0)?,
                model: row.get(1)?,
                prompt_cache_hit_tokens: row.get(2)?,
                prompt_cache_miss_tokens: row.get(3)?,
                cache_hit_ratio: row.get(4)?,
                timestamp_ms: row.get(5)?,
            })
        })
        .map_err(|e| format!("query state graph cache metadata: {e}"))?;

    let mut out = BTreeMap::new();
    for row in rows {
        let metadata = row.map_err(|e| format!("read state graph cache metadata row: {e}"))?;
        out.insert(metadata.event_id.clone(), metadata);
    }
    Ok(out)
}

fn query_graph_failure_metadata(
    sqlite_path: &Path,
) -> Result<BTreeMap<String, GraphFailureMetadata>, String> {
    let conn = Connection::open(sqlite_path)
        .map_err(|e| format!("open sqlite projection '{}': {e}", sqlite_path.display()))?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT event_id, event_type, source, error_preview, run_id, timestamp_ms, payload_json
            FROM failures
            ORDER BY timestamp_ms, event_id
            LIMIT 10000
            "#,
        )
        .map_err(|e| format!("prepare state graph failure metadata query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
            ))
        })
        .map_err(|e| format!("query state graph failure metadata: {e}"))?;

    let mut out = BTreeMap::new();
    for row in rows {
        let (event_id, event_type, source, error_preview, run_id, timestamp_ms, payload_json) =
            row.map_err(|e| format!("read state graph failure metadata row: {e}"))?;
        let payload = serde_json::from_str::<Value>(&payload_json).unwrap_or(Value::Null);
        let event = json_event_for_failure_taxonomy(&event_id, &event_type, &run_id, &payload);
        let taxonomy = classify_failure_event(&event);
        out.insert(
            event_id.clone(),
            GraphFailureMetadata {
                event_id,
                event_type,
                class: taxonomy.class.to_string(),
                owner: taxonomy.owner.to_string(),
                retryable: taxonomy.retryable,
                source,
                error_preview,
                run_id,
                timestamp_ms,
            },
        );
    }
    Ok(out)
}

fn json_event_for_failure_taxonomy(
    event_id: &str,
    event_type: &str,
    run_id: &Option<String>,
    payload: &Value,
) -> Value {
    serde_json::json!({
        "event_id": event_id,
        "event_type": event_type,
        "run_id": run_id,
        "payload": payload,
    })
}

fn query_graph_policy_metadata(
    sqlite_path: &Path,
) -> Result<BTreeMap<String, GraphPolicyMetadata>, String> {
    let conn = Connection::open(sqlite_path)
        .map_err(|e| format!("open sqlite projection '{}': {e}", sqlite_path.display()))?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT event_id, event_type, run_id, payload_json
            FROM state_events
            ORDER BY timestamp_ms, event_id
            LIMIT 10000
            "#,
        )
        .map_err(|e| format!("prepare state graph policy metadata query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| format!("query state graph policy metadata: {e}"))?;

    let mut out = BTreeMap::new();
    for row in rows {
        let (event_id, event_type, run_id, payload_json) =
            row.map_err(|e| format!("read state graph policy metadata row: {e}"))?;
        let payload = serde_json::from_str::<Value>(&payload_json).unwrap_or(Value::Null);
        if let Some(metadata) =
            graph_policy_metadata_from_payload(event_id.clone(), event_type, run_id, &payload)
        {
            out.insert(event_id, metadata);
        }
    }
    Ok(out)
}

fn graph_policy_metadata_from_payload(
    event_id: String,
    event_type: String,
    run_id: Option<String>,
    payload: &Value,
) -> Option<GraphPolicyMetadata> {
    let context_policy = payload_str(payload, "context_policy").map(str::to_string);
    let prompt_layout = payload_scalar_id(payload, "layout_version")
        .or_else(|| payload_scalar_id(payload, "prompt_layout_version"))
        .map(|version| format!("prompt_layout_v{version}"));
    let prompt_version = payload_prompt_version_id(payload);
    let schema_name = payload_str(payload, "schema_name")
        .or_else(|| payload_str(payload, "tool_name"))
        .map(str::to_string);
    let schema_version = payload_scalar_id(payload, "schema_version");
    let source_provenance_scan_source =
        payload_str(payload, "source_provenance_scan_source").map(str::to_string);
    let source_provenance_passed = payload_bool(payload, "source_provenance_passed");
    let source_provenance_findings = payload_u64(payload, "source_provenance_findings");
    let stable_blocks = payload_string_array(payload, "stable_prefix_blocks").len();
    let dynamic_blocks = payload_string_array(payload, "dynamic_suffix_blocks").len();
    let included_blocks = context_included_block_names(payload);
    let instruction_files = payload_string_array(payload, "include_instruction_files");

    if context_policy.is_none()
        && prompt_layout.is_none()
        && prompt_version.is_none()
        && schema_name.is_none()
        && schema_version.is_none()
        && source_provenance_scan_source.is_none()
        && source_provenance_passed.is_none()
        && source_provenance_findings.is_none()
        && stable_blocks == 0
        && dynamic_blocks == 0
        && included_blocks.is_empty()
        && instruction_files.is_empty()
    {
        return None;
    }

    Some(GraphPolicyMetadata {
        event_id,
        event_type,
        run_id,
        context_policy,
        prompt_layout,
        prompt_version,
        schema_name,
        schema_version,
        source_provenance_scan_source,
        source_provenance_passed,
        source_provenance_findings,
        stable_blocks,
        dynamic_blocks,
        included_blocks,
        instruction_files,
    })
}

fn payload_scalar_id(payload: &Value, key: &str) -> Option<String> {
    match payload.get(key)? {
        Value::String(text) if !text.trim().is_empty() => Some(text.trim().to_string()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

fn payload_prompt_version_id(payload: &Value) -> Option<String> {
    if let Some(prompt_version) = payload_scalar_id(payload, "prompt_version") {
        return Some(prompt_version);
    }
    payload_scalar_id(payload, "prompt_layout_version")
        .or_else(|| payload_scalar_id(payload, "layout_version"))
        .map(|version| format!("prompt_layout_v{version}"))
}

fn query_graph_timeline(
    sqlite_path: &Path,
    id: &str,
    depth: usize,
    limit: usize,
) -> Result<Vec<GraphTimelineStep>, String> {
    let max_depth = depth.clamp(1, 4);
    let limit = limit.clamp(1, 100);
    let conn = Connection::open(sqlite_path)
        .map_err(|e| format!("open sqlite projection '{}': {e}", sqlite_path.display()))?;
    let mut queue = VecDeque::from([(id.to_string(), 0usize)]);
    let mut visited_nodes = BTreeSet::from([id.to_string()]);
    let mut seen_relations = BTreeSet::new();
    let mut steps = Vec::new();

    while let Some((node, node_depth)) = queue.pop_front() {
        if node_depth >= max_depth {
            continue;
        }
        let relation_depth = node_depth + 1;
        for step in query_graph_timeline_node(&conn, &node, relation_depth)? {
            let key = (
                step.src_id.clone(),
                step.relation.clone(),
                step.dst_id.clone(),
                step.event_id.clone(),
            );
            if seen_relations.insert(key) {
                for neighbor in [&step.src_id, &step.dst_id] {
                    if visited_nodes.insert(neighbor.clone()) {
                        queue.push_back((neighbor.clone(), relation_depth));
                    }
                }
                steps.push(step);
            }
        }
    }

    steps.sort_by(|a, b| {
        a.timestamp_ms
            .cmp(&b.timestamp_ms)
            .then_with(|| a.depth.cmp(&b.depth))
            .then_with(|| a.event_id.cmp(&b.event_id))
            .then_with(|| a.relation.cmp(&b.relation))
            .then_with(|| a.src_id.cmp(&b.src_id))
            .then_with(|| a.dst_id.cmp(&b.dst_id))
    });
    steps.truncate(limit);
    Ok(steps)
}

fn query_graph_timeline_node(
    conn: &Connection,
    id: &str,
    depth: usize,
) -> Result<Vec<GraphTimelineStep>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                r.src_id,
                r.relation,
                r.dst_id,
                r.dst_kind,
                r.event_id,
                COALESCE(e.timestamp_ms, 0),
                COALESCE(e.event_type, 'unknown')
            FROM state_relations r
            LEFT JOIN state_events e ON e.event_id = r.event_id
            WHERE r.src_id = ?1 OR r.dst_id = ?1
            ORDER BY COALESCE(e.timestamp_ms, 0), r.event_id, r.relation, r.src_id, r.dst_id
            LIMIT 500
            "#,
        )
        .map_err(|e| format!("prepare state graph timeline query: {e}"))?;
    let rows = stmt
        .query_map(params![id], |row| {
            Ok(GraphTimelineStep {
                depth,
                src_id: row.get(0)?,
                relation: row.get(1)?,
                dst_id: row.get(2)?,
                dst_kind: row.get(3)?,
                event_id: row.get(4)?,
                timestamp_ms: row.get(5)?,
                event_type: row.get(6)?,
            })
        })
        .map_err(|e| format!("query state graph timeline: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| format!("read state graph timeline row: {e}"))?);
    }
    Ok(out)
}

fn query_graph_path(
    sqlite_path: &Path,
    from: &str,
    to: &str,
    depth: usize,
) -> Result<Vec<GraphPathStep>, String> {
    if from == to {
        return Ok(Vec::new());
    }
    let max_depth = depth.clamp(1, 4);
    let mut queue = VecDeque::from([(from.to_string(), 0usize)]);
    let mut visited_nodes = BTreeSet::from([from.to_string()]);
    let mut parents: BTreeMap<String, (String, crate::state::StateRelation)> = BTreeMap::new();

    while let Some((node, node_depth)) = queue.pop_front() {
        if node_depth >= max_depth {
            continue;
        }
        for relation in crate::state::query_sqlite_relations(sqlite_path, &node)? {
            let neighbors = [&relation.src_id, &relation.dst_id];
            for neighbor in neighbors {
                if neighbor == &node {
                    continue;
                }
                if !visited_nodes.insert(neighbor.clone()) {
                    continue;
                }
                parents.insert(neighbor.clone(), (node.clone(), relation.clone()));
                if neighbor == to {
                    return reconstruct_graph_path(from, to, &parents);
                }
                queue.push_back((neighbor.clone(), node_depth + 1));
            }
        }
    }

    Ok(Vec::new())
}

fn reconstruct_graph_path(
    from: &str,
    to: &str,
    parents: &BTreeMap<String, (String, crate::state::StateRelation)>,
) -> Result<Vec<GraphPathStep>, String> {
    let mut cursor = to.to_string();
    let mut reversed = Vec::new();
    while cursor != from {
        let Some((parent, relation)) = parents.get(&cursor) else {
            return Err(format!("incomplete graph path from '{from}' to '{to}'"));
        };
        reversed.push(GraphPathStep {
            depth: 0,
            from_id: parent.clone(),
            to_id: cursor.clone(),
            relation: relation.clone(),
        });
        cursor = parent.clone();
    }
    reversed.reverse();
    for (idx, step) in reversed.iter_mut().enumerate() {
        step.depth = idx + 1;
    }
    Ok(reversed)
}

pub(crate) fn infer_graph_node_kind(id: &str) -> String {
    if id.starts_with("event_") || id.starts_with("evt-") {
        "event"
    } else if id.starts_with("patch-") {
        "patch"
    } else if id.starts_with("eval-") {
        "eval"
    } else if id.starts_with("run-") {
        "run"
    } else if id.starts_with("trace-") {
        "trace"
    } else if id.starts_with("hyp-") {
        "hypothesis"
    } else if id.starts_with("decision-") || id.starts_with("dec-") {
        "decision"
    } else if id.starts_with("source_provenance_finding:")
        || id.starts_with("required_gate:")
        || id.starts_with("deepseek_protocol_check:")
        || id == "release_fixture_breadth_minimum"
        || id == "release_fixture_risk_minimum"
        || id == "release_fixture_agent_mutation_scope"
        || id == "promotion_fixture_risk_coverage"
    {
        "evidence"
    } else if id.starts_with("source_provenance_scan:") || id == "release_source_provenance_audit" {
        "policy"
    } else if id.starts_with("issue:") {
        "issue"
    } else if id.contains("/artifacts/") || id.starts_with("artifact:") {
        "artifact"
    } else if id.contains('/') || id.ends_with(".rs") || id.ends_with(".md") {
        "file"
    } else {
        "unknown"
    }
    .to_string()
}

fn build_eval_report(
    events: &[Value],
    harness_version: Option<&str>,
    patch_id: Option<&str>,
) -> Result<String, String> {
    let mut rows = Vec::new();
    for event in events {
        let Some(payload) = event.get("payload") else {
            continue;
        };
        let typed = serde_json::from_value::<EvalResult>(payload.clone()).ok();
        if typed.is_none() && !is_eval_event(event, payload) {
            continue;
        }

        let eval_id = typed
            .as_ref()
            .map(|eval| eval.eval_id.as_str())
            .or_else(|| payload.get("eval_id").and_then(|v| v.as_str()))
            .unwrap_or("-");
        let version = typed
            .as_ref()
            .map(|eval| eval.harness_version.as_str())
            .or_else(|| payload.get("harness_version").and_then(|v| v.as_str()))
            .unwrap_or("-");
        let patch = typed
            .as_ref()
            .and_then(|eval| eval.patch_id.as_deref())
            .or_else(|| payload.get("patch_id").and_then(|v| v.as_str()))
            .unwrap_or("-");

        if harness_version
            .map(|filter| filter != version)
            .unwrap_or(false)
        {
            continue;
        }
        if patch_id.map(|filter| filter != patch).unwrap_or(false) {
            continue;
        }

        let suite = typed
            .as_ref()
            .map(|eval| eval.suite.as_str())
            .or_else(|| payload.get("suite").and_then(|v| v.as_str()))
            .unwrap_or("-");
        let status = typed
            .as_ref()
            .map(|eval| serialized_label(&eval.status))
            .or_else(|| {
                payload
                    .get("status")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .or_else(|| {
                payload
                    .get("passed")
                    .and_then(|v| v.as_bool())
                    .map(|passed| if passed { "passed" } else { "failed" }.to_string())
            })
            .unwrap_or_else(|| "-".to_string());
        let score = typed
            .as_ref()
            .and_then(|eval| eval.score)
            .or_else(|| payload.get("score").and_then(|v| v.as_f64()));
        let artifact = typed
            .as_ref()
            .and_then(eval_artifact_uri)
            .or_else(|| eval_artifact_uri_from_payload(payload));
        let reproducibility = eval_reproducibility_detail(typed.as_ref(), payload);
        let fixture_suite = eval_fixture_suite_detail(typed.as_ref(), payload);
        let pass_fail = eval_pass_fail_detail(typed.as_ref(), payload);
        let failure_ids = eval_failure_ids_detail(typed.as_ref(), payload);

        rows.push(format!(
            "  {:<18} {:<16} {:<12} {:<10} {:<10} {}{}{}{}{}{}",
            eval_id,
            version,
            patch,
            suite,
            status,
            score
                .map(|value| format!("score={value:.3}"))
                .unwrap_or_else(|| "score=-".to_string()),
            pass_fail,
            failure_ids,
            artifact
                .map(|uri| format!(" artifact={uri}"))
                .unwrap_or_default(),
            fixture_suite,
            reproducibility
        ));
    }

    if rows.is_empty() {
        return Err("no eval results found".to_string());
    }

    let mut out = String::new();
    out.push_str("State evals\n");
    for row in rows {
        out.push_str(&row);
        out.push('\n');
    }
    Ok(out.trim_end().to_string())
}

fn build_cache_recent_report(events: &[Value], limit: usize) -> Result<String, String> {
    let recent = events
        .iter()
        .rev()
        .filter(|event| event_string(event, "event_type") == Some("CacheMetricsRecorded"))
        .take(limit)
        .collect::<Vec<_>>();

    if recent.is_empty() {
        return Err("no DeepSeek cache metrics found".to_string());
    }

    let mut hit_tokens = 0u64;
    let mut miss_tokens = 0u64;
    let mut rows = Vec::new();
    for event in &recent {
        let payload = event.get("payload").unwrap_or(&Value::Null);
        let hit = payload
            .get("prompt_cache_hit_tokens")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        let miss = payload
            .get("prompt_cache_miss_tokens")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        hit_tokens += hit;
        miss_tokens += miss;

        let ratio = payload
            .get("cache_hit_ratio")
            .and_then(|value| value.as_f64())
            .unwrap_or_else(|| cache_ratio(hit, miss));
        rows.push(format!(
            "  {} model={} hit={} miss={} ratio={} run={} trace={}",
            event_string(event, "event_id").unwrap_or("-"),
            payload
                .get("model")
                .and_then(|value| value.as_str())
                .unwrap_or("-"),
            hit,
            miss,
            format_percent(ratio),
            event_string(event, "run_id").unwrap_or("-"),
            event_string(event, "trace_id").unwrap_or("-")
        ));
    }

    let ratio = cache_ratio(hit_tokens, miss_tokens);
    let mut out = String::new();
    out.push_str("State cache metrics\n");
    out.push_str(&format!("  recent events: {}\n", recent.len()));
    out.push_str(&format!("  hit tokens:    {hit_tokens}\n"));
    out.push_str(&format!("  miss tokens:   {miss_tokens}\n"));
    out.push_str(&format!("  hit ratio:     {}\n", format_percent(ratio)));
    out.push_str("\nRecent cache events\n");
    for row in rows {
        out.push_str(&row);
        out.push('\n');
    }
    Ok(out.trim_end().to_string())
}

fn build_state_journal_draft(events: &[Value]) -> Result<String, String> {
    if events.is_empty() {
        return Err("no state events available for journal generation".to_string());
    }

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut runs: BTreeMap<String, String> = BTreeMap::new();
    let mut timeline = Vec::new();
    let mut failures = Vec::new();
    let mut patch_outcomes = Vec::new();
    let mut evals = Vec::new();
    let mut decisions = Vec::new();

    for event in events {
        let kind = event_string(event, "event_type").unwrap_or("Unknown");
        *counts.entry(kind.to_string()).or_default() += 1;
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        if let Some(run_id) = event_string(event, "run_id") {
            if kind == "RunStarted" {
                runs.entry(run_id.to_string())
                    .or_insert_with(|| "started".to_string());
            } else if kind == "RunCompleted" {
                runs.insert(
                    run_id.to_string(),
                    payload_str(&payload, "status")
                        .unwrap_or("completed")
                        .to_string(),
                );
            }
        }

        match kind {
            "FailureObserved" | "JsonOutputFailure" | "ToolSchemaFailure" => {
                let source = payload_str(&payload, "source")
                    .or_else(|| payload_str(&payload, "operation"))
                    .unwrap_or("unknown");
                let preview = payload_str(&payload, "error_preview")
                    .or_else(|| payload_str(&payload, "error"))
                    .or_else(|| payload_str(&payload, "operation"))
                    .unwrap_or("failure recorded");
                let line = format!(
                    "{} {} {}: {}",
                    event_timestamp(event),
                    event_string(event, "event_id").unwrap_or("?"),
                    source,
                    preview_line(preview, 160)
                );
                failures.push(line.clone());
                timeline.push(format!("failure: {line}"));
            }
            "PatchPromoted" | "PatchRejected" | "RevertPerformed" => {
                let patch_id = payload_str(&payload, "patch_id").unwrap_or("-");
                let reason = payload_str(&payload, "reason")
                    .or_else(|| payload_str(&payload, "intent"))
                    .unwrap_or("patch lifecycle event");
                let line = format!(
                    "{} {} {} {}: {}",
                    event_timestamp(event),
                    event_string(event, "event_id").unwrap_or("?"),
                    kind,
                    patch_id,
                    preview_line(reason, 160)
                );
                patch_outcomes.push(line.clone());
                timeline.push(format!("patch: {line}"));
            }
            "PatchEvaluated" | "TestCompleted" if is_eval_event(event, &payload) => {
                let eval_id = payload_str(&payload, "eval_id").unwrap_or("-");
                let status = payload_str(&payload, "status")
                    .or_else(|| {
                        payload.get("passed").and_then(|value| {
                            value
                                .as_bool()
                                .map(|passed| if passed { "passed" } else { "failed" })
                        })
                    })
                    .unwrap_or("-");
                let score = payload
                    .get("score")
                    .and_then(|value| value.as_f64())
                    .map(|value| format!(" score={value:.3}"))
                    .unwrap_or_default();
                let line = format!(
                    "{} {} {} {}{}",
                    event_timestamp(event),
                    event_string(event, "event_id").unwrap_or("?"),
                    eval_id,
                    status,
                    score
                );
                evals.push(line.clone());
                timeline.push(format!("eval: {line}"));
            }
            "PatchEvaluated" | "TestCompleted" => {}
            "DecisionRecorded" => {
                let decision = payload_str(&payload, "decision")
                    .or_else(|| payload_str(&payload, "decision_type"))
                    .unwrap_or("decision");
                let rationale = payload_str(&payload, "rationale")
                    .or_else(|| payload_str(&payload, "reason"))
                    .unwrap_or("-");
                let line = format!(
                    "{} {} {}: {}",
                    event_timestamp(event),
                    event_string(event, "event_id").unwrap_or("?"),
                    decision,
                    preview_line(rationale, 160)
                );
                decisions.push(line.clone());
                timeline.push(format!("decision: {line}"));
            }
            "RunCompleted" => {
                let line = format!(
                    "{} {} run={} status={}",
                    event_timestamp(event),
                    event_string(event, "event_id").unwrap_or("?"),
                    event_string(event, "run_id").unwrap_or("-"),
                    payload_str(&payload, "status").unwrap_or("completed")
                );
                timeline.push(format!("run: {line}"));
            }
            _ => {}
        }
    }

    let completed_runs = runs
        .values()
        .filter(|status| status.as_str() != "started")
        .count();
    let mut out = String::new();
    out.push_str("# State Journal Draft\n\n");
    out.push_str("This draft is generated from yoagent-state events for human review. It does not modify journals/JOURNAL.md automatically.\n\n");
    out.push_str("## Summary\n\n");
    out.push_str(&format!("- events: {}\n", events.len()));
    out.push_str(&format!(
        "- runs: {} tracked, {} completed\n",
        runs.len(),
        completed_runs
    ));
    out.push_str(&format!(
        "- failures: {}\n",
        counts.get("FailureObserved").copied().unwrap_or(0)
            + counts.get("JsonOutputFailure").copied().unwrap_or(0)
            + counts.get("ToolSchemaFailure").copied().unwrap_or(0)
    ));
    out.push_str(&format!(
        "- eval events: {}\n",
        counts.get("PatchEvaluated").copied().unwrap_or(0)
            + counts.get("TestCompleted").copied().unwrap_or(0)
    ));
    out.push_str(&format!(
        "- decisions: {}\n",
        counts.get("DecisionRecorded").copied().unwrap_or(0)
    ));

    if !counts.is_empty() {
        out.push_str("\n## Event Counts\n\n");
        for (kind, count) in counts {
            out.push_str(&format!("- {kind}: {count}\n"));
        }
    }

    push_report_section(&mut out, "Recent Timeline", &timeline, 12);
    push_report_section(&mut out, "Failures", &failures, 8);
    push_report_section(&mut out, "Eval Results", &evals, 8);
    push_report_section(&mut out, "Patch Outcomes", &patch_outcomes, 8);
    push_report_section(&mut out, "Decisions", &decisions, 8);

    out.push_str("\n## Draft Notes\n\n");
    if failures.is_empty() && patch_outcomes.is_empty() && evals.is_empty() && decisions.is_empty()
    {
        out.push_str("- No journal-worthy state events found yet.\n");
    } else {
        if !failures.is_empty() {
            out.push_str("- Mention recurring failures and whether a follow-up hypothesis or patch exists.\n");
        }
        if !evals.is_empty() {
            out.push_str("- Include eval status before claiming a harness improvement.\n");
        }
        if !patch_outcomes.is_empty() {
            out.push_str(
                "- Tie promoted or rejected patches back to the evidence that justified them.\n",
            );
        }
        if !decisions.is_empty() {
            out.push_str(
                "- Preserve decision rationale so future runs do not repeat settled choices.\n",
            );
        }
    }

    Ok(out.trim_end().to_string())
}

pub(crate) fn push_report_section(out: &mut String, title: &str, rows: &[String], limit: usize) {
    if rows.is_empty() {
        return;
    }
    out.push_str(&format!("\n## {title}\n\n"));
    let start = rows.len().saturating_sub(limit);
    for row in rows.iter().skip(start) {
        out.push_str(&format!("- {row}\n"));
    }
}

fn is_eval_event(event: &Value, payload: &Value) -> bool {
    event
        .get("event_type")
        .and_then(|v| v.as_str())
        .map(|kind| kind == "PatchEvaluated" || kind == "TestCompleted")
        .unwrap_or(false)
        && (payload.get("eval_id").is_some()
            || payload.get("harness_version").is_some()
            || payload.get("suite").is_some())
}

fn eval_artifact_uri(eval: &EvalResult) -> Option<&str> {
    eval_artifact_uri_from_payload(&eval.metrics)
}

fn eval_reproducibility_detail(typed: Option<&EvalResult>, payload: &Value) -> String {
    let manifest = typed
        .and_then(|eval| eval.metrics.get("reproducibility"))
        .or_else(|| payload.get("reproducibility"))
        .or_else(|| {
            payload
                .get("metrics")
                .and_then(|metrics| metrics.get("reproducibility"))
        });
    let Some(manifest) = manifest else {
        return String::new();
    };

    let mut detail = String::new();
    if let Some(dirty) = manifest.get("git_dirty").and_then(Value::as_bool) {
        detail.push_str(&format!(" dirty={}", if dirty { "yes" } else { "no" }));
    }
    if let Some(source) = payload_str(manifest, "agent_command_source") {
        detail.push_str(&format!(" agent_source={source}"));
    }
    detail
}

fn eval_fixture_suite_detail(typed: Option<&EvalResult>, payload: &Value) -> String {
    let fixture_suite = typed
        .and_then(|eval| eval.metrics.get("fixture_suite"))
        .or_else(|| payload.get("fixture_suite"))
        .or_else(|| {
            payload
                .get("metrics")
                .and_then(|metrics| metrics.get("fixture_suite"))
        });
    let Some(fixture_suite) = fixture_suite else {
        return String::new();
    };

    let mut detail = String::new();
    if let Some(task_count) = payload_u64(fixture_suite, "task_count") {
        detail.push_str(&format!(" fixture_tasks={task_count}"));
    }
    if let Some(command_count) = payload_u64(fixture_suite, "command_count") {
        detail.push_str(&format!(" fixture_commands={command_count}"));
    }
    if let Some(categories) = fixture_suite.get("categories").and_then(Value::as_object) {
        detail.push_str(&format!(
            " fixture_categories=[{}]",
            format_json_count_map(categories)
        ));
    }
    if let Some(risks) = fixture_suite.get("risk_labels").and_then(Value::as_object) {
        detail.push_str(&format!(
            " fixture_risks=[{}]",
            format_json_count_map(risks)
        ));
    }
    detail
}

fn format_json_count_map(values: &serde_json::Map<String, Value>) -> String {
    let labels = values
        .iter()
        .filter_map(|(key, value)| payload_value_u64(value).map(|count| format!("{key}={count}")))
        .collect::<Vec<_>>();
    if labels.is_empty() {
        "-".to_string()
    } else {
        labels.join(", ")
    }
}

fn eval_pass_fail_detail(typed: Option<&EvalResult>, payload: &Value) -> String {
    let passed = typed
        .map(|eval| eval.passed)
        .or_else(|| payload_u64(payload, "passed"));
    let failed = typed
        .map(|eval| eval.failed)
        .or_else(|| payload_u64(payload, "failed"));

    match (passed, failed) {
        (Some(passed), Some(failed)) => format!(" passed={passed} failed={failed}"),
        (Some(passed), None) => format!(" passed={passed}"),
        (None, Some(failed)) => format!(" failed={failed}"),
        (None, None) => String::new(),
    }
}

fn eval_failure_ids_detail(typed: Option<&EvalResult>, payload: &Value) -> String {
    let ids = typed
        .map(|eval| eval.failure_event_ids.clone())
        .or_else(|| {
            payload
                .get("failure_event_ids")
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
        })
        .unwrap_or_default();

    if ids.is_empty() {
        String::new()
    } else {
        format!(" failures=[{}]", compact_id_list(&ids, 4))
    }
}

fn compact_id_list(ids: &[String], limit: usize) -> String {
    let shown = ids
        .iter()
        .take(limit)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    if ids.len() > limit {
        format!("{shown}, +{} more", ids.len() - limit)
    } else {
        shown
    }
}

fn format_count_map<K: std::fmt::Display>(counts: &BTreeMap<K, usize>) -> String {
    if counts.is_empty() {
        return "-".to_string();
    }
    counts
        .iter()
        .map(|(key, count)| format!("{key}={count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_u64_count_map(counts: &BTreeMap<String, u64>) -> String {
    if counts.is_empty() {
        return "-".to_string();
    }
    counts
        .iter()
        .map(|(key, count)| format!("{key}={count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn format_top_relation_counts(counts: &BTreeMap<String, usize>, limit: usize) -> String {
    if counts.is_empty() {
        return "-".to_string();
    }
    let mut pairs = counts.iter().collect::<Vec<_>>();
    pairs.sort_by(|(left_key, left_count), (right_key, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| left_key.cmp(right_key))
    });
    let shown = pairs
        .iter()
        .take(limit)
        .map(|(key, count)| format!("{key}={count}"))
        .collect::<Vec<_>>()
        .join(", ");
    if pairs.len() > limit {
        format!("{shown}, +{} more", pairs.len() - limit)
    } else {
        shown
    }
}

fn eval_artifact_uri_from_payload(payload: &Value) -> Option<&str> {
    payload
        .get("artifact_uri")
        .and_then(|value| value.as_str())
        .or_else(|| {
            payload
                .get("metrics")
                .and_then(|metrics| metrics.get("artifact_uri"))
                .and_then(|value| value.as_str())
        })
}

#[derive(Debug, Clone)]
struct PatchSummary {
    patch_id: String,
    status: String,
    kind: String,
    risk_level: String,
    intent: String,
    base_harness_version: String,
    state_version: Option<u64>,
    base_git_commit: String,
    updated_at_ms: u64,
    event_count: usize,
}

fn build_patch_list_report(
    events: &[Value],
    status_filter: Option<&str>,
) -> Result<String, String> {
    let summaries = patch_summaries(events);
    if summaries.is_empty() {
        return Err("no harness patches found".to_string());
    }

    let normalized_filter = status_filter.map(normalize_status);
    let mut out = String::new();
    out.push_str("State patches\n");
    let mut shown = 0usize;
    for summary in summaries.values() {
        if normalized_filter
            .as_deref()
            .map(|filter| normalize_status(&summary.status) == filter)
            .unwrap_or(true)
        {
            shown += 1;
            out.push_str(&format!(
                "  {:<24} {:<17} {:<16} {:<8} harness={} state={} base={} {}{}\n",
                summary.patch_id,
                summary.status,
                summary.kind,
                summary.risk_level,
                summary.base_harness_version,
                summary
                    .state_version
                    .map(|version| version.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                summary.base_git_commit,
                summary.intent,
                if summary.event_count > 1 {
                    format!(" ({})", summary.event_count)
                } else {
                    String::new()
                }
            ));
        }
    }

    if shown == 0 {
        return Err(format!(
            "no harness patches found for status '{}'",
            status_filter.unwrap_or("")
        ));
    }
    Ok(out.trim_end().to_string())
}

fn build_patch_show_report(events: &[Value], patch_id: &str) -> Result<String, String> {
    let summaries = patch_summaries(events);
    let Some(summary) = summaries.get(patch_id) else {
        return Err(format!("no harness patch found for '{patch_id}'"));
    };
    let related: Vec<&Value> = events
        .iter()
        .filter(|event| {
            event
                .get("payload")
                .and_then(extract_patch_id)
                .map(|candidate| candidate == patch_id)
                .unwrap_or(false)
        })
        .collect();

    let proposal_payload = related
        .iter()
        .find(|event| {
            event
                .get("event_type")
                .and_then(|v| v.as_str())
                .map(|kind| kind == "PatchProposed")
                .unwrap_or(false)
        })
        .and_then(|event| event.get("payload"))
        .or_else(|| related.first().and_then(|event| event.get("payload")))
        .cloned()
        .unwrap_or(Value::Null);

    let mut out = String::new();
    out.push_str(&format!("State patch: {}\n", summary.patch_id));
    out.push_str(&format!("status: {}\n", summary.status));
    out.push_str(&format!("kind:   {}\n", summary.kind));
    out.push_str(&format!("risk:   {}\n", summary.risk_level));
    out.push_str(&format!("intent: {}\n", summary.intent));
    if let Some(base_harness_version) = payload_str(&proposal_payload, "base_harness_version") {
        out.push_str(&format!("base harness: {base_harness_version}\n"));
    }
    if let Some(state_version) = payload_u64(&proposal_payload, "state_version") {
        out.push_str(&format!("state version: {state_version}\n"));
    }
    if summary.base_git_commit != "-" {
        out.push_str(&format!("base:   {}\n", summary.base_git_commit));
    }

    let evidence = evidence_ids(&proposal_payload);
    if !evidence.is_empty() {
        out.push_str(&format!("evidence: {}\n", evidence.join(", ")));
        let evidence_preview = evidence
            .iter()
            .filter_map(|id| {
                find_target_event(events, id)
                    .map(|event| format!("  {}", format_event_value(event)))
            })
            .collect::<Vec<_>>();
        if !evidence_preview.is_empty() {
            out.push_str("Evidence:\n");
            for row in evidence_preview.iter().take(5) {
                out.push_str(row);
                out.push('\n');
            }
        }
    }

    let expected_effects = payload_string_array(&proposal_payload, "expected_effects");
    if !expected_effects.is_empty() {
        out.push_str("expected effects:\n");
        for effect in expected_effects {
            out.push_str(&format!("  - {}\n", preview_line(&effect, 160)));
        }
    }

    if let Some(eval_plan) = payload_str(&proposal_payload, "eval_plan") {
        out.push_str(&format!("eval plan: {}\n", preview_line(eval_plan, 160)));
    } else {
        let eval_plan = payload_string_array(&proposal_payload, "eval_plan");
        if !eval_plan.is_empty() {
            out.push_str("eval plan:\n");
            for step in eval_plan {
                out.push_str(&format!("  - {}\n", preview_line(&step, 160)));
            }
        }
    }
    let rollback_plan = payload_string_array(&proposal_payload, "rollback_plan");
    if !rollback_plan.is_empty() {
        out.push_str("rollback plan:\n");
        for step in rollback_plan {
            out.push_str(&format!("  - {}\n", preview_line(&step, 160)));
        }
    }

    let outcomes = patch_outcome_evidence(events, patch_id);
    if !outcomes.is_empty() {
        out.push_str("Outcomes:\n");
        for outcome in outcomes.iter().take(8) {
            out.push_str(&format!("  {outcome}\n"));
        }
    }

    out.push_str(&format!(
        "payload: {}\n",
        serde_json::to_string_pretty(&proposal_payload).unwrap_or_else(|_| "{}".to_string())
    ));
    out.push_str("\nTimeline:\n");
    for event in related {
        out.push_str(&format!("  {}\n", format_event_value(event)));
    }
    Ok(out.trim_end().to_string())
}

fn patch_summaries(events: &[Value]) -> BTreeMap<String, PatchSummary> {
    let mut summaries = BTreeMap::new();
    for event in events {
        let Some(payload) = event.get("payload") else {
            continue;
        };
        let Some(patch_id) = extract_patch_id(payload) else {
            continue;
        };
        let timestamp = event
            .get("timestamp_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let entry = summaries
            .entry(patch_id.to_string())
            .or_insert_with(|| PatchSummary {
                patch_id: patch_id.to_string(),
                status: "unknown".to_string(),
                kind: "-".to_string(),
                risk_level: "-".to_string(),
                intent: "-".to_string(),
                base_harness_version: "-".to_string(),
                state_version: None,
                base_git_commit: "-".to_string(),
                updated_at_ms: 0,
                event_count: 0,
            });

        entry.event_count += 1;
        if timestamp >= entry.updated_at_ms {
            entry.updated_at_ms = timestamp;
            entry.status = patch_status(event, payload).unwrap_or_else(|| entry.status.clone());
        }
        if let Ok(patch) = serde_json::from_value::<HarnessPatch>(payload.clone()) {
            entry.kind = serialized_label(&patch.kind);
            entry.risk_level = serialized_label(&patch.risk_level);
            entry.intent = patch.intent;
            entry.base_harness_version = patch.base_harness_version;
            entry.state_version = Some(u64::from(patch.state_version));
            entry.base_git_commit = patch.base_git_commit.unwrap_or_else(|| "-".to_string());
            if timestamp >= entry.updated_at_ms {
                entry.status = serialized_label(&patch.status);
            }
        }
        if let Some(kind) = payload.get("kind").and_then(|v| v.as_str()) {
            entry.kind = kind.to_string();
        }
        if let Some(risk) = payload.get("risk_level").and_then(|v| v.as_str()) {
            entry.risk_level = risk.to_string();
        }
        if let Some(intent) = payload.get("intent").and_then(|v| v.as_str()) {
            entry.intent = intent.to_string();
        }
        if let Some(base_harness) = payload.get("base_harness_version").and_then(|v| v.as_str()) {
            entry.base_harness_version = base_harness.to_string();
        }
        if let Some(state_version) = payload_u64(payload, "state_version") {
            entry.state_version = Some(state_version);
        }
        if let Some(base) = payload.get("base_git_commit").and_then(|v| v.as_str()) {
            entry.base_git_commit = base.to_string();
        }
    }
    summaries
}

fn serialized_label<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "-".to_string())
}

pub(crate) fn payload_str<'a>(payload: &'a Value, key: &str) -> Option<&'a str> {
    payload.get(key).and_then(|value| value.as_str())
}

fn payload_u64(payload: &Value, key: &str) -> Option<u64> {
    payload.get(key).and_then(|value| value.as_u64())
}

fn payload_u64_count_map(payload: &Value, key: &str) -> BTreeMap<String, u64> {
    payload
        .get(key)
        .and_then(Value::as_object)
        .map(|values| {
            values
                .iter()
                .filter_map(|(key, value)| value.as_u64().map(|count| (key.clone(), count)))
                .collect()
        })
        .unwrap_or_default()
}

fn value_u64_count_map(value: &Value) -> BTreeMap<String, u64> {
    value
        .as_object()
        .map(|values| {
            values
                .iter()
                .filter_map(|(key, value)| value.as_u64().map(|count| (key.clone(), count)))
                .collect()
        })
        .unwrap_or_default()
}

fn payload_value_u64(value: &Value) -> Option<u64> {
    value.as_u64()
}

fn payload_bool(payload: &Value, key: &str) -> Option<bool> {
    payload.get(key).and_then(|value| value.as_bool())
}

fn payload_f64(payload: &Value, key: &str) -> Option<f64> {
    payload.get(key).and_then(|value| value.as_f64())
}

fn patch_status(event: &Value, payload: &Value) -> Option<String> {
    payload
        .get("status")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            event
                .get("event_type")
                .and_then(|v| v.as_str())
                .and_then(event_type_patch_status)
                .map(|s| s.to_string())
        })
}

fn event_type_patch_status(event_type: &str) -> Option<&'static str> {
    match event_type {
        "PatchProposed" => Some("proposed"),
        "PatchApplied" => Some("applied_in_fork"),
        "PatchEvaluated" => Some("evaluated"),
        "PatchPromoted" => Some("promoted"),
        "PatchRejected" => Some("rejected"),
        "RevertPerformed" => Some("reverted"),
        "HumanApprovalRequested" => Some("needs_human"),
        "HumanApprovalReceived" => Some("approved_for_fork"),
        _ => None,
    }
}

fn normalize_status(status: &str) -> String {
    status.trim().replace(['-', ' '], "_").to_ascii_lowercase()
}

fn find_target_event<'a>(events: &'a [Value], id: &str) -> Option<&'a Value> {
    if id == "last-failure" {
        return events.iter().rev().find(|event| {
            event
                .get("event_type")
                .and_then(|v| v.as_str())
                .map(is_failure_event_type)
                .unwrap_or(false)
        });
    }

    events.iter().find(|event| event_matches_id(event, id))
}

fn is_failure_event_type(kind: &str) -> bool {
    matches!(
        kind,
        "FailureObserved" | "JsonOutputFailure" | "ToolSchemaFailure"
    )
}

fn event_matches_id(event: &Value, id: &str) -> bool {
    for key in ["event_id", "run_id", "trace_id"] {
        if event_string(event, key)
            .map(|value| value == id)
            .unwrap_or(false)
        {
            return true;
        }
    }

    let Some(payload) = event.get("payload") else {
        return false;
    };
    for key in [
        "patch_id",
        "patch_event_id",
        "decision_id",
        "hypothesis_id",
        "eval_id",
        "commit",
        "commit_id",
        "harness_version",
    ] {
        if payload
            .get(key)
            .and_then(|v| v.as_str())
            .map(|value| value == id)
            .unwrap_or(false)
        {
            return true;
        }
    }

    false
}

fn related_events<'a>(events: &'a [Value], target: &Value) -> Vec<&'a Value> {
    let target_event_id = event_string(target, "event_id").unwrap_or("");
    let target_run_id = event_string(target, "run_id");
    let target_patch_id = target.get("payload").and_then(extract_patch_id);
    let target_commit_ids = target
        .get("payload")
        .map(commit_ids_from_payload)
        .unwrap_or_default();

    events
        .iter()
        .filter(|event| {
            event_string(event, "event_id") == Some(target_event_id)
                || target_run_id
                    .map(|run_id| event_string(event, "run_id") == Some(run_id))
                    .unwrap_or(false)
                || target_patch_id
                    .map(|patch_id| {
                        event
                            .get("payload")
                            .and_then(extract_patch_id)
                            .map(|candidate| candidate == patch_id)
                            .unwrap_or(false)
                    })
                    .unwrap_or(false)
                || event
                    .get("payload")
                    .map(commit_ids_from_payload)
                    .unwrap_or_default()
                    .iter()
                    .any(|candidate| target_commit_ids.iter().any(|target| target == candidate))
                || parent_ids(event)
                    .iter()
                    .any(|parent| parent == target_event_id)
        })
        .take(20)
        .collect()
}

fn find_commit_event<'a>(events: &'a [Value], commit: &str) -> Option<&'a Value> {
    events.iter().find(|event| {
        event_string(event, "event_type") == Some("CommitCreated")
            && event
                .get("payload")
                .map(commit_ids_from_payload)
                .unwrap_or_default()
                .iter()
                .any(|candidate| candidate == commit)
    })
}

fn commit_revert_events<'a>(events: &'a [Value], commit: &str) -> Vec<&'a Value> {
    events
        .iter()
        .filter(|event| {
            event_string(event, "event_type") == Some("RevertPerformed")
                && event
                    .get("payload")
                    .and_then(|payload| payload_str(payload, "reverted_commit"))
                    .map(|candidate| candidate == commit)
                    .unwrap_or(false)
        })
        .collect()
}

fn commit_ids_from_payload(payload: &Value) -> Vec<String> {
    ["commit", "commit_id", "reverted_commit"]
        .iter()
        .filter_map(|key| payload_str(payload, key).map(|value| value.to_string()))
        .collect()
}

fn lineage_matches(event: &Value, id: &str, patch_id: &str, commit_ids: &[String]) -> bool {
    event_matches_id(event, id)
        || event
            .get("payload")
            .and_then(extract_patch_id)
            .map(|candidate| candidate == patch_id)
            .unwrap_or(false)
        || event
            .get("payload")
            .map(commit_ids_from_payload)
            .unwrap_or_default()
            .iter()
            .any(|candidate| commit_ids.iter().any(|id| id == candidate))
        || parent_ids(event).iter().any(|parent| parent == id)
        || event
            .get("payload")
            .map(evidence_ids)
            .unwrap_or_default()
            .iter()
            .any(|evidence_id| evidence_id == id)
}

pub(crate) fn event_string<'a>(event: &'a Value, key: &str) -> Option<&'a str> {
    event.get(key).and_then(|v| v.as_str())
}

fn event_timestamp(event: &Value) -> u64 {
    event
        .get("timestamp_ms")
        .or_else(|| event.get("ts_ms"))
        .and_then(|value| value.as_u64())
        .unwrap_or(0)
}

fn cache_ratio(hit: u64, miss: u64) -> f64 {
    let total = hit + miss;
    if total == 0 {
        0.0
    } else {
        hit as f64 / total as f64
    }
}

fn format_percent(ratio: f64) -> String {
    format!("{:.2}%", ratio * 100.0)
}

fn parent_ids(event: &Value) -> Vec<String> {
    event
        .get("parent_event_ids")
        .and_then(|v| v.as_array())
        .map(|ids| {
            ids.iter()
                .filter_map(|id| id.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn evidence_ids(payload: &Value) -> Vec<String> {
    payload
        .get("evidence_event_ids")
        .and_then(|v| v.as_array())
        .map(|ids| {
            ids.iter()
                .filter_map(|id| id.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn why_evidence_ids(payload: &Value) -> Vec<String> {
    let mut ids = evidence_ids(payload);
    ids.extend(payload_string_array(payload, "approval_event_ids"));
    ids.sort();
    ids.dedup();
    ids
}

fn extract_patch_id(payload: &Value) -> Option<&str> {
    payload.get("patch_id").and_then(|v| v.as_str())
}

fn print_usage() {
    println!(
        "Usage: yoyo state <command>\n\n  init\n  tail [--limit N]\n  trace <run-id|trace-id>\n  lifecycle [--limit N] [--json]\n  project --rebuild\n  migrate\n  recover [--output PATH] [--replace]\n  retention [--days N] [--archive PATH] [--prune]\n  memory synthesize [--output PATH] [--record]\n  memory list [--status proposed|promoted|rejected]\n  memory promote <candidate-id> [--reason TEXT]\n  memory reject <candidate-id> [--reason TEXT]\n  journal generate [--output PATH]\n  export <path>\n  import <path> [--replace]\n  graph <event-id|patch-id|eval-id|commit> [--depth N] [--to TARGET]\n  graph summary <event-id|patch-id|eval-id|commit> [--depth N]\n  graph clusters <event-id|patch-id|eval-id|commit> [--depth N] [--json]\n  graph impact <event-id|patch-id|eval-id|commit> [--depth N] [--json]\n  graph signals <event-id|patch-id|eval-id|commit> [--depth N] [--json]\n  graph evidence <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n  graph files <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n  graph evals <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n  graph patches <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n  graph decisions <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n  graph hypotheses <event-id|hypothesis-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n  graph versions <event-id|harness-version|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n  graph runs <event-id|run-id|trace-id|task-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n  graph artifacts <event-id|artifact-uri|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n  graph models <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n  graph tools <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n  graph commands <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n  graph tests <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n  graph commits <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n  graph memories <event-id|memory-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n  graph issues <event-id|issue-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n  graph cache <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n  graph failures <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n  graph policies <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n  graph protocol <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n  graph timeline <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n  graph hotspots [--limit N] [--json]\n  policies --recent [--limit N]\n  fixes --recent [--class CLASS] [--limit N]\n  rollbacks --recent [--limit N] [--json]\n  evals [--harness-version VERSION] [--patch-id PATCH]\n  patches [--status STATUS]\n  patches show <patch-id>\n  why <event-id|last-failure|patch-id|commit|run-id> [--summary] [--limit N]\n  lineage <event-id|patch-id|commit>\n  failures --recent\n  crashes [--limit N] [--json]\n  cache --recent"
    );
    println!("  graph summary accepts --json for machine-readable output");
    println!("  graph clusters accepts --json for machine-readable output");
    println!("  graph impact accepts --json for machine-readable output");
    println!("  graph signals accepts --json for machine-readable output");
    println!("  graph evidence accepts --json for machine-readable output");
    println!("  graph files accepts --json for machine-readable output");
    println!("  graph evals accepts --json for machine-readable output");
    println!("  graph patches accepts --json for machine-readable output");
    println!("  graph decisions accepts --json for machine-readable output");
    println!("  graph hypotheses accepts --json for machine-readable output");
    println!("  graph versions accepts --json for machine-readable output");
    println!("  graph runs accepts --json for machine-readable output");
    println!("  graph artifacts accepts --json for machine-readable output");
    println!("  graph models accepts --json for machine-readable output");
    println!("  graph tools accepts --json for machine-readable output");
    println!("  graph commands accepts --json for machine-readable output");
    println!("  graph tests accepts --json for machine-readable output");
    println!("  graph commits accepts --json for machine-readable output");
    println!("  graph memories accepts --json for machine-readable output");
    println!("  graph issues accepts --json for machine-readable output");
    println!("  graph cache accepts --json for machine-readable output");
    println!("  graph failures accepts --json for machine-readable output");
    println!("  graph policies accepts --json for machine-readable output");
    println!("  graph protocol accepts --json for machine-readable output");
    println!("  graph timeline accepts --json for machine-readable output");
    println!("  graph hotspots accepts --json for machine-readable output");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands_state_memory::{
        build_state_memory_candidates, build_state_memory_records, build_state_memory_synthesis,
        record_state_memory_candidates, record_state_memory_decision, StateMemoryCandidate,
    };
    use serde_json::json;

    fn event(id: &str, kind: &str, run: &str, payload: Value) -> Value {
        json!({
            "event_id": id,
            "event_type": kind,
            "schema_version": 1,
            "timestamp_ms": 10,
            "actor": "harness",
            "run_id": run,
            "session_id": null,
            "trace_id": "trace-1",
            "parent_event_ids": [],
            "payload": payload,
        })
    }

    fn event_at(id: &str, kind: &str, run: &str, timestamp_ms: i64, payload: Value) -> Value {
        let mut value = event(id, kind, run, payload);
        value["timestamp_ms"] = Value::from(timestamp_ms);
        value
    }

    fn write_jsonl(path: &Path, events: &[Value]) {
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(path, format!("{raw}\n")).unwrap();
    }

    #[test]
    fn failure_taxonomy_classifies_common_harness_failures() {
        let context = event(
            "evt-context",
            "FailureObserved",
            "run-1",
            json!({"source": "repair", "error_preview": "retry_state.rs missing from context"}),
        );
        let permission = event(
            "evt-permission",
            "FailureObserved",
            "run-1",
            json!({"source": "tool", "error_preview": "permission denied"}),
        );
        let transport = event(
            "evt-transport",
            "FailureObserved",
            "run-1",
            json!({"source": "deepseek", "error_preview": "503 network timeout"}),
        );
        let schema = event(
            "evt-schema",
            "ToolSchemaFailure",
            "run-1",
            json!({"source": "strict_tool_schema"}),
        );

        assert_eq!(classify_failure_event(&context).class, "context_miss");
        assert!(classify_failure_event(&context).retryable);
        assert_eq!(classify_failure_event(&permission).class, "permission");
        assert!(!classify_failure_event(&permission).retryable);
        assert_eq!(classify_failure_event(&transport).class, "transport");
        assert_eq!(classify_failure_event(&schema).class, "tool_schema");
        assert_eq!(classify_failure_event(&schema).owner, "model_output");
    }

    #[test]
    fn formatted_failure_event_includes_taxonomy_fields() {
        let failure = event(
            "evt-json",
            "JsonOutputFailure",
            "run-1",
            json!({"source": "json_output", "attempts": []}),
        );

        let line = format_failure_event_value(&failure);

        assert!(line.contains("JsonOutputFailure"));
        assert!(line.contains("class=json_output"));
        assert!(line.contains("owner=model_output"));
        assert!(line.contains("retryable=true"));
    }

    #[test]
    fn trace_report_reconstructs_run_timeline_with_counts() {
        let events = vec![
            event_at(
                "evt-other",
                "RunStarted",
                "run-other",
                1,
                json!({"task": "skip"}),
            ),
            event_at(
                "evt-2",
                "ToolCallCompleted",
                "run-1",
                20,
                json!({"tool": "bash"}),
            ),
            event_at(
                "evt-1",
                "RunStarted",
                "run-1",
                10,
                json!({"task": "fix bug"}),
            ),
            event_at(
                "evt-3",
                "FailureObserved",
                "run-1",
                30,
                json!({"source": "test", "error_preview": "assertion failed"}),
            ),
            event_at(
                "evt-4",
                "RunCompleted",
                "run-1",
                40,
                json!({"status": "error"}),
            ),
        ];

        let report = build_trace_report(&events, "run-1").unwrap();

        assert!(report.contains("State trace: run-1"));
        assert!(report.contains("run:   run-1"));
        assert!(report.contains("status: error"));
        assert!(report.contains("events: 4"));
        assert!(report.contains("RunStarted: 1"));
        assert!(report.contains("ToolCallCompleted: 1"));
        assert!(report.contains("FailureObserved: 1"));
        assert!(report.contains("RunCompleted: 1"));
        assert!(report.contains("Timeline:"));
        assert!(report.find("evt-1").unwrap() < report.find("evt-2").unwrap());
        assert!(report.find("evt-2").unwrap() < report.find("evt-3").unwrap());
        assert!(report.find("evt-3").unwrap() < report.find("evt-4").unwrap());
        assert!(!report.contains("evt-other"));
    }

    #[test]
    fn recent_failure_report_summarizes_limited_recent_failures() {
        let events = vec![
            event_at(
                "evt-old",
                "FailureObserved",
                "run-1",
                1,
                json!({"source": "repair", "error_preview": "old context missing"}),
            ),
            event_at("evt-run", "RunStarted", "run-2", 2, json!({"task": "x"})),
            event_at(
                "evt-permission",
                "FailureObserved",
                "run-2",
                3,
                json!({"source": "tool", "error_preview": "permission denied"}),
            ),
            event_at(
                "evt-schema",
                "ToolSchemaFailure",
                "run-3",
                4,
                json!({"source": "strict_tool_schema"}),
            ),
        ];

        let report = build_recent_failure_report(&events, 2).unwrap();

        assert!(report.contains("State failures"));
        assert!(report.contains("recent events: 2"));
        assert!(report.contains("retryable:     1"));
        assert!(report.contains("classes:       permission=1, tool_schema=1"));
        assert!(report.contains("evt-schema"));
        assert!(report.contains("evt-permission"));
        assert!(!report.contains("evt-old"));
    }

    #[test]
    fn export_events_validates_and_writes_normalized_jsonl() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let export_path = dir.path().join("export").join("state.jsonl");
        write_jsonl(
            &events_path,
            &[event(
                "evt-1",
                "RunStarted",
                "run-1",
                json!({"task": "smoke"}),
            )],
        );

        let count = export_events(&events_path, &export_path).unwrap();

        assert_eq!(count, 1);
        let raw = std::fs::read_to_string(export_path).unwrap();
        assert!(raw.contains("\"id\":\"evt-1\""));
        assert!(raw.contains("\"kind\":\"RunStarted\""));
        assert!(raw.ends_with('\n'));
    }

    #[test]
    fn export_events_redacts_legacy_raw_payloads() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let export_path = dir.path().join("export").join("state.jsonl");
        write_jsonl(
            &events_path,
            &[event(
                "evt-secret-export",
                "FailureObserved",
                "run-1",
                json!({
                    "source": "export",
                    "api_key": "sk-exportsecret123456789",
                    "error_preview": "Authorization: Bearer exportbearer123456"
                }),
            )],
        );

        let count = export_events(&events_path, &export_path).unwrap();

        assert_eq!(count, 1);
        let raw = std::fs::read_to_string(export_path).unwrap();
        assert!(!raw.contains("sk-exportsecret123456789"));
        assert!(!raw.contains("exportbearer123456"));
        assert!(raw.contains("[redacted]"));
    }

    #[test]
    fn tail_formats_canonical_yoagent_state_lines_through_adapter() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        crate::state::append_event(
            &events_path,
            &crate::state::StateEvent {
                event_id: "evt-tail".into(),
                event_type: crate::state::EventType::RunStarted,
                schema_version: yoagent_state::CURRENT_EVENT_SCHEMA_VERSION,
                timestamp_ms: 10,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-tail".into()),
                session_id: None,
                trace_id: "trace-tail".into(),
                parent_event_ids: Vec::new(),
                payload: json!({"task": "tail"}),
            },
        )
        .unwrap();

        let raw = std::fs::read_to_string(&events_path).unwrap();
        let line = raw.lines().next().unwrap();
        let value = event_line_value(line).unwrap();

        assert_eq!(value["event_id"], "evt-tail");
        assert_eq!(value["event_type"], "RunStarted");
        assert_eq!(value["run_id"], "run-tail");
        assert!(value["payload"].get("_yoyo").is_none());
        assert!(format_event_value(&value).contains("run=run-tail"));
    }

    #[test]
    fn tail_formats_common_events_as_human_summaries() {
        let edit = event(
            "evt-edit",
            "FileEdited",
            "run-1",
            json!({
                "tool_call_id": "tool-1",
                "path": "src/lib.rs",
                "edit_kind": "edit_file",
                "old_line_count": 2,
                "new_line_count": 3,
                "diff_preview": "- old\n+ new"
            }),
        );
        let failure = event(
            "evt-failure",
            "FailureObserved",
            "run-1",
            json!({
                "source": "tool",
                "tool_name": "bash",
                "error_preview": "permission denied while running deploy"
            }),
        );
        let tool = event(
            "evt-tool",
            "ToolCallStarted",
            "run-1",
            json!({
                "tool_call_id": "tool-2",
                "tool_name": "write_file",
                "args": {
                    "path": "src/main.rs",
                    "content_omitted": true,
                    "content_line_count": 20
                }
            }),
        );

        let edit_line = format_event_value(&edit);
        let failure_line = format_event_value(&failure);
        let tool_line = format_event_value(&tool);

        assert!(edit_line.contains("path=src/lib.rs"));
        assert!(edit_line.contains("lines=2->3"));
        assert!(edit_line.contains("diff=- old | + new"));
        assert!(!edit_line.contains("\"diff_preview\""));

        assert!(failure_line.contains("class=permission"));
        assert!(failure_line.contains("owner=user_or_policy"));
        assert!(failure_line.contains("error=permission denied"));
        assert!(!failure_line.contains("\"error_preview\""));

        assert!(tool_line.contains("tool=write_file"));
        assert!(tool_line.contains("path=src/main.rs"));
        assert!(tool_line.contains("content_omitted"));
        assert!(!tool_line.contains("\"args\""));
    }

    #[test]
    fn import_events_replace_rebuilds_projection() {
        let dir = tempfile::tempdir().unwrap();
        let import_path = dir.path().join("import.jsonl");
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        write_jsonl(
            &import_path,
            &[event(
                "evt-failure",
                "FailureObserved",
                "run-1",
                json!({"source": "tool", "error_preview": "boom"}),
            )],
        );

        let report = import_events(&import_path, &events_path, &sqlite_path, true).unwrap();

        assert_eq!(report.imported_events, 1);
        assert!(report.replaced);
        assert_eq!(report.projection.events, 1);
        assert_eq!(report.projection.failures, 1);
        let conn = rusqlite::Connection::open(sqlite_path).unwrap();
        let failure_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM failures", [], |row| row.get(0))
            .unwrap();
        assert_eq!(failure_count, 1);
    }

    #[test]
    fn import_events_redacts_external_payloads_before_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let import_path = dir.path().join("import.jsonl");
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        write_jsonl(
            &import_path,
            &[event(
                "evt-secret-import",
                "FailureObserved",
                "run-1",
                json!({
                    "source": "import",
                    "api_key": "sk-importsecret123456789",
                    "error_preview": "Authorization: Bearer importbearer123456"
                }),
            )],
        );

        let report = import_events(&import_path, &events_path, &sqlite_path, true).unwrap();

        assert_eq!(report.imported_events, 1);
        let raw = std::fs::read_to_string(&events_path).unwrap();
        assert!(!raw.contains("sk-importsecret123456789"));
        assert!(!raw.contains("importbearer123456"));
        assert!(raw.contains("[redacted]"));
        let payload_json: String = rusqlite::Connection::open(sqlite_path)
            .unwrap()
            .query_row(
                "SELECT payload_json FROM state_events WHERE event_id = 'evt-secret-import'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!payload_json.contains("sk-importsecret123456789"));
        assert!(!payload_json.contains("importbearer123456"));
    }

    #[test]
    fn import_events_append_preserves_existing_log() {
        let dir = tempfile::tempdir().unwrap();
        let import_path = dir.path().join("import.jsonl");
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        write_jsonl(
            &events_path,
            &[event("evt-existing", "RunStarted", "run-1", json!({}))],
        );
        write_jsonl(
            &import_path,
            &[event("evt-imported", "RunCompleted", "run-1", json!({}))],
        );

        let report = import_events(&import_path, &events_path, &sqlite_path, false).unwrap();

        assert_eq!(report.imported_events, 1);
        assert!(!report.replaced);
        assert_eq!(report.projection.events, 2);
        let raw = std::fs::read_to_string(events_path).unwrap();
        assert!(raw.contains("evt-existing"));
        assert!(raw.contains("evt-imported"));
    }

    #[test]
    fn import_events_rejects_invalid_event_jsonl() {
        let dir = tempfile::tempdir().unwrap();
        let import_path = dir.path().join("bad.jsonl");
        std::fs::write(&import_path, r#"{"event_id":"evt-bad"}"#).unwrap();

        let err = import_events(
            &import_path,
            &dir.path().join("events.jsonl"),
            &dir.path().join("state.sqlite"),
            true,
        )
        .unwrap_err();

        assert!(err.contains("missing 'event_type'"));
    }

    #[test]
    fn retention_policy_reports_old_events_without_replacing_source() {
        const DAY_MS: i64 = 86_400_000;
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let now = 20 * DAY_MS;
        write_jsonl(
            &events_path,
            &[
                event_at(
                    "evt-old",
                    "RunStarted",
                    "run-1",
                    now - 10 * DAY_MS,
                    json!({"task": "old"}),
                ),
                event_at(
                    "evt-new",
                    "RunCompleted",
                    "run-1",
                    now - DAY_MS,
                    json!({"status": "completed"}),
                ),
            ],
        );

        let report = apply_retention_policy(&events_path, &sqlite_path, 7, None, false, now)
            .expect("retention report");

        assert_eq!(report.total_events, 2);
        assert_eq!(report.kept_events, 1);
        assert_eq!(report.archived_events, 1);
        assert!(report.archive_path.is_none());
        assert!(report.backup_path.is_none());
        assert!(!report.pruned);
        assert_eq!(report.projected_events, 0);
        let raw = std::fs::read_to_string(events_path).unwrap();
        assert!(raw.contains("evt-old"));
        assert!(raw.contains("evt-new"));
    }

    #[test]
    fn retention_policy_archives_and_prunes_with_backup_and_projection_rebuild() {
        const DAY_MS: i64 = 86_400_000;
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let archive_path = dir.path().join("archive").join("old-events.jsonl");
        let now = 20 * DAY_MS;
        write_jsonl(
            &events_path,
            &[
                event_at(
                    "evt-old-failure",
                    "FailureObserved",
                    "run-1",
                    now - 10 * DAY_MS,
                    json!({"source": "test", "error_preview": "old boom"}),
                ),
                event_at(
                    "evt-new-failure",
                    "FailureObserved",
                    "run-1",
                    now - DAY_MS,
                    json!({"source": "test", "error_preview": "new boom"}),
                ),
            ],
        );

        let report = apply_retention_policy(
            &events_path,
            &sqlite_path,
            7,
            Some(&archive_path),
            true,
            now,
        )
        .expect("retention prune");

        assert_eq!(report.total_events, 2);
        assert_eq!(report.kept_events, 1);
        assert_eq!(report.archived_events, 1);
        assert_eq!(report.archive_path.as_deref(), Some(archive_path.as_path()));
        assert!(report.backup_path.is_some());
        assert!(report.pruned);
        assert_eq!(report.projected_events, 1);

        let archived = std::fs::read_to_string(&archive_path).unwrap();
        assert!(archived.contains("\"id\":\"evt-old-failure\""));
        assert!(!archived.contains("evt-new-failure"));

        let current = std::fs::read_to_string(&events_path).unwrap();
        assert!(current.contains("\"id\":\"evt-new-failure\""));
        assert!(!current.contains("evt-old-failure"));

        let backup = std::fs::read_to_string(report.backup_path.unwrap()).unwrap();
        assert!(backup.contains("evt-old-failure"));
        assert!(backup.contains("evt-new-failure"));

        let conn = rusqlite::Connection::open(sqlite_path).unwrap();
        let failure_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM failures", [], |row| row.get(0))
            .unwrap();
        assert_eq!(failure_count, 1);
    }

    #[test]
    fn retention_archive_redacts_legacy_raw_payloads() {
        const DAY_MS: i64 = 86_400_000;
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let archive_path = dir.path().join("archive").join("old-events.jsonl");
        let now = 20 * DAY_MS;
        write_jsonl(
            &events_path,
            &[
                event_at(
                    "evt-old-secret",
                    "FailureObserved",
                    "run-1",
                    now - 10 * DAY_MS,
                    json!({
                        "source": "retention",
                        "api_key": "sk-archivesecret123456789",
                        "error_preview": "Authorization: Bearer archivebearer123456"
                    }),
                ),
                event_at(
                    "evt-new",
                    "RunCompleted",
                    "run-1",
                    now - DAY_MS,
                    json!({"status": "completed"}),
                ),
            ],
        );

        let report = apply_retention_policy(
            &events_path,
            &sqlite_path,
            7,
            Some(&archive_path),
            false,
            now,
        )
        .expect("retention archive");

        assert_eq!(report.archived_events, 1);
        let archived = std::fs::read_to_string(&archive_path).unwrap();
        assert!(archived.contains("\"id\":\"evt-old-secret\""));
        assert!(!archived.contains("sk-archivesecret123456789"));
        assert!(!archived.contains("archivebearer123456"));
        assert!(archived.contains("[redacted]"));
    }

    #[test]
    fn retention_backup_redacts_legacy_raw_payloads() {
        const DAY_MS: i64 = 86_400_000;
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let now = 20 * DAY_MS;
        write_jsonl(
            &events_path,
            &[
                event_at(
                    "evt-old-secret",
                    "FailureObserved",
                    "run-1",
                    now - 10 * DAY_MS,
                    json!({
                        "source": "retention",
                        "api_key": "sk-backupsecret123456789",
                        "error_preview": "Authorization: Bearer backupbearer123456"
                    }),
                ),
                event_at(
                    "evt-new",
                    "RunCompleted",
                    "run-1",
                    now - DAY_MS,
                    json!({"status": "completed"}),
                ),
            ],
        );

        let report = apply_retention_policy(&events_path, &sqlite_path, 7, None, true, now)
            .expect("retention prune");

        let backup_path = report.backup_path.expect("backup path");
        let backup = std::fs::read_to_string(backup_path).unwrap();
        assert!(backup.contains("\"id\":\"evt-old-secret\""));
        assert!(backup.contains("\"id\":\"evt-new\""));
        assert!(!backup.contains("sk-backupsecret123456789"));
        assert!(!backup.contains("backupbearer123456"));
        assert!(backup.contains("[redacted]"));
    }

    #[test]
    fn state_memory_synthesis_distills_failures_hypotheses_and_patch_outcomes() {
        let events = vec![
            event(
                "evt-failure-1",
                "FailureObserved",
                "run-1",
                json!({
                    "source": "test",
                    "error_preview": "missing retry_state.rs in context"
                }),
            ),
            event(
                "evt-failure-2",
                "FailureObserved",
                "run-2",
                json!({
                    "source": "test",
                    "error_preview": "same failing file omitted again"
                }),
            ),
            event(
                "evt-hypothesis",
                "HypothesisCreated",
                "run-2",
                json!({
                    "hypothesis_id": "hyp-context",
                    "summary": "repair planning misses files named in test output",
                    "confidence": 0.82
                }),
            ),
            event(
                "evt-promoted",
                "PatchPromoted",
                "run-3",
                json!({
                    "patch_id": "patch-context",
                    "intent": "include files referenced by failing test output"
                }),
            ),
            event(
                "evt-rejected",
                "PatchRejected",
                "run-4",
                json!({
                    "patch_id": "patch-shell",
                    "reason": "weakens shell approval policy"
                }),
            ),
            event(
                "evt-decision",
                "DecisionRecorded",
                "run-4",
                json!({
                    "decision": "reject",
                    "rationale": "safety regression outweighed speed benefit"
                }),
            ),
        ];

        let report = build_state_memory_synthesis(&events).unwrap();

        assert!(report.contains("# State Memory Synthesis"));
        assert!(report.contains("- failures: 2"));
        assert!(report.contains("- test: 2"));
        assert!(report.contains(
            "hyp-context: repair planning misses files named in test output confidence=0.82"
        ));
        assert!(report.contains("patch-context: include files referenced by failing test output"));
        assert!(report.contains("patch-shell: weakens shell approval policy"));
        assert!(report.contains("reject: safety regression outweighed speed benefit"));
        assert!(report.contains("does not mutate project memory automatically"));
    }

    #[test]
    fn state_memory_synthesis_can_write_reviewable_artifact() {
        let dir = tempfile::tempdir().unwrap();
        let output_path = dir.path().join("memory").join("state-memory.md");
        let report = build_state_memory_synthesis(&[event(
            "evt-json",
            "JsonOutputFailure",
            "run-1",
            json!({"operation": "structured-extraction"}),
        )])
        .unwrap();

        write_text_artifact(&output_path, &report, "state memory synthesis").unwrap();

        let raw = std::fs::read_to_string(output_path).unwrap();
        assert!(raw.contains("State Memory Synthesis"));
        assert!(raw.contains("structured-extraction"));
    }

    #[test]
    fn state_memory_candidates_build_structured_proposals() {
        let events = vec![
            event(
                "evt-failure-1",
                "FailureObserved",
                "run-1",
                json!({"source": "test", "error_preview": "context omitted retry_state.rs"}),
            ),
            event(
                "evt-failure-2",
                "FailureObserved",
                "run-2",
                json!({"source": "test", "error_preview": "same context miss"}),
            ),
            event(
                "evt-hypothesis",
                "HypothesisCreated",
                "run-2",
                json!({
                    "hypothesis_id": "hyp-context",
                    "summary": "repair planning should include files named in test output",
                    "confidence": 0.91,
                    "evidence_event_ids": ["evt-failure-1", "evt-failure-2"]
                }),
            ),
            event(
                "evt-promoted",
                "PatchPromoted",
                "run-3",
                json!({
                    "patch_id": "patch-context",
                    "intent": "include failing test referenced files in dynamic context"
                }),
            ),
            event(
                "evt-rejected",
                "PatchRejected",
                "run-4",
                json!({
                    "patch_id": "patch-shell",
                    "reason": "weakens shell approval policy"
                }),
            ),
        ];

        let candidates = build_state_memory_candidates(&events);

        assert_eq!(candidates.len(), 4);
        assert!(candidates.iter().any(|candidate| {
            candidate.candidate_id == "memory-hypothesis-hyp-context"
                && candidate.source == "high_confidence_hypothesis"
                && candidate.evidence_event_ids
                    == vec![
                        "evt-failure-1".to_string(),
                        "evt-failure-2".to_string(),
                        "evt-hypothesis".to_string(),
                    ]
        }));
        assert!(candidates.iter().any(|candidate| {
            candidate.candidate_id == "memory-recurring_failure-test"
                && candidate.source == "recurring_failure_source"
                && candidate.evidence_event_ids
                    == vec!["evt-failure-1".to_string(), "evt-failure-2".to_string()]
        }));
        assert!(candidates
            .iter()
            .any(|candidate| candidate.candidate_id == "memory-promoted_patch-patch-context"));
        assert!(candidates
            .iter()
            .any(|candidate| candidate.candidate_id == "memory-rejected_patch-patch-shell"));
    }

    #[test]
    fn memory_candidate_payload_records_review_required_lineage() {
        let candidate = StateMemoryCandidate {
            candidate_id: "memory-hypothesis-hyp-context".to_string(),
            source: "high_confidence_hypothesis".to_string(),
            summary: "repair planning should include files named in test output".to_string(),
            evidence_event_ids: vec!["evt-hypothesis".to_string()],
        };

        let payload = candidate.to_payload(1234);

        assert_eq!(payload["candidate_id"], "memory-hypothesis-hyp-context");
        assert_eq!(payload["status"], "proposed");
        assert_eq!(payload["proposed_by"], "state_memory_synthesis");
        assert_eq!(payload["proposed_at_ms"], 1234);
        assert_eq!(payload["review_required"], true);
        assert_eq!(payload["evidence_event_ids"][0], "evt-hypothesis");
    }

    #[test]
    fn state_memory_record_appends_reviewable_memory_proposal_events_once() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        write_jsonl(
            &events_path,
            &[
                event(
                    "evt-failure-1",
                    "FailureObserved",
                    "run-1",
                    json!({"source": "test", "error_preview": "context omitted retry_state.rs"}),
                ),
                event(
                    "evt-failure-2",
                    "FailureObserved",
                    "run-2",
                    json!({"source": "test", "error_preview": "same context miss"}),
                ),
            ],
        );
        let events = read_events(&events_path).unwrap();

        let event_ids = record_state_memory_candidates(&events, &events_path).unwrap();

        assert_eq!(event_ids.len(), 1);
        let recorded = read_events(&events_path).unwrap();
        let proposal = recorded
            .iter()
            .find(|event| event_string(event, "event_type") == Some("MemoryProposed"))
            .expect("memory proposal event");
        assert_eq!(
            proposal["payload"]["candidate_id"],
            "memory-recurring_failure-test"
        );
        assert_eq!(proposal["payload"]["review_required"], true);
        assert_eq!(proposal["payload"]["proposed_by"], "state_memory_synthesis");

        let duplicate_event_ids = record_state_memory_candidates(&recorded, &events_path).unwrap();
        assert!(duplicate_event_ids.is_empty());
    }

    #[test]
    fn state_memory_records_track_promoted_and_rejected_candidates() {
        let events = vec![
            event(
                "evt-proposed-1",
                "MemoryProposed",
                "run-1",
                json!({
                    "candidate_id": "memory-hypothesis-hyp-context",
                    "source": "high_confidence_hypothesis",
                    "summary": "include files named in test output",
                    "evidence_event_ids": ["evt-hypothesis"],
                    "status": "proposed"
                }),
            ),
            event(
                "evt-promoted-1",
                "MemoryPromoted",
                "run-2",
                json!({
                    "candidate_id": "memory-hypothesis-hyp-context",
                    "source": "high_confidence_hypothesis",
                    "summary": "include files named in test output",
                    "reason": "useful repeated repair rule"
                }),
            ),
            event(
                "evt-proposed-2",
                "MemoryProposed",
                "run-2",
                json!({
                    "candidate_id": "memory-rejected_patch-patch-shell",
                    "source": "rejected_harness_patch",
                    "summary": "weakens shell approval policy",
                    "status": "proposed"
                }),
            ),
            event(
                "evt-rejected-1",
                "MemoryRejected",
                "run-3",
                json!({
                    "candidate_id": "memory-rejected_patch-patch-shell",
                    "source": "rejected_harness_patch",
                    "summary": "weakens shell approval policy",
                    "reason": "too narrow for durable memory"
                }),
            ),
        ];

        let records = build_state_memory_records(&events);

        let promoted = records
            .iter()
            .find(|record| record.candidate_id == "memory-hypothesis-hyp-context")
            .unwrap();
        assert_eq!(promoted.status, "promoted");
        assert_eq!(
            promoted.proposed_event_id.as_deref(),
            Some("evt-proposed-1")
        );
        assert_eq!(
            promoted.decision_event_id.as_deref(),
            Some("evt-promoted-1")
        );
        assert_eq!(
            promoted.reason.as_deref(),
            Some("useful repeated repair rule")
        );

        let rejected = records
            .iter()
            .find(|record| record.candidate_id == "memory-rejected_patch-patch-shell")
            .unwrap();
        assert_eq!(rejected.status, "rejected");
        assert_eq!(
            rejected.decision_event_id.as_deref(),
            Some("evt-rejected-1")
        );
        assert_eq!(
            rejected.reason.as_deref(),
            Some("too narrow for durable memory")
        );
    }

    #[test]
    fn state_memory_decision_appends_promoted_event_with_proposal_lineage() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        write_jsonl(
            &events_path,
            &[event(
                "evt-proposed",
                "MemoryProposed",
                "run-1",
                json!({
                    "candidate_id": "memory-hypothesis-hyp-context",
                    "source": "high_confidence_hypothesis",
                    "summary": "include files named in test output",
                    "evidence_event_ids": ["evt-hypothesis"],
                    "status": "proposed"
                }),
            )],
        );
        let events = read_events(&events_path).unwrap();

        let event_id = record_state_memory_decision(
            &events,
            &events_path,
            "memory-hypothesis-hyp-context",
            true,
            "accepted after review",
        )
        .unwrap();

        let recorded = read_events(&events_path).unwrap();
        let promoted = recorded
            .iter()
            .find(|event| event_string(event, "event_id") == Some(event_id.as_str()))
            .expect("promoted memory event");
        assert_eq!(event_string(promoted, "event_type"), Some("MemoryPromoted"));
        assert_eq!(
            promoted["payload"]["candidate_id"],
            "memory-hypothesis-hyp-context"
        );
        assert_eq!(promoted["payload"]["proposed_event_id"], "evt-proposed");
        assert_eq!(promoted["payload"]["reason"], "accepted after review");
        assert_eq!(
            promoted["payload"]["evidence_event_ids"][0],
            "evt-hypothesis"
        );

        let err = record_state_memory_decision(
            &recorded,
            &events_path,
            "memory-hypothesis-hyp-context",
            false,
            "late rejection",
        )
        .unwrap_err();
        assert!(err.contains("already promoted"));
    }

    #[test]
    fn state_journal_draft_summarizes_runs_failures_evals_and_decisions() {
        let events = vec![
            event(
                "evt-run-start",
                "RunStarted",
                "run-1",
                json!({"task": "repair failing test"}),
            ),
            event(
                "evt-failure",
                "FailureObserved",
                "run-1",
                json!({
                    "source": "test",
                    "error_preview": "context omitted retry_state.rs"
                }),
            ),
            event(
                "evt-eval",
                "PatchEvaluated",
                "run-1",
                json!({
                    "eval_id": "eval-1",
                    "suite": "local-smoke",
                    "status": "passed",
                    "score": 0.91
                }),
            ),
            event(
                "evt-promoted",
                "PatchPromoted",
                "run-1",
                json!({
                    "patch_id": "patch-context",
                    "reason": "reduced context misses"
                }),
            ),
            event(
                "evt-decision",
                "DecisionRecorded",
                "run-1",
                json!({
                    "decision": "promote",
                    "rationale": "eval passed and failure class was addressed"
                }),
            ),
            event(
                "evt-run-done",
                "RunCompleted",
                "run-1",
                json!({"status": "completed"}),
            ),
        ];

        let report = build_state_journal_draft(&events).unwrap();

        assert!(report.contains("# State Journal Draft"));
        assert!(report.contains("does not modify journals/JOURNAL.md automatically"));
        assert!(report.contains("- runs: 1 tracked, 1 completed"));
        assert!(report.contains("- failures: 1"));
        assert!(report.contains("- eval events: 1"));
        assert!(report.contains("- decisions: 1"));
        assert!(report.contains("context omitted retry_state.rs"));
        assert!(report.contains("eval-1 passed score=0.910"));
        assert!(report.contains("PatchPromoted patch-context: reduced context misses"));
        assert!(report.contains("promote: eval passed and failure class was addressed"));
    }

    #[test]
    fn state_journal_draft_can_write_reviewable_artifact() {
        let dir = tempfile::tempdir().unwrap();
        let output_path = dir.path().join("journals").join("state-draft.md");
        let report = build_state_journal_draft(&[event(
            "evt-run-done",
            "RunCompleted",
            "run-1",
            json!({"status": "completed"}),
        )])
        .unwrap();

        write_text_artifact(&output_path, &report, "state journal draft").unwrap();

        let raw = std::fs::read_to_string(output_path).unwrap();
        assert!(raw.contains("State Journal Draft"));
        assert!(raw.contains("status=completed"));
    }

    #[test]
    fn recover_events_writes_normalized_recovery_file_without_replacing_source() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let output_path = dir.path().join("events.recovered.jsonl");
        std::fs::write(
            &events_path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&event(
                    "evt-valid",
                    "RunStarted",
                    "run-1",
                    json!({"task": "recover"})
                ))
                .unwrap(),
                r#"{"event_id":"evt-bad""#
            ),
        )
        .unwrap();

        let report = recover_events(&events_path, &sqlite_path, Some(&output_path), false).unwrap();

        assert_eq!(report.valid_events, 1);
        assert_eq!(report.invalid_lines.len(), 1);
        assert!(!report.replaced);
        assert!(report.backup_path.is_none());
        assert_eq!(report.projected_events, 0);
        let original = std::fs::read_to_string(&events_path).unwrap();
        assert!(original.contains("evt-bad"));
        let recovered = std::fs::read_to_string(&output_path).unwrap();
        assert!(recovered.contains("\"id\":\"evt-valid\""));
        assert!(!recovered.contains("evt-bad"));
    }

    #[test]
    fn recover_events_redacts_valid_payloads_before_recovery_file() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let output_path = dir.path().join("events.recovered.jsonl");
        std::fs::write(
            &events_path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&event(
                    "evt-secret-recover",
                    "FailureObserved",
                    "run-1",
                    json!({
                        "source": "recover",
                        "secret": "sk-recoversecret123456789",
                        "error_preview": "Authorization: Bearer recoverbearer123456"
                    })
                ))
                .unwrap(),
                "not-json"
            ),
        )
        .unwrap();

        let report = recover_events(&events_path, &sqlite_path, Some(&output_path), false).unwrap();

        assert_eq!(report.valid_events, 1);
        let original = std::fs::read_to_string(&events_path).unwrap();
        assert!(original.contains("sk-recoversecret123456789"));
        let recovered = std::fs::read_to_string(&output_path).unwrap();
        assert!(!recovered.contains("sk-recoversecret123456789"));
        assert!(!recovered.contains("recoverbearer123456"));
        assert!(recovered.contains("[redacted]"));
    }

    #[test]
    fn recover_events_replace_backs_up_log_and_rebuilds_projection() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        std::fs::write(
            &events_path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&event(
                    "evt-failure",
                    "FailureObserved",
                    "run-1",
                    json!({"source": "test", "error_preview": "boom"})
                ))
                .unwrap(),
                "not-json"
            ),
        )
        .unwrap();

        let report = recover_events(&events_path, &sqlite_path, None, true).unwrap();

        assert_eq!(report.valid_events, 1);
        assert_eq!(report.invalid_lines.len(), 1);
        assert!(report.replaced);
        assert_eq!(report.projected_events, 1);
        let backup_path = report.backup_path.expect("backup path");
        let backup = std::fs::read_to_string(backup_path).unwrap();
        assert!(backup.contains("\"id\":\"evt-failure\""));
        assert!(!backup.contains("not-json"));
        let recovered = std::fs::read_to_string(&events_path).unwrap();
        assert!(recovered.contains("\"id\":\"evt-failure\""));
        assert!(!recovered.contains("not-json"));
        let conn = rusqlite::Connection::open(sqlite_path).unwrap();
        let failure_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM failures", [], |row| row.get(0))
            .unwrap();
        assert_eq!(failure_count, 1);
    }

    #[test]
    fn recover_events_replace_writes_redacted_backup() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        std::fs::write(
            &events_path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&event(
                    "evt-secret-backup",
                    "FailureObserved",
                    "run-1",
                    json!({
                        "source": "recover",
                        "api_key": "sk-recoverybackupsecret123456789",
                        "error_preview": "Authorization: Bearer recoverybackupbearer123456"
                    })
                ))
                .unwrap(),
                "not-json"
            ),
        )
        .unwrap();

        let report = recover_events(&events_path, &sqlite_path, None, true).unwrap();

        let backup_path = report.backup_path.expect("backup path");
        let backup = std::fs::read_to_string(backup_path).unwrap();
        assert!(backup.contains("\"id\":\"evt-secret-backup\""));
        assert!(!backup.contains("sk-recoverybackupsecret123456789"));
        assert!(!backup.contains("recoverybackupbearer123456"));
        assert!(!backup.contains("not-json"));
        assert!(backup.contains("[redacted]"));
    }

    #[test]
    fn why_report_finds_last_failure() {
        let events = vec![
            event("evt-1", "RunStarted", "run-1", json!({"task": "x"})),
            event(
                "evt-2",
                "FailureObserved",
                "run-1",
                json!({"source": "tool", "error_preview": "boom"}),
            ),
        ];
        let report = build_why_report(&events, "last-failure").unwrap();
        assert!(report.contains("FailureObserved evt-2"));
        assert!(report.contains("boom"));
        assert!(report.contains("Related timeline"));
    }

    #[test]
    fn why_report_explains_failure_with_context_hypothesis_and_patch() {
        let events = vec![
            event(
                "evt-context",
                "ContextBuilt",
                "run-1",
                json!({
                    "context_policy": "context_policy@v3",
                    "layout_version": "deepseek-native-v1",
                    "include_instruction_files": ["YOYO.md", "AGENTS.md"],
                    "included_blocks": ["system_contract", "recent_failures", "selected_files"]
                }),
            ),
            event(
                "evt-failure",
                "FailureObserved",
                "run-1",
                json!({
                    "source": "test",
                    "error_preview": "missing retry_state.rs in context"
                }),
            ),
            event(
                "evt-hypothesis",
                "HypothesisCreated",
                "run-1",
                json!({
                    "hypothesis_id": "hyp-1",
                    "failure_event_id": "evt-failure",
                    "summary": "repair plan missed the failing test file",
                    "confidence": 0.8
                }),
            ),
            event(
                "evt-patch",
                "PatchProposed",
                "run-1",
                json!({
                    "patch_id": "context-policy-v4",
                    "kind": "context_policy",
                    "risk_level": "medium",
                    "intent": "include files referenced by failing test output",
                    "evidence_event_ids": ["evt-failure"]
                }),
            ),
        ];

        let report = build_why_report(&events, "last-failure").unwrap();

        assert!(report.contains("Explanation"));
        assert!(report.contains("failure source: test"));
        assert!(report.contains("active context: policy=context_policy@v3"));
        assert!(report.contains("instructions=[YOYO.md, AGENTS.md]"));
        assert!(report.contains("blocks=[system_contract, recent_failures, selected_files]"));
        assert!(report.contains("hyp-1: repair plan missed the failing test file"));
        assert!(report.contains("context-policy-v4"));
        assert!(report.contains("include files referenced by failing test output"));
    }

    #[test]
    fn policy_report_summarizes_context_and_schema_lineage() {
        let events = vec![
            event(
                "evt-context",
                "ContextBuilt",
                "run-1",
                json!({
                    "context_policy": "deepseek_native",
                    "layout_version": 7,
                    "include_instruction_files": ["YOYO.md", "AGENTS.md"],
                    "stable_prefix_blocks": ["deepseek_native_system_contract", "strict_tool_schemas"],
                    "dynamic_suffix_blocks": ["selected_recent_events", "failure_evidence"],
                    "included_blocks": [
                        {"name": "deepseek_native_system_contract"},
                        {"name": "strict_tool_schemas"}
                    ]
                }),
            ),
            event(
                "evt-schema",
                "ToolSchemaFailure",
                "run-1",
                json!({
                    "source": "strict_tool_schema",
                    "schema_name": "propose_edit",
                    "schema_version": 1,
                    "valid": false,
                    "repair_action": "retry"
                }),
            ),
        ];

        let report = build_policy_report(&events, 10).unwrap();

        assert!(report.contains("State policies"));
        assert!(report.contains("context policy=deepseek_native layout=7"));
        assert!(report.contains("stable=2 dynamic=2"));
        assert!(report.contains("instructions=[YOYO.md, AGENTS.md]"));
        assert!(report.contains("included=[deepseek_native_system_contract, strict_tool_schemas]"));
        assert!(report.contains("schema name=propose_edit version=1"));
        assert!(report.contains("valid=false repair_action=retry"));
    }

    #[test]
    fn cache_recent_report_summarizes_limited_recent_metrics() {
        let events = vec![
            event_at(
                "evt-cache-old",
                "CacheMetricsRecorded",
                "run-1",
                1,
                json!({
                    "model": "deepseek-v4-flash",
                    "prompt_cache_hit_tokens": 10,
                    "prompt_cache_miss_tokens": 90
                }),
            ),
            event_at("evt-run", "RunStarted", "run-2", 2, json!({"task": "x"})),
            event_at(
                "evt-cache-new",
                "CacheMetricsRecorded",
                "run-2",
                3,
                json!({
                    "model": "deepseek-v4-pro",
                    "prompt_cache_hit_tokens": 40,
                    "prompt_cache_miss_tokens": 10,
                    "cache_hit_ratio": 0.8
                }),
            ),
        ];

        let report = build_cache_recent_report(&events, 1).unwrap();

        assert!(report.contains("State cache metrics"));
        assert!(report.contains("recent events: 1"));
        assert!(report.contains("hit tokens:    40"));
        assert!(report.contains("miss tokens:   10"));
        assert!(report.contains("hit ratio:     80.00%"));
        assert!(report.contains("evt-cache-new model=deepseek-v4-pro"));
        assert!(!report.contains("evt-cache-old"));
    }

    #[test]
    fn cache_recent_report_reads_canonical_yoagent_state_events() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let event = crate::state::StateEvent {
            event_id: "evt-cache".into(),
            event_type: crate::state::EventType::CacheMetricsRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-1".into()),
            session_id: None,
            trace_id: "trace-1".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "model": crate::deepseek::DEFAULT_MODEL,
                "prompt_cache_hit_tokens": 90,
                "prompt_cache_miss_tokens": 10
            }),
        };
        crate::state::append_event(&path, &event).unwrap();

        let events = read_events(&path).unwrap();
        let report = build_cache_recent_report(&events, 12).unwrap();

        assert!(report.contains("recent events: 1"));
        assert!(report.contains("hit ratio:     90.00%"));
        assert!(report.contains("evt-cache model=deepseek"));
    }

    #[test]
    fn failure_fix_report_maps_failure_classes_to_promoted_patches() {
        let events = vec![
            event(
                "evt-failure",
                "FailureObserved",
                "run-1",
                json!({
                    "source": "test",
                    "error_preview": "retry_state.rs missing from context"
                }),
            ),
            event(
                "evt-patch",
                "PatchProposed",
                "run-2",
                json!({
                    "patch_id": "context-policy-v4",
                    "kind": "context_policy",
                    "risk_level": "medium",
                    "intent": "include files referenced by failing test output",
                    "evidence_event_ids": ["evt-failure"]
                }),
            ),
            event(
                "evt-eval",
                "PatchEvaluated",
                "run-3",
                json!({
                    "patch_id": "context-policy-v4",
                    "eval_id": "eval-1",
                    "suite": "local-smoke",
                    "status": "passed",
                    "score": 0.94
                }),
            ),
            event(
                "evt-promoted",
                "PatchPromoted",
                "run-4",
                json!({
                    "patch_id": "context-policy-v4",
                    "reason": "reduced context misses in local smoke"
                }),
            ),
        ];

        let report = build_failure_fix_report(&events, Some("context_miss"), 10).unwrap();

        assert!(report.contains("State failure fixes"));
        assert!(report.contains("classes=[context_miss]"));
        assert!(report.contains("patch=context-policy-v4"));
        assert!(report.contains("kind=context_policy"));
        assert!(report.contains("include files referenced by failing test output"));
        assert!(report.contains("evt-failure: retry_state.rs missing from context"));
        assert!(report.contains("eval eval-1: suite=local-smoke status=passed score=0.940"));
        assert!(build_failure_fix_report(&events, Some("transport"), 10)
            .unwrap_err()
            .contains("transport"));
    }

    #[test]
    fn rollback_report_surfaces_failed_eval_and_revert_lineage() {
        let events = vec![
            event(
                "evt-patch",
                "PatchProposed",
                "run-1",
                json!({
                    "patch_id": "patch-risky",
                    "kind": "context_policy",
                    "risk_level": "medium",
                    "intent": "rank historical failures higher",
                    "base_git_commit": "abc123",
                    "rollback_plan": ["revert context ranking change"]
                }),
            ),
            event(
                "evt-eval",
                "PatchEvaluated",
                "run-2",
                json!({
                    "patch_id": "patch-risky",
                    "eval_id": "eval-risky",
                    "harness_version": "genome-v2",
                    "suite": "local-smoke",
                    "status": "failed",
                    "score": 0.40
                }),
            ),
            event(
                "evt-revert",
                "RevertPerformed",
                "run-3",
                json!({
                    "patch_id": "patch-risky",
                    "status": "reverted",
                    "reason": "candidate regressed local smoke",
                    "reverted_commit": "def456"
                }),
            ),
        ];

        let report = build_rollback_report(&events, 10).unwrap();

        assert!(report.contains("State rollback candidates"));
        assert!(report.contains("candidate rollback harness=genome-v2"));
        assert!(report.contains("patch=patch-risky"));
        assert!(report.contains("status=failed"));
        assert!(report.contains("eval=eval-risky"));
        assert!(report.contains("kind=context_policy"));
        assert!(report.contains("base=abc123"));
        assert!(report.contains("rollback_plan=[revert context ranking change]"));
        assert!(report.contains("already_reverted=evt-revert"));
        assert!(report.contains("reverted patch=patch-risky"));
        assert!(report.contains("reverted_commit=def456"));

        let payload = build_rollback_payload(&events, 10).unwrap();
        assert_eq!(payload["diagnostic"].as_str().unwrap(), "state_rollbacks");
        assert_eq!(payload["candidate_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["revert_count"].as_u64().unwrap(), 1);
        let rows = payload["rows"].as_array().unwrap();
        let candidate = rows
            .iter()
            .find(|row| row["kind"].as_str() == Some("candidate"))
            .unwrap();
        assert_eq!(candidate["patch_id"].as_str().unwrap(), "patch-risky");
        assert_eq!(candidate["eval_id"].as_str().unwrap(), "eval-risky");
        assert_eq!(candidate["harness_version"].as_str().unwrap(), "genome-v2");
        assert_eq!(candidate["patch_kind"].as_str().unwrap(), "context_policy");
        assert_eq!(candidate["risk"].as_str().unwrap(), "medium");
        assert_eq!(candidate["base_git_commit"].as_str().unwrap(), "abc123");
        assert_eq!(
            candidate["rollback_plan"][0].as_str().unwrap(),
            "revert context ranking change"
        );
        assert_eq!(
            candidate["already_reverted"].as_str().unwrap(),
            "evt-revert"
        );
        let revert = rows
            .iter()
            .find(|row| row["kind"].as_str() == Some("revert"))
            .unwrap();
        assert_eq!(revert["patch_id"].as_str().unwrap(), "patch-risky");
        assert_eq!(revert["reverted_commit"].as_str().unwrap(), "def456");
        assert_eq!(revert["event_id"].as_str().unwrap(), "evt-revert");
    }

    #[test]
    fn why_report_links_similar_failures_and_patch_outcomes() {
        let events = vec![
            event_at(
                "evt-old-failure",
                "FailureObserved",
                "run-old",
                1,
                json!({
                    "source": "test",
                    "error_preview": "retry_state.rs missing from context during timeout repair"
                }),
            ),
            event_at(
                "evt-old-patch",
                "PatchProposed",
                "run-old",
                2,
                json!({
                    "patch_id": "context-policy-v4",
                    "kind": "context_policy",
                    "risk_level": "medium",
                    "intent": "include files referenced by failing test output",
                    "evidence_event_ids": ["evt-old-failure"]
                }),
            ),
            event_at(
                "evt-old-eval",
                "PatchEvaluated",
                "run-old",
                3,
                json!({
                    "patch_id": "context-policy-v4",
                    "eval_id": "eval-1",
                    "suite": "local-smoke",
                    "status": "passed",
                    "score": 0.95
                }),
            ),
            event_at(
                "evt-old-promoted",
                "PatchPromoted",
                "run-old",
                4,
                json!({
                    "patch_id": "context-policy-v4",
                    "reason": "reduced context misses"
                }),
            ),
            event_at(
                "evt-new-failure",
                "FailureObserved",
                "run-new",
                10,
                json!({
                    "source": "test",
                    "error_preview": "retry_state.rs missing from context"
                }),
            ),
        ];

        let report = build_why_report(&events, "last-failure").unwrap();

        assert!(report.contains("similar failures"));
        assert!(report.contains("evt-old-failure"));
        assert!(report.contains("candidate patches"));
        assert!(report.contains("context-policy-v4"));
        assert!(report.contains("eval eval-1: suite=local-smoke status=passed score=0.950"));
        assert!(report.contains("decision: promoted reason=reduced context misses"));
        assert!(report.contains("next actions"));
        assert!(report.contains("yoyo state patches show context-policy-v4"));
        assert!(report.contains(
            "yoyo evolve harness promote context-policy-v4 --baseline-eval <baseline-eval> --candidate-eval eval-1"
        ));
    }

    #[test]
    fn why_report_suggests_patch_proposal_when_no_candidate_exists() {
        let events = vec![event(
            "evt-context-miss",
            "FailureObserved",
            "run-1",
            json!({
                "source": "test",
                "error_preview": "context omitted retry_state.rs"
            }),
        )];

        let report = build_why_report(&events, "last-failure").unwrap();

        assert!(report.contains("next actions"));
        assert!(report.contains("yoyo eval replay --from-state --limit 5"));
        assert!(report.contains(
            "yoyo evolve harness propose --from-state evt-context-miss --kind context_policy"
        ));
    }

    #[test]
    fn why_report_surfaces_promotion_metric_evidence() {
        let events = vec![
            event_at(
                "evt-old-failure",
                "FailureObserved",
                "run-old",
                1,
                json!({
                    "source": "test",
                    "error_preview": "retry_state.rs missing from context"
                }),
            ),
            event_at(
                "evt-patch",
                "PatchProposed",
                "run-old",
                2,
                json!({
                    "patch_id": "context-policy-v5",
                    "kind": "context_policy",
                    "risk_level": "medium",
                    "intent": "include failure-adjacent retry files",
                    "evidence_event_ids": ["evt-old-failure"]
                }),
            ),
            event_at(
                "evt-compare",
                "DecisionRecorded",
                "run-old",
                3,
                json!({
                    "patch_id": "context-policy-v5",
                    "decision_type": "harness_patch_comparison",
                    "decision": "not_eligible",
                    "rationale": "candidate token_total increases beyond 10% budget gate",
                    "promotion_decision": {
                        "eligible": false,
                        "criterion": null,
                        "reason": "candidate token_total increases beyond 10% budget gate",
                        "baseline_eval_id": "eval-base",
                        "candidate_eval_id": "eval-candidate",
                        "metric_evidence": {
                            "token_total": {
                                "baseline": 120,
                                "candidate": 220,
                                "delta": 100
                            },
                            "fixture_suite": {
                                "baseline": {
                                    "task_count": 238,
                                    "command_count": 476,
                                    "risk_labels": {
                                        "high": 4,
                                        "medium": 114,
                                        "low": 120
                                    }
                                },
                                "candidate": {
                                    "task_count": 239,
                                    "command_count": 478,
                                    "risk_labels": {
                                        "high": 4,
                                        "medium": 115,
                                        "low": 120
                                    }
                                }
                            },
                            "model_route_tasks": {
                                "baseline": {
                                    "memory_compression": 1,
                                    "root_cause": 3
                                },
                                "candidate": {
                                    "fim": 1,
                                    "root_cause": 2
                                }
                            },
                            "protocol_eval": {
                                "eval_id": "eval-protocol",
                                "status": "passed",
                                "created_at_ms": 99,
                                "git_dirty": false,
                                "protocol_checks": {
                                    "total": 5,
                                    "passes": 5,
                                    "strict": 1,
                                    "thinking": 1,
                                    "stream": 1,
                                    "json": 1,
                                    "transport": 1
                                }
                            },
                            "metrics": [
                                {
                                    "metric": "cost_usd",
                                    "baseline": 0.10,
                                    "candidate": 0.20,
                                    "delta": 0.10
                                },
                                {
                                    "metric": "model_calls",
                                    "baseline": 4,
                                    "candidate": 5,
                                    "delta": 1
                                }
                            ]
                        }
                    }
                }),
            ),
            event_at(
                "evt-new-failure",
                "FailureObserved",
                "run-new",
                10,
                json!({
                    "source": "test",
                    "error_preview": "retry_state.rs missing from context during repair"
                }),
            ),
        ];

        let report = build_why_report(&events, "last-failure").unwrap();

        assert!(report.contains("context-policy-v5"));
        assert!(report.contains(
            "promotion: not_eligible criterion=- baseline=eval-base candidate=eval-candidate"
        ));
        assert!(report.contains("candidate token_total increases beyond 10% budget gate"));
        assert!(report.contains("promotion metrics: token_total=+100 (120 -> 220)"));
        assert!(report.contains(
            "fixture_suite tasks=238 -> 239 commands=476 -> 478 risks=[high=4, low=120, medium=114] -> [high=4, low=120, medium=115]"
        ));
        assert!(report.contains(
            "model_routes [memory_compression=1, root_cause=3] -> [fim=1, root_cause=2]"
        ));
        assert!(report.contains(
            "protocol_eval id=eval-protocol status=passed dirty=no created_at_ms=99 checks=5/5 strict=1 thinking=1 stream=1 json=1 transport=1"
        ));
        assert!(report.contains("cost_usd=+0.100000 (0.100000 -> 0.200000)"));
    }

    #[test]
    fn why_report_explains_json_output_check_pass_evidence() {
        let events = vec![event_at(
            "evt-json-pass",
            "DecisionRecorded",
            "run-json",
            1,
            json!({
                "source": "json_output",
                "decision_type": "deepseek_json_output_check",
                "check": "json-check",
                "decision": "passed",
                "schema_name": "summary",
                "attempt_count": 2,
                "retry_used": true,
                "attempt_statuses": ["empty", "parsed"]
            }),
        )];

        let report = build_why_report(&events, "evt-json-pass").unwrap();

        assert!(report.contains("JSON output check: json-check decision=passed"));
        assert!(report.contains("schema: summary attempts=2 retry_used=yes"));
        assert!(report.contains("attempt statuses: empty, parsed"));
        assert!(report.contains(
            "inspect JSON output evidence: yoyo state graph evidence evt-json-pass --depth 2"
        ));
        assert!(report
            .contains("inspect schema lineage: yoyo state graph policies evt-json-pass --depth 2"));
        assert!(report.contains("rerun JSON protocol check: yoyo deepseek json-check"));
    }

    #[test]
    fn why_report_explains_strict_tool_call_check_pass_evidence() {
        let events = vec![event_at(
            "evt-strict-pass",
            "DecisionRecorded",
            "run-strict",
            1,
            json!({
                "source": "deepseek_protocol_check",
                "decision_type": "deepseek_strict_tool_call_check",
                "check": "test-tool-call",
                "decision": "passed",
                "schema_count": 3,
                "schema_names": ["inspect_file", "propose_edit", "record_failure"],
                "selected_tool_count": 2,
                "selected_tool_names": ["inspect_file", "propose_edit"],
                "model": "deepseek-v4-pro",
                "thinking": "enabled",
                "reasoning_effort": "high",
                "stream": false,
                "max_tokens": 512
            }),
        )];

        let report = build_why_report(&events, "evt-strict-pass").unwrap();

        assert!(report.contains("strict tool-call check: test-tool-call decision=passed"));
        assert!(
            report.contains("request policy: model=deepseek-v4-pro thinking=enabled effort=high")
        );
        assert!(report.contains("schemas: total=3 selected=2"));
        assert!(report.contains("selected tools: inspect_file, propose_edit"));
        assert!(report.contains("validated schemas: inspect_file, propose_edit, record_failure"));
        assert!(report.contains(
            "inspect strict tool-call evidence: yoyo state graph evidence evt-strict-pass --depth 2"
        ));
        assert!(report.contains(
            "inspect strict schema lineage: yoyo state graph policies evt-strict-pass --depth 2"
        ));
        assert!(report.contains(
            "rerun strict tool-call protocol check: yoyo deepseek test-tool-call --record --json"
        ));
    }

    #[test]
    fn why_report_explains_transport_policy_check_pass_evidence() {
        let events = vec![event_at(
            "evt-transport-pass",
            "DecisionRecorded",
            "run-transport",
            1,
            json!({
                "source": "deepseek_protocol_check",
                "decision_type": "deepseek_transport_policy_check",
                "check": "transport-check",
                "decision": "passed",
                "transport_class": "rate_limited",
                "status": 429,
                "attempt": 0,
                "max_retries": 2,
                "retryable": true,
                "next_backoff_ms": 1000,
                "reason": "rate limit response can be retried with bounded backoff",
                "error_preview": "rate limit"
            }),
        )];

        let report = build_why_report(&events, "evt-transport-pass").unwrap();

        assert!(report.contains("transport policy check: transport-check decision=passed"));
        assert!(report
            .contains("class: rate_limited status=429 retryable=yes attempt=0/2 backoff=1000ms"));
        assert!(report.contains("rate limit response can be retried with bounded backoff"));
        assert!(report.contains(
            "inspect transport evidence: yoyo state graph evidence evt-transport-pass --depth 2"
        ));
        assert!(report.contains(
            "inspect transport policy lineage: yoyo state graph policies evt-transport-pass --depth 2"
        ));
        assert!(report.contains(
            "rerun transport protocol check: yoyo deepseek transport-check --status 429 --error 'rate limit' --record --json"
        ));
    }

    #[test]
    fn why_report_explains_thinking_protocol_check_pass_evidence() {
        let events = vec![event_at(
            "evt-thinking-pass",
            "DecisionRecorded",
            "run-thinking",
            1,
            json!({
                "source": "deepseek_protocol_check",
                "decision_type": "deepseek_thinking_protocol_check",
                "check": "test-thinking",
                "decision": "passed",
                "diagnostic_source": "builtin-probe",
                "probe": {
                    "source": "builtin-probe",
                    "message_count": 2,
                    "assistant_tool_call_turns": 1,
                    "assistant_tool_call_turns_with_reasoning_content": 1,
                    "assistant_tool_call_turns_missing_reasoning_content": 0,
                    "tool_result_turns": 1
                }
            }),
        )];

        let report = build_why_report(&events, "evt-thinking-pass").unwrap();

        assert!(report.contains("thinking protocol check: test-thinking decision=passed"));
        assert!(report.contains(
            "probe: source=builtin-probe messages=2 assistant_tool_calls=1 reasoning_present=1 reasoning_missing=0 tool_results=1"
        ));
        assert!(report.contains(
            "inspect thinking evidence: yoyo state graph evidence evt-thinking-pass --depth 2"
        ));
        assert!(report.contains(
            "inspect thinking protocol policy: yoyo state graph policies evt-thinking-pass --depth 2"
        ));
        assert!(report.contains(
            "rerun thinking protocol check: yoyo deepseek test-thinking --record --json"
        ));
    }

    #[test]
    fn why_report_explains_streaming_protocol_check_pass_evidence() {
        let events = vec![event_at(
            "evt-stream-pass",
            "DecisionRecorded",
            "run-stream",
            1,
            json!({
                "source": "deepseek_protocol_check",
                "decision_type": "deepseek_streaming_protocol_check",
                "check": "stream-check",
                "decision": "passed",
                "content_chars": 4,
                "reasoning_content_chars": 16,
                "tool_call_count": 1,
                "finish_reason": "stop",
                "input_tokens": 12,
                "output_tokens": 3,
                "cache_hit_tokens": 8,
                "cache_miss_tokens": 4
            }),
        )];

        let report = build_why_report(&events, "evt-stream-pass").unwrap();

        assert!(report.contains("streaming protocol check: stream-check decision=passed"));
        assert!(
            report.contains("stream: finish=stop content_chars=4 reasoning_chars=16 tool_calls=1")
        );
        assert!(report.contains("usage: input=12 output=3 cache_hit=8 cache_miss=4"));
        assert!(report.contains(
            "inspect streaming evidence: yoyo state graph evidence evt-stream-pass --depth 2"
        ));
        assert!(report.contains(
            "inspect streaming protocol policy: yoyo state graph policies evt-stream-pass --depth 2"
        ));
        assert!(report.contains(
            "rerun streaming protocol check: yoyo deepseek stream-check --record --json"
        ));
    }

    #[test]
    fn why_report_treats_promotion_approvals_as_evidence() {
        let events = vec![
            event_at(
                "evt-approval",
                "HumanApprovalReceived",
                "run-1",
                1,
                json!({
                    "patch_id": "patch-risky",
                    "approval_scope": "harness_patch_promotion",
                    "reason": "approved after review"
                }),
            ),
            event_at(
                "evt-decision",
                "DecisionRecorded",
                "run-1",
                2,
                json!({
                    "patch_id": "patch-risky",
                    "decision_type": "harness_patch_promotion",
                    "decision": "promote",
                    "rationale": "candidate passed with approval",
                    "approval_event_ids": ["evt-approval"],
                    "promotion_decision": {
                        "eligible": true,
                        "criterion": "score_improved",
                        "reason": "candidate score improved"
                    }
                }),
            ),
        ];

        let report = build_why_report(&events, "evt-decision").unwrap();

        assert!(report.contains("Evidence:"));
        assert!(report.contains("HumanApprovalReceived"));
        assert!(report.contains("evt-approval"));
        assert!(report.contains("approved after review"));
    }

    #[test]
    fn why_report_explains_promotion_protocol_gate() {
        let events = vec![event_at(
            "evt-promote",
            "DecisionRecorded",
            "run-promote",
            1,
            json!({
                "patch_id": "patch-1",
                "decision_type": "harness_patch_promotion",
                "decision": "block_promotion",
                "rationale": "latest protocol eval is older than candidate eval",
                "eval_id": "eval-candidate",
                "promotion_decision": {
                    "eligible": false,
                    "criterion": null,
                    "reason": "latest protocol eval is older than candidate eval",
                    "baseline_eval_id": "eval-base",
                    "candidate_eval_id": "eval-candidate",
                    "protocol_eval_id": "eval-protocol-old"
                }
            }),
        )];

        let report = build_why_report(&events, "evt-promote").unwrap();

        assert!(report.contains("Explanation:"));
        assert!(report.contains(
            "promotion decision: type=harness_patch_promotion decision=block_promotion eligible=no criterion=-"
        ));
        assert!(report.contains(
            "patch: patch-1 eval=eval-candidate baseline=eval-base candidate=eval-candidate protocol=eval-protocol-old"
        ));
        assert!(report
            .contains("inspect protocol lineage: yoyo state graph signals evt-promote --depth 2"));
        assert!(report.contains(
            "inspect protocol eval evidence: yoyo state graph evals evt-promote --depth 2"
        ));
        assert!(report.contains("rerun protocol eval: yoyo eval run --suite protocol-deepseek"));
        assert!(report.contains("review patch lifecycle: yoyo state lineage patch-1"));
    }

    #[test]
    fn why_report_guides_promotion_fixture_risk_mismatch() {
        let events = vec![event_at(
            "evt-promote",
            "DecisionRecorded",
            "run-promote",
            1,
            json!({
                "patch_id": "patch-1",
                "decision_type": "harness_patch_promotion",
                "decision": "block_promotion",
                "reason": "baseline and candidate fixture suite risk-label coverage differ",
                "eval_id": "eval-candidate",
                "promotion_decision": {
                    "eligible": false,
                    "criterion": null,
                    "reason": "baseline and candidate fixture suite risk-label coverage differ",
                    "baseline_eval_id": "eval-base",
                    "candidate_eval_id": "eval-candidate",
                    "protocol_eval_id": null,
                    "metric_evidence": {
                        "fixture_suite": {
                            "baseline": {
                                "task_count": 243,
                                "command_count": 486,
                                "risk_labels": {
                                    "high": 4,
                                    "low": 123,
                                    "medium": 116
                                }
                            },
                            "candidate": {
                                "task_count": 243,
                                "command_count": 486,
                                "risk_labels": {
                                    "high": 3,
                                    "low": 123,
                                    "medium": 117
                                }
                            }
                        },
                        "model_route_tasks": {
                            "baseline": {
                                "memory_compression": 1,
                                "root_cause": 3
                            },
                            "candidate": {
                                "fim": 1,
                                "root_cause": 2
                            }
                        }
                    }
                }
            }),
        )];

        let report = build_why_report(&events, "evt-promote").unwrap();

        assert!(report.contains(
            "promotion metrics: fixture_suite tasks=243 -> 243 commands=486 -> 486 risks=[high=4, low=123, medium=116] -> [high=3, low=123, medium=117]"
        ));
        assert!(
            report.contains("compare fixture evals: yoyo eval compare eval-base eval-candidate")
        );
        assert!(report.contains(
            "inspect promotion decision: yoyo state graph decisions evt-promote --depth 2"
        ));
        assert!(report.contains(
            "inspect promotion fixture evidence: yoyo state graph evidence evt-promote --depth 2"
        ));
        assert!(report.contains("review patch lifecycle: yoyo state lineage patch-1"));
    }

    #[test]
    fn why_report_guides_dirty_promotion_eval_block() {
        let events = vec![event_at(
            "evt-promote",
            "DecisionRecorded",
            "run-promote",
            1,
            json!({
                "patch_id": "patch-1",
                "decision_type": "harness_patch_promotion",
                "decision": "block_promotion",
                "reason": "baseline eval was run from a dirty worktree",
                "eval_id": "eval-candidate",
                "promotion_decision": {
                    "eligible": false,
                    "criterion": null,
                    "reason": "baseline eval was run from a dirty worktree",
                    "baseline_eval_id": "eval-base",
                    "candidate_eval_id": "eval-candidate",
                    "protocol_eval_id": null,
                    "suite": "local-smoke"
                }
            }),
        )];

        let report = build_why_report(&events, "evt-promote").unwrap();

        assert!(report.contains("reason: baseline eval was run from a dirty worktree"));
        assert!(report.contains("inspect dirty baseline eval: yoyo eval report eval-base"));
        assert!(!report.contains("inspect dirty candidate eval: yoyo eval report eval-candidate"));
        assert!(report.contains(
            "inspect dirty eval signals: yoyo state graph signals evt-promote --depth 2"
        ));
        assert!(report
            .contains("inspect dirty eval evidence: yoyo state graph evals evt-promote --depth 2"));
        assert!(report.contains("rerun clean fixture eval: yoyo eval run --suite local-smoke"));
        assert!(report.contains("review patch lifecycle: yoyo state lineage patch-1"));
    }

    #[test]
    fn why_report_guides_promotion_required_gate_block() {
        let events = vec![event_at(
            "evt-promote",
            "DecisionRecorded",
            "run-promote",
            1,
            json!({
                "patch_id": "patch-1",
                "decision_type": "harness_patch_promotion",
                "decision": "block_promotion",
                "reason": "candidate eval is missing required gate evidence: cargo check",
                "eval_id": "eval-candidate",
                "promotion_decision": {
                    "eligible": false,
                    "criterion": null,
                    "reason": "candidate eval is missing required gate evidence: cargo check",
                    "baseline_eval_id": "eval-base",
                    "candidate_eval_id": "eval-candidate",
                    "protocol_eval_id": null,
                    "suite": "local-smoke"
                }
            }),
        )];

        let report = build_why_report(&events, "evt-promote").unwrap();

        assert!(report
            .contains("reason: candidate eval is missing required gate evidence: cargo check"));
        assert!(report.contains("inspect candidate gate evidence: yoyo eval report eval-candidate"));
        assert!(!report.contains("inspect baseline gate evidence: yoyo eval report eval-base"));
        assert!(report.contains("rerun missing promotion gates: cargo check"));
        assert!(report.contains(
            "inspect promotion gate evidence: yoyo state graph evals evt-promote --depth 2"
        ));
        assert!(report.contains(
            "inspect promotion decision: yoyo state graph decisions evt-promote --depth 2"
        ));
        assert!(report.contains("rerun promotion fixture eval: yoyo eval run --suite local-smoke"));
        assert!(report.contains("review patch lifecycle: yoyo state lineage patch-1"));
    }

    #[test]
    fn why_report_guides_promotion_budget_gate_block() {
        let events = vec![event_at(
            "evt-promote",
            "DecisionRecorded",
            "run-promote",
            1,
            json!({
                "patch_id": "patch-1",
                "decision_type": "harness_patch_promotion",
                "decision": "block_promotion",
                "reason": "candidate token_total increases beyond 10% budget gate",
                "eval_id": "eval-candidate",
                "promotion_decision": {
                    "eligible": false,
                    "criterion": null,
                    "reason": "candidate token_total increases beyond 10% budget gate",
                    "baseline_eval_id": "eval-base",
                    "candidate_eval_id": "eval-candidate",
                    "protocol_eval_id": null,
                    "suite": "local-smoke",
                    "metric_evidence": {
                        "token_total": {
                            "baseline": 120,
                            "candidate": 220,
                            "delta": 100
                        },
                        "metrics": [
                            {
                                "metric": "cost_usd",
                                "baseline": 0.10,
                                "candidate": 0.20,
                                "delta": 0.10
                            }
                        ]
                    }
                }
            }),
        )];

        let report = build_why_report(&events, "evt-promote").unwrap();

        assert!(report.contains("promotion metrics: token_total=+100 (120 -> 220)"));
        assert!(report.contains("cost_usd=+0.100000 (0.100000 -> 0.200000)"));
        assert!(
            report.contains("compare budget evidence: yoyo eval compare eval-base eval-candidate")
        );
        assert!(report.contains("inspect baseline budget eval: yoyo eval report eval-base"));
        assert!(report.contains("inspect candidate budget eval: yoyo eval report eval-candidate"));
        assert!(report.contains(
            "inspect promotion budget decision: yoyo state graph decisions evt-promote --depth 2"
        ));
        assert!(report.contains(
            "inspect promotion budget evals: yoyo state graph evals evt-promote --depth 2"
        ));
        assert!(report.contains("rerun budget fixture eval: yoyo eval run --suite local-smoke"));
        assert!(report.contains("review patch lifecycle: yoyo state lineage patch-1"));
    }

    #[test]
    fn why_report_guides_promotion_harness_quality_regression_block() {
        let events = vec![event_at(
            "evt-promote",
            "DecisionRecorded",
            "run-promote",
            1,
            json!({
                "patch_id": "patch-1",
                "decision_type": "harness_patch_promotion",
                "decision": "block_promotion",
                "reason": "candidate malformed_tool_call_rate regresses harness quality gate",
                "eval_id": "eval-candidate",
                "promotion_decision": {
                    "eligible": false,
                    "criterion": null,
                    "reason": "candidate malformed_tool_call_rate regresses harness quality gate",
                    "baseline_eval_id": "eval-base",
                    "candidate_eval_id": "eval-candidate",
                    "protocol_eval_id": null,
                    "suite": "local-smoke",
                    "metric_evidence": {
                        "metrics": [
                            {
                                "metric": "malformed_tool_call_rate",
                                "baseline": 0.10,
                                "candidate": 0.30,
                                "delta": 0.20
                            }
                        ]
                    }
                }
            }),
        )];

        let report = build_why_report(&events, "evt-promote").unwrap();

        assert!(report
            .contains("reason: candidate malformed_tool_call_rate regresses harness quality gate"));
        assert!(
            report.contains("compare harness quality: yoyo eval compare eval-base eval-candidate")
        );
        assert!(report.contains("inspect candidate quality eval: yoyo eval report eval-candidate"));
        assert!(report.contains(
            "inspect promotion quality decision: yoyo state graph decisions evt-promote --depth 2"
        ));
        assert!(report.contains(
            "inspect promotion quality evals: yoyo state graph evals evt-promote --depth 2"
        ));
        assert!(report.contains("rerun quality fixture eval: yoyo eval run --suite local-smoke"));
        assert!(report.contains("review patch lifecycle: yoyo state lineage patch-1"));
    }

    #[test]
    fn why_report_guides_promotion_rollback_plan_block() {
        let events = vec![event_at(
            "evt-promote",
            "DecisionRecorded",
            "run-promote",
            1,
            json!({
                "patch_id": "patch-1",
                "decision_type": "harness_patch_promotion",
                "decision": "needs_human",
                "reason": "patch has no rollback plan",
                "eval_id": "eval-candidate",
                "promotion_decision": {
                    "eligible": true,
                    "criterion": "pass_rate_improved",
                    "reason": "candidate pass rate improved without regression",
                    "baseline_eval_id": "eval-base",
                    "candidate_eval_id": "eval-candidate",
                    "protocol_eval_id": null,
                    "suite": "local-smoke"
                },
                "safety_gate": {
                    "allowed": false,
                    "requires_human_approval": false,
                    "reason": "patch has no rollback plan",
                    "patch_kind": "context_policy",
                    "risk_level": "low",
                    "approval_event_ids": []
                },
                "approval_event_ids": []
            }),
        )];

        let report = build_why_report(&events, "evt-promote").unwrap();

        assert!(report.contains("safety: allowed=false human_approval_required=false"));
        assert!(report.contains("inspect rollback plan: yoyo state patches show patch-1"));
        assert!(report.contains(
            "propose rollback-safe patch update: yoyo evolve harness propose --from-state evt-promote --rollback-plan <step>"
        ));
        assert!(report.contains(
            "inspect promotion safety decision: yoyo state graph decisions evt-promote --depth 2"
        ));
        assert!(report.contains("review patch lifecycle: yoyo state lineage patch-1"));
    }

    #[test]
    fn why_report_guides_promotion_human_approval_block() {
        let events = vec![event_at(
            "evt-promote",
            "DecisionRecorded",
            "run-promote",
            1,
            json!({
                "patch_id": "patch-risky",
                "decision_type": "harness_patch_promotion",
                "decision": "needs_human",
                "reason": "high-risk or safety patch has no fresh HumanApprovalReceived event for promotion",
                "eval_id": "eval-candidate",
                "promotion_decision": {
                    "eligible": true,
                    "criterion": "pass_rate_improved",
                    "reason": "candidate pass rate improved without regression",
                    "baseline_eval_id": "eval-base",
                    "candidate_eval_id": "eval-candidate",
                    "protocol_eval_id": null,
                    "suite": "local-smoke"
                },
                "safety_gate": {
                    "allowed": false,
                    "requires_human_approval": true,
                    "reason": "high-risk or safety patch has no fresh HumanApprovalReceived event for promotion",
                    "patch_kind": "permission_policy",
                    "risk_level": "high",
                    "approval_event_ids": []
                },
                "approval_event_ids": []
            }),
        )];

        let report = build_why_report(&events, "evt-promote").unwrap();

        assert!(report.contains("safety: allowed=false human_approval_required=true"));
        assert!(report
            .contains("inspect promotion approval scope: yoyo state patches show patch-risky"));
        assert!(report.contains(
            "request promotion approval: yoyo evolve harness approve patch-risky --reason <text>"
        ));
        assert!(report.contains(
            "inspect promotion approval evidence: yoyo state graph decisions evt-promote --depth 2"
        ));
        assert!(report.contains("review patch lifecycle: yoyo state lineage patch-risky"));
    }

    #[test]
    fn why_report_guides_release_gate_protocol_block() {
        let events = vec![event_at(
            "evt-release-gate",
            "DecisionRecorded",
            "run-release",
            1,
            json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "max_age_hours": 1,
                "reason": "latest protocol eval is older than latest suite eval",
                "last_eval_id": "eval-suite-pass",
                "last_eval_status": "passed",
                "last_eval_git_dirty": false,
                "stale": false,
                "require_protocol": true,
                "protocol_eval_id": "eval-protocol-old",
                "protocol_eval_status": "passed",
                "protocol_eval_git_dirty": false,
                "protocol_stale": false,
                "protocol_older_than_eval": true,
                "protocol_check_counts": {
                    "total": 5,
                    "passes": 5,
                    "strict": 1,
                    "thinking": 1,
                    "stream": 1,
                    "json": 1,
                    "transport": 1
                },
                "source_provenance_passed": true,
                "source_provenance_findings": 0,
                "source_provenance_scan_source": "git",
                "source_provenance_scanned_files": 9,
                "source_provenance_skipped_files": 0
            }),
        )];

        let report = build_why_report(&events, "evt-release-gate").unwrap();

        assert!(report.contains(
            "protocol gate: required=yes id=eval-protocol-old status=passed dirty=no stale=no older_than_suite=yes checks=5/5 strict=1 thinking=1 stream=1 json=1 transport=1"
        ));
        assert!(report.contains(
            "inspect protocol evidence: yoyo state graph evals evt-release-gate --depth 2"
        ));
        assert!(report.contains(
            "inspect protocol signals: yoyo state graph signals evt-release-gate --depth 2"
        ));
        assert!(report.contains("rerun protocol eval: yoyo eval run --suite protocol-deepseek"));
        assert!(report.contains(
            "rerun release gate: yoyo eval release-gate --suite local-smoke --max-age-hours 1"
        ));
    }

    #[test]
    fn why_report_guides_dirty_release_gate_eval_block() {
        let events = vec![event_at(
            "evt-release-gate",
            "DecisionRecorded",
            "run-release",
            1,
            json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "max_age_hours": 2,
                "reason": "latest eval or required protocol eval was run from a dirty worktree",
                "last_eval_id": "eval-suite-dirty",
                "last_eval_status": "passed",
                "last_eval_git_dirty": true,
                "stale": false,
                "require_protocol": true,
                "protocol_eval_id": "eval-protocol-dirty",
                "protocol_eval_status": "passed",
                "protocol_eval_git_dirty": true,
                "protocol_stale": false,
                "protocol_older_than_eval": false,
                "source_provenance_passed": true,
                "source_provenance_findings": 0,
                "source_provenance_scan_source": "git",
                "source_provenance_scanned_files": 9,
                "source_provenance_skipped_files": 0
            }),
        )];

        let report = build_why_report(&events, "evt-release-gate").unwrap();

        assert!(
            report.contains("latest eval: id=eval-suite-dirty status=passed dirty=yes stale=no")
        );
        assert!(report.contains(
            "protocol gate: required=yes id=eval-protocol-dirty status=passed dirty=yes"
        ));
        assert!(report.contains("inspect dirty suite eval: yoyo eval report eval-suite-dirty"));
        assert!(
            report.contains("inspect dirty protocol eval: yoyo eval report eval-protocol-dirty")
        );
        assert!(report.contains(
            "inspect dirty release eval signals: yoyo state graph signals evt-release-gate --depth 2"
        ));
        assert!(report.contains(
            "inspect dirty release eval evidence: yoyo state graph evals evt-release-gate --depth 2"
        ));
        assert!(report.contains("rerun clean suite eval: yoyo eval run --suite local-smoke"));
        assert!(
            report.contains("rerun clean protocol eval: yoyo eval run --suite protocol-deepseek")
        );
        assert!(!report.contains("rerun protocol eval: yoyo eval run --suite protocol-deepseek"));
        assert!(report.contains(
            "rerun release gate: yoyo eval release-gate --suite local-smoke --max-age-hours 2"
        ));
    }

    #[test]
    fn why_report_guides_stale_release_gate_eval_block() {
        let events = vec![event_at(
            "evt-release-gate",
            "DecisionRecorded",
            "run-release",
            1,
            json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "max_age_hours": 1,
                "reason": "latest eval is older than max age",
                "last_eval_id": "eval-suite-stale",
                "last_eval_status": "passed",
                "last_eval_git_dirty": false,
                "stale": true,
                "require_protocol": false,
                "source_provenance_passed": true,
                "source_provenance_findings": 0,
                "source_provenance_scan_source": "git",
                "source_provenance_scanned_files": 9,
                "source_provenance_skipped_files": 0
            }),
        )];

        let report = build_why_report(&events, "evt-release-gate").unwrap();

        assert!(
            report.contains("latest eval: id=eval-suite-stale status=passed dirty=no stale=yes")
        );
        assert!(report.contains("inspect stale suite eval: yoyo eval report eval-suite-stale"));
        assert!(report.contains(
            "inspect stale release eval signals: yoyo state graph signals evt-release-gate --depth 2"
        ));
        assert!(report.contains(
            "inspect stale release eval evidence: yoyo state graph evals evt-release-gate --depth 2"
        ));
        assert!(report.contains("rerun fresh suite eval: yoyo eval run --suite local-smoke"));
        assert!(report.contains(
            "rerun release gate: yoyo eval release-gate --suite local-smoke --max-age-hours 1"
        ));
    }

    #[test]
    fn why_report_guides_failed_release_gate_eval_block() {
        let events = vec![event_at(
            "evt-release-gate",
            "DecisionRecorded",
            "run-release",
            1,
            json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "max_age_hours": 1,
                "reason": "latest eval did not pass",
                "last_eval_id": "eval-suite-failed",
                "last_eval_status": "failed",
                "last_eval_git_dirty": false,
                "stale": false,
                "require_protocol": false,
                "source_provenance_passed": true,
                "source_provenance_findings": 0,
                "source_provenance_scan_source": "git",
                "source_provenance_scanned_files": 9,
                "source_provenance_skipped_files": 0
            }),
        )];

        let report = build_why_report(&events, "evt-release-gate").unwrap();

        assert!(
            report.contains("latest eval: id=eval-suite-failed status=failed dirty=no stale=no")
        );
        assert!(report.contains("inspect failed suite eval: yoyo eval report eval-suite-failed"));
        assert!(report.contains(
            "inspect failed release eval signals: yoyo state graph signals evt-release-gate --depth 2"
        ));
        assert!(report.contains(
            "inspect failed release eval evidence: yoyo state graph evals evt-release-gate --depth 2"
        ));
        assert!(report.contains("rerun suite eval: yoyo eval run --suite local-smoke"));
        assert!(report.contains(
            "rerun release gate: yoyo eval release-gate --suite local-smoke --max-age-hours 1"
        ));
    }

    #[test]
    fn why_report_guides_release_gate_replay_failure_block() {
        let events = vec![event_at(
            "evt-release-gate",
            "DecisionRecorded",
            "run-release",
            1,
            json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "max_age_hours": 1,
                "reason": "state failures were recorded after latest eval",
                "last_eval_id": "eval-suite-pass",
                "last_eval_status": "passed",
                "last_eval_git_dirty": false,
                "stale": false,
                "replay_failures_after_eval": 2,
                "replay_command": "yoyo eval replay --from-state --limit 2",
                "require_protocol": false,
                "source_provenance_passed": true,
                "source_provenance_findings": 0,
                "source_provenance_scan_source": "git",
                "source_provenance_scanned_files": 9,
                "source_provenance_skipped_files": 0
            }),
        )];

        let report = build_why_report(&events, "evt-release-gate").unwrap();

        assert!(
            report.contains("replay failures: 2 command=yoyo eval replay --from-state --limit 2")
        );
        assert!(report.contains("replay state failures: yoyo eval replay --from-state --limit 2"));
        assert!(report.contains(
            "inspect replay failure signals: yoyo state graph signals evt-release-gate --depth 2"
        ));
        assert!(report.contains(
            "inspect replay failure evidence: yoyo state graph failures evt-release-gate --depth 2"
        ));
        assert!(report.contains("review recent state failures: yoyo state failures --recent"));
        assert!(report.contains(
            "rerun release gate: yoyo eval release-gate --suite local-smoke --max-age-hours 1"
        ));
    }

    #[test]
    fn why_report_guides_release_gate_fixture_agent_scope_block() {
        let events = vec![event_at(
            "evt-release-gate",
            "DecisionRecorded",
            "run-release",
            1,
            json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "max_age_hours": 1,
                "reason": "latest eval has fixture agent mutation-scope failures: 1",
                "last_eval_id": "eval-suite-scope",
                "last_eval_status": "passed",
                "last_eval_git_dirty": false,
                "last_eval_mutation_scope_failures": 1,
                "last_eval_unexpected_changed_files": 3,
                "stale": false,
                "require_protocol": false,
                "source_provenance_passed": true,
                "source_provenance_findings": 0,
                "source_provenance_scan_source": "git",
                "source_provenance_scanned_files": 9,
                "source_provenance_skipped_files": 0
            }),
        )];

        let report = build_why_report(&events, "evt-release-gate").unwrap();

        assert!(report.contains("fixture agent scope: failures=1 unexpected_files=3"));
        assert!(
            report.contains("inspect fixture agent scope eval: yoyo eval report eval-suite-scope")
        );
        assert!(report.contains(
            "inspect fixture agent scope signals: yoyo state graph signals evt-release-gate --depth 2"
        ));
        assert!(report.contains(
            "inspect fixture agent changed files: yoyo state graph files evt-release-gate --depth 2"
        ));
        assert!(report.contains(
            "inspect fixture agent scope evidence: yoyo state graph evidence evt-release-gate --depth 2"
        ));
        assert!(report.contains("rerun fixture suite eval: yoyo eval run --suite local-smoke"));
        assert!(report.contains(
            "rerun release gate: yoyo eval release-gate --suite local-smoke --max-age-hours 1"
        ));
    }

    #[test]
    fn why_report_guides_release_gate_fixture_coverage_block() {
        let events = vec![event_at(
            "evt-release-gate",
            "DecisionRecorded",
            "run-release",
            1,
            json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "max_age_hours": 1,
                "reason": "latest eval fixture suite breadth is below required minimum",
                "last_eval_id": "eval-suite-narrow",
                "last_eval_status": "passed",
                "last_eval_git_dirty": false,
                "last_eval_fixture_task_count": 240,
                "last_eval_fixture_command_count": 480,
                "last_eval_fixture_risk_labels": {
                    "high": 4,
                    "low": 120,
                    "medium": 116
                },
                "last_eval_model_route_tasks": {
                    "memory_compression": 1,
                    "root_cause": 3
                },
                "min_fixture_task_count": 245,
                "min_fixture_command_count": 490,
                "min_fixture_risk_labels": {
                    "high": 5,
                    "medium": 100
                },
                "fixture_breadth_satisfied": false,
                "fixture_risk_satisfied": false,
                "stale": false,
                "require_protocol": false,
                "source_provenance_passed": true,
                "source_provenance_findings": 0,
                "source_provenance_scan_source": "git",
                "source_provenance_scanned_files": 9,
                "source_provenance_skipped_files": 0
            }),
        )];

        let report = build_why_report(&events, "evt-release-gate").unwrap();

        assert!(report.contains("fixture risks: high=4, low=120, medium=116"));
        assert!(report.contains("model routes: memory_compression=1, root_cause=3"));
        assert!(
            report.contains("fixture breadth gate: min_tasks=245 min_commands=490 satisfied=no")
        );
        assert!(report.contains("fixture risk gate: min=high=5, medium=100 satisfied=no"));
        assert!(
            report.contains("inspect fixture coverage eval: yoyo eval report eval-suite-narrow")
        );
        assert!(report.contains(
            "inspect fixture coverage signals: yoyo state graph signals evt-release-gate --depth 2"
        ));
        assert!(report.contains(
            "inspect fixture coverage evidence: yoyo state graph evidence evt-release-gate --depth 2"
        ));
        assert!(report.contains(
            "inspect fixture coverage evals: yoyo state graph evals evt-release-gate --depth 2"
        ));
        assert!(report.contains("rerun fixture coverage suite: yoyo eval run --suite local-smoke"));
        assert!(report.contains(
            "rerun release gate: yoyo eval release-gate --suite local-smoke --max-age-hours 1 --min-fixture-tasks 245 --min-fixture-commands 490 --min-fixture-high-risk 5 --min-fixture-medium-risk 100"
        ));
    }

    #[test]
    fn why_report_guides_release_gate_required_gate_block() {
        let events = vec![event_at(
            "evt-release-gate",
            "DecisionRecorded",
            "run-release",
            1,
            json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "max_age_hours": 1,
                "reason": "latest eval is missing required gate evidence: cargo fmt --check",
                "last_eval_id": "eval-suite-partial",
                "last_eval_status": "passed",
                "last_eval_git_dirty": false,
                "missing_required_gates": ["cargo fmt --check", "cargo check"],
                "stale": false,
                "require_protocol": false,
                "source_provenance_passed": true,
                "source_provenance_findings": 0,
                "source_provenance_scan_source": "git",
                "source_provenance_scanned_files": 9,
                "source_provenance_skipped_files": 0
            }),
        )];

        let report = build_why_report(&events, "evt-release-gate").unwrap();

        assert!(report.contains("missing gates: cargo fmt --check, cargo check"));
        assert!(report.contains("inspect required-gate eval: yoyo eval report eval-suite-partial"));
        assert!(report.contains(
            "inspect required-gate signals: yoyo state graph signals evt-release-gate --depth 2"
        ));
        assert!(report.contains(
            "inspect required-gate evidence: yoyo state graph evidence evt-release-gate --depth 2"
        ));
        assert!(report.contains(
            "inspect required-gate decision: yoyo state graph decisions evt-release-gate --depth 2"
        ));
        assert!(report.contains("rerun missing gates: cargo fmt --check && cargo check"));
        assert!(
            report.contains("rerun required-gate suite eval: yoyo eval run --suite local-smoke")
        );
        assert!(report.contains(
            "rerun release gate: yoyo eval release-gate --suite local-smoke --max-age-hours 1"
        ));
    }

    #[test]
    fn why_report_explains_release_gate_source_provenance_block() {
        let events = vec![event_at(
            "evt-release-gate",
            "DecisionRecorded",
            "run-release",
            1,
            json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "max_age_hours": 1,
                "reason": "source provenance audit did not pass",
                "last_eval_id": "eval-pass",
                "last_eval_status": "passed",
                "last_eval_git_dirty": false,
                "last_eval_fixture_task_count": 240,
                "last_eval_fixture_command_count": 480,
                "last_eval_fixture_risk_labels": {
                    "high": 4,
                    "low": 120,
                    "medium": 116
                },
                "min_fixture_task_count": 245,
                "min_fixture_command_count": 490,
                "min_fixture_risk_labels": {
                    "high": 5,
                    "medium": 100
                },
                "fixture_breadth_satisfied": false,
                "fixture_risk_satisfied": false,
                "missing_required_gates": ["cargo clippy --all-targets --all-features -- -D warnings"],
                "stale": false,
                "replay_failures_after_eval": 1,
                "replay_command": "yoyo eval replay --from-state --limit 1",
                "require_protocol": true,
                "protocol_eval_id": "eval-protocol",
                "protocol_eval_status": "passed",
                "protocol_eval_git_dirty": false,
                "protocol_stale": false,
                "protocol_older_than_eval": false,
                "protocol_check_counts": {
                    "total": 5,
                    "passes": 5,
                    "strict": 1,
                    "thinking": 1,
                    "stream": 1,
                    "json": 1,
                    "transport": 1
                },
                "source_provenance_passed": false,
                "source_provenance_findings": 1,
                "source_provenance_finding_summaries": [
                    "src/a.rs: source path escapes repository"
                ],
                "source_provenance_scan_source": "git",
                "source_provenance_scanned_files": 9,
                "source_provenance_skipped_files": 0
            }),
        )];

        let report = build_why_report(&events, "evt-release-gate").unwrap();

        assert!(report.contains("Explanation:"));
        assert!(report.contains("release decision: block_release"));
        assert!(report.contains("suite: local-smoke max_age_hours=1"));
        assert!(report.contains(
            "latest eval: id=eval-pass status=passed dirty=no stale=no fixture_tasks=240 fixture_commands=480"
        ));
        assert!(report.contains("fixture risks: high=4, low=120, medium=116"));
        assert!(
            report.contains("fixture breadth gate: min_tasks=245 min_commands=490 satisfied=no")
        );
        assert!(report.contains("fixture risk gate: min=high=5, medium=100 satisfied=no"));
        assert!(report.contains(
            "protocol gate: required=yes id=eval-protocol status=passed dirty=no stale=no older_than_suite=no checks=5/5 strict=1 thinking=1 stream=1 json=1 transport=1"
        ));
        assert!(report
            .contains("missing gates: cargo clippy --all-targets --all-features -- -D warnings"));
        assert!(
            report.contains("replay failures: 1 command=yoyo eval replay --from-state --limit 1")
        );
        assert!(report.contains("source audit: blocked findings=1 source=git scanned=9 skipped=0"));
        assert!(report.contains("source finding: src/a.rs: source path escapes repository"));
        assert!(report.contains(
            "inspect source audit policy: yoyo state graph policies evt-release-gate --depth 2"
        ));
        assert!(report.contains(
            "inspect source audit evidence: yoyo state graph evidence evt-release-gate --depth 2"
        ));
        assert!(report.contains(
            "inspect source audit signals: yoyo state graph signals evt-release-gate --depth 2"
        ));
        assert!(report.contains(
            "inspect source audit impact: yoyo state graph impact evt-release-gate --depth 2"
        ));
        assert!(report.contains(
            "inspect source audit decision: yoyo state graph decisions evt-release-gate --depth 2"
        ));
        assert!(report.contains(
            "rerun source audit release gate: yoyo eval release-gate --suite local-smoke --max-age-hours 1"
        ));
        assert!(report.contains(
            "rerun release gate: yoyo eval release-gate --suite local-smoke --max-age-hours 1 --min-fixture-tasks 245 --min-fixture-commands 490 --min-fixture-high-risk 5 --min-fixture-medium-risk 100"
        ));
    }

    #[test]
    fn why_report_explains_commit_and_revert_lineage() {
        let events = vec![
            event_at(
                "evt-commit",
                "CommitCreated",
                "run-commit",
                1,
                json!({
                    "commit": "abc123",
                    "branch": "deepseek-native-bootstrap",
                    "files": ["src/context.rs", "src/state.rs"],
                    "message": "rank failing files first"
                }),
            ),
            event_at(
                "evt-revert",
                "RevertPerformed",
                "run-revert",
                2,
                json!({
                    "commit": "def456",
                    "reverted_commit": "abc123",
                    "branch": "deepseek-native-bootstrap",
                    "files": ["src/context.rs"],
                    "reason": "candidate regressed local smoke"
                }),
            ),
        ];

        let commit_report = build_why_report(&events, "abc123").unwrap();
        assert!(commit_report.contains("State why: abc123"));
        assert!(commit_report.contains("event: CommitCreated evt-commit"));
        assert!(commit_report.contains("commit: abc123"));
        assert!(commit_report.contains("branch: deepseek-native-bootstrap"));
        assert!(commit_report.contains("modified files: src/context.rs, src/state.rs"));
        assert!(commit_report.contains("later reverts:"));
        assert!(commit_report.contains("evt-revert"));
        assert!(commit_report.contains("revert_commit=def456"));

        let revert_report = build_why_report(&events, "def456").unwrap();
        assert!(revert_report.contains("event: RevertPerformed evt-revert"));
        assert!(revert_report.contains("revert commit: def456"));
        assert!(revert_report.contains("reverted commit: abc123"));
        assert!(revert_report.contains("original:"));
        assert!(revert_report.contains("evt-commit"));
        assert!(revert_report.contains("affected files: src/context.rs"));
    }

    #[test]
    fn last_failure_includes_json_output_failures() {
        let events = vec![
            event(
                "evt-1",
                "FailureObserved",
                "run-1",
                json!({"error_preview": "older"}),
            ),
            event(
                "evt-2",
                "JsonOutputFailure",
                "run-1",
                json!({"operation": "structured-extraction", "attempts": []}),
            ),
        ];

        let report = build_why_report(&events, "last-failure").unwrap();
        assert!(report.contains("JsonOutputFailure evt-2"));
        assert!(report.contains("structured-extraction"));
    }

    #[test]
    fn why_report_suggests_alternatives_when_no_failure_found() {
        // Empty state: no events
        let events: Vec<Value> = vec![];
        let err = build_why_report(&events, "last-failure").unwrap_err();
        assert!(err.contains("no state event found for 'last-failure'"));
        assert!(
            err.contains("state tail --limit 5"),
            "should suggest state tail when empty, got: {err}"
        );

        // All green: successful sessions but no failures
        let events = vec![
            event("evt-start", "RunStarted", "run-1", json!({"task": "x"})),
            event(
                "evt-done",
                "RunCompleted",
                "run-1",
                json!({"status": "success"}),
            ),
        ];
        let err = build_why_report(&events, "last-failure").unwrap_err();
        assert!(
            err.contains("successful"),
            "should mention successful sessions, got: {err}"
        );
        assert!(
            err.contains("state crashes --limit 10"),
            "should suggest state crashes, got: {err}"
        );
        assert!(
            err.contains("state why last-crash"),
            "should suggest state why last-crash, got: {err}"
        );

        // Active incomplete run: RunStarted without RunCompleted
        let events = vec![
            event("evt-start", "RunStarted", "run-1", json!({"task": "x"})),
            event("evt-other", "DecisionRecorded", "run-1", json!({})),
        ];
        let err = build_why_report(&events, "last-failure").unwrap_err();
        assert!(
            err.contains("in progress"),
            "should mention session is in progress, got: {err}"
        );
        assert!(
            err.contains("run-1"),
            "should show incomplete run ID, got: {err}"
        );
        assert!(
            err.contains("1970-01-01"),
            "should show run start timestamp, got: {err}"
        );

        // Multiple incomplete runs: shows at most 5
        let mut events_multi: Vec<Value> = vec![];
        for i in 1..=7 {
            let run_id = format!("run-{i}");
            let ts = 10 * i;
            let ev = json!({
                "event_id": format!("evt-start-{i}"),
                "event_type": "RunStarted",
                "schema_version": 1,
                "timestamp_ms": ts,
                "actor": "harness",
                "run_id": &run_id,
                "session_id": null,
                "trace_id": "trace-1",
                "parent_event_ids": [],
                "payload": json!({"task": "x"}),
            });
            events_multi.push(ev);
        }
        let err = build_why_report(&events_multi, "last-failure").unwrap_err();
        assert!(
            err.contains("in progress"),
            "should mention session is in progress, got: {err}"
        );
        // Should show the first 5 incomplete runs
        let run_count = err.matches("run-").count();
        assert!(
            run_count >= 5,
            "should show at least 5 incomplete run references, got {run_count}: {err}"
        );
        // run-7 should NOT appear (beyond the cap of 5)
        assert!(
            !err.contains("run-7"),
            "should cap at 5 incomplete runs, got: {err}"
        );

        // Custom ID (not last-failure) keeps generic guidance
        let events = vec![
            event("evt-start", "RunStarted", "run-1", json!({"task": "x"})),
            event(
                "evt-done",
                "RunCompleted",
                "run-1",
                json!({"status": "success"}),
            ),
        ];
        let err = build_why_report(&events, "evt-nonexistent").unwrap_err();
        // Custom IDs get a generic suggestion, not the last-failure-specific alternatives
        assert!(err.contains("no state event found for 'evt-nonexistent'"));
    }

    #[test]
    fn lineage_report_links_patch_events_by_patch_id_and_evidence() {
        let events = vec![
            event(
                "evt-failure",
                "FailureObserved",
                "run-1",
                json!({"error": "boom"}),
            ),
            event(
                "evt-patch",
                "PatchProposed",
                "run-1",
                json!({
                    "patch_id": "patch-1",
                    "evidence_event_ids": ["evt-failure"],
                }),
            ),
            event(
                "evt-eval",
                "PatchEvaluated",
                "run-1",
                json!({"patch_id": "patch-1", "passed": true}),
            ),
        ];
        let report = build_lineage_report(&events, "patch-1").unwrap();
        assert!(report.contains("PatchProposed"));
        assert!(report.contains("PatchEvaluated"));

        let evidence_report = build_lineage_report(&events, "evt-failure").unwrap();
        assert!(evidence_report.contains("PatchProposed"));
    }

    #[test]
    fn lineage_report_links_commit_and_revert_events_by_commit_id() {
        let events = vec![
            event_at(
                "evt-commit",
                "CommitCreated",
                "run-commit",
                1,
                json!({
                    "commit": "abc123",
                    "branch": "deepseek-native-bootstrap",
                    "files": ["src/context.rs"]
                }),
            ),
            event_at(
                "evt-revert",
                "RevertPerformed",
                "run-revert",
                2,
                json!({
                    "commit": "def456",
                    "reverted_commit": "abc123",
                    "branch": "deepseek-native-bootstrap",
                    "files": ["src/context.rs"]
                }),
            ),
        ];

        let original = build_lineage_report(&events, "abc123").unwrap();
        assert!(original.contains("State lineage: abc123"));
        assert!(original.contains("CommitCreated"));
        assert!(original.contains("evt-commit"));
        assert!(original.contains("RevertPerformed"));
        assert!(original.contains("evt-revert"));

        let revert = build_lineage_report(&events, "def456").unwrap();
        assert!(revert.contains("CommitCreated"));
        assert!(revert.contains("RevertPerformed"));
    }

    #[test]
    fn patch_list_summarizes_latest_patch_status() {
        let events = vec![
            event(
                "evt-patch",
                "PatchProposed",
                "run-1",
                json!({
                    "patch_id": "patch-1",
                    "kind": "tool_schema",
                    "status": "proposed",
                    "risk_level": "medium",
                    "intent": "tighten patch schema",
                    "base_harness_version": "genome-v1",
                    "state_version": 1,
                    "base_git_commit": "abc123",
                }),
            ),
            event(
                "evt-eval",
                "PatchEvaluated",
                "run-1",
                json!({"patch_id": "patch-1", "passed": true}),
            ),
        ];
        let report = build_patch_list_report(&events, None).unwrap();
        assert!(report.contains("patch-1"));
        assert!(report.contains("evaluated"));
        assert!(report.contains("tool_schema"));
        assert!(report.contains("harness=genome-v1"));
        assert!(report.contains("state=1"));
        assert!(report.contains("base=abc123"));

        let filtered = build_patch_list_report(&events, Some("evaluated")).unwrap();
        assert!(filtered.contains("patch-1"));
        assert!(build_patch_list_report(&events, Some("rejected")).is_err());
    }

    #[test]
    fn patch_show_includes_payload_and_timeline() {
        let events = vec![
            event(
                "evt-failure",
                "FailureObserved",
                "run-1",
                json!({
                    "source": "test",
                    "error_preview": "context omitted retry_state.rs"
                }),
            ),
            event(
                "evt-patch",
                "PatchProposed",
                "run-1",
                json!({
                    "patch_id": "patch-1",
                    "kind": "prompt_policy",
                    "status": "proposed",
                    "risk_level": "low",
                    "intent": "prefer stable output",
                    "base_harness_version": "genome-v1",
                    "state_version": 1,
                    "base_git_commit": "abc123",
                    "evidence_event_ids": ["evt-failure"],
                    "expected_effects": [
                        "reduce malformed prompt output",
                        "keep prompt policy rollback simple"
                    ],
                    "eval_plan": [
                        "run local-smoke prompt policy fixtures",
                        "cargo check"
                    ],
                    "rollback_plan": ["restore previous prompt wording"]
                }),
            ),
            event(
                "evt-eval",
                "PatchEvaluated",
                "run-1",
                json!({
                    "patch_id": "patch-1",
                    "eval_id": "eval-1",
                    "suite": "local-smoke",
                    "status": "passed",
                    "score": 0.92
                }),
            ),
            event(
                "evt-reject",
                "PatchRejected",
                "run-1",
                json!({"patch_id": "patch-1", "reason": "too broad"}),
            ),
        ];
        let report = build_patch_show_report(&events, "patch-1").unwrap();
        assert!(report.contains("State patch: patch-1"));
        assert!(report.contains("status: rejected"));
        assert!(report.contains("prefer stable output"));
        assert!(report.contains("base harness: genome-v1"));
        assert!(report.contains("state version: 1"));
        assert!(report.contains("base:   abc123"));
        assert!(report.contains("evidence: evt-failure"));
        assert!(report.contains("context omitted retry_state.rs"));
        assert!(report.contains("expected effects:"));
        assert!(report.contains("- reduce malformed prompt output"));
        assert!(report.contains("- keep prompt policy rollback simple"));
        assert!(report.contains("eval plan:"));
        assert!(report.contains("- run local-smoke prompt policy fixtures"));
        assert!(report.contains("- cargo check"));
        assert!(report.contains("rollback plan:"));
        assert!(report.contains("restore previous prompt wording"));
        assert!(report.contains("Outcomes:"));
        assert!(report.contains("eval eval-1: suite=local-smoke status=passed score=0.920"));
        assert!(report.contains("decision: rejected reason=too broad"));
        assert!(report.contains("PatchRejected"));
    }

    #[test]
    fn patch_show_includes_promotion_safety_gate_evidence() {
        let events = vec![
            event(
                "evt-patch",
                "PatchProposed",
                "run-1",
                json!({
                    "patch_id": "patch-risky",
                    "kind": "safety",
                    "status": "proposed",
                    "risk_level": "high",
                    "intent": "tighten approval checks",
                    "rollback_plan": ["restore previous promotion policy"]
                }),
            ),
            event(
                "evt-eval",
                "PatchEvaluated",
                "run-1",
                json!({
                    "patch_id": "patch-risky",
                    "eval_id": "eval-candidate",
                    "suite": "local-smoke",
                    "status": "passed",
                    "score": 1.0
                }),
            ),
            event(
                "evt-promote",
                "PatchPromoted",
                "run-1",
                json!({
                    "patch_id": "patch-risky",
                    "reason": "candidate improved promotion auditability",
                    "approval_event_ids": ["evt-approval"],
                    "safety_gate": {
                        "allowed": true,
                        "requires_human_approval": true,
                        "reason": "fresh human approval event found for promotion",
                        "patch_kind": "safety",
                        "risk_level": "high",
                        "approval_event_ids": ["evt-approval"]
                    }
                }),
            ),
            event(
                "evt-decision",
                "DecisionRecorded",
                "run-1",
                json!({
                    "patch_id": "patch-risky",
                    "decision_type": "harness_patch_promotion",
                    "decision": "promote",
                    "rationale": "candidate improved promotion auditability",
                    "eval_id": "eval-candidate",
                    "approval_event_ids": ["evt-approval"],
                    "promotion_decision": {
                        "eligible": true,
                        "criterion": "score_improved",
                        "baseline_eval_id": "eval-base",
                        "candidate_eval_id": "eval-candidate",
                        "reason": "candidate score improved"
                    },
                    "safety_gate": {
                        "allowed": true,
                        "requires_human_approval": true,
                        "reason": "fresh human approval event found for promotion",
                        "patch_kind": "safety",
                        "risk_level": "high",
                        "approval_event_ids": ["evt-approval"]
                    }
                }),
            ),
        ];

        let report = build_patch_show_report(&events, "patch-risky").unwrap();

        assert!(report.contains("promotion: eligible criterion=score_improved"));
        assert!(report
            .contains("safety: allowed=true human_approval_required=true approvals=evt-approval"));
        assert!(report.contains("eval eval-candidate: suite=local-smoke status=passed score=1.000"));
    }

    #[test]
    fn eval_report_filters_by_harness_version_and_patch() {
        let events = vec![
            event(
                "evt-eval-1",
                "PatchEvaluated",
                "run-1",
                json!({
                    "eval_id": "eval-1",
                    "harness_version": "harness-a",
                    "patch_id": "patch-1",
                    "suite": "local-smoke",
                    "status": "passed",
                    "score": 0.9,
                    "passed": 12,
                    "failed": 1,
                    "failure_event_ids": ["evt-failure-context"],
                    "metrics": {
                        "artifact_uri": ".yoyo/state/artifacts/evals/eval-1.json",
                        "reproducibility": {
                            "git_dirty": true,
                            "agent_command_source": "default-agent",
                            "git_status_short": [" M src/commands_state.rs"],
                            "replay_command": "yoyo eval fixtures run --suite local-smoke"
                        },
                        "fixture_suite": {
                            "task_count": 2,
                            "command_count": 3,
                            "categories": {
                                "context-miss challenge": 1,
                                "schema/tool-call challenge": 1
                            },
                            "risk_labels": {
                                "medium": 1,
                                "high": 1
                            }
                        }
                    }
                }),
            ),
            event(
                "evt-eval-2",
                "PatchEvaluated",
                "run-1",
                json!({
                    "eval_id": "eval-2",
                    "harness_version": "harness-b",
                    "patch_id": "patch-2",
                    "suite": "local-smoke",
                    "status": "failed",
                    "score": 0.4
                }),
            ),
        ];
        let report = build_eval_report(&events, Some("harness-a"), Some("patch-1")).unwrap();
        assert!(report.contains("eval-1"));
        assert!(!report.contains("eval-2"));
        assert!(report.contains("score=0.900"));
        assert!(report.contains("passed=12 failed=1"));
        assert!(report.contains("failures=[evt-failure-context]"));
        assert!(report.contains("artifact=.yoyo/state/artifacts/evals/eval-1.json"));
        assert!(report.contains("dirty=yes"));
        assert!(report.contains("agent_source=default-agent"));
        assert!(report.contains("fixture_tasks=2"));
        assert!(report.contains("fixture_commands=3"));
        assert!(report.contains(
            "fixture_categories=[context-miss challenge=1, schema/tool-call challenge=1]"
        ));
        assert!(report.contains("fixture_risks=[high=1, medium=1]"));
    }

    #[test]
    fn graph_report_reads_sqlite_relations() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-patch".into(),
            event_type: crate::state::EventType::PatchProposed,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-1".into()),
            session_id: None,
            trace_id: "trace-1".into(),
            parent_event_ids: vec!["evt-parent".into()],
            payload: json!({
                "patch_id": "patch-1",
                "evidence_event_ids": ["evt-failure"],
                "status": "proposed"
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_report(&sqlite_path, "patch-1", 1).unwrap();

        assert!(report.contains("uses_patch"));
        assert!(report.contains("evt-patch"));
        let evidence_report = build_graph_report(&sqlite_path, "evt-failure", 1).unwrap();
        assert!(evidence_report.contains("addresses"));
        assert!(evidence_report.contains("supported_by"));
    }

    #[test]
    fn graph_report_traverses_bounded_relation_depth() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-patch".into(),
                event_type: crate::state::EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 1,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-1",
                    "evidence_event_ids": ["evt-failure"],
                    "status": "proposed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-eval".into(),
                event_type: crate::state::EventType::PatchEvaluated,
                schema_version: 1,
                timestamp_ms: 2,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "eval_id": "eval-1",
                    "patch_id": "patch-1",
                    "suite": "local-smoke",
                    "status": "passed",
                    "score": 1.0,
                    "passed": 1,
                    "failed": 0,
                    "metrics": {},
                    "failure_event_ids": [],
                    "created_at_ms": 2
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let shallow = build_graph_report(&sqlite_path, "evt-failure", 1).unwrap();
        assert!(shallow.contains("d1 patch-1 -[addresses]-> evt-failure"));
        assert!(shallow.contains("d1 patch-1 -[addresses]-> evt-failure (patch -> event)"));
        assert!(shallow.contains("d1 evt-patch -[supported_by]-> evt-failure"));
        assert!(!shallow.contains("records_eval"));

        let traversed = build_graph_report(&sqlite_path, "evt-failure", 4).unwrap();
        assert!(traversed.contains("State graph: evt-failure depth=4"));
        assert!(traversed.contains("d2 evt-patch -[uses_patch]-> patch-1"));
        assert!(traversed.contains("d2 evt-eval -[uses_patch]-> patch-1"));
        assert!(traversed.contains("d2 patch-1 -[tested_by]-> eval-1"));
        assert!(traversed.contains("d3 evt-eval -[records_eval]-> eval-1"));

        let summary = build_graph_summary_report(&sqlite_path, "evt-failure", 4).unwrap();
        assert!(summary.contains("State graph summary: evt-failure depth=4"));
        assert!(summary.contains("relations:"));
        assert!(summary.contains("addresses=1"));
        assert!(summary.contains("tested_by=1"));
        assert!(summary.contains("by depth:"));

        let payload = build_graph_summary_payload(&sqlite_path, "evt-failure", 4).unwrap();
        assert_eq!(payload["diagnostic"], "state_graph_summary");
        assert_eq!(payload["id"], "evt-failure");
        assert_eq!(payload["depth"], 4);
        assert!(payload["node_count"].as_u64().unwrap() > 0);
        assert!(payload["relation_count"].as_u64().unwrap() > 0);
        assert_eq!(payload["by_relation"]["addresses"], 1);
        assert_eq!(payload["by_relation"]["tested_by"], 1);
        assert_eq!(payload["by_depth"]["1"], 2);
        assert!(payload["by_node_kind"]["patch"].as_u64().unwrap() > 0);
    }

    #[test]
    fn graph_report_shows_release_source_provenance_endpoint_kinds() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "source provenance audit failed",
                "source_provenance_passed": false,
                "source_provenance_findings": 1,
                "source_provenance_scan_source": "git",
                "source_provenance_finding_summaries": [
                    "src/a.rs: source path escapes repository"
                ]
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_report(&sqlite_path, "evt-release-gate", 2).unwrap();

        assert!(report.contains(
            "evt-release-gate -[blocked_by_source_provenance_audit]-> release_source_provenance_audit (event -> policy)"
        ));
        assert!(report.contains(
            "evt-release-gate -[has_source_provenance_finding]-> source_provenance_finding:src/a.rs: source path escapes repository (event -> evidence)"
        ));
        assert!(report.contains(
            "source_provenance_finding:src/a.rs: source path escapes repository -[supports_source_provenance_audit]-> release_source_provenance_audit (evidence -> policy)"
        ));
    }

    #[test]
    fn graph_timeline_report_orders_reachable_relations_by_event_time() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-patch".into(),
                event_type: crate::state::EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 10,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-1",
                    "evidence_event_ids": ["evt-failure"],
                    "status": "proposed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-eval".into(),
                event_type: crate::state::EventType::PatchEvaluated,
                schema_version: 1,
                timestamp_ms: 20,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "eval_id": "eval-1",
                    "patch_id": "patch-1",
                    "suite": "local-smoke",
                    "status": "passed",
                    "score": 1.0,
                    "passed": 1,
                    "failed": 0,
                    "metrics": {},
                    "failure_event_ids": ["evt-failure"],
                    "created_at_ms": 20
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-decision".into(),
                event_type: crate::state::EventType::DecisionRecorded,
                schema_version: 1,
                timestamp_ms: 30,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "decision_id": "decision-1",
                    "decision": "promote",
                    "patch_id": "patch-1",
                    "reason": "eval passed"
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_timeline_report(&sqlite_path, "evt-failure", 4, 20).unwrap();

        assert!(report.contains("State graph timeline: evt-failure depth=4 limit=20"));
        assert!(report.contains("t=10 d1 PatchProposed patch-1 -[addresses]-> evt-failure"));
        assert!(report.contains("t=20"));
        assert!(report.contains("PatchEvaluated"));
        assert!(report.contains("t=30"));
        assert!(report.contains("DecisionRecorded"));
        let patch_pos = report.find("\n  t=10").unwrap();
        let eval_pos = report.find("\n  t=20").unwrap();
        let decision_pos = report.find("\n  t=30").unwrap();
        assert!(patch_pos < eval_pos);
        assert!(eval_pos < decision_pos);

        let payload = build_graph_timeline_payload(&sqlite_path, "evt-failure", 4, 20).unwrap();

        assert_eq!(
            payload["diagnostic"].as_str().unwrap(),
            "state_graph_timeline"
        );
        assert_eq!(payload["id"].as_str().unwrap(), "evt-failure");
        assert_eq!(payload["depth"].as_u64().unwrap(), 4);
        assert_eq!(payload["limit"].as_u64().unwrap(), 20);
        let timeline = payload["timeline"].as_array().unwrap();
        assert_eq!(
            payload["relation_count"].as_u64().unwrap(),
            timeline.len() as u64
        );
        let patch_row = timeline
            .iter()
            .position(|row| {
                row["event_id"].as_str() == Some("evt-patch")
                    && row["src_id"].as_str() == Some("patch-1")
                    && row["relation"].as_str() == Some("addresses")
                    && row["dst_id"].as_str() == Some("evt-failure")
            })
            .unwrap();
        let eval_row = timeline
            .iter()
            .position(|row| row["event_type"].as_str() == Some("PatchEvaluated"))
            .unwrap();
        let decision_row = timeline
            .iter()
            .position(|row| row["event_type"].as_str() == Some("DecisionRecorded"))
            .unwrap();
        assert!(patch_row < eval_row);
        assert!(eval_row < decision_row);
        assert_eq!(timeline[patch_row]["timestamp_ms"].as_i64().unwrap(), 10);
        assert_eq!(
            timeline[patch_row]["event_type"].as_str().unwrap(),
            "PatchProposed"
        );
        assert_eq!(timeline[eval_row]["timestamp_ms"].as_i64().unwrap(), 20);
        assert_eq!(timeline[decision_row]["timestamp_ms"].as_i64().unwrap(), 30);
        assert_eq!(
            timeline[decision_row]["event_type"].as_str().unwrap(),
            "DecisionRecorded"
        );
    }

    #[test]
    fn graph_evidence_report_filters_evidence_relations_with_event_provenance() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-patch".into(),
                event_type: crate::state::EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 10,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-1",
                    "evidence_event_ids": ["evt-failure"],
                    "status": "proposed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-eval".into(),
                event_type: crate::state::EventType::PatchEvaluated,
                schema_version: 1,
                timestamp_ms: 20,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "eval_id": "eval-1",
                    "patch_id": "patch-1",
                    "suite": "local-smoke",
                    "status": "failed",
                    "score": 0.5,
                    "passed": 1,
                    "failed": 1,
                    "metrics": {},
                    "failure_event_ids": ["evt-failure"],
                    "created_at_ms": 20
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-decision".into(),
                event_type: crate::state::EventType::DecisionRecorded,
                schema_version: 1,
                timestamp_ms: 30,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "decision_id": "decision-1",
                    "decision": "reject",
                    "patch_id": "patch-1",
                    "reason": "eval failed"
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_evidence_report(&sqlite_path, "evt-failure", 4, 20).unwrap();

        assert!(report.contains("State graph evidence: evt-failure depth=4 limit=20"));
        assert!(report.contains("by relation:"));
        assert!(report.contains("addresses=1"));
        assert!(report.contains("evaluated_failure=1"));
        assert!(report.contains("t=10 d1 PatchProposed patch-1 -[addresses]-> evt-failure"));
        assert!(
            report.contains("t=20 d1 PatchEvaluated evt-eval -[evaluated_failure]-> evt-failure")
        );
        assert!(!report.contains("uses_patch"));
        assert!(!report.contains("DecisionRecorded"));

        let payload = build_graph_evidence_payload(&sqlite_path, "evt-failure", 4, 20).unwrap();
        assert_eq!(
            payload["diagnostic"].as_str().unwrap(),
            "state_graph_evidence"
        );
        assert_eq!(payload["id"].as_str().unwrap(), "evt-failure");
        assert_eq!(payload["depth"].as_u64().unwrap(), 4);
        assert_eq!(payload["limit"].as_u64().unwrap(), 20);
        assert_eq!(payload["evidence_count"].as_u64().unwrap(), 2);
        assert_eq!(payload["relations"]["addresses"].as_u64().unwrap(), 1);
        assert_eq!(
            payload["relations"]["evaluated_failure"].as_u64().unwrap(),
            1
        );
        let evidence = payload["evidence"].as_array().unwrap();
        assert_eq!(evidence.len(), 2);
        assert_eq!(evidence[0]["timestamp_ms"].as_i64().unwrap(), 10);
        assert_eq!(evidence[0]["depth"].as_u64().unwrap(), 1);
        assert_eq!(evidence[0]["event_id"].as_str().unwrap(), "evt-patch");
        assert_eq!(evidence[0]["event_type"].as_str().unwrap(), "PatchProposed");
        assert_eq!(evidence[0]["src_id"].as_str().unwrap(), "patch-1");
        assert_eq!(evidence[0]["relation"].as_str().unwrap(), "addresses");
        assert_eq!(evidence[0]["dst_id"].as_str().unwrap(), "evt-failure");
        assert_eq!(evidence[0]["dst_kind"].as_str().unwrap(), "event");
    }

    #[test]
    fn graph_evidence_report_surfaces_source_provenance_findings() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 10,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "source provenance audit did not pass",
                "source_provenance_passed": false,
                "source_provenance_findings": 1,
                "source_provenance_finding_summaries": [
                    "src/a.rs: source path escapes repository"
                ],
                "source_provenance_scan_source": "git"
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_evidence_report(&sqlite_path, "evt-release-gate", 2, 20).unwrap();

        assert!(report.contains("blocked_by_source_provenance_audit=1"));
        assert!(report.contains("has_source_provenance_finding=1"));
        assert!(report.contains("supports_source_provenance_audit=2"));
        assert!(report.contains(
            "evt-release-gate -[blocked_by_source_provenance_audit]-> release_source_provenance_audit (policy)"
        ));
        assert!(report.contains(
            "evt-release-gate -[has_source_provenance_finding]-> source_provenance_finding:src/a.rs: source path escapes repository (evidence)"
        ));
        assert!(report.contains(
            "source_provenance_finding:src/a.rs: source path escapes repository -[supports_source_provenance_audit]-> release_source_provenance_audit (policy)"
        ));
        assert_eq!(
            infer_graph_node_kind(
                "source_provenance_finding:src/a.rs: source path escapes repository"
            ),
            "evidence"
        );
    }

    #[test]
    fn graph_evidence_report_surfaces_json_output_check_pass() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-json-pass".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 10,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-json".into()),
            session_id: None,
            trace_id: "trace-json".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "json_output",
                "decision_type": "deepseek_json_output_check",
                "check": "json-check",
                "decision": "passed",
                "schema_name": "summary",
                "attempt_count": 1,
                "retry_used": false,
                "attempt_statuses": ["parsed"]
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_evidence_report(&sqlite_path, "evt-json-pass", 2, 20).unwrap();

        assert!(report.contains("State graph evidence: evt-json-pass depth=2 limit=20"));
        assert!(report.contains("by relation: supports_json_output_check=1"));
        assert!(report.contains(
            "t=10 d1 DecisionRecorded summary -[supports_json_output_check]-> evt-json-pass (event) via evt-json-pass"
        ));
    }

    #[test]
    fn graph_evidence_report_surfaces_strict_tool_call_check_pass() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-strict-pass".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 10,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-strict".into()),
            session_id: None,
            trace_id: "trace-strict".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "deepseek_protocol_check",
                "decision_type": "deepseek_strict_tool_call_check",
                "check": "test-tool-call",
                "decision": "passed",
                "schema_count": 3,
                "schema_names": ["inspect_file", "propose_edit", "record_failure"],
                "selected_tool_count": 2,
                "selected_tool_names": ["inspect_file", "propose_edit"],
                "model": "deepseek-v4-pro",
                "thinking": "enabled",
                "reasoning_effort": "high",
                "stream": false,
                "max_tokens": 512
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_evidence_report(&sqlite_path, "evt-strict-pass", 2, 20).unwrap();

        assert!(report.contains("State graph evidence: evt-strict-pass depth=2 limit=20"));
        assert!(report.contains("by relation: supports_strict_tool_call_check=3"));
        assert!(report.contains(
            "t=10 d1 DecisionRecorded inspect_file -[supports_strict_tool_call_check]-> evt-strict-pass (event) via evt-strict-pass"
        ));
    }

    #[test]
    fn graph_evidence_report_surfaces_transport_policy_check_pass() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-transport-pass".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 10,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-transport".into()),
            session_id: None,
            trace_id: "trace-transport".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "deepseek_protocol_check",
                "decision_type": "deepseek_transport_policy_check",
                "check": "transport-check",
                "decision": "passed",
                "transport_class": "rate_limited",
                "status": 429,
                "attempt": 0,
                "max_retries": 2,
                "retryable": true,
                "next_backoff_ms": 1000,
                "reason": "rate limit response can be retried with bounded backoff",
                "error_preview": "rate limit"
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report =
            build_graph_evidence_report(&sqlite_path, "evt-transport-pass", 2, 20).unwrap();

        assert!(report.contains("State graph evidence: evt-transport-pass depth=2 limit=20"));
        assert!(report.contains("by relation: supports_transport_policy_check=1"));
        assert!(report.contains(
            "t=10 d1 DecisionRecorded transport_class:rate_limited -[supports_transport_policy_check]-> evt-transport-pass (event) via evt-transport-pass"
        ));
    }

    #[test]
    fn graph_evidence_report_surfaces_thinking_protocol_check_pass() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-thinking-pass".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 10,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-thinking".into()),
            session_id: None,
            trace_id: "trace-thinking".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "deepseek_protocol_check",
                "decision_type": "deepseek_thinking_protocol_check",
                "check": "test-thinking",
                "decision": "passed",
                "diagnostic_source": "builtin-probe",
                "probe": {
                    "source": "builtin-probe",
                    "message_count": 2,
                    "assistant_tool_call_turns": 1,
                    "assistant_tool_call_turns_with_reasoning_content": 1,
                    "assistant_tool_call_turns_missing_reasoning_content": 0,
                    "tool_result_turns": 1
                }
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_evidence_report(&sqlite_path, "evt-thinking-pass", 2, 20).unwrap();

        assert!(report.contains("State graph evidence: evt-thinking-pass depth=2 limit=20"));
        assert!(report.contains("by relation: supports_thinking_protocol_check=1"));
        assert!(report.contains(
            "t=10 d1 DecisionRecorded thinking_probe:builtin-probe -[supports_thinking_protocol_check]-> evt-thinking-pass (event) via evt-thinking-pass"
        ));
    }

    #[test]
    fn graph_evidence_report_surfaces_streaming_protocol_check_pass() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-stream-pass".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 10,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-stream".into()),
            session_id: None,
            trace_id: "trace-stream".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "deepseek_protocol_check",
                "decision_type": "deepseek_streaming_protocol_check",
                "check": "stream-check",
                "decision": "passed",
                "content_chars": 4,
                "reasoning_content_chars": 16,
                "tool_call_count": 1,
                "finish_reason": "stop",
                "input_tokens": 12,
                "output_tokens": 3,
                "cache_hit_tokens": 8,
                "cache_miss_tokens": 4
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_evidence_report(&sqlite_path, "evt-stream-pass", 2, 20).unwrap();

        assert!(report.contains("State graph evidence: evt-stream-pass depth=2 limit=20"));
        assert!(report.contains("by relation: supports_streaming_protocol_check=1"));
        assert!(report.contains(
            "t=10 d1 DecisionRecorded streaming_probe:stream-check -[supports_streaming_protocol_check]-> evt-stream-pass (event) via evt-stream-pass"
        ));
    }

    #[test]
    fn graph_summary_report_counts_release_source_provenance_node_kinds() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 10,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "source provenance audit did not pass",
                "source_provenance_passed": false,
                "source_provenance_findings": 1,
                "source_provenance_finding_summaries": [
                    "src/a.rs: source path escapes repository"
                ],
                "source_provenance_scan_source": "git"
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_summary_report(&sqlite_path, "evt-release-gate", 2).unwrap();

        assert!(report.contains("State graph summary: evt-release-gate depth=2"));
        assert!(report.contains("by dst kind:"));
        assert!(report.contains("by node kind:"));
        assert!(report.contains("evidence=1"));
        assert!(report.contains("event=1"));
        assert!(report.contains("policy=2"));
        assert!(report.contains("blocked_by_source_provenance_audit=1"));
        assert!(report.contains("has_source_provenance_finding=1"));
    }

    #[test]
    fn graph_summary_report_uses_relation_kinds_for_policy_nodes() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-context".into(),
            event_type: crate::state::EventType::ContextBuilt,
            schema_version: 1,
            timestamp_ms: 10,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-context".into()),
            session_id: None,
            trace_id: "trace-context".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "context_policy": "deepseek_native@v2",
                "layout_version": 7,
                "prompt_version": "deepseek_native_prompt@v7",
                "include_instruction_files": ["YOYO.md", "AGENTS.md"],
                "stable_prefix_blocks": ["deepseek_native_system_contract", "strict_tool_schemas"],
                "dynamic_suffix_blocks": ["selected_recent_events"],
                "schema_name": "edit_plan",
                "schema_version": 3
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_summary_report(&sqlite_path, "evt-context", 1).unwrap();

        assert!(report.contains("State graph summary: evt-context depth=1"));
        assert!(report.contains("by node kind:"));
        assert!(report.contains("context_policy=1"));
        assert!(report.contains("prompt_layout=1"));
        assert!(report.contains("prompt_version=1"));
        assert!(report.contains("context_block=3"));
        assert!(report.contains("instruction_file=2"));
        assert!(report.contains("tool_schema=1"));
        assert!(report.contains("tool_schema_version=1"));
        assert!(!report.contains("unknown=6"));
    }

    #[test]
    fn graph_evidence_report_surfaces_source_provenance_pass_status() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 10,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "allow_release",
                "suite": "local-smoke",
                "reason": "source provenance audit passed",
                "source_provenance_passed": true,
                "source_provenance_findings": 0,
                "source_provenance_scan_source": "git"
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_evidence_report(&sqlite_path, "evt-release-gate", 2, 20).unwrap();

        assert!(report.contains("passed_source_provenance_audit=1"));
        assert!(report.contains(
            "evt-release-gate -[passed_source_provenance_audit]-> release_source_provenance_audit (policy)"
        ));
        assert!(!report.contains("blocked_by_source_provenance_audit"));
    }

    #[test]
    fn graph_evidence_report_surfaces_release_gate_missing_required_gates() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 10,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "latest eval is missing required gate evidence",
                "missing_required_gates": ["cargo fmt --check"],
                "source_provenance_passed": true
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_evidence_report(&sqlite_path, "evt-release-gate", 2, 20).unwrap();

        assert!(report.contains("missing_required_gate=1"));
        assert!(report.contains(
            "evt-release-gate -[missing_required_gate]-> required_gate:cargo fmt --check (evidence)"
        ));
        assert!(!report.contains("blocks_release_gate"));
        assert_eq!(
            infer_graph_node_kind("required_gate:cargo fmt --check"),
            "evidence"
        );
    }

    #[test]
    fn graph_evidence_report_surfaces_release_gate_protocol_check_counts() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 10,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "release_ready",
                "suite": "local-smoke",
                "reason": "latest eval and protocol eval passed and are fresh",
                "require_protocol": true,
                "protocol_eval_id": "eval-protocol-pass",
                "protocol_eval_status": "passed",
                "protocol_eval_git_dirty": false,
                "protocol_stale": false,
                "protocol_older_than_eval": false,
                "protocol_check_counts": {
                    "total": 5,
                    "passes": 5,
                    "strict": 1,
                    "thinking": 1,
                    "stream": 1,
                    "json": 1,
                    "transport": 1
                }
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_evidence_report(&sqlite_path, "evt-release-gate", 2, 20).unwrap();

        assert!(report.contains("State graph evidence: evt-release-gate depth=2 limit=20"));
        assert!(report.contains("covers_protocol_check=5"));
        assert!(report.contains(
            "evt-release-gate -[covers_protocol_check]-> deepseek_protocol_check:streaming (evidence)"
        ));
        assert!(report.contains(
            "evt-release-gate -[covers_protocol_check]-> deepseek_protocol_check:transport_policy (evidence)"
        ));
    }

    #[test]
    fn graph_evidence_report_surfaces_release_gate_fixture_breadth_minimum() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 10,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "latest eval fixture suite breadth is below required minimum",
                "last_eval_fixture_task_count": 244,
                "last_eval_fixture_command_count": 488,
                "min_fixture_task_count": 245,
                "min_fixture_command_count": 490,
                "fixture_breadth_satisfied": false
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_evidence_report(&sqlite_path, "evt-release-gate", 2, 20).unwrap();

        assert!(report.contains("State graph evidence: evt-release-gate depth=2 limit=20"));
        assert!(report.contains("fixture_breadth_below_minimum=1"));
        assert!(report.contains(
            "evt-release-gate -[fixture_breadth_below_minimum]-> release_fixture_breadth_minimum (evidence)"
        ));
        assert!(!report.contains("blocks_release_gate"));
        assert_eq!(
            infer_graph_node_kind("release_fixture_breadth_minimum"),
            "evidence"
        );
    }

    #[test]
    fn graph_evidence_report_surfaces_promotion_fixture_risk_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-promote".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 10,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-promote".into()),
            session_id: None,
            trace_id: "trace-promote".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "harness_patch_promotion",
                "decision": "block_promotion",
                "patch_id": "patch-1",
                "reason": "baseline and candidate fixture suite risk-label coverage differ",
                "promotion_decision": {
                    "eligible": false,
                    "reason": "baseline and candidate fixture suite risk-label coverage differ",
                    "baseline_eval_id": "eval-base",
                    "candidate_eval_id": "eval-candidate"
                }
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_evidence_report(&sqlite_path, "evt-promote", 2, 20).unwrap();

        assert!(report.contains("State graph evidence: evt-promote depth=2 limit=20"));
        assert!(report.contains("promotion_fixture_risk_mismatch=1"));
        assert!(report.contains(
            "evt-promote -[promotion_fixture_risk_mismatch]-> promotion_fixture_risk_coverage (evidence)"
        ));
        assert!(!report.contains("blocks_promotion"));
        assert_eq!(
            infer_graph_node_kind("promotion_fixture_risk_coverage"),
            "evidence"
        );
    }

    #[test]
    fn graph_evidence_report_surfaces_release_gate_fixture_agent_mutation_scope() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 10,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "latest eval has fixture agent mutation-scope failures: 1",
                "last_eval_id": "eval-scope-fail",
                "last_eval_status": "passed",
                "last_eval_mutation_scope_failures": 1,
                "last_eval_unexpected_changed_files": 3,
                "source_provenance_passed": true
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_evidence_report(&sqlite_path, "evt-release-gate", 2, 20).unwrap();

        assert!(report.contains("State graph evidence: evt-release-gate depth=2 limit=20"));
        assert!(report.contains("fixture_agent_mutation_scope_block=1"));
        assert!(report.contains(
            "evt-release-gate -[fixture_agent_mutation_scope_block]-> release_fixture_agent_mutation_scope (evidence)"
        ));
        assert!(!report.contains("blocks_release_gate"));
        assert_eq!(
            infer_graph_node_kind("release_fixture_agent_mutation_scope"),
            "evidence"
        );
    }

    #[test]
    fn graph_hotspots_report_ranks_high_degree_projected_nodes() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-patch".into(),
                event_type: crate::state::EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 1,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-1",
                    "evidence_event_ids": ["evt-failure"],
                    "status": "proposed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-eval".into(),
                event_type: crate::state::EventType::PatchEvaluated,
                schema_version: 1,
                timestamp_ms: 2,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "eval_id": "eval-1",
                    "patch_id": "patch-1",
                    "suite": "local-smoke",
                    "status": "passed",
                    "score": 1.0,
                    "passed": 1,
                    "failed": 0,
                    "metrics": {},
                    "failure_event_ids": [],
                    "created_at_ms": 2
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-decision".into(),
                event_type: crate::state::EventType::DecisionRecorded,
                schema_version: 1,
                timestamp_ms: 3,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "decision_id": "decision-1",
                    "decision": "promote",
                    "patch_id": "patch-1",
                    "reason": "eval passed"
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report =
            crate::commands_state_graph::build_graph_hotspots_report(&sqlite_path, 3).unwrap();

        assert!(report.contains("State graph hotspots limit=3"));
        assert!(report.contains("patch-1"));
        assert!(report.contains("kind=patch"));
        assert!(report.contains("degree="));
        assert!(report.contains("uses_patch="));

        let payload =
            crate::commands_state_graph::build_graph_hotspots_payload(&sqlite_path, 3).unwrap();

        assert_eq!(payload["diagnostic"], "state_graph_hotspots");
        assert_eq!(payload["limit"], 3);
        assert_eq!(payload["hotspot_count"], 3);
        assert_eq!(payload["hotspots"][0]["id"], "patch-1");
        assert_eq!(payload["hotspots"][0]["kind"], "patch");
        assert!(payload["hotspots"][0]["degree"].as_u64().unwrap() > 0);
        assert!(
            payload["hotspots"][0]["relations"]["uses_patch"]
                .as_u64()
                .unwrap()
                > 0
        );
    }

    #[test]
    fn graph_clusters_report_groups_first_hop_neighborhoods() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-patch".into(),
                event_type: crate::state::EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 1,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-1",
                    "evidence_event_ids": ["evt-failure"],
                    "status": "proposed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-eval".into(),
                event_type: crate::state::EventType::PatchEvaluated,
                schema_version: 1,
                timestamp_ms: 2,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "eval_id": "eval-1",
                    "patch_id": "patch-1",
                    "suite": "local-smoke",
                    "status": "passed",
                    "score": 1.0,
                    "passed": 1,
                    "failed": 0,
                    "metrics": {},
                    "failure_event_ids": [],
                    "created_at_ms": 2
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_clusters_report(&sqlite_path, "evt-failure", 4).unwrap();

        assert!(report.contains("State graph clusters: evt-failure depth=4"));
        assert!(report.contains("clusters:"));
        assert!(report.contains("patch-1"));
        assert!(report.contains("via addresses"));
        assert!(report.contains("eval-1"));
        assert!(report.contains("tested_by=1"));
        assert!(report.contains("kinds="));

        let payload = build_graph_clusters_payload(&sqlite_path, "evt-failure", 4).unwrap();
        assert_eq!(payload["diagnostic"], "state_graph_clusters");
        assert_eq!(payload["id"], "evt-failure");
        assert_eq!(payload["depth"], 4);
        assert!(payload["cluster_count"].as_u64().unwrap() > 0);
        assert!(payload["clusters"]
            .as_array()
            .unwrap()
            .iter()
            .any(|cluster| cluster["seed"] == "patch-1"
                && cluster["seed_relation"] == "addresses"
                && cluster["relation_counts"]["tested_by"] == 1));
    }

    #[test]
    fn graph_clusters_report_uses_relation_kinds_for_policy_nodes() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-context".into(),
            event_type: crate::state::EventType::ContextBuilt,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-context".into()),
            session_id: None,
            trace_id: "trace-context".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "context_policy": "deepseek_native@v2",
                "layout_version": 7,
                "prompt_version": "deepseek_native_prompt@v7",
                "stable_prefix_blocks": ["deepseek_native_system_contract", "strict_tool_schemas"],
                "dynamic_suffix_blocks": ["selected_recent_events"],
                "schema_name": "edit_plan",
                "schema_version": 3
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_clusters_report(&sqlite_path, "evt-context", 1).unwrap();

        assert!(report.contains("State graph clusters: evt-context depth=1"));
        assert!(report.contains("deepseek_native@v2"));
        assert!(report.contains("kinds=context_policy=1"));
        assert!(report.contains("prompt_layout_v7"));
        assert!(report.contains("kinds=prompt_layout=1"));
        assert!(report.contains("deepseek_native_prompt@v7"));
        assert!(report.contains("kinds=prompt_version=1"));
        assert!(report.contains("edit_plan@v3"));
        assert!(report.contains("kinds=tool_schema_version=1"));
    }

    #[test]
    fn graph_impact_report_summarizes_patch_eval_decision_and_file_reachability() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-patch".into(),
                event_type: crate::state::EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 1,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-1",
                    "evidence_event_ids": ["evt-failure"],
                    "status": "proposed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-eval".into(),
                event_type: crate::state::EventType::PatchEvaluated,
                schema_version: 1,
                timestamp_ms: 2,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "eval_id": "eval-1",
                    "patch_id": "patch-1",
                    "suite": "local-smoke",
                    "status": "passed",
                    "score": 1.0,
                    "passed": 1,
                    "failed": 0,
                    "metrics": {},
                    "failure_event_ids": ["evt-failure"],
                    "created_at_ms": 2
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-decision".into(),
                event_type: crate::state::EventType::DecisionRecorded,
                schema_version: 1,
                timestamp_ms: 3,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "decision_id": "decision-1",
                    "decision": "promote",
                    "patch_id": "patch-1",
                    "reason": "eval passed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-file".into(),
                event_type: crate::state::EventType::FileEdited,
                schema_version: 1,
                timestamp_ms: 4,
                actor: crate::state::Actor::Tool,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "path": "src/context.rs",
                    "lines_added": 2,
                    "lines_removed": 1
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_impact_report(&sqlite_path, "evt-failure", 4).unwrap();

        assert!(report.contains("State graph impact: evt-failure depth=4"));
        assert!(report.contains("patches:     patch-1"));
        assert!(report.contains("evals:       eval-1"));
        assert!(report.contains("decisions:   decision-1"));
        assert!(report.contains("files:       src/context.rs"));
        assert!(report.contains("evidence:"));
        assert!(report.contains("evt-failure"));
        assert!(report.contains("positives:   validated_by=1"));
        assert!(report.contains("risks:       evaluated_failure=1"));
        assert!(report.contains("addresses=1"));

        let payload = build_graph_impact_payload(&sqlite_path, "evt-failure", 4).unwrap();
        assert_eq!(payload["diagnostic"], "state_graph_impact");
        assert_eq!(payload["id"], "evt-failure");
        assert_eq!(payload["depth"], 4);
        assert!(payload["node_count"].as_u64().unwrap() > 0);
        assert!(payload["relation_count"].as_u64().unwrap() > 0);
        assert_eq!(payload["patches"][0], "patch-1");
        assert_eq!(payload["evals"][0], "eval-1");
        assert_eq!(payload["decisions"][0], "decision-1");
        assert_eq!(payload["files"][0], "src/context.rs");
        assert!(payload["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("evt-failure")));
        assert_eq!(payload["positive_signals"]["validated_by"], 1);
        assert_eq!(payload["risk_signals"]["evaluated_failure"], 1);
        assert_eq!(payload["relations"]["addresses"], 1);
    }

    #[test]
    fn graph_impact_report_surfaces_release_gate_evidence_nodes() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "latest eval is missing required gate evidence",
                "missing_required_gates": ["cargo fmt --check"],
                "source_provenance_passed": true
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_impact_report(&sqlite_path, "evt-release-gate", 2).unwrap();

        assert!(report.contains("State graph impact: evt-release-gate depth=2"));
        assert!(report.contains("evidence:    evt-release-gate"));
        assert!(report.contains("required_gate:cargo fmt --check"));
        assert!(report.contains("risks:       blocks_release_gate=1, missing_required_gate=1"));
    }

    #[test]
    fn graph_impact_report_surfaces_release_gate_protocol_check_counts() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "release_ready",
                "suite": "local-smoke",
                "reason": "latest eval and protocol eval passed and are fresh",
                "require_protocol": true,
                "protocol_eval_id": "eval-protocol-pass",
                "protocol_eval_status": "passed",
                "protocol_eval_git_dirty": false,
                "protocol_stale": false,
                "protocol_older_than_eval": false,
                "protocol_check_counts": {
                    "total": 5,
                    "passes": 5,
                    "strict": 1,
                    "thinking": 1,
                    "stream": 1,
                    "json": 1,
                    "transport": 1
                }
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_impact_report(&sqlite_path, "evt-release-gate", 2).unwrap();

        assert!(report.contains("State graph impact: evt-release-gate depth=2"));
        assert!(report.contains("evidence:"));
        assert!(report.contains("deepseek_protocol_check:streaming"));
        assert!(report.contains("deepseek_protocol_check:transport_policy"));
        assert!(report.contains("positives:   covers_protocol_check=5"));
        assert!(report.contains("covers_protocol_check=5"));
    }

    #[test]
    fn graph_impact_report_surfaces_json_output_check_pass() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-json-pass".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 10,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-json".into()),
            session_id: None,
            trace_id: "trace-json".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "json_output",
                "decision_type": "deepseek_json_output_check",
                "check": "json-check",
                "decision": "passed",
                "schema_name": "summary",
                "attempt_count": 1,
                "retry_used": false,
                "attempt_statuses": ["parsed"]
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_impact_report(&sqlite_path, "evt-json-pass", 2).unwrap();

        assert!(report.contains("State graph impact: evt-json-pass depth=2"));
        assert!(report.contains("policies:    summary"));
        assert!(report.contains("evidence:    evt-json-pass"));
        assert!(report.contains("positives:   supports_json_output_check=1"));
    }

    #[test]
    fn graph_impact_report_surfaces_strict_tool_call_check_pass() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-strict-pass".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 10,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-strict".into()),
            session_id: None,
            trace_id: "trace-strict".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "deepseek_protocol_check",
                "decision_type": "deepseek_strict_tool_call_check",
                "check": "test-tool-call",
                "decision": "passed",
                "schema_count": 3,
                "schema_names": ["inspect_file", "propose_edit", "record_failure"],
                "selected_tool_count": 2,
                "selected_tool_names": ["inspect_file", "propose_edit"],
                "model": "deepseek-v4-pro",
                "thinking": "enabled",
                "reasoning_effort": "high",
                "stream": false,
                "max_tokens": 512
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_impact_report(&sqlite_path, "evt-strict-pass", 2).unwrap();

        assert!(report.contains("State graph impact: evt-strict-pass depth=2"));
        assert!(report.contains("policies:    inspect_file, propose_edit, record_failure"));
        assert!(report.contains("evidence:    evt-strict-pass"));
        assert!(report.contains("positives:   supports_strict_tool_call_check=3"));
    }

    #[test]
    fn graph_impact_report_surfaces_transport_policy_check_pass() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-transport-pass".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 10,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-transport".into()),
            session_id: None,
            trace_id: "trace-transport".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "deepseek_protocol_check",
                "decision_type": "deepseek_transport_policy_check",
                "check": "transport-check",
                "decision": "passed",
                "transport_class": "rate_limited",
                "status": 429,
                "attempt": 0,
                "max_retries": 2,
                "retryable": true,
                "next_backoff_ms": 1000,
                "reason": "rate limit response can be retried with bounded backoff",
                "error_preview": "rate limit"
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_impact_report(&sqlite_path, "evt-transport-pass", 2).unwrap();

        assert!(report.contains("State graph impact: evt-transport-pass depth=2"));
        assert!(report.contains("policies:    deepseek_transport_policy"));
        assert!(report.contains("evidence:"));
        assert!(report.contains("evt-transport-pass"));
        assert!(report.contains("transport_class:rate_limited"));
        assert!(report.contains("positives:   supports_transport_policy_check=1"));
    }

    #[test]
    fn graph_impact_report_surfaces_thinking_protocol_check_pass() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-thinking-pass".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 10,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-thinking".into()),
            session_id: None,
            trace_id: "trace-thinking".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "deepseek_protocol_check",
                "decision_type": "deepseek_thinking_protocol_check",
                "check": "test-thinking",
                "decision": "passed",
                "diagnostic_source": "builtin-probe",
                "probe": {
                    "source": "builtin-probe",
                    "message_count": 2,
                    "assistant_tool_call_turns": 1,
                    "assistant_tool_call_turns_with_reasoning_content": 1,
                    "assistant_tool_call_turns_missing_reasoning_content": 0,
                    "tool_result_turns": 1
                }
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_impact_report(&sqlite_path, "evt-thinking-pass", 2).unwrap();

        assert!(report.contains("State graph impact: evt-thinking-pass depth=2"));
        assert!(report.contains("policies:    deepseek_thinking_protocol_policy"));
        assert!(report.contains("evidence:"));
        assert!(report.contains("evt-thinking-pass"));
        assert!(report.contains("thinking_probe:builtin-probe"));
        assert!(report.contains("positives:   supports_thinking_protocol_check=1"));
    }

    #[test]
    fn graph_impact_report_surfaces_streaming_protocol_check_pass() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-stream-pass".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 10,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-stream".into()),
            session_id: None,
            trace_id: "trace-stream".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "deepseek_protocol_check",
                "decision_type": "deepseek_streaming_protocol_check",
                "check": "stream-check",
                "decision": "passed",
                "content_chars": 4,
                "reasoning_content_chars": 16,
                "tool_call_count": 1,
                "finish_reason": "stop",
                "input_tokens": 12,
                "output_tokens": 3,
                "cache_hit_tokens": 8,
                "cache_miss_tokens": 4
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_impact_report(&sqlite_path, "evt-stream-pass", 2).unwrap();

        assert!(report.contains("State graph impact: evt-stream-pass depth=2"));
        assert!(report.contains("policies:    deepseek_streaming_protocol_policy"));
        assert!(report.contains("evidence:"));
        assert!(report.contains("evt-stream-pass"));
        assert!(report.contains("streaming_probe:stream-check"));
        assert!(report.contains("positives:   supports_streaming_protocol_check=1"));
    }

    #[test]
    fn graph_impact_report_surfaces_release_gate_fixture_breadth_blocks() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "latest eval fixture suite breadth is below required minimum",
                "last_eval_fixture_task_count": 244,
                "last_eval_fixture_command_count": 488,
                "min_fixture_task_count": 245,
                "min_fixture_command_count": 490,
                "fixture_breadth_satisfied": false
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_impact_report(&sqlite_path, "evt-release-gate", 2).unwrap();

        assert!(report.contains("State graph impact: evt-release-gate depth=2"));
        assert!(report.contains("evidence:    evt-release-gate"));
        assert!(report.contains("release_fixture_breadth_minimum"));
        assert!(
            report.contains("risks:       blocks_release_gate=1, fixture_breadth_below_minimum=1")
        );
    }

    #[test]
    fn graph_impact_report_surfaces_release_gate_fixture_risk_blocks() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "latest eval fixture risk coverage is below required minimum",
                "last_eval_fixture_risk_labels": {
                    "high": 4,
                    "medium": 120
                },
                "min_fixture_risk_labels": {
                    "high": 5
                },
                "fixture_risk_satisfied": false
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_impact_report(&sqlite_path, "evt-release-gate", 2).unwrap();

        assert!(report.contains("State graph impact: evt-release-gate depth=2"));
        assert!(report.contains("evidence:    evt-release-gate"));
        assert!(report.contains("release_fixture_risk_minimum"));
        assert!(report
            .contains("risks:       blocks_release_gate=1, fixture_risk_coverage_below_minimum=1"));
    }

    #[test]
    fn graph_impact_report_surfaces_promotion_fixture_risk_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-promote".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-promote".into()),
            session_id: None,
            trace_id: "trace-promote".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "harness_patch_promotion",
                "decision": "block_promotion",
                "patch_id": "patch-1",
                "reason": "baseline and candidate fixture suite risk-label coverage differ",
                "promotion_decision": {
                    "eligible": false,
                    "reason": "baseline and candidate fixture suite risk-label coverage differ",
                    "baseline_eval_id": "eval-base",
                    "candidate_eval_id": "eval-candidate"
                }
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_impact_report(&sqlite_path, "evt-promote", 2).unwrap();

        assert!(report.contains("State graph impact: evt-promote depth=2"));
        assert!(report.contains("evidence:    evt-promote"));
        assert!(report.contains("promotion_fixture_risk_coverage"));
        assert!(
            report.contains("risks:       blocks_promotion=1, promotion_fixture_risk_mismatch=1")
        );
    }

    #[test]
    fn graph_impact_report_surfaces_release_gate_fixture_agent_mutation_scope_blocks() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "latest eval has fixture agent mutation-scope failures: 1",
                "last_eval_id": "eval-scope-fail",
                "last_eval_status": "passed",
                "last_eval_mutation_scope_failures": 1,
                "last_eval_unexpected_changed_files": 3,
                "source_provenance_passed": true
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_impact_report(&sqlite_path, "evt-release-gate", 2).unwrap();

        assert!(report.contains("State graph impact: evt-release-gate depth=2"));
        assert!(report.contains("evidence:    evt-release-gate"));
        assert!(report.contains("release_fixture_agent_mutation_scope"));
        assert!(report
            .contains("risks:       blocks_release_gate=1, fixture_agent_mutation_scope_block=1"));
    }

    #[test]
    fn graph_impact_report_surfaces_source_provenance_finding_evidence_nodes() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "source provenance audit failed",
                "source_provenance_passed": false,
                "source_provenance_findings": 1,
                "source_provenance_scan_source": "git",
                "source_provenance_finding_summaries": [
                    "src/a.rs: source path escapes repository"
                ]
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_impact_report(&sqlite_path, "evt-release-gate", 2).unwrap();

        assert!(report.contains("State graph impact: evt-release-gate depth=2"));
        assert!(report.contains("evidence:    evt-release-gate"));
        assert!(report
            .contains("policies:    release_source_provenance_audit, source_provenance_scan:git"));
        assert!(
            report.contains("source_provenance_finding:src/a.rs: source path escapes repository")
        );
        assert!(report.contains("blocked_by_source_provenance_audit=1"));
        assert!(report.contains("has_source_provenance_finding=1"));
    }

    #[test]
    fn graph_files_report_surfaces_reachable_file_relations() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-patch".into(),
                event_type: crate::state::EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 1,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-1",
                    "evidence_event_ids": ["evt-failure"],
                    "status": "proposed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-file".into(),
                event_type: crate::state::EventType::FileEdited,
                schema_version: 1,
                timestamp_ms: 2,
                actor: crate::state::Actor::Tool,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "path": "src/context.rs",
                    "lines_added": 2,
                    "lines_removed": 1
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_files_report(&sqlite_path, "evt-failure", 4, 20).unwrap();

        assert!(report.contains("State graph files: evt-failure depth=4 limit=20"));
        assert!(report.contains("files:       src/context.rs"));
        assert!(report.contains("modified=1"));
        assert!(report.contains("modified_file=1"));
        assert!(report.contains("FileEdited evt-file -[modified_file]-> src/context.rs"));
        assert!(report.contains("via evt-file"));

        let payload = build_graph_files_payload(&sqlite_path, "evt-failure", 4, 20).unwrap();
        assert_eq!(payload["diagnostic"].as_str().unwrap(), "state_graph_files");
        assert_eq!(payload["id"].as_str().unwrap(), "evt-failure");
        assert_eq!(payload["depth"].as_u64().unwrap(), 4);
        assert_eq!(payload["limit"].as_u64().unwrap(), 20);
        assert_eq!(payload["file_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["relation_count"].as_u64().unwrap(), 2);
        assert_eq!(payload["files"][0].as_str().unwrap(), "src/context.rs");
        assert_eq!(payload["relations"]["modified"].as_u64().unwrap(), 1);
        assert_eq!(payload["relations"]["modified_file"].as_u64().unwrap(), 1);
        let file_relations = payload["file_relations"].as_array().unwrap();
        assert_eq!(file_relations.len(), 2);
        let modified_file = file_relations
            .iter()
            .find(|row| row["relation"].as_str() == Some("modified_file"))
            .unwrap();
        assert_eq!(modified_file["event_id"].as_str().unwrap(), "evt-file");
        assert_eq!(modified_file["event_type"].as_str().unwrap(), "FileEdited");
        assert_eq!(modified_file["src_id"].as_str().unwrap(), "evt-file");
        assert_eq!(modified_file["dst_id"].as_str().unwrap(), "src/context.rs");
        assert_eq!(modified_file["dst_kind"].as_str().unwrap(), "file");
    }

    #[test]
    fn graph_evals_report_surfaces_reachable_eval_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-patch".into(),
                event_type: crate::state::EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 1,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-1",
                    "evidence_event_ids": ["evt-failure"],
                    "status": "proposed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-eval".into(),
                event_type: crate::state::EventType::PatchEvaluated,
                schema_version: 1,
                timestamp_ms: 2,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "eval_id": "eval-1",
                    "patch_id": "patch-1",
                    "harness_version": "genome-v1",
                    "suite": "local-smoke",
                    "status": "passed",
                    "score": 0.875,
                    "passed": 7,
                    "failed": 1,
                    "metrics": {
                        "fixture_suite": {
                            "task_count": 2,
                            "command_count": 3
                        },
                        "state_metrics": {
                            "deepseek_protocol_checks": 5,
                            "deepseek_protocol_passes": 5,
                            "deepseek_protocol_failures": 0,
                            "deepseek_strict_tool_call_checks": 1,
                            "deepseek_thinking_protocol_checks": 1,
                            "deepseek_streaming_protocol_checks": 1,
                            "deepseek_json_output_checks": 1,
                            "deepseek_transport_policy_checks": 1,
                            "model_route_tasks": {
                                "memory_compression": 1,
                                "root_cause": 2
                            }
                        }
                    },
                    "failure_event_ids": ["evt-failure"],
                    "created_at_ms": 2
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_evals_report(&sqlite_path, "evt-failure", 4, 20).unwrap();

        assert!(report.contains("State graph evals: evt-failure depth=4 limit=20"));
        assert!(report.contains("evals:       eval-1"));
        assert!(report.contains("tested_by=1"));
        assert!(report.contains("evaluated_failure=1"));
        assert!(report.contains("covers_protocol_check=5"));
        assert!(report.contains(
            "suite=local-smoke status=passed score=0.875 patch=patch-1 fixture_tasks=2 fixture_commands=3 model_routes=[memory_compression=1, root_cause=2] protocol_checks=5 protocol_passes=5 strict=1 thinking=1 stream=1 json=1 transport=1"
        ));
        assert!(report.contains("patch-1 -[tested_by]-> eval-1"));
        assert!(report.contains(
            "eval-1 -[covers_protocol_check]-> deepseek_protocol_check:thinking_protocol"
        ));
        assert!(report.contains("evt-eval -[evaluated_failure]-> evt-failure"));

        let payload = build_graph_evals_payload(&sqlite_path, "evt-failure", 4, 20).unwrap();
        assert_eq!(payload["diagnostic"].as_str().unwrap(), "state_graph_evals");
        assert_eq!(payload["id"].as_str().unwrap(), "evt-failure");
        assert_eq!(payload["depth"].as_u64().unwrap(), 4);
        assert_eq!(payload["limit"].as_u64().unwrap(), 20);
        assert_eq!(payload["eval_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["evals"][0].as_str().unwrap(), "eval-1");
        assert_eq!(payload["relations"]["tested_by"].as_u64().unwrap(), 1);
        assert_eq!(
            payload["relations"]["evaluated_failure"].as_u64().unwrap(),
            1
        );
        assert_eq!(
            payload["relations"]["covers_protocol_check"]
                .as_u64()
                .unwrap(),
            5
        );
        let metadata = &payload["eval_metadata"]["eval-1"];
        assert_eq!(metadata["suite"].as_str().unwrap(), "local-smoke");
        assert_eq!(metadata["status"].as_str().unwrap(), "passed");
        assert_eq!(metadata["score"].as_f64().unwrap(), 0.875);
        assert_eq!(metadata["patch_id"].as_str().unwrap(), "patch-1");
        assert_eq!(metadata["fixture_task_count"].as_u64().unwrap(), 2);
        assert_eq!(metadata["fixture_command_count"].as_u64().unwrap(), 3);
        assert_eq!(metadata["deepseek_protocol_checks"].as_u64().unwrap(), 5);
        assert_eq!(
            metadata["deepseek_strict_tool_call_checks"]
                .as_u64()
                .unwrap(),
            1
        );
        assert_eq!(metadata["model_route_tasks"]["root_cause"], json!(2));
        assert_eq!(
            metadata["model_route_tasks"]["memory_compression"],
            json!(1)
        );
        let eval_relations = payload["eval_relations"].as_array().unwrap();
        let protocol_row = eval_relations
            .iter()
            .find(|row| {
                row["relation"].as_str() == Some("covers_protocol_check")
                    && row["dst_id"].as_str() == Some("deepseek_protocol_check:thinking_protocol")
            })
            .unwrap();
        assert_eq!(protocol_row["eval_id"].as_str().unwrap(), "eval-1");
        let failure_row = eval_relations
            .iter()
            .find(|row| row["relation"].as_str() == Some("evaluated_failure"))
            .unwrap();
        assert_eq!(failure_row["event_id"].as_str().unwrap(), "evt-eval");
        assert_eq!(failure_row["eval_id"].as_str().unwrap(), "eval-1");
    }

    #[test]
    fn graph_evals_report_surfaces_fixture_agent_attempt_changed_files() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = crate::state::StateEvent {
            event_id: "evt-agent-eval".into(),
            event_type: crate::state::EventType::PatchEvaluated,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-agent-eval".into()),
            session_id: None,
            trace_id: "trace-agent-eval".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "eval_id": "eval-agent-1",
                "patch_id": "patch-agent-1",
                "harness_version": "genome-v1",
                "suite": "fixture-attempts:local-smoke",
                "status": "failed",
                "score": 0.0,
                "metrics": {
                    "fixture_suite": {
                        "task_count": 1,
                        "command_count": 2
                    },
                    "fixture_agent_attempts": [
                        {
                            "task_id": "ranked-context-file-selection",
                            "passed": false,
                            "changed_files": [
                                "src/context.rs",
                                "src/eval_fixtures.rs"
                            ],
                            "unexpected_changed_files": [
                                "src/eval_fixtures.rs"
                            ]
                        }
                    ]
                },
                "created_at_ms": 1
            }),
        };
        std::fs::write(
            &events_path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_evals_report(&sqlite_path, "eval-agent-1", 2, 20).unwrap();

        assert!(report.contains("State graph evals: eval-agent-1 depth=2 limit=20"));
        assert!(report.contains("evals:       eval-agent-1"));
        assert!(report.contains("agent_attempt_changed_file=2"));
        assert!(report.contains("agent_attempt_unexpected_file=1"));
        assert!(report.contains(
            "suite=fixture-attempts:local-smoke status=failed score=0.000 patch=patch-agent-1 fixture_tasks=1 fixture_commands=2 agent_changes=2 [src/context.rs, src/eval_fixtures.rs] unexpected_agent_changes=1 [src/eval_fixtures.rs]"
        ));
        assert!(report.contains("eval-agent-1 -[agent_attempt_changed_file]-> src/context.rs"));
        assert!(
            report.contains("eval-agent-1 -[agent_attempt_changed_file]-> src/eval_fixtures.rs")
        );
        assert!(
            report.contains("eval-agent-1 -[agent_attempt_unexpected_file]-> src/eval_fixtures.rs")
        );

        let evidence = build_graph_evidence_report(&sqlite_path, "eval-agent-1", 1, 20).unwrap();

        assert!(evidence.contains("State graph evidence: eval-agent-1 depth=1 limit=20"));
        assert!(evidence.contains("agent_attempt_changed_file=2"));
        assert!(evidence.contains("agent_attempt_unexpected_file=1"));
        assert!(evidence.contains("eval-agent-1 -[agent_attempt_changed_file]-> src/context.rs"));
        assert!(evidence
            .contains("eval-agent-1 -[agent_attempt_unexpected_file]-> src/eval_fixtures.rs"));

        let signals = build_graph_signals_report(&sqlite_path, "eval-agent-1", 1).unwrap();

        assert!(signals.contains("risks:     agent_attempt_unexpected_file=1"));
        assert!(signals
            .contains("d1 eval-agent-1 -[agent_attempt_unexpected_file]-> src/eval_fixtures.rs"));
    }

    #[test]
    fn graph_patches_report_surfaces_reachable_patch_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-patch".into(),
                event_type: crate::state::EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 1,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-1",
                    "kind": "context_policy",
                    "risk_level": "medium",
                    "base_harness_version": "genome-v1",
                    "state_version": 1,
                    "base_git_commit": "abc123",
                    "rollback_plan": ["revert context policy", "restore previous fixture baseline"],
                    "evidence_event_ids": ["evt-failure"],
                    "status": "proposed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-eval".into(),
                event_type: crate::state::EventType::PatchEvaluated,
                schema_version: 1,
                timestamp_ms: 2,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "eval_id": "eval-1",
                    "patch_id": "patch-1",
                    "suite": "local-smoke",
                    "status": "passed",
                    "score": 1.0,
                    "passed": 1,
                    "failed": 0,
                    "metrics": {},
                    "failure_event_ids": ["evt-failure"],
                    "created_at_ms": 2
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_patches_report(&sqlite_path, "evt-failure", 4, 20).unwrap();

        assert!(report.contains("State graph patches: evt-failure depth=4 limit=20"));
        assert!(report.contains("patches:    patch-1"));
        assert!(report.contains("addresses=1"));
        assert!(report.contains("tested_by=1"));
        assert!(report.contains(
            "status=evaluated kind=context_policy risk=medium harness=genome-v1 state=1 base=abc123 rollback_steps=2"
        ));
        assert!(report.contains("patch-1 -[addresses]-> evt-failure"));
        assert!(report.contains("patch-1 -[tested_by]-> eval-1"));

        let payload = build_graph_patches_payload(&sqlite_path, "evt-failure", 4, 20).unwrap();
        assert_eq!(
            payload["diagnostic"].as_str().unwrap(),
            "state_graph_patches"
        );
        assert_eq!(payload["id"].as_str().unwrap(), "evt-failure");
        assert_eq!(payload["depth"].as_u64().unwrap(), 4);
        assert_eq!(payload["limit"].as_u64().unwrap(), 20);
        assert_eq!(payload["patch_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["patches"][0].as_str().unwrap(), "patch-1");
        assert_eq!(payload["relations"]["addresses"].as_u64().unwrap(), 1);
        assert_eq!(payload["relations"]["tested_by"].as_u64().unwrap(), 1);
        let metadata = &payload["patch_metadata"]["patch-1"];
        assert_eq!(metadata["status"].as_str().unwrap(), "evaluated");
        assert_eq!(metadata["kind"].as_str().unwrap(), "context_policy");
        assert_eq!(metadata["risk_level"].as_str().unwrap(), "medium");
        assert_eq!(
            metadata["base_harness_version"].as_str().unwrap(),
            "genome-v1"
        );
        assert_eq!(metadata["state_version"].as_u64().unwrap(), 1);
        assert_eq!(metadata["base_git_commit"].as_str().unwrap(), "abc123");
        assert_eq!(metadata["rollback_plan_steps"].as_u64().unwrap(), 2);
        let patch_relations = payload["patch_relations"].as_array().unwrap();
        let tested_by = patch_relations
            .iter()
            .find(|row| row["relation"].as_str() == Some("tested_by"))
            .unwrap();
        assert_eq!(tested_by["patch_id"].as_str().unwrap(), "patch-1");
        assert_eq!(tested_by["src_id"].as_str().unwrap(), "patch-1");
        assert_eq!(tested_by["dst_id"].as_str().unwrap(), "eval-1");
        let addresses = patch_relations
            .iter()
            .find(|row| row["relation"].as_str() == Some("addresses"))
            .unwrap();
        assert_eq!(addresses["event_id"].as_str().unwrap(), "evt-patch");
        assert_eq!(addresses["patch_id"].as_str().unwrap(), "patch-1");
    }

    #[test]
    fn graph_decisions_report_surfaces_reachable_decision_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-patch".into(),
                event_type: crate::state::EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 1,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-1",
                    "evidence_event_ids": ["evt-failure"],
                    "status": "proposed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-decision".into(),
                event_type: crate::state::EventType::DecisionRecorded,
                schema_version: 1,
                timestamp_ms: 2,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "decision_id": "decision-1",
                    "decision_type": "harness_patch_promotion",
                    "decision": "promote",
                    "status": "recorded",
                    "patch_id": "patch-1",
                    "eval_id": "eval-1",
                    "rationale": "candidate improved auditability"
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_decisions_report(&sqlite_path, "evt-failure", 4, 20).unwrap();

        assert!(report.contains("State graph decisions: evt-failure depth=4 limit=20"));
        assert!(report.contains("decisions:  decision-1"));
        assert!(report.contains("records_decision=1"));
        assert!(report.contains("uses_patch=1"));
        assert!(report.contains(
            "type=harness_patch_promotion decision=promote status=recorded patch=patch-1 eval=eval-1"
        ));
        assert!(report.contains("evt-decision -[records_decision]-> decision-1"));

        let payload = build_graph_decisions_payload(&sqlite_path, "evt-failure", 4, 20).unwrap();
        assert_eq!(
            payload["diagnostic"].as_str().unwrap(),
            "state_graph_decisions"
        );
        assert_eq!(payload["id"].as_str().unwrap(), "evt-failure");
        assert_eq!(payload["depth"].as_u64().unwrap(), 4);
        assert_eq!(payload["limit"].as_u64().unwrap(), 20);
        assert_eq!(payload["decision_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["decisions"][0].as_str().unwrap(), "decision-1");
        assert_eq!(
            payload["relations"]["records_decision"].as_u64().unwrap(),
            1
        );
        assert_eq!(payload["relations"]["uses_patch"].as_u64().unwrap(), 1);
        let metadata = &payload["decision_metadata"]["decision-1"];
        assert_eq!(
            metadata["decision_type"].as_str().unwrap(),
            "harness_patch_promotion"
        );
        assert_eq!(metadata["decision"].as_str().unwrap(), "promote");
        assert_eq!(metadata["status"].as_str().unwrap(), "recorded");
        assert_eq!(metadata["patch_id"].as_str().unwrap(), "patch-1");
        assert_eq!(metadata["eval_id"].as_str().unwrap(), "eval-1");
        assert_eq!(
            metadata["reason"].as_str().unwrap(),
            "candidate improved auditability"
        );
        let decision_relations = payload["decision_relations"].as_array().unwrap();
        let records_decision = decision_relations
            .iter()
            .find(|row| row["relation"].as_str() == Some("records_decision"))
            .unwrap();
        assert_eq!(
            records_decision["decision_id"].as_str().unwrap(),
            "decision-1"
        );
        assert_eq!(
            records_decision["event_id"].as_str().unwrap(),
            "evt-decision"
        );
        assert_eq!(records_decision["dst_id"].as_str().unwrap(), "decision-1");
    }

    #[test]
    fn graph_decisions_report_surfaces_release_gate_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "source provenance audit did not pass",
                "missing_required_gates": ["cargo check"],
                "replay_failures_after_eval": 2,
                "last_eval_fixture_task_count": 240,
                "last_eval_fixture_command_count": 480,
                "last_eval_fixture_risk_labels": {
                    "high": 4,
                    "low": 120,
                    "medium": 116
                },
                "last_eval_model_route_tasks": {
                    "memory_compression": 1,
                    "root_cause": 3
                },
                "last_eval_mutation_scope_failures": 1,
                "last_eval_unexpected_changed_files": 3,
                "min_fixture_task_count": 245,
                "min_fixture_command_count": 490,
                "min_fixture_risk_labels": {
                    "high": 5,
                    "medium": 100
                },
                "fixture_breadth_satisfied": false,
                "fixture_risk_satisfied": false,
                "require_protocol": true,
                "protocol_eval_status": "passed",
                "protocol_eval_git_dirty": false,
                "protocol_check_counts": {
                    "total": 5,
                    "passes": 5,
                    "strict": 1,
                    "thinking": 1,
                    "stream": 1,
                    "json": 1,
                    "transport": 1
                },
                "source_provenance_passed": false,
                "source_provenance_findings": 1,
                "source_provenance_scan_source": "git"
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_decisions_report(&sqlite_path, "evt-release-gate", 2, 20).unwrap();

        assert!(report.contains("State graph decisions: evt-release-gate depth=2 limit=20"));
        assert!(report.contains("decisions:  evt-release-gate"));
        assert!(report.contains("type=release_gate decision=block_release"));
        assert!(report.contains(
            "suite=local-smoke missing_gates=1 replay_failures=2 fixture_tasks=240 fixture_commands=480 fixture_risks=[high=4, low=120, medium=116] model_routes=[memory_compression=1, root_cause=3] mutation_scope_failures=1 unexpected_files=3 min_fixture_tasks=245 min_fixture_commands=490 min_fixture_risks=[high=5, medium=100] fixture_breadth_ok=no fixture_risk_ok=no protocol_required=yes protocol_status=passed protocol_dirty=no protocol_checks=5/5 protocol_strict=1 protocol_thinking=1 protocol_stream=1 protocol_json=1 protocol_transport=1 source_audit=blocked source_findings=1 source_scan=git"
        ));
        assert!(report.contains("reason=source provenance audit did not pass"));
        let payload =
            build_graph_decisions_payload(&sqlite_path, "evt-release-gate", 2, 20).unwrap();
        let metadata = &payload["decision_metadata"]["evt-release-gate"];
        assert_eq!(
            metadata["last_eval_model_route_tasks"]["root_cause"],
            json!(3)
        );
        assert_eq!(
            metadata["last_eval_model_route_tasks"]["memory_compression"],
            json!(1)
        );
    }

    #[test]
    fn graph_decisions_report_surfaces_promotion_fixture_risk_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-promote".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-promote".into()),
            session_id: None,
            trace_id: "trace-promote".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "harness_patch_promotion",
                "decision": "block_promotion",
                "status": "recorded",
                "patch_id": "patch-risk-labels",
                "reason": "baseline and candidate fixture suite risk-label coverage differ",
                "promotion_decision": {
                    "eligible": false,
                    "criterion": null,
                    "reason": "baseline and candidate fixture suite risk-label coverage differ",
                    "baseline_eval_id": "eval-base",
                    "candidate_eval_id": "eval-candidate",
                    "protocol_eval_id": null,
                    "metric_evidence": {
                        "fixture_suite": {
                            "baseline": {
                                "task_count": 243,
                                "command_count": 486,
                                "risk_labels": {
                                    "high": 4,
                                    "low": 123,
                                    "medium": 116
                                }
                            },
                            "candidate": {
                                "task_count": 243,
                                "command_count": 486,
                                "risk_labels": {
                                    "high": 3,
                                    "low": 123,
                                    "medium": 117
                                }
                            }
                        },
                        "model_route_tasks": {
                            "baseline": {
                                "memory_compression": 1,
                                "root_cause": 3
                            },
                            "candidate": {
                                "fim": 1,
                                "root_cause": 2
                            }
                        }
                    }
                }
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_decisions_report(&sqlite_path, "evt-promote", 2, 20).unwrap();

        assert!(report.contains("State graph decisions: evt-promote depth=2 limit=20"));
        assert!(report.contains("type=harness_patch_promotion decision=block_promotion"));
        assert!(report.contains("promotion_eligible=no"));
        assert!(report.contains("baseline=eval-base candidate=eval-candidate"));
        assert!(report.contains("fixture_tasks=243->243 fixture_commands=486->486"));
        assert!(report.contains(
            "fixture_risks=[high=4, low=123, medium=116]->[high=3, low=123, medium=117]"
        ));
        assert!(report
            .contains("model_routes=[memory_compression=1, root_cause=3]->[fim=1, root_cause=2]"));
        assert!(report.contains(
            "promotion_reason=baseline and candidate fixture suite risk-label coverage differ"
        ));

        let payload = build_graph_decisions_payload(&sqlite_path, "evt-promote", 2, 20).unwrap();
        let metadata = &payload["decision_metadata"]["evt-promote"];
        assert_eq!(
            metadata["promotion_model_route_baseline"]["root_cause"],
            json!(3)
        );
        assert_eq!(metadata["promotion_model_route_candidate"]["fim"], json!(1));
    }

    #[test]
    fn graph_hypotheses_report_surfaces_reachable_cause_support_and_contradictions() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-failure".into(),
                event_type: crate::state::EventType::FailureObserved,
                schema_version: 1,
                timestamp_ms: 1,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "context",
                    "error_preview": "context omitted retry_state.rs"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-log".into(),
                event_type: crate::state::EventType::FailureObserved,
                schema_version: 1,
                timestamp_ms: 2,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "log",
                    "error_preview": "retry_state.rs missing from selected files"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-cache-ok".into(),
                event_type: crate::state::EventType::CacheMetricsRecorded,
                schema_version: 1,
                timestamp_ms: 3,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "model": "deepseek-v4-pro",
                    "prompt_cache_hit_tokens": 10,
                    "prompt_cache_miss_tokens": 0
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-hypothesis".into(),
                event_type: crate::state::EventType::HypothesisCreated,
                schema_version: 1,
                timestamp_ms: 4,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "hypothesis_id": "hyp-context-miss",
                    "failure_event_id": "evt-failure",
                    "summary": "context selector omitted retry_state.rs",
                    "evidence_event_ids": ["evt-failure", "evt-log"],
                    "contradicting_evidence_event_ids": ["evt-cache-ok"],
                    "confidence": 0.8,
                    "status": "open"
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_hypotheses_report(&sqlite_path, "evt-failure", 4, 20).unwrap();

        assert!(report.contains("State graph hypotheses: evt-failure depth=4 limit=20"));
        assert!(report.contains("hypotheses: hyp-context-miss"));
        assert!(report.contains("failures:   evt-failure"));
        assert!(report.contains("statuses:   open=1"));
        assert!(report.contains("supports:   evt-failure, evt-log"));
        assert!(report.contains("contradicts: evt-cache-ok"));
        assert!(report.contains("records_hypothesis=1"));
        assert!(report.contains("caused_by=1"));
        assert!(report.contains("supports=2"));
        assert!(report.contains("contradicts=1"));
        assert!(report.contains(
            "hypothesis=hyp-context-miss failure=evt-failure confidence=0.800 status=open"
        ));
        assert!(report.contains("summary=context selector omitted retry_state.rs"));
        assert!(report.contains("evt-failure -[caused_by]-> hyp-context-miss"));
        assert!(report.contains("evt-log -[supports]-> hyp-context-miss"));
        assert!(report.contains("evt-cache-ok -[contradicts]-> hyp-context-miss"));

        let payload = build_graph_hypotheses_payload(&sqlite_path, "evt-failure", 4, 20).unwrap();
        assert_eq!(
            payload["diagnostic"].as_str().unwrap(),
            "state_graph_hypotheses"
        );
        assert_eq!(payload["id"].as_str().unwrap(), "evt-failure");
        assert_eq!(payload["depth"].as_u64().unwrap(), 4);
        assert_eq!(payload["limit"].as_u64().unwrap(), 20);
        assert_eq!(payload["hypothesis_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["failure_count"].as_u64().unwrap(), 2);
        assert_eq!(
            payload["hypotheses"][0].as_str().unwrap(),
            "hyp-context-miss"
        );
        let failures = payload["failures"].as_array().unwrap();
        assert!(failures
            .iter()
            .any(|failure| failure.as_str() == Some("evt-failure")));
        assert_eq!(payload["statuses"]["open"].as_u64().unwrap(), 1);
        assert_eq!(
            payload["relations"]["records_hypothesis"].as_u64().unwrap(),
            1
        );
        assert_eq!(payload["relations"]["supports"].as_u64().unwrap(), 2);
        assert_eq!(payload["relations"]["contradicts"].as_u64().unwrap(), 1);
        assert!(payload["supporting_events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event.as_str() == Some("evt-log")));
        assert_eq!(
            payload["contradicting_events"][0].as_str().unwrap(),
            "evt-cache-ok"
        );
        let metadata = &payload["hypothesis_metadata"]["hyp-context-miss"];
        assert_eq!(
            metadata["failure_event_id"].as_str().unwrap(),
            "evt-failure"
        );
        assert_eq!(
            metadata["summary"].as_str().unwrap(),
            "context selector omitted retry_state.rs"
        );
        assert_eq!(metadata["confidence"].as_f64().unwrap(), 0.8);
        assert_eq!(metadata["status"].as_str().unwrap(), "open");
        let hypothesis_relations = payload["hypothesis_relations"].as_array().unwrap();
        let supports = hypothesis_relations
            .iter()
            .find(|row| {
                row["relation"].as_str() == Some("supports")
                    && row["src_id"].as_str() == Some("evt-log")
            })
            .unwrap();
        assert_eq!(
            supports["hypothesis_id"].as_str().unwrap(),
            "hyp-context-miss"
        );
        let contradicts = hypothesis_relations
            .iter()
            .find(|row| row["relation"].as_str() == Some("contradicts"))
            .unwrap();
        assert_eq!(contradicts["src_id"].as_str().unwrap(), "evt-cache-ok");
        assert_eq!(
            contradicts["hypothesis_id"].as_str().unwrap(),
            "hyp-context-miss"
        );
    }

    #[test]
    fn graph_versions_report_surfaces_reachable_harness_version_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-failure".into(),
                event_type: crate::state::EventType::FailureObserved,
                schema_version: 1,
                timestamp_ms: 1,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "eval",
                    "harness_version": "genome-v2",
                    "error_preview": "latest harness failed local smoke"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-patch".into(),
                event_type: crate::state::EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 2,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-1",
                    "kind": "context_policy",
                    "risk_level": "medium",
                    "base_harness_version": "genome-v1",
                    "evidence_event_ids": ["evt-failure"],
                    "status": "proposed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-eval".into(),
                event_type: crate::state::EventType::PatchEvaluated,
                schema_version: 1,
                timestamp_ms: 3,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "eval_id": "eval-1",
                    "patch_id": "patch-1",
                    "harness_version": "genome-v2",
                    "suite": "local-smoke",
                    "status": "passed",
                    "score": 1.0,
                    "passed": 1,
                    "failed": 0,
                    "failure_event_ids": ["evt-failure"],
                    "metrics": {},
                    "created_at_ms": 3
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_versions_report(&sqlite_path, "patch-1", 4, 20).unwrap();

        assert!(report.contains("State graph versions: patch-1 depth=4 limit=20"));
        assert!(report.contains("versions:   genome-v1, genome-v2"));
        assert!(report.contains("bases:      genome-v1"));
        assert!(report.contains("patches:    patch-1"));
        assert!(report.contains("evals:      eval-1"));
        assert!(report.contains("statuses:   passed=1, proposed=1"));
        assert!(report.contains("based_on_harness_version=1"));
        assert!(report.contains("uses_harness_version=2"));
        assert!(report.contains("harness=- base=genome-v1 patch=patch-1"));
        assert!(report.contains("harness=genome-v2 base=- patch=patch-1 eval=eval-1"));
        assert!(report.contains("suite=local-smoke status=passed run=run-1"));
        assert!(report.contains("evt-patch -[based_on_harness_version]-> genome-v1"));
        assert!(report.contains("evt-eval -[uses_harness_version]-> genome-v2"));

        let payload = build_graph_versions_payload(&sqlite_path, "patch-1", 4, 20).unwrap();
        assert_eq!(
            payload["diagnostic"].as_str().unwrap(),
            "state_graph_versions"
        );
        assert_eq!(payload["id"].as_str().unwrap(), "patch-1");
        assert_eq!(payload["depth"].as_u64().unwrap(), 4);
        assert_eq!(payload["limit"].as_u64().unwrap(), 20);
        assert_eq!(payload["version_count"].as_u64().unwrap(), 2);
        assert_eq!(payload["base_version_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["patch_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["eval_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["versions"][0].as_str().unwrap(), "genome-v1");
        assert_eq!(payload["versions"][1].as_str().unwrap(), "genome-v2");
        assert_eq!(payload["base_versions"][0].as_str().unwrap(), "genome-v1");
        assert_eq!(payload["patches"][0].as_str().unwrap(), "patch-1");
        assert_eq!(payload["evals"][0].as_str().unwrap(), "eval-1");
        assert_eq!(payload["statuses"]["passed"].as_u64().unwrap(), 1);
        assert_eq!(payload["statuses"]["proposed"].as_u64().unwrap(), 1);
        assert_eq!(
            payload["relations"]["based_on_harness_version"]
                .as_u64()
                .unwrap(),
            1
        );
        assert_eq!(
            payload["relations"]["uses_harness_version"]
                .as_u64()
                .unwrap(),
            2
        );
        let patch_metadata = &payload["version_metadata"]["evt-patch"];
        assert_eq!(
            patch_metadata["base_harness_version"].as_str().unwrap(),
            "genome-v1"
        );
        assert_eq!(patch_metadata["patch_id"].as_str().unwrap(), "patch-1");
        assert_eq!(patch_metadata["status"].as_str().unwrap(), "proposed");
        let eval_metadata = &payload["version_metadata"]["evt-eval"];
        assert_eq!(
            eval_metadata["harness_version"].as_str().unwrap(),
            "genome-v2"
        );
        assert_eq!(eval_metadata["eval_id"].as_str().unwrap(), "eval-1");
        assert_eq!(eval_metadata["suite"].as_str().unwrap(), "local-smoke");
        let version_relations = payload["version_relations"].as_array().unwrap();
        let base_relation = version_relations
            .iter()
            .find(|row| row["relation"].as_str() == Some("based_on_harness_version"))
            .unwrap();
        assert_eq!(
            base_relation["version_event_id"].as_str().unwrap(),
            "evt-patch"
        );
        assert_eq!(base_relation["dst_id"].as_str().unwrap(), "genome-v1");
        let eval_relation = version_relations
            .iter()
            .find(|row| {
                row["relation"].as_str() == Some("uses_harness_version")
                    && row["event_id"].as_str() == Some("evt-eval")
            })
            .unwrap();
        assert_eq!(
            eval_relation["version_event_id"].as_str().unwrap(),
            "evt-eval"
        );
        assert_eq!(eval_relation["dst_id"].as_str().unwrap(), "genome-v2");
    }

    #[test]
    fn graph_runs_report_surfaces_reachable_run_trace_and_task_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-run-start".into(),
                event_type: crate::state::EventType::RunStarted,
                schema_version: 1,
                timestamp_ms: 1,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "status": "started",
                    "task_id": "context-smoke"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-command".into(),
                event_type: crate::state::EventType::CommandCompleted,
                schema_version: 1,
                timestamp_ms: 2,
                actor: crate::state::Actor::Tool,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "command": "cargo test context_smoke",
                    "task_ids": ["context-smoke", "state-redaction"],
                    "is_error": false,
                    "result_preview": "ok"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-eval".into(),
                event_type: crate::state::EventType::PatchEvaluated,
                schema_version: 1,
                timestamp_ms: 3,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "eval_id": "eval-1",
                    "patch_id": "patch-1",
                    "suite": "local-smoke",
                    "status": "passed",
                    "score": 1.0,
                    "passed": 2,
                    "failed": 0,
                    "metrics": {
                        "fixture_tasks": [
                            {"task_id": "context-smoke", "passed": true},
                            {"task_id": "state-redaction", "passed": true}
                        ]
                    },
                    "created_at_ms": 3
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_runs_report(&sqlite_path, "run-1", 4, 20).unwrap();

        assert!(report.contains("State graph runs: run-1 depth=4 limit=20"));
        assert!(report.contains("runs:       run-1"));
        assert!(report.contains("traces:     trace-1"));
        assert!(report.contains("tasks:      context-smoke, state-redaction"));
        assert!(report.contains("statuses:   ok=1, passed=1, started=1"));
        assert!(report.contains("RunStarted=1"));
        assert!(report.contains("CommandCompleted=1"));
        assert!(report.contains("PatchEvaluated=1"));
        assert!(report.contains("observed_in=3"));
        assert!(report.contains("traced_by=3"));
        assert!(report.contains("records_task=3"));
        assert!(report.contains("tested_by=2"));
        assert!(report.contains("run=run-1 trace=trace-1 tasks=[context-smoke] status=started"));
        assert!(report
            .contains("run=run-1 trace=trace-1 tasks=[context-smoke,state-redaction] status=ok"));
        assert!(report.contains("evt-command -[records_task]-> state-redaction"));
        assert!(report.contains("context-smoke -[tested_by]-> eval-1"));

        let payload = build_graph_runs_payload(&sqlite_path, "run-1", 4, 20).unwrap();
        assert_eq!(payload["diagnostic"].as_str().unwrap(), "state_graph_runs");
        assert_eq!(payload["id"].as_str().unwrap(), "run-1");
        assert_eq!(payload["depth"].as_u64().unwrap(), 4);
        assert_eq!(payload["limit"].as_u64().unwrap(), 20);
        assert_eq!(payload["run_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["trace_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["task_count"].as_u64().unwrap(), 2);
        assert_eq!(payload["runs"][0].as_str().unwrap(), "run-1");
        assert_eq!(payload["traces"][0].as_str().unwrap(), "trace-1");
        assert_eq!(payload["tasks"][0].as_str().unwrap(), "context-smoke");
        assert_eq!(payload["tasks"][1].as_str().unwrap(), "state-redaction");
        assert_eq!(payload["statuses"]["started"].as_u64().unwrap(), 1);
        assert_eq!(payload["statuses"]["ok"].as_u64().unwrap(), 1);
        assert_eq!(payload["statuses"]["passed"].as_u64().unwrap(), 1);
        assert_eq!(payload["event_types"]["RunStarted"].as_u64().unwrap(), 1);
        assert_eq!(
            payload["event_types"]["CommandCompleted"].as_u64().unwrap(),
            1
        );
        assert_eq!(
            payload["event_types"]["PatchEvaluated"].as_u64().unwrap(),
            1
        );
        assert_eq!(payload["relations"]["observed_in"].as_u64().unwrap(), 3);
        assert_eq!(payload["relations"]["traced_by"].as_u64().unwrap(), 3);
        assert_eq!(payload["relations"]["records_task"].as_u64().unwrap(), 3);
        assert_eq!(payload["relations"]["tested_by"].as_u64().unwrap(), 2);
        let command_metadata = &payload["run_metadata"]["evt-command"];
        assert_eq!(command_metadata["run_id"].as_str().unwrap(), "run-1");
        assert_eq!(command_metadata["trace_id"].as_str().unwrap(), "trace-1");
        assert_eq!(
            command_metadata["task_ids"][0].as_str().unwrap(),
            "context-smoke"
        );
        assert_eq!(
            command_metadata["task_ids"][1].as_str().unwrap(),
            "state-redaction"
        );
        assert_eq!(command_metadata["status"].as_str().unwrap(), "ok");
        let run_relations = payload["run_relations"].as_array().unwrap();
        let records_task = run_relations
            .iter()
            .find(|row| {
                row["relation"].as_str() == Some("records_task")
                    && row["dst_id"].as_str() == Some("state-redaction")
            })
            .unwrap();
        assert_eq!(
            records_task["run_event_id"].as_str().unwrap(),
            "evt-command"
        );
        let tested_by = run_relations
            .iter()
            .find(|row| {
                row["relation"].as_str() == Some("tested_by")
                    && row["src_id"].as_str() == Some("context-smoke")
            })
            .unwrap();
        assert_eq!(tested_by["dst_id"].as_str().unwrap(), "eval-1");
    }

    #[test]
    fn graph_artifacts_report_surfaces_reachable_eval_artifact_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-eval".into(),
            event_type: crate::state::EventType::PatchEvaluated,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-1".into()),
            session_id: None,
            trace_id: "trace-1".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "eval_id": "eval-1",
                "patch_id": "patch-1",
                "suite": "local-smoke",
                "status": "passed",
                "score": 1.0,
                "metrics": {
                    "artifact_uri": ".yoyo/state/artifacts/evals/eval-1.json",
                    "artifacts": [{
                        "kind": "eval_report",
                        "uri": ".yoyo/state/artifacts/evals/eval-1.json"
                    }],
                    "reproducibility": {
                        "mode": "fixtures",
                        "agent_command_source": "default-agent",
                        "git_dirty": false,
                        "replay_command": "yoyo eval fixtures run --suite local-smoke",
                        "commands": [
                            "cargo test graph_artifacts",
                            "cargo run --quiet --bin yyds -- eval fixtures validate --suite local-smoke"
                        ]
                    },
                    "fixture_suite": {
                        "task_count": 2,
                        "command_count": 3
                    }
                },
                "created_at_ms": 1
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_artifacts_report(&sqlite_path, "eval-1", 4, 20).unwrap();

        assert!(report.contains("State graph artifacts: eval-1 depth=4 limit=20"));
        assert!(report.contains("artifacts:  .yoyo/state/artifacts/evals/eval-1.json"));
        assert!(report.contains("evals:      eval-1"));
        assert!(report.contains("patches:    patch-1"));
        assert!(report.contains("statuses:   passed=1"));
        assert!(report.contains("repro modes: fixtures=1"));
        assert!(report.contains("agent sources: default-agent=1"));
        assert!(report.contains("dirtiness:  clean=1"));
        assert!(report.contains("has_artifact=1"));
        assert!(report.contains("references_artifact=1"));
        assert!(report.contains("artifact=.yoyo/state/artifacts/evals/eval-1.json"));
        assert!(report.contains("eval=eval-1 patch=patch-1 suite=local-smoke status=passed"));
        assert!(report.contains("repro=fixtures agent_source=default-agent dirty=no commands=2"));
        assert!(report.contains("fixture_tasks=2 fixture_commands=3"));
        assert!(report.contains("replay=yoyo eval fixtures run --suite local-smoke"));
        assert!(report.contains("eval-1 -[has_artifact]-> .yoyo/state/artifacts/evals/eval-1.json"));
        assert!(report
            .contains("evt-eval -[references_artifact]-> .yoyo/state/artifacts/evals/eval-1.json"));

        let payload = build_graph_artifacts_payload(&sqlite_path, "eval-1", 4, 20).unwrap();
        assert_eq!(
            payload["diagnostic"].as_str().unwrap(),
            "state_graph_artifacts"
        );
        assert_eq!(payload["id"].as_str().unwrap(), "eval-1");
        assert_eq!(payload["depth"].as_u64().unwrap(), 4);
        assert_eq!(payload["limit"].as_u64().unwrap(), 20);
        assert_eq!(payload["artifact_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["eval_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["patch_count"].as_u64().unwrap(), 1);
        assert_eq!(
            payload["artifacts"][0].as_str().unwrap(),
            ".yoyo/state/artifacts/evals/eval-1.json"
        );
        assert_eq!(payload["evals"][0].as_str().unwrap(), "eval-1");
        assert_eq!(payload["patches"][0].as_str().unwrap(), "patch-1");
        assert_eq!(payload["statuses"]["passed"].as_u64().unwrap(), 1);
        assert_eq!(payload["repro_modes"]["fixtures"].as_u64().unwrap(), 1);
        assert_eq!(
            payload["agent_sources"]["default-agent"].as_u64().unwrap(),
            1
        );
        assert_eq!(payload["dirtiness"]["clean"].as_u64().unwrap(), 1);
        assert_eq!(payload["relations"]["has_artifact"].as_u64().unwrap(), 1);
        assert_eq!(
            payload["relations"]["references_artifact"]
                .as_u64()
                .unwrap(),
            1
        );
        let metadata = &payload["artifact_metadata"][".yoyo/state/artifacts/evals/eval-1.json"];
        assert_eq!(metadata["eval_id"].as_str().unwrap(), "eval-1");
        assert_eq!(metadata["patch_id"].as_str().unwrap(), "patch-1");
        assert_eq!(metadata["suite"].as_str().unwrap(), "local-smoke");
        assert_eq!(metadata["status"].as_str().unwrap(), "passed");
        assert_eq!(metadata["repro_mode"].as_str().unwrap(), "fixtures");
        assert_eq!(
            metadata["agent_command_source"].as_str().unwrap(),
            "default-agent"
        );
        assert!(!metadata["git_dirty"].as_bool().unwrap());
        assert_eq!(metadata["command_count"].as_u64().unwrap(), 2);
        assert_eq!(metadata["fixture_task_count"].as_u64().unwrap(), 2);
        assert_eq!(metadata["fixture_command_count"].as_u64().unwrap(), 3);
        assert_eq!(
            metadata["replay_command"].as_str().unwrap(),
            "yoyo eval fixtures run --suite local-smoke"
        );
        let artifact_relations = payload["artifact_relations"].as_array().unwrap();
        let has_artifact = artifact_relations
            .iter()
            .find(|row| row["relation"].as_str() == Some("has_artifact"))
            .unwrap();
        assert_eq!(has_artifact["src_id"].as_str().unwrap(), "eval-1");
        assert_eq!(
            has_artifact["artifact_uri"].as_str().unwrap(),
            ".yoyo/state/artifacts/evals/eval-1.json"
        );
        let references_artifact = artifact_relations
            .iter()
            .find(|row| row["relation"].as_str() == Some("references_artifact"))
            .unwrap();
        assert_eq!(
            references_artifact["event_id"].as_str().unwrap(),
            "evt-eval"
        );
    }

    #[test]
    fn graph_models_report_surfaces_reachable_model_call_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-failure".into(),
                event_type: crate::state::EventType::FailureObserved,
                schema_version: 1,
                timestamp_ms: 1,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "repair",
                    "error_preview": "context miss"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-patch".into(),
                event_type: crate::state::EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 2,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-1",
                    "kind": "context_policy",
                    "risk_level": "low",
                    "evidence_event_ids": ["evt-failure"],
                    "status": "proposed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-model-start".into(),
                event_type: crate::state::EventType::ModelCallStarted,
                schema_version: 1,
                timestamp_ms: 3,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "model_call_id": "model-call-1",
                    "model": "deepseek-v4-pro",
                    "route_task": "root-cause",
                    "thinking": {"type": "enabled"},
                    "reasoning_effort": "high"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-model-done".into(),
                event_type: crate::state::EventType::ModelCallCompleted,
                schema_version: 1,
                timestamp_ms: 4,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "model_call_id": "model-call-1",
                    "model": "deepseek-v4-pro",
                    "route_task": "root-cause",
                    "thinking": {"type": "enabled"},
                    "reasoning_effort": "high",
                    "input_tokens": 100,
                    "output_tokens": 20,
                    "cache_read_tokens": 70,
                    "cache_write_tokens": 5
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_models_report(&sqlite_path, "patch-1", 4, 20).unwrap();

        assert!(report.contains("State graph models: patch-1 depth=4 limit=20"));
        assert!(report.contains("calls:      model-call-1"));
        assert!(report.contains("models:     deepseek-v4-pro"));
        assert!(report.contains("route tasks: root_cause"));
        assert!(report.contains("tokens:     in=100 out=20 cache_read=70 cache_write=5"));
        assert!(report.contains("records_model_call=2"));
        assert!(report.contains(
            "call=model-call-1 model=deepseek-v4-pro route=root_cause thinking=enabled effort=high"
        ));
        assert!(report.contains("tokens=in:100 out:20 cache_read:70 cache_write:5 run=run-1"));
        assert!(report.contains("evt-model-done -[records_model_call]-> model-call-1"));

        let payload = build_graph_models_payload(&sqlite_path, "patch-1", 4, 20).unwrap();
        assert_eq!(
            payload["diagnostic"].as_str().unwrap(),
            "state_graph_models"
        );
        assert_eq!(payload["id"].as_str().unwrap(), "patch-1");
        assert_eq!(payload["depth"].as_u64().unwrap(), 4);
        assert_eq!(payload["limit"].as_u64().unwrap(), 20);
        assert_eq!(payload["model_call_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["model_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["route_task_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["model_calls"][0].as_str().unwrap(), "model-call-1");
        assert_eq!(payload["models"][0].as_str().unwrap(), "deepseek-v4-pro");
        assert_eq!(payload["route_tasks"][0].as_str().unwrap(), "root_cause");
        assert_eq!(payload["tokens"]["input"].as_u64().unwrap(), 100);
        assert_eq!(payload["tokens"]["output"].as_u64().unwrap(), 20);
        assert_eq!(payload["tokens"]["cache_read"].as_u64().unwrap(), 70);
        assert_eq!(payload["tokens"]["cache_write"].as_u64().unwrap(), 5);
        assert_eq!(
            payload["relations"]["records_model_call"].as_u64().unwrap(),
            2
        );
        let metadata = &payload["model_metadata"]["model-call-1"];
        assert_eq!(metadata["model"].as_str().unwrap(), "deepseek-v4-pro");
        assert_eq!(metadata["route_task"].as_str().unwrap(), "root_cause");
        assert_eq!(metadata["thinking"].as_str().unwrap(), "enabled");
        assert_eq!(metadata["reasoning_effort"].as_str().unwrap(), "high");
        assert_eq!(metadata["input_tokens"].as_u64().unwrap(), 100);
        assert_eq!(metadata["output_tokens"].as_u64().unwrap(), 20);
        assert_eq!(metadata["cache_read_tokens"].as_u64().unwrap(), 70);
        assert_eq!(metadata["cache_write_tokens"].as_u64().unwrap(), 5);
        assert_eq!(metadata["run_id"].as_str().unwrap(), "run-1");
        let model_relations = payload["model_relations"].as_array().unwrap();
        let completed = model_relations
            .iter()
            .find(|row| {
                row["event_id"].as_str() == Some("evt-model-done")
                    && row["relation"].as_str() == Some("records_model_call")
            })
            .unwrap();
        assert_eq!(completed["model_call_id"].as_str().unwrap(), "model-call-1");
        assert_eq!(
            completed["relation"].as_str().unwrap(),
            "records_model_call"
        );
    }

    #[test]
    fn graph_tools_report_surfaces_reachable_tool_call_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-failure".into(),
                event_type: crate::state::EventType::FailureObserved,
                schema_version: 1,
                timestamp_ms: 1,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "repair",
                    "error_preview": "context miss"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-patch".into(),
                event_type: crate::state::EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 2,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-1",
                    "kind": "context_policy",
                    "risk_level": "low",
                    "evidence_event_ids": ["evt-failure"],
                    "status": "proposed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-tool-start".into(),
                event_type: crate::state::EventType::ToolCallStarted,
                schema_version: 1,
                timestamp_ms: 3,
                actor: crate::state::Actor::Tool,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "tool_call_id": "tool-call-1",
                    "tool_name": "read_file",
                    "args": {"path": "src/state.rs"}
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-tool-done".into(),
                event_type: crate::state::EventType::ToolCallCompleted,
                schema_version: 1,
                timestamp_ms: 4,
                actor: crate::state::Actor::Tool,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "tool_call_id": "tool-call-1",
                    "tool_name": "read_file",
                    "status": "ok",
                    "result_preview": "read 120 lines"
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_tools_report(&sqlite_path, "patch-1", 4, 20).unwrap();

        assert!(report.contains("State graph tools: patch-1 depth=4 limit=20"));
        assert!(report.contains("calls:      tool-call-1"));
        assert!(report.contains("tools:      read_file"));
        assert!(report.contains("statuses:   ok=1"));
        assert!(report.contains("records_tool_call=2"));
        assert!(report.contains("call=tool-call-1 tool=read_file status=ok"));
        assert!(report.contains("result=read 120 lines run=run-1"));
        assert!(report.contains("evt-tool-done -[records_tool_call]-> tool-call-1"));

        let payload = build_graph_tools_payload(&sqlite_path, "patch-1", 4, 20).unwrap();
        assert_eq!(payload["diagnostic"].as_str().unwrap(), "state_graph_tools");
        assert_eq!(payload["id"].as_str().unwrap(), "patch-1");
        assert_eq!(payload["depth"].as_u64().unwrap(), 4);
        assert_eq!(payload["limit"].as_u64().unwrap(), 20);
        assert_eq!(payload["tool_call_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["tool_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["tool_calls"][0].as_str().unwrap(), "tool-call-1");
        assert_eq!(payload["tools"][0].as_str().unwrap(), "read_file");
        assert_eq!(payload["statuses"]["ok"].as_u64().unwrap(), 1);
        assert_eq!(
            payload["relations"]["records_tool_call"].as_u64().unwrap(),
            2
        );
        let metadata = &payload["tool_metadata"]["tool-call-1"];
        assert_eq!(metadata["tool_name"].as_str().unwrap(), "read_file");
        assert_eq!(metadata["status"].as_str().unwrap(), "ok");
        assert_eq!(
            metadata["result_preview"].as_str().unwrap(),
            "read 120 lines"
        );
        assert_eq!(metadata["run_id"].as_str().unwrap(), "run-1");
        let tool_relations = payload["tool_relations"].as_array().unwrap();
        let completed = tool_relations
            .iter()
            .find(|row| {
                row["event_id"].as_str() == Some("evt-tool-done")
                    && row["relation"].as_str() == Some("records_tool_call")
            })
            .unwrap();
        assert_eq!(completed["tool_call_id"].as_str().unwrap(), "tool-call-1");
        assert_eq!(completed["relation"].as_str().unwrap(), "records_tool_call");
    }

    #[test]
    fn graph_commands_report_surfaces_reachable_command_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-failure".into(),
                event_type: crate::state::EventType::FailureObserved,
                schema_version: 1,
                timestamp_ms: 1,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "test",
                    "error_preview": "cargo test failed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-patch".into(),
                event_type: crate::state::EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 2,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-1",
                    "kind": "test",
                    "risk_level": "low",
                    "evidence_event_ids": ["evt-failure"],
                    "status": "proposed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-command-start".into(),
                event_type: crate::state::EventType::CommandStarted,
                schema_version: 1,
                timestamp_ms: 3,
                actor: crate::state::Actor::Tool,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "command": "cargo test commands_state::tests::graph_commands"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-command-done".into(),
                event_type: crate::state::EventType::CommandCompleted,
                schema_version: 1,
                timestamp_ms: 4,
                actor: crate::state::Actor::Tool,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "command": "cargo test commands_state::tests::graph_commands",
                    "is_error": false,
                    "result_preview": "test result: ok"
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_commands_report(&sqlite_path, "patch-1", 4, 20).unwrap();

        assert!(report.contains("State graph commands: patch-1 depth=4 limit=20"));
        assert!(report.contains("events:     evt-command-done, evt-command-start"));
        assert!(report.contains("statuses:   ok=1"));
        assert!(report.contains("observed_in=2"));
        assert!(report.contains("command=cargo test commands_state::tests::graph_commands"));
        assert!(report.contains("status=ok result=test result: ok run=run-1"));
        assert!(report.contains("CommandCompleted evt-command-done -[observed_in]-> run-1"));

        let payload = build_graph_commands_payload(&sqlite_path, "patch-1", 4, 20).unwrap();
        assert_eq!(
            payload["diagnostic"].as_str().unwrap(),
            "state_graph_commands"
        );
        assert_eq!(payload["id"].as_str().unwrap(), "patch-1");
        assert_eq!(payload["depth"].as_u64().unwrap(), 4);
        assert_eq!(payload["limit"].as_u64().unwrap(), 20);
        assert_eq!(payload["command_event_count"].as_u64().unwrap(), 2);
        assert_eq!(
            payload["command_events"][0].as_str().unwrap(),
            "evt-command-done"
        );
        assert_eq!(
            payload["command_events"][1].as_str().unwrap(),
            "evt-command-start"
        );
        assert_eq!(payload["statuses"]["ok"].as_u64().unwrap(), 1);
        assert_eq!(payload["relations"]["observed_in"].as_u64().unwrap(), 2);
        let metadata = &payload["command_metadata"]["evt-command-done"];
        assert_eq!(metadata["event_type"].as_str().unwrap(), "CommandCompleted");
        assert_eq!(metadata["status"].as_str().unwrap(), "ok");
        assert_eq!(
            metadata["command"].as_str().unwrap(),
            "cargo test commands_state::tests::graph_commands"
        );
        assert_eq!(
            metadata["result_preview"].as_str().unwrap(),
            "test result: ok"
        );
        assert_eq!(metadata["run_id"].as_str().unwrap(), "run-1");
        let command_relations = payload["command_relations"].as_array().unwrap();
        let completed = command_relations
            .iter()
            .find(|row| {
                row["event_id"].as_str() == Some("evt-command-done")
                    && row["relation"].as_str() == Some("observed_in")
            })
            .unwrap();
        assert_eq!(
            completed["command_event_id"].as_str().unwrap(),
            "evt-command-done"
        );
        assert_eq!(completed["dst_id"].as_str().unwrap(), "run-1");
    }

    #[test]
    fn graph_tests_report_surfaces_reachable_test_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-failure".into(),
                event_type: crate::state::EventType::FailureObserved,
                schema_version: 1,
                timestamp_ms: 1,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "test",
                    "error_preview": "local smoke failed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-patch".into(),
                event_type: crate::state::EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 2,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-1",
                    "kind": "test",
                    "risk_level": "low",
                    "evidence_event_ids": ["evt-failure"],
                    "status": "proposed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-test-start".into(),
                event_type: crate::state::EventType::TestStarted,
                schema_version: 1,
                timestamp_ms: 3,
                actor: crate::state::Actor::Tool,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "test_kind": "local-smoke",
                    "command": "cargo run --quiet --bin yyds -- eval fixtures validate --suite local-smoke"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-test-done".into(),
                event_type: crate::state::EventType::TestCompleted,
                schema_version: 1,
                timestamp_ms: 4,
                actor: crate::state::Actor::Tool,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "test_kind": "local-smoke",
                    "command": "cargo run --quiet --bin yyds -- eval fixtures validate --suite local-smoke",
                    "passed": true,
                    "result_preview": "Eval fixture suite is valid"
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_tests_report(&sqlite_path, "patch-1", 4, 20).unwrap();

        assert!(report.contains("State graph tests: patch-1 depth=4 limit=20"));
        assert!(report.contains("events:     evt-test-done, evt-test-start"));
        assert!(report.contains("kinds:      local-smoke"));
        assert!(report.contains("statuses:   passed=1"));
        assert!(report.contains("observed_in=2"));
        assert!(report.contains("test=local-smoke command=cargo run --quiet --bin yyds"));
        assert!(report.contains("status=passed result=Eval fixture suite is valid run=run-1"));
        assert!(report.contains("TestCompleted evt-test-done -[observed_in]-> run-1"));

        let payload = build_graph_tests_payload(&sqlite_path, "patch-1", 4, 20).unwrap();
        assert_eq!(payload["diagnostic"].as_str().unwrap(), "state_graph_tests");
        assert_eq!(payload["id"].as_str().unwrap(), "patch-1");
        assert_eq!(payload["depth"].as_u64().unwrap(), 4);
        assert_eq!(payload["limit"].as_u64().unwrap(), 20);
        assert_eq!(payload["test_event_count"].as_u64().unwrap(), 2);
        assert_eq!(payload["test_kind_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["test_events"][0].as_str().unwrap(), "evt-test-done");
        assert_eq!(
            payload["test_events"][1].as_str().unwrap(),
            "evt-test-start"
        );
        assert_eq!(payload["test_kinds"][0].as_str().unwrap(), "local-smoke");
        assert_eq!(payload["statuses"]["passed"].as_u64().unwrap(), 1);
        assert_eq!(payload["relations"]["observed_in"].as_u64().unwrap(), 2);
        let metadata = &payload["test_metadata"]["evt-test-done"];
        assert_eq!(metadata["event_type"].as_str().unwrap(), "TestCompleted");
        assert_eq!(metadata["test_kind"].as_str().unwrap(), "local-smoke");
        assert_eq!(metadata["status"].as_str().unwrap(), "passed");
        assert_eq!(
            metadata["command"].as_str().unwrap(),
            "cargo run --quiet --bin yyds -- eval fixtures validate --suite local-smoke"
        );
        assert_eq!(
            metadata["result_preview"].as_str().unwrap(),
            "Eval fixture suite is valid"
        );
        assert_eq!(metadata["run_id"].as_str().unwrap(), "run-1");
        let test_relations = payload["test_relations"].as_array().unwrap();
        let completed = test_relations
            .iter()
            .find(|row| {
                row["event_id"].as_str() == Some("evt-test-done")
                    && row["relation"].as_str() == Some("observed_in")
            })
            .unwrap();
        assert_eq!(
            completed["test_event_id"].as_str().unwrap(),
            "evt-test-done"
        );
        assert_eq!(completed["dst_id"].as_str().unwrap(), "run-1");
    }

    #[test]
    fn graph_commits_report_surfaces_reachable_commit_and_revert_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-failure".into(),
                event_type: crate::state::EventType::FailureObserved,
                schema_version: 1,
                timestamp_ms: 1,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "release",
                    "error_preview": "commit lineage hidden"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-patch".into(),
                event_type: crate::state::EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 2,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-1",
                    "kind": "state_graph",
                    "risk_level": "low",
                    "evidence_event_ids": ["evt-failure"],
                    "status": "proposed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-commit".into(),
                event_type: crate::state::EventType::CommitCreated,
                schema_version: 1,
                timestamp_ms: 3,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "commit": "abc123",
                    "branch": "deepseek-native-bootstrap",
                    "message": "Add state graph commit analytics",
                    "files": ["src/commands_state.rs"]
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-revert".into(),
                event_type: crate::state::EventType::RevertPerformed,
                schema_version: 1,
                timestamp_ms: 4,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "commit": "def456",
                    "reverted_commit": "abc123",
                    "branch": "deepseek-native-bootstrap",
                    "reason": "rollback drill",
                    "files": ["src/commands_state.rs"]
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_commits_report(&sqlite_path, "patch-1", 4, 20).unwrap();

        assert!(report.contains("State graph commits: patch-1 depth=4 limit=20"));
        assert!(report.contains("events:     evt-commit, evt-revert"));
        assert!(report.contains("commits:    abc123, def456"));
        assert!(report.contains("reverted:   abc123"));
        assert!(report.contains("branches:   deepseek-native-bootstrap"));
        assert!(report.contains("files:      src/commands_state.rs"));
        assert!(report.contains("records_commit=2"));
        assert!(report.contains("reverted_commit=1"));
        assert!(report.contains("on_branch=2"));
        assert!(report.contains("commit=abc123 reverted=- branch=deepseek-native-bootstrap"));
        assert!(report.contains("message=Add state graph commit analytics"));
        assert!(report.contains("commit=def456 reverted=abc123"));
        assert!(report.contains("reason=rollback drill run=run-1"));
        assert!(report.contains("CommitCreated evt-commit -[records_commit]-> abc123"));
        assert!(report.contains("RevertPerformed evt-revert -[reverted_commit]-> abc123"));

        let payload = build_graph_commits_payload(&sqlite_path, "patch-1", 4, 20).unwrap();
        assert_eq!(
            payload["diagnostic"].as_str().unwrap(),
            "state_graph_commits"
        );
        assert_eq!(payload["id"].as_str().unwrap(), "patch-1");
        assert_eq!(payload["depth"].as_u64().unwrap(), 4);
        assert_eq!(payload["limit"].as_u64().unwrap(), 20);
        assert_eq!(payload["commit_event_count"].as_u64().unwrap(), 2);
        assert_eq!(payload["commit_count"].as_u64().unwrap(), 2);
        assert_eq!(payload["reverted_commit_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["branch_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["file_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["commit_events"][0].as_str().unwrap(), "evt-commit");
        assert_eq!(payload["commit_events"][1].as_str().unwrap(), "evt-revert");
        assert_eq!(payload["commits"][0].as_str().unwrap(), "abc123");
        assert_eq!(payload["commits"][1].as_str().unwrap(), "def456");
        assert_eq!(payload["reverted_commits"][0].as_str().unwrap(), "abc123");
        assert_eq!(
            payload["branches"][0].as_str().unwrap(),
            "deepseek-native-bootstrap"
        );
        assert_eq!(
            payload["files"][0].as_str().unwrap(),
            "src/commands_state.rs"
        );
        assert_eq!(payload["relations"]["records_commit"].as_u64().unwrap(), 2);
        assert_eq!(payload["relations"]["reverted_commit"].as_u64().unwrap(), 1);
        assert_eq!(payload["relations"]["on_branch"].as_u64().unwrap(), 2);
        let commit_metadata = &payload["commit_metadata"]["evt-commit"];
        assert_eq!(
            commit_metadata["event_type"].as_str().unwrap(),
            "CommitCreated"
        );
        assert_eq!(commit_metadata["commit"].as_str().unwrap(), "abc123");
        assert_eq!(
            commit_metadata["branch"].as_str().unwrap(),
            "deepseek-native-bootstrap"
        );
        assert_eq!(
            commit_metadata["message"].as_str().unwrap(),
            "Add state graph commit analytics"
        );
        assert_eq!(
            commit_metadata["files"][0].as_str().unwrap(),
            "src/commands_state.rs"
        );
        let revert_metadata = &payload["commit_metadata"]["evt-revert"];
        assert_eq!(
            revert_metadata["event_type"].as_str().unwrap(),
            "RevertPerformed"
        );
        assert_eq!(revert_metadata["commit"].as_str().unwrap(), "def456");
        assert_eq!(
            revert_metadata["reverted_commit"].as_str().unwrap(),
            "abc123"
        );
        assert_eq!(
            revert_metadata["reason"].as_str().unwrap(),
            "rollback drill"
        );
        assert_eq!(revert_metadata["run_id"].as_str().unwrap(), "run-1");
        let commit_relations = payload["commit_relations"].as_array().unwrap();
        let records_commit = commit_relations
            .iter()
            .find(|row| {
                row["event_id"].as_str() == Some("evt-commit")
                    && row["relation"].as_str() == Some("records_commit")
            })
            .unwrap();
        assert_eq!(
            records_commit["commit_event_id"].as_str().unwrap(),
            "evt-commit"
        );
        assert_eq!(records_commit["dst_id"].as_str().unwrap(), "abc123");
        let reverted_commit = commit_relations
            .iter()
            .find(|row| {
                row["event_id"].as_str() == Some("evt-revert")
                    && row["relation"].as_str() == Some("reverted_commit")
            })
            .unwrap();
        assert_eq!(
            reverted_commit["commit_event_id"].as_str().unwrap(),
            "evt-revert"
        );
        assert_eq!(reverted_commit["dst_id"].as_str().unwrap(), "abc123");
    }

    #[test]
    fn graph_memories_report_surfaces_reachable_memory_lifecycle_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-failure".into(),
                event_type: crate::state::EventType::FailureObserved,
                schema_version: 1,
                timestamp_ms: 1,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "context",
                    "error_preview": "context selector missed retry_state.rs"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-memory-proposed".into(),
                event_type: crate::state::EventType::MemoryProposed,
                schema_version: 1,
                timestamp_ms: 2,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "candidate_id": "memory-recurring_failure-context",
                    "status": "proposed",
                    "source": "recurring_failure_source",
                    "summary": "Context failures should bias future context selection.",
                    "evidence_event_ids": ["evt-failure"],
                    "proposed_by": "state_memory_synthesis"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-memory-promoted".into(),
                event_type: crate::state::EventType::MemoryPromoted,
                schema_version: 1,
                timestamp_ms: 3,
                actor: crate::state::Actor::User,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "candidate_id": "memory-recurring_failure-context",
                    "status": "promoted",
                    "source": "recurring_failure_source",
                    "summary": "Context failures should bias future context selection.",
                    "reason": "durable context policy lesson",
                    "proposed_event_id": "evt-memory-proposed",
                    "evidence_event_ids": ["evt-failure"]
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_memories_report(&sqlite_path, "evt-failure", 4, 20).unwrap();

        assert!(report.contains("State graph memories: evt-failure depth=4 limit=20"));
        assert!(report.contains("candidates: memory-recurring_failure-context"));
        assert!(report.contains("events:     evt-memory-promoted, evt-memory-proposed"));
        assert!(report.contains("statuses:   promoted=1, proposed=1"));
        assert!(report.contains("sources:    recurring_failure_source=2"));
        assert!(report.contains("evidence:   "));
        assert!(report.contains("evt-failure"));
        assert!(report.contains("records_memory=2"));
        assert!(report.contains("proposes_memory=1"));
        assert!(report.contains("promoted_memory=1"));
        assert!(report.contains("supported_by=2"));
        assert!(report.contains("derived_from=1"));
        assert!(report.contains("candidate=memory-recurring_failure-context status=proposed"));
        assert!(report.contains("summary=Context failures should bias future context selection."));
        assert!(report.contains("candidate=memory-recurring_failure-context status=promoted"));
        assert!(report.contains("reason=durable context policy lesson"));
        assert!(report.contains(
            "MemoryProposed evt-memory-proposed -[proposes_memory]-> memory-recurring_failure-context"
        ));
        assert!(report.contains(
            "MemoryPromoted evt-memory-promoted -[promoted_memory]-> memory-recurring_failure-context"
        ));

        let payload = build_graph_memories_payload(&sqlite_path, "evt-failure", 4, 20).unwrap();
        assert_eq!(
            payload["diagnostic"].as_str().unwrap(),
            "state_graph_memories"
        );
        assert_eq!(payload["id"].as_str().unwrap(), "evt-failure");
        assert_eq!(payload["depth"].as_u64().unwrap(), 4);
        assert_eq!(payload["limit"].as_u64().unwrap(), 20);
        assert_eq!(payload["memory_candidate_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["memory_event_count"].as_u64().unwrap(), 2);
        assert_eq!(payload["evidence_event_count"].as_u64().unwrap(), 3);
        assert_eq!(
            payload["memory_candidates"][0].as_str().unwrap(),
            "memory-recurring_failure-context"
        );
        assert_eq!(
            payload["memory_events"][0].as_str().unwrap(),
            "evt-memory-promoted"
        );
        assert_eq!(
            payload["memory_events"][1].as_str().unwrap(),
            "evt-memory-proposed"
        );
        assert_eq!(
            payload["evidence_events"][0].as_str().unwrap(),
            "evt-failure"
        );
        assert_eq!(
            payload["evidence_events"][1].as_str().unwrap(),
            "evt-memory-promoted"
        );
        assert_eq!(
            payload["evidence_events"][2].as_str().unwrap(),
            "evt-memory-proposed"
        );
        assert_eq!(payload["statuses"]["promoted"].as_u64().unwrap(), 1);
        assert_eq!(payload["statuses"]["proposed"].as_u64().unwrap(), 1);
        assert_eq!(
            payload["sources"]["recurring_failure_source"]
                .as_u64()
                .unwrap(),
            2
        );
        assert_eq!(payload["relations"]["records_memory"].as_u64().unwrap(), 2);
        assert_eq!(payload["relations"]["proposes_memory"].as_u64().unwrap(), 1);
        assert_eq!(payload["relations"]["promoted_memory"].as_u64().unwrap(), 1);
        assert_eq!(payload["relations"]["supported_by"].as_u64().unwrap(), 2);
        assert_eq!(payload["relations"]["derived_from"].as_u64().unwrap(), 1);
        let proposed = &payload["memory_metadata"]["evt-memory-proposed"];
        assert_eq!(proposed["event_type"].as_str().unwrap(), "MemoryProposed");
        assert_eq!(
            proposed["candidate_id"].as_str().unwrap(),
            "memory-recurring_failure-context"
        );
        assert_eq!(proposed["status"].as_str().unwrap(), "proposed");
        assert_eq!(
            proposed["source"].as_str().unwrap(),
            "recurring_failure_source"
        );
        assert_eq!(
            proposed["evidence_event_ids"][0].as_str().unwrap(),
            "evt-failure"
        );
        let promoted = &payload["memory_metadata"]["evt-memory-promoted"];
        assert_eq!(promoted["event_type"].as_str().unwrap(), "MemoryPromoted");
        assert_eq!(promoted["status"].as_str().unwrap(), "promoted");
        assert_eq!(
            promoted["proposed_event_id"].as_str().unwrap(),
            "evt-memory-proposed"
        );
        assert_eq!(
            promoted["reason"].as_str().unwrap(),
            "durable context policy lesson"
        );
        assert_eq!(promoted["run_id"].as_str().unwrap(), "run-1");
        let memory_relations = payload["memory_relations"].as_array().unwrap();
        let proposed_relation = memory_relations
            .iter()
            .find(|row| {
                row["event_id"].as_str() == Some("evt-memory-proposed")
                    && row["relation"].as_str() == Some("proposes_memory")
            })
            .unwrap();
        assert_eq!(
            proposed_relation["memory_event_id"].as_str().unwrap(),
            "evt-memory-proposed"
        );
        assert_eq!(
            proposed_relation["dst_id"].as_str().unwrap(),
            "memory-recurring_failure-context"
        );
        let promoted_relation = memory_relations
            .iter()
            .find(|row| {
                row["event_id"].as_str() == Some("evt-memory-promoted")
                    && row["relation"].as_str() == Some("promoted_memory")
            })
            .unwrap();
        assert_eq!(
            promoted_relation["memory_event_id"].as_str().unwrap(),
            "evt-memory-promoted"
        );
        assert_eq!(
            promoted_relation["dst_id"].as_str().unwrap(),
            "memory-recurring_failure-context"
        );
    }

    #[test]
    fn graph_issues_report_surfaces_reachable_issue_intake_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-failure".into(),
                event_type: crate::state::EventType::FailureObserved,
                schema_version: 1,
                timestamp_ms: 1,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "eval",
                    "error_preview": "repair churn was not tracked"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-issue-intake".into(),
                event_type: crate::state::EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 2,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-issue-1",
                    "kind": "eval",
                    "risk_level": "low",
                    "status": "proposed",
                    "intent": "track DeepSeek repair churn",
                    "evidence_event_ids": ["evt-failure"],
                    "intake_source": "self_filed_improvement_issue",
                    "intake_kind": "issue",
                    "intake_summary": "Track DeepSeek repair churn",
                    "intake_details": "Self-filed issue should enter the harness patch lifecycle"
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_issues_report(&sqlite_path, "evt-failure", 4, 20).unwrap();

        assert!(report.contains("State graph issues: evt-failure depth=4 limit=20"));
        assert!(report.contains("issues:     issue:patch-issue-1"));
        assert!(report.contains("patches:    patch-issue-1"));
        assert!(report.contains("events:     evt-issue-intake"));
        assert!(report.contains("sources:    self_filed_improvement_issue=1"));
        assert!(report.contains("kinds:      eval=1"));
        assert!(report.contains("records_issue=1"));
        assert!(report.contains("addresses_patch=1"));
        assert!(report.contains("uses_patch=1"));
        assert!(report.contains("supported_by=1"));
        assert!(report.contains("issue=issue:patch-issue-1 patch=patch-issue-1"));
        assert!(report.contains("source=self_filed_improvement_issue kind=issue"));
        assert!(report.contains("patch_kind=eval risk=low status=proposed"));
        assert!(report.contains("summary=Track DeepSeek repair churn"));
        assert!(
            report.contains("details=Self-filed issue should enter the harness patch lifecycle")
        );
        assert!(report
            .contains("PatchProposed evt-issue-intake -[records_issue]-> issue:patch-issue-1"));
        assert!(
            report.contains("PatchProposed issue:patch-issue-1 -[addresses_patch]-> patch-issue-1")
        );

        let payload = build_graph_issues_payload(&sqlite_path, "evt-failure", 4, 20).unwrap();
        assert_eq!(
            payload["diagnostic"].as_str().unwrap(),
            "state_graph_issues"
        );
        assert_eq!(payload["id"].as_str().unwrap(), "evt-failure");
        assert_eq!(payload["depth"].as_u64().unwrap(), 4);
        assert_eq!(payload["limit"].as_u64().unwrap(), 20);
        assert_eq!(payload["issue_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["patch_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["issue_event_count"].as_u64().unwrap(), 1);
        assert_eq!(
            payload["issues"][0].as_str().unwrap(),
            "issue:patch-issue-1"
        );
        assert_eq!(payload["patches"][0].as_str().unwrap(), "patch-issue-1");
        assert_eq!(
            payload["issue_events"][0].as_str().unwrap(),
            "evt-issue-intake"
        );
        assert_eq!(
            payload["sources"]["self_filed_improvement_issue"]
                .as_u64()
                .unwrap(),
            1
        );
        assert_eq!(payload["kinds"]["eval"].as_u64().unwrap(), 1);
        assert_eq!(payload["relations"]["records_issue"].as_u64().unwrap(), 1);
        assert_eq!(payload["relations"]["addresses_patch"].as_u64().unwrap(), 1);
        assert_eq!(payload["relations"]["uses_patch"].as_u64().unwrap(), 1);
        assert_eq!(payload["relations"]["supported_by"].as_u64().unwrap(), 1);
        let metadata = &payload["issue_metadata"]["evt-issue-intake"];
        assert_eq!(
            metadata["issue_id"].as_str().unwrap(),
            "issue:patch-issue-1"
        );
        assert_eq!(metadata["patch_id"].as_str().unwrap(), "patch-issue-1");
        assert_eq!(
            metadata["intake_source"].as_str().unwrap(),
            "self_filed_improvement_issue"
        );
        assert_eq!(metadata["intake_kind"].as_str().unwrap(), "issue");
        assert_eq!(
            metadata["summary"].as_str().unwrap(),
            "Track DeepSeek repair churn"
        );
        assert_eq!(
            metadata["details"].as_str().unwrap(),
            "Self-filed issue should enter the harness patch lifecycle"
        );
        assert_eq!(metadata["kind"].as_str().unwrap(), "eval");
        assert_eq!(metadata["risk_level"].as_str().unwrap(), "low");
        assert_eq!(metadata["status"].as_str().unwrap(), "proposed");
        assert_eq!(metadata["run_id"].as_str().unwrap(), "run-1");
        let issue_relations = payload["issue_relations"].as_array().unwrap();
        let records_issue = issue_relations
            .iter()
            .find(|row| {
                row["event_id"].as_str() == Some("evt-issue-intake")
                    && row["relation"].as_str() == Some("records_issue")
            })
            .unwrap();
        assert_eq!(
            records_issue["issue_event_id"].as_str().unwrap(),
            "evt-issue-intake"
        );
        assert_eq!(
            records_issue["dst_id"].as_str().unwrap(),
            "issue:patch-issue-1"
        );
        let addresses_patch = issue_relations
            .iter()
            .find(|row| {
                row["src_id"].as_str() == Some("issue:patch-issue-1")
                    && row["relation"].as_str() == Some("addresses_patch")
            })
            .unwrap();
        assert_eq!(
            addresses_patch["issue_event_id"].as_str().unwrap(),
            "evt-issue-intake"
        );
        assert_eq!(addresses_patch["dst_id"].as_str().unwrap(), "patch-issue-1");
    }

    #[test]
    fn graph_cache_report_surfaces_reachable_cache_metrics() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-failure".into(),
                event_type: crate::state::EventType::FailureObserved,
                schema_version: 1,
                timestamp_ms: 1,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "repair",
                    "error_preview": "context miss"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-patch".into(),
                event_type: crate::state::EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 2,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-1",
                    "kind": "context_policy",
                    "risk_level": "low",
                    "evidence_event_ids": ["evt-failure"],
                    "status": "proposed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-cache".into(),
                event_type: crate::state::EventType::CacheMetricsRecorded,
                schema_version: 1,
                timestamp_ms: 3,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "model": "deepseek-v4-pro",
                    "prompt_cache_hit_tokens": 90,
                    "prompt_cache_miss_tokens": 10,
                    "cache_hit_ratio": 0.9
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_cache_report(&sqlite_path, "patch-1", 4, 20).unwrap();

        assert!(report.contains("State graph cache: patch-1 depth=4 limit=20"));
        assert!(report.contains("events:     evt-cache"));
        assert!(report.contains("models:     deepseek-v4-pro"));
        assert!(report.contains("totals:     hit=90 miss=10 ratio=0.900"));
        assert!(report.contains("observed_in=1"));
        assert!(report.contains("cache=evt-cache model=deepseek-v4-pro hit=90 miss=10 ratio=0.900"));
        assert!(report.contains("evt-cache -[observed_in]-> run-1"));

        let payload = build_graph_cache_payload(&sqlite_path, "patch-1", 4, 20).unwrap();

        assert_eq!(payload["diagnostic"].as_str().unwrap(), "state_graph_cache");
        assert_eq!(payload["id"].as_str().unwrap(), "patch-1");
        assert_eq!(payload["depth"].as_u64().unwrap(), 4);
        assert_eq!(payload["limit"].as_u64().unwrap(), 20);
        assert_eq!(payload["cache_event_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["model_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["cache_events"][0].as_str().unwrap(), "evt-cache");
        assert_eq!(payload["models"][0].as_str().unwrap(), "deepseek-v4-pro");
        assert_eq!(payload["totals"]["hit"].as_i64().unwrap(), 90);
        assert_eq!(payload["totals"]["miss"].as_i64().unwrap(), 10);
        assert!((payload["totals"]["ratio"].as_f64().unwrap() - 0.9).abs() < f64::EPSILON);
        assert_eq!(payload["relations"]["observed_in"].as_u64().unwrap(), 1);
        let metadata = &payload["cache_metadata"]["evt-cache"];
        assert_eq!(metadata["model"].as_str().unwrap(), "deepseek-v4-pro");
        assert_eq!(metadata["prompt_cache_hit_tokens"].as_i64().unwrap(), 90);
        assert_eq!(metadata["prompt_cache_miss_tokens"].as_i64().unwrap(), 10);
        assert!((metadata["cache_hit_ratio"].as_f64().unwrap() - 0.9).abs() < f64::EPSILON);
        assert_eq!(metadata["timestamp_ms"].as_i64().unwrap(), 3);
        let cache_relations = payload["cache_relations"].as_array().unwrap();
        let observed = cache_relations
            .iter()
            .find(|row| {
                row["event_id"].as_str() == Some("evt-cache")
                    && row["relation"].as_str() == Some("observed_in")
            })
            .unwrap();
        assert_eq!(observed["cache_event_id"].as_str().unwrap(), "evt-cache");
        assert_eq!(observed["dst_id"].as_str().unwrap(), "run-1");
    }

    #[test]
    fn graph_failures_report_surfaces_reachable_failure_taxonomy() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-failure".into(),
                event_type: crate::state::EventType::FailureObserved,
                schema_version: 1,
                timestamp_ms: 1,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "repair",
                    "error_preview": "context missing retry_state.rs from prompt"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-patch".into(),
                event_type: crate::state::EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 2,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-1",
                    "kind": "context_policy",
                    "risk_level": "low",
                    "evidence_event_ids": ["evt-failure"],
                    "status": "proposed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-eval".into(),
                event_type: crate::state::EventType::PatchEvaluated,
                schema_version: 1,
                timestamp_ms: 3,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "eval_id": "eval-1",
                    "patch_id": "patch-1",
                    "suite": "local-smoke",
                    "status": "passed",
                    "score": 1.0,
                    "passed": 1,
                    "failed": 0,
                    "metrics": {},
                    "failure_event_ids": ["evt-failure"],
                    "created_at_ms": 3
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_failures_report(&sqlite_path, "patch-1", 4, 20).unwrap();

        assert!(report.contains("State graph failures: patch-1 depth=4 limit=20"));
        assert!(report.contains("failures:   evt-failure"));
        assert!(report.contains("retryable:  1"));
        assert!(report.contains("classes:    context_miss=1"));
        assert!(report.contains("addresses=1"));
        assert!(report.contains("evaluated_failure=1"));
        assert!(report.contains(
            "failure=evt-failure class=context_miss owner=harness retryable=true source=repair"
        ));
        assert!(report.contains("signal=context missing retry_state.rs from prompt"));
        assert!(report.contains("run=run-1"));
        assert!(report.contains("patch-1 -[addresses]-> evt-failure"));
        assert!(report.contains("evt-eval -[evaluated_failure]-> evt-failure"));

        let payload = build_graph_failures_payload(&sqlite_path, "patch-1", 4, 20).unwrap();

        assert_eq!(
            payload["diagnostic"].as_str().unwrap(),
            "state_graph_failures"
        );
        assert_eq!(payload["id"].as_str().unwrap(), "patch-1");
        assert_eq!(payload["depth"].as_u64().unwrap(), 4);
        assert_eq!(payload["limit"].as_u64().unwrap(), 20);
        assert_eq!(payload["failure_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["retryable_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["failures"][0].as_str().unwrap(), "evt-failure");
        assert_eq!(payload["classes"]["context_miss"].as_u64().unwrap(), 1);
        assert_eq!(payload["relations"]["addresses"].as_u64().unwrap(), 1);
        assert_eq!(
            payload["relations"]["evaluated_failure"].as_u64().unwrap(),
            1
        );
        let metadata = &payload["failure_metadata"]["evt-failure"];
        assert_eq!(metadata["event_type"].as_str().unwrap(), "FailureObserved");
        assert_eq!(metadata["class"].as_str().unwrap(), "context_miss");
        assert_eq!(metadata["owner"].as_str().unwrap(), "harness");
        assert!(metadata["retryable"].as_bool().unwrap());
        assert_eq!(metadata["source"].as_str().unwrap(), "repair");
        assert_eq!(
            metadata["error_preview"].as_str().unwrap(),
            "context missing retry_state.rs from prompt"
        );
        assert_eq!(metadata["run_id"].as_str().unwrap(), "run-1");
        assert_eq!(metadata["timestamp_ms"].as_i64().unwrap(), 1);
        let failure_relations = payload["failure_relations"].as_array().unwrap();
        let addresses = failure_relations
            .iter()
            .find(|row| {
                row["src_id"].as_str() == Some("patch-1")
                    && row["relation"].as_str() == Some("addresses")
            })
            .unwrap();
        assert_eq!(
            addresses["failure_event_id"].as_str().unwrap(),
            "evt-failure"
        );
        assert_eq!(addresses["dst_id"].as_str().unwrap(), "evt-failure");
        let evaluated = failure_relations
            .iter()
            .find(|row| {
                row["event_id"].as_str() == Some("evt-eval")
                    && row["relation"].as_str() == Some("evaluated_failure")
            })
            .unwrap();
        assert_eq!(
            evaluated["failure_event_id"].as_str().unwrap(),
            "evt-failure"
        );
    }

    #[test]
    fn graph_policies_report_surfaces_reachable_context_and_schema_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-failure".into(),
                event_type: crate::state::EventType::FailureObserved,
                schema_version: 1,
                timestamp_ms: 1,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "context",
                    "error_preview": "context omitted retry_state.rs"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-context".into(),
                event_type: crate::state::EventType::ContextBuilt,
                schema_version: 1,
                timestamp_ms: 2,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "context_policy": "deepseek_native@v2",
                    "layout_version": 7,
                    "prompt_version": "deepseek_native_prompt@v7",
                    "include_instruction_files": ["YOYO.md", "AGENTS.md"],
                    "stable_prefix_blocks": ["deepseek_native_system_contract", "strict_tool_schemas"],
                    "dynamic_suffix_blocks": ["selected_recent_events"],
                    "included_blocks": [
                        {"name": "deepseek_native_system_contract"},
                        {"name": "strict_tool_schemas"}
                    ]
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-schema".into(),
                event_type: crate::state::EventType::ToolSchemaFailure,
                schema_version: 1,
                timestamp_ms: 3,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "strict_tool_schema",
                    "schema_name": "propose_edit",
                    "schema_version": 2,
                    "valid": false,
                    "repair_action": "retry"
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_policies_report(&sqlite_path, "evt-failure", 4, 20).unwrap();

        assert!(report.contains("State graph policies: evt-failure depth=4 limit=20"));
        assert!(report.contains("policies:   deepseek_native@v2"));
        assert!(report.contains("schemas:    propose_edit, propose_edit@v2"));
        assert!(report.contains("prompts:    deepseek_native_prompt@v7, prompt_layout_v7"));
        assert!(report.contains(
            "blocks:     deepseek_native_system_contract, selected_recent_events, strict_tool_schemas"
        ));
        assert!(report.contains("instruction files: AGENTS.md, YOYO.md"));
        assert!(report.contains("uses_context_policy=1"));
        assert!(report.contains("uses_instruction_file=2"));
        assert!(report.contains("uses_schema_version=1"));
        assert!(report.contains(
            "policy=deepseek_native@v2 layout=prompt_layout_v7 prompt=deepseek_native_prompt@v7 schema=- version=- source_audit=- source_scan=- source_findings=- blocks=2/1 instructions=[YOYO.md, AGENTS.md] included=[deepseek_native_system_contract, strict_tool_schemas] run=run-1"
        ));
        assert!(report.contains(
            "policy=- layout=- prompt=- schema=propose_edit version=2 source_audit=- source_scan=- source_findings=- blocks=0/0 instructions=[] included=[] run=run-1"
        ));
        assert!(report.contains("evt-context -[uses_context_policy]-> deepseek_native@v2"));
        assert!(report.contains("evt-context -[uses_instruction_file]-> YOYO.md"));
        assert!(report.contains("evt-schema -[uses_schema_version]-> propose_edit@v2"));

        let payload = build_graph_policies_payload(&sqlite_path, "evt-failure", 4, 20).unwrap();

        assert_eq!(
            payload["diagnostic"].as_str().unwrap(),
            "state_graph_policies"
        );
        assert_eq!(payload["id"].as_str().unwrap(), "evt-failure");
        assert_eq!(payload["depth"].as_u64().unwrap(), 4);
        assert_eq!(payload["limit"].as_u64().unwrap(), 20);
        assert_eq!(payload["policy_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["schema_count"].as_u64().unwrap(), 2);
        assert_eq!(payload["prompt_count"].as_u64().unwrap(), 2);
        assert_eq!(payload["block_count"].as_u64().unwrap(), 3);
        assert_eq!(
            payload["policies"][0].as_str().unwrap(),
            "deepseek_native@v2"
        );
        assert_eq!(payload["schemas"][0].as_str().unwrap(), "propose_edit");
        assert_eq!(payload["schemas"][1].as_str().unwrap(), "propose_edit@v2");
        assert_eq!(
            payload["prompts"][0].as_str().unwrap(),
            "deepseek_native_prompt@v7"
        );
        assert_eq!(payload["prompts"][1].as_str().unwrap(), "prompt_layout_v7");
        assert_eq!(
            payload["blocks"][0].as_str().unwrap(),
            "deepseek_native_system_contract"
        );
        assert_eq!(payload["instruction_file_count"].as_u64().unwrap(), 2);
        assert_eq!(
            payload["instruction_files"][0].as_str().unwrap(),
            "AGENTS.md"
        );
        assert_eq!(
            payload["relations"]["uses_context_policy"]
                .as_u64()
                .unwrap(),
            1
        );
        assert_eq!(
            payload["relations"]["uses_instruction_file"]
                .as_u64()
                .unwrap(),
            2
        );
        assert_eq!(
            payload["relations"]["uses_schema_version"]
                .as_u64()
                .unwrap(),
            1
        );
        let context_metadata = &payload["policy_metadata"]["evt-context"];
        assert_eq!(
            context_metadata["context_policy"].as_str().unwrap(),
            "deepseek_native@v2"
        );
        assert_eq!(
            context_metadata["prompt_layout"].as_str().unwrap(),
            "prompt_layout_v7"
        );
        assert_eq!(
            context_metadata["prompt_version"].as_str().unwrap(),
            "deepseek_native_prompt@v7"
        );
        assert_eq!(context_metadata["stable_blocks"].as_u64().unwrap(), 2);
        assert_eq!(context_metadata["dynamic_blocks"].as_u64().unwrap(), 1);
        assert_eq!(
            context_metadata["included_blocks"][0].as_str().unwrap(),
            "deepseek_native_system_contract"
        );
        assert_eq!(
            context_metadata["instruction_files"][0].as_str().unwrap(),
            "YOYO.md"
        );
        assert_eq!(context_metadata["run_id"].as_str().unwrap(), "run-1");
        let schema_metadata = &payload["policy_metadata"]["evt-schema"];
        assert_eq!(
            schema_metadata["schema_name"].as_str().unwrap(),
            "propose_edit"
        );
        assert_eq!(schema_metadata["schema_version"].as_str().unwrap(), "2");
        let policy_relations = payload["policy_relations"].as_array().unwrap();
        let context_relation = policy_relations
            .iter()
            .find(|row| {
                row["event_id"].as_str() == Some("evt-context")
                    && row["relation"].as_str() == Some("uses_context_policy")
            })
            .unwrap();
        assert_eq!(context_relation["src_id"].as_str().unwrap(), "evt-context");
        assert_eq!(
            context_relation["dst_id"].as_str().unwrap(),
            "deepseek_native@v2"
        );
        assert_eq!(
            context_relation["metadata_event_id"].as_str().unwrap(),
            "evt-context"
        );
        let instruction_relation = policy_relations
            .iter()
            .find(|row| {
                row["event_id"].as_str() == Some("evt-context")
                    && row["relation"].as_str() == Some("uses_instruction_file")
                    && row["dst_id"].as_str() == Some("YOYO.md")
            })
            .unwrap();
        assert_eq!(
            instruction_relation["dst_kind"].as_str().unwrap(),
            "instruction_file"
        );
        let schema_relation = policy_relations
            .iter()
            .find(|row| {
                row["event_id"].as_str() == Some("evt-schema")
                    && row["relation"].as_str() == Some("uses_schema_version")
            })
            .unwrap();
        assert_eq!(
            schema_relation["dst_id"].as_str().unwrap(),
            "propose_edit@v2"
        );
    }

    #[test]
    fn graph_policies_report_surfaces_release_source_provenance_policy() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 10,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "source provenance audit did not pass",
                "source_provenance_passed": false,
                "source_provenance_findings": 1,
                "source_provenance_scan_source": "git",
                "source_provenance_finding_summaries": [
                    "src/a.rs: source path escapes repository"
                ]
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_policies_report(&sqlite_path, "evt-release-gate", 2, 20).unwrap();

        assert!(report.contains("State graph policies: evt-release-gate depth=2 limit=20"));
        assert!(report
            .contains("policies:   release_source_provenance_audit, source_provenance_scan:git"));
        assert!(report.contains("blocked_by_source_provenance_audit=1"));
        assert!(report.contains("used_source_provenance_scan=1"));
        assert!(report.contains("supports_source_provenance_audit=2"));
        assert!(report.contains("source_audit=blocked source_scan=git source_findings=1"));
        assert!(report.contains(
            "evt-release-gate -[blocked_by_source_provenance_audit]-> release_source_provenance_audit (policy)"
        ));
        assert!(report.contains(
            "source_provenance_scan:git -[supports_source_provenance_audit]-> release_source_provenance_audit (policy)"
        ));
    }

    #[test]
    fn graph_policies_report_surfaces_json_output_schema_validation() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-json-pass".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 10,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-json".into()),
            session_id: None,
            trace_id: "trace-json".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "json_output",
                "decision_type": "deepseek_json_output_check",
                "check": "json-check",
                "decision": "passed",
                "schema_name": "summary",
                "attempt_count": 1,
                "retry_used": false,
                "attempt_statuses": ["parsed"]
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_policies_report(&sqlite_path, "evt-json-pass", 2, 20).unwrap();

        assert!(report.contains("State graph policies: evt-json-pass depth=2 limit=20"));
        assert!(report.contains("schemas:    summary"));
        assert!(report.contains("uses_schema=1"));
        assert!(report.contains("supports_json_output_check=1"));
        assert!(report.contains("schema=summary"));
        assert!(report.contains("summary -[supports_json_output_check]-> evt-json-pass (event)"));
    }

    #[test]
    fn graph_policies_report_surfaces_strict_tool_call_schema_validation() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-strict-pass".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 10,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-strict".into()),
            session_id: None,
            trace_id: "trace-strict".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "deepseek_protocol_check",
                "decision_type": "deepseek_strict_tool_call_check",
                "check": "test-tool-call",
                "decision": "passed",
                "schema_count": 3,
                "schema_names": ["inspect_file", "propose_edit", "record_failure"],
                "selected_tool_count": 2,
                "selected_tool_names": ["inspect_file", "propose_edit"],
                "model": "deepseek-v4-pro",
                "thinking": "enabled",
                "reasoning_effort": "high",
                "stream": false,
                "max_tokens": 512
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_policies_report(&sqlite_path, "evt-strict-pass", 2, 20).unwrap();

        assert!(report.contains("State graph policies: evt-strict-pass depth=2 limit=20"));
        assert!(report.contains("schemas:    inspect_file, propose_edit, record_failure"));
        assert!(report.contains("uses_schema=2"));
        assert!(report.contains("supports_strict_tool_call_check=3"));
        assert!(report
            .contains("inspect_file -[supports_strict_tool_call_check]-> evt-strict-pass (event)"));
    }

    #[test]
    fn graph_policies_report_surfaces_transport_policy_check_pass() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-transport-pass".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 10,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-transport".into()),
            session_id: None,
            trace_id: "trace-transport".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "deepseek_protocol_check",
                "decision_type": "deepseek_transport_policy_check",
                "check": "transport-check",
                "decision": "passed",
                "transport_class": "rate_limited",
                "status": 429,
                "attempt": 0,
                "max_retries": 2,
                "retryable": true,
                "next_backoff_ms": 1000,
                "reason": "rate limit response can be retried with bounded backoff",
                "error_preview": "rate limit"
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report =
            build_graph_policies_report(&sqlite_path, "evt-transport-pass", 2, 20).unwrap();

        assert!(report.contains("State graph policies: evt-transport-pass depth=2 limit=20"));
        assert!(report.contains("policies:   deepseek_transport_policy"));
        assert!(report.contains("uses_transport_policy=1"));
        assert!(report.contains("supports_transport_policy_check=1"));
        assert!(report.contains(
            "evt-transport-pass -[uses_transport_policy]-> deepseek_transport_policy (policy)"
        ));
        assert!(report.contains(
            "transport_class:rate_limited -[supports_transport_policy_check]-> evt-transport-pass (event)"
        ));
    }

    #[test]
    fn graph_policies_report_surfaces_thinking_protocol_check_pass() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-thinking-pass".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 10,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-thinking".into()),
            session_id: None,
            trace_id: "trace-thinking".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "deepseek_protocol_check",
                "decision_type": "deepseek_thinking_protocol_check",
                "check": "test-thinking",
                "decision": "passed",
                "diagnostic_source": "builtin-probe",
                "probe": {
                    "source": "builtin-probe",
                    "message_count": 2,
                    "assistant_tool_call_turns": 1,
                    "assistant_tool_call_turns_with_reasoning_content": 1,
                    "assistant_tool_call_turns_missing_reasoning_content": 0,
                    "tool_result_turns": 1
                }
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_policies_report(&sqlite_path, "evt-thinking-pass", 2, 20).unwrap();

        assert!(report.contains("State graph policies: evt-thinking-pass depth=2 limit=20"));
        assert!(report.contains("policies:   deepseek_thinking_protocol_policy"));
        assert!(report.contains("uses_thinking_protocol_policy=1"));
        assert!(report.contains("supports_thinking_protocol_check=1"));
        assert!(report.contains(
            "evt-thinking-pass -[uses_thinking_protocol_policy]-> deepseek_thinking_protocol_policy (policy)"
        ));
        assert!(report.contains(
            "thinking_probe:builtin-probe -[supports_thinking_protocol_check]-> evt-thinking-pass (event)"
        ));
    }

    #[test]
    fn graph_policies_report_surfaces_streaming_protocol_check_pass() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-stream-pass".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 10,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-stream".into()),
            session_id: None,
            trace_id: "trace-stream".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "deepseek_protocol_check",
                "decision_type": "deepseek_streaming_protocol_check",
                "check": "stream-check",
                "decision": "passed",
                "content_chars": 4,
                "reasoning_content_chars": 16,
                "tool_call_count": 1,
                "finish_reason": "stop",
                "input_tokens": 12,
                "output_tokens": 3,
                "cache_hit_tokens": 8,
                "cache_miss_tokens": 4
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_policies_report(&sqlite_path, "evt-stream-pass", 2, 20).unwrap();

        assert!(report.contains("State graph policies: evt-stream-pass depth=2 limit=20"));
        assert!(report.contains("policies:   deepseek_streaming_protocol_policy"));
        assert!(report.contains("uses_streaming_protocol_policy=1"));
        assert!(report.contains("supports_streaming_protocol_check=1"));
        assert!(report.contains(
            "evt-stream-pass -[uses_streaming_protocol_policy]-> deepseek_streaming_protocol_policy (policy)"
        ));
        assert!(report.contains(
            "streaming_probe:stream-check -[supports_streaming_protocol_check]-> evt-stream-pass (event)"
        ));
    }

    #[test]
    fn graph_signals_report_focuses_positive_and_risk_edges() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-patch".into(),
                event_type: crate::state::EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 1,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-1",
                    "evidence_event_ids": ["evt-failure"],
                    "status": "proposed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-eval".into(),
                event_type: crate::state::EventType::PatchEvaluated,
                schema_version: 1,
                timestamp_ms: 2,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "eval_id": "eval-1",
                    "patch_id": "patch-1",
                    "suite": "local-smoke",
                    "status": "passed",
                    "score": 1.0,
                    "passed": 1,
                    "failed": 0,
                    "metrics": {},
                    "failure_event_ids": ["evt-failure"],
                    "created_at_ms": 2
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_signals_report(&sqlite_path, "evt-failure", 4).unwrap();

        assert!(report.contains("State graph signals: evt-failure depth=4"));
        assert!(report.contains("positives: validated_by=1"));
        assert!(report.contains("risks:     evaluated_failure=1"));
        assert!(report.contains("positive paths"));
        assert!(report.contains("risk paths"));
        assert!(report.contains("patch-1 -[validated_by]-> eval-1"));
        assert!(report.contains("-[evaluated_failure]-> evt-failure"));

        let payload = build_graph_signals_payload(&sqlite_path, "evt-failure", 4).unwrap();
        assert_eq!(payload["diagnostic"], "state_graph_signals");
        assert_eq!(payload["id"], "evt-failure");
        assert_eq!(payload["depth"], 4);
        assert_eq!(payload["positive_counts"]["validated_by"], 1);
        assert_eq!(payload["risk_counts"]["evaluated_failure"], 1);
        assert!(payload["positive_paths"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["src_id"] == "patch-1"
                && edge["relation"] == "validated_by"
                && edge["dst_id"] == "eval-1"));
        assert!(
            payload["risk_paths"]
                .as_array()
                .unwrap()
                .iter()
                .any(|edge| edge["relation"] == "evaluated_failure"
                    && edge["dst_id"] == "evt-failure")
        );
    }

    #[test]
    fn graph_signals_report_surfaces_release_gate_missing_required_gates() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "latest eval is missing required gate evidence",
                "missing_required_gates": ["cargo fmt --check"],
                "source_provenance_passed": true
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_signals_report(&sqlite_path, "evt-release-gate", 2).unwrap();

        assert!(report.contains("State graph signals: evt-release-gate depth=2"));
        assert!(report.contains("risks:     blocks_release_gate=1, missing_required_gate=1"));
        assert!(report.contains("risk paths"));
        assert!(report.contains(
            "evt-release-gate -[missing_required_gate]-> required_gate:cargo fmt --check"
        ));
        assert!(report
            .contains("required_gate:cargo fmt --check -[blocks_release_gate]-> evt-release-gate"));
    }

    #[test]
    fn graph_signals_report_surfaces_release_gate_protocol_check_counts() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "release_ready",
                "suite": "local-smoke",
                "reason": "latest eval and protocol eval passed and are fresh",
                "require_protocol": true,
                "protocol_eval_id": "eval-protocol-pass",
                "protocol_eval_status": "passed",
                "protocol_eval_git_dirty": false,
                "protocol_stale": false,
                "protocol_older_than_eval": false,
                "protocol_check_counts": {
                    "total": 5,
                    "passes": 5,
                    "strict": 1,
                    "thinking": 1,
                    "stream": 1,
                    "json": 1,
                    "transport": 1
                }
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_signals_report(&sqlite_path, "evt-release-gate", 2).unwrap();

        assert!(report.contains("State graph signals: evt-release-gate depth=2"));
        assert!(report.contains("positives: covers_protocol_check=5"));
        assert!(report.contains("positive paths"));
        assert!(report.contains(
            "evt-release-gate -[covers_protocol_check]-> deepseek_protocol_check:streaming"
        ));
        assert!(report.contains(
            "evt-release-gate -[covers_protocol_check]-> deepseek_protocol_check:transport_policy"
        ));
    }

    #[test]
    fn graph_protocol_report_surfaces_release_gate_protocol_check_counts() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "release_ready",
                "suite": "local-smoke",
                "reason": "latest eval and protocol eval passed and are fresh",
                "require_protocol": true,
                "protocol_eval_id": "eval-protocol-pass",
                "protocol_eval_status": "passed",
                "protocol_eval_git_dirty": false,
                "protocol_stale": false,
                "protocol_older_than_eval": false,
                "protocol_check_counts": {
                    "total": 5,
                    "passes": 5,
                    "strict": 1,
                    "thinking": 1,
                    "stream": 1,
                    "json": 1,
                    "transport": 1
                }
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_protocol_report(&sqlite_path, "evt-release-gate", 2, 20).unwrap();

        assert!(report.contains("State graph protocol: evt-release-gate depth=2 limit=20"));
        assert!(report.contains("checks:    deepseek_protocol_check:json_output"));
        assert!(report.contains("deepseek_protocol_check:streaming"));
        assert!(report.contains("deepseek_protocol_check:transport_policy"));
        assert!(report.contains("events:    evt-release-gate"));
        assert!(report.contains("by relation: covers_protocol_check=5"));
        assert!(report.contains(
            "evt-release-gate -[covers_protocol_check]-> deepseek_protocol_check:streaming"
        ));

        let payload =
            build_graph_protocol_payload(&sqlite_path, "evt-release-gate", 2, 20).unwrap();

        assert_eq!(
            payload["diagnostic"].as_str().unwrap(),
            "state_graph_protocol"
        );
        assert_eq!(payload["id"].as_str().unwrap(), "evt-release-gate");
        assert_eq!(payload["depth"].as_u64().unwrap(), 2);
        assert_eq!(payload["limit"].as_u64().unwrap(), 20);
        assert_eq!(payload["check_count"].as_u64().unwrap(), 5);
        assert_eq!(payload["eval_count"].as_u64().unwrap(), 0);
        assert_eq!(payload["decision_count"].as_u64().unwrap(), 0);
        assert_eq!(payload["event_count"].as_u64().unwrap(), 1);
        assert_eq!(payload["relation_count"].as_u64().unwrap(), 5);
        assert_eq!(
            payload["checks"][0].as_str().unwrap(),
            "deepseek_protocol_check:json_output"
        );
        assert!(payload["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|check| check.as_str() == Some("deepseek_protocol_check:streaming")));
        assert_eq!(payload["events"][0].as_str().unwrap(), "evt-release-gate");
        assert_eq!(
            payload["relations"]["covers_protocol_check"]
                .as_u64()
                .unwrap(),
            5
        );
        let protocol_relations = payload["protocol_relations"].as_array().unwrap();
        let streaming = protocol_relations
            .iter()
            .find(|row| {
                row["relation"].as_str() == Some("covers_protocol_check")
                    && row["dst_id"].as_str() == Some("deepseek_protocol_check:streaming")
            })
            .unwrap();
        assert_eq!(streaming["event_id"].as_str().unwrap(), "evt-release-gate");
        assert_eq!(streaming["src_id"].as_str().unwrap(), "evt-release-gate");
        assert_eq!(streaming["dst_kind"].as_str().unwrap(), "evidence");
    }

    #[test]
    fn graph_signals_report_surfaces_release_gate_fixture_breadth_blocks() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "latest eval fixture suite breadth is below required minimum",
                "last_eval_fixture_task_count": 244,
                "last_eval_fixture_command_count": 488,
                "min_fixture_task_count": 245,
                "min_fixture_command_count": 490,
                "fixture_breadth_satisfied": false
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_signals_report(&sqlite_path, "evt-release-gate", 2).unwrap();

        assert!(report.contains("State graph signals: evt-release-gate depth=2"));
        assert!(
            report.contains("risks:     blocks_release_gate=1, fixture_breadth_below_minimum=1")
        );
        assert!(report.contains(
            "evt-release-gate -[fixture_breadth_below_minimum]-> release_fixture_breadth_minimum"
        ));
        assert!(report
            .contains("release_fixture_breadth_minimum -[blocks_release_gate]-> evt-release-gate"));
    }

    #[test]
    fn graph_signals_report_surfaces_release_gate_fixture_risk_blocks() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "latest eval fixture risk coverage is below required minimum",
                "last_eval_fixture_risk_labels": {
                    "high": 4,
                    "medium": 120
                },
                "min_fixture_risk_labels": {
                    "high": 5
                },
                "fixture_risk_satisfied": false
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_signals_report(&sqlite_path, "evt-release-gate", 2).unwrap();

        assert!(report.contains("State graph signals: evt-release-gate depth=2"));
        assert!(report
            .contains("risks:     blocks_release_gate=1, fixture_risk_coverage_below_minimum=1"));
        assert!(report.contains(
            "evt-release-gate -[fixture_risk_coverage_below_minimum]-> release_fixture_risk_minimum"
        ));
        assert!(report
            .contains("release_fixture_risk_minimum -[blocks_release_gate]-> evt-release-gate"));
    }

    #[test]
    fn graph_signals_report_surfaces_release_gate_fixture_agent_mutation_scope_blocks() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "latest eval has fixture agent mutation-scope failures: 1",
                "last_eval_id": "eval-scope-fail",
                "last_eval_status": "passed",
                "last_eval_mutation_scope_failures": 1,
                "last_eval_unexpected_changed_files": 3,
                "source_provenance_passed": true
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_signals_report(&sqlite_path, "evt-release-gate", 2).unwrap();

        assert!(report.contains("State graph signals: evt-release-gate depth=2"));
        assert!(report
            .contains("risks:     blocks_release_gate=1, fixture_agent_mutation_scope_block=1"));
        assert!(report.contains(
            "evt-release-gate -[fixture_agent_mutation_scope_block]-> release_fixture_agent_mutation_scope"
        ));
        assert!(report.contains(
            "release_fixture_agent_mutation_scope -[blocks_release_gate]-> evt-release-gate"
        ));
    }

    #[test]
    fn graph_signals_report_surfaces_release_gate_source_provenance_paths() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "source provenance audit failed",
                "source_provenance_passed": false,
                "source_provenance_findings": 1,
                "source_provenance_scan_source": "git",
                "source_provenance_finding_summaries": [
                    "src/a.rs: source path escapes repository"
                ]
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_signals_report(&sqlite_path, "evt-release-gate", 2).unwrap();

        assert!(report.contains("State graph signals: evt-release-gate depth=2"));
        assert!(report.contains("positives: supports_source_provenance_audit=2"));
        assert!(report.contains("risks:     blocked_by_source_provenance_audit=1"));
        assert!(report.contains("positive paths"));
        assert!(report.contains("risk paths"));
        assert!(report.contains(
            "evt-release-gate -[blocked_by_source_provenance_audit]-> release_source_provenance_audit"
        ));
        assert!(report.contains(
            "source_provenance_finding:src/a.rs: source path escapes repository -[supports_source_provenance_audit]-> release_source_provenance_audit"
        ));
    }

    #[test]
    fn graph_signals_report_surfaces_promotion_protocol_eval_support() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-promote".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-promote".into()),
            session_id: None,
            trace_id: "trace-promote".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "harness_patch_promotion",
                "decision": "promote",
                "patch_id": "patch-1",
                "reason": "candidate score improved",
                "promotion_decision": {
                    "eligible": true,
                    "criterion": "pass_rate_improved",
                    "reason": "candidate score improves over baseline",
                    "candidate_eval_id": "eval-candidate",
                    "protocol_eval_id": "eval-protocol-pass"
                }
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_signals_report(&sqlite_path, "evt-promote", 2).unwrap();

        assert!(report.contains("State graph signals: evt-promote depth=2"));
        assert!(report.contains("positives: supports_promotion=1"));
        assert!(report.contains("eval-protocol-pass -[supports_promotion]-> evt-promote"));
    }

    #[test]
    fn graph_signals_report_surfaces_json_output_check_pass() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-json-pass".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-json".into()),
            session_id: None,
            trace_id: "trace-json".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "json_output",
                "decision_type": "deepseek_json_output_check",
                "check": "json-check",
                "decision": "passed",
                "schema_name": "summary",
                "attempt_count": 1,
                "retry_used": false,
                "attempt_statuses": ["parsed"]
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_signals_report(&sqlite_path, "evt-json-pass", 2).unwrap();

        assert!(report.contains("State graph signals: evt-json-pass depth=2"));
        assert!(report.contains("positives: supports_json_output_check=1"));
        assert!(report.contains("positive paths"));
        assert!(report.contains("summary -[supports_json_output_check]-> evt-json-pass"));
    }

    #[test]
    fn graph_signals_report_surfaces_strict_tool_call_check_pass() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-strict-pass".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-strict".into()),
            session_id: None,
            trace_id: "trace-strict".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "deepseek_protocol_check",
                "decision_type": "deepseek_strict_tool_call_check",
                "check": "test-tool-call",
                "decision": "passed",
                "schema_count": 3,
                "schema_names": ["inspect_file", "propose_edit", "record_failure"],
                "selected_tool_count": 2,
                "selected_tool_names": ["inspect_file", "propose_edit"],
                "model": "deepseek-v4-pro",
                "thinking": "enabled",
                "reasoning_effort": "high",
                "stream": false,
                "max_tokens": 512
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_signals_report(&sqlite_path, "evt-strict-pass", 2).unwrap();

        assert!(report.contains("State graph signals: evt-strict-pass depth=2"));
        assert!(report.contains("positives: supports_strict_tool_call_check=3"));
        assert!(report.contains("positive paths"));
        assert!(
            report.contains("inspect_file -[supports_strict_tool_call_check]-> evt-strict-pass")
        );
    }

    #[test]
    fn graph_signals_report_surfaces_transport_policy_check_pass() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-transport-pass".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-transport".into()),
            session_id: None,
            trace_id: "trace-transport".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "deepseek_protocol_check",
                "decision_type": "deepseek_transport_policy_check",
                "check": "transport-check",
                "decision": "passed",
                "transport_class": "rate_limited",
                "status": 429,
                "attempt": 0,
                "max_retries": 2,
                "retryable": true,
                "next_backoff_ms": 1000,
                "reason": "rate limit response can be retried with bounded backoff",
                "error_preview": "rate limit"
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_signals_report(&sqlite_path, "evt-transport-pass", 2).unwrap();

        assert!(report.contains("State graph signals: evt-transport-pass depth=2"));
        assert!(report.contains("positives: supports_transport_policy_check=1"));
        assert!(report.contains("positive paths"));
        assert!(report.contains(
            "transport_class:rate_limited -[supports_transport_policy_check]-> evt-transport-pass"
        ));
    }

    #[test]
    fn graph_signals_report_surfaces_thinking_protocol_check_pass() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-thinking-pass".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-thinking".into()),
            session_id: None,
            trace_id: "trace-thinking".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "deepseek_protocol_check",
                "decision_type": "deepseek_thinking_protocol_check",
                "check": "test-thinking",
                "decision": "passed",
                "diagnostic_source": "builtin-probe",
                "probe": {
                    "source": "builtin-probe",
                    "message_count": 2,
                    "assistant_tool_call_turns": 1,
                    "assistant_tool_call_turns_with_reasoning_content": 1,
                    "assistant_tool_call_turns_missing_reasoning_content": 0,
                    "tool_result_turns": 1
                }
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_signals_report(&sqlite_path, "evt-thinking-pass", 2).unwrap();

        assert!(report.contains("State graph signals: evt-thinking-pass depth=2"));
        assert!(report.contains("positives: supports_thinking_protocol_check=1"));
        assert!(report.contains("positive paths"));
        assert!(report.contains(
            "thinking_probe:builtin-probe -[supports_thinking_protocol_check]-> evt-thinking-pass"
        ));
    }

    #[test]
    fn graph_signals_report_surfaces_streaming_protocol_check_pass() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-stream-pass".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-stream".into()),
            session_id: None,
            trace_id: "trace-stream".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "deepseek_protocol_check",
                "decision_type": "deepseek_streaming_protocol_check",
                "check": "stream-check",
                "decision": "passed",
                "content_chars": 4,
                "reasoning_content_chars": 16,
                "tool_call_count": 1,
                "finish_reason": "stop",
                "input_tokens": 12,
                "output_tokens": 3,
                "cache_hit_tokens": 8,
                "cache_miss_tokens": 4
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_signals_report(&sqlite_path, "evt-stream-pass", 2).unwrap();

        assert!(report.contains("State graph signals: evt-stream-pass depth=2"));
        assert!(report.contains("positives: supports_streaming_protocol_check=1"));
        assert!(report.contains("positive paths"));
        assert!(report.contains(
            "streaming_probe:stream-check -[supports_streaming_protocol_check]-> evt-stream-pass"
        ));
    }

    #[test]
    fn graph_signals_report_surfaces_stale_promotion_protocol_eval() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-promote".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-promote".into()),
            session_id: None,
            trace_id: "trace-promote".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "harness_patch_promotion",
                "decision": "block_promotion",
                "patch_id": "patch-1",
                "reason": "latest protocol eval is older than candidate eval",
                "promotion_decision": {
                    "eligible": false,
                    "reason": "latest protocol eval is older than candidate eval",
                    "candidate_eval_id": "eval-candidate",
                    "protocol_eval_id": "eval-protocol-old"
                }
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_signals_report(&sqlite_path, "evt-promote", 2).unwrap();

        assert!(report.contains("State graph signals: evt-promote depth=2"));
        assert!(report
            .contains("risks:     blocked_by_stale_protocol_eval=1, older_than_candidate_eval=1"));
        assert!(report.contains("eval-protocol-old -[older_than_candidate_eval]-> eval-candidate"));
        assert!(
            report.contains("eval-candidate -[blocked_by_stale_protocol_eval]-> eval-protocol-old")
        );
    }

    #[test]
    fn graph_signals_report_surfaces_promotion_fixture_risk_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-promote".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-promote".into()),
            session_id: None,
            trace_id: "trace-promote".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "harness_patch_promotion",
                "decision": "block_promotion",
                "patch_id": "patch-1",
                "reason": "baseline and candidate fixture suite risk-label coverage differ",
                "promotion_decision": {
                    "eligible": false,
                    "reason": "baseline and candidate fixture suite risk-label coverage differ",
                    "baseline_eval_id": "eval-base",
                    "candidate_eval_id": "eval-candidate"
                }
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_signals_report(&sqlite_path, "evt-promote", 2).unwrap();

        assert!(report.contains("State graph signals: evt-promote depth=2"));
        assert!(report.contains("risks:     blocks_promotion=1, promotion_fixture_risk_mismatch=1"));
        assert!(report.contains(
            "evt-promote -[promotion_fixture_risk_mismatch]-> promotion_fixture_risk_coverage"
        ));
        assert!(
            report.contains("promotion_fixture_risk_coverage -[blocks_promotion]-> evt-promote")
        );
    }

    #[test]
    fn graph_path_report_finds_shortest_projected_relation_path() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-patch".into(),
                event_type: crate::state::EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 1,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-1",
                    "evidence_event_ids": ["evt-failure"],
                    "status": "proposed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-eval".into(),
                event_type: crate::state::EventType::PatchEvaluated,
                schema_version: 1,
                timestamp_ms: 2,
                actor: crate::state::Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "eval_id": "eval-1",
                    "patch_id": "patch-1",
                    "suite": "local-smoke",
                    "status": "passed",
                    "score": 1.0,
                    "passed": 1,
                    "failed": 0,
                    "metrics": {},
                    "failure_event_ids": [],
                    "created_at_ms": 2
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_path_report(&sqlite_path, "evt-failure", "eval-1", 4).unwrap();

        assert!(report.contains("State graph path: evt-failure -> eval-1 depth=2/4"));
        assert!(report.contains("d1 evt-failure <-[addresses]- patch-1"));
        assert!(report.contains("d1 evt-failure <-[addresses]- patch-1 (patch -> event)"));
        assert!(report.contains("d2 patch-1 -[tested_by]-> eval-1"));
        assert!(report.contains("d2 patch-1 -[tested_by]-> eval-1 (patch -> eval)"));

        let missing = build_graph_path_report(&sqlite_path, "evt-failure", "eval-1", 1);
        assert!(missing.unwrap_err().contains("within depth 1"));
    }

    #[test]
    fn graph_path_report_shows_reverse_release_policy_edge_kinds() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [crate::state::StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: crate::state::EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "source provenance audit failed",
                "source_provenance_passed": false,
                "source_provenance_findings": 1,
                "source_provenance_scan_source": "git",
                "source_provenance_finding_summaries": [
                    "src/a.rs: source path escapes repository"
                ]
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();
        crate::state::rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let report = build_graph_path_report(
            &sqlite_path,
            "release_source_provenance_audit",
            "evt-release-gate",
            2,
        )
        .unwrap();

        assert!(report.contains(
            "release_source_provenance_audit <-[blocked_by_source_provenance_audit]- evt-release-gate (event -> policy)"
        ));
    }

    #[test]
    fn tail_json_output_passes_raw_jsonl_lines() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let events = vec![
            json!({"event_id":"evt-a","event_type":"RunStarted","timestamp_ms":100,"run_id":"run-1","payload":{"task":"test"}}),
            json!({"event_id":"evt-b","event_type":"RunCompleted","timestamp_ms":200,"run_id":"run-1","payload":{"status":"success"}}),
            json!({"event_id":"evt-c","event_type":"ToolCallStarted","timestamp_ms":300,"run_id":"run-2","payload":{"tool_name":"bash"}}),
        ];
        write_jsonl(&events_path, &events);

        let lines = read_tail(&events_path, 2).unwrap();
        assert_eq!(lines.len(), 2, "read_tail respects limit");
        // Last two events: evt-b and evt-c
        let parsed: Vec<Value> = lines
            .iter()
            .map(|l| serde_json::from_str::<Value>(l).unwrap())
            .collect();
        assert_eq!(parsed[0]["event_id"], "evt-b");
        assert_eq!(parsed[1]["event_id"], "evt-c");
    }

    #[test]
    fn state_lifecycle_json_reports_dangling_model_call_last_event() {
        let events = vec![
            event_at(
                "evt-run",
                "RunStarted",
                "run-1",
                100,
                json!({"task": "debug state"}),
            ),
            event_at(
                "evt-model-start",
                "ModelCallStarted",
                "run-1",
                110,
                json!({"model": "deepseek-v4-pro"}),
            ),
            event_at(
                "evt-edit",
                "FileEdited",
                "run-1",
                120,
                json!({"path": "journals/JOURNAL.md"}),
            ),
        ];

        let payload = build_state_lifecycle_json(&events);

        assert_eq!(payload["balanced"], false);
        assert_eq!(payload["runs"]["incomplete"], 1);
        assert_eq!(payload["model_calls"]["started"], 1);
        assert_eq!(payload["model_calls"]["completed"], 0);
        assert_eq!(payload["model_calls"]["incomplete"], 1);
        let incomplete = &payload["model_calls"]["incomplete_runs"][0];
        assert_eq!(incomplete["run_id"], "run-1");
        assert_eq!(incomplete["model"], "deepseek-v4-pro");
        assert_eq!(incomplete["last_event"]["event_type"], "FileEdited");
        assert_eq!(incomplete["last_event"]["path"], "journals/JOURNAL.md");
    }

    #[test]
    fn state_lifecycle_json_pairs_stream_closed_terminal_model_call() {
        let events = vec![
            event_at(
                "evt-run",
                "RunStarted",
                "run-1",
                100,
                json!({"task": "debug state"}),
            ),
            event_at(
                "evt-model-start",
                "ModelCallStarted",
                "run-1",
                110,
                json!({"model": "deepseek-v4-pro"}),
            ),
            event_at(
                "evt-model-end",
                "ModelCallCompleted",
                "run-1",
                130,
                json!({
                    "model": "deepseek-v4-pro",
                    "status": "stream_closed_without_agent_end",
                    "error_detail": "event_channel_closed_before_agent_end"
                }),
            ),
            event_at(
                "evt-run-end",
                "RunCompleted",
                "run-1",
                140,
                json!({"status": "success"}),
            ),
        ];

        let payload = build_state_lifecycle_json(&events);

        assert_eq!(payload["balanced"], true);
        assert_eq!(payload["runs"]["incomplete"], 0);
        assert_eq!(payload["model_calls"]["started"], 1);
        assert_eq!(payload["model_calls"]["completed"], 1);
        assert_eq!(payload["model_calls"]["incomplete"], 0);
        assert_eq!(payload["model_calls"]["unmatched_completed"], 0);
    }

    #[test]
    fn state_summary_empty_suggests_diagnostic_paths() {
        let events: Vec<Value> = vec![];
        let summary = build_state_summary(&events);

        assert!(
            summary.contains("State: empty"),
            "should report empty state, got: {summary}"
        );
        assert!(
            summary.contains("Diagnostic paths"),
            "should show diagnostic paths header, got: {summary}"
        );
        assert!(
            summary.contains("state doctor"),
            "should suggest state doctor, got: {summary}"
        );
        assert!(
            summary.contains("state crashes"),
            "should suggest state crashes, got: {summary}"
        );
        assert!(
            summary.contains("state init"),
            "should suggest state init, got: {summary}"
        );
        assert!(
            summary.contains("state tail --limit 5"),
            "should suggest state tail --limit 5, got: {summary}"
        );
        assert!(
            summary.contains("full health check"),
            "should describe what state doctor does, got: {summary}"
        );
        assert!(
            summary.contains("startup crashes"),
            "should describe what state crashes finds, got: {summary}"
        );
        assert!(
            summary.contains("auto-initialized"),
            "should explain auto-init, got: {summary}"
        );
        assert!(
            summary.contains("most recent events"),
            "should describe what state tail shows, got: {summary}"
        );
    }

    #[test]
    fn lifecycle_pairs_matched_model_calls_with_same_run_id() {
        let started = event(
            "mc-1-start",
            "ModelCallStarted",
            "run-1",
            json!({"model_call_id": "mc-1", "model": "deepseek-v4-pro"}),
        );
        let completed = event(
            "mc-1-end",
            "ModelCallCompleted",
            "run-1",
            json!({"model_call_id": "mc-1", "model": "deepseek-v4-pro", "status": "completed"}),
        );
        let result = build_state_lifecycle_json(&[started, completed]);

        assert_eq!(
            result["model_calls"]["started"], 1,
            "expected 1 model call started"
        );
        assert_eq!(
            result["model_calls"]["completed"], 1,
            "expected 1 model call completed"
        );
        assert_eq!(
            result["model_calls"]["incomplete"], 0,
            "expected 0 incomplete model calls"
        );
        assert_eq!(
            result["model_calls"]["unmatched_completed"], 0,
            "expected 0 unmatched completed"
        );
        assert_eq!(
            result["balanced"], true,
            "expected lifecycle to be balanced"
        );
    }

    #[test]
    fn lifecycle_detects_unpaired_model_call_started() {
        let started = event(
            "mc-2-start",
            "ModelCallStarted",
            "run-2",
            json!({"model_call_id": "mc-2", "model": "deepseek-v4-pro"}),
        );
        // No matching ModelCallCompleted
        let result = build_state_lifecycle_json(&[started]);

        assert_eq!(
            result["model_calls"]["started"], 1,
            "expected 1 model call started"
        );
        assert_eq!(
            result["model_calls"]["completed"], 0,
            "expected 0 model calls completed"
        );
        assert!(
            result["model_calls"]["incomplete"].as_u64().unwrap_or(0) > 0,
            "expected incomplete model calls > 0 when started has no matching completed"
        );
        assert_eq!(
            result["balanced"], false,
            "expected lifecycle to be unbalanced"
        );
    }

    #[test]
    fn lifecycle_detects_unmatched_model_call_completed() {
        let completed = event(
            "mc-3-end",
            "ModelCallCompleted",
            "run-3",
            json!({"model_call_id": "mc-3", "model": "deepseek-v4-pro", "status": "completed"}),
        );
        // No matching ModelCallStarted
        let result = build_state_lifecycle_json(&[completed]);

        assert_eq!(
            result["model_calls"]["started"], 0,
            "expected 0 model calls started"
        );
        assert_eq!(
            result["model_calls"]["completed"], 1,
            "expected 1 model call completed"
        );
        assert!(
            result["model_calls"]["unmatched_completed"]
                .as_u64()
                .unwrap_or(0)
                > 0,
            "expected unmatched completed > 0 when completed has no matching started"
        );
        assert_eq!(
            result["balanced"], false,
            "expected lifecycle to be unbalanced"
        );
    }
}
