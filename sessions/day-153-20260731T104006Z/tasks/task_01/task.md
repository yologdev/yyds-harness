Title: Use healthy-codebase fallback when assessment is missing
Files: scripts/preseed_session_plan.py
Issue: none
Origin: planner (refined from harness-seed)

Evidence:
- Trajectory Day 153: assessment timed out after 600s (exit 124), producing no assessment.md
- Trajectory: planner_no_task_count=1 — multiple recent sessions landed 0 tasks
- Trajectory graph pressure: "Make planning failure actionable (planner_no_task_count=1)"
- Log feedback: "planner produced no usable task -> bound discovery and require a selected task artifact before implementation work starts"
- The `choose_task()` function at line 997 already returns `_healthy_codebase_fallback()` when `analysis_only_active` is True
- But `analysis_only_active` depends on metrics parsed from assessment text (line 936: `_has_analysis_only_pressure(metrics)`). When assessment is missing, those metrics are absent, so `analysis_only_active` is False, and the self-referential fallback at line 1000 is used instead
- The `assessment_was_missing` path (lines 1030-1078) enriches the fallback with trajectory gnomes but doesn't change which task is selected — it's still the self-referential "Repair evidence-backed planning" meta-task
- Day 115 learning: "Fallback self-reference turns 'nothing broken' into busywork you can't refuse"

Edit Surface:
- scripts/preseed_session_plan.py

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If `_healthy_codebase_fallback()` already runs when `assessment_was_missing`, mark this task obsolete and explain why the self-test at line 1953+ still shows the old behavior.

Objective:
When the assessment is missing (timed out, didn't produce assessment.md), the `choose_task()` no-candidates fallback should return `_healthy_codebase_fallback()` — a verifiable src/state.rs task — instead of the self-referential "Repair evidence-backed planning" meta-task.

Why this matters:
The harness reached planning with no assessment, seeded a meta-task about fixing itself, and the cycle continues: assessment timeout → no assessment → self-referential planning fix → more assessment timeouts. Breaking this cycle with `_healthy_codebase_fallback()` produces a `src/state.rs` task that passes `cargo build && cargo test` — concrete forward progress instead of another planning-pipeline change.

This directly addresses `planner_no_task_count=1` by ensuring missing-assessment sessions still produce a verifiable Rust task.

Success Criteria:
- When `assessment_was_missing` is True and zero candidates match, `choose_task()` returns the `_healthy_codebase_fallback()` task (title: "Add a small verifiable improvement to src/", files: src/state.rs)
- When `assessment_was_missing` is False (normal flow), existing behavior is preserved — `analysis_only_active` still gates the healthy fallback
- `python3 scripts/preseed_session_plan.py --test` passes (all existing tests, including the `assessment_was_missing` tests around line 1953)

Verification:
- python3 scripts/preseed_session_plan.py --test

Expected Evidence:
- Next session with missing assessment produces a task targeting src/state.rs instead of scripts/preseed_session_plan.py
- planner_no_task_count drops toward zero
- The assessment-missing → no-code-landed chain is broken by a verifiable Rust change

Implementation Notes:
- The change is in `choose_task()` between lines 997-1000: after the `analysis_only_active` early-return check, add a second early-return check for `assessment_was_missing`
- Specifically: `if assessment_was_missing: return _healthy_codebase_fallback()`
- This goes BEFORE the fallback dict construction at line 1000, so the self-referential fallback is never built when assessment is missing
- The `assessment_was_missing` block at lines 1030-1078 (which enriches the fallback dict with trajectory gnomes) becomes dead code for the no-candidates path but may still be reached when candidates exist — do NOT delete it, just add the early return
- The `_healthy_codebase_fallback()` at line 1276 already returns a properly formatted task dict — no changes needed there
- The change is ~2 lines (one condition + one return)
- Update the self-test at line 1953+ if it asserts the old fallback title for the assessment-missing case. The test should now expect the `_healthy_codebase_fallback()` title ("Add a small verifiable improvement to src/") when `assessment_was_missing` is True
