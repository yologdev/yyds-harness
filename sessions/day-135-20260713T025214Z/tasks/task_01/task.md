Title: Filter active-session runs from unmatched lifecycle count in dashboard
Files: scripts/build_evolution_dashboard.py
Issue: #97
Origin: planner (refined from harness-seed)

Evidence:
- Trajectory Day 134: `state_run_unmatched_non_validation_completed_count=35` — #1 graph-pressure item.
- Dashboard at `scripts/build_evolution_dashboard.py:2395-2440` computes unmatched runs as:
  `run_unmatched_completed_ids = sorted(run_id for run_id in run_completed_ids if run_id not in run_started_ids)`.
  This includes runs from the currently-active session whose RunStarted events haven't been written yet
  (normal — the session is still running, so Start/Completed events are in-progress).
- Assessment confirms: "Some may be from the current active session (normal — runs haven't finished yet
  so their Start events haven't been paired)."
- Day 132 already fixed a field-name bug swapping `unmatched_completed_details` for
  `unmatched_non_validation_completed_details`. The 35 remaining are at least partly active-session noise.
- Issue #97 was reverted Day 134 — agent spent 23 turns investigating without landing code.
  This narrowed version targets ONE file with ONE concrete filter.

Edit Surface:
- scripts/build_evolution_dashboard.py — add active-session run exclusion to the unmatched count computation
  at or near line 2395. Extract the current session's run_id prefix (e.g., from the newest RunStarted or
  SessionStarted event, or from a timestamp-based heuristic for runs newer than 1 hour), then skip those
  run_ids when computing `run_unmatched_completed_ids`.

Verifier:
- python3 -c "import scripts.build_evolution_dashboard as b; print('import ok')"
- python3 scripts/build_evolution_dashboard.py --help 2>&1 | head -5

Fallback:
- If the filter can't identify the active session without reading env vars or external state, skip the
  runtime filter and instead document in a comment at line 2435 that the count includes in-progress runs.
- If all 35 are genuinely orphaned (not active-session), mark the task complete with a note that the
  filter was added but didn't change the count — the remaining runs need a different fix.
- Do NOT modify event files or src/ Rust code. Keep changes to this one file.

Objective:
Reduce noise in the `state_run_unmatched_non_validation_completed_count` by excluding runs from the
currently-active session that legitimately haven't had their RunStarted events written yet.

Why this matters:
This is the #1 graph-pressure item in trajectory. The unmatched lifecycle count feeds into evo-readiness
assessment — when active-session noise inflates it, the signal-to-noise ratio drops and real problems
are harder to spot. A session that can't distinguish "genuinely orphaned runs" from "runs still in progress"
can't trust its own health metrics.

Success Criteria:
- The dashboard computation at line ~2395 excludes runs whose run_id belongs to the current active session.
- The filter is defensive: if it can't identify the active session, it falls back to the existing behavior
  rather than crashing.
- `python3 scripts/build_evolution_dashboard.py` still runs without errors.

Verification:
- python3 scripts/build_evolution_dashboard.py 2>&1 | python3 -c "
import sys, json
d = json.load(sys.stdin)
runs = d.get('state_lifecycle', {}).get('runs', {})
print(f'unmatched_non_validation_completed={runs.get(\"unmatched_non_validation_completed\", \"?\")}')"
- Confirm the count is ≤ 35 (same or lower — lower if the filter catches active-session runs).

Expected Evidence:
- Next trajectory shows `state_run_unmatched_non_validation_completed_count` ≤ 35 (lower if active-session
  runs were inflating it).
- The filter appears in the dashboard source at the unmatched computation site.
- No regression: dashboard still produces valid JSON output.

Implementation Notes:
1. The unmatched computation is around lines 2395-2440 in `build_evolution_dashboard.py`. Look for
   `run_unmatched_completed_ids = sorted(run_id for run_id in run_completed_ids if run_id not in run_started_ids)`.
2. To identify active-session runs: find the newest `SessionStarted` or `RunStarted` event timestamp,
   then exclude run_ids whose start event timestamp (or the run_id itself, if it contains a timestamp)
   is within the last 2 hours. Or use a simpler heuristic: if `run_started_ids` is empty for a given
   run AND the run's events are all from the same session file (per-session events), skip it.
3. The simplest safe approach: after computing `run_unmatched_completed_ids`, also compute
   `run_unmatched_from_current_session` — runs that appear in the per-session events file for the
   current session (no cross-session file). Subtract those from the count.
4. Keep the change to ≤10 lines. A one-line filter addition is ideal.
5. Test with: `python3 scripts/build_evolution_dashboard.py 2>&1 | head -1` to confirm it runs.
