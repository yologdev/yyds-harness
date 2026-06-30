//! /state crashes subcommand — detect and report crashed sessions.
//! Extracted from commands_state.rs.

use crate::commands_state::{
    default_events_path, event_string, event_timestamp_ms, flag_value, format_timestamp_ms,
};
use crate::format::*;
use serde_json::Value;
use std::collections::VecDeque;
use std::path::Path;

/// Maximum events to scan for crash detection before capping.
/// Matching the state doctor's sampling cap of 20K events.
const MAX_EVENTS_SCAN: usize = 20_000;

/// Read only the last `limit` events from the state log, returning them as parsed JSON.
/// Returns the parsed events, the total line count, and the number actually scanned.
fn read_tail_events_capped(
    path: &Path,
    cap: usize,
) -> Result<(Vec<Value>, usize, usize), std::io::Error> {
    let raw = std::fs::read_to_string(path)?;
    let total: usize = raw.lines().count();

    let lines: VecDeque<&str> = {
        let mut deque = VecDeque::new();
        for line in raw.lines() {
            deque.push_back(line);
            if deque.len() > cap {
                deque.pop_front();
            }
        }
        deque
    };
    let scanned = lines.len();

    let events: Vec<Value> = lines
        .into_iter()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect();

    Ok((events, total, scanned))
}

