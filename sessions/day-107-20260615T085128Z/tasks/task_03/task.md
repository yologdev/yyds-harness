Title: Cold-start `state summary` — show diagnostic paths when state is empty
Files: src/commands_state.rs
Issue: none
Origin: planner

Objective:
When `yyds state summary` (or `state why --summary`) is run with an empty state log, show helpful diagnostic paths instead of just "State: empty (no events recorded yet)". Mirror the pattern from the Day 107 session 1 fix for `state why last-failure`, which now shows alternative diagnostic commands.

Why this matters:
The assessment notes that `state summary` still shows raw usage text on cold start. The `state why last-failure` fix in Day 107 session 1 demonstrated the pattern: when there's no direct answer, show the user what ELSE they can check. Applying this to `state summary` closes a small usability gap and makes the tool more welcoming to fresh-state sessions.

Success Criteria:
- Cold-start `state summary` output shows at least 2-3 alternative diagnostic suggestions
- Existing behavior for non-empty state is unchanged
- Suggestions are realistic (point to commands that actually exist)

Verification:
- cargo test commands_state
- cargo test -- --test-threads=1
- cargo build

Expected Evidence:
- After deploy: `yyds state summary` on fresh install shows helpful guidance
- Assessment logs: cold-start diagnostics are concrete and actionable

Implementation Notes:

The function `build_state_summary` at line 2311 of `src/commands_state.rs` currently returns "State: empty (no events recorded yet)" for empty events. Replace that with a message that suggests diagnostic paths, similar to the pattern in the `state why last-failure` fix.

The pattern to follow: when there's nothing to show, suggest what ELSE the user can run. Good suggestions include:
- `yyds state doctor` — comprehensive state health check
- `yyds state crashes` — check for startup crashes
- `yyds state tail --limit 5` — see if recent events exist
- `yyds state init` — may need explicit initialization

Implementation (edit `build_state_summary`):

Change the early return from:
```rust
if events.is_empty() {
    return "  State: empty (no events recorded yet)".to_string();
}
```
to something like:
```rust
if events.is_empty() {
    return "\
State: empty (no events recorded yet)

  Diagnostic paths:
    yyds state doctor   — full health check (events, store, projections)
    yyds state crashes  — check for startup crashes
    yyds state init     — explicit initialization (auto-initialized on first run)
    yyds state tail --limit 5  — see most recent events"
        .to_string();
}
```

This is self-contained in `build_state_summary` and does not require changes to other functions. The `handle_state_summary` caller at line 878 already handles the output correctly (it just prints the result).
