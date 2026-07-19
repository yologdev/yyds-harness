# Task 02 — Obsolete

**Title:** Add unbounded-command warning to bash safety analysis  
**Status:** Obsolete — already landed by Task 1

## Evidence of prior completion

Task 1 (Day 141, 02:47 session), commit `460a1e03`, landed 142 lines in `src/safety.rs`:

- `check_unbounded_command()` function (lines 1175–1244) — detects:
  - `find /` or `find ~` without `-maxdepth`
  - `grep -r /` / `grep -r ~`
  - `rg pattern /` / `rg pattern ~`
- `is_root_or_home()` helper (lines 1168–1173) — exact match for `/`, `~`, `~/`, `$HOME`, etc.
- Wiring into `analyze_bash_command()` at line 158 as check #27
- 3 test functions (55 passing tests total):
  - `test_analyze_unbounded_find` — 13 assertions covering find warnings and safe bypasses
  - `test_analyze_unbounded_grep_recursive` — 10 assertions
  - `test_analyze_unbounded_ripgrep` — 10 assertions

All success criteria from this task are met:
1. ✅ `analyze_bash_command` detects `find /` / `find ~` without `-maxdepth`
2. ✅ `cargo test safety` passes (55 tests, 0 failures)
3. ✅ No false positives: `find src/`, `find .`, `find / -maxdepth 1` all pass clean

## Why this task was re-created

The planner likely didn't detect that Task 1 already implemented this. The prior attempt mentioned in the task (touching safety.rs + tool_wrappers.rs, reverted by evaluator timeout) was the *failed* attempt. Task 1 succeeded with a narrower scope — same day, earlier session.

## Gap (not blocking, but noted)

The task's implementation notes mention broader detection (`find /tmp`, `find ~/logs` — paths *starting* with `/` or `~` rather than exact match). Task 1 chose the fallback approach (exact match only), which was explicitly allowed by the task's own fallback clause. The primary high-value pattern (`find /` and `find ~`) is covered.

If broader path-prefix detection is desired, a future task could expand `is_root_or_home` to use `starts_with('/') || starts_with('~')` with appropriate safeguards against flag-like arguments.
