Title: Stabilize run completion guard panic test
Files: src/state.rs
Issue: none
Origin: harness-seed
validated_against_assessment: true

Objective:
Make the `run_completion_guard_reports_error_on_panic` test deterministic and preserve the panic-path RunCompleted/FailureObserved behavior it verifies.

Why this matters:
The assessment found the run-completion guard test failing once and passing on retry. A one-file flaky-test repair is more landable than broad lifecycle cleanup and directly protects the state evidence used by DeepSeek evolution.

Success Criteria:
- The panic-path test no longer depends on timing, shared global state, or ambiguous event ordering.
- The production panic-path lifecycle events remain unchanged.
- The task touches only `src/state.rs` unless verification exposes a direct dependency.

Verification:
- cargo test --lib state::tests::run_completion_guard -- --exact
- cargo test --lib state::tests::run_completion_guard
- cargo check

Expected Evidence:
- Future CI/log feedback stops repeating the `run_completion_guard` flaky failure.
- Task lineage links a strict one-file source change to the lifecycle reliability issue.

Implementation Notes:
- This task was seeded by the harness before planner exploration because recent runs reached planning without durable task files.
- Treat it as a minimum viable task for Day 107 (22:24); refine it if the planner has stronger evidence, but do not leave the session with zero task files.
- Keep the change scoped to the listed files unless verification reveals a direct dependency.
