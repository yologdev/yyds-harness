Title: Add recovery hint for "argument list too long" and "broken pipe" bash failures
Files: src/tool_wrappers.rs
Issue: none
Origin: planner (refined from harness seed)

Evidence:
- Trajectory log feedback (Day 130): "shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks"
- `targeted_recovery_hint` in `src/tool_wrappers.rs:994` covers 7 bash failure classes (exit code, timeout, spawn, file-not-found, permission-denied, command-not-found, generic) but misses two common Linux shell errors: E2BIG ("Argument list too long") from glob expansion on large directories, and EPIPE ("Broken pipe") from pipeline failures where the reader closes before the writer finishes.
- No historical unrecovered tool failures visible in current dashboard — this is a preventative hardening, not a bug fix.
- Assessment confirms tree is clean and build passes.

Edit Surface:
- src/tool_wrappers.rs

Verifier:
- cargo test tool_wrappers
- cargo check

Fallback:
- If `targeted_recovery_hint` already handles these patterns or the function has been refactored away, write an obsolete-task note and the file path checked.
- Do not attempt to validate with real bash commands — this is a string-matching change, not an integration test.

Objective:
Add two targeted recovery hints for "argument list too long" (E2BIG) and "broken pipe" (EPIPE) bash failure patterns so agents get actionable guidance (use `find ... -exec`, split into batches, handle SIGPIPE) instead of a generic fallback hint.

Why this matters:
The trajectory's "Break recurring log failure fingerprints (recurring_failure_count=1)" graph pressure points to shell command failures. When agents hit E2BIG or EPIPE without a specific hint, they waste turns retrying the same broad command. A targeted hint gives them the escape hatch immediately. This is a small, concrete improvement that passes `cargo build && cargo test` — the most landable task type when the codebase is healthy.

Success Criteria:
- Two new `else if` branches added to the `"bash"` match arm in `targeted_recovery_hint`
- "argument list too long" (case-insensitive match) returns a hint suggesting `find ... -exec`, `xargs`, or splitting into batches
- "broken pipe" (case-insensitive match) returns a hint suggesting `set -o pipefail`, handling SIGPIPE, or restructuring the pipeline
- Existing hints unchanged
- `cargo test` passes

Verification:
- cargo test tool_wrappers
- cargo check

Expected Evidence:
- `src/tool_wrappers.rs` appears in task lineage as the sole changed file
- Future bash failures matching "argument list too long" or "broken pipe" produce targeted hints instead of the generic fallback

Implementation Notes:
- Add the new branches before the generic fallback `else` at line 1044, so specific patterns match before the catch-all
- Match on `msg_lower.contains("argument list too long")` and `msg_lower.contains("broken pipe")`
- Keep hints concise (1-2 lines) with an actionable command suggestion
