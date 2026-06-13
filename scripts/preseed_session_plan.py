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
            "scripts/evolve.sh, scripts/append_terminal_state_events.py, "
            "scripts/log_feedback.py, scripts/summarize_state_gnomes.py"
        ),
        "objective": (
            "Ensure every yyds DeepSeek invocation and model-call path records paired terminal lifecycle "
            "events, and keep feedback lessons precise when lifecycle imbalance is real."
        ),
        "why": (
            "The assessment found incomplete run/model-call lifecycle gnomes. Those signals affect "
            "state feedback, assessment trust, and future task selection more directly than dashboard display."
        ),
        "success": [
            "Normal, timeout, error, and completion-file early-stop paths record terminal RunCompleted and ModelCallCompleted events.",
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
        "files": "src/tools.rs, scripts/log_feedback.py, scripts/evolve.sh",
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
        "files": "src/commands_state.rs, src/lib.rs",
        "objective": (
            "Reduce `commands_state.rs` by extracting one cohesive state CLI subsystem into a dedicated module "
            "without changing command behavior."
        ),
        "why": (
            "The assessment found `commands_state.rs` still represents roughly 16% of the Rust codebase. "
            "Continued small extractions lower maintenance risk for state/eval/evolution work."
        ),
        "success": [
            "One cohesive state CLI subsystem moves out of `commands_state.rs`.",
            "The public state command behavior and tests remain unchanged.",
            "The extraction touches no unrelated modules.",
        ],
        "verification": [
            "cargo test commands_state",
            "cargo check",
        ],
        "evidence": [
            "Task lineage shows the new module and `commands_state.rs` shrink.",
            "Dashboard work evidence lists the extracted module as a source file.",
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


def choose_task(assessment: str) -> dict[str, object]:
    current = current_evidence_text(assessment)
    lower = (current if current.strip() else assessment).lower()
    metrics = numeric_metrics(lower)
    lifecycle_metrics_present = has_lifecycle_metrics(metrics)
    for task in TASKS:
        if not any(key in lower for key in task["keys"]):
            continue
        if any(key in lower for key in task.get("reject_keys", ())):
            continue
        if task["title"] == LIFECYCLE_TASK_TITLE and lifecycle_metrics_present:
            if not has_actionable_lifecycle_gap(metrics):
                continue
            return task
        if task["title"] == SEARCH_FRICTION_TASK_TITLE:
            if not has_actionable_search_friction(lower):
                continue
            return task
        return task
    return {
        "title": "Repair evidence-backed planning after no-task sessions",
        "files": "skills/evolve/SKILL.md, skills/self-assess/SKILL.md, scripts/task_manifest.py",
        "objective": (
            "Improve yyds planning guidance and task manifest validation so an evidence-rich assessment "
            "is reliably converted into concrete task files."
        ),
        "why": (
            "The harness reached planning with no task artifacts. That makes evolution look healthy while "
            "skipping implementation, so planning reliability itself becomes the highest-priority repair."
        ),
        "success": [
            "The planning skill explicitly prioritizes writing task artifacts before extra exploration.",
            "Task manifest warnings make no-task planning failures visible.",
            "Future planning failures preserve enough evidence to select a repair task.",
        ],
        "verification": [
            "python3 -m unittest scripts.test_task_manifest",
            "python3 scripts/task_manifest.py --help",
        ],
        "evidence": [
            "Future dashboard sessions show selected task artifacts instead of an empty implementation phase.",
            "planning_failed remains visible when it occurs.",
        ],
    }


def render_task(task: dict[str, object], day: str, session_time: str) -> str:
    success = "\n".join(f"- {item}" for item in task["success"])
    verification = "\n".join(f"- {item}" for item in task["verification"])
    evidence = "\n".join(f"- {item}" for item in task["evidence"])
    return f"""Title: {task["title"]}
Files: {task["files"]}
Issue: none
Origin: harness-seed

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

## Structured State Snapshot
lifecycle gnomes: state_run_started_count=18; state_run_completed_count=18; state_run_incomplete_count=2; state_run_unmatched_completed_count=2; state_run_unmatched_non_validation_completed_count=0; state_run_unstarted_input_validation_error_count=2; deepseek_model_call_started_count=1; deepseek_model_call_completed_count=0; deepseek_model_call_incomplete_count=1

## Bugs / Friction Found
State lifecycle unhealthy: runs incomplete 2; model calls incomplete 1.
Tool failures: search_regex_error=57; search_binary_match=19
"""
        task = choose_task(assessment)
        assert task["title"] == "Close yyds state and model lifecycle gaps", task
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
        text = render_task(task, "103", "12:53")
        assert "Title:" in text and "Success Criteria:" in text and "Origin: harness-seed" in text
        assessment = "Assessment phase produced a transcript but did not write session_plan/assessment.md."
        task = choose_task(assessment)
        assert task["title"] == "Repair evidence-backed planning after no-task sessions", task
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
