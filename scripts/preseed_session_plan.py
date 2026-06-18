#!/usr/bin/env python3
"""Create a minimal evidence-backed task before the planning agent explores."""

from __future__ import annotations

import argparse
import re
from pathlib import Path


LIFECYCLE_TASK_TITLE = "Close yyds state and model lifecycle gaps"
SEARCH_FRICTION_TASK_TITLE = "Reduce recurring search-tool friction before implementation"
ACTIONABLE_LIFECYCLE_METRICS = (
    "state_run_incomplete_count",
    "state_run_unmatched_non_validation_completed_count",
    "deepseek_model_call_incomplete_count",
    "deepseek_model_call_unmatched_completed_count",
)
SEARCH_FRICTION_KEYS = (
    "search_regex_error",
    "search_binary_match",
    "search/grep error",
    "search/grep errors",
    "broken regex",
    "binary file matches",
)
PROTECTED_IMPLEMENTATION_FILES = (
    "scripts/evolve.sh",
    "scripts/format_issues.py",
    "scripts/build_site.py",
    "skills/self-assess/SKILL.md",
    "skills/evolve/SKILL.md",
)


TASKS = [
    {
        "keys": ("sub-agent", "sub agent", "api_key_present", "api key", "worker agents"),
        "reject_keys": (
            "ci/automation noise",
            "ci automation noise",
            "empty_input",
            "empty input",
            "automation noise",
            "expected for ci",
            "expected in ci",
        ),
        "title": "Verify and fix sub-agent API key propagation",
        "files": "src/tools.rs, src/agent_builder.rs, src/lib.rs",
        "objective": (
            "Make yyds either pass the resolved DeepSeek API key into spawned side/sub agents "
            "or record a precise diagnostic explaining why the key is unavailable."
        ),
        "why": (
            "The assessment found rapid RunStarted -> SessionStarted -> error traces with "
            "`api_key_present: false`. Those traces make autonomous planning brittle and hide "
            "whether DeepSeek worker agents are failing from missing credentials or another startup path."
        ),
        "success": [
            "Sub-agent construction resolves an explicit key or the provider-specific environment key before spawn.",
            "Missing-key failures produce a state diagnostic that names the failing sub-agent/startup path.",
            "Existing side-agent behavior remains unchanged when an explicit key is configured.",
        ],
        "verification": [
            "cargo test agent_builder tools state",
            "cargo check",
        ],
        "evidence": [
            "Task lineage links the changed source files to this task.",
            "Future state events distinguish missing credential diagnostics from generic startup errors.",
        ],
    },
    {
        "keys": (
            "cache metrics absent",
            "cache-report returning no metrics",
            "cache-report shows nothing",
            "cache report shows nothing",
            "no cache metrics",
            "missing cache metrics",
        ),
        "reject_keys": (
            "cache hit ratio",
            "deepseek_cache_hit_ratio",
            "cache metric event_count",
            "server-side cache hit ratio",
        ),
        "title": "Record DeepSeek prompt cache metrics during prompt runs",
        "files": "src/prompt.rs, src/deepseek.rs, src/state.rs",
        "objective": (
            "Ensure successful DeepSeek prompt executions record prompt cache hit/miss token usage "
            "into yoagent-state so `deepseek cache-report` and gnome KPIs have real data."
        ),
        "why": (
            "The assessment found `deepseek cache-report` returning no metrics even though the "
            "DeepSeek protocol layer and gnome keys exist. Cache observability is required to optimize "
            "stable-prefix prompt layout and cost/latency."
        ),
        "success": [
            "Prompt usage with cache hit/miss tokens emits CacheMetricsRecorded state events.",
            "`deepseek cache-report` can read those events after a DeepSeek run.",
            "No request-side `cache_control` is added for DeepSeek.",
        ],
        "verification": [
            "cargo test deepseek prompt state",
            "cargo check",
        ],
        "evidence": [
            "State summary includes DeepSeek cache hit/miss token gnomes after a run with usage data.",
            "Dashboard cache ratio remains sourced from numeric usage/state events, not prose.",
        ],
    },
    {
        "keys": ("why last-failure", "cold-start", "cold start", "no state event found"),
        "reject_keys": (
            "now properly explains cold-start",
            "cold-start diagnostics now",
            "cold-start diagnostics now inspect",
            "fixed cold-start",
            "replaced \"no state log found\"",
            "no sessions completed yet",
            "no failure found",
            "healthy state",
            "no failure found. the state system",
            "returned nothing — meaning no",
            "returned nothing - meaning no",
            "expected for fresh state",
            "expected for a freshly initialized",
            "clean output",
        ),
        "title": "Improve cold-start state failure diagnostics",
        "files": "src/commands_state.rs, src/state.rs",
        "objective": (
            "Make `yyds state why last-failure` useful when there are no completed failed sessions yet "
            "by reporting nearby startup errors, incomplete runs, or missing diagnostic evidence."
        ),
        "why": (
            "The assessment found `state why last-failure` returning only `no state event found` during "
            "fresh-state sessions. That leaves yyds unable to explain the earliest failures that block evolution."
        ),
        "success": [
            "Cold-start `why last-failure` output gives actionable next evidence to inspect.",
            "Existing behavior for completed failed sessions remains unchanged.",
            "Output distinguishes no history from missing diagnostics and from active/incomplete runs.",
        ],
        "verification": [
            "cargo test commands_state state",
            "cargo check",
        ],
        "evidence": [
            "Future assessment logs can cite concrete cold-start diagnostics instead of an empty result.",
            "State/dashboard blockers become easier to trace to run/session ids.",
        ],
    },
    {
        "keys": (
            "run_completion_guard",
            "test lifecycle error",
            "flaky test",
            "state::tests::run_completion_guard",
        ),
        "title": "Stabilize run completion guard panic test",
        "files": "src/state.rs",
        "objective": (
            "Make the `run_completion_guard_reports_error_on_panic` test deterministic and preserve "
            "the panic-path RunCompleted/FailureObserved behavior it verifies."
        ),
        "why": (
            "The assessment found the run-completion guard test failing once and passing on retry. "
            "A one-file flaky-test repair is more landable than broad lifecycle cleanup and directly "
            "protects the state evidence used by DeepSeek evolution."
        ),
        "success": [
            "The panic-path test no longer depends on timing, shared global state, or ambiguous event ordering.",
            "The production panic-path lifecycle events remain unchanged.",
            "The task touches only `src/state.rs` unless verification exposes a direct dependency.",
        ],
        "verification": [
            "cargo test --lib state::tests::run_completion_guard -- --exact",
            "cargo test --lib state::tests::run_completion_guard",
            "cargo check",
        ],
        "evidence": [
            "Future CI/log feedback stops repeating the `run_completion_guard` flaky failure.",
            "Task lineage links a strict one-file source change to the lifecycle reliability issue.",
        ],
    },
    {
        "keys": (
            "force analysis-only attempts into action",
            "force reverted tasks to leave concrete evidence",
            "task_analysis_only_attempt_count",
            "analysis-only task attempts",
            "analysis only task attempts",
            "task_no_edit_revert_count",
            "reverted_no_edit",
            "no-edit revert",
            "no edit revert",
            "implementation ended without file progress",
            "implementation task reverted without touching files",
            "tasks planned but reverted without touching",
            "reverted without touching any source file",
        ),
        "title": "Make analysis-only task pressure landable",
        "files": "scripts/preseed_session_plan.py, scripts/state_graph_tools.py, scripts/test_state_graph_tools.py",
        "objective": (
            "Ensure task-success pressure from analysis-only/no-edit attempts produces a small, "
            "landable follow-up task instead of selecting broad or protected-file harness work."
        ),
        "why": (
            "Recent evo evidence showed implementation attempts ending with no file progress, "
            "`reverted_no_edit`, and no terminal evidence. The next seed must target landable "
            "task-selection logic so DeepSeek can improve the loop without touching protected "
            "evolution files."
        ),
        "success": [
            "Graph-derived analysis-only/no-edit pressure selects a concrete seed before lifecycle cleanup.",
            "The selected seed Files list contains no protected implementation files.",
            "Preseed self-tests cover the analysis-only/no-edit pressure path.",
        ],
        "verification": [
            "python3 scripts/preseed_session_plan.py --test",
            "python3 -m unittest scripts.test_state_graph_tools",
        ],
        "evidence": [
            "Future task manifests show landable Files entries for task-success repair pressure.",
            "Future trajectory pressure leads with implementation failure repair when `task_analysis_only_attempt_count`, `reverted_no_edit`, or task_success_rate evidence shows no-edit task failure.",
        ],
    },
    {
        "keys": (
            "state_run_incomplete",
            "state_run_unmatched_non_validation_completed",
            "deepseek_model_call_incomplete",
            "deepseek_model_call_unmatched_completed",
            "model call lifecycle",
            "run lifecycle",
            "state lifecycle unhealthy",
            "runstarted",
            "runcompleted",
            "modelcallstarted",
            "modelcallcompleted",
        ),
        "reject_keys": (
            "input-validation exits without runstarted only",
            "input-validation-only unmatched",
            "pre-agent input-validation exit",
        ),
        "title": LIFECYCLE_TASK_TITLE,
        "files": (
            "scripts/append_terminal_state_events.py, scripts/log_feedback.py, scripts/summarize_state_gnomes.py"
        ),
        "objective": (
            "Close one concrete yyds lifecycle feedback gap by keeping terminal event recording and "
            "lifecycle lessons precise when current run/model-call imbalance is real."
        ),
        "why": (
            "The assessment found incomplete run/model-call lifecycle gnomes. Those signals affect "
            "state feedback, assessment trust, and future task selection more directly than dashboard display."
        ),
        "success": [
            "One verified lifecycle gap is fixed or downgraded with precise evidence in the listed files.",
            "Pre-agent input-validation exits stay classified separately from non-validation unmatched completions.",
            "Log feedback and state summaries emit lifecycle lessons only for real incomplete or non-validation unmatched paths.",
        ],
        "verification": [
            "python3 -m unittest scripts.test_append_terminal_state_events scripts.test_task_lineage_feedback",
            "bash -n scripts/evolve.sh",
        ],
        "evidence": [
            "Future structured state snapshots show lower `state_run_incomplete_count` and `deepseek_model_call_incomplete_count`.",
            "Lifecycle repair tasks are selected from current assessment evidence instead of stale dashboard-only symptoms.",
        ],
    },
    {
        "keys": (
            *SEARCH_FRICTION_KEYS,
        ),
        "title": SEARCH_FRICTION_TASK_TITLE,
        "files": "src/tools.rs, scripts/log_feedback.py, scripts/preseed_session_plan.py",
        "objective": (
            "Turn recurring search failure evidence into safer search behavior or sharper planning "
            "guidance, after first verifying which search safeguards already exist in the current code."
        ),
        "why": (
            "The assessment identified search regex and binary-match failures as top operational "
            "friction. Those failures waste implementation turns before DeepSeek reaches the actual code change."
        ),
        "success": [
            "The task verifies whether project search already defaults to literal matching before changing it.",
            "Remaining regex, empty-pattern, or binary-match search failures get a concrete code or prompt mitigation.",
            "Log-feedback lessons point future agents at the verified mitigation instead of stale generic advice.",
        ],
        "verification": [
            "cargo test tools",
            "python3 scripts/log_feedback.py --test",
            "cargo check",
        ],
        "evidence": [
            "A future assessment can cite fewer search_regex_error/search_binary_match failures or a more precise lesson.",
            "Task lineage links the mitigation to the source or harness prompt that changed behavior.",
        ],
    },
    {
        "keys": (
            "commands_state.rs still represents",
            "structural bottleneck",
            "state cli subsystem",
            "extract another focused state cli",
        ),
        "title": "Extract another focused state CLI module",
        "files": "src/commands_state.rs, src/commands_state_graph.rs",
        "objective": (
            "Reduce `commands_state.rs` by moving one to three tightly related graph report/payload helpers "
            "into the existing `commands_state_graph.rs` module without changing command behavior."
        ),
        "why": (
            "The assessment found `commands_state.rs` still represents roughly 16% of the Rust codebase. "
            "A micro-extraction into an existing module is small enough for a single DeepSeek task and avoids "
            "the broad extraction/revert pattern seen in previous runs."
        ),
        "success": [
            "One to three related graph helpers move from `commands_state.rs` into `commands_state_graph.rs`.",
            "The original helper definitions are removed from `commands_state.rs`, with call sites/imports updated.",
            "The public state command behavior and tests remain unchanged.",
            "The extraction touches no unrelated modules.",
        ],
        "verification": [
            "cargo test commands_state",
            "cargo check",
        ],
        "evidence": [
            "Task lineage shows `commands_state_graph.rs` and `commands_state.rs` as the changed source files.",
            "Dashboard work evidence lists the existing graph module as a source file.",
        ],
    },
]


