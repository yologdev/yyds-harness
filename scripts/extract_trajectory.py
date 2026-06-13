#!/usr/bin/env python3
"""
extract_trajectory.py — Build the YOUR TRAJECTORY block injected into Phase A1
(assess) and Phase A2 (plan) prompts. Aggregates audit-log session evidence,
git log, and gh run history into a structured markdown summary so yoyo sees
ground truth about its own recent trajectory before deciding what to work on.

Inputs (env vars):
  YOYO_AUDIT_DIR       Path to audit-log worktree's `sessions/` directory.
  YOYO_REPO            owner/repo slug for `gh` calls (e.g. "yologdev/yoyo-evolve").
  YOYO_DAY             Current day number (used only for window calc + display).
  YOYO_TRAJECTORY_OUT  Output file path. Default: .yoyo/session_staging/trajectory.md.

Output:
  Writes a single markdown blob to YOYO_TRAJECTORY_OUT. ~1-2KB target, hard-capped
  at 100 lines / 2KB. Always exits 0; failure modes degrade per-section and write
  "(no trajectory data yet)" if no signal could be gathered.
"""
import json
import os
import re
import subprocess
import sys
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Optional, Union

from state_graph_tools import evolution_suggestions, ordered_sessions

# ── Configuration constants ──────────────────────────────────────────────
WINDOW_SESSIONS = 10           # last N sessions in the outcomes section
OUTCOME_DISPLAY_SESSIONS = 6   # newest session rows shown before compact omission note
WINDOW_DAYS = 14               # git log window
MAX_FAILED_RUNS = 5            # cap on `gh run view --log-failed` calls
GH_RUN_VIEW_TIMEOUT = 10       # seconds per gh run view
GH_RUN_LIST_TIMEOUT = 10       # seconds for gh run list
STUCK_ON_THRESHOLD = 3         # ≥N attempts AND 0 successes → flag
TOTAL_LINE_CAP = 100
TOTAL_BYTE_CAP = 3072
TOOL_FAILURE_RECENT_TASK_PATTERNS: dict[str, tuple[str, ...]] = {
    "search_regex_error": (
        "regex-error",
        "regex error",
        "unmatched",
    ),
    "search_binary_match": (
        "binary-match",
        "binary match",
        "binary file matches",
    ),
}

# ── Helpers ──────────────────────────────────────────────────────────────


def warn(msg: str) -> None:
    print(f"extract_trajectory: WARN: {msg}", file=sys.stderr)


def run_cmd(cmd: list[str], timeout: int = 10) -> tuple[int, str, str]:
    """Run a command, capture output. Returns (rc, stdout, stderr). Never raises.
    Uses start_new_session=True so a TimeoutExpired SIGKILLs the entire process
    group (including grandchildren like git/curl spawned by gh), not just the
    immediate child — prevents zombie buildup over many sessions."""
    try:
        r = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=timeout,
            start_new_session=True,
        )
        return r.returncode, r.stdout, r.stderr
    except subprocess.TimeoutExpired as e:
        warn(f"timed out after {timeout}s: {' '.join(cmd[:3])}...")
        # Best-effort kill of the whole process group; subprocess.run already
        # killed the immediate child but grandchildren may persist.
        try:
            if e.pid is not None:
                os.killpg(os.getpgid(e.pid), 9)  # SIGKILL
        except (ProcessLookupError, PermissionError, OSError):
            pass
        return 124, "", "timeout"
    except (FileNotFoundError, OSError) as e:
        warn(f"command failed: {' '.join(cmd[:3])}... — {e}")
        return 1, "", str(e)


def strip_ansi(s: str) -> str:
    return re.sub(r"\x1b\[[0-9;]*[a-zA-Z]", "", s)


def truncate_lines(s: str, n: int) -> str:
    lines = s.splitlines()
    if len(lines) <= n:
        return s
    return "\n".join(lines[:n] + [f"... ({len(lines) - n} more lines truncated)"])


def truncate_text(value: Any, limit: int) -> str:
    text = str(value or "")
    if len(text) <= limit:
        return text
    return text[: max(0, limit - 3)].rstrip() + "..."


def drop_dangling_trailing_section_header(s: str) -> str:
    lines = s.splitlines()
    last_header = None
    for index, line in enumerate(lines):
        if line.startswith("## "):
            last_header = index
    if last_header is None:
        return s
    if any(line.strip() for line in lines[last_header + 1:]):
        return s
    return "\n".join(lines[:last_header]).rstrip()


# ── Section 1: Recent session outcomes ───────────────────────────────────


def outcome_sort_time(data: dict[str, Any], fallback: float) -> float:
    ts = str(data.get("ts") or "").strip()
    if ts:
        normalized = ts.replace("Z", "+00:00")
        try:
            return datetime.fromisoformat(normalized).timestamp()
        except ValueError:
            pass
    return fallback


def load_recent_session_outcomes(audit_dir: Path) -> list[tuple[Path, dict[str, Any]]]:
    """Read last N outcome.json files, sorted newest-first by outcome timestamp.
    Returns dicts unchanged from outcome.json — sort metadata is kept on a
    side tuple, never mutated into the parsed object (defends against keys
    like `_mtime` colliding with future schema additions). File mtime is only
    a fallback because copied audit artifacts can have non-chronological mtimes."""
    if not audit_dir.exists() or not audit_dir.is_dir():
        return []
    triples: list[tuple[float, str, dict]] = []
    for child in audit_dir.iterdir():
        if not child.is_dir():
            continue
        outcome = child / "outcome.json"
        if not outcome.is_file():
            continue
        try:
            data = json.loads(outcome.read_text(errors="replace"))
        except (OSError, json.JSONDecodeError, UnicodeDecodeError) as e:
            warn(f"skipped malformed {outcome}: {e}")
            continue
        try:
            mtime = outcome.stat().st_mtime
        except OSError as e:
            warn(f"could not stat {outcome}: {e}")
            mtime = 0.0
        triples.append((outcome_sort_time(data, mtime), child.name, data))
    triples.sort(key=lambda t: t[0], reverse=True)
    return [(audit_dir / t[1], t[2]) for t in triples[:WINDOW_SESSIONS]]


