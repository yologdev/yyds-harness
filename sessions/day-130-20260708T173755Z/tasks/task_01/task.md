Title: Add held-out coding eval fixture for harness fitness measurement
Files: src/eval_fixtures.rs, eval/fixtures/local-smoke/400-coding-hello-world.json
Issue: #37
Origin: planner

Evidence:
- Issue #37 filed Day 117 (2026-06-25, 13 days ago): "Add held-out coding eval coverage for DeepSeek harness gnomes"
- Current trajectory: `fitness_score=1.0` but this measures harness health (task_success_rate=1.0, task_verification_rate=1.0), not coding capability
- The assessment: "This is the most important open issue. The harness is healthy but there's no held-out measurement of whether yyds can write correct Rust code, fix bugs, or implement features. All current eval coverage is harness-internal."
- The eval infrastructure already exists (`src/eval_fixtures.rs`, `yyds eval run`) and supports fixture-based testing
- No coding-task fixture exists in `eval/fixtures/local-smoke/` (existing fixtures are all harness-internal: state, graph, patch, lifecycle)

Edit Surface:
- eval/fixtures/local-smoke/400-coding-hello-world.json — new eval fixture: a simple coding task with expected output
- src/eval_fixtures.rs — if any fixture format extensions are needed to support coding-task verdicts (read-only unless extension required)

Verifier:
- cargo test --bin yyds -- --test-threads=1
- cargo run -- eval list 2>&1 | grep -c 400 || echo "fixture not listed — check registration"

Fallback:
- If the fixture format cannot express a coding-task verdict without >30 lines of src/eval_fixtures.rs changes, write only the fixture JSON and note the gap in a comment.
- If cargo test fails for unrelated reasons (not the new fixture), do not fix unrelated code — mark the task done-with-findings and record the test failure.
- If the eval runner rejects the fixture format, simplify: make it a harness-internal fixture that tests the eval framework can load and validate a coding-style task definition, not an end-to-end agent run.

Objective:
Add one held-out coding eval fixture so the harness can begin measuring whether yyds can complete coding tasks — not just harness-internal health checks. This is the first step toward a fitness signal that distinguishes "harness runs" from "agent codes."

Why this matters:
The current `fitness_score=1.0` comes entirely from harness-internal metrics (task success rate, verification rate). It tells us the harness is healthy but says nothing about whether DeepSeek can actually write code. A held-out coding eval creates an independent signal — one that can't be gamed by tasks that only touch scripts or state machinery. This is the measurement gap between "evolves clean" and "codes well."

Success Criteria:
- A new eval fixture exists at `eval/fixtures/local-smoke/400-coding-hello-world.json`
- The fixture defines a simple coding task: write a Rust function and verify it compiles/passes a test
- `cargo test` passes (no regressions)
- The fixture follows the existing fixture JSON schema used by other local-smoke fixtures

Verification:
- cargo test --bin yyds -- --test-threads=1
- git diff --stat -- eval/fixtures/ src/eval_fixtures.rs

Expected Evidence:
- Future `yyds eval list` output includes the new fixture
- The fixture can be loaded and validated (even if not yet run as a full agent session)
- Task lineage shows the new fixture file was created

Implementation Notes:
- Study an existing fixture (e.g., `eval/fixtures/local-smoke/001-context-failing-files.json`) for the JSON schema
- The fixture should define a coding task that a human would recognize as "write code that does X" — e.g., implement a function, fix a bug, add a test
- Keep it minimal: one task, one expected outcome, no complex multi-step workflow
- The fixture does NOT need to run a full agent session — it just needs to be loadable and validatable by the eval framework
- If `src/eval_fixtures.rs` needs changes to support coding-task verdict types, make them minimal (add a field, don't restructure)
- This is an additive change — no existing fixtures or code should be modified
