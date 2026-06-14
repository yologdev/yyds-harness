#!/usr/bin/env python3
"""Tests for scripts/state_graph_tools.py."""

from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import state_graph_tools  # noqa: E402


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def write_jsonl(path: Path, rows: list[dict[str, object]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, separators=(",", ":")) + "\n")


class StateGraphTools(unittest.TestCase):
    def test_ordered_sessions_accepts_single_session_directory(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "session_staging"
            write_json(
                session / "log_feedback.json",
                {"metrics": {"provider_error_count": 2}},
            )

            self.assertEqual(state_graph_tools.ordered_sessions(session), [session])

    def test_replay_check_and_causal_chain(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_jsonl(
                session / "state/events.jsonl",
                [
                    {"event_type": "DecisionRecorded", "payload": {"decision": "tasks_selected"}},
                    {"event_type": "PatchEvaluated", "payload": {"suite": "log-feedback"}},
                ],
            )
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 2,
                    "event_counts": {"DecisionRecorded": 1, "PatchEvaluated": 1},
                    "latest_gnomes": {
                        "task_artifact_coverage": 1.0,
                        "evaluator_unverified_count": 1,
                        "max_task_turn_count": 30,
                    },
                    "task_lineage": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "task_title": "Improve eval",
                            "status": "completed",
                            "source_files": ["src/eval.rs"],
                            "touched_files": ["src/eval.rs"],
                            "commit_shas": ["abcdef123"],
                            "eval": {"verdict": "PASS", "reason": "ok"},
                            "gnome_deltas": {"coding_log_score": 0.1},
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
                            "title": "Improve eval",
                            "files": ["src/eval.rs"],
                            "artifact_path": "tasks/task_01/task.md",
                        }
                    ],
                },
            )
            (session / "tasks/task_01").mkdir(parents=True)
            (session / "tasks/task_01/task.md").write_text("Title: Improve eval\n", encoding="utf-8")
            write_json(session / "tasks/task_01/decision.json", {"task_id": "task_01"})
            write_json(session / "tasks/task_01/eval_attempt_1.json", {"status": "pass"})

            replay = state_graph_tools.replay_check(session.parent)
            self.assertEqual(replay["state_replay_integrity_rate"], 1.0)
            self.assertTrue(replay["sessions"][0]["ok"])

            chains = state_graph_tools.build_causal_chains(session)
            self.assertEqual(chains[0]["task_id"], "task_01")
            self.assertEqual(chains[0]["planned_files"], ["src/eval.rs"])
            self.assertEqual(chains[0]["source_files"], ["src/eval.rs"])
            self.assertEqual(chains[0]["commit_shas"], ["abcdef123"])
            self.assertEqual(chains[0]["eval_verdict"], "PASS")

            suggestions = state_graph_tools.evolution_suggestions(session)
            titles = [item["title"] for item in suggestions]
            self.assertIn("Bound evaluator checks so verdicts are not skipped", titles)
            self.assertIn("Split high-turn tasks into narrower plans", titles)

    def test_evolution_suggestions_prioritize_lifecycle_gaps(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "deepseek_model_call_incomplete_count": 1,
                        "state_run_incomplete_count": 0,
                        "state_run_unmatched_non_validation_completed_count": 0,
                        "search_error_count": 4,
                        "max_task_turn_count": 30,
                    }
                },
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=3)

            self.assertEqual(
                suggestions[0]["title"],
                "Close yyds state and model lifecycle gaps",
            )
            self.assertEqual(suggestions[0]["metric"], "deepseek_model_call_incomplete_count")
            self.assertIn("model calls incomplete=1", suggestions[0]["reason"])
            self.assertIn(
                "Harden search commands and pattern escaping",
                [item["title"] for item in suggestions],
            )

    def test_evolution_suggestions_surface_abnormal_model_completion_pressure(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
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

            suggestions = state_graph_tools.evolution_suggestions(session, limit=3)

            self.assertEqual(
                suggestions[0]["title"],
                "Close yyds state and model lifecycle gaps",
            )
            self.assertEqual(suggestions[0]["metric"], "deepseek_model_call_abnormal_completed_count")
            self.assertIn("model calls abnormal=1", suggestions[0]["reason"])
            self.assertIn("model_abnormal/model_completion_without_start=1", suggestions[0]["reason"])

    def test_evolution_suggestions_surface_unattempted_selected_tasks(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "task_unattempted_count": 1,
                        "task_artifact_coverage": 1.0,
                    }
                },
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=3)
            unattempted = next(
                item
                for item in suggestions
                if item["title"] == "Preserve budget to start every selected task"
            )

            self.assertEqual(unattempted["metric"], "task_unattempted_count")
            self.assertEqual(unattempted["value"], 1)
            self.assertIn("never attempted", unattempted["reason"])

    def test_evolution_suggestions_surface_obsolete_selected_tasks(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "task_obsolete_count": 1,
                        "task_artifact_coverage": 1.0,
                    }
                },
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=3)
            obsolete = next(
                item
                for item in suggestions
                if item["title"] == "Replace stale or already-satisfied tasks"
            )

            self.assertEqual(obsolete["metric"], "task_obsolete_count")
            self.assertEqual(obsolete["value"], 1)
            self.assertIn("obsolete or already satisfied", obsolete["reason"])

    def test_evolution_suggestions_surface_raw_seed_contradictions(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "task_seed_contradiction_count": 1,
                        "task_artifact_coverage": 1.0,
                    }
                },
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=3)
            contradiction = next(
                item
                for item in suggestions
                if item["title"] == "Validate seeded tasks against fresh assessment"
            )

            self.assertEqual(contradiction["metric"], "task_seed_contradiction_count")
            self.assertEqual(contradiction["value"], 1)
            self.assertIn("contradicted by assessment evidence", contradiction["reason"])

    def test_evolution_suggestions_surface_measured_execution_state_and_cache_pressure(self):
        cases = [
            (
                "state_live_baseline_shrink_count",
                "Keep live state append-only",
                "fewer events than the replay baseline",
            ),
            (
                "task_api_error_count",
                "Recover API-error tasks instead of generic reverts",
                "provider/API errors",
            ),
            (
                "provider_error_count",
                "Recover provider errors before task attempts",
                "outside task-scoped API reverts",
            ),
            (
                "task_no_edit_revert_count",
                "Force reverted tasks to leave concrete evidence",
                "reverted without touching files",
            ),
            (
                "task_scope_mismatch_count",
                "Align implementation edits with task file scope",
                "outside the selected task surface",
            ),
            (
                "protected_file_revert_count",
                "Route protected-file work through explicit approval",
                "protected files",
            ),
            (
                "tool_error_count",
                "Recover failed tool actions before scoring",
                "Failed tool actions",
            ),
            (
                "prompt_heredoc_expansion_error_count",
                "Quote generated prompts before execution",
                "Prompt heredocs expanded",
            ),
            (
                "deepseek_cache_ratio_unverified_count",
                "Ignore prose-only DeepSeek cache ratios",
                "without token-backed cache metrics",
            ),
            (
                "deepseek_cache_metric_missing_count",
                "Record token-backed DeepSeek cache metrics",
                "cache metric events were missing",
            ),
        ]
        for metric, title, reason_snippet in cases:
            with self.subTest(metric=metric), tempfile.TemporaryDirectory() as tmp:
                session = Path(tmp) / "sessions/day-1"
                write_json(
                    session / "state/summary.json",
                    {
                        "latest_gnomes": {
                            metric: 1,
                            "task_artifact_coverage": 1.0,
                        }
                    },
                )

                suggestions = state_graph_tools.evolution_suggestions(session, limit=10)
                suggestion = next(item for item in suggestions if item["title"] == title)

                self.assertEqual(suggestion["metric"], metric)
                self.assertEqual(suggestion["value"], 1)
                self.assertIn(reason_snippet, suggestion["reason"])

    def test_evolution_suggestions_surface_low_state_capture_pressure(self):
        cases = [
            (
                {"state_operational_capture_coverage": 0.0, "state_capture_coverage": 1.0},
                "Restore operational state capture",
                "state_operational_capture_coverage",
                "Operational yoagent-state events",
            ),
            (
                {"state_capture_coverage": 0.0},
                "Restore state event capture",
                "state_capture_coverage",
                "state/events.jsonl",
            ),
        ]
        for gnomes, title, metric, reason_snippet in cases:
            with self.subTest(metric=metric), tempfile.TemporaryDirectory() as tmp:
                session = Path(tmp) / "sessions/day-1"
                latest_gnomes = {"task_artifact_coverage": 1.0}
                latest_gnomes.update(gnomes)
                write_json(session / "state/summary.json", {"latest_gnomes": latest_gnomes})

                suggestions = state_graph_tools.evolution_suggestions(session, limit=10)
                suggestion = next(item for item in suggestions if item["title"] == title)

                self.assertEqual(suggestion["metric"], metric)
                self.assertEqual(suggestion["value"], 0.0)
                self.assertIn(reason_snippet, suggestion["reason"])

    def test_evolution_suggestions_prioritize_provider_recovery_over_state_cleanup(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "provider_error_count": 2,
                        "state_operational_capture_coverage": 0.0,
                        "state_replay_integrity_rate": 0.0,
                        "task_artifact_coverage": 1.0,
                    }
                },
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=3)

            self.assertEqual(suggestions[0]["title"], "Recover provider errors before task attempts")
            self.assertEqual(suggestions[0]["metric"], "provider_error_count")
            self.assertEqual(suggestions[0]["value"], 2)

    def test_evolution_suggestions_prioritize_provider_recovery_over_missing_task_artifacts(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "provider_error_count": 2,
                        "task_artifact_coverage": 0.0,
                        "selected_task_count": 1,
                    }
                },
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=3)

            self.assertEqual(suggestions[0]["title"], "Recover provider errors before task attempts")
            self.assertEqual(suggestions[0]["metric"], "provider_error_count")

    def test_evolution_suggestions_skip_task_artifact_cleanup_when_provider_blocked_before_tasks(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "provider_error_count": 2,
                        "planner_no_task_count": 1,
                        "task_artifact_coverage": 0.0,
                        "task_lineage_capture_coverage": 0.0,
                        "selected_task_count": 0,
                        "tasks_attempted": 0,
                        "transcript_task_attempt_count": 0,
                    }
                },
            )
            write_json(
                session / "tasks/manifest.json",
                {"planner": {"planning_failed": True, "task_count": 0, "selected_task_count": 0}},
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=10)
            titles = [item["title"] for item in suggestions]

            self.assertEqual(suggestions[0]["title"], "Recover provider errors before task attempts")
            self.assertIn("Treat planning failure as provider-blocked", titles)
            self.assertNotIn("Restore task artifact coverage", titles)
            self.assertNotIn("Restore explicit task lineage capture", titles)

    def test_evolution_suggestions_prioritize_provider_recovery_over_provider_blocked_planning(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "provider_error_count": 2,
                        "planner_no_task_count": 1,
                        "task_artifact_coverage": 0.0,
                    }
                },
            )
            write_json(
                session / "tasks/manifest.json",
                {"planner": {"planning_failed": True, "task_count": 0, "selected_task_count": 0}},
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=5)

            self.assertEqual(suggestions[0]["title"], "Recover provider errors before task attempts")
            self.assertEqual(suggestions[0]["metric"], "provider_error_count")
            blocked = next(item for item in suggestions if item["metric"] == "planner_no_task_count")
            self.assertEqual(blocked["title"], "Treat planning failure as provider-blocked")
            self.assertLess(blocked["priority"], suggestions[0]["priority"])

    def test_evolution_suggestions_surface_low_task_lineage_capture_pressure(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "task_artifact_coverage": 1.0,
                        "task_lineage_capture_coverage": 0.0,
                        "selected_task_count": 1,
                    }
                },
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=10)
            suggestion = next(
                item for item in suggestions if item["title"] == "Restore explicit task lineage capture"
            )

            self.assertEqual(suggestion["metric"], "task_lineage_capture_coverage")
            self.assertEqual(suggestion["value"], 0.0)
            self.assertIn("commit SHAs", suggestion["reason"])
            self.assertIn("gnome deltas", suggestion["reason"])

    def test_evolution_suggestions_surface_state_replay_integrity_pressure(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "state_replay_integrity_rate": 0.0,
                        "task_artifact_coverage": 1.0,
                    }
                },
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=10)
            suggestion = next(item for item in suggestions if item["title"] == "Repair state replay integrity")

            self.assertEqual(suggestion["metric"], "state_replay_integrity_rate")
            self.assertEqual(suggestion["value"], 0.0)
            self.assertIn("state/events.jsonl", suggestion["reason"])
            self.assertIn("task artifacts", suggestion["reason"])

    def test_evolution_suggestions_skip_replay_integrity_for_live_staging_feedback(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / ".yoyo/session_staging"
            write_json(
                session / "log_feedback.json",
                {
                    "metrics": {
                        "state_feedback_source": "live_staging",
                        "state_capture_coverage": 1.0,
                        "state_replay_integrity_rate": 0.0,
                    }
                },
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=10)

            self.assertNotIn(
                "Repair state replay integrity",
                [item["title"] for item in suggestions],
            )

    def test_evolution_suggestions_surface_state_failure_count_pressure(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {"latest_gnomes": {"state_failure_count": 2, "task_artifact_coverage": 1.0}},
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=10)
            suggestion = next(
                item for item in suggestions if item["title"] == "Repair recorded state failure events"
            )

            self.assertEqual(suggestion["metric"], "state_failure_count")
            self.assertEqual(suggestion["value"], 2)
            self.assertIn("replay fixture", suggestion["reason"])

    def test_evolution_suggestions_surface_json_parse_failure_pressure(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {"latest_gnomes": {"json_parse_failure_rate": 0.25, "task_artifact_coverage": 1.0}},
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=10)
            suggestion = next(
                item for item in suggestions if item["title"] == "Reduce DeepSeek JSON parse failures"
            )

            self.assertEqual(suggestion["metric"], "json_parse_failure_rate")
            self.assertEqual(suggestion["value"], 0.25)
            self.assertIn("structured-output prompts", suggestion["reason"])

    def test_evolution_suggestions_surface_tool_call_and_context_quality_pressure(self):
        cases = [
            (
                "tool_call_malformed_rate",
                0.5,
                "Reduce malformed tool-call outputs",
                "tool schema instructions",
            ),
            (
                "context_miss_rate",
                0.4,
                "Reduce DeepSeek context misses",
                "prompt prefix or retrieval path",
            ),
        ]
        for metric, value, title, reason_snippet in cases:
            with self.subTest(metric=metric), tempfile.TemporaryDirectory() as tmp:
                session = Path(tmp) / "sessions/day-1"
                write_json(
                    session / "state/summary.json",
                    {"latest_gnomes": {metric: value, "task_artifact_coverage": 1.0}},
                )

                suggestions = state_graph_tools.evolution_suggestions(session, limit=10)
                suggestion = next(item for item in suggestions if item["title"] == title)

                self.assertEqual(suggestion["metric"], metric)
                self.assertEqual(suggestion["value"], value)
                self.assertIn(reason_snippet, suggestion["reason"])

    def test_evolution_suggestions_surface_low_task_verification_pressure(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "task_artifact_coverage": 1.0,
                        "task_verification_rate": 0.5,
                        "evaluator_unverified_count": 0,
                    }
                },
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=10)
            suggestion = next(
                item for item in suggestions if item["title"] == "Require strict verifier evidence for tasks"
            )

            self.assertEqual(suggestion["metric"], "task_verification_rate")
            self.assertEqual(suggestion["value"], 0.5)
            self.assertIn("verifier artifacts", suggestion["reason"])

    def test_evolution_suggestions_prioritize_low_task_success_rate(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "task_artifact_coverage": 1.0,
                        "task_success_rate": 0.5,
                        "session_success_rate": 0.0,
                    }
                },
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=10)
            suggestion = next(
                item for item in suggestions if item["title"] == "Raise verified task success rate"
            )

            self.assertEqual(suggestion["metric"], "task_success_rate")
            self.assertEqual(suggestion["value"], 0.5)
            self.assertIn("highest-frequency failure class", suggestion["reason"])

    def test_evolution_suggestions_fall_back_to_outcome_task_success_rate(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {"tasks_attempted": 2, "tasks_succeeded": 1},
            )
            write_json(
                session / "state/summary.json",
                {"latest_gnomes": {"task_artifact_coverage": 1.0}},
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=10)
            suggestion = next(
                item for item in suggestions if item["title"] == "Raise verified task success rate"
            )

            self.assertEqual(suggestion["metric"], "outcome_task_success_rate")
            self.assertEqual(suggestion["value"], 0.5)

    def test_evolution_suggestions_surface_low_session_success_rate(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "task_artifact_coverage": 1.0,
                        "task_success_rate": 1.0,
                        "session_success_rate": 0.0,
                    }
                },
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=10)
            suggestion = next(
                item for item in suggestions if item["title"] == "Raise session success rate"
            )

            self.assertEqual(suggestion["metric"], "session_success_rate")
            self.assertEqual(suggestion["value"], 0.0)
            self.assertIn("session outcome pass end to end", suggestion["reason"])

    def test_evolution_suggestions_do_not_blame_session_success_when_provider_blocked(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "provider_error_count": 3,
                        "session_success_rate": 0.0,
                        "task_artifact_coverage": None,
                        "task_success_rate": None,
                    }
                },
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=10)
            titles = [item["title"] for item in suggestions]

            self.assertIn("Recover provider errors before task attempts", titles)
            self.assertNotIn("Raise session success rate", titles)

    def test_evolution_suggestions_surface_low_mechanical_verification_pressure(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "task_artifact_coverage": 1.0,
                        "task_verification_rate": 1.0,
                        "task_mechanical_verification_rate": 0.0,
                    }
                },
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=10)
            suggestion = next(
                item for item in suggestions if item["title"] == "Preserve mechanical verification artifacts"
            )

            self.assertEqual(suggestion["metric"], "task_mechanical_verification_rate")
            self.assertEqual(suggestion["value"], 0.0)
            self.assertIn("deterministic build, test, or eval artifacts", suggestion["reason"])

    def test_evolution_suggestions_include_lifecycle_cause_detail(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "deepseek_model_call_incomplete_count": 1,
                        "state_run_incomplete_count": 1,
                    },
                    "state_lifecycle": {
                        "runs": {
                            "incomplete_runs": [
                                {
                                    "run_id": "run-open",
                                    "last_event": {"kind": "CacheMetricsRecorded"},
                                }
                            ]
                        },
                        "model_calls": {
                            "incomplete_runs": [
                                {
                                    "run_id": "run-model",
                                    "last_event": {"kind": "CommandCompleted"},
                                }
                            ]
                        },
                    },
                },
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=1)

            self.assertEqual(suggestions[0]["title"], "Close yyds state and model lifecycle gaps")
            self.assertIn("causes:", suggestions[0]["reason"])
            self.assertIn("model_incomplete/open_after_command=1", suggestions[0]["reason"])
            self.assertIn("state_incomplete/open_after_cache_metrics=1", suggestions[0]["reason"])

    def test_evolution_suggestions_preserve_missing_assessment_artifacts(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
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

            suggestions = state_graph_tools.evolution_suggestions(session, limit=3)
            assessment = next(
                item for item in suggestions if item["title"] == "Preserve assessment artifacts"
            )

            self.assertEqual(assessment["metric"], "assessment_artifact_missing_count")
            self.assertEqual(assessment["value"], 1)
            self.assertIn("Assessment evidence exists", assessment["reason"])

    def test_evolution_suggestions_classify_provider_blocked_assessment_diagnostic_gap(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {"planning_failed": True, "task_count": 0, "selected_task_count": 0},
                    "artifacts": {"assessment": None, "assessment_missing": None},
                },
            )
            transcript_dir = session / "transcripts"
            transcript_dir.mkdir(parents=True)
            transcript_dir.joinpath("assess.log").write_text(
                "error: Network error: reqwest::Error { source: dns error }\n"
                "API error with no fallback configured. Exiting.\n",
                encoding="utf-8",
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=5)
            assessment = next(
                item
                for item in suggestions
                if item["title"] == "Preserve provider-blocked assessment diagnostic"
            )

            self.assertEqual(assessment["metric"], "assessment_artifact_missing_count")
            self.assertIn("assessment_missing.md diagnostic", assessment["reason"])
            self.assertIn("recovers provider access", assessment["reason"])

    def test_evolution_suggestions_accept_preserved_assessment_artifact(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {"planning_failed": False, "task_count": 0, "selected_task_count": 0},
                    "artifacts": {"assessment": "tasks/assessment.md", "assessment_missing": None},
                },
            )
            (session / "tasks").mkdir(parents=True, exist_ok=True)
            (session / "tasks/assessment.md").write_text("# Assessment\n", encoding="utf-8")
            transcript_dir = session / "transcripts"
            transcript_dir.mkdir(parents=True)
            transcript_dir.joinpath("assess.log").write_text("assessment phase ran\n", encoding="utf-8")

            suggestions = state_graph_tools.evolution_suggestions(session, limit=3)

            self.assertNotIn("Preserve assessment artifacts", [item["title"] for item in suggestions])

    def test_evolution_suggestions_accept_preserved_assessment_missing_diagnostic(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {
                        "planning_failed": True,
                        "task_count": 0,
                        "selected_task_count": 0,
                        "assessment_missing_present": True,
                    },
                    "artifacts": {"assessment": None, "assessment_missing": "tasks/assessment_missing.md"},
                },
            )
            (session / "tasks").mkdir(parents=True, exist_ok=True)
            (session / "tasks/assessment_missing.md").write_text(
                "Assessment phase hit a provider/API error before writing assessment.md.\n",
                encoding="utf-8",
            )
            transcript_dir = session / "transcripts"
            transcript_dir.mkdir(parents=True)
            transcript_dir.joinpath("assess.log").write_text(
                "API error with no fallback configured. Exiting.\n",
                encoding="utf-8",
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=5)

            self.assertNotIn("Preserve assessment artifacts", [item["title"] for item in suggestions])

    def test_evolution_suggestions_require_task_expected_evidence(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
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

            suggestions = state_graph_tools.evolution_suggestions(session, limit=3)
            spec_suggestion = next(
                item for item in suggestions if item["title"] == "Require task evidence specs"
            )

            self.assertEqual(spec_suggestion["metric"], "missing_expected_evidence_count")
            self.assertEqual(spec_suggestion["value"], 1)
            self.assertIn("Selected task specs lacked Expected Evidence", spec_suggestion["reason"])

    def test_evolution_suggestions_accept_task_expected_evidence(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {"planning_failed": False, "task_count": 1, "selected_task_count": 1},
                    "selected_tasks": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "title": "Improve verifiable task",
                            "files": ["scripts/evolve.sh"],
                            "expected_evidence": "trajectory shows missing evidence pressure only when absent",
                            "quality": {"has_expected_evidence": True},
                        }
                    ],
                    "warnings": [],
                },
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=3)

            self.assertNotIn("Require task evidence specs", [item["title"] for item in suggestions])

    def test_evolution_suggestions_surface_contradicted_task_specs(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {"planning_failed": False, "task_count": 1, "selected_task_count": 1},
                    "selected_tasks": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "title": "Stale seed",
                            "files": ["scripts/evolve.sh"],
                            "expected_evidence": "fresh assessment accepts the seed",
                            "quality": {
                                "has_expected_evidence": True,
                                "assessment_alignment": {"contradicted_by_assessment": True},
                            },
                        }
                    ],
                    "warnings": ["task_01:assessment_contradiction"],
                },
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=3)
            contradiction = next(
                item
                for item in suggestions
                if item["title"] == "Replace assessment-contradicted task specs"
            )

            self.assertEqual(contradiction["metric"], "task_manifest_seed_contradiction_count")
            self.assertEqual(contradiction["value"], 1)
            self.assertIn("assessment_contradiction=1", contradiction["reason"])

    def test_evolution_suggestions_surface_thin_generic_task_specs(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {"planning_failed": False, "task_count": 1, "selected_task_count": 1},
                    "selected_tasks": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "title": "Self-improvement",
                            "files": [],
                            "expected_evidence": "dashboard row exists",
                            "quality": {
                                "has_expected_evidence": True,
                                "generic_self_improvement": True,
                                "score": 0.4,
                            },
                        }
                    ],
                    "warnings": [
                        "task_01:generic_self_improvement",
                        "task_01:missing_files",
                        "task_01:thin_task_spec",
                    ],
                },
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=3)
            spec = next(item for item in suggestions if item["title"] == "Tighten selected task specs")

            self.assertEqual(spec["metric"], "task_spec_warning_count")
            self.assertEqual(spec["value"], 3)
            self.assertIn("generic_self_improvement=1", spec["reason"])
            self.assertIn("missing_files=1", spec["reason"])
            self.assertIn("thin_task_spec=1", spec["reason"])

    def test_evolution_suggestions_surface_low_task_spec_quality_gnome(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {"latest_gnomes": {"task_spec_quality_score": 0.5}},
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=3)
            spec = next(item for item in suggestions if item["title"] == "Tighten selected task specs")

            self.assertEqual(spec["metric"], "task_spec_quality_score")
            self.assertEqual(spec["value"], 0.5)
            self.assertIn("detailed manifest warnings were unavailable", spec["reason"])

    def test_lifecycle_cause_summary_skips_input_validation_exits(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {
                    "state_lifecycle": {
                        "runs": {
                            "unmatched_completed_details": [
                                {
                                    "run_id": "run-empty",
                                    "last_event": {
                                        "kind": "RunCompleted",
                                        "status": "error",
                                        "error_detail": "empty_input",
                                    },
                                },
                                {
                                    "run_id": "run-real",
                                    "last_event": {
                                        "kind": "RunCompleted",
                                        "status": "error",
                                        "error_detail": "tool_failed",
                                    },
                                },
                            ]
                        }
                    },
                },
            )

            summary = state_graph_tools.lifecycle_cause_summary(session)

            self.assertNotIn("input_validation_exit_without_run_start", summary)
            self.assertIn("state_unmatched/run_error_without_start=1", summary)

    def test_evolution_suggestions_suppress_ambiguous_bare_fatal_search_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "search_error_count": 1,
                        "max_task_turn_count": 30,
                    }
                },
            )
            write_json(
                session / "log_feedback.json",
                {
                    "metrics": {
                        "search_error_count": 1,
                        "evidence": [
                            "evolve\tRun evolution session\t2026-06-13T17:28:20Z fatal: no pattern given"
                        ],
                        "failure_fingerprints": [
                            {"fingerprint": "fatal: no pattern given", "count": 1}
                        ],
                    }
                },
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=3)

            self.assertNotIn(
                "Harden search commands and pattern escaping",
                [item["title"] for item in suggestions],
            )
            self.assertIn(
                "Split high-turn tasks into narrower plans",
                [item["title"] for item in suggestions],
            )

    def test_evolution_suggestions_surface_recurring_log_failures(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
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

            suggestions = state_graph_tools.evolution_suggestions(session, limit=10)
            recurring = next(
                item for item in suggestions if item["title"] == "Break recurring log failure fingerprints"
            )

            self.assertEqual(recurring["metric"], "recurring_failure_count")
            self.assertEqual(recurring["value"], 1)
            self.assertIn("Max recurrence=3", recurring["reason"])
            self.assertIn("cargo test failed in eval fixture", recurring["reason"])

    def test_evolution_suggestions_surface_repair_loop_churn(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "log_feedback.json",
                {"metrics": {"repair_loop_count": 2}},
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=10)
            repair = next(item for item in suggestions if item["title"] == "Reduce repair-loop churn")

            self.assertEqual(repair["metric"], "repair_loop_count")
            self.assertEqual(repair["value"], 2)
            self.assertIn("retry-after-failure churn", repair["reason"])
            self.assertIn("targeted fixtures", repair["reason"])

    def test_evolution_suggestions_reframe_verified_high_turn_tasks(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "max_task_turn_count": 30,
                        "selected_task_count": 1,
                        "task_strict_verified_count": 1,
                        "tasks_succeeded": 1,
                        "task_revert_count": 0,
                        "task_scope_mismatch_count": 0,
                        "task_unlanded_source_count": 0,
                        "task_api_error_count": 0,
                        "evaluator_unverified_count": 0,
                    }
                },
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=3)
            titles = [item["title"] for item in suggestions]

            self.assertIn("Reduce successful-task turn overhead", titles)
            self.assertNotIn("Split high-turn tasks into narrower plans", titles)

    def test_evolution_suggestions_keep_explicit_search_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {"latest_gnomes": {"search_error_count": 1}},
            )
            write_json(
                session / "log_feedback.json",
                {
                    "metrics": {
                        "search_error_count": 1,
                        "evidence": ["Search error: regex parse error near --json"],
                    }
                },
            )

            suggestions = state_graph_tools.evolution_suggestions(session, limit=3)

            self.assertIn(
                "Harden search commands and pattern escaping",
                [item["title"] for item in suggestions],
            )

    def test_corrected_latest_gnomes_records_resolved_seed_replacement(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {"latest_gnomes": {"task_seed_contradiction_count": 1}},
            )
            write_json(
                session / "log_feedback.json",
                {
                    "metrics": {
                        "selected_task_count": 1,
                        "task_strict_verified_count": 1,
                        "tasks_succeeded": 1,
                        "task_revert_count": 0,
                        "task_obsolete_count": 0,
                        "task_manifest_seed_contradiction_count": 0,
                        "task_seed_contradiction_count": 1,
                    }
                },
            )

            gnomes = state_graph_tools.corrected_latest_gnomes(session)

            self.assertEqual(gnomes["task_seed_contradiction_count"], 0)
            self.assertEqual(gnomes["task_seed_replacement_count"], 1)

    def test_task_artifacts_clear_stale_raw_and_evaluator_gnomes(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "evaluator_unverified_count": 1,
                        "task_unverified_raw_attempt_count": 1,
                        "task_unverified_raw_success_count": 1,
                    }
                },
            )
            write_json(
                session / "tasks/manifest.json",
                {
                    "selected_tasks": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "title": "Verified task",
                            "files": ["src/eval.rs"],
                        }
                    ]
                },
            )
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "status": "completed",
                    "planned_files": ["src/eval.rs"],
                    "touched_files": ["src/eval.rs"],
                    "source_files": ["src/eval.rs"],
                    "commit_shas": ["abc123"],
                },
            )
            write_json(
                session / "tasks/task_01/eval_attempt_1.json",
                {"task_id": "task_01", "status": "pass", "verdict": "PASS"},
            )

            gnomes = state_graph_tools.corrected_latest_gnomes(session)
            suggestions = state_graph_tools.evolution_suggestions(session)

            self.assertEqual(gnomes["evaluator_unverified_count"], 0)
            self.assertEqual(gnomes["task_unverified_raw_attempt_count"], 0)
            self.assertEqual(gnomes["task_unverified_raw_success_count"], 0)
            self.assertEqual(gnomes["task_success_rate"], 1.0)
            self.assertEqual(gnomes["task_verification_rate"], 1.0)
            self.assertEqual(gnomes["task_mechanical_verification_rate"], 1.0)
            self.assertNotIn(
                "Bound evaluator checks so verdicts are not skipped",
                [item["title"] for item in suggestions],
            )

    def test_seed_contradiction_artifact_suppresses_evaluator_pressure(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_json(
                session / "state/summary.json",
                {"latest_gnomes": {"evaluator_unverified_count": 1}},
            )
            write_json(
                session / "log_feedback.json",
                {"metrics": {"task_seed_contradiction_count": 1}},
            )
            write_json(
                session / "tasks/manifest.json",
                {
                    "selected_tasks": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "title": "Contradicted seed",
                            "origin": "harness-seed",
                            "files": ["src/eval.rs"],
                        }
                    ]
                },
            )
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "status": "reverted",
                    "revert_reason": "Task scope mismatch: task produced no git-visible file changes",
                    "planned_files": ["src/eval.rs"],
                    "touched_files": [],
                    "source_files": [],
                    "commit_shas": [],
                },
            )

            gnomes = state_graph_tools.corrected_latest_gnomes(session)
            suggestions = state_graph_tools.evolution_suggestions(session)

            self.assertEqual(gnomes["task_seed_contradiction_count"], 1)
            self.assertEqual(gnomes["evaluator_unverified_count"], 0)
            self.assertEqual(gnomes["task_no_edit_revert_count"], 0)
            self.assertNotIn(
                "Bound evaluator checks so verdicts are not skipped",
                [item["title"] for item in suggestions],
            )

    def test_compare_previous_session(self):
        with tempfile.TemporaryDirectory() as tmp:
            sessions = Path(tmp) / "sessions"
            base = sessions / "day-1"
            cand = sessions / "day-2"
            write_json(base / "outcome.json", {"day": 1, "ts": "2026-01-01T00:00:00Z", "tasks_succeeded": 1})
            write_json(cand / "outcome.json", {"day": 2, "ts": "2026-01-02T00:00:00Z", "tasks_succeeded": 2})
            write_json(
                base / "state/summary.json",
                {"latest_gnomes": {"coding_log_score": 0.7, "context_miss_rate": 0.5}},
            )
            write_json(
                cand / "state/summary.json",
                {"latest_gnomes": {"coding_log_score": 0.9, "context_miss_rate": 0.25}},
            )

            comparison = state_graph_tools.compare_sessions(sessions, "previous", "latest")
            self.assertEqual(comparison["baseline_session"], "day-1")
            self.assertEqual(comparison["candidate_session"], "day-2")
            self.assertAlmostEqual(comparison["gnome_deltas"]["coding_log_score"]["delta"], 0.2)
            self.assertAlmostEqual(comparison["gnome_deltas"]["context_miss_rate"]["delta"], -0.25)

    def test_replay_check_requires_state_artifacts(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            session.mkdir(parents=True)

            replay = state_graph_tools.replay_check(session.parent)
            self.assertEqual(replay["state_replay_integrity_rate"], 0.0)
            self.assertFalse(replay["sessions"][0]["ok"])
            self.assertFalse(replay["sessions"][0]["events_available"])
            self.assertFalse(replay["sessions"][0]["summary_available"])
            self.assertIn("missing_state_events_jsonl", replay["sessions"][0]["mismatches"])
            self.assertIn("missing_state_summary_json", replay["sessions"][0]["mismatches"])


if __name__ == "__main__":
    unittest.main()