def load_outcomes(audit_dir: Path) -> list[dict[str, Any]]:
    # Backward-compatible helper for callers/tests that only need raw outcomes.
    return [outcome for _, outcome in load_recent_session_outcomes(audit_dir)]


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8", errors="replace"))
    except (OSError, json.JSONDecodeError, UnicodeDecodeError):
        return None


def session_sort_time(session_dir: Path) -> float:
    fallback = 0.0
    for name in ("outcome.json", "log_feedback.json"):
        path = session_dir / name
        if not path.is_file():
            continue
        try:
            fallback = max(fallback, path.stat().st_mtime)
        except OSError:
            pass
        data = load_json(path)
        if isinstance(data, dict):
            ts_value = data.get("ts") or data.get("generated_at")
            if ts_value:
                return outcome_sort_time({"ts": ts_value}, fallback)
    return fallback


def int_metric(metrics: dict[str, Any], key: str) -> int:
    value = metrics.get(key)
    if isinstance(value, bool):
        return 0
    if isinstance(value, int):
        return value
    if isinstance(value, float):
        return int(value)
    if isinstance(value, str):
        try:
            return int(float(value))
        except ValueError:
            return 0
    return 0


def resolved_seed_replacement_metrics(metrics: dict[str, Any]) -> bool:
    seed_contradictions = int_metric(metrics, "task_seed_contradiction_count")
    if seed_contradictions <= 0:
        return False
    if int_metric(metrics, "task_manifest_seed_contradiction_count") > 0:
        return False
    selected = int_metric(metrics, "selected_task_count")
    strict_verified = int_metric(metrics, "task_strict_verified_count")
    succeeded = int_metric(metrics, "tasks_succeeded")
    return bool(
        selected > 0
        and strict_verified >= selected
        and succeeded >= selected
        and int_metric(metrics, "task_revert_count") == 0
        and int_metric(metrics, "task_obsolete_count") == 0
    )


def suppress_resolved_seed_feedback(feedback: dict[str, Any]) -> dict[str, Any]:
    metrics = feedback.get("metrics") if isinstance(feedback.get("metrics"), dict) else {}
    if not metrics or not resolved_seed_replacement_metrics(metrics):
        return feedback
    corrected = dict(feedback)
    corrected_metrics = dict(metrics)
    seed_contradictions = int_metric(corrected_metrics, "task_seed_contradiction_count")
    corrected_metrics["task_seed_contradiction_count"] = 0
    corrected_metrics["task_seed_replacement_count"] = max(
        int_metric(corrected_metrics, "task_seed_replacement_count"),
        seed_contradictions,
    )
    corrected["metrics"] = corrected_metrics
    lessons = feedback.get("top_lessons")
    if isinstance(lessons, list):
        corrected["top_lessons"] = [
            lesson
            for lesson in lessons
            if not (
                isinstance(lesson, dict)
                and lesson.get("kind") == "task_seed_contradiction"
            )
        ]
    return corrected


def compact_values(values: list[str]) -> list[str]:
    out: list[str] = []
    for value in values:
        text = " ".join(str(value).split()).strip()
        if text and text not in out:
            out.append(text)
    return out


def split_files(value: str) -> list[str]:
    return compact_values([part.strip() for part in value.replace(";", ",").split(",")])


def parse_planned_files(path: Path) -> list[str]:
    if not path.is_file():
        return []
    try:
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError:
        return []
    for line in lines:
        if line.startswith("Files:"):
            return split_files(line.partition(":")[2])
    return []


def path_matches(planned: str, touched: str) -> bool:
    planned = planned.strip().strip("/")
    touched = touched.strip().strip("/")
    if not planned or not touched:
        return False
    return touched == planned or touched.startswith(f"{planned}/")


def file_overlap(planned: list[str], touched: list[str]) -> bool:
    return any(path_matches(plan, touch) for plan in planned for touch in touched)


def explicit_pass(value: Any) -> bool:
    text = str(value or "").strip().lower()
    return text in {"pass", "passed", "ok", "success"} or text.startswith("pass:")


def explicit_fail(value: Any) -> bool:
    text = str(value or "").strip().lower()
    return text in {"fail", "failed", "failure"} or text.startswith("fail:")


def eval_timed_out_after_verdict(eval_data: dict[str, Any]) -> bool:
    if int(eval_data.get("exit_code") or 0) != 124:
        return False
    return (
        explicit_pass(eval_data.get("status"))
        or explicit_pass(eval_data.get("verdict"))
        or explicit_fail(eval_data.get("status"))
        or explicit_fail(eval_data.get("verdict"))
        or bool(eval_data.get("verdict_file"))
    )


def eval_passed(evals: list[dict[str, Any]], lineage_eval: Any) -> bool:
    if evals:
        return any(
            isinstance(eval_data, dict)
            and not eval_timed_out_after_verdict(eval_data)
            and (explicit_pass(eval_data.get("status")) or explicit_pass(eval_data.get("verdict")))
            for eval_data in evals
        )
    if isinstance(lineage_eval, dict):
        return explicit_pass(lineage_eval.get("verdict"))
    return False


def task_manifest_tasks(session_dir: Path) -> list[dict[str, Any]]:
    manifest = load_json(session_dir / "tasks" / "manifest.json")
    if not isinstance(manifest, dict):
        return []
    tasks = manifest.get("tasks")
    if isinstance(tasks, list):
        return [task for task in tasks if isinstance(task, dict)]
    selected = manifest.get("selected_tasks")
    if isinstance(selected, list):
        return [task for task in selected if isinstance(task, dict)]
    return []


def task_descriptor_text(task: dict[str, Any]) -> str:
    fields: list[str] = []
    for key in (
        "title",
        "name",
        "summary",
        "body",
        "body_preview",
        "description",
        "objective",
        "why",
    ):
        value = task.get(key)
        if isinstance(value, str):
            fields.append(value)
    return " ".join(fields).lower()


