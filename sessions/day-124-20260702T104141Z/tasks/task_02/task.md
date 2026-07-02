Title: Add held-out coding eval fixture for DeepSeek prompt layout determinism
Files: eval/fixtures/local-smoke/370-deepseek-prompt-layout-determinism-eval.json
Issue: #37
Origin: planner

Evidence:
- Issue #37 (OPEN, Jun 25): "Add held-out coding eval coverage for DeepSeek
  harness gnomes." No implementation yet. The harness has eval infrastructure
  (`src/eval_fixtures.rs`, `yyds eval fixtures score`) but lacks fixture
  coverage for DeepSeek-specific behaviors.
- The assessment identifies this as "the most substantive open issue" and a
  capability gap: fitness score is derived entirely from task success rate, not
  from coding-capability evaluations.
- Fixture #369 (`deepseek-prompt-layout-determinism.json`) exists and tests
  prompt layout determinism at the CLI/command level. A complementary fixture
  that tests determinism at the eval level — verifying that repeated eval runs
  with identical input produce identical PromptSnapshot artifacts — would close
  a held-out coding eval gap.
- The existing fixture format is well-established (see any local-smoke fixture)
  and this task is additive only.

Edit Surface:
- eval/fixtures/local-smoke/370-deepseek-prompt-layout-determinism-eval.json (new file)

Verifier:
- cargo run -- yyds eval fixtures score --fixture 370-deepseek-prompt-layout-determinism-eval

Fallback:
- If the eval fixture infrastructure doesn't support the needed assertions
  (PromptSnapshot comparison across runs), write a smaller fixture that tests
  a simpler aspect of DeepSeek prompt behavior — such as cache prefix stability
  or tool schema ordering determinism.
- If the eval framework times out or crashes on the fixture, narrow the fixture
  to test one assertion instead of multiple.

Objective:
Add one held-out coding eval fixture that verifies DeepSeek prompt layout
determinism survives round-trip through the eval framework, closing one
gap in issue #37's target areas.

Why this matters:
The harness measures its fitness score entirely from task success rate. Adding
held-out coding eval fixtures creates a second, independent signal: can the
DeepSeek harness produce deterministic, cache-friendly prompt layouts under eval
conditions? This is a fitness gnome the dashboard can track — when prompt layout
breaks, the eval fixture fails before a session wastes tokens on broken prompts.

Success Criteria:
- A new fixture file exists at eval/fixtures/local-smoke/370-deepseek-prompt-layout-determinism-eval.json
- The fixture validates against `yyds eval fixtures score` (or `yyds eval fixtures list`)
- The fixture tests at least one DeepSeek-specific behavior (prompt layout
  determinism, cache prefix stability, or tool schema ordering)
- The fixture uses the same format as existing local-smoke fixtures
- No source code changes needed — fixture is additive only

Verification:
- cargo run -- yyds eval fixtures list | grep 370
- cargo run -- yyds eval fixtures score --fixture 370-deepseek-prompt-layout-determinism-eval
- Verify fixture JSON is valid: python3 -c "import json; json.load(open('eval/fixtures/local-smoke/370-deepseek-prompt-layout-determinism-eval.json'))"

Expected Evidence:
- New fixture file in eval/fixtures/local-smoke/
- Fixture appears in `yyds eval fixtures list` output
- Fixture passes or produces a clear verdict (not a crash/timeout)

Implementation:
1. Study existing DeepSeek eval fixtures for format conventions. Start with
   `369-deepseek-prompt-layout-determinism.json` as the closest precedent.
   Also check `355-deepseek-yoagent-model-config-boundary.json` and
   `356-deepseek-cache-report-model-breakdown.json` for DeepSeek-specific
   fixture patterns.
2. Create a new fixture that tests prompt layout determinism under eval
   conditions. The fixture should:
   - Define a minimal agent task (e.g., "return the string 'ok'")
   - Run it twice with identical configuration
   - Assert that the prompt structure (tool definitions, system prompt,
     message layout) is identical across both runs
   - Use the existing fixture format: expect at minimum `domain`, `task`,
     `commands` (or `evaluator`) fields
3. The fixture should be small and fast — under 5 seconds to evaluate.
4. If the eval framework requires a specific assertion format, follow existing
   fixture conventions exactly. Do not invent new assertion types.