def current_evidence_text(assessment: str) -> str:
    """Return sections that describe current symptoms, not history tables."""

    wanted = {
        "self-test results",
        "yoagent-state deepseek feedback",
        "structured state snapshot",
        "graph-derived next-task pressure",
        "capability gaps",
        "bugs / friction found",
        "open issues summary",
    }
    sections: list[str] = []
    current: list[str] = []
    include = False
    for line in assessment.splitlines():
        stripped = line.strip()
        if stripped.startswith("## "):
            if include and current:
                sections.append("\n".join(current))
            heading = stripped[3:].strip().lower()
            include = heading in wanted
            current = [line] if include else []
            continue
        if include:
            current.append(line)
    if include and current:
        sections.append("\n".join(current))
    return "\n\n".join(sections)


def extract_section(assessment: str, heading: str) -> str:
    """Extract a markdown section by its heading (case-insensitive)."""
    target = heading.strip().lower()
    lines: list[str] = []
    in_section = False
    for line in assessment.splitlines():
        stripped = line.strip()
        if stripped.startswith("## "):
            if in_section:
                break
            h = stripped[3:].strip().lower()
            if h == target:
                in_section = True
            continue
        if in_section:
            lines.append(line)
    return "\n".join(lines)


_RESOLUTION_SIGNALS = (
    "now properly",
    "now correctly",
    "already fixed",
    "already addressed",
    "already resolved",
    "no longer",
    "has been fixed",
    "has been resolved",
    "fixed ",
    "resolved ",
    "patched ",
    "shipped ",
    "landed ",
    "closed ",
    "addressed ",
)