def recently_addressed_tool_failure_categories(audit_dir: Path) -> dict[str, str]:
    """Map historical tool-failure categories to recent strictly verified tasks.

    The dashboard projection aggregates cumulative failure counts. When a recent
    verified task already targeted a category, the trajectory should preserve the
    count as history without making it look like fresh unresolved pressure.
    """
    addressed: dict[str, str] = {}
    for session_dir, _ in load_recent_session_outcomes(audit_dir):
        verification = strict_task_verification(session_dir)
        verified_ids = {
            str(row.get("task_id") or "")
            for row in verification.get("rows", [])
            if row.get("strict_success")
        }
        if not verified_ids:
            continue
        for task in task_manifest_tasks(session_dir):
            task_id = str(task.get("task_id") or "")
            if task_id not in verified_ids:
                continue
            title = str(task.get("title") or task_id)
            title_text = title.lower()
            matched_by_title = False
            for category, patterns in TOOL_FAILURE_RECENT_TASK_PATTERNS.items():
                if category in addressed:
                    continue
                if any(pattern in title_text for pattern in patterns):
                    addressed[category] = title
                    matched_by_title = True
            if matched_by_title:
                continue
            text = task_descriptor_text(task)
            for category, patterns in TOOL_FAILURE_RECENT_TASK_PATTERNS.items():
                if category in addressed:
                    continue
                if any(pattern in text for pattern in patterns):
                    addressed[category] = title
    return addressed


def strict_task_verification(session_dir: Path) -> dict[str, Any]:
    tasks_dir = session_dir / "tasks"
    if not tasks_dir.is_dir():
        return {"task_count": 0, "verified_task_count": 0, "rows": []}

    selected = task_manifest_tasks(session_dir)
    artifact_ids = [
        path.name
        for path in sorted(tasks_dir.iterdir())
        if path.is_dir() and (path / "outcome.json").is_file()
    ]
    if not selected:
        selected = [{"task_id": task_id} for task_id in artifact_ids]

    rows: list[dict[str, Any]] = []
    for task in selected:
        task_id = str(task.get("task_id") or "")
        if not task_id:
            continue
        task_dir = tasks_dir / task_id
        outcome = load_json(task_dir / "outcome.json")
        if not isinstance(outcome, dict):
            outcome = {}
        evals = [
            eval_data
            for eval_path in sorted(task_dir.glob("eval_attempt_*.json"))
            if isinstance((eval_data := load_json(eval_path)), dict)
        ]
        planned = [
            str(path)
            for path in (
                task.get("files")
                or outcome.get("planned_files")
                or parse_planned_files(task_dir / "task.md")
                or []
            )
            if path
        ]
        touched = [
            str(path)
            for path in (
                outcome.get("source_files")
                or outcome.get("touched_files")
                or []
            )
            if path
        ]
        commits = [str(sha) for sha in (outcome.get("commit_shas") or []) if sha]
        overlap = file_overlap(planned, touched) if planned and touched else False
        verified = eval_passed(evals, outcome.get("eval"))
        timeout_with_verdict = any(
            isinstance(eval_data, dict) and eval_timed_out_after_verdict(eval_data)
            for eval_data in evals
        )
        status = str(outcome.get("status") or "")
        problems: list[str] = []
        if not planned:
            problems.append("missing_planned_files")
        if not touched:
            problems.append("no_touched_files")
        if planned and touched and not overlap:
            problems.append("no_planned_file_overlap")
        if timeout_with_verdict:
            problems.append("evaluator_timed_out_after_verdict")
        if not verified:
            problems.append("no_passing_verifier")
        if touched and not commits:
            problems.append("source_edits_not_landed")
        rows.append(
            {
                "task_id": task_id,
                "strict_success": status == "completed" and overlap and verified and not problems,
                "problems": problems,
            }
        )

    return {
        "task_count": len(rows),
        "verified_task_count": sum(1 for row in rows if row["strict_success"]),
        "rows": rows,
    }


def summarize_task_problems(rows: list[dict[str, Any]]) -> str:
    counts: Counter[str] = Counter()
    for row in rows:
        for problem in row.get("problems") or []:
            counts[str(problem)] += 1
    if not counts:
        return ""
    labels = {
        "missing_planned_files": "missing planned files",
        "no_touched_files": "no touched files",
        "no_planned_file_overlap": "no planned-file overlap",
        "evaluator_timed_out_after_verdict": "evaluator timeout after verdict",
        "no_passing_verifier": "no passing verifier",
        "source_edits_not_landed": "source edits not landed",
    }
    return "; ".join(
        f"{count} {labels.get(name, name.replace('_', ' '))}"
        for name, count in counts.most_common(3)
    )


