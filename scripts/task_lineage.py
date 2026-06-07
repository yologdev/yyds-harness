#!/usr/bin/env python3
"""Build explicit per-task lineage payloads for evolution state events."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


NON_SOURCE_PREFIXES = (
    ".yoyo/",
    "journals/",
    "memory/",
    "session_plan/",
    "sessions/",
    "site/",
)
NON_SOURCE_FILES = {".skill_evolve_counter", "DAY_COUNT", "ISSUES_TODAY.md"}


def source_file(path: str) -> bool:
    if not path:
        return False
    if path.startswith(NON_SOURCE_PREFIXES):
        return False
    return path not in NON_SOURCE_FILES


def compact(values: list[str], limit: int = 80) -> list[str]:
    out: list[str] = []
    for value in values:
        text = " ".join(str(value).split())
        if text and text not in out:
            out.append(text)
        if len(out) >= limit:
            break
    return out


def git(repo: Path, args: list[str]) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo), *args],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    return result.stdout if result.returncode == 0 else ""


def task_id(task_number: int) -> str:
    return f"task_{task_number:02d}"


def parse_task_file(path: Path | None) -> dict[str, Any]:
    if path is None or not path.is_file():
        return {"planned_files": [], "issue": None}
    title = ""
    files: list[str] = []
    issue: str | None = None
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.startswith("Title:") and not title:
            title = line.partition(":")[2].strip()
        elif line.startswith("Files:"):
            raw = line.partition(":")[2].strip()
            files = compact([part.strip() for part in raw.replace(";", ",").split(",")])
        elif line.startswith("Issue:"):
            issue = line.partition(":")[2].strip() or None
    return {"task_file_title": title, "planned_files": files, "issue": issue}


def parse_eval_file(path: Path | None) -> dict[str, Any] | None:
    if path is None or not path.is_file():
        return None
    verdict = ""
    reason = ""
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.lower().startswith("verdict:"):
            verdict = line.partition(":")[2].strip()
        elif line.lower().startswith("reason:"):
            reason = line.partition(":")[2].strip()
    if not verdict and not reason:
        return None
    return {"verdict": verdict or None, "reason": reason or None}


def commit_records(repo: Path, base: str, head: str) -> list[dict[str, Any]]:
    if not base or not head or base == head:
        return []
    raw = git(
        repo,
        [
            "log",
            "--reverse",
            "--format=%x1e%H%x00%s",
            "--name-only",
            f"{base}..{head}",
        ],
    )
    commits: list[dict[str, Any]] = []
    for raw_record in raw.split("\x1e"):
        record = raw_record.strip()
        if not record:
            continue
        lines = [line for line in record.splitlines() if line.strip()]
        if not lines or "\x00" not in lines[0]:
            continue
        sha, subject = lines[0].split("\x00", 1)
        files = compact(lines[1:])
        commits.append(
            {
                "sha": sha,
                "short_sha": sha[:7],
                "subject": subject,
                "files": files,
                "source_files": [path for path in files if source_file(path)],
            }
        )
    return commits


def changed_files(repo: Path, base: str, head: str) -> list[str]:
    files: list[str] = []
    if base and head and base != head:
        files.extend(git(repo, ["diff", "--name-only", f"{base}..{head}"]).splitlines())
    files.extend(git(repo, ["diff", "--cached", "--name-only"]).splitlines())
    files.extend(git(repo, ["diff", "--name-only"]).splitlines())
    return compact(files)


def build_payload(args: argparse.Namespace) -> dict[str, Any]:
    head = args.head or git(args.repo_root, ["rev-parse", "HEAD"]).strip()
    files = changed_files(args.repo_root, args.base, head)
    commits = commit_records(args.repo_root, args.base, head)
    task_meta = parse_task_file(args.task_file)
    payload: dict[str, Any] = {
        "phase": "task",
        "task_id": task_id(args.task_number),
        "task_number": args.task_number,
        "task_title": args.task_title,
        "status": args.status,
        "base_commit": args.base or None,
        "head_commit": head or None,
        "touched_files": files,
        "source_files": [path for path in files if source_file(path)],
        "commit_shas": [commit["sha"] for commit in commits],
        "commits": commits,
        "eval": parse_eval_file(args.eval_file),
        "revert_reason": args.reason or None,
        "gnome_deltas": {},
    }
    payload.update(task_meta)
    return payload


def load_events(events_path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    if not events_path.is_file():
        return rows
    with events_path.open(encoding="utf-8", errors="replace") as handle:
        for line in handle:
            text = line.strip()
            if not text:
                continue
            try:
                value = json.loads(text)
            except json.JSONDecodeError:
                continue
            if isinstance(value, dict):
                rows.append(value)
    return rows


def event_payload(event: dict[str, Any]) -> dict[str, Any]:
    value = event.get("payload")
    return value if isinstance(value, dict) else {}


def task_rows(events: list[dict[str, Any]]) -> list[dict[str, Any]]:
    tasks: dict[str, dict[str, Any]] = {}
    for event in events:
        data = event_payload(event)
        if data.get("phase") != "task":
            continue
        tid = str(data.get("task_id") or "")
        if not tid and isinstance(data.get("task_number"), int):
            tid = task_id(int(data["task_number"]))
        if not tid:
            continue
        row = tasks.setdefault(
            tid,
            {
                "task_id": tid,
                "task_number": data.get("task_number"),
                "task_title": data.get("task_title"),
                "source_files": [],
                "commit_shas": [],
            },
        )
        for key in ("task_title", "status", "head_commit"):
            if data.get(key) is not None:
                row[key] = data.get(key)
        for key in ("source_files", "touched_files", "commit_shas"):
            values = data.get(key)
            if isinstance(values, list):
                row[key] = compact([str(value) for value in values])
    return sorted(
        tasks.values(),
        key=lambda row: (
            row.get("task_number") if isinstance(row.get("task_number"), int) else 999,
            str(row.get("task_id") or ""),
        ),
    )


def build_link_payload(args: argparse.Namespace) -> dict[str, Any]:
    head = args.head or git(args.repo_root, ["rev-parse", "HEAD"]).strip()
    tasks = task_rows(load_events(args.events))
    commits = [commit for commit in commit_records(args.repo_root, args.base, head) if commit["source_files"]]
    known = {
        str(sha)
        for task in tasks
        for sha in (task.get("commit_shas") or [])
    }
    linked_tasks: list[dict[str, Any]] = []
    assigned: set[str] = set()
    for task in tasks:
        task_sources = set(task.get("source_files") or [])
        linked = [
            commit
            for commit in commits
            if commit["sha"] not in known and task_sources.intersection(commit.get("source_files") or [])
        ]
        if not linked:
            continue
        assigned.update(str(commit["sha"]) for commit in linked)
        linked_tasks.append(
            {
                "task_id": task.get("task_id"),
                "task_number": task.get("task_number"),
                "task_title": task.get("task_title"),
                "linked_by": "source_file_overlap",
                "linked_commit_shas": [commit["sha"] for commit in linked],
                "linked_commits": linked,
            }
        )
    return {
        "phase": "task_commit_linkage",
        "decision_type": "task_commit_linkage",
        "base_commit": args.base or None,
        "head_commit": head or None,
        "tasks": linked_tasks,
        "unassigned_source_commits": [
            commit for commit in commits if commit["sha"] not in known and commit["sha"] not in assigned
        ],
    }


def run_self_tests() -> int:
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        repo = Path(tmp)
        subprocess.run(["git", "-C", str(repo), "init"], check=True, stdout=subprocess.DEVNULL)
        subprocess.run(["git", "-C", str(repo), "config", "user.name", "Test"], check=True)
        subprocess.run(["git", "-C", str(repo), "config", "user.email", "test@example.com"], check=True)
        (repo / "src").mkdir()
        (repo / "session_plan").mkdir()
        (repo / "src/lib.rs").write_text("pub fn a() {}\n", encoding="utf-8")
        subprocess.run(["git", "-C", str(repo), "add", "src/lib.rs"], check=True)
        subprocess.run(["git", "-C", str(repo), "commit", "-m", "base"], check=True, stdout=subprocess.DEVNULL)
        base = git(repo, ["rev-parse", "HEAD"]).strip()
        (repo / "src/lib.rs").write_text("pub fn b() {}\n", encoding="utf-8")
        (repo / "session_plan/task_01.md").write_text(
            "Title: Improve thing\nFiles: src/lib.rs, session_plan/task_01.md\nIssue: none\n",
            encoding="utf-8",
        )
        subprocess.run(["git", "-C", str(repo), "add", "src/lib.rs", "session_plan/task_01.md"], check=True)
        subprocess.run(
            ["git", "-C", str(repo), "commit", "-m", "Day 1 (00:00): Improve thing (Task 1)"],
            check=True,
            stdout=subprocess.DEVNULL,
        )
        args = argparse.Namespace(
            repo_root=repo,
            base=base,
            head="",
            task_number=1,
            task_title="Improve thing",
            status="completed",
            task_file=repo / "session_plan/task_01.md",
            eval_file=None,
            reason="",
        )
        payload = build_payload(args)
        assert payload["task_id"] == "task_01"
        assert payload["source_files"] == ["src/lib.rs"]
        assert payload["planned_files"] == ["src/lib.rs", "session_plan/task_01.md"]
        assert len(payload["commit_shas"]) == 1
        link_args = argparse.Namespace(repo_root=repo, base=base, head="", events=repo / "events.jsonl")
        append = {
            "event_type": "RunCompleted",
            "payload": {
                "phase": "task",
                "task_id": "task_01",
                "task_number": 1,
                "task_title": "Improve thing",
                "source_files": ["src/lib.rs"],
                "commit_shas": [],
            },
        }
        (repo / "events.jsonl").write_text(json.dumps(append) + "\n", encoding="utf-8")
        link_payload = build_link_payload(link_args)
        assert link_payload["tasks"][0]["linked_commit_shas"]
    print("task_lineage self-tests passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--base", default="")
    parser.add_argument("--head", default="")
    parser.add_argument("--task-number", type=int)
    parser.add_argument("--task-title", default="")
    parser.add_argument("--status", default="")
    parser.add_argument("--task-file", type=Path)
    parser.add_argument("--eval-file", type=Path)
    parser.add_argument("--reason", default="")
    parser.add_argument("--link-commits", action="store_true")
    parser.add_argument("--events", type=Path)
    parser.add_argument("--test", action="store_true")
    args = parser.parse_args()
    if args.test:
        return run_self_tests()
    if args.link_commits:
        if args.events is None or not args.base:
            parser.error("--events and --base are required with --link-commits")
        json.dump(build_link_payload(args), sys.stdout, sort_keys=True, separators=(",", ":"))
        sys.stdout.write("\n")
        return 0
    if args.task_number is None or not args.task_title or not args.status:
        parser.error("--task-number, --task-title, and --status are required unless --test is used")
    json.dump(build_payload(args), sys.stdout, sort_keys=True, separators=(",", ":"))
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
