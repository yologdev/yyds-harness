Title: Tighten `state why last-failure` messaging when incomplete runs exist
Files: src/commands_state.rs
Issue: none
Origin: planner (refined from harness-seed)
validated_against_assessment: true

Evidence:
- `yyds state why last-failure` output (verified Day 113 23:04): first line says "no state event found for 'last-failure'" — then immediately reports useful diagnostics (1 incomplete run, suggestions). The leading error message is confusing and wrong: the search DID find meaningful evidence.
- The `build_why_report` function at src/commands_state.rs:1118 handles `id == "last-failure"` but always emits the "no state event found" line before falling through to the incomplete-run scan at line 1153.
- Assessment (Day 113 23:04) confirms `state why last-failure` "correctly reports no failures and 1 incomplete run" — the diagnostics are correct but the framing line contradicts them.

Edit Surface:
- src/commands_state.rs

Verifier:
- cargo test commands_state state -- --test-threads=1

Fallback:
- If `state why last-failure` already shows a different first line that doesn't say "no state event found" when incomplete runs are detected, mark this task obsolete.

Objective:
Make `yyds state why last-failure` say "No completed failure sessions found" (instead of "no state event found") when the scan detects incomplete runs or other nearby evidence, so the first line doesn't contradict the diagnostics that follow.

Why this matters:
A confusing first line undermines trust in state diagnostics. When a user runs `state why last-failure` and the first thing they see is "no state event found for 'last-failure'" — but the next lines show incomplete runs and actionable suggestions — the contradiction makes the tool feel broken even though it works correctly. The fix is to distinguish "no completed failure" from "no evidence at all."

Success Criteria:
- When there are 0 completed failures but ≥1 incomplete run: first line says "No completed failure sessions found." (not "no state event found for 'last-failure'")
- When there are 0 completed failures and 0 incomplete runs: first line says "No completed failure sessions found." with appropriate "no nearby evidence" follow-up
- When there is a completed failure (existing behavior): output unchanged
- `cargo test commands_state state` passes

Verification:
- cargo test commands_state state -- --test-threads=1
- cargo check

Expected Evidence:
- `state why last-failure` output no longer begins with a self-contradicting error line when incomplete runs exist
- `cargo test commands_state state` passes with existing and any new test coverage

Implementation Notes:
- The key location is `build_why_report` in `src/commands_state.rs`. When `id == "last-failure"` and no FailureObserved event is found, but incomplete runs are detected in the fallback scan, change the leading message from "no state event found for 'last-failure'" to something like "No completed failure sessions found." or "No failure event found — nearest evidence:"
- Existing tests at lines ~15658+ (`why_report_finds_last_failure`, etc.) test the `build_why_report` function. Add or update assertions to verify the new messaging.
- Keep the existing "no state event found" wording for cases where no nearby evidence (incomplete runs, stash errors, diagnostic errors) exists at all — that's the honest message when truly nothing is found.