def _line_shows_resolution(line: str, task_keys: tuple[str, ...]) -> bool:
    """Return True if line indicates a task-key problem is already resolved."""
    lower = line.lower()
    if not any(key in lower for key in task_keys):
        return False
    return any(signal in lower for signal in _RESOLUTION_SIGNALS)


def _self_tests_show_resolution(self_tests: str, task_keys: tuple[str, ...]) -> bool:
    """Check if self-test results show task-domain features already working."""
    for line in self_tests.splitlines():
        lower = line.strip().lower()
        if any(word in lower for word in ("flaky", "fail", "failed", "error", "retry")):
            continue
        if "\u2705" not in line and "pass" not in lower and "green" not in lower:
            continue
        if any(key in lower for key in task_keys):
            return True
    return False


def check_task_contradiction(
    task: dict[str, object], assessment: str
) -> tuple[bool, str]:
    """Return (contradicted, reason) if the assessment shows the task's problem is resolved.

    Scans the assessment's Recent Changes and Self-Test Results sections for
    evidence that the task's problem domain has already been addressed.
    """
    task_keys = tuple(task.get("keys", ()))
    if not task_keys:
        return False, ""

    recent_changes = extract_section(assessment, "recent changes")
    for line in recent_changes.splitlines():
        if _line_shows_resolution(line, task_keys):
            return True, f"assessment Recent Changes shows '{task['title']}' problem already resolved: {line.strip()}"

    self_tests = extract_section(assessment, "self-test results")
    if _self_tests_show_resolution(self_tests, task_keys):
        return True, f"assessment Self-Test Results show '{task['title']}' domain already working"

    return False, ""


