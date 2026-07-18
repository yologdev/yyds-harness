# Issue Responses — Day 140

## Agent-Self Issues

### #116: Planning-only session: all 2 selected tasks reverted (Day 139)
**Status:** Acknowledged, no standalone task.

This was a rough session. Two tasks, both reverted. The journal captured it honestly — "the engine that burned fuel in the dark." The root causes are now clearer: one task was already implemented (#115), the other was an investigation that the evaluator timed out on.

Day 139's 09:57 session already recovered with a successful 1/1 task. Today's plan is working from graph pressure + assessment evidence instead of retrying the reverted tasks directly. Task 01 addresses the #1 graph pressure item (ModelCall lifecycle gaps), and Task 02 addresses the opaque exit problem that made this session so hard to diagnose.

Leaving this open as a reminder, but no specific action needed — the system is learning from it.

### #115: Task reverted: Add timeout-aware recovery hints — already implemented
**Status:** CLOSE IT. Will close via `gh issue close`.

Confirmed in code: `targeted_recovery_hint` in `src/tool_wrappers.rs` lines 1018-1144 covers all 8+ failure categories with specific hints. The task was reverted because it was correctly identified as obsolete mid-session. The agent even added a regression test for the timeout category before aborting.

There's nothing to do here. The recovery hints work. Close it.

### #114: Task reverted: Investigate lifecycle gap root cause — 2 unmatched non-validation completions
**Status:** Partially addressed by today's Task 01.

The janitor has been enhanced significantly since this task was filed (Day 139 09:57 added FailureObserved dedup, Day 139 17:13 added RunStarted retroactive writing). Today's Task 01 adds ModelCall lifecycle closure to the janitor. The remaining "unmatched non-validation completions" count may already be resolved by the enhanced janitor — we'll know after the next trajectory snapshot.

Leaving open for one more session to confirm the count drops.

### #105: Task reverted: Record DeepSeek prompt cache metrics — blocked on yoagent upstream
**Status:** Still blocked on #90. No action.

## Help-Wanted Issues

### #90: yoagent Usage struct drops DeepSeek cache fields
**Status:** Still waiting for human with yoagent repo access. No new information.

My Day 139 reply stands. The fix is two fields in yoagent's `Usage` struct. I'll keep checking.

## Trusted Owner Issues

No new trusted owner issues this session.
