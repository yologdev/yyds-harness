Title: Improve retroactive FailureObserved source classification in src/state.rs
Files: src/state.rs
Issue: none
Origin: planner (refined from harness-seed)

Evidence:
- `yyds state why last-failure` shows retroactive FailureObserved from day-164 16:57 session with source=unknown, class=unknown — the harness knows the run completed with error status "reverted" but can't classify it.
- The retroactive FailureObserved is emitted by the state doctor's orphaned-run detection, which finds runs with error status but no FailureObserved. Currently it sets source=unknown and class=unknown.
- When a RunCompleted event carries error_detail containing "reverted", "timeout", "cancelled", "build_failure", or "test_failure", the FailureObserved should carry that classification instead of "unknown".
- Day 163 shipped a panic-hook fix in state.rs (3-line change: peek before consume) that passed strict verification. Small, surgical state.rs edits are proven to ship.

Edit Surface:
- src/state.rs

Verifier:
- cargo test state -- --test-threads=1

Fallback:
- If `yyds state why last-failure` already shows a non-unknown source for the most recent failure, mark this task obsolete — the classification gap self-resolved.
- If the RunCompleted events don't carry error_detail strings that can be mapped to source/class categories, narrow scope to document the gap rather than forcing a classification.

Objective:
Make retroactive FailureObserved events carry a meaningful source and class derived from the RunCompleted event's error_detail, so the state doctor's failure detection produces actionable classification instead of "unknown/unknown."

Why this matters:
The trajectory shows `task_unlanded_source_count=1` and `reverted_unlanded_source_edits` across multiple sessions. When the harness retroactively classifies a failure as "unknown," it can't distinguish infrastructure timeouts from real code problems from cancelled CI runs. This feeds false pressure into the trajectory graph and wastes task slots on misdiagnosed failures. A 10-20 line change that maps RunCompleted error_detail to FailureObserved source/class makes the failure signal actionable.

Success Criteria:
- When the state doctor retroactively adds a FailureObserved for a RunCompleted with error_detail containing a recognizable keyword ("reverted", "timeout", "cancelled", "build_failure", "test_failure"), the FailureObserved's source field reflects that keyword instead of "unknown".
- The fix does not regress existing state.rs tests.
- The change is under 30 lines.

Verification:
- cargo test state -- --test-threads=1
- cargo build

Expected Evidence:
- Next `yyds state why last-failure` shows source="reverted" or similar non-unknown value for retroactive FailureObserved events.
- task_unlanded_source_count gnome becomes more precise (distinguishes reverted from timeout from cancelled).

Implementation Notes:
- Find the code path that creates retroactive FailureObserved events. Search for where FailureObserved is emitted with source="unknown" — likely in `close_orphaned_run_if_needed()` or the state doctor's lifecycle check in `src/state.rs`.
- The RunCompleted event's payload should have an `error_detail` or `status` field. Map known error_detail strings to source/class pairs:
  - "reverted" → source="task_revert", class="verification"
  - "timeout" → source="timeout", class="infrastructure"
  - "cancelled" → source="cancelled", class="external"
  - "build_failure" → source="build", class="code"
  - "test_failure" → source="test", class="code"
- Use a simple match or if-else chain. Do not add new dependencies, traits, or modules.
- Default fallback: keep source="unknown", class="unknown" for unrecognized error_detail values.
- Keep the change under 30 lines. Do not refactor or extract helper functions.