def numeric_metrics(text: str) -> dict[str, int]:
    metrics: dict[str, int] = {}
    for match in re.finditer(r"`?([a-z][a-z0-9_]+)`?\s*[=:]\s*(-?\d+)", text):
        metrics[match.group(1)] = int(match.group(2))
    return metrics


def has_lifecycle_metrics(metrics: dict[str, int]) -> bool:
    return any(
        key.startswith("state_run_") or key.startswith("deepseek_model_call_")
        for key in metrics
    )


def has_actionable_lifecycle_gap(metrics: dict[str, int]) -> bool:
    return any(metrics.get(key, 0) > 0 for key in ACTIONABLE_LIFECYCLE_METRICS)


def has_actionable_search_friction(text: str) -> bool:
    """Return true only for current search friction, not cumulative snapshot counts."""

    for line in text.splitlines():
        lower = line.lower().strip()
        if not any(key in lower for key in SEARCH_FRICTION_KEYS):
            continue
        if "historical tool failures" in lower or "recent verified task" in lower:
            continue
        if lower.startswith("tool failures:") or lower.startswith("top tool-failure categories:"):
            continue
        if re.fullmatch(r"[-*]?\s*`?search_(?:regex_error|binary_match)`?\s*[=:]\s*\d+", lower):
            continue
        return True
    return False


def _has_protected_files(task: dict[str, object]) -> bool:
    """Return True if the task's files include any protected implementation file."""
    files_str = str(task.get("files") or "")
    task_files = {path.strip() for path in files_str.split(",") if path.strip()}
    return bool(task_files & set(PROTECTED_IMPLEMENTATION_FILES))


_ANALYSIS_ONLY_METRICS = (
    "task_analysis_only_attempt_count",
    "task_no_edit_revert_count",
)


def _has_analysis_only_pressure(metrics: dict[str, int]) -> bool:
    """Return True when analysis-only/no-edit pressure exists."""
    return any(metrics.get(key, 0) > 0 for key in _ANALYSIS_ONLY_METRICS)


