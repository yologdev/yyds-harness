#!/usr/bin/env python3
"""Ensure a verified evolution task has landed source commit evidence."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent))

import task_lineage  # noqa: E402


def git(repo: Path, args: list[str], check: bool = False) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "-C", str(repo), *args],
        check=check,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=30,
    )


def git_lines(repo: Path, args: list[str]) -> list[str]:
    result = git(repo, args)
    if result.returncode != 0:
        return []
    return [line.strip() for line in result.stdout.splitlines() if line.strip()]


def uncommitted_source_files(repo: Path) -> list[str]:
    files = []
    files.extend(git_lines(repo, ["diff", "--cached", "--name-only"]))
    files.extend(git_lines(repo, ["diff", "--name-only"]))
    files.extend(git_lines(repo, ["ls-files", "--others", "--exclude-standard"]))
    return task_lineage.compact([path for path in files if task_lineage.source_file(path)])


def source_commit_records(repo: Path, base: str, head: str) -> list[dict[str, Any]]:
    return [
        commit
        for commit in task_lineage.commit_records(repo, base, head)
        if commit.get("source_files")
    ]


def status(repo: Path, base: str) -> dict[str, Any]:
    head = git(repo, ["rev-parse", "HEAD"]).stdout.strip()
    touched = [
        path
        for path in task_lineage.changed_files(repo, base, head)
        if task_lineage.source_file(path)
    ]
    source_commits = source_commit_records(repo, base, head)
    uncommitted = uncommitted_source_files(repo)
    ok = not touched or bool(source_commits)
    if ok:
        reason = "task source changes have landed commit evidence"
    elif uncommitted:
        reason = "task has uncommitted source changes"
    else:
        reason = "task has source changes but no landed source commit"
    return {
        "ok": ok,
        "reason": reason,
        "base_commit": base or None,
        "head_commit": head or None,
        "source_files": touched,
        "uncommitted_source_files": uncommitted,
        "source_commit_shas": [str(commit.get("sha")) for commit in source_commits if commit.get("sha")],
        "source_commits": source_commits,
    }


def auto_commit(repo: Path, files: list[str], message: str) -> dict[str, Any]:
    if not files:
        return {"attempted": False}
    add = git(repo, ["add", "--", *files])
    if add.returncode != 0:
        return {"attempted": True, "ok": False, "reason": add.stderr.strip() or "git add failed"}
    diff = git(repo, ["diff", "--cached", "--quiet"])
    if diff.returncode == 0:
        file_exists = [f for f in files if (repo / f).exists()]
        file_missing = [f for f in files if not (repo / f).exists()]
        reason_parts = ["no staged source changes after git add"]
        if file_missing:
            reason_parts.append(f"missing from disk: {', '.join(file_missing)}")
        if file_exists:
            reason_parts.append(f"present on disk but nothing staged: {', '.join(file_exists)}")
        return {
            "attempted": True,
            "ok": True,
            "reason": "; ".join(reason_parts),
            "file_exists": file_exists,
            "file_missing": file_missing,
            "committed": False,
        }
    commit = git(repo, ["commit", "-m", message])
    return {
        "attempted": True,
        "ok": commit.returncode == 0,
        "reason": commit.stderr.strip() or commit.stdout.strip() or "git commit completed",
        "committed": commit.returncode == 0,
    }


def verify(repo: Path, base: str, message: str, auto: bool) -> dict[str, Any]:
    before = status(repo, base)
    commit_result: dict[str, Any] = {"attempted": False}
    if auto and before["uncommitted_source_files"]:
        commit_result = auto_commit(repo, before["uncommitted_source_files"], message)
    after = status(repo, base)
    result = dict(after)
    result["auto_commit"] = commit_result
    if commit_result.get("attempted") and not commit_result.get("ok"):
        result["ok"] = False
        result["reason"] = f"auto-commit failed: {commit_result.get('reason')}"
    elif commit_result.get("attempted") and commit_result.get("ok") and not after["ok"]:
        result["reason"] = (
            f"{after['reason']}; auto-commit reported ok but no commit landed"
            f" ({commit_result.get('reason')})"
        )
    return result


def run_self_tests() -> int:
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        repo = Path(tmp)
        git(repo, ["init"], check=True)
        git(repo, ["config", "user.name", "Test"], check=True)
        git(repo, ["config", "user.email", "test@example.com"], check=True)
        (repo / "src").mkdir()
        (repo / "session_plan").mkdir()
        (repo / "src/lib.rs").write_text("pub fn before() {}\n", encoding="utf-8")
        git(repo, ["add", "src/lib.rs"], check=True)
        git(repo, ["commit", "-m", "base"], check=True)
        base = git(repo, ["rev-parse", "HEAD"], check=True).stdout.strip()

        (repo / "src/lib.rs").write_text("pub fn after() {}\n", encoding="utf-8")
        unlanded = verify(repo, base, "task commit", auto=False)
        assert unlanded["ok"] is False
        assert unlanded["uncommitted_source_files"] == ["src/lib.rs"]

        landed = verify(repo, base, "task commit", auto=True)
        assert landed["ok"] is True
        assert landed["source_commit_shas"]
        assert landed["auto_commit"]["attempted"] is True

        (repo / "session_plan/eval.md").write_text("Verdict: PASS\n", encoding="utf-8")
        bookkeeping = verify(repo, git(repo, ["rev-parse", "HEAD"], check=True).stdout.strip(), "noop", auto=True)
        assert bookkeeping["ok"] is True
        assert bookkeeping["auto_commit"]["attempted"] is False

        # Edge case: file exists on disk and is tracked but has no changes.
        # auto_commit should report the file as present but nothing staged.
        staged_noop = auto_commit(repo, ["src/lib.rs"], "no-change")
        assert staged_noop["attempted"] is True
        assert staged_noop["ok"] is True
        assert staged_noop["committed"] is False
        assert staged_noop["file_exists"] == ["src/lib.rs"]
        assert staged_noop["file_missing"] == []
        assert "present on disk but nothing staged" in staged_noop["reason"]

        # Edge case: file does not exist on disk.
        phantom = auto_commit(repo, ["src/ghost.rs"], "phantom")
        assert phantom["attempted"] is True
        assert phantom["ok"] is False
        assert "git add failed" in phantom["reason"] or "did not match" in phantom.get("reason", "")

        # Edge case: verify with auto=True but file is already committed.
        # verify should report ok=True since source_files are landed.
        new_base = git(repo, ["rev-parse", "HEAD"], check=True).stdout.strip()
        already_landed = verify(repo, new_base, "noop", auto=True)
        assert already_landed["ok"] is True
        assert already_landed["auto_commit"]["attempted"] is False
    print("task_completion_gate self-tests passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--base", required=False, default="")
    parser.add_argument("--message", default="Verified task source changes")
    parser.add_argument("--auto-commit", action="store_true")
    parser.add_argument("--test", action="store_true")
    args = parser.parse_args()
    if args.test:
        return run_self_tests()
    if not args.base:
        parser.error("--base is required unless --test is used")
    json.dump(verify(args.repo_root, args.base, args.message, args.auto_commit), sys.stdout)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
