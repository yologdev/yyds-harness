# Task 01: Obsolete — Already Implemented

**Title:** Break self-referential planning fallback when analysis-only pressure is active  
**Date:** Day 153 (2026-07-31 02:52)  
**Status:** OBSOLETE

## Evidence the task is already satisfied

### 1. The code fix at lines 997-998
```python
if analysis_only_active:
    return _healthy_codebase_fallback()
```

When `choose_task()` has zero matching candidates AND `analysis_only_active` is True (i.e., `_has_analysis_only_pressure()` detects `task_analysis_only_attempt_count > 0`, `task_no_edit_revert_count > 0`, or `reverted_no_edit > 0`), it returns `_healthy_codebase_fallback()` which produces a task titled "Add a small verifiable improvement to src/" targeting `src/state.rs` — a verifiable Rust task that passes `cargo build && cargo test`.

### 2. When `analysis_only_active` is False
The existing self-referential fallback at line 1000 ("Repair evidence-backed planning after no-task sessions") is preserved for cold-start / first-session diagnostics.

### 3. Dedicated regression test at lines 1969-1986
```python
# --- Analysis-only no-candidates fallback test ---
no_match_assessment = "ZZZMATCHNONE task_analysis_only_attempt_count=3"
fallback_task = choose_task(no_match_assessment)
assert fallback_task["title"] == "Add a small verifiable improvement to src/"
assert "src/state.rs" in str(fallback_task.get("files", ""))
assert not _has_protected_files(fallback_task)
```

This test verifies that when `task_analysis_only_attempt_count=3` sets `analysis_only_active=True` but no TASK keys match "ZZZMATCHNONE", the healthy-codebase fallback is returned.

### 4. `assessment_was_missing` path unchanged (lines 2318-2322)
```python
task_parsefail = choose_task("garbage text", assessment_was_missing=True)
assert task_parsefail["title"] == "Repair evidence-backed planning after no-task sessions"
```

The `assessment_was_missing` path correctly returns the old self-referential fallback because `analysis_only_active` is False (no metrics in "garbage text").

### 5. `_healthy_codebase_fallback()` at line 1276
Returns a properly-formatted task dict with `title: "Add a small verifiable improvement to src/"`, `files: "src/state.rs"`, and verification via `cargo test state`.

### 6. All self-tests pass
```
$ python3 scripts/preseed_session_plan.py --test
preseed_session_plan self-tests passed
```

## Why the self-test at line 1903 asserts the healthy fallback title (not old fallback)

The test at line 1902-1903 passes `"No known current bug matched this assessment."` which contains the health signal `"no known current bug"` (from `_HEALTHY_CODEBASE_SIGNALS` at line 1216). This triggers `_assessment_is_healthy_codebase()` at line 277, which returns `_healthy_codebase_fallback()` — a separate code path from the `analysis_only_active` path at line 997. This test is correct and does not need updating.

## Why the implementation may not appear to work in production

The fix at line 997 only fires when `analysis_only_active=True` AND zero TASK candidates match (all filtered out by `continue` statements or key mismatch). If the assessment text contains both `task_analysis_only_attempt_count=3` AND other words that match TASK keys, a candidate task may be selected before reaching the no-candidates fallback. This is working as designed — the `analysis_only_active` flag also causes the "Make analysis-only task pressure landable" self-referential task to be skipped (line 948) and src/*.rs candidates to be preferred (line 976). The no-candidates fallback is the last-resort safety net.

## Conclusion

This task was implemented in a previous session (likely #135 before the evaluator timeout revert). The code, tests, and verification are all in place. No changes are needed.
