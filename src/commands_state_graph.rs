//! Graph subcommand handlers extracted from commands_state.rs.
//!
//! Handles `yoyo state graph` and all graph sub-subcommands.

use crate::commands_state::default_events_path;
use crate::commands_state::default_store_path;
use crate::commands_state::flag_value;
use crate::format::*;

use crate::commands_state::build_graph_artifacts_payload;
use crate::commands_state::build_graph_artifacts_report;
use crate::commands_state::build_graph_cache_payload;
use crate::commands_state::build_graph_cache_report;
use crate::commands_state::build_graph_clusters_payload;
use crate::commands_state::build_graph_clusters_report;
use crate::commands_state::build_graph_commands_payload;
use crate::commands_state::build_graph_commands_report;
use crate::commands_state::build_graph_commits_payload;
use crate::commands_state::build_graph_commits_report;
use crate::commands_state::build_graph_decisions_payload;
use crate::commands_state::build_graph_decisions_report;
use crate::commands_state::build_graph_evals_payload;
use crate::commands_state::build_graph_evals_report;
use crate::commands_state::build_graph_evidence_payload;
use crate::commands_state::build_graph_evidence_report;
use crate::commands_state::build_graph_failures_payload;
use crate::commands_state::build_graph_failures_report;
use crate::commands_state::build_graph_files_payload;
use crate::commands_state::build_graph_files_report;
use crate::commands_state::build_graph_hotspots_payload;
use crate::commands_state::build_graph_hotspots_report;
use crate::commands_state::build_graph_hypotheses_payload;
use crate::commands_state::build_graph_hypotheses_report;
use crate::commands_state::build_graph_impact_payload;
use crate::commands_state::build_graph_impact_report;
use crate::commands_state::build_graph_issues_payload;
use crate::commands_state::build_graph_issues_report;
use crate::commands_state::build_graph_memories_payload;
use crate::commands_state::build_graph_memories_report;
use crate::commands_state::build_graph_models_payload;
use crate::commands_state::build_graph_models_report;
use crate::commands_state::build_graph_patches_payload;
use crate::commands_state::build_graph_patches_report;
use crate::commands_state::build_graph_path_report;
use crate::commands_state::build_graph_policies_payload;
use crate::commands_state::build_graph_policies_report;
use crate::commands_state::build_graph_protocol_payload;
use crate::commands_state::build_graph_protocol_report;
use crate::commands_state::build_graph_report;
use crate::commands_state::build_graph_runs_payload;
use crate::commands_state::build_graph_runs_report;
use crate::commands_state::build_graph_signals_payload;
use crate::commands_state::build_graph_signals_report;
use crate::commands_state::build_graph_summary_payload;
use crate::commands_state::build_graph_summary_report;
use crate::commands_state::build_graph_tests_payload;
use crate::commands_state::build_graph_tests_report;
use crate::commands_state::build_graph_timeline_payload;
use crate::commands_state::build_graph_timeline_report;
use crate::commands_state::build_graph_tools_payload;
use crate::commands_state::build_graph_tools_report;
use crate::commands_state::build_graph_versions_payload;
use crate::commands_state::build_graph_versions_report;