def render_outcomes(
    session_outcomes: Union[list[tuple[Path, dict[str, Any]]], list[dict[str, Any]]],
) -> str:
    if not session_outcomes:
        return ""
    normalized: list[tuple[Optional[Path], dict[str, Any]]] = []
    for item in session_outcomes:
        if isinstance(item, tuple):
            normalized.append(item)
        else:
            normalized.append((None, item))
    display = normalized[:OUTCOME_DISPLAY_SESSIONS]
    if len(normalized) > len(display):
        lines = [f"## Recent session outcomes (newest {len(display)} of {len(normalized)})"]
    else:
        lines = ["## Recent session outcomes (last {})".format(len(normalized))]
    for session_dir, o in display:
        day = o.get("day", "?")
        ts = (o.get("ts") or "").replace("T", " ").rstrip("Z")
        attempted = o.get("tasks_attempted", 0)
        succeeded = o.get("tasks_succeeded", 0)
        build_ok = o.get("build_ok", False)
        test_ok = o.get("test_ok", False)
        reverted = o.get("reverted", False)
        verification = strict_task_verification(session_dir) if session_dir else {"task_count": 0}
        strict_total = int(verification.get("task_count") or 0)
        strict_verified = int(verification.get("verified_task_count") or 0)

        if reverted:
            icon = "❌"
            note = "REVERTED entire session"
        elif attempted == 0:
            icon = "•"
            note = "no tasks attempted"
        elif strict_total:
            if strict_verified == strict_total and build_ok and test_ok:
                icon = "✅"
                note = f"{strict_verified}/{strict_total} strict verified; build OK, tests OK"
            else:
                icon = "⚠️"
                issues = [f"{strict_verified}/{strict_total} strict verified"]
                if succeeded != strict_verified:
                    issues.append(f"raw outcome {succeeded}/{attempted}")
                problem_summary = summarize_task_problems(verification.get("rows") or [])
                if problem_summary:
                    issues.append(problem_summary)
                if not build_ok:
                    issues.append("build broken")
                if not test_ok:
                    issues.append("tests broken")
                note = "; ".join(issues) or "partial"
        elif session_dir and attempted > 0:
            icon = "⚠️"
            note = f"raw outcome {succeeded}/{attempted} lacks strict task evidence"
            if not build_ok:
                note += "; build broken"
            if not test_ok:
                note += "; tests broken"
        elif succeeded == attempted and build_ok and test_ok:
            icon = "✅"
            note = "build OK, tests OK"
        else:
            icon = "⚠️"
            issues = []
            if succeeded < attempted:
                issues.append(f"{attempted - succeeded} task(s) reverted")
            if not build_ok:
                issues.append("build broken")
            if not test_ok:
                issues.append("tests broken")
            note = ", ".join(issues) or "partial"

        lines.append(f"day-{day} ({ts}): tasks {succeeded}/{attempted} {icon} — {note}")
    omitted = len(normalized) - len(display)
    if omitted > 0:
        lines.append(f"... {omitted} older session outcome(s) omitted")
    return "\n".join(lines)


# ── Section 2: Per-task success rate from git log ────────────────────────


# Match commit messages like:
#   "Day 49 (16:24): Wire remaining useful bare subcommands (Task 3)"
#   "Day 57 (14:37): /watch multi-command support — run lint AND test in sequence (Task 2)"
TASK_COMMIT_RE = re.compile(
    r"^Day\s+(\d+)\s+\([^)]+\):\s+(.+?)\s+\(Task\s+\d+\)\s*$"
)
REVERT_COMMIT_RE = re.compile(
    r"^Day\s+\d+\s+\([^)]+\):\s+revert session changes", re.IGNORECASE
)


def collect_task_commits() -> tuple[list[tuple[int, str]], int]:
    """Return ([(day, title), ...], revert_commits_in_window)."""
    rc, stdout, _ = run_cmd(
        ["git", "log", f"--since={WINDOW_DAYS} days ago", "--format=%s"],
        timeout=15,
    )
    if rc != 0:
        return [], 0
    tasks = []
    reverts = 0
    for line in stdout.splitlines():
        m = TASK_COMMIT_RE.match(line)
        if m:
            tasks.append((int(m.group(1)), m.group(2).strip()))
            continue
        if REVERT_COMMIT_RE.match(line):
            reverts += 1
    return tasks, reverts


def render_task_success(tasks: list[tuple[int, str]]) -> str:
    if not tasks:
        return ""
    # Group by title; count attempts. Without ground truth on success per-task,
    # we treat the FIRST appearance of a title as 1 attempt; a re-appearance
    # within the window as another attempt. A title that appears with later
    # work on the same area without the agent re-trying it is a likely success.
    # That heuristic is weak — but it's the best we can do from commit messages
    # alone. We surface STUCK only when the threshold is unambiguous.
    title_attempts: defaultdict[str, list[int]] = defaultdict(list)
    for day, title in tasks:
        title_attempts[title].append(day)

    lines = ["## Per-task activity (last {} days)".format(WINDOW_DAYS)]
    stuck_titles = []
    for title, days in sorted(title_attempts.items(), key=lambda kv: -len(kv[1])):
        attempts = len(days)
        if attempts >= STUCK_ON_THRESHOLD:
            stuck_titles.append((title, attempts, days))
        # Cap output at top 5 most-active titles
        if len(lines) > 6:
            continue
        last_day = max(days)
        truncated_title = title[:60] + ("…" if len(title) > 60 else "")
        lines.append(f"\"{truncated_title}\": {attempts} attempt(s), last day-{last_day}")

    if stuck_titles:
        lines.append("")
        lines.append("⚠️ Possibly stuck (≥{} attempts in window):".format(STUCK_ON_THRESHOLD))
        for title, attempts, days in stuck_titles[:3]:
            t = title[:60] + ("…" if len(title) > 60 else "")
            lines.append(f"  - \"{t}\": {attempts}× (days {min(days)}-{max(days)})")
    return "\n".join(lines)


# ── Section 3: Reverts in window (already counted above) ─────────────────


def render_reverts(reverts: int, total_sessions: int) -> str:
    if total_sessions == 0:
        return ""
    if reverts == 0:
        return f"## Reverts in window\n0 of last ~{total_sessions} sessions had reverts."
    return f"## Reverts in window\n{reverts} revert commit(s) in last {WINDOW_DAYS} days."


# ── Section 4: Recurring CI errors via gh run view --log-failed ──────────


ERROR_LINE_RE = re.compile(r"(error|panicked|FAILED|fatal)", re.IGNORECASE)


def fingerprint_error_line(line: str) -> str:
    """Normalize an error line to a clusterable fingerprint."""
    s = strip_ansi(line).strip()
    # Strip GitHub Actions log prefix: <word> <word> ... <timestamp>
    # e.g. "social unknown step 2026-04-15T15:31:42.5342991Z error: auth"
    # The timestamp has format YYYY-MM-DDTHH:MM:SS[.fraction]Z
    s = re.sub(
        r"^(?:[A-Za-z_][\w-]*\s+)*"              # zero or more word prefixes
        r"\d{4}-\d{2}-\d{2}T[\d:.]+Z?\s*",        # ISO timestamp with subseconds
        "", s
    )
    # Strip leading log timestamps (standalone, at start of line)
    s = re.sub(r"^\d{4}-\d{2}-\d{2}T?[\d:.,Z+ ]*\s*", "", s)
    # Strip CI step prefixes like "build |" or "test │"
    s = re.sub(r"^[A-Za-z_-]+\s*[\|│]\s*", "", s)
    # Normalize file:line:column to file:N:N
    s = re.sub(r":\d+:\d+", ":N:N", s)
    s = re.sub(r":\d+\b", ":N", s)
    # Normalize hex addresses (0x7fff1234abcd) and UUIDs
    s = re.sub(r"0x[0-9a-fA-F]{4,}", "<HEX>", s)
    s = re.sub(
        r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}",
        "<UUID>", s
    )
    # Lowercase, collapse whitespace, truncate to 80 chars
    return re.sub(r"\s+", " ", s.lower())[:80]


