//! Controlled harness patch lifecycle commands.

use crate::format::*;
use crate::state::{
    Actor, EvalResult, EvalStatus, EventType, HarnessPatch, HarnessPatchKind, HarnessPatchRisk,
    HarnessPatchStatus, StateConfig, StateRecorder,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

fn default_events_path() -> PathBuf {
    let (config, _) = crate::config::load_deepseek_config_file();
    config
        .get("state_events")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".yoyo/state/events.jsonl"))
}

fn default_store_path() -> Option<PathBuf> {
    let (config, _) = crate::config::load_deepseek_config_file();
    config.get("state_store").map(PathBuf::from)
}

pub fn handle_evolve_subcommand(args: &[String]) {
    if args.get(2).map(|s| s.as_str()) != Some("harness") {
        print_usage();
        return;
    }

    match args.get(3).map(|s| s.as_str()).unwrap_or("help") {
        "propose" => handle_propose(args),
        "feedback" => handle_feedback(args),
        "issue" => handle_issue(args),
        "apply" => handle_apply(args),
        "rollback" => handle_rollback(args),
        "eval" => handle_eval(args),
        "promote" => handle_promote(args),
        "approve" => handle_approve(args),
        "reject" => handle_reject(args),
        _ => print_usage(),
    }
}

fn handle_feedback(args: &[String]) {
    let Some(summary) = flag_value(args, "--summary") else {
        eprintln!(
            "{YELLOW}  Usage: yoyo evolve harness feedback --summary <text> [--kind KIND] [--risk RISK] [--evidence EVENT[,EVENT]] [--expected-effect TEXT] [--eval-plan TEXT] [--rollback-plan TEXT] [--dry-run]{RESET}"
        );
        return;
    };
    handle_intake(
        args,
        IntakeSource::HumanFeedback,
        summary,
        flag_value(args, "--details"),
    );
}

fn handle_issue(args: &[String]) {
    let Some(title) = flag_value(args, "--title") else {
        eprintln!(
            "{YELLOW}  Usage: yoyo evolve harness issue --title <text> [--body TEXT] [--kind KIND] [--risk RISK] [--evidence EVENT[,EVENT]] [--expected-effect TEXT] [--eval-plan TEXT] [--rollback-plan TEXT] [--dry-run]{RESET}"
        );
        return;
    };
    handle_intake(
        args,
        IntakeSource::SelfFiledIssue,
        title,
        flag_value(args, "--body"),
    );
}

fn handle_intake(args: &[String], source: IntakeSource, summary: &str, details: Option<&String>) {
    let kind = match flag_value(args, "--kind").map(|value| parse_kind(value)) {
        Some(Ok(kind)) => kind,
        Some(Err(e)) => {
            eprintln!("{YELLOW}  {e}{RESET}");
            return;
        }
        None => HarnessPatchKind::Other,
    };
    let risk_level = match flag_value(args, "--risk").map(|value| parse_risk(value)) {
        Some(Ok(risk)) => risk,
        Some(Err(e)) => {
            eprintln!("{YELLOW}  {e}{RESET}");
            return;
        }
        None => HarnessPatchRisk::Medium,
    };
    let payload = build_improvement_intake_payload(ImprovementIntakeOptions {
        source,
        summary: summary.to_string(),
        details: details.map(|value| value.to_string()),
        patch_id: new_patch_id(),
        kind,
        risk_level,
        evidence_event_ids: collect_split_values(args, "--evidence"),
        expected_effects: collect_values(args, "--expected-effect"),
        eval_plan: collect_values(args, "--eval-plan"),
        rollback_plan: collect_values(args, "--rollback-plan"),
        created_at_ms: now_ms(),
    });

    if args.iter().any(|arg| arg == "--dry-run") {
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
        );
        return;
    }

    match append_patch_event(EventType::PatchProposed, payload.clone()) {
        Ok(event_id) => {
            println!("Harness improvement intake recorded");
            println!(
                "  source:   {}",
                payload
                    .get("intake_source")
                    .and_then(Value::as_str)
                    .unwrap_or("-")
            );
            println!(
                "  patch id: {}",
                payload
                    .get("patch_id")
                    .and_then(Value::as_str)
                    .unwrap_or("-")
            );
            println!("  event id: {event_id}");
            if let Ok(patch) = serde_json::from_value::<HarnessPatch>(payload) {
                record_patch_risk_score(&patch, &event_id);
            }
        }
        Err(e) => eprintln!("{RED}  failed to record improvement intake: {e}{RESET}"),
    }
}

fn handle_propose(args: &[String]) {
    let from_state = flag_value(args, "--from-state").map(String::as_str);
    let intent = flag_value(args, "--intent").map(String::as_str);
    if intent.is_none() && from_state.is_none() {
        eprintln!(
            "{YELLOW}  Usage: yoyo evolve harness propose (--intent <text> | --from-state <event-id|last-failure>) [--kind KIND] [--risk RISK] [--evidence EVENT[,EVENT]] [--expected-effect TEXT] [--eval-plan TEXT] [--rollback-plan TEXT] [--dry-run]{RESET}"
        );
        return;
    };

    let state_defaults = match from_state {
        Some(reference) => {
            let path = default_events_path();
            let events = match read_events(&path) {
                Ok(events) => events,
                Err(_) => {
                    eprintln!("{YELLOW}  no state log found at {}{RESET}", path.display());
                    return;
                }
            };
            match build_state_backed_proposal_defaults(&events, reference) {
                Ok(defaults) => Some(defaults),
                Err(e) => {
                    eprintln!("{YELLOW}  {e}{RESET}");
                    return;
                }
            }
        }
        None => None,
    };

    let kind = match flag_value(args, "--kind").map(|value| parse_kind(value)) {
        Some(Ok(kind)) => kind,
        Some(Err(e)) => {
            eprintln!("{YELLOW}  {e}{RESET}");
            return;
        }
        None => state_defaults
            .as_ref()
            .map(|defaults| defaults.kind.clone())
            .unwrap_or(HarnessPatchKind::Other),
    };
    let risk_level = match flag_value(args, "--risk").map(|value| parse_risk(value)) {
        Some(Ok(risk)) => risk,
        Some(Err(e)) => {
            eprintln!("{YELLOW}  {e}{RESET}");
            return;
        }
        None => state_defaults
            .as_ref()
            .map(|defaults| defaults.risk_level.clone())
            .unwrap_or(HarnessPatchRisk::Medium),
    };

    let genome = crate::deepseek::DeepSeekHarnessGenome::default();
    let evidence_event_ids = merge_cli_or_default(
        collect_split_values(args, "--evidence"),
        state_defaults
            .as_ref()
            .map(|defaults| defaults.evidence_event_ids.clone())
            .unwrap_or_default(),
    );
    let patch = build_harness_patch(PatchProposalOptions {
        patch_id: new_patch_id(),
        kind,
        risk_level,
        base_harness_version: genome.version.clone(),
        base_git_commit: current_git_commit(),
        intent: intent
            .map(str::to_string)
            .or_else(|| {
                state_defaults
                    .as_ref()
                    .map(|defaults| defaults.intent.clone())
            })
            .unwrap_or_else(|| "Improve DeepSeek harness behavior".to_string()),
        evidence_event_ids,
        expected_effects: merge_cli_or_default(
            collect_values(args, "--expected-effect"),
            state_defaults
                .as_ref()
                .map(|defaults| defaults.expected_effects.clone())
                .unwrap_or_default(),
        ),
        eval_plan: merge_cli_or_default(
            collect_values(args, "--eval-plan"),
            state_defaults
                .as_ref()
                .map(|defaults| defaults.eval_plan.clone())
                .unwrap_or_default(),
        ),
        rollback_plan: merge_cli_or_default(
            collect_values(args, "--rollback-plan"),
            state_defaults
                .as_ref()
                .map(|defaults| defaults.rollback_plan.clone())
                .unwrap_or_default(),
        ),
        default_eval_plan: default_harness_patch_eval_plan(&genome),
        created_at_ms: now_ms(),
    });

    if args.iter().any(|arg| arg == "--dry-run") {
        println!(
            "{}",
            serde_json::to_string_pretty(&patch).unwrap_or_else(|_| "{}".to_string())
        );
        return;
    }

    match append_patch_event(
        EventType::PatchProposed,
        serde_json::to_value(&patch).unwrap(),
    ) {
        Ok(event_id) => {
            println!("Harness patch proposed");
            println!("  patch id: {id}", id = patch.patch_id);
            println!("  event id: {event_id}");
            println!("  status:   proposed");
            record_patch_risk_score(&patch, &event_id);
        }
        Err(e) => eprintln!("{RED}  failed to record patch proposal: {e}{RESET}"),
    }
}

fn handle_eval(args: &[String]) {
    let Some(patch_id) = args.get(4) else {
        eprintln!(
            "{YELLOW}  Usage: yoyo evolve harness eval <patch-id> [--worktree PATH] [--dry-run] [--force]{RESET}"
        );
        return;
    };
    let dry_run = args.iter().any(|arg| arg == "--dry-run");
    let force = args.iter().any(|arg| arg == "--force");
    let mut worktree = flag_value(args, "--worktree").map(PathBuf::from);

    if !force {
        let path = default_events_path();
        let events = match read_events(&path) {
            Ok(events) => events,
            Err(_) => {
                eprintln!("{YELLOW}  no state log found at {}{RESET}", path.display());
                return;
            }
        };
        match eval_worktree_for_patch(&events, patch_id, worktree) {
            Ok(selected_worktree) => worktree = selected_worktree,
            Err(reason) => {
                eprintln!("{YELLOW}  {reason} (use --force to evaluate anyway){RESET}");
                return;
            }
        }
        let active_repo = current_git_root()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        if let Err(reason) = eval_worktree_isolation_gate(worktree.as_deref(), &active_repo) {
            eprintln!("{YELLOW}  {reason} (use --force only after human review){RESET}");
            return;
        }
    }

    let _ = crate::commands_eval::run_eval_for_patch_in(patch_id, dry_run, worktree);
}

fn handle_apply(args: &[String]) {
    let Some(patch_id) = args.get(4) else {
        eprintln!(
            "{YELLOW}  Usage: yoyo evolve harness apply <patch-id> --patch-file PATH [--worktree PATH] [--check] [--dry-run] [--force]{RESET}"
        );
        return;
    };
    let Some(patch_file) = flag_value(args, "--patch-file") else {
        eprintln!("{YELLOW}  missing --patch-file PATH{RESET}");
        return;
    };
    let dry_run = args.iter().any(|arg| arg == "--dry-run");
    let check_only = args.iter().any(|arg| arg == "--check");
    let force = args.iter().any(|arg| arg == "--force");
    let mut patch_metadata = None;

    if !force {
        let path = default_events_path();
        let events = match read_events(&path) {
            Ok(events) => events,
            Err(_) => {
                eprintln!("{YELLOW}  no state log found at {}{RESET}", path.display());
                return;
            }
        };
        let Some(patch) = find_patch_safety_metadata(&events, patch_id) else {
            eprintln!(
                "{YELLOW}  no proposed harness patch found for '{patch_id}' (use --force to apply anyway){RESET}"
            );
            return;
        };
        patch_metadata = Some(patch.clone());
        if let Err(reason) = apply_mutation_surface_gate(&patch) {
            eprintln!("{YELLOW}  patch '{patch_id}' cannot be applied: {reason} (use --force only after human review){RESET}");
            return;
        }
        let gate = apply_approval_gate(&events, &patch);
        if !gate.allowed {
            if gate.requires_human_approval {
                let requested_at_ms = now_ms();
                let request_payload = build_harness_approval_request_payload(
                    &patch,
                    &gate,
                    "harness_patch_fork_apply",
                    requested_at_ms,
                );
                match append_patch_event(EventType::HumanApprovalRequested, request_payload) {
                    Ok(event_id) => {
                        eprintln!("{YELLOW}  approval request event id: {event_id}{RESET}");
                    }
                    Err(e) => eprintln!("{RED}  failed to record approval request: {e}{RESET}"),
                }
                eprintln!(
                    "{YELLOW}  patch '{patch_id}' cannot be applied without human approval: {}{RESET}",
                    gate.reason
                );
                eprintln!(
                    "{YELLOW}  run: yoyo evolve harness approve {patch_id} --reason <text>{RESET}"
                );
            } else {
                eprintln!(
                    "{YELLOW}  patch '{patch_id}' cannot be applied: {}{RESET}",
                    gate.reason
                );
            }
            return;
        }
    }

    let worktree = flag_value(args, "--worktree")
        .map(PathBuf::from)
        .unwrap_or_else(|| default_worktree_path(patch_id));

    if dry_run {
        println!("Harness patch apply dry run");
        println!("  patch id:   {patch_id}");
        println!("  patch file: {patch_file}");
        println!("  worktree:   {}", worktree.display());
        println!("  check only: {check_only}");
        if let Some(patch) = patch_metadata.as_ref() {
            println!("  kind:       {}", enum_label(&patch.kind));
            println!("  risk:       {}", enum_label(&patch.risk_level));
            if !patch.rollback_plan.is_empty() {
                println!("  rollback:   {}", patch.rollback_plan.join(" | "));
            }
        }
        return;
    }

    let patch_file = match std::fs::canonicalize(patch_file) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("{RED}  failed to resolve patch file '{patch_file}': {e}{RESET}");
            return;
        }
    };

    match apply_patch_in_worktree(&patch_file, &worktree, check_only) {
        Ok(report) => {
            if check_only {
                println!("Harness patch apply check passed");
            } else {
                println!("Harness patch applied in worktree");
            }
            println!("  patch id: {patch_id}");
            println!("  worktree: {}", report.worktree_path.display());
            if check_only {
                println!("  mode:     check");
            }
            if !check_only {
                let payload = build_apply_payload(
                    patch_id,
                    &report,
                    check_only,
                    patch_metadata.as_ref(),
                    now_ms(),
                );
                match append_patch_event(EventType::PatchApplied, payload) {
                    Ok(event_id) => println!("  event id: {event_id}"),
                    Err(e) => eprintln!("{RED}  failed to record patch apply: {e}{RESET}"),
                }
            }
        }
        Err(e) => {
            eprintln!("{RED}  failed to apply harness patch: {e}{RESET}");
            let _ = append_patch_event(
                EventType::FailureObserved,
                json!({
                    "source": "harness_patch_apply",
                    "patch_id": patch_id,
                    "patch_file": patch_file.display().to_string(),
                    "worktree_path": worktree.display().to_string(),
                    "error_preview": e,
                }),
            );
        }
    }
}

fn handle_rollback(args: &[String]) {
    let Some(patch_id) = args.get(4) else {
        eprintln!(
            "{YELLOW}  Usage: yoyo evolve harness rollback <patch-id> [--worktree PATH] [--patch-file PATH] [--reason TEXT] [--dry-run] [--force]{RESET}"
        );
        return;
    };
    let dry_run = args.iter().any(|arg| arg == "--dry-run");
    let force = args.iter().any(|arg| arg == "--force");
    let reason = flag_value(args, "--reason")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "rollback requested".to_string());
    let path = default_events_path();
    let events = match read_events(&path) {
        Ok(events) => events,
        Err(_) if force => Vec::new(),
        Err(_) => {
            eprintln!("{YELLOW}  no state log found at {}{RESET}", path.display());
            return;
        }
    };
    if !force && !patch_exists(&events, patch_id) {
        eprintln!("{YELLOW}  no proposed harness patch found for '{patch_id}'{RESET}");
        return;
    }

    let applied = latest_applied_patch(&events, patch_id);
    let patch_file = flag_value(args, "--patch-file")
        .map(PathBuf::from)
        .or_else(|| applied.as_ref().map(|applied| applied.patch_file.clone()));
    let Some(patch_file) = patch_file else {
        eprintln!(
            "{YELLOW}  no patch file found for '{patch_id}' (pass --patch-file PATH or rollback an applied patch){RESET}"
        );
        return;
    };
    let worktree = flag_value(args, "--worktree")
        .map(PathBuf::from)
        .or_else(|| {
            applied
                .as_ref()
                .map(|applied| applied.worktree_path.clone())
        })
        .unwrap_or_else(|| default_worktree_path(patch_id));

    if dry_run {
        println!("Harness patch rollback dry run");
        println!("  patch id:   {patch_id}");
        println!("  patch file: {}", patch_file.display());
        println!("  worktree:   {}", worktree.display());
        println!("  reason:     {reason}");
        return;
    }

    match rollback_patch_in_worktree(&patch_file, &worktree) {
        Ok(report) => {
            println!("Harness patch rolled back in worktree");
            println!("  patch id: {patch_id}");
            println!("  worktree: {}", report.worktree_path.display());
            let reverted_at_ms = now_ms();
            let payload = build_rollback_payload(patch_id, &report, &reason, reverted_at_ms);
            match append_patch_event(EventType::RevertPerformed, payload) {
                Ok(event_id) => {
                    println!("  event id: {event_id}");
                    let decision_payload =
                        build_lifecycle_decision_payload(LifecycleDecisionOptions {
                            patch_id,
                            decision_type: "harness_patch_rollback",
                            decision: "rollback",
                            rationale: &reason,
                            status: "recorded",
                            patch_event_id: Some(&event_id),
                            eval_id: None,
                            forced: force,
                            decided_at_ms: reverted_at_ms,
                            promotion_decision: None,
                            safety_gate: None,
                        });
                    match append_patch_event(EventType::DecisionRecorded, decision_payload) {
                        Ok(decision_event_id) => {
                            println!("  decision event id: {decision_event_id}")
                        }
                        Err(e) => {
                            eprintln!("{RED}  failed to record rollback decision: {e}{RESET}")
                        }
                    }
                }
                Err(e) => eprintln!("{RED}  failed to record patch rollback: {e}{RESET}"),
            }
        }
        Err(e) => {
            eprintln!("{RED}  failed to rollback harness patch: {e}{RESET}");
            let _ = append_patch_event(
                EventType::FailureObserved,
                json!({
                    "source": "harness_patch_rollback",
                    "patch_id": patch_id,
                    "patch_file": patch_file.display().to_string(),
                    "worktree_path": worktree.display().to_string(),
                    "error_preview": e,
                }),
            );
        }
    }
}

fn handle_promote(args: &[String]) {
    let Some(patch_id) = args.get(4) else {
        eprintln!("{YELLOW}  Usage: yoyo evolve harness promote <patch-id> [--baseline-eval EVAL] [--candidate-eval EVAL] [--approval-event EVENT[,EVENT]] [--reason <text>] [--force]{RESET}");
        return;
    };
    let force = args.iter().any(|arg| arg == "--force");
    let baseline_eval_id = flag_value(args, "--baseline-eval").map(|s| s.as_str());
    let candidate_eval_id = flag_value(args, "--candidate-eval").map(|s| s.as_str());
    let explicit_approval_event_ids = collect_split_values(args, "--approval-event");
    let reason = flag_value(args, "--reason")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "promotion criteria satisfied".to_string());
    let path = default_events_path();
    let Ok(events) = read_events(&path) else {
        eprintln!("{YELLOW}  no state log found at {}{RESET}", path.display());
        return;
    };
    let Some(patch) = find_patch_safety_metadata(&events, patch_id) else {
        eprintln!("{YELLOW}  no proposed harness patch found for '{patch_id}'{RESET}");
        return;
    };

    let decision = promotion_decision(&events, patch_id, baseline_eval_id, candidate_eval_id);
    let compared_at_ms = now_ms();
    let comparison_payload =
        build_comparison_decision_payload(patch_id, &decision, force, compared_at_ms);
    match append_patch_event(EventType::DecisionRecorded, comparison_payload) {
        Ok(event_id) => println!("  comparison event id: {event_id}"),
        Err(e) => eprintln!("{RED}  failed to record patch comparison: {e}{RESET}"),
    }
    if !decision.eligible && !force {
        eprintln!(
            "{YELLOW}  patch '{patch_id}' does not satisfy promotion criteria: {} (use --force to promote anyway){RESET}",
            decision.reason
        );
        return;
    }
    let min_approval_ms = decision.candidate_eval_id.as_deref().and_then(|eval_id| {
        patch_evals(&events)
            .into_iter()
            .find(|eval| eval.eval_id == eval_id)
            .map(|eval| eval.created_at_ms)
    });
    let safety_gate = promotion_safety_gate_after_candidate_eval(
        &events,
        &patch,
        &explicit_approval_event_ids,
        min_approval_ms,
    );
    if !safety_gate.allowed {
        if safety_gate.requires_human_approval {
            let requested_at_ms = now_ms();
            let request_payload = build_harness_approval_request_payload(
                &patch,
                &safety_gate,
                "harness_patch_promotion",
                requested_at_ms,
            );
            match append_patch_event(EventType::HumanApprovalRequested, request_payload) {
                Ok(event_id) => {
                    eprintln!("{YELLOW}  approval request event id: {event_id}{RESET}");
                }
                Err(e) => eprintln!("{RED}  failed to record approval request: {e}{RESET}"),
            }
            eprintln!(
                "{YELLOW}  patch '{patch_id}' cannot be promoted without human approval: {}{RESET}",
                safety_gate.reason
            );
            eprintln!(
                "{YELLOW}  run: yoyo evolve harness approve {patch_id} --reason <text>{RESET}"
            );
        } else {
            eprintln!(
                "{YELLOW}  patch '{patch_id}' cannot be promoted: {}{RESET}",
                safety_gate.reason
            );
        }
        return;
    }

    println!("Promotion evidence");
    for line in promotion_evidence_lines(&decision, force) {
        println!("  {line}");
    }

    let promoted_at_ms = now_ms();
    let decision_eval_id = decision.candidate_eval_id.clone();
    let payload = json!({
        "patch_id": patch_id,
        "status": "promoted",
        "reason": reason,
        "eval_id": decision_eval_id.clone(),
        "baseline_eval_id": decision.baseline_eval_id,
        "candidate_eval_id": decision_eval_id.clone(),
        "promotion_decision": decision,
        "approval_event_ids": safety_gate.approval_event_ids,
        "safety_gate": safety_gate,
        "forced": force,
        "promoted_at_ms": promoted_at_ms,
    });
    match append_patch_event(EventType::PatchPromoted, payload) {
        Ok(event_id) => {
            println!("Harness patch promoted");
            println!("  patch id: {patch_id}");
            println!("  event id: {event_id}");
            let decision_payload = build_lifecycle_decision_payload(LifecycleDecisionOptions {
                patch_id,
                decision_type: "harness_patch_promotion",
                decision: "promote",
                rationale: &reason,
                status: "recorded",
                patch_event_id: Some(&event_id),
                eval_id: decision_eval_id.as_deref(),
                forced: force,
                decided_at_ms: promoted_at_ms,
                promotion_decision: Some(&decision),
                safety_gate: Some(&safety_gate),
            });
            match append_patch_event(EventType::DecisionRecorded, decision_payload) {
                Ok(decision_event_id) => println!("  decision event id: {decision_event_id}"),
                Err(e) => eprintln!("{RED}  failed to record promotion decision: {e}{RESET}"),
            }
        }
        Err(e) => eprintln!("{RED}  failed to record patch promotion: {e}{RESET}"),
    }
}

