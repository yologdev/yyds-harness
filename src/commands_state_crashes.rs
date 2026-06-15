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
    let show_all = args.iter().any(|a| a == "--all");
    let path = default_events_path();
    let Ok(events) = read_events(&path) else {
        eprintln!("{YELLOW}  no state log found at {}{RESET}", path.display());
        return;
    };
    match build_crashes_report(&events, limit, json_output, show_all) {
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

    // Split crashes into real vs preflight
    let is_preflight = |c: &CrashEntry| -> bool {
        let detail = c.error_detail.to_lowercase();
        detail.contains("empty_input") || detail.contains("slash_command_in_piped_mode")
    };

    let (preflight_crashes, real_crashes): (Vec<CrashEntry>, Vec<CrashEntry>) =
        crashes.into_iter().partition(|c| is_preflight(c));
    let preflight_hidden = preflight_crashes.len();

    // Select which crashes to display
    let display_crashes: Vec<CrashEntry> = if show_all {
        // In --all mode, merge all: real first, then preflight
        let mut all = real_crashes.clone();
        all.extend(preflight_crashes);
        all
    } else {
        real_crashes
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
                if is_preflight(c) {
                    entry["preflight"] = serde_json::Value::Bool(true);
                }
                entry
            })
            .collect();
        let mut output = serde_json::json!({
            "crashes": entries,
            "total_crashes": total_crashes,
            "window_sessions": window_sessions,
        });
        if !show_all && preflight_hidden > 0 {
            output["preflight_crashes_hidden"] = serde_json::Value::Number(preflight_hidden.into());
        }
        return serde_json::to_string_pretty(&output).map_err(|e| format!("serialize JSON: {e}"));
    }

    // Human-readable table
    if display_crashes.is_empty() {
        if preflight_hidden > 0 {
            return Ok(format!(
                "No real crash sessions found in recent history. ({} harness preflight crash{} hidden; use --all to show)",
                preflight_hidden,
                if preflight_hidden == 1 { "" } else { "es" }
            ));
        }
        return Ok("No crash sessions found in recent history.".to_string());
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
        let mut err = if crash.error_detail.is_empty() {
            "(none)".to_string()
        } else {
            crash.error_detail.clone()
        };
        // In --all mode, label preflight entries
        if show_all && is_preflight(crash) {
            err.push_str(" (preflight)");
        }
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

    if !show_all && preflight_hidden > 0 {
        lines.push(format!(
            "({} harness preflight crash{} hidden; use --all to show)",
            preflight_hidden,
            if preflight_hidden == 1 { "" } else { "es" }
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

    /// Helper: create a RunCompleted error event.
    fn error_event(run_id: &str, ts_ms: i64, error_detail: &str) -> Value {
        json!({
            "event_type": "RunCompleted",
            "run_id": run_id,
            "timestamp_ms": ts_ms,
            "payload": {
                "status": "error",
                "error_detail": error_detail
            }
        })
    }

    /// Helper: create a SessionStarted event.
    fn session_event(run_id: &str, ts_ms: i64, api_key_present: bool) -> Value {
        json!({
            "event_type": "SessionStarted",
            "run_id": run_id,
            "timestamp_ms": ts_ms,
            "payload": {
                "api_key_present": api_key_present
            }
        })
    }

    #[test]
    fn test_hides_preflight_crashes_by_default() {
        let events = vec![
            session_event("run-1", 100, true),
            session_event("run-2", 200, true),
            session_event("run-3", 300, true),
            error_event("run-1", 150, "empty_input"),
            error_event("run-2", 250, "slash_command_in_piped_mode"),
            // Real crash: no tool calls, but not a preflight error
            error_event("run-3", 350, "some_real_error"),
        ];
        let report = build_crashes_report(&events, 10, false, false).unwrap();

        // Should NOT show preflight entries
        assert!(
            !report.contains("empty_input"),
            "preflight crash (empty_input) should be hidden by default:\n{report}"
        );
        assert!(
            !report.contains("slash_command_in_piped_mode"),
            "preflight crash should be hidden by default:\n{report}"
        );
        // Should show real crash
        assert!(
            report.contains("some_real_error"),
            "real crash should be visible:\n{report}"
        );
        // Should have hidden count
        assert!(
            report.contains("2 harness preflight crashes hidden"),
            "should report hidden crash count:\n{report}"
        );
    }

    #[test]
    fn test_show_all_includes_preflight_with_label() {
        let events = vec![
            session_event("run-1", 100, true),
            session_event("run-2", 200, true),
            session_event("run-3", 300, true),
            error_event("run-1", 150, "empty_input"),
            error_event("run-2", 250, "slash_command_in_piped_mode"),
            error_event("run-3", 350, "real_error"),
        ];
        let report = build_crashes_report(&events, 10, false, true).unwrap();

        // Should show preflight entries with (preflight) label
        assert!(
            report.contains("empty_input (preflight)"),
            "--all should show preflight with label:\n{report}"
        );
        assert!(
            report.contains("slash_command_in_piped_mode (preflight)"),
            "--all should show preflight with label:\n{report}"
        );
        // Real crash should NOT have (preflight) label
        let real_line = report.lines().find(|l| l.contains("real_error")).unwrap();
        assert!(
            !real_line.contains("(preflight)"),
            "real error should not get preflight label:\n{real_line}"
        );
        // Should NOT have the hidden message in --all mode
        assert!(
            !report.contains("hidden"),
            "--all mode should not show hidden message:\n{report}"
        );
    }

    #[test]
    fn test_json_mode_includes_preflight_crashes_hidden() {
        let events = vec![
            session_event("run-1", 100, true),
            session_event("run-2", 200, true),
            error_event("run-1", 150, "empty_input"),
            error_event("run-2", 250, "real_error"),
        ];
        let report = build_crashes_report(&events, 10, true, false).unwrap();
        let parsed: Value = serde_json::from_str(&report).unwrap();

        // Should have preflight_crashes_hidden count
        assert_eq!(
            parsed["preflight_crashes_hidden"].as_i64(),
            Some(1),
            "should report 1 hidden preflight crash:\n{report}"
        );
        // Should show only real crash in the crashes array
        let crashes = parsed["crashes"].as_array().unwrap();
        assert_eq!(
            crashes.len(),
            1,
            "should have 1 crash (real only):\n{report}"
        );
        assert_eq!(
            crashes[0]["error_detail"].as_str(),
            Some("real_error"),
            "the visible crash should be the real one:\n{report}"
        );
    }

    #[test]
    fn test_json_all_mode_includes_preflight_with_flag() {
        let events = vec![
            session_event("run-1", 100, true),
            session_event("run-2", 200, true),
            error_event("run-1", 150, "empty_input"),
            error_event("run-2", 250, "real_error"),
        ];
        let report = build_crashes_report(&events, 10, true, true).unwrap();
        let parsed: Value = serde_json::from_str(&report).unwrap();

        // Should NOT have preflight_crashes_hidden in --all mode
        assert!(
            parsed.get("preflight_crashes_hidden").is_none(),
            "--all mode should not include preflight_crashes_hidden:\n{report}"
        );
        // Should have both crashes
        let crashes = parsed["crashes"].as_array().unwrap();
        assert_eq!(
            crashes.len(),
            2,
            "should have 2 crashes in --all mode:\n{report}"
        );
        // Preflight entry should have preflight: true
        let preflight_entry = crashes
            .iter()
            .find(|c| c["error_detail"].as_str() == Some("empty_input"))
            .unwrap();
        assert_eq!(
            preflight_entry["preflight"].as_bool(),
            Some(true),
            "preflight entry should have preflight=true:\n{report}"
        );
        // Real entry should NOT have preflight field
        let real_entry = crashes
            .iter()
            .find(|c| c["error_detail"].as_str() == Some("real_error"))
            .unwrap();
        assert!(
            real_entry.get("preflight").is_none(),
            "real entry should not have preflight field:\n{report}"
        );
    }

    #[test]
    fn test_no_preflight_no_hidden_message() {
        let events = vec![
            session_event("run-1", 100, true),
            error_event("run-1", 150, "real_error"),
        ];
        let report = build_crashes_report(&events, 10, false, false).unwrap();

        assert!(
            !report.contains("hidden"),
            "should not show hidden message when no preflight:\n{report}"
        );
        assert!(
            report.contains("real_error"),
            "should show real crash:\n{report}"
        );
    }

    #[test]
    fn test_all_preflight_shows_only_preflight_in_all_mode() {
        let events = vec![
            session_event("run-1", 100, true),
            session_event("run-2", 200, true),
            error_event("run-1", 150, "empty_input"),
            error_event("run-2", 250, "slash_command_in_piped_mode"),
        ];
        // Default mode: all hidden
        let report_default = build_crashes_report(&events, 10, false, false).unwrap();
        assert!(
            report_default.contains("No real crash sessions"),
            "default should show no real crashes:\n{report_default}"
        );
        assert!(
            report_default.contains("2 harness preflight crashes hidden"),
            "default should report hidden count:\n{report_default}"
        );

        // --all mode: all shown
        let report_all = build_crashes_report(&events, 10, false, true).unwrap();
        assert!(
            report_all.contains("empty_input (preflight)"),
            "--all should show preflight:\n{report_all}"
        );
        assert!(
            report_all.contains("slash_command_in_piped_mode (preflight)"),
            "--all should show preflight:\n{report_all}"
        );
    }
}