pub fn handle_graph_subcommand(args: &[String]) {
    if args.get(3).map(|arg| arg.as_str()) == Some("clusters") {
        let Some(id) = args.get(4) else {
            eprintln!(
                        "{YELLOW}  Usage: yoyo state graph clusters <event-id|patch-id|eval-id|commit> [--depth N] [--json]{RESET}"
                    );
            return;
        };
        let depth = flag_value(args, "--depth")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(3);
        handle_graph_clusters(id, depth, args.iter().any(|arg| arg == "--json"));
        return;
    }
    if args.get(3).map(|arg| arg.as_str()) == Some("impact") {
        let Some(id) = args.get(4) else {
            eprintln!(
                        "{YELLOW}  Usage: yoyo state graph impact <event-id|patch-id|eval-id|commit> [--depth N] [--json]{RESET}"
                    );
            return;
        };
        let depth = flag_value(args, "--depth")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(3);
        handle_graph_impact(id, depth, args.iter().any(|arg| arg == "--json"));
        return;
    }
    if args.get(3).map(|arg| arg.as_str()) == Some("signals") {
        let Some(id) = args.get(4) else {
            eprintln!(
                        "{YELLOW}  Usage: yoyo state graph signals <event-id|patch-id|eval-id|commit> [--depth N] [--json]{RESET}"
                    );
            return;
        };
        let depth = flag_value(args, "--depth")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(3);
        handle_graph_signals(id, depth, args.iter().any(|arg| arg == "--json"));
        return;
    }
    if args.get(3).map(|arg| arg.as_str()) == Some("evidence") {
        let Some(id) = args.get(4) else {
            eprintln!(
                        "{YELLOW}  Usage: yoyo state graph evidence <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]{RESET}"
                    );
            return;
        };
        let depth = flag_value(args, "--depth")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(3);
        let limit = flag_value(args, "--limit")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(20);
        handle_graph_evidence(id, depth, limit, args.iter().any(|arg| arg == "--json"));
        return;
    }
    if args.get(3).map(|arg| arg.as_str()) == Some("files") {
        let Some(id) = args.get(4) else {
            eprintln!(
                        "{YELLOW}  Usage: yoyo state graph files <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]{RESET}"
                    );
            return;
        };
        let depth = flag_value(args, "--depth")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(3);
        let limit = flag_value(args, "--limit")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(20);
        handle_graph_files(id, depth, limit, args.iter().any(|arg| arg == "--json"));
        return;
    }
    if args.get(3).map(|arg| arg.as_str()) == Some("evals") {
        let Some(id) = args.get(4) else {
            eprintln!(
                        "{YELLOW}  Usage: yoyo state graph evals <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]{RESET}"
                    );
            return;
        };
        let depth = flag_value(args, "--depth")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(3);
        let limit = flag_value(args, "--limit")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(20);
        handle_graph_evals(id, depth, limit, args.iter().any(|arg| arg == "--json"));
        return;
    }
    if args.get(3).map(|arg| arg.as_str()) == Some("patches") {
        let Some(id) = args.get(4) else {
            eprintln!(
                        "{YELLOW}  Usage: yoyo state graph patches <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]{RESET}"
                    );
            return;
        };
        let depth = flag_value(args, "--depth")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(3);
        let limit = flag_value(args, "--limit")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(20);
        handle_graph_patches(id, depth, limit, args.iter().any(|arg| arg == "--json"));
        return;
    }
    if args.get(3).map(|arg| arg.as_str()) == Some("decisions") {
        let Some(id) = args.get(4) else {
            eprintln!(
                        "{YELLOW}  Usage: yoyo state graph decisions <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]{RESET}"
                    );
            return;
        };
        let depth = flag_value(args, "--depth")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(3);
        let limit = flag_value(args, "--limit")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(20);
        handle_graph_decisions(id, depth, limit, args.iter().any(|arg| arg == "--json"));
        return;
    }
    if args.get(3).map(|arg| arg.as_str()) == Some("hypotheses") {
        let Some(id) = args.get(4) else {
            eprintln!(
                        "{YELLOW}  Usage: yoyo state graph hypotheses <event-id|hypothesis-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]{RESET}"
                    );
            return;
        };
        let depth = flag_value(args, "--depth")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(3);
        let limit = flag_value(args, "--limit")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(20);
        handle_graph_hypotheses(id, depth, limit, args.iter().any(|arg| arg == "--json"));
        return;
    }
    if args.get(3).map(|arg| arg.as_str()) == Some("versions") {
        let Some(id) = args.get(4) else {
            eprintln!(
                        "{YELLOW}  Usage: yoyo state graph versions <event-id|harness-version|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]{RESET}"
                    );
            return;
        };
        let depth = flag_value(args, "--depth")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(3);
        let limit = flag_value(args, "--limit")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(20);
        handle_graph_versions(id, depth, limit, args.iter().any(|arg| arg == "--json"));
        return;
    }
    if args.get(3).map(|arg| arg.as_str()) == Some("runs") {
        let Some(id) = args.get(4) else {
            eprintln!(
                        "{YELLOW}  Usage: yoyo state graph runs <event-id|run-id|trace-id|task-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]{RESET}"
                    );
            return;
        };
        let depth = flag_value(args, "--depth")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(3);
        let limit = flag_value(args, "--limit")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(20);
        handle_graph_runs(id, depth, limit, args.iter().any(|arg| arg == "--json"));
        return;
    }
    if args.get(3).map(|arg| arg.as_str()) == Some("artifacts") {
        let Some(id) = args.get(4) else {
            eprintln!(
                        "{YELLOW}  Usage: yoyo state graph artifacts <event-id|artifact-uri|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]{RESET}"
                    );
            return;
        };
        let depth = flag_value(args, "--depth")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(3);
        let limit = flag_value(args, "--limit")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(20);
        handle_graph_artifacts(id, depth, limit, args.iter().any(|arg| arg == "--json"));
        return;
    }
    if args.get(3).map(|arg| arg.as_str()) == Some("models") {
        let Some(id) = args.get(4) else {
            eprintln!(
                        "{YELLOW}  Usage: yoyo state graph models <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]{RESET}"
                    );
            return;
        };
        let depth = flag_value(args, "--depth")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(3);
        let limit = flag_value(args, "--limit")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(20);
        handle_graph_models(id, depth, limit, args.iter().any(|arg| arg == "--json"));
        return;
    }
    if args.get(3).map(|arg| arg.as_str()) == Some("tools") {
        let Some(id) = args.get(4) else {
            eprintln!(
                        "{YELLOW}  Usage: yoyo state graph tools <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]{RESET}"
                    );
            return;
        };
        let depth = flag_value(args, "--depth")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(3);
        let limit = flag_value(args, "--limit")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(20);
        handle_graph_tools(id, depth, limit, args.iter().any(|arg| arg == "--json"));
        return;
    }
    if args.get(3).map(|arg| arg.as_str()) == Some("commands") {
        let Some(id) = args.get(4) else {
            eprintln!(
                        "{YELLOW}  Usage: yoyo state graph commands <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]{RESET}"
                    );
            return;
        };
        let depth = flag_value(args, "--depth")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(3);
        let limit = flag_value(args, "--limit")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(20);
        handle_graph_commands(id, depth, limit, args.iter().any(|arg| arg == "--json"));
        return;
    }
    if args.get(3).map(|arg| arg.as_str()) == Some("tests") {
        let Some(id) = args.get(4) else {
            eprintln!(
                        "{YELLOW}  Usage: yoyo state graph tests <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]{RESET}"
                    );
            return;
        };
        let depth = flag_value(args, "--depth")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(3);
        let limit = flag_value(args, "--limit")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(20);
        handle_graph_tests(id, depth, limit, args.iter().any(|arg| arg == "--json"));
        return;
    }
    if args.get(3).map(|arg| arg.as_str()) == Some("commits") {
        let Some(id) = args.get(4) else {
            eprintln!(
                        "{YELLOW}  Usage: yoyo state graph commits <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]{RESET}"
                    );
            return;
        };
        let depth = flag_value(args, "--depth")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(3);
        let limit = flag_value(args, "--limit")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(20);
        handle_graph_commits(id, depth, limit, args.iter().any(|arg| arg == "--json"));
        return;
    }
    if args.get(3).map(|arg| arg.as_str()) == Some("memories") {
        let Some(id) = args.get(4) else {
            eprintln!(
                        "{YELLOW}  Usage: yoyo state graph memories <event-id|memory-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]{RESET}"
                    );
            return;
        };
        let depth = flag_value(args, "--depth")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(3);
        let limit = flag_value(args, "--limit")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(20);
        handle_graph_memories(id, depth, limit, args.iter().any(|arg| arg == "--json"));
        return;
    }
    if args.get(3).map(|arg| arg.as_str()) == Some("issues") {
        let Some(id) = args.get(4) else {
            eprintln!(
                        "{YELLOW}  Usage: yoyo state graph issues <event-id|issue-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]{RESET}"
                    );
            return;
        };
        let depth = flag_value(args, "--depth")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(3);
        let limit = flag_value(args, "--limit")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(20);
        handle_graph_issues(id, depth, limit, args.iter().any(|arg| arg == "--json"));
        return;
    }
    if args.get(3).map(|arg| arg.as_str()) == Some("cache") {
        let Some(id) = args.get(4) else {
            eprintln!(
                        "{YELLOW}  Usage: yoyo state graph cache <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]{RESET}"
                    );
            return;
        };
        let depth = flag_value(args, "--depth")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(3);
        let limit = flag_value(args, "--limit")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(20);
        handle_graph_cache(id, depth, limit, args.iter().any(|arg| arg == "--json"));
        return;
    }
    if args.get(3).map(|arg| arg.as_str()) == Some("failures") {
        let Some(id) = args.get(4) else {
            eprintln!(
                        "{YELLOW}  Usage: yoyo state graph failures <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]{RESET}"
                    );
            return;
        };
        let depth = flag_value(args, "--depth")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(3);
        let limit = flag_value(args, "--limit")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(20);
        handle_graph_failures(id, depth, limit, args.iter().any(|arg| arg == "--json"));
        return;
    }
    if args.get(3).map(|arg| arg.as_str()) == Some("policies") {
        let Some(id) = args.get(4) else {
            eprintln!(
                        "{YELLOW}  Usage: yoyo state graph policies <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]{RESET}"
                    );
            return;
        };
        let depth = flag_value(args, "--depth")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(3);
        let limit = flag_value(args, "--limit")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(20);
        handle_graph_policies(id, depth, limit, args.iter().any(|arg| arg == "--json"));
        return;
    }
    if args.get(3).map(|arg| arg.as_str()) == Some("protocol") {
        let Some(id) = args.get(4) else {
            eprintln!(
                        "{YELLOW}  Usage: yoyo state graph protocol <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]{RESET}"
                    );
            return;
        };
        let depth = flag_value(args, "--depth")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(3);
        let limit = flag_value(args, "--limit")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(20);
        handle_graph_protocol(id, depth, limit, args.iter().any(|arg| arg == "--json"));
        return;
    }
    if args.get(3).map(|arg| arg.as_str()) == Some("timeline") {
        let Some(id) = args.get(4) else {
            eprintln!(
                        "{YELLOW}  Usage: yoyo state graph timeline <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]{RESET}"
                    );
            return;
        };
        let depth = flag_value(args, "--depth")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(3);
        let limit = flag_value(args, "--limit")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(20);
        handle_graph_timeline(id, depth, limit, args.iter().any(|arg| arg == "--json"));
        return;
    }
    if args.get(3).map(|arg| arg.as_str()) == Some("hotspots") {
        let limit = flag_value(args, "--limit")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(10);
        handle_graph_hotspots(limit, args.iter().any(|arg| arg == "--json"));
        return;
    }
    if args.get(3).map(|arg| arg.as_str()) == Some("summary") {
        let Some(id) = args.get(4) else {
            eprintln!(
                        "{YELLOW}  Usage: yoyo state graph summary <event-id|patch-id|eval-id|commit> [--depth N] [--json]{RESET}"
                    );
            return;
        };
        let depth = flag_value(args, "--depth")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(2);
        handle_graph_summary(id, depth, args.iter().any(|arg| arg == "--json"));
        return;
    }
    let Some(id) = args.get(3) else {
        eprintln!(
                    "{YELLOW}  Usage: yoyo state graph <event-id|patch-id|eval-id|commit> [--depth N] [--to TARGET]\n         yoyo state graph summary <event-id|patch-id|eval-id|commit> [--depth N] [--json]\n         yoyo state graph clusters <event-id|patch-id|eval-id|commit> [--depth N] [--json]\n         yoyo state graph impact <event-id|patch-id|eval-id|commit> [--depth N] [--json]\n         yoyo state graph signals <event-id|patch-id|eval-id|commit> [--depth N] [--json]\n         yoyo state graph evidence <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n         yoyo state graph files <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n         yoyo state graph evals <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n         yoyo state graph patches <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n         yoyo state graph decisions <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n         yoyo state graph hypotheses <event-id|hypothesis-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n         yoyo state graph versions <event-id|harness-version|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n         yoyo state graph runs <event-id|run-id|trace-id|task-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n         yoyo state graph artifacts <event-id|artifact-uri|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n         yoyo state graph models <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n         yoyo state graph tools <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n         yoyo state graph commands <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n         yoyo state graph tests <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n         yoyo state graph commits <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n         yoyo state graph memories <event-id|memory-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n         yoyo state graph issues <event-id|issue-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n         yoyo state graph cache <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n         yoyo state graph failures <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n         yoyo state graph policies <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n         yoyo state graph protocol <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n         yoyo state graph timeline <event-id|patch-id|eval-id|commit> [--depth N] [--limit N] [--json]\n         yoyo state graph hotspots [--limit N] [--json]{RESET}"
                );
        return;
    };
    let depth = flag_value(args, "--depth")
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(1);
    let target = flag_value(args, "--to").map(String::as_str);
    handle_graph(id, depth, target);
}

