#!/usr/bin/env python3
"""Tests for scripts/build_evolution_dashboard.py."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from build_evolution_dashboard import (  # noqa: E402
    build,
    count_claim,
    failed_tool_pattern_summary,
    run_health,
    run_health_reasons,
    summarize_events_for_work,
    summarize_transcript_actions,
    task_verification_problem_reasons,
    task_verification_summary,
    transcript_summary,
)


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def write_events(path: Path, rows: list[dict[str, object]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, separators=(",", ":")) + "\n")


class BuildEvolutionDashboard(unittest.TestCase):
    def test_transcript_action_paths_drop_pseudo_workspace_dot(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            transcript_dir = session / "transcripts"
            transcript_dir.mkdir(parents=True)
            transcript_dir.joinpath("task_01_attempt1.log").write_text(
                "\n".join(
                    [
                        "  ▶ read /home/runner/work/yyds-harness/yyds-harness/src/state.rs ✓ (5ms)",
                        "  ▶ edit /home/runner/work/yyds-harness/yyds-harness/session_plan/eval_task_1.md ✓ (5ms)",
                        "  ▶ edit /home/runner/work/yyds-harness/yyds-harness/.github/workflows/ci.yml ✓ (5ms)",
                        "  ▶ edit /home/runner/work/yyds-harness/yyds-harness/.gitignore ✓ (5ms)",
                    ]
                )
                + "\n",
                encoding="utf-8",
            )

            actions = summarize_transcript_actions(session)

            self.assertIn("src/state.rs", actions["read_files"])
            self.assertIn("session_plan/eval_task_1.md", actions["edited_files"])
            self.assertIn(".github/workflows/ci.yml", actions["edited_files"])
            self.assertIn(".gitignore", actions["edited_files"])
            self.assertNotIn(".src/state.rs", actions["read_files"])
            self.assertNotIn(".session_plan/eval_task_1.md", actions["edited_files"])
            self.assertEqual(actions["read_file_count"], 1)
            self.assertEqual(actions["edited_file_count"], 3)

    def test_state_event_paths_drop_workspace_prefix(self):
        events = [
            {
                "kind": "FileEdited",
                "payload": {
                    "path": "/home/runner/work/yyds-harness/yyds-harness/session_plan/eval_task_3.md"
                },
            },
            {
                "kind": "FileRead",
                "payload": {
                    "path": "/home/runner/work/yyds-harness/yyds-harness/src/deepseek.rs"
                },
            },
        ]

        work = summarize_events_for_work(events)

        self.assertEqual(work["edited_files"], ["session_plan/eval_task_3.md"])
        self.assertEqual(work["read_files"], ["src/deepseek.rs"])
        self.assertEqual(work["edited_file_count"], 1)
        self.assertEqual(work["read_file_count"], 1)

    def test_transcript_search_in_src_does_not_emit_pseudo_dot_src(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            transcript_dir = session / "transcripts"
            transcript_dir.mkdir(parents=True)
            transcript_dir.joinpath("task_01_attempt1.log").write_text(
                "  ▶ search 'mark_run_completed_with_error' in /home/runner/work/yyds-harness/yyds-harness/src ✗ (17ms)\n",
                encoding="utf-8",
            )

            actions = summarize_transcript_actions(session)

            self.assertNotIn(".src", actions["read_files"])
            self.assertNotIn("src", actions["read_files"])
            self.assertIn("search 'mark_run_completed_with_error' in src", actions["failed_commands"])
            self.assertIn("search 'mark_run_completed_with_error' in src", actions["failed_tools"])

    def test_transcript_failed_tool_count_is_not_capped_by_sample(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            transcript_dir = session / "transcripts"
            transcript_dir.mkdir(parents=True)
            transcript_dir.joinpath("task_01_attempt1.log").write_text(
                "\n".join(
                    f"  ▶ search 'needle_{index}' in src/file_{index}.rs ✗ (17ms)"
                    for index in range(10)
                )
                + "\n",
                encoding="utf-8",
            )

            actions = summarize_transcript_actions(session)

            self.assertEqual(len(actions["failed_tools"]), 8)
            self.assertEqual(actions["failed_tool_count"], 10)
            self.assertEqual(actions["failed_command_count"], 10)
            self.assertEqual(actions["failed_tool_summary"]["total_count"], 10)
            self.assertEqual(
                actions["failed_tool_summary"]["category_counts"],
                {"search_tool_error": 10},
            )

    def test_failed_tool_pattern_summary_uses_distinct_normalized_failures(self):
        summary = failed_tool_pattern_summary(
            [
                "search 'needle' in src/lib.rs: Search error: grep: Unmatched ( or \\(",
                "search 'needle' in src/lib.rs: Search error: grep: Unmatched ( or \\(",
                "read src/main.rs: Cannot access src/main.rs: No such file or directory (os error 2)",
            ]
        )

        self.assertEqual(summary["total_count"], 2)
        self.assertEqual(
            summary["category_counts"],
            {"missing_file_read": 1, "search_regex_error": 1},
        )

    def test_transcript_failed_edit_keeps_error_detail(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            transcript_dir = session / "transcripts"
            transcript_dir.mkdir(parents=True)
            transcript_dir.joinpath("task_02_attempt1.log").write_text(
                "\n".join(
                    [
                        "  ▶ edit /home/runner/work/yyds-harness/yyds-harness/src/prompt.rs (2 → 3 lines)",
                        "                        api_error = Some(error_msg);",
                        '  +                     crate::state::stash_diagnostic_error(&format!("prompt: api_error: {error_msg}"));',
                        "                    }",
                        " ✗ (12ms)",
                        "      │ old_text matches 2 locations in /home/runner/work/yyds-harness/yyds-harness/src/prompt.rs. Include more surrounding context to make the match unique.",
                    ]
                )
                + "\n",
                encoding="utf-8",
            )

            actions = summarize_transcript_actions(session)

            self.assertEqual(
                actions["failed_tools"],
                [
                    "edit src/prompt.rs: old_text matches 2 locations in src/prompt.rs. Include more surrounding context to make the match unique."
                ],
            )
            self.assertEqual(
                actions["failed_tool_summary"]["category_counts"],
                {"edit_context_mismatch": 1},
            )

    def test_build_fix_transcripts_are_not_classified_as_other(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp)
            transcript_dir = session / "transcripts"
            transcript_dir.mkdir(parents=True)
            transcript_dir.joinpath("bfix_task2_attempt1.log").write_text(
                "╭─ Turn 1 ─╮\n▶ edit src/lib.rs\n",
                encoding="utf-8",
            )
            transcript_dir.joinpath("fix_task2_attempt1.log").write_text(
                "╭─ Turn 1 ─╮\n▶ edit src/lib.rs\n",
                encoding="utf-8",
            )

            summary = transcript_summary(session)

            self.assertEqual(summary["phase_counts"]["build_fix"], 1)
            self.assertEqual(summary["phase_counts"]["fix"], 1)
            self.assertNotIn("other", summary["phase_counts"])
            phases = {row["name"]: row["phase"] for row in summary["files"]}
            self.assertEqual(phases["bfix_task2_attempt1.log"], "build_fix")
            self.assertEqual(phases["fix_task2_attempt1.log"], "fix")

    def test_task_turn_gnomes_are_corrected_from_repair_attempt_artifacts(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(session / "outcome.json", {"day": 1, "tasks_attempted": 1, "tasks_succeeded": 0})
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "max_task_turn_count": 5,
                        "avg_task_turn_count": 5,
                        "total_task_turn_count": 5,
                    },
                    "gnome_keys": ["max_task_turn_count", "avg_task_turn_count", "total_task_turn_count"],
                },
            )
            transcript_dir = session / "transcripts"
            transcript_dir.mkdir(parents=True)
            transcript_dir.joinpath("task_01_attempt1.log").write_text(
                "╭─ Turn 5 ─╮\n▶ edit src/lib.rs\n",
                encoding="utf-8",
            )
            transcript_dir.joinpath("bfix_task1_attempt1.log").write_text(
                "╭─ Turn 18 ─╮\n▶ cargo test\n",
                encoding="utf-8",
            )
            task_dir = session / "tasks/task_01"
            task_dir.mkdir(parents=True)
            task_dir.joinpath("attempts.jsonl").write_text(
                "\n".join(
                    [
                        json.dumps(
                            {
                                "task_id": "task_01",
                                "phase": "implementation",
                                "attempt": 1,
                                "stage_name": "task_01_attempt1",
                                "transcript_path": "transcripts/task_01_attempt1.log",
                            },
                            separators=(",", ":"),
                        ),
                        json.dumps(
                            {
                                "task_id": "task_01",
                                "phase": "build_fix",
                                "attempt": 1,
                                "stage_name": "bfix_task1_attempt1",
                                "transcript_path": "transcripts/bfix_task1_attempt1.log",
                            },
                            separators=(",", ":"),
                        ),
                    ]
                )
                + "\n",
                encoding="utf-8",
            )

            data = build(root / "sessions", root / "out", repo_root=root)

            latest = data["sessions"][0]["latest_gnomes"]
            self.assertEqual(latest["max_task_turn_count"], 18)
            self.assertEqual(latest["avg_task_turn_count"], 18)
            self.assertEqual(latest["total_task_turn_count"], 18)
            self.assertEqual(data["sessions"][0]["work_summary"]["task_artifacts"][0]["max_turn_count"], 18)

    def test_task_verification_surfaces_lineage_eval_statuses(self):
        verification = task_verification_summary(
            {
                "tasks": [
                    {
                        "task_id": "task_01",
                        "title": "Add state diagnostics",
                        "files": ["src/state.rs"],
                    }
                ]
            },
            [],
            [
                {
                    "task_id": "task_01",
                    "task_title": "Add state diagnostics",
                    "status": "completed",
                    "planned_files": ["src/state.rs"],
                    "source_files": ["src/state.rs"],
                    "commit_shas": ["abc123"],
                    "eval": {"verdict": "PASS", "reason": "Verifier passed from state lineage."},
                }
            ],
        )

        row = verification["rows"][0]
        self.assertTrue(row["strict_success"])
        self.assertEqual(row["eval_statuses"], ["pass"])
        self.assertEqual(row["eval_evidence_source"], "state_lineage")

    def test_task_verification_surfaces_eval_attempt_details(self):
        verification = task_verification_summary(
            {
                "tasks": [
                    {
                        "task_id": "task_01",
                        "title": "Verify task attempt",
                        "files": ["src/state.rs"],
                    }
                ]
            },
            [
                {
                    "task_id": "task_01",
                    "status": "reverted",
                    "source_files": ["src/state.rs"],
                    "evals": [
                        {
                            "task_id": "task_01",
                            "status": "timeout",
                            "exit_code": 124,
                            "reason": "Evaluator timed out before a trusted verdict.",
                            "transcript_path": "transcripts/eval_task_1_attempt1.log",
                        }
                    ],
                }
            ],
            [],
        )

        row = verification["rows"][0]
        self.assertFalse(row["strict_success"])
        self.assertEqual(row["eval_evidence_source"], "task_artifact")
        self.assertEqual(row["eval_statuses"], ["timeout"])
        self.assertEqual(row["eval_attempt_count"], 1)
        self.assertEqual(row["latest_eval_attempt"]["attempt"], 1)
        self.assertEqual(row["latest_eval_attempt"]["status"], "timeout")
        self.assertEqual(row["latest_eval_attempt"]["exit_code"], 124)
        self.assertEqual(
            row["latest_eval_attempt"]["transcript_path"],
            "transcripts/eval_task_1_attempt1.log",
        )
        self.assertIn("no_passing_verifier", row["problems"])

    def test_structured_task_states_link_transcripts_and_unattempted_tasks(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "tasks_attempted": 1,
                    "tasks_succeeded": 0,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "task_lineage": [
                        {
                            "task_id": "task_01",
                            "task_title": "Transcript backed task",
                            "status": "completed",
                            "planned_files": ["src/state.rs"],
                            "source_files": ["src/state.rs"],
                            "commit_shas": [],
                        }
                    ]
                },
            )
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {"planning_failed": False, "task_count": 2, "selected_task_count": 2},
                    "selected_tasks": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "title": "Transcript backed task",
                            "files": ["src/state.rs"],
                            "artifact_path": "tasks/task_01/task.md",
                        },
                        {
                            "task_id": "task_03",
                            "task_number": 3,
                            "title": "Never attempted task",
                            "files": ["src/lib.rs"],
                            "artifact_path": "tasks/task_03/task.md",
                        },
                    ],
                    "artifacts": {"manifest": "tasks/manifest.json"},
                },
            )
            (session / "tasks/task_03").mkdir(parents=True)
            transcript_dir = session / "transcripts"
            transcript_dir.mkdir(parents=True)
            transcript_dir.joinpath("task_01_attempt1.log").write_text(
                "  ▶ edit src/state.rs (1 -> 2 lines)\n",
                encoding="utf-8",
            )
            transcript_dir.joinpath("eval_task1_attempt1.log").write_text(
                "Verdict: FAIL\n",
                encoding="utf-8",
            )

            data = build(root / "sessions", root / "out")
            states = {
                row["task_id"]: row
                for row in data["sessions"][0]["work_summary"]["task_states"]["tasks"]
            }

            self.assertEqual(states["task_01"]["implementation_attempt_count"], 1)
            self.assertEqual(states["task_01"]["implementation_transcripts"], ["transcripts/task_01_attempt1.log"])
            self.assertEqual(states["task_01"]["eval_transcripts"], ["transcripts/eval_task1_attempt1.log"])
            self.assertIn("transcripts", states["task_01"]["evidence_sources"])
            self.assertTrue(states["task_01"]["attempted"])
            self.assertFalse(states["task_03"]["attempted"])
            self.assertEqual(states["task_03"]["state"], "not_attempted")
            states_json = json.loads((root / "out/states.json").read_text(encoding="utf-8"))
            state_rows = {
                row["task_id"]: row
                for row in states_json["sessions"][0]["tasks"]
            }
            self.assertEqual(state_rows["task_01"]["transcript_paths"], [
                "transcripts/task_01_attempt1.log",
                "transcripts/eval_task1_attempt1.log",
            ])
            html = (root / "out/index.html").read_text(encoding="utf-8")
            self.assertIn("Task states", html)
            self.assertIn("renderTaskStates", html)

    def test_dashboard_normalizes_annotated_planned_files_from_manifest(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(session / "outcome.json", {"ts": "2026-01-01T00:00:00Z"})
            write_json(session / "state/summary.json", {"latest_gnomes": {}, "gnome_keys": []})
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
                            "title": "Extract diagnostics module",
                            "files": [
                                "src/commands_state.rs",
                                "src/commands_state_diagnostics.rs (new)",
                            ],
                            "artifact_path": "tasks/task_01/task.md",
                            "quality": {"score": 1.0},
                        }
                    ],
                    "artifacts": {"manifest": "tasks/manifest.json"},
                },
            )
            task_dir = session / "tasks/task_01"
            task_dir.mkdir(parents=True)
            task_dir.joinpath("task.md").write_text(
                "Title: Extract diagnostics module\nFiles: src/commands_state_diagnostics.rs (new)\n",
                encoding="utf-8",
            )

            data = build(root / "sessions", root / "out", repo_root=root)
            task = data["sessions"][0]["work_summary"]["task_manifest"]["tasks"][0]

            self.assertEqual(
                task["files"],
                ["src/commands_state.rs", "src/commands_state_diagnostics.rs"],
            )
            self.assertNotIn("src/commands_state_diagnostics.rs (new)", task["files"])

    def test_state_event_failures_join_started_tool_args(self):
        events = [
            {
                "kind": "ToolCallStarted",
                "payload": {
                    "tool_call_id": "call_1",
                    "tool_name": "bash",
                    "args": {"description": "cd repo && git diff HEAD --stat"},
                },
            },
            {
                "kind": "CommandCompleted",
                "payload": {
                    "tool_call_id": "call_1",
                    "is_error": True,
                    "result_preview": "Invalid arguments: missing 'command' parameter",
                },
            },
            {
                "kind": "ToolCallCompleted",
                "payload": {
                    "tool_call_id": "call_1",
                    "tool_name": "bash",
                    "is_error": True,
                    "result_preview": "Invalid arguments: missing 'command' parameter",
                },
            },
            {
                "kind": "ToolCallStarted",
                "payload": {
                    "tool_call_id": "call_2",
                    "tool_name": "bash",
                    "args": {"command": "DEEPSEEK_API_KEY=sk-secret123456 ./target/debug/yyds --prompt hi"},
                },
            },
            {
                "kind": "ToolCallCompleted",
                "payload": {
                    "tool_call_id": "call_2",
                    "tool_name": "bash",
                    "is_error": True,
                    "result_preview": "Command timed out after 60s",
                },
            },
            {
                "kind": "ToolCallStarted",
                "payload": {
                    "tool_call_id": "call_3",
                    "tool_name": "bash",
                    "args": {"command": "git show missing-sha --no-stat -p"},
                },
            },
            {
                "kind": "CommandCompleted",
                "payload": {
                    "tool_call_id": "call_3",
                    "is_error": False,
                    "result_preview": "Exit code: 128",
                },
            },
            {
                "kind": "ToolCallCompleted",
                "payload": {
                    "tool_call_id": "call_3",
                    "tool_name": "bash",
                    "is_error": False,
                    "result_preview": "Exit code: 128",
                },
            },
        ]

        work = summarize_events_for_work(events)

        self.assertIn(
            "bash description: cd repo && git diff HEAD --stat: Invalid arguments: missing 'command' parameter",
            work["failed_tools"],
        )
        self.assertIn(
            "bash DEEPSEEK_API_KEY=sk-[REDACTED] ./target/debug/yyds --prompt hi: Command timed out after 60s",
            work["failed_tools"],
        )
        self.assertIn(
            "bash git show missing-sha --no-stat -p: Exit code: 128",
            work["failed_tools"],
        )
        self.assertNotIn("bash", work["failed_tools"])
        self.assertIn("git show missing-sha --no-stat -p", work["failed_commands"])

    def test_latest_decision_prefers_session_plan_over_permission_policy(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(session / "outcome.json", {"day": 1})
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 3,
                    "event_counts": {"DecisionRecorded": 2, "RunCompleted": 1},
                    "latest_decision": {
                        "decision_type": "tool_permission_policy",
                        "reason": "allowed medium-risk file_operation via session_always",
                    },
                    "decisions": [
                        {
                            "decision_type": "session_plan",
                            "decision": "tasks_selected",
                            "reason": "planning phase selected implementation tasks for this evolution session",
                        },
                        {
                            "decision_type": "tool_permission_policy",
                            "reason": "allowed medium-risk file_operation via session_always",
                        },
                    ],
                },
            )

            data = build(root / "sessions", root / "out")
            latest = data["sessions"][0]["latest_decision"]

            self.assertEqual(latest["decision_type"], "session_plan")
            self.assertEqual(latest["decision"], "tasks_selected")

    def test_data_contract_reports_generated_at_and_latest_session(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            older = root / "sessions/day-2-20260602T000000Z"
            newer = root / "sessions/day-10-20260610T000000Z"
            write_json(
                older / "outcome.json",
                {
                    "day": 2,
                    "ts": "2026-06-02T00:00:00Z",
                    "tasks_attempted": 0,
                    "tasks_succeeded": 0,
                },
            )
            write_json(older / "state/summary.json", {"latest_gnomes": {"coding_log_score": 0.2}})
            write_json(
                newer / "outcome.json",
                {
                    "day": 10,
                    "ts": "2026-06-10T00:00:00Z",
                    "tasks_attempted": 0,
                    "tasks_succeeded": 0,
                    "source_sha": "fedcba9876543210fedcba9876543210fedcba98",
                    "source_ref": "main",
                    "github_sha": "0123456789abcdef0123456789abcdef01234567",
                    "github_ref": "refs/heads/main",
                    "github_ref_name": "main",
                },
            )
            write_json(newer / "state/summary.json", {"latest_gnomes": {"coding_log_score": 0.9}})

            data = build(root / "sessions", root / "out")

            self.assertRegex(data["generated_at"], r"^\d{4}-\d{2}-\d{2}T")
            self.assertEqual([session["id"] for session in data["sessions"]], [older.name, newer.name])
            self.assertEqual(data["aggregate"]["latest_session_id"], newer.name)
            self.assertEqual(data["aggregate"]["latest_ts"], "2026-06-10T00:00:00Z")
            self.assertEqual(data["aggregate"]["latest_gnomes"]["coding_log_score"], 0.9)
            self.assertEqual(data["sessions"][1]["source_sha"], "fedcba9876543210fedcba9876543210fedcba98")
            self.assertEqual(data["sessions"][1]["source_ref"], "main")
            self.assertEqual(data["sessions"][1]["github_sha"], "0123456789abcdef0123456789abcdef01234567")
            self.assertEqual(data["sessions"][1]["github_ref_name"], "main")
            html = (root / "out/index.html").read_text(encoding="utf-8")
            self.assertIn("function sessionSourceLine", html)
            self.assertIn("source revision not recorded", html)

    def test_derives_operational_state_capture_from_trace_quality(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(session / "outcome.json", {"day": 1, "tasks_attempted": 0, "tasks_succeeded": 0})
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 2,
                    "event_counts": {"RunStarted": 1, "PatchEvaluated": 1},
                    "latest_gnomes": {"state_capture_coverage": 1.0},
                    "gnome_keys": ["state_capture_coverage"],
                    "evals": [{"suite": "log-feedback", "gnomes": {"state_capture_coverage": 1.0}}],
                },
            )

            data = build(root / "sessions", root / "out")
            latest = data["sessions"][0]["latest_eval"]["gnomes"]

            self.assertEqual(latest["state_capture_coverage"], 1.0)
            self.assertEqual(latest["state_operational_capture_coverage"], 0.0)
            self.assertIn("state_operational_capture_coverage", data["aggregate"]["gnome_keys"])

    def test_dashboard_prioritizes_operational_state_capture(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(session / "outcome.json", {"day": 1, "tasks_attempted": 0, "tasks_succeeded": 0})
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 2,
                    "event_counts": {"RunStarted": 1, "PatchEvaluated": 1},
                    "latest_gnomes": {
                        "coding_log_score": 0.8,
                        "task_success_rate": 0.0,
                        "workflow_success_rate": 1.0,
                        "state_capture_coverage": 1.0,
                    },
                    "gnome_keys": [
                        "coding_log_score",
                        "task_success_rate",
                        "workflow_success_rate",
                        "state_capture_coverage",
                    ],
                    "evals": [
                        {
                            "suite": "log-feedback",
                            "gnomes": {
                                "coding_log_score": 0.8,
                                "task_success_rate": 0.0,
                                "workflow_success_rate": 1.0,
                                "state_capture_coverage": 1.0,
                            },
                        }
                    ],
                },
            )

            build(root / "sessions", root / "out")
            html = (root / "out/index.html").read_text(encoding="utf-8")
            priority_block = html.split("const priorityGnomes = [", 1)[1].split("];", 1)[0]

            self.assertLess(
                priority_block.index('"state_operational_capture_coverage"'),
                priority_block.index('"state_capture_coverage"'),
            )
            self.assertIn('return "-"', html)

    def test_derives_work_summary_and_gnome_history(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "build_ok": True,
                    "test_ok": True,
                    "tasks_attempted": 2,
                    "tasks_succeeded": 2,
                    "reverted": False,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 5,
                    "event_counts": {"FileEdited": 1, "CommandStarted": 1},
                    "latest_gnomes": {
                        "coding_log_score": 0.8,
                        "coding_log_available": True,
                        "evolution_friction_count": 2,
                    },
                    "gnome_keys": [
                        "coding_log_score",
                        "coding_log_available",
                        "evolution_friction_count",
                    ],
                    "evals": [
                        {
                            "eval_id": "eval-1",
                            "suite": "log-feedback",
                            "status": "passed",
                            "score": 0.8,
                            "gnomes": {
                                "coding_log_score": 0.8,
                                "coding_log_available": True,
                                "closed_loop_fix_rate": None,
                                "evolution_friction_count": 2,
                            },
                        }
                    ],
                    "patches": [{"patch_id": "patch-1"}],
                    "decisions": [{"decision": "promote", "eligible": True}],
                    "task_lineage": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "task_title": "Improve dashboard",
                            "status": "completed",
                            "source_files": ["scripts/build_evolution_dashboard.py"],
                            "commit_shas": ["abc123"],
                            "eval": {"verdict": "PASS"},
                            "gnome_deltas": {"coding_log_score": 0.1},
                        }
                    ],
                    "blockers": [],
                },
            )
            write_events(
                session / "state/events.jsonl",
                [
                    {
                        "kind": "FileEdited",
                        "payload": {"path": "scripts/build_evolution_dashboard.py"},
                    },
                    {
                        "kind": "CommandStarted",
                        "payload": {"command": "cargo test"},
                    },
                ],
            )
            (session / "transcripts").mkdir()
            (session / "transcripts/plan.log").write_text("plan\n", encoding="utf-8")
            (session / "transcripts/task_01_attempt1.log").write_text(
                "╭─ Turn 1 ─╮\n▶ read src/lib.rs\n╭─ Turn 4 ─╮\n▶ cargo test\n",
                encoding="utf-8",
            )
            (session / "tasks/task_01").mkdir(parents=True)
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {
                        "planning_failed": False,
                        "task_count": 1,
                        "selected_task_count": 1,
                        "assessment_present": True,
                    },
                    "selected_tasks": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "title": "Improve dashboard",
                            "files": ["scripts/build_evolution_dashboard.py"],
                            "issue": "none",
                            "origin": "planner",
                            "artifact_path": "tasks/task_01/task.md",
                            "quality": {"score": 1.0, "generic_self_improvement": False},
                        }
                    ],
                    "artifacts": {
                        "manifest": "tasks/manifest.json",
                        "assessment": "tasks/assessment.md",
                    },
                    "warnings": [],
                },
            )
            (session / "tasks/assessment.md").write_text("# Assessment\n", encoding="utf-8")
            (session / "tasks/task_01/task.md").write_text("Title: Improve dashboard\nFiles: scripts/build_evolution_dashboard.py\n", encoding="utf-8")
            (session / "tasks/task_01/attempts.jsonl").write_text(
                json.dumps(
                    {
                        "task_id": "task_01",
                        "phase": "implementation",
                        "attempt": 1,
                        "stage_name": "task_01_attempt1",
                        "transcript_path": "transcripts/task_01_attempt1.log",
                        "exit_code": 0,
                        "status": "completed",
                        "line_count": 2,
                    },
                    separators=(",", ":"),
                )
                + "\n",
                encoding="utf-8",
            )
            write_json(
                session / "tasks/task_01/eval_attempt_1.json",
                {
                    "task_id": "task_01",
                    "attempt": 1,
                    "status": "pass",
                    "verdict": "Verdict: PASS",
                    "reason": "Evaluator saw the dashboard evidence and checks passed.",
                    "transcript_path": "transcripts/eval_task1_attempt1.log",
                },
            )
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "task_title": "Improve dashboard",
                    "status": "completed",
                },
            )

            data = build(root / "sessions", root / "out")

            self.assertEqual(data["schema_version"], 2)
            self.assertEqual(
                data["gnome_numeric_keys"],
                [
                    "avg_task_turn_count",
                    "coding_log_score",
                    "evaluator_timeout_with_verdict_count",
                    "evaluator_unverified_count",
                    "evolution_friction_count",
                    "max_task_turn_count",
                    "session_success_rate",
                    "task_artifact_coverage",
                    "task_success_rate",
                    "task_unlanded_source_count",
                    "total_task_turn_count",
                ],
            )
            self.assertEqual(
                data["gnome_history"][0]["values"],
                {
                    "avg_task_turn_count": 4.0,
                    "coding_log_score": 0.8,
                    "evaluator_timeout_with_verdict_count": 0.0,
                    "evaluator_unverified_count": 0.0,
                    "evolution_friction_count": 2.0,
                    "max_task_turn_count": 4.0,
                    "session_success_rate": 1.0,
                    "task_artifact_coverage": 1.0,
                    "task_success_rate": 1.0,
                    "task_unlanded_source_count": 0.0,
                    "total_task_turn_count": 4.0,
                },
            )

            work = data["sessions"][0]["work_summary"]
            html = (root / "out/index.html").read_text(encoding="utf-8")
            self.assertIn("const planned = text((row.planned_files || [])", html)
            self.assertIn("const touched = text((row.touched_files || [])", html)
            self.assertNotIn("planned ${(row.planned_files || [])", html)
            self.assertIn("1/1 verified tasks", work["headline"])
            self.assertIn("outcome reported 2/2 tasks", work["headline"])
            self.assertEqual(work["edited_files"], ["scripts/build_evolution_dashboard.py"])
            self.assertEqual(work["commands"], ["cargo test"])
            self.assertEqual(work["transcripts"]["phase_counts"], {"plan": 1, "task": 1})
            self.assertEqual(work["transcripts"]["files"][1]["line_count"], 4)
            self.assertEqual(work["task_manifest"]["selected_task_count"], 1)
            self.assertFalse(work["task_manifest"]["planning_failed"])
            self.assertEqual(work["task_manifest"]["tasks"][0]["quality_score"], 1.0)
            self.assertEqual(work["task_artifacts"][0]["task_id"], "task_01")
            self.assertEqual(work["task_artifacts"][0]["attempt_count"], 1)
            self.assertEqual(work["task_artifacts"][0]["max_turn_count"], 4)
            self.assertEqual(work["task_artifacts"][0]["attempts"][0]["turn_count"], 4)
            self.assertEqual(work["task_artifacts"][0]["eval_statuses"], ["pass"])
            self.assertEqual(
                work["task_artifacts"][0]["evals"][0]["reason"],
                "Evaluator saw the dashboard evidence and checks passed.",
            )
            self.assertTrue(work["task_artifacts"][0]["has_outcome"])
            self.assertEqual(work["task_verification"]["verified_task_count"], 1)
            self.assertEqual(work["task_verification"]["unverified_task_count"], 0)
            self.assertEqual(work["causal_chains"][0]["task_id"], "task_01")
            self.assertEqual(work["causal_chains"][0]["planned_files"], ["scripts/build_evolution_dashboard.py"])
            self.assertEqual(work["causal_chains"][0]["commit_shas"], ["abc123"])
            self.assertIn("task.md", [row["name"] for row in work["task_artifacts"][0]["artifacts"]])
            self.assertEqual(work["patch_count"], 1)
            self.assertEqual(work["decision_count"], 1)
            self.assertEqual(work["task_lineage"][0]["task_id"], "task_01")
            self.assertEqual(work["task_lineage"][0]["gnome_deltas"], {"coding_log_score": 0.1})
            self.assertTrue(work["task_lineage"][0]["strict_success"])
            self.assertEqual(work["task_lineage"][0]["verification_status"], "strict_pass")
            self.assertEqual(work["task_lineage"][0]["verification_problems"], [])
            self.assertEqual(data["sessions"][0]["trace_quality"]["status"], "full")
            self.assertEqual(data["sessions"][0]["health"], "partial")
            self.assertEqual(data["sessions"][0]["health_reasons"], ["state lifecycle not observed"])
            self.assertIn("Health reason:", html)
            self.assertIn("function healthReasonsOf(session)", html)
            self.assertIn("function renderStateLifecycle(work)", html)
            self.assertIn("State lifecycle", html)
            self.assertIn("incomplete run", html)
            self.assertIn("unmatched completed model call", html)
            self.assertIn("Task evidence: raw outcome", html)
            self.assertIn("artifact bundles", html)
            self.assertIn('String(session.health || "").trim()', html)
            self.assertIn(
                "const harnessAttention = failedToolCount > 0 || failedCommandCount > failedToolCount || lifecycleMissing || lifecycleUnhealthy || assessmentMissing",
                html,
            )
            self.assertIn("const evalRows = (task.evals || [])", html)
            self.assertIn("evalRow.reason", html)

    def test_run_health_demotes_verified_sessions_with_harness_attention(self):
        base = {
            "tasks_attempted": 1,
            "tasks_succeeded": 1,
            "build_ok": True,
            "test_ok": True,
            "work_summary": {
                "task_manifest": {"planning_failed": False},
                "task_verification": {"task_count": 1, "verified_task_count": 1},
                "state_lifecycle": {"observed": True, "healthy": True},
            },
        }

        self.assertEqual(run_health(base), "passed")
        self.assertEqual(run_health_reasons(base), ["verified tasks and clean harness evidence"])

        cases = [
            (
                {"failed_tools": ["edit src/lib.rs: old_text did not match"]},
                ["1 failed tool action(s)"],
            ),
            (
                {"failed_command_count": 2, "failed_tool_count": 0},
                ["2 failed command/check(s)"],
            ),
            (
                {"state_lifecycle": {"observed": False, "healthy": False}},
                ["state lifecycle not observed"],
            ),
            (
                {
                    "state_lifecycle": {
                        "observed": True,
                        "healthy": False,
                        "runs": {"incomplete": 2, "unmatched_completed": 1},
                        "model_calls": {"incomplete": 3, "unmatched_completed": 4},
                    }
                },
                [
                    "state lifecycle unhealthy (runs incomplete 2; runs unmatched 1; model calls incomplete 3; model calls unmatched 4)"
                ],
            ),
            (
                {"assessment_artifact_present": False, "assessment_transcript_present": True},
                ["assessment artifact missing"],
            ),
            (
                {"assessment_artifact_present": False, "assessment_diagnostic_present": True},
                ["assessment artifact missing"],
            ),
        ]
        for work_update, expected_reasons in cases:
            with self.subTest(work_update=work_update):
                session = json.loads(json.dumps(base))
                session["work_summary"].update(work_update)
                self.assertEqual(run_health(session), "partial")
                self.assertEqual(run_health_reasons(session), expected_reasons)

        incomplete = json.loads(json.dumps(base))
        incomplete["work_summary"]["task_verification"] = {
            "task_count": 3,
            "verified_task_count": 2,
            "rows": [
                {
                    "task_id": "task_01",
                    "problems": [
                        "no_passing_verifier",
                        "source_edits_not_landed",
                        "no_landed_source_commit",
                        "no_planned_file_overlap",
                    ],
                },
                {
                    "task_id": "task_02",
                    "problems": [
                        "no_passing_verifier",
                        "no_touched_files",
                        "evaluator_timed_out_after_verdict",
                    ],
                },
            ],
        }
        incomplete["work_summary"]["failed_tools"] = ["read src/main.rs: missing"]
        self.assertEqual(run_health(incomplete), "partial")
        self.assertEqual(
            task_verification_problem_reasons(incomplete["work_summary"]),
            [
                "2 task(s) without passing verifier",
                "1 unlanded source task(s)",
                "1 task(s) without planned-file overlap",
                "1 task(s) without touched files",
                "1 evaluator timeout(s) after verdict",
            ],
        )
        self.assertEqual(
            run_health_reasons(incomplete),
            [
                "2/3 verified tasks",
                "2 task(s) without passing verifier",
                "1 unlanded source task(s)",
                "1 task(s) without planned-file overlap",
                "1 task(s) without touched files",
                "1 evaluator timeout(s) after verdict",
                "1 failed tool action(s)",
            ],
        )

    def test_state_pipeline_diagnostics_are_visible(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(session / "outcome.json", {"day": 1})
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 2,
                    "event_counts": {"RunStarted": 1, "CacheMetricsRecorded": 1},
                },
            )
            write_json(
                session / "state_replay.json",
                {"files_read": 3, "events_written": 20, "duplicates_skipped": 2},
            )
            write_json(
                session / "state/merge_state_delta.json",
                {
                    "live_events": 25,
                    "base_lines": 20,
                    "effective_base_lines": 20,
                    "baseline_shrunk": 0,
                    "delta_events": 5,
                    "added": 4,
                    "skipped_duplicate": 1,
                    "session_events_after": 6,
                },
            )
            (session / "state/append_state_event.log").write_text(
                "append_state_event.py failed once\n",
                encoding="utf-8",
            )

            data = build(root / "sessions", root / "out")
            current = data["sessions"][0]
            pipeline = current["work_summary"]["state_pipeline"]
            html = (root / "out/index.html").read_text(encoding="utf-8")

            self.assertEqual(current["trace_quality"]["operational_event_count"], 1)
            self.assertEqual(pipeline["replay_scope"], "audit_history")
            self.assertEqual(pipeline["replay_events_written"], 20)
            self.assertEqual(pipeline["merge_scope"], "live_delta")
            self.assertEqual(pipeline["merge_added_events"], 4)
            self.assertFalse(pipeline["merge_baseline_shrunk"])
            self.assertEqual(pipeline["merge_effective_base_lines"], 20)
            self.assertEqual(pipeline["append_problem_lines"], 1)
            self.assertIn("State pipeline", html)
            self.assertIn("audit replay", html)
            self.assertIn("merge_baseline_shrunk", html)

    def test_state_pipeline_explains_missing_live_merge_diagnostics(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(session / "outcome.json", {"day": 1})
            write_json(session / "state/summary.json", {})
            write_json(
                session / "state_replay.json",
                {"files_read": 3, "events_written": 20, "duplicates_skipped": 2},
            )

            data = build(root / "sessions", root / "out")
            pipeline = data["sessions"][0]["work_summary"]["state_pipeline"]
            html = (root / "out/index.html").read_text(encoding="utf-8")

            self.assertEqual(pipeline["replay_scope"], "audit_history")
            self.assertIsNone(pipeline["merge_scope"])
            self.assertIn("live delta merge diagnostics not recorded", html)

    def test_strict_verification_requires_landed_source_commit(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "build_ok": True,
                    "test_ok": True,
                    "tasks_attempted": 1,
                    "tasks_succeeded": 1,
                    "reverted": False,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 4,
                    "event_counts": {"RunStarted": 1, "RunCompleted": 1, "PatchEvaluated": 1},
                    "latest_gnomes": {"coding_log_score": 0.8},
                    "gnome_keys": ["coding_log_score"],
                    "evals": [{"suite": "log-feedback", "status": "passed", "score": 0.8}],
                    "task_lineage": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "task_title": "Unlanded source edit",
                            "status": "completed",
                            "planned_files": ["src/state.rs"],
                            "source_files": ["src/state.rs"],
                            "commit_shas": [],
                            "eval": {"verdict": "PASS"},
                        }
                    ],
                },
            )
            (session / "tasks/task_01").mkdir(parents=True)
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {"planning_failed": False, "task_count": 1, "selected_task_count": 1},
                    "selected_tasks": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "title": "Unlanded source edit",
                            "files": ["src/state.rs"],
                            "artifact_path": "tasks/task_01/task.md",
                            "quality": {"score": 1.0},
                        }
                    ],
                    "artifacts": {"manifest": "tasks/manifest.json"},
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

            data = build(root / "sessions", root / "out")
            session_data = data["sessions"][0]
            verification = session_data["work_summary"]["task_verification"]

            self.assertEqual(verification["verified_task_count"], 0)
            self.assertEqual(verification["unverified_task_count"], 1)
            self.assertIn("source_edits_not_landed", verification["rows"][0]["problems"])
            self.assertIn("no_landed_source_commit", verification["rows"][0]["problems"])
            task_state = session_data["work_summary"]["task_states"]["tasks"][0]
            self.assertEqual(task_state["state"], "unlanded_source_edits")
            self.assertTrue(task_state["attempted"])
            self.assertEqual(task_state["planned_files"], ["src/state.rs"])
            self.assertEqual(task_state["touched_files"], ["src/state.rs"])
            self.assertEqual(task_state["landed_commit_shas"], [])
            self.assertEqual(task_state["eval_attempt_count"], 1)
            self.assertIn("task_artifacts", task_state["evidence_sources"])
            self.assertIn("eval_attempts", task_state["evidence_sources"])
            self.assertEqual(session_data["latest_gnomes"]["task_success_rate"], 0.0)
            self.assertEqual(session_data["latest_gnomes"]["session_success_rate"], 0.0)
            self.assertEqual(session_data["latest_gnomes"]["task_unlanded_source_count"], 1)
            self.assertEqual(session_data["work_summary"]["unlanded_source_task_count"], 1)
            self.assertIn("1 unlanded source task(s)", session_data["work_summary"]["headline"])
            self.assertEqual(session_data["health"], "attention")
            states = json.loads((root / "out/states.json").read_text(encoding="utf-8"))
            self.assertEqual(states["schema_version"], 1)
            self.assertEqual(states["summary"]["task_count"], 1)
            self.assertEqual(states["summary"]["state_counts"]["unlanded_source_edits"], 1)
            self.assertEqual(states["sessions"][0]["tasks"][0]["state"], "unlanded_source_edits")

    def test_reverted_source_task_counts_as_unlanded_source_work(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "build_ok": True,
                    "test_ok": True,
                    "tasks_attempted": 1,
                    "tasks_succeeded": 0,
                    "reverted": False,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 4,
                    "event_counts": {"RunStarted": 1, "RunCompleted": 1},
                    "latest_gnomes": {"task_unlanded_source_count": 0, "coding_log_score": 0.8},
                    "gnome_keys": ["task_unlanded_source_count", "coding_log_score"],
                    "task_lineage": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "task_title": "Timed out source edit",
                            "status": "reverted",
                            "planned_files": ["src/state.rs"],
                            "source_files": ["src/state.rs"],
                            "commit_shas": [],
                            "eval": {"verdict": "TIMEOUT", "status": "timeout"},
                            "revert_reason": "Evaluator timed out without a verifier verdict",
                        }
                    ],
                },
            )
            (session / "tasks/task_01").mkdir(parents=True)
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {"planning_failed": False, "task_count": 1, "selected_task_count": 1},
                    "selected_tasks": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "title": "Timed out source edit",
                            "files": ["src/state.rs"],
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
                    "source_files": ["src/state.rs"],
                    "commit_shas": [],
                    "revert_reason": "Evaluator timed out without a verifier verdict",
                },
            )
            write_json(
                session / "tasks/task_01/eval_attempt_1.json",
                {"task_id": "task_01", "status": "timeout", "exit_code": 124},
            )

            data = build(root / "sessions", root / "out")
            session_data = data["sessions"][0]
            verification = session_data["work_summary"]["task_verification"]
            problems = verification["rows"][0]["problems"]

            self.assertEqual(verification["verified_task_count"], 0)
            self.assertEqual(verification["unverified_task_count"], 1)
            self.assertIn("source_edits_not_landed", problems)
            self.assertNotIn("no_landed_source_commit", problems)
            self.assertIn("no_passing_verifier", problems)
            self.assertEqual(session_data["latest_gnomes"]["task_unlanded_source_count"], 1)
            self.assertEqual(session_data["work_summary"]["unlanded_source_task_count"], 1)
            self.assertIn("1 unlanded source task(s)", session_data["work_summary"]["headline"])
            self.assertEqual(
                session_data["work_summary"]["evolution_suggestions"][0]["metric"],
                "evaluator_unverified_count",
            )
            self.assertTrue(
                any(
                    row.get("metric") == "task_unlanded_source_count"
                    and row.get("title") == "Make source-edit outcomes land or explain reverts"
                    for row in session_data["work_summary"]["evolution_suggestions"]
                )
            )

    def test_corrects_selected_but_unattempted_task_metrics(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "build_ok": True,
                    "test_ok": True,
                    "tasks_attempted": 2,
                    "tasks_succeeded": 0,
                    "reverted": False,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 4,
                    "event_counts": {"RunStarted": 1, "RunCompleted": 1, "PatchEvaluated": 1},
                    "latest_gnomes": {
                        "coding_log_score": 0.8,
                        "task_artifact_coverage": 1.5,
                    },
                    "gnome_keys": ["coding_log_score", "task_artifact_coverage"],
                    "evals": [
                        {
                            "suite": "log-feedback",
                            "status": "failed",
                            "score": 0.8,
                            "gnomes": {
                                "coding_log_score": 0.8,
                                "task_artifact_coverage": 1.5,
                            },
                        }
                    ],
                },
            )
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {"planning_failed": False, "task_count": 3, "selected_task_count": 3},
                    "selected_tasks": [
                        {"task_id": "task_01", "task_number": 1, "title": "Task 1", "artifact_path": "tasks/task_01/task.md"},
                        {"task_id": "task_02", "task_number": 2, "title": "Task 2", "artifact_path": "tasks/task_02/task.md"},
                        {"task_id": "task_03", "task_number": 3, "title": "Task 3", "artifact_path": "tasks/task_03/task.md"},
                    ],
                    "artifacts": {"manifest": "tasks/manifest.json"},
                },
            )
            for task_id in ("task_01", "task_02", "task_03"):
                write_json(session / f"tasks/{task_id}/decision.json", {"task_id": task_id})

            data = build(root / "sessions", root / "out")
            latest = data["sessions"][0]["latest_gnomes"]
            work = data["sessions"][0]["work_summary"]

            self.assertEqual(latest["task_unattempted_count"], 1)
            self.assertEqual(latest["task_artifact_coverage"], 1.0)
            self.assertEqual(data["sessions"][0]["latest_eval"]["gnomes"]["task_unattempted_count"], 1)
            self.assertEqual(data["sessions"][0]["latest_eval"]["gnomes"]["task_artifact_coverage"], 1.0)
            self.assertEqual(data["aggregate"]["latest_gnomes"]["task_unattempted_count"], 1)
            self.assertIn("task_unattempted_count", data["aggregate"]["gnome_keys"])
            self.assertIn("1 selected task(s) not attempted", work["headline"])
            suggestion_metrics = {row["metric"] for row in work["evolution_suggestions"]}
            self.assertIn("task_unattempted_count", suggestion_metrics)

    def test_timed_out_verdict_does_not_count_as_strict_pass(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "build_ok": True,
                    "test_ok": True,
                    "tasks_attempted": 1,
                    "tasks_succeeded": 1,
                    "reverted": False,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 4,
                    "event_counts": {"RunStarted": 1, "RunCompleted": 1, "PatchEvaluated": 1},
                    "latest_gnomes": {"coding_log_score": 0.8},
                    "gnome_keys": ["coding_log_score"],
                    "evals": [{"suite": "log-feedback", "status": "passed", "score": 0.8}],
                    "task_lineage": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "task_title": "Timeout after verdict",
                            "status": "completed",
                            "planned_files": ["src/state.rs"],
                            "source_files": ["src/state.rs"],
                            "commit_shas": ["abc123"],
                            "eval": {"verdict": "PASS"},
                        }
                    ],
                },
            )
            (session / "tasks/task_01").mkdir(parents=True)
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {"planning_failed": False, "task_count": 1, "selected_task_count": 1},
                    "selected_tasks": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "title": "Timeout after verdict",
                            "files": ["src/state.rs"],
                            "artifact_path": "tasks/task_01/task.md",
                            "quality": {"score": 1.0},
                        }
                    ],
                    "artifacts": {"manifest": "tasks/manifest.json"},
                },
            )
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "status": "completed",
                    "source_files": ["src/state.rs"],
                    "commit_shas": ["abc123"],
                },
            )
            write_json(
                session / "tasks/task_01/eval_attempt_1.json",
                {
                    "task_id": "task_01",
                    "status": "pass",
                    "exit_code": 124,
                    "verdict": "Verdict: PASS",
                    "verdict_file": "eval_attempt_1.md",
                },
            )

            data = build(root / "sessions", root / "out")
            session_data = data["sessions"][0]
            verification = session_data["work_summary"]["task_verification"]

            self.assertEqual(verification["verified_task_count"], 0)
            self.assertEqual(verification["unverified_task_count"], 1)
            self.assertIn("evaluator_timed_out_after_verdict", verification["rows"][0]["problems"])
            self.assertIn("no_passing_verifier", verification["rows"][0]["problems"])
            self.assertTrue(verification["rows"][0]["latest_eval_attempt"]["timed_out_after_verdict"])
            self.assertEqual(verification["rows"][0]["latest_eval_attempt"]["exit_code"], 124)
            self.assertEqual(session_data["latest_gnomes"]["task_success_rate"], 0.0)
            self.assertEqual(session_data["latest_gnomes"]["session_success_rate"], 0.0)
            self.assertEqual(session_data["latest_gnomes"]["evaluator_timeout_with_verdict_count"], 1)
            self.assertEqual(session_data["health"], "attention")

    def test_task_lineage_gnome_snapshots_use_corrected_strict_metrics(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "build_ok": True,
                    "test_ok": True,
                    "tasks_attempted": 1,
                    "tasks_succeeded": 1,
                    "reverted": False,
                },
            )
            stale_gnomes = {
                "coding_log_score": 0.8,
                "session_success_rate": 1.0,
                "task_success_rate": 0.5,
            }
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 4,
                    "event_counts": {"RunStarted": 1, "RunCompleted": 1},
                    "latest_gnomes": stale_gnomes,
                    "gnome_keys": list(stale_gnomes),
                    "evals": [{"suite": "log-feedback", "status": "passed", "score": 0.8}],
                    "task_lineage": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "task_title": "Stale strict metric",
                            "status": "completed",
                            "planned_files": ["src/state.rs"],
                            "source_files": ["src/state.rs"],
                            "commit_shas": [],
                            "eval": {"verdict": "PASS"},
                            "gnome_metrics": stale_gnomes,
                            "gnome_deltas": {
                                "session_success_rate": 1.0,
                                "task_success_rate": -0.5,
                            },
                        }
                    ],
                },
            )
            (session / "tasks/task_01").mkdir(parents=True)
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {"planning_failed": False, "task_count": 1, "selected_task_count": 1},
                    "selected_tasks": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "title": "Stale strict metric",
                            "files": ["src/state.rs"],
                            "artifact_path": "tasks/task_01/task.md",
                            "quality": {"score": 1.0},
                        }
                    ],
                    "artifacts": {"manifest": "tasks/manifest.json"},
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

            data = build(root / "sessions", root / "out")
            session_data = data["sessions"][0]
            lineage = session_data["work_summary"]["task_lineage"][0]

            self.assertEqual(session_data["latest_gnomes"]["task_success_rate"], 0.0)
            self.assertEqual(session_data["latest_gnomes"]["session_success_rate"], 0.0)
            self.assertEqual(lineage["gnome_metrics"]["task_success_rate"], 0.0)
            self.assertEqual(lineage["gnome_metrics"]["session_success_rate"], 0.0)
            self.assertEqual(lineage["gnome_metrics"]["task_unlanded_source_count"], 1)
            self.assertEqual(lineage["gnome_deltas"]["task_success_rate"], -1.0)
            self.assertEqual(lineage["gnome_deltas"]["session_success_rate"], 0.0)
            self.assertEqual(
                lineage["gnome_corrections"]["task_success_rate"],
                {"from": 0.5, "to": 0.0},
            )
            self.assertEqual(
                session_data["latest_eval"]["score"],
                session_data["latest_gnomes"]["coding_log_score"],
            )
            self.assertEqual(
                session_data["work_summary"]["latest_eval_score"],
                session_data["latest_gnomes"]["coding_log_score"],
            )
            html = (root / "out/index.html").read_text(encoding="utf-8")
            self.assertIn("corrected gnome(s)", html)
            self.assertIn("function summarizeGnomeMovement", html)
            self.assertIn("Task success", html)
            self.assertIn("metricDeltaValue", html)

    def test_cache_ratio_without_token_evidence_is_marked_unverified(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "tasks_attempted": 0,
                    "tasks_succeeded": 0,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "deepseek_cache_hit_ratio": 0.91,
                        "deepseek_cache_hit_tokens": None,
                        "deepseek_cache_miss_tokens": None,
                    },
                    "gnome_keys": [
                        "deepseek_cache_hit_ratio",
                        "deepseek_cache_hit_tokens",
                        "deepseek_cache_miss_tokens",
                    ],
                    "evals": [
                        {
                            "suite": "log-feedback",
                            "status": "passed",
                            "gnomes": {
                                "deepseek_cache_hit_ratio": 0.91,
                                "deepseek_cache_hit_tokens": None,
                                "deepseek_cache_miss_tokens": None,
                            },
                        }
                    ],
                },
            )

            data = build(root / "sessions", root / "out")
            session_data = data["sessions"][0]
            latest = session_data["latest_gnomes"]

            self.assertIsNone(latest["deepseek_cache_hit_ratio"])
            self.assertEqual(latest["deepseek_cache_ratio_unverified_count"], 1)
            self.assertIsNone(session_data["latest_eval"]["gnomes"]["deepseek_cache_hit_ratio"])
            self.assertEqual(
                session_data["latest_eval"]["gnome_corrections"]["deepseek_cache_hit_ratio"],
                {"from": 0.91, "to": None},
            )
            self.assertNotIn("deepseek_cache_hit_ratio", data["gnome_history"][0]["values"])
            self.assertEqual(data["gnome_history"][0]["values"]["deepseek_cache_ratio_unverified_count"], 1.0)
            self.assertIn("deepseek_cache_ratio_unverified_count", data["aggregate"]["gnome_keys"])
            self.assertIn("deepseek_cache_ratio_unverified_count", data["gnome_numeric_keys"])
            html = (root / "out/index.html").read_text(encoding="utf-8")
            self.assertIn("Cache evidence", html)
            self.assertIn("token evidence was missing", html)
            self.assertIn("CacheMetricsRecorded events", html)

    def test_cache_prose_ratio_is_marked_unverified(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "tasks_attempted": 0,
                    "tasks_succeeded": 0,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "deepseek_cache_hit_ratio": None,
                        "deepseek_cache_hit_tokens": None,
                        "deepseek_cache_miss_tokens": None,
                        "deepseek_cache_prose_mention_count": 2,
                    },
                    "gnome_keys": [
                        "deepseek_cache_hit_ratio",
                        "deepseek_cache_hit_tokens",
                        "deepseek_cache_miss_tokens",
                        "deepseek_cache_prose_mention_count",
                    ],
                    "evals": [
                        {
                            "suite": "log-feedback",
                            "status": "failed",
                            "gnomes": {
                                "deepseek_cache_hit_ratio": None,
                                "deepseek_cache_hit_tokens": None,
                                "deepseek_cache_miss_tokens": None,
                                "deepseek_cache_prose_mention_count": 2,
                            },
                        }
                    ],
                },
            )

            data = build(root / "sessions", root / "out")
            session_data = data["sessions"][0]
            latest = session_data["latest_gnomes"]

            self.assertIsNone(latest["deepseek_cache_hit_ratio"])
            self.assertEqual(latest["deepseek_cache_ratio_unverified_count"], 2)
            self.assertEqual(
                session_data["latest_eval"]["gnome_corrections"]["deepseek_cache_ratio_unverified_count"],
                {"from": None, "to": 2},
            )
            self.assertEqual(data["gnome_history"][0]["values"]["deepseek_cache_ratio_unverified_count"], 2.0)
            html = (root / "out/index.html").read_text(encoding="utf-8")
            self.assertIn("DeepSeek cache ratio report(s) were withheld", html)

    def test_missing_deepseek_cache_metric_events_are_claimed(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "tasks_attempted": 0,
                    "tasks_succeeded": 0,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "deepseek_cache_hit_ratio": None,
                        "deepseek_cache_hit_tokens": None,
                        "deepseek_cache_miss_tokens": None,
                    },
                    "gnome_keys": [
                        "deepseek_cache_hit_ratio",
                        "deepseek_cache_hit_tokens",
                        "deepseek_cache_miss_tokens",
                    ],
                },
            )
            write_events(
                session / "state/events.jsonl",
                [
                    {
                        "kind": "ModelCallCompleted",
                        "payload": {
                            "model": "deepseek-v4-pro",
                            "input_tokens": 100,
                            "output_tokens": 20,
                            "cache_read_tokens": 50,
                        },
                    }
                ],
            )

            data = build(root / "sessions", root / "out")
            html = (root / "out/index.html").read_text(encoding="utf-8")
            claims = json.loads((root / "out/claims.json").read_text(encoding="utf-8"))
            cache_claim = next(
                claim
                for claim in claims["sessions"][0]["claims"]
                if claim["name"] == "deepseek_cache_ratio_is_token_backed_or_marked_unverified"
            )

            latest = data["sessions"][0]["latest_gnomes"]
            self.assertEqual(latest["deepseek_cache_metric_missing_count"], 1)
            self.assertEqual(cache_claim["status"], "missing")
            self.assertEqual(cache_claim["actual"]["expected_metric_events"], 1)
            self.assertEqual(cache_claim["actual"]["metric_events"], 0)
            self.assertIn("Completed DeepSeek model calls", cache_claim["detail"])
            self.assertIn("Missing cache metric events", html)
            self.assertIn("completed DeepSeek model call", html)

    def test_deepseek_cache_metrics_are_backfilled_from_state_events(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "tasks_attempted": 0,
                    "tasks_succeeded": 0,
                },
            )
            write_json(session / "state/summary.json", {"latest_gnomes": {}, "gnome_keys": []})
            write_events(
                session / "state/events.jsonl",
                [
                    {
                        "kind": "ModelCallCompleted",
                        "payload": {
                            "_yoyo": {"run_id": "run-cache"},
                            "model": "deepseek-v4-pro",
                            "input_tokens": 20,
                            "cache_read_tokens": 80,
                        },
                    },
                    {
                        "kind": "CacheMetricsRecorded",
                        "payload": {
                            "_yoyo": {"run_id": "run-cache"},
                            "model": "deepseek-v4-pro",
                            "prompt_cache_hit_tokens": 80,
                            "prompt_cache_miss_tokens": 20,
                            "cache_hit_ratio": 0.8,
                        },
                    },
                ],
            )

            data = build(root / "sessions", root / "out")
            claims = json.loads((root / "out/claims.json").read_text(encoding="utf-8"))
            cache_claim = next(
                claim
                for claim in claims["sessions"][0]["claims"]
                if claim["name"] == "deepseek_cache_ratio_is_token_backed_or_marked_unverified"
            )

            session_data = data["sessions"][0]
            latest = session_data["latest_gnomes"]
            work = session_data["work_summary"]
            self.assertEqual(work["deepseek_cache_hit_tokens"], 80)
            self.assertEqual(work["deepseek_cache_miss_tokens"], 20)
            self.assertEqual(work["deepseek_cache_hit_ratio"], 0.8)
            self.assertEqual(latest["deepseek_cache_hit_tokens"], 80)
            self.assertEqual(latest["deepseek_cache_miss_tokens"], 20)
            self.assertEqual(latest["deepseek_cache_hit_ratio"], 0.8)
            self.assertEqual(latest["deepseek_cache_metric_expected_count"], 1)
            self.assertEqual(latest["deepseek_cache_metric_event_count"], 1)
            self.assertEqual(latest["deepseek_cache_metric_missing_count"], 0)
            self.assertEqual(cache_claim["status"], "proven")
            self.assertEqual(cache_claim["session_id"], "day-1")
            self.assertTrue(
                all(claim["session_id"] == "day-1" for claim in claims["sessions"][0]["claims"])
            )

    def test_incomplete_deepseek_model_calls_are_claimed(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "tasks_attempted": 0,
                    "tasks_succeeded": 0,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "deepseek_cache_hit_ratio": None,
                        "deepseek_cache_hit_tokens": None,
                        "deepseek_cache_miss_tokens": None,
                    },
                    "gnome_keys": [
                        "deepseek_cache_hit_ratio",
                        "deepseek_cache_hit_tokens",
                        "deepseek_cache_miss_tokens",
                    ],
                },
            )
            write_events(
                session / "state/events.jsonl",
                [
                    {
                        "kind": "ModelCallStarted",
                        "payload": {"_yoyo": {"run_id": "run-incomplete"}, "model": "deepseek-v4-pro"},
                    },
                    {
                        "kind": "FileEdited",
                        "payload": {
                            "_yoyo": {"run_id": "run-incomplete"},
                            "path": "/home/runner/work/yyds-harness/yyds-harness/journals/JOURNAL.md",
                        },
                    }
                ],
            )

            data = build(root / "sessions", root / "out")
            html = (root / "out/index.html").read_text(encoding="utf-8")
            claims = json.loads((root / "out/claims.json").read_text(encoding="utf-8"))
            model_claim = next(
                claim
                for claim in claims["sessions"][0]["claims"]
                if claim["name"] == "deepseek_model_call_lifecycle_balanced"
            )
            cache_claim = next(
                claim
                for claim in claims["sessions"][0]["claims"]
                if claim["name"] == "deepseek_cache_ratio_is_token_backed_or_marked_unverified"
            )

            latest = data["sessions"][0]["latest_gnomes"]
            lifecycle = data["sessions"][0]["work_summary"]["state_lifecycle"]
            self.assertEqual(latest["deepseek_model_call_started_count"], 1)
            self.assertEqual(latest["deepseek_model_call_completed_count"], 0)
            self.assertEqual(latest["deepseek_model_call_incomplete_count"], 1)
            self.assertFalse(lifecycle["balanced"])
            self.assertEqual(lifecycle["model_calls"]["started"], 1)
            self.assertEqual(lifecycle["model_calls"]["completed"], 0)
            self.assertEqual(lifecycle["model_calls"]["incomplete"], 1)
            self.assertEqual(lifecycle["model_calls"]["incomplete_runs"][0]["last_event"]["kind"], "FileEdited")
            self.assertEqual(latest["deepseek_cache_metric_expected_count"], 0)
            self.assertEqual(latest["deepseek_cache_metric_missing_count"], 0)
            self.assertEqual(model_claim["status"], "missing")
            self.assertEqual(model_claim["actual"]["started"], 1)
            self.assertEqual(model_claim["actual"]["completed"], 0)
            self.assertEqual(model_claim["actual"]["incomplete"], 1)
            self.assertEqual(model_claim["actual"]["unmatched_completed"], 0)
            self.assertEqual(model_claim["actual"]["incomplete_runs"][0]["run_id"], "run-incomplete")
            self.assertEqual(model_claim["actual"]["incomplete_runs"][0]["last_event"]["kind"], "FileEdited")
            self.assertEqual(
                model_claim["actual"]["incomplete_runs"][0]["last_event"]["path"],
                "journals/JOURNAL.md",
            )
            self.assertIn("do not pair by run_id", model_claim["detail"])
            self.assertEqual(cache_claim["status"], "observed")
            self.assertEqual(cache_claim["actual"]["expected_metric_events"], 0)
            self.assertIn("No trusted DeepSeek cache ratio", cache_claim["detail"])
            self.assertIn("Incomplete DeepSeek model calls", html)
            self.assertIn("started without a matching ModelCallCompleted", html)

    def test_model_call_lifecycle_pairs_by_run_id_not_just_counts(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(session / "outcome.json", {"day": 1, "tasks_attempted": 0, "tasks_succeeded": 0})
            write_json(session / "state/summary.json", {"latest_gnomes": {}, "gnome_keys": []})
            write_events(
                session / "state/events.jsonl",
                [
                    {
                        "kind": "ModelCallStarted",
                        "payload": {"_yoyo": {"run_id": "run-a"}, "model": "deepseek-v4-pro"},
                    },
                    {
                        "kind": "RunCompleted",
                        "payload": {
                            "_yoyo": {"run_id": "run-a"},
                            "status": "error",
                            "error": "exit code 1",
                            "error_detail": "empty_input",
                        },
                    },
                    {
                        "kind": "ModelCallCompleted",
                        "payload": {
                            "_yoyo": {"run_id": "run-b"},
                            "model": "deepseek-v4-pro",
                            "input_tokens": 10,
                        },
                    },
                ],
            )

            data = build(root / "sessions", root / "out")
            claims = json.loads((root / "out/claims.json").read_text(encoding="utf-8"))
            model_claim = next(
                claim
                for claim in claims["sessions"][0]["claims"]
                if claim["name"] == "deepseek_model_call_lifecycle_balanced"
            )
            work = data["sessions"][0]["work_summary"]

            self.assertEqual(work["deepseek_model_call_incomplete_count"], 1)
            self.assertEqual(work["deepseek_model_call_unmatched_completed_count"], 1)
            self.assertEqual(work["deepseek_model_call_incomplete_runs"][0]["run_id"], "run-a")
            self.assertEqual(work["deepseek_model_call_incomplete_runs"][0]["error_detail"], "empty_input")
            self.assertEqual(work["deepseek_model_call_incomplete_runs"][0]["last_event"]["kind"], "RunCompleted")
            self.assertEqual(work["deepseek_model_call_incomplete_runs"][0]["last_event"]["error_detail"], "empty_input")
            self.assertEqual(model_claim["status"], "missing")
            self.assertEqual(model_claim["actual"]["started"], 1)
            self.assertEqual(model_claim["actual"]["completed"], 1)
            self.assertEqual(model_claim["actual"]["incomplete"], 1)
            self.assertEqual(model_claim["actual"]["unmatched_completed"], 1)
            self.assertEqual(model_claim["actual"]["incomplete_runs"][0]["run_id"], "run-a")
            self.assertEqual(model_claim["actual"]["unmatched_completed_runs"], ["run-b"])

    def test_model_call_lifecycle_pairs_wrapped_yoyo_run_ids(self):
        work = summarize_events_for_work(
            [
                {
                    "payload": {
                        "_yoyo": {"event_type": "ModelCallStarted", "run_id": "run-wrapped"},
                        "value": {"model": "deepseek-v4-pro"},
                    }
                },
                {
                    "payload": {
                        "_yoyo": {"event_type": "ModelCallCompleted", "run_id": "run-wrapped"},
                        "value": {"model": "deepseek-v4-pro", "status": "completed"},
                    }
                },
            ]
        )

        self.assertEqual(work["deepseek_model_call_started_count"], 1)
        self.assertEqual(work["deepseek_model_call_completed_count"], 1)
        self.assertEqual(work["deepseek_model_call_incomplete_count"], 0)
        self.assertEqual(work["deepseek_model_call_unmatched_completed_count"], 0)
        self.assertTrue(work["state_lifecycle"]["observed"])
        self.assertTrue(work["state_lifecycle"]["balanced"])
        self.assertTrue(work["state_lifecycle"]["healthy"])

    def test_state_lifecycle_summary_structures_run_and_model_evidence(self):
        work = summarize_events_for_work(
            [
                {
                    "kind": "RunStarted",
                    "run_id": "run-open",
                    "payload": {"status": "started"},
                },
                {
                    "kind": "ModelCallStarted",
                    "run_id": "run-open",
                    "payload": {"model": "deepseek-v4-pro"},
                },
                {
                    "kind": "FileEdited",
                    "run_id": "run-open",
                    "payload": {"path": "/home/runner/work/yyds-harness/yyds-harness/src/lib.rs"},
                },
                {
                    "kind": "RunCompleted",
                    "run_id": "run-closed-without-start",
                    "payload": {"status": "completed"},
                },
            ]
        )

        lifecycle = work["state_lifecycle"]
        self.assertTrue(lifecycle["observed"])
        self.assertFalse(lifecycle["balanced"])
        self.assertFalse(lifecycle["healthy"])
        self.assertEqual(lifecycle["runs"]["started"], 1)
        self.assertEqual(lifecycle["runs"]["completed"], 1)
        self.assertEqual(lifecycle["runs"]["incomplete"], 1)
        self.assertEqual(lifecycle["runs"]["unmatched_completed"], 1)
        self.assertEqual(lifecycle["runs"]["incomplete_runs"][0]["run_id"], "run-open")
        self.assertEqual(lifecycle["runs"]["incomplete_runs"][0]["last_event"]["kind"], "FileEdited")
        self.assertEqual(lifecycle["runs"]["incomplete_runs"][0]["last_event"]["path"], "src/lib.rs")
        self.assertEqual(lifecycle["runs"]["unmatched_completed_runs"], ["run-closed-without-start"])
        self.assertEqual(lifecycle["model_calls"]["started"], 1)
        self.assertEqual(lifecycle["model_calls"]["completed"], 0)
        self.assertEqual(lifecycle["model_calls"]["incomplete"], 1)
        self.assertEqual(lifecycle["model_calls"]["incomplete_runs"][0]["run_id"], "run-open")

    def test_state_lifecycle_without_events_is_balanced_but_not_healthy(self):
        lifecycle = summarize_events_for_work([])["state_lifecycle"]

        self.assertFalse(lifecycle["observed"])
        self.assertTrue(lifecycle["balanced"])
        self.assertFalse(lifecycle["healthy"])

    def test_run_lifecycle_imbalance_is_claimed_separately_from_model_calls(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(session / "outcome.json", {"ts": "2026-01-01T00:00:00Z"})
            write_json(session / "state/summary.json", {"latest_gnomes": {}, "gnome_keys": []})
            write_events(
                session / "state/events.jsonl",
                [
                    {
                        "kind": "RunStarted",
                        "run_id": "run-open",
                        "payload": {"status": "started"},
                    },
                    {
                        "kind": "ModelCallStarted",
                        "run_id": "run-model-balanced",
                        "payload": {"model": "deepseek-v4-pro"},
                    },
                    {
                        "kind": "ModelCallCompleted",
                        "run_id": "run-model-balanced",
                        "payload": {"model": "deepseek-v4-pro", "status": "completed"},
                    },
                ],
            )

            data = build(root / "sessions", root / "out", repo_root=root)
            claims = json.loads((root / "out/claims.json").read_text(encoding="utf-8"))
            html = (root / "out/index.html").read_text(encoding="utf-8")
            session_claims = {claim["name"]: claim for claim in claims["sessions"][0]["claims"]}
            latest = data["sessions"][0]["latest_gnomes"]

            self.assertEqual(latest["state_run_started_count"], 1)
            self.assertEqual(latest["state_run_completed_count"], 0)
            self.assertEqual(latest["state_run_incomplete_count"], 1)
            self.assertEqual(latest["state_run_unmatched_completed_count"], 0)
            self.assertEqual(latest["deepseek_model_call_incomplete_count"], 0)
            self.assertIn("runs incomplete ${Number(runs.incomplete || 0)}", html)
            self.assertIn("model calls unmatched ${Number(modelCalls.unmatched_completed || 0)}", html)
            self.assertEqual(session_claims["deepseek_model_call_lifecycle_balanced"]["status"], "proven")
            self.assertEqual(session_claims["state_run_lifecycle_balanced"]["status"], "missing")
            self.assertEqual(
                session_claims["state_run_lifecycle_balanced"]["actual"]["incomplete_runs"][0]["run_id"],
                "run-open",
            )

    def test_abnormal_completed_model_calls_are_observed_not_clean(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(session / "outcome.json", {"ts": "2026-01-01T00:00:00Z"})
            write_json(
                session / "state/summary.json",
                {
                    "generated_at": "2026-01-01T00:00:00Z",
                    "evals": [
                        {
                            "suite": "log-feedback",
                            "gnomes": {
                                "deepseek_model_call_started_count": 1,
                                "deepseek_model_call_completed_count": 1,
                                "deepseek_model_call_abnormal_completed_count": 1,
                                "deepseek_model_call_incomplete_count": 0,
                                "deepseek_model_call_unmatched_completed_count": 0,
                            },
                        }
                    ],
                },
            )
            write_events(
                session / "state/events.jsonl",
                [
                    {
                        "kind": "ModelCallStarted",
                        "run_id": "run-stream-closed",
                        "payload": {"model": "deepseek-v4-pro"},
                    },
                    {
                        "kind": "ModelCallCompleted",
                        "run_id": "run-stream-closed",
                        "payload": {
                            "model": "deepseek-v4-pro",
                            "status": "stream_closed_without_agent_end",
                            "error_detail": "event_channel_closed_before_agent_end",
                        },
                    },
                ],
            )

            build(root / "sessions", root / "out", repo_root=root)
            data = json.loads((root / "out/data.json").read_text(encoding="utf-8"))
            claims = json.loads((root / "out/claims.json").read_text(encoding="utf-8"))

            work = data["sessions"][0]["work_summary"]
            latest = data["sessions"][0]["latest_gnomes"]
            model_claim = next(
                claim
                for claim in claims["sessions"][0]["claims"]
                if claim["name"] == "deepseek_model_call_lifecycle_balanced"
            )
            self.assertEqual(work["deepseek_model_call_abnormal_completed_count"], 1)
            self.assertEqual(latest["deepseek_model_call_abnormal_completed_count"], 1)
            self.assertEqual(work["deepseek_model_call_incomplete_count"], 0)
            self.assertEqual(model_claim["status"], "observed")
            self.assertEqual(model_claim["actual"]["abnormal_completed"], 1)
            self.assertEqual(
                model_claim["actual"]["abnormal_completed_runs"][0]["status"],
                "stream_closed_without_agent_end",
            )

    def test_manifest_files_backfilled_from_task_artifact(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "build_ok": True,
                    "test_ok": True,
                    "tasks_attempted": 1,
                    "tasks_succeeded": 1,
                    "reverted": False,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 5,
                    "event_counts": {"RunStarted": 1, "RunCompleted": 1},
                    "latest_gnomes": {"coding_log_score": 0.8},
                    "gnome_keys": ["coding_log_score"],
                    "evals": [{"suite": "log-feedback", "status": "passed", "score": 0.8}],
                },
            )
            (session / "tasks/task_01").mkdir(parents=True)
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {"planning_failed": False, "task_count": 1, "selected_task_count": 1},
                    "selected_tasks": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "title": "Wire panic hook",
                            "files": [],
                            "artifact_path": "tasks/task_01/task.md",
                            "quality": {"score": 1.0},
                        }
                    ],
                    "artifacts": {"manifest": "tasks/manifest.json"},
                },
            )
            (session / "tasks/task_01/task.md").write_text(
                "\n".join(
                    [
                        "Title: Wire panic hook",
                        "",
                        "Files: src/state.rs",
                        "",
                        "Issue: none",
                        "",
                        "Origin: planner",
                        "",
                        "Objective: Make panic evidence visible.",
                        "",
                        "Success Criteria:",
                        "- Panic diagnostic is stashed.",
                        "",
                        "Verification:",
                        "- cargo test",
                    ]
                )
                + "\n",
                encoding="utf-8",
            )
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "status": "completed",
                    "source_files": ["src/state.rs"],
                    "commit_shas": ["abc123"],
                },
            )
            write_json(
                session / "tasks/task_01/eval_attempt_1.json",
                {"task_id": "task_01", "status": "pass", "verdict": "Verdict: PASS"},
            )

            data = build(root / "sessions", root / "out")
            work = data["sessions"][0]["work_summary"]

            self.assertEqual(work["task_manifest"]["tasks"][0]["files"], ["src/state.rs"])
            self.assertEqual(
                work["task_verification"]["rows"][0]["planned_files"],
                ["src/state.rs"],
            )
            self.assertEqual(work["task_verification"]["verified_task_count"], 1)

    def test_derives_source_changes_from_matching_session_commits(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            repo = root / "repo"
            repo.mkdir()
            subprocess.run(["git", "-C", str(repo), "init"], check=True, stdout=subprocess.DEVNULL)
            subprocess.run(
                ["git", "-C", str(repo), "config", "user.email", "test@example.com"],
                check=True,
            )
            subprocess.run(["git", "-C", str(repo), "config", "user.name", "Test"], check=True)
            (repo / "src").mkdir()
            (repo / "session_plan").mkdir()
            (repo / ".yoyo").mkdir()
            (repo / "src/lib.rs").write_text("pub fn ok() {}\n", encoding="utf-8")
            (repo / "session_plan/task_01.md").write_text("plan\n", encoding="utf-8")
            (repo / ".yoyo/context-semantic-index.json").write_text("{}\n", encoding="utf-8")
            subprocess.run(["git", "-C", str(repo), "add", "src/lib.rs"], check=True)
            subprocess.run(
                ["git", "-C", str(repo), "commit", "-m", "Day 1 (00:00): implement source"],
                check=True,
                stdout=subprocess.DEVNULL,
            )
            subprocess.run(
                ["git", "-C", str(repo), "add", "session_plan/task_01.md", ".yoyo/context-semantic-index.json"],
                check=True,
            )
            subprocess.run(
                ["git", "-C", str(repo), "commit", "-m", "Day 1 (00:00): session wrap-up"],
                check=True,
                stdout=subprocess.DEVNULL,
            )

            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "session_time": "00:00",
                    "tasks_attempted": 1,
                    "tasks_succeeded": 1,
                },
            )
            write_json(session / "state/summary.json", {})
            write_events(
                session / "state/events.jsonl",
                [{"kind": "FileEdited", "payload": {"path": "journals/JOURNAL.md"}}],
            )

            data = build(root / "sessions", root / "out", repo)

            work = data["sessions"][0]["work_summary"]
            self.assertEqual(work["source_changed_files"], ["src/lib.rs"])
            self.assertEqual(work["touched_source_files"], ["src/lib.rs"])
            self.assertEqual(work["edited_files"], ["journals/JOURNAL.md"])
            self.assertIn("1 source file(s) changed", work["headline"])
            self.assertEqual(work["source_patch_count"], 1)
            self.assertEqual(work["landed_patch_count"], 1)
            self.assertEqual(work["state_patch_count"], 0)
            self.assertEqual(work["landed_commit_count"], 2)
            self.assertEqual(work["source_commit_count"], 1)
            self.assertEqual(work["bookkeeping_commit_count"], 1)
            self.assertEqual(len(work["source_commits"]), 1)
            self.assertEqual(len(work["bookkeeping_commits"]), 1)
            self.assertEqual(work["commits"][0]["subject"], "Day 1 (00:00): implement source")
            self.assertEqual(work["commits"][1]["source_files"], [])
            self.assertEqual(work["source_commits"][0]["subject"], "Day 1 (00:00): implement source")
            self.assertEqual(work["bookkeeping_commits"][0]["subject"], "Day 1 (00:00): session wrap-up")

    def test_non_source_edits_are_labeled_as_bookkeeping(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {"day": 1, "tasks_attempted": 1, "tasks_succeeded": 0},
            )
            write_json(session / "state/summary.json", {})
            write_events(
                session / "state/events.jsonl",
                [
                    {"kind": "FileEdited", "payload": {"path": "journals/JOURNAL.md"}},
                    {"kind": "FileEdited", "payload": {"path": "session_plan/task_01.md"}},
                ],
            )

            data = build(root / "sessions", root / "out")
            work = data["sessions"][0]["work_summary"]

            self.assertEqual(work["edited_files"], ["journals/JOURNAL.md", "session_plan/task_01.md"])
            self.assertEqual(work["touched_source_files"], [])
            self.assertIn("2 evidence/bookkeeping file(s) edited", work["headline"])
            self.assertNotIn("2 file(s) edited", work["headline"])
            html = (root / "out/index.html").read_text(encoding="utf-8")
            self.assertIn("const evidenceFiles = work.edited_files || []", html)
            self.assertIn("Evidence/bookkeeping edits", html)
            self.assertIn("No source changes recorded.", html)
            self.assertNotIn("work.edited_files);", html)

    def test_feedback_only_trace_is_explicit(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "github_run_id": "123",
                    "github_run_attempt": "2",
                    "tasks_attempted": 1,
                    "tasks_succeeded": 1,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 1,
                    "event_counts": {"PatchEvaluated": 1},
                    "evals": [{"suite": "log-feedback", "eval_id": "log-feedback-123-1"}],
                },
            )
            write_events(
                session / "state/events.jsonl",
                [{"event_type": "PatchEvaluated", "payload": {"suite": "log-feedback"}}],
            )

            data = build(root / "sessions", root / "out")

            current = data["sessions"][0]
            self.assertEqual(current["github_run_id"], "123")
            self.assertEqual(current["github_run_attempt"], "2")
            self.assertEqual(current["trace_quality"]["status"], "feedback_only")
            self.assertEqual(current["trace_quality"]["trace_event_count"], 0)
            self.assertEqual(data["aggregate"]["feedback_only_sessions"], 1)
            self.assertEqual(data["aggregate"]["full_trace_sessions"], 0)

    def test_task_lineage_only_trace_is_thin_not_full(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(session / "outcome.json", {"day": 1})
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 7,
                    "event_counts": {
                        "RunStarted": 2,
                        "RunCompleted": 2,
                        "DecisionRecorded": 1,
                        "TaskLineageLinked": 1,
                        "PatchEvaluated": 1,
                    },
                    "evals": [{"suite": "log-feedback", "eval_id": "log-feedback-123-1"}],
                },
            )

            data = build(root / "sessions", root / "out")
            current = data["sessions"][0]

            self.assertEqual(current["trace_quality"]["status"], "thin")
            self.assertEqual(current["trace_quality"]["label"], "task-lineage trace")
            self.assertEqual(current["trace_quality"]["operational_event_count"], 0)
            self.assertEqual(current["trace_quality"]["operational_capture_coverage"], 0.0)
            self.assertEqual(current["trace_quality"]["task_lineage_event_count"], 1)
            self.assertEqual(current["trace_quality"]["task_lineage_capture_coverage"], 1.0)
            self.assertEqual(data["aggregate"]["full_trace_sessions"], 0)
            self.assertEqual(data["aggregate"]["lifecycle_trace_sessions"], 0)
            self.assertIn("Operational traces", (root / "out/index.html").read_text(encoding="utf-8"))

    def test_task_lineage_eval_is_enriched_from_task_artifacts(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {"day": 1, "tasks_attempted": 1, "tasks_succeeded": 1, "build_ok": True, "test_ok": True},
            )
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 3,
                    "event_counts": {"RunStarted": 1, "RunCompleted": 1, "PatchEvaluated": 1},
                    "task_lineage": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "task_title": "Timeout eval should be visible",
                            "status": "completed",
                            "planned_files": ["src/state.rs"],
                            "source_files": ["src/state.rs"],
                        }
                    ],
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
                            "title": "Timeout eval should be visible",
                            "files": ["src/state.rs"],
                            "artifact_path": "tasks/task_01/task.md",
                        }
                    ],
                },
            )
            (session / "tasks/task_01").mkdir(parents=True)
            (session / "tasks/task_01/task.md").write_text("Title: Timeout eval should be visible\n", encoding="utf-8")
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "task_title": "Timeout eval should be visible",
                    "status": "completed",
                    "source_files": ["src/state.rs"],
                },
            )
            write_json(
                session / "tasks/task_01/eval_attempt_1.json",
                {
                    "task_id": "task_01",
                    "attempt": 1,
                    "status": "timeout",
                    "exit_code": 124,
                    "transcript_path": "transcripts/eval_task1_attempt1.log",
                },
            )

            data = build(root / "sessions", root / "out")
            lineage = data["sessions"][0]["work_summary"]["task_lineage"][0]
            causal = data["sessions"][0]["work_summary"]["causal_chains"][0]
            verification = data["sessions"][0]["work_summary"]["task_verification"]

            self.assertEqual(lineage["eval"]["verdict"], "TIMEOUT")
            self.assertEqual(lineage["eval"]["status"], "timeout")
            self.assertIn("timed out", lineage["eval"]["reason"])
            self.assertEqual(lineage["eval"]["transcript_path"], "transcripts/eval_task1_attempt1.log")
            self.assertFalse(lineage["strict_success"])
            self.assertEqual(lineage["verification_status"], "strict_failed")
            self.assertIn("no_passing_verifier", lineage["verification_problems"])
            self.assertFalse(causal["strict_success"])
            self.assertEqual(causal["verification_status"], "strict_failed")
            self.assertIn("no_passing_verifier", causal["verification_problems"])
            self.assertEqual(verification["verified_task_count"], 0)
            self.assertIn("no_passing_verifier", verification["rows"][0]["problems"])

    def test_missing_assessment_is_visible_in_session_work_summary(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {"day": 1, "tasks_attempted": 1, "tasks_succeeded": 0, "build_ok": True, "test_ok": True},
            )
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 2,
                    "event_counts": {"RunStarted": 1, "RunCompleted": 1},
                    "task_lineage": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "task_title": "Fallback task",
                            "status": "started",
                            "planned_files": ["src/lib.rs"],
                        }
                    ],
                },
            )
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {
                        "planning_failed": False,
                        "task_count": 1,
                        "selected_task_count": 1,
                        "assessment_present": False,
                        "assessment_missing_present": True,
                    },
                    "selected_tasks": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "title": "Fallback task",
                            "files": ["src/lib.rs"],
                            "artifact_path": "tasks/task_01/task.md",
                        }
                    ],
                    "artifacts": {
                        "manifest": "tasks/manifest.json",
                        "assessment": None,
                        "assessment_missing": "tasks/assessment_missing.md",
                    },
                },
            )
            (session / "tasks/task_01").mkdir(parents=True)
            (session / "tasks/task_01/task.md").write_text("Title: Fallback task\n", encoding="utf-8")
            (session / "tasks/assessment_missing.md").write_text(
                "# Assessment Missing - Day 1\n",
                encoding="utf-8",
            )
            (session / "transcripts").mkdir()
            (session / "transcripts/assess.log").write_text("assessment phase ran\n", encoding="utf-8")
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "task_title": "Fallback task",
                    "status": "reverted",
                    "revert_reason": "Task scope mismatch: task produced no git-visible file changes",
                    "planned_files": ["src/lib.rs"],
                    "touched_files": [],
                    "source_files": [],
                },
            )

            data = build(root / "sessions", root / "out")
            work = data["sessions"][0]["work_summary"]
            html = (root / "out/index.html").read_text(encoding="utf-8")

            self.assertIn("assessment artifact missing (assess transcript present)", work["headline"])
            self.assertFalse(work["assessment_artifact_present"])
            self.assertTrue(work["assessment_transcript_present"])
            self.assertFalse(work["task_manifest"]["assessment_present"])
            self.assertTrue(work["task_manifest"]["assessment_missing_present"])
            self.assertEqual(
                work["task_manifest"]["artifacts"]["assessment_missing"],
                "tasks/assessment_missing.md",
            )
            self.assertEqual(
                work["task_artifacts"][0]["revert_reason"],
                "Task scope mismatch: task produced no git-visible file changes",
            )
            self.assertEqual(
                work["task_verification"]["rows"][0]["revert_reason"],
                "Task scope mismatch: task produced no git-visible file changes",
            )
            self.assertEqual(
                work["task_lineage"][0]["revert_reason"],
                "Task scope mismatch: task produced no git-visible file changes",
            )
            self.assertIn("assessment artifact", html)
            self.assertIn("revert_reason", html)

            claims = json.loads((root / "out/claims.json").read_text(encoding="utf-8"))
            assessment_claim = next(
                claim
                for claim in claims["sessions"][0]["claims"]
                if claim["name"] == "assessment_artifact_and_transcript_state"
            )
            self.assertEqual(assessment_claim["status"], "observed")
            self.assertTrue(assessment_claim["actual"]["diagnostic_present"])
            self.assertIn("diagnostic artifact", assessment_claim["detail"])
            self.assertIn("tasks/assessment_missing.md", assessment_claim["evidence"])

    def test_assessment_transcript_without_manifest_marks_artifact_missing(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "tasks_attempted": 1,
                    "tasks_succeeded": 1,
                    "build_ok": True,
                    "test_ok": True,
                },
            )
            write_json(session / "state/summary.json", {"latest_gnomes": {}, "gnome_keys": []})
            transcript_dir = session / "transcripts"
            transcript_dir.mkdir(parents=True)
            transcript_dir.joinpath("assess.log").write_text("assessment phase ran\n", encoding="utf-8")

            data = build(root / "sessions", root / "out")
            claims = json.loads((root / "out/claims.json").read_text(encoding="utf-8"))
            work = data["sessions"][0]["work_summary"]
            assessment_claim = next(
                claim
                for claim in claims["sessions"][0]["claims"]
                if claim["name"] == "assessment_artifact_and_transcript_state"
            )

            self.assertFalse(work["assessment_artifact_present"])
            self.assertTrue(work["assessment_transcript_present"])
            self.assertFalse(work["assessment_diagnostic_present"])
            self.assertIn("assessment artifact missing (assess transcript present)", work["headline"])
            self.assertEqual(assessment_claim["status"], "observed")
            self.assertEqual(assessment_claim["actual"]["artifact_present"], False)
            self.assertTrue(assessment_claim["actual"]["transcript_present"])
            self.assertIn("assessment artifact is missing", assessment_claim["detail"])
            self.assertEqual(assessment_claim["evidence"], ["transcripts/assess.log"])

    def test_assessment_claim_evidence_excludes_absent_diagnostic_artifact(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "tasks_attempted": 1,
                    "tasks_succeeded": 1,
                    "build_ok": True,
                    "test_ok": True,
                },
            )
            write_json(session / "state/summary.json", {"latest_gnomes": {}, "gnome_keys": []})
            write_json(
                session / "tasks/manifest.json",
                {
                    "assessment_present": True,
                    "assessment_missing_present": False,
                    "artifacts": {
                        "assessment": "tasks/assessment.md",
                        "assessment_missing": None,
                    },
                    "tasks": [],
                },
            )
            (session / "tasks").mkdir(parents=True, exist_ok=True)
            (session / "tasks/assessment.md").write_text("# Assessment\n", encoding="utf-8")
            transcript_dir = session / "transcripts"
            transcript_dir.mkdir(parents=True)
            transcript_dir.joinpath("assess.log").write_text("assessment phase ran\n", encoding="utf-8")

            build(root / "sessions", root / "out")
            claims = json.loads((root / "out/claims.json").read_text(encoding="utf-8"))
            assessment_claim = next(
                claim
                for claim in claims["sessions"][0]["claims"]
                if claim["name"] == "assessment_artifact_and_transcript_state"
            )

            self.assertEqual(assessment_claim["status"], "proven")
            self.assertEqual(
                assessment_claim["evidence"],
                ["tasks/assessment.md", "transcripts/assess.log"],
            )
            self.assertNotIn("tasks/assessment_missing.md", assessment_claim["evidence"])

    def test_transcript_actions_fill_work_evidence_when_state_events_are_sparse(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "tasks_attempted": 1,
                    "tasks_succeeded": 0,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 3,
                    "event_counts": {"RunStarted": 1, "RunCompleted": 1, "PatchEvaluated": 1},
                    "evals": [{"suite": "log-feedback", "status": "failed", "score": 0.4}],
                },
            )
            (session / "transcripts").mkdir(parents=True)
            (session / "transcripts/task_01_attempt1.log").write_text(
                "\n".join(
                    [
                        "  ▶ read src/state.rs:30..80 ✓ (5ms)",
                        "  ▶ search 'fn build_why_report' in src/commands_state.rs ✗ (17ms)",
                        "  ▶ edit src/state.rs (3 → 5 lines)",
                        "  ▶ $ cd /home/runner/work/yyds-harness/yyds-harness && cargo test state::tests::panic_hook ✓ (141ms)",
                        "  ✗ Watch failed: `cargo clippy --all-targets -- -D warnings && cargo test`",
                    ]
                )
                + "\n",
                encoding="utf-8",
            )

            data = build(root / "sessions", root / "out")
            session_data = data["sessions"][0]
            work = session_data["work_summary"]

            self.assertEqual(work["read_files"], ["src/state.rs", "src/commands_state.rs"])
            self.assertEqual(work["edited_files"], ["src/state.rs"])
            self.assertEqual(work["touched_source_files"], ["src/state.rs"])
            self.assertEqual(
                work["commands"],
                [
                    "cargo test state::tests::panic_hook",
                    "cargo clippy --all-targets -- -D warnings && cargo test",
                ],
            )
            self.assertIn(
                "cargo clippy --all-targets -- -D warnings && cargo test",
                work["failed_commands"],
            )
            self.assertIn(
                "search 'fn build_why_report' in src/commands_state.rs",
                work["failed_tools"],
            )
            self.assertIn("1 source file(s) touched", work["headline"])
            self.assertIn("1 failed tool action(s)", work["headline"])
            self.assertIn("2 failed command/check(s)", work["labels"])
            self.assertIn("2 command/check signal(s)", work["labels"])
            self.assertIn("2 failed command/check(s)", session_data["health_reasons"])
            self.assertEqual(work["command_count"], 2)
            self.assertEqual(work["failed_command_count"], 2)
            self.assertEqual(work["failed_tool_count"], 1)
            self.assertEqual(work["read_file_count"], 2)
            self.assertEqual(work["edited_file_count"], 1)
            self.assertEqual(work["touched_source_file_count"], 1)
            self.assertEqual(work["transcript_actions"]["command_count"], 2)
            self.assertEqual(work["transcript_actions"]["failed_command_count"], 2)
            self.assertEqual(work["transcript_actions"]["failed_tool_count"], 1)
            self.assertEqual(
                work["failed_tool_summary"]["category_counts"],
                {"search_tool_error": 1},
            )
            self.assertEqual(work["transcript_actions"]["read_file_count"], 2)
            self.assertEqual(work["transcript_actions"]["edited_file_count"], 1)
            self.assertEqual(session_data["latest_gnomes"]["tool_error_count"], 1)
            self.assertEqual(work["transcript_actions"]["edited_files"], ["src/state.rs"])
            states = json.loads((root / "out/states.json").read_text(encoding="utf-8"))
            self.assertEqual(
                states["sessions"][0]["tool_failures"]["summary"]["category_counts"],
                {"search_tool_error": 1},
            )
            html = (root / "out/index.html").read_text(encoding="utf-8")
            data_json = (root / "out/data.json").read_text(encoding="utf-8")
            self.assertIn("tool fails", html)
            self.assertIn("failed checks", html)
            self.assertIn("search_tool_error", data_json)

    def test_file_evidence_totals_use_uncapped_counts(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "tasks_attempted": 0,
                    "tasks_succeeded": 0,
                },
            )
            transcript_dir = session / "transcripts"
            transcript_dir.mkdir(parents=True)
            transcript_dir.joinpath("task_01_attempt1.log").write_text(
                "\n".join(
                    [f"  ▶ read src/read_{index}.rs ✓ (5ms)" for index in range(14)]
                    + [f"  ▶ edit journals/evidence_{index}.md ✓ (5ms)" for index in range(13)]
                )
                + "\n",
                encoding="utf-8",
            )

            data = build(root / "sessions", root / "out")
            work = data["sessions"][0]["work_summary"]
            html = (root / "out/index.html").read_text(encoding="utf-8")

            self.assertEqual(len(work["read_files"]), 12)
            self.assertEqual(work["read_file_count"], 14)
            self.assertEqual(len(work["edited_files"]), 12)
            self.assertEqual(work["edited_file_count"], 13)
            self.assertEqual(work["touched_source_file_count"], 0)
            self.assertIn("13 evidence/bookkeeping file(s) edited", work["headline"])
            self.assertIn("readFileCount", html)
            self.assertIn("sampleCountLabel(work.read_files, readFileCount)", html)

    def test_failed_tool_totals_use_uncapped_counts(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "tasks_attempted": 1,
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
                "\n".join(
                    f"  ▶ search 'needle_{index}' in src/file_{index}.rs ✗ (17ms)"
                    for index in range(10)
                )
                + "\n",
                encoding="utf-8",
            )

            data = build(root / "sessions", root / "out")
            session_data = data["sessions"][0]
            work = session_data["work_summary"]
            audit = session_data["state_gnome_audit"]
            claims = json.loads((root / "out/claims.json").read_text(encoding="utf-8"))
            failed_tool_claim = next(
                claim
                for claim in claims["sessions"][0]["claims"]
                if claim["name"] == "failed_tool_actions_match_tool_error_gnome"
            )

            self.assertEqual(len(work["failed_tools"]), 8)
            self.assertEqual(work["failed_tool_count"], 10)
            self.assertIn("10 failed tool action(s)", work["headline"])
            self.assertIn("10 failed tool action(s)", session_data["health_reasons"])
            self.assertEqual(session_data["latest_gnomes"]["tool_error_count"], 10)
            self.assertEqual(work["corrected_gnome_lessons"][0]["kind"], "tool_error")
            self.assertEqual(work["corrected_gnome_lessons"][0]["count"], 10)
            self.assertEqual(work["corrected_gnome_lessons"][0]["source"], "corrected_gnomes")
            self.assertGreaterEqual(audit["correction_count"], 1)
            self.assertEqual(audit["corrections_by_source"].get("transcripts"), 1)
            tool_error_audit = next(row for row in audit["corrections"] if row["key"] == "tool_error_count")
            self.assertEqual(
                tool_error_audit,
                {
                    "key": "tool_error_count",
                    "from": 0,
                    "to": 10,
                    "source": "transcripts",
                    "reason": "transcript action parsing corrected the gnome",
                },
            )
            self.assertEqual(failed_tool_claim["expected"]["minimum_count"], 10)
            self.assertEqual(len(failed_tool_claim["evidence"]), 8)
            html = (root / "out/index.html").read_text(encoding="utf-8")
            data_json = (root / "out/data.json").read_text(encoding="utf-8")
            self.assertIn("Feedback lessons", html)
            self.assertIn("Corrected gnome pressure", html)
            self.assertIn("State/gnome audit", html)
            self.assertIn("failed tool actions were recovered from transcripts", data_json)
            self.assertIn("transcript action parsing corrected the gnome", data_json)

    def test_corrected_lessons_use_failed_tool_categories(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "tasks_attempted": 1,
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
                "\n".join(
                    [
                        "  ▶ search 'fn handle_run\\(' in src/commands_eval.rs ✗ (17ms)",
                        "      │ Search error: grep: Unmatched ( or \\(",
                    ]
                )
                + "\n",
                encoding="utf-8",
            )

            data = build(root / "sessions", root / "out")
            work = data["sessions"][0]["work_summary"]
            category_lesson = work["corrected_gnome_lessons"][0]

            self.assertEqual(work["failed_tool_summary"]["category_counts"], {"search_regex_error": 1})
            self.assertEqual(category_lesson["kind"], "tool_error_search_regex")
            self.assertEqual(category_lesson["source"], "failed_tool_summary")
            self.assertEqual(category_lesson["metric"], "failed_tool_summary.search_regex_error")
            self.assertEqual(category_lesson["count"], 1)
            self.assertIn("rg --fixed-strings", category_lesson["action"])
            self.assertNotIn(
                "tool_error",
                [
                    row["kind"]
                    for row in work["corrected_gnome_lessons"][1:]
                ],
            )

    def test_log_feedback_top_lessons_are_exposed(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "tasks_attempted": 0,
                    "tasks_succeeded": 0,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "evals": [
                        {
                            "eval_id": "log-feedback-1",
                            "suite": "log-feedback",
                            "status": "failed",
                            "score": 0.4,
                            "gnomes": {"coding_log_score": 0.4},
                        }
                    ],
                    "latest_gnomes": {"coding_log_score": 0.4},
                    "gnome_keys": ["coding_log_score"],
                },
            )
            write_json(
                session / "log_feedback.json",
                {
                    "metrics": {"coding_log_score": 0.4},
                    "top_lessons": [
                        {
                            "kind": "search_error",
                            "fingerprint": "search tool or grep produced an error",
                            "action": "prefer the hardened search tool and scoped literal searches",
                        }
                    ],
                },
            )

            data = build(root / "sessions", root / "out")
            session_data = data["sessions"][0]
            lessons = session_data["latest_eval"]["top_lessons"]

            self.assertEqual(lessons[0]["kind"], "search_error")
            self.assertEqual(
                session_data["work_summary"]["log_feedback_top_lessons"][0]["action"],
                "prefer the hardened search tool and scoped literal searches",
            )
            html = (root / "out/index.html").read_text(encoding="utf-8")
            data_json = (root / "out/data.json").read_text(encoding="utf-8")
            self.assertIn("Feedback lessons", html)
            self.assertIn("Raw log-feedback", html)
            self.assertIn("search tool or grep produced an error", data_json)

    def test_build_writes_structured_claims_projection(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "tasks_attempted": 0,
                    "tasks_succeeded": 0,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "tool_error_count": 0,
                        "deepseek_cache_hit_ratio": None,
                        "deepseek_cache_prose_mention_count": 1,
                    },
                    "gnome_keys": [
                        "tool_error_count",
                        "deepseek_cache_hit_ratio",
                        "deepseek_cache_prose_mention_count",
                    ],
                },
            )
            (session / "transcripts").mkdir(parents=True)
            (session / "transcripts/task_01_attempt1.log").write_text(
                "  ▶ search 'missing symbol' in src/lib.rs ✗ (17ms)\n",
                encoding="utf-8",
            )

            build(root / "sessions", root / "out")
            claims = json.loads((root / "out/claims.json").read_text(encoding="utf-8"))
            session_claims = {
                row["name"]: row
                for row in claims["sessions"][0]["claims"]
            }

            self.assertEqual(claims["schema_version"], 1)
            self.assertEqual(claims["summary"]["session_count"], 1)
            self.assertEqual(
                session_claims["failed_tool_actions_match_tool_error_gnome"]["status"],
                "proven",
            )
            self.assertEqual(
                session_claims["failed_tool_actions_match_tool_error_gnome"]["expected"],
                {"minimum_count": 1},
            )
            self.assertEqual(
                session_claims["failed_tool_actions_match_tool_error_gnome"]["actual"]["count"],
                1,
            )
            self.assertEqual(
                session_claims["failed_tool_actions_match_tool_error_gnome"]["actual"]["raw"],
                1,
            )
            self.assertEqual(
                session_claims["failed_tool_actions_match_tool_error_gnome"]["actual"]["evidence_count"],
                1,
            )
            self.assertEqual(
                session_claims["deepseek_cache_ratio_is_token_backed_or_marked_unverified"]["status"],
                "proven",
            )
            self.assertEqual(
                session_claims["deepseek_cache_ratio_is_token_backed_or_marked_unverified"]["actual"]["unverified_count"],
                1,
            )
            self.assertEqual(
                session_claims["failed_tool_summary_counts_match_failures"]["status"],
                "proven",
            )
            self.assertEqual(
                session_claims["failed_tool_summary_counts_match_failures"]["actual"]["category_counts"],
                {"search_tool_error": 1},
            )
            self.assertEqual(
                session_claims["task_state_counts_match_rows"]["status"],
                "proven",
            )
            self.assertEqual(
                session_claims["task_state_counts_match_rows"]["actual"]["task_count"],
                0,
            )

    def test_count_claim_observes_zero_evidence_when_metric_is_absent(self):
        claim = count_claim(
            "unlanded_source_tasks_match_gnome",
            0,
            None,
            [],
            "Tasks with source edits and no landed source commit should be reflected in task_unlanded_source_count.",
        )

        self.assertEqual(claim["status"], "observed")
        self.assertEqual(claim["expected"], {"minimum_count": 0})
        self.assertEqual(claim["actual"]["count"], None)
        self.assertEqual(claim["actual"]["raw"], None)
        self.assertEqual(claim["actual"]["evidence_count"], 0)
        self.assertIn("No matching evidence was found", claim["detail"])

    def test_session_sort_is_natural_by_day_and_timestamp(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            sessions = root / "sessions"
            for name, day, ts in [
                ("day-99-20260607T224346Z", 99, "2026-06-07T22:43:46Z"),
                ("day-100-20260608T003408Z", 100, "2026-06-08T00:34:08Z"),
                ("day-98-20260606T163045Z", 98, "2026-06-06T16:30:45Z"),
            ]:
                write_json(sessions / name / "outcome.json", {"day": day, "ts": ts})
                write_json(sessions / name / "state/summary.json", {})

            data = build(sessions, root / "out")

            self.assertEqual(
                [session["id"] for session in data["sessions"]],
                [
                    "day-98-20260606T163045Z",
                    "day-99-20260607T224346Z",
                    "day-100-20260608T003408Z",
                ],
            )

    def test_aggregate_gnome_keys_include_corrected_latest_gnomes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "build_ok": True,
                    "test_ok": True,
                    "tasks_attempted": 1,
                    "tasks_succeeded": 1,
                    "reverted": False,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "coding_log_score": 0.8,
                        "session_success_rate": 1.0,
                        "task_success_rate": 1.0,
                        "tasks_attempted": 3,
                        "tasks_succeeded": 3,
                    },
                    "gnome_keys": [
                        "coding_log_score",
                        "session_success_rate",
                        "task_success_rate",
                        "tasks_attempted",
                        "tasks_succeeded",
                    ],
                },
            )
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "task_title": "Unlanded task",
                    "status": "completed",
                    "planned_files": ["src/state.rs"],
                    "touched_files": ["src/state.rs"],
                    "source_files": ["src/state.rs"],
                    "commit_shas": [],
                },
            )
            write_json(
                session / "tasks/task_01/eval_attempt_1.json",
                {
                    "task_id": "task_01",
                    "status": "pass",
                    "exit_code": 0,
                    "verdict": "Verdict: PASS",
                },
            )
            write_json(
                session / "tasks/manifest.json",
                {
                    "selected_tasks": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "title": "Unlanded task",
                            "files": ["src/state.rs"],
                        }
                    ],
                    "artifacts": {"manifest": "tasks/manifest.json"},
                },
            )

            data = build(root / "sessions", root / "out")

            self.assertIn("coding_log_score", data["aggregate"]["gnome_keys"])
            self.assertIn("task_unlanded_source_count", data["aggregate"]["gnome_keys"])
            self.assertEqual(data["aggregate"]["latest_gnomes"]["task_unlanded_source_count"], 1)

    def test_dashboard_merges_log_feedback_metrics_missing_from_state_summary(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "build_ok": True,
                    "test_ok": True,
                    "tasks_attempted": 0,
                    "tasks_succeeded": 0,
                    "reverted": False,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {"coding_log_score": 0.8},
                    "gnome_keys": ["coding_log_score"],
                    "evals": [{"suite": "log-feedback", "gnomes": {"coding_log_score": 0.8}}],
                },
            )
            write_json(
                session / "log_feedback.json",
                {
                    "metrics": {
                        "coding_log_score": 0.7,
                        "state_live_baseline_shrink_count": 1,
                        "state_operational_capture_coverage": 1.0,
                        "evidence": ["not a gnome"],
                        "workflow_conclusion": "success",
                    }
                },
            )

            data = build(root / "sessions", root / "out")
            latest = data["sessions"][0]["latest_gnomes"]

            self.assertEqual(latest["state_live_baseline_shrink_count"], 1)
            self.assertEqual(latest["state_operational_capture_coverage"], 1.0)
            self.assertNotIn("evidence", latest)
            self.assertNotIn("workflow_conclusion", latest)
            self.assertEqual(data["gnome_history"][0]["values"]["state_live_baseline_shrink_count"], 1.0)
            self.assertIn("state_live_baseline_shrink_count", data["aggregate"]["gnome_keys"])
            self.assertEqual(data["sessions"][0]["latest_eval"]["gnomes"]["state_live_baseline_shrink_count"], 1)

    def test_dashboard_suppresses_legacy_projection_reset_as_state_shrink(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "build_ok": True,
                    "test_ok": True,
                    "tasks_attempted": 0,
                    "tasks_succeeded": 0,
                    "reverted": False,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "coding_log_score": 0.8,
                        "state_live_baseline_shrink_count": 1,
                    },
                    "gnome_keys": ["coding_log_score", "state_live_baseline_shrink_count"],
                    "evals": [
                        {
                            "suite": "log-feedback",
                            "gnomes": {
                                "coding_log_score": 0.8,
                                "state_live_baseline_shrink_count": 1,
                            },
                        }
                    ],
                },
            )
            write_json(
                session / "state/merge_state_delta.json",
                {
                    "baseline_shrunk": 1,
                    "base_lines": 1885,
                    "effective_base_lines": 0,
                    "live_events": 64,
                    "added": 64,
                    "session_events_before": 4,
                },
            )

            data = build(root / "sessions", root / "out")
            latest = data["sessions"][0]["latest_gnomes"]

            self.assertEqual(latest["state_live_baseline_shrink_count"], 0)
            self.assertEqual(data["sessions"][0]["latest_eval"]["gnomes"]["state_live_baseline_shrink_count"], 0)

    def test_dashboard_planning_failure_artifact_coverage_is_zero(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "build_ok": True,
                    "test_ok": True,
                    "tasks_attempted": 0,
                    "tasks_succeeded": 0,
                    "reverted": False,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "coding_log_score": 0.7,
                        "session_success_rate": 1.0,
                        "task_artifact_coverage": 1.0,
                    },
                    "gnome_keys": ["coding_log_score", "session_success_rate", "task_artifact_coverage"],
                    "evals": [
                        {
                            "suite": "log-feedback",
                            "gnomes": {
                                "coding_log_score": 0.7,
                                "session_success_rate": 1.0,
                                "task_artifact_coverage": 1.0,
                            },
                        }
                    ],
                },
            )
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {"planning_failed": True, "task_count": 0, "selected_task_count": 0},
                    "selected_tasks": [],
                    "warnings": ["planner_produced_no_task_files"],
                    "artifacts": {"manifest": "tasks/manifest.json", "planning_failure": "tasks/planning_failure.md"},
                },
            )

            data = build(root / "sessions", root / "out")
            latest = data["sessions"][0]["latest_gnomes"]

            self.assertEqual(latest["planner_no_task_count"], 1)
            self.assertEqual(latest["session_success_rate"], 0.0)
            self.assertEqual(latest["task_artifact_coverage"], 0.0)
            self.assertEqual(data["sessions"][0]["health"], "attention")

    def test_raw_outcome_tasks_without_strict_evidence_are_not_success(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "build_ok": True,
                    "test_ok": True,
                    "tasks_attempted": 3,
                    "tasks_succeeded": 3,
                    "reverted": False,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {"coding_log_score": 0.8},
                    "gnome_keys": ["coding_log_score"],
                },
            )

            data = build(root / "sessions", root / "out")
            session_data = data["sessions"][0]
            aggregate = data["aggregate"]

            self.assertEqual(session_data["health"], "attention")
            self.assertIn("3/3 raw outcome task(s)", session_data["work_summary"]["headline"])
            self.assertIn("missing strict task evidence", session_data["work_summary"]["labels"])
            self.assertIsNone(session_data["latest_gnomes"]["task_success_rate"])
            self.assertEqual(session_data["latest_gnomes"]["session_success_rate"], 0.0)
            self.assertEqual(session_data["latest_gnomes"]["raw_tasks_attempted"], 3)
            self.assertEqual(session_data["latest_gnomes"]["raw_tasks_succeeded"], 3)
            self.assertEqual(session_data["latest_gnomes"]["task_unverified_raw_attempt_count"], 3)
            self.assertEqual(session_data["latest_gnomes"]["task_unverified_raw_success_count"], 3)
            self.assertIn("task_unverified_raw_success_count", data["aggregate"]["gnome_keys"])
            self.assertNotIn("task_success_rate", data["gnome_history"][0]["values"])
            self.assertEqual(data["gnome_history"][0]["values"]["task_unverified_raw_success_count"], 3.0)
            self.assertEqual(aggregate["tasks_attempted"], 0)
            self.assertEqual(aggregate["tasks_succeeded"], 0)
            self.assertIsNone(aggregate["task_success_rate"])
            self.assertEqual(aggregate["raw_task_outcome_attempted"], 3)
            self.assertEqual(aggregate["raw_task_outcome_succeeded"], 3)
            self.assertEqual(aggregate["unverified_raw_task_outcome_attempted"], 3)
            self.assertEqual(aggregate["unverified_raw_task_outcome_succeeded"], 3)

    def test_task_artifacts_without_manifest_still_enter_strict_verification(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "build_ok": True,
                    "test_ok": True,
                    "tasks_attempted": 1,
                    "tasks_succeeded": 1,
                    "reverted": False,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {"coding_log_score": 0.8},
                    "gnome_keys": ["coding_log_score"],
                },
            )
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "task_title": "Historical task without manifest",
                    "status": "completed",
                    "source_files": ["src/state.rs"],
                    "commit_shas": [],
                },
            )
            write_json(
                session / "tasks/task_01/eval_attempt_1.json",
                {
                    "task_id": "task_01",
                    "status": "pass",
                    "exit_code": 0,
                    "verdict": "Verdict: PASS",
                },
            )

            data = build(root / "sessions", root / "out")
            verification = data["sessions"][0]["work_summary"]["task_verification"]

            self.assertEqual(verification["task_count"], 1)
            self.assertEqual(verification["verified_task_count"], 0)
            self.assertEqual(verification["unverified_task_count"], 1)
            self.assertEqual(verification["rows"][0]["task_id"], "task_01")
            self.assertIn("missing_planned_files", verification["rows"][0]["problems"])
            self.assertIn("no_landed_source_commit", verification["rows"][0]["problems"])
            self.assertEqual(data["aggregate"]["tasks_attempted"], 1)
            self.assertEqual(data["aggregate"]["tasks_succeeded"], 0)
            self.assertEqual(data["aggregate"]["unverified_raw_task_outcome_attempted"], 0)

    def test_missing_optional_artifacts_do_not_fail(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(session / "outcome.json", {"tasks_attempted": 0, "tasks_succeeded": 0})
            write_json(session / "state/summary.json", {})

            data = build(root / "sessions", root / "out")

            self.assertEqual(len(data["sessions"]), 1)
            self.assertFalse(data["sessions"][0]["work_summary"]["task_verification"]["all_verified"])
            self.assertEqual(data["gnome_history"][0]["values"], {})
            self.assertEqual(
                data["sessions"][0]["work_summary"]["headline"],
                "No detailed work signals captured",
            )


if __name__ == "__main__":
    unittest.main()
