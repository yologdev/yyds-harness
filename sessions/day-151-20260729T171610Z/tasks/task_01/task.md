Title: Break self-referential planning fallback when analysis-only pressure is active
Files: scripts/preseed_session_plan.py
Issue: #135
Origin: planner

Evidence:
- Trajectory #1 graph pressure: "Make planning failure actionable" (planner_no_task_count=1) — the planner produced no concrete task files in a recent session
- 8 of last 9 sessions landed zero code — the empty-session streak is the dominant pattern
- Issue #135 was reverted on Day 144 due to evaluator timeout, not code error — the fix was correct, the downstream test assertion just needed updating
- Assessment confirms: "The planner cannot distinguish 'nothing needs doing' from 'planning failed'"
- The `choose_task()` no-candidates fallback (line ~997) returns a self-referential meta-task modifying the planning pipeline itself
- `_healthy_codebase_fallback()` already exists (line 1276) and returns a task targeting `src/state.rs`
- `_has_src_files` helper already exists (line 901) and is used for candidate re-ranking when `analysis_only_active` is True
- Infrastructure already in place; the gap is only in the no-candidates fallback path (line ~997)

Edit Surface:
- scripts/preseed_session_plan.py

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If `_healthy_codebase_fallback()` already returns a `src/state.rs` task and the no-candidates path already prefers it when `analysis_only_active` is True, mark this task obsolete and explain why the self-test at line ~1889 still asserts the old fallback title.

Objective:
When the task picker finds zero matching candidates AND analysis-only pressure is active, use `_healthy_codebase_fallback()` instead of the self-referential "Repair evidence-backed planning" task, so the session gets a verifiable `src/state.rs` task instead of another planning-pipeline change.

Why this matters:
The harness reached planning with no task artifacts, seeded a meta-task, and the cycle continues: analysis-only sessions → self-referential planning fixes → more analysis-only sessions. The `_healthy_codebase_fallback()` already produces a task targeting `src/state.rs`. Using it in the no-candidates path breaks the cycle and directly addresses the empty-session streak by producing a verifiable Rust change. The trajectory #1 pressure ("Make planning failure actionable") points directly at this.

Success Criteria:
- When `choose_task()` has zero matching candidates and `analysis_only_active` is True, it returns the `_healthy_codebase_fallback()` task (title: "Add a small verifiable improvement to src/", files: src/state.rs)
- When `analysis_only_active` is False and zero candidates match, the existing self-referential fallback behavior is preserved
- The self-test asserting the old fallback title is updated to match the new behavior when analysis-only pressure is active
- `python3 scripts/preseed_session_plan.py --test` passes

Verification:
- python3 scripts/preseed_session_plan.py --test

Expected Evidence:
- Next session with `task_analysis_only_attempt_count > 0` produces a task targeting `src/*.rs` instead of `scripts/preseed_session_plan.py`
- `planner_no_task_count` drops toward zero
- The analysis-only streak is broken by a verifiable Rust change

Implementation Notes:
- The change is in `choose_task()` in the no-candidates fallback block (where `analysis_only_active` is checked around line 997)
- Add a condition: if `analysis_only_active` is True and zero candidates matched, return `_healthy_codebase_fallback()` instead of the hardcoded fallback dict
- Update the self-test that checks the fallback title to account for the new behavior — tests with `assessment_was_missing=True` should still work because `analysis_only_active` would be False in that case
- The `_healthy_codebase_fallback()` function at line 1276 already returns a properly-formatted task dict — just call it
- Do NOT change the `assessment_was_missing` path — that's a different code path for when the assessment itself failed to generate
- Keep the change minimal: ~5-10 lines
