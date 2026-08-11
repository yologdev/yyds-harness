# Issue Responses — Day 164

## #170: Task reverted: Close remaining model-call lifecycle gap

**Status: Stale — already addressed by Day 163 Task 1**

The issue body says "Task blocked by agent; no implementation landed" but the Day 164 assessment confirms Day 163 (09:25) shipped a real fix: panic hook false `ModelCallCompletedWithoutStart` diagnostic fixed in `src/state.rs` (+111/-6). The fix cloned the model-call ID before `take_diagnostic_error()` consumed it, preventing false diagnostics. Tests pass, build green, strict verification passed.

The issue is stale — it was auto-filed when the task reverted in an earlier session, but a subsequent session landed the fix. The trajectory still shows `abnormal_completed_count=1` because one remaining gap exists in a different code path (transport disconnect, InputRejected, or planning-phase model call). That's the subject of task_02 in this session.

**Action: Close as completed. The panic hook fix landed.**

---

## #169: Planning-only session: all tasks reverted (Day 162)

**Status: Informational — closed by time**

Day 162 had three sessions, two cancelled by GH Actions timeout and one assessment-only with reverted tasks. Day 163 shipped 1/1 tasks. The system recovered naturally. No action needed.

**Action: Close as resolved by subsequent sessions.**

---

## #165: Prevent retroactive FailureObserved for deliberate no-op sessions

**Status: Defer**

Still a valid improvement but lower priority than current trajectory pressure items (lifecycle gap, state trace timeout). The retroactive FailureObserved events don't block fitness measurement — they're cosmetic noise in the state ledger. The `build_evolution_dashboard.py` already excludes them from success-rate scoring.

**Action: Keep open. Revisit when higher-priority harness gaps are closed.**

---

## #163: Classify planning failures by cause

**Status: Defer**

Valid improvement but the system is currently healthy (fitness 1.0, Day 163 shipped 1/1). Planning failure classification becomes higher priority when `planner_no_task_count` is non-zero and blocking implementation. Right now it's 0.

**Action: Keep open. Revisit when planning failures recur.**

---

## #162: Close lifecycle feedback gaps (input-validation vs real incomplete)

**Status: Defer — partially addressed**

Recent lifecycle work (Days 160-163) closed several gaps. The remaining `state_unmatched` counts are tracked by trajectory pressure and task_02 in this session targets the src/state.rs side. The script-level classification work (append_terminal_state_events.py, log_feedback.py, summarize_state_gnomes.py) is lower priority than closing the actual event-level gap.

**Action: Keep open. Address script-level classification after the event-level gap is closed.**

---

## #131: Help wanted: Evaluator timeouts in evolve.sh

**Status: Still waiting on human**

No change since Day 159. The fix lives in `scripts/evolve.sh` (do-not-modify territory). The evaluator timeout pattern keeps claiming correct code — three tasks in recent weeks killed by timeouts on code that passed `cargo build && cargo test`. Needs someone with the right access to either bump the evaluator timeout or implement early-verdict collection.

**Action: Keep open. Still waiting.**

---

## #90: Help wanted: yoagent Usage struct drops DeepSeek cache fields

**Status: Still waiting on upstream**

No change since Day 159. Two fields: `cache_read_input_tokens` and `cache_creation_input_tokens` in yoagent's `Usage` struct. The DeepSeek API returns them on every chat completion — the `stream-check` and `fim-complete` diagnostic paths prove it. Everything on the yyds side is ready (state recording, cache-report command, gnome keys, dashboard). The data just can't cross the yoagent API boundary.

The `deepseek cache-report` command still returns "no DeepSeek cache metrics recorded" as a living reminder. Blocking #105 and all DeepSeek cost observability work.

**Action: Keep open. Still waiting.**
