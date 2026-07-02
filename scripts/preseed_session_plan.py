#!/usr/bin/env python3
"""Create a minimal evidence-backed task before the planning agent explores."""

from __future__ import annotations

import argparse
import glob
import os
import re
import subprocess
from pathlib import Path


LIFECYCLE_TASK_TITLE = "Close yyds state and model lifecycle gaps"
SEARCH_FRICTION_TASK_TITLE = "Reduce recurring search-tool friction before implementation"
ANALYSIS_ONLY_TASK_TITLE = "Make analysis-only task pressure landable"
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

_FIXTURE_DIR = "eval/fixtures/local-smoke"
_FIXTURE_FILE_RE = re.compile(
    r"""
    (?:fixture|eval)[\s#]*  # Context words: "fixture" or "eval"
    (\d{3,4})               # Fixture number (3-4 digits)
    (?!\d)                   # Not part of a longer number
    """,
    re.IGNORECASE | re.VERBOSE,
)
# Broader fallback: bare NNN-description pattern (e.g., "369-deepseek-prompt-layout")
_BARE_FIXTURE_NUM_RE = re.compile(
    r"""
    \b(\d{3,4})              # 3-4 digit number at word boundary
    -[\w-]{4,}                # followed by hyphen + at least 4 more word/hyphen chars
    """,
    re.VERBOSE,
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
        # Keep this seed tied to the actual broken output. Assessments mention
        # `state why last-failure` every run, so broad cold-start phrases would
        # keep reactivating this task after the command is healthy.
        "keys": ("no state event found",),
        "reject_keys": (
            "now properly explains cold-start",
            "cold-start diagnostics now",
            "cold-start diagnostics now inspect",
            "fixed cold-start",
            "replaced \"no state log found\"",
            "no completed failure sessions",
            "correctly reports",
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
            "tool-path recovery",
            "tool path recovery",
            "targeted_recovery_hint",
            "permission error recovery",
            "common tool failure",
            "tool failure pattern",
            "file-not-found recovery",
            "permission-denied recovery",
            "insufficient recovery hints",
            "missing recovery hint",
        ),
        "title": "Add recovery hints for common tool-path and permission errors",
        "files": "src/tool_wrappers.rs",
        "objective": (
            "Extend `targeted_recovery_hint` in `src/tool_wrappers.rs` to cover at least one "
            "additional common tool failure pattern (e.g., file-not-found, permission-denied, "
            "or invalid-path errors) with a targeted recovery hint that helps agents self-correct "
            "without manual intervention."
        ),
        "why": (
            "Agents are hitting tool failures without adequate recovery guidance. A small, "
            "concrete src/*.rs improvement is more landable than script-level seed repair and "
            "directly raises task success rate."
        ),
        "success": [
            "At least one new targeted recovery hint is added to `targeted_recovery_hint`.",
            "The new hint fires on a common tool-failure pattern seen in audit logs.",
            "Existing recovery hint behavior remains unchanged.",
            "The change touches only `src/tool_wrappers.rs`.",
        ],
        "verification": [
            "cargo test tool_wrappers",
            "cargo check",
        ],
        "evidence": [
            "Future tool error recovery rates improve for the covered pattern.",
            "Task lineage shows `src/tool_wrappers.rs` as the changed file.",
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
    "already in place",
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
    "made landable",
    "given enough standalone",
    "marked obsolete",
    "obsolete —",
    "obsolete -",
    "criteria already satisfied",
    "already satisfied",
    "reverted without",
    "reverted — no edit",
    "reverted_no_edit",
)


def _line_shows_resolution(line: str, task_keys: tuple[str, ...]) -> bool:
    """Return True if line indicates a task-key problem is already resolved."""
    lower = line.lower()
    if not any(key in lower for key in task_keys):
        return False
    # Session-date prefix (Day NNN) describing work already completed
    if re.match(r"day\s+\d+", lower):
        return True
    return any(signal in lower for signal in _RESOLUTION_SIGNALS)


_OBSOLETE_REVERTED_MARKERS = (
    "marked obsolete",
    "obsolete_already_satisfied",
    "reverted_no_edit",
    "reverted — no edit",
    "reverted — no",
    "criteria already satisfied",
    "reverted without",
)


def _line_shows_obsolete_or_reverted(
    line: str, task_keys: tuple[str, ...], task_title: str
) -> bool:
    """Return True if line indicates a task was marked obsolete or reverted without edits.

    This is a second-pass check for task-state evidence patterns that
    `_line_shows_resolution` can miss when task keys are metric names
    that don't appear verbatim in assessment prose (e.g. the
    analysis-only task whose keys are ``task_analysis_only_attempt_count``,
    ``reverted_no_edit``, etc., while the assessment writes "analysis-only
    pressure marked obsolete — criteria already satisfied").
    """
    lower = line.lower()
    if not any(m in lower for m in _OBSOLETE_REVERTED_MARKERS):
        return False
    # Match via task keys (substring)
    if any(key in lower for key in task_keys):
        return True
    # Match via significant words from the task title
    title_lower = task_title.lower()
    title_words = title_lower.split()
    for phrase_len in (3, 2, 1):
        for i in range(len(title_words) - phrase_len + 1):
            phrase = " ".join(title_words[i : i + phrase_len])
            if len(phrase) > 5 and phrase in lower:
                return True
    return False


def _self_tests_show_resolution(self_tests: str, task_keys: tuple[str, ...]) -> bool:
    """Check if self-test results show task-domain features already working."""
    for line in self_tests.splitlines():
        lower = line.strip().lower()
        if re.search(r'\b(?:flaky|fail|failed|error|retry)\b', lower):
            continue
        if "\u2705" not in line and "pass" not in lower and "green" not in lower:
            continue
        if any(key in lower for key in task_keys):
            return True
    return False


def _has_cold_start_failure_evidence(text: str) -> bool:
    """Return True only when current evidence shows the old cold-start failure."""
    lower = text.lower()
    if "state why last-failure" not in lower:
        return False
    if "no state event found" not in lower:
        return False
    healthy_phrases = (
        "expected for fresh state",
        "expected for a freshly initialized",
        "clean output",
        "no completed failure sessions",
        "correctly reports",
        "no failure found",
        "healthy state",
        "now properly explains cold-start",
    )
    return not any(phrase in lower for phrase in healthy_phrases)


def _fixture_paths_for_number(num: str) -> list[str]:
    """Return matching fixture paths for a fixture number."""
    pattern = os.path.join(_FIXTURE_DIR, f"{num}-*.json")
    return sorted(glob.glob(pattern))


def _check_fixture_already_exists(task: dict[str, object]) -> tuple[bool, str]:
    """Return (contradicted, reason) if the task references a fixture that already exists.

    Scans the task title and objective for fixture number references (e.g. ``#369``,
    ``fixture 369``, ``369-deepseek-prompt-layout-determinism``), resolves them
    to ``eval/fixtures/local-smoke/NNN-*.json`` paths, and checks whether the
    file already exists on disk.
    """
    title = str(task.get("title", ""))
    objective = str(task.get("objective", ""))
    combined = f"{title}\n{objective}"

    # Collect fixture numbers from both patterns
    fixture_nums: set[str] = set()
    for match in _FIXTURE_FILE_RE.finditer(combined):
        fixture_nums.add(match.group(1))
    for match in _BARE_FIXTURE_NUM_RE.finditer(combined):
        fixture_nums.add(match.group(1))

    for num in sorted(fixture_nums):
        paths = _fixture_paths_for_number(num)
        if paths:
            return True, (
                f"fixture already exists: {paths[0]} — "
                f"suppressing stale task '{task.get('title', '')}'"
            )

    return False, ""


def check_task_contradiction(
    task: dict[str, object], assessment: str
) -> tuple[bool, str]:
    """Return (contradicted, reason) if the assessment shows the task's problem is resolved.

    Scans the assessment's Recent Changes and Self-Test Results sections for
    evidence that the task's problem domain has already been addressed.
    Also checks whether fixture files referenced by the task already exist on disk.
    """
    # Check fixture file existence before scanning assessment text
    fixture_contradicted, fixture_reason = _check_fixture_already_exists(task)
    if fixture_contradicted:
        return True, fixture_reason

    task_keys = tuple(task.get("keys", ()))
    if not task_keys:
        return False, ""

    recent_changes = extract_section(assessment, "recent changes")
    for line in recent_changes.splitlines():
        if _line_shows_resolution(line, task_keys):
            return True, f"assessment Recent Changes shows '{task['title']}' problem already resolved: {line.strip()}"

    # Second pass: detect task-state evidence patterns (obsolete, reverted
    # without edits) even when task keys don't appear verbatim in the line.
    for line in recent_changes.splitlines():
        if _line_shows_obsolete_or_reverted(
            line, task_keys, str(task.get("title", ""))
        ):
            return True, f"assessment shows '{task['title']}' problem domain already obsolete/reverted: {line.strip()}"

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


_GIT_AVAILABLE: bool | None = None
_git_tracked_cache: set[str] | None = None


def _git_tracked_files() -> set[str] | None:
    """Return the set of git-tracked files (cached), or None if git unavailable."""
    global _GIT_AVAILABLE, _git_tracked_cache
    if _GIT_AVAILABLE is False:
        return None
    if _git_tracked_cache is not None:
        return _git_tracked_cache
    try:
        result = subprocess.run(
            ["git", "ls-files"],
            capture_output=True, text=True, timeout=10,
        )
        if result.returncode == 0:
            _GIT_AVAILABLE = True
            _git_tracked_cache = set(result.stdout.splitlines())
            return _git_tracked_cache
        _GIT_AVAILABLE = False
        return None
    except Exception:
        _GIT_AVAILABLE = False
        return None


def _candidate_files_exist(candidate: dict[str, object]) -> bool:
    """Return True if at least one file listed in the candidate's files field exists.

    When git is available, only git-tracked files are counted.
    When git is unavailable, falls back to plain file existence.
    """
    files_str = str(candidate.get("files") or "")
    tracked = _git_tracked_files()
    for path in files_str.split(","):
        path = path.strip()
        if not path:
            continue
        if not os.path.isfile(path):
            continue
        if tracked is not None and path not in tracked:
            continue
        return True
    return False


_ANALYSIS_ONLY_METRICS = (
    "task_analysis_only_attempt_count",
    "task_no_edit_revert_count",
    "reverted_no_edit",
)


def _has_analysis_only_pressure(metrics: dict[str, int]) -> bool:
    """Return True when analysis-only/no-edit pressure exists."""
    return any(metrics.get(key, 0) > 0 for key in _ANALYSIS_ONLY_METRICS)


def _analysis_only_seed_recently_blocked(text: str) -> bool:
    """Return True when the recurring analysis-only seed just failed as analysis-only.

    This prevents the preseed layer from reselecting the same self-referential
    task after the harness already captured a blocked/no-progress artifact for
    that exact title. The planner can still choose a stronger replacement task.
    """
    lower = text.lower()
    if ANALYSIS_ONLY_TASK_TITLE.lower() not in lower:
        return False
    blocked_markers = (
        "analysis_only_no_terminal_evidence",
        "task blocked by analysis-only implementation attempt",
        "task blocked by no-progress implementation attempts",
        "no file progress",
        "no-progress implementation",
        "no implementation landed",
        "obsolete",
        "criteria already satisfied",
        "marked obsolete",
    )
    return any(marker in lower for marker in blocked_markers)


def _task_file_count(task: dict[str, object]) -> int:
    """Return the number of files listed in a task's files field."""
    files_str = str(task.get("files") or "")
    return len([path for path in files_str.split(",") if path.strip()])


def _has_src_files(task: dict[str, object]) -> bool:
    """Return True if the task's files list contains at least one src/*.rs file."""
    files_str = str(task.get("files") or "")
    for path in files_str.split(","):
        path = path.strip()
        if path and path.startswith("src/") and path.endswith(".rs"):
            return True
    return False


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
        if (
            task["title"] == "Improve cold-start state failure diagnostics"
            and not _has_cold_start_failure_evidence(lower)
        ):
            continue
        if any(key in lower for key in task.get("reject_keys", ())):
            continue
        if task["title"] == ANALYSIS_ONLY_TASK_TITLE and (_analysis_only_seed_recently_blocked(lower) or analysis_only_active):
            continue
        if _has_protected_files(task):
            continue
        if not _candidate_files_exist(task):
            continue
        if analysis_only_active and _task_file_count(task) > 3:
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

    # When analysis-only/no-edit pressure is active, prefer candidates with
    # verifiable source files (src/*.rs) over script-only candidates, since
    # src edits produce cargo-testable commits and reduce reverted_no_edit risk.
    if analysis_only_active:
        # Stable sort: preserve relative order, src-file tasks first
        candidates.sort(key=lambda c: not _has_src_files(c))

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

    fallback = {
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
    if not _candidate_files_exist(fallback):
        # Guarantee at least one existing file
        fallback["files"] = "scripts/preseed_session_plan.py"

    if _assessment_is_healthy_codebase(lower, current):
        return _healthy_codebase_fallback()
    return fallback


_HEALTHY_CODEBASE_SIGNALS = (
    "no clunky friction found",
    "no clunky friction.",
    "no known current bug",
    "no current bugs",
    "no actionable bugs",
    "codebase is healthy",
    "codebase is stable",
    "harness is healthy",
    "no src/ bugs",
    "no source bugs",
    "no implementation bugs",
    "no friction found",
    "all checks passed",
    "nothing to fix",
    "no failures found",
    "no issues found",
    "clean bill of health",
)


_UNRESOLVED_BUG_RE = re.compile(
    # Non-zero tool-failure / error / lifecycle-incomplete metrics.
    # These contradict a health-signal claim of "no bugs" because they show
    # unresolved failures in the structured state snapshot.
    r'(?:search_regex_error|search_binary_match|_\w*error|incomplete_count)'
    r'\s*=\s*[1-9]\d*',
    re.IGNORECASE,
)


def _has_unresolved_bug_indicators(text: str) -> bool:
    """Return True if *text* contains explicit non-zero unresolved-bug metrics."""
    return bool(_UNRESOLVED_BUG_RE.search(text))


def _assessment_is_healthy_codebase(lower: str, current: str) -> bool:
    """Return True when the assessment describes a healthy codebase with no src/ bugs.

    Checks both the full assessment (lower) and the current-evidence subset
    for signals that no actionable bugs exist. Also requires that the assessment
    does NOT contain explicit unresolved bug indicators.
    """
    # Unresolved bug indicators contradict any health-signal language.
    if _has_unresolved_bug_indicators(lower):
        return False
    current_lower = current.lower()
    if _has_unresolved_bug_indicators(current_lower):
        return False

    # Check for explicit health signals
    for signal in _HEALTHY_CODEBASE_SIGNALS:
        if signal in lower:
            return True

    # Also check the current-evidence subset for health signals
    for signal in _HEALTHY_CODEBASE_SIGNALS:
        if signal in current_lower:
            return True

    return False


def _healthy_codebase_fallback() -> dict[str, object]:
    """Return a fallback task that records an honest stable-codebase observation.

    When the assessment shows the codebase is healthy with no src/ bugs,
    this produces a non-self-referential task that writes to journals/
    instead of modifying the planning pipeline.
    """
    task: dict[str, object] = {
        "title": "Record honest stable-codebase journal entry",
        "files": "journals/JOURNAL.md",
        "objective": (
            "Write a journal entry recording that the assessment found the codebase healthy "
            "with no actionable src/ bugs. This is a valid outcome, not a planning failure."
        ),
        "why": (
            "The assessment found no actionable bugs in src/. Recording this honestly preserves "
            "trajectory evidence without fabricating a self-referential pipeline-fix task that "
            "cycles without ever passing strict verification (no cargo build && cargo test)."
        ),
        "success": [
            "Journal entry records the stable-codebase observation honestly.",
            "No source code changes are made (none needed).",
            "The task produces terminal evidence without modifying planning scripts.",
        ],
        "verification": [
            "python3 scripts/preseed_session_plan.py --test",
        ],
        "evidence": [
            "Future trajectory shows honest stable-codebase cycles alongside real src/ improvements.",
            "Strict verification passes because the task doesn't require cargo build && cargo test.",
            "No self-referential pipeline-fix tasks are produced for healthy codebase assessments.",
        ],
    }
    if not _candidate_files_exist(task):
        task["files"] = "journals/JOURNAL.md"
    return task


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
        assert task["title"] != ANALYSIS_ONLY_TASK_TITLE, (
            f"analysis_only_active should skip ANALYSIS_ONLY_TASK_TITLE, got {task['title']}"
        )
        # Task should be a landable task with source files, not analysis-only
        assert "scripts/" in str(task["files"]), (
            f"Expected landable task with script files, got {task.get('files')}"
        )
        text = render_task(task, "107", "21:55")
        # Rendered output should contain the task title
        assert task["title"] in text, f"Rendered text missing task title: {task['title']}"

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
        assert task["title"] != ANALYSIS_ONLY_TASK_TITLE, (
            f"analysis_only_active should skip analysis-only task, got {task['title']}"
        )
        assert task["title"] != "Improve cold-start state failure diagnostics", task
        # Should select a landable task (not analysis-only)
        assert task.get("files"), f"Expected task with files, got {task}"


        # --- Analysis-only pressure: reverted_no_edit now skips analysis-only task ---
        # reverted_no_edit alone (no lifecycle metrics) → landable task, not analysis-only
        assessment = """# Assessment

## Graph-derived Next-Task Pressure
- **Task-state counts**: reverted_no_edit=4 in recent window. These are sessions where tasks were assigned but produced no file changes.
"""
        task = choose_task(assessment)
        assert task["title"] != ANALYSIS_ONLY_TASK_TITLE, (
            f"reverted_no_edit=4 with analysis_only_active should skip analysis-only task, got {task['title']}"
        )
        assert task.get("files"), f"Expected landable task, got {task}"

        # --- Analysis-only pressure + lifecycle metrics: landable task wins ---
        # When analysis-only pressure is active, lifecycle task is also skipped (line 752-754)
        # and a landable non-analysis-only task is selected instead.
        assessment = """# Assessment

## Graph-derived Next-Task Pressure
- Force analysis-only attempts into action (reverted_no_edit=4): Sessions with no file progress.
- Close yyds state and model lifecycle gaps (state_run_incomplete_count=2): Lifecycle gnomes show unpaired terminal events.
"""
        task = choose_task(assessment)
        assert task["title"] != ANALYSIS_ONLY_TASK_TITLE, (
            f"analysis_only_active should skip analysis-only task, got {task['title']}"
        )
        assert task["title"] != LIFECYCLE_TASK_TITLE, (
            f"analysis_only_active should also skip lifecycle task, got {task['title']}"
        )
        assert task.get("files"), f"Expected landable task, got {task}"

        # --- Analysis-only pressure: file-count guard ---
        # When analysis-only pressure is active, tasks with >3 files are skipped
        # (line 749). The selected landable task should have ≤3 files.
        # All current non-analysis-only TASKS have ≤3 files, so this is future-proofing.
        assessment = """# Assessment

## Graph-derived Next-Task Pressure
- Force analysis-only attempts into action (task_analysis_only_attempt_count=3): Implementation ended without file progress.
"""
        task = choose_task(assessment)
        assert task["title"] != ANALYSIS_ONLY_TASK_TITLE, (
            f"analysis_only_active should skip analysis-only task, got {task['title']}"
        )
        files_count = _task_file_count(task)
        assert files_count <= 3, (
            f"Analysis-only pressure should select task with ≤3 files, got {files_count}: {task['files']}"
        )
        assert not _has_protected_files(task), (
            f"Analysis-only pressure should skip protected files, got: {task['files']}"
        )

        # --- Evidence-aware re-ranking: prefer src-file candidates when analysis-only pressure active ---
        # Assessment with reverted_no_edit + api_key_present: sub-agent task (src/*.rs) should win
        # over analysis-only task (scripts/*.py) because src files produce cargo-testable commits.
        assessment = """# Assessment

## Graph-derived Next-Task Pressure
- **Task-state counts**: reverted_no_edit=2 in recent window. Sessions with no file progress.
- **Sub-agent spawn failures**: api_key_present validation.

## Bugs / Friction Found
- Sub-agent calls failing due to missing API keys.
"""
        task = choose_task(assessment)
        assert task["title"] == "Verify and fix sub-agent API key propagation", (
            f"With reverted_no_edit pressure, src-file candidate should win over scripts-only, got {task['title']}"
        )
        assert "src/" in str(task["files"]), (
            f"Selected task should have src/*.rs files, got {task['files']}"
        )
        assert not _has_protected_files(task), (
            f"Evidence-aware selection should skip protected files, got: {task['files']}"
        )

        # --- Evidence-aware re-ranking: picks landable task when analysis-only active ---
        # When reverted_no_edit pressure exists, analysis_only_active=True skips the
        # analysis-only task and selects the next-best landable candidate.
        assessment = """# Assessment

## Graph-derived Next-Task Pressure
- Force analysis-only attempts into action (reverted_no_edit=3): Sessions with no file progress.
"""
        task = choose_task(assessment)
        assert task["title"] != ANALYSIS_ONLY_TASK_TITLE, (
            f"analysis_only_active should skip analysis-only task, got {task['title']}"
        )
        assert task.get("files"), f"Expected landable task with files, got {task}"

        # --- Analysis-only pressure: task_no_edit_revert_count alone triggers analysis-only task ---
        assessment = """# Assessment

## Graph-derived Next-Task Pressure
- **Task-state counts**: task_no_edit_revert_count=3 in recent window. Tasks reverted without touching source files.
"""
        task = choose_task(assessment)
        assert task["title"] != ANALYSIS_ONLY_TASK_TITLE, (
            f"task_no_edit_revert_count alone should NOT select analysis-only task (analysis_only_active blocks), got {task['title']}"
        )
        assert not _has_protected_files(task), (
            f"Analysis-only pressure from task_no_edit_revert_count should skip protected files, got: {task['files']}"
        )
        assert _task_file_count(task) <= 3, (
            f"Analysis-only pressure should select task with ≤3 files, got {_task_file_count(task)}"
        )
        assessment = """# Assessment

## Recent session outcomes
day-115: tasks 2/3; task states: analysis_only_no_terminal_evidence=1

## Graph-derived Next-Task Pressure
- Raise verified task success rate (task_success_rate=0.666): selected tasks did not all finish.

## Bugs / Friction Found
- Task 1 "Make analysis-only task pressure landable" was blocked by analysis-only implementation attempt with no file progress and no implementation landed.
"""
        task = choose_task(assessment)
        assert task["title"] != ANALYSIS_ONLY_TASK_TITLE, (
            f"recently blocked analysis-only seed should not be selected again, got {task['title']}"
        )

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

## Self-Test Results
- `yyds state why last-failure`: correctly reports "No completed failure sessions" + 1 incomplete run

## Structured State Snapshot
state_run_incomplete_count=1
"""
        task = choose_task(assessment)
        assert task["title"] != "Improve cold-start state failure diagnostics", (
            f"healthy last-failure output should not reactivate cold-start seed, got {task['title']}"
        )
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
        assert task["title"] == "Record honest stable-codebase journal entry", task
        assert not _has_protected_files(task), task
        assert "scripts/preseed_session_plan.py" not in str(task["files"]), (
            "Healthy fallback must not self-reference planning scripts"
        )
        text = render_task(task, "103", "12:53")
        assert "Title:" in text and "Success Criteria:" in text and "Origin: harness-seed" in text
        assert "Evidence:\n-" in text
        assert "journals/JOURNAL.md" in str(task["files"]), (
            "Healthy fallback should target journals/ not planning scripts"
        )
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
        # Test 7: Stale seed — analysis-only task contradicted when assessment
        # says it was "made landable" or "given enough standalone weight"
        assessment = """# Assessment

## Recent Changes
**Analysis-only task pressure made landable** (`scripts/preseed_session_plan.py`): The `task_no_edit_revert_count` metric was given enough standalone weight to trigger recovery tasks by itself without requiring task_analysis_only_attempt_count > 0. The seed now prefers single-file src/*.rs candidates.

## Bugs / Friction Found
no-edit revert pressure from prior session.
"""
        # The analysis-only task keys ('no-edit revert') match the Bugs section,
        # but Recent Changes says the fix was "made landable" — so it should
        # be selected but contradicted (validated_against_assessment=False)
        task = choose_task(assessment)
        assert task["title"] == "Make analysis-only task pressure landable", (
            f"Expected analysis-only task, got {task['title']}"
        )
        assert task.get("validated_against_assessment") is False, (
            f"Analysis-only task should be contradicted when made landable, "
            f"got validated_against_assessment={task.get('validated_against_assessment')}"
        )
        assert task.get("contradiction_reason"), "Expected contradiction_reason for stale seed"
        assert "made landable" in str(task.get("contradiction_reason", "")).lower() or \
            "given enough standalone" in str(task.get("contradiction_reason", "")).lower(), (
            f"contradiction_reason should mention 'made landable' or 'given enough standalone', "
            f"got: {task.get('contradiction_reason')}"
        )
        # Test 8: Session-date prefix (Day NNN) alone triggers resolution when
        # task keys are present, even without explicit resolution verbs
        assessment = """# Assessment

## Recent Changes
Day 114 adjusted the `task_no_edit_revert_count` weighting to ensure analysis-only
tasks never select files that would risk protected-file reverts.

## Bugs / Friction Found
no-edit revert pressure from prior session.
"""
        task = choose_task(assessment)
        assert task["title"] == "Make analysis-only task pressure landable", (
            f"Expected analysis-only task, got {task['title']}"
        )
        assert task.get("validated_against_assessment") is False, (
            f"Day-prefixed line with task key should be contradicted, "
            f"got validated_against_assessment={task.get('validated_against_assessment')}"
        )
        assert task.get("contradiction_reason"), "Expected contradiction_reason for Day-prefixed resolution"
        # Test 9: Obsolete-seed contradiction — assessment marks task as
        # "marked obsolete — criteria already satisfied" without using task
        # keys verbatim (regression for Day 118 preseed re-seeding).
        assessment = """# Assessment

## Recent Changes
Day 118 (03:50) | 3 tasks | 2/3 verified. Task 1 (analysis-only pressure) marked obsolete — criteria already satisfied.

## Bugs / Friction Found
no-edit revert pressure from prior session.
"""
        task = choose_task(assessment)
        assert task["title"] == "Make analysis-only task pressure landable", (
            f"Expected analysis-only task, got {task['title']}"
        )
        assert task.get("validated_against_assessment") is False, (
            f"Obsolete-seed task should be contradicted, "
            f"got validated_against_assessment={task.get('validated_against_assessment')}"
        )
        assert task.get("contradiction_reason"), (
            "Expected contradiction_reason for obsolete-seed detection"
        )
        reason = str(task.get("contradiction_reason", "")).lower()
        assert "obsolete" in reason or "reverted" in reason, (
            f"contradiction_reason should mention 'obsolete' or 'reverted', "
            f"got: {task.get('contradiction_reason')}"
        )
        for candidate in TASKS:
            protected = [
                path.strip()
                for path in str(candidate.get("files") or "").split(",")
                if path.strip() in PROTECTED_IMPLEMENTATION_FILES
            ]
            assert not protected, f"{candidate['title']} includes protected implementation files: {protected}"
        fallback = choose_task("No known current bug matched this assessment.")
        assert fallback["title"] == "Record honest stable-codebase journal entry", fallback
        assert not _has_protected_files(fallback), fallback
        assert "scripts/preseed_session_plan.py" not in str(fallback["files"]), (
            "Healthy fallback must not self-reference planning scripts"
        )
        # --- File-existence validation tests ---
        # All TASKS candidates must have at least one existing file
        for candidate in TASKS:
            assert _candidate_files_exist(candidate), (
                f"Candidate '{candidate['title']}' has no existing files: {candidate.get('files')}"
            )
        # Helper: candidate with all-existing files returns True
        assert _candidate_files_exist({"files": "scripts/preseed_session_plan.py"})
        # Helper: candidate with some-existing files returns True
        assert _candidate_files_exist(
            {"files": "scripts/preseed_session_plan.py, nonexistent/file/ghost.py"}
        )
        # Helper: candidate with all-missing files returns False
        assert not _candidate_files_exist(
            {"files": "nonexistent/file/ghost.py, another/missing/path.rs"}
        )
        # Helper: candidate with empty files returns False
        assert not _candidate_files_exist({"files": ""})
        assert not _candidate_files_exist({})
        # Fallback always has at least one existing file
        assert _candidate_files_exist(fallback), "Fallback task files don't exist"
        # Analysis-only task passes file-existence check
        for candidate in TASKS:
            if candidate["title"] == "Make analysis-only task pressure landable":
                assert _candidate_files_exist(candidate), (
                    f"Analysis-only task files don't exist: {candidate['files']}"
                )
                break
        # --- Git-tracked file validation tests ---
        # gitignored file that exists on disk but is NOT tracked => False
        assert os.path.isfile("target/CACHEDIR.TAG"), (
            "Test precond: target/CACHEDIR.TAG must exist")
        assert not _candidate_files_exist(
            {"files": "target/CACHEDIR.TAG"}), (
            "gitignored file should be rejected")
        # Mix: gitignored (exists) + tracked (exists) => True
        assert _candidate_files_exist(
            {"files": "target/CACHEDIR.TAG, scripts/preseed_session_plan.py"}), (
            "mixed tracked+ignored: at least one tracked file should pass")
        # All gitignored => False
        assert not _candidate_files_exist(
            {"files": "target/CACHEDIR.TAG, Cargo.lock"}), (
            "all-gitignored files should be rejected")
        # --- Analysis-only pressure escape-hatch test ---
        # When task_analysis_only_attempt_count > 0 appears in evidence,
        # analysis_only_active is True and the picker must skip
        # ANALYSIS_ONLY_TASK_TITLE in favor of a landable task.
        assessment = """# Assessment

## Graph-derived Next-Task Pressure
- Force analysis-only attempts into action (task_analysis_only_attempt_count=1): Implementation ended without file progress or terminal evidence.
"""
        task = choose_task(assessment)
        assert task["title"] != ANALYSIS_ONLY_TASK_TITLE, (
            f"analysis_only_active should skip analysis-only task, got {task['title']}"
        )
        # The selected task should have at least one editable source file
        # (not just scripts/evolve.sh or similar protected paths)
        assert task.get("files"), (
            f"Expected task with files, got {task}"
        )
        # --- Fixture existence contradiction tests ---
        # Test: a task referencing an existing fixture (e.g., #369) is contradicted
        fixture_task = {
            "title": "Add held-out coding eval fixture for DeepSeek prompt layout determinism",
            "keys": ("deepseek-prompt-layout",),
            "objective": "Create eval fixture #369 for DeepSeek prompt layout determinism.",
            "files": "eval/fixtures/local-smoke/369-deepseek-prompt-layout-determinism.json",
        }
        assert os.path.isfile(
            "eval/fixtures/local-smoke/369-deepseek-prompt-layout-determinism.json"
        ), "Test precond: fixture 369 must exist on disk"
        contradicted, reason = check_task_contradiction(fixture_task, "dummy assessment")
        assert contradicted, (
            f"Task referencing existing fixture #369 should be contradicted, "
            f"got contradicted={contradicted}, reason={reason}"
        )
        assert "fixture already exists" in reason.lower(), (
            f"Contradiction reason should mention 'fixture already exists', "
            f"got: {reason}"
        )
        assert "369" in reason, (
            f"Contradiction reason should mention fixture number 369, got: {reason}"
        )
        # Test: a fixture task with a non-existent fixture number is not contradicted
        nonexistent_task = {
            "title": "Add eval fixture #99999 for some-future-feature",
            "keys": ("some-future-feature",),
            "objective": "Create fixture 99999-some-future-feature.",
            "files": "eval/fixtures/local-smoke/99999-some-future-feature.json",
        }
        contradicted2, reason2 = check_task_contradiction(nonexistent_task, "dummy assessment")
        assert not contradicted2, (
            f"Task referencing nonexistent fixture #99999 should NOT be contradicted, "
            f"got contradicted={contradicted2}, reason={reason2}"
        )
        # Test: bare NNN-description pattern (e.g., "369-deepseek-prompt-layout")
        # in title triggers fixture detection even without "fixture" prefix
        bare_task = {
            "title": "369-deepseek-prompt-layout-determinism is missing coverage",
            "keys": ("coverage",),
            "objective": "Add tests for 369-deepseek-prompt-layout.",
            "files": "src/placeholder.rs",
        }
        contradicted3, reason3 = check_task_contradiction(bare_task, "dummy assessment")
        assert contradicted3, (
            f"Task with bare 369-description pattern should be contradicted, "
            f"got contradicted={contradicted3}, reason={reason3}"
        )
        assert "fixture already exists" in reason3.lower(), (
            f"Contradiction reason should mention 'fixture already exists', "
            f"got: {reason3}"
        )
        # Test: a task without fixture references is not contradicted by this check
        normal_task = {
            "title": "Fix a completely unrelated bug",
            "keys": ("unrelated-bug",),
            "objective": "This has nothing to do with fixtures.",
            "files": "src/main.rs",
        }
        contradicted4, reason4 = check_task_contradiction(normal_task, "dummy assessment")
        assert not contradicted4, (
            f"Task without fixture references should NOT be contradicted, "
            f"got contradicted={contradicted4}, reason={reason4}"
        )
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
