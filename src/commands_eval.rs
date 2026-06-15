//! Local harness evaluation commands.

use crate::format::*;
use crate::state::{Actor, EvalResult, EvalStatus, EventType, StateConfig, StateRecorder};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

const SOURCE_PROVENANCE_FINDING_SUMMARY_LIMIT: usize = 5;
const SOURCE_PROVENANCE_FINDING_SUMMARY_MAX_CHARS: usize = 240;
const SOURCE_PROVENANCE_FINDING_MARKER_MAX_CHARS: usize = 96;

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

pub fn handle_eval_subcommand(args: &[String]) {
    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("help");
    match sub {
        "run" => handle_run(args),
        "fixtures" => handle_fixtures(args),
        "schedule" => handle_schedule(args),
        "release-gate" => handle_release_gate(args),
        "replay" => handle_replay(args),
        "report" => {
            let Some(eval_id) = args.get(3) else {
                eprintln!("{YELLOW}  Usage: yoyo eval report <eval-id|run-id|trace-id>{RESET}");
                return;
            };
            handle_report(eval_id);
        }
        "compare" => {
            let (Some(baseline), Some(candidate)) = (args.get(3), args.get(4)) else {
                eprintln!(
                    "{YELLOW}  Usage: yoyo eval compare <baseline-eval|run-id> <candidate-eval|run-id>{RESET}"
                );
                return;
            };
            handle_compare(baseline, candidate);
        }
        _ => print_usage(),
    }
}

fn handle_replay(args: &[String]) {
    if !args.iter().any(|arg| arg == "--from-state") {
        eprintln!("{YELLOW}  Usage: yoyo eval replay --from-state [--limit N] [--json]{RESET}");
        return;
    }
    let limit = flag_value(args, "--limit")
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(10);
    let events_path = default_events_path();
    let events = match read_events(&events_path) {
        Ok(events) => events,
        Err(e) => {
            eprintln!(
                "{YELLOW}  no state log found at {}: {e}{RESET}",
                events_path.display()
            );
            return;
        }
    };
    let report = build_failure_replay_report(&events, "local-smoke", limit);
    if args.iter().any(|arg| arg == "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&report.to_json()).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!("{}", format_failure_replay_report(&report));
    }
}

fn handle_schedule(args: &[String]) {
    let genome = crate::deepseek::DeepSeekHarnessGenome::default();
    let suite = flag_value(args, "--suite").unwrap_or(&genome.test_policy.benchmark_subset);
    let interval_hours = flag_value(args, "--interval-hours")
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(24);
    let record = args.iter().any(|arg| arg == "--record");
    let json_output = args.iter().any(|arg| arg == "--json");
    let events_path = default_events_path();
    let events = read_events(&events_path).unwrap_or_default();
    let report = build_eval_schedule_report(&events, suite, interval_hours, now_ms());

    if record {
        if let Err(e) = record_eval_schedule_decision(&report) {
            eprintln!("{YELLOW}  failed to record eval schedule decision: {e}{RESET}");
        }
    }

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&report.to_json()).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!("{}", format_eval_schedule_report(&report));
    }
}

fn handle_release_gate(args: &[String]) {
    let genome = crate::deepseek::DeepSeekHarnessGenome::default();
    let suite = flag_value(args, "--suite").unwrap_or(&genome.test_policy.benchmark_subset);
    let max_age_hours = flag_value(args, "--max-age-hours")
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(24);
    let record = args.iter().any(|arg| arg == "--record");
    let json_output = args.iter().any(|arg| arg == "--json");
    let fail_on_block = args.iter().any(|arg| arg == "--fail");
    let require_protocol = args.iter().any(|arg| arg == "--require-protocol");
    let min_fixture_tasks =
        flag_value(args, "--min-fixture-tasks").and_then(|raw| raw.parse::<u64>().ok());
    let min_fixture_commands =
        flag_value(args, "--min-fixture-commands").and_then(|raw| raw.parse::<u64>().ok());
    let min_fixture_risk_labels = release_gate_min_fixture_risk_labels(args);
    let events = read_events(&default_events_path()).unwrap_or_default();
    let now = now_ms();
    let report = if require_protocol
        || min_fixture_tasks.is_some()
        || min_fixture_commands.is_some()
        || !min_fixture_risk_labels.is_empty()
    {
        build_release_gate_report_with_policy(
            &events,
            suite,
            max_age_hours,
            now,
            ReleaseGatePolicy {
                require_protocol,
                min_fixture_tasks,
                min_fixture_commands,
                min_fixture_risk_labels,
            },
        )
    } else {
        build_release_gate_report(&events, suite, max_age_hours, now)
    };

    if record {
        if let Err(e) = record_release_gate_decision(&report) {
            eprintln!("{YELLOW}  failed to record release gate decision: {e}{RESET}");
        }
    }

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&report.to_json()).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!("{}", format_release_gate_report(&report));
    }

    if fail_on_block && !report.ready {
        std::process::exit(1);
    }
}

fn release_gate_min_fixture_risk_labels(args: &[String]) -> BTreeMap<String, u64> {
    [
        ("--min-fixture-low-risk", "low"),
        ("--min-fixture-medium-risk", "medium"),
        ("--min-fixture-high-risk", "high"),
    ]
    .into_iter()
    .filter_map(|(flag, label)| {
        flag_value(args, flag)
            .and_then(|raw| raw.parse::<u64>().ok())
            .map(|count| (label.to_string(), count))
    })
    .collect()
}

fn handle_fixtures(args: &[String]) {
    let action = args.get(3).map(|s| s.as_str()).unwrap_or("list");
    let suite_name = flag_value(args, "--suite").map_or("local-smoke", |s| s.as_str());
    let suite = match crate::eval_fixtures::load_fixture_suite(suite_name) {
        Ok(suite) => suite,
        Err(e) => {
            eprintln!("{YELLOW}  {e}{RESET}");
            return;
        }
    };

    match action {
        "list" => {
            let group_by_domain = args
                .windows(2)
                .any(|w| w[0] == "--group-by" && w[1] == "domain");
            if group_by_domain {
                println!(
                    "{}",
                    crate::eval_fixtures::format_fixture_list_by_domain(&suite)
                );
            } else {
                println!("{}", crate::eval_fixtures::format_fixture_list(&suite));
            }
        }
        "validate" => {
            println!("Eval fixture suite is valid");
            println!("  suite: {}", suite.suite);
            println!("  tasks: {}", suite.tasks.len());
        }
        "run" => handle_fixture_run(args, suite),
        "attempt" => handle_fixture_attempt(args, suite),
        _ => eprintln!(
            "{YELLOW}  Usage: yoyo eval fixtures <list|validate|run|attempt> [--suite local-smoke] [--task TASK] [--worktree PATH] [--agent-command CMD|--default-agent] [--dry-run]{RESET}"
        ),
    }
}

fn handle_fixture_run(args: &[String], suite: crate::eval_fixtures::FixtureSuite) {
    let dry_run = args.iter().any(|arg| arg == "--dry-run");
    let task_filter = flag_value(args, "--task").map(|s| s.as_str());
    let tasks = suite
        .tasks
        .iter()
        .filter(|task| task_filter.map(|id| id == task.task_id).unwrap_or(true))
        .collect::<Vec<_>>();

    if tasks.is_empty() {
        eprintln!(
            "{YELLOW}  no fixture tasks found for filter '{}'{RESET}",
            task_filter.unwrap_or("")
        );
        return;
    }

    if dry_run {
        println!("Eval fixture dry run");
        println!("  suite: {}", suite.suite);
        for task in &tasks {
            println!("  task:  {} ({})", task.task_id, task.category);
            for command in &task.tests {
                println!("  test:  {command}");
            }
        }
        return;
    }

    println!("Eval fixture run");
    println!("  suite: {}", suite.suite);
    let started = Instant::now();
    let started_wall_ms = now_ms();
    let mut results = Vec::new();
    for task in &tasks {
        println!("  running task: {}", task.task_id);
        let result = crate::eval_fixtures::run_fixture_task(task);
        println!(
            "  result:       {}",
            if result.passed { "passed" } else { "failed" }
        );
        results.push(result);
    }

    let mut eval = build_fixture_eval_result(
        &suite.suite,
        &crate::deepseek::DeepSeekHarnessGenome::default().version,
        &results,
        started.elapsed().as_millis() as u64,
    );
    attach_fixture_suite_metadata(&mut eval, &tasks);
    attach_eval_reproducibility_manifest(
        &mut eval,
        "fixtures",
        None,
        format!("yoyo eval fixtures run --suite {}", suite.suite),
        fixture_result_commands(&results),
    );
    if let Err(e) = record_fixture_failure_events(&mut eval, &results) {
        eprintln!("{YELLOW}  failed to link fixture failures into state: {e}{RESET}");
    }
    attach_eval_state_metrics(&mut eval, started_wall_ms, now_ms());
    match append_eval_result(&eval) {
        Ok(recorded) => {
            println!("  eval id:      {}", eval.eval_id);
            println!("  event id:     {}", recorded.event_id);
            println!("  artifact:     {}", recorded.artifact_path.display());
            println!("  status:       {}", status_label(&eval.status));
        }
        Err(e) => eprintln!("{RED}  failed to record fixture eval result: {e}{RESET}"),
    }
}

fn handle_fixture_attempt(args: &[String], suite: crate::eval_fixtures::FixtureSuite) {
    let dry_run = args.iter().any(|arg| arg == "--dry-run");
    let use_default_agent =
        args.iter().any(|arg| arg == "--default-agent") || env_flag("YOYO_EVAL_USE_DEFAULT_AGENT");
    let task_filter = flag_value(args, "--task").map(|s| s.as_str());
    let worktree = flag_value(args, "--worktree")
        .or_else(|| flag_value(args, "--workdir"))
        .map(PathBuf::from);
    let agent_command = fixture_attempt_agent_command_spec(args, use_default_agent);
    let tasks = suite
        .tasks
        .iter()
        .filter(|task| task_filter.map(|id| id == task.task_id).unwrap_or(true))
        .collect::<Vec<_>>();

    if tasks.is_empty() {
        eprintln!(
            "{YELLOW}  no fixture tasks found for filter '{}'{RESET}",
            task_filter.unwrap_or("")
        );
        return;
    }

    if dry_run {
        let template = agent_command
            .as_ref()
            .map(|command| command.template.as_str())
            .unwrap_or(DEFAULT_AGENT_TEMPLATE_HINT);
        println!("Eval fixture agent attempt dry run");
        println!("  suite:    {}", suite.suite);
        println!(
            "  worktree: {}",
            worktree
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "<required for apply>".to_string())
        );
        println!(
            "  agent source: {}",
            agent_command
                .as_ref()
                .map(|command| command.source)
                .unwrap_or("required")
        );
        for task in &tasks {
            println!("  task:     {} ({})", task.task_id, task.category);
            println!(
                "  agent:    {}",
                crate::eval_fixtures::render_agent_command(template, task)
            );
            for command in &task.tests {
                println!("  test:     {command}");
            }
        }
        return;
    }

    let Some(worktree) = worktree else {
        eprintln!("{YELLOW}  fixture agent attempts require --worktree PATH{RESET}");
        return;
    };
    let Some(agent_command) = agent_command else {
        eprintln!(
            "{YELLOW}  fixture agent attempts require --agent-command CMD, YOYO_EVAL_AGENT_COMMAND, or --default-agent{RESET}"
        );
        return;
    };

    println!("Eval fixture agent attempt");
    println!("  suite:    {}", suite.suite);
    println!("  worktree: {}", worktree.display());
    let started = Instant::now();
    let started_wall_ms = now_ms();
    let mut results = Vec::new();
    for task in &tasks {
        println!("  running task: {}", task.task_id);
        match crate::eval_fixtures::run_fixture_agent_attempt(
            task,
            &worktree,
            &agent_command.template,
        ) {
            Ok(result) => {
                println!(
                    "  agent:        {}",
                    if result.agent_result.passed {
                        "passed"
                    } else {
                        "failed"
                    }
                );
                println!(
                    "  result:       {}",
                    if result.passed { "passed" } else { "failed" }
                );
                results.push(result);
            }
            Err(e) => {
                eprintln!("{YELLOW}  {e}{RESET}");
                return;
            }
        }
    }

    let mut eval = build_fixture_agent_attempt_eval_result(
        &suite.suite,
        &crate::deepseek::DeepSeekHarnessGenome::default().version,
        &results,
        started.elapsed().as_millis() as u64,
    );
    attach_fixture_suite_metadata(&mut eval, &tasks);
    attach_eval_reproducibility_manifest(
        &mut eval,
        "fixture-agent-attempts",
        Some(&worktree),
        format!(
            "yoyo eval fixtures attempt --suite {} --worktree {} --agent-command {}",
            suite.suite,
            shell_quote(&worktree.display().to_string()),
            shell_quote(&agent_command.template)
        ),
        fixture_agent_attempt_commands(&results),
    );
    attach_fixture_agent_attempt_source(&mut eval, agent_command.source);
    if let Err(e) = record_fixture_agent_attempt_failure_events(&mut eval, &results) {
        eprintln!("{YELLOW}  failed to link fixture agent failures into state: {e}{RESET}");
    }
    attach_eval_state_metrics(&mut eval, started_wall_ms, now_ms());
    match append_eval_result(&eval) {
        Ok(recorded) => {
            println!("  eval id:      {}", eval.eval_id);
            println!("  event id:     {}", recorded.event_id);
            println!("  artifact:     {}", recorded.artifact_path.display());
            println!("  status:       {}", status_label(&eval.status));
        }
        Err(e) => eprintln!("{RED}  failed to record fixture agent eval result: {e}{RESET}"),
    }
}

fn handle_run(args: &[String]) {
    let genome = crate::deepseek::DeepSeekHarnessGenome::default();
    let suite = flag_value(args, "--suite").unwrap_or(&genome.test_policy.benchmark_subset);
    let harness_version = flag_value(args, "--harness-version").unwrap_or(&genome.version);
    let patch_id = flag_value(args, "--patch-id").map(|s| s.to_string());
    let dry_run = args.iter().any(|arg| arg == "--dry-run");
    let workdir = flag_value(args, "--worktree")
        .or_else(|| flag_value(args, "--workdir"))
        .map(PathBuf::from);
    let task_log = flag_value(args, "--task-log").map(PathBuf::from);
    let task_git_base = flag_value(args, "--task-git-base").map(|s| s.to_string());
    let _ = run_eval(EvalRunOptions {
        suite,
        harness_version,
        patch_id,
        dry_run,
        workdir,
        task_log,
        task_git_base,
    });
}

#[derive(Debug, Clone)]
pub struct EvalRunOptions<'a> {
    pub suite: &'a str,
    pub harness_version: &'a str,
    pub patch_id: Option<String>,
    pub dry_run: bool,
    pub workdir: Option<PathBuf>,
    /// Optional path to the task's implementation transcript log.
    /// When set, the evaluator checks for terminal evidence markers
    /// and file-modification evidence; if gates pass but no evidence
    /// exists, the result is downgraded to `NoEvidence` instead of
    /// `Passed`.
    pub task_log: Option<PathBuf>,
    /// Optional git base commit for the task (used with --task-log to
    /// check whether any source files changed during the task window).
    pub task_git_base: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EvalScheduleReport {
    suite: String,
    interval_hours: u64,
    due: bool,
    reason: String,
    last_eval_id: Option<String>,
    last_eval_status: Option<String>,
    last_eval_ms: Option<u128>,
    next_due_ms: u128,
    now_ms: u128,
}

