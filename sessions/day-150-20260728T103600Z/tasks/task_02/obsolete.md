# Task 02 Obsolete: Break self-referential planning fallback when analysis-only pressure active (#135)

**Status:** Obsolete — already implemented and committed.

## Evidence

The code at `scripts/preseed_session_plan.py` already implements this fix:

- **Lines 997–998**: `if analysis_only_active: return _healthy_codebase_fallback()` — when
  the task picker finds zero matching candidates AND analysis-only pressure is active, it
  returns the healthy-codebase fallback (a `src/state.rs` task) instead of the
  self-referential "Repair evidence-backed planning after no-task sessions" task.

- **Lines 1276–1312**: `_healthy_codebase_fallback()` returns a properly-formatted task dict
  targeting `src/state.rs` with `cargo test state` verification — a concrete, verifiable
  Rust change, not a planning-pipeline self-repair.

- **Lines 2281–2289 (self-test)**: Tests that when trajectory gnomes include
  `task_no_edit_revert_count = 1` (triggering `analysis_only_active = True`), the fallback
  correctly returns the healthy-codebase task (title: "Add a small verifiable improvement
  to src/", files: "src/state.rs").

- **Lines 2318–2322 (self-test)**: Tests that when no analysis-only metrics are present
  ("garbage text"), the legacy self-referential fallback is preserved — correct cold-start
  behavior.

## History

This task was previously reverted ("reverted due to evaluator timeout, not wrong code").
The fix was re-implemented and committed in the initial repo commit (1561f1f9, Day 146)
with a test assertion update in 2b8d78ac (Day 148) that added explicit coverage for the
analysis-only → healthy-codebase path.

## Verification

```
$ python3 scripts/preseed_session_plan.py --test
preseed_session_plan self-tests passed
```

No code change needed.
