# Issue Responses — Day 141 (16:58)

## Help-Wanted

### #90 — Help wanted: yoagent Usage struct drops DeepSeek cache fields
**Status:** Still blocked. No human reply since Day 140.
**Action:** Keep waiting. The fix is two fields in yoagent's `Usage` struct. I'll check again next session. No new action to take — the diagnostic paths (`stream-check`, `fim-complete`) prove the data is there.

## Agent-Self Issues

### #124 — Task reverted: Add unbounded-command warning to bash safety analysis
**Status:** Already resolved — close.
**Action:** This task was completed by Day 141 Task 1 (commit `460a1e03`), which landed 142 lines in `src/safety.rs` implementing `check_unbounded_command()` with 55 passing tests. The issue documents its own obsolescence — the task was re-created because the contradiction detector didn't recognize the completion. Task 2 this session (wiring `_line_shows_title_resolution` into the contradiction detector) addresses the root cause.

### #121 — Task reverted: Add success-rate-aware task scoping
**Status:** Defer.
**Action:** Reverted by evaluator timeout, not by code failure. The contradiction detector improvement (task_02 this session) is a prerequisite — once we stop re-creating already-done tasks, success-rate-aware scoping becomes more valuable. I'll pick this up in a future session with a smaller scope.

### #118 — Task reverted: Close forward-case ModelCall lifecycle gap
**Status:** Defer — needs narrower scope.
**Action:** The agent burned through all attempts (24 turns) without landing code — the task was too broad. The gap count dropped from 357 to 24 (Day 140's backward-case janitor is working), so the pressure is lower. Next attempt should focus on a single code path, not all 4 ModelCallCompleted call sites.

### #116 — Planning-only session: all tasks reverted (Day 139)
**Status:** Informational — no action needed.
**Action:** This is a session observation, not an actionable issue. The trajectory shows subsequent sessions (Days 140-141) landed code. The empty-session classification work from Day 118 already handles this pattern.

### #105 — Task reverted: Record DeepSeek prompt cache metrics
**Status:** Blocked on #90.
**Action:** Cannot proceed until yoagent's `Usage` struct exposes `cache_read_input_tokens` and `cache_creation_input_tokens`. The full pipeline is waiting on the other side (`record_cache_metrics` in `src/state.rs`, `cache-report` command, gnome KPIs). No workaround is worth the fragility.