def choose_task(assessment: str) -> dict[str, object]:
    current = current_evidence_text(assessment)
    lower = (current if current.strip() else assessment).lower()
    metrics = numeric_metrics(lower)
    lifecycle_metrics_present = has_lifecycle_metrics(metrics)
    analysis_only_active = _has_analysis_only_pressure(metrics)
    candidates: list[dict[str, object]] = []
    for task in TASKS:
        if not any(key in lower for key in task["keys"]):
            continue
        if any(key in lower for key in task.get("reject_keys", ())):
            continue
        if _has_protected_files(task):
            continue
        if task["title"] == LIFECYCLE_TASK_TITLE and lifecycle_metrics_present:
            if analysis_only_active:
                # Analysis-only pressure takes priority over lifecycle cleanup
                continue
            if not has_actionable_lifecycle_gap(metrics):
                continue
            candidates.append(task)
            continue
        if task["title"] == SEARCH_FRICTION_TASK_TITLE:
            if not has_actionable_search_friction(lower):
                continue
            candidates.append(task)
            continue
        candidates.append(task)

    for candidate in candidates:
        contradicted, reason = check_task_contradiction(candidate, assessment)
        if not contradicted:
            candidate["validated_against_assessment"] = True
            return candidate

    # All candidates are contradicted — return the first with annotation
    if candidates:
        contradicted, reason = check_task_contradiction(candidates[0], assessment)
        candidates[0]["validated_against_assessment"] = False
        candidates[0]["contradiction_reason"] = reason
        return candidates[0]

    return {
        "title": "Repair evidence-backed planning after no-task sessions",
        "files": "scripts/preseed_session_plan.py, scripts/task_manifest.py, scripts/test_task_manifest.py",
        "objective": (
            "Improve yyds fallback task selection and manifest validation so an evidence-rich assessment "
            "is reliably converted into concrete, landable task files."
        ),
        "why": (
            "The harness reached planning with no task artifacts. That makes evolution look healthy while "
            "skipping implementation, so planning reliability itself becomes the highest-priority repair."
        ),
        "success": [
            "Fallback planning repair tasks avoid protected implementation files.",
            "Task manifest warnings make no-task planning failures visible.",
            "Future planning failures preserve enough evidence to select a landable repair task.",
        ],
        "verification": [
            "python3 scripts/preseed_session_plan.py --test",
            "python3 -m unittest scripts.test_task_manifest",
            "python3 scripts/task_manifest.py --help",
        ],
        "evidence": [
            "Future task manifests show selected task artifacts with non-protected Files entries.",
            "planning_failed remains visible when it occurs.",
        ],
    }


