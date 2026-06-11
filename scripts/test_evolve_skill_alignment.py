import pathlib
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[1]
EVOLVE_SKILL = ROOT / "skills" / "evolve" / "SKILL.md"
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
        self.assertIn("Follow the evolve skill rules", script)


if __name__ == "__main__":
    unittest.main()