fn build_harness_approval_request_payload(
    patch: &PatchSafetyMetadata,
    safety_gate: &PromotionSafetyGate,
    approval_scope: &str,
    requested_at_ms: u128,
) -> Value {
    json!({
        "patch_id": patch.patch_id,
        "kind": patch.kind,
        "risk_level": patch.risk_level,
        "reason": safety_gate.reason,
        "approval_scope": approval_scope,
        "requested_by": "yoyo-ds-harness",
        "required": safety_gate.requires_human_approval,
        "requested_at_ms": requested_at_ms,
    })
}

fn handle_approve(args: &[String]) {
    let Some(patch_id) = args.get(4) else {
        eprintln!(
            "{YELLOW}  Usage: yoyo evolve harness approve <patch-id> [--reason <text>]{RESET}"
        );
        return;
    };
    let path = default_events_path();
    let Ok(events) = read_events(&path) else {
        eprintln!("{YELLOW}  no state log found at {}{RESET}", path.display());
        return;
    };
    let Some(patch) = find_patch_safety_metadata(&events, patch_id) else {
        eprintln!("{YELLOW}  no proposed harness patch found for '{patch_id}'{RESET}");
        return;
    };
    let reason = flag_value(args, "--reason")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "human approved high-risk harness patch".to_string());
    let payload = json!({
        "patch_id": patch_id,
        "kind": patch.kind,
        "risk_level": patch.risk_level,
        "reason": reason,
        "approval_scope": "harness_patch_promotion",
        "approved_at_ms": now_ms(),
    });

    let recorder = StateRecorder::new(StateConfig {
        enabled: true,
        fail_soft: false,
        events_path: default_events_path(),
        store_path: default_store_path(),
    });
    match recorder.append(EventType::HumanApprovalReceived, Actor::User, payload) {
        Ok(event_id) => {
            println!("Harness patch approval recorded");
            println!("  patch id: {patch_id}");
            println!("  event id: {event_id}");
        }
        Err(e) => eprintln!("{RED}  failed to record patch approval: {e}{RESET}"),
    }
}

fn handle_reject(args: &[String]) {
    let Some(patch_id) = args.get(4) else {
        eprintln!(
            "{YELLOW}  Usage: yoyo evolve harness reject <patch-id> [--reason <text>]{RESET}"
        );
        return;
    };
    let path = default_events_path();
    let Ok(events) = read_events(&path) else {
        eprintln!("{YELLOW}  no state log found at {}{RESET}", path.display());
        return;
    };
    if !patch_exists(&events, patch_id) {
        eprintln!("{YELLOW}  no proposed harness patch found for '{patch_id}'{RESET}");
        return;
    }

    let reason = flag_value(args, "--reason")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "rejected by harness lifecycle".to_string());
    let rejected_at_ms = now_ms();
    let payload = json!({
        "patch_id": patch_id,
        "status": "rejected",
        "reason": reason,
        "rejected_at_ms": rejected_at_ms,
    });
    match append_patch_event(EventType::PatchRejected, payload) {
        Ok(event_id) => {
            println!("Harness patch rejected");
            println!("  patch id: {patch_id}");
            println!("  event id: {event_id}");
            let decision_payload = build_lifecycle_decision_payload(LifecycleDecisionOptions {
                patch_id,
                decision_type: "harness_patch_rejection",
                decision: "reject",
                rationale: &reason,
                status: "recorded",
                patch_event_id: Some(&event_id),
                eval_id: None,
                forced: false,
                decided_at_ms: rejected_at_ms,
                promotion_decision: None,
                safety_gate: None,
            });
            match append_patch_event(EventType::DecisionRecorded, decision_payload) {
                Ok(decision_event_id) => println!("  decision event id: {decision_event_id}"),
                Err(e) => eprintln!("{RED}  failed to record rejection decision: {e}{RESET}"),
            }
        }
        Err(e) => eprintln!("{RED}  failed to record patch rejection: {e}{RESET}"),
    }
}

#[derive(Debug, Clone)]
struct PatchProposalOptions {
    patch_id: String,
    kind: HarnessPatchKind,
    risk_level: HarnessPatchRisk,
    base_harness_version: String,
    base_git_commit: Option<String>,
    intent: String,
    evidence_event_ids: Vec<String>,
    expected_effects: Vec<String>,
    eval_plan: Vec<String>,
    rollback_plan: Vec<String>,
    default_eval_plan: Vec<String>,
    created_at_ms: u128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IntakeSource {
    HumanFeedback,
    SelfFiledIssue,
}

impl IntakeSource {
    fn label(self) -> &'static str {
        match self {
            IntakeSource::HumanFeedback => "human_feedback",
            IntakeSource::SelfFiledIssue => "self_filed_improvement_issue",
        }
    }
}

#[derive(Debug, Clone)]
struct ImprovementIntakeOptions {
    source: IntakeSource,
    summary: String,
    details: Option<String>,
    patch_id: String,
    kind: HarnessPatchKind,
    risk_level: HarnessPatchRisk,
    evidence_event_ids: Vec<String>,
    expected_effects: Vec<String>,
    eval_plan: Vec<String>,
    rollback_plan: Vec<String>,
    created_at_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StateBackedProposalDefaults {
    intent: String,
    kind: HarnessPatchKind,
    risk_level: HarnessPatchRisk,
    evidence_event_ids: Vec<String>,
    expected_effects: Vec<String>,
    eval_plan: Vec<String>,
    rollback_plan: Vec<String>,
}

fn build_state_backed_proposal_defaults(
    events: &[Value],
    reference: &str,
) -> Result<StateBackedProposalDefaults, String> {
    let failure = find_failure_event(events, reference)
        .ok_or_else(|| format!("no state failure found for '{reference}'"))?;
    let event_id = failure
        .get("event_id")
        .and_then(Value::as_str)
        .unwrap_or(reference);
    let event_type = failure
        .get("event_type")
        .and_then(Value::as_str)
        .unwrap_or("FailureObserved");
    let payload = failure.get("payload").unwrap_or(&Value::Null);
    let signal = failure_signal(payload);
    let source = payload_text(payload, &["source", "operation", "tool_name"]).unwrap_or("unknown");
    let kind = infer_patch_kind_from_failure(event_type, source, &signal);
    let risk_level = infer_patch_risk_from_failure(source, &signal, &kind);
    let kind_label = enum_label(&kind);

    Ok(StateBackedProposalDefaults {
        intent: format!(
            "Improve {kind_label} after state failure {event_id}: {}",
            preview(&signal, 180)
        ),
        kind,
        risk_level,
        evidence_event_ids: vec![event_id.to_string()],
        expected_effects: vec![
            format!("reduce recurrence of state failure {event_id}"),
            format!("improve {kind_label} reliability for {source} failures"),
        ],
        eval_plan: vec![
            format!("yoyo state why {event_id}"),
            "yoyo eval replay --from-state --limit 1".to_string(),
            "yoyo eval run --suite protocol-deepseek".to_string(),
            "cargo check".to_string(),
        ],
        rollback_plan: vec![
            format!("reject patch if replay for state failure {event_id} does not improve"),
            "revert any applied harness patch before promotion".to_string(),
        ],
    })
}

fn find_failure_event<'a>(events: &'a [Value], reference: &str) -> Option<&'a Value> {
    if reference == "last-failure" {
        return events.iter().rev().find(|event| is_failure_event(event));
    }
    events.iter().find(|event| {
        event
            .get("event_id")
            .and_then(Value::as_str)
            .map(|event_id| event_id == reference)
            .unwrap_or(false)
            && is_failure_event(event)
    })
}

fn is_failure_event(event: &Value) -> bool {
    match event.get("event_type").and_then(Value::as_str) {
        Some("FailureObserved" | "JsonOutputFailure" | "ToolSchemaFailure") => true,
        Some("CommandCompleted" | "TestCompleted") => event
            .get("payload")
            .and_then(|payload| payload.get("is_error").or_else(|| payload.get("failed")))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        _ => false,
    }
}

fn failure_signal(payload: &Value) -> String {
    payload_text(
        payload,
        &[
            "error_preview",
            "error",
            "failure_summary",
            "summary",
            "operation",
            "result_preview",
            "stderr",
        ],
    )
    .unwrap_or("failure recorded")
    .to_string()
}

fn payload_text<'a>(payload: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| payload.get(*key).and_then(Value::as_str))
        .filter(|value| !value.trim().is_empty())
}

fn infer_patch_kind_from_failure(event_type: &str, source: &str, signal: &str) -> HarnessPatchKind {
    let text = normalize(&format!("{event_type} {source} {signal}"));
    if event_type == "ToolSchemaFailure" || text.contains("schema") || text.contains("tool_call") {
        HarnessPatchKind::ToolSchema
    } else if event_type == "JsonOutputFailure" || text.contains("json") {
        HarnessPatchKind::PromptPolicy
    } else if text.contains("context") || text.contains("prompt_layout") {
        HarnessPatchKind::ContextPolicy
    } else if text.contains("permission") || text.contains("approval") {
        HarnessPatchKind::PermissionPolicy
    } else if text.contains("fim") || text.contains("edit") || text.contains("repair") {
        HarnessPatchKind::RepairPolicy
    } else if text.contains("eval") || text.contains("fixture") || text.contains("benchmark") {
        HarnessPatchKind::Eval
    } else if text.contains("state") || text.contains("sqlite") || text.contains("projection") {
        HarnessPatchKind::StateProjection
    } else if text.contains("transport")
        || text.contains("timeout")
        || text.contains("retry")
        || text.contains("http")
        || text.contains("api")
    {
        HarnessPatchKind::Transport
    } else {
        HarnessPatchKind::RepairPolicy
    }
}

fn infer_patch_risk_from_failure(
    source: &str,
    signal: &str,
    kind: &HarnessPatchKind,
) -> HarnessPatchRisk {
    let text = normalize(&format!("{source} {signal}"));
    if matches!(
        kind,
        HarnessPatchKind::PermissionPolicy
            | HarnessPatchKind::ShellPolicy
            | HarnessPatchKind::Safety
    ) || text.contains("credential")
        || text.contains("secret")
    {
        HarnessPatchRisk::High
    } else if matches!(
        kind,
        HarnessPatchKind::Transport | HarnessPatchKind::StateProjection
    ) {
        HarnessPatchRisk::Medium
    } else {
        HarnessPatchRisk::Low
    }
}

fn merge_cli_or_default(cli_values: Vec<String>, defaults: Vec<String>) -> Vec<String> {
    if cli_values.is_empty() {
        defaults
    } else {
        cli_values
    }
}

fn default_harness_patch_eval_plan(genome: &crate::deepseek::DeepSeekHarnessGenome) -> Vec<String> {
    let mut plan = genome.test_policy.required_gates.clone();
    plan.push("yoyo eval run --suite protocol-deepseek".to_string());
    plan
}

fn build_improvement_intake_payload(options: ImprovementIntakeOptions) -> Value {
    let genome = crate::deepseek::DeepSeekHarnessGenome::default();
    let patch = build_harness_patch(PatchProposalOptions {
        patch_id: options.patch_id,
        kind: options.kind,
        risk_level: options.risk_level,
        base_harness_version: genome.version.clone(),
        base_git_commit: current_git_commit(),
        intent: options.summary.clone(),
        evidence_event_ids: options.evidence_event_ids,
        expected_effects: options.expected_effects,
        eval_plan: options.eval_plan,
        rollback_plan: options.rollback_plan,
        default_eval_plan: default_harness_patch_eval_plan(&genome),
        created_at_ms: options.created_at_ms,
    });
    let mut payload = serde_json::to_value(&patch).unwrap_or(Value::Null);
    if let Value::Object(object) = &mut payload {
        object.insert("intake_source".to_string(), json!(options.source.label()));
        object.insert("intake_summary".to_string(), json!(options.summary));
        object.insert("intake_details".to_string(), json!(options.details));
        object.insert(
            "intake_kind".to_string(),
            json!(match options.source {
                IntakeSource::HumanFeedback => "feedback",
                IntakeSource::SelfFiledIssue => "issue",
            }),
        );
    }
    payload
}

fn build_harness_patch(options: PatchProposalOptions) -> HarnessPatch {
    let eval_plan = if options.eval_plan.is_empty() {
        options.default_eval_plan
    } else {
        options.eval_plan
    };
    let rollback_plan = if options.rollback_plan.is_empty() {
        vec!["append PatchRejected and revert any applied harness change".to_string()]
    } else {
        options.rollback_plan
    };

    HarnessPatch {
        patch_id: options.patch_id,
        kind: options.kind,
        status: HarnessPatchStatus::Proposed,
        base_harness_version: options.base_harness_version,
        base_git_commit: options.base_git_commit,
        state_version: 1,
        intent: options.intent,
        evidence_event_ids: options.evidence_event_ids,
        expected_effects: options.expected_effects,
        risk_level: options.risk_level,
        eval_plan,
        rollback_plan,
        created_at_ms: options.created_at_ms,
    }
}

fn append_patch_event(event_type: EventType, payload: Value) -> Result<String, String> {
    let recorder = StateRecorder::new(StateConfig {
        enabled: true,
        fail_soft: false,
        events_path: default_events_path(),
        store_path: default_store_path(),
    });
    recorder.append(event_type, Actor::Harness, payload)
}

fn record_patch_risk_score(patch: &HarnessPatch, proposal_event_id: &str) {
    let payload = build_risk_score_decision_payload(patch, proposal_event_id, now_ms());
    match append_patch_event(EventType::DecisionRecorded, payload) {
        Ok(event_id) => println!("  risk score event id: {event_id}"),
        Err(e) => eprintln!("{RED}  failed to record patch risk score: {e}{RESET}"),
    }
}

#[derive(Debug, Clone)]
struct LifecycleDecisionOptions<'a> {
    patch_id: &'a str,
    decision_type: &'a str,
    decision: &'a str,
    rationale: &'a str,
    status: &'a str,
    patch_event_id: Option<&'a str>,
    eval_id: Option<&'a str>,
    forced: bool,
    decided_at_ms: u128,
    promotion_decision: Option<&'a PromotionDecision>,
    safety_gate: Option<&'a PromotionSafetyGate>,
}

fn build_lifecycle_decision_payload(options: LifecycleDecisionOptions<'_>) -> Value {
    json!({
        "decision_id": format!("decision-{}-{}", sanitize_path_segment(options.patch_id), options.decided_at_ms),
        "decision_type": options.decision_type,
        "decision": options.decision,
        "rationale": options.rationale,
        "status": options.status,
        "patch_id": options.patch_id,
        "patch_event_id": options.patch_event_id,
        "eval_id": options.eval_id,
        "forced": options.forced,
        "decided_at_ms": options.decided_at_ms,
        "promotion_decision": options.promotion_decision,
        "safety_gate": options.safety_gate,
        "approval_event_ids": options.safety_gate
            .map(|gate| gate.approval_event_ids.clone())
            .unwrap_or_default(),
    })
}

fn build_risk_score_decision_payload(
    patch: &HarnessPatch,
    proposal_event_id: &str,
    risk_scored_at_ms: u128,
) -> Value {
    let rationale = format!(
        "patch kind '{}' assigned '{}' lifecycle risk",
        enum_label(&patch.kind),
        enum_label(&patch.risk_level)
    );
    let mut payload = build_lifecycle_decision_payload(LifecycleDecisionOptions {
        patch_id: &patch.patch_id,
        decision_type: "harness_patch_risk_score",
        decision: enum_label(&patch.risk_level),
        rationale: &rationale,
        status: "risk_scored",
        patch_event_id: Some(proposal_event_id),
        eval_id: None,
        forced: false,
        decided_at_ms: risk_scored_at_ms,
        promotion_decision: None,
        safety_gate: None,
    });
    if let Value::Object(object) = &mut payload {
        object.insert("kind".to_string(), json!(&patch.kind));
        object.insert("risk_level".to_string(), json!(&patch.risk_level));
        object.insert(
            "risk_policy".to_string(),
            json!("explicit_or_default_patch_risk"),
        );
        object.insert("scored_at_ms".to_string(), json!(risk_scored_at_ms));
    }
    payload
}

fn enum_label<T: serde::Serialize>(value: &T) -> &'static str {
    match serde_json::to_value(value).ok().and_then(|value| {
        value.as_str().map(|label| match label {
            "context_policy" => "context_policy",
            "prompt_policy" => "prompt_policy",
            "thinking_policy" => "thinking_policy",
            "model_routing_policy" => "model_routing_policy",
            "tool_schema" => "tool_schema",
            "test_policy" => "test_policy",
            "repair_policy" => "repair_policy",
            "memory_policy" => "memory_policy",
            "eval" => "eval",
            "permission_policy" => "permission_policy",
            "bash_policy" => "bash_policy",
            "shell_policy" => "shell_policy",
            "state_projection" => "state_projection",
            "transport" => "transport",
            "safety" => "safety",
            "other" => "other",
            "low" => "low",
            "medium" => "medium",
            "high" => "high",
            "critical" => "critical",
            _ => "unknown",
        })
    }) {
        Some(label) => label,
        None => "unknown",
    }
}

fn build_comparison_decision_payload(
    patch_id: &str,
    decision: &PromotionDecision,
    forced: bool,
    compared_at_ms: u128,
) -> Value {
    let outcome = if decision.eligible {
        "eligible"
    } else {
        "not_eligible"
    };
    build_lifecycle_decision_payload(LifecycleDecisionOptions {
        patch_id,
        decision_type: "harness_patch_comparison",
        decision: outcome,
        rationale: &decision.reason,
        status: "compared",
        patch_event_id: None,
        eval_id: decision.candidate_eval_id.as_deref(),
        forced,
        decided_at_ms: compared_at_ms,
        promotion_decision: Some(decision),
        safety_gate: None,
    })
}

fn promotion_evidence_lines(decision: &PromotionDecision, forced: bool) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!(
        "candidate eval: {} suite={} score={} passed={} failed={}",
        decision.candidate_eval_id.as_deref().unwrap_or("-"),
        decision.suite.as_deref().unwrap_or("-"),
        format_optional_f64(decision.candidate_score),
        format_optional_u64(decision.candidate_passed),
        format_optional_u64(decision.candidate_failed)
    ));
    lines.push(format!(
        "baseline eval:  {} score={} passed={} failed={}",
        decision.baseline_eval_id.as_deref().unwrap_or("-"),
        format_optional_f64(decision.baseline_score),
        format_optional_u64(decision.baseline_passed),
        format_optional_u64(decision.baseline_failed)
    ));
    if let Some(protocol_eval) = promotion_protocol_eval_evidence_line(decision) {
        lines.push(protocol_eval);
    } else if let Some(protocol_eval_id) = &decision.protocol_eval_id {
        lines.push(format!("protocol eval:  {protocol_eval_id}"));
    }
    if let Some(fixture_suite) = promotion_fixture_suite_evidence_line(decision) {
        lines.push(fixture_suite);
    }
    if let Some(model_routes) = promotion_model_route_evidence_line(decision) {
        lines.push(model_routes);
    }
    lines.push(format!(
        "decision: {} criterion={} forced={}",
        if decision.eligible {
            "eligible"
        } else {
            "not_eligible"
        },
        decision.criterion.as_deref().unwrap_or("-"),
        forced
    ));
    lines.push(format!("reason: {}", decision.reason));
    lines
}

fn promotion_model_route_evidence_line(decision: &PromotionDecision) -> Option<String> {
    let routes = decision
        .metric_evidence
        .as_ref()?
        .get("model_route_tasks")?;
    let baseline = routes.get("baseline");
    let candidate = routes.get("candidate");
    let baseline_label = json_u64_map_label(baseline);
    let candidate_label = json_u64_map_label(candidate);
    if baseline_label == "-" && candidate_label == "-" {
        return None;
    }
    Some(format!(
        "model routes: baseline [{baseline_label}] candidate [{candidate_label}]"
    ))
}

fn promotion_fixture_suite_evidence_line(decision: &PromotionDecision) -> Option<String> {
    let fixture_suite = decision.metric_evidence.as_ref()?.get("fixture_suite")?;
    let baseline = fixture_suite.get("baseline")?;
    let candidate = fixture_suite.get("candidate")?;
    Some(format!(
        "fixture suite: baseline tasks={} commands={} risks=[{}] candidate tasks={} commands={} risks=[{}]",
        json_u64_label(baseline.get("task_count")),
        json_u64_label(baseline.get("command_count")),
        json_u64_map_label(baseline.get("risk_labels")),
        json_u64_label(candidate.get("task_count")),
        json_u64_label(candidate.get("command_count")),
        json_u64_map_label(candidate.get("risk_labels"))
    ))
}

fn promotion_protocol_eval_evidence_line(decision: &PromotionDecision) -> Option<String> {
    let protocol_eval_id = decision.protocol_eval_id.as_deref()?;
    let protocol = decision.metric_evidence.as_ref()?.get("protocol_eval")?;
    let status = protocol
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("-");
    let dirty = protocol
        .get("git_dirty")
        .and_then(Value::as_bool)
        .map(yes_no)
        .unwrap_or("-");
    let checks = format_protocol_check_evidence(protocol)
        .map(|checks| format!(" {checks}"))
        .unwrap_or_default();
    Some(format!(
        "protocol eval:  {protocol_eval_id} status={status} dirty={dirty} created_at_ms={}{}",
        json_u64_label(protocol.get("created_at_ms")),
        checks
    ))
}

