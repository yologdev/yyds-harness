Title: Add held-out eval fixture for DeepSeek transport error recovery
Files: eval/fixtures/local-smoke/037-deepseek-transport-error-recovery.json
Issue: #37
Origin: planner

Evidence:
- Issue #37 is OPEN: "Add held-out coding eval coverage for DeepSeek harness gnomes"
- The capability fitness score sits at 0.5 with `coding_log_score` unknown — eval coverage is the path to measurement
- The trajectory shows `task_success_rate=0.5, task_verification_rate=0.5` — improvement needs measurement, and eval fixtures are the trust layer
- Day 131 added a "hello world" fixture (036-deepseek-transport-error-policy.json); that template proves the eval infrastructure works
- Existing fixtures under eval/fixtures/local-smoke/ cover: FIM HTTP protocol regression (#123), critical file operation approval (#198), transport error policy (#036), bash approval (#062), context search ranking (#113), tool policy lineage (#059), bash policy lineage (#061), run completion lineage (#022), empty session classification (empty-session-reason-classification.json)
- Transport error recovery (#037) is explicitly listed in the assessment's capability gaps and has NO existing fixture

Edit Surface:
- eval/fixtures/local-smoke/037-deepseek-transport-error-recovery.json (NEW file, additive only)
- No source code changes. No build impact.

Verifier:
- yyds eval run --fixture 037-deepseek-transport-error-recovery
- (or the equivalent evaluate command that runs a single fixture and reports pass/fail)

Fallback:
- If the eval infrastructure doesn't support individual fixture runs, write the fixture JSON with correct schema and verify it parses with `python3 -c "import json; json.load(open('eval/fixtures/local-smoke/037-deepseek-transport-error-recovery.json'))"`.
- If the fixture JSON schema is unclear, study existing fixtures like 036-deepseek-transport-error-policy.json for the pattern.

Objective:
Create a held-out eval fixture that verifies yyds handles DeepSeek transport errors (connection reset, timeout, HTTP 5xx) without crashing or corrupting state. The fixture should define: a simulated transport error scenario, the expected agent behavior (retry, graceful degradation, or clear error reporting), and a pass/fail verdict.

Why this matters:
DeepSeek transport reliability is a core harness gnome. Without eval coverage, regressions in error recovery are invisible until they cause session failures. The fitness gnomes `retry_success_rate` and `coding_log_score` depend on transport resilience. This fixture converts "unknown" fitness into measurable evidence.

Success Criteria:
- A new fixture file exists at eval/fixtures/local-smoke/037-deepseek-transport-error-recovery.json
- The fixture validates against the existing fixture schema (same structure as 036-deepseek-transport-error-policy.json)
- The fixture defines at least one transport error scenario (connection reset, timeout, or HTTP 5xx)
- The fixture can be run via the eval command infrastructure

Verification:
- python3 -c "import json; f=json.load(open('eval/fixtures/local-smoke/037-deepseek-transport-error-recovery.json')); assert 'task' in f or 'tasks' in f or 'scenario' in f; print('OK: fixture parses and has expected structure')"
- yyds eval list --domain local-smoke (confirm fixture appears in listing)

Expected Evidence:
- Task lineage shows `eval/fixtures/local-smoke/037-deepseek-transport-error-recovery.json` as a created file
- Next assessment notes expanded eval fixture coverage for DeepSeek harness gnomes
- `coding_log_score` and `retry_success_rate` have at least one held-out eval data point

Implementation Notes:
- Study eval/fixtures/local-smoke/036-deepseek-transport-error-policy.json for the fixture schema pattern
- The fixture should define:
  1. A scenario name and description
  2. The simulated transport error (e.g., connection reset, 504 timeout, 503 unavailable)
  3. The expected yyds behavior (e.g., retries N times, records FailureObserved, emits RunCompleted)
  4. A pass/fail verdict based on state events or command output
- Keep the fixture focused on ONE transport error type — don't try to cover all error modes
- The fixture is a JSON definition file, not a Rust test. It uses the existing eval infrastructure.
- If the eval infrastructure doesn't support simulating transport errors (only runs real commands), design the fixture as a specification that can be manually verified or run against real DeepSeek endpoints.
