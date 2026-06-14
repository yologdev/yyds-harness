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
            write_json(base / "state/summary.json", {"latest_gnomes": {"coding_log_score": 0.7}})
            write_json(cand / "state/summary.json", {"latest_gnomes": {"coding_log_score": 0.9}})

            comparison = state_graph_tools.compare_sessions(sessions, "previous", "latest")
            self.assertEqual(comparison["baseline_session"], "day-1")
            self.assertEqual(comparison["candidate_session"], "day-2")
            self.assertAlmostEqual(comparison["gnome_deltas"]["coding_log_score"]["delta"], 0.2)

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
