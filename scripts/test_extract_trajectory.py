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


class ExtractTrajectoryTests(unittest.TestCase):
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
                    "fingerprint": "DeepSeek model call lifecycle was incomplete",
                    "action": "close model-call lifecycle events",
                }
            ],
        )

        self.assertIn("Corrected top lessons for next run:", rendered)
        self.assertIn("DeepSeek model call lifecycle was incomplete", rendered)
        self.assertIn("Historical repeated across prior log feedback:", rendered)
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
            self.assertIn("no planned-file overlap", rendered)
            self.assertIn("no passing verifier", rendered)

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
                    "run_id": "run-model-open",
                    "payload": {"model": "deepseek-v4-pro"},
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
            self.assertIn("lifecycle gaps:", rendered)
            self.assertIn("state_incomplete=1", rendered)
            self.assertIn("model_incomplete=1", rendered)
            self.assertIn("top task states:", rendered)
            self.assertIn("deepseek_model_call_lifecycle_balanced", rendered)
            self.assertIn("latest=day-1", rendered)
            self.assertIn("task states:", rendered)
            self.assertIn("unlanded_source_edits=1", rendered)
            self.assertIn("tool failures:", rendered)
            self.assertIn("search_tool_error=1", rendered)
            self.assertIn("lifecycle gnomes:", rendered)
            self.assertIn("state_run_incomplete_count=1", rendered)
            self.assertIn("deepseek_model_call_incomplete_count=1", rendered)

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
                    }
                },
            )

            rendered = extract_trajectory.render_graph_suggestions(audit_dir)

            self.assertIn("## Graph-derived next-task pressure", rendered)
            self.assertIn("Close yyds state and model lifecycle gaps", rendered)
            self.assertIn("deepseek_model_call_incomplete_count=1", rendered)


if __name__ == "__main__":
    unittest.main()
