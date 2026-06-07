#!/usr/bin/env python3
"""Tests for task lineage state feedback plumbing."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import log_feedback  # noqa: E402
import summarize_state_gnomes  # noqa: E402
import task_lineage  # noqa: E402


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def append_event(path: Path, event_type: str, payload: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    event = {
        "event_id": f"evt-{event_type}-{len(path.read_text(encoding='utf-8').splitlines()) if path.exists() else 0}",
        "event_type": event_type,
        "payload": payload,
    }
    with path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(event, separators=(",", ":")) + "\n")


class TaskLineageFeedback(unittest.TestCase):
    def test_task_lineage_payload_captures_source_commits(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            subprocess.run(["git", "-C", str(repo), "init"], check=True, stdout=subprocess.DEVNULL)
            subprocess.run(["git", "-C", str(repo), "config", "user.name", "Test"], check=True)
            subprocess.run(["git", "-C", str(repo), "config", "user.email", "test@example.com"], check=True)
            (repo / "src").mkdir()
            (repo / "src/lib.rs").write_text("pub fn before() {}\n", encoding="utf-8")
            subprocess.run(["git", "-C", str(repo), "add", "src/lib.rs"], check=True)
            subprocess.run(["git", "-C", str(repo), "commit", "-m", "base"], check=True, stdout=subprocess.DEVNULL)
            base = subprocess.check_output(["git", "-C", str(repo), "rev-parse", "HEAD"], text=True).strip()

            (repo / "session_plan").mkdir()
            task_file = repo / "session_plan/task_01.md"
            task_file.write_text("Title: Add lineage\nFiles: src/lib.rs\nIssue: none\n", encoding="utf-8")
            eval_file = repo / "session_plan/eval_task_1.md"
            eval_file.write_text("Verdict: PASS\nReason: works\n", encoding="utf-8")
            (repo / "src/lib.rs").write_text("pub fn after() {}\n", encoding="utf-8")
            subprocess.run(["git", "-C", str(repo), "add", "src/lib.rs"], check=True)
            subprocess.run(
                ["git", "-C", str(repo), "commit", "-m", "Day 1 (00:00): Add lineage (Task 1)"],
                check=True,
                stdout=subprocess.DEVNULL,
            )

            args = type(
                "Args",
                (),
                {
                    "repo_root": repo,
                    "base": base,
                    "head": "",
                    "task_number": 1,
                    "task_title": "Add lineage",
                    "status": "completed",
                    "task_file": task_file,
                    "eval_file": eval_file,
                    "reason": "",
                },
            )()
            payload = task_lineage.build_payload(args)

            self.assertEqual(payload["task_id"], "task_01")
            self.assertEqual(payload["source_files"], ["src/lib.rs"])
            self.assertEqual(len(payload["commit_shas"]), 1)
            self.assertEqual(payload["eval"], {"verdict": "PASS", "reason": "works"})

    def test_log_feedback_links_gnome_deltas_to_tasks(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            previous = root / "sessions/day-0"
            events_path = session / "state/events.jsonl"
            write_json(
                previous / "log_feedback.json",
                {"metrics": {"coding_log_score": 0.4, "task_success_rate": 0.5}},
            )
            write_json(
                session / "outcome.json",
                {
                    "tasks_attempted": 1,
                    "tasks_succeeded": 1,
                    "build_ok": True,
                    "test_ok": True,
                    "reverted": False,
                },
            )
            append_event(
                events_path,
                "RunStarted",
                {
                    "phase": "task",
                    "task_id": "task_01",
                    "task_number": 1,
                    "task_title": "Improve state",
                    "planned_files": ["src/state.rs"],
                },
            )
            append_event(
                events_path,
                "RunCompleted",
                {
                    "phase": "task",
                    "task_id": "task_01",
                    "task_number": 1,
                    "task_title": "Improve state",
                    "status": "completed",
                    "source_files": ["src/state.rs"],
                    "commit_shas": ["abc"],
                    "eval": {"verdict": "PASS", "reason": "ok"},
                },
            )

            assessment = log_feedback.build_assessment(
                session_dir=session,
                log_available=True,
                log_error="",
                log_text="all good",
                repo="owner/repo",
                run_id="123",
                run_attempt="1",
                workflow_conclusion="success",
            )
            log_feedback.write_assessment(session, assessment, append_state=True)
            summary = summarize_state_gnomes.summarize(
                summarize_state_gnomes.load_jsonl(events_path),
                events_path,
            )

            task = summary["task_lineage"][0]
            self.assertEqual(task["task_id"], "task_01")
            self.assertEqual(task["source_files"], ["src/state.rs"])
            self.assertEqual(task["eval"]["verdict"], "PASS")
            self.assertIn("coding_log_score", task["gnome_metrics"])
            self.assertGreater(task["gnome_deltas"]["coding_log_score"], 0)

    def test_summary_merges_post_wrapup_commit_links(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            events_path = root / "events.jsonl"
            append_event(
                events_path,
                "RunCompleted",
                {
                    "phase": "task",
                    "task_id": "task_01",
                    "task_number": 1,
                    "task_title": "Finish later",
                    "status": "completed",
                    "source_files": ["src/context.rs"],
                    "commit_shas": [],
                },
            )
            append_event(
                events_path,
                "TaskLineageLinked",
                {
                    "phase": "task_commit_linkage",
                    "decision_type": "task_commit_linkage",
                    "tasks": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "task_title": "Finish later",
                            "linked_by": "source_file_overlap",
                            "linked_commit_shas": ["sha-wrap"],
                            "linked_commits": [
                                {
                                    "sha": "sha-wrap",
                                    "short_sha": "sha-wra",
                                    "subject": "Day 1 (00:00): session wrap-up",
                                    "source_files": ["src/context.rs"],
                                }
                            ],
                        }
                    ],
                },
            )

            summary = summarize_state_gnomes.summarize(
                summarize_state_gnomes.load_jsonl(events_path),
                events_path,
            )

            task = summary["task_lineage"][0]
            self.assertEqual(task["commit_shas"], ["sha-wrap"])
            self.assertEqual(task["commit_linkage_method"], "source_file_overlap")
            self.assertEqual(task["commits"][0]["subject"], "Day 1 (00:00): session wrap-up")


if __name__ == "__main__":
    unittest.main()
