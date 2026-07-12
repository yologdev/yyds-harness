Title: Add diagnostic visibility to state-only tool failure reconciliation
Files: scripts/extract_trajectory.py, scripts/build_evolution_dashboard.py
Issue: none
Origin: planner

Evidence:
- Trajectory Day 134: `state_only_failed_tool_count=61` — "State events contained
  failed tool actions without matching transcript entries."
- Trajectory Day 134: `transcript_only_failed_tool_count=1` — "Recent transcripts
  contained failed tool actions absent from state events."
- The 61 count is large enough to suggest either a real recording gap (tools
  failing before the transcript layer sees them) or a matching/labeling artifact
  where state events and transcript entries use different labels for the same
  failure, causing them to never match.
- Dashboard at scripts/build_evolution_dashboard.py:2712 computes this via
  `unique_delta_count(state_failed_tools_all, transcript_failed_tools_all)` and
  `unique_delta_labels(state_failed_tools_all, transcript_failed_tools_all)` at
  line 2714 — but the LABELS are not surfaced in the trajectory or dashboard
  UI in a way that lets anyone diagnose WHAT tools are failing.
- Trajectory extractor at scripts/extract_trajectory.py:1617-1639 reads the
  counts but doesn't surface the labels.
- Without label visibility, we can't tell if 61 is a real gap or a matching bug.

Edit Surface:
- scripts/extract_trajectory.py — add state-only failure labels to the trajectory
  output (currently only shows count, not which tools). Add a few lines to
  surface `state_only_failed_tool_labels` from the action evidence summary.
- scripts/build_evolution_dashboard.py — no changes needed (labels are already
  computed at line 2714, just not surfaced in trajectory). If investigation
  reveals a matching bug, fix the `unique_delta_labels` or `unique_delta_count`
  logic.

Verifier:
- python3 scripts/extract_trajectory.py 2>&1 | grep -i 'state_only\|tool_failure\|failed_tool' | head -10

Fallback:
- If the labels reveal that all 61 are from the current session or are clearly
  matching artifacts (same tool, different label format), fix the matching logic
  in scripts/build_evolution_dashboard.py. Do not modify src/ files.
- If the labels reveal genuine recording gaps in src/state.rs or src/prompt.rs,
  write findings to session_plan/task_02_findings.md and stop — do not exceed scope.

Objective:
Make state-only tool failures diagnostic-able by surfacing the specific tool
labels in the trajectory output. This lets future sessions see patterns ("all
61 are bash tool errors from before Day 120") and decide whether to fix the
matching or the recording.

Why this matters:
61 is a big number. If it's real, we have a recording gap where tool failures
aren't making it into transcripts — that's evidence loss. If it's a matching
artifact, we're alarming on noise and wasting planning attention. Either way,
the first step is making the number transparent enough to diagnose. Without
labels, it's just a scary number that nobody can act on.

Success Criteria:
- `scripts/extract_trajectory.py` output includes `state_only_failed_tool_labels`
  (or similar) showing which specific tool labels are in the state-only bucket.
- The trajectory's "Graph-derived next-task pressure" section shows concrete
  tool names alongside the count, e.g., "Reconcile state-only tool failures
  (bash=40, read_file=12, write_file=9)".
- The 61 count remains (if real) or drops (if a matching bug is found and fixed).

Verification:
- Run: `python3 scripts/extract_trajectory.py 2>&1 | grep -A5 'state_only'`
- Confirm tool labels are visible in the output.
- If a matching bug was fixed: `python3 scripts/build_evolution_dashboard.py 2>&1 | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('action_evidence_summary',{}).get('state_only_failed_tool_count','?'))"` should show a lower number.

Expected Evidence:
- Next trajectory shows `state_only_failed_tool_count` with labels.
- Dashboard HTML shows state-only failure labels in tooltips (line 7346 already
  has this — the HTML renders `state_only_failed_tool_labels`).
- If a matching bug was fixed: the count drops meaningfully.

Implementation Notes:
1. In scripts/extract_trajectory.py, find where `state_only_failed_tool_count`
   is read (around line 1617). Add reading of `state_only_failed_tool_labels`
   from the same evidence summary.
2. Add the labels to the "Recent tool failures" or "Historical unrecovered tool
   failures" section of the trajectory output. Keep it compact — a few lines
   showing the top 5-10 labels by frequency.
3. If labels are empty but count is 61, that's itself a finding — it means the
   dashboard computed labels but didn't pass them to the trajectory input. Fix
   the data flow.
4. If labels show a clear pattern (e.g., all are "bash" with slight variations),
   note it in the output so future sessions can see the pattern.
5. Do not read or modify src/ files. This is script-only work.
