//! Memory subcommand handlers extracted from commands_state.rs.
//!
//! Handles `yoyo state memory` and all memory sub-subcommands.

use crate::commands_state::{
    current_time_ms, default_events_path, default_store_path, event_string, evidence_ids,
    flag_value, payload_str, preview_line, push_report_section, read_events, write_text_artifact,
};
use crate::format::*;
use crate::state::{Actor, EventType, StateConfig, StateRecorder};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn memory_candidate_id(kind: &str, key: &str) -> String {
    let normalized = key
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    if normalized.is_empty() {
        format!("memory-{kind}")
    } else {
        format!("memory-{kind}-{normalized}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StateMemoryCandidate {
    pub(crate) candidate_id: String,
    pub(crate) source: String,
    pub(crate) summary: String,
    pub(crate) evidence_event_ids: Vec<String>,
}

impl StateMemoryCandidate {
    pub(crate) fn to_payload(&self, proposed_at_ms: i64) -> Value {
        serde_json::json!({
            "candidate_id": self.candidate_id,
            "source": self.source,
            "summary": self.summary,
            "evidence_event_ids": self.evidence_event_ids,
            "status": "proposed",
            "proposed_by": "state_memory_synthesis",
            "proposed_at_ms": proposed_at_ms,
            "review_required": true,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StateMemoryRecord {
    pub(crate) candidate_id: String,
    pub(crate) status: String,
    pub(crate) source: String,
    pub(crate) summary: String,
    pub(crate) proposed_event_id: Option<String>,
    pub(crate) decision_event_id: Option<String>,
    pub(crate) reason: Option<String>,
    pub(crate) evidence_event_ids: Vec<String>,
}

pub(crate) fn build_state_memory_candidates(events: &[Value]) -> Vec<StateMemoryCandidate> {
    let mut failure_sources: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut candidates = Vec::new();

    for event in events {
        let event_type = event_string(event, "event_type").unwrap_or("Unknown");
        let event_id = event_string(event, "event_id").unwrap_or("-");
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        match event_type {
            "FailureObserved" | "JsonOutputFailure" | "ToolSchemaFailure" => {
                let source = payload_str(&payload, "source")
                    .or_else(|| payload_str(&payload, "operation"))
                    .unwrap_or("unknown");
                failure_sources
                    .entry(source.to_string())
                    .or_default()
                    .push(event_id.to_string());
            }
            "HypothesisCreated" => {
                let confidence = payload
                    .get("confidence")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(0.0);
                if confidence >= 0.75 {
                    if let Some(summary) = payload_str(&payload, "summary") {
                        let key = payload_str(&payload, "hypothesis_id").unwrap_or(event_id);
                        let mut evidence_event_ids = vec![event_id.to_string()];
                        evidence_event_ids.extend(evidence_ids(&payload));
                        evidence_event_ids.sort();
                        evidence_event_ids.dedup();
                        candidates.push(StateMemoryCandidate {
                            candidate_id: memory_candidate_id("hypothesis", key),
                            source: "high_confidence_hypothesis".to_string(),
                            summary: preview_line(summary, 180),
                            evidence_event_ids,
                        });
                    }
                }
            }
            "PatchPromoted" => {
                if let Some(patch_id) = payload_str(&payload, "patch_id") {
                    candidates.push(StateMemoryCandidate {
                        candidate_id: memory_candidate_id("promoted_patch", patch_id),
                        source: "promoted_harness_patch".to_string(),
                        summary: preview_line(
                            payload_str(&payload, "intent")
                                .or_else(|| payload_str(&payload, "reason"))
                                .unwrap_or("promoted harness patch"),
                            180,
                        ),
                        evidence_event_ids: vec![event_id.to_string()],
                    });
                }
            }
            "PatchRejected" => {
                if let Some(patch_id) = payload_str(&payload, "patch_id") {
                    candidates.push(StateMemoryCandidate {
                        candidate_id: memory_candidate_id("rejected_patch", patch_id),
                        source: "rejected_harness_patch".to_string(),
                        summary: preview_line(
                            payload_str(&payload, "reason")
                                .or_else(|| payload_str(&payload, "intent"))
                                .unwrap_or("rejected harness patch"),
                            180,
                        ),
                        evidence_event_ids: vec![event_id.to_string()],
                    });
                }
            }
            _ => {}
        }
    }

    for (source, event_ids) in failure_sources {
        if event_ids.len() >= 2 {
            candidates.push(StateMemoryCandidate {
                candidate_id: memory_candidate_id("recurring_failure", &source),
                source: "recurring_failure_source".to_string(),
                summary: format!(
                    "Failure source '{source}' appeared {} times in recent state.",
                    event_ids.len()
                ),
                evidence_event_ids: event_ids,
            });
        }
    }

    candidates.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
    candidates
}

pub(crate) fn build_state_memory_synthesis(events: &[Value]) -> Result<String, String> {
    if events.is_empty() {
        return Err("no state events available for memory synthesis".to_string());
    }

    let memory_candidates = build_state_memory_candidates(events);
    let mut failure_sources: BTreeMap<String, usize> = BTreeMap::new();
    let mut recent_failures = Vec::new();
    let mut hypotheses = Vec::new();
    let mut promoted = Vec::new();
    let mut rejected = Vec::new();
    let mut decisions = Vec::new();

    for event in events {
        let event_type = event_string(event, "event_type").unwrap_or("Unknown");
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        match event_type {
            "FailureObserved" | "JsonOutputFailure" | "ToolSchemaFailure" => {
                let source = payload_str(&payload, "source")
                    .or_else(|| payload_str(&payload, "operation"))
                    .unwrap_or("unknown");
                *failure_sources.entry(source.to_string()).or_default() += 1;
                let preview = payload_str(&payload, "error_preview")
                    .or_else(|| payload_str(&payload, "error"))
                    .or_else(|| payload_str(&payload, "operation"))
                    .unwrap_or("failure recorded");
                recent_failures.push(format!(
                    "{} {}: {}",
                    event_string(event, "event_id").unwrap_or("?"),
                    source,
                    preview_line(preview, 140)
                ));
            }
            "HypothesisCreated" => {
                if let Some(summary) = payload_str(&payload, "summary") {
                    let confidence = payload
                        .get("confidence")
                        .and_then(|value| value.as_f64())
                        .map(|value| format!(" confidence={value:.2}"))
                        .unwrap_or_default();
                    hypotheses.push(format!(
                        "{}: {}{}",
                        payload_str(&payload, "hypothesis_id").unwrap_or("-"),
                        summary,
                        confidence
                    ));
                }
            }
            "PatchPromoted" => {
                if let Some(patch_id) = payload_str(&payload, "patch_id") {
                    promoted.push(format!(
                        "{}: {}",
                        patch_id,
                        payload_str(&payload, "intent")
                            .or_else(|| payload_str(&payload, "reason"))
                            .unwrap_or("promoted harness patch")
                    ));
                }
            }
            "PatchRejected" => {
                if let Some(patch_id) = payload_str(&payload, "patch_id") {
                    rejected.push(format!(
                        "{}: {}",
                        patch_id,
                        payload_str(&payload, "reason")
                            .or_else(|| payload_str(&payload, "intent"))
                            .unwrap_or("rejected harness patch")
                    ));
                }
            }
            "DecisionRecorded" => {
                let decision = payload_str(&payload, "decision")
                    .or_else(|| payload_str(&payload, "decision_type"))
                    .unwrap_or("decision");
                let rationale = payload_str(&payload, "rationale")
                    .or_else(|| payload_str(&payload, "reason"))
                    .unwrap_or("-");
                decisions.push(format!("{decision}: {}", preview_line(rationale, 140)));
            }
            _ => {}
        }
    }

    let mut out = String::new();
    out.push_str("# State Memory Synthesis\n\n");
    out.push_str("This report distills durable, reviewable memory candidates from yoagent-state events. It does not mutate project memory automatically.\n\n");
    out.push_str("## Summary\n\n");
    out.push_str(&format!("- events: {}\n", events.len()));
    out.push_str(&format!("- failures: {}\n", recent_failures.len()));
    out.push_str(&format!("- hypotheses: {}\n", hypotheses.len()));
    out.push_str(&format!("- promoted patches: {}\n", promoted.len()));
    out.push_str(&format!("- rejected patches: {}\n", rejected.len()));

    if !failure_sources.is_empty() {
        out.push_str("\n## Recurring Failure Sources\n\n");
        for (source, count) in failure_sources {
            out.push_str(&format!("- {source}: {count}\n"));
        }
    }

    push_report_section(&mut out, "Recent Failures", &recent_failures, 5);
    push_report_section(&mut out, "Hypotheses", &hypotheses, 8);
    push_report_section(&mut out, "Promoted Harness Lessons", &promoted, 8);
    push_report_section(&mut out, "Rejected Harness Lessons", &rejected, 8);
    push_report_section(&mut out, "Decisions", &decisions, 8);
    if !memory_candidates.is_empty() {
        out.push_str("\n## Structured Memory Proposals\n\n");
        for candidate in &memory_candidates {
            out.push_str(&format!(
                "- {} [{}]: {} (evidence: {})\n",
                candidate.candidate_id,
                candidate.source,
                candidate.summary,
                candidate.evidence_event_ids.join(", ")
            ));
        }
    }

    if recent_failures.is_empty()
        && hypotheses.is_empty()
        && promoted.is_empty()
        && rejected.is_empty()
        && decisions.is_empty()
    {
        out.push_str("\n## Candidate Memories\n\n- No durable memory candidates found yet.\n");
    } else {
        out.push_str("\n## Candidate Memories\n\n");
        if !hypotheses.is_empty() {
            out.push_str(
                "- Review high-confidence hypotheses before promoting them to project memory.\n",
            );
        }
        if !promoted.is_empty() {
            out.push_str("- Treat promoted harness patches as candidate operating rules.\n");
        }
        if !rejected.is_empty() {
            out.push_str("- Keep rejected patch rationales visible to avoid repeating unsafe or ineffective changes.\n");
        }
        if !recent_failures.is_empty() {
            out.push_str(
                "- Use recurring failure sources to bias future context and repair policy.\n",
            );
        }
    }

    Ok(out.trim_end().to_string())
}

pub(crate) fn record_state_memory_candidates(
    events: &[Value],
    events_path: &Path,
) -> Result<Vec<String>, String> {
    let existing_candidate_ids = events
        .iter()
        .filter(|event| event_string(event, "event_type") == Some("MemoryProposed"))
        .filter_map(|event| {
            event
                .get("payload")
                .and_then(|payload| payload_str(payload, "candidate_id"))
                .map(|candidate_id| candidate_id.to_string())
        })
        .collect::<BTreeSet<_>>();
    let candidates = build_state_memory_candidates(events);
    let recorder = StateRecorder::new(StateConfig {
        enabled: true,
        fail_soft: false,
        events_path: events_path.to_path_buf(),
        store_path: Some(default_store_path(events_path)),
    });
    let proposed_at_ms = current_time_ms();
    let mut event_ids = Vec::new();
    for candidate in &candidates {
        if existing_candidate_ids.contains(&candidate.candidate_id) {
            continue;
        }
        let event_id = recorder.append(
            EventType::MemoryProposed,
            Actor::Harness,
            candidate.to_payload(proposed_at_ms),
        )?;
        event_ids.push(event_id);
    }
    Ok(event_ids)
}

pub(crate) fn build_state_memory_records(events: &[Value]) -> Vec<StateMemoryRecord> {
    let mut records = BTreeMap::<String, StateMemoryRecord>::new();
    for event in events {
        let event_type = event_string(event, "event_type").unwrap_or("Unknown");
        if !matches!(
            event_type,
            "MemoryProposed" | "MemoryPromoted" | "MemoryRejected"
        ) {
            continue;
        }
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        let Some(candidate_id) = payload_str(&payload, "candidate_id") else {
            continue;
        };
        let entry = records
            .entry(candidate_id.to_string())
            .or_insert_with(|| StateMemoryRecord {
                candidate_id: candidate_id.to_string(),
                status: "proposed".to_string(),
                source: "-".to_string(),
                summary: "-".to_string(),
                proposed_event_id: None,
                decision_event_id: None,
                reason: None,
                evidence_event_ids: Vec::new(),
            });
        if let Some(source) = payload_str(&payload, "source") {
            entry.source = source.to_string();
        }
        if let Some(summary) = payload_str(&payload, "summary") {
            entry.summary = summary.to_string();
        }
        let payload_evidence_ids = evidence_ids(&payload);
        if !payload_evidence_ids.is_empty() {
            entry.evidence_event_ids = payload_evidence_ids;
        }
        match event_type {
            "MemoryProposed" => {
                entry.status = payload_str(&payload, "status")
                    .unwrap_or("proposed")
                    .to_string();
                entry.proposed_event_id = event_string(event, "event_id").map(|id| id.to_string());
            }
            "MemoryPromoted" => {
                entry.status = "promoted".to_string();
                entry.decision_event_id = event_string(event, "event_id").map(|id| id.to_string());
                entry.reason = payload_str(&payload, "reason").map(|reason| reason.to_string());
            }
            "MemoryRejected" => {
                entry.status = "rejected".to_string();
                entry.decision_event_id = event_string(event, "event_id").map(|id| id.to_string());
                entry.reason = payload_str(&payload, "reason").map(|reason| reason.to_string());
            }
            _ => {}
        }
    }
    records.into_values().collect()
}

pub(crate) fn record_state_memory_decision(
    events: &[Value],
    events_path: &Path,
    candidate_id: &str,
    promote: bool,
    reason: &str,
) -> Result<String, String> {
    let records = build_state_memory_records(events);
    let record = records
        .iter()
        .find(|record| record.candidate_id == candidate_id)
        .ok_or_else(|| format!("memory candidate '{candidate_id}' was not found"))?;
    if record.status != "proposed" {
        return Err(format!(
            "memory candidate '{candidate_id}' is already {}",
            record.status
        ));
    }
    let event_type = if promote {
        EventType::MemoryPromoted
    } else {
        EventType::MemoryRejected
    };
    let status = if promote { "promoted" } else { "rejected" };
    let recorder = StateRecorder::new(StateConfig {
        enabled: true,
        fail_soft: false,
        events_path: events_path.to_path_buf(),
        store_path: Some(default_store_path(events_path)),
    });
    recorder.append(
        event_type,
        Actor::User,
        serde_json::json!({
            "candidate_id": record.candidate_id,
            "status": status,
            "source": record.source,
            "summary": record.summary,
            "reason": reason,
            "proposed_event_id": record.proposed_event_id,
            "evidence_event_ids": record.evidence_event_ids,
            "decided_at_ms": current_time_ms(),
        }),
    )
}

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
