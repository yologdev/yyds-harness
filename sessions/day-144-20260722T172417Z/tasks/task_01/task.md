Title: Break self-referential planning fallback when analysis-only pressure is active
Files: scripts/preseed_session_plan.py
Issue: #135
Origin: planner (refined from harness-seed #135 re-attempt)

Evidence:
- Trajectory: task_analysis_only_attempt_count=3 — last 3 sessions landed zero src/*.rs changes
- Trajectory: planner_no_task_count=1 — previous day-144 session had 0/0 tasks
- Trajectory: task_no_edit_revert_count=1 — task reverted without touching files
- Previous attempt (#135) was reverted due to evaluator timeout, not code defect
- `choose_task()` no-candidates fallback (lines 993-1022) returns "Repair evidence-backed planning after no-task sessions" — a self-referential meta-task
- `_healthy_codebase_fallback()` (line 1269) already returns a `src/state.rs` task but isn't wired into the no-candidates path
- `analysis_only_active` flag is already computed at line 932 from trajectory metrics
- `_has_src_files` helper exists at line 897
- Day 115 learning: "Fallback self-reference turns 'nothing broken' into busywork you can't refuse"

Edit Surface:
- scripts/preseed_session_plan.py

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If `_healthy_codebase_fallback()` already returns a src/*.rs task and the no-candidates path already prefers it when `analysis_only_active` is True, mark this task obsolete.
- If the self-test at line 1889 already expects the new behavior, check whether this was already fixed and mark obsolete.

Objective:
When `choose_task()` finds zero matching candidates AND `analysis_only_active` is True (from trajectory metrics), return the `_healthy_codebase_fallback()` task (title: "Add a small verifiable improvement to src/", files: src/state.rs) instead of the self-referential "Repair evidence-backed planning" fallback.

Why this matters:
The harness reached planning with no task artifacts, seeded a meta-task about planning itself, and the cycle continues: analysis-only sessions → self-referential planning fixes → more analysis-only sessions. The `_healthy_codebase_fallback()` already produces a `src/state.rs` task that passes `cargo build && cargo test`. Using it in the no-candidates path directly addresses `task_analysis_only_attempt_count=3` by producing a verifiable Rust change.

Success Criteria:
- When `choose_task()` has zero matching candidates and `analysis_only_active` is True, it returns the `_healthy_codebase_fallback()` task
- When `analysis_only_active` is False and zero candidates match, the existing self-referential fallback behavior is preserved
- The `assessment_was_missing` path (lines 1023+) is NOT changed — it's a different code path
- `python3 scripts/preseed_session_plan.py --test` passes

Verification:
- python3 scripts/preseed_session_plan.py --test

Expected Evidence:
- Next session with task_analysis_only_attempt_count > 0 produces a task targeting src/*.rs instead of scripts/preseed_session_plan.py
- planner_no_task_count drops toward zero
- The analysis-only streak is broken by a verifiable Rust change

Implementation Notes:
- The change is in `choose_task()` around lines 993-1022 — the no-candidates fallback block
- Add a condition at line 993: if `analysis_only_active` is True and zero candidates matched, return `_healthy_codebase_fallback()` instead of the hardcoded fallback dict
- The `_healthy_codebase_fallback()` function at line 1269 already returns a properly-formatted task dict — just call it
- Do NOT change the `assessment_was_missing` path (lines 1023-1065)
- Keep the change minimal: ~5-10 lines
