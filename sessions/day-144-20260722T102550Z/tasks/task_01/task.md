Title: Break self-referential planning fallback when analysis-only pressure is active
Files: scripts/preseed_session_plan.py
Issue: none
Origin: planner (refined from harness-seed)
validated_against_assessment: true

Evidence:
- Trajectory: task_analysis_only_attempt_count=3 — last 3 sessions landed zero src/*.rs changes
- Trajectory: planner_no_task_count=1 — previous day-144 session had 0/0 tasks
- The `choose_task()` no-candidates fallback (lines 993-1022) returns "Repair evidence-backed planning after no-task sessions" which is a self-referential meta-task modifying the planning pipeline itself
- Day 115 learning: "Fallback self-reference turns 'nothing broken' into busywork you can't refuse"
- `_healthy_codebase_fallback()` already exists (line 1268) and returns a task targeting `src/state.rs` — it's just not wired into the no-candidates path
- `_has_src_files` helper already exists (line 897) and is used for candidate re-ranking when `analysis_only_active` is True
- Infrastructure is already in place; the gap is only in the no-candidates fallback path

Edit Surface:
- scripts/preseed_session_plan.py

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If `_healthy_codebase_fallback()` already returns a src/*.rs task and the no-candidates path already prefers it when analysis_only_active is True, mark this task obsolete and explain why the self-test at line 1891 still asserts the old fallback title.

Objective:
When the task picker finds zero matching candidates AND analysis-only pressure is active (task_analysis_only_attempt_count >= 1), use `_healthy_codebase_fallback()` instead of the self-referential "Repair evidence-backed planning" task, so the session gets a verifiable src/ Rust task instead of another planning-pipeline change.

Why this matters:
The harness reached planning with no task artifacts, seeded this meta-task, and the cycle continues: analysis-only sessions → self-referential planning fixes → more analysis-only sessions. The `_healthy_codebase_fallback()` already produces a `src/state.rs` task that passes `cargo build && cargo test`. Using it in the no-candidates path breaks the cycle and directly addresses `task_analysis_only_attempt_count=3` by producing a verifiable Rust change.

Success Criteria:
- When `choose_task()` has zero matching candidates and `analysis_only_active` is True, it returns the `_healthy_codebase_fallback()` task (title: "Add a small verifiable improvement to src/", files: src/state.rs)
- When `analysis_only_active` is False and zero candidates match, the existing self-referential fallback behavior is preserved (so cold-start / first-session diagnostics still work)
- The self-test at line 1889 (which asserts the old fallback title) is updated to match the new behavior when analysis-only pressure is active
- `python3 scripts/preseed_session_plan.py --test` passes

Verification:
- python3 scripts/preseed_session_plan.py --test

Expected Evidence:
- Next session with task_analysis_only_attempt_count > 0 produces a task targeting src/*.rs instead of scripts/preseed_session_plan.py
- planner_no_task_count drops toward zero
- The analysis-only streak is broken by a verifiable Rust change

Implementation Notes:
- The change is in `choose_task()` around lines 993-1022 — the no-candidates fallback block
- Add a condition: if `analysis_only_active` is True and zero candidates matched, return `_healthy_codebase_fallback()` instead of the hardcoded fallback dict
- Update the self-test at line 1889 to account for the new behavior — the test that checks `assessment_was_missing` (lines 1958+) should still work because `analysis_only_active` would be False in that case
- The `_healthy_codebase_fallback()` function at line 1268 already returns a properly-formatted task dict — just call it instead of constructing the hardcoded fallback
- Do NOT change the `assessment_was_missing` path (lines 1023-1065) — that's a different code path for when the assessment itself failed to generate
- Keep the change minimal: ~5-10 lines
