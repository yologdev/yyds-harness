//! Memory subcommand handlers extracted from commands_state.rs.
//!
//! Handles `yoyo state memory` and all memory sub-subcommands.

use crate::commands_state::build_state_memory_records;
use crate::commands_state::build_state_memory_synthesis;
use crate::commands_state::default_events_path;
use crate::commands_state::flag_value;
use crate::commands_state::preview_line;
use crate::commands_state::read_events;
use crate::commands_state::record_state_memory_candidates;
use crate::commands_state::record_state_memory_decision;
use crate::commands_state::write_text_artifact;
use crate::format::*;
use std::path::PathBuf;

pub fn handle_memory(args: &[String]) {
    match args.first().map(|arg| arg.as_str()) {
        Some("synthesize") => handle_memory_synthesize(args),
        Some("list") => handle_memory_list(&args[1..]),
        Some("promote") => handle_memory_decision(&args[1..], true),
        Some("reject") => handle_memory_decision(&args[1..], false),
        _ => print_memory_usage(),
    }
}

fn print_memory_usage() {
    eprintln!("{YELLOW}  Usage: yoyo state memory synthesize [--output PATH] [--record]{RESET}");
    eprintln!(
        "{YELLOW}         yoyo state memory list [--status proposed|promoted|rejected]{RESET}"
    );
    eprintln!("{YELLOW}         yoyo state memory promote <candidate-id> [--reason TEXT]{RESET}");
    eprintln!("{YELLOW}         yoyo state memory reject <candidate-id> [--reason TEXT]{RESET}");
}

fn handle_memory_synthesize(args: &[String]) {
    let record = args.iter().any(|arg| arg == "--record");
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
    match build_state_memory_synthesis(&events) {
        Ok(report) => {
            if record {
                match record_state_memory_candidates(&events, &events_path) {
                    Ok(event_ids) => {
                        println!(
                            "State memory candidates recorded\n  proposed: {}",
                            event_ids.len()
                        );
                        if !event_ids.is_empty() {
                            println!("  events:   {}", event_ids.join(", "));
                        }
                    }
                    Err(e) => {
                        eprintln!("{RED}  failed to record state memory candidates: {e}{RESET}")
                    }
                }
            }
            if let Some(path) = output_path {
                match write_text_artifact(&path, &report, "state memory synthesis") {
                    Ok(()) => println!(
                        "State memory synthesis written\n  output: {}",
                        path.display()
                    ),
                    Err(e) => {
                        eprintln!("{RED}  failed to write state memory synthesis: {e}{RESET}")
                    }
                }
            } else {
                println!("{report}");
            }
        }
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_memory_list(args: &[String]) {
    let status_filter = flag_value(args, "--status").map(|status| status.as_str());
    let events_path = default_events_path();
    let Ok(events) = read_events(&events_path) else {
        eprintln!(
            "{YELLOW}  no state log found at {}{RESET}",
            events_path.display()
        );
        return;
    };
    let records = build_state_memory_records(&events);
    println!("State memory candidates");
    let mut shown = 0usize;
    for record in records {
        if status_filter
            .map(|status| status != record.status)
            .unwrap_or(false)
        {
            continue;
        }
        println!(
            "  {:<10} {:<32} {:<28} {}",
            record.status,
            record.candidate_id,
            record.source,
            preview_line(&record.summary, 90)
        );
        shown += 1;
    }
    if shown == 0 {
        println!("  none");
    }
}

fn handle_memory_decision(args: &[String], promote: bool) {
    let Some(candidate_id) = args.first() else {
        print_memory_usage();
        return;
    };
    let reason = flag_value(args, "--reason")
        .map(|reason| reason.as_str())
        .unwrap_or(if promote {
            "approved for durable memory"
        } else {
            "rejected from durable memory"
        });
    let events_path = default_events_path();
    let Ok(events) = read_events(&events_path) else {
        eprintln!(
            "{YELLOW}  no state log found at {}{RESET}",
            events_path.display()
        );
        return;
    };
    match record_state_memory_decision(&events, &events_path, candidate_id, promote, reason) {
        Ok(event_id) => {
            let action = if promote { "promoted" } else { "rejected" };
            println!("State memory candidate {action}");
            println!("  candidate: {candidate_id}");
            println!("  event:     {event_id}");
        }
        Err(e) => eprintln!("{RED}  failed to record state memory decision: {e}{RESET}"),
    }
}
