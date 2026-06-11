#!/usr/bin/env python3
"""Regression tests for scripts/skill_evolve.sh harness guarantees."""

from __future__ import annotations

import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "scripts" / "skill_evolve.sh"


class SkillEvolveHarnessTests(unittest.TestCase):
    def setUp(self) -> None:
        self.script = SCRIPT.read_text(encoding="utf-8")

    def test_cycle_records_fallback_journal_event_when_agent_is_silent(self):
        self.assertIn("JOURNAL_EVENTS_BEFORE=", self.script)
        self.assertIn("JOURNAL_EVENTS_AFTER=$(journal_event_count)", self.script)
        self.assertIn("agent produced no journal event or diff", self.script)
        self.assertIn('append_harness_journal_event "NO-OP"', self.script)
        self.assertIn("counter reset is auditable", self.script)

    def test_uncommitted_agent_changes_are_validated_and_committed(self):
        self.assertIn("changed_files_since_cycle_start()", self.script)
        self.assertIn('git diff --name-only "$HEAD_BEFORE"', self.script)
        self.assertIn("git ls-files --others --exclude-standard", self.script)
        self.assertIn("commit_uncommitted_cycle_changes()", self.script)
        self.assertIn('git add -- "$f"', self.script)
        self.assertIn("skill-evolve: record cycle event", self.script)

    def test_missing_journal_with_agent_changes_is_refused_and_reverted(self):
        self.assertIn("agent changed files but wrote no journal event", self.script)
        self.assertIn("revert_agent_work", self.script)
        self.assertIn('append_harness_journal_event "refused"', self.script)
        self.assertIn("did not append the required skills/_journal.md event", self.script)

    def test_revert_paths_leave_refused_journal_evidence(self):
        self.assertIn("DIFF SCOPE VIOLATION", self.script)
        self.assertIn("outside the skill-evolve allow-list", self.script)
        self.assertIn("build broken after agent commit", self.script)
        self.assertIn("broke cargo build", self.script)
        self.assertIn("tests broken after agent commit", self.script)
        self.assertIn("broke cargo test", self.script)


if __name__ == "__main__":
    unittest.main()
