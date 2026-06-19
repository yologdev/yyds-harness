Title: Reconcile state-only tool failure recording
Files: scripts/log_feedback.py, scripts/evolve.sh
Issue: none
Origin: planner

Evidence:
- Graph-derived pressure row #5: "Reconcile state-only tool failures (state_only_failed_tool_count=7): State events contained failed tool actions without matching transcript entries."
- Trajectory action evidence: `state_only_failed_tools=7` — tool failures recorded in state events without matching transcript entries.
- Dashboard `unique_delta_labels()` (Day 110) now returns actual tool names for state/transcript disagreements, making this discrepancy diagnosable.
- The two recording systems are: (a) state events written by the Rust harness via `StateRecorder::record()` in `src/state.rs`, and (b) transcript action entries parsed by `scripts/log_feedback.py` from agent log output.
- If state records a `ToolCallCompleted` with error status but the transcript parser misses it, the discrepancy could be: a tool failure that happens outside the transcript's action-line regex, a state event written without a corresponding transcript line, or a transcript line that log_feedback.py's parser doesn't recognize.

Edit Surface:
- scripts/log_feedback.py, scripts/evolve.sh

Verifier:
- python3 scripts/log_feedback.py --self-test (if available) or python3 -c "import scripts.log_feedback" (syntax check)
- grep -n 'state_only_failed\|failed_tool.*transcript\|transcript.*failed_tool' scripts/log_feedback.py scripts/evolve.sh
- After fix: `yyds state failures tools --limit 20` should show failures with matching transcript evidence

Fallback:
- If the discrepancy is caused by a Rust-side bug in `src/state.rs` (e.g., `record_failure` being called without a corresponding tool event), note this in the task outcome and limit the fix to documentation of where the Rust fix should go. Do not expand beyond the listed files.
- If all 7 state-only failures are from historical sessions that pre-date the transcript parser, mark as "historical artifact — no current bug" and close.

Objective:
Ensure every tool failure recorded in state events has a matching transcript entry, so that `yyds state failures` output is complete and cross-references are reliable.

Why this matters:
When state and transcript disagree about what failed, the harness cannot trust its own diagnostics. `state why` and `state failures` commands rely on state events; session dashboards and assessments rely on transcript parsing. A discrepancy between them means one side is blind to real failures — either state is over-reporting (false positives wasting attention) or transcript is under-reporting (real failures go unnoticed). The Day 110 `unique_delta_labels()` work makes this discrepancy VISIBLE but doesn't fix it.

Success Criteria:
- The 7 current `state_only_failed_tools` are either matched to transcript entries (parser improvement) or explained as historical artifacts from before transcript parsing existed.
- New tool failures recorded during this session appear in both state events AND transcript parsing.
- `yyds state failures tools --limit 20` shows failures with transcript evidence when available.

Verification:
- bash -n scripts/evolve.sh
- python3 -c "import ast; ast.parse(open('scripts/log_feedback.py').read()); print('OK')" 
- After the session: run `yyds state failures tools --limit 20` and verify new failures have transcript references.

Expected Evidence:
- Future trajectory shows `state_only_failed_tool_count` dropping toward 0.
- Dashboard `unique_delta_labels()` shows 0 state-only tool names in the delta.
- Assessment self-test `state failures tools` shows failures with transcript source annotations.

Implementation Notes:
- Step 1 (diagnose): In `log_feedback.py`, add a diagnostic mode that lists all state-only tool failures with their event IDs, timestamps, and tool names. Compare against transcript action entries for the same session. Report which failures are truly missing from transcripts vs. which have different event IDs but the same content.
- Step 2 (fix): If the parser regex misses certain failure patterns, add those patterns. If state events are written for internal bookkeeping that has no tool output (e.g., evaluator verdicts), annotate them as non-tool events. If the discrepancy is from sessions before transcript parsing was added, log them as "pre-transcript historical" and exclude from current counts.
- Step 3 (prevention): In `evolve.sh`, after each implementation task, run a quick reconciliation: compare state tool-failure count vs. transcript tool-failure count. Log a warning if they differ by more than 1.
- Keep changes minimal — this is a diagnosis + targeted fix task, not a rewrite.
- If investigation reveals the root cause is in Rust code (`src/state.rs`, `src/tool_wrappers.rs`), document the finding but do NOT edit Rust files. File a follow-up issue instead.