fn format_protocol_check_evidence(protocol: &Value) -> Option<String> {
    let checks = protocol.get("protocol_checks")?;
    let total = json_u64_label(checks.get("total"));
    let passes = json_u64_label(checks.get("passes"));
    Some(format!(
        "checks={passes}/{total} strict={} thinking={} stream={} json={} transport={}",
        json_u64_label(checks.get("strict")),
        json_u64_label(checks.get("thinking")),
        json_u64_label(checks.get("stream")),
        json_u64_label(checks.get("json")),
        json_u64_label(checks.get("transport"))
    ))
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn json_u64_label(value: Option<&Value>) -> String {
    value
        .and_then(value_as_u64)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn json_u64_map_label(value: Option<&Value>) -> String {
    let Some(map) = value
        .and_then(Value::as_object)
        .map(|object| {
            object
                .iter()
                .filter_map(|(key, value)| value_as_u64(value).map(|count| (key.clone(), count)))
                .collect::<BTreeMap<_, _>>()
        })
        .filter(|map| !map.is_empty())
    else {
        return "-".to_string();
    };
    map.into_iter()
        .map(|(key, count)| format!("{key}={count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_optional_f64(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "-".to_string())
}

fn format_optional_u64(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn read_events(path: &Path) -> Result<Vec<Value>, std::io::Error> {
    crate::state::read_compatibility_events(path).map_err(std::io::Error::other)
}

fn patch_exists(events: &[Value], patch_id: &str) -> bool {
    events.iter().any(|event| {
        event
            .get("event_type")
            .and_then(|v| v.as_str())
            .map(|kind| kind == "PatchProposed")
            .unwrap_or(false)
            && event
                .get("payload")
                .and_then(|payload| payload.get("patch_id"))
                .and_then(|v| v.as_str())
                .map(|candidate| candidate == patch_id)
                .unwrap_or(false)
    })
}

fn eval_worktree_for_patch(
    events: &[Value],
    patch_id: &str,
    requested_worktree: Option<PathBuf>,
) -> Result<Option<PathBuf>, String> {
    if !patch_exists(events, patch_id) {
        return Err(format!("no proposed harness patch found for '{patch_id}'"));
    }
    let Some(applied) = latest_applied_patch(events, patch_id) else {
        return Err(format!(
            "no applied harness patch found for '{patch_id}'; run yoyo evolve harness apply first"
        ));
    };
    Ok(requested_worktree.or(Some(applied.worktree_path)))
}

fn eval_worktree_isolation_gate(
    selected_worktree: Option<&Path>,
    active_repo: &Path,
) -> Result<(), String> {
    let Some(worktree) = selected_worktree else {
        return Err("no isolated eval worktree selected".to_string());
    };
    if same_filesystem_path(worktree, active_repo) {
        return Err(format!(
            "eval worktree '{}' points at the active repository checkout",
            worktree.display()
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct PatchSafetyMetadata {
    patch_id: String,
    #[serde(default = "default_patch_kind")]
    kind: HarnessPatchKind,
    #[serde(default = "default_patch_risk")]
    risk_level: HarnessPatchRisk,
    #[serde(default)]
    rollback_plan: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct HarnessEvolutionPolicy {
    allowed_patch_types: Option<Vec<HarnessPatchKind>>,
    require_human_approval_for: Vec<HarnessPatchKind>,
}

fn default_patch_kind() -> HarnessPatchKind {
    HarnessPatchKind::Other
}

fn default_patch_risk() -> HarnessPatchRisk {
    HarnessPatchRisk::Medium
}

fn find_patch_safety_metadata(events: &[Value], patch_id: &str) -> Option<PatchSafetyMetadata> {
    events.iter().rev().find_map(|event| {
        if !event
            .get("event_type")
            .and_then(|v| v.as_str())
            .map(|kind| kind == "PatchProposed")
            .unwrap_or(false)
        {
            return None;
        }
        let payload = event.get("payload")?;
        if !payload
            .get("patch_id")
            .and_then(|v| v.as_str())
            .map(|candidate| candidate == patch_id)
            .unwrap_or(false)
        {
            return None;
        }
        serde_json::from_value::<PatchSafetyMetadata>(payload.clone()).ok()
    })
}

fn active_harness_evolution_policy() -> HarnessEvolutionPolicy {
    let (config, _) = crate::config::load_deepseek_config_file();
    harness_evolution_policy_from_config(&config)
}

fn harness_evolution_policy_from_config(
    config: &std::collections::HashMap<String, String>,
) -> HarnessEvolutionPolicy {
    HarnessEvolutionPolicy {
        allowed_patch_types: config
            .get("evolve_harness_allowed_patch_types")
            .and_then(|raw| parse_patch_kind_list(raw).ok())
            .filter(|items| !items.is_empty()),
        require_human_approval_for: config
            .get("evolve_harness_require_human_approval_for")
            .and_then(|raw| parse_patch_kind_list(raw).ok())
            .unwrap_or_default(),
    }
}

#[cfg(test)]
fn latest_passed_eval(events: &[Value], patch_id: &str) -> Option<String> {
    events.iter().rev().find_map(|event| {
        if !event
            .get("event_type")
            .and_then(|v| v.as_str())
            .map(|kind| kind == "PatchEvaluated")
            .unwrap_or(false)
        {
            return None;
        }
        let payload = event.get("payload")?;
        let eval = serde_json::from_value::<EvalResult>(payload.clone()).ok()?;
        if eval.patch_id.as_deref() == Some(patch_id) && eval.status == EvalStatus::Passed {
            Some(eval.eval_id)
        } else {
            None
        }
    })
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct PromotionDecision {
    eligible: bool,
    criterion: Option<String>,
    reason: String,
    baseline_eval_id: Option<String>,
    candidate_eval_id: Option<String>,
    protocol_eval_id: Option<String>,
    suite: Option<String>,
    baseline_score: Option<f64>,
    candidate_score: Option<f64>,
    baseline_passed: Option<u64>,
    candidate_passed: Option<u64>,
    baseline_failed: Option<u64>,
    candidate_failed: Option<u64>,
    metric_evidence: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct PromotionSafetyGate {
    allowed: bool,
    requires_human_approval: bool,
    reason: String,
    patch_kind: HarnessPatchKind,
    risk_level: HarnessPatchRisk,
    approval_event_ids: Vec<String>,
}

fn promotion_safety_gate_after_candidate_eval(
    events: &[Value],
    patch: &PatchSafetyMetadata,
    explicit_approval_event_ids: &[String],
    min_approval_ms: Option<u128>,
) -> PromotionSafetyGate {
    if !patch_has_rollback_plan(patch) {
        return PromotionSafetyGate {
            allowed: false,
            requires_human_approval: false,
            reason: "patch has no rollback plan".to_string(),
            patch_kind: patch.kind.clone(),
            risk_level: patch.risk_level.clone(),
            approval_event_ids: Vec::new(),
        };
    }

    let requires_human_approval = patch_requires_human_approval(patch);
    if !requires_human_approval {
        return PromotionSafetyGate {
            allowed: true,
            requires_human_approval: false,
            reason: "patch risk does not require human approval".to_string(),
            patch_kind: patch.kind.clone(),
            risk_level: patch.risk_level.clone(),
            approval_event_ids: Vec::new(),
        };
    }

    let approval_event_ids = if explicit_approval_event_ids.is_empty() {
        latest_human_promotion_approval_event(events, &patch.patch_id, min_approval_ms)
            .into_iter()
            .collect()
    } else {
        valid_explicit_promotion_approval_events(
            events,
            &patch.patch_id,
            explicit_approval_event_ids,
            min_approval_ms,
        )
    };
    if approval_event_ids.is_empty() {
        return PromotionSafetyGate {
            allowed: false,
            requires_human_approval: true,
            reason:
                "high-risk or safety patch has no fresh HumanApprovalReceived event for promotion"
                    .to_string(),
            patch_kind: patch.kind.clone(),
            risk_level: patch.risk_level.clone(),
            approval_event_ids,
        };
    }

    PromotionSafetyGate {
        allowed: true,
        requires_human_approval: true,
        reason: "fresh human approval event found for promotion".to_string(),
        patch_kind: patch.kind.clone(),
        risk_level: patch.risk_level.clone(),
        approval_event_ids,
    }
}

fn apply_approval_gate(events: &[Value], patch: &PatchSafetyMetadata) -> PromotionSafetyGate {
    let requires_human_approval = patch_requires_human_approval(patch);
    if !requires_human_approval {
        return PromotionSafetyGate {
            allowed: true,
            requires_human_approval: false,
            reason: "patch risk does not require human approval before fork apply".to_string(),
            patch_kind: patch.kind.clone(),
            risk_level: patch.risk_level.clone(),
            approval_event_ids: Vec::new(),
        };
    }

    let approval_event_ids: Vec<String> = latest_human_approval_event(events, &patch.patch_id)
        .into_iter()
        .collect();
    if approval_event_ids.is_empty() {
        return PromotionSafetyGate {
            allowed: false,
            requires_human_approval: true,
            reason:
                "high-risk or safety patch has no HumanApprovalReceived event before fork apply"
                    .to_string(),
            patch_kind: patch.kind.clone(),
            risk_level: patch.risk_level.clone(),
            approval_event_ids,
        };
    }

    PromotionSafetyGate {
        allowed: true,
        requires_human_approval: true,
        reason: "human approval found for fork apply".to_string(),
        patch_kind: patch.kind.clone(),
        risk_level: patch.risk_level.clone(),
        approval_event_ids,
    }
}

fn apply_mutation_surface_gate(patch: &PatchSafetyMetadata) -> Result<(), String> {
    let policy = active_harness_evolution_policy();
    apply_mutation_surface_gate_with_policy(patch, &policy)
}

fn apply_mutation_surface_gate_with_policy(
    patch: &PatchSafetyMetadata,
    policy: &HarnessEvolutionPolicy,
) -> Result<(), String> {
    if patch_kind_allowed_for_unforced_apply(&patch.kind, policy) {
        return Ok(());
    }
    Err(format!(
        "patch kind '{}' is outside the initial constrained harness mutation surface",
        enum_label(&patch.kind)
    ))
}

fn patch_kind_allowed_for_unforced_apply(
    kind: &HarnessPatchKind,
    policy: &HarnessEvolutionPolicy,
) -> bool {
    if let Some(allowed) = &policy.allowed_patch_types {
        return allowed.iter().any(|allowed_kind| allowed_kind == kind);
    }
    matches!(
        kind,
        HarnessPatchKind::ContextPolicy
            | HarnessPatchKind::PromptPolicy
            | HarnessPatchKind::ToolSchema
            | HarnessPatchKind::ThinkingPolicy
            | HarnessPatchKind::ModelRoutingPolicy
            | HarnessPatchKind::TestPolicy
            | HarnessPatchKind::RepairPolicy
            | HarnessPatchKind::MemoryPolicy
    )
}

fn patch_has_rollback_plan(patch: &PatchSafetyMetadata) -> bool {
    patch
        .rollback_plan
        .iter()
        .any(|step| !step.trim().is_empty())
}

fn patch_requires_human_approval(patch: &PatchSafetyMetadata) -> bool {
    let policy = active_harness_evolution_policy();
    patch_requires_human_approval_with_policy(patch, &policy)
}

fn patch_requires_human_approval_with_policy(
    patch: &PatchSafetyMetadata,
    policy: &HarnessEvolutionPolicy,
) -> bool {
    if policy
        .require_human_approval_for
        .iter()
        .any(|kind| kind == &patch.kind)
    {
        return true;
    }
    matches!(
        patch.kind,
        HarnessPatchKind::Safety
            | HarnessPatchKind::PermissionPolicy
            | HarnessPatchKind::ShellPolicy
    ) || matches!(
        patch.risk_level,
        HarnessPatchRisk::High | HarnessPatchRisk::Critical
    )
}

fn latest_human_approval_event(events: &[Value], patch_id: &str) -> Option<String> {
    events.iter().rev().find_map(|event| {
        if is_human_approval_for_patch(event, patch_id) {
            event
                .get("event_id")
                .and_then(|v| v.as_str())
                .map(|id| id.to_string())
        } else {
            None
        }
    })
}

fn latest_human_promotion_approval_event(
    events: &[Value],
    patch_id: &str,
    min_approval_ms: Option<u128>,
) -> Option<String> {
    events.iter().rev().find_map(|event| {
        if is_human_promotion_approval_for_patch(event, patch_id, min_approval_ms) {
            event
                .get("event_id")
                .and_then(|v| v.as_str())
                .map(|id| id.to_string())
        } else {
            None
        }
    })
}

fn valid_explicit_promotion_approval_events(
    events: &[Value],
    patch_id: &str,
    explicit_approval_event_ids: &[String],
    min_approval_ms: Option<u128>,
) -> Vec<String> {
    explicit_approval_event_ids
        .iter()
        .filter(|event_id| {
            events.iter().any(|event| {
                event
                    .get("event_id")
                    .and_then(|v| v.as_str())
                    .map(|candidate| candidate == event_id.as_str())
                    .unwrap_or(false)
                    && is_human_promotion_approval_for_patch(event, patch_id, min_approval_ms)
            })
        })
        .cloned()
        .collect()
}

fn is_human_approval_for_patch(event: &Value, patch_id: &str) -> bool {
    event
        .get("event_type")
        .and_then(|v| v.as_str())
        .map(|kind| kind == "HumanApprovalReceived")
        .unwrap_or(false)
        && event
            .get("payload")
            .and_then(|payload| payload.get("patch_id"))
            .and_then(|v| v.as_str())
            .map(|candidate| candidate == patch_id)
            .unwrap_or(false)
}

fn is_human_promotion_approval_for_patch(
    event: &Value,
    patch_id: &str,
    min_approval_ms: Option<u128>,
) -> bool {
    is_human_approval_for_patch(event, patch_id)
        && event
            .get("payload")
            .and_then(|payload| payload.get("approval_scope"))
            .and_then(|v| v.as_str())
            .map(|scope| scope == "harness_patch_promotion")
            .unwrap_or(false)
        && min_approval_ms
            .map(|min_ms| approval_event_ms(event).unwrap_or(0) >= min_ms)
            .unwrap_or(true)
}

fn approval_event_ms(event: &Value) -> Option<u128> {
    event
        .get("payload")
        .and_then(|payload| payload.get("approved_at_ms"))
        .and_then(json_u128)
        .or_else(|| event.get("timestamp_ms").and_then(json_u128))
}

fn json_u128(value: &Value) -> Option<u128> {
    value
        .as_u64()
        .map(u128::from)
        .or_else(|| value.as_str().and_then(|raw| raw.parse::<u128>().ok()))
}

impl PromotionDecision {
    fn reject(reason: impl Into<String>, candidate: Option<&EvalResult>) -> Self {
        Self {
            eligible: false,
            criterion: None,
            reason: reason.into(),
            baseline_eval_id: None,
            candidate_eval_id: candidate.map(|eval| eval.eval_id.clone()),
            protocol_eval_id: None,
            suite: candidate.map(|eval| eval.suite.clone()),
            baseline_score: None,
            candidate_score: candidate.and_then(|eval| eval.score),
            baseline_passed: None,
            candidate_passed: candidate.map(|eval| eval.passed),
            baseline_failed: None,
            candidate_failed: candidate.map(|eval| eval.failed),
            metric_evidence: None,
        }
    }

    fn compared(
        eligible: bool,
        criterion: Option<&str>,
        reason: impl Into<String>,
        baseline: &EvalResult,
        candidate: &EvalResult,
    ) -> Self {
        Self {
            eligible,
            criterion: criterion.map(|value| value.to_string()),
            reason: reason.into(),
            baseline_eval_id: Some(baseline.eval_id.clone()),
            candidate_eval_id: Some(candidate.eval_id.clone()),
            protocol_eval_id: None,
            suite: Some(candidate.suite.clone()),
            baseline_score: baseline.score,
            candidate_score: candidate.score,
            baseline_passed: Some(baseline.passed),
            candidate_passed: Some(candidate.passed),
            baseline_failed: Some(baseline.failed),
            candidate_failed: Some(candidate.failed),
            metric_evidence: Some(promotion_metric_evidence(baseline, candidate)),
        }
    }

    fn with_protocol_eval(mut self, protocol: &EvalResult) -> Self {
        self.protocol_eval_id = Some(protocol.eval_id.clone());
        let protocol_evidence = protocol_eval_promotion_evidence(protocol);
        match &mut self.metric_evidence {
            Some(Value::Object(evidence)) => {
                evidence.insert("protocol_eval".to_string(), protocol_evidence);
            }
            _ => {
                self.metric_evidence = Some(json!({
                    "protocol_eval": protocol_evidence,
                }));
            }
        }
        self
    }
}

fn protocol_eval_promotion_evidence(protocol: &EvalResult) -> Value {
    let mut evidence = json!({
        "eval_id": protocol.eval_id,
        "suite": protocol.suite,
        "status": eval_status_label(&protocol.status),
        "score": protocol.score,
        "passed": protocol.passed,
        "failed": protocol.failed,
        "created_at_ms": protocol.created_at_ms,
        "git_dirty": eval_reproducibility_git_dirty(protocol),
    });
    if let Some(counts) = protocol_eval_protocol_check_counts(protocol) {
        evidence["protocol_checks"] = counts;
    }
    evidence
}

fn protocol_eval_protocol_check_counts(protocol: &EvalResult) -> Option<Value> {
    let metrics = protocol.metrics.get("state_metrics")?;
    let mut counts = serde_json::Map::new();
    for (label, key) in [
        ("total", "deepseek_protocol_checks"),
        ("passes", "deepseek_protocol_passes"),
        ("strict", "deepseek_strict_tool_call_checks"),
        ("thinking", "deepseek_thinking_protocol_checks"),
        ("stream", "deepseek_streaming_protocol_checks"),
        ("json", "deepseek_json_output_checks"),
        ("transport", "deepseek_transport_policy_checks"),
    ] {
        if let Some(value) = metrics.get(key).and_then(value_as_u64) {
            counts.insert(label.to_string(), json!(value));
        }
    }
    (!counts.is_empty()).then_some(Value::Object(counts))
}

fn eval_status_label(status: &EvalStatus) -> &'static str {
    match status {
        EvalStatus::Passed => "passed",
        EvalStatus::Failed => "failed",
        EvalStatus::Error => "error",
        EvalStatus::NoEvidence => "no_evidence",
    }
}

fn promotion_decision(
    events: &[Value],
    patch_id: &str,
    baseline_eval_id: Option<&str>,
    candidate_eval_id: Option<&str>,
) -> PromotionDecision {
    let evals = patch_evals(events);
    let candidate = match candidate_eval_id {
        Some(eval_id) => evals.iter().find(|eval| eval.eval_id == eval_id),
        None => latest_patch_eval(&evals, patch_id),
    };
    let Some(candidate) = candidate else {
        return PromotionDecision::reject("no candidate eval found for patch", None);
    };
    if candidate.patch_id.as_deref() != Some(patch_id) {
        return PromotionDecision::reject(
            "candidate eval does not belong to patch",
            Some(candidate),
        );
    }
    if candidate.status != EvalStatus::Passed {
        return PromotionDecision::reject("candidate eval did not pass", Some(candidate));
    }
    if eval_reproducibility_git_dirty(candidate) == Some(true) {
        return PromotionDecision::reject(
            "candidate eval was run from a dirty worktree",
            Some(candidate),
        );
    }
    let missing_required_gates = eval_missing_required_promotion_gates(candidate);
    if !missing_required_gates.is_empty() {
        return PromotionDecision::reject(
            format!(
                "candidate eval is missing required gate evidence: {}",
                missing_required_gates.join(", ")
            ),
            Some(candidate),
        );
    }
    let protocol_eval = latest_protocol_eval(&evals);
    let protocol_required = patch_requires_protocol_eval(events, patch_id);
    if protocol_required {
        let Some(protocol_eval) = protocol_eval else {
            return PromotionDecision::reject(
                "patch eval plan requires protocol eval but none was found",
                Some(candidate),
            );
        };
        if protocol_eval.status != EvalStatus::Passed {
            return PromotionDecision::reject("latest protocol eval did not pass", Some(candidate))
                .with_protocol_eval(protocol_eval);
        }
        if eval_reproducibility_git_dirty(protocol_eval) == Some(true) {
            return PromotionDecision::reject(
                "latest protocol eval was run from a dirty worktree",
                Some(candidate),
            )
            .with_protocol_eval(protocol_eval);
        }
        if protocol_eval.created_at_ms < candidate.created_at_ms {
            return PromotionDecision::reject(
                "latest protocol eval is older than candidate eval",
                Some(candidate),
            )
            .with_protocol_eval(protocol_eval);
        }
    } else if let Some(protocol_eval) = protocol_eval {
        if protocol_eval.status != EvalStatus::Passed {
            return PromotionDecision::reject("latest protocol eval did not pass", Some(candidate))
                .with_protocol_eval(protocol_eval);
        }
        if eval_reproducibility_git_dirty(protocol_eval) == Some(true) {
            return PromotionDecision::reject(
                "latest protocol eval was run from a dirty worktree",
                Some(candidate),
            )
            .with_protocol_eval(protocol_eval);
        }
    }

    let baseline = match baseline_eval_id {
        Some(eval_id) => evals.iter().find(|eval| eval.eval_id == eval_id),
        None => latest_baseline_eval(&evals, &candidate.suite),
    };
    let Some(baseline) = baseline else {
        return PromotionDecision::reject(
            "no baseline eval found for candidate suite",
            Some(candidate),
        );
    };
    if eval_reproducibility_git_dirty(baseline) == Some(true) {
        return PromotionDecision::compared(
            false,
            None,
            "baseline eval was run from a dirty worktree",
            baseline,
            candidate,
        );
    }
    if baseline.suite != candidate.suite {
        return PromotionDecision::compared(
            false,
            None,
            "baseline and candidate suites differ",
            baseline,
            candidate,
        );
    }
    let missing_baseline_gates = eval_missing_required_promotion_gates(baseline);
    if !missing_baseline_gates.is_empty() {
        return PromotionDecision::compared(
            false,
            None,
            format!(
                "baseline eval is missing required gate evidence: {}",
                missing_baseline_gates.join(", ")
            ),
            baseline,
            candidate,
        );
    }
    if fixture_suite_breadth_mismatch(baseline, candidate) {
        return PromotionDecision::compared(
            false,
            None,
            "baseline and candidate fixture suite breadth differ",
            baseline,
            candidate,
        );
    }
    if fixture_suite_risk_label_mismatch(baseline, candidate) {
        return PromotionDecision::compared(
            false,
            None,
            "baseline and candidate fixture suite risk-label coverage differ",
            baseline,
            candidate,
        );
    }

    promotion_comparison(baseline, candidate)
}

fn promotion_comparison(baseline: &EvalResult, candidate: &EvalResult) -> PromotionDecision {
    const EPSILON: f64 = 0.000_001;
    const MAX_NON_QUALITY_BUDGET_INCREASE_RATIO: f64 = 0.25;
    let baseline_score = baseline.score.unwrap_or(0.0);
    let candidate_score = candidate.score.unwrap_or(0.0);
    let no_pass_regression = candidate_score + EPSILON >= baseline_score
        && candidate.passed >= baseline.passed
        && candidate.failed <= baseline.failed;

    if candidate_score > baseline_score + EPSILON {
        return PromotionDecision::compared(
            true,
            Some("pass_rate_improved"),
            "candidate score improves over baseline",
            baseline,
            candidate,
        );
    }
    if no_pass_regression && candidate.status == EvalStatus::Passed {
        if let Some(reason) =
            uncontrolled_budget_increase(baseline, candidate, MAX_NON_QUALITY_BUDGET_INCREASE_RATIO)
        {
            return PromotionDecision::compared(false, None, reason, baseline, candidate);
        }
        if let Some(reason) = harness_quality_regression(baseline, candidate) {
            return PromotionDecision::compared(false, None, reason, baseline, candidate);
        }
        if metric_less(candidate, baseline, "cost_usd")
            || metric_less(candidate, baseline, "token_cost_usd")
            || metric_less(candidate, baseline, "cost_per_successful_task")
            || metric_less(candidate, baseline, "cost_per_successful_task_usd")
        {
            return PromotionDecision::compared(
                true,
                Some("cost_reduced_no_regression"),
                "candidate cost is lower with no pass-rate regression",
                baseline,
                candidate,
            );
        }
        if token_total_less(candidate, baseline) {
            return PromotionDecision::compared(
                true,
                Some("token_usage_reduced_no_regression"),
                "candidate token usage is lower with no pass-rate regression",
                baseline,
                candidate,
            );
        }
        if metric_less(candidate, baseline, "total_duration_ms")
            || metric_less(candidate, baseline, "latency_ms")
            || metric_less(candidate, baseline, "latency_per_successful_task_ms")
        {
            return PromotionDecision::compared(
                true,
                Some("duration_reduced_no_regression"),
                "candidate duration is lower with no pass-rate regression",
                baseline,
                candidate,
            );
        }
        if metric_greater(candidate, baseline, "cache_hit_ratio")
            || metric_greater(candidate, baseline, "cache_ratio")
        {
            return PromotionDecision::compared(
                true,
                Some("cache_ratio_improved"),
                "candidate cache ratio improves with no pass-rate regression",
                baseline,
                candidate,
            );
        }
        if metric_greater(candidate, baseline, "tool_call_success_rate")
            || metric_greater(candidate, baseline, "tool_success_rate")
            || metric_less(candidate, baseline, "malformed_tool_call_rate")
        {
            return PromotionDecision::compared(
                true,
                Some("tool_reliability_improved"),
                "candidate tool reliability improves with no pass-rate regression",
                baseline,
                candidate,
            );
        }
        if metric_less(candidate, baseline, "json_parse_failure_rate")
            || metric_less(candidate, baseline, "json_output_failures")
        {
            return PromotionDecision::compared(
                true,
                Some("json_reliability_improved"),
                "candidate JSON reliability improves with no pass-rate regression",
                baseline,
                candidate,
            );
        }
        if metric_less(candidate, baseline, "context_miss_rate")
            || metric_less(candidate, baseline, "context_misses")
        {
            return PromotionDecision::compared(
                true,
                Some("context_miss_reduced"),
                "candidate context misses are lower with no pass-rate regression",
                baseline,
                candidate,
            );
        }
        if metric_less(candidate, baseline, "repair_loop_count") {
            return PromotionDecision::compared(
                true,
                Some("repair_loop_reduced"),
                "candidate repair loop count is lower with no pass-rate regression",
                baseline,
                candidate,
            );
        }
        if metric_less(candidate, baseline, "fixture_agent_mutation_scope_failures")
            || metric_less(
                candidate,
                baseline,
                "fixture_agent_unexpected_changed_file_count",
            )
            || metric_less(
                candidate,
                baseline,
                "fixture_agent_mutation_scope_failure_rate",
            )
        {
            return PromotionDecision::compared(
                true,
                Some("mutation_scope_improved"),
                "candidate fixture agent mutation scope failures are lower with no pass-rate regression",
                baseline,
                candidate,
            );
        }
        if metric_less(candidate, baseline, "failures") {
            return PromotionDecision::compared(
                true,
                Some("state_failure_reduced"),
                "candidate state failure count is lower with no pass-rate regression",
                baseline,
                candidate,
            );
        }
        if metric_greater(candidate, baseline, "fim_success_rate")
            || metric_greater(candidate, baseline, "fim_compile_rate")
            || metric_less(candidate, baseline, "fim_rollback_rate")
        {
            return PromotionDecision::compared(
                true,
                Some("fim_quality_improved"),
                "candidate FIM quality improves with no pass-rate regression",
                baseline,
                candidate,
            );
        }
        if metric_less(candidate, baseline, "permission_prompt_rate")
            || metric_less(candidate, baseline, "human_intervention_rate")
            || metric_less(candidate, baseline, "human_interventions")
        {
            return PromotionDecision::compared(
                true,
                Some("intervention_pressure_reduced"),
                "candidate permission or human intervention pressure is lower with no pass-rate regression",
                baseline,
                candidate,
            );
        }
        if metric_less(candidate, baseline, "rollback_count")
            || metric_less(candidate, baseline, "rollback_rate")
        {
            return PromotionDecision::compared(
                true,
                Some("rollback_reduced"),
                "candidate rollback metric is lower with no pass-rate regression",
                baseline,
                candidate,
            );
        }
        if metric_greater(candidate, baseline, "coding_log_score")
            || metric_greater(candidate, baseline, "workflow_success_rate")
            || metric_greater(candidate, baseline, "session_success_rate")
            || metric_greater(candidate, baseline, "task_success_rate")
            || metric_greater(candidate, baseline, "retry_success_rate")
            || metric_greater(candidate, baseline, "closed_loop_fix_rate")
        {
            return PromotionDecision::compared(
                true,
                Some("log_feedback_improved"),
                "candidate GitHub Actions log feedback improves with no pass-rate regression",
                baseline,
                candidate,
            );
        }
        if metric_less(candidate, baseline, "recurring_failure_count")
            || metric_less(candidate, baseline, "max_failure_fingerprint_recurrence")
            || metric_less(candidate, baseline, "provider_error_count")
            || metric_less(candidate, baseline, "provider_blocked_session_count")
        {
            return PromotionDecision::compared(
                true,
                Some("log_feedback_reliability_improved"),
                "candidate GitHub Actions log failure pressure is lower with no pass-rate regression",
                baseline,
                candidate,
            );
        }
        return PromotionDecision::compared(
            false,
            None,
            "candidate passed but no promotion metric improved",
            baseline,
            candidate,
        );
    }

    PromotionDecision::compared(
        false,
        None,
        "candidate does not improve or preserve baseline results",
        baseline,
        candidate,
    )
}

fn uncontrolled_budget_increase(
    baseline: &EvalResult,
    candidate: &EvalResult,
    max_increase_ratio: f64,
) -> Option<String> {
    for key in [
        "cost_usd",
        "token_cost_usd",
        "cost_per_successful_task",
        "cost_per_successful_task_usd",
    ] {
        if metric_increase_ratio(candidate, baseline, key)
            .map(|ratio| ratio > max_increase_ratio)
            .unwrap_or(false)
        {
            return Some(format!(
                "candidate {key} increases beyond {:.0}% budget gate",
                max_increase_ratio * 100.0
            ));
        }
    }

    match (
        token_total(&candidate.metrics),
        token_total(&baseline.metrics),
    ) {
        (Some(candidate_total), Some(baseline_total)) if baseline_total > 0 => {
            let ratio = (candidate_total as f64 - baseline_total as f64) / baseline_total as f64;
            if ratio > max_increase_ratio {
                return Some(format!(
                    "candidate token_total increases beyond {:.0}% budget gate",
                    max_increase_ratio * 100.0
                ));
            }
        }
        _ => {}
    }

    None
}

fn harness_quality_regression(baseline: &EvalResult, candidate: &EvalResult) -> Option<String> {
    for key in [
        "malformed_tool_call_rate",
        "json_parse_failure_rate",
        "context_miss_rate",
        "repair_loop_count",
        "fixture_agent_mutation_scope_failures",
        "fixture_agent_unexpected_changed_file_count",
        "fixture_agent_mutation_scope_failure_rate",
        "permission_prompt_rate",
        "human_intervention_rate",
        "human_interventions",
        "rollback_count",
        "rollback_rate",
        "fim_rollback_rate",
        "recurring_failure_count",
        "max_failure_fingerprint_recurrence",
        "provider_error_count",
        "provider_blocked_session_count",
    ] {
        if metric_greater(candidate, baseline, key) {
            return Some(format!("candidate {key} regresses harness quality gate"));
        }
    }
    for key in [
        "fim_success_rate",
        "fim_compile_rate",
        "coding_log_score",
        "workflow_success_rate",
        "session_success_rate",
        "task_success_rate",
        "state_capture_coverage",
        "audit_capture_coverage",
    ] {
        if metric_less(candidate, baseline, key) {
            return Some(format!("candidate {key} regresses harness quality gate"));
        }
    }
    None
}

fn patch_evals(events: &[Value]) -> Vec<EvalResult> {
    events
        .iter()
        .filter_map(|event| {
            if !event
                .get("event_type")
                .and_then(|v| v.as_str())
                .map(|kind| kind == "PatchEvaluated")
                .unwrap_or(false)
            {
                return None;
            }
            serde_json::from_value::<EvalResult>(event.get("payload")?.clone()).ok()
        })
        .collect()
}

fn latest_patch_eval<'a>(evals: &'a [EvalResult], patch_id: &str) -> Option<&'a EvalResult> {
    evals
        .iter()
        .rev()
        .find(|eval| eval.patch_id.as_deref() == Some(patch_id))
}

fn latest_baseline_eval<'a>(evals: &'a [EvalResult], suite: &str) -> Option<&'a EvalResult> {
    evals
        .iter()
        .rev()
        .find(|eval| eval.patch_id.is_none() && eval.suite == suite)
}

fn latest_protocol_eval(evals: &[EvalResult]) -> Option<&EvalResult> {
    evals
        .iter()
        .rev()
        .find(|eval| eval_is_type(eval, "protocol"))
}

fn patch_requires_protocol_eval(events: &[Value], patch_id: &str) -> bool {
    events.iter().rev().any(|event| {
        let Some(payload) = event.get("payload") else {
            return false;
        };
        event.get("event_type").and_then(Value::as_str) == Some("PatchProposed")
            && payload.get("patch_id").and_then(Value::as_str) == Some(patch_id)
            && payload
                .get("eval_plan")
                .and_then(Value::as_array)
                .map(|steps| {
                    steps
                        .iter()
                        .filter_map(Value::as_str)
                        .any(|step| step.contains("protocol-deepseek"))
                })
                .unwrap_or(false)
    })
}

fn eval_is_type(eval: &EvalResult, expected: &str) -> bool {
    eval.metrics
        .get("eval_type")
        .and_then(Value::as_str)
        .map(|eval_type| eval_type == expected)
        .unwrap_or_else(|| eval.suite.to_ascii_lowercase().contains(expected))
}

fn eval_reproducibility_git_dirty(eval: &EvalResult) -> Option<bool> {
    eval.metrics
        .get("reproducibility")
        .and_then(|manifest| manifest.get("git_dirty"))
        .and_then(Value::as_bool)
}

fn eval_missing_required_promotion_gates(eval: &EvalResult) -> Vec<String> {
    let Some(commands) = eval_reproducibility_commands(eval) else {
        return crate::deepseek::DeepSeekHarnessGenome::default()
            .test_policy
            .required_gates;
    };
    crate::deepseek::DeepSeekHarnessGenome::default()
        .test_policy
        .required_gates
        .into_iter()
        .filter(|gate| {
            !commands
                .iter()
                .any(|command| command_satisfies_required_gate(command, gate))
        })
        .collect()
}

fn eval_reproducibility_commands(eval: &EvalResult) -> Option<Vec<String>> {
    eval.metrics
        .get("reproducibility")
        .and_then(|manifest| manifest.get("commands"))
        .and_then(Value::as_array)
        .map(|commands| {
            commands
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
}

fn fixture_suite_breadth_mismatch(baseline: &EvalResult, candidate: &EvalResult) -> bool {
    match (
        eval_fixture_suite_breadth(baseline),
        eval_fixture_suite_breadth(candidate),
    ) {
        (Some(baseline), Some(candidate)) => baseline != candidate,
        _ => false,
    }
}

fn eval_fixture_suite_breadth(eval: &EvalResult) -> Option<(Option<u64>, Option<u64>)> {
    let fixture_suite = eval.metrics.get("fixture_suite")?;
    Some((
        fixture_suite.get("task_count").and_then(value_as_u64),
        fixture_suite.get("command_count").and_then(value_as_u64),
    ))
}

fn fixture_suite_risk_label_mismatch(baseline: &EvalResult, candidate: &EvalResult) -> bool {
    match (
        eval_fixture_suite_risk_labels(baseline),
        eval_fixture_suite_risk_labels(candidate),
    ) {
        (Some(baseline), Some(candidate)) => baseline != candidate,
        _ => false,
    }
}

fn eval_fixture_suite_risk_labels(eval: &EvalResult) -> Option<BTreeMap<String, u64>> {
    let fixture_suite = eval.metrics.get("fixture_suite")?;
    let risks = fixture_suite.get("risk_labels")?.as_object()?;
    Some(
        risks
            .iter()
            .filter_map(|(label, value)| value_as_u64(value).map(|count| (label.clone(), count)))
            .collect(),
    )
}

fn command_satisfies_required_gate(command: &str, required_gate: &str) -> bool {
    let command = command.split_whitespace().collect::<Vec<_>>().join(" ");
    let required_gate = required_gate
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    command == required_gate
}

fn metric_greater(candidate: &EvalResult, baseline: &EvalResult, key: &str) -> bool {
    match (
        metric_number(&candidate.metrics, key),
        metric_number(&baseline.metrics, key),
    ) {
        (Some(candidate), Some(baseline)) => candidate > baseline,
        _ => false,
    }
}

fn metric_less(candidate: &EvalResult, baseline: &EvalResult, key: &str) -> bool {
    match (
        metric_number(&candidate.metrics, key),
        metric_number(&baseline.metrics, key),
    ) {
        (Some(candidate), Some(baseline)) => candidate < baseline,
        _ => false,
    }
}

fn metric_increase_ratio(candidate: &EvalResult, baseline: &EvalResult, key: &str) -> Option<f64> {
    let candidate = metric_number(&candidate.metrics, key)?;
    let baseline = metric_number(&baseline.metrics, key)?;
    if baseline <= 0.0 {
        return None;
    }
    Some((candidate - baseline) / baseline)
}

fn metric_number(metrics: &Value, key: &str) -> Option<f64> {
    metrics
        .get(key)
        .and_then(Value::as_f64)
        .or_else(|| {
            metrics
                .get("state_metrics")
                .and_then(|state_metrics| state_metrics.get(key))
                .and_then(Value::as_f64)
        })
        .or_else(|| metric_u64(metrics, key).map(|value| value as f64))
}

fn metric_u64(metrics: &Value, key: &str) -> Option<u64> {
    metrics.get(key).and_then(value_as_u64).or_else(|| {
        metrics
            .get("state_metrics")
            .and_then(|state_metrics| state_metrics.get(key))
            .and_then(value_as_u64)
    })
}

fn value_as_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_i64().and_then(|v| u64::try_from(v).ok()))
}

fn token_total_less(candidate: &EvalResult, baseline: &EvalResult) -> bool {
    match (
        token_total(&candidate.metrics),
        token_total(&baseline.metrics),
    ) {
        (Some(candidate), Some(baseline)) => candidate < baseline,
        _ => false,
    }
}

fn token_total(metrics: &Value) -> Option<u64> {
    let input = metric_u64(metrics, "input_tokens")?;
    let output = metric_u64(metrics, "output_tokens")?;
    Some(input + output)
}

fn promotion_metric_evidence(baseline: &EvalResult, candidate: &EvalResult) -> Value {
    let metric_keys = [
        "cost_usd",
        "cost_per_successful_task_usd",
        "total_duration_ms",
        "latency_ms",
        "latency_per_successful_task_ms",
        "input_tokens",
        "output_tokens",
        "cache_hit_ratio",
        "model_calls",
        "tool_calls",
        "failures",
        "file_edits",
        "context_misses",
        "context_miss_rate",
        "repair_loop_count",
        "fixture_agent_attempts",
        "fixture_agent_mutation_scope_failures",
        "fixture_agent_mutation_scope_failure_rate",
        "fixture_agent_changed_file_count",
        "fixture_agent_unexpected_changed_file_count",
        "malformed_tool_call_rate",
        "json_parse_failure_rate",
        "json_output_failures",
        "tool_schema_failures",
        "permission_prompt_rate",
        "human_intervention_rate",
        "human_interventions",
        "thinking_mode_usage_rate",
        "fim_success_rate",
        "fim_compile_rate",
        "fim_rollback_rate",
        "fim_token_savings",
        "rollback_count",
        "rollback_rate",
    ];
    let metrics = metric_keys
        .iter()
        .map(|key| {
            let baseline_value = metric_number(&baseline.metrics, key);
            let candidate_value = metric_number(&candidate.metrics, key);
            json!({
                "metric": key,
                "baseline": baseline_value,
                "candidate": candidate_value,
                "delta": match (baseline_value, candidate_value) {
                    (Some(baseline), Some(candidate)) => Some(candidate - baseline),
                    _ => None,
                },
            })
        })
        .collect::<Vec<_>>();
    let mut evidence = json!({
        "baseline_eval_id": baseline.eval_id,
        "candidate_eval_id": candidate.eval_id,
        "baseline_score": baseline.score,
        "candidate_score": candidate.score,
        "baseline_passed": baseline.passed,
        "candidate_passed": candidate.passed,
        "baseline_failed": baseline.failed,
        "candidate_failed": candidate.failed,
        "token_total": {
            "baseline": token_total(&baseline.metrics),
            "candidate": token_total(&candidate.metrics),
            "delta": match (token_total(&baseline.metrics), token_total(&candidate.metrics)) {
                (Some(baseline), Some(candidate)) => Some(candidate as i128 - baseline as i128),
                _ => None,
            },
        },
        "metrics": metrics,
    });
    let fixture_suite = json!({
        "baseline": fixture_suite_promotion_evidence(baseline),
        "candidate": fixture_suite_promotion_evidence(candidate),
    });
    if fixture_suite.get("baseline").is_some_and(Value::is_object)
        || fixture_suite.get("candidate").is_some_and(Value::is_object)
    {
        evidence["fixture_suite"] = fixture_suite;
    }
    let model_route_tasks = json!({
        "baseline": model_route_task_promotion_evidence(baseline),
        "candidate": model_route_task_promotion_evidence(candidate),
    });
    if model_route_tasks
        .get("baseline")
        .is_some_and(|value| value.as_object().is_some_and(|map| !map.is_empty()))
        || model_route_tasks
            .get("candidate")
            .is_some_and(|value| value.as_object().is_some_and(|map| !map.is_empty()))
    {
        evidence["model_route_tasks"] = model_route_tasks;
    }
    evidence
}

fn model_route_task_promotion_evidence(eval: &EvalResult) -> Value {
    let Some(routes) = eval
        .metrics
        .get("state_metrics")
        .and_then(|metrics| metrics.get("model_route_tasks"))
        .and_then(Value::as_object)
    else {
        return json!({});
    };
    let routes = routes
        .iter()
        .filter_map(|(route, count)| value_as_u64(count).map(|count| (route.clone(), count)))
        .collect::<BTreeMap<_, _>>();
    json!(routes)
}

fn fixture_suite_promotion_evidence(eval: &EvalResult) -> Option<Value> {
    let fixture_suite = eval.metrics.get("fixture_suite")?;
    Some(json!({
        "task_count": fixture_suite.get("task_count").and_then(value_as_u64),
        "command_count": fixture_suite.get("command_count").and_then(value_as_u64),
        "categories": fixture_suite.get("categories").cloned(),
        "risk_labels": fixture_suite.get("risk_labels").cloned(),
    }))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ApplyWorktreeReport {
    worktree_path: PathBuf,
    patch_file: PathBuf,
    worktree_stdout: String,
    apply_stdout: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AppliedPatchLocation {
    worktree_path: PathBuf,
    patch_file: PathBuf,
}

fn apply_patch_in_worktree(
    patch_file: &Path,
    worktree: &Path,
    check_only: bool,
) -> Result<ApplyWorktreeReport, String> {
    validate_patch_file_paths(patch_file)?;
    let worktree_stdout = ensure_worktree(worktree)?;
    let mut args = vec![
        "-C".to_string(),
        worktree.display().to_string(),
        "apply".to_string(),
    ];
    if check_only {
        args.push("--check".to_string());
    }
    args.push(patch_file.display().to_string());
    let apply_output = run_git_owned(args)?;
    Ok(ApplyWorktreeReport {
        worktree_path: worktree.to_path_buf(),
        patch_file: patch_file.to_path_buf(),
        worktree_stdout: preview(&worktree_stdout, 2000),
        apply_stdout: preview(&apply_output, 2000),
    })
}

fn latest_applied_patch(events: &[Value], patch_id: &str) -> Option<AppliedPatchLocation> {
    events.iter().rev().find_map(|event| {
        if !event
            .get("event_type")
            .and_then(|v| v.as_str())
            .map(|kind| kind == "PatchApplied")
            .unwrap_or(false)
        {
            return None;
        }
        let payload = event.get("payload")?;
        if !payload
            .get("patch_id")
            .and_then(|v| v.as_str())
            .map(|candidate| candidate == patch_id)
            .unwrap_or(false)
        {
            return None;
        }
        Some(AppliedPatchLocation {
            worktree_path: PathBuf::from(payload.get("worktree_path")?.as_str()?),
            patch_file: PathBuf::from(payload.get("patch_file")?.as_str()?),
        })
    })
}

fn rollback_patch_in_worktree(
    patch_file: &Path,
    worktree: &Path,
) -> Result<ApplyWorktreeReport, String> {
    validate_patch_file_paths(patch_file)?;
    if !worktree.is_dir() {
        return Err(format!("worktree '{}' does not exist", worktree.display()));
    }
    let args = vec![
        "-C".to_string(),
        worktree.display().to_string(),
        "apply".to_string(),
        "-R".to_string(),
        patch_file.display().to_string(),
    ];
    let apply_output = run_git_owned(args)?;
    Ok(ApplyWorktreeReport {
        worktree_path: worktree.to_path_buf(),
        patch_file: patch_file.to_path_buf(),
        worktree_stdout: "existing worktree".to_string(),
        apply_stdout: preview(&apply_output, 2000),
    })
}

fn validate_patch_file_paths(patch_file: &Path) -> Result<usize, String> {
    let raw = std::fs::read_to_string(patch_file)
        .map_err(|e| format!("read patch file '{}': {e}", patch_file.display()))?;
    let mut checked = 0;
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            let mut parts = rest.split_whitespace();
            for target in [parts.next(), parts.next()].into_iter().flatten() {
                validate_patch_target_path(target)?;
                checked += 1;
            }
        } else if let Some(target) = line.strip_prefix("--- ") {
            if let Some(path) = target.split_whitespace().next() {
                validate_patch_target_path(path)?;
                checked += 1;
            }
        } else if let Some(target) = line.strip_prefix("+++ ") {
            if let Some(path) = target.split_whitespace().next() {
                validate_patch_target_path(path)?;
                checked += 1;
            }
        }
    }
    if checked == 0 {
        return Err(format!(
            "patch file '{}' has no diff path headers to validate",
            patch_file.display()
        ));
    }
    Ok(checked)
}

fn validate_patch_target_path(raw_target: &str) -> Result<(), String> {
    if raw_target == "/dev/null" {
        return Ok(());
    }
    let trimmed = raw_target.trim_matches('"');
    let target = trimmed
        .strip_prefix("a/")
        .or_else(|| trimmed.strip_prefix("b/"))
        .unwrap_or(trimmed);
    let path = Path::new(target);
    if target.is_empty() || path.is_absolute() {
        return Err(format!(
            "patch target '{raw_target}' is not a relative repository path"
        ));
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(format!(
            "patch target '{raw_target}' contains path traversal"
        ));
    }
    Ok(())
}

fn ensure_worktree(worktree: &Path) -> Result<String, String> {
    if worktree.exists() {
        if worktree.is_dir() {
            return Ok("existing worktree".to_string());
        }
        return Err(format!(
            "worktree path '{}' exists but is not a directory",
            worktree.display()
        ));
    }
    if let Some(parent) = worktree.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create worktree parent '{}': {e}", parent.display()))?;
    }
    run_git_owned(vec![
        "worktree".to_string(),
        "add".to_string(),
        "--detach".to_string(),
        worktree.display().to_string(),
        "HEAD".to_string(),
    ])
}

fn run_git_owned(args: Vec<String>) -> Result<String, String> {
    let output = Command::new("git")
        .args(&args)
        .output()
        .map_err(|e| format!("run git {}: {e}", args.join(" ")))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "git {} failed (exit {}): {}{}",
            args.join(" "),
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr).trim(),
            if output.stdout.is_empty() {
                String::new()
            } else {
                format!("\n{}", String::from_utf8_lossy(&output.stdout).trim())
            }
        ))
    }
}

fn default_worktree_path(patch_id: &str) -> PathBuf {
    PathBuf::from(".yoyo")
        .join("evolve")
        .join("worktrees")
        .join(sanitize_path_segment(patch_id))
}

fn sanitize_path_segment(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "patch".to_string()
    } else {
        sanitized
    }
}

fn build_apply_payload(
    patch_id: &str,
    report: &ApplyWorktreeReport,
    check_only: bool,
    patch: Option<&PatchSafetyMetadata>,
    applied_at_ms: u128,
) -> Value {
    json!({
        "patch_id": patch_id,
        "status": "applied_in_fork",
        "check_only": check_only,
        "kind": patch.map(|patch| enum_label(&patch.kind)),
        "risk_level": patch.map(|patch| enum_label(&patch.risk_level)),
        "rollback_plan": patch
            .map(|patch| patch.rollback_plan.clone())
            .unwrap_or_default(),
        "base_git_commit": current_git_commit(),
        "worktree_path": report.worktree_path.display().to_string(),
        "patch_file": report.patch_file.display().to_string(),
        "git_worktree_stdout": report.worktree_stdout,
        "git_apply_stdout": report.apply_stdout,
        "applied_at_ms": applied_at_ms,
    })
}

fn build_rollback_payload(
    patch_id: &str,
    report: &ApplyWorktreeReport,
    reason: &str,
    reverted_at_ms: u128,
) -> Value {
    json!({
        "patch_id": patch_id,
        "status": "reverted",
        "reason": reason,
        "worktree_path": report.worktree_path.display().to_string(),
        "patch_file": report.patch_file.display().to_string(),
        "git_apply_reverse_stdout": report.apply_stdout,
        "reverted_at_ms": reverted_at_ms,
    })
}

fn parse_kind(raw: &str) -> Result<HarnessPatchKind, String> {
    match normalize(raw).as_str() {
        "context_policy" | "context" => Ok(HarnessPatchKind::ContextPolicy),
        "prompt_policy" | "prompt" => Ok(HarnessPatchKind::PromptPolicy),
        "thinking_policy" | "thinking" => Ok(HarnessPatchKind::ThinkingPolicy),
        "model_routing_policy" | "model_routing" | "routing" => {
            Ok(HarnessPatchKind::ModelRoutingPolicy)
        }
        "tool_schema" | "tool" | "schema" => Ok(HarnessPatchKind::ToolSchema),
        "test_policy" | "test" => Ok(HarnessPatchKind::TestPolicy),
        "repair_policy" | "repair" => Ok(HarnessPatchKind::RepairPolicy),
        "memory_policy" | "memory" => Ok(HarnessPatchKind::MemoryPolicy),
        "permission_policy" | "permission" | "permissions" => {
            Ok(HarnessPatchKind::PermissionPolicy)
        }
        "shell_policy" | "shell" => Ok(HarnessPatchKind::ShellPolicy),
        "state_projection" | "state" => Ok(HarnessPatchKind::StateProjection),
        "eval" | "evaluation" => Ok(HarnessPatchKind::Eval),
        "transport" => Ok(HarnessPatchKind::Transport),
        "safety" => Ok(HarnessPatchKind::Safety),
        "other" => Ok(HarnessPatchKind::Other),
        _ => Err(format!(
            "unknown patch kind '{raw}' (expected context_policy, prompt_policy, thinking_policy, model_routing_policy, tool_schema, test_policy, repair_policy, memory_policy, permission_policy, shell_policy, state_projection, eval, transport, safety, other)"
        )),
    }
}

fn parse_patch_kind_list(raw: &str) -> Result<Vec<HarnessPatchKind>, String> {
    raw.trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|item| item.trim().trim_matches('"').trim_matches('\'').trim())
        .filter(|item| !item.is_empty())
        .map(parse_kind)
        .collect()
}

