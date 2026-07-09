Title: Add DeepSeek FIM routing held-out eval fixture
Files: eval/fixtures/local-smoke/402-deepseek-fim-routing.json
Issue: #37
Origin: planner

Evidence:
- Issue #37: "Add held-out coding eval coverage for DeepSeek harness gnomes" — target areas include "FIM (fill-in-the-middle) routing correctness"
- Assessment: "capability fitness gnomes still lack held-out baselines for FIM routing, prompt layout determinism, transport error recovery, and cache behavior"
- Day 130 added first fixture (400). Task 02 above adds cache behavior (401). FIM routing is the next highest-priority gap.
- The FIM completion path exists: `src/commands_deepseek.rs` has `handle_fim_slash_command`, `src/deepseek.rs` has FIM model profile. The `/fim` REPL command is functional.
- No eval fixture currently gates FIM routing correctness — if the FIM prompt routing breaks (wrong prefix/suffix ordering, wrong model selection), we won't know until user reports.

Edit Surface:
- eval/fixtures/local-smoke/402-deepseek-fim-routing.json (new file)

Verifier:
- cargo test eval_fixtures -- --nocapture
- yyds eval fixtures run 402

Fallback:
- If no existing test function covers FIM routing and writing a new one would exceed the 3-file task limit, write a fixture that references an existing FIM-related test or a manual verification step.

Objective:
Add a held-out eval fixture that gates DeepSeek FIM (fill-in-the-middle) routing correctness — verifying that the FIM prompt builder produces correctly ordered prefix/suffix and that the FIM model profile is selected correctly.

Why this matters:
FIM is a DeepSeek-specific feature used for code completion. If the FIM routing breaks (prefix/suffix swapped, wrong model, malformed prompt), code completion silently degrades. An eval fixture gates this regression. This directly improves DeepSeek harness reliability.

Success Criteria:
- New fixture file at eval/fixtures/local-smoke/402-deepseek-fim-routing.json
- Fixture validates: `yyds eval fixtures list` shows it
- Fixture tests one of: FIM prefix/suffix ordering, FIM model profile selection, FIM prompt structure integrity
- Fixture is 10-30 lines

Verification:
- cargo test eval_fixtures -- --nocapture
- python3 -c "import json; json.load(open('eval/fixtures/local-smoke/402-deepseek-fim-routing.json'))"
- yyds eval fixtures list

Expected Evidence:
- eval/fixtures/local-smoke/402-deepseek-fim-routing.json exists and loads
- Future `yyds eval fixtures run 402` gates FIM routing regressions
- Issue #37 has coverage across 3 areas (hello-world coding, cache behavior, FIM routing)

Implementation Notes:
Follow existing fixture format. `repo_fixture: "self"`. The `tests` array should reference test functions in `src/deepseek.rs` or `src/commands_deepseek.rs` that verify FIM behavior.

Target one specific FIM behavior:
- Option A: Verify `build_fim_prompt` or equivalent produces correct prefix/suffix ordering (test in `src/deepseek.rs`)
- Option B: Verify FIM model profile selection (test in `src/commands_deepseek.rs`)
- Option C: Verify `/fim` command argument parsing

If no existing test matches, write a small new test in `src/deepseek.rs` and name it in the fixture. The fixture `expected_files` should list `src/deepseek.rs`.

Risk label: `"medium"` — FIM routing is important but secondary to prompt layout determinism (which already has fixture #369).

This is purely additive work — create one new fixture JSON file. If a new test function is needed in `src/deepseek.rs`, that's the only source change (at most ~20 lines).
