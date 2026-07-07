Title: Add held-out eval fixture for tool-failure event recording consistency
Files: eval/fixtures/local-smoke/372-tool-failure-recording.json
Issue: #37
Origin: planner

Evidence:
- Graph-derived next-task pressure rows 4-5: "Reconcile transcript-only tool failures (transcript_only_failed_tool_count=4)" and "Reconcile state-only tool failures (state_only_failed_tool_count=29)" — 33 total discrepancies between state event stream and transcript logs for tool failures.
- The state DOES record tool_name in FailureObserved events (confirmed: prompt.rs:906 records `"tool_name": tool_name.clone()` in the FailureObserved payload). The gap is not about missing tool metadata — it's about some failure paths recording to one stream but not the other.
- Day 127 added the lifecycle-pairing canary fixture (371-state-lifecycle-pairing.json) which tests RunStarted/RunCompleted pairing. A complementary fixture for tool failure recording would catch regressions where the state recorder stops capturing FailureObserved from tool execution paths.
- Issue #37 (agent-self, June 25) tracks the broader eval coverage gap for DeepSeek harness gnomes. This fixture adds one concrete piece of coverage.
- The existing eval fixture infrastructure supports `repo_fixture: "self"` with arbitrary cargo test commands. A fixture that verifies FailureObserved event structure from a known failure simulation would be additive and verifiable.

Edit Surface:
- eval/fixtures/local-smoke/372-tool-failure-recording.json — new fixture file only

Verifier:
- cargo test --test integration -- --test-threads=1 (fixture runner validates JSON schema)
- python3 -c "import json; f=json.load(open('eval/fixtures/local-smoke/372-tool-failure-recording.json')); assert f['task_id']; assert f['tests']; print('ok')"

Fallback:
- If the eval runner doesn't support the test command shape needed, simplify to a pure JSON schema validation fixture (no runtime test).
- If the fixture runner has a pre-existing bug that prevents new fixtures from being loaded, mark the task done-with-findings and file the runner bug as a separate issue.

Objective:
Add a held-out eval fixture that verifies FailureObserved events recorded during tool execution contain the required fields (source, tool_name, error_preview) — a canary that catches regressions in the tool-failure recording pipeline.

Why this matters:
The state/transcript reconciliation gap (33 discrepancies) means post-hoc diagnosis loses fidelity. Without a fixture that explicitly tests FailureObserved recording from tool execution paths, a regression that silently stops recording tool failures would only be discovered when the discrepancy count grows — which requires a human or agent to notice the counter drift. A held-out eval fixture converts "we'll notice when the gap grows" into "the test fails immediately."

This is a first step toward closing the eval coverage gap tracked in #37. The fixture is additive (no code changes) and independently verifiable.

Success Criteria:
- New fixture file at eval/fixtures/local-smoke/372-tool-failure-recording.json passes JSON schema validation
- The fixture's test command verifies that a known tool-failure path produces a well-formed FailureObserved event
- The fixture is loadable by the eval runner (`cargo test --test integration`)

Verification:
- python3 -c "import json; json.load(open('eval/fixtures/local-smoke/372-tool-failure-recording.json')); print('valid JSON')"
- cargo test --test integration -- --test-threads=1 2>&1 | grep -E "(372|tool-failure|FAILED|ok)"

Expected Evidence:
- Task lineage shows eval/fixtures/local-smoke/372-tool-failure-recording.json created
- Future eval runs include this fixture in the local-smoke suite
- If the tool-failure recording pipeline regresses, this fixture catches it before the state/transcript gap widens

Implementation Notes:
- Follow the existing fixture format (see 371-state-lifecycle-pairing.json and 004-cache-metrics.json for examples).
- The fixture should be a canary: it should PASS when the recording pipeline is healthy. Unlike 371 (which intentionally fails until lifecycle gaps are closed), this fixture should verify correct behavior.
- Recommended test command shape: `cargo test state::tests::panic_hook_records_failure_observed -- --nocapture` (verifies the panic hook records FailureObserved with correct fields). If that test already exists and passes, the fixture's job is to ensure it STAYS passing across harness changes.
- Alternative: `cargo run -- state tail --limit 10000 --json 2>/dev/null | python3 -c "import sys,json; events=[json.loads(l) for l in sys.stdin]; tool_failures=[e for e in events if e.get('kind')=='FailureObserved' and e.get('payload',{}).get('source')=='tool']; assert len(tool_failures)>0, 'no tool FailureObserved events'; print(f'ok - {len(tool_failures)} tool FailureObserved events')"`
- category: "state/integrity", risk_label: "medium"
- expected_files should include the fixture itself and the source files that record FailureObserved: src/state.rs, src/prompt.rs