fn parse_risk(raw: &str) -> Result<HarnessPatchRisk, String> {
    match normalize(raw).as_str() {
        "low" => Ok(HarnessPatchRisk::Low),
        "medium" | "med" => Ok(HarnessPatchRisk::Medium),
        "high" => Ok(HarnessPatchRisk::High),
        "critical" => Ok(HarnessPatchRisk::Critical),
        _ => Err(format!(
            "unknown patch risk '{raw}' (expected low, medium, high, critical)"
        )),
    }
}

fn normalize(raw: &str) -> String {
    raw.trim().replace(['-', ' '], "_").to_ascii_lowercase()
}

fn flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a String> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|idx| args.get(idx + 1))
}

fn collect_values(args: &[String], flag: &str) -> Vec<String> {
    args.iter()
        .enumerate()
        .filter_map(|(idx, arg)| {
            if arg == flag {
                args.get(idx + 1).map(|value| value.trim().to_string())
            } else {
                None
            }
        })
        .filter(|value| !value.is_empty())
        .collect()
}

fn collect_split_values(args: &[String], flag: &str) -> Vec<String> {
    collect_values(args, flag)
        .into_iter()
        .flat_map(|value| {
            value
                .split(',')
                .map(|part| part.trim().to_string())
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn current_git_commit() -> Option<String> {
    Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|commit| !commit.is_empty())
}

fn current_git_root() -> Option<PathBuf> {
    Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()))
        .filter(|path| !path.as_os_str().is_empty())
}

