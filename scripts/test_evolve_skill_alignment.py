import pathlib
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[1]
EVOLVE_SKILL = ROOT / "skills" / "evolve" / "SKILL.md"
SELF_ASSESS_SKILL = ROOT / "skills" / "self-assess" / "SKILL.md"
EVOLVE_SCRIPT = ROOT / "scripts" / "evolve.sh"


class EvolveSkillAlignmentTests(unittest.TestCase):
    def test_evolve_skill_is_yyds_deepseek_native(self):
        text = EVOLVE_SKILL.read_text(encoding="utf-8")

        self.assertIn("name: evolve", text)
        self.assertIn("generation 1 DeepSeek-native branch", text)
        self.assertIn("DeepSeek-backed coding work", text)
        self.assertIn("yoagent-state", text)
        self.assertIn("yologdev/yyds-harness", text)

    def test_evolve_skill_does_not_point_autonomy_at_gen0(self):
        text = EVOLVE_SKILL.read_text(encoding="utf-8")

        self.assertNotIn("gh issue create --repo yologdev/yoyo-evolve", text)
        self.assertNotIn("gh issue list --repo yologdev/yoyo-evolve", text)
        self.assertNotIn("best open-source coding agent in the world", text)
        self.assertNotIn("Never modify scripts/evolve.sh", text)

    def test_autonomous_evolution_loads_local_evolve_skill(self):
        script = EVOLVE_SCRIPT.read_text(encoding="utf-8")

        self.assertIn("YOYO_SKILL_FLAGS=(--skills ./skills)", script)
        self.assertIn("First read and follow \\`skills/evolve/SKILL.md\\`", script)
        self.assertIn("canonical\nimplementation contract for yyds self-evolution", script)
        self.assertIn("Follow the evolve skill rules", script)
        self.assertIn("Verify guessed file paths with \\`list_files\\` or \\`git ls-files <path>\\`", script)
        self.assertIn("Prefer \\`list_files\\` and the \\`search\\` tool for code discovery", script)
        self.assertIn("Before editing, identify the task's Objective", script)
        self.assertIn("Expected Evidence sections", script)
        self.assertIn("task lineage, dashboard artifacts, state events, or gnome metrics", script)
        self.assertIn("Do not assume \\`rg\\` is installed", script)
        self.assertIn("\\`grep -R -F -- '--json' src/\\`", script)
        self.assertIn("Do not send escaped regex snippets like \\`fn handle_run\\\\(\\`", script)
        self.assertIn("\\`grep -R -F -- 'fn handle_run(' src/\\`", script)

    def test_assessment_phase_uses_self_assess_skill(self):
        script = EVOLVE_SCRIPT.read_text(encoding="utf-8")

        self.assertIn("First read and follow \\`skills/self-assess/SKILL.md\\`", script)
        self.assertIn("canonical assessment contract for yyds", script)
        self.assertIn("Structured state snapshot", script)
        self.assertIn("claim health, latest lifecycle gnomes, unresolved claim families", script)
        self.assertIn("recent tool failures", script)
        self.assertIn("recent action evidence", script)
        self.assertIn("current harness\n   pressure", script)
        self.assertIn("historical unrecovered tool failures", script)
        self.assertIn('"recent verified task"', script)
        self.assertIn("do not\n   promote it into Bugs / Friction Found", script)
        self.assertIn("Graph-derived next-task pressure", script)
        self.assertIn("copy every rendered recommendation and metric", script)
        self.assertIn("not dashboard-only display", script)

    def test_planning_phase_interprets_recent_trajectory_labels(self):
        script = EVOLVE_SCRIPT.read_text(encoding="utf-8")

        self.assertIn('Always treat "Graph-derived next-task pressure" as current task-selection evidence', script)
        self.assertIn("graph-ranked state/log pressure, not dashboard decoration", script)
        self.assertIn("say which graph-pressure row you are deferring and why", script)
        self.assertIn("If you plan directly from YOUR TRAJECTORY", script)
        self.assertIn('"Graph-derived next-task pressure"', script)
        self.assertIn('"recent tool failures"', script)
        self.assertIn('"recent action evidence"', script)
        self.assertIn('"historical unrecovered tool failures" as context only', script)

    def test_self_assess_skill_is_yyds_deepseek_native(self):
        text = SELF_ASSESS_SKILL.read_text(encoding="utf-8")

        self.assertIn("name: self-assess", text)
        self.assertIn("generation 1 DeepSeek-native harness branch", text)
        self.assertIn("yoagent-state feedback", text)
        self.assertIn("gnome values", text)
        self.assertIn("DeepSeek-backed coding", text)

    def test_self_assess_skill_uses_evolution_evidence(self):
        text = SELF_ASSESS_SKILL.read_text(encoding="utf-8")

        self.assertIn("audit-log", text)
        self.assertIn("task manifests", text)
        self.assertIn("dashboard JSON", text)
        self.assertIn("states.json", text)
        self.assertIn("claims.json", text)
        self.assertIn("state/events.jsonl", text)
        self.assertIn("prompt-cache regressions", text)
        self.assertIn("Structured State Snapshot", text)
        self.assertIn("top unresolved claim families", text)
        self.assertIn("recent tool failures", text)
        self.assertIn("recent action evidence", text)
        self.assertIn("historical unrecovered tool failures", text)

    def test_self_assess_skill_preserves_bounded_assessment_contract(self):
        text = SELF_ASSESS_SKILL.read_text(encoding="utf-8")

        self.assertIn("preflight `cargo build` / `cargo test` result as baseline evidence", text)
        self.assertIn("Run only bounded, directly relevant checks", text)
        self.assertIn("Do not\n   rerun full `cargo test`, full clippy, broad source scans", text)
        self.assertIn("write `session_plan/assessment.md`, write\nthat file and stop", text)
        self.assertIn("should not be\ncommitted from the assessment phase", text)

    def test_evolve_skill_teaches_search_discipline_from_audit_friction(self):
        text = EVOLVE_SKILL.read_text(encoding="utf-8")

        self.assertIn("verify it exists with\n   `list_files` or a repository file listing", text)
        self.assertIn("search for the owning module", text)
        self.assertIn("Prefer `list_files` and the `search` tool", text)
        self.assertIn("do not assume `rg` is installed", text)
        self.assertIn("flag-like literals such as `--json`", text)
        self.assertIn("Do not send escaped regex snippets such as `fn handle_run\\(`", text)
        self.assertIn("`grep -R -F -- 'fn handle_run(' src/`", text)
        self.assertIn("inspect the actual git\n   diff", text)
        self.assertIn("required obsolete-task note", text)
        self.assertIn("Do not spend the task budget on analysis", text)


if __name__ == "__main__":
    unittest.main()