def collect_failed_ci_fingerprints(repo: str) -> list[tuple[str, list[str]]]:
    """Return [(fingerprint, [run_ids_seen_at])]. Capped at MAX_FAILED_RUNS fetches.
    Silent return-empty paths now warn() so a misconfigured token / rate-limit
    doesn't masquerade as 'no failed runs' (would defeat the recurring-error
    detection this section exists for)."""
    if not repo:
        warn("YOYO_REPO empty — skipping recurring-CI-error section")
        return []
    rc, stdout, stderr = run_cmd(
        [
            "gh", "run", "list", "--repo", repo,
            "--status", "failure", "--limit", str(MAX_FAILED_RUNS),
            "--json", "databaseId,createdAt,name,workflowName",
        ],
        timeout=GH_RUN_LIST_TIMEOUT,
    )
    if rc != 0:
        warn(f"gh run list rc={rc}: {(stderr or '').strip()[:200]}")
        return []
    try:
        runs = json.loads(stdout)
    except json.JSONDecodeError as e:
        warn(f"gh run list returned non-JSON: {e}")
        return []
    if not runs:
        return []

    fingerprints: defaultdict[str, list[str]] = defaultdict(list)
    fetch_errors = 0
    for run in runs:
        run_id = str(run.get("databaseId") or "")
        if not run_id:
            continue
        rc2, log_stdout, stderr2 = run_cmd(
            ["gh", "run", "view", run_id, "--repo", repo, "--log-failed"],
            timeout=GH_RUN_VIEW_TIMEOUT,
        )
        if rc2 != 0:
            fetch_errors += 1
            warn(f"gh run view {run_id} rc={rc2}: {(stderr2 or '').strip()[:120]}")
            continue
        tail = log_stdout.splitlines()[-50:]
        seen_in_run = set()
        for ln in tail:
            if ERROR_LINE_RE.search(ln):
                fp = fingerprint_error_line(ln)
                if fp and fp not in seen_in_run:
                    fingerprints[fp].append(run_id)
                    seen_in_run.add(fp)
    if fetch_errors and not fingerprints:
        warn(f"all {fetch_errors} gh run view fetch(es) failed — section will be empty")
    return sorted(fingerprints.items(), key=lambda kv: -len(kv[1]))


def render_ci_errors(clusters: list[tuple[str, list[str]]]) -> str:
    if not clusters:
        return ""
    lines = ["## Recurring CI errors (failed runs in window)"]
    for fp, run_ids in clusters[:5]:
        n = len(run_ids)
        marker = f"{n}×" if n > 1 else "1×"
        # Truncate fingerprint to keep line tidy
        fp_short = fp[:90]
        lines.append(f"[{marker}] {fp_short}")
    return "\n".join(lines)


# ── Section 5: Provider/API health from audit.jsonl files ────────────────


PROVIDER_ERROR_RE = re.compile(r'"type"\s*:\s*"error"|provider_error|rate_limit', re.IGNORECASE)


AUDIT_FILE_SIZE_CAP = 10 * 1024 * 1024  # 10MB per file — guard against runaway audit.jsonl


def collect_provider_errors(audit_dir: Path) -> tuple[int, int]:
    """Return (sessions_examined, total_provider_error_hits).
    Streams audit.jsonl line-by-line so a multi-MB file doesn't slurp into
    memory. Per-file size cap (10MB) protects against pathological cases."""
    if not audit_dir.exists():
        return 0, 0
    sessions = 0
    hits = 0
    session_dirs = sorted(
        [child for child in audit_dir.iterdir() if child.is_dir()],
        key=session_sort_time,
        reverse=True,
    )
    for child in session_dirs:
        audit = child / "audit.jsonl"
        if not audit.is_file():
            continue
        sessions += 1
        try:
            size = audit.stat().st_size
            if size > AUDIT_FILE_SIZE_CAP:
                warn(f"{audit} is {size} bytes (>{AUDIT_FILE_SIZE_CAP}); scanning first {AUDIT_FILE_SIZE_CAP}B only")
            with audit.open(encoding="utf-8", errors="replace") as f:
                bytes_read = 0
                for line in f:
                    bytes_read += len(line)
                    if bytes_read > AUDIT_FILE_SIZE_CAP:
                        break
                    if PROVIDER_ERROR_RE.search(line):
                        hits += 1
        except OSError as e:
            warn(f"skipped {audit}: {e}")
        if sessions >= WINDOW_SESSIONS:
            break
    return sessions, hits


def render_provider_health(sessions: int, hits: int) -> str:
    if sessions == 0:
        return ""
    if hits == 0:
        return f"## Provider/API health\n{sessions} sessions, no provider errors detected."
    return f"## Provider/API health\n{sessions} sessions, {hits} provider error hit(s) in audit.jsonl."


# ── Section 6: Log feedback evals from prior GitHub Actions runs ─────────


def load_log_feedback(audit_dir: Path) -> list[dict]:
    if not audit_dir.exists() or not audit_dir.is_dir():
        return []
    triples: list[tuple[float, str, dict]] = []
    for child in audit_dir.iterdir():
        if not child.is_dir():
            continue
        feedback = child / "log_feedback.json"
        if not feedback.is_file():
            continue
        try:
            data = json.loads(feedback.read_text(encoding="utf-8", errors="replace"))
        except (OSError, json.JSONDecodeError, UnicodeDecodeError) as e:
            warn(f"skipped malformed {feedback}: {e}")
            continue
        data = suppress_resolved_seed_feedback(data)
        triples.append((session_sort_time(child), child.name, data))
    triples.sort(key=lambda t: t[0], reverse=True)
    return [t[2] for t in triples[:WINDOW_SESSIONS]]