fn same_filesystem_path(left: &Path, right: &Path) -> bool {
    comparable_path(left) == comparable_path(right)
}

fn comparable_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(path)
        }
    })
}

fn preview(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let mut out: String = trimmed.chars().take(max_chars).collect();
    out.push_str("\n...[truncated]");
    out
}

fn new_patch_id() -> String {
    format!("patch-{}-{}", now_ms(), std::process::id())
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn print_usage() {
    println!(
        "Usage: yoyo evolve harness <command>\n\n  propose (--intent <text> | --from-state <event-id|last-failure>) [--kind KIND] [--risk RISK] [--evidence EVENT[,EVENT]] [--expected-effect TEXT] [--eval-plan TEXT] [--rollback-plan TEXT] [--dry-run]\n  feedback --summary <text> [--details TEXT] [--kind KIND] [--risk RISK] [--evidence EVENT[,EVENT]] [--expected-effect TEXT] [--eval-plan TEXT] [--rollback-plan TEXT] [--dry-run]\n  issue --title <text> [--body TEXT] [--kind KIND] [--risk RISK] [--evidence EVENT[,EVENT]] [--expected-effect TEXT] [--eval-plan TEXT] [--rollback-plan TEXT] [--dry-run]\n  apply <patch-id> --patch-file PATH [--worktree PATH] [--check] [--dry-run] [--force]\n  rollback <patch-id> [--worktree PATH] [--patch-file PATH] [--reason TEXT] [--dry-run] [--force]\n  eval <patch-id> [--worktree PATH] [--dry-run] [--force]\n  approve <patch-id> [--reason TEXT]\n  promote <patch-id> [--baseline-eval EVAL] [--candidate-eval EVAL] [--approval-event EVENT[,EVENT]] [--reason TEXT] [--force]\n  reject <patch-id> [--reason TEXT]"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn event(kind: &str, payload: Value) -> Value {
        event_with_id("evt-1", kind, payload)
    }

    fn event_with_id(event_id: &str, kind: &str, payload: Value) -> Value {
        json!({
            "event_id": event_id,
            "event_type": kind,
            "schema_version": 1,
            "timestamp_ms": 10,
            "actor": "harness",
            "run_id": "run-1",
            "session_id": null,
            "trace_id": "trace-1",
            "parent_event_ids": [],
            "payload": payload,
        })
    }

    fn eval_payload(
        eval_id: &str,
        patch_id: Option<&str>,
        score: f64,
        passed: u64,
        failed: u64,
    ) -> Value {
        json!({
            "eval_id": eval_id,
            "harness_version": "h1",
            "patch_id": patch_id,
            "suite": "local-smoke",
            "status": if failed == 0 { "passed" } else { "failed" },
            "score": score,
            "passed": passed,
            "failed": failed,
            "metrics": {
                "total_duration_ms": 100,
                "reproducibility": {
                    "mode": "gates",
                    "git_dirty": false,
                    "commands": [
                        "cargo fmt --check",
                        "cargo check",
                        "cargo test --bin yyds -- --test-threads=1",
                        "cargo test --test integration -- --test-threads=1"
                    ]
                }
            },
            "failure_event_ids": [],
            "created_at_ms": 1
        })
    }

    #[test]
    fn proposal_builder_defaults_to_required_eval_gates() {
        let patch = build_harness_patch(PatchProposalOptions {
            patch_id: "patch-1".into(),
            kind: HarnessPatchKind::Eval,
            risk_level: HarnessPatchRisk::Low,
            base_harness_version: "ds-harness-genome-v1".into(),
            base_git_commit: Some("abc123".into()),
            intent: "measure candidate prompts".into(),
            evidence_event_ids: vec!["evt-failure".into()],
            expected_effects: vec!["fewer repair loops".into()],
            eval_plan: Vec::new(),
            rollback_plan: Vec::new(),
            default_eval_plan: vec!["cargo check".into(), "cargo test".into()],
            created_at_ms: 1,
        });

        assert_eq!(patch.patch_id, "patch-1");
        assert_eq!(patch.kind, HarnessPatchKind::Eval);
        assert_eq!(patch.status, HarnessPatchStatus::Proposed);
        assert_eq!(patch.eval_plan, vec!["cargo check", "cargo test"]);
        assert_eq!(patch.rollback_plan.len(), 1);
        assert_eq!(patch.evidence_event_ids, vec!["evt-failure"]);
    }

    #[test]
    fn improvement_intake_payload_preserves_patch_lifecycle_shape() {
        let payload = build_improvement_intake_payload(ImprovementIntakeOptions {
            source: IntakeSource::HumanFeedback,
            summary: "Context misses retry_state.rs too often".into(),
            details: Some("User observed repeated repairs without the failing file.".into()),
            patch_id: "patch-feedback-1".into(),
            kind: HarnessPatchKind::PromptPolicy,
            risk_level: HarnessPatchRisk::Medium,
            evidence_event_ids: vec!["evt-failure".into()],
            expected_effects: vec!["lower context miss rate".into()],
            eval_plan: vec!["cargo test context::tests::selects_failing_files_from_recent_state_events -- --nocapture".into()],
            rollback_plan: Vec::new(),
            created_at_ms: 42,
        });

        assert_eq!(payload["patch_id"], "patch-feedback-1");
        assert_eq!(payload["intake_source"], "human_feedback");
        assert_eq!(payload["intake_kind"], "feedback");
        assert_eq!(
            payload["intake_summary"],
            "Context misses retry_state.rs too often"
        );
        assert_eq!(payload["evidence_event_ids"][0], "evt-failure");
        assert_eq!(payload["expected_effects"][0], "lower context miss rate");

        let patch: HarnessPatch = serde_json::from_value(payload).unwrap();
        assert_eq!(patch.patch_id, "patch-feedback-1");
        assert_eq!(patch.kind, HarnessPatchKind::PromptPolicy);
        assert_eq!(patch.status, HarnessPatchStatus::Proposed);
        assert_eq!(patch.rollback_plan.len(), 1);
    }

    #[test]
    fn state_backed_proposal_defaults_use_failure_evidence_and_policy_kind() {
        let events = vec![event_with_id(
            "evt-context-miss",
            "FailureObserved",
            json!({
                "source": "context_selector",
                "error_preview": "context omitted src/retry_state.rs during repair"
            }),
        )];

        let defaults = build_state_backed_proposal_defaults(&events, "last-failure").unwrap();

        assert_eq!(defaults.kind, HarnessPatchKind::ContextPolicy);
        assert_eq!(defaults.risk_level, HarnessPatchRisk::Low);
        assert_eq!(defaults.evidence_event_ids, vec!["evt-context-miss"]);
        assert!(defaults.intent.contains("evt-context-miss"));
        assert!(defaults.intent.contains("context omitted"));
        assert!(defaults
            .expected_effects
            .iter()
            .any(|effect| effect.contains("reduce recurrence")));
        assert!(defaults
            .eval_plan
            .contains(&"yoyo eval replay --from-state --limit 1".to_string()));
        assert!(defaults
            .eval_plan
            .contains(&"yoyo eval run --suite protocol-deepseek".to_string()));
        assert!(defaults
            .rollback_plan
            .iter()
            .any(|step| step.contains("reject patch")));
    }

    #[test]
    fn state_backed_proposal_classifies_schema_and_json_failures() {
        let events = vec![
            event_with_id(
                "evt-json",
                "JsonOutputFailure",
                json!({"source": "deepseek_json", "error": "invalid JSON object"}),
            ),
            event_with_id(
                "evt-schema",
                "ToolSchemaFailure",
                json!({"tool_name": "propose_edit", "error": "missing required schema_version"}),
            ),
        ];

        let json_defaults = build_state_backed_proposal_defaults(&events, "evt-json").unwrap();
        let schema_defaults = build_state_backed_proposal_defaults(&events, "evt-schema").unwrap();

        assert_eq!(json_defaults.kind, HarnessPatchKind::PromptPolicy);
        assert_eq!(schema_defaults.kind, HarnessPatchKind::ToolSchema);
        assert_eq!(schema_defaults.evidence_event_ids, vec!["evt-schema"]);
    }

    #[test]
    fn self_filed_issue_intake_defaults_to_required_eval_gates() {
        let payload = build_improvement_intake_payload(ImprovementIntakeOptions {
            source: IntakeSource::SelfFiledIssue,
            summary: "Track tool repair churn".into(),
            details: None,
            patch_id: "patch-issue-1".into(),
            kind: HarnessPatchKind::Eval,
            risk_level: HarnessPatchRisk::Low,
            evidence_event_ids: Vec::new(),
            expected_effects: Vec::new(),
            eval_plan: Vec::new(),
            rollback_plan: Vec::new(),
            created_at_ms: 43,
        });

        assert_eq!(payload["intake_source"], "self_filed_improvement_issue");
        assert_eq!(payload["intake_kind"], "issue");
        assert!(payload["eval_plan"]
            .as_array()
            .map(|items| !items.is_empty())
            .unwrap_or(false));
        assert!(payload["eval_plan"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("yoyo eval run --suite protocol-deepseek")));
    }

    #[test]
    fn default_harness_patch_eval_plan_includes_protocol_eval() {
        let genome = crate::deepseek::DeepSeekHarnessGenome::default();
        let plan = default_harness_patch_eval_plan(&genome);

        for gate in &genome.test_policy.required_gates {
            assert!(plan.contains(gate));
        }
        assert!(plan.contains(&"yoyo eval run --suite protocol-deepseek".to_string()));
        assert!(plan.len() > genome.test_policy.required_gates.len());
    }

    #[test]
    fn latest_passed_eval_requires_matching_patch_and_status() {
        let events = vec![
            event(
                "PatchProposed",
                json!({"patch_id": "patch-1", "intent": "x"}),
            ),
            event(
                "PatchEvaluated",
                json!({
                    "eval_id": "eval-failed",
                    "harness_version": "h1",
                    "patch_id": "patch-1",
                    "suite": "local-smoke",
                    "status": "failed",
                    "score": 0.5,
                    "passed": 1,
                    "failed": 1,
                    "metrics": {},
                    "failure_event_ids": [],
                    "created_at_ms": 1
                }),
            ),
            event(
                "PatchEvaluated",
                json!({
                    "eval_id": "eval-passed",
                    "harness_version": "h1",
                    "patch_id": "patch-1",
                    "suite": "local-smoke",
                    "status": "passed",
                    "score": 1.0,
                    "passed": 2,
                    "failed": 0,
                    "metrics": {},
                    "failure_event_ids": [],
                    "created_at_ms": 2
                }),
            ),
        ];

        assert!(patch_exists(&events, "patch-1"));
        assert_eq!(
            latest_passed_eval(&events, "patch-1").as_deref(),
            Some("eval-passed")
        );
        assert_eq!(latest_passed_eval(&events, "patch-2"), None);
    }

    #[test]
    fn promotion_decision_accepts_score_improvement_over_baseline() {
        let events = vec![
            event("PatchEvaluated", eval_payload("eval-base", None, 0.5, 1, 1)),
            event(
                "PatchEvaluated",
                eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0),
            ),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(decision.eligible);
        assert_eq!(decision.criterion.as_deref(), Some("pass_rate_improved"));
        assert_eq!(decision.baseline_eval_id.as_deref(), Some("eval-base"));
        assert_eq!(
            decision.candidate_eval_id.as_deref(),
            Some("eval-candidate")
        );
        assert!(decision
            .metric_evidence
            .as_ref()
            .and_then(|evidence| evidence.get("protocol_eval"))
            .is_none());
    }

    #[test]
    fn promotion_decision_rejects_without_baseline() {
        let events = vec![event(
            "PatchEvaluated",
            eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0),
        )];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(!decision.eligible);
        assert_eq!(
            decision.reason,
            "no baseline eval found for candidate suite"
        );
        assert_eq!(
            decision.candidate_eval_id.as_deref(),
            Some("eval-candidate")
        );
    }

    #[test]
    fn promotion_decision_blocks_failed_latest_protocol_eval() {
        let mut protocol = eval_payload("eval-protocol", None, 0.5, 1, 1);
        protocol["suite"] = json!("protocol-deepseek");
        protocol["metrics"]["eval_type"] = json!("protocol");
        let events = vec![
            event("PatchEvaluated", eval_payload("eval-base", None, 0.5, 1, 1)),
            event(
                "PatchEvaluated",
                eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0),
            ),
            event("PatchEvaluated", protocol),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(!decision.eligible);
        assert_eq!(decision.reason, "latest protocol eval did not pass");
        assert_eq!(
            decision.candidate_eval_id.as_deref(),
            Some("eval-candidate")
        );
    }

    #[test]
    fn promotion_decision_blocks_missing_protocol_eval_when_patch_plan_requires_it() {
        let events = vec![
            event(
                "PatchProposed",
                json!({
                    "patch_id": "patch-1",
                    "eval_plan": ["cargo check", "yoyo eval run --suite protocol-deepseek"],
                }),
            ),
            event("PatchEvaluated", eval_payload("eval-base", None, 0.5, 1, 1)),
            event(
                "PatchEvaluated",
                eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0),
            ),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(!decision.eligible);
        assert_eq!(
            decision.reason,
            "patch eval plan requires protocol eval but none was found"
        );
        assert_eq!(
            decision.candidate_eval_id.as_deref(),
            Some("eval-candidate")
        );
    }

    #[test]
    fn promotion_decision_blocks_stale_protocol_eval_for_planned_protocol_gate() {
        let mut protocol = eval_payload("eval-protocol", None, 1.0, 2, 0);
        protocol["suite"] = json!("protocol-deepseek");
        protocol["metrics"]["eval_type"] = json!("protocol");
        protocol["created_at_ms"] = json!(5);
        let mut candidate = eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0);
        candidate["created_at_ms"] = json!(10);
        let events = vec![
            event(
                "PatchProposed",
                json!({
                    "patch_id": "patch-1",
                    "eval_plan": ["cargo check", "yoyo eval run --suite protocol-deepseek"],
                }),
            ),
            event("PatchEvaluated", eval_payload("eval-base", None, 0.5, 1, 1)),
            event("PatchEvaluated", protocol),
            event("PatchEvaluated", candidate),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(!decision.eligible);
        assert_eq!(
            decision.reason,
            "latest protocol eval is older than candidate eval"
        );
        assert_eq!(
            decision.candidate_eval_id.as_deref(),
            Some("eval-candidate")
        );
    }

    #[test]
    fn promotion_decision_blocks_dirty_protocol_eval_for_planned_protocol_gate() {
        let mut protocol = eval_payload("eval-protocol-dirty", None, 1.0, 2, 0);
        protocol["suite"] = json!("protocol-deepseek");
        protocol["metrics"]["eval_type"] = json!("protocol");
        protocol["metrics"]["reproducibility"] = json!({
            "git_dirty": true,
            "git_status_short": [" M src/deepseek.rs"],
            "replay_command": "yoyo eval run --suite protocol-deepseek"
        });
        protocol["metrics"]["state_metrics"] = json!({
            "deepseek_protocol_checks": 5,
            "deepseek_protocol_passes": 5,
            "deepseek_strict_tool_call_checks": 1,
            "deepseek_thinking_protocol_checks": 1,
            "deepseek_streaming_protocol_checks": 1,
            "deepseek_json_output_checks": 1,
            "deepseek_transport_policy_checks": 1
        });
        protocol["created_at_ms"] = json!(10);
        let mut candidate = eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0);
        candidate["created_at_ms"] = json!(5);
        let events = vec![
            event(
                "PatchProposed",
                json!({
                    "patch_id": "patch-1",
                    "eval_plan": ["cargo check", "yoyo eval run --suite protocol-deepseek"],
                }),
            ),
            event("PatchEvaluated", eval_payload("eval-base", None, 0.5, 1, 1)),
            event("PatchEvaluated", protocol),
            event("PatchEvaluated", candidate),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(!decision.eligible);
        assert_eq!(
            decision.reason,
            "latest protocol eval was run from a dirty worktree"
        );
        assert_eq!(
            decision.protocol_eval_id.as_deref(),
            Some("eval-protocol-dirty")
        );
        assert_eq!(
            decision.candidate_eval_id.as_deref(),
            Some("eval-candidate")
        );
        let protocol_evidence = decision
            .metric_evidence
            .as_ref()
            .and_then(|evidence| evidence.get("protocol_eval"))
            .expect("protocol eval evidence");
        assert_eq!(protocol_evidence["eval_id"], "eval-protocol-dirty");
        assert_eq!(protocol_evidence["status"], "passed");
        assert_eq!(protocol_evidence["git_dirty"], true);
        assert_eq!(protocol_evidence["created_at_ms"], 10);
        assert_eq!(protocol_evidence["protocol_checks"]["total"], 5);
        assert_eq!(protocol_evidence["protocol_checks"]["passes"], 5);
        assert_eq!(protocol_evidence["protocol_checks"]["stream"], 1);
    }

    #[test]
    fn promotion_decision_allows_passing_latest_protocol_eval() {
        let mut protocol = eval_payload("eval-protocol", None, 1.0, 2, 0);
        protocol["suite"] = json!("protocol-deepseek");
        protocol["metrics"]["eval_type"] = json!("protocol");
        protocol["created_at_ms"] = json!(10);
        let mut candidate = eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0);
        candidate["created_at_ms"] = json!(5);
        let events = vec![
            event(
                "PatchProposed",
                json!({
                    "patch_id": "patch-1",
                    "eval_plan": ["cargo check", "yoyo eval run --suite protocol-deepseek"],
                }),
            ),
            event("PatchEvaluated", eval_payload("eval-base", None, 0.5, 1, 1)),
            event("PatchEvaluated", protocol),
            event("PatchEvaluated", candidate),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(decision.eligible);
        assert_eq!(decision.criterion.as_deref(), Some("pass_rate_improved"));
    }

    #[test]
    fn promotion_decision_blocks_dirty_candidate_eval_evidence() {
        let mut candidate = eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0);
        candidate["metrics"]["reproducibility"] = json!({
            "git_dirty": true,
            "git_status_short": [" M src/commands_evolve.rs"],
            "replay_command": "yoyo eval run --suite local-smoke"
        });
        let events = vec![
            event("PatchEvaluated", eval_payload("eval-base", None, 0.5, 1, 1)),
            event("PatchEvaluated", candidate),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(!decision.eligible);
        assert_eq!(
            decision.reason,
            "candidate eval was run from a dirty worktree"
        );
        assert_eq!(
            decision.candidate_eval_id.as_deref(),
            Some("eval-candidate")
        );
    }

    #[test]
    fn promotion_decision_blocks_missing_required_gate_evidence() {
        let mut candidate = eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0);
        candidate["metrics"]["reproducibility"] = json!({
            "git_dirty": false,
            "commands": [
                "cargo fmt --check",
                "cargo test --bin yyds -- --test-threads=1",
                "cargo test --test integration -- --test-threads=1"
            ]
        });
        let events = vec![
            event("PatchEvaluated", eval_payload("eval-base", None, 0.5, 1, 1)),
            event("PatchEvaluated", candidate),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(!decision.eligible);
        assert!(decision
            .reason
            .contains("candidate eval is missing required gate evidence"));
        assert!(decision.reason.contains("cargo check"));
    }

    #[test]
    fn promotion_decision_blocks_dirty_baseline_eval_evidence() {
        let mut baseline = eval_payload("eval-base", None, 0.5, 1, 1);
        baseline["metrics"]["reproducibility"] = json!({
            "git_dirty": true,
            "git_status_short": [" M src/context.rs"],
            "replay_command": "yoyo eval run --suite local-smoke"
        });
        let events = vec![
            event("PatchEvaluated", baseline),
            event(
                "PatchEvaluated",
                eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0),
            ),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(!decision.eligible);
        assert_eq!(
            decision.reason,
            "baseline eval was run from a dirty worktree"
        );
        assert_eq!(decision.baseline_eval_id.as_deref(), Some("eval-base"));
        assert_eq!(
            decision.candidate_eval_id.as_deref(),
            Some("eval-candidate")
        );
    }

    #[test]
    fn promotion_decision_blocks_missing_baseline_required_gate_evidence() {
        let mut baseline = eval_payload("eval-base", None, 0.5, 1, 1);
        baseline["metrics"]["reproducibility"] = json!({
            "git_dirty": false,
            "commands": [
                "cargo fmt --check",
                "cargo test --bin yyds -- --test-threads=1",
                "cargo test --test integration -- --test-threads=1"
            ]
        });
        let events = vec![
            event("PatchEvaluated", baseline),
            event(
                "PatchEvaluated",
                eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0),
            ),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(!decision.eligible);
        assert!(decision
            .reason
            .contains("baseline eval is missing required gate evidence"));
        assert!(decision.reason.contains("cargo check"));
        assert_eq!(decision.baseline_eval_id.as_deref(), Some("eval-base"));
        assert_eq!(
            decision.candidate_eval_id.as_deref(),
            Some("eval-candidate")
        );
    }

    #[test]
    fn promotion_decision_blocks_fixture_suite_breadth_mismatch() {
        let mut baseline = eval_payload("eval-base", None, 1.0, 2, 0);
        baseline["metrics"]["fixture_suite"] = json!({
            "task_count": 242,
            "command_count": 484
        });
        let mut candidate = eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0);
        candidate["metrics"]["fixture_suite"] = json!({
            "task_count": 243,
            "command_count": 486
        });
        candidate["metrics"]["state_metrics"] = json!({
            "cost_usd": 0.10
        });
        baseline["metrics"]["state_metrics"] = json!({
            "cost_usd": 0.20
        });
        let events = vec![
            event("PatchEvaluated", baseline),
            event("PatchEvaluated", candidate),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(!decision.eligible);
        assert_eq!(
            decision.reason,
            "baseline and candidate fixture suite breadth differ"
        );
        assert_eq!(decision.baseline_eval_id.as_deref(), Some("eval-base"));
        assert_eq!(
            decision.candidate_eval_id.as_deref(),
            Some("eval-candidate")
        );
        let evidence = decision.metric_evidence.as_ref().unwrap();
        assert_eq!(
            evidence["fixture_suite"]["baseline"]["task_count"],
            json!(242)
        );
        assert_eq!(
            evidence["fixture_suite"]["candidate"]["task_count"],
            json!(243)
        );
    }

    #[test]
    fn promotion_decision_blocks_fixture_suite_risk_label_mismatch() {
        let mut baseline = eval_payload("eval-base", None, 1.0, 2, 0);
        baseline["metrics"]["fixture_suite"] = json!({
            "task_count": 243,
            "command_count": 486,
            "risk_labels": {
                "high": 4,
                "medium": 116,
                "low": 123
            }
        });
        baseline["metrics"]["state_metrics"] = json!({
            "cost_usd": 0.20
        });
        let mut candidate = eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0);
        candidate["metrics"]["fixture_suite"] = json!({
            "task_count": 243,
            "command_count": 486,
            "risk_labels": {
                "high": 3,
                "medium": 117,
                "low": 123
            }
        });
        candidate["metrics"]["state_metrics"] = json!({
            "cost_usd": 0.10
        });
        let events = vec![
            event("PatchEvaluated", baseline),
            event("PatchEvaluated", candidate),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(!decision.eligible);
        assert_eq!(
            decision.reason,
            "baseline and candidate fixture suite risk-label coverage differ"
        );
        assert_eq!(decision.baseline_eval_id.as_deref(), Some("eval-base"));
        assert_eq!(
            decision.candidate_eval_id.as_deref(),
            Some("eval-candidate")
        );
        let evidence = decision.metric_evidence.as_ref().unwrap();
        assert_eq!(
            evidence["fixture_suite"]["baseline"]["risk_labels"]["high"],
            json!(4)
        );
        assert_eq!(
            evidence["fixture_suite"]["candidate"]["risk_labels"]["high"],
            json!(3)
        );
    }

    #[test]
    fn promotion_decision_uses_explicit_eval_ids() {
        let events = vec![
            event(
                "PatchEvaluated",
                eval_payload("eval-old-base", None, 0.25, 1, 3),
            ),
            event("PatchEvaluated", eval_payload("eval-base", None, 1.0, 2, 0)),
            event(
                "PatchEvaluated",
                eval_payload("eval-other-patch", Some("patch-2"), 1.0, 2, 0),
            ),
            event(
                "PatchEvaluated",
                eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0),
            ),
        ];

        let decision = promotion_decision(
            &events,
            "patch-1",
            Some("eval-old-base"),
            Some("eval-candidate"),
        );

        assert!(decision.eligible);
        assert_eq!(decision.criterion.as_deref(), Some("pass_rate_improved"));
        assert_eq!(decision.baseline_eval_id.as_deref(), Some("eval-old-base"));
        assert_eq!(
            decision.candidate_eval_id.as_deref(),
            Some("eval-candidate")
        );
    }

    #[test]
    fn promotion_evidence_lines_show_eval_ids_scores_and_decision_before_promotion() {
        let decision = PromotionDecision {
            eligible: true,
            criterion: Some("pass_rate_improved".into()),
            reason: "candidate pass rate improved without regression".into(),
            baseline_eval_id: Some("eval-base".into()),
            candidate_eval_id: Some("eval-candidate".into()),
            protocol_eval_id: None,
            suite: Some("local-smoke".into()),
            baseline_score: Some(0.7),
            candidate_score: Some(0.9),
            baseline_passed: Some(7),
            candidate_passed: Some(9),
            baseline_failed: Some(3),
            candidate_failed: Some(1),
            metric_evidence: None,
        };

        let lines = promotion_evidence_lines(&decision, false);

        assert!(lines[0].contains("candidate eval: eval-candidate suite=local-smoke score=0.900"));
        assert!(lines[1].contains("baseline eval:  eval-base score=0.700"));
        assert!(lines[2].contains("decision: eligible criterion=pass_rate_improved forced=false"));
        assert!(lines[3].contains("candidate pass rate improved without regression"));
    }

    #[test]
    fn promotion_evidence_lines_show_fixture_suite_breadth_before_promotion() {
        let decision = PromotionDecision {
            eligible: true,
            criterion: Some("cost_reduced_no_regression".into()),
            reason: "candidate cost is lower with no pass-rate regression".into(),
            baseline_eval_id: Some("eval-base".into()),
            candidate_eval_id: Some("eval-candidate".into()),
            protocol_eval_id: None,
            suite: Some("local-smoke".into()),
            baseline_score: Some(1.0),
            candidate_score: Some(1.0),
            baseline_passed: Some(10),
            candidate_passed: Some(10),
            baseline_failed: Some(0),
            candidate_failed: Some(0),
            metric_evidence: Some(json!({
                "fixture_suite": {
                    "baseline": {
                        "task_count": 243,
                        "command_count": 486,
                        "risk_labels": {
                            "high": 4,
                            "medium": 116,
                            "low": 123
                        }
                    },
                    "candidate": {
                        "task_count": 243,
                        "command_count": 486,
                        "risk_labels": {
                            "high": 4,
                            "medium": 116,
                            "low": 123
                        }
                    }
                }
            })),
        };

        let lines = promotion_evidence_lines(&decision, false);

        assert!(lines[2].contains(
            "fixture suite: baseline tasks=243 commands=486 risks=[high=4, low=123, medium=116] candidate tasks=243 commands=486 risks=[high=4, low=123, medium=116]"
        ));
        assert!(lines[3].contains("decision: eligible criterion=cost_reduced_no_regression"));
    }

    #[test]
    fn promotion_evidence_lines_show_model_route_distribution_before_promotion() {
        let decision = PromotionDecision {
            eligible: true,
            criterion: Some("cost_reduced_no_regression".into()),
            reason: "candidate cost is lower with no pass-rate regression".into(),
            baseline_eval_id: Some("eval-base".into()),
            candidate_eval_id: Some("eval-candidate".into()),
            protocol_eval_id: None,
            suite: Some("local-smoke".into()),
            baseline_score: Some(1.0),
            candidate_score: Some(1.0),
            baseline_passed: Some(10),
            candidate_passed: Some(10),
            baseline_failed: Some(0),
            candidate_failed: Some(0),
            metric_evidence: Some(json!({
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
            })),
        };

        let lines = promotion_evidence_lines(&decision, false);

        assert!(lines[2].contains(
            "model routes: baseline [memory_compression=1, root_cause=3] candidate [fim=1, root_cause=2]"
        ));
        assert!(lines[3].contains("decision: eligible criterion=cost_reduced_no_regression"));
    }

    #[test]
    fn promotion_evidence_lines_show_protocol_eval_before_promotion() {
        let decision = PromotionDecision {
            eligible: true,
            criterion: Some("pass_rate_improved".into()),
            reason: "candidate pass rate improved and protocol eval passed".into(),
            baseline_eval_id: Some("eval-base".into()),
            candidate_eval_id: Some("eval-candidate".into()),
            protocol_eval_id: Some("eval-protocol".into()),
            suite: Some("local-smoke".into()),
            baseline_score: Some(0.8),
            candidate_score: Some(0.9),
            baseline_passed: Some(8),
            candidate_passed: Some(9),
            baseline_failed: Some(2),
            candidate_failed: Some(1),
            metric_evidence: Some(json!({
                "protocol_eval": {
                    "eval_id": "eval-protocol",
                    "suite": "protocol-deepseek",
                    "status": "passed",
                    "created_at_ms": 10,
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
                }
            })),
        };

        let lines = promotion_evidence_lines(&decision, false);

        assert!(lines[0].contains("candidate eval: eval-candidate"));
        assert!(lines[1].contains("baseline eval:  eval-base"));
        assert!(lines[2].contains(
            "protocol eval:  eval-protocol status=passed dirty=no created_at_ms=10 checks=5/5 strict=1 thinking=1 stream=1 json=1 transport=1"
        ));
        assert!(lines[3].contains("decision: eligible criterion=pass_rate_improved forced=false"));
    }

    #[test]
    fn promotion_decision_rejects_passed_candidate_without_improvement_metric() {
        let events = vec![
            event("PatchEvaluated", eval_payload("eval-base", None, 1.0, 2, 0)),
            event(
                "PatchEvaluated",
                eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0),
            ),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(!decision.eligible);
        assert_eq!(
            decision.reason,
            "candidate passed but no promotion metric improved"
        );
    }

    #[test]
    fn promotion_decision_accepts_cost_reduction_from_state_metrics() {
        let mut baseline = eval_payload("eval-base", None, 1.0, 2, 0);
        baseline["metrics"]["state_metrics"] = json!({
            "cost_usd": 0.20,
            "input_tokens": 100,
            "output_tokens": 20,
            "model_route_tasks": {
                "root_cause": 2
            }
        });
        baseline["metrics"]["fixture_suite"] = json!({
            "task_count": 238,
            "command_count": 476,
            "categories": {"state/graph": 12},
            "risk_labels": {"low": 200}
        });
        let mut candidate = eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0);
        candidate["metrics"]["state_metrics"] = json!({
            "cost_usd": 0.10,
            "input_tokens": 110,
            "output_tokens": 20,
            "model_route_tasks": {
                "memory_compression": 1,
                "root_cause": 1
            }
        });
        candidate["metrics"]["fixture_suite"] = json!({
            "task_count": 238,
            "command_count": 476,
            "categories": {"state/graph": 13},
            "risk_labels": {"low": 200}
        });
        let events = vec![
            event("PatchEvaluated", baseline),
            event("PatchEvaluated", candidate),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(decision.eligible);
        assert_eq!(
            decision.criterion.as_deref(),
            Some("cost_reduced_no_regression")
        );
        let evidence = decision.metric_evidence.as_ref().unwrap();
        assert_eq!(evidence["baseline_eval_id"], "eval-base");
        assert_eq!(evidence["candidate_eval_id"], "eval-candidate");
        assert_eq!(evidence["token_total"]["baseline"], 120);
        assert_eq!(evidence["token_total"]["candidate"], 130);
        assert_eq!(
            evidence["fixture_suite"]["baseline"]["task_count"],
            json!(238)
        );
        assert_eq!(
            evidence["fixture_suite"]["candidate"]["command_count"],
            json!(476)
        );
        assert_eq!(
            evidence["fixture_suite"]["candidate"]["categories"]["state/graph"],
            json!(13)
        );
        assert_eq!(
            evidence["model_route_tasks"]["baseline"]["root_cause"],
            json!(2)
        );
        assert_eq!(
            evidence["model_route_tasks"]["candidate"]["memory_compression"],
            json!(1)
        );
        assert!(evidence["metrics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|metric| metric["metric"] == "cost_usd"
                && metric["baseline"] == 0.20
                && metric["candidate"] == 0.10));
    }

    #[test]
    fn promotion_decision_accepts_derived_cost_and_latency_reductions() {
        let mut baseline = eval_payload("eval-base", None, 1.0, 2, 0);
        baseline["metrics"]["state_metrics"] = json!({
            "cost_per_successful_task_usd": 0.20,
            "latency_per_successful_task_ms": 2000
        });
        let mut candidate = eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0);
        candidate["metrics"]["state_metrics"] = json!({
            "cost_per_successful_task_usd": 0.10,
            "latency_per_successful_task_ms": 2500
        });
        let events = vec![
            event("PatchEvaluated", baseline),
            event("PatchEvaluated", candidate),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(decision.eligible);
        assert_eq!(
            decision.criterion.as_deref(),
            Some("cost_reduced_no_regression")
        );

        let mut baseline = eval_payload("eval-base", None, 1.0, 2, 0);
        baseline["metrics"]["state_metrics"] = json!({
            "latency_per_successful_task_ms": 2000
        });
        let mut candidate = eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0);
        candidate["metrics"]["state_metrics"] = json!({
            "latency_per_successful_task_ms": 1000
        });
        let events = vec![
            event("PatchEvaluated", baseline),
            event("PatchEvaluated", candidate),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(decision.eligible);
        assert_eq!(
            decision.criterion.as_deref(),
            Some("duration_reduced_no_regression")
        );
    }

    #[test]
    fn promotion_decision_accepts_nested_cache_ratio_improvement() {
        let mut baseline = eval_payload("eval-base", None, 1.0, 2, 0);
        baseline["metrics"]["state_metrics"] = json!({
            "cache_hit_ratio": 0.40
        });
        let mut candidate = eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0);
        candidate["metrics"]["state_metrics"] = json!({
            "cache_hit_ratio": 0.65
        });
        let events = vec![
            event("PatchEvaluated", baseline),
            event("PatchEvaluated", candidate),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(decision.eligible);
        assert_eq!(decision.criterion.as_deref(), Some("cache_ratio_improved"));
    }

    #[test]
    fn promotion_decision_accepts_harness_quality_metric_improvements() {
        let mut baseline = eval_payload("eval-base", None, 1.0, 2, 0);
        baseline["metrics"]["state_metrics"] = json!({
            "malformed_tool_call_rate": 0.50,
            "json_parse_failure_rate": 0.25,
            "repair_loop_count": 3,
            "fim_success_rate": 0.50,
            "permission_prompt_rate": 0.50
        });
        let mut candidate = eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0);
        candidate["metrics"]["state_metrics"] = json!({
            "malformed_tool_call_rate": 0.10,
            "json_parse_failure_rate": 0.25,
            "repair_loop_count": 3,
            "fim_success_rate": 0.50,
            "permission_prompt_rate": 0.50
        });
        let events = vec![
            event("PatchEvaluated", baseline.clone()),
            event("PatchEvaluated", candidate.clone()),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(decision.eligible);
        assert_eq!(
            decision.criterion.as_deref(),
            Some("tool_reliability_improved")
        );
        let evidence = decision.metric_evidence.as_ref().unwrap();
        assert!(evidence["metrics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|metric| {
                metric["metric"] == "malformed_tool_call_rate"
                    && metric["baseline"] == 0.50
                    && metric["candidate"] == 0.10
            }));

        candidate["metrics"]["state_metrics"] = json!({
            "malformed_tool_call_rate": 0.50,
            "json_parse_failure_rate": 0.05,
            "repair_loop_count": 3,
            "fim_success_rate": 0.50,
            "permission_prompt_rate": 0.50
        });
        let events = vec![
            event("PatchEvaluated", baseline.clone()),
            event("PatchEvaluated", candidate.clone()),
        ];
        let decision = promotion_decision(&events, "patch-1", None, None);
        assert!(decision.eligible);
        assert_eq!(
            decision.criterion.as_deref(),
            Some("json_reliability_improved")
        );

        candidate["metrics"]["state_metrics"] = json!({
            "malformed_tool_call_rate": 0.50,
            "json_parse_failure_rate": 0.25,
            "repair_loop_count": 1,
            "fim_success_rate": 0.50,
            "permission_prompt_rate": 0.50
        });
        let events = vec![
            event("PatchEvaluated", baseline.clone()),
            event("PatchEvaluated", candidate.clone()),
        ];
        let decision = promotion_decision(&events, "patch-1", None, None);
        assert!(decision.eligible);
        assert_eq!(decision.criterion.as_deref(), Some("repair_loop_reduced"));

        candidate["metrics"]["state_metrics"] = json!({
            "malformed_tool_call_rate": 0.50,
            "json_parse_failure_rate": 0.25,
            "repair_loop_count": 3,
            "fim_success_rate": 0.90,
            "permission_prompt_rate": 0.50
        });
        let events = vec![
            event("PatchEvaluated", baseline.clone()),
            event("PatchEvaluated", candidate.clone()),
        ];
        let decision = promotion_decision(&events, "patch-1", None, None);
        assert!(decision.eligible);
        assert_eq!(decision.criterion.as_deref(), Some("fim_quality_improved"));

        candidate["metrics"]["state_metrics"] = json!({
            "malformed_tool_call_rate": 0.50,
            "json_parse_failure_rate": 0.25,
            "repair_loop_count": 3,
            "fim_success_rate": 0.50,
            "permission_prompt_rate": 0.10
        });
        let events = vec![
            event("PatchEvaluated", baseline),
            event("PatchEvaluated", candidate),
        ];
        let decision = promotion_decision(&events, "patch-1", None, None);
        assert!(decision.eligible);
        assert_eq!(
            decision.criterion.as_deref(),
            Some("intervention_pressure_reduced")
        );
    }

    #[test]
    fn promotion_decision_uses_fixture_agent_mutation_scope_metrics() {
        let mut baseline = eval_payload("eval-base", None, 1.0, 2, 0);
        baseline["metrics"]["state_metrics"] = json!({
            "cost_usd": 0.20,
            "fixture_agent_mutation_scope_failures": 2,
            "fixture_agent_unexpected_changed_file_count": 3,
            "fixture_agent_mutation_scope_failure_rate": 0.50
        });
        let mut candidate = eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0);
        candidate["metrics"]["state_metrics"] = json!({
            "cost_usd": 0.20,
            "fixture_agent_mutation_scope_failures": 0,
            "fixture_agent_unexpected_changed_file_count": 0,
            "fixture_agent_mutation_scope_failure_rate": 0.0
        });
        let events = vec![
            event("PatchEvaluated", baseline.clone()),
            event("PatchEvaluated", candidate.clone()),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(decision.eligible);
        assert_eq!(
            decision.criterion.as_deref(),
            Some("mutation_scope_improved")
        );
        assert_eq!(
            decision.reason,
            "candidate fixture agent mutation scope failures are lower with no pass-rate regression"
        );
        let evidence = decision.metric_evidence.as_ref().unwrap();
        assert!(evidence["metrics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|metric| {
                metric["metric"] == "fixture_agent_mutation_scope_failures"
                    && metric["baseline"] == 2.0
                    && metric["candidate"] == 0.0
            }));

        baseline["metrics"]["state_metrics"] = json!({
            "cost_usd": 0.20,
            "fixture_agent_mutation_scope_failures": 0,
            "fixture_agent_unexpected_changed_file_count": 0,
            "fixture_agent_mutation_scope_failure_rate": 0.0
        });
        candidate["metrics"]["state_metrics"] = json!({
            "cost_usd": 0.10,
            "fixture_agent_mutation_scope_failures": 1,
            "fixture_agent_unexpected_changed_file_count": 1,
            "fixture_agent_mutation_scope_failure_rate": 0.50
        });
        let events = vec![
            event("PatchEvaluated", baseline),
            event("PatchEvaluated", candidate),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(!decision.eligible);
        assert_eq!(decision.criterion, None);
        assert_eq!(
            decision.reason,
            "candidate fixture_agent_mutation_scope_failures regresses harness quality gate"
        );
    }

    #[test]
    fn promotion_decision_accepts_log_feedback_metric_improvements() {
        let mut baseline = eval_payload("eval-base", None, 1.0, 2, 0);
        baseline["metrics"]["state_metrics"] = json!({
            "coding_log_score": 0.70,
            "workflow_success_rate": 1.0,
            "session_success_rate": 1.0,
            "task_success_rate": 0.80,
            "recurring_failure_count": 2
        });
        let mut candidate = eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0);
        candidate["metrics"]["state_metrics"] = json!({
            "coding_log_score": 0.85,
            "workflow_success_rate": 1.0,
            "session_success_rate": 1.0,
            "task_success_rate": 0.80,
            "recurring_failure_count": 2
        });
        let events = vec![
            event("PatchEvaluated", baseline.clone()),
            event("PatchEvaluated", candidate.clone()),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(decision.eligible);
        assert_eq!(decision.criterion.as_deref(), Some("log_feedback_improved"));

        candidate["metrics"]["state_metrics"] = json!({
            "coding_log_score": 0.70,
            "workflow_success_rate": 1.0,
            "session_success_rate": 1.0,
            "task_success_rate": 0.80,
            "recurring_failure_count": 0
        });
        let events = vec![
            event("PatchEvaluated", baseline),
            event("PatchEvaluated", candidate),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(decision.eligible);
        assert_eq!(
            decision.criterion.as_deref(),
            Some("log_feedback_reliability_improved")
        );

        let mut baseline = eval_payload("eval-base", None, 1.0, 2, 0);
        baseline["metrics"]["state_metrics"] = json!({
            "coding_log_score": 0.25,
            "workflow_success_rate": 0.0,
            "session_success_rate": 0.0,
            "provider_error_count": 6,
            "provider_blocked_session_count": 1
        });
        let mut candidate = eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0);
        candidate["metrics"]["state_metrics"] = json!({
            "coding_log_score": 0.25,
            "workflow_success_rate": 0.0,
            "session_success_rate": 0.0,
            "provider_error_count": 0,
            "provider_blocked_session_count": 0
        });
        let events = vec![
            event("PatchEvaluated", baseline),
            event("PatchEvaluated", candidate),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(decision.eligible);
        assert_eq!(
            decision.criterion.as_deref(),
            Some("log_feedback_reliability_improved")
        );
    }

    #[test]
    fn promotion_decision_blocks_log_feedback_regressions() {
        let mut baseline = eval_payload("eval-base", None, 1.0, 2, 0);
        baseline["metrics"]["state_metrics"] = json!({
            "coding_log_score": 0.90,
            "state_capture_coverage": 1.0,
            "audit_capture_coverage": 1.0,
            "recurring_failure_count": 0
        });
        let mut candidate = eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0);
        candidate["metrics"]["state_metrics"] = json!({
            "cost_usd": 0.10,
            "coding_log_score": 0.60,
            "state_capture_coverage": 1.0,
            "audit_capture_coverage": 1.0,
            "recurring_failure_count": 0
        });
        let events = vec![
            event("PatchEvaluated", baseline.clone()),
            event("PatchEvaluated", candidate.clone()),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(!decision.eligible);
        assert_eq!(
            decision.reason,
            "candidate coding_log_score regresses harness quality gate"
        );

        candidate["metrics"]["state_metrics"] = json!({
            "cost_usd": 0.10,
            "coding_log_score": 0.90,
            "state_capture_coverage": 1.0,
            "audit_capture_coverage": 1.0,
            "recurring_failure_count": 3
        });
        let events = vec![
            event("PatchEvaluated", baseline),
            event("PatchEvaluated", candidate),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(!decision.eligible);
        assert_eq!(
            decision.reason,
            "candidate recurring_failure_count regresses harness quality gate"
        );

        let mut baseline = eval_payload("eval-base", None, 1.0, 2, 0);
        baseline["metrics"]["state_metrics"] = json!({
            "coding_log_score": 0.90,
            "state_capture_coverage": 1.0,
            "audit_capture_coverage": 1.0,
            "provider_error_count": 0,
            "provider_blocked_session_count": 0
        });
        let mut candidate = eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0);
        candidate["metrics"]["state_metrics"] = json!({
            "cost_usd": 0.10,
            "coding_log_score": 0.90,
            "state_capture_coverage": 1.0,
            "audit_capture_coverage": 1.0,
            "provider_error_count": 2,
            "provider_blocked_session_count": 1
        });
        let events = vec![
            event("PatchEvaluated", baseline),
            event("PatchEvaluated", candidate),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(!decision.eligible);
        assert_eq!(
            decision.reason,
            "candidate provider_error_count regresses harness quality gate"
        );
    }

    #[test]
    fn promotion_decision_blocks_harness_quality_regression() {
        let mut baseline = eval_payload("eval-base", None, 1.0, 2, 0);
        baseline["metrics"]["state_metrics"] = json!({
            "cost_usd": 0.20,
            "malformed_tool_call_rate": 0.10,
            "json_parse_failure_rate": 0.05,
            "repair_loop_count": 1,
            "fim_success_rate": 0.90,
            "permission_prompt_rate": 0.10
        });
        let mut candidate = eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0);
        candidate["metrics"]["state_metrics"] = json!({
            "cost_usd": 0.10,
            "malformed_tool_call_rate": 0.30,
            "json_parse_failure_rate": 0.05,
            "repair_loop_count": 1,
            "fim_success_rate": 0.90,
            "permission_prompt_rate": 0.10
        });
        let events = vec![
            event("PatchEvaluated", baseline.clone()),
            event("PatchEvaluated", candidate.clone()),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(!decision.eligible);
        assert_eq!(decision.criterion, None);
        assert_eq!(
            decision.reason,
            "candidate malformed_tool_call_rate regresses harness quality gate"
        );

        candidate["metrics"]["state_metrics"] = json!({
            "cost_usd": 0.10,
            "malformed_tool_call_rate": 0.10,
            "json_parse_failure_rate": 0.05,
            "repair_loop_count": 1,
            "fim_success_rate": 0.50,
            "permission_prompt_rate": 0.10
        });
        let events = vec![
            event("PatchEvaluated", baseline),
            event("PatchEvaluated", candidate),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(!decision.eligible);
        assert_eq!(decision.criterion, None);
        assert_eq!(
            decision.reason,
            "candidate fim_success_rate regresses harness quality gate"
        );
    }

    #[test]
    fn promotion_decision_blocks_uncontrolled_budget_increase() {
        let mut baseline = eval_payload("eval-base", None, 1.0, 2, 0);
        baseline["metrics"]["state_metrics"] = json!({
            "cache_hit_ratio": 0.40,
            "cost_usd": 0.10,
            "input_tokens": 100,
            "output_tokens": 20
        });
        let mut candidate = eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0);
        candidate["metrics"]["state_metrics"] = json!({
            "cache_hit_ratio": 0.80,
            "cost_usd": 0.20,
            "input_tokens": 180,
            "output_tokens": 40
        });
        let events = vec![
            event("PatchEvaluated", baseline),
            event("PatchEvaluated", candidate),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(!decision.eligible);
        assert_eq!(decision.criterion, None);
        assert!(decision.reason.contains("budget gate"));
    }

    #[test]
    fn promotion_budget_gate_does_not_block_pass_rate_improvement() {
        let mut baseline = eval_payload("eval-base", None, 0.5, 1, 1);
        baseline["metrics"]["state_metrics"] = json!({
            "cost_usd": 0.10,
            "input_tokens": 100,
            "output_tokens": 20
        });
        let mut candidate = eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0);
        candidate["metrics"]["state_metrics"] = json!({
            "cost_usd": 0.30,
            "input_tokens": 300,
            "output_tokens": 80
        });
        let events = vec![
            event("PatchEvaluated", baseline),
            event("PatchEvaluated", candidate),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(decision.eligible);
        assert_eq!(decision.criterion.as_deref(), Some("pass_rate_improved"));
    }

    #[test]
    fn promotion_decision_accepts_token_reduction_from_state_metrics() {
        let mut baseline = eval_payload("eval-base", None, 1.0, 2, 0);
        baseline["metrics"]["state_metrics"] = json!({
            "input_tokens": 100,
            "output_tokens": 50
        });
        let mut candidate = eval_payload("eval-candidate", Some("patch-1"), 1.0, 2, 0);
        candidate["metrics"]["state_metrics"] = json!({
            "input_tokens": 90,
            "output_tokens": 40
        });
        let events = vec![
            event("PatchEvaluated", baseline),
            event("PatchEvaluated", candidate),
        ];

        let decision = promotion_decision(&events, "patch-1", None, None);

        assert!(decision.eligible);
        assert_eq!(
            decision.criterion.as_deref(),
            Some("token_usage_reduced_no_regression")
        );
    }

    #[test]
    fn promotion_safety_gate_allows_low_risk_patch_without_approval() {
        let patch = PatchSafetyMetadata {
            patch_id: "patch-1".into(),
            kind: HarnessPatchKind::PromptPolicy,
            risk_level: HarnessPatchRisk::Low,
            rollback_plan: vec!["reject patch".into()],
        };

        let gate = promotion_safety_gate_after_candidate_eval(&[], &patch, &[], None);

        assert!(gate.allowed);
        assert!(!gate.requires_human_approval);
        assert!(gate.approval_event_ids.is_empty());
    }

    #[test]
    fn promotion_safety_gate_blocks_high_risk_patch_without_approval() {
        let patch = PatchSafetyMetadata {
            patch_id: "patch-1".into(),
            kind: HarnessPatchKind::Safety,
            risk_level: HarnessPatchRisk::High,
            rollback_plan: vec!["revert patch".into()],
        };

        let gate = promotion_safety_gate_after_candidate_eval(&[], &patch, &[], None);

        assert!(!gate.allowed);
        assert!(gate.requires_human_approval);
        assert!(gate.reason.contains("HumanApprovalReceived"));
    }

    #[test]
    fn promotion_safety_gate_blocks_permission_and_shell_policy_without_approval() {
        for kind in [
            HarnessPatchKind::PermissionPolicy,
            HarnessPatchKind::ShellPolicy,
        ] {
            let patch = PatchSafetyMetadata {
                patch_id: "patch-1".into(),
                kind,
                risk_level: HarnessPatchRisk::Low,
                rollback_plan: vec!["revert policy change".into()],
            };

            let gate = promotion_safety_gate_after_candidate_eval(&[], &patch, &[], None);

            assert!(!gate.allowed);
            assert!(gate.requires_human_approval);
            assert!(gate.reason.contains("HumanApprovalReceived"));
        }
    }

    #[test]
    fn promotion_safety_gate_blocks_patch_without_rollback_plan() {
        let patch = PatchSafetyMetadata {
            patch_id: "patch-1".into(),
            kind: HarnessPatchKind::PromptPolicy,
            risk_level: HarnessPatchRisk::Low,
            rollback_plan: vec!["  ".into()],
        };

        let gate = promotion_safety_gate_after_candidate_eval(&[], &patch, &[], None);

        assert!(!gate.allowed);
        assert!(!gate.requires_human_approval);
        assert!(gate.reason.contains("rollback plan"));
    }

    #[test]
    fn harness_approval_request_payload_records_patch_risk_and_scope() {
        let patch = PatchSafetyMetadata {
            patch_id: "patch-1".into(),
            kind: HarnessPatchKind::Safety,
            risk_level: HarnessPatchRisk::High,
            rollback_plan: vec!["revert patch".into()],
        };
        let gate = promotion_safety_gate_after_candidate_eval(&[], &patch, &[], None);

        let payload =
            build_harness_approval_request_payload(&patch, &gate, "harness_patch_promotion", 42);

        assert_eq!(payload["patch_id"], "patch-1");
        assert_eq!(payload["kind"], "safety");
        assert_eq!(payload["risk_level"], "high");
        assert_eq!(payload["approval_scope"], "harness_patch_promotion");
        assert_eq!(payload["required"], true);
        assert_eq!(payload["requested_at_ms"], 42);
        assert!(payload["reason"]
            .as_str()
            .unwrap()
            .contains("HumanApprovalReceived"));
    }

    #[test]
    fn apply_approval_gate_allows_low_risk_patch_without_approval() {
        let patch = PatchSafetyMetadata {
            patch_id: "patch-apply".into(),
            kind: HarnessPatchKind::PromptPolicy,
            risk_level: HarnessPatchRisk::Low,
            rollback_plan: vec!["reject patch".into()],
        };

        let gate = apply_approval_gate(&[], &patch);

        assert!(gate.allowed);
        assert!(!gate.requires_human_approval);
        assert!(gate.approval_event_ids.is_empty());
    }

    #[test]
    fn apply_approval_gate_blocks_high_risk_patch_without_approval() {
        let patch = PatchSafetyMetadata {
            patch_id: "patch-apply".into(),
            kind: HarnessPatchKind::PermissionPolicy,
            risk_level: HarnessPatchRisk::High,
            rollback_plan: vec!["revert permission policy".into()],
        };

        let gate = apply_approval_gate(&[], &patch);

        assert!(!gate.allowed);
        assert!(gate.requires_human_approval);
        assert!(gate.reason.contains("before fork apply"));
        assert!(gate.reason.contains("HumanApprovalReceived"));
    }

    #[test]
    fn apply_approval_gate_accepts_matching_human_approval_event() {
        let patch = PatchSafetyMetadata {
            patch_id: "patch-apply".into(),
            kind: HarnessPatchKind::PermissionPolicy,
            risk_level: HarnessPatchRisk::High,
            rollback_plan: vec!["revert permission policy".into()],
        };
        let events = vec![
            event_with_id(
                "approval-other",
                "HumanApprovalReceived",
                json!({"patch_id": "other-patch"}),
            ),
            event_with_id(
                "approval-apply",
                "HumanApprovalReceived",
                json!({"patch_id": "patch-apply"}),
            ),
        ];

        let gate = apply_approval_gate(&events, &patch);

        assert!(gate.allowed);
        assert!(gate.requires_human_approval);
        assert_eq!(gate.approval_event_ids, vec!["approval-apply"]);
    }

    #[test]
    fn apply_mutation_surface_allows_initial_evolvable_patch_kinds() {
        for kind in [
            HarnessPatchKind::ContextPolicy,
            HarnessPatchKind::PromptPolicy,
            HarnessPatchKind::ToolSchema,
            HarnessPatchKind::ThinkingPolicy,
            HarnessPatchKind::ModelRoutingPolicy,
            HarnessPatchKind::TestPolicy,
            HarnessPatchKind::RepairPolicy,
            HarnessPatchKind::MemoryPolicy,
        ] {
            let patch = PatchSafetyMetadata {
                patch_id: "patch-surface".into(),
                kind,
                risk_level: HarnessPatchRisk::Medium,
                rollback_plan: vec!["reject patch".into()],
            };

            assert!(apply_mutation_surface_gate(&patch).is_ok());
        }
    }

    #[test]
    fn apply_mutation_surface_blocks_non_initial_patch_kinds() {
        for kind in [
            HarnessPatchKind::PermissionPolicy,
            HarnessPatchKind::ShellPolicy,
            HarnessPatchKind::StateProjection,
            HarnessPatchKind::Eval,
            HarnessPatchKind::Transport,
            HarnessPatchKind::Safety,
            HarnessPatchKind::Other,
        ] {
            let patch = PatchSafetyMetadata {
                patch_id: "patch-surface".into(),
                kind,
                risk_level: HarnessPatchRisk::Medium,
                rollback_plan: vec!["reject patch".into()],
            };

            let err = apply_mutation_surface_gate(&patch).unwrap_err();
            assert!(err.contains("outside the initial constrained harness mutation surface"));
        }
    }

    #[test]
    fn harness_evolution_policy_parses_configured_patch_gates() {
        let config = std::collections::HashMap::from([
            (
                "evolve_harness_allowed_patch_types".to_string(),
                "[\"context_policy\", \"repair_policy\"]".to_string(),
            ),
            (
                "evolve_harness_require_human_approval_for".to_string(),
                "[\"model_routing_policy\", \"memory_policy\"]".to_string(),
            ),
        ]);

        let policy = harness_evolution_policy_from_config(&config);

        assert_eq!(
            policy.allowed_patch_types,
            Some(vec![
                HarnessPatchKind::ContextPolicy,
                HarnessPatchKind::RepairPolicy
            ])
        );
        assert_eq!(
            policy.require_human_approval_for,
            vec![
                HarnessPatchKind::ModelRoutingPolicy,
                HarnessPatchKind::MemoryPolicy
            ]
        );
    }

    #[test]
    fn apply_mutation_surface_uses_configured_allowed_patch_types() {
        let policy = HarnessEvolutionPolicy {
            allowed_patch_types: Some(vec![HarnessPatchKind::ContextPolicy]),
            require_human_approval_for: Vec::new(),
        };
        let context_patch = PatchSafetyMetadata {
            patch_id: "patch-context".into(),
            kind: HarnessPatchKind::ContextPolicy,
            risk_level: HarnessPatchRisk::Low,
            rollback_plan: vec!["reject patch".into()],
        };
        let repair_patch = PatchSafetyMetadata {
            patch_id: "patch-repair".into(),
            kind: HarnessPatchKind::RepairPolicy,
            risk_level: HarnessPatchRisk::Low,
            rollback_plan: vec!["reject patch".into()],
        };

        assert!(apply_mutation_surface_gate_with_policy(&context_patch, &policy).is_ok());
        let err = apply_mutation_surface_gate_with_policy(&repair_patch, &policy).unwrap_err();
        assert!(err.contains("outside the initial constrained harness mutation surface"));
    }

    #[test]
    fn human_approval_gate_uses_configured_patch_kinds() {
        let policy = HarnessEvolutionPolicy {
            allowed_patch_types: None,
            require_human_approval_for: vec![HarnessPatchKind::ModelRoutingPolicy],
        };
        let patch = PatchSafetyMetadata {
            patch_id: "patch-routing".into(),
            kind: HarnessPatchKind::ModelRoutingPolicy,
            risk_level: HarnessPatchRisk::Low,
            rollback_plan: vec!["revert routing".into()],
        };

        assert!(patch_requires_human_approval_with_policy(&patch, &policy));
    }

    #[test]
    fn promotion_safety_gate_accepts_matching_human_approval_event() {
        let patch = PatchSafetyMetadata {
            patch_id: "patch-1".into(),
            kind: HarnessPatchKind::Safety,
            risk_level: HarnessPatchRisk::High,
            rollback_plan: vec!["revert patch".into()],
        };
        let events = vec![
            event_with_id(
                "approval-other",
                "HumanApprovalReceived",
                json!({"patch_id": "patch-2"}),
            ),
            event_with_id(
                "approval-1",
                "HumanApprovalReceived",
                json!({
                    "patch_id": "patch-1",
                    "approval_scope": "harness_patch_promotion",
                    "approved_at_ms": 10,
                }),
            ),
        ];

        let gate = promotion_safety_gate_after_candidate_eval(&events, &patch, &[], None);

        assert!(gate.allowed);
        assert!(gate.requires_human_approval);
        assert_eq!(gate.approval_event_ids, vec!["approval-1"]);
    }

    #[test]
    fn promotion_safety_gate_validates_explicit_approval_event_ids() {
        let patch = PatchSafetyMetadata {
            patch_id: "patch-1".into(),
            kind: HarnessPatchKind::Safety,
            risk_level: HarnessPatchRisk::High,
            rollback_plan: vec!["revert patch".into()],
        };
        let events = vec![
            event_with_id(
                "approval-other",
                "HumanApprovalReceived",
                json!({"patch_id": "patch-2"}),
            ),
            event_with_id(
                "approval-1",
                "HumanApprovalReceived",
                json!({
                    "patch_id": "patch-1",
                    "approval_scope": "harness_patch_promotion",
                    "approved_at_ms": 10,
                }),
            ),
        ];

        let gate = promotion_safety_gate_after_candidate_eval(
            &events,
            &patch,
            &["approval-other".into()],
            None,
        );
        assert!(!gate.allowed);

        let gate = promotion_safety_gate_after_candidate_eval(
            &events,
            &patch,
            &["approval-1".into()],
            None,
        );
        assert!(gate.allowed);
        assert_eq!(gate.approval_event_ids, vec!["approval-1"]);
    }

    #[test]
    fn promotion_safety_gate_rejects_wrong_scope_approval_for_promotion() {
        let patch = PatchSafetyMetadata {
            patch_id: "patch-1".into(),
            kind: HarnessPatchKind::PermissionPolicy,
            risk_level: HarnessPatchRisk::Low,
            rollback_plan: vec!["revert permission policy".into()],
        };
        let events = vec![event_with_id(
            "approval-apply",
            "HumanApprovalReceived",
            json!({
                "patch_id": "patch-1",
                "approval_scope": "harness_patch_apply",
                "approved_at_ms": 50,
            }),
        )];

        let gate = promotion_safety_gate_after_candidate_eval(&events, &patch, &[], None);

        assert!(!gate.allowed);
        assert!(gate.requires_human_approval);
        assert!(gate.reason.contains("promotion"));
        assert!(gate.approval_event_ids.is_empty());
    }

    #[test]
    fn promotion_safety_gate_rejects_approval_older_than_candidate_eval() {
        let patch = PatchSafetyMetadata {
            patch_id: "patch-1".into(),
            kind: HarnessPatchKind::PermissionPolicy,
            risk_level: HarnessPatchRisk::Low,
            rollback_plan: vec!["revert permission policy".into()],
        };
        let events = vec![event_with_id(
            "approval-old",
            "HumanApprovalReceived",
            json!({
                "patch_id": "patch-1",
                "approval_scope": "harness_patch_promotion",
                "approved_at_ms": 40,
            }),
        )];

        let gate = promotion_safety_gate_after_candidate_eval(&events, &patch, &[], Some(50));

        assert!(!gate.allowed);
        assert!(gate.requires_human_approval);
        assert!(gate.approval_event_ids.is_empty());

        let fresh_events = vec![event_with_id(
            "approval-fresh",
            "HumanApprovalReceived",
            json!({
                "patch_id": "patch-1",
                "approval_scope": "harness_patch_promotion",
                "approved_at_ms": 50,
            }),
        )];
        let fresh_gate =
            promotion_safety_gate_after_candidate_eval(&fresh_events, &patch, &[], Some(50));

        assert!(fresh_gate.allowed);
        assert_eq!(fresh_gate.approval_event_ids, vec!["approval-fresh"]);
    }

    #[test]
    fn parses_kind_risk_and_comma_evidence() {
        let args = vec![
            "yoyo".into(),
            "evolve".into(),
            "harness".into(),
            "propose".into(),
            "--evidence".into(),
            "evt-1, evt-2".into(),
            "--evidence".into(),
            "evt-3".into(),
        ];

        assert_eq!(
            parse_kind("tool-schema").unwrap(),
            HarnessPatchKind::ToolSchema
        );
        assert_eq!(
            parse_kind("context-policy").unwrap(),
            HarnessPatchKind::ContextPolicy
        );
        assert_eq!(
            parse_kind("permission-policy").unwrap(),
            HarnessPatchKind::PermissionPolicy
        );
        assert_eq!(
            parse_kind("shell-policy").unwrap(),
            HarnessPatchKind::ShellPolicy
        );
        assert_eq!(parse_risk("critical").unwrap(), HarnessPatchRisk::Critical);
        assert_eq!(
            collect_split_values(&args, "--evidence"),
            vec!["evt-1", "evt-2", "evt-3"]
        );
    }

    #[test]
    fn default_worktree_path_sanitizes_patch_id() {
        assert_eq!(
            default_worktree_path("patch/one two").display().to_string(),
            ".yoyo/evolve/worktrees/patch_one_two"
        );
        assert_eq!(
            default_worktree_path("").display().to_string(),
            ".yoyo/evolve/worktrees/patch"
        );
    }

    #[test]
    fn patch_path_validation_accepts_relative_git_diff_targets() {
        let dir = tempfile::tempdir().unwrap();
        let patch = dir.path().join("ok.patch");
        std::fs::write(
            &patch,
            "diff --git a/src/lib.rs b/src/lib.rs\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1 +1 @@\n-old\n+new\n",
        )
        .unwrap();

        let checked = validate_patch_file_paths(&patch).unwrap();

        assert_eq!(checked, 4);
    }

    #[test]
    fn patch_path_validation_rejects_traversal_targets() {
        let dir = tempfile::tempdir().unwrap();
        let patch = dir.path().join("bad.patch");
        std::fs::write(
            &patch,
            "diff --git a/src/lib.rs b/../outside.rs\n--- a/src/lib.rs\n+++ b/../outside.rs\n@@ -1 +1 @@\n-old\n+new\n",
        )
        .unwrap();

        let err = validate_patch_file_paths(&patch).unwrap_err();

        assert!(err.contains("path traversal"));
    }

    #[test]
    fn patch_path_validation_rejects_absolute_targets() {
        let dir = tempfile::tempdir().unwrap();
        let patch = dir.path().join("absolute.patch");
        std::fs::write(
            &patch,
            "diff --git a/src/lib.rs /tmp/outside.rs\n--- a/src/lib.rs\n+++ /tmp/outside.rs\n@@ -1 +1 @@\n-old\n+new\n",
        )
        .unwrap();

        let err = validate_patch_file_paths(&patch).unwrap_err();

        assert!(err.contains("relative repository path"));
    }

    #[test]
    fn apply_payload_records_worktree_lineage() {
        let report = ApplyWorktreeReport {
            worktree_path: PathBuf::from(".yoyo/evolve/worktrees/patch-1"),
            patch_file: PathBuf::from("/tmp/patch.diff"),
            worktree_stdout: "prepared".into(),
            apply_stdout: "applied".into(),
        };

        let patch = PatchSafetyMetadata {
            patch_id: "patch-1".into(),
            kind: HarnessPatchKind::ContextPolicy,
            risk_level: HarnessPatchRisk::Low,
            rollback_plan: vec!["revert context selector change".into()],
        };

        let payload = build_apply_payload("patch-1", &report, false, Some(&patch), 42);

        assert_eq!(payload["patch_id"], "patch-1");
        assert_eq!(payload["status"], "applied_in_fork");
        assert_eq!(payload["check_only"], false);
        assert_eq!(payload["kind"], "context_policy");
        assert_eq!(payload["risk_level"], "low");
        assert_eq!(
            payload["rollback_plan"][0],
            "revert context selector change"
        );
        assert_eq!(payload["worktree_path"], ".yoyo/evolve/worktrees/patch-1");
        assert_eq!(payload["patch_file"], "/tmp/patch.diff");
        assert_eq!(payload["git_worktree_stdout"], "prepared");
        assert_eq!(payload["git_apply_stdout"], "applied");
        assert_eq!(payload["applied_at_ms"], 42);
        assert!(payload.get("base_git_commit").is_some());
    }

    #[test]
    fn latest_applied_patch_prefers_latest_location() {
        let events = vec![
            event_with_id(
                "apply-1",
                "PatchApplied",
                json!({
                    "patch_id": "patch-1",
                    "worktree_path": ".yoyo/evolve/worktrees/old",
                    "patch_file": "/tmp/old.patch"
                }),
            ),
            event_with_id(
                "apply-2",
                "PatchApplied",
                json!({
                    "patch_id": "patch-1",
                    "worktree_path": ".yoyo/evolve/worktrees/new",
                    "patch_file": "/tmp/new.patch"
                }),
            ),
        ];

        let location = latest_applied_patch(&events, "patch-1").unwrap();

        assert_eq!(
            location.worktree_path,
            PathBuf::from(".yoyo/evolve/worktrees/new")
        );
        assert_eq!(location.patch_file, PathBuf::from("/tmp/new.patch"));
        assert_eq!(latest_applied_patch(&events, "patch-2"), None);
    }

    #[test]
    fn eval_worktree_requires_patch_applied_before_eval() {
        let events = vec![event(
            "PatchProposed",
            json!({
                "patch_id": "patch-eval",
                "intent": "tighten context selector"
            }),
        )];

        let err = eval_worktree_for_patch(&events, "patch-eval", None).unwrap_err();

        assert!(err.contains("no applied harness patch found"));
    }

    #[test]
    fn eval_worktree_defaults_to_latest_applied_worktree() {
        let events = vec![
            event(
                "PatchProposed",
                json!({
                    "patch_id": "patch-eval",
                    "intent": "tighten context selector"
                }),
            ),
            event_with_id(
                "apply-1",
                "PatchApplied",
                json!({
                    "patch_id": "patch-eval",
                    "worktree_path": ".yoyo/evolve/worktrees/patch-eval",
                    "patch_file": "/tmp/patch.diff"
                }),
            ),
        ];

        let worktree = eval_worktree_for_patch(&events, "patch-eval", None).unwrap();

        assert_eq!(
            worktree,
            Some(PathBuf::from(".yoyo/evolve/worktrees/patch-eval"))
        );
    }

    #[test]
    fn eval_worktree_preserves_explicit_worktree_after_apply_gate() {
        let events = vec![
            event(
                "PatchProposed",
                json!({
                    "patch_id": "patch-eval",
                    "intent": "tighten context selector"
                }),
            ),
            event_with_id(
                "apply-1",
                "PatchApplied",
                json!({
                    "patch_id": "patch-eval",
                    "worktree_path": ".yoyo/evolve/worktrees/patch-eval",
                    "patch_file": "/tmp/patch.diff"
                }),
            ),
        ];

        let worktree = eval_worktree_for_patch(
            &events,
            "patch-eval",
            Some(PathBuf::from("/tmp/explicit-worktree")),
        )
        .unwrap();

        assert_eq!(worktree, Some(PathBuf::from("/tmp/explicit-worktree")));
    }

    #[test]
    fn eval_worktree_isolation_blocks_active_repo_checkout() {
        let repo = tempfile::tempdir().unwrap();

        let err = eval_worktree_isolation_gate(Some(repo.path()), repo.path()).unwrap_err();

        assert!(err.contains("active repository checkout"));
    }

    #[test]
    fn eval_worktree_isolation_allows_separate_worktree_path() {
        let repo = tempfile::tempdir().unwrap();
        let worktree = repo
            .path()
            .join(".yoyo")
            .join("evolve")
            .join("worktrees")
            .join("patch-1");

        assert!(eval_worktree_isolation_gate(Some(&worktree), repo.path()).is_ok());
    }

    #[test]
    fn rollback_payload_records_revert_lineage() {
        let report = ApplyWorktreeReport {
            worktree_path: PathBuf::from(".yoyo/evolve/worktrees/patch-1"),
            patch_file: PathBuf::from("/tmp/patch.diff"),
            worktree_stdout: "existing worktree".into(),
            apply_stdout: "reversed".into(),
        };

        let payload = build_rollback_payload("patch-1", &report, "bad regression", 99);

        assert_eq!(payload["patch_id"], "patch-1");
        assert_eq!(payload["status"], "reverted");
        assert_eq!(payload["reason"], "bad regression");
        assert_eq!(payload["worktree_path"], ".yoyo/evolve/worktrees/patch-1");
        assert_eq!(payload["patch_file"], "/tmp/patch.diff");
        assert_eq!(payload["git_apply_reverse_stdout"], "reversed");
        assert_eq!(payload["reverted_at_ms"], 99);
    }

    #[test]
    fn lifecycle_decision_payload_links_patch_event_eval_and_reason() {
        let promotion_decision = PromotionDecision {
            eligible: true,
            criterion: Some("cost_reduced_no_regression".into()),
            reason: "candidate cost is lower with no pass-rate regression".into(),
            baseline_eval_id: Some("eval-base".into()),
            candidate_eval_id: Some("eval-1".into()),
            protocol_eval_id: None,
            suite: Some("local-smoke".into()),
            baseline_score: Some(1.0),
            candidate_score: Some(1.0),
            baseline_passed: Some(2),
            candidate_passed: Some(2),
            baseline_failed: Some(0),
            candidate_failed: Some(0),
            metric_evidence: Some(json!({"metrics": []})),
        };
        let safety_gate = PromotionSafetyGate {
            allowed: true,
            requires_human_approval: true,
            reason: "fresh human approval event found for promotion".into(),
            patch_kind: HarnessPatchKind::Safety,
            risk_level: HarnessPatchRisk::High,
            approval_event_ids: vec!["evt-approval".into()],
        };
        let payload = build_lifecycle_decision_payload(LifecycleDecisionOptions {
            patch_id: "patch/one two",
            decision_type: "harness_patch_promotion",
            decision: "promote",
            rationale: "candidate improved local smoke",
            status: "recorded",
            patch_event_id: Some("evt-promote"),
            eval_id: Some("eval-1"),
            forced: false,
            decided_at_ms: 123,
            promotion_decision: Some(&promotion_decision),
            safety_gate: Some(&safety_gate),
        });

        assert_eq!(payload["decision_id"], "decision-patch_one_two-123");
        assert_eq!(payload["decision_type"], "harness_patch_promotion");
        assert_eq!(payload["decision"], "promote");
        assert_eq!(payload["rationale"], "candidate improved local smoke");
        assert_eq!(payload["status"], "recorded");
        assert_eq!(payload["patch_id"], "patch/one two");
        assert_eq!(payload["patch_event_id"], "evt-promote");
        assert_eq!(payload["eval_id"], "eval-1");
        assert_eq!(payload["forced"], false);
        assert_eq!(
            payload["promotion_decision"]["criterion"],
            "cost_reduced_no_regression"
        );
        assert_eq!(
            payload["promotion_decision"]["metric_evidence"]["metrics"],
            json!([])
        );
        assert_eq!(payload["approval_event_ids"], json!(["evt-approval"]));
        assert_eq!(payload["safety_gate"]["allowed"], true);
        assert_eq!(payload["safety_gate"]["requires_human_approval"], true);
        assert_eq!(payload["safety_gate"]["patch_kind"], "safety");
        assert_eq!(payload["safety_gate"]["risk_level"], "high");
    }

    #[test]
    fn comparison_decision_payload_records_compared_lifecycle_stage() {
        let promotion_decision = PromotionDecision {
            eligible: false,
            criterion: None,
            reason: "candidate passed but no promotion metric improved".into(),
            baseline_eval_id: Some("eval-base".into()),
            candidate_eval_id: Some("eval-candidate".into()),
            protocol_eval_id: None,
            suite: Some("local-smoke".into()),
            baseline_score: Some(1.0),
            candidate_score: Some(1.0),
            baseline_passed: Some(2),
            candidate_passed: Some(2),
            baseline_failed: Some(0),
            candidate_failed: Some(0),
            metric_evidence: Some(json!({"metrics": []})),
        };

        let payload =
            build_comparison_decision_payload("patch-compare", &promotion_decision, false, 456);

        assert_eq!(payload["decision_id"], "decision-patch-compare-456");
        assert_eq!(payload["decision_type"], "harness_patch_comparison");
        assert_eq!(payload["decision"], "not_eligible");
        assert_eq!(payload["status"], "compared");
        assert_eq!(payload["patch_id"], "patch-compare");
        assert_eq!(payload["eval_id"], "eval-candidate");
        assert_eq!(payload["forced"], false);
        assert_eq!(
            payload["promotion_decision"]["reason"],
            "candidate passed but no promotion metric improved"
        );
    }

    #[test]
    fn risk_score_decision_payload_records_risk_scored_lifecycle_stage() {
        let patch = build_harness_patch(PatchProposalOptions {
            patch_id: "patch-risk".into(),
            kind: HarnessPatchKind::PermissionPolicy,
            risk_level: HarnessPatchRisk::High,
            base_harness_version: "genome-v1".into(),
            base_git_commit: Some("abc123".into()),
            intent: "tighten permission approval policy".into(),
            evidence_event_ids: vec!["evt-permission-risk".into()],
            expected_effects: vec!["lower unsafe approval rate".into()],
            eval_plan: vec!["cargo test commands_state::tests::policy_report".into()],
            rollback_plan: vec!["reject patch".into()],
            default_eval_plan: Vec::new(),
            created_at_ms: 1,
        });

        let payload = build_risk_score_decision_payload(&patch, "evt-propose", 789);

        assert_eq!(payload["decision_id"], "decision-patch-risk-789");
        assert_eq!(payload["decision_type"], "harness_patch_risk_score");
        assert_eq!(payload["decision"], "high");
        assert_eq!(payload["status"], "risk_scored");
        assert_eq!(payload["patch_id"], "patch-risk");
        assert_eq!(payload["patch_event_id"], "evt-propose");
        assert_eq!(payload["kind"], "permission_policy");
        assert_eq!(payload["risk_level"], "high");
        assert_eq!(payload["risk_policy"], "explicit_or_default_patch_risk");
        assert_eq!(payload["scored_at_ms"], 789);
    }

    #[test]
    fn patch_lookup_reads_canonical_yoagent_state_events() {
        let patch = build_harness_patch(PatchProposalOptions {
            patch_id: "patch-canonical".into(),
            kind: HarnessPatchKind::PromptPolicy,
            risk_level: HarnessPatchRisk::Medium,
            base_harness_version: "genome-v1".into(),
            base_git_commit: None,
            intent: "include failing files".into(),
            evidence_event_ids: Vec::new(),
            expected_effects: Vec::new(),
            eval_plan: vec!["cargo check".into()],
            rollback_plan: vec!["reject patch".into()],
            default_eval_plan: Vec::new(),
            created_at_ms: 1,
        });
        let event = crate::state::StateEvent {
            event_id: "evt-patch".into(),
            event_type: EventType::PatchProposed,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-1".into()),
            session_id: None,
            trace_id: "trace-1".into(),
            parent_event_ids: Vec::new(),
            payload: serde_json::to_value(&patch).unwrap(),
        };
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        crate::state::append_event(&path, &event).unwrap();
        let events = read_events(&path).unwrap();

        assert!(patch_exists(&events, "patch-canonical"));
        let safety = find_patch_safety_metadata(&events, "patch-canonical").unwrap();
        assert_eq!(safety.kind, HarnessPatchKind::PromptPolicy);
        assert_eq!(safety.risk_level, HarnessPatchRisk::Medium);
    }
}
