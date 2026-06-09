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
import task_completion_gate  # noqa: E402
import task_lineage  # noqa: E402
import task_verification_gate  # noqa: E402


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
    def test_evolve_records_lineage_to_stable_session_state_delta(self):
        evolve = Path(__file__).with_name("evolve.sh").read_text(encoding="utf-8")
        self.assertIn('SESSION_STAGING="${RUNNER_TEMP:-/tmp}/yoyo-session-staging-${STATE_SESSION_ID}-$$"', evolve)
        self.assertNotIn('SESSION_STAGING=".yoyo/session_staging"', evolve)
        self.assertIn('SESSION_STATE_EVENTS="$SESSION_STAGING/state/events.jsonl"', evolve)
        self.assertIn('STATE_APPEND_LOG="$SESSION_STAGING/state/append_state_event.log"', evolve)
        self.assertIn("append_state_event_checked()", evolve)
        self.assertIn("inline fallback failed", evolve)
        self.assertNotIn('${4:-{}}', evolve)
        self.assertNotIn('${2:-{}}', evolve)
        self.assertIn('--payload-file "$payload_file"', evolve)
        self.assertIn('payload=json.loads(pathlib.Path(payload_path).read_text', evolve)
        self.assertIn('append_state_event_checked "$STATE_EVENTS" "live"', evolve)
        self.assertIn('append_state_event_checked "$SESSION_STATE_EVENTS" "session"', evolve)
        self.assertIn("scripts/merge_state_delta.py", evolve)
        self.assertIn('--base-lines "$STATE_BASE_LINES"', evolve)
        self.assertIn("merge_state_delta.json", evolve)
        self.assertIn('--events "$SESSION_STATE_EVENTS"', evolve)
        self.assertIn('--link-commits \\\n    --events "$SESSION_STATE_EVENTS"', evolve)
        self.assertNotIn('tail -n +"$((STATE_BASE_LINES + 1))"', evolve)
        self.assertIn('TASK_EVIDENCE_DIR="$SESSION_STAGING/tasks/$TASK_ID"', evolve)
        self.assertIn('cp "$TASK_FILE" "$TASK_EVIDENCE_DIR/task.md"', evolve)
        self.assertIn('append_task_attempt_evidence()', evolve)
        self.assertIn('write_task_eval_evidence()', evolve)
        self.assertIn('run_agent_with_completion_watch()', evolve)
        self.assertIn('^Verdict:\\s*(PASS|FAIL)\\b', evolve)
        self.assertIn('write_task_outcome_evidence()', evolve)
        self.assertIn('scripts/task_manifest.py', evolve)
        self.assertIn('planning_failure.md', evolve)
        self.assertIn('Planning guard failed: planning agent produced 0 tasks', evolve)
        self.assertIn('Evaluator: timed out — failing task because no verifier verdict exists', evolve)
        self.assertIn('EVAL_VERDICT_TOKEN', evolve)
        self.assertIn('[[:punct:]]*', evolve)
        self.assertIn('[ "$EVAL_VERDICT_TOKEN" = "PASS" ]', evolve)
        self.assertIn('scripts/task_verification_gate.py', evolve)
        self.assertIn('scripts/task_completion_gate.py', evolve)
        self.assertIn('auto-committed verified source changes', evolve)
        self.assertIn('Task completion missing landed source commit', evolve)
        self.assertNotIn('Title: Self-improvement', evolve)
        self.assertNotIn('identify the most impactful improvement', evolve)
        self.assertIn('manifest.json', evolve)
        self.assertIn('attempts.jsonl', evolve)
        self.assertIn('eval_attempt_${attempt}.json', evolve)
        self.assertIn('outcome.json', evolve)

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

    def test_single_task_linkage_leaves_unplanned_source_commits_unassigned(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            subprocess.run(["git", "-C", str(repo), "init"], check=True, stdout=subprocess.DEVNULL)
            subprocess.run(["git", "-C", str(repo), "config", "user.name", "Test"], check=True)
            subprocess.run(["git", "-C", str(repo), "config", "user.email", "test@example.com"], check=True)
            (repo / "src").mkdir()
            (repo / "state").mkdir()
            (repo / "src/lib.rs").write_text("pub fn before() {}\n", encoding="utf-8")
            subprocess.run(["git", "-C", str(repo), "add", "src/lib.rs"], check=True)
            subprocess.run(["git", "-C", str(repo), "commit", "-m", "base"], check=True, stdout=subprocess.DEVNULL)
            base = subprocess.check_output(["git", "-C", str(repo), "rev-parse", "HEAD"], text=True).strip()

            (repo / "state/events.jsonl").write_text(
                json.dumps(
                    {
                        "event_type": "RunCompleted",
                        "payload": {
                            "phase": "task",
                            "task_id": "task_01",
                            "task_number": 1,
                            "task_title": "Finish wrap-up source",
                            "status": "completed",
                            "planned_files": ["src/context.rs"],
                            "source_files": [],
                            "commit_shas": [],
                        },
                    }
                )
                + "\n",
                encoding="utf-8",
            )
            (repo / "src/lib.rs").write_text("pub fn after() {}\n", encoding="utf-8")
            subprocess.run(["git", "-C", str(repo), "add", "src/lib.rs"], check=True)
            subprocess.run(
                ["git", "-C", str(repo), "commit", "-m", "Day 1 (00:00): session wrap-up"],
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
                    "events": repo / "state/events.jsonl",
                },
            )()
            payload = task_lineage.build_link_payload(args)

            self.assertEqual(payload["tasks"], [])
            self.assertEqual(len(payload["unassigned_source_commits"]), 1)
            self.assertEqual(payload["unassigned_source_commits"][0]["source_files"], ["src/lib.rs"])

    def test_task_verification_gate_requires_planned_file_overlap(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            subprocess.run(["git", "-C", str(repo), "init"], check=True, stdout=subprocess.DEVNULL)
            subprocess.run(["git", "-C", str(repo), "config", "user.name", "Test"], check=True)
            subprocess.run(["git", "-C", str(repo), "config", "user.email", "test@example.com"], check=True)
            (repo / "src").mkdir()
            (repo / "docs").mkdir()
            (repo / "session_plan").mkdir()
            (repo / "src/lib.rs").write_text("pub fn before() {}\n", encoding="utf-8")
            (repo / "docs/readme.md").write_text("before\n", encoding="utf-8")
            subprocess.run(["git", "-C", str(repo), "add", "."], check=True)
            subprocess.run(["git", "-C", str(repo), "commit", "-m", "base"], check=True, stdout=subprocess.DEVNULL)
            base = subprocess.check_output(["git", "-C", str(repo), "rev-parse", "HEAD"], text=True).strip()

            task = repo / "session_plan/task_01.md"
            task.write_text("Title: Docs\nFiles: docs/readme.md\nIssue: none\n", encoding="utf-8")
            (repo / "docs/readme.md").write_text("after\n", encoding="utf-8")
            ok = task_verification_gate.verify(repo, base, task)
            self.assertTrue(ok["ok"])
            self.assertEqual(ok["overlapping_files"], ["docs/readme.md"])

            task.write_text("Title: Wrong\nFiles: src/lib.rs\nIssue: none\n", encoding="utf-8")
            bad = task_verification_gate.verify(repo, base, task)
            self.assertFalse(bad["ok"])
            self.assertEqual(bad["reason"], "task changes do not overlap planned Files entries")

    def test_task_completion_gate_auto_commits_verified_source_changes(self):
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
            base = subprocess.check_output(["git", "-C", str(repo), "rev-parse", "HEAD"], text=True).strip()

            (repo / "src/lib.rs").write_text("pub fn after() {}\n", encoding="utf-8")
            unlanded = task_completion_gate.verify(repo, base, "Task commit", auto=False)
            self.assertFalse(unlanded["ok"])
            self.assertEqual(unlanded["uncommitted_source_files"], ["src/lib.rs"])

            landed = task_completion_gate.verify(repo, base, "Task commit", auto=True)
            self.assertTrue(landed["ok"])
            self.assertTrue(landed["source_commit_shas"])
            self.assertTrue(landed["auto_commit"]["attempted"])

            (repo / "session_plan/eval.md").write_text("Verdict: PASS\n", encoding="utf-8")
            bookkeeping = task_completion_gate.verify(
                repo,
                subprocess.check_output(["git", "-C", str(repo), "rev-parse", "HEAD"], text=True).strip(),
                "Noop",
                auto=True,
            )
            self.assertTrue(bookkeeping["ok"])
            self.assertFalse(bookkeeping["auto_commit"]["attempted"])

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
            self.assertIn("task_manifest_available", task["gnome_metrics"])
            self.assertIn("task_artifact_coverage", task["gnome_metrics"])
            self.assertIn("state_replay_integrity_rate", task["gnome_metrics"])
            self.assertGreater(task["gnome_deltas"]["coding_log_score"], 0)
            self.assertIn("task_manifest_available", summary["latest_gnomes"])
            self.assertIn("task_artifact_coverage", summary["latest_gnomes"])
            self.assertIn("state_replay_integrity_rate", summary["latest_gnomes"])

    def test_log_feedback_session_success_uses_strict_verified_tasks(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
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
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {"task_count": 1, "selected_task_count": 1},
                    "selected_tasks": [{"task_id": "task_01"}],
                },
            )
            write_json(
                session / "tasks/task_01/outcome.json",
                {"task_id": "task_01", "status": "completed"},
            )

            assessment = log_feedback.build_assessment(
                session_dir=session,
                log_available=True,
                log_error="",
                log_text="Build: PASS\nTests: PASS\n",
                repo="owner/repo",
                run_id="123",
                run_attempt="1",
                workflow_conclusion="success",
            )

            metrics = assessment["metrics"]
            self.assertEqual(metrics["task_success_rate"], 0.0)
            self.assertEqual(metrics["session_success_rate"], 0.0)
            self.assertEqual(metrics["evaluator_unverified_count"], 1)
            self.assertEqual(metrics["task_unlanded_source_count"], 0)

    def test_log_feedback_requires_landed_commit_for_passed_source_task(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
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
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {"task_count": 1, "selected_task_count": 1},
                    "selected_tasks": [{"task_id": "task_01"}],
                },
            )
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "status": "completed",
                    "source_files": ["src/state.rs"],
                    "commit_shas": [],
                },
            )
            write_json(
                session / "tasks/task_01/eval_attempt_1.json",
                {"task_id": "task_01", "status": "pass", "verdict": "Verdict: PASS"},
            )

            assessment = log_feedback.build_assessment(
                session_dir=session,
                log_available=True,
                log_error="",
                log_text="Build: PASS\nTests: PASS\n",
                repo="owner/repo",
                run_id="123",
                run_attempt="1",
                workflow_conclusion="success",
            )

            metrics = assessment["metrics"]
            self.assertEqual(metrics["task_success_rate"], 0.0)
            self.assertEqual(metrics["session_success_rate"], 0.0)
            self.assertEqual(metrics["evaluator_unverified_count"], 1)
            self.assertEqual(metrics["task_unlanded_source_count"], 1)

    def test_log_feedback_uses_state_cache_metric_events(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "tasks_attempted": 0,
                    "tasks_succeeded": 0,
                    "build_ok": True,
                    "test_ok": True,
                    "reverted": False,
                },
            )
            append_event(
                session / "state/events.jsonl",
                "CacheMetricsRecorded",
                {
                    "model": "deepseek-v4-pro",
                    "prompt_cache_hit_tokens": 80,
                    "prompt_cache_miss_tokens": 20,
                    "cache_hit_ratio": 0.8,
                },
            )
            append_event(
                session / "state/events.jsonl",
                "CacheMetricsRecorded",
                {
                    "model": "deepseek-v4-pro",
                    "prompt_cache_hit_tokens": 20,
                    "prompt_cache_miss_tokens": 0,
                    "cache_hit_ratio": 1.0,
                },
            )

            assessment = log_feedback.build_assessment(
                session_dir=session,
                log_available=True,
                log_error="",
                log_text="Build: PASS\nTests: PASS\n",
                repo="owner/repo",
                run_id="123",
                run_attempt="1",
                workflow_conclusion="success",
            )

            metrics = assessment["metrics"]
            self.assertEqual(metrics["deepseek_cache_hit_tokens"], 100)
            self.assertEqual(metrics["deepseek_cache_miss_tokens"], 20)
            self.assertAlmostEqual(metrics["deepseek_cache_hit_ratio"], 100 / 120, places=6)
            self.assertEqual(metrics["deepseek_cache_metric_source"], "state")
            self.assertEqual(metrics["deepseek_cache_metric_event_count"], 2)

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
