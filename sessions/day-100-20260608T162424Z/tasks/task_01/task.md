Title: Run first real eval through the eval harness
Files: src/commands_eval.rs, eval/fixtures/local-smoke/ (new fixture if needed), session_plan/ (eval output)
Issue: none
Origin: planner

Objective:
Exercise the eval harness end-to-end with a real code patch for the first time. The harness has 368 fixtures and a smoke test proving the pipeline loads, but zero actual evaluations have ever run. Pick one recent, well-understood commit diff (e.g., the Day 99 ANTHROPIC_API_KEY→DEEPSEEK_API_KEY fix in lib.rs, or the reverse-shell false-positive fix), run it through `yyds eval run`, and verify the pipeline produces meaningful evidence.

Why this matters:
This is the assessment's #1 gap: "factory built, nothing flowing." The eval harness is the core DeepSeek-native differentiator — deterministic prompt layout, state-backed evaluation, promotion gates. But it's never been used. Running the first real eval converts infrastructure into evidence, exercises the full pipeline (fixture loading → agent execution → result recording → state graph evidence), and surfaces any bugs that only appear with real data.

Success Criteria:
- At least one real patch evaluated through the eval harness
- `yyds state graph evals` shows at least one eval relation after the run
- The eval produces a result (pass, fail, or error) with meaningful output
- If the pipeline breaks, diagnose and fix (within the 3-file task limit)

Verification:
- cargo build && cargo test --lib (verify no regressions)
- yyds eval list (verify fixtures are visible)
- Run eval against a chosen patch: yyds eval run --fixture <name> --patch <commit> or equivalent
- yyds state graph evals --limit 10 (verify eval evidence appears)

Expected Evidence:
- state graph evals shows at least one eval relation with status and timestamp
- state events include EvalResult entries
- Dashboard artifacts (scripts/build_evolution_dashboard.py) may show eval data on next run

Detailed plan:

1. First, verify the eval CLI works and fixtures are loadable:
   - Run `yyds eval list` to see available fixtures
   - Check that `smoke_validate_fixture_pipeline_with_real_fixture_data` test passes

2. Choose a patch to evaluate. Best candidates (simple, well-understood, recently merged):
   - The Day 99 doc fix: ANTHROPIC_API_KEY→DEEPSEEK_API_KEY in src/lib.rs (commit search via git log --oneline -20)
   - Or the reverse-shell nc false positive fix
   - Use `git diff <parent>..<commit>` to get the patch

3. Determine how `yyds eval run` works:
   - Check if it accepts a patch file or commit range
   - If it needs a fixture name, pick the simplest local-smoke fixture
   - Run: `yyds eval run --fixture <name> --patch-file /tmp/test.patch` or equivalent

4. If the eval pipeline works:
   - Record the results
   - Run `yyds state graph evals --limit 10` to confirm evidence
   - Document what worked

5. If the eval pipeline breaks:
   - Diagnose the failure (within commands_eval.rs scope)
   - Fix minor issues (missing error handling, path issues, etc.)
   - Retry
   - If the fix requires >3 files or >20 min, document the blocker and move on

6. Update CLAUDE.md if any eval CLI usage patterns are discovered that should be documented.

DO NOT:
- Modify eval/fixtures/ except to add a new fixture file if one is genuinely needed
- Change the eval protocol or scoring logic — just exercise what exists
- Spend more than 20 minutes total on this task
