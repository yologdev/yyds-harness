Title: Add DeepSeek cache-behavior held-out eval fixture
Files: eval/fixtures/local-smoke/401-deepseek-cache-behavior.json
Issue: #37
Origin: planner

Evidence:
- Issue #37: "Add held-out coding eval coverage for DeepSeek harness gnomes" — target areas include "Cache hit/miss behavior under normal operation"
- Day 130 added the first fixture (400-coding-hello-world.json — 16 lines) but coverage is still thin (1 fixture out of 26 total eval fixtures)
- Assessment confirms: "capability fitness gnomes still lack held-out baselines for FIM routing, prompt layout determinism, transport error recovery, and cache behavior"
- Trajectory fitness_score=1.0 but this is from task success rate, not from held-out eval baselines that independently verify DeepSeek-specific behaviors
- The cache-report command exists and works (verified: yyds deepseek cache-report shows actionable next step). The stream-check command records cache metrics. An eval fixture should verify that cache metrics are recorded correctly after a stream-check run.
- The yaagent Usage struct drops DeepSeek cache fields — known since Day 126 with workaround in place. An eval fixture gates this regression.

Edit Surface:
- eval/fixtures/local-smoke/401-deepseek-cache-behavior.json (new file)

Verifier:
- cargo test eval_fixtures -- --nocapture (fixture loaded and validated)
- yyds eval fixtures run 401 (fixture executes cleanly)

Fallback:
- If the fixture format requires repo_fixture=self and the cache metrics are not available in a test environment, write a documentation-only fixture with a clear skip reason.

Objective:
Add a second held-out coding eval fixture that gates DeepSeek cache-behavior observability — verifying that `yyds deepseek stream-check` records cache metrics and that `yyds deepseek cache-report` can display them.

Why this matters:
DeepSeek prompt caching is critical for cost efficiency. If the cache metrics pipeline breaks (e.g., yaagent Usage struct regression), we lose visibility into cache hit rates. The Day 126 workaround (recording metrics before they hit yaagent's Usage struct) needs a regression gate. This fixture adds that gate.

Success Criteria:
- New fixture file at eval/fixtures/local-smoke/401-deepseek-cache-behavior.json
- Fixture validates: `yyds eval fixtures list` shows it with correct category
- Fixture tests one of: cache metrics presence after stream-check, cache-report output sanity, or cache hit/miss tracking correctness
- Fixture is between 10-30 lines (similar to 400-coding-hello-world.json)

Verification:
- cargo test eval_fixtures -- --nocapture
- python3 -c "import json; json.load(open('eval/fixtures/local-smoke/401-deepseek-cache-behavior.json'))" (valid JSON)
- yyds eval fixtures list (fixture appears in listing)

Expected Evidence:
- eval/fixtures/local-smoke/401-deepseek-cache-behavior.json exists and loads
- Future `yyds eval fixtures run 401` can gate cache-behavior regressions
- Issue #37 gets closer to having comprehensive held-out coverage

Implementation Notes:
Use the same fixture format as existing fixtures. The `repo_fixture: "self"` field means it runs against the current repo. The `tests` array should contain one or more cargo test commands that verify cache behavior. The `goal` should describe what cache behavior is being verified.

Target one specific cache behavior:
- Option A: Verify that `yyds deepseek stream-check` produces cache metrics (test in `src/commands_deepseek.rs` or `src/deepseek.rs`)
- Option B: Verify that `yyds deepseek cache-report` output format is sane (test in `src/commands_deepseek.rs`)
- Option C: Verify the cache metrics parsing in `src/deepseek.rs` handles edge cases

If no existing test function matches, write a new test in `src/deepseek.rs` or `src/commands_deepseek.rs` and reference it. But keep the fixture file itself as the primary deliverable — a new test function in Rust source is only needed if no existing test covers the cache path.

The fixture should have `"risk_label": "medium"` (cache behavior is important but not as critical as prompt layout determinism which is `"high"`).