fn handle_graph(id: &str, depth: usize, target: Option<&str>) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    let report = match target {
        Some(target) => build_graph_path_report(&sqlite_path, id, target, depth),
        None => build_graph_report(&sqlite_path, id, depth),
    };
    match report {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_summary(id: &str, depth: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_summary_payload(&sqlite_path, id, depth) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_summary_report(&sqlite_path, id, depth) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_clusters(id: &str, depth: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_clusters_payload(&sqlite_path, id, depth) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_clusters_report(&sqlite_path, id, depth) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_impact(id: &str, depth: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_impact_payload(&sqlite_path, id, depth) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_impact_report(&sqlite_path, id, depth) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_signals(id: &str, depth: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_signals_payload(&sqlite_path, id, depth) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_signals_report(&sqlite_path, id, depth) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_evidence(id: &str, depth: usize, limit: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_evidence_payload(&sqlite_path, id, depth, limit) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_evidence_report(&sqlite_path, id, depth, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_files(id: &str, depth: usize, limit: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_files_payload(&sqlite_path, id, depth, limit) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_files_report(&sqlite_path, id, depth, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_evals(id: &str, depth: usize, limit: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_evals_payload(&sqlite_path, id, depth, limit) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_evals_report(&sqlite_path, id, depth, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_patches(id: &str, depth: usize, limit: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_patches_payload(&sqlite_path, id, depth, limit) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_patches_report(&sqlite_path, id, depth, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_decisions(id: &str, depth: usize, limit: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_decisions_payload(&sqlite_path, id, depth, limit) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_decisions_report(&sqlite_path, id, depth, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_hypotheses(id: &str, depth: usize, limit: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_hypotheses_payload(&sqlite_path, id, depth, limit) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_hypotheses_report(&sqlite_path, id, depth, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_versions(id: &str, depth: usize, limit: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_versions_payload(&sqlite_path, id, depth, limit) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_versions_report(&sqlite_path, id, depth, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_runs(id: &str, depth: usize, limit: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_runs_payload(&sqlite_path, id, depth, limit) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_runs_report(&sqlite_path, id, depth, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_artifacts(id: &str, depth: usize, limit: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_artifacts_payload(&sqlite_path, id, depth, limit) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_artifacts_report(&sqlite_path, id, depth, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_models(id: &str, depth: usize, limit: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_models_payload(&sqlite_path, id, depth, limit) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_models_report(&sqlite_path, id, depth, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_tools(id: &str, depth: usize, limit: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_tools_payload(&sqlite_path, id, depth, limit) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_tools_report(&sqlite_path, id, depth, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_commands(id: &str, depth: usize, limit: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_commands_payload(&sqlite_path, id, depth, limit) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_commands_report(&sqlite_path, id, depth, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_tests(id: &str, depth: usize, limit: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_tests_payload(&sqlite_path, id, depth, limit) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_tests_report(&sqlite_path, id, depth, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_commits(id: &str, depth: usize, limit: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_commits_payload(&sqlite_path, id, depth, limit) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_commits_report(&sqlite_path, id, depth, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_memories(id: &str, depth: usize, limit: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_memories_payload(&sqlite_path, id, depth, limit) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_memories_report(&sqlite_path, id, depth, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_issues(id: &str, depth: usize, limit: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_issues_payload(&sqlite_path, id, depth, limit) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_issues_report(&sqlite_path, id, depth, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_cache(id: &str, depth: usize, limit: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_cache_payload(&sqlite_path, id, depth, limit) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_cache_report(&sqlite_path, id, depth, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_failures(id: &str, depth: usize, limit: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_failures_payload(&sqlite_path, id, depth, limit) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_failures_report(&sqlite_path, id, depth, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_policies(id: &str, depth: usize, limit: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_policies_payload(&sqlite_path, id, depth, limit) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_policies_report(&sqlite_path, id, depth, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_protocol(id: &str, depth: usize, limit: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_protocol_payload(&sqlite_path, id, depth, limit) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_protocol_report(&sqlite_path, id, depth, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_timeline(id: &str, depth: usize, limit: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_timeline_payload(&sqlite_path, id, depth, limit) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_timeline_report(&sqlite_path, id, depth, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_graph_hotspots(limit: usize, json_output: bool) {
    let events_path = default_events_path();
    let sqlite_path = default_store_path(&events_path);
    if !sqlite_path.exists() {
        eprintln!(
            "{YELLOW}  no state projection found at {} (run `yoyo state project --rebuild`){RESET}",
            sqlite_path.display()
        );
        return;
    }
    if json_output {
        match build_graph_hotspots_payload(&sqlite_path, limit) {
            Ok(payload) => println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            ),
            Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
        }
        return;
    }
    match build_graph_hotspots_report(&sqlite_path, limit) {
        Ok(report) => println!("{report}"),
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}