impl EvalScheduleReport {
    fn to_json(&self) -> Value {
        json!({
            "suite": self.suite,
            "interval_hours": self.interval_hours,
            "due": self.due,
            "reason": self.reason,
            "last_eval_id": self.last_eval_id,
            "last_eval_status": self.last_eval_status,
            "last_eval_ms": self.last_eval_ms,
            "next_due_ms": self.next_due_ms,
            "now_ms": self.now_ms,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProtocolCheckCounts {
    total: Option<u64>,
    passes: Option<u64>,
    strict: Option<u64>,
    thinking: Option<u64>,
    stream: Option<u64>,
    json: Option<u64>,
    transport: Option<u64>,
}

impl ProtocolCheckCounts {
    fn to_json(&self) -> Value {
        json!({
            "total": self.total,
            "passes": self.passes,
            "strict": self.strict,
            "thinking": self.thinking,
            "stream": self.stream,
            "json": self.json,
            "transport": self.transport,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReleaseGateReport {
    suite: String,
    max_age_hours: u64,
    ready: bool,
    reason: String,
    last_eval_id: Option<String>,
    last_eval_status: Option<String>,
    last_eval_ms: Option<u128>,
    last_eval_git_dirty: Option<bool>,
    last_eval_fixture_task_count: Option<u64>,
    last_eval_fixture_command_count: Option<u64>,
    last_eval_fixture_risk_labels: BTreeMap<String, u64>,
    last_eval_model_route_tasks: BTreeMap<String, u64>,
    last_eval_mutation_scope_failures: Option<u64>,
    last_eval_unexpected_changed_files: Option<u64>,
    min_fixture_task_count: Option<u64>,
    min_fixture_command_count: Option<u64>,
    min_fixture_risk_labels: BTreeMap<String, u64>,
    fixture_breadth_satisfied: bool,
    fixture_risk_satisfied: bool,
    missing_required_gates: Vec<String>,
    stale: bool,
    replay_failures_after_eval: usize,
    replay_command: Option<String>,
    require_protocol: bool,
    protocol_eval_id: Option<String>,
    protocol_eval_status: Option<String>,
    protocol_eval_ms: Option<u128>,
    protocol_eval_git_dirty: Option<bool>,
    protocol_check_counts: Option<ProtocolCheckCounts>,
    protocol_stale: bool,
    protocol_older_than_eval: bool,
    source_provenance_passed: bool,
    source_provenance_findings: usize,
    source_provenance_finding_summaries: Vec<String>,
    source_provenance_scan_source: String,
    source_provenance_scanned_files: usize,
    source_provenance_skipped_files: usize,
    now_ms: u128,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ReleaseGatePolicy {
    require_protocol: bool,
    min_fixture_tasks: Option<u64>,
    min_fixture_commands: Option<u64>,
    min_fixture_risk_labels: BTreeMap<String, u64>,
}

impl ReleaseGateReport {
    fn to_json(&self) -> Value {
        json!({
            "suite": self.suite,
            "max_age_hours": self.max_age_hours,
            "ready": self.ready,
            "reason": self.reason,
            "last_eval_id": self.last_eval_id,
            "last_eval_status": self.last_eval_status,
            "last_eval_ms": self.last_eval_ms,
            "last_eval_git_dirty": self.last_eval_git_dirty,
            "last_eval_fixture_task_count": self.last_eval_fixture_task_count,
            "last_eval_fixture_command_count": self.last_eval_fixture_command_count,
            "last_eval_fixture_risk_labels": self.last_eval_fixture_risk_labels,
            "last_eval_model_route_tasks": self.last_eval_model_route_tasks,
            "last_eval_mutation_scope_failures": self.last_eval_mutation_scope_failures,
            "last_eval_unexpected_changed_files": self.last_eval_unexpected_changed_files,
            "min_fixture_task_count": self.min_fixture_task_count,
            "min_fixture_command_count": self.min_fixture_command_count,
            "min_fixture_risk_labels": self.min_fixture_risk_labels,
            "fixture_breadth_satisfied": self.fixture_breadth_satisfied,
            "fixture_risk_satisfied": self.fixture_risk_satisfied,
            "missing_required_gates": self.missing_required_gates,
            "stale": self.stale,
            "replay_failures_after_eval": self.replay_failures_after_eval,
            "replay_command": self.replay_command,
            "require_protocol": self.require_protocol,
            "protocol_eval_id": self.protocol_eval_id,
            "protocol_eval_status": self.protocol_eval_status,
            "protocol_eval_ms": self.protocol_eval_ms,
            "protocol_eval_git_dirty": self.protocol_eval_git_dirty,
            "protocol_check_counts": self.protocol_check_counts.as_ref().map(ProtocolCheckCounts::to_json),
            "protocol_stale": self.protocol_stale,
            "protocol_older_than_eval": self.protocol_older_than_eval,
            "source_provenance_passed": self.source_provenance_passed,
            "source_provenance_findings": self.source_provenance_findings,
            "source_provenance_finding_summaries": self.source_provenance_finding_summaries,
            "source_provenance_scan_source": self.source_provenance_scan_source,
            "source_provenance_scanned_files": self.source_provenance_scanned_files,
            "source_provenance_skipped_files": self.source_provenance_skipped_files,
            "now_ms": self.now_ms,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FailureReplayCandidate {
    event_id: String,
    event_type: String,
    failure_class: String,
    source: String,
    run_id: Option<String>,
    timestamp_ms: u128,
    priority_score: u64,
    retryable: bool,
    priority_reasons: Vec<String>,
    signature: String,
    suggested_command: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FailureReplayReport {
    suite: String,
    total_failures: usize,
    candidates: Vec<FailureReplayCandidate>,
}

impl FailureReplayReport {
    fn to_json(&self) -> Value {
        json!({
            "suite": self.suite,
            "total_failures": self.total_failures,
            "candidates": self.candidates.iter().map(|candidate| {
                json!({
                    "event_id": candidate.event_id,
                    "event_type": candidate.event_type,
                    "failure_class": candidate.failure_class,
                    "source": candidate.source,
                    "run_id": candidate.run_id,
                    "timestamp_ms": candidate.timestamp_ms,
                    "priority_score": candidate.priority_score,
                    "retryable": candidate.retryable,
                    "priority_reasons": candidate.priority_reasons,
                    "signature": candidate.signature,
                    "suggested_command": candidate.suggested_command,
                })
            }).collect::<Vec<_>>(),
        })
    }
}

pub fn run_eval_for_patch_in(
    patch_id: &str,
    dry_run: bool,
    workdir: Option<PathBuf>,
) -> Option<EvalResult> {
    let genome = crate::deepseek::DeepSeekHarnessGenome::default();
    run_eval(EvalRunOptions {
        suite: &genome.test_policy.benchmark_subset,
        harness_version: &genome.version,
        patch_id: Some(patch_id.to_string()),
        dry_run,
        workdir,
        task_log: None,
        task_git_base: None,
    })
}

pub fn run_eval(options: EvalRunOptions<'_>) -> Option<EvalResult> {
    let genome = crate::deepseek::DeepSeekHarnessGenome::default();
    let gates = eval_gates_for_suite(options.suite, &genome);

    if options.dry_run {
        println!("Eval dry run");
        println!("  suite:           {}", options.suite);
        println!("  harness version: {}", options.harness_version);
        if let Some(patch_id) = &options.patch_id {
            println!("  patch:           {patch_id}");
        }
        if let Some(workdir) = &options.workdir {
            println!("  worktree:        {}", workdir.display());
        }
        for gate in &gates {
            println!("  gate:            {gate}");
        }
        return None;
    }

    println!("Eval run");
    println!("  suite:           {}", options.suite);
    println!("  harness version: {}", options.harness_version);
    if let Some(workdir) = &options.workdir {
        println!("  worktree:        {}", workdir.display());
    }

    let started = Instant::now();
    let started_wall_ms = now_ms();
    let mut gate_results = Vec::new();
    for gate in &gates {
        println!("  running:         {gate}");
        let result = run_gate(gate, options.workdir.as_deref());
        println!(
            "  result:          {} ({:.2}s)",
            if result.passed { "passed" } else { "failed" },
            result.duration_ms as f64 / 1000.0
        );
        gate_results.push(result);
    }

    let mut eval = build_eval_result(
        options.suite,
        options.harness_version,
        options.patch_id.clone(),
        &gate_results,
        started.elapsed().as_millis() as u64,
    );

    // If task evidence check is requested and all gates passed, verify that
    // the task actually produced verifiable evidence (source changes or
    // terminal markers).  Analysis-only tasks that pass gates but produced
    // no code are downgraded to NoEvidence so they don't inflate the
    // task_success_rate metric.
    if eval.status == EvalStatus::Passed {
        if let Some(ref task_log) = options.task_log {
            if !check_task_has_evidence(task_log, options.task_git_base.as_deref()) {
                println!("  evidence:        no source changes or terminal markers → NoEvidence");
                eval.status = EvalStatus::NoEvidence;
                // NoEvidence counts neither as passed nor failed in the
                // success-rate metric.
                eval.passed = 0;
                eval.failed = 0;
            }
        }
    }

    attach_eval_reproducibility_manifest(
        &mut eval,
        "gates",
        options.workdir.as_deref(),
        eval_run_replay_command(&options),
        gate_results
            .iter()
            .map(|gate| gate.command.clone())
            .collect(),
    );
    if let Err(e) = record_gate_failure_events(&mut eval, &gate_results) {
        eprintln!("{YELLOW}  failed to link eval gate failures into state: {e}{RESET}");
    }
    attach_eval_state_metrics(&mut eval, started_wall_ms, now_ms());
    match append_eval_result(&eval) {
        Ok(recorded) => {
            println!("  eval id:         {}", eval.eval_id);
            println!("  event id:        {}", recorded.event_id);
            println!("  artifact:        {}", recorded.artifact_path.display());
            println!("  status:          {}", status_label(&eval.status));
        }
        Err(e) => {
            eprintln!("{RED}  failed to record eval result: {e}{RESET}");
        }
    }
    Some(eval)
}

/// Check whether a task has verifiable evidence: terminal evidence markers in
/// the task transcript (`TASK_TERMINAL_EVIDENCE: changed|obsolete|blocked`) or
/// actual source file modifications (git diff since the task base commit).
///
/// Returns `true` if evidence exists, `false` if the task was analysis-only with
/// no concrete output.
fn check_task_has_evidence(task_log: &Path, task_git_base: Option<&str>) -> bool {
    // 1. Check for terminal evidence markers in the transcript.
    if let Ok(content) = std::fs::read_to_string(task_log) {
        if let Ok(re) = regex::Regex::new(r"TASK_TERMINAL_EVIDENCE:\s*(changed|obsolete|blocked)") {
            if re.is_match(&content) {
                return true;
            }
        }
    }

    // 2. If task_git_base is provided, check whether any source file
    //    actually changed during the task window.
    if let Some(base) = task_git_base {
        // Unstaged changes vs the task base.
        if let Ok(output) = Command::new("git")
            .args([
                "diff",
                "--name-only",
                base,
                "HEAD",
                "--",
                "src/",
                "Cargo.toml",
                "Cargo.lock",
            ])
            .output()
        {
            if output.status.success() && !String::from_utf8_lossy(&output.stdout).trim().is_empty()
            {
                return true;
            }
        }
        // Staged changes — may include newly added files.
        if let Ok(output) = Command::new("git")
            .args([
                "diff",
                "--cached",
                "--name-only",
                "--",
                "src/",
                "Cargo.toml",
                "Cargo.lock",
            ])
            .output()
        {
            if output.status.success() && !String::from_utf8_lossy(&output.stdout).trim().is_empty()
            {
                return true;
            }
        }
        // Untracked source files.
        if let Ok(output) = Command::new("git")
            .args([
                "ls-files",
                "--others",
                "--exclude-standard",
                "--",
                "src/",
                "Cargo.toml",
                "Cargo.lock",
            ])
            .output()
        {
            if output.status.success() && !String::from_utf8_lossy(&output.stdout).trim().is_empty()
            {
                return true;
            }
        }
    }

    false
}

fn eval_gates_for_suite(
    suite: &str,
    genome: &crate::deepseek::DeepSeekHarnessGenome,
) -> Vec<String> {
    if eval_type_for_suite(suite, None) == "protocol" {
        genome.test_policy.protocol_gates.clone()
    } else {
        genome.test_policy.required_gates.clone()
    }
}

fn handle_report(eval_id: &str) {
    let Ok(events) = read_events(&default_events_path()) else {
        eprintln!("{YELLOW}  no state log found at .yoyo/state/events.jsonl{RESET}");
        return;
    };
    match find_eval(&events, eval_id) {
        Some(eval) => println!("{}", format_eval_report(&eval)),
        None => eprintln!("{YELLOW}  no eval result found for '{eval_id}'{RESET}"),
    }
}

fn handle_compare(baseline_id: &str, candidate_id: &str) {
    let Ok(events) = read_events(&default_events_path()) else {
        eprintln!("{YELLOW}  no state log found at .yoyo/state/events.jsonl{RESET}");
        return;
    };
    let Some(baseline) = find_eval(&events, baseline_id) else {
        eprintln!("{YELLOW}  no eval result found for '{baseline_id}'{RESET}");
        return;
    };
    let Some(candidate) = find_eval(&events, candidate_id) else {
        eprintln!("{YELLOW}  no eval result found for '{candidate_id}'{RESET}");
        return;
    };
    println!("{}", format_eval_compare(&baseline, &candidate));
}

#[derive(Debug, Clone)]
struct GateResult {
    command: String,
    passed: bool,
    status_code: Option<i32>,
    duration_ms: u64,
    stdout_preview: String,
    stderr_preview: String,
}

fn run_gate(command: &str, workdir: Option<&Path>) -> GateResult {
    let started = Instant::now();
    let mut cmd = Command::new("/bin/sh");
    cmd.arg("-lc").arg(command);
    if let Some(workdir) = workdir {
        cmd.current_dir(workdir);
    }
    let output = cmd.output();
    match output {
        Ok(output) => GateResult {
            command: command.to_string(),
            passed: output.status.success(),
            status_code: output.status.code(),
            duration_ms: started.elapsed().as_millis() as u64,
            stdout_preview: preview(&String::from_utf8_lossy(&output.stdout), 2000),
            stderr_preview: preview(&String::from_utf8_lossy(&output.stderr), 2000),
        },
        Err(e) => GateResult {
            command: command.to_string(),
            passed: false,
            status_code: None,
            duration_ms: started.elapsed().as_millis() as u64,
            stdout_preview: String::new(),
            stderr_preview: e.to_string(),
        },
    }
}

fn build_eval_result(
    suite: &str,
    harness_version: &str,
    patch_id: Option<String>,
    gates: &[GateResult],
    total_duration_ms: u64,
) -> EvalResult {
    let passed = gates.iter().filter(|gate| gate.passed).count() as u64;
    let failed = gates.iter().filter(|gate| !gate.passed).count() as u64;
    let status = if failed == 0 {
        EvalStatus::Passed
    } else {
        EvalStatus::Failed
    };
    let score = if gates.is_empty() {
        Some(0.0)
    } else {
        Some(passed as f64 / gates.len() as f64)
    };
    let eval_type = eval_type_for_suite(suite, patch_id.as_deref());

    EvalResult {
        eval_id: format!("eval-{}-{}", now_ms(), std::process::id()),
        harness_version: harness_version.to_string(),
        patch_id,
        suite: suite.to_string(),
        status,
        score,
        passed,
        failed,
        metrics: json!({
            "eval_type": eval_type,
            "total_duration_ms": total_duration_ms,
            "gates": gates.iter().map(gate_to_json).collect::<Vec<_>>(),
        }),
        failure_event_ids: Vec::new(),
        created_at_ms: now_ms(),
    }
}

fn build_fixture_eval_result(
    suite: &str,
    harness_version: &str,
    results: &[crate::eval_fixtures::FixtureTaskResult],
    total_duration_ms: u64,
) -> EvalResult {
    let passed = results.iter().filter(|result| result.passed).count() as u64;
    let failed = results.iter().filter(|result| !result.passed).count() as u64;
    EvalResult {
        eval_id: format!("eval-{}-{}", now_ms(), std::process::id()),
        harness_version: harness_version.to_string(),
        patch_id: None,
        suite: format!("fixtures:{suite}"),
        status: if failed == 0 {
            EvalStatus::Passed
        } else {
            EvalStatus::Failed
        },
        score: if results.is_empty() {
            Some(0.0)
        } else {
            Some(passed as f64 / results.len() as f64)
        },
        passed,
        failed,
        metrics: json!({
            "eval_type": "coding_task",
            "total_duration_ms": total_duration_ms,
            "fixture_tasks": results,
        }),
        failure_event_ids: Vec::new(),
        created_at_ms: now_ms(),
    }
}

fn build_fixture_agent_attempt_eval_result(
    suite: &str,
    harness_version: &str,
    results: &[crate::eval_fixtures::FixtureAgentAttemptResult],
    total_duration_ms: u64,
) -> EvalResult {
    let passed = results.iter().filter(|result| result.passed).count() as u64;
    let failed = results.iter().filter(|result| !result.passed).count() as u64;
    EvalResult {
        eval_id: format!("eval-{}-{}", now_ms(), std::process::id()),
        harness_version: harness_version.to_string(),
        patch_id: None,
        suite: format!("fixture-attempts:{suite}"),
        status: if failed == 0 {
            EvalStatus::Passed
        } else {
            EvalStatus::Failed
        },
        score: if results.is_empty() {
            Some(0.0)
        } else {
            Some(passed as f64 / results.len() as f64)
        },
        passed,
        failed,
        metrics: json!({
            "eval_type": "coding_task",
            "total_duration_ms": total_duration_ms,
            "fixture_agent_attempts": results,
        }),
        failure_event_ids: Vec::new(),
        created_at_ms: now_ms(),
    }
}

fn attach_fixture_suite_metadata(
    eval: &mut EvalResult,
    tasks: &[&crate::eval_fixtures::BenchmarkTask],
) {
    let mut categories = BTreeMap::<String, usize>::new();
    let mut risks = BTreeMap::<String, usize>::new();
    let mut command_count = 0usize;
    let task_metadata = tasks
        .iter()
        .map(|task| {
            *categories.entry(task.category.clone()).or_default() += 1;
            *risks.entry(task.risk_label.clone()).or_default() += 1;
            command_count += task.tests.len();
            json!({
                "task_id": task.task_id.as_str(),
                "category": task.category.as_str(),
                "risk_label": task.risk_label.as_str(),
                "test_count": task.tests.len(),
                "expected_file_count": task.expected_files.len(),
            })
        })
        .collect::<Vec<_>>();
    let metadata = json!({
        "task_count": tasks.len(),
        "command_count": command_count,
        "categories": categories,
        "risk_labels": risks,
        "tasks": task_metadata,
    });

    if let Some(metrics) = eval.metrics.as_object_mut() {
        metrics.insert("fixture_suite".to_string(), metadata);
    }
}

fn default_state_recorder() -> StateRecorder {
    StateRecorder::new(StateConfig {
        enabled: true,
        fail_soft: false,
        events_path: default_events_path(),
        store_path: default_store_path(),
    })
}

fn record_gate_failure_events(eval: &mut EvalResult, gates: &[GateResult]) -> Result<(), String> {
    let recorder = default_state_recorder();
    record_gate_failure_events_with(eval, gates, &recorder)
}

fn record_gate_failure_events_with(
    eval: &mut EvalResult,
    gates: &[GateResult],
    recorder: &StateRecorder,
) -> Result<(), String> {
    for gate in gates.iter().filter(|gate| !gate.passed) {
        let event_id = recorder.append(
            EventType::FailureObserved,
            Actor::Harness,
            json!({
                "source": "eval_gate",
                "eval_id": eval.eval_id,
                "suite": eval.suite,
                "harness_version": eval.harness_version,
                "patch_id": eval.patch_id,
                "command": gate.command,
                "status_code": gate.status_code,
                "duration_ms": gate.duration_ms,
                "stdout_preview": gate.stdout_preview,
                "stderr_preview": gate.stderr_preview,
                "error_preview": failure_preview(&gate.stderr_preview, &gate.stdout_preview),
            }),
        )?;
        eval.failure_event_ids.push(event_id);
    }
    Ok(())
}

fn record_fixture_failure_events(
    eval: &mut EvalResult,
    results: &[crate::eval_fixtures::FixtureTaskResult],
) -> Result<(), String> {
    let recorder = default_state_recorder();
    record_fixture_failure_events_with(eval, results, &recorder)
}

fn record_fixture_failure_events_with(
    eval: &mut EvalResult,
    results: &[crate::eval_fixtures::FixtureTaskResult],
    recorder: &StateRecorder,
) -> Result<(), String> {
    for result in results.iter().filter(|result| !result.passed) {
        let failed_commands = result
            .command_results
            .iter()
            .filter(|command| !command.passed)
            .map(fixture_command_failure_json)
            .collect::<Vec<_>>();
        let error_preview = failed_commands
            .first()
            .and_then(|command| command.get("error_preview"))
            .and_then(Value::as_str)
            .unwrap_or("fixture task failed")
            .to_string();
        let event_id = recorder.append(
            EventType::FailureObserved,
            Actor::Harness,
            json!({
                "source": "eval_fixture_task",
                "eval_id": eval.eval_id,
                "suite": eval.suite,
                "harness_version": eval.harness_version,
                "patch_id": eval.patch_id,
                "task_id": result.task_id,
                "failed_commands": failed_commands,
                "error_preview": error_preview,
            }),
        )?;
        eval.failure_event_ids.push(event_id);
    }
    Ok(())
}

fn record_fixture_agent_attempt_failure_events(
    eval: &mut EvalResult,
    results: &[crate::eval_fixtures::FixtureAgentAttemptResult],
) -> Result<(), String> {
    let recorder = default_state_recorder();
    record_fixture_agent_attempt_failure_events_with(eval, results, &recorder)
}

fn record_fixture_agent_attempt_failure_events_with(
    eval: &mut EvalResult,
    results: &[crate::eval_fixtures::FixtureAgentAttemptResult],
    recorder: &StateRecorder,
) -> Result<(), String> {
    for result in results.iter().filter(|result| !result.passed) {
        let failed_commands = result
            .command_results
            .iter()
            .filter(|command| !command.passed)
            .map(fixture_command_failure_json)
            .collect::<Vec<_>>();
        let error_preview = if !result.agent_result.passed {
            failure_preview(
                &result.agent_result.stderr_preview,
                &result.agent_result.stdout_preview,
            )
        } else if !result.mutation_scope_passed {
            format!(
                "fixture agent mutated unexpected files: {}",
                result.unexpected_changed_files.join(", ")
            )
        } else {
            failed_commands
                .first()
                .and_then(|command| command.get("error_preview"))
                .and_then(Value::as_str)
                .unwrap_or("fixture agent attempt failed")
                .to_string()
        };
        let event_id = recorder.append(
            EventType::FailureObserved,
            Actor::Harness,
            json!({
                "source": "eval_fixture_agent_attempt",
                "eval_id": eval.eval_id,
                "suite": eval.suite,
                "harness_version": eval.harness_version,
                "patch_id": eval.patch_id,
                "task_id": result.task_id,
                "worktree": result.worktree,
                "agent_command": result.agent_command,
                "agent_passed": result.agent_result.passed,
                "agent_status_code": result.agent_result.status_code,
                "mutation_scope_passed": result.mutation_scope_passed,
                "changed_files": result.changed_files,
                "changed_file_count": result.changed_files.len(),
                "unexpected_changed_files": result.unexpected_changed_files,
                "unexpected_changed_file_count": result.unexpected_changed_files.len(),
                "failed_commands": failed_commands,
                "error_preview": error_preview,
            }),
        )?;
        eval.failure_event_ids.push(event_id);
    }
    Ok(())
}

fn fixture_command_failure_json(command: &crate::eval_fixtures::FixtureCommandResult) -> Value {
    json!({
        "command": command.command,
        "status_code": command.status_code,
        "duration_ms": command.duration_ms,
        "stdout_preview": command.stdout_preview,
        "stderr_preview": command.stderr_preview,
        "error_preview": failure_preview(&command.stderr_preview, &command.stdout_preview),
    })
}

fn attach_eval_reproducibility_manifest(
    eval: &mut EvalResult,
    mode: &str,
    worktree: Option<&Path>,
    replay_command: String,
    commands: Vec<String>,
) {
    let eval_type = eval_type_for_eval(eval);
    let manifest = json!({
        "manifest_version": 1,
        "mode": mode,
        "eval_type": eval_type,
        "suite": eval.suite,
        "harness_version": eval.harness_version,
        "patch_id": eval.patch_id,
        "git_commit": current_git_commit(worktree),
        "git_dirty": current_git_dirty(worktree),
        "git_status_short": current_git_status_short(worktree, 12),
        "worktree": worktree.map(|path| path.display().to_string()),
        "replay_command": replay_command,
        "commands": commands,
    });
    if let Some(metrics) = eval.metrics.as_object_mut() {
        metrics.insert("reproducibility".to_string(), manifest);
    } else {
        eval.metrics = json!({
            "reproducibility": manifest,
            "value": eval.metrics,
        });
    }
}

fn attach_fixture_agent_attempt_source(eval: &mut EvalResult, source: &str) {
    if let Some(manifest) = eval
        .metrics
        .get_mut("reproducibility")
        .and_then(Value::as_object_mut)
    {
        manifest.insert(
            "agent_command_source".to_string(),
            Value::String(source.to_string()),
        );
    }
}

fn eval_type_for_eval(eval: &EvalResult) -> String {
    eval.metrics
        .get("eval_type")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| eval_type_for_suite(&eval.suite, eval.patch_id.as_deref()).to_string())
}

fn eval_type_for_suite(suite: &str, patch_id: Option<&str>) -> &'static str {
    if patch_id.is_some() {
        return "self_evolution";
    }
    let normalized = suite.trim().to_ascii_lowercase();
    if normalized.starts_with("fixtures:") || normalized.starts_with("fixture-attempts:") {
        return "coding_task";
    }
    if normalized.contains("protocol") {
        return "protocol";
    }
    if normalized.contains("replay") || normalized.contains("regression") {
        return "regression";
    }
    "harness"
}

fn eval_run_replay_command(options: &EvalRunOptions<'_>) -> String {
    let mut command = format!(
        "yoyo eval run --suite {} --harness-version {}",
        shell_quote(options.suite),
        shell_quote(options.harness_version)
    );
    if let Some(patch_id) = &options.patch_id {
        command.push_str(&format!(" --patch-id {}", shell_quote(patch_id)));
    }
    if let Some(workdir) = &options.workdir {
        command.push_str(&format!(
            " --worktree {}",
            shell_quote(&workdir.display().to_string())
        ));
    }
    command
}

fn fixture_result_commands(results: &[crate::eval_fixtures::FixtureTaskResult]) -> Vec<String> {
    results
        .iter()
        .flat_map(|result| result.command_results.iter())
        .map(|command| command.command.clone())
        .collect()
}

fn fixture_agent_attempt_commands(
    results: &[crate::eval_fixtures::FixtureAgentAttemptResult],
) -> Vec<String> {
    let mut commands = Vec::new();
    for result in results {
        commands.push(result.agent_command.clone());
        commands.extend(
            result
                .command_results
                .iter()
                .map(|command| command.command.clone()),
        );
    }
    commands
}

fn current_git_commit(worktree: Option<&Path>) -> Option<String> {
    let mut command = Command::new("git");
    if let Some(worktree) = worktree {
        command.arg("-C").arg(worktree);
    }
    command.args(["rev-parse", "HEAD"]);
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!commit.is_empty()).then_some(commit)
}

fn current_git_status_short(worktree: Option<&Path>, limit: usize) -> Vec<String> {
    let mut command = Command::new("git");
    if let Some(worktree) = worktree {
        command.arg("-C").arg(worktree);
    }
    command.args(["status", "--short"]);
    let Ok(output) = command.output() else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .take(limit)
        .map(ToString::to_string)
        .collect()
}

fn current_git_dirty(worktree: Option<&Path>) -> Option<bool> {
    let mut command = Command::new("git");
    if let Some(worktree) = worktree {
        command.arg("-C").arg(worktree);
    }
    command.args(["status", "--porcelain"]);
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(!output.stdout.is_empty())
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/' | ':'))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn failure_preview(stderr: &str, stdout: &str) -> String {
    if !stderr.trim().is_empty() {
        preview(stderr, 400)
    } else if !stdout.trim().is_empty() {
        preview(stdout, 400)
    } else {
        "command failed without output".to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecordedEval {
    event_id: String,
    artifact_path: PathBuf,
}

fn append_eval_result(eval: &EvalResult) -> Result<RecordedEval, String> {
    let events_path = default_events_path();
    let (eval, artifact_path) = attach_eval_artifact(eval, &events_path)?;
    let recorder = StateRecorder::new(StateConfig {
        enabled: true,
        fail_soft: false,
        events_path,
        store_path: default_store_path(),
    });
    let event_id = recorder.append(
        EventType::PatchEvaluated,
        Actor::Harness,
        serde_json::to_value(&eval).map_err(|e| format!("serialize eval result: {e}"))?,
    )?;
    Ok(RecordedEval {
        event_id,
        artifact_path,
    })
}

fn attach_eval_artifact(
    eval: &EvalResult,
    events_path: &Path,
) -> Result<(EvalResult, PathBuf), String> {
    let mut eval = eval.clone();
    let artifact_path = eval_artifact_path(events_path, &eval.eval_id);
    let artifact_uri = artifact_path.display().to_string();
    let artifact_meta = json!({
        "kind": "eval_report",
        "uri": artifact_uri.clone(),
        "format": "json",
    });
    if let Some(metrics) = eval.metrics.as_object_mut() {
        metrics.insert("artifact_uri".to_string(), Value::String(artifact_uri));
        metrics.insert("artifacts".to_string(), Value::Array(vec![artifact_meta]));
    } else {
        eval.metrics = json!({
            "artifact_uri": artifact_uri,
            "artifacts": [artifact_meta],
            "value": eval.metrics,
        });
    }
    let eval = redact_eval_result(&eval)?;

    if let Some(parent) = artifact_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create eval artifact directory '{}': {e}", parent.display()))?;
    }
    std::fs::write(
        &artifact_path,
        serde_json::to_vec_pretty(&eval).map_err(|e| format!("serialize eval artifact: {e}"))?,
    )
    .map_err(|e| format!("write eval artifact '{}': {e}", artifact_path.display()))?;

    Ok((eval, artifact_path))
}

fn redact_eval_result(eval: &EvalResult) -> Result<EvalResult, String> {
    let value = serde_json::to_value(eval).map_err(|e| format!("serialize eval result: {e}"))?;
    serde_json::from_value(crate::state::redact_state_payload(&value))
        .map_err(|e| format!("deserialize redacted eval result: {e}"))
}

fn eval_artifact_path(events_path: &Path, eval_id: &str) -> PathBuf {
    events_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("artifacts")
        .join("evals")
        .join(format!("{}.json", sanitize_artifact_segment(eval_id)))
}

fn sanitize_artifact_segment(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if sanitized.is_empty() {
        "eval".to_string()
    } else {
        sanitized
    }
}

fn read_events(path: &Path) -> Result<Vec<Value>, std::io::Error> {
    crate::state::read_compatibility_events(path).map_err(std::io::Error::other)
}

fn find_eval(events: &[Value], eval_id: &str) -> Option<EvalResult> {
    let mut latest_matching_event: Option<(u128, EvalResult)> = None;
    for event in events {
        let Some(payload) = event.get("payload") else {
            continue;
        };
        let Ok(eval) = serde_json::from_value::<EvalResult>(payload.clone()) else {
            continue;
        };
        if eval.eval_id == eval_id {
            return Some(eval);
        }
        if eval_event_matches_reference(event, eval_id) {
            let timestamp = event_timestamp_ms(event);
            if latest_matching_event
                .as_ref()
                .map(|(existing, _)| timestamp >= *existing)
                .unwrap_or(true)
            {
                latest_matching_event = Some((timestamp, eval));
            }
        }
    }
    latest_matching_event.map(|(_, eval)| eval)
}

fn eval_event_matches_reference(event: &Value, reference: &str) -> bool {
    ["event_id", "run_id", "trace_id"]
        .iter()
        .any(|key| event.get(*key).and_then(Value::as_str) == Some(reference))
}

fn format_json_count_object(items: &serde_json::Map<String, Value>) -> String {
    let labels = items
        .iter()
        .filter_map(|(key, value)| value.as_u64().map(|count| format!("{key}={count}")))
        .collect::<Vec<_>>();
    if labels.is_empty() {
        "-".to_string()
    } else {
        labels.join(", ")
    }
}

fn format_count_map(items: &BTreeMap<String, u64>) -> String {
    if items.is_empty() {
        return "-".to_string();
    }
    items
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_optional_u64(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn format_eval_report(eval: &EvalResult) -> String {
    let mut out = String::new();
    out.push_str(&format!("Eval report: {}\n", eval.eval_id));
    out.push_str(&format!("harness: {}\n", eval.harness_version));
    out.push_str(&format!("suite:   {}\n", eval.suite));
    out.push_str(&format!("type:    {}\n", eval_type_for_eval(eval)));
    out.push_str(&format!("status:  {}\n", status_label(&eval.status)));
    out.push_str(&format!(
        "score:   {}\n",
        eval.score
            .map(|score| format!("{score:.3}"))
            .unwrap_or_else(|| "-".to_string())
    ));
    out.push_str(&format!("passed:  {}\n", eval.passed));
    out.push_str(&format!("failed:  {}", eval.failed));
    if let Some(artifact_uri) = eval_artifact_uri(eval) {
        out.push_str(&format!("\nartifact: {artifact_uri}"));
    }
    if let Some(manifest) = eval_reproducibility_manifest(eval) {
        out.push_str("\nreproducibility:");
        if let Some(mode) = manifest.get("mode").and_then(Value::as_str) {
            out.push_str(&format!("\n  mode:     {mode}"));
        }
        if let Some(agent_source) = manifest.get("agent_command_source").and_then(Value::as_str) {
            out.push_str(&format!("\n  agent source: {agent_source}"));
        }
        if let Some(replay) = manifest.get("replay_command").and_then(Value::as_str) {
            out.push_str(&format!("\n  replay:   {replay}"));
        }
        if let Some(dirty) = manifest.get("git_dirty").and_then(Value::as_bool) {
            out.push_str(&format!(
                "\n  git dirty: {}",
                if dirty { "yes" } else { "no" }
            ));
        }
        if let Some(status) = manifest.get("git_status_short").and_then(Value::as_array) {
            if !status.is_empty() {
                out.push_str(&format!("\n  git status lines: {}", status.len()));
                for line in status.iter().filter_map(Value::as_str).take(6) {
                    out.push_str(&format!("\n  - {line}"));
                }
                if status.len() > 6 {
                    out.push_str(&format!("\n  - ... {} more", status.len() - 6));
                }
            }
        }
        if let Some(commands) = manifest.get("commands").and_then(Value::as_array) {
            out.push_str(&format!("\n  commands: {}", commands.len()));
            for command in commands.iter().filter_map(Value::as_str).take(6) {
                out.push_str(&format!("\n  - {command}"));
            }
            if commands.len() > 6 {
                out.push_str(&format!("\n  - ... {} more", commands.len() - 6));
            }
        }
    }
    if let Some(fixture_suite) = eval.metrics.get("fixture_suite") {
        out.push_str("\nfixture suite:");
        if let Some(task_count) = fixture_suite.get("task_count").and_then(Value::as_u64) {
            out.push_str(&format!("\n  tasks:    {task_count}"));
        }
        if let Some(command_count) = fixture_suite.get("command_count").and_then(Value::as_u64) {
            out.push_str(&format!("\n  commands: {command_count}"));
        }
        if let Some(categories) = fixture_suite.get("categories").and_then(Value::as_object) {
            out.push_str(&format!(
                "\n  categories: {}",
                format_json_count_object(categories)
            ));
        }
        if let Some(risks) = fixture_suite.get("risk_labels").and_then(Value::as_object) {
            out.push_str(&format!(
                "\n  risks:    {}",
                format_json_count_object(risks)
            ));
        }
    }
    if let Some(attempts) = eval
        .metrics
        .get("fixture_agent_attempts")
        .and_then(Value::as_array)
    {
        let mut changed_files = BTreeSet::<String>::new();
        let mut unexpected_changed_files = BTreeSet::<String>::new();
        for attempt in attempts {
            if let Some(files) = attempt.get("changed_files").and_then(Value::as_array) {
                changed_files.extend(files.iter().filter_map(Value::as_str).map(str::to_string));
            }
            if let Some(files) = attempt
                .get("unexpected_changed_files")
                .and_then(Value::as_array)
            {
                unexpected_changed_files
                    .extend(files.iter().filter_map(Value::as_str).map(str::to_string));
            }
        }
        if !changed_files.is_empty() {
            out.push_str("\nfixture agent patch:");
            out.push_str(&format!("\n  changed files: {}", changed_files.len()));
            for file in changed_files.iter().take(6) {
                out.push_str(&format!("\n  - {file}"));
            }
            if changed_files.len() > 6 {
                out.push_str(&format!("\n  - ... {} more", changed_files.len() - 6));
            }
            if !unexpected_changed_files.is_empty() {
                out.push_str(&format!(
                    "\n  unexpected files: {}",
                    unexpected_changed_files.len()
                ));
                for file in unexpected_changed_files.iter().take(6) {
                    out.push_str(&format!("\n  ! {file}"));
                }
                if unexpected_changed_files.len() > 6 {
                    out.push_str(&format!(
                        "\n  ! ... {} more",
                        unexpected_changed_files.len() - 6
                    ));
                }
            }
        }
    }
    if let Some(metrics) = eval_state_metrics(eval) {
        out.push_str("\nstate metrics:");
        out.push_str(&format!(
            "\n  model/tool calls: {}/{}",
            metric_u64(metrics, "model_calls"),
            metric_u64(metrics, "tool_calls")
        ));
        out.push_str(&format!(
            "\n  commands/tests:   {}/{}",
            metric_u64(metrics, "command_runs"),
            metric_u64(metrics, "test_runs")
        ));
        out.push_str(&format!(
            "\n  failures/edits:   {}/{}",
            metric_u64(metrics, "failures"),
            metric_u64(metrics, "file_edits")
        ));
        out.push_str(&format!(
            "\n  repair loops:     {}",
            metric_u64(metrics, "repair_loop_count")
        ));
        out.push_str(&format!(
            "\n  failure classes:  json={} schema={} context_miss={} rollbacks={}",
            metric_u64(metrics, "json_output_failures"),
            metric_u64(metrics, "tool_schema_failures"),
            metric_u64(metrics, "context_miss_failures"),
            metric_u64(metrics, "rollback_count")
        ));
        out.push_str(&format!(
            "\n  quality rates:    malformed_tool={} json_parse={} context_miss={} permission={} human={}",
            format_optional_percent(metrics, "malformed_tool_call_rate"),
            format_optional_percent(metrics, "json_parse_failure_rate"),
            format_optional_percent(metrics, "context_miss_rate"),
            format_optional_percent(metrics, "permission_prompt_rate"),
            format_optional_percent(metrics, "human_intervention_rate")
        ));
        out.push_str(&format!(
            "\n  deepseek modes:   thinking={}/{} ({}) fim={}/{} ({})",
            metric_u64(metrics, "thinking_model_calls"),
            metric_u64(metrics, "model_calls"),
            format_optional_percent(metrics, "thinking_mode_usage_rate"),
            metric_u64(metrics, "fim_successes"),
            metric_u64(metrics, "fim_attempts"),
            format_optional_percent(metrics, "fim_success_rate")
        ));
        if let Some(routes) = metrics.get("model_route_tasks").and_then(Value::as_object) {
            if !routes.is_empty() {
                out.push_str(&format!(
                    "\n  model routes:     {}",
                    format_json_count_object(routes)
                ));
            }
        }
        out.push_str(&format!(
            "\n  fim quality:      compile={}/{} ({}) rollback={} ({}) token_savings={}",
            metric_u64(metrics, "fim_compile_successes"),
            metric_u64(metrics, "fim_compile_checks"),
            format_optional_percent(metrics, "fim_compile_rate"),
            metric_u64(metrics, "fim_rollbacks"),
            format_optional_percent(metrics, "fim_rollback_rate"),
            metric_u64(metrics, "fim_token_savings")
        ));
        if metric_u64(metrics, "fixture_agent_attempts") > 0 {
            out.push_str(&format!(
                "\n  fixture agent:   attempts={} scope_failures={} unexpected_files={} scope_rate={}",
                metric_u64(metrics, "fixture_agent_attempts"),
                metric_u64(metrics, "fixture_agent_mutation_scope_failures"),
                metric_u64(metrics, "fixture_agent_unexpected_changed_file_count"),
                format_optional_percent(metrics, "fixture_agent_mutation_scope_failure_rate")
            ));
        }
        if metric_u64(metrics, "deepseek_protocol_checks") > 0 {
            out.push_str(&format!(
                "\n  protocol checks: strict={} thinking={} stream={} json={} transport={} passes={}/{}",
                metric_u64(metrics, "deepseek_strict_tool_call_checks"),
                metric_u64(metrics, "deepseek_thinking_protocol_checks"),
                metric_u64(metrics, "deepseek_streaming_protocol_checks"),
                metric_u64(metrics, "deepseek_json_output_checks"),
                metric_u64(metrics, "deepseek_transport_policy_checks"),
                metric_u64(metrics, "deepseek_protocol_passes"),
                metric_u64(metrics, "deepseek_protocol_checks")
            ));
        }
        out.push_str(&format!(
            "\n  tokens in/out:    {}/{}",
            metric_u64(metrics, "input_tokens"),
            metric_u64(metrics, "output_tokens")
        ));
        out.push_str(&format!(
            "\n  cache hit ratio:  {}",
            metric_f64(metrics, "cache_hit_ratio")
                .map(|value| format!("{:.2}%", value * 100.0))
                .unwrap_or_else(|| "-".to_string())
        ));
        out.push_str(&format!(
            "\n  cost/latency:     ${:.6}/{}ms",
            metric_f64(metrics, "cost_usd").unwrap_or_default(),
            metric_u64(metrics, "latency_ms")
        ));
        out.push_str(&format!(
            "\n  cost per pass:   {}",
            metric_f64(metrics, "cost_per_successful_task_usd")
                .map(|value| format!("${value:.6}"))
                .unwrap_or_else(|| "-".to_string())
        ));
        out.push_str(&format!(
            "\n  latency per pass:{}",
            metric_u64_opt(metrics, "latency_per_successful_task_ms")
                .map(|value| format!(" {value}ms"))
                .unwrap_or_else(|| " -".to_string())
        ));
    }
    out
}

fn eval_artifact_uri(eval: &EvalResult) -> Option<&str> {
    eval.metrics
        .get("artifact_uri")
        .and_then(|value| value.as_str())
}

fn eval_reproducibility_manifest(eval: &EvalResult) -> Option<&Value> {
    eval.metrics.get("reproducibility")
}

fn format_eval_compare(baseline: &EvalResult, candidate: &EvalResult) -> String {
    let base_score = baseline.score.unwrap_or(0.0);
    let candidate_score = candidate.score.unwrap_or(0.0);
    let delta = candidate_score - base_score;
    let mut out = format!(
        "Eval compare\n  baseline:  {} score={base_score:.3} passed={} failed={}\n  candidate: {} score={candidate_score:.3} passed={} failed={}\n  delta:     {delta:+.3}",
        baseline.eval_id,
        baseline.passed,
        baseline.failed,
        candidate.eval_id,
        candidate.passed,
        candidate.failed
    );
    if let Some(line) = format_eval_compare_fixture_suite(baseline, candidate) {
        out.push_str(&line);
    }
    if let (Some(base_metrics), Some(candidate_metrics)) =
        (eval_state_metrics(baseline), eval_state_metrics(candidate))
    {
        out.push_str("\n  metric deltas:");
        out.push_str(&format!(
            "\n    model calls:       {}",
            format_u64_delta(base_metrics, candidate_metrics, "model_calls")
        ));
        out.push_str(&format!(
            "\n    tool calls:        {}",
            format_u64_delta(base_metrics, candidate_metrics, "tool_calls")
        ));
        out.push_str(&format!(
            "\n    failures:          {}",
            format_u64_delta(base_metrics, candidate_metrics, "failures")
        ));
        out.push_str(&format!(
            "\n    schema failures:   {}",
            format_u64_delta(base_metrics, candidate_metrics, "tool_schema_failures")
        ));
        out.push_str(&format!(
            "\n    json failures:     {}",
            format_u64_delta(base_metrics, candidate_metrics, "json_output_failures")
        ));
        out.push_str(&format!(
            "\n    context misses:    {}",
            format_u64_delta(base_metrics, candidate_metrics, "context_miss_failures")
        ));
        out.push_str(&format!(
            "\n    repair loops:      {}",
            format_u64_delta(base_metrics, candidate_metrics, "repair_loop_count")
        ));
        out.push_str(&format!(
            "\n    malformed tools:   {}",
            format_percent_delta(base_metrics, candidate_metrics, "malformed_tool_call_rate")
        ));
        out.push_str(&format!(
            "\n    json parse rate:   {}",
            format_percent_delta(base_metrics, candidate_metrics, "json_parse_failure_rate")
        ));
        out.push_str(&format!(
            "\n    thinking rate:    {}",
            format_percent_delta(base_metrics, candidate_metrics, "thinking_mode_usage_rate")
        ));
        if let Some(route_line) = format_route_task_compare(base_metrics, candidate_metrics) {
            out.push_str(&route_line);
        }
        out.push_str(&format!(
            "\n    fim success rate: {}",
            format_percent_delta(base_metrics, candidate_metrics, "fim_success_rate")
        ));
        out.push_str(&format!(
            "\n    fim compile rate: {}",
            format_percent_delta(base_metrics, candidate_metrics, "fim_compile_rate")
        ));
        out.push_str(&format!(
            "\n    fim rollback rate: {}",
            format_percent_delta(base_metrics, candidate_metrics, "fim_rollback_rate")
        ));
        out.push_str(&format!(
            "\n    fim token savings: {}",
            format_u64_delta(base_metrics, candidate_metrics, "fim_token_savings")
        ));
        out.push_str(&format!(
            "\n    fixture agent scope failures: {}",
            format_u64_delta(
                base_metrics,
                candidate_metrics,
                "fixture_agent_mutation_scope_failures"
            )
        ));
        out.push_str(&format!(
            "\n    fixture agent unexpected files: {}",
            format_u64_delta(
                base_metrics,
                candidate_metrics,
                "fixture_agent_unexpected_changed_file_count"
            )
        ));
        out.push_str(&format!(
            "\n    fixture agent scope rate: {}",
            format_percent_delta(
                base_metrics,
                candidate_metrics,
                "fixture_agent_mutation_scope_failure_rate"
            )
        ));
        out.push_str(&format!(
            "\n    permission rate: {}",
            format_percent_delta(base_metrics, candidate_metrics, "permission_prompt_rate")
        ));
        out.push_str(&format!(
            "\n    human rate:      {}",
            format_percent_delta(base_metrics, candidate_metrics, "human_intervention_rate")
        ));
        out.push_str(&format!(
            "\n    file edits:        {}",
            format_u64_delta(base_metrics, candidate_metrics, "file_edits")
        ));
        out.push_str(&format!(
            "\n    input tokens:      {}",
            format_u64_delta(base_metrics, candidate_metrics, "input_tokens")
        ));
        out.push_str(&format!(
            "\n    output tokens:     {}",
            format_u64_delta(base_metrics, candidate_metrics, "output_tokens")
        ));
        out.push_str(&format!(
            "\n    cache hit ratio:   {}",
            format_percent_delta(base_metrics, candidate_metrics, "cache_hit_ratio")
        ));
        out.push_str(&format!(
            "\n    cost:              {}",
            format_money_delta(base_metrics, candidate_metrics, "cost_usd")
        ));
        out.push_str(&format!(
            "\n    latency:           {}",
            format_u64_delta(base_metrics, candidate_metrics, "latency_ms")
        ));
        out.push_str(&format!(
            "\n    cost per pass:     {}",
            format_money_delta(
                base_metrics,
                candidate_metrics,
                "cost_per_successful_task_usd"
            )
        ));
        out.push_str(&format!(
            "\n    latency per pass:  {}",
            format_u64_delta(
                base_metrics,
                candidate_metrics,
                "latency_per_successful_task_ms"
            )
        ));
    }
    out
}

fn format_route_task_compare(base_metrics: &Value, candidate_metrics: &Value) -> Option<String> {
    let baseline = base_metrics
        .get("model_route_tasks")
        .and_then(Value::as_object);
    let candidate = candidate_metrics
        .get("model_route_tasks")
        .and_then(Value::as_object);
    if baseline.map(|items| items.is_empty()).unwrap_or(true)
        && candidate.map(|items| items.is_empty()).unwrap_or(true)
    {
        return None;
    }
    Some(format!(
        "\n    model routes:     baseline [{}] candidate [{}]",
        baseline
            .map(format_json_count_object)
            .unwrap_or_else(|| "-".to_string()),
        candidate
            .map(format_json_count_object)
            .unwrap_or_else(|| "-".to_string())
    ))
}

fn format_eval_compare_fixture_suite(
    baseline: &EvalResult,
    candidate: &EvalResult,
) -> Option<String> {
    let baseline_has_fixture_suite = baseline.metrics.get("fixture_suite").is_some();
    let candidate_has_fixture_suite = candidate.metrics.get("fixture_suite").is_some();
    if !baseline_has_fixture_suite && !candidate_has_fixture_suite {
        return None;
    }
    let baseline_counts = eval_fixture_suite_counts(baseline);
    let candidate_counts = eval_fixture_suite_counts(candidate);
    let breadth = if baseline_has_fixture_suite
        && candidate_has_fixture_suite
        && baseline_counts == candidate_counts
    {
        "matched"
    } else if baseline_has_fixture_suite && candidate_has_fixture_suite {
        "mismatch"
    } else {
        "incomplete"
    };
    Some(format!(
        "\n  fixture suite: baseline tasks={} commands={} candidate tasks={} commands={} breadth={breadth}",
        format_optional_u64(baseline_counts.0),
        format_optional_u64(baseline_counts.1),
        format_optional_u64(candidate_counts.0),
        format_optional_u64(candidate_counts.1)
    ))
}

fn build_failure_replay_report(events: &[Value], suite: &str, limit: usize) -> FailureReplayReport {
    let mut failures = events
        .iter()
        .filter(|event| is_state_failure_event(event))
        .collect::<Vec<_>>();
    failures.sort_by_key(|event| std::cmp::Reverse(event_timestamp_ms(event)));

    let total_failures = failures.len();
    let mut candidates = failures
        .into_iter()
        .filter_map(|event| {
            let event_id = event
                .get("event_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if event_id.is_empty() {
                return None;
            }
            let event_type = event
                .get("event_type")
                .and_then(Value::as_str)
                .unwrap_or("FailureObserved")
                .to_string();
            let payload = event.get("payload").unwrap_or(&Value::Null);
            let source = payload_string(payload, "source")
                .or_else(|| payload_string(payload, "operation"))
                .or_else(|| payload_string(payload, "tool_name"))
                .unwrap_or_else(|| event_type.clone());
            let signature = failure_replay_signature(payload, &event_type);
            let priority = failure_replay_priority(payload, &event_type);
            Some(FailureReplayCandidate {
                event_id: event_id.clone(),
                event_type,
                failure_class: priority.failure_class,
                source,
                run_id: event
                    .get("run_id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                timestamp_ms: event_timestamp_ms(event),
                priority_score: priority.score,
                retryable: priority.retryable,
                priority_reasons: priority.reasons,
                signature,
                suggested_command: format!(
                    "yoyo state why {event_id} && yoyo eval fixtures run --suite {suite}"
                ),
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .priority_score
            .cmp(&left.priority_score)
            .then_with(|| right.timestamp_ms.cmp(&left.timestamp_ms))
            .then_with(|| left.event_id.cmp(&right.event_id))
    });
    candidates.truncate(limit);

    FailureReplayReport {
        suite: suite.to_string(),
        total_failures,
        candidates,
    }
}

fn format_failure_replay_report(report: &FailureReplayReport) -> String {
    let mut out = String::new();
    out.push_str("Historical failure replay\n");
    out.push_str(&format!("  suite:          {}\n", report.suite));
    out.push_str(&format!("  failures found: {}\n", report.total_failures));
    out.push_str(&format!("  candidates:     {}\n", report.candidates.len()));
    for candidate in &report.candidates {
        out.push_str(&format!(
            "  - {} {} class={} priority={} retryable={} source={} run={}\n",
            candidate.event_type,
            candidate.event_id,
            candidate.failure_class,
            candidate.priority_score,
            if candidate.retryable { "yes" } else { "no" },
            candidate.source,
            candidate.run_id.as_deref().unwrap_or("-")
        ));
        if !candidate.priority_reasons.is_empty() {
            out.push_str(&format!(
                "    priority:  {}\n",
                candidate.priority_reasons.join("; ")
            ));
        }
        out.push_str(&format!("    signature: {}\n", candidate.signature));
        out.push_str(&format!("    replay:    {}\n", candidate.suggested_command));
    }
    out.trim_end().to_string()
}

fn is_state_failure_event(event: &Value) -> bool {
    matches!(
        event.get("event_type").and_then(Value::as_str),
        Some("FailureObserved" | "JsonOutputFailure" | "ToolSchemaFailure")
    )
}

fn event_timestamp_ms(event: &Value) -> u128 {
    event
        .get("timestamp_ms")
        .and_then(value_as_u128)
        .unwrap_or_default()
}

fn failure_replay_signature(payload: &Value, event_type: &str) -> String {
    let source = payload_string(payload, "source")
        .or_else(|| payload_string(payload, "operation"))
        .or_else(|| payload_string(payload, "tool_name"))
        .unwrap_or_else(|| event_type.to_string());
    let detail = payload_string(payload, "error_preview")
        .or_else(|| payload_string(payload, "failure_summary"))
        .or_else(|| payload_string(payload, "repair_instruction"))
        .unwrap_or_else(|| event_type.to_string());
    format!("{}: {}", preview(&source, 80), preview(&detail, 160))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FailureReplayPriority {
    failure_class: String,
    score: u64,
    retryable: bool,
    reasons: Vec<String>,
}

fn failure_replay_priority(payload: &Value, event_type: &str) -> FailureReplayPriority {
    let text = failure_replay_text(payload, event_type);
    let mut reasons: Vec<String> = Vec::new();
    let mut failure_class = "unknown";
    let mut score: u64 = 20;
    let mut retryable = false;

    if event_type == "ToolSchemaFailure" {
        failure_class = "tool_schema";
        score = 95;
        retryable = true;
        reasons.push(
            "strict tool schema failures usually need immediate repair-loop replay".to_string(),
        );
    } else if event_type == "JsonOutputFailure" {
        failure_class = "json_output";
        score = 90;
        retryable = true;
        reasons.push(
            "JSON output failures can be replayed against deterministic extraction policy"
                .to_string(),
        );
    } else if failure_payload_mentions_context_miss(payload) {
        failure_class = "context_miss";
        score = 85;
        retryable = true;
        reasons.push(
            "context miss evidence should be replayed with current context ranking".to_string(),
        );
    } else if failure_text_mentions_transport(&text) {
        failure_class = "transport";
        score = 70;
        retryable = true;
        reasons.push("transport-like failure can validate timeout/retry policy".to_string());
    } else if payload_is_deepseek_fim(payload) || text.contains("fim") {
        failure_class = "fim";
        score = 65;
        retryable = true;
        reasons.push("FIM failure should be replayed through guarded local-edit path".to_string());
    } else if failure_text_mentions_permission(&text) {
        failure_class = "permission";
        score = 55;
        reasons
            .push("permission failure needs policy review before replay can succeed".to_string());
    } else if failure_text_mentions_eval(&text) {
        failure_class = "eval";
        score = 50;
        retryable = true;
        reasons
            .push("eval/test failure should be replayed against current fixture suite".to_string());
    } else if failure_text_mentions_tool_execution(&text) {
        failure_class = "tool_execution";
        score = 45;
        retryable = true;
        reasons.push("tool execution failure may reproduce with current tool policy".to_string());
    } else {
        reasons.push("unclassified failure kept for historical replay coverage".to_string());
    }

    let repair_loops = payload_repair_loop_count(payload);
    if repair_loops > 0 {
        score = score.saturating_add(repair_loops.min(3) * 5);
        reasons.push(format!("repair_loop_count={repair_loops}"));
    }
    if payload_string(payload, "repair_instruction").is_some() {
        score = score.saturating_add(5);
        retryable = true;
        reasons.push("repair instruction is present".to_string());
    }
    if text.contains("regression") || text.contains("release gate") {
        score = score.saturating_add(5);
        reasons.push("release/regression signal is present".to_string());
    }

    FailureReplayPriority {
        failure_class: failure_class.to_string(),
        score,
        retryable,
        reasons,
    }
}

fn failure_replay_text(payload: &Value, event_type: &str) -> String {
    let mut parts = vec![event_type.to_string()];
    for key in [
        "source",
        "operation",
        "tool_name",
        "error_preview",
        "failure_summary",
        "repair_instruction",
        "hypothesis",
        "next_repair_step",
        "class",
        "failure_class",
        "owner",
        "status",
        "error_class",
    ] {
        if let Some(value) = payload_string(payload, key) {
            parts.push(value);
        }
    }
    parts.join(" ").to_ascii_lowercase()
}

fn failure_text_mentions_transport(text: &str) -> bool {
    [
        "transport",
        "timeout",
        "timed out",
        "retryable",
        "rate limit",
        "429",
        "500",
        "502",
        "503",
        "504",
        "network",
        "connection",
        "api failure",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn failure_text_mentions_permission(text: &str) -> bool {
    ["permission", "approval", "denied", "human approval"]
        .iter()
        .any(|needle| text.contains(needle))
}

fn failure_text_mentions_eval(text: &str) -> bool {
    ["eval", "fixture", "test failed", "verification failed"]
        .iter()
        .any(|needle| text.contains(needle))
}

fn failure_text_mentions_tool_execution(text: &str) -> bool {
    ["tool", "command failed", "exit status", "stderr"]
        .iter()
        .any(|needle| text.contains(needle))
}

fn payload_string(payload: &Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn build_eval_schedule_report(
    events: &[Value],
    suite: &str,
    interval_hours: u64,
    now_ms: u128,
) -> EvalScheduleReport {
    let interval_ms = u128::from(interval_hours).saturating_mul(3_600_000);
    let last_eval = latest_eval_for_suite(events, suite);

    let Some((last_eval_ms, eval)) = last_eval else {
        return EvalScheduleReport {
            suite: suite.to_string(),
            interval_hours,
            due: true,
            reason: "no prior eval found for suite".to_string(),
            last_eval_id: None,
            last_eval_status: None,
            last_eval_ms: None,
            next_due_ms: now_ms,
            now_ms,
        };
    };

    let next_due_ms = last_eval_ms.saturating_add(interval_ms);
    let due = now_ms >= next_due_ms;
    EvalScheduleReport {
        suite: suite.to_string(),
        interval_hours,
        due,
        reason: if due {
            "interval elapsed since last eval".to_string()
        } else {
            "last eval is still inside interval".to_string()
        },
        last_eval_id: Some(eval.eval_id),
        last_eval_status: Some(status_label(&eval.status).to_string()),
        last_eval_ms: Some(last_eval_ms),
        next_due_ms,
        now_ms,
    }
}

fn build_release_gate_report(
    events: &[Value],
    suite: &str,
    max_age_hours: u64,
    now_ms: u128,
) -> ReleaseGateReport {
    build_release_gate_report_with_options(events, suite, max_age_hours, now_ms, false)
}

fn build_release_gate_report_with_options(
    events: &[Value],
    suite: &str,
    max_age_hours: u64,
    now_ms: u128,
    require_protocol: bool,
) -> ReleaseGateReport {
    build_release_gate_report_with_policy(
        events,
        suite,
        max_age_hours,
        now_ms,
        ReleaseGatePolicy {
            require_protocol,
            min_fixture_tasks: None,
            min_fixture_commands: None,
            min_fixture_risk_labels: BTreeMap::new(),
        },
    )
}

fn build_release_gate_report_with_policy(
    events: &[Value],
    suite: &str,
    max_age_hours: u64,
    now_ms: u128,
    policy: ReleaseGatePolicy,
) -> ReleaseGateReport {
    let source_audit = release_source_provenance_audit();
    build_release_gate_report_with_policy_and_source_audit(
        events,
        suite,
        max_age_hours,
        now_ms,
        policy,
        source_audit,
    )
}

fn build_release_gate_report_with_policy_and_source_audit(
    events: &[Value],
    suite: &str,
    max_age_hours: u64,
    now_ms: u128,
    policy: ReleaseGatePolicy,
    source_audit: crate::release::SourceProvenanceAudit,
) -> ReleaseGateReport {
    let require_protocol = policy.require_protocol;
    let source_provenance_passed = source_audit.passed;
    let source_provenance_findings = source_audit.findings.len();
    let source_provenance_finding_summaries = source_provenance_finding_summaries(&source_audit);
    let source_provenance_scan_source = source_audit.scan_source.to_string();
    let source_provenance_scanned_files = source_audit.scanned_files;
    let source_provenance_skipped_files = source_audit.skipped_files;
    let max_age_ms = u128::from(max_age_hours).saturating_mul(3_600_000);
    let Some((last_eval_ms, eval)) = latest_eval_for_suite(events, suite) else {
        let fixture_risk_satisfied = policy.min_fixture_risk_labels.is_empty();
        return ReleaseGateReport {
            suite: suite.to_string(),
            max_age_hours,
            ready: false,
            reason: "no prior eval found for suite".to_string(),
            last_eval_id: None,
            last_eval_status: None,
            last_eval_ms: None,
            last_eval_git_dirty: None,
            last_eval_fixture_task_count: None,
            last_eval_fixture_command_count: None,
            last_eval_fixture_risk_labels: BTreeMap::new(),
            last_eval_model_route_tasks: BTreeMap::new(),
            last_eval_mutation_scope_failures: None,
            last_eval_unexpected_changed_files: None,
            min_fixture_task_count: policy.min_fixture_tasks,
            min_fixture_command_count: policy.min_fixture_commands,
            min_fixture_risk_labels: policy.min_fixture_risk_labels,
            fixture_breadth_satisfied: false,
            fixture_risk_satisfied,
            missing_required_gates: Vec::new(),
            stale: true,
            replay_failures_after_eval: 0,
            replay_command: None,
            require_protocol,
            protocol_eval_id: None,
            protocol_eval_status: None,
            protocol_eval_ms: None,
            protocol_eval_git_dirty: None,
            protocol_check_counts: None,
            protocol_stale: require_protocol,
            protocol_older_than_eval: false,
            source_provenance_passed,
            source_provenance_findings,
            source_provenance_finding_summaries,
            source_provenance_scan_source,
            source_provenance_scanned_files,
            source_provenance_skipped_files,
            now_ms,
        };
    };

    let stale = now_ms.saturating_sub(last_eval_ms) > max_age_ms;
    let passed = eval.status == EvalStatus::Passed && eval.failed == 0;
    let last_eval_git_dirty = eval_reproducibility_manifest(&eval)
        .and_then(|manifest| manifest.get("git_dirty")?.as_bool());
    let fixture_evidence_eval = if eval_has_fixture_suite_metadata(&eval) {
        Some((last_eval_ms, eval.clone()))
    } else {
        latest_release_fixture_evidence_eval(events, suite, max_age_ms, now_ms)
    };
    let (last_eval_fixture_task_count, last_eval_fixture_command_count) = fixture_evidence_eval
        .as_ref()
        .map(|(_, eval)| eval_fixture_suite_counts(eval))
        .unwrap_or((None, None));
    let last_eval_fixture_risk_labels = fixture_evidence_eval
        .as_ref()
        .map(|(_, eval)| eval_fixture_suite_risk_labels(eval))
        .unwrap_or_default();
    let last_eval_model_route_tasks = eval_model_route_tasks(&eval);
    let (last_eval_mutation_scope_failures, last_eval_unexpected_changed_files) =
        eval_fixture_agent_mutation_scope_counts(&eval);
    let fixture_breadth_satisfied = release_fixture_breadth_satisfied(
        last_eval_fixture_task_count,
        last_eval_fixture_command_count,
        &policy,
    );
    let fixture_risk_satisfied =
        release_fixture_risk_satisfied(&last_eval_fixture_risk_labels, &policy);
    let missing_required_gates = eval_missing_required_release_gates(&eval);
    let replay_failures_after_eval = replay_failure_count_after(events, last_eval_ms);
    let replay_command = if replay_failures_after_eval > 0 {
        Some(format!(
            "yoyo eval replay --from-state --limit {replay_failures_after_eval}"
        ))
    } else {
        None
    };
    let protocol_eval = latest_eval_by_type(events, "protocol");
    let protocol_eval_id = protocol_eval.as_ref().map(|(_, eval)| eval.eval_id.clone());
    let protocol_eval_status = protocol_eval
        .as_ref()
        .map(|(_, eval)| status_label(&eval.status).to_string());
    let protocol_eval_ms = protocol_eval.as_ref().map(|(timestamp, _)| *timestamp);
    let protocol_eval_git_dirty = protocol_eval
        .as_ref()
        .and_then(|(_, eval)| eval_reproducibility_manifest(eval))
        .and_then(|manifest| manifest.get("git_dirty")?.as_bool());
    let protocol_check_counts = protocol_eval
        .as_ref()
        .and_then(|(_, eval)| protocol_check_counts_for_eval(eval));
    let protocol_stale = protocol_eval_ms
        .map(|timestamp| now_ms.saturating_sub(timestamp) > max_age_ms)
        .unwrap_or(require_protocol);
    let protocol_passed = protocol_eval
        .as_ref()
        .map(|(_, eval)| eval.status == EvalStatus::Passed && eval.failed == 0)
        .unwrap_or(false);
    let protocol_older_than_eval = require_protocol
        && protocol_eval_ms
            .map(|timestamp| timestamp < last_eval_ms)
            .unwrap_or(false);
    let protocol_ready = !require_protocol
        || (protocol_passed
            && !protocol_stale
            && !protocol_older_than_eval
            && protocol_eval_git_dirty != Some(true));
    let ready = passed
        && !stale
        && last_eval_git_dirty != Some(true)
        && missing_required_gates.is_empty()
        && fixture_breadth_satisfied
        && fixture_risk_satisfied
        && last_eval_mutation_scope_failures.unwrap_or_default() == 0
        && last_eval_unexpected_changed_files.unwrap_or_default() == 0
        && replay_failures_after_eval == 0
        && protocol_ready
        && source_provenance_passed;
    let reason = if !passed {
        "latest eval did not pass".to_string()
    } else if stale {
        "latest eval is older than max age".to_string()
    } else if last_eval_git_dirty == Some(true) {
        "latest eval was run from a dirty worktree".to_string()
    } else if !missing_required_gates.is_empty() {
        format!(
            "latest eval is missing required gate evidence: {}",
            missing_required_gates.join(", ")
        )
    } else if !fixture_breadth_satisfied {
        format_release_fixture_breadth_reason(
            last_eval_fixture_task_count,
            last_eval_fixture_command_count,
            &policy,
        )
    } else if !fixture_risk_satisfied {
        format_release_fixture_risk_reason(&last_eval_fixture_risk_labels, &policy)
    } else if last_eval_mutation_scope_failures.unwrap_or_default() > 0 {
        format!(
            "latest eval has fixture agent mutation-scope failures: {}",
            last_eval_mutation_scope_failures.unwrap_or_default()
        )
    } else if last_eval_unexpected_changed_files.unwrap_or_default() > 0 {
        format!(
            "latest eval has unexpected fixture agent changed files: {}",
            last_eval_unexpected_changed_files.unwrap_or_default()
        )
    } else if replay_failures_after_eval > 0 {
        "state failures were recorded after latest eval".to_string()
    } else if require_protocol && protocol_eval.is_none() {
        "no protocol eval found".to_string()
    } else if require_protocol && !protocol_passed {
        "latest protocol eval did not pass".to_string()
    } else if require_protocol && protocol_stale {
        "latest protocol eval is older than max age".to_string()
    } else if require_protocol && protocol_older_than_eval {
        "latest protocol eval is older than latest suite eval".to_string()
    } else if require_protocol && protocol_eval_git_dirty == Some(true) {
        "latest protocol eval was run from a dirty worktree".to_string()
    } else if !source_provenance_passed {
        "source provenance audit did not pass".to_string()
    } else if require_protocol {
        "latest eval and protocol eval passed and are fresh".to_string()
    } else {
        "latest eval passed and is fresh".to_string()
    };

    ReleaseGateReport {
        suite: suite.to_string(),
        max_age_hours,
        ready,
        reason,
        last_eval_id: Some(eval.eval_id),
        last_eval_status: Some(status_label(&eval.status).to_string()),
        last_eval_ms: Some(last_eval_ms),
        last_eval_git_dirty,
        last_eval_fixture_task_count,
        last_eval_fixture_command_count,
        last_eval_fixture_risk_labels,
        last_eval_model_route_tasks,
        last_eval_mutation_scope_failures,
        last_eval_unexpected_changed_files,
        min_fixture_task_count: policy.min_fixture_tasks,
        min_fixture_command_count: policy.min_fixture_commands,
        min_fixture_risk_labels: policy.min_fixture_risk_labels,
        fixture_breadth_satisfied,
        fixture_risk_satisfied,
        missing_required_gates,
        stale,
        replay_failures_after_eval,
        replay_command,
        require_protocol,
        protocol_eval_id,
        protocol_eval_status,
        protocol_eval_ms,
        protocol_eval_git_dirty,
        protocol_check_counts,
        protocol_stale,
        protocol_older_than_eval,
        source_provenance_passed,
        source_provenance_findings,
        source_provenance_finding_summaries,
        source_provenance_scan_source,
        source_provenance_scanned_files,
        source_provenance_skipped_files,
        now_ms,
    }
}

fn release_source_provenance_audit() -> crate::release::SourceProvenanceAudit {
    crate::release::audit_source_provenance_repository(std::path::Path::new("."))
}

fn source_provenance_finding_summaries(
    audit: &crate::release::SourceProvenanceAudit,
) -> Vec<String> {
    audit
        .findings
        .iter()
        .take(SOURCE_PROVENANCE_FINDING_SUMMARY_LIMIT)
        .map(source_provenance_finding_summary)
        .collect()
}

fn source_provenance_finding_summary(finding: &crate::release::SourceProvenanceFinding) -> String {
    let summary = format!("{}: {}", finding.path, finding.marker);
    if summary.chars().count() <= SOURCE_PROVENANCE_FINDING_SUMMARY_MAX_CHARS {
        return summary;
    }

    let marker = compact_text_end(&finding.marker, SOURCE_PROVENANCE_FINDING_MARKER_MAX_CHARS);
    let path_budget = SOURCE_PROVENANCE_FINDING_SUMMARY_MAX_CHARS
        .saturating_sub(marker.chars().count())
        .saturating_sub(2);
    let path = compact_text_middle(&finding.path, path_budget);
    let compact = format!("{path}: {marker}");
    compact_text_end(&compact, SOURCE_PROVENANCE_FINDING_SUMMARY_MAX_CHARS)
}

fn compact_text_end(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }
    let mut compact = text
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    compact.push_str("...");
    compact
}

fn compact_text_middle(text: &str, max_chars: usize) -> String {
    let chars = text.chars().collect::<Vec<_>>();
    if chars.len() <= max_chars {
        return text.to_string();
    }
    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }
    let head_len = (max_chars - 3) / 2;
    let tail_len = max_chars - 3 - head_len;
    let head = chars[..head_len].iter().collect::<String>();
    let tail = chars[chars.len() - tail_len..].iter().collect::<String>();
    format!("{head}...{tail}")
}

fn eval_fixture_suite_counts(eval: &EvalResult) -> (Option<u64>, Option<u64>) {
    let fixture_suite = eval.metrics.get("fixture_suite");
    (
        fixture_suite
            .and_then(|suite| suite.get("task_count"))
            .and_then(value_as_u64),
        fixture_suite
            .and_then(|suite| suite.get("command_count"))
            .and_then(value_as_u64),
    )
}

fn eval_has_fixture_suite_metadata(eval: &EvalResult) -> bool {
    let (task_count, command_count) = eval_fixture_suite_counts(eval);
    task_count.is_some()
        || command_count.is_some()
        || !eval_fixture_suite_risk_labels(eval).is_empty()
}

fn eval_fixture_suite_risk_labels(eval: &EvalResult) -> BTreeMap<String, u64> {
    eval.metrics
        .get("fixture_suite")
        .and_then(|suite| suite.get("risk_labels"))
        .and_then(Value::as_object)
        .map(|risks| {
            risks
                .iter()
                .filter_map(|(risk, count)| value_as_u64(count).map(|count| (risk.clone(), count)))
                .collect()
        })
        .unwrap_or_default()
}

fn eval_model_route_tasks(eval: &EvalResult) -> BTreeMap<String, u64> {
    eval.metrics
        .get("state_metrics")
        .and_then(|metrics| metrics.get("model_route_tasks"))
        .and_then(Value::as_object)
        .map(|routes| {
            routes
                .iter()
                .filter_map(|(route, count)| {
                    value_as_u64(count).map(|count| (route.clone(), count))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn eval_fixture_agent_mutation_scope_counts(eval: &EvalResult) -> (Option<u64>, Option<u64>) {
    let metrics = eval.metrics.get("state_metrics");
    (
        metrics
            .and_then(|metrics| metrics.get("fixture_agent_mutation_scope_failures"))
            .and_then(value_as_u64),
        metrics
            .and_then(|metrics| metrics.get("fixture_agent_unexpected_changed_file_count"))
            .and_then(value_as_u64),
    )
}

fn protocol_check_counts_for_eval(eval: &EvalResult) -> Option<ProtocolCheckCounts> {
    let metrics = eval.metrics.get("state_metrics")?;
    let counts = ProtocolCheckCounts {
        total: metrics
            .get("deepseek_protocol_checks")
            .and_then(value_as_u64),
        passes: metrics
            .get("deepseek_protocol_passes")
            .and_then(value_as_u64),
        strict: metrics
            .get("deepseek_strict_tool_call_checks")
            .and_then(value_as_u64),
        thinking: metrics
            .get("deepseek_thinking_protocol_checks")
            .and_then(value_as_u64),
        stream: metrics
            .get("deepseek_streaming_protocol_checks")
            .and_then(value_as_u64),
        json: metrics
            .get("deepseek_json_output_checks")
            .and_then(value_as_u64),
        transport: metrics
            .get("deepseek_transport_policy_checks")
            .and_then(value_as_u64),
    };
    [
        counts.total,
        counts.passes,
        counts.strict,
        counts.thinking,
        counts.stream,
        counts.json,
        counts.transport,
    ]
    .iter()
    .any(Option::is_some)
    .then_some(counts)
}

fn release_fixture_breadth_satisfied(
    task_count: Option<u64>,
    command_count: Option<u64>,
    policy: &ReleaseGatePolicy,
) -> bool {
    let tasks_ok = policy
        .min_fixture_tasks
        .map(|minimum| task_count.map(|count| count >= minimum).unwrap_or(false))
        .unwrap_or(true);
    let commands_ok = policy
        .min_fixture_commands
        .map(|minimum| command_count.map(|count| count >= minimum).unwrap_or(false))
        .unwrap_or(true);
    tasks_ok && commands_ok
}

fn format_release_fixture_breadth_reason(
    task_count: Option<u64>,
    command_count: Option<u64>,
    policy: &ReleaseGatePolicy,
) -> String {
    format!(
        "latest eval fixture suite breadth is below required minimum: tasks {} required {} commands {} required {}",
        format_optional_u64(task_count),
        format_optional_u64(policy.min_fixture_tasks),
        format_optional_u64(command_count),
        format_optional_u64(policy.min_fixture_commands)
    )
}

fn release_fixture_risk_satisfied(
    risk_labels: &BTreeMap<String, u64>,
    policy: &ReleaseGatePolicy,
) -> bool {
    policy
        .min_fixture_risk_labels
        .iter()
        .all(|(label, minimum)| risk_labels.get(label).copied().unwrap_or(0) >= *minimum)
}

fn format_release_fixture_risk_reason(
    risk_labels: &BTreeMap<String, u64>,
    policy: &ReleaseGatePolicy,
) -> String {
    let parts = ["high", "medium", "low"]
        .into_iter()
        .filter_map(|label| {
            policy.min_fixture_risk_labels.get(label).map(|minimum| {
                format!(
                    "{label} {} required {}",
                    risk_labels.get(label).copied().unwrap_or(0),
                    minimum
                )
            })
        })
        .collect::<Vec<_>>();
    format!(
        "latest eval fixture risk coverage is below required minimum: {}",
        parts.join(" ")
    )
}

fn eval_missing_required_release_gates(eval: &EvalResult) -> Vec<String> {
    let commands = eval_command_evidence(eval);
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

fn eval_command_evidence(eval: &EvalResult) -> Vec<String> {
    if let Some(commands) = eval_reproducibility_manifest(eval)
        .and_then(|manifest| manifest.get("commands"))
        .and_then(Value::as_array)
    {
        return commands
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect();
    }
    eval.metrics
        .get("gates")
        .and_then(Value::as_array)
        .map(|gates| {
            gates
                .iter()
                .filter_map(|gate| gate.get("command").and_then(Value::as_str))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn command_satisfies_required_gate(command: &str, required_gate: &str) -> bool {
    let command = command.split_whitespace().collect::<Vec<_>>().join(" ");
    let required_gate = required_gate
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    command == required_gate
}

fn replay_failure_count_after(events: &[Value], after_ms: u128) -> usize {
    events
        .iter()
        .filter(|event| is_state_failure_event(event) && event_timestamp_ms(event) > after_ms)
        .count()
}

fn latest_eval_for_suite(events: &[Value], suite: &str) -> Option<(u128, EvalResult)> {
    events
        .iter()
        .filter_map(|event| {
            let payload = event.get("payload")?;
            let eval = serde_json::from_value::<EvalResult>(payload.clone()).ok()?;
            if eval.suite == suite
                || eval.suite == format!("fixtures:{suite}")
                || eval.suite == format!("fixture-attempts:{suite}")
            {
                let timestamp = eval.created_at_ms.max(
                    event
                        .get("timestamp_ms")
                        .and_then(value_as_u128)
                        .unwrap_or_default(),
                );
                Some((timestamp, eval))
            } else {
                None
            }
        })
        .max_by_key(|(timestamp, _)| *timestamp)
}

fn latest_release_fixture_evidence_eval(
    events: &[Value],
    suite: &str,
    max_age_ms: u128,
    now_ms: u128,
) -> Option<(u128, EvalResult)> {
    events
        .iter()
        .filter_map(|event| {
            let payload = event.get("payload")?;
            let eval = serde_json::from_value::<EvalResult>(payload.clone()).ok()?;
            if !(eval.suite == suite
                || eval.suite == format!("fixtures:{suite}")
                || eval.suite == format!("fixture-attempts:{suite}"))
            {
                return None;
            }
            if !eval_has_fixture_suite_metadata(&eval) {
                return None;
            }
            if eval.status != EvalStatus::Passed || eval.failed != 0 {
                return None;
            }
            if eval_reproducibility_manifest(&eval)
                .and_then(|manifest| manifest.get("git_dirty")?.as_bool())
                == Some(true)
            {
                return None;
            }
            let timestamp = eval.created_at_ms.max(
                event
                    .get("timestamp_ms")
                    .and_then(value_as_u128)
                    .unwrap_or_default(),
            );
            if now_ms.saturating_sub(timestamp) > max_age_ms {
                return None;
            }
            Some((timestamp, eval))
        })
        .max_by_key(|(timestamp, _)| *timestamp)
}

fn latest_eval_by_type(events: &[Value], eval_type: &str) -> Option<(u128, EvalResult)> {
    events
        .iter()
        .filter_map(|event| {
            let payload = event.get("payload")?;
            let eval = serde_json::from_value::<EvalResult>(payload.clone()).ok()?;
            if eval_type_for_eval(&eval) == eval_type {
                let timestamp = eval.created_at_ms.max(
                    event
                        .get("timestamp_ms")
                        .and_then(value_as_u128)
                        .unwrap_or_default(),
                );
                Some((timestamp, eval))
            } else {
                None
            }
        })
        .max_by_key(|(timestamp, _)| *timestamp)
}

fn format_eval_schedule_report(report: &EvalScheduleReport) -> String {
    let mut out = String::new();
    out.push_str("Eval schedule\n");
    out.push_str(&format!("  suite:          {}\n", report.suite));
    out.push_str(&format!("  interval hours: {}\n", report.interval_hours));
    out.push_str(&format!(
        "  due:            {}\n",
        if report.due { "yes" } else { "no" }
    ));
    out.push_str(&format!("  reason:         {}\n", report.reason));
    if let Some(eval_id) = &report.last_eval_id {
        out.push_str(&format!("  last eval:      {eval_id}\n"));
    }
    if let Some(status) = &report.last_eval_status {
        out.push_str(&format!("  last status:    {status}\n"));
    }
    if let Some(last_eval_ms) = report.last_eval_ms {
        out.push_str(&format!("  last eval ms:   {last_eval_ms}\n"));
    }
    out.push_str(&format!("  next due ms:    {}", report.next_due_ms));
    out
}

fn format_release_gate_report(report: &ReleaseGateReport) -> String {
    let mut out = String::new();
    out.push_str("Eval release gate\n");
    out.push_str(&format!("  suite:         {}\n", report.suite));
    out.push_str(&format!("  max age hours: {}\n", report.max_age_hours));
    out.push_str(&format!(
        "  protocol req:  {}\n",
        if report.require_protocol { "yes" } else { "no" }
    ));
    out.push_str(&format!(
        "  ready:         {}\n",
        if report.ready { "yes" } else { "no" }
    ));
    out.push_str(&format!("  reason:        {}\n", report.reason));
    if let Some(eval_id) = &report.last_eval_id {
        out.push_str(&format!("  last eval:     {eval_id}\n"));
    }
    if let Some(status) = &report.last_eval_status {
        out.push_str(&format!("  last status:   {status}\n"));
    }
    if let Some(last_eval_ms) = report.last_eval_ms {
        out.push_str(&format!("  last eval ms:  {last_eval_ms}\n"));
    }
    if let Some(dirty) = report.last_eval_git_dirty {
        out.push_str(&format!(
            "  last eval dirty: {}\n",
            if dirty { "yes" } else { "no" }
        ));
    }
    if report.last_eval_fixture_task_count.is_some()
        || report.last_eval_fixture_command_count.is_some()
        || report.min_fixture_task_count.is_some()
        || report.min_fixture_command_count.is_some()
    {
        out.push_str(&format!(
            "  fixture suite: tasks={} commands={} min_tasks={} min_commands={} breadth_ok={}\n",
            format_optional_u64(report.last_eval_fixture_task_count),
            format_optional_u64(report.last_eval_fixture_command_count),
            format_optional_u64(report.min_fixture_task_count),
            format_optional_u64(report.min_fixture_command_count),
            if report.fixture_breadth_satisfied {
                "yes"
            } else {
                "no"
            }
        ));
    }
    if !report.last_eval_fixture_risk_labels.is_empty() {
        out.push_str(&format!(
            "  fixture risks: {}\n",
            format_count_map(&report.last_eval_fixture_risk_labels)
        ));
    }
    if !report.last_eval_model_route_tasks.is_empty() {
        out.push_str(&format!(
            "  model routes:  {}\n",
            format_count_map(&report.last_eval_model_route_tasks)
        ));
    }
    if !report.min_fixture_risk_labels.is_empty() {
        out.push_str(&format!(
            "  fixture risk gate: min={} satisfied={}\n",
            format_count_map(&report.min_fixture_risk_labels),
            if report.fixture_risk_satisfied {
                "yes"
            } else {
                "no"
            }
        ));
    }
    if report.last_eval_mutation_scope_failures.is_some()
        || report.last_eval_unexpected_changed_files.is_some()
    {
        out.push_str(&format!(
            "  fixture agent: scope_failures={} unexpected_files={}\n",
            format_optional_u64(report.last_eval_mutation_scope_failures),
            format_optional_u64(report.last_eval_unexpected_changed_files)
        ));
    }
    if !report.missing_required_gates.is_empty() {
        out.push_str(&format!(
            "  missing gates: {}\n",
            report.missing_required_gates.join(", ")
        ));
    }
    out.push_str(&format!(
        "  stale:         {}",
        if report.stale { "yes" } else { "no" }
    ));
    out.push_str(&format!(
        "\n  source audit:  {} findings={}",
        if report.source_provenance_passed {
            "passed"
        } else {
            "failed"
        },
        report.source_provenance_findings
    ));
    out.push_str(&format!(
        " source={} scanned={} skipped={}",
        report.source_provenance_scan_source,
        report.source_provenance_scanned_files,
        report.source_provenance_skipped_files
    ));
    for finding in &report.source_provenance_finding_summaries {
        out.push_str(&format!("\n  source finding: {finding}"));
    }
    if report.replay_failures_after_eval > 0 {
        out.push_str(&format!(
            "\n  replay failures after eval: {}",
            report.replay_failures_after_eval
        ));
    }
    if let Some(command) = &report.replay_command {
        out.push_str(&format!("\n  replay command: {command}"));
    }
    if report.require_protocol {
        if let Some(eval_id) = &report.protocol_eval_id {
            out.push_str(&format!("\n  protocol eval: {eval_id}"));
        }
        if let Some(status) = &report.protocol_eval_status {
            out.push_str(&format!("\n  protocol status: {status}"));
        }
        if let Some(timestamp) = report.protocol_eval_ms {
            out.push_str(&format!("\n  protocol eval ms: {timestamp}"));
        }
        if let Some(dirty) = report.protocol_eval_git_dirty {
            out.push_str(&format!(
                "\n  protocol dirty: {}",
                if dirty { "yes" } else { "no" }
            ));
        }
        if let Some(counts) = &report.protocol_check_counts {
            out.push_str(&format!(
                "\n  protocol checks: {}/{} strict={} thinking={} stream={} json={} transport={}",
                format_optional_u64(counts.passes),
                format_optional_u64(counts.total),
                format_optional_u64(counts.strict),
                format_optional_u64(counts.thinking),
                format_optional_u64(counts.stream),
                format_optional_u64(counts.json),
                format_optional_u64(counts.transport)
            ));
        }
        out.push_str(&format!(
            "\n  protocol stale: {}",
            if report.protocol_stale { "yes" } else { "no" }
        ));
        out.push_str(&format!(
            "\n  protocol older than eval: {}",
            if report.protocol_older_than_eval {
                "yes"
            } else {
                "no"
            }
        ));
    }
    out
}

fn record_eval_schedule_decision(report: &EvalScheduleReport) -> Result<String, String> {
    let recorder = default_state_recorder();
    recorder.append(
        EventType::DecisionRecorded,
        Actor::Harness,
        json!({
            "decision_type": "eval_schedule",
            "decision": if report.due { "run_eval" } else { "skip_eval" },
            "suite": report.suite,
            "interval_hours": report.interval_hours,
            "reason": report.reason,
            "last_eval_id": report.last_eval_id,
            "last_eval_status": report.last_eval_status,
            "last_eval_ms": report.last_eval_ms,
            "next_due_ms": report.next_due_ms,
            "now_ms": report.now_ms,
        }),
    )
}

fn record_release_gate_decision(report: &ReleaseGateReport) -> Result<String, String> {
    let recorder = default_state_recorder();
    recorder.append(
        EventType::DecisionRecorded,
        Actor::Harness,
        json!({
            "decision_type": "release_gate",
            "decision": if report.ready { "release_ready" } else { "block_release" },
            "suite": report.suite,
            "max_age_hours": report.max_age_hours,
            "reason": report.reason,
            "last_eval_id": report.last_eval_id,
            "last_eval_status": report.last_eval_status,
            "last_eval_ms": report.last_eval_ms,
            "last_eval_git_dirty": report.last_eval_git_dirty,
            "last_eval_fixture_task_count": report.last_eval_fixture_task_count,
            "last_eval_fixture_command_count": report.last_eval_fixture_command_count,
            "last_eval_fixture_risk_labels": report.last_eval_fixture_risk_labels,
            "last_eval_model_route_tasks": report.last_eval_model_route_tasks,
            "last_eval_mutation_scope_failures": report.last_eval_mutation_scope_failures,
            "last_eval_unexpected_changed_files": report.last_eval_unexpected_changed_files,
            "min_fixture_task_count": report.min_fixture_task_count,
            "min_fixture_command_count": report.min_fixture_command_count,
            "min_fixture_risk_labels": report.min_fixture_risk_labels,
            "fixture_breadth_satisfied": report.fixture_breadth_satisfied,
            "fixture_risk_satisfied": report.fixture_risk_satisfied,
            "missing_required_gates": report.missing_required_gates,
            "stale": report.stale,
            "replay_failures_after_eval": report.replay_failures_after_eval,
            "replay_command": report.replay_command,
            "require_protocol": report.require_protocol,
            "protocol_eval_id": report.protocol_eval_id,
            "protocol_eval_status": report.protocol_eval_status,
            "protocol_eval_ms": report.protocol_eval_ms,
            "protocol_eval_git_dirty": report.protocol_eval_git_dirty,
            "protocol_check_counts": report.protocol_check_counts.as_ref().map(ProtocolCheckCounts::to_json),
            "protocol_stale": report.protocol_stale,
            "protocol_older_than_eval": report.protocol_older_than_eval,
            "source_provenance_passed": report.source_provenance_passed,
            "source_provenance_findings": report.source_provenance_findings,
            "source_provenance_finding_summaries": report.source_provenance_finding_summaries,
            "source_provenance_scan_source": report.source_provenance_scan_source,
            "source_provenance_scanned_files": report.source_provenance_scanned_files,
            "source_provenance_skipped_files": report.source_provenance_skipped_files,
            "now_ms": report.now_ms,
        }),
    )
}

fn gate_to_json(gate: &GateResult) -> Value {
    json!({
        "command": gate.command,
        "passed": gate.passed,
        "status_code": gate.status_code,
        "duration_ms": gate.duration_ms,
        "stdout_preview": gate.stdout_preview,
        "stderr_preview": gate.stderr_preview,
    })
}

fn attach_eval_state_metrics(eval: &mut EvalResult, started_ms: u128, ended_ms: u128) {
    let events_path = default_events_path();
    let mut state_metrics = collect_eval_state_metrics_from(&events_path, started_ms, ended_ms);
    attach_eval_payload_metrics(&mut state_metrics, eval);
    attach_eval_success_metrics(&mut state_metrics, eval.passed);
    if let Some(metrics) = eval.metrics.as_object_mut() {
        metrics.insert("state_metrics".to_string(), state_metrics);
    } else {
        eval.metrics = json!({
            "state_metrics": state_metrics,
            "value": eval.metrics,
        });
    }
}

fn attach_eval_payload_metrics(state_metrics: &mut Value, eval: &EvalResult) {
    let Some(metrics) = state_metrics.as_object_mut() else {
        return;
    };
    let Some(attempts) = eval
        .metrics
        .get("fixture_agent_attempts")
        .and_then(Value::as_array)
    else {
        return;
    };

    let mut mutation_scope_failures = 0u64;
    let mut changed_file_count = 0u64;
    let mut unexpected_changed_file_count = 0u64;
    for attempt in attempts {
        let unexpected_count = attempt
            .get("unexpected_changed_files")
            .and_then(Value::as_array)
            .map(|files| files.len() as u64)
            .unwrap_or_default();
        let mutation_scope_passed = attempt
            .get("mutation_scope_passed")
            .and_then(Value::as_bool)
            .unwrap_or(unexpected_count == 0);
        if !mutation_scope_passed || unexpected_count > 0 {
            mutation_scope_failures += 1;
        }
        changed_file_count += attempt
            .get("changed_files")
            .and_then(Value::as_array)
            .map(|files| files.len() as u64)
            .unwrap_or_default();
        unexpected_changed_file_count += unexpected_count;
    }

    metrics.insert("fixture_agent_attempts".to_string(), json!(attempts.len()));
    metrics.insert(
        "fixture_agent_mutation_scope_failures".to_string(),
        json!(mutation_scope_failures),
    );
    metrics.insert(
        "fixture_agent_mutation_scope_failure_rate".to_string(),
        ratio_json(mutation_scope_failures, attempts.len() as u64),
    );
    metrics.insert(
        "fixture_agent_changed_file_count".to_string(),
        json!(changed_file_count),
    );
    metrics.insert(
        "fixture_agent_unexpected_changed_file_count".to_string(),
        json!(unexpected_changed_file_count),
    );
}

fn attach_eval_success_metrics(state_metrics: &mut Value, passed: u64) {
    let Some(metrics) = state_metrics.as_object_mut() else {
        return;
    };
    let cost = metrics
        .get("cost_usd")
        .and_then(Value::as_f64)
        .unwrap_or_default();
    let latency = metrics
        .get("latency_ms")
        .and_then(value_as_u64)
        .unwrap_or_default();
    if passed == 0 {
        metrics.insert("cost_per_successful_task_usd".to_string(), Value::Null);
        metrics.insert("latency_per_successful_task_ms".to_string(), Value::Null);
    } else {
        metrics.insert(
            "cost_per_successful_task_usd".to_string(),
            json!(cost / passed as f64),
        );
        metrics.insert(
            "latency_per_successful_task_ms".to_string(),
            json!(latency / passed),
        );
    }
}

fn collect_eval_state_metrics_from(events_path: &Path, started_ms: u128, ended_ms: u128) -> Value {
    let events = match read_events(events_path) {
        Ok(events) => events,
        Err(e) => {
            return json!({
                "available": false,
                "source": events_path.display().to_string(),
                "error": preview(&e.to_string(), 400),
                "window_start_ms": started_ms,
                "window_end_ms": ended_ms,
            });
        }
    };

    let mut summary = EvalStateMetrics::default();
    for event in events {
        let timestamp = event
            .get("timestamp_ms")
            .and_then(value_as_u128)
            .unwrap_or_default();
        if timestamp < started_ms || timestamp > ended_ms {
            continue;
        }
        summary.record_event(&event);
    }
    summary.to_json(events_path, started_ms, ended_ms)
}

#[derive(Debug, Default)]
struct EvalStateMetrics {
    events: u64,
    model_calls: u64,
    tool_calls: u64,
    command_runs: u64,
    test_runs: u64,
    file_reads: u64,
    file_edits: u64,
    failures: u64,
    failure_observed: u64,
    thinking_model_calls: u64,
    fim_attempts: u64,
    fim_successes: u64,
    fim_compile_checks: u64,
    fim_compile_successes: u64,
    fim_rollbacks: u64,
    fim_token_savings: u64,
    deepseek_protocol_checks: u64,
    deepseek_protocol_passes: u64,
    deepseek_protocol_failures: u64,
    deepseek_strict_tool_call_checks: u64,
    deepseek_thinking_protocol_checks: u64,
    deepseek_streaming_protocol_checks: u64,
    deepseek_json_output_checks: u64,
    deepseek_transport_policy_checks: u64,
    repair_loop_count: u64,
    json_output_failures: u64,
    tool_schema_failures: u64,
    context_miss_failures: u64,
    rollback_count: u64,
    human_approval_requests: u64,
    human_approval_responses: u64,
    cache_metrics_events: u64,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_write_tokens: u64,
    prompt_cache_hit_tokens: u64,
    prompt_cache_miss_tokens: u64,
    cost_usd: f64,
    latency_ms: u64,
    model_route_tasks: BTreeMap<String, u64>,
}

impl EvalStateMetrics {
    fn record_event(&mut self, event: &Value) {
        self.events += 1;
        let event_type = event
            .get("event_type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let payload = event.get("payload").unwrap_or(&Value::Null);

        match event_type {
            "ModelCallCompleted" => {
                self.model_calls += 1;
                if model_payload_uses_thinking(payload) {
                    self.thinking_model_calls += 1;
                }
                if let Some(route_task) = payload_model_route_task(payload) {
                    *self.model_route_tasks.entry(route_task).or_default() += 1;
                }
                self.input_tokens += payload_u64(payload, "input_tokens").unwrap_or_default();
                self.output_tokens += payload_u64(payload, "output_tokens").unwrap_or_default();
                self.cache_read_tokens +=
                    payload_u64(payload, "cache_read_tokens").unwrap_or_default();
                self.cache_write_tokens +=
                    payload_u64(payload, "cache_write_tokens").unwrap_or_default();
                self.cost_usd += payload_f64(payload, "cost_usd").unwrap_or_default();
                self.latency_ms += payload_u64(payload, "latency_ms").unwrap_or_default();
            }
            "ToolCallCompleted" => {
                self.tool_calls += 1;
                self.latency_ms += payload_u64(payload, "duration_ms").unwrap_or_default();
            }
            "CommandCompleted" => {
                self.command_runs += 1;
                self.latency_ms += payload_u64(payload, "duration_ms").unwrap_or_default();
            }
            "TestCompleted" => {
                self.test_runs += 1;
                if payload_is_deepseek_fim(payload) {
                    self.record_fim_compile_result(payload);
                }
            }
            "FileRead" => self.file_reads += 1,
            "FileEdited" => {
                self.file_edits += 1;
                if payload_is_deepseek_fim(payload) {
                    self.fim_attempts += 1;
                    self.fim_successes += 1;
                    self.record_fim_quality_payload(payload);
                }
            }
            "FailureObserved" => {
                self.failures += 1;
                self.failure_observed += 1;
                self.repair_loop_count += payload_repair_loop_count(payload);
                if payload_is_deepseek_fim(payload) {
                    self.fim_attempts += 1;
                    self.record_fim_quality_payload(payload);
                }
                if failure_payload_mentions_context_miss(payload) {
                    self.context_miss_failures += 1;
                }
            }
            "JsonOutputFailure" => {
                self.failures += 1;
                self.json_output_failures += 1;
                self.repair_loop_count += payload_repair_loop_count(payload);
            }
            "ToolSchemaFailure" => {
                self.failures += 1;
                self.tool_schema_failures += 1;
                self.repair_loop_count += payload_repair_loop_count(payload);
            }
            "DecisionRecorded" => {
                self.repair_loop_count += payload_repair_loop_count(payload);
                self.record_deepseek_protocol_decision(payload);
            }
            "RevertPerformed" => {
                self.rollback_count += 1;
                if payload_is_deepseek_fim(payload) {
                    self.fim_rollbacks += 1;
                }
            }
            "HumanApprovalRequested" => self.human_approval_requests += 1,
            "HumanApprovalReceived" => self.human_approval_responses += 1,
            "CacheMetricsRecorded" => {
                self.cache_metrics_events += 1;
                self.prompt_cache_hit_tokens +=
                    payload_u64(payload, "prompt_cache_hit_tokens").unwrap_or_default();
                self.prompt_cache_miss_tokens +=
                    payload_u64(payload, "prompt_cache_miss_tokens").unwrap_or_default();
            }
            _ => {}
        }
    }

    fn record_deepseek_protocol_decision(&mut self, payload: &Value) {
        let Some(decision_type) = payload.get("decision_type").and_then(Value::as_str) else {
            return;
        };
        match decision_type {
            "deepseek_strict_tool_call_check" => self.deepseek_strict_tool_call_checks += 1,
            "deepseek_thinking_protocol_check" => self.deepseek_thinking_protocol_checks += 1,
            "deepseek_streaming_protocol_check" => self.deepseek_streaming_protocol_checks += 1,
            "deepseek_json_output_check" => self.deepseek_json_output_checks += 1,
            "deepseek_transport_policy_check" => self.deepseek_transport_policy_checks += 1,
            _ => return,
        }
        self.deepseek_protocol_checks += 1;
        if payload.get("decision").and_then(Value::as_str) == Some("passed") {
            self.deepseek_protocol_passes += 1;
        } else {
            self.deepseek_protocol_failures += 1;
        }
    }

    fn record_fim_quality_payload(&mut self, payload: &Value) {
        self.record_fim_compile_result(payload);
        self.fim_token_savings += fim_token_savings(payload);
    }

    fn record_fim_compile_result(&mut self, payload: &Value) {
        if let Some(passed) = payload_compile_passed(payload) {
            self.fim_compile_checks += 1;
            if passed {
                self.fim_compile_successes += 1;
            }
        }
    }

    fn to_json(&self, events_path: &Path, started_ms: u128, ended_ms: u128) -> Value {
        let cache_hit_ratio = if self.prompt_cache_hit_tokens + self.prompt_cache_miss_tokens == 0 {
            Value::Null
        } else {
            json!(
                self.prompt_cache_hit_tokens as f64
                    / (self.prompt_cache_hit_tokens + self.prompt_cache_miss_tokens) as f64
            )
        };
        let malformed_tool_call_rate = ratio_json(self.tool_schema_failures, self.tool_calls);
        let json_parse_failure_rate = ratio_json(self.json_output_failures, self.model_calls);
        let context_miss_rate = ratio_json(self.context_miss_failures, self.failures);
        let permission_prompt_rate = ratio_json(self.human_approval_requests, self.tool_calls);
        let human_intervention_rate = ratio_json(self.human_approval_responses, self.tool_calls);
        let thinking_mode_usage_rate = ratio_json(self.thinking_model_calls, self.model_calls);
        let fim_success_rate = ratio_json(self.fim_successes, self.fim_attempts);
        let fim_compile_rate = ratio_json(self.fim_compile_successes, self.fim_compile_checks);
        let fim_rollback_rate = ratio_json(self.fim_rollbacks, self.fim_attempts);
        let mut metrics = serde_json::Map::new();
        metrics.insert("available".to_string(), json!(true));
        metrics.insert(
            "source".to_string(),
            json!(events_path.display().to_string()),
        );
        metrics.insert("window_start_ms".to_string(), json!(started_ms));
        metrics.insert("window_end_ms".to_string(), json!(ended_ms));
        metrics.insert("events".to_string(), json!(self.events));
        metrics.insert("model_calls".to_string(), json!(self.model_calls));
        metrics.insert(
            "model_route_tasks".to_string(),
            json!(self.model_route_tasks),
        );
        metrics.insert("tool_calls".to_string(), json!(self.tool_calls));
        metrics.insert("command_runs".to_string(), json!(self.command_runs));
        metrics.insert("test_runs".to_string(), json!(self.test_runs));
        metrics.insert("file_reads".to_string(), json!(self.file_reads));
        metrics.insert("file_edits".to_string(), json!(self.file_edits));
        metrics.insert("failures".to_string(), json!(self.failures));
        metrics.insert("failure_observed".to_string(), json!(self.failure_observed));
        metrics.insert(
            "thinking_model_calls".to_string(),
            json!(self.thinking_model_calls),
        );
        metrics.insert(
            "thinking_mode_usage_rate".to_string(),
            thinking_mode_usage_rate,
        );
        metrics.insert("fim_attempts".to_string(), json!(self.fim_attempts));
        metrics.insert("fim_successes".to_string(), json!(self.fim_successes));
        metrics.insert("fim_success_rate".to_string(), fim_success_rate);
        metrics.insert(
            "fim_compile_checks".to_string(),
            json!(self.fim_compile_checks),
        );
        metrics.insert(
            "fim_compile_successes".to_string(),
            json!(self.fim_compile_successes),
        );
        metrics.insert("fim_compile_rate".to_string(), fim_compile_rate);
        metrics.insert("fim_rollbacks".to_string(), json!(self.fim_rollbacks));
        metrics.insert("fim_rollback_rate".to_string(), fim_rollback_rate);
        metrics.insert(
            "fim_token_savings".to_string(),
            json!(self.fim_token_savings),
        );
        metrics.insert(
            "deepseek_protocol_checks".to_string(),
            json!(self.deepseek_protocol_checks),
        );
        metrics.insert(
            "deepseek_protocol_passes".to_string(),
            json!(self.deepseek_protocol_passes),
        );
        metrics.insert(
            "deepseek_protocol_failures".to_string(),
            json!(self.deepseek_protocol_failures),
        );
        metrics.insert(
            "deepseek_strict_tool_call_checks".to_string(),
            json!(self.deepseek_strict_tool_call_checks),
        );
        metrics.insert(
            "deepseek_thinking_protocol_checks".to_string(),
            json!(self.deepseek_thinking_protocol_checks),
        );
        metrics.insert(
            "deepseek_streaming_protocol_checks".to_string(),
            json!(self.deepseek_streaming_protocol_checks),
        );
        metrics.insert(
            "deepseek_json_output_checks".to_string(),
            json!(self.deepseek_json_output_checks),
        );
        metrics.insert(
            "deepseek_transport_policy_checks".to_string(),
            json!(self.deepseek_transport_policy_checks),
        );
        metrics.insert(
            "repair_loop_count".to_string(),
            json!(self.repair_loop_count),
        );
        metrics.insert(
            "json_output_failures".to_string(),
            json!(self.json_output_failures),
        );
        metrics.insert(
            "tool_schema_failures".to_string(),
            json!(self.tool_schema_failures),
        );
        metrics.insert(
            "context_miss_failures".to_string(),
            json!(self.context_miss_failures),
        );
        metrics.insert("rollback_count".to_string(), json!(self.rollback_count));
        metrics.insert(
            "malformed_tool_call_rate".to_string(),
            malformed_tool_call_rate,
        );
        metrics.insert(
            "json_parse_failure_rate".to_string(),
            json_parse_failure_rate,
        );
        metrics.insert("context_miss_rate".to_string(), context_miss_rate);
        metrics.insert("permission_prompt_rate".to_string(), permission_prompt_rate);
        metrics.insert(
            "human_interventions".to_string(),
            json!(self.human_approval_responses),
        );
        metrics.insert(
            "human_intervention_rate".to_string(),
            human_intervention_rate,
        );
        metrics.insert(
            "human_approval_requests".to_string(),
            json!(self.human_approval_requests),
        );
        metrics.insert(
            "human_approval_responses".to_string(),
            json!(self.human_approval_responses),
        );
        metrics.insert(
            "cache_metrics_events".to_string(),
            json!(self.cache_metrics_events),
        );
        metrics.insert("input_tokens".to_string(), json!(self.input_tokens));
        metrics.insert("output_tokens".to_string(), json!(self.output_tokens));
        metrics.insert(
            "cache_read_tokens".to_string(),
            json!(self.cache_read_tokens),
        );
        metrics.insert(
            "cache_write_tokens".to_string(),
            json!(self.cache_write_tokens),
        );
        metrics.insert(
            "prompt_cache_hit_tokens".to_string(),
            json!(self.prompt_cache_hit_tokens),
        );
        metrics.insert(
            "prompt_cache_miss_tokens".to_string(),
            json!(self.prompt_cache_miss_tokens),
        );
        metrics.insert("cache_hit_ratio".to_string(), cache_hit_ratio);
        metrics.insert("cost_usd".to_string(), json!(self.cost_usd));
        metrics.insert("latency_ms".to_string(), json!(self.latency_ms));
        Value::Object(metrics)
    }
}

fn payload_u64(payload: &Value, key: &str) -> Option<u64> {
    payload.get(key).and_then(value_as_u64)
}

fn payload_f64(payload: &Value, key: &str) -> Option<f64> {
    payload.get(key).and_then(Value::as_f64)
}

fn ratio_json(numerator: u64, denominator: u64) -> Value {
    if denominator == 0 {
        Value::Null
    } else {
        json!(numerator as f64 / denominator as f64)
    }
}

fn failure_payload_mentions_context_miss(payload: &Value) -> bool {
    [
        "error_preview",
        "failure_summary",
        "hypothesis",
        "next_repair_step",
    ]
    .iter()
    .filter_map(|key| payload.get(*key).and_then(Value::as_str))
    .any(|text| {
        let lower = text.to_ascii_lowercase();
        lower.contains("context") && (lower.contains("missing") || lower.contains("miss"))
    })
}

fn model_payload_uses_thinking(payload: &Value) -> bool {
    payload_bool(payload, "thinking_observed")
        .or_else(|| payload_bool(payload, "thinking"))
        .unwrap_or(false)
        || payload_thinking_string(payload, "thinking")
        || payload_thinking_string(payload, "thinking_mode")
        || payload_thinking_string(payload, "deepseek_thinking")
        || payload_thinking_string(payload, "reasoning_effort")
        || payload
            .get("thinking")
            .and_then(Value::as_object)
            .map(|object| {
                object
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                    || object
                        .get("mode")
                        .and_then(Value::as_str)
                        .map(thinking_label_enabled)
                        .unwrap_or(false)
            })
            .unwrap_or(false)
}

fn payload_model_route_task(payload: &Value) -> Option<String> {
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

fn payload_bool(payload: &Value, key: &str) -> Option<bool> {
    payload.get(key).and_then(Value::as_bool)
}

fn payload_thinking_string(payload: &Value, key: &str) -> bool {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(thinking_label_enabled)
        .unwrap_or(false)
}

fn thinking_label_enabled(raw: &str) -> bool {
    let normalized = raw.trim().to_ascii_lowercase().replace('-', "_");
    !normalized.is_empty()
        && !matches!(
            normalized.as_str(),
            "off" | "none" | "disabled" | "false" | "no" | "non_thinking" | "no_thinking"
        )
}

fn payload_is_deepseek_fim(payload: &Value) -> bool {
    ["source", "operation", "tool_name", "mode", "task", "scope"]
        .iter()
        .filter_map(|key| payload.get(*key).and_then(Value::as_str))
        .any(|text| {
            let normalized = text.to_ascii_lowercase();
            normalized.contains("fim") || normalized.contains("deepseek_fim")
        })
}

fn payload_compile_passed(payload: &Value) -> Option<bool> {
    [
        "compile_passed",
        "build_passed",
        "test_passed",
        "passed",
        "check_passed",
    ]
    .iter()
    .find_map(|key| payload_bool(payload, key))
}

fn fim_token_savings(payload: &Value) -> u64 {
    for key in ["fim_token_savings", "token_savings", "tokens_saved"] {
        if let Some(value) = payload_u64(payload, key) {
            return value;
        }
    }
    let Some(baseline) = payload_u64(payload, "baseline_input_tokens")
        .or_else(|| payload_u64(payload, "full_context_input_tokens"))
        .or_else(|| payload_u64(payload, "estimated_full_context_tokens"))
    else {
        return 0;
    };
    let actual = payload_u64(payload, "input_tokens")
        .or_else(|| payload_u64(payload, "fim_input_tokens"))
        .unwrap_or_default();
    baseline.saturating_sub(actual)
}

fn payload_repair_loop_count(payload: &Value) -> u64 {
    if let Some(count) = payload_u64(payload, "repair_loop_count") {
        return count;
    }
    if let Some(failed_attempt) = payload_u64(payload, "failed_attempt") {
        return failed_attempt.saturating_sub(1);
    }
    if let Some(attempt) = payload_u64(payload, "attempt") {
        return attempt.saturating_sub(1);
    }
    if let Some(attempts) = payload.get("attempts").and_then(Value::as_array) {
        return attempts.len().saturating_sub(1) as u64;
    }

    let explicit_retry_decision = ["decision_type", "decision", "repair_action", "action"]
        .iter()
        .filter_map(|key| payload.get(*key).and_then(Value::as_str))
        .any(|text| {
            let normalized = text.to_ascii_lowercase();
            normalized.contains("repair") || normalized.contains("retry")
        });
    if explicit_retry_decision {
        1
    } else {
        0
    }
}

fn eval_state_metrics(eval: &EvalResult) -> Option<&Value> {
    let metrics = eval.metrics.get("state_metrics")?;
    if metrics
        .get("available")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        Some(metrics)
    } else {
        None
    }
}

fn metric_u64(metrics: &Value, key: &str) -> u64 {
    metrics.get(key).and_then(value_as_u64).unwrap_or_default()
}

fn metric_u64_opt(metrics: &Value, key: &str) -> Option<u64> {
    metrics.get(key).and_then(value_as_u64)
}

fn metric_f64(metrics: &Value, key: &str) -> Option<f64> {
    metrics.get(key).and_then(Value::as_f64)
}

fn format_u64_delta(baseline: &Value, candidate: &Value, key: &str) -> String {
    let base = metric_u64(baseline, key);
    let current = metric_u64(candidate, key);
    let delta = i128::from(current) - i128::from(base);
    format!("{delta:+} ({base} -> {current})")
}

fn format_percent_delta(baseline: &Value, candidate: &Value, key: &str) -> String {
    let Some(base) = metric_f64(baseline, key) else {
        return "-".to_string();
    };
    let Some(current) = metric_f64(candidate, key) else {
        return "-".to_string();
    };
    let delta = (current - base) * 100.0;
    format!(
        "{delta:+.2}pp ({:.2}% -> {:.2}%)",
        base * 100.0,
        current * 100.0
    )
}

fn format_money_delta(baseline: &Value, candidate: &Value, key: &str) -> String {
    let base = metric_f64(baseline, key).unwrap_or_default();
    let current = metric_f64(candidate, key).unwrap_or_default();
    let delta = current - base;
    format!("{delta:+.6} (${base:.6} -> ${current:.6})")
}

fn format_optional_percent(metrics: &Value, key: &str) -> String {
    metric_f64(metrics, key)
        .map(|value| format!("{:.2}%", value * 100.0))
        .unwrap_or_else(|| "-".to_string())
}

fn value_as_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_i64().and_then(|v| u64::try_from(v).ok()))
}

fn value_as_u128(value: &Value) -> Option<u128> {
    value
        .as_u64()
        .map(|v| v as u128)
        .or_else(|| value.as_i64().and_then(|v| u128::try_from(v).ok()))
}

fn flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a String> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|idx| args.get(idx + 1))
}

fn status_label(status: &EvalStatus) -> &'static str {
    match status {
        EvalStatus::Passed => "passed",
        EvalStatus::Failed => "failed",
        EvalStatus::Error => "error",
        EvalStatus::NoEvidence => "no_evidence",
    }
}

fn preview(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    let mut out = if trimmed.chars().count() <= max_chars {
        trimmed.to_string()
    } else {
        let mut out: String = trimmed.chars().take(max_chars).collect();
        out.push_str("\n...[truncated]");
        out
    };
    if let Value::String(redacted) = crate::state::redact_state_payload(&Value::String(out.clone()))
    {
        out = redacted;
    }
    out
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn print_usage() {
    println!(
        "Usage: yoyo eval <command>\n\n  run [--suite local-smoke] [--patch-id PATCH] [--harness-version VERSION] [--worktree PATH] [--dry-run]\n  schedule [--suite local-smoke] [--interval-hours N] [--record] [--json]\n  release-gate [--suite local-smoke] [--max-age-hours N] [--require-protocol] [--min-fixture-tasks N] [--min-fixture-commands N] [--min-fixture-high-risk N] [--min-fixture-medium-risk N] [--min-fixture-low-risk N] [--record] [--json] [--fail]\n  replay --from-state [--limit N] [--json]\n  fixtures <list|validate|run|attempt> [--suite local-smoke] [--task TASK] [--worktree PATH] [--agent-command CMD|--default-agent] [--dry-run]\n  report <eval-id|run-id|trace-id>\n  compare <baseline-eval|run-id> <candidate-eval|run-id>"
    );
}

const DEFAULT_AGENT_TEMPLATE_HINT: &str = "yyds --yes --no-update-check --prompt {goal}";

#[derive(Debug, Clone, PartialEq, Eq)]
struct FixtureAttemptAgentCommand {
    template: String,
    source: &'static str,
}

fn fixture_attempt_agent_command_spec(
    args: &[String],
    use_default_agent: bool,
) -> Option<FixtureAttemptAgentCommand> {
    if let Some(value) = flag_value(args, "--agent-command") {
        return Some(FixtureAttemptAgentCommand {
            template: value.to_string(),
            source: "explicit",
        });
    }
    if let Ok(value) = std::env::var("YOYO_EVAL_AGENT_COMMAND") {
        return Some(FixtureAttemptAgentCommand {
            template: value,
            source: "env",
        });
    }
    use_default_agent.then(|| FixtureAttemptAgentCommand {
        template: default_fixture_agent_command_template(),
        source: "default-agent",
    })
}

fn default_fixture_agent_command_template() -> String {
    let binary = std::env::current_exe()
        .ok()
        .map(|path| path.display().to_string())
        .filter(|path| !path.trim().is_empty())
        .unwrap_or_else(|| "yyds".to_string());
    format!(
        "{} --yes --no-update-check --prompt {{goal}}",
        shell_quote(&binary)
    )
}

fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .map(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate(command: &str, passed: bool) -> GateResult {
        GateResult {
            command: command.to_string(),
            passed,
            status_code: Some(if passed { 0 } else { 1 }),
            duration_ms: 10,
            stdout_preview: "ok".to_string(),
            stderr_preview: String::new(),
        }
    }

    fn required_gate_results(passed: bool) -> Vec<GateResult> {
        crate::deepseek::DeepSeekHarnessGenome::default()
            .test_policy
            .required_gates
            .iter()
            .map(|gate_command| gate(gate_command, passed))
            .collect()
    }

    fn passing_source_provenance_audit() -> crate::release::SourceProvenanceAudit {
        crate::release::SourceProvenanceAudit {
            policy_version: crate::release::SOURCE_PROVENANCE_POLICY_VERSION,
            allowed_reference_domains: crate::release::allowed_public_reference_domains(),
            forbidden_markers: crate::release::forbidden_source_provenance_markers(),
            scan_source: "git",
            scanned_files: 1,
            skipped_files: 0,
            findings: Vec::new(),
            passed: true,
        }
    }

    #[test]
    fn eval_result_scores_passed_gates() {
        let eval = build_eval_result(
            "local-smoke",
            "genome-v1",
            Some("patch-1".into()),
            &[gate("cargo check", true), gate("cargo test", false)],
            20,
        );
        assert_eq!(eval.status, EvalStatus::Failed);
        assert_eq!(eval.passed, 1);
        assert_eq!(eval.failed, 1);
        assert_eq!(eval.score, Some(0.5));
        assert_eq!(eval.patch_id.as_deref(), Some("patch-1"));
    }

    #[test]
    fn eval_compare_reports_score_delta() {
        let baseline = build_eval_result("local", "a", None, &[gate("a", true)], 10);
        let candidate =
            build_eval_result("local", "b", None, &[gate("a", true), gate("b", false)], 20);
        let report = format_eval_compare(&baseline, &candidate);
        assert!(report.contains("delta:     -0.500"));
    }

    #[test]
    fn eval_compare_reports_fixture_suite_breadth_and_mismatch() {
        let mut baseline = build_eval_result("local-smoke", "a", None, &[gate("a", true)], 10);
        baseline.metrics["fixture_suite"] = json!({
            "task_count": 244,
            "command_count": 488
        });
        let mut candidate = build_eval_result("local-smoke", "b", None, &[gate("a", true)], 10);
        candidate.metrics["fixture_suite"] = json!({
            "task_count": 245,
            "command_count": 490
        });

        let report = format_eval_compare(&baseline, &candidate);

        assert!(report.contains(
            "fixture suite: baseline tasks=244 commands=488 candidate tasks=245 commands=490 breadth=mismatch"
        ));
    }

    #[test]
    fn eval_report_surfaces_state_metrics_and_derived_cost() {
        let mut eval = build_eval_result(
            "local-smoke",
            "genome-v1",
            None,
            &[gate("a", true), gate("b", true)],
            20,
        );
        let mut state_metrics = json!({
            "available": true,
            "model_calls": 2,
            "tool_calls": 3,
            "command_runs": 4,
            "test_runs": 1,
            "failures": 0,
            "json_output_failures": 0,
            "tool_schema_failures": 0,
            "context_miss_failures": 0,
            "rollback_count": 0,
            "malformed_tool_call_rate": 0.0,
            "json_parse_failure_rate": 0.0,
            "context_miss_rate": 0.0,
            "permission_prompt_rate": 0.0,
            "human_intervention_rate": 0.0,
            "thinking_model_calls": 1,
            "thinking_mode_usage_rate": 0.5,
            "fim_attempts": 1,
            "fim_successes": 1,
            "fim_success_rate": 1.0,
            "fim_compile_checks": 1,
            "fim_compile_successes": 1,
            "fim_compile_rate": 1.0,
            "fim_rollbacks": 0,
            "fim_rollback_rate": 0.0,
            "fim_token_savings": 700,
            "deepseek_protocol_checks": 5,
            "deepseek_protocol_passes": 5,
            "deepseek_protocol_failures": 0,
            "deepseek_strict_tool_call_checks": 1,
            "deepseek_thinking_protocol_checks": 1,
            "deepseek_json_output_checks": 1,
            "deepseek_transport_policy_checks": 1,
            "repair_loop_count": 2,
            "file_edits": 2,
            "input_tokens": 100,
            "output_tokens": 50,
            "cache_hit_ratio": 0.75,
            "cost_usd": 0.04,
            "latency_ms": 2000,
            "latency_per_successful_task_ms": 1000
        });
        state_metrics["deepseek_streaming_protocol_checks"] = json!(1);
        attach_eval_success_metrics(&mut state_metrics, eval.passed);
        eval.metrics["state_metrics"] = state_metrics;

        let report = format_eval_report(&eval);

        assert!(report.contains("state metrics:"));
        assert!(report.contains("model/tool calls: 2/3"));
        assert!(report.contains("cache hit ratio:  75.00%"));
        assert!(report.contains("cost per pass:   $0.020000"));
        assert!(report.contains("latency per pass: 1000ms"));
        assert!(report.contains("repair loops:     2"));
        assert!(report.contains("failure classes:  json=0 schema=0 context_miss=0 rollbacks=0"));
        assert!(report.contains(
            "quality rates:    malformed_tool=0.00% json_parse=0.00% context_miss=0.00% permission=0.00% human=0.00%"
        ));
        assert!(report.contains("deepseek modes:   thinking=1/2 (50.00%) fim=1/1 (100.00%)"));
        assert!(report.contains(
            "fim quality:      compile=1/1 (100.00%) rollback=0 (0.00%) token_savings=700"
        ));
        assert!(report.contains(
            "protocol checks: strict=1 thinking=1 stream=1 json=1 transport=1 passes=5/5"
        ));
    }

    #[test]
    fn eval_payload_metrics_surface_fixture_agent_mutation_scope() {
        let result = crate::eval_fixtures::FixtureAgentAttemptResult {
            task_id: "task-1".into(),
            passed: false,
            worktree: "/tmp/worktree".into(),
            agent_command: "yyds --prompt goal".into(),
            agent_result: crate::eval_fixtures::FixtureCommandResult {
                command: "yyds --prompt goal".into(),
                passed: true,
                status_code: Some(0),
                duration_ms: 10,
                stdout_preview: String::new(),
                stderr_preview: String::new(),
            },
            mutation_scope_passed: false,
            changed_files: vec!["src/context.rs".into(), "src/lib.rs".into()],
            unexpected_changed_files: vec!["src/lib.rs".into()],
            command_results: Vec::new(),
        };
        let mut eval =
            build_fixture_agent_attempt_eval_result("local-smoke", "genome-v1", &[result], 10);
        let mut state_metrics = json!({"available": true});
        attach_eval_payload_metrics(&mut state_metrics, &eval);
        attach_eval_success_metrics(&mut state_metrics, eval.passed);
        eval.metrics["state_metrics"] = state_metrics;

        assert_eq!(eval.metrics["state_metrics"]["fixture_agent_attempts"], 1);
        assert_eq!(
            eval.metrics["state_metrics"]["fixture_agent_mutation_scope_failures"],
            1
        );
        assert_eq!(
            eval.metrics["state_metrics"]["fixture_agent_changed_file_count"],
            2
        );
        assert_eq!(
            eval.metrics["state_metrics"]["fixture_agent_unexpected_changed_file_count"],
            1
        );
        assert_eq!(
            eval.metrics["state_metrics"]["fixture_agent_mutation_scope_failure_rate"],
            1.0
        );

        let report = format_eval_report(&eval);

        assert!(report.contains(
            "fixture agent:   attempts=1 scope_failures=1 unexpected_files=1 scope_rate=100.00%"
        ));
    }

    #[test]
    fn eval_state_metrics_classifies_harness_quality_failures() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-model".into(),
                event_type: EventType::ModelCallCompleted,
                schema_version: 1,
                timestamp_ms: 10,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({"input_tokens": 10, "output_tokens": 5}),
            },
            crate::state::StateEvent {
                event_id: "evt-tool".into(),
                event_type: EventType::ToolCallCompleted,
                schema_version: 1,
                timestamp_ms: 11,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({"duration_ms": 4}),
            },
            crate::state::StateEvent {
                event_id: "evt-schema".into(),
                event_type: EventType::ToolSchemaFailure,
                schema_version: 1,
                timestamp_ms: 12,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({"source": "strict_tool_schema"}),
            },
            crate::state::StateEvent {
                event_id: "evt-json".into(),
                event_type: EventType::JsonOutputFailure,
                schema_version: 1,
                timestamp_ms: 13,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({"source": "json_output"}),
            },
            crate::state::StateEvent {
                event_id: "evt-context".into(),
                event_type: EventType::FailureObserved,
                schema_version: 1,
                timestamp_ms: 14,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "repair",
                    "error_preview": "retry_state.rs missing from context during repair"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-approval".into(),
                event_type: EventType::HumanApprovalRequested,
                schema_version: 1,
                timestamp_ms: 15,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({}),
            },
            crate::state::StateEvent {
                event_id: "evt-approval-response".into(),
                event_type: EventType::HumanApprovalReceived,
                schema_version: 1,
                timestamp_ms: 16,
                actor: Actor::User,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({}),
            },
            crate::state::StateEvent {
                event_id: "evt-revert".into(),
                event_type: EventType::RevertPerformed,
                schema_version: 1,
                timestamp_ms: 17,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({}),
            },
        ];
        for event in events {
            crate::state::append_event(&events_path, &event).unwrap();
        }

        let metrics = collect_eval_state_metrics_from(&events_path, 10, 17);

        assert_eq!(metrics["failures"], 3);
        assert_eq!(metrics["tool_schema_failures"], 1);
        assert_eq!(metrics["json_output_failures"], 1);
        assert_eq!(metrics["context_miss_failures"], 1);
        assert_eq!(metrics["rollback_count"], 1);
        assert_eq!(metrics["malformed_tool_call_rate"], 1.0);
        assert_eq!(metrics["json_parse_failure_rate"], 1.0);
        assert_eq!(metrics["context_miss_rate"], 1.0 / 3.0);
        assert_eq!(metrics["permission_prompt_rate"], 1.0);
        assert_eq!(metrics["human_interventions"], 1);
        assert_eq!(metrics["human_intervention_rate"], 1.0);
    }

    #[test]
    fn eval_state_metrics_counts_explicit_repair_loops() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-json".into(),
                event_type: EventType::JsonOutputFailure,
                schema_version: 1,
                timestamp_ms: 10,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "json_output",
                    "attempts": [
                        {"status": "invalid"},
                        {"status": "invalid"}
                    ]
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-schema".into(),
                event_type: EventType::ToolSchemaFailure,
                schema_version: 1,
                timestamp_ms: 11,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "strict_tool_schema",
                    "failed_attempt": 3,
                    "max_repair_turns": 2,
                    "repair_action": "abort"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-decision".into(),
                event_type: EventType::DecisionRecorded,
                schema_version: 1,
                timestamp_ms: 12,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "decision_type": "tool_schema_repair",
                    "repair_action": "retry"
                }),
            },
        ];
        for event in events {
            crate::state::append_event(&events_path, &event).unwrap();
        }

        let metrics = collect_eval_state_metrics_from(&events_path, 10, 12);

        assert_eq!(metrics["repair_loop_count"], 4);
        assert_eq!(metrics["json_output_failures"], 1);
        assert_eq!(metrics["tool_schema_failures"], 1);
    }

    #[test]
    fn eval_state_metrics_tracks_deepseek_mode_usage() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-thinking".into(),
                event_type: EventType::ModelCallCompleted,
                schema_version: 1,
                timestamp_ms: 10,
                actor: Actor::Yoyo,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "model": "deepseek-chat",
                    "thinking_observed": true,
                    "route_task": "root-cause"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-nonthinking".into(),
                event_type: EventType::ModelCallCompleted,
                schema_version: 1,
                timestamp_ms: 11,
                actor: Actor::Yoyo,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "model": "deepseek-chat",
                    "thinking_mode": "disabled",
                    "route_task": "memory compression"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-fim".into(),
                event_type: EventType::FileEdited,
                schema_version: 1,
                timestamp_ms: 12,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "deepseek_fim_apply",
                    "file_path": "src/lib.rs",
                    "compile_passed": true,
                    "baseline_input_tokens": 1000,
                    "input_tokens": 300
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-fim-failure".into(),
                event_type: EventType::FailureObserved,
                schema_version: 1,
                timestamp_ms: 13,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "deepseek_fim_apply",
                    "error_preview": "patch rejected",
                    "compile_passed": false
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-fim-revert".into(),
                event_type: EventType::RevertPerformed,
                schema_version: 1,
                timestamp_ms: 14,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({"source": "deepseek_fim_apply", "reverted_file": "src/lib.rs"}),
            },
        ];
        for event in events {
            crate::state::append_event(&events_path, &event).unwrap();
        }

        let metrics = collect_eval_state_metrics_from(&events_path, 10, 14);

        assert_eq!(metrics["model_calls"], 2);
        assert_eq!(metrics["model_route_tasks"]["root_cause"], 1);
        assert_eq!(metrics["model_route_tasks"]["memory_compression"], 1);
        assert_eq!(metrics["thinking_model_calls"], 1);
        assert_eq!(metrics["thinking_mode_usage_rate"], 0.5);
        assert_eq!(metrics["fim_attempts"], 2);
        assert_eq!(metrics["fim_successes"], 1);
        assert_eq!(metrics["fim_success_rate"], 0.5);
        assert_eq!(metrics["fim_compile_checks"], 2);
        assert_eq!(metrics["fim_compile_successes"], 1);
        assert_eq!(metrics["fim_compile_rate"], 0.5);
        assert_eq!(metrics["fim_rollbacks"], 1);
        assert_eq!(metrics["fim_rollback_rate"], 0.5);
        assert_eq!(metrics["fim_token_savings"], 700);
    }

    #[test]
    fn eval_state_metrics_tracks_deepseek_protocol_pass_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let events = [
            crate::state::StateEvent {
                event_id: "evt-strict-pass".into(),
                event_type: EventType::DecisionRecorded,
                schema_version: 1,
                timestamp_ms: 10,
                actor: Actor::Harness,
                run_id: Some("run-protocol".into()),
                session_id: None,
                trace_id: "trace-protocol".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "deepseek_protocol_check",
                    "decision_type": "deepseek_strict_tool_call_check",
                    "check": "test-tool-call",
                    "decision": "passed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-thinking-pass".into(),
                event_type: EventType::DecisionRecorded,
                schema_version: 1,
                timestamp_ms: 11,
                actor: Actor::Harness,
                run_id: Some("run-protocol".into()),
                session_id: None,
                trace_id: "trace-protocol".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "deepseek_protocol_check",
                    "decision_type": "deepseek_thinking_protocol_check",
                    "check": "test-thinking",
                    "decision": "passed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-json-pass".into(),
                event_type: EventType::DecisionRecorded,
                schema_version: 1,
                timestamp_ms: 12,
                actor: Actor::Harness,
                run_id: Some("run-protocol".into()),
                session_id: None,
                trace_id: "trace-protocol".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "json_output",
                    "decision_type": "deepseek_json_output_check",
                    "check": "json-check",
                    "decision": "passed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-stream-pass".into(),
                event_type: EventType::DecisionRecorded,
                schema_version: 1,
                timestamp_ms: 13,
                actor: Actor::Harness,
                run_id: Some("run-protocol".into()),
                session_id: None,
                trace_id: "trace-protocol".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "deepseek_protocol_check",
                    "decision_type": "deepseek_streaming_protocol_check",
                    "check": "stream-check",
                    "decision": "passed"
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-transport-pass".into(),
                event_type: EventType::DecisionRecorded,
                schema_version: 1,
                timestamp_ms: 14,
                actor: Actor::Harness,
                run_id: Some("run-protocol".into()),
                session_id: None,
                trace_id: "trace-protocol".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "deepseek_protocol_check",
                    "decision_type": "deepseek_transport_policy_check",
                    "check": "transport-check",
                    "decision": "passed"
                }),
            },
        ];
        for event in events {
            crate::state::append_event(&events_path, &event).unwrap();
        }

        let metrics = collect_eval_state_metrics_from(&events_path, 10, 14);

        assert_eq!(metrics["deepseek_protocol_checks"], 5);
        assert_eq!(metrics["deepseek_protocol_passes"], 5);
        assert_eq!(metrics["deepseek_protocol_failures"], 0);
        assert_eq!(metrics["deepseek_strict_tool_call_checks"], 1);
        assert_eq!(metrics["deepseek_thinking_protocol_checks"], 1);
        assert_eq!(metrics["deepseek_streaming_protocol_checks"], 1);
        assert_eq!(metrics["deepseek_json_output_checks"], 1);
        assert_eq!(metrics["deepseek_transport_policy_checks"], 1);
    }

    #[test]
    fn eval_compare_reports_state_metric_deltas() {
        let mut baseline = build_eval_result("local", "a", None, &[gate("a", true)], 10);
        baseline.metrics["state_metrics"] = json!({
            "available": true,
            "model_calls": 1,
            "tool_calls": 2,
            "failures": 1,
            "tool_schema_failures": 1,
            "json_output_failures": 1,
            "context_miss_failures": 1,
            "repair_loop_count": 3,
            "malformed_tool_call_rate": 0.5,
            "json_parse_failure_rate": 1.0,
            "permission_prompt_rate": 0.25,
            "human_intervention_rate": 0.5,
            "thinking_mode_usage_rate": 0.0,
            "model_route_tasks": {
                "root_cause": 1
            },
            "fim_success_rate": 0.5,
            "fim_compile_rate": 0.5,
            "fim_rollback_rate": 0.5,
            "fim_token_savings": 100,
            "fixture_agent_mutation_scope_failures": 2,
            "fixture_agent_unexpected_changed_file_count": 3,
            "fixture_agent_mutation_scope_failure_rate": 0.5,
            "file_edits": 1,
            "input_tokens": 100,
            "output_tokens": 20,
            "cache_hit_ratio": 0.5,
            "cost_usd": 0.05,
            "cost_per_successful_task_usd": 0.05,
            "latency_ms": 1000,
            "latency_per_successful_task_ms": 1000
        });
        let mut candidate = build_eval_result("local", "b", None, &[gate("a", true)], 10);
        candidate.metrics["state_metrics"] = json!({
            "available": true,
            "model_calls": 2,
            "tool_calls": 1,
            "failures": 0,
            "tool_schema_failures": 0,
            "json_output_failures": 0,
            "context_miss_failures": 0,
            "repair_loop_count": 1,
            "malformed_tool_call_rate": 0.0,
            "json_parse_failure_rate": 0.0,
            "permission_prompt_rate": 0.75,
            "human_intervention_rate": 0.0,
            "thinking_mode_usage_rate": 0.5,
            "model_route_tasks": {
                "root_cause": 1,
                "memory_compression": 1
            },
            "fim_success_rate": 1.0,
            "fim_compile_rate": 1.0,
            "fim_rollback_rate": 0.0,
            "fim_token_savings": 250,
            "fixture_agent_mutation_scope_failures": 0,
            "fixture_agent_unexpected_changed_file_count": 0,
            "fixture_agent_mutation_scope_failure_rate": 0.0,
            "file_edits": 1,
            "input_tokens": 90,
            "output_tokens": 25,
            "cache_hit_ratio": 0.75,
            "cost_usd": 0.04,
            "cost_per_successful_task_usd": 0.04,
            "latency_ms": 800,
            "latency_per_successful_task_ms": 800
        });

        let report = format_eval_compare(&baseline, &candidate);

        assert!(report.contains("metric deltas:"));
        assert!(report.contains("model calls:       +1 (1 -> 2)"));
        assert!(report.contains("failures:          -1 (1 -> 0)"));
        assert!(report.contains("schema failures:   -1 (1 -> 0)"));
        assert!(report.contains("json failures:     -1 (1 -> 0)"));
        assert!(report.contains("context misses:    -1 (1 -> 0)"));
        assert!(report.contains("repair loops:      -2 (3 -> 1)"));
        assert!(report.contains("malformed tools:   -50.00pp (50.00% -> 0.00%)"));
        assert!(report.contains("json parse rate:   -100.00pp (100.00% -> 0.00%)"));
        assert!(report.contains("thinking rate:    +50.00pp (0.00% -> 50.00%)"));
        assert!(report.contains(
            "model routes:     baseline [root_cause=1] candidate [memory_compression=1, root_cause=1]"
        ));
        assert!(report.contains("fim success rate: +50.00pp (50.00% -> 100.00%)"));
        assert!(report.contains("fim compile rate: +50.00pp (50.00% -> 100.00%)"));
        assert!(report.contains("fim rollback rate: -50.00pp (50.00% -> 0.00%)"));
        assert!(report.contains("fim token savings: +150 (100 -> 250)"));
        assert!(report.contains("fixture agent scope failures: -2 (2 -> 0)"));
        assert!(report.contains("fixture agent unexpected files: -3 (3 -> 0)"));
        assert!(report.contains("fixture agent scope rate: -50.00pp (50.00% -> 0.00%)"));
        assert!(report.contains("permission rate: +50.00pp (25.00% -> 75.00%)"));
        assert!(report.contains("human rate:      -50.00pp (50.00% -> 0.00%)"));
        assert!(report.contains("cache hit ratio:   +25.00pp (50.00% -> 75.00%)"));
        assert!(report.contains("cost:              -0.010000 ($0.050000 -> $0.040000)"));
        assert!(report.contains("latency per pass:  -200 (1000 -> 800)"));
    }

    #[test]
    fn failure_replay_report_builds_candidates_from_state_failures() {
        let events = vec![
            json!({
                "event_id": "evt-old",
                "event_type": "FailureObserved",
                "timestamp_ms": 10,
                "run_id": "run-old",
                "payload": {
                    "source": "fixture",
                    "error_preview": "retry_state.rs missing from context"
                }
            }),
            json!({
                "event_id": "evt-schema",
                "event_type": "ToolSchemaFailure",
                "timestamp_ms": 20,
                "run_id": "run-new",
                "payload": {
                    "source": "strict_tool_schema",
                    "tool_name": "propose_edit",
                    "repair_instruction": "Retry propose_edit with schema_version"
                }
            }),
            json!({
                "event_id": "evt-cache",
                "event_type": "CacheMetricsRecorded",
                "timestamp_ms": 30,
                "payload": {}
            }),
        ];

        let replay = build_failure_replay_report(&events, "local-smoke", 5);
        let report = format_failure_replay_report(&replay);

        assert_eq!(replay.total_failures, 2);
        assert_eq!(replay.candidates.len(), 2);
        assert_eq!(replay.candidates[0].event_id, "evt-schema");
        assert!(report.contains("Historical failure replay"));
        assert!(report.contains("ToolSchemaFailure evt-schema"));
        assert!(report.contains("class=tool_schema priority="));
        assert!(report.contains("retryable=yes"));
        assert!(report
            .contains("strict tool schema failures usually need immediate repair-loop replay"));
        assert!(report.contains("yoyo state why evt-schema"));
        assert!(report.contains("yoyo eval fixtures run --suite local-smoke"));
    }

    #[test]
    fn failure_replay_report_honors_limit_and_json_shape() {
        let events = vec![
            json!({
                "event_id": "evt-1",
                "event_type": "FailureObserved",
                "timestamp_ms": 1,
                "payload": {"source": "test", "error_preview": "one"}
            }),
            json!({
                "event_id": "evt-2",
                "event_type": "JsonOutputFailure",
                "timestamp_ms": 2,
                "payload": {"operation": "json", "error_preview": "two"}
            }),
        ];

        let replay = build_failure_replay_report(&events, "nightly", 1);
        let value = replay.to_json();

        assert_eq!(value["total_failures"], 2);
        assert_eq!(value["candidates"].as_array().unwrap().len(), 1);
        assert_eq!(value["candidates"][0]["event_id"], "evt-2");
        assert_eq!(value["candidates"][0]["failure_class"], "json_output");
        assert_eq!(value["candidates"][0]["retryable"], true);
        assert!(value["candidates"][0]["priority_score"].as_u64().unwrap() >= 90);
        assert!(!value["candidates"][0]["priority_reasons"]
            .as_array()
            .unwrap()
            .is_empty());
        assert!(value["candidates"][0]["suggested_command"]
            .as_str()
            .unwrap()
            .contains("--suite nightly"));
    }

    #[test]
    fn failure_replay_report_prioritizes_replayable_failure_classes() {
        let events = vec![
            json!({
                "event_id": "evt-new-transport",
                "event_type": "FailureObserved",
                "timestamp_ms": 30,
                "payload": {
                    "source": "deepseek_provider",
                    "error_preview": "transport timeout after retry budget"
                }
            }),
            json!({
                "event_id": "evt-context",
                "event_type": "FailureObserved",
                "timestamp_ms": 20,
                "payload": {
                    "source": "context_selector",
                    "error_preview": "retry_state.rs missing from context",
                    "repair_instruction": "Replay with expanded context evidence"
                }
            }),
            json!({
                "event_id": "evt-schema",
                "event_type": "ToolSchemaFailure",
                "timestamp_ms": 10,
                "payload": {
                    "source": "strict_tool_schema",
                    "tool_name": "propose_edit",
                    "repair_loop_count": 2
                }
            }),
        ];

        let replay = build_failure_replay_report(&events, "local-smoke", 2);

        assert_eq!(replay.total_failures, 3);
        assert_eq!(replay.candidates.len(), 2);
        assert_eq!(replay.candidates[0].event_id, "evt-schema");
        assert_eq!(replay.candidates[0].failure_class, "tool_schema");
        assert!(replay.candidates[0].priority_score > replay.candidates[1].priority_score);
        assert_eq!(replay.candidates[1].event_id, "evt-context");
        assert_eq!(replay.candidates[1].failure_class, "context_miss");
        assert!(replay
            .candidates
            .iter()
            .all(|candidate| candidate.retryable));
        let report = format_failure_replay_report(&replay);
        assert!(report.contains("priority:"));
        assert!(report.contains("repair_loop_count=2"));
        assert!(!report.contains("evt-new-transport"));
    }

    #[test]
    fn eval_schedule_is_due_when_suite_has_no_prior_eval() {
        let report = build_eval_schedule_report(&[], "local-smoke", 24, 1_000);

        assert!(report.due);
        assert_eq!(report.reason, "no prior eval found for suite");
        assert_eq!(report.next_due_ms, 1_000);
        assert!(format_eval_schedule_report(&report).contains("due:            yes"));
    }

    #[test]
    fn eval_schedule_uses_last_matching_eval_interval() {
        let mut old_eval =
            build_eval_result("local-smoke", "genome-v1", None, &[gate("old", true)], 10);
        old_eval.eval_id = "eval-old".into();
        old_eval.created_at_ms = 1_000;
        let mut latest_eval = build_eval_result(
            "fixtures:local-smoke",
            "genome-v1",
            None,
            &[gate("new", true)],
            10,
        );
        latest_eval.eval_id = "eval-new".into();
        latest_eval.created_at_ms = 10_000;
        let events = vec![
            json!({
                "event_id": "evt-old",
                "event_type": "PatchEvaluated",
                "timestamp_ms": 1_000,
                "payload": old_eval,
            }),
            json!({
                "event_id": "evt-new",
                "event_type": "PatchEvaluated",
                "timestamp_ms": 10_000,
                "payload": latest_eval,
            }),
        ];

        let report = build_eval_schedule_report(&events, "local-smoke", 1, 10_000 + 3_599_999);
        assert!(!report.due);
        assert_eq!(report.last_eval_id.as_deref(), Some("eval-new"));
        assert_eq!(report.next_due_ms, 10_000 + 3_600_000);

        let due = build_eval_schedule_report(&events, "local-smoke", 1, 10_000 + 3_600_000);
        assert!(due.due);
        assert_eq!(due.reason, "interval elapsed since last eval");
    }

    #[test]
    fn eval_schedule_matches_fixture_agent_attempt_runs() {
        let mut eval = build_fixture_agent_attempt_eval_result("local-smoke", "genome-v1", &[], 10);
        eval.eval_id = "eval-attempt".into();
        eval.created_at_ms = 10_000;
        let events = vec![json!({
            "event_id": "evt-attempt",
            "event_type": "PatchEvaluated",
            "timestamp_ms": 10_000,
            "payload": eval,
        })];

        let report = build_eval_schedule_report(&events, "local-smoke", 1, 10_000 + 3_599_999);

        assert!(!report.due);
        assert_eq!(report.last_eval_id.as_deref(), Some("eval-attempt"));
    }

    #[test]
    fn eval_schedule_json_exposes_recordable_decision_fields() {
        let report = build_eval_schedule_report(&[], "local-smoke", 12, 42);
        let value = report.to_json();

        assert_eq!(value["suite"], "local-smoke");
        assert_eq!(value["interval_hours"], 12);
        assert_eq!(value["due"], true);
        assert_eq!(value["reason"], "no prior eval found for suite");
        assert!(value["last_eval_id"].is_null());
    }

    #[test]
    fn release_gate_blocks_when_no_eval_exists() {
        let report = build_release_gate_report(&[], "local-smoke", 24, 100);

        assert!(!report.ready);
        assert!(report.stale);
        assert_eq!(report.reason, "no prior eval found for suite");
        assert!(format_release_gate_report(&report).contains("ready:         no"));
    }

    #[test]
    fn release_gate_blocks_failed_or_stale_eval() {
        let mut failed_eval = build_eval_result(
            "fixtures:local-smoke",
            "genome-v1",
            None,
            &[gate("test", false)],
            10,
        );
        failed_eval.eval_id = "eval-failed".into();
        failed_eval.created_at_ms = 10_000;
        let events = vec![json!({
            "event_id": "evt-failed",
            "event_type": "PatchEvaluated",
            "timestamp_ms": 10_000,
            "payload": failed_eval,
        })];

        let failed = build_release_gate_report(&events, "local-smoke", 24, 20_000);
        assert!(!failed.ready);
        assert_eq!(failed.reason, "latest eval did not pass");

        let mut passed_eval = build_eval_result(
            "local-smoke",
            "genome-v1",
            None,
            &required_gate_results(true),
            10,
        );
        passed_eval.eval_id = "eval-old-pass".into();
        passed_eval.created_at_ms = 10_000;
        let events = vec![json!({
            "event_id": "evt-old-pass",
            "event_type": "PatchEvaluated",
            "timestamp_ms": 10_000,
            "payload": passed_eval,
        })];
        let stale = build_release_gate_report(&events, "local-smoke", 1, 10_000 + 3_600_001);
        assert!(!stale.ready);
        assert!(stale.stale);
        assert_eq!(stale.reason, "latest eval is older than max age");
    }

    #[test]
    fn release_gate_allows_fresh_passing_eval_and_serializes_decision() {
        let mut eval = build_eval_result(
            "local-smoke",
            "genome-v1",
            None,
            &required_gate_results(true),
            10,
        );
        eval.eval_id = "eval-pass".into();
        eval.created_at_ms = 10_000;
        eval.metrics["fixture_suite"] = json!({
            "task_count": 240,
            "command_count": 480,
            "risk_labels": {
                "high": 4,
                "low": 120,
                "medium": 116
            }
        });
        eval.metrics["state_metrics"] = json!({
            "model_route_tasks": {
                "memory_compression": 1,
                "root_cause": 3
            }
        });
        let events = vec![json!({
            "event_id": "evt-pass",
            "event_type": "PatchEvaluated",
            "timestamp_ms": 10_000,
            "payload": eval,
        })];

        let report = build_release_gate_report_with_policy_and_source_audit(
            &events,
            "local-smoke",
            1,
            10_000 + 3_599_999,
            ReleaseGatePolicy {
                require_protocol: false,
                min_fixture_tasks: None,
                min_fixture_commands: None,
                min_fixture_risk_labels: BTreeMap::new(),
            },
            passing_source_provenance_audit(),
        );
        assert!(report.ready);
        assert!(!report.stale);
        assert_eq!(report.reason, "latest eval passed and is fresh");
        assert_eq!(report.last_eval_id.as_deref(), Some("eval-pass"));
        let value = report.to_json();
        assert_eq!(value["ready"], true);
        assert_eq!(value["last_eval_status"], "passed");
        assert_eq!(value["last_eval_fixture_task_count"], 240);
        assert_eq!(value["last_eval_fixture_command_count"], 480);
        assert_eq!(value["last_eval_fixture_risk_labels"]["high"], 4);
        assert_eq!(value["last_eval_fixture_risk_labels"]["medium"], 116);
        assert_eq!(value["last_eval_model_route_tasks"]["root_cause"], 3);
        assert_eq!(
            value["last_eval_model_route_tasks"]["memory_compression"],
            1
        );
        assert_eq!(value["replay_failures_after_eval"], 0);
        assert!(value["replay_command"].is_null());
        assert_eq!(value["source_provenance_passed"], true);
        assert_eq!(value["source_provenance_findings"], 0);
        let scan_source = value["source_provenance_scan_source"].as_str().unwrap();
        assert!(matches!(scan_source, "git" | "filesystem"));
        assert!(format_release_gate_report(&report).contains("source audit:  passed findings=0"));
        assert!(
            format_release_gate_report(&report).contains("fixture suite: tasks=240 commands=480")
        );
        assert!(format_release_gate_report(&report)
            .contains("fixture risks: high=4, low=120, medium=116"));
        assert!(format_release_gate_report(&report)
            .contains("model routes:  memory_compression=1, root_cause=3"));
        assert!(format_release_gate_report(&report).contains("source="));
    }

    #[test]
    fn release_gate_combines_required_gate_eval_with_fixture_evidence_eval() {
        let mut fixture_eval = build_eval_result(
            "fixtures:local-smoke",
            "genome-v1",
            None,
            &[gate("fixture", true)],
            10,
        );
        fixture_eval.eval_id = "eval-fixtures".into();
        fixture_eval.created_at_ms = 10_000;
        fixture_eval.metrics["fixture_suite"] = json!({
            "task_count": 240,
            "command_count": 480,
            "risk_labels": {
                "high": 4,
                "low": 120,
                "medium": 116
            }
        });
        fixture_eval.metrics["reproducibility"] = json!({
            "git_dirty": false,
            "commands": ["cargo test fixture"]
        });
        let mut gate_eval = build_eval_result(
            "local-smoke",
            "genome-v1",
            None,
            &required_gate_results(true),
            10,
        );
        gate_eval.eval_id = "eval-gates".into();
        gate_eval.created_at_ms = 20_000;
        let events = vec![
            json!({
                "event_id": "evt-fixtures",
                "event_type": "PatchEvaluated",
                "timestamp_ms": 10_000,
                "payload": fixture_eval,
            }),
            json!({
                "event_id": "evt-gates",
                "event_type": "PatchEvaluated",
                "timestamp_ms": 20_000,
                "payload": gate_eval,
            }),
        ];

        let report = build_release_gate_report_with_policy_and_source_audit(
            &events,
            "local-smoke",
            1,
            20_000 + 3_000_000,
            ReleaseGatePolicy {
                require_protocol: false,
                min_fixture_tasks: Some(200),
                min_fixture_commands: Some(300),
                min_fixture_risk_labels: BTreeMap::from([
                    ("high".to_string(), 1),
                    ("medium".to_string(), 1),
                    ("low".to_string(), 1),
                ]),
            },
            passing_source_provenance_audit(),
        );

        assert!(report.ready);
        assert_eq!(report.last_eval_id.as_deref(), Some("eval-gates"));
        assert_eq!(report.last_eval_fixture_task_count, Some(240));
        assert_eq!(report.last_eval_fixture_command_count, Some(480));
        assert_eq!(report.last_eval_fixture_risk_labels["high"], 4);
        assert!(report.fixture_breadth_satisfied);
        assert!(report.fixture_risk_satisfied);
    }

    #[test]
    fn release_gate_blocks_when_fixture_suite_breadth_below_minimum() {
        let mut eval = build_eval_result(
            "local-smoke",
            "genome-v1",
            None,
            &required_gate_results(true),
            10,
        );
        eval.eval_id = "eval-pass".into();
        eval.created_at_ms = 10_000;
        eval.metrics["fixture_suite"] = json!({
            "task_count": 244,
            "command_count": 488
        });
        let events = vec![json!({
            "event_id": "evt-pass",
            "event_type": "PatchEvaluated",
            "timestamp_ms": 10_000,
            "payload": eval,
        })];

        let report = build_release_gate_report_with_policy(
            &events,
            "local-smoke",
            1,
            10_000 + 3_599_999,
            ReleaseGatePolicy {
                require_protocol: false,
                min_fixture_tasks: Some(245),
                min_fixture_commands: Some(490),
                min_fixture_risk_labels: BTreeMap::new(),
            },
        );

        assert!(!report.ready);
        assert!(!report.fixture_breadth_satisfied);
        assert_eq!(report.last_eval_fixture_task_count, Some(244));
        assert_eq!(report.min_fixture_task_count, Some(245));
        assert!(report
            .reason
            .contains("latest eval fixture suite breadth is below required minimum"));
        assert_eq!(report.to_json()["fixture_breadth_satisfied"], false);
        assert_eq!(report.to_json()["min_fixture_task_count"], 245);
        assert!(format_release_gate_report(&report).contains(
            "fixture suite: tasks=244 commands=488 min_tasks=245 min_commands=490 breadth_ok=no"
        ));
    }

    #[test]
    fn release_gate_blocks_when_fixture_risk_coverage_below_minimum() {
        let mut eval = build_eval_result(
            "local-smoke",
            "genome-v1",
            None,
            &required_gate_results(true),
            10,
        );
        eval.eval_id = "eval-risk-pass".into();
        eval.created_at_ms = 10_000;
        eval.metrics["fixture_suite"] = json!({
            "task_count": 250,
            "command_count": 500,
            "risk_labels": {
                "high": 4,
                "medium": 120,
                "low": 126
            }
        });
        let events = vec![json!({
            "event_id": "evt-risk-pass",
            "event_type": "PatchEvaluated",
            "timestamp_ms": 10_000,
            "payload": eval,
        })];

        let report = build_release_gate_report_with_policy(
            &events,
            "local-smoke",
            1,
            10_000 + 3_599_999,
            ReleaseGatePolicy {
                require_protocol: false,
                min_fixture_tasks: None,
                min_fixture_commands: None,
                min_fixture_risk_labels: BTreeMap::from([
                    ("high".to_string(), 5),
                    ("medium".to_string(), 100),
                ]),
            },
        );

        assert!(!report.ready);
        assert!(report.fixture_breadth_satisfied);
        assert!(!report.fixture_risk_satisfied);
        assert_eq!(report.last_eval_fixture_risk_labels["high"], 4);
        assert_eq!(report.min_fixture_risk_labels["high"], 5);
        assert!(report
            .reason
            .contains("latest eval fixture risk coverage is below required minimum"));
        assert_eq!(report.to_json()["fixture_risk_satisfied"], false);
        assert_eq!(report.to_json()["min_fixture_risk_labels"]["high"], 5);
        assert!(format_release_gate_report(&report)
            .contains("fixture risk gate: min=high=5, medium=100 satisfied=no"));
    }

    #[test]
    fn release_gate_blocks_dirty_worktree_eval_artifact() {
        let mut eval = build_eval_result(
            "local-smoke",
            "genome-v1",
            None,
            &required_gate_results(true),
            10,
        );
        eval.eval_id = "eval-dirty-pass".into();
        eval.created_at_ms = 10_000;
        eval.metrics["reproducibility"] = json!({
            "mode": "fixture",
            "replay_command": "yoyo eval fixtures run --suite local-smoke",
            "git_dirty": true,
            "git_status_short": [" M src/commands_eval.rs"],
            "commands": crate::deepseek::DeepSeekHarnessGenome::default().test_policy.required_gates,
        });
        let events = vec![json!({
            "event_id": "evt-dirty-pass",
            "event_type": "PatchEvaluated",
            "timestamp_ms": 10_000,
            "payload": eval,
        })];

        let report = build_release_gate_report(&events, "local-smoke", 1, 10_000 + 3_599_999);

        assert!(!report.ready);
        assert_eq!(report.reason, "latest eval was run from a dirty worktree");
        assert_eq!(report.last_eval_git_dirty, Some(true));
        assert_eq!(report.to_json()["last_eval_git_dirty"], true);
        assert!(format_release_gate_report(&report).contains("last eval dirty: yes"));
    }

    #[test]
    fn release_gate_blocks_missing_required_gate_evidence() {
        let mut eval = build_eval_result(
            "local-smoke",
            "genome-v1",
            None,
            &[gate("cargo check", true)],
            10,
        );
        eval.eval_id = "eval-incomplete-pass".into();
        eval.created_at_ms = 10_000;
        let events = vec![json!({
            "event_id": "evt-incomplete-pass",
            "event_type": "PatchEvaluated",
            "timestamp_ms": 10_000,
            "payload": eval,
        })];

        let report = build_release_gate_report(&events, "local-smoke", 1, 10_000 + 3_599_999);

        assert!(!report.ready);
        assert!(report
            .reason
            .contains("latest eval is missing required gate evidence"));
        assert!(report
            .missing_required_gates
            .contains(&"cargo fmt --check".to_string()));
        assert!(format_release_gate_report(&report).contains("missing gates:"));
        assert!(report.to_json()["missing_required_gates"]
            .as_array()
            .map(|items| !items.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn release_gate_blocks_fixture_agent_mutation_scope_failures() {
        let mut eval = build_eval_result(
            "local-smoke",
            "genome-v1",
            None,
            &required_gate_results(true),
            10,
        );
        eval.eval_id = "eval-scope-fail".into();
        eval.created_at_ms = 10_000;
        eval.metrics["state_metrics"] = json!({
            "fixture_agent_attempts": 2,
            "fixture_agent_mutation_scope_failures": 1,
            "fixture_agent_unexpected_changed_file_count": 3,
            "fixture_agent_mutation_scope_failure_rate": 0.5
        });
        let events = vec![json!({
            "event_id": "evt-scope-fail",
            "event_type": "PatchEvaluated",
            "timestamp_ms": 10_000,
            "payload": eval,
        })];

        let report = build_release_gate_report(&events, "local-smoke", 1, 10_000 + 3_599_999);

        assert!(!report.ready);
        assert_eq!(
            report.reason,
            "latest eval has fixture agent mutation-scope failures: 1"
        );
        assert_eq!(report.last_eval_mutation_scope_failures, Some(1));
        assert_eq!(report.last_eval_unexpected_changed_files, Some(3));
        assert_eq!(report.to_json()["last_eval_mutation_scope_failures"], 1);
        assert_eq!(report.to_json()["last_eval_unexpected_changed_files"], 3);
        assert!(format_release_gate_report(&report)
            .contains("fixture agent: scope_failures=1 unexpected_files=3"));
    }

    #[test]
    fn source_provenance_finding_summaries_bound_each_summary() {
        let audit = crate::release::SourceProvenanceAudit {
            policy_version: crate::release::SOURCE_PROVENANCE_POLICY_VERSION,
            allowed_reference_domains: Vec::new(),
            forbidden_markers: Vec::new(),
            scan_source: "git",
            scanned_files: 1,
            skipped_files: 0,
            findings: vec![crate::release::SourceProvenanceFinding {
                path: format!("src/{}/leaked_source.rs", "very/deep/path".repeat(20)),
                marker: format!("{} forbidden copied source marker", "external ".repeat(20)),
            }],
            passed: false,
        };

        let summaries = source_provenance_finding_summaries(&audit);
        assert_eq!(summaries.len(), 1);
        assert!(
            summaries[0].chars().count() <= SOURCE_PROVENANCE_FINDING_SUMMARY_MAX_CHARS,
            "{}",
            summaries[0]
        );
        assert!(summaries[0].contains(": external external"));
        assert!(summaries[0].contains("..."));
        assert!(!summaries[0].contains("forbidden copied source marker"));
    }

    #[test]
    fn release_gate_source_provenance_findings_are_bounded_and_visible() {
        let audit = crate::release::SourceProvenanceAudit {
            policy_version: crate::release::SOURCE_PROVENANCE_POLICY_VERSION,
            allowed_reference_domains: Vec::new(),
            forbidden_markers: Vec::new(),
            scan_source: "git",
            scanned_files: 8,
            skipped_files: 1,
            findings: vec![
                crate::release::SourceProvenanceFinding {
                    path: "src/a.rs".to_string(),
                    marker: "source path escapes repository".to_string(),
                },
                crate::release::SourceProvenanceFinding {
                    path: "src/b.rs".to_string(),
                    marker: "source file unreadable".to_string(),
                },
                crate::release::SourceProvenanceFinding {
                    path: "src/c.rs".to_string(),
                    marker: "source file unreadable".to_string(),
                },
                crate::release::SourceProvenanceFinding {
                    path: "src/d.rs".to_string(),
                    marker: "source file unreadable".to_string(),
                },
                crate::release::SourceProvenanceFinding {
                    path: "src/e.rs".to_string(),
                    marker: "source file unreadable".to_string(),
                },
                crate::release::SourceProvenanceFinding {
                    path: "src/f.rs".to_string(),
                    marker: "source file unreadable".to_string(),
                },
            ],
            passed: false,
        };
        let summaries = source_provenance_finding_summaries(&audit);
        assert_eq!(summaries.len(), SOURCE_PROVENANCE_FINDING_SUMMARY_LIMIT);

        let report = ReleaseGateReport {
            suite: "local-smoke".to_string(),
            max_age_hours: 1,
            ready: false,
            reason: "source provenance audit did not pass".to_string(),
            last_eval_id: Some("eval-pass".to_string()),
            last_eval_status: Some("passed".to_string()),
            last_eval_ms: Some(10_000),
            last_eval_git_dirty: Some(false),
            last_eval_fixture_task_count: None,
            last_eval_fixture_command_count: None,
            last_eval_fixture_risk_labels: BTreeMap::new(),
            last_eval_model_route_tasks: BTreeMap::new(),
            last_eval_mutation_scope_failures: None,
            last_eval_unexpected_changed_files: None,
            min_fixture_task_count: None,
            min_fixture_command_count: None,
            min_fixture_risk_labels: BTreeMap::new(),
            fixture_breadth_satisfied: true,
            fixture_risk_satisfied: true,
            missing_required_gates: Vec::new(),
            stale: false,
            replay_failures_after_eval: 0,
            replay_command: None,
            require_protocol: false,
            protocol_eval_id: None,
            protocol_eval_status: None,
            protocol_eval_ms: None,
            protocol_eval_git_dirty: None,
            protocol_check_counts: None,
            protocol_stale: false,
            protocol_older_than_eval: false,
            source_provenance_passed: false,
            source_provenance_findings: audit.findings.len(),
            source_provenance_finding_summaries: summaries,
            source_provenance_scan_source: audit.scan_source.to_string(),
            source_provenance_scanned_files: audit.scanned_files,
            source_provenance_skipped_files: audit.skipped_files,
            now_ms: 20_000,
        };

        let value = report.to_json();
        assert_eq!(value["source_provenance_findings"], 6);
        assert_eq!(
            value["source_provenance_finding_summaries"]
                .as_array()
                .unwrap()
                .len(),
            SOURCE_PROVENANCE_FINDING_SUMMARY_LIMIT
        );
        let formatted = format_release_gate_report(&report);
        assert!(formatted.contains("source finding: src/a.rs: source path escapes repository"));
        assert!(!formatted.contains("src/f.rs"));
    }

    #[test]
    fn release_gate_blocks_failures_recorded_after_latest_eval() {
        let mut eval = build_eval_result(
            "local-smoke",
            "genome-v1",
            None,
            &required_gate_results(true),
            10,
        );
        eval.eval_id = "eval-pass".into();
        eval.created_at_ms = 10_000;
        let events = vec![
            json!({
                "event_id": "evt-pass",
                "event_type": "PatchEvaluated",
                "timestamp_ms": 10_000,
                "payload": eval,
            }),
            json!({
                "event_id": "evt-before",
                "event_type": "FailureObserved",
                "timestamp_ms": 9_000,
                "payload": {"source": "old", "error_preview": "covered by eval"},
            }),
            json!({
                "event_id": "evt-after",
                "event_type": "ToolSchemaFailure",
                "timestamp_ms": 10_001,
                "payload": {"source": "strict_tool_schema", "error_preview": "new failure"},
            }),
        ];

        let report = build_release_gate_report(&events, "local-smoke", 1, 10_000 + 3_599_999);

        assert!(!report.ready);
        assert!(!report.stale);
        assert_eq!(
            report.reason,
            "state failures were recorded after latest eval"
        );
        assert_eq!(report.replay_failures_after_eval, 1);
        assert_eq!(
            report.replay_command.as_deref(),
            Some("yoyo eval replay --from-state --limit 1")
        );
        assert!(format_release_gate_report(&report).contains("replay command"));
    }

    #[test]
    fn release_gate_requires_fresh_passing_protocol_eval_when_requested() {
        let mut eval = build_eval_result(
            "local-smoke",
            "genome-v1",
            None,
            &required_gate_results(true),
            10,
        );
        eval.eval_id = "eval-pass".into();
        eval.created_at_ms = 10_000;
        let base_event = json!({
            "event_id": "evt-pass",
            "event_type": "PatchEvaluated",
            "timestamp_ms": 10_000,
            "payload": eval,
        });

        let no_protocol = build_release_gate_report_with_options(
            std::slice::from_ref(&base_event),
            "local-smoke",
            1,
            10_000 + 3_599_999,
            true,
        );
        assert!(!no_protocol.ready);
        assert_eq!(no_protocol.reason, "no protocol eval found");
        assert!(no_protocol.protocol_stale);
        assert!(format_release_gate_report(&no_protocol).contains("protocol req:  yes"));

        let mut failed_protocol = build_eval_result(
            "protocol-deepseek",
            "genome-v1",
            None,
            &[gate("deepseek protocol", false)],
            10,
        );
        failed_protocol.eval_id = "eval-protocol-failed".into();
        failed_protocol.created_at_ms = 11_000;
        let failed_report = build_release_gate_report_with_options(
            &[
                base_event.clone(),
                json!({
                    "event_id": "evt-protocol-failed",
                    "event_type": "PatchEvaluated",
                    "timestamp_ms": 11_000,
                    "payload": failed_protocol,
                }),
            ],
            "local-smoke",
            1,
            11_000,
            true,
        );
        assert!(!failed_report.ready);
        assert_eq!(failed_report.reason, "latest protocol eval did not pass");
        assert_eq!(
            failed_report.protocol_eval_id.as_deref(),
            Some("eval-protocol-failed")
        );

        let mut passed_protocol = build_eval_result(
            "protocol-deepseek",
            "genome-v1",
            None,
            &[gate("deepseek protocol", true)],
            10,
        );
        passed_protocol.eval_id = "eval-protocol-pass".into();
        passed_protocol.created_at_ms = 12_000;
        passed_protocol.metrics["state_metrics"] = json!({
            "deepseek_protocol_checks": 5,
            "deepseek_protocol_passes": 5,
            "deepseek_strict_tool_call_checks": 1,
            "deepseek_thinking_protocol_checks": 1,
            "deepseek_streaming_protocol_checks": 1,
            "deepseek_json_output_checks": 1,
            "deepseek_transport_policy_checks": 1
        });
        let ready = build_release_gate_report_with_policy_and_source_audit(
            &[
                base_event,
                json!({
                    "event_id": "evt-protocol-pass",
                    "event_type": "PatchEvaluated",
                    "timestamp_ms": 12_000,
                    "payload": passed_protocol,
                }),
            ],
            "local-smoke",
            1,
            12_000,
            ReleaseGatePolicy {
                require_protocol: true,
                min_fixture_tasks: None,
                min_fixture_commands: None,
                min_fixture_risk_labels: BTreeMap::new(),
            },
            passing_source_provenance_audit(),
        );
        assert!(ready.ready);
        assert_eq!(
            ready.reason,
            "latest eval and protocol eval passed and are fresh"
        );
        assert_eq!(ready.protocol_eval_status.as_deref(), Some("passed"));
        let counts = ready
            .protocol_check_counts
            .as_ref()
            .expect("protocol check counts");
        assert_eq!(counts.total, Some(5));
        assert_eq!(counts.passes, Some(5));
        assert_eq!(counts.stream, Some(1));
        assert_eq!(ready.to_json()["require_protocol"], true);
        assert_eq!(ready.to_json()["protocol_check_counts"]["stream"], 1);
        assert!(format_release_gate_report(&ready)
            .contains("protocol checks: 5/5 strict=1 thinking=1 stream=1 json=1 transport=1"));
    }

    #[test]
    fn release_gate_blocks_dirty_protocol_eval_when_required() {
        let mut eval = build_eval_result(
            "local-smoke",
            "genome-v1",
            None,
            &required_gate_results(true),
            10,
        );
        eval.eval_id = "eval-pass".into();
        eval.created_at_ms = 10_000;

        let mut protocol_eval = build_eval_result(
            "protocol-deepseek",
            "genome-v1",
            None,
            &[gate("deepseek protocol", true)],
            10,
        );
        protocol_eval.eval_id = "eval-protocol-dirty".into();
        protocol_eval.created_at_ms = 11_000;
        protocol_eval.metrics["reproducibility"] = json!({
            "mode": "protocol",
            "replay_command": "yoyo deepseek protocol-diagnostic",
            "git_dirty": true,
            "git_status_short": [" M src/deepseek.rs"],
            "commands": ["yoyo deepseek protocol-diagnostic"]
        });

        let report = build_release_gate_report_with_options(
            &[
                json!({
                    "event_id": "evt-pass",
                    "event_type": "PatchEvaluated",
                    "timestamp_ms": 10_000,
                    "payload": eval,
                }),
                json!({
                    "event_id": "evt-protocol-dirty",
                    "event_type": "PatchEvaluated",
                    "timestamp_ms": 11_000,
                    "payload": protocol_eval,
                }),
            ],
            "local-smoke",
            1,
            11_000,
            true,
        );

        assert!(!report.ready);
        assert_eq!(
            report.reason,
            "latest protocol eval was run from a dirty worktree"
        );
        assert_eq!(report.protocol_eval_git_dirty, Some(true));
        assert_eq!(report.to_json()["protocol_eval_git_dirty"], true);
        assert!(format_release_gate_report(&report).contains("protocol dirty: yes"));
    }

    #[test]
    fn release_gate_blocks_protocol_eval_older_than_latest_suite_eval() {
        let mut eval = build_eval_result(
            "local-smoke",
            "genome-v1",
            None,
            &required_gate_results(true),
            10,
        );
        eval.eval_id = "eval-pass".into();
        eval.created_at_ms = 20_000;

        let mut protocol_eval = build_eval_result(
            "protocol-deepseek",
            "genome-v1",
            None,
            &[gate("deepseek protocol", true)],
            10,
        );
        protocol_eval.eval_id = "eval-protocol-pass".into();
        protocol_eval.created_at_ms = 10_000;

        let report = build_release_gate_report_with_options(
            &[
                json!({
                    "event_id": "evt-protocol-pass",
                    "event_type": "PatchEvaluated",
                    "timestamp_ms": 10_000,
                    "payload": protocol_eval,
                }),
                json!({
                    "event_id": "evt-pass",
                    "event_type": "PatchEvaluated",
                    "timestamp_ms": 20_000,
                    "payload": eval,
                }),
            ],
            "local-smoke",
            1,
            20_000,
            true,
        );

        assert!(!report.ready);
        assert_eq!(
            report.reason,
            "latest protocol eval is older than latest suite eval"
        );
        assert!(report.protocol_older_than_eval);
        assert_eq!(report.to_json()["protocol_older_than_eval"], true);
        assert!(format_release_gate_report(&report).contains("protocol older than eval: yes"));
    }

    #[test]
    fn eval_artifact_writer_attaches_report_uri_and_writes_json() {
        let mut eval = build_eval_result("local", "a", None, &[gate("a", true)], 10);
        eval.eval_id = "eval/artifact test".to_string();
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("state").join("events.jsonl");

        let (eval, artifact_path) = attach_eval_artifact(&eval, &events_path).unwrap();

        assert_eq!(
            artifact_path,
            dir.path()
                .join("state")
                .join("artifacts")
                .join("evals")
                .join("eval_artifact_test.json")
        );
        let expected_uri = artifact_path.display().to_string();
        assert_eq!(
            eval.metrics["artifact_uri"].as_str(),
            Some(expected_uri.as_str())
        );
        let raw = std::fs::read_to_string(&artifact_path).unwrap();
        let artifact: EvalResult = serde_json::from_str(&raw).unwrap();
        assert_eq!(artifact.eval_id, "eval/artifact test");
        assert_eq!(
            artifact.metrics["artifacts"][0]["kind"].as_str(),
            Some("eval_report")
        );
    }

    #[test]
    fn eval_artifact_writer_redacts_command_preview_secrets() {
        let mut eval = build_eval_result("local", "a", None, &[gate("a", true)], 10);
        eval.eval_id = "eval-secret".to_string();
        eval.metrics["commands"] = json!([{
            "command": "print-secret",
            "stdout_preview": "api_key=sk-testsecret123456789",
            "stderr_preview": "Authorization: Bearer secretbearer123456"
        }]);
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("state").join("events.jsonl");

        let (eval, artifact_path) = attach_eval_artifact(&eval, &events_path).unwrap();
        let raw = std::fs::read_to_string(&artifact_path).unwrap();

        assert!(!raw.contains("sk-testsecret123456789"));
        assert!(!raw.contains("secretbearer123456"));
        assert!(raw.contains("[redacted]"));
        assert!(!serde_json::to_string(&eval)
            .unwrap()
            .contains("sk-testsecret123456789"));
    }

    #[test]
    fn eval_reproducibility_manifest_records_replay_command_and_gates() {
        let mut eval = build_eval_result(
            "local-smoke",
            "genome-v1",
            Some("patch-1".into()),
            &[gate("cargo check", true), gate("cargo test", true)],
            20,
        );
        let options = EvalRunOptions {
            suite: "local-smoke",
            harness_version: "genome-v1",
            patch_id: Some("patch-1".into()),
            dry_run: false,
            workdir: None,
            task_log: None,
            task_git_base: None,
        };

        attach_eval_reproducibility_manifest(
            &mut eval,
            "gates",
            None,
            eval_run_replay_command(&options),
            vec!["cargo check".into(), "cargo test".into()],
        );

        let manifest = &eval.metrics["reproducibility"];
        assert_eq!(manifest["manifest_version"], 1);
        assert_eq!(manifest["mode"], "gates");
        assert_eq!(manifest["suite"], "local-smoke");
        assert_eq!(manifest["patch_id"], "patch-1");
        assert!(manifest["git_dirty"].is_boolean());
        assert!(manifest["git_status_short"].is_array());
        assert_eq!(manifest["commands"].as_array().unwrap().len(), 2);
        assert!(manifest["replay_command"]
            .as_str()
            .unwrap()
            .contains("--patch-id patch-1"));
        assert_eq!(eval.metrics["eval_type"], "self_evolution");
        assert_eq!(manifest["eval_type"], "self_evolution");
    }

    #[test]
    fn reproducibility_manifest_detects_dirty_worktree() {
        let dir = tempfile::tempdir().unwrap();
        let status = Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .arg("init")
            .status()
            .unwrap();
        assert!(status.success());
        std::fs::write(dir.path().join("uncommitted.txt"), "dirty\n").unwrap();

        assert_eq!(current_git_dirty(Some(dir.path())), Some(true));
        let status_lines = current_git_status_short(Some(dir.path()), 4);

        assert!(status_lines
            .iter()
            .any(|line| line.contains("uncommitted.txt")));
    }

    #[test]
    fn eval_report_surfaces_dirty_worktree_reproducibility_evidence() {
        let mut eval = build_eval_result(
            "local-smoke",
            "genome-v1",
            Some("patch-1".into()),
            &[gate("cargo check", true)],
            10,
        );
        eval.metrics["reproducibility"] = json!({
            "manifest_version": 1,
            "mode": "gates",
            "eval_type": "self_evolution",
            "suite": "local-smoke",
            "git_dirty": true,
            "git_status_short": [" M src/context.rs"],
            "replay_command": "yoyo eval run --suite local-smoke",
            "commands": ["cargo check"]
        });

        let report = format_eval_report(&eval);

        assert!(report.contains("git dirty: yes"));
        assert!(report.contains("git status lines: 1"));
        assert!(report.contains("M src/context.rs"));
    }

    #[test]
    fn eval_report_surfaces_protocol_reproducibility_commands() {
        let genome = crate::deepseek::DeepSeekHarnessGenome::default();
        let gates = eval_gates_for_suite("protocol-deepseek", &genome)
            .into_iter()
            .map(|command| gate(&command, true))
            .collect::<Vec<_>>();
        let mut eval = build_eval_result("protocol-deepseek", &genome.version, None, &gates, 20);
        let options = EvalRunOptions {
            suite: "protocol-deepseek",
            harness_version: &genome.version,
            patch_id: None,
            dry_run: false,
            workdir: None,
            task_log: None,
            task_git_base: None,
        };
        attach_eval_reproducibility_manifest(
            &mut eval,
            "gates",
            None,
            eval_run_replay_command(&options),
            gates.iter().map(|gate| gate.command.clone()).collect(),
        );

        let report = format_eval_report(&eval);

        assert!(report.contains("type:    protocol"));
        assert!(report.contains("reproducibility:"));
        assert!(report.contains("mode:     gates"));
        assert!(report.contains("replay:   yoyo eval run --suite protocol-deepseek"));
        assert!(report.contains("deepseek test-tool-call --record --json"));
        assert!(report.contains("deepseek test-thinking --record --json"));
        assert!(report.contains("deepseek stream-check --record --json"));
        assert!(report.contains("deepseek json-check --input '{\"ok\":true}' --record --json"));
        assert!(report.contains("deepseek transport-check --status 429"));
        assert!(
            report.contains("transport-check --status 429 --error 'rate limit' --record --json")
        );
    }

    #[test]
    fn eval_result_classifies_layered_eval_types() {
        let harness_eval = build_eval_result(
            "local-smoke",
            "genome-v1",
            None,
            &[gate("cargo check", true)],
            10,
        );
        let protocol_eval = build_eval_result(
            "protocol-deepseek",
            "genome-v1",
            None,
            &[gate("cargo test deepseek", true)],
            10,
        );
        let self_evolution_eval = build_eval_result(
            "local-smoke",
            "genome-v1",
            Some("patch-1".into()),
            &[gate("cargo check", true)],
            10,
        );
        let regression_eval = build_eval_result(
            "state-regression-replay",
            "genome-v1",
            None,
            &[gate("yoyo eval replay --from-state", true)],
            10,
        );
        let fixture_eval = build_fixture_eval_result("local-smoke", "genome-v1", &[], 10);

        assert_eq!(harness_eval.metrics["eval_type"], "harness");
        assert_eq!(protocol_eval.metrics["eval_type"], "protocol");
        assert_eq!(self_evolution_eval.metrics["eval_type"], "self_evolution");
        assert_eq!(regression_eval.metrics["eval_type"], "regression");
        assert_eq!(fixture_eval.metrics["eval_type"], "coding_task");
    }

    #[test]
    fn protocol_eval_suite_uses_deepseek_protocol_gates() {
        let genome = crate::deepseek::DeepSeekHarnessGenome::default();

        let protocol_gates = eval_gates_for_suite("protocol-deepseek", &genome);
        let benchmark_gates = eval_gates_for_suite("local-smoke", &genome);

        assert_eq!(protocol_gates, genome.test_policy.protocol_gates);
        assert_eq!(benchmark_gates, genome.test_policy.required_gates);
        assert!(protocol_gates
            .iter()
            .any(|gate| gate.contains("deepseek test-tool-call --record --json")));
        assert!(protocol_gates
            .iter()
            .any(|gate| gate.contains("deepseek test-thinking --record --json")));
        assert!(protocol_gates
            .iter()
            .any(|gate| gate.contains("deepseek stream-check --record --json")));
        assert!(protocol_gates.iter().any(
            |gate| gate.contains("deepseek json-check --input '{\"ok\":true}' --record --json")
        ));
        assert!(protocol_gates
            .iter()
            .any(|gate| gate.contains("deepseek transport-check")
                && gate.contains("--record")
                && gate.contains("--json")));
        assert!(!protocol_gates.iter().any(|gate| gate == "cargo check"));
    }

    #[test]
    fn protocol_eval_gates_record_pass_evidence_for_state_lineage() {
        let genome = crate::deepseek::DeepSeekHarnessGenome::default();

        let protocol_gates = eval_gates_for_suite("protocol-deepseek", &genome);

        for command in [
            "deepseek test-tool-call",
            "deepseek test-thinking",
            "deepseek stream-check",
            "deepseek json-check",
            "deepseek transport-check",
        ] {
            let gate = protocol_gates
                .iter()
                .find(|gate| gate.contains(command))
                .unwrap_or_else(|| panic!("missing protocol gate for {command}"));
            assert!(
                gate.contains("--record"),
                "protocol gate '{gate}' must record pass evidence"
            );
            assert!(
                gate.contains("--json"),
                "protocol gate '{gate}' must keep machine-readable output"
            );
        }
    }

    #[test]
    fn fixture_reproducibility_manifest_records_commands() {
        let result = crate::eval_fixtures::FixtureTaskResult {
            task_id: "task-1".into(),
            passed: true,
            command_results: vec![crate::eval_fixtures::FixtureCommandResult {
                command: "cargo test task_1".into(),
                passed: true,
                status_code: Some(0),
                duration_ms: 10,
                stdout_preview: String::new(),
                stderr_preview: String::new(),
            }],
        };
        let mut eval = build_fixture_eval_result(
            "local-smoke",
            "genome-v1",
            std::slice::from_ref(&result),
            10,
        );

        attach_eval_reproducibility_manifest(
            &mut eval,
            "fixtures",
            None,
            "yoyo eval fixtures run --suite local-smoke".into(),
            fixture_result_commands(&[result]),
        );

        let manifest = &eval.metrics["reproducibility"];
        assert_eq!(manifest["mode"], "fixtures");
        assert_eq!(manifest["commands"][0], "cargo test task_1");
    }

    #[test]
    fn fixture_eval_records_suite_category_and_risk_metadata() {
        let task = crate::eval_fixtures::BenchmarkTask {
            task_id: "task-1".into(),
            category: "context-miss challenge".into(),
            repo_fixture: "self".into(),
            initial_commit: "current".into(),
            goal: "keep context evidence visible".into(),
            tests: vec!["cargo test context_task".into(), "cargo check".into()],
            hidden_failure_mode: "context category hidden from eval summary".into(),
            expected_files: vec!["src/context.rs".into()],
            risk_label: "medium".into(),
        };
        let result = crate::eval_fixtures::FixtureTaskResult {
            task_id: "task-1".into(),
            passed: true,
            command_results: Vec::new(),
        };
        let mut eval = build_fixture_eval_result(
            "local-smoke",
            "genome-v1",
            std::slice::from_ref(&result),
            10,
        );

        attach_fixture_suite_metadata(&mut eval, &[&task]);

        assert_eq!(eval.metrics["fixture_suite"]["task_count"], 1);
        assert_eq!(eval.metrics["fixture_suite"]["command_count"], 2);
        assert_eq!(
            eval.metrics["fixture_suite"]["categories"]["context-miss challenge"],
            1
        );
        assert_eq!(eval.metrics["fixture_suite"]["risk_labels"]["medium"], 1);
        let report = format_eval_report(&eval);
        assert!(report.contains("fixture suite:"));
        assert!(report.contains("categories: context-miss challenge=1"));
        assert!(report.contains("risks:    medium=1"));
    }

    #[test]
    fn shell_quote_preserves_replayable_arguments() {
        assert_eq!(shell_quote("local-smoke"), "local-smoke");
        assert_eq!(
            shell_quote("agent --flag 'quoted'"),
            "'agent --flag '\\''quoted'\\'''"
        );
    }

    #[test]
    fn default_fixture_agent_command_uses_current_binary_and_prompt_placeholder() {
        let command = default_fixture_agent_command_template();
        assert!(!command.contains("--deepseek-native"));
        assert!(command.contains("--yes"));
        assert!(command.contains("--no-update-check"));
        assert!(command.contains("--prompt {goal}"));
    }

    #[test]
    fn fixture_attempt_agent_command_prefers_explicit_command_over_default() {
        let args = vec![
            "yoyo".to_string(),
            "eval".to_string(),
            "fixtures".to_string(),
            "attempt".to_string(),
            "--default-agent".to_string(),
            "--agent-command".to_string(),
            "custom {goal}".to_string(),
        ];

        let command = fixture_attempt_agent_command_spec(&args, true).unwrap();
        assert_eq!(command.template, "custom {goal}");
        assert_eq!(command.source, "explicit");
    }

    #[test]
    fn fixture_agent_attempt_reproducibility_records_agent_command_source() {
        let args = vec![
            "yoyo".to_string(),
            "eval".to_string(),
            "fixtures".to_string(),
            "attempt".to_string(),
            "--default-agent".to_string(),
        ];
        let command = fixture_attempt_agent_command_spec(&args, true).unwrap();
        assert_eq!(command.source, "default-agent");

        let result = crate::eval_fixtures::FixtureAgentAttemptResult {
            task_id: "task-1".into(),
            passed: true,
            worktree: "/tmp/worktree".into(),
            agent_command: "yyds --prompt goal".into(),
            agent_result: crate::eval_fixtures::FixtureCommandResult {
                command: "yyds --prompt goal".into(),
                passed: true,
                status_code: Some(0),
                duration_ms: 10,
                stdout_preview: String::new(),
                stderr_preview: String::new(),
            },
            mutation_scope_passed: true,
            changed_files: vec!["src/context.rs".into()],
            unexpected_changed_files: Vec::new(),
            command_results: vec![crate::eval_fixtures::FixtureCommandResult {
                command: "cargo test task_1".into(),
                passed: true,
                status_code: Some(0),
                duration_ms: 10,
                stdout_preview: String::new(),
                stderr_preview: String::new(),
            }],
        };
        let mut eval = build_fixture_agent_attempt_eval_result(
            "local-smoke",
            "genome-v1",
            std::slice::from_ref(&result),
            10,
        );
        attach_eval_reproducibility_manifest(
            &mut eval,
            "fixture-agent-attempts",
            None,
            "yoyo eval fixtures attempt --suite local-smoke --default-agent".into(),
            fixture_agent_attempt_commands(&[result]),
        );
        attach_fixture_agent_attempt_source(&mut eval, command.source);

        assert_eq!(
            eval.metrics["reproducibility"]["agent_command_source"],
            "default-agent"
        );
        let report = format_eval_report(&eval);
        assert!(report.contains("agent source: default-agent"));
        assert!(report.contains("fixture agent patch:"));
        assert!(report.contains("changed files: 1"));
        assert!(report.contains("src/context.rs"));
    }

    #[test]
    fn eval_report_surfaces_fixture_agent_unexpected_changed_files() {
        let result = crate::eval_fixtures::FixtureAgentAttemptResult {
            task_id: "task-1".into(),
            passed: false,
            worktree: "/tmp/worktree".into(),
            agent_command: "yyds --prompt goal".into(),
            agent_result: crate::eval_fixtures::FixtureCommandResult {
                command: "yyds --prompt goal".into(),
                passed: true,
                status_code: Some(0),
                duration_ms: 10,
                stdout_preview: String::new(),
                stderr_preview: String::new(),
            },
            mutation_scope_passed: false,
            changed_files: vec!["src/context.rs".into(), "src/lib.rs".into()],
            unexpected_changed_files: vec!["src/lib.rs".into()],
            command_results: Vec::new(),
        };
        let eval =
            build_fixture_agent_attempt_eval_result("local-smoke", "genome-v1", &[result], 10);

        let report = format_eval_report(&eval);

        assert!(report.contains("fixture agent patch:"));
        assert!(report.contains("changed files: 2"));
        assert!(report.contains("unexpected files: 1"));
        assert!(report.contains("! src/lib.rs"));
    }

    #[test]
    fn find_eval_reads_canonical_yoagent_state_event_payloads() {
        let eval = build_eval_result("local", "a", None, &[gate("a", true)], 10);
        let event = crate::state::StateEvent {
            event_id: "evt-eval".into(),
            event_type: EventType::PatchEvaluated,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-1".into()),
            session_id: None,
            trace_id: "trace-1".into(),
            parent_event_ids: Vec::new(),
            payload: serde_json::to_value(&eval).unwrap(),
        };
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        crate::state::append_event(&path, &event).unwrap();
        let events = read_events(&path).unwrap();

        let found = find_eval(&events, &eval.eval_id).unwrap();

        assert_eq!(found.eval_id, eval.eval_id);
        assert_eq!(found.score, Some(1.0));
    }

    #[test]
    fn find_eval_resolves_latest_eval_by_run_or_trace_reference() {
        let mut old_eval = build_eval_result("local", "old", None, &[gate("old", true)], 10);
        old_eval.eval_id = "eval-old".to_string();
        let mut new_eval = build_eval_result("local", "new", None, &[gate("new", true)], 10);
        new_eval.eval_id = "eval-new".to_string();
        let events = vec![
            json!({
                "event_id": "evt-old",
                "event_type": "PatchEvaluated",
                "timestamp_ms": 10,
                "run_id": "run-eval",
                "trace_id": "trace-eval",
                "payload": old_eval,
            }),
            json!({
                "event_id": "evt-new",
                "event_type": "PatchEvaluated",
                "timestamp_ms": 20,
                "run_id": "run-eval",
                "trace_id": "trace-eval",
                "payload": new_eval,
            }),
        ];

        assert_eq!(find_eval(&events, "run-eval").unwrap().eval_id, "eval-new");
        assert_eq!(
            find_eval(&events, "trace-eval").unwrap().eval_id,
            "eval-new"
        );
        assert_eq!(find_eval(&events, "evt-old").unwrap().eval_id, "eval-old");
        assert_eq!(find_eval(&events, "eval-old").unwrap().eval_id, "eval-old");
    }

    #[test]
    fn failed_eval_gates_record_failure_events_and_link_eval() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let recorder = StateRecorder::new(StateConfig {
            enabled: true,
            fail_soft: false,
            events_path: events_path.clone(),
            store_path: None,
        });
        let mut eval = build_eval_result(
            "local-smoke",
            "genome-v1",
            Some("patch-1".into()),
            &[gate("cargo check", true), gate("cargo test", false)],
            20,
        );

        record_gate_failure_events_with(
            &mut eval,
            &[gate("cargo check", true), gate("cargo test", false)],
            &recorder,
        )
        .unwrap();

        assert_eq!(eval.failure_event_ids.len(), 1);
        let events = read_events(&events_path).unwrap();
        let failure = events
            .iter()
            .find(|event| event["event_type"] == "FailureObserved")
            .unwrap();
        assert_eq!(failure["payload"]["source"], "eval_gate");
        assert_eq!(failure["payload"]["eval_id"], eval.eval_id);
        assert_eq!(failure["payload"]["patch_id"], "patch-1");
        assert_eq!(failure["payload"]["command"], "cargo test");
    }

    #[test]
    fn failed_fixture_tasks_record_failure_events_and_link_eval() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let recorder = StateRecorder::new(StateConfig {
            enabled: true,
            fail_soft: false,
            events_path: events_path.clone(),
            store_path: None,
        });
        let result = crate::eval_fixtures::FixtureTaskResult {
            task_id: "context-miss".into(),
            passed: false,
            command_results: vec![crate::eval_fixtures::FixtureCommandResult {
                command: "cargo test context".into(),
                passed: false,
                status_code: Some(101),
                duration_ms: 12,
                stdout_preview: String::new(),
                stderr_preview: "missing src/context.rs".into(),
            }],
        };
        let mut eval = build_fixture_eval_result(
            "local-smoke",
            "genome-v1",
            std::slice::from_ref(&result),
            20,
        );

        record_fixture_failure_events_with(&mut eval, &[result], &recorder).unwrap();

        assert_eq!(eval.failure_event_ids.len(), 1);
        let events = read_events(&events_path).unwrap();
        let failure = events
            .iter()
            .find(|event| event["event_type"] == "FailureObserved")
            .unwrap();
        assert_eq!(failure["payload"]["source"], "eval_fixture_task");
        assert_eq!(failure["payload"]["eval_id"], eval.eval_id);
        assert_eq!(failure["payload"]["task_id"], "context-miss");
        assert_eq!(
            failure["payload"]["failed_commands"][0]["error_preview"],
            "missing src/context.rs"
        );
    }

    #[test]
    fn failed_fixture_agent_attempts_record_failure_events_and_link_eval() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let recorder = StateRecorder::new(StateConfig {
            enabled: true,
            fail_soft: false,
            events_path: events_path.clone(),
            store_path: None,
        });
        let result = crate::eval_fixtures::FixtureAgentAttemptResult {
            task_id: "context-miss".into(),
            passed: false,
            worktree: "/tmp/worktree".into(),
            agent_command: "yyds -p 'fix'".into(),
            agent_result: crate::eval_fixtures::FixtureCommandResult {
                command: "yyds -p 'fix'".into(),
                passed: true,
                status_code: Some(0),
                duration_ms: 12,
                stdout_preview: String::new(),
                stderr_preview: String::new(),
            },
            mutation_scope_passed: false,
            changed_files: vec!["src/context.rs".into()],
            unexpected_changed_files: vec!["src/context.rs".into()],
            command_results: Vec::new(),
        };
        let mut eval = build_fixture_agent_attempt_eval_result(
            "local-smoke",
            "genome-v1",
            std::slice::from_ref(&result),
            20,
        );

        record_fixture_agent_attempt_failure_events_with(&mut eval, &[result], &recorder).unwrap();

        assert_eq!(eval.failure_event_ids.len(), 1);
        let events = read_events(&events_path).unwrap();
        let failure = events
            .iter()
            .find(|event| event["event_type"] == "FailureObserved")
            .unwrap();
        assert_eq!(failure["payload"]["source"], "eval_fixture_agent_attempt");
        assert_eq!(failure["payload"]["eval_id"], eval.eval_id);
        assert_eq!(failure["payload"]["task_id"], "context-miss");
        assert_eq!(failure["payload"]["agent_passed"], true);
        assert_eq!(failure["payload"]["mutation_scope_passed"], false);
        assert_eq!(failure["payload"]["changed_file_count"], 1);
        assert_eq!(failure["payload"]["changed_files"][0], "src/context.rs");
        assert_eq!(failure["payload"]["unexpected_changed_file_count"], 1);
        assert_eq!(
            failure["payload"]["unexpected_changed_files"][0],
            "src/context.rs"
        );
        assert_eq!(
            failure["payload"]["error_preview"],
            "fixture agent mutated unexpected files: src/context.rs"
        );
    }

    #[test]
    fn eval_state_metrics_collects_canonical_yoagent_state_events() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        for event in [
            crate::state::StateEvent {
                event_id: "evt-before".into(),
                event_type: EventType::ModelCallCompleted,
                schema_version: 1,
                timestamp_ms: 5,
                actor: Actor::Yoyo,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({"input_tokens": 999}),
            },
            crate::state::StateEvent {
                event_id: "evt-model".into(),
                event_type: EventType::ModelCallCompleted,
                schema_version: 1,
                timestamp_ms: 10,
                actor: Actor::Yoyo,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "input_tokens": 30,
                    "output_tokens": 7,
                    "cache_read_tokens": 3,
                    "cache_write_tokens": 2,
                    "cost_usd": 0.01,
                    "latency_ms": 50
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-cache".into(),
                event_type: EventType::CacheMetricsRecorded,
                schema_version: 1,
                timestamp_ms: 11,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "prompt_cache_hit_tokens": 70,
                    "prompt_cache_miss_tokens": 30
                }),
            },
            crate::state::StateEvent {
                event_id: "evt-tool".into(),
                event_type: EventType::ToolCallCompleted,
                schema_version: 1,
                timestamp_ms: 12,
                actor: Actor::Tool,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({"duration_ms": 20}),
            },
            crate::state::StateEvent {
                event_id: "evt-failure".into(),
                event_type: EventType::FailureObserved,
                schema_version: 1,
                timestamp_ms: 13,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({"source": "test"}),
            },
        ] {
            crate::state::append_event(&path, &event).unwrap();
        }

        let metrics = collect_eval_state_metrics_from(&path, 10, 20);

        assert_eq!(metrics["available"], true);
        assert_eq!(metrics["events"], 4);
        assert_eq!(metrics["model_calls"], 1);
        assert_eq!(metrics["tool_calls"], 1);
        assert_eq!(metrics["failures"], 1);
        assert_eq!(metrics["input_tokens"], 30);
        assert_eq!(metrics["output_tokens"], 7);
        assert_eq!(metrics["prompt_cache_hit_tokens"], 70);
        assert_eq!(metrics["prompt_cache_miss_tokens"], 30);
        assert_eq!(metrics["cache_hit_ratio"], 0.7);
        assert_eq!(metrics["latency_ms"], 70);
        assert_eq!(metrics["cost_usd"], 0.01);
    }

    #[test]
    fn preview_truncates_on_char_boundary() {
        let out = preview("éééé", 2);
        assert_eq!(out, "éé\n...[truncated]");
    }

    #[test]
    fn fixture_eval_result_scores_task_results() {
        let eval = build_fixture_eval_result(
            "local-smoke",
            "genome-v1",
            &[
                crate::eval_fixtures::FixtureTaskResult {
                    task_id: "task-1".into(),
                    passed: true,
                    command_results: Vec::new(),
                },
                crate::eval_fixtures::FixtureTaskResult {
                    task_id: "task-2".into(),
                    passed: false,
                    command_results: Vec::new(),
                },
            ],
            10,
        );
        assert_eq!(eval.suite, "fixtures:local-smoke");
        assert_eq!(eval.status, EvalStatus::Failed);
        assert_eq!(eval.score, Some(0.5));
        assert_eq!(eval.passed, 1);
        assert_eq!(eval.failed, 1);
    }
}
