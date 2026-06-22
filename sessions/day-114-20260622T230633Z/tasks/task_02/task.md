Title: Close the 1 transcript-only tool failure gap in state/transcript reconciliation
Files: scripts/build_evolution_dashboard.py, scripts/test_build_evolution_dashboard.py
Issue: none
Origin: planner

Evidence:
- Day 114 trajectory: transcript_only_failed_tool_count=1 — one tool failure visible in transcripts absent from state evidence
- Day 114 structured state snapshot: "recent action evidence: state_only_failed_tools=33, transcript_only_failed_tools=1"
- Day 114 state snapshot expected evidence: "task_02=Future dashboard runs show transcript_only_failed_tool_count=0 when state capture is compl"
- Assessment confirms: "1 transcript-only tool failure — State evidence capture missed one tool failure that was visible in transcripts. The gap is small (1 event) but the reconciliation machinery should close it."
- Trajectory graph-derived pressure #5: "Reconcile transcript-only tool failures (transcript_only_failed_tool_count=1)"

Edit Surface:
- scripts/build_evolution_dashboard.py, scripts/test_build_evolution_dashboard.py

Verifier:
- python3 -m pytest scripts/test_build_evolution_dashboard.py -x -q 2>/dev/null || python3 -m unittest scripts.test_build_evolution_dashboard

Fallback:
- If investigation reveals the 1-event gap is a legitimate one-time state recording miss (not a code bug — e.g., a network timeout during event flush), write session_plan/task_02_obsolete.md with the finding and note that evolve.sh state recording is the fix surface (protected).
- If the gap is in protected evolve.sh state recording logic, mark obsolete and note the protected-file constraint.

Objective:
Diagnose and fix the 1-transcript-only-tool-failure gap so that all tool failures visible in transcripts are also captured in state evidence, driving transcript_only_failed_tool_count to 0.

Why this matters:
State/transcript reconciliation is a core harness reliability metric. When state misses tool failures, the dashboard undercounts failures and the evolution feedback loop operates on incomplete data. This gap was expected evidence from a previous session's task_02 — closing it demonstrates the harness can complete cross-session repair loops.

Success Criteria:
- The root cause of the 1 transcript-only gap is identified
- If the bug is in reconciliation (normalization mismatch, comparison logic), it is fixed
- If the gap is in state recording (protected evolve.sh), an obsolete note documents the finding
- The fix or finding is verifiable

Verification:
- python3 -m unittest scripts.test_build_evolution_dashboard
- Manual: check that the normalize_transcript_path / normalize_evidence_path functions handle the mismatched path form
- If a code fix: verify with a unit test that exercises the specific mismatch case

Expected Evidence:
- Future dashboard runs show transcript_only_failed_tool_count=0 when state capture is complete
- The specific mismatched tool failure event is reconciled (appears in both state and transcript evidence)

Implementation Notes:
- Start by understanding how unique_delta_count / unique_delta_labels compute the transcript_only count at lines ~2639-2673 of build_evolution_dashboard.py
- Compare normalize_transcript_path (line ~608) and normalize_evidence_path (line ~636) — a normalization mismatch between these two functions is the most likely cause of a false transcript-only count
- Check if the 1 mismatched event has a path difference (e.g., `./src/file.rs` vs `src/file.rs`, or `$HOME` expansion)
- If the mismatch is in how failed_tools are extracted from event_data vs transcript_actions, trace the extraction logic
- This is a small investigation task — do not refactor the reconciliation system. Find the specific mismatch and apply the narrowest fix.
