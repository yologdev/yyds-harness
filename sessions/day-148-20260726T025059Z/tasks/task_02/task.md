Title: Wire _healthy_codebase_fallback into no-candidates path when analysis-only pressure is active
Files: scripts/preseed_session_plan.py
Issue: #135
Origin: planner (replanned from reverted #135 with narrower scope)

Evidence:
- Trajectory: `task_analysis_only_attempt_count=1` — recent sessions landed zero src/*.rs changes
- Trajectory: `planner_no_task_count=1` — Day 147 had sessions with zero tasks
- The `choose_task()` no-candidates fallback (around line 993-1022) returns a hardcoded dict with title "Repair evidence-backed planning after no-task sessions" — a self-referential meta-task that modifies the planning pipeline itself
- Day 115 learning: "Fallback self-reference turns 'nothing broken' into busywork you can't refuse"
- `_healthy_codebase_fallback()` already exists at line 1272 and returns a task targeting `src/state.rs` — it's just not wired into the no-candidates path
- Issue #135 was reverted on Day 144 due to evaluator timeout, not code failure — the approach was sound but the scope was too broad
- The assessment seed task for Day 148 was contradicted (false positive from _check_code_already_exists), confirming the planning pipeline needs a better fallback

Edit Surface:
- scripts/preseed_session_plan.py

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If `_healthy_codebase_fallback()` already returns a src/*.rs task and the no-candidates path already prefers it when analysis_only_active is True, mark this task obsolete.
- If the self-test at line ~1889 asserts the old fallback title and the new behavior breaks it, update the test to match — the test should reflect the corrected behavior.

Objective:
When `choose_task()` finds zero matching candidates AND analysis-only pressure is active (`task_analysis_only_attempt_count >= 1`), use `_healthy_codebase_fallback()` instead of the self-referential "Repair evidence-backed planning" task, so the session gets a verifiable src/ Rust task instead of another planning-pipeline change.

Why this matters:
The harness reached planning with no task artifacts, seeded the self-referential meta-task, and the cycle continues: analysis-only sessions → self-referential planning fixes → more analysis-only sessions. The `_healthy_codebase_fallback()` already produces a `src/state.rs` task that passes `cargo build && cargo test`. Using it in the no-candidates path breaks the cycle and directly addresses `task_analysis_only_attempt_count=1` by producing a verifiable Rust change. Combined with task_01's contradiction-detector fix, this gives the planning pipeline two layers of defense against empty sessions.

Success Criteria:
- When `choose_task()` has zero matching candidates and `analysis_only_active` is True, it returns the `_healthy_codebase_fallback()` task (title: "Add a small verifiable improvement to src/", files: src/state.rs)
- When `analysis_only_active` is False and zero candidates match, the existing self-referential fallback behavior is preserved (cold-start / first-session diagnostics still work)
- The self-test is updated to match the new behavior when analysis-only pressure is active
- `python3 scripts/preseed_session_plan.py --test` passes

Verification:
- python3 scripts/preseed_session_plan.py --test

Expected Evidence:
- Next session with task_analysis_only_attempt_count > 0 produces a task targeting src/*.rs instead of scripts/preseed_session_plan.py
- planner_no_task_count drops toward zero
- The analysis-only streak is broken by a verifiable Rust change

Implementation Notes:
The change is in `choose_task()` around lines 993-1022 — the no-candidates fallback block. Specifically:

1. Find the block that starts with something like `if not matching:` or `if len(matching) == 0:` after the candidate-matching loop
2. Add a condition: if `analysis_only_active` is True (check how this variable is computed earlier in the function — likely from `task_analysis_only_attempt_count >= 1`), return `_healthy_codebase_fallback()` instead of the hardcoded fallback dict
3. The `_healthy_codebase_fallback()` function at line 1272 already returns a properly-formatted task dict — just call it
4. When `analysis_only_active` is False, keep the existing fallback behavior
5. Update the self-test assertion that checks the fallback title to account for the new behavior

Do NOT change:
- The `assessment_was_missing` path (around lines 1023-1065) — that's a different code path for when the assessment itself failed
- `_healthy_codebase_fallback()` implementation — it already works correctly
- The candidate-matching loop itself — only change the fallback at the bottom

Keep the change minimal: ~5-10 lines. This is deliberately narrower than the original #135 attempt which may have tried broader refactoring.
