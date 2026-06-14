#!/usr/bin/env python3
"""Tests for trajectory extraction evidence summaries."""

from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
from unittest import mock
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import extract_trajectory  # noqa: E402


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def write_events(path: Path, rows: list[dict[str, object]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, separators=(",", ":")) + "\n")


class ExtractTrajectoryTests(unittest.TestCase):
    def test_run_cmd_timeout_without_pid_degrades(self) -> None:
        with mock.patch.object(
            extract_trajectory.subprocess,
            "run",
            side_effect=extract_trajectory.subprocess.TimeoutExpired(["gh", "run", "list"], 1),
        ):
            rc, stdout, stderr = extract_trajectory.run_cmd(["gh", "run", "list"], timeout=1)

        self.assertEqual(rc, 124)
        self.assertEqual(stdout, "")
        self.assertEqual(stderr, "timeout")

    def test_drop_dangling_trailing_section_header(self) -> None:
        rendered = extract_trajectory.drop_dangling_trailing_section_header(
            "# YOUR TRAJECTORY\n\n## Graph-derived next-task pressure\n- keep me\n\n## Structured state snapshot"
        )

        self.assertIn("## Graph-derived next-task pressure", rendered)
        self.assertNotIn("## Structured state snapshot", rendered)

    def test_main_keeps_graph_pressure_before_truncated_snapshot(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "trajectory.md"
            env = {
                "YOYO_AUDIT_DIR": tmp,
                "YOYO_REPO": "owner/repo",
                "YOYO_DAY": "106",
                "YOYO_TRAJECTORY_OUT": str(out),
            }
            graph = (
                "## Graph-derived next-task pressure\n"
                "- Close yyds state and model lifecycle gaps "
                "(deepseek_model_call_incomplete_count=1): current graph pressure."
            )
            oversized_snapshot = (
                "## Structured state snapshot\n"
                + "\n".join(f"claims detail {index}: " + ("x" * 80) for index in range(20))
            )
            with mock.patch.dict(os.environ, env, clear=False), \
                mock.patch.object(extract_trajectory, "TOTAL_BYTE_CAP", 520), \
                mock.patch.object(extract_trajectory, "load_recent_session_outcomes", return_value=[]), \
                mock.patch.object(extract_trajectory, "collect_task_commits", return_value=([], 0)), \
                mock.patch.object(extract_trajectory, "collect_provider_errors", return_value=(0, 0)), \
                mock.patch.object(extract_trajectory, "collect_failed_ci_fingerprints", return_value=[]), \
                mock.patch.object(extract_trajectory, "load_log_feedback", return_value=[]), \
                mock.patch.object(extract_trajectory, "load_corrected_log_feedback_lessons", return_value=[]), \
                mock.patch.object(extract_trajectory, "render_graph_suggestions", return_value=graph), \
                mock.patch.object(extract_trajectory, "render_structured_state_snapshot", return_value=oversized_snapshot):

                self.assertEqual(extract_trajectory.main(), 0)

            rendered = out.read_text(encoding="utf-8")
            self.assertIn("## Graph-derived next-task pressure", rendered)
            self.assertIn("deepseek_model_call_incomplete_count=1", rendered)
            self.assertNotIn("## Structured state snapshot\n... (truncated", rendered)

    def test_main_keeps_structured_state_before_verbose_log_feedback(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "trajectory.md"
            env = {
                "YOYO_AUDIT_DIR": tmp,
                "YOYO_REPO": "owner/repo",
                "YOYO_DAY": "106",
                "YOYO_TRAJECTORY_OUT": str(out),
            }
            graph = (
                "## Graph-derived next-task pressure\n"
                "- Close yyds state and model lifecycle gaps "
                "(deepseek_model_call_incomplete_count=1): current graph pressure."
            )
            structured = (
                "## Structured state snapshot\n"
                "claims: 7/9 proven; lifecycle causes: model_incomplete/open_after_command=1"
            )
            verbose_feedback = (
                "## GitHub Actions log feedback\n"
                "latest score=0.9 confidence=1.0\n"
                + "\n".join(f"- repeated historical detail {index}: " + ("x" * 60) for index in range(12))
            )
            with mock.patch.dict(os.environ, env, clear=False), \
                mock.patch.object(extract_trajectory, "TOTAL_BYTE_CAP", 760), \
                mock.patch.object(extract_trajectory, "load_recent_session_outcomes", return_value=[]), \
                mock.patch.object(extract_trajectory, "collect_task_commits", return_value=([], 0)), \
                mock.patch.object(extract_trajectory, "collect_provider_errors", return_value=(0, 0)), \
                mock.patch.object(extract_trajectory, "collect_failed_ci_fingerprints", return_value=[]), \
                mock.patch.object(extract_trajectory, "load_log_feedback", return_value=[{"metrics": {}}]), \
                mock.patch.object(extract_trajectory, "load_corrected_log_feedback_lessons", return_value=[]), \
                mock.patch.object(extract_trajectory, "render_graph_suggestions", return_value=graph), \
                mock.patch.object(extract_trajectory, "render_structured_state_snapshot", return_value=structured), \
                mock.patch.object(extract_trajectory, "render_log_feedback", return_value=verbose_feedback):

                self.assertEqual(extract_trajectory.main(), 0)

            rendered = out.read_text(encoding="utf-8")
            self.assertIn("## Graph-derived next-task pressure", rendered)
            self.assertIn("## Structured state snapshot", rendered)
            self.assertIn("model_incomplete/open_after_command=1", rendered)
            self.assertLess(
                rendered.index("## Structured state snapshot"),
                rendered.index("## GitHub Actions log feedback"),
            )

    def test_main_keeps_corrected_log_feedback_before_truncated_snapshot(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "trajectory.md"
            env = {
                "YOYO_AUDIT_DIR": tmp,
                "YOYO_REPO": "owner/repo",
                "YOYO_DAY": "106",
                "YOYO_TRAJECTORY_OUT": str(out),
            }
            graph = (
                "## Graph-derived next-task pressure\n"
                "- Close yyds state and model lifecycle gaps "
                "(deepseek_model_call_incomplete_count=1): current graph pressure."
            )
            corrected_feedback = (
                "## GitHub Actions log feedback\n"
                "latest score=0.9 confidence=1.0 recurring_failures=1 state_capture=1.0\n"
                "Corrected top lessons for next run:\n"
                "- DeepSeek model call lifecycle was incomplete: "
                "model_incomplete/open_after_command=1 -> close model-call lifecycle events"
            )
            oversized_snapshot = (
                "## Structured state snapshot\n"
                + "\n".join(f"claims detail {index}: " + ("x" * 80) for index in range(18))
            )
            with mock.patch.dict(os.environ, env, clear=False), \
                mock.patch.object(extract_trajectory, "TOTAL_BYTE_CAP", 840), \
                mock.patch.object(extract_trajectory, "load_recent_session_outcomes", return_value=[]), \
                mock.patch.object(extract_trajectory, "collect_task_commits", return_value=([], 0)), \
                mock.patch.object(
                    extract_trajectory,
                    "render_task_success",
                    return_value="## Per-task activity (last 14 days)\n" + "\n".join(
                        f"low priority task detail {index}: " + ("x" * 60)
                        for index in range(6)
                    ),
                ), \
                mock.patch.object(extract_trajectory, "collect_provider_errors", return_value=(0, 0)), \
                mock.patch.object(extract_trajectory, "collect_failed_ci_fingerprints", return_value=[]), \
                mock.patch.object(extract_trajectory, "load_log_feedback", return_value=[{"metrics": {}}]), \
                mock.patch.object(
                    extract_trajectory,
                    "load_corrected_log_feedback_lessons",
                    return_value=[
                        {
                            "fingerprint": "DeepSeek model call lifecycle was incomplete: model_incomplete/open_after_command=1",
                            "action": "close model-call lifecycle events",
                        }
                    ],
                ), \
                mock.patch.object(extract_trajectory, "render_graph_suggestions", return_value=graph), \
                mock.patch.object(extract_trajectory, "render_structured_state_snapshot", return_value=oversized_snapshot), \
                mock.patch.object(extract_trajectory, "render_log_feedback", return_value=corrected_feedback):

                self.assertEqual(extract_trajectory.main(), 0)

            rendered = out.read_text(encoding="utf-8")
            self.assertIn("## GitHub Actions log feedback", rendered)
            self.assertIn("model_incomplete/open_after_command=1", rendered)
            self.assertIn("## Structured state snapshot", rendered)
            self.assertLess(
                rendered.index("## GitHub Actions log feedback"),
                rendered.index("## Structured state snapshot"),
            )
            self.assertNotIn("## Per-task activity", rendered)

    def test_recent_outcomes_sort_by_outcome_timestamp_not_file_mtime(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            older = audit_dir / "day-1"
            newer = audit_dir / "day-2"
            write_json(
                older / "outcome.json",
                {"day": 1, "ts": "2026-01-01T00:00:00Z", "tasks_attempted": 0},
            )
            write_json(
                newer / "outcome.json",
                {"day": 2, "ts": "2026-01-02T00:00:00Z", "tasks_attempted": 0},
            )
            os.utime(newer / "outcome.json", (1, 1))
            os.utime(older / "outcome.json", (2, 2))

            sessions = extract_trajectory.load_recent_session_outcomes(audit_dir)

            self.assertEqual(sessions[0][0].name, "day-2")
            self.assertEqual(sessions[1][0].name, "day-1")

    def test_log_feedback_uses_session_timestamp_for_latest_feedback(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            older = audit_dir / "day-1"
            newer = audit_dir / "day-2"
            write_json(older / "outcome.json", {"ts": "2026-01-01T00:00:00Z"})
            write_json(newer / "outcome.json", {"ts": "2026-01-02T00:00:00Z"})
            write_json(older / "log_feedback.json", {"metrics": {"coding_log_score": 0.1}})
            write_json(newer / "log_feedback.json", {"metrics": {"coding_log_score": 0.9}})
            os.utime(newer / "log_feedback.json", (1, 1))
            os.utime(older / "log_feedback.json", (2, 2))

            feedbacks = extract_trajectory.load_log_feedback(audit_dir)

            self.assertEqual(feedbacks[0]["metrics"]["coding_log_score"], 0.9)

    def test_log_feedback_suppresses_resolved_seed_replacement(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(session / "outcome.json", {"ts": "2026-01-01T00:00:00Z"})
            write_json(
                session / "log_feedback.json",
                {
                    "metrics": {
                        "coding_log_score": 0.9,
                        "selected_task_count": 1,
                        "task_strict_verified_count": 1,
                        "tasks_succeeded": 1,
                        "task_seed_contradiction_count": 1,
                        "task_manifest_seed_contradiction_count": 0,
                        "task_revert_count": 0,
                        "task_obsolete_count": 0,
                    },
                    "top_lessons": [
                        {
                            "kind": "task_seed_contradiction",
                            "fingerprint": "seeded task was contradicted by fresh assessment evidence",
                            "action": "replace stale seed",
                        },
                        {
                            "kind": "high_task_turn_count",
                            "fingerprint": "max task turn count is high",
                            "action": "split tasks earlier",
                        },
                    ],
                },
            )

            feedbacks = extract_trajectory.load_log_feedback(audit_dir)

            metrics = feedbacks[0]["metrics"]
            self.assertEqual(metrics["task_seed_contradiction_count"], 0)
            self.assertEqual(metrics["task_seed_replacement_count"], 1)
            self.assertEqual(
                [lesson["kind"] for lesson in feedbacks[0]["top_lessons"]],
                ["high_task_turn_count"],
            )

    def test_log_feedback_prefers_corrected_lessons(self) -> None:
        rendered = extract_trajectory.render_log_feedback(
            [
                {
                    "metrics": {
                        "coding_log_score": 0.9,
                        "coding_log_confidence": 1.0,
                        "recurring_failure_count": 1,
                        "state_capture_coverage": 1.0,
                        "failure_fingerprints": [
                            {"fingerprint": "fatal: no pattern given"},
                        ],
                    },
                    "top_lessons": [
                        {
                            "fingerprint": "seeded task was contradicted by fresh assessment evidence",
                            "action": "replace stale seed",
                        }
                    ],
                },
                {
                    "metrics": {
                        "failure_fingerprints": [
                            {"fingerprint": "fatal: no pattern given"},
                        ],
                    },
                },
            ],
            [
                {
                    "fingerprint": "DeepSeek model call lifecycle was incomplete: model_incomplete/open_after_command=1",
                    "action": "close model-call lifecycle events",
                }
            ],
        )

        self.assertIn("Corrected top lessons for next run:", rendered)
        self.assertIn("DeepSeek model call lifecycle was incomplete", rendered)
        self.assertIn("model_incomplete/open_after_command=1", rendered)
        self.assertIn(
            "Historical repeated across prior log feedback (context only; corrected lessons are current pressure):",
            rendered,
        )
        self.assertNotIn("seeded task was contradicted", rendered)

    def test_raw_success_without_task_artifacts_is_not_rendered_as_verified(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-01-01T00:00:00Z",
                    "tasks_attempted": 3,
                    "tasks_succeeded": 3,
                    "build_ok": True,
                    "test_ok": True,
                    "reverted": False,
                },
            )

            rendered = extract_trajectory.render_outcomes(
                extract_trajectory.load_recent_session_outcomes(audit_dir)
            )

            self.assertIn("tasks 3/3", rendered)
            self.assertIn("raw outcome 3/3 lacks strict task evidence", rendered)
            self.assertNotIn("tasks 3/3 ✅", rendered)

    def test_recent_outcomes_display_is_capped_with_omission_note(self) -> None:
        outcomes = [
            {
                "day": index,
                "ts": f"2026-01-{index:02d}T00:00:00Z",
                "tasks_attempted": 0,
                "tasks_succeeded": 0,
                "build_ok": True,
                "test_ok": True,
                "reverted": False,
            }
            for index in range(1, 9)
        ]

        rendered = extract_trajectory.render_outcomes(outcomes)

        self.assertIn("## Recent session outcomes (newest 6 of 8)", rendered)
        self.assertIn("day-1", rendered)
        self.assertIn("day-6", rendered)
        self.assertNotIn("day-7", rendered)
        self.assertIn("... 2 older session outcome(s) omitted", rendered)

    def test_strict_verified_task_artifacts_are_rendered_as_verified(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-01-01T00:00:00Z",
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
                    "tasks": [
                        {
                            "task_id": "task_01",
                            "title": "Tighten trajectory evidence",
                            "files": ["src/lib.rs"],
                        }
                    ]
                },
            )
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "status": "completed",
                    "planned_files": ["src/lib.rs"],
                    "source_files": ["src/lib.rs"],
                    "commit_shas": ["abc123"],
                },
            )
            write_json(
                session / "tasks/task_01/eval_attempt_1.json",
                {"task_id": "task_01", "status": "pass", "verdict": "Verdict: PASS"},
            )

            rendered = extract_trajectory.render_outcomes(
                extract_trajectory.load_recent_session_outcomes(audit_dir)
            )

            self.assertIn("tasks 1/1 ✅", rendered)
            self.assertIn("1/1 strict verified; build OK, tests OK", rendered)

    def test_unverified_task_artifacts_show_strict_failure_reasons(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-01-01T00:00:00Z",
                    "tasks_attempted": 1,
                    "tasks_succeeded": 1,
                    "build_ok": True,
                    "test_ok": True,
                    "reverted": False,
                },
            )
            write_json(
                session / "tasks/manifest.json",
                {"tasks": [{"task_id": "task_01", "files": ["src/lib.rs"]}]},
            )
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "status": "completed",
                    "planned_files": ["src/lib.rs"],
                    "source_files": ["docs/readme.md"],
                    "commit_shas": [],
                },
            )

            rendered = extract_trajectory.render_outcomes(
                extract_trajectory.load_recent_session_outcomes(audit_dir)
            )

            self.assertIn("tasks 1/1 ⚠️", rendered)
            self.assertIn("0/1 strict verified", rendered)
            self.assertIn("raw outcome 1/1", rendered)
            self.assertIn("task states: unlanded_source_edits=1", rendered)
            self.assertNotIn("no planned-file overlap", rendered)
            self.assertNotIn("no passing verifier", rendered)

    def test_recent_outcomes_surface_classified_seed_contradiction_state(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-01-01T00:00:00Z",
                    "tasks_attempted": 1,
                    "tasks_succeeded": 0,
                    "build_ok": True,
                    "test_ok": True,
                    "reverted": False,
                },
            )
            write_json(
                session / "log_feedback.json",
                {
                    "metrics": {
                        "failure_fingerprints": [
                            {
                                "fingerprint": (
                                    "The seed task_01.md has a factual error: "
                                    "the assessment clearly shows the opposite."
                                ),
                                "count": 1,
                            }
                        ],
                    }
                },
            )
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {"planning_failed": False, "task_count": 1, "selected_task_count": 1},
                    "selected_tasks": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "title": "Stale seeded task",
                            "origin": "harness-seed",
                            "files": ["src/lib.rs"],
                            "artifact_path": "tasks/task_01/task.md",
                        }
                    ],
                    "artifacts": {"manifest": "tasks/manifest.json"},
                },
            )
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "status": "reverted",
                    "revert_reason": "Task scope mismatch: task produced no git-visible file changes",
                    "planned_files": ["src/lib.rs"],
                    "source_files": [],
                    "touched_files": [],
                    "commit_shas": [],
                },
            )

            rendered = extract_trajectory.render_outcomes(
                extract_trajectory.load_recent_session_outcomes(audit_dir)
            )

            self.assertIn("task states: reverted_seed_contradicted=1", rendered)
            self.assertNotIn("no touched files", rendered)
            self.assertNotIn("no passing verifier", rendered)

    def test_structured_state_snapshot_surfaces_claims_task_states_and_tool_failures(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-01-01T00:00:00Z",
                    "tasks_attempted": 1,
                    "tasks_succeeded": 0,
                    "build_ok": True,
                    "test_ok": True,
                    "reverted": False,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {"tool_error_count": 1},
                    "gnome_keys": ["tool_error_count"],
                },
            )
            write_json(
                session / "tasks/manifest.json",
                {"tasks": [{"task_id": "task_01", "title": "Fix search", "files": ["src/lib.rs"]}]},
            )
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "status": "reverted",
                    "planned_files": ["src/lib.rs"],
                    "touched_files": ["src/lib.rs"],
                    "source_files": ["src/lib.rs"],
                    "commit_shas": [],
                },
            )
            transcript_dir = session / "transcripts"
            transcript_dir.mkdir(parents=True)
            transcript_dir.joinpath("task_01_attempt1.log").write_text(
                "  ▶ search 'fn handle_run\\(' in src/commands.rs ✗ (17ms)\n",
                encoding="utf-8",
            )
            events = [
                {"kind": "RunStarted", "run_id": "run-open", "payload": {"status": "started"}},
                {
                    "kind": "ModelCallStarted",
                    "run_id": "run-open",
                    "payload": {"model": "deepseek-v4-pro"},
                },
                {
                    "kind": "FileEdited",
                    "run_id": "run-open",
                    "payload": {"path": "/work/repo/src/lib.rs"},
                },
            ]
            state_dir = session / "state"
            state_dir.mkdir(parents=True, exist_ok=True)
            state_dir.joinpath("events.jsonl").write_text(
                "\n".join(json.dumps(event) for event in events) + "\n",
                encoding="utf-8",
            )

            rendered = extract_trajectory.render_structured_state_snapshot(audit_dir)

            self.assertIn("## Structured state snapshot", rendered)
            self.assertIn("claims:", rendered)
            self.assertIn("non-proven", rendered)
            self.assertIn("recent non-proven claims:", rendered)
            self.assertIn("model_lifecycle=1 missing", rendered)
            self.assertIn("run_lifecycle=1 missing", rendered)
            self.assertIn("lifecycle gaps:", rendered)
            self.assertIn("\n- lifecycle gaps:", rendered)
            self.assertIn("state_incomplete=1", rendered)
            self.assertIn("model_incomplete=1", rendered)
            self.assertIn("lifecycle causes:", rendered)
            self.assertIn("model_incomplete/open_after_file_edit=1", rendered)
            self.assertIn("state_incomplete/open_after_file_edit=1", rendered)
            self.assertIn("lifecycle aggregate:", rendered)
            self.assertIn("observed=1/1", rendered)
            self.assertIn("unhealthy=1", rendered)
            self.assertIn("run_incomplete=1", rendered)
            self.assertIn("recent task issues:", rendered)
            self.assertIn("deepseek_model_call_lifecycle_balanced", rendered)
            self.assertIn("latest=day-1", rendered)
            self.assertIn("unlanded_source_edits=1", rendered)
            self.assertIn("recent tool failures:", rendered)
            self.assertIn("\n- recent tool failures:", rendered)
            self.assertIn("unrecovered=1/1", rendered)
            self.assertIn("failed_commands=1", rendered)
            self.assertIn("recent action evidence:", rendered)
            self.assertIn("\n- recent action evidence:", rendered)
            self.assertIn("transcript_only_failed_tools=1", rendered)
            self.assertIn("search_tool_error=1", rendered)
            self.assertIn("lifecycle gnomes:", rendered)
            self.assertIn("state_run_incomplete_count=1", rendered)
            self.assertIn("deepseek_model_call_incomplete_count=1", rendered)

    def test_structured_state_snapshot_uses_recent_task_issues_not_global_history(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            for index in range(6):
                session = audit_dir / f"day-{index + 1}"
                write_json(
                    session / "outcome.json",
                    {
                        "day": index + 1,
                        "ts": f"2026-01-0{index + 1}T00:00:00Z",
                        "tasks_attempted": 1,
                        "tasks_succeeded": 1 if index > 0 else 0,
                        "build_ok": index > 0,
                        "test_ok": index > 0,
                        "reverted": index == 0,
                    },
                )
                write_json(
                    session / "tasks/manifest.json",
                    {
                        "planner": {
                            "planning_failed": False,
                            "task_count": 1,
                            "selected_task_count": 1,
                        },
                        "selected_tasks": [
                            {
                                "task_id": "task_01",
                                "task_number": 1,
                                "title": "Keep task pressure recent",
                                "origin": "planner",
                                "files": ["src/lib.rs"],
                                "artifact_path": "tasks/task_01/task.md",
                            }
                        ],
                        "warnings": [],
                        "artifacts": {"manifest": "tasks/manifest.json"},
                    },
                )
                (session / "tasks/task_01").mkdir(parents=True, exist_ok=True)
                (session / "tasks/task_01/task.md").write_text(
                    "Title: Keep task pressure recent\n",
                    encoding="utf-8",
                )
                if index == 0:
                    transcript_dir = session / "transcripts"
                    transcript_dir.mkdir(parents=True)
                    transcript_dir.joinpath("task_01_attempt1.log").write_text(
                        "  ▶ search 'fn old\\(' in src/lib.rs ✗ (17ms)\n",
                        encoding="utf-8",
                    )
                    write_json(
                        session / "tasks/task_01/outcome.json",
                        {
                            "task_id": "task_01",
                            "status": "reverted",
                            "planned_files": ["src/lib.rs"],
                            "touched_files": ["scripts/other.py"],
                            "source_files": ["scripts/other.py"],
                            "commit_shas": ["old123"],
                        },
                    )
                    write_json(
                        session / "tasks/task_01/eval_attempt_1.json",
                        {"task_id": "task_01", "status": "fail", "verdict": "Verdict: FAIL"},
                    )
                else:
                    write_json(
                        session / "tasks/task_01/outcome.json",
                        {
                            "task_id": "task_01",
                            "status": "completed",
                            "planned_files": ["src/lib.rs"],
                            "touched_files": ["src/lib.rs"],
                            "source_files": ["src/lib.rs"],
                            "commit_shas": [f"abc{index}"],
                        },
                    )
                    write_json(
                        session / "tasks/task_01/eval_attempt_1.json",
                        {"task_id": "task_01", "status": "pass", "verdict": "Verdict: PASS"},
                    )

            rendered = extract_trajectory.render_structured_state_snapshot(audit_dir)
            first_line = next(line for line in rendered.splitlines() if line.startswith("claims:"))

            self.assertNotIn("top task states:", first_line)
            self.assertNotIn("recent task issues:", first_line)
            self.assertNotIn("recent tool failures:", first_line)
            self.assertIn("historical unrecovered tool failures:", rendered)
            self.assertNotIn("reverted_scope_mismatch", rendered)

    def test_structured_state_snapshot_surfaces_dashboard_dataset_warnings(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-01-01T00:00:00Z",
                    "tasks_attempted": 1,
                    "tasks_succeeded": 1,
                    "build_ok": True,
                    "test_ok": True,
                },
            )

            with unittest.mock.patch(
                "build_evolution_dashboard.dashboard_dataset_warnings",
                return_value=["session work summaries lack current task-state evidence."],
            ):
                rendered = extract_trajectory.render_structured_state_snapshot(audit_dir)

            self.assertIn("dashboard dataset warnings:", rendered)
            self.assertIn("lack current task-state evidence", rendered)

    def test_structured_state_snapshot_surfaces_recent_action_coverage(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            for day in (1, 2):
                session = audit_dir / f"day-{day}"
                write_json(
                    session / "outcome.json",
                    {
                        "day": day,
                        "ts": f"2026-01-0{day}T00:00:00Z",
                        "tasks_attempted": 1,
                        "tasks_succeeded": 1,
                    },
                )
                write_json(session / "state/summary.json", {"latest_gnomes": {}, "gnome_keys": []})
            write_events(
                audit_dir / "day-1/state/events.jsonl",
                [
                    {
                        "kind": "CommandStarted",
                        "payload": {
                            "tool_call_id": "tool-1",
                            "command": "cargo check",
                        },
                    }
                ],
            )
            transcript_dir = audit_dir / "day-2/transcripts"
            transcript_dir.mkdir(parents=True)
            transcript_dir.joinpath("task_01_attempt1.log").write_text(
                "  ▶ read src/lib.rs ✓ (5ms)\n",
                encoding="utf-8",
            )

            rendered = extract_trajectory.render_structured_state_snapshot(audit_dir)

            self.assertIn("recent action evidence:", rendered)
            self.assertIn("coverage=state 1/2, transcripts 1/2", rendered)

    def test_structured_state_snapshot_surfaces_gnome_evidence_audit(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-01-01T00:00:00Z",
                    "tasks_attempted": 0,
                    "tasks_succeeded": 0,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {"tool_error_count": 0},
                    "gnome_keys": ["tool_error_count"],
                },
            )
            transcript_dir = session / "transcripts"
            transcript_dir.mkdir(parents=True)
            transcript_dir.joinpath("task_01_attempt1.log").write_text(
                "  \u25b6 search 'needle' in src/lib.rs \u2717 (17ms)\n",
                encoding="utf-8",
            )

            rendered = extract_trajectory.render_structured_state_snapshot(audit_dir)

            self.assertIn("gnome evidence audit:", rendered)
            self.assertIn("adjusted=1 across 1 session(s)", rendered)
            self.assertIn("top_sources=transcripts=1", rendered)
            self.assertIn("reconciliation_not_raw_bug_count", rendered)

    def test_structured_state_snapshot_surfaces_recent_task_expected_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-01-01T00:00:00Z",
                    "tasks_attempted": 1,
                    "tasks_succeeded": 0,
                },
            )
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {"planning_failed": False, "task_count": 1, "selected_task_count": 1},
                    "selected_tasks": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "title": "Expose expected evidence",
                            "files": ["scripts/task_manifest.py"],
                            "artifact_path": "tasks/task_01/task.md",
                        }
                    ],
                    "artifacts": {"manifest": "tasks/manifest.json"},
                },
            )
            task_dir = session / "tasks/task_01"
            task_dir.mkdir(parents=True)
            task_dir.joinpath("task.md").write_text(
                "Title: Expose expected evidence\n"
                "Files: scripts/task_manifest.py\n"
                "Expected Evidence:\n"
                "- states.json task row carries expected evidence text\n",
                encoding="utf-8",
            )

            rendered = extract_trajectory.render_structured_state_snapshot(audit_dir)

            self.assertIn("recent task issues:", rendered)
            self.assertIn("recent task expected evidence:", rendered)
            self.assertIn("task_01=states.json task row carries expected evidence text", rendered)

    def test_structured_state_snapshot_prioritizes_recent_unresolved_claims(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            for index in range(6):
                session = audit_dir / f"day-{index + 1}"
                write_json(
                    session / "outcome.json",
                    {"day": index + 1, "ts": f"2026-01-0{index + 1}T00:00:00Z"},
                )
                write_json(session / "state/summary.json", {"latest_gnomes": {}, "gnome_keys": []})
                transcript_dir = session / "transcripts"
                transcript_dir.mkdir(parents=True)
                transcript_dir.joinpath("assess.log").write_text(
                    "Assessment phase transcript exists.\n",
                    encoding="utf-8",
                )
                if index > 0:
                    write_json(
                        session / "tasks/manifest.json",
                        {"artifacts": {"assessment": "tasks/assessment.md"}},
                    )
                    (session / "tasks/assessment.md").parent.mkdir(parents=True, exist_ok=True)
                    (session / "tasks/assessment.md").write_text("# Assessment\n", encoding="utf-8")

            rendered = extract_trajectory.render_structured_state_snapshot(audit_dir)
            first_line = next(line for line in rendered.splitlines() if line.startswith("claims:"))

            self.assertIn("recent non-proven claims:", first_line)
            self.assertNotIn("assessment_artifact", first_line)
            self.assertNotIn("recent assessment artifacts:", first_line)

    def test_structured_state_snapshot_surfaces_recent_assessment_classifications(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            for index in range(5):
                session = audit_dir / f"day-{index + 1}"
                write_json(
                    session / "outcome.json",
                    {"day": index + 1, "ts": f"2026-01-0{index + 1}T00:00:00Z"},
                )
                write_json(session / "state/summary.json", {"latest_gnomes": {}, "gnome_keys": []})
                transcript_dir = session / "transcripts"
                transcript_dir.mkdir(parents=True)
                transcript_dir.joinpath("assess.log").write_text("Assessment transcript.\n", encoding="utf-8")
                if index == 4:
                    write_json(
                        session / "tasks/manifest.json",
                        {"artifacts": {"assessment_missing": "tasks/assessment_missing.md"}},
                    )
                    (session / "tasks/assessment_missing.md").parent.mkdir(parents=True, exist_ok=True)
                    (session / "tasks/assessment_missing.md").write_text(
                        "Assessment was missing.\n",
                        encoding="utf-8",
                    )
                else:
                    write_json(
                        session / "tasks/manifest.json",
                        {"artifacts": {"assessment": "tasks/assessment.md"}},
                    )
                    (session / "tasks/assessment.md").parent.mkdir(parents=True, exist_ok=True)
                    (session / "tasks/assessment.md").write_text("# Assessment\n", encoding="utf-8")

            rendered = extract_trajectory.render_structured_state_snapshot(audit_dir)
            first_line = next(line for line in rendered.splitlines() if line.startswith("claims:"))

            self.assertNotIn("recent assessment artifacts:", first_line)
            self.assertIn(
                "\n- recent assessment artifacts: missing_with_diagnostic=1",
                rendered,
            )

    def test_structured_state_snapshot_omits_classified_input_validation_from_lifecycle_gaps(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(session / "outcome.json", {"day": 1, "ts": "2026-01-01T00:00:00Z"})
            write_json(session / "state/summary.json", {"latest_gnomes": {}, "gnome_keys": []})
            state_dir = session / "state"
            state_dir.mkdir(parents=True, exist_ok=True)
            state_dir.joinpath("events.jsonl").write_text(
                "\n".join(
                    json.dumps(row)
                    for row in [
                        {
                            "kind": "SessionStarted",
                            "run_id": "run-empty",
                            "payload": {"model": "deepseek-v4-pro"},
                        },
                        {
                            "kind": "RunCompleted",
                            "run_id": "run-empty",
                            "payload": {
                                "status": "error",
                                "error": "exit code 1",
                                "error_detail": "empty_input",
                            },
                        },
                    ]
                )
                + "\n",
                encoding="utf-8",
            )

            rendered = extract_trajectory.render_structured_state_snapshot(audit_dir)
            first_line = next(line for line in rendered.splitlines() if line.startswith("claims:"))

            self.assertNotIn("state_unmatched_completed", first_line)
            self.assertNotIn("state_unmatched_non_validation", first_line)
            self.assertNotIn("state_incomplete", first_line)

    def test_structured_state_snapshot_qualifies_recently_addressed_tool_failures(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            title = "Add regex-error recovery hint to search tool error messages"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-01-01T00:00:00Z",
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
                    "tasks": [
                        {
                            "task_id": "task_01",
                            "title": title,
                            "files": ["src/search.rs"],
                        }
                    ]
                },
            )
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "status": "completed",
                    "planned_files": ["src/search.rs"],
                    "source_files": ["src/search.rs"],
                    "commit_shas": ["abc123"],
                },
            )
            write_json(
                session / "tasks/task_01/eval_attempt_1.json",
                {"task_id": "task_01", "status": "pass", "verdict": "Verdict: PASS"},
            )
            transcript_dir = session / "transcripts"
            transcript_dir.mkdir(parents=True)
            transcript_dir.joinpath("task_01_attempt1.log").write_text(
                "\n".join(
                    [
                        "  ▶ search 'fn handle_run\\(' in src/search.rs ✗ (17ms)",
                        "      │ Search error: grep: Unmatched ( or \\(",
                    ]
                )
                + "\n",
                encoding="utf-8",
            )

            rendered = extract_trajectory.render_structured_state_snapshot(audit_dir)

            self.assertIn("historical unrecovered tool failures:", rendered)
            self.assertIn("search_regex_error=1", rendered)
            self.assertIn("(recent verified task: Add regex-error recovery hint", rendered)

    def test_structured_state_snapshot_omits_recovered_tool_failures_from_pressure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            command = "git show abc123 --stat"
            write_json(
                session / "outcome.json",
                {"day": 1, "ts": "2026-01-01T00:00:00Z", "tasks_attempted": 0, "tasks_succeeded": 0},
            )
            write_json(session / "state/summary.json", {"latest_gnomes": {}, "gnome_keys": []})
            state_dir = session / "state"
            state_dir.mkdir(parents=True, exist_ok=True)
            state_dir.joinpath("events.jsonl").write_text(
                "\n".join(
                    json.dumps(row)
                    for row in [
                        {
                            "kind": "ToolCallStarted",
                            "payload": {
                                "tool_call_id": "tool-1",
                                "tool_name": "bash",
                                "args": {"command": command},
                            },
                        },
                        {
                            "kind": "ToolCallCompleted",
                            "payload": {
                                "tool_call_id": "tool-1",
                                "tool_name": "bash",
                                "is_error": False,
                                "result_preview": "Exit code: 128",
                            },
                        },
                    ]
                )
                + "\n",
                encoding="utf-8",
            )
            session.joinpath("audit.jsonl").write_text(
                json.dumps({"tool": "bash", "success": True, "args": {"command": command}}) + "\n",
                encoding="utf-8",
            )

            rendered = extract_trajectory.render_structured_state_snapshot(audit_dir)

            self.assertIn("## Structured state snapshot", rendered)
            self.assertNotIn("historical unrecovered tool failures:", rendered)
            self.assertNotIn("bash_tool_error", rendered)

    def test_recently_addressed_tool_failures_prefer_title_over_context_body(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            regex_session = audit_dir / "day-1"
            binary_session = audit_dir / "day-2"

            for session, day, ts, title, body in (
                (
                    regex_session,
                    1,
                    "2026-01-01T00:00:00Z",
                    "Add regex-error recovery hint to search tool error messages",
                    "",
                ),
                (
                    binary_session,
                    2,
                    "2026-01-02T00:00:00Z",
                    "Extend search tool with binary-match recovery hints",
                    "Extends Day 1 regex-error recovery to search_binary_match=19.",
                ),
            ):
                write_json(
                    session / "outcome.json",
                    {
                        "day": day,
                        "ts": ts,
                        "tasks_attempted": 1,
                        "tasks_succeeded": 1,
                    },
                )
                write_json(
                    session / "tasks/manifest.json",
                    {
                        "tasks": [
                            {
                                "task_id": "task_01",
                                "title": title,
                                "body_preview": body,
                                "files": ["src/search.rs"],
                            }
                        ]
                    },
                )
                write_json(
                    session / "tasks/task_01/outcome.json",
                    {
                        "task_id": "task_01",
                        "status": "completed",
                        "planned_files": ["src/search.rs"],
                        "source_files": ["src/search.rs"],
                        "commit_shas": ["abc123"],
                    },
                )
                write_json(
                    session / "tasks/task_01/eval_attempt_1.json",
                    {"task_id": "task_01", "status": "pass", "verdict": "Verdict: PASS"},
                )

            addressed = extract_trajectory.recently_addressed_tool_failure_categories(audit_dir)

            self.assertEqual(
                addressed["search_regex_error"],
                "Add regex-error recovery hint to search tool error messages",
            )
            self.assertEqual(
                addressed["search_binary_match"],
                "Extend search tool with binary-match recovery hints",
            )

    def test_graph_suggestions_surface_lifecycle_pressure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(
                session / "outcome.json",
                {"day": 1, "ts": "2026-01-01T00:00:00Z"},
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "deepseek_model_call_incomplete_count": 1,
                        "search_error_count": 3,
                    },
                    "state_lifecycle": {
                        "model_calls": {
                            "incomplete_runs": [
                                {
                                    "run_id": "run-open",
                                    "last_event": {"kind": "CommandCompleted"},
                                }
                            ]
                        }
                    },
                },
            )

            rendered = extract_trajectory.render_graph_suggestions(audit_dir)

            self.assertIn("## Graph-derived next-task pressure", rendered)
            self.assertIn("Close yyds state and model lifecycle gaps", rendered)
            self.assertIn("deepseek_model_call_incomplete_count=1", rendered)
            self.assertIn("model_incomplete/open_after_command=1", rendered)

    def test_graph_suggestions_surface_abnormal_model_completion_pressure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(
                session / "outcome.json",
                {"day": 1, "ts": "2026-01-01T00:00:00Z"},
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "deepseek_model_call_abnormal_completed_count": 1,
                    },
                    "state_lifecycle": {
                        "model_calls": {
                            "abnormal_completed_runs": [
                                {
                                    "run_id": "run-abnormal",
                                    "last_event": {"kind": "ModelCallCompleted"},
                                }
                            ]
                        }
                    },
                },
            )

            rendered = extract_trajectory.render_graph_suggestions(audit_dir)

            self.assertIn("## Graph-derived next-task pressure", rendered)
            self.assertIn("Close yyds state and model lifecycle gaps", rendered)
            self.assertIn("deepseek_model_call_abnormal_completed_count=1", rendered)
            self.assertIn("model_abnormal/model_completion_without_start=1", rendered)

    def test_graph_suggestions_include_unattempted_task_pressure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(
                session / "outcome.json",
                {"day": 1, "ts": "2026-01-01T00:00:00Z"},
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "task_unattempted_count": 1,
                        "task_artifact_coverage": 1.0,
                    }
                },
            )

            rendered = extract_trajectory.render_graph_suggestions(audit_dir)

            self.assertIn("## Graph-derived next-task pressure", rendered)
            self.assertIn("Preserve budget to start every selected task", rendered)
            self.assertIn("task_unattempted_count=1", rendered)

    def test_graph_suggestions_include_obsolete_task_pressure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(
                session / "outcome.json",
                {"day": 1, "ts": "2026-01-01T00:00:00Z"},
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "task_obsolete_count": 1,
                        "task_artifact_coverage": 1.0,
                    }
                },
            )

            rendered = extract_trajectory.render_graph_suggestions(audit_dir)

            self.assertIn("## Graph-derived next-task pressure", rendered)
            self.assertIn("Replace stale or already-satisfied tasks", rendered)
            self.assertIn("task_obsolete_count=1", rendered)

    def test_graph_suggestions_include_raw_seed_contradiction_pressure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(
                session / "outcome.json",
                {"day": 1, "ts": "2026-01-01T00:00:00Z"},
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "task_seed_contradiction_count": 1,
                        "task_artifact_coverage": 1.0,
                    }
                },
            )

            rendered = extract_trajectory.render_graph_suggestions(audit_dir)

            self.assertIn("## Graph-derived next-task pressure", rendered)
            self.assertIn("Validate seeded tasks against fresh assessment", rendered)
            self.assertIn("task_seed_contradiction_count=1", rendered)

    def test_graph_suggestions_include_measured_execution_state_and_cache_pressure(self) -> None:
        cases = [
            ("state_live_baseline_shrink_count", "Keep live state append-only"),
            ("task_api_error_count", "Recover API-error tasks instead of generic reverts"),
            ("provider_error_count", "Recover provider errors before task attempts"),
            ("task_no_edit_revert_count", "Force reverted tasks to leave concrete evidence"),
            ("task_scope_mismatch_count", "Align implementation edits with task file scope"),
            ("protected_file_revert_count", "Route protected-file work through explicit approval"),
            ("tool_error_count", "Recover failed tool actions before scoring"),
            ("prompt_heredoc_expansion_error_count", "Quote generated prompts before execution"),
            ("deepseek_cache_ratio_unverified_count", "Ignore prose-only DeepSeek cache ratios"),
            ("deepseek_cache_metric_missing_count", "Record token-backed DeepSeek cache metrics"),
        ]
        for metric, title in cases:
            with self.subTest(metric=metric), tempfile.TemporaryDirectory() as tmp:
                audit_dir = Path(tmp)
                session = audit_dir / "day-1"
                write_json(
                    session / "outcome.json",
                    {"day": 1, "ts": "2026-01-01T00:00:00Z"},
                )
                write_json(
                    session / "state/summary.json",
                    {
                        "latest_gnomes": {
                            metric: 1,
                            "task_artifact_coverage": 1.0,
                        }
                    },
                )

                rendered = extract_trajectory.render_graph_suggestions(audit_dir)

                self.assertIn("## Graph-derived next-task pressure", rendered)
                self.assertIn(title, rendered)
                self.assertIn(f"{metric}=1", rendered)

    def test_graph_suggestions_surface_recurring_log_failure_pressure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(session / "outcome.json", {"day": 1, "ts": "2026-01-01T00:00:00Z"})
            write_json(
                session / "log_feedback.json",
                {
                    "metrics": {
                        "recurring_failure_count": 1,
                        "max_failure_fingerprint_recurrence": 3,
                        "failure_fingerprints": [
                            {"fingerprint": "cargo test failed in eval fixture", "count": 2}
                        ],
                    }
                },
            )

            rendered = extract_trajectory.render_graph_suggestions(audit_dir)

            self.assertIn("## Graph-derived next-task pressure", rendered)
            self.assertIn("Break recurring log failure fingerprints", rendered)
            self.assertIn("recurring_failure_count=1", rendered)

    def test_graph_suggestions_surface_repair_loop_churn_pressure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(session / "outcome.json", {"day": 1, "ts": "2026-01-01T00:00:00Z"})
            write_json(
                session / "log_feedback.json",
                {"metrics": {"repair_loop_count": 2}},
            )

            rendered = extract_trajectory.render_graph_suggestions(audit_dir)

            self.assertIn("## Graph-derived next-task pressure", rendered)
            self.assertIn("Reduce repair-loop churn", rendered)
            self.assertIn("repair_loop_count=2", rendered)

    def test_graph_suggestions_surface_low_state_capture_pressure(self) -> None:
        cases = [
            (
                {"state_operational_capture_coverage": 0.0, "state_capture_coverage": 1.0},
                "Restore operational state capture",
                "state_operational_capture_coverage=0.0",
            ),
            (
                {"state_capture_coverage": 0.0},
                "Restore state event capture",
                "state_capture_coverage=0.0",
            ),
        ]
        for gnomes, title, metric_text in cases:
            with self.subTest(title=title), tempfile.TemporaryDirectory() as tmp:
                audit_dir = Path(tmp)
                session = audit_dir / "day-1"
                latest_gnomes = {"task_artifact_coverage": 1.0}
                latest_gnomes.update(gnomes)
                write_json(session / "outcome.json", {"day": 1, "ts": "2026-01-01T00:00:00Z"})
                write_json(session / "state/summary.json", {"latest_gnomes": latest_gnomes})

                rendered = extract_trajectory.render_graph_suggestions(audit_dir)

                self.assertIn("## Graph-derived next-task pressure", rendered)
                self.assertIn(title, rendered)
                self.assertIn(metric_text, rendered)

    def test_graph_suggestions_surface_state_replay_integrity_pressure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(session / "outcome.json", {"day": 1, "ts": "2026-01-01T00:00:00Z"})
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "state_replay_integrity_rate": 0.0,
                        "task_artifact_coverage": 1.0,
                    }
                },
            )

            rendered = extract_trajectory.render_graph_suggestions(audit_dir)

            self.assertIn("## Graph-derived next-task pressure", rendered)
            self.assertIn("Repair state replay integrity", rendered)
            self.assertIn("state_replay_integrity_rate=0.0", rendered)

    def test_graph_suggestions_surface_state_failure_count_pressure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(session / "outcome.json", {"day": 1, "ts": "2026-01-01T00:00:00Z"})
            write_json(
                session / "state/summary.json",
                {"latest_gnomes": {"state_failure_count": 2, "task_artifact_coverage": 1.0}},
            )

            rendered = extract_trajectory.render_graph_suggestions(audit_dir)

            self.assertIn("## Graph-derived next-task pressure", rendered)
            self.assertIn("Repair recorded state failure events", rendered)
            self.assertIn("state_failure_count=2", rendered)

    def test_graph_suggestions_surface_json_parse_failure_pressure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(session / "outcome.json", {"day": 1, "ts": "2026-01-01T00:00:00Z"})
            write_json(
                session / "state/summary.json",
                {"latest_gnomes": {"json_parse_failure_rate": 0.25, "task_artifact_coverage": 1.0}},
            )

            rendered = extract_trajectory.render_graph_suggestions(audit_dir)

            self.assertIn("## Graph-derived next-task pressure", rendered)
            self.assertIn("Reduce DeepSeek JSON parse failures", rendered)
            self.assertIn("json_parse_failure_rate=0.25", rendered)

    def test_graph_suggestions_surface_low_task_lineage_capture_pressure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(session / "outcome.json", {"day": 1, "ts": "2026-01-01T00:00:00Z"})
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "task_artifact_coverage": 1.0,
                        "task_lineage_capture_coverage": 0.0,
                    }
                },
            )

            rendered = extract_trajectory.render_graph_suggestions(audit_dir)

            self.assertIn("## Graph-derived next-task pressure", rendered)
            self.assertIn("Restore explicit task lineage capture", rendered)
            self.assertIn("task_lineage_capture_coverage=0.0", rendered)

    def test_graph_suggestions_surface_low_task_verification_pressure(self) -> None:
        cases = [
            (
                {
                    "task_verification_rate": 0.5,
                    "evaluator_unverified_count": 0,
                },
                "Require strict verifier evidence for tasks",
                "task_verification_rate=0.5",
            ),
            (
                {
                    "task_verification_rate": 1.0,
                    "task_mechanical_verification_rate": 0.0,
                },
                "Preserve mechanical verification artifacts",
                "task_mechanical_verification_rate=0.0",
            ),
        ]
        for gnomes, title, metric_text in cases:
            with self.subTest(title=title), tempfile.TemporaryDirectory() as tmp:
                audit_dir = Path(tmp)
                session = audit_dir / "day-1"
                latest_gnomes = {"task_artifact_coverage": 1.0}
                latest_gnomes.update(gnomes)
                write_json(session / "outcome.json", {"day": 1, "ts": "2026-01-01T00:00:00Z"})
                write_json(session / "state/summary.json", {"latest_gnomes": latest_gnomes})

                rendered = extract_trajectory.render_graph_suggestions(audit_dir)

                self.assertIn("## Graph-derived next-task pressure", rendered)
                self.assertIn(title, rendered)
                self.assertIn(metric_text, rendered)

    def test_graph_suggestions_surface_recent_action_evidence_drift(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-01-01T00:00:00Z",
                    "tasks_attempted": 1,
                    "tasks_succeeded": 0,
                    "build_ok": True,
                    "test_ok": True,
                    "reverted": False,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "task_artifact_coverage": 1.0,
                        "tool_error_count": 0,
                    }
                },
            )
            transcript_dir = session / "transcripts"
            transcript_dir.mkdir(parents=True)
            transcript_dir.joinpath("task_01_attempt1.log").write_text(
                "\n".join(
                    [
                        "  ▶ search 'fn handle_run\\(' in src/commands.rs ✗ (17ms)",
                        "      │ Search error: grep: Unmatched ( or \\(",
                    ]
                )
                + "\n",
                encoding="utf-8",
            )

            rendered = extract_trajectory.render_graph_suggestions(audit_dir)

            self.assertIn("## Graph-derived next-task pressure", rendered)
            self.assertIn("Use fixed-string search for regex-like patterns", rendered)
            self.assertIn("failed_tool_summary.search_regex_error=1", rendered)
            self.assertIn("Reconcile transcript-only tool failures", rendered)
            self.assertIn("transcript_only_failed_tool_count=1", rendered)
            self.assertIn("Restore action evidence coverage", rendered)
            self.assertIn("action_evidence_coverage_gap_count=1", rendered)

    def test_graph_suggestions_render_five_ranked_pressures(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(
                session / "outcome.json",
                {"day": 1, "ts": "2026-01-01T00:00:00Z"},
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "state_live_baseline_shrink_count": 1,
                        "evaluator_unverified_count": 1,
                        "task_unlanded_source_count": 1,
                        "task_api_error_count": 1,
                        "task_obsolete_count": 1,
                        "task_artifact_coverage": 1.0,
                    }
                },
            )

            rendered = extract_trajectory.render_graph_suggestions(audit_dir)

            bullets = [line for line in rendered.splitlines() if line.startswith("- ")]
            self.assertEqual(len(bullets), 5)
            self.assertIn("task_api_error_count=1", rendered)
            self.assertIn("task_obsolete_count=1", rendered)

    def test_graph_suggestions_include_missing_expected_evidence_pressure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(
                session / "outcome.json",
                {"day": 1, "ts": "2026-01-01T00:00:00Z"},
            )
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {"planning_failed": False, "task_count": 1, "selected_task_count": 1},
                    "selected_tasks": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "title": "Improve vague task",
                            "files": ["scripts/evolve.sh"],
                            "quality": {"has_expected_evidence": False},
                        }
                    ],
                    "warnings": ["task_01:missing_expected_evidence"],
                },
            )

            rendered = extract_trajectory.render_graph_suggestions(audit_dir)

            self.assertIn("## Graph-derived next-task pressure", rendered)
            self.assertIn("Require task evidence specs", rendered)
            self.assertIn("missing_expected_evidence_count=1", rendered)

    def test_graph_suggestions_include_low_task_spec_quality_pressure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(session / "outcome.json", {"day": 1, "ts": "2026-01-01T00:00:00Z"})
            write_json(
                session / "state/summary.json",
                {"latest_gnomes": {"task_spec_quality_score": 0.5}},
            )

            rendered = extract_trajectory.render_graph_suggestions(audit_dir)

            self.assertIn("## Graph-derived next-task pressure", rendered)
            self.assertIn("Tighten selected task specs", rendered)
            self.assertIn("task_spec_quality_score=0.5", rendered)

    def test_graph_suggestions_include_missing_assessment_artifact_pressure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(
                session / "outcome.json",
                {"day": 1, "ts": "2026-01-01T00:00:00Z"},
            )
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {"planning_failed": False, "task_count": 0, "selected_task_count": 0},
                    "artifacts": {"assessment": None, "assessment_missing": None},
                },
            )
            transcript_dir = session / "transcripts"
            transcript_dir.mkdir(parents=True)
            transcript_dir.joinpath("assess.log").write_text("assessment phase ran\n", encoding="utf-8")

            rendered = extract_trajectory.render_graph_suggestions(audit_dir)

            self.assertIn("## Graph-derived next-task pressure", rendered)
            self.assertIn("Preserve assessment artifacts", rendered)
            self.assertIn("assessment_artifact_missing_count=1", rendered)


if __name__ == "__main__":
    unittest.main()
