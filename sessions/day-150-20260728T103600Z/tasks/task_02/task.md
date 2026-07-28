# Task 02: Break self-referential planning fallback when analysis-only pressure active (#135)

Title: Break self-referential planning fallback when analysis-only pressure is active
Files: scripts/preseed_session_plan.py
Issue: #135
Origin: planner

Evidence:
- Trajectory: task_analysis_only_attempt_count=3, planner_no_task_count=1
- 7 of 9 recent sessions landed zero src/*.rs changes — the analysis-only pressure metric is active but not breaking the cycle
- The `choose_task()` no-candidates fallback returns "Repair evidence-backed planning after no-task sessions" — a self-referential meta-task modifying the planning pipeline itself
- Day 115 learning: "Fallback self-reference turns 'nothing broken' into busywork you can't refuse"
- `_healthy_codebase_fallback()` already exists and returns a task targeting `src/state.rs` — it's just not wired into the no-candidates path when analysis-only pressure is active
- The assessment says: "planner still resolves 'no obvious problems' to silence" — this fix gives the planner a concrete escape hatch
- Issue #135 is OPEN with "Task reverted: Break self-referential planning fallback when analysis-only pressure is active" — reverted due to evaluator timeout, not wrong code

Edit Surface:
- scripts/preseed_session_plan.py

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If `_healthy_codebase_fallback()` already returns a src/*.rs task and the no-candidates path already prefers it when analysis_only_active is True, mark this task obsolete and explain why the self-test still asserts the old fallback title.
- If the self-test assertion needs updating, update it — the test should verify the new behavior.

Objective:
When the task picker finds zero matching candidates AND analysis-only pressure is active (task_analysis_only_attempt_count >= 1), use `_healthy_codebase_fallback()` instead of the self-referential "Repair evidence-backed planning" task, so the session gets a verifiable src/ Rust task instead of another planning-pipeline change.

Why this matters:
The harness reaches planning with no task artifacts, seeds a meta-task about the planning pipeline, and the cycle continues: analysis-only sessions → self-referential planning fixes → more analysis-only sessions. The `_healthy_codebase_fallback()` already produces a `src/state.rs` task that passes `cargo build && cargo test`. Using it in the no-candidates path breaks the cycle and directly addresses `task_analysis_only_attempt_count` by producing a verifiable Rust change. This is #3 in graph-derived pressure: "Raise session success rate."

Success Criteria:
- When `choose_task()` has zero matching candidates and `analysis_only_active` is True, it returns the `_healthy_codebase_fallback()` task (title: "Add a small verifiable improvement to src/", files: src/state.rs)
- When `analysis_only_active` is False and zero candidates match, the existing self-referential fallback behavior is preserved (cold-start / first-session diagnostics still work)
- `python3 scripts/preseed_session_plan.py --test` passes
- The analysis-only streak is broken — next session produces a Rust task instead of a planning-pipeline self-repair

Verification:
- python3 scripts/preseed_session_plan.py --test

Expected Evidence:
- Next session with task_analysis_only_attempt_count > 0 produces a task targeting src/*.rs instead of scripts/preseed_session_plan.py
- planner_no_task_count drops toward zero
- session_success_rate rises above 0.0

Implementation Notes:
- The change is in `choose_task()` — the no-candidates fallback block
- Add a condition: if `analysis_only_active` is True and zero candidates matched, return `_healthy_codebase_fallback()` instead of the hardcoded fallback dict
- Update any self-test assertions that check the old fallback title to match the new behavior when analysis-only pressure is active
- The `_healthy_codebase_fallback()` function already exists and returns a properly-formatted task dict — just call it instead of constructing the hardcoded fallback
- Do NOT change the `assessment_was_missing` path — that's a different code path for when the assessment itself failed to generate
- Keep the change minimal: ~5-10 lines
