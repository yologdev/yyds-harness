Title: Add held-out eval fixture for DeepSeek FIM routing correctness
Files: src/eval_fixtures.rs
Issue: #37
Origin: planner

Evidence:
- Assessment Day 130 capability gap #4: "Eval fixture coverage: Open issue #37 requests additional held-out coding eval coverage for DeepSeek harness gnomes. The current 372-fixture suite covers protocol gates, state, context, and regression scenarios but could benefit from more coding-task eval coverage."
- Trajectory: "fitness_score: unknown" — we lack held-out coding eval evidence to measure against Claude Code or Cursor.
- Issue #37 lists FIM (fill-in-the-middle) routing correctness as one of the target areas.
- `yyds eval fixtures list` shows 372 tasks in local-smoke suite, all fixture categories present — the infrastructure works, we just need more coverage.
- `src/eval_fixtures.rs` (1,697 lines) contains `FixtureSuite`, `BenchmarkTask`, and `validate` infrastructure. The `DEFAULT_FIXTURE_ROOT` points to fixture directories.
- `src/deepseek.rs:route_fim_for_prompt` contains the FIM routing logic that needs eval coverage.

Edit Surface:
- src/eval_fixtures.rs

Verifier:
- cargo test eval_fixtures
- yyds eval fixtures list (confirm fixture count increased)

Fallback:
- If FIM routing logic has been removed or significantly restructured since assessment, write an obsolete-task note.
- If the eval fixture infrastructure has changed format, adapt or write planning_failure.md.
- Do not modify `src/deepseek.rs` — this is a fixture-only task.

Objective:
Add one eval fixture that tests DeepSeek FIM routing correctness: given a prompt that should trigger FIM routing (code completion context with identifiable prefix/suffix boundaries), verify that `route_fim_for_prompt` returns a `FimRouteDecision` with the correct `route_to_fim` flag and expected model selection. The fixture should be a minimal smoke test: one concrete prompt → one expected routing outcome.

Why this matters:
"fitness_score: unknown" is a direct statement that we cannot measure whether our changes improve coding capability. FIM routing is a foundational DeepSeek harness behavior — if it routes wrong, completion quality degrades silently. An eval fixture gives us a held-out gate: any future change to FIM routing logic must pass this test. This is the smallest step toward closing issue #37 and getting a measurable fitness gnome.

Success Criteria:
- One new `BenchmarkTask` added to the eval fixture suite that tests FIM routing
- The fixture validates that a known FIM-eligible prompt routes correctly
- The fixture validates that a known non-FIM prompt does NOT route to FIM
- `cargo test eval_fixtures` passes
- `yyds eval fixtures list` shows at least 373 tasks (was 372)

Verification:
- cargo test eval_fixtures
- cargo check

Expected Evidence:
- `src/eval_fixtures.rs` appears in task lineage as the sole changed file
- Eval fixture count increases from 372 → 373+
- Future `yyds eval run` can measure FIM routing correctness as a held-out gate
- This is the first step toward a `coding_log_score` fitness gnome
