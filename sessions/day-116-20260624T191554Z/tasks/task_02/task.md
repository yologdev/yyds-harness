Title: Surface state/transcript tool failure mismatch labels in trajectory output
Files: scripts/extract_trajectory.py, scripts/test_extract_trajectory.py
Issue: none
Origin: planner

Evidence:
- Graph pressure row #4: transcript_only_failed_tool_count=2 — recent transcripts contained failed tool actions absent from state events.
- Graph pressure row #5: state_only_failed_tool_count=36 — state events contained failed tool actions without matching transcript entries.
- Assessment bug #1 [MEDIUM]: "36 state events show tool failures without matching transcript entries, and 2 transcripts show failures without state events. This indicates either: (a) tool failures are being recorded inconsistently between the two tracking systems, or (b) the reconciliation logic has a bug. Root cause unknown without further investigation."
- The current trajectory output only shows counts (`state_only_failed_tool_count=36`, `transcript_only_failed_tool_count=2`) but not the specific failure labels. Without labels, the mismatch can't be diagnosed or fixed.
- The assessment confirms the root cause is unknown — this task is characterization, not repair.

Edit Surface:
- scripts/extract_trajectory.py, scripts/test_extract_trajectory.py

Verifier:
- python3 -m unittest scripts.test_extract_trajectory

Fallback:
- If the trajectory extractor already surfaces specific failure labels for state-only and transcript-only mismatches, or if the mismatch counts drop to 0 in the current trajectory snapshot, write session_plan/task_02_obsolete.md with the evidence.

Objective:
Add diagnostic output to the trajectory extractor that surfaces the specific tool failure labels (not just counts) for state-only and transcript-only mismatches. This enables the next session to see which specific failure categories are mismatched and fix the root cause (recording bug or reconciliation bug) with concrete evidence.

Why this matters:
The 36 state-only vs 2 transcript-only mismatch is a data integrity gap. Without knowing which failure labels are involved, no implementation agent can fix the root cause. This task makes the mismatch diagnosable by adding label-level detail to the trajectory output, converting "36 state-only failures" into "36 state-only failures: bash_tool_error=14, read_file_error=8, ..." so the next session can act on the actual categories.

This addresses graph pressure rows #4 and #5. The state_capture_coverage diagnostic gnome is blocked until this mismatch is understood.

Success Criteria:
- When `state_only_failed_tool_count > 0` or `transcript_only_failed_tool_count > 0`, the rendered trajectory output includes the specific failure labels (not just counts) for each category.
- The label output is compact — at most 3 lines per category, showing the top failure labels by frequency.
- `python3 -m unittest scripts.test_extract_trajectory` passes, including new test cases that verify label output when mismatches exist.

Verification:
- python3 -m unittest scripts.test_extract_trajectory

Expected Evidence:
- Next trajectory snapshot shows labels like `state_only_failed_tool_count=36 (bash_tool_error=12, edit_file_error=8, ...)` instead of just the count.
- The label detail enables the next planning session to identify whether the mismatch is concentrated in specific tool categories (pointing to a recording bug in those categories) or spread evenly (pointing to a reconciliation regex mismatch).

Implementation Notes:
- The trajectory extractor already has `render_graph_suggestions` which produces the graph-derived pressure lines. The state-only and transcript-only counts are computed in `collect_provider_errors` or nearby functions — find where `state_only_failed_tool_count` and `transcript_only_failed_tool_count` are computed and add label-level detail.
- Look at `scripts/build_evolution_dashboard.py` for `failed_tool_category()` and `transcript_only_failed_tool_labels` — these already compute the labels but the trajectory extractor only renders counts.
- Do not change the reconciliation logic itself — only add diagnostic output. The fix belongs in a follow-up session once the labels are visible.
- Add test fixtures in `scripts/test_extract_trajectory.py` that verify label rendering for both state-only and transcript-only mismatch cases.
