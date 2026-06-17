#!/usr/bin/env python3
"""Verify that an evolution task changed files it planned to change."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


def compact(values: list[str], limit: int = 120) -> list[str]:
    out: list[str] = []
    for value in values:
        text = " ".join(str(value).split()).strip()
        if text and text not in out:
            out.append(text)
        if len(out) >= limit:
            break
    return out


def split_files(value: str) -> list[str]:
    return compact([part.strip() for part in value.replace(";", ",").split(",")])


def parse_planned_files(path: Path) -> list[str]:
    if not path.is_file():
        return []
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.startswith("Files:"):
            return split_files(line.partition(":")[2])
    return []


def git(repo: Path, args: list[str]) -> list[str]:
    result = subprocess.run(
        ["git", "-C", str(repo), *args],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    if result.returncode != 0:
        return []
    return [line.strip() for line in result.stdout.splitlines() if line.strip()]


def changed_files(repo: Path, base: str) -> list[str]:
    files: list[str] = []
    head = git(repo, ["rev-parse", "HEAD"])
    current = head[0] if head else ""
    if base and current and base != current:
        files.extend(git(repo, ["diff", "--name-only", f"{base}..{current}"]))
    files.extend(git(repo, ["diff", "--cached", "--name-only"]))
    files.extend(git(repo, ["diff", "--name-only"]))
    files.extend(git(repo, ["ls-files", "--others", "--exclude-standard"]))
    return compact(files)


def git_diff_summary(repo: Path) -> dict[str, list[str]]:
    """Collect staged, unstaged, and untracked files as three separate lists."""
    return {
        "staged": git(repo, ["diff", "--cached", "--name-only"]),
        "unstaged": git(repo, ["diff", "--name-only"]),
        "untracked": git(repo, ["ls-files", "--others", "--exclude-standard"]),
    }


def path_matches(planned: str, touched: str) -> bool:
    planned = planned.strip().strip("/")
    touched = touched.strip().strip("/")
    if not planned or not touched:
        return False
    return touched == planned or touched.startswith(f"{planned}/")


def verify(repo: Path, base: str, task_file: Path) -> dict[str, Any]:
    planned = parse_planned_files(task_file)
    touched = changed_files(repo, base)
    overlapping = compact(
        [
            touched_file
            for touched_file in touched
            if any(path_matches(planned_file, touched_file) for planned_file in planned)
        ]
    )
    ok = bool(planned and touched and overlapping)
    if not planned:
        reason = "task file has no Files: entries"
    elif not touched:
        reason = "task produced no git-visible file changes"
    elif not overlapping:
        reason = "task changes do not overlap planned Files entries"
    else:
        reason = "task changed planned files"
    return {
        "ok": ok,
        "reason": reason,
        "planned_files": planned,
        "touched_files": touched,
        "overlapping_files": overlapping,
        "unplanned_touched_files": [
            path
            for path in touched
            if not any(path_matches(planned_file, path) for planned_file in planned)
        ],
        "diff_summary": git_diff_summary(repo),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--base", default="")
    parser.add_argument("--task-file", type=Path)
    parser.add_argument("--test", action="store_true")
    args = parser.parse_args()
    if args.test:
        return run_self_tests()
    if not args.base or args.task_file is None:
        parser.error("--base and --task-file are required unless --test is used")
    json.dump(verify(args.repo_root, args.base, args.task_file), fp=sys.stdout)
    return 0


def run_self_tests() -> int:
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        repo = Path(tmp)
        subprocess.run(["git", "-C", str(repo), "init"], check=True, stdout=subprocess.DEVNULL)
        subprocess.run(["git", "-C", str(repo), "config", "user.name", "Test"], check=True)
        subprocess.run(["git", "-C", str(repo), "config", "user.email", "test@example.com"], check=True)
        (repo / "src").mkdir()
        (repo / "session_plan").mkdir()
        (repo / "src/lib.rs").write_text("pub fn before() {}\n", encoding="utf-8")
        subprocess.run(["git", "-C", str(repo), "add", "src/lib.rs"], check=True)
        subprocess.run(["git", "-C", str(repo), "commit", "-m", "base"], check=True, stdout=subprocess.DEVNULL)
        base = git(repo, ["rev-parse", "HEAD"])[0]
        task = repo / "session_plan/task_01.md"
        task.write_text("Title: x\nFiles: src/lib.rs\n", encoding="utf-8")
        (repo / "src/lib.rs").write_text("pub fn after() {}\n", encoding="utf-8")
        assert verify(repo, base, task)["ok"] is True
        task.write_text("Title: x\nFiles: README.md\n", encoding="utf-8")
        failed = verify(repo, base, task)
        assert failed["ok"] is False
        assert failed["reason"] == "task changes do not overlap planned Files entries"
        # Test diff_summary captures staged, unstaged, and untracked files.
        task.write_text("Title: x\nFiles: src/lib.rs\n", encoding="utf-8")
        subprocess.run(["git", "-C", str(repo), "add", "src/lib.rs"], check=True)
        unstaged_file = repo / "src/unstaged.rs"
        unstaged_file.write_text("// unstaged\n", encoding="utf-8")
        subprocess.run(["git", "-C", str(repo), "add", unstaged_file])
        # Now modify it to make it unstaged
        unstaged_file.write_text("// unstaged modified\n", encoding="utf-8")
        untracked_file = repo / "new_file.txt"
        untracked_file.write_text("untracked\n", encoding="utf-8")
        result = verify(repo, base, task)
        ds = result["diff_summary"]
        assert "src/lib.rs" in ds["staged"], f"expected src/lib.rs in staged, got {ds['staged']}"
        assert "src/unstaged.rs" in ds["unstaged"], f"expected src/unstaged.rs in unstaged, got {ds['unstaged']}"
        assert "new_file.txt" in ds["untracked"], f"expected new_file.txt in untracked, got {ds['untracked']}"
    print("task_verification_gate self-tests passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