def load_corrected_log_feedback_lessons(audit_dir: Path) -> list[dict[str, Any]]:
    if not audit_dir.exists() or not audit_dir.is_dir():
        return []
    try:
        from build_evolution_dashboard import load_sessions
    except Exception as e:  # pragma: no cover - defensive degradation for cron
        warn(f"could not import dashboard lessons for trajectory feedback: {e}")
        return []
    try:
        sessions = load_sessions(audit_dir, Path.cwd())
    except Exception as e:  # pragma: no cover - defensive degradation for cron
        warn(f"could not build corrected trajectory feedback lessons: {e}")
        return []
    if not sessions:
        return []
    work = sessions[-1].get("work_summary") if isinstance(sessions[-1].get("work_summary"), dict) else {}
    lessons = work.get("corrected_gnome_lessons")
    if not isinstance(lessons, list):
        return []
    return [lesson for lesson in lessons if isinstance(lesson, dict)]


def render_log_feedback(
    feedbacks: list[dict],
    corrected_lessons: Optional[list[dict[str, Any]]] = None,
) -> str:
    if not feedbacks:
        return ""
    latest = feedbacks[0]
    metrics = latest.get("metrics") if isinstance(latest.get("metrics"), dict) else {}
    score = metrics.get("coding_log_score")
    confidence = metrics.get("coding_log_confidence")
    recurring = metrics.get("recurring_failure_count", 0)
    capture = metrics.get("state_capture_coverage")
    lines = [
        "## GitHub Actions log feedback",
        f"latest score={score} confidence={confidence} recurring_failures={recurring} state_capture={capture}",
    ]

    lessons = corrected_lessons if corrected_lessons else latest.get("top_lessons")
    if isinstance(lessons, list) and lessons:
        if corrected_lessons:
            lines.append("Corrected top lessons for next run:")
        else:
            lines.append("Top lessons for next run:")
        for lesson in lessons[:3]:
            if not isinstance(lesson, dict):
                continue
            fp = str(lesson.get("fingerprint") or "")[:90]
            action = str(lesson.get("action") or "")[:100]
            if fp:
                lines.append(f"- {fp} -> {action}")

    recurring_counter: Counter[str] = Counter()
    for feedback in feedbacks:
        m = feedback.get("metrics")
        if not isinstance(m, dict):
            continue
        for item in m.get("failure_fingerprints", []) or []:
            if isinstance(item, dict) and item.get("fingerprint"):
                recurring_counter[str(item["fingerprint"])] += 1
    repeated = [(fp, count) for fp, count in recurring_counter.most_common(3) if count > 1]
    if repeated:
        lines.append("Historical repeated across prior log feedback:")
        for fp, count in repeated:
            lines.append(f"- {count}x {fp[:90]}")
    return "\n".join(lines)