pub(crate) fn handle_crashes(args: &[String]) {
    let limit = flag_value(args, "--limit")
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(10);
    let json_output = args.iter().any(|a| a == "--json");
    let show_all = args.iter().any(|a| a == "--all");
    let path = default_events_path();

    // Use capped tail read to avoid timeout on large event files.
    // Matching the state doctor's approach: scan only the most recent
    // MAX_EVENTS_SCAN events when the log is larger than that.
    let (events, truncation_note) = match read_tail_events_capped(&path, MAX_EVENTS_SCAN) {
        Ok((events, total, scanned)) => {
            let note = if total > MAX_EVENTS_SCAN {
                format!(
                    "\n{DIM}Scanned most recent {scanned} of {total} events (capped for performance).{RESET}\n"
                )
            } else {
                String::new()
            };
            (events, note)
        }
        Err(_) => {
            eprintln!("{YELLOW}  no state log found at {}{RESET}", path.display());
            return;
        }
    };

    if events.is_empty() {
        eprintln!("{YELLOW}  no state log found at {}{RESET}", path.display());
        return;
    }

    match build_crashes_report(&events, limit, json_output, show_all) {
        Ok(report) => {
            if !truncation_note.is_empty() {
                println!("{truncation_note}");
            }
            println!("{report}");
        }
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
    is_preflight: bool,
}

fn is_preflight_error(error_detail: &str) -> bool {
    error_detail.contains("empty_input") || error_detail.contains("slash_command_in_piped_mode")
}

fn build_crashes_report(
    events: &[Value],
    limit: usize,
    json_output: bool,
    show_all: bool,
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
        let is_preflight = is_preflight_error(&error_detail);

        crashes.push(CrashEntry {
            run_id,
            ts_ms,
            api_key_present,
            error_detail,
            duration_ms,
            is_preflight,
        });
    }

    let total_crashes = crashes.len();
    let preflight_count = crashes.iter().filter(|c| c.is_preflight).count();
    let preflight_crashes_hidden = if show_all { 0 } else { preflight_count };
    let window_sessions = sessions.len();

    // Determine which crashes to display
    let display_crashes: Vec<&CrashEntry> = if show_all {
        crashes.iter().collect()
    } else {
        crashes.iter().filter(|c| !c.is_preflight).collect()
    };

    if json_output {
        let entries: Vec<Value> = display_crashes
            .iter()
            .map(|c| {
                let mut entry = serde_json::json!({
                    "run_id": c.run_id,
                    "ts_ms": c.ts_ms,
                    "api_key_present": c.api_key_present,
                    "error_detail": c.error_detail,
                    "duration_ms": c.duration_ms,
                });
                if c.is_preflight {
                    entry["preflight"] = serde_json::Value::Bool(true);
                }
                entry
            })
            .collect();
        let output = serde_json::json!({
            "crashes": entries,
            "total_crashes": total_crashes,
            "preflight_crashes_hidden": preflight_crashes_hidden,
            "window_sessions": window_sessions,
        });
        return serde_json::to_string_pretty(&output).map_err(|e| format!("serialize JSON: {e}"));
    }

    // Human-readable table
    if display_crashes.is_empty() {
        let mut msg = "No crash sessions found in recent history.".to_string();
        if preflight_crashes_hidden > 0 {
            msg.push(' ');
            msg.push_str(&format!(
                "({} harness preflight crash{} hidden; use --all to show)",
                preflight_crashes_hidden,
                if preflight_crashes_hidden == 1 {
                    ""
                } else {
                    "es"
                }
            ));
        }
        return Ok(msg);
    }

    let mut lines: Vec<String> = Vec::new();
    let header = format!("Crashed sessions (last {}):", display_crashes.len());
    lines.push(header);
    lines.push(format!(
        "  {:<40} {:<18} {:<5}  {}",
        "RUN", "WHEN", "KEY?", "ERROR"
    ));

    for crash in &display_crashes {
        let when = if now > crash.ts_ms {
            format_relative_ms(now - crash.ts_ms)
        } else {
            format_timestamp_ms(crash.ts_ms)
        };
        let key_str = if crash.api_key_present { "yes" } else { "no" };
        let err = if crash.error_detail.is_empty() {
            "(none)".to_string()
        } else if crash.is_preflight {
            format!("{} (preflight)", crash.error_detail)
        } else {
            crash.error_detail.clone()
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

    if preflight_crashes_hidden > 0 {
        lines.push(format!(
            "({} harness preflight crash{} hidden; use --all to show)",
            preflight_crashes_hidden,
            if preflight_crashes_hidden == 1 {
                ""
            } else {
                "es"
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_event(id: &str, kind: &str, run_id: &str, payload: Value) -> Value {
        json!({
            "event_id": id,
            "event_type": kind,
            "schema_version": 1,
            "timestamp_ms": 1000,
            "actor": "harness",
            "run_id": run_id,
            "session_id": null,
            "trace_id": "trace-1",
            "parent_event_ids": [],
            "payload": payload,
        })
    }

    #[test]
    fn preflight_empty_input_filtered_by_default() {
        let events = vec![
            make_event(
                "e1",
                "SessionStarted",
                "run-1",
                json!({"api_key_present": true}),
            ),
            make_event(
                "e2",
                "RunCompleted",
                "run-1",
                json!({"status": "error", "error_detail": "empty_input"}),
            ),
        ];
        let report = build_crashes_report(&events, 10, false, false).unwrap();
        // In default mode, preflight crash should be hidden
        assert!(report.contains("No crash sessions found"));
        assert!(report.contains("1 harness preflight crash hidden"));
    }

    #[test]
    fn preflight_slash_command_filtered_by_default() {
        let events = vec![
            make_event(
                "e1",
                "SessionStarted",
                "run-1",
                json!({"api_key_present": true}),
            ),
            make_event(
                "e2",
                "RunCompleted",
                "run-1",
                json!({"status": "error", "error_detail": "slash_command_in_piped_mode"}),
            ),
        ];
        let report = build_crashes_report(&events, 10, false, false).unwrap();
        assert!(report.contains("No crash sessions found"));
        assert!(report.contains("1 harness preflight crash hidden"));
    }

    #[test]
    fn real_crash_still_appears() {
        let events = vec![
            make_event(
                "e1",
                "SessionStarted",
                "run-1",
                json!({"api_key_present": false}),
            ),
            make_event(
                "e2",
                "RunCompleted",
                "run-1",
                json!({"status": "error", "error_detail": "api_key missing"}),
            ),
        ];
        let report = build_crashes_report(&events, 10, false, false).unwrap();
        assert!(report.contains("Crashed sessions"));
        assert!(report.contains("api_key missing"));
        assert!(!report.contains("preflight"));
    }

    #[test]
    fn show_all_includes_preflight_with_label() {
        let events = vec![
            make_event(
                "e1",
                "SessionStarted",
                "run-1",
                json!({"api_key_present": true}),
            ),
            make_event(
                "e2",
                "RunCompleted",
                "run-1",
                json!({"status": "error", "error_detail": "empty_input"}),
            ),
        ];
        let report = build_crashes_report(&events, 10, false, true).unwrap();
        assert!(report.contains("empty_input (preflight)"));
        assert!(!report.contains("hidden"));
    }

    #[test]
    fn json_output_includes_preflight_crashes_hidden() {
        let events = vec![
            make_event(
                "e1",
                "SessionStarted",
                "run-1",
                json!({"api_key_present": true}),
            ),
            make_event(
                "e2",
                "RunCompleted",
                "run-1",
                json!({"status": "error", "error_detail": "empty_input"}),
            ),
            make_event(
                "e3",
                "SessionStarted",
                "run-2",
                json!({"api_key_present": false}),
            ),
            make_event(
                "e4",
                "RunCompleted",
                "run-2",
                json!({"status": "error", "error_detail": "no api key"}),
            ),
        ];
        let report = build_crashes_report(&events, 10, true, false).unwrap();
        let parsed: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(parsed["preflight_crashes_hidden"], 1);
        assert_eq!(parsed["total_crashes"], 2);
        assert_eq!(parsed["crashes"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn json_show_all_includes_preflight_flag() {
        let events = vec![
            make_event(
                "e1",
                "SessionStarted",
                "run-1",
                json!({"api_key_present": true}),
            ),
            make_event(
                "e2",
                "RunCompleted",
                "run-1",
                json!({"status": "error", "error_detail": "empty_input"}),
            ),
        ];
        let report = build_crashes_report(&events, 10, true, true).unwrap();
        let parsed: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(parsed["preflight_crashes_hidden"], 0);
        assert_eq!(parsed["crashes"][0]["preflight"], true);
    }

    #[test]
    fn preflight_mixed_with_real_default_mode() {
        let events = vec![
            make_event(
                "e1",
                "SessionStarted",
                "run-1",
                json!({"api_key_present": true}),
            ),
            make_event(
                "e2",
                "RunCompleted",
                "run-1",
                json!({"status": "error", "error_detail": "empty_input"}),
            ),
            make_event(
                "e3",
                "SessionStarted",
                "run-2",
                json!({"api_key_present": false}),
            ),
            make_event(
                "e4",
                "RunCompleted",
                "run-2",
                json!({"status": "error", "error_detail": "timeout"}),
            ),
        ];
        let report = build_crashes_report(&events, 10, false, false).unwrap();
        // Should show the real crash (timeout) but not empty_input
        assert!(report.contains("timeout"));
        assert!(!report.contains("empty_input"));
        assert!(report.contains("1 harness preflight crash hidden"));
    }

    #[test]
    fn is_preflight_error_detection() {
        assert!(is_preflight_error("empty_input"));
        assert!(is_preflight_error("some empty_input error"));
        assert!(is_preflight_error("slash_command_in_piped_mode"));
        assert!(is_preflight_error(
            "got slash_command_in_piped_mode failure"
        ));
        assert!(!is_preflight_error("api_key missing"));
        assert!(!is_preflight_error(""));
    }

    #[test]
    fn read_tail_events_capped_truncates() {
        use std::io::Write;
        // Create a temp file with 100 events, test that cap=30 returns only 30
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        for i in 0..100 {
            writeln!(
                tmp,
                r#"{{"event_type":"SessionStarted","run_id":"run-{}","payload":{{"api_key_present":true}}}}"#,
                i
            )
            .unwrap();
        }
        tmp.flush().unwrap();
        let (events, total, scanned) = read_tail_events_capped(tmp.path(), 30).unwrap();
        assert_eq!(total, 100);
        assert_eq!(scanned, 30);
        assert_eq!(events.len(), 30);
    }

    #[test]
    fn read_tail_events_capped_no_cap_when_under_limit() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        for i in 0..5 {
            writeln!(
                tmp,
                r#"{{"event_type":"SessionStarted","run_id":"run-{}","payload":{{"api_key_present":true}}}}"#,
                i
            )
            .unwrap();
        }
        tmp.flush().unwrap();
        let (events, total, scanned) = read_tail_events_capped(tmp.path(), 100).unwrap();
        assert_eq!(total, 5);
        assert_eq!(scanned, 5);
        assert_eq!(events.len(), 5);
    }

    #[test]
    fn read_tail_events_capped_skips_empty_lines() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(
            tmp,
            r#"{{"event_type":"SessionStarted","run_id":"run-a","payload":{{"api_key_present":true}}}}"#
        )
        .unwrap();
        writeln!(tmp, "").unwrap();
        writeln!(
            tmp,
            r#"{{"event_type":"RunCompleted","run_id":"run-b","payload":{{"status":"error","error_detail":"timeout"}}}}"#
        )
        .unwrap();
        writeln!(tmp, "").unwrap();
        tmp.flush().unwrap();
        let (events, total, _scanned) = read_tail_events_capped(tmp.path(), 10).unwrap();
        // Empty lines are counted in total but not parsed as events
        assert_eq!(total, 4);
        assert_eq!(events.len(), 2);
    }
}
