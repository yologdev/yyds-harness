Title: Distinguish cold-start "no data" from healthy "no failures" in state why last-failure
Files: src/commands_state.rs
Issue: none
Origin: planner

Evidence:
- Assessment self-test: `yyds state why last-failure` produced "No completed failed sessions exist" in a fresh CI state store that had only the current (in-progress) run. The message is technically correct but unactionable — it doesn't tell the user whether the system is working (all sessions passed) or broken (no sessions have completed at all).
- In a cold-start CI environment, "No completed failed sessions exist" is misleading: it sounds like a healthy state when in fact no sessions have finished recording. The user needs to know they should run `yyds state init` and wait for a session to complete.
- The code at line 1162 (`src/commands_state.rs`) already has the `id == "last-failure"` branch for the cold-start message path. It should differentiate between: (a) no completed runs at all → cold start; (b) completed runs exist but none failed → healthy; (c) failures exist but weren't in the scanned window → bounded scan limit.

Edit Surface:
- src/commands_state.rs — the `handle_why` function, specifically the diagnostic messages around line 1155-1180 and the `find_incomplete_runs` fallback logic around line 1197-1243.

Verifier:
- cargo test --bin yyds -- --test-threads=1
- cargo test --test integration -- --test-threads=1

Fallback:
- If `state why last-failure` with an explicit `--limit` already produces actionable output in the CI environment, or the cold-start detection already exists, write task_02_obsolete.md.

Objective:
Make `yyds state why last-failure` produce a distinct message for cold-start environments (no completed runs) vs. healthy environments (completed runs exist, none failed), so the user knows whether to initialize state or look elsewhere for problems.

Why this matters:
The `state why` command is the primary diagnostic entry point. When it says "No completed failed sessions exist," the user doesn't know if:
1. The system is healthy (all sessions passed) — no action needed
2. The system hasn't recorded anything yet (cold start) — needs `yyds state init` and a completed session
3. There might be failures but the scan window was too narrow — needs `--limit 200000`

The assessment itself hit this confusion: the CI state store had only 288 events from the current in-progress run. "No completed failed sessions exist" was correct but hidden the fact that NO sessions had completed at all. A cold-start message would have made the next action obvious.

Success Criteria:
- When 0 completed runs exist: message says "no completed sessions exist yet — this may be a cold start"
- When completed runs exist but 0 failures: message says "all N completed sessions passed" (or similar)
- When failures might exist outside the scan window: message already exists (line 1191-1193)

Verification:
- cargo build && cargo test --bin yyds
- Manual check: `yyds state why last-failure` in a state store with 0 completed runs → should say something distinct from a store with completed-but-healthy runs
- Existing behavior when failures are found must not regress

Expected Evidence:
- Assessment self-tests in future sessions show `state why last-failure` producing different messages for cold-start vs. healthy-no-failures
- User-facing diagnostic quality gnome improves (fewer "I ran this and it said nothing useful" moments)

Implementation Notes:
- The change should be small: modify the message at line 1162-1163 based on whether any RunCompleted events exist in the scanned events.
- Check `find_incomplete_runs` already parses the events for run lifecycle. Use a similar approach to detect completed runs.
- The existing fallback logic (lines 1197-1243) already detects incomplete runs — extend it to also detect the cold-start case.
- Do not change the `DEFAULT_WHY_LIMIT` or `read_tail_events` logic — the bounded scan is already correct.