def render_structured_state_snapshot(audit_dir: Path) -> str:
    if not audit_dir.exists() or not audit_dir.is_dir():
        return ""
    try:
        from build_evolution_dashboard import (
            build_claims_projection,
            build_dashboard_claim_summary,
            build_states_projection,
            load_sessions,
        )
    except Exception as e:  # pragma: no cover - defensive degradation for cron
        warn(f"could not import dashboard projections for trajectory snapshot: {e}")
        return ""

    try:
        sessions = load_sessions(audit_dir, Path.cwd())
        if not sessions:
            return ""
        generated_at = (
            datetime.now(timezone.utc)
            .replace(microsecond=0)
            .isoformat()
            .replace("+00:00", "Z")
        )
        claims = build_claims_projection(sessions, generated_at, audit_dir)
        claim_summary = build_dashboard_claim_summary(claims)
        states = build_states_projection(sessions, generated_at, audit_dir)
    except Exception as e:  # pragma: no cover - defensive degradation for cron
        warn(f"could not build structured trajectory snapshot: {e}")
        return ""

    claim_total = int(claim_summary.get("claim_count") or 0)
    claim_counts = (
        claim_summary.get("status_counts")
        if isinstance(claim_summary.get("status_counts"), dict)
        else {}
    )
    proven = int(claim_counts.get("proven") or 0)
    unresolved = int(claim_summary.get("unresolved_count") or 0)
    state_summary = states.get("summary") if isinstance(states.get("summary"), dict) else {}
    state_counts = state_summary.get("state_counts") if isinstance(state_summary.get("state_counts"), dict) else {}
    tool_failures: Counter[str] = Counter()
    addressed_tool_failures = recently_addressed_tool_failure_categories(audit_dir)
    latest_lifecycle_counts: Counter[str] = Counter()
    for session in states.get("sessions", []) if isinstance(states.get("sessions"), list) else []:
        if not isinstance(session, dict):
            continue
        failure_summary = (
            (session.get("tool_failures") or {}).get("summary")
            if isinstance(session.get("tool_failures"), dict)
            else {}
        )
        category_counts = (
            failure_summary.get("category_counts")
            if isinstance(failure_summary, dict)
            else {}
        )
        if isinstance(category_counts, dict):
            for category, count in category_counts.items():
                tool_failures[str(category)] += int(count or 0)
        lifecycle = session.get("lifecycle") if isinstance(session.get("lifecycle"), dict) else {}
        runs = lifecycle.get("runs") if isinstance(lifecycle.get("runs"), dict) else {}
        model_calls = (
            lifecycle.get("model_calls") if isinstance(lifecycle.get("model_calls"), dict) else {}
        )
        session_lifecycle_counts: Counter[str] = Counter()
        for prefix, source in (("state_run", runs), ("deepseek_model_call", model_calls)):
            for key in (
                "started",
                "completed",
                "incomplete",
                "unmatched_completed",
                "unmatched_non_validation_completed",
                "unstarted_input_validation_error",
                "abnormal_completed",
            ):
                value = source.get(key)
                if isinstance(value, (int, float)) and not isinstance(value, bool):
                    session_lifecycle_counts[f"{prefix}_{key}_count"] += int(value)
        if session_lifecycle_counts:
            latest_lifecycle_counts = session_lifecycle_counts

    top_states: list[tuple[str, Any]] = []
    if state_counts:
        top_states = sorted(
            state_counts.items(),
            key=lambda item: (-int(item[1] or 0), str(item[0])),
        )[:5]
    top_tool_failures = tool_failures.most_common(5)

    lines = ["## Structured state snapshot"]
    if claim_total:
        summary_parts = [f"claims: {proven}/{claim_total} proven; {unresolved} unresolved"]
        if latest_lifecycle_counts:
            lifecycle_summary = []
            for key, label in (
                ("state_run_incomplete_count", "state_incomplete"),
                ("state_run_unmatched_completed_count", "state_unmatched_completed"),
                ("state_run_unmatched_non_validation_completed_count", "state_unmatched_non_validation"),
                ("deepseek_model_call_incomplete_count", "model_incomplete"),
                ("deepseek_model_call_unmatched_completed_count", "model_unmatched_completed"),
            ):
                if key in latest_lifecycle_counts:
                    lifecycle_summary.append(f"{label}={latest_lifecycle_counts[key]}")
            if lifecycle_summary:
                summary_parts.append("lifecycle gaps: " + ", ".join(lifecycle_summary))
        if top_states:
            summary_parts.append(
                "top task states: "
                + ", ".join(f"{state}={count}" for state, count in top_states[:3])
            )
        if top_tool_failures:
            tool_summary = []
            for category, count in top_tool_failures[:3]:
                suffix = " addressed" if category in addressed_tool_failures else ""
                tool_summary.append(f"{category}={count}{suffix}")
            summary_parts.append("historical tool failures: " + ", ".join(tool_summary))
        lines.append("; ".join(summary_parts))
    if latest_lifecycle_counts:
        keys = (
            "state_run_started_count",
            "state_run_completed_count",
            "state_run_incomplete_count",
            "state_run_unmatched_completed_count",
            "state_run_unmatched_non_validation_completed_count",
            "state_run_unstarted_input_validation_error_count",
            "deepseek_model_call_started_count",
            "deepseek_model_call_completed_count",
            "deepseek_model_call_incomplete_count",
            "deepseek_model_call_unmatched_completed_count",
            "deepseek_model_call_abnormal_completed_count",
        )
        lines.append(
            "latest lifecycle gnomes: "
            + "; ".join(
                f"{key}={latest_lifecycle_counts[key]}" for key in keys if key in latest_lifecycle_counts
            )
        )
    for row in (claim_summary.get("top_unresolved") or [])[:3]:
        if not isinstance(row, dict):
            continue
        latest = row.get("latest_session_id")
        latest_text = f" latest={latest}" if latest else ""
        lines.append(
            f"- {row.get('status', 'unknown')} "
            f"{row.get('count', 0)}x {row.get('name', 'unknown_claim')}"
            f"{latest_text}"
        )
    if top_states:
        lines.append("task states: " + "; ".join(f"{state}={count}" for state, count in top_states))
    if top_tool_failures:
        failure_parts = []
        for category, count in top_tool_failures:
            part = f"{category}={count}"
            addressed_title = addressed_tool_failures.get(category)
            if addressed_title:
                title = addressed_title[:48] + ("..." if len(addressed_title) > 48 else "")
                part += f" (recent verified task: {title})"
            failure_parts.append(part)
        lines.append(
            "historical tool failures: "
            + "; ".join(failure_parts)
        )
    if len(lines) == 1:
        return ""
    return "\n".join(lines)


def render_graph_suggestions(audit_dir: Path) -> str:
    sessions = ordered_sessions(audit_dir)
    if not sessions:
        return ""
    latest = sessions[-1]
    suggestions = evolution_suggestions(latest, limit=3)
    if not suggestions:
        return ""
    lines = ["## Graph-derived next-task pressure"]
    for suggestion in suggestions:
        lines.append(
            "- {} ({}={}): {}".format(
                truncate_text(suggestion.get("title"), 80),
                suggestion.get("metric"),
                suggestion.get("value"),
                truncate_text(suggestion.get("reason"), 72),
            )
        )
    return "\n".join(lines)


# ── Final assembly ───────────────────────────────────────────────────────


