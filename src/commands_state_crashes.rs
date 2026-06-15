//! /state crashes subcommand — detect and report crashed sessions.
//! Extracted from commands_state.rs.

use crate::commands_state::{
    default_events_path, event_string, event_timestamp_ms, flag_value, format_timestamp_ms,
    read_events,
};
use crate::format::*;
use serde_json::Value;

pub(crate) fn handle_crashes(args: &[String]) {
    let limit = flag_value(args, "--limit")
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(10);
    let json_output = args.iter().any(|a| a == "--json");
    let path = default_events_path();
    let Ok(events) = read_events(&path) else {
        eprintln!("{YELLOW}  no state log found at {}{RESET}", path.display());
        return;
    };
    match build_crashes_report(&events, limit, json_output) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

#[derive(Debug, Clone)]
struct CrashEntry {
    run_id: String,
    ts_ms: i64,
    api_key_present: bool,
    error_detail: String,
    duration_ms: i64,
}

fn build_crashes_report(
    events: &[Value],
    limit: usize,
    json_output: bool,
) -> Result<String, String> {
    // Find all RunCompleted events with status=error, collect crashes
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    // Build a map run_id -> SessionStarted (timestamp_ms, api_key_present)
    let mut sessions: std::collections::HashMap<String, (i64, bool)> =
        std::collections::HashMap::new();
    // Track tool calls per run_id: set of run_ids that have at least one ToolCallStarted
    let mut tool_calls: std::collections::HashSet<String> = std::collections::HashSet::new();

    for event in events {
        let event_type = event_string(event, "event_type").unwrap_or("");
        let run_id = event_string(event, "run_id").unwrap_or("").to_string();
        if run_id.is_empty() {
            continue;
        }
        match event_type {
            "SessionStarted" => {
                let ts = event_timestamp_ms(event).unwrap_or(0);
                let api_key = event
                    .get("payload")
                    .and_then(|p| p.get("api_key_present"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                sessions.insert(run_id, (ts, api_key));
            }
            "ToolCallStarted" => {
                tool_calls.insert(run_id);
            }
            _ => {}
        }
    }

    // Now scan RunCompleted events (most recent first), find crashes
    let mut crashes: Vec<CrashEntry> = Vec::new();
    for event in events.iter().rev() {
        if crashes.len() >= limit {
            break;
        }
        let event_type = event_string(event, "event_type").unwrap_or("");
        if event_type != "RunCompleted" {
            continue;
        }
        let run_id = event_string(event, "run_id").unwrap_or("").to_string();
        if run_id.is_empty() {
            continue;
        }
        let status = event
            .get("payload")
            .and_then(|p| p.get("status"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if status != "error" {
            continue;
        }

        // Must have a matching SessionStarted
        let Some(&(session_ts, api_key_present)) = sessions.get(&run_id) else {
            continue; // incomplete data
        };

        // Crash condition: api_key_present == false OR no tool calls
        let has_tool_calls = tool_calls.contains(&run_id);
        if api_key_present && has_tool_calls {
            continue; // not a crash session
        }

        let ts_ms = event_timestamp_ms(event).unwrap_or(0);
        let error_detail = event
            .get("payload")
            .and_then(|p| p.get("error_detail"))
            .and_then(|v| v.as_str())
            .or_else(|| {
                event
                    .get("payload")
                    .and_then(|p| p.get("error"))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("")
            .to_string();
        let duration_ms = ts_ms.saturating_sub(session_ts);

        crashes.push(CrashEntry {
            run_id,
            ts_ms,
            api_key_present,
            error_detail,
            duration_ms,
        });
    }

    let total_crashes = crashes.len();
    let window_sessions = sessions.len();

    if json_output {
        let entries: Vec<Value> = crashes
            .iter()
            .map(|c| {
                serde_json::json!({
                    "run_id": c.run_id,
                    "ts_ms": c.ts_ms,
                    "api_key_present": c.api_key_present,
                    "error_detail": c.error_detail,
                    "duration_ms": c.duration_ms,
                })
            })
            .collect();
        let output = serde_json::json!({
            "crashes": entries,
            "total_crashes": total_crashes,
            "window_sessions": window_sessions,
        });
        return serde_json::to_string_pretty(&output).map_err(|e| format!("serialize JSON: {e}"));
    }

    // Human-readable table
    if crashes.is_empty() {
        return Ok("No crash sessions found in recent history.".to_string());
    }

    let mut lines: Vec<String> = Vec::new();
    let header = format!("Crashed sessions (last {}):", crashes.len());
    lines.push(header);
    lines.push(format!(
        "  {:<40} {:<18} {:<5}  {}",
        "RUN", "WHEN", "KEY?", "ERROR"
    ));

    for crash in &crashes {
        let when = if now > crash.ts_ms {
            format_relative_ms(now - crash.ts_ms)
        } else {
            format_timestamp_ms(crash.ts_ms)
        };
        let key_str = if crash.api_key_present { "yes" } else { "no" };
        let err = if crash.error_detail.is_empty() {
            "(none)"
        } else {
            &crash.error_detail
        };
        // Truncate run_id for display
        let short_run = if crash.run_id.len() > 38 {
            format!("{}…", &crash.run_id[..37])
        } else {
            crash.run_id.clone()
        };
        lines.push(format!(
            "  {:<40} {:<18} {:<5}  {}",
            short_run, when, key_str, err
        ));
    }

    Ok(lines.join("\n"))
}

fn format_relative_ms(diff_ms: i64) -> String {
    let secs = diff_ms / 1000;
    if secs < 60 {
        format!("{}s ago", secs)
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}
