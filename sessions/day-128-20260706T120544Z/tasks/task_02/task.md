Title: Add held-out eval fixture for DeepSeek cache metric propagation
Files: eval/fixtures/local-smoke/372-deepseek-cache-metric-regression.json
Issue: #37
Origin: planner

Evidence:
- The cache-metric workaround in parse_chat_completion_sse and parse_fim_completion_response records cache hits directly before data enters yoagent (because yoagent's Usage struct drops DeepSeek cache fields). This workaround is functional but has no regression test.
- An existing cache-metrics fixture (004-cache-metrics.json) covers serialization bugs but doesn't test that cache metrics survive the full agent pipeline — if a new code path is added that goes through yoagent without the direct-recording workaround, cache metrics silently drop to zero.
- The assessment confirms: "Cache metrics silently drop to zero when new code paths use yoagent without the direct-recording workaround (Day 125 discovery, partially fixed)."
- Graph pressure: capability fitness score is "unknown" — adding held-out eval evidence for a specific DeepSeek harness behavior (cache metric propagation) directly raises fitness measurement quality.
- The `yyds deepseek cache-report` command exists and already explains when metrics are empty — the fixture can use this command as its verification gate.

Edit Surface:
- eval/fixtures/local-smoke/372-deepseek-cache-metric-regression.json (new file)

Verifier:
- python3 -c "import json; f=json.load(open('eval/fixtures/local-smoke/372-deepseek-cache-metric-regression.json')); assert f['task_id']; assert f['category']; assert f['tests']; print('ok')"
- cargo test -- --test-threads=1 (fixture is additive, no source changes)

Fallback:
- If the fixture cannot be validated because it requires a live DeepSeek API key or real cache-populated state that isn't available in the test environment, mark it as a "requires-api" fixture with risk_label=high and a note explaining the gating condition.
- If the existing 004-cache-metrics.json fixture already covers this regression path after inspection, mark this task done-with-findings and update the existing fixture's goal if needed.

Objective:
Add one held-out eval fixture that catches silent cache-metric regression — when cache_read_input_tokens or cache_creation_input_tokens are present in the raw API response but absent from the recorded state events after passing through the agent pipeline.

Why this matters:
Cache metrics are the primary signal for whether DeepSeek's context caching is working (reducing token costs). When they silently drop to zero, the harness loses visibility into cost efficiency. A regression test that verifies cache metrics survive the full pipeline prevents the workaround from silently breaking when new code paths are added.

Success Criteria:
- New fixture file exists at eval/fixtures/local-smoke/372-deepseek-cache-metric-regression.json
- Fixture follows the existing format (task_id, category, repo_fixture, initial_commit, goal, tests, hidden_failure_mode, expected_files, risk_label)
- Fixture's tests array includes at least one command that verifies cache metrics are populated in state events (e.g., `yyds deepseek cache-report` or a state query filtering for ModelCallCompleted events with cache fields)
- Fixture does not break `cargo test` (additive-only change)

Verification:
- python3 -c "import json; json.load(open('eval/fixtures/local-smoke/372-deepseek-cache-metric-regression.json')); print('valid json')"
- cargo test (fixture is additive, no code changes to break)

Expected Evidence:
- Future eval runs include the new fixture in the deepseek/cache category
- If cache metrics regress, the fixture fails with a clear hidden_failure_mode message
- Capability fitness gnomes gain one more held-out eval data point for the cache behavior dimension

Implementation Notes:
- Follow the existing fixture format from eval/fixtures/local-smoke/004-cache-metrics.json and 036-deepseek-transport-error-policy.json as templates.
- Category should be "deepseek/cache regression" or similar DeepSeek-specific category.
- repo_fixture: "self", initial_commit: "current"
- The tests array should include a state query that reads ModelCallCompleted events and checks for non-zero cache fields. Use `yyds state query` or `yyds deepseek cache-report` depending on which is more deterministic in test environments.
- hidden_failure_mode should describe what silently breaks: "Cache metrics (cache_read_input_tokens, cache_creation_input_tokens) are present in raw API responses but absent from recorded state events after passing through the yoagent agent pipeline."
- expected_files: list the source files that own the cache recording path (src/deepseek.rs, src/state.rs)
- If the fixture depends on having real cache-populated state (non-deterministic in CI), use a design that checks for the presence/absence pattern rather than specific numeric values.
