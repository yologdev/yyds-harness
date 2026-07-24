Title: Filter harness-internal ModelCallCompleted events from unmatched count
Files: scripts/summarize_state_gnomes.py
Issue: none
Origin: planner

Evidence:
- Trajectory graph pressure #1: deepseek_model_call_unmatched_completed_count=154 — lifecycle causes include model_completion_without_start=8
- Assessment: "154 ModelCallCompleted events without matching ModelCallStarted — the state recorder captures completions but sometimes misses starts"
- These are evt-harness-* prefix events with tokens=in:0 out:0 cache_read:0 cache_write:0 — harness-internal cleanup, not real model calls
- The counting logic in summarize_state_gnomes.py line 357 adds every run with a ModelCallCompleted to `model_call_completed_runs`, then line 394-395 counts runs in that set without a matching ModelCallStarted
- Harness-internal events produce zero-token ModelCallCompleted records that inflate the unmatched count without representing real model activity

Edit Surface:
- scripts/summarize_state_gnomes.py

Verifier:
- python3 -c "import scripts.summarize_state_gnomes"  (syntax check)
- Manual: run `target/debug/yyds state doctor` or check trajectory after next session for decreased unmatched count

Fallback:
- If evt-harness events already have a `retroactive: true` flag that can be checked instead of ID prefix, prefer that check.
- If the unmatched count does not decrease after the fix, mark this task obsolete and note that the unmatched events are in runs that also have real ModelCallCompleted events (the count is already correct and the issue is elsewhere).

Objective:
Stop counting harness-internal (evt-harness-* prefix) ModelCallCompleted events as unmatched model calls, so the `deepseek_model_call_unmatched_completed_count` metric reflects only real agent-model lifecycle gaps.

Why this matters:
The trajectory consistently reports 150+ unmatched ModelCallCompleted events as the #1 graph-derived pressure. But these are harness-internal cleanup events (evt-harness-* prefix, zero tokens), not real model API calls. They inflate the count and obscure whether any real agent-model lifecycle gaps remain. A 3-line filter in the counting script makes the metric trustworthy.

Success Criteria:
- evt-harness-* ModelCallCompleted events are excluded from `model_call_completed_runs` in summarize_state_gnomes.py
- The `deepseek_model_call_unmatched_completed_count` gnome decreases (historical 154 from past runs remain, but new runs add zero from harness-internal events)
- No regression in the state lifecycle report output

Verification:
- python3 -c "import scripts.summarize_state_gnomes; print('syntax ok')"
- Manual: after next evolution session, check trajectory for decreased unmatched count

Expected Evidence:
- Trajectory deepseek_model_call_unmatched_completed_count stops growing from harness-internal events
- State lifecycle report shows only real agent-model lifecycle gaps
- Graph pressure #1 diminishes

Implementation Notes:
- The fix is at line 356-357 in `scripts/summarize_state_gnomes.py` — inside the `elif kind == "ModelCallCompleted"` block
- Before `model_call_completed_runs.add(run_id)`, check if the event ID starts with `evt-harness-`:
  ```python
  if run_id:
      evt_id = str(event_id(event) or "")
      if not evt_id.startswith("evt-harness-"):
          model_call_completed_runs.add(run_id)
  ```
- `event_id()` is already defined at line 153 — returns `event.get("event_id") or event.get("id")`
- This is a ~3 line change — just wrap the `model_call_completed_runs.add(run_id)` call with an evt-harness prefix check
- Do NOT change any other logic — the filter only affects whether harness-internal completions count toward the unmatched total
- If the harness-internal events also have `retroactive: true` in their payload, prefer checking that flag instead of event ID prefix (more semantically correct)