def main() -> int:
    audit_dir_str = os.environ.get("YOYO_AUDIT_DIR", "")
    repo = os.environ.get("YOYO_REPO", "")
    day = os.environ.get("YOYO_DAY", "?")
    out_path_str = os.environ.get(
        "YOYO_TRAJECTORY_OUT", ".yoyo/session_staging/trajectory.md"
    )
    out_path = Path(out_path_str)
    out_path.parent.mkdir(parents=True, exist_ok=True)

    # Drop any stale output from a prior session — guards against the case
    # where extractor errors mid-run and a partial file survives. Matches
    # the contract evolve.sh expects: file present iff this run wrote it.
    try:
        out_path.unlink()
    except FileNotFoundError:
        pass
    except OSError as e:
        warn(f"could not unlink stale {out_path}: {e}")

    audit_dir = Path(audit_dir_str) if audit_dir_str else Path("/dev/null")

    header = (
        f"# YOUR TRAJECTORY\n\n"
        f"Last computed: {datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%MZ')}. "
        f"Day {day}. Window: last {WINDOW_SESSIONS} sessions / {WINDOW_DAYS} days.\n"
    )

    # Gather all sections (each falls back to "" silently on no-data)
    outcomes = load_recent_session_outcomes(audit_dir)
    tasks, reverts = collect_task_commits()
    sessions_audited, provider_hits = collect_provider_errors(audit_dir)
    ci_clusters = collect_failed_ci_fingerprints(repo)
    log_feedback = load_log_feedback(audit_dir)
    corrected_feedback_lessons = load_corrected_log_feedback_lessons(audit_dir)

    sections: list[str] = []
    s = render_outcomes(outcomes)
    if s:
        sections.append(s)
    s = render_task_success(tasks)
    if s:
        sections.append(s)
    s = render_reverts(reverts, len(outcomes))
    if s:
        sections.append(s)
    s = render_ci_errors(ci_clusters)
    if s:
        sections.append(s)
    s = render_provider_health(sessions_audited, provider_hits)
    if s:
        sections.append(s)
    s = render_graph_suggestions(audit_dir)
    if s:
        sections.append(s)
    s = render_log_feedback(log_feedback, corrected_feedback_lessons)
    if s:
        sections.append(s)
    s = render_structured_state_snapshot(audit_dir)
    if s:
        sections.append(s)

    if not sections:
        body = "(no trajectory data yet — audit-log is empty and no recent task commits found)"
    else:
        body = "\n\n".join(sections)

    output = header + "\n" + body + "\n"
    # Hard-cap: lines and bytes. Bytes-cap reserves room for the truncation
    # marker so the FINAL output stays under TOTAL_BYTE_CAP (the marker
    # itself was previously appended after the cap, allowing the file to
    # exceed it by ~37 bytes).
    output = truncate_lines(output, TOTAL_LINE_CAP)
    truncation_marker = "\n... (truncated to fit token budget)\n"
    marker_bytes = len(truncation_marker.encode("utf-8"))
    if len(output.encode("utf-8")) > TOTAL_BYTE_CAP:
        budget = TOTAL_BYTE_CAP - marker_bytes
        b = output.encode("utf-8")[:budget]
        # Back off to last newline within b for clean cut
        idx = b.rfind(b"\n")
        if idx > 0:
            b = b[:idx]
        output = (
            drop_dangling_trailing_section_header(b.decode("utf-8", errors="ignore"))
            + truncation_marker
        )

    try:
        out_path.write_text(output)
    except OSError as e:
        warn(f"could not write {out_path}: {e}")
        return 1
    return 0


def run_self_tests() -> int:
    """Self-tests for fingerprint clustering. Run with --test flag."""
    failures = 0

    def assert_eq(label: str, got: str, want: str) -> None:
        nonlocal failures
        if got != want:
            print(f"  FAIL: {label}")
            print(f"    got:  {got!r}")
            print(f"    want: {want!r}")
            failures += 1
        else:
            print(f"  ok: {label}")

    print("=== fingerprint_error_line self-tests ===\n")

    # 1. GH Actions prefixes with different timestamps cluster together
    line_a = "social unknown step 2026-04-15T15:31:42.5342991Z error: auth token expired"
    line_b = "social unknown step 2026-04-08T07:12:03.8992940Z error: auth token expired"
    fp_a = fingerprint_error_line(line_a)
    fp_b = fingerprint_error_line(line_b)
    assert_eq("GH Actions auth errors cluster", fp_a, fp_b)
    # Verify the prefix was actually stripped
    assert_eq("GH Actions prefix stripped", fp_a, "error: auth token expired")

    # 2. Different GH Actions workflows with same error cluster
    line_c = "evolve build test 2026-04-20T10:00:00.1Z FAILED: cargo test exit code 1"
    line_d = "evolve build test 2026-04-21T14:30:00.9999Z FAILED: cargo test exit code 1"
    fp_c = fingerprint_error_line(line_c)
    fp_d = fingerprint_error_line(line_d)
    assert_eq("different workflow timestamps cluster", fp_c, fp_d)

    # 3. Standalone ISO timestamps at line start still stripped
    line_e = "2026-04-15T15:31:42Z error: something broke"
    line_f = "2026-04-08T07:12:03Z error: something broke"
    fp_e = fingerprint_error_line(line_e)
    fp_f = fingerprint_error_line(line_f)
    assert_eq("standalone timestamps cluster", fp_e, fp_f)

    # 4. Hex addresses are normalized
    line_g = "panicked at 0x7fff1234abcd: null pointer"
    line_h = "panicked at 0xdeadbeef9876: null pointer"
    fp_g = fingerprint_error_line(line_g)
    fp_h = fingerprint_error_line(line_h)
    assert_eq("hex addresses cluster", fp_g, fp_h)
    assert_eq("hex replaced with placeholder", "panicked at <hex>: null pointer", fp_g)

    # 5. UUIDs are normalized
    line_i = "error: session 550e8400-e29b-41d4-a716-446655440000 not found"
    line_j = "error: session a1b2c3d4-e5f6-7890-abcd-ef1234567890 not found"
    fp_i = fingerprint_error_line(line_i)
    fp_j = fingerprint_error_line(line_j)
    assert_eq("UUIDs cluster", fp_i, fp_j)

    # 6. file:line:column normalised
    line_k = "error[E0308]: src/main.rs:42:10: type mismatch"
    line_l = "error[E0308]: src/main.rs:99:5: type mismatch"
    fp_k = fingerprint_error_line(line_k)
    fp_l = fingerprint_error_line(line_l)
    assert_eq("file:line:col clusters", fp_k, fp_l)

    # 7. ANSI codes stripped
    line_m = "\x1b[31merror\x1b[0m: something failed"
    fp_m = fingerprint_error_line(line_m)
    assert_eq("ANSI stripped", fp_m, "error: something failed")

    # 8. Subsecond precision doesn't prevent clustering
    line_n = "ci build run 2026-01-01T00:00:00.1Z fatal: git push rejected"
    line_o = "ci build run 2026-06-15T23:59:59.9999999Z fatal: git push rejected"
    fp_n = fingerprint_error_line(line_n)
    fp_o = fingerprint_error_line(line_o)
    assert_eq("subsecond precision clusters", fp_n, fp_o)

    print(f"\n{'ALL PASSED' if failures == 0 else f'{failures} FAILURE(S)'}")
    return 1 if failures else 0


if __name__ == "__main__":
    if "--test" in sys.argv:
        sys.exit(run_self_tests())
    sys.exit(main())