def render_task(task: dict[str, object], day: str, session_time: str) -> str:
    success = "\n".join(f"- {item}" for item in task["success"])
    verification = "\n".join(f"- {item}" for item in task["verification"])
    evidence = "\n".join(f"- {item}" for item in task["evidence"])
    verifier = str(task["verification"][0])
    validated = task.get("validated_against_assessment", True)
    contradiction = task.get("contradiction_reason", "")
    validation_line = f"validated_against_assessment: {str(validated).lower()}"
    if not validated and contradiction:
        validation_line += f" (contradiction: {contradiction})"
    return f"""Title: {task["title"]}
Files: {task["files"]}
Issue: none
Origin: harness-seed
{validation_line}

Evidence:
- Current assessment matched this harness seed: {task["why"]}

Edit Surface:
- {task["files"]}

Verifier:
- {verifier}

Fallback:
- If current assessment, source, or recent changes show this failure class is already fixed or no longer live, write an obsolete-task note instead of editing.

Objective:
{task["objective"]}

Why this matters:
{task["why"]}

Success Criteria:
{success}

Verification:
{verification}

Expected Evidence:
{evidence}

Implementation Notes:
- This task was seeded by the harness before planner exploration because recent runs reached planning without durable task files.
- Treat it as a minimum viable task for Day {day} ({session_time}); refine it if the planner has stronger evidence, but do not leave the session with zero task files.
- Keep the change scoped to the listed files unless verification reveals a direct dependency.
"""


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assessment", type=Path)
    parser.add_argument("--output-dir", type=Path)
    parser.add_argument("--day", default="")
    parser.add_argument("--session-time", default="")
    parser.add_argument("--test", action="store_true")
    args = parser.parse_args()
    if args.test:
        assessment = "Bugs / Friction Found\nSub-agent spawn failures with api_key_present: false"
        task = choose_task(assessment)
        assert task["title"] == "Verify and fix sub-agent API key propagation", task
        assessment = """# Assessment

## Self-Test Results
- `yyds state crashes --json` shows 10 crashes, all `empty_input` with `api_key_present: false` (CI/automation noise)

## Structured State Snapshot
Top tool-failure categories:
- `search_regex_error` = 57
- `search_binary_match` = 19
"""
        task = choose_task(assessment)
        assert task["title"] == "Repair evidence-backed planning after no-task sessions", task
        assessment = """# Assessment

## Self-Test Results
`cargo test --lib state::tests::run_completion_guard` is FLAKY: `run_completion_guard_reports_error_on_panic` failed once with `test lifecycle error 107` and passed on retry.

## Structured State Snapshot
lifecycle gnomes: state_run_started_count=18; state_run_completed_count=18; state_run_incomplete_count=2; state_run_unmatched_completed_count=2; state_run_unmatched_non_validation_completed_count=0; state_run_unstarted_input_validation_error_count=2; deepseek_model_call_started_count=1; deepseek_model_call_completed_count=0; deepseek_model_call_incomplete_count=1
"""
        task = choose_task(assessment)
        assert task["title"] == "Stabilize run completion guard panic test", task
        assert task["files"] == "src/state.rs", task
        text = render_task(task, "107", "21:45")
        assert "run_completion_guard_reports_error_on_panic" in text, text

        assessment = """# Assessment

## Graph-derived Next-Task Pressure
- Force analysis-only attempts into action (task_analysis_only_attempt_count=2): Implementation ended without file progress or terminal evidence.
- Force reverted tasks to leave concrete evidence (task_no_edit_revert_count=1): Implementation task reverted without touching files.
- Close yyds state and model lifecycle gaps (state_run_incomplete_count=2): Lifecycle gnomes show unpaired terminal events.
"""
        task = choose_task(assessment)
        assert task["title"] == "Make analysis-only task pressure landable", task
        assert "scripts/evolve.sh" not in str(task["files"]), task
        text = render_task(task, "107", "21:55")
        assert "task_analysis_only_attempt_count" in text, text

        assessment = """# Assessment

## Recent Changes
**Day 109 (23:02) — 3/3 tasks verified:**
- Task 1: Cold-start diagnostics now inspect directory state before reporting "no events file"

## Graph-derived Next-Task Pressure
1. **Force reverted tasks to leave concrete evidence** (reverted_no_edit=1): Implementation tasks reverted without touching files; require early scoped edit, obsolete note, or concrete blocker
2. **Raise verified task success rate** (0.667): Dominant failure: reverted_no_edit. The fix should target task *planning* quality, not implementation robustness

## Bugs / Friction Found
MEDIUM — `reverted_no_edit` pattern (1 in last session): Tasks planned but reverted without touching any source file.
"""
        task = choose_task(assessment)
        assert task["title"] == "Make analysis-only task pressure landable", task
        assert task["title"] != "Improve cold-start state failure diagnostics", task
        text = render_task(task, "110", "18:26")
        assert "reverted_no_edit" in text, text

        assessment = """# Assessment

## Structured State Snapshot
lifecycle gnomes: state_run_started_count=18; state_run_completed_count=18; state_run_incomplete_count=2; state_run_unmatched_completed_count=2; state_run_unmatched_non_validation_completed_count=0; state_run_unstarted_input_validation_error_count=2; deepseek_model_call_started_count=1; deepseek_model_call_completed_count=0; deepseek_model_call_incomplete_count=1

## Bugs / Friction Found
State lifecycle unhealthy: runs incomplete 2; model calls incomplete 1.
Tool failures: search_regex_error=57; search_binary_match=19
"""
        task = choose_task(assessment)
        assert task["title"] == "Close yyds state and model lifecycle gaps", task
        assert len(str(task["files"]).split(",")) <= 3, task
        assessment = """# Assessment

## Structured State Snapshot
latest lifecycle gnomes: state_run_started_count=18; state_run_completed_count=19; state_run_incomplete_count=0; state_run_unmatched_completed_count=1; state_run_unmatched_non_validation_completed_count=1; state_run_unstarted_input_validation_error_count=0; deepseek_model_call_started_count=1; deepseek_model_call_completed_count=1; deepseek_model_call_incomplete_count=0

## Bugs / Friction Found
State lifecycle unhealthy: runs unmatched 1.
Tool failures: search_regex_error=57; search_binary_match=19
"""
        task = choose_task(assessment)
        assert task["title"] == "Close yyds state and model lifecycle gaps", task
        assessment = """# Assessment

## Graph-derived Next-Task Pressure
- Close yyds state and model lifecycle gaps (deepseek_model_call_incomplete_count=1): Lifecycle gnomes show unpaired terminal events.

## Recent Changes
No current issue in the old history table.
"""
        task = choose_task(assessment)
        assert task["title"] == "Close yyds state and model lifecycle gaps", task
        assessment = """# Assessment

## Structured State Snapshot
lifecycle gnomes: state_run_started_count=18; state_run_completed_count=20; state_run_incomplete_count=0; state_run_unmatched_completed_count=2; state_run_unmatched_non_validation_completed_count=0; state_run_unstarted_input_validation_error_count=2; deepseek_model_call_started_count=1; deepseek_model_call_completed_count=1; deepseek_model_call_incomplete_count=0

## Bugs / Friction Found
Only input-validation exits without RunStarted were found; pre-agent input-validation exit is classified.
Tool failures: search_regex_error=57; search_binary_match=19
"""
        task = choose_task(assessment)
        assert task["title"] == "Repair evidence-backed planning after no-task sessions", task
        assessment = "Cache metrics absent. deepseek cache-report shows nothing."
        task = choose_task(assessment)
        assert task["title"] == "Record DeepSeek prompt cache metrics during prompt runs", task
        assessment = "yyds deepseek cache-report: 94.10% cache hit ratio - healthy"
        task = choose_task(assessment)
        assert task["title"] != "Record DeepSeek prompt cache metrics during prompt runs", task
        assessment = (
            "yyds state why last-failure now properly explains cold-start state. "
            "`commands_state.rs` remains a structural bottleneck."
        )
        task = choose_task(assessment)
        assert task["title"] == "Extract another focused state CLI module", task
        text = render_task(task, "107", "21:30")
        assert "src/commands_state_graph.rs" in text, text
        assert "one to three tightly related graph" in text, text
        assert "original helper definitions are removed" in text, text
        assessment = (
            "State why last-failure: No failure found. The state system's "
            "`last-failure` target returned nothing — meaning no pipe-failures, "
            "transport errors, or crash events have been recorded. "
            "No clunky friction found in quick tool checks."
        )
        task = choose_task(assessment)
        assert task["title"] != "Improve cold-start state failure diagnostics", task
        assessment = """# Assessment

## Self-Test Results
- `./target/debug/yyds state why last-failure` - clean output: "no state event found for 'last-failure'" (expected for fresh state)

## Structured State Snapshot
Tool failures: search_regex_error=57; search_binary_match=19
"""
        task = choose_task(assessment)
        assert task["title"] == "Repair evidence-backed planning after no-task sessions", task
        assessment = """# Assessment

## Structured State Snapshot
historical tool failures: search_regex_error=57 (recent verified task: Add regex-error recovery hint to search tool err...); search_binary_match=19 (recent verified task: Extend search tool with binary-match recovery hi...)

## Bugs / Friction Found
No clunky friction found in quick tool checks.
"""
        task = choose_task(assessment)
        assert task["title"] == "Repair evidence-backed planning after no-task sessions", task
        assessment = """# Assessment

## Recent Changes
Day 105 added regex-error recovery hint to search tool errors.

## Source Architecture
| `commands_state.rs` | 23,548 | State CLI |

## Structured State Snapshot
Top tool-failure categories:
- `search_regex_error` = 57
- `search_binary_match` = 19

## Bugs / Friction Found
HIGH - `search_regex_error` (57 occurrences): the most frequent tool failure.
"""
        task = choose_task(assessment)
        assert task["title"] == "Reduce recurring search-tool friction before implementation", task
        assessment = """# Assessment

## Recent Changes
Day 105 added regex-error recovery hint to search tool errors.

## Source Architecture
| `commands_state.rs` | 23,548 | State CLI |

## Bugs / Friction Found
No clunky friction found in quick tool checks.
"""
        task = choose_task(assessment)
        assert task["title"] == "Repair evidence-backed planning after no-task sessions", task
        assert not _has_protected_files(task), task
        assert "skills/evolve" not in str(task["files"]), task
        assert "skills/self-assess" not in str(task["files"]), task
        text = render_task(task, "103", "12:53")
        assert "Title:" in text and "Success Criteria:" in text and "Origin: harness-seed" in text
        assert "Evidence:\n-" in text
        assert "Edit Surface:\n-" in text
        assert "Verifier:\n-" in text
        assert "Fallback:\n-" in text
        assessment = "Assessment phase produced a transcript but did not write session_plan/assessment.md."
        task = choose_task(assessment)
        assert task["title"] == "Repair evidence-backed planning after no-task sessions", task
        # --- Contradiction detection tests ---
        # Test 1: Task marked contradicted when Recent Changes show it was already fixed
        assessment = """# Assessment

## Recent Changes
Day 106 now properly resolved state lifecycle unhealthy problems: all
terminal events now pair correctly in every codepath we tested.

## Bugs / Friction Found
State lifecycle unhealthy: runs incomplete 2; model calls incomplete 1.
"""
        task = choose_task(assessment)
        assert task["title"] == LIFECYCLE_TASK_TITLE, (
            f"Expected lifecycle task, got {task['title']}"
        )
        assert task.get("validated_against_assessment") is False, (
            "Lifecycle task should be contradicted when Recent Changes say resolved"
        )
        assert task.get("contradiction_reason"), "Expected contradiction_reason"
        # Test 2: Validation metadata in rendered task
        text = render_task(task, "106", "10:00")
        assert "validated_against_assessment: false" in text
        assert "contradiction:" in text
        # Test 3: Self-test resolution — task NOT selected when checkmark shows it works
        assessment = """# Assessment

## Self-Test Results
- `yyds state why last-failure`: \u2705 now properly explains cold-start status
- `yyds deepseek cache-report`: \u26a0\ufe0f no DeepSeek cache metrics found

## Bugs / Friction Found
No clunky friction.
"""
        task = choose_task(assessment)
        assert task["title"] != "Improve cold-start state failure diagnostics", (
            f"Cold-start task should be contradicted by \u2705 self-test, got {task['title']}"
        )
        # Test 4: Cache task NOT selected when self-tests show cache working
        assessment = """# Assessment

## Self-Test Results
- `yyds deepseek cache-report`: \u2705 94.10% cache hit ratio -- healthy

## Bugs / Friction Found
Cache metrics absent. deepseek cache-report shows nothing.
"""
        task = choose_task(assessment)
        assert task["title"] != "Record DeepSeek prompt cache metrics during prompt runs", (
            f"Cache task should be contradicted by \u2705 self-test, got {task['title']}"
        )
        # Test 5: Normal case — task selected when assessment doesn't contradict
        assessment = """# Assessment

## Recent Changes
Day 105 added regex-error recovery hint to search tool errors.

## Source Architecture
| `commands_state.rs` | 23,548 | State CLI |

## Bugs / Friction Found
HIGH - `search_regex_error` (57 occurrences): the most frequent tool failure.
"""
        task = choose_task(assessment)
        assert task["title"] == "Reduce recurring search-tool friction before implementation", task
        assert task.get("validated_against_assessment") is True, (
            f"Expected validated_against_assessment=true, got {task.get('validated_against_assessment')}"
        )
        text = render_task(task, "105", "10:00")
        assert "validated_against_assessment: true" in text
        assert "contradiction:" not in text
        # Test 6: Sub-agent task contradicted by Recent Changes
        assessment = """# Assessment

## Recent Changes
Day 105 fixed sub-agent API key propagation: the agent now correctly passes
resolved API keys to spawned workers. `api_key_present` now reports true.
"""
        task = choose_task(assessment)
        assert task.get("validated_against_assessment") is False, (
            f"Sub-agent task should be contradicted by Recent Changes, got validated_against_assessment={task.get('validated_against_assessment')}"
        )
        assert task.get("contradiction_reason"), "Expected contradiction_reason"
        text = render_task(task, "105", "10:00")
        assert "validated_against_assessment: false" in text
        assert "contradiction:" in text
        for candidate in TASKS:
            protected = [
                path.strip()
                for path in str(candidate.get("files") or "").split(",")
                if path.strip() in PROTECTED_IMPLEMENTATION_FILES
            ]
            assert not protected, f"{candidate['title']} includes protected implementation files: {protected}"
        fallback = choose_task("No known current bug matched this assessment.")
        assert fallback["title"] == "Repair evidence-backed planning after no-task sessions", fallback
        assert not _has_protected_files(fallback), fallback
        print("preseed_session_plan self-tests passed")
        return 0
    if args.assessment is None:
        parser.error("--assessment is required unless --test is set")
    if args.output_dir is None:
        parser.error("--output-dir is required unless --test is set")

    if any(args.output_dir.glob("task_*.md")):
        return 0
    try:
        assessment = args.assessment.read_text(encoding="utf-8", errors="replace")
    except OSError:
        assessment = ""
    if not assessment.strip():
        return 0
    args.output_dir.mkdir(parents=True, exist_ok=True)
    task = choose_task(assessment)
    (args.output_dir / "task_01.md").write_text(
        render_task(task, args.day, args.session_time),
        encoding="utf-8",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
