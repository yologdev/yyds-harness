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
        self.assertIn("First read and follow `skills/evolve/SKILL.md`", script)
        self.assertIn("canonical\nimplementation contract for yyds self-evolution", script)
        self.assertIn("Follow the evolve skill rules", script)
        self.assertIn("Verify guessed file paths with \\`rg --files\\`", script)
        self.assertIn("literal/fixed-string search", script)

    def test_assessment_phase_uses_self_assess_skill(self):
        script = EVOLVE_SCRIPT.read_text(encoding="utf-8")

        self.assertIn("First read and follow `skills/self-assess/SKILL.md`", script)
        self.assertIn("canonical assessment contract for yyds", script)

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
        self.assertIn("state/events.jsonl", text)
        self.assertIn("prompt-cache regressions", text)

    def test_evolve_skill_teaches_search_discipline_from_audit_friction(self):
        text = EVOLVE_SKILL.read_text(encoding="utf-8")

        self.assertIn("verify it exists with the\n   repo file list (`rg --files`)", text)
        self.assertIn("search for the owning module", text)
        self.assertIn("fixed-string/literal searches", text)
        self.assertIn("regex punctuation", text)


if __name__ == "__main__":
    unittest.main()
