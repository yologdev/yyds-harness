Title: Add issue #90 tracking link to deepseek cache-report output
Files: src/commands_deepseek.rs
Issue: #90
Origin: planner

Evidence:
- `yyds deepseek cache-report` returns "no DeepSeek cache metrics recorded from agent chat completions" because yoagent::Usage drops cache token fields — but doesn't tell users where to track upstream resolution.
- Issue #90 is the tracking issue for this gap, filed as agent-help-wanted, no replies yet. Users who hit this diagnostic have no way to discover the issue.
- Assessment Day 136: "deepseek cache-report correctly explains the yoagent gap but the message could helpfully link to issue #90 so users can track upstream resolution."

Edit Surface:
- src/commands_deepseek.rs (two locations: text error path ~line 2066-2076, JSON payload ~line 1930-1944)

Verifier:
- cargo build && cargo test --bin yyds
- `yyds deepseek cache-report` output text includes "https://github.com/yologdev/yyds-harness/issues/90"
- `yyds deepseek cache-report --json` JSON payload includes issue_url field or note

Fallback:
- If the cache-report error message has already been updated to reference issue #90, mark task obsolete.

Objective:
Add a tracking reference to issue #90 in the `yyds deepseek cache-report` output so users who encounter the "no cache metrics" diagnostic know where the upstream fix is tracked.

Why this matters:
The cache-metric gap (issue #90) is the most concrete competitive deficiency: without cache hit/miss metrics, yyds cannot prove its deterministic prompt layout work saves money. Users hitting the diagnostic should be able to discover the tracking issue and optionally subscribe for updates.

Success Criteria:
- `yyds deepseek cache-report` (text mode, no metrics case) includes a line referencing issue #90
- `yyds deepseek cache-report --json` (JSON mode, no metrics case) includes an `issue_url` or `tracking_issue` field
- Both modes still pass `cargo build && cargo test --bin yyds`

Verification:
- cargo build --release
- cargo test --bin yyds
- cargo run -- deepseek cache-report 2>&1 | grep -q "issues/90" && echo "PASS" || echo "FAIL: issue link missing"
- cargo run -- deepseek cache-report --json 2>&1 | grep -q "issues/90" && echo "PASS" || echo "FAIL: issue link missing in JSON"

Expected Evidence:
- Task lineage shows src/commands_deepseek.rs edit
- `yyds deepseek cache-report` shows issue #90 reference
- Build and tests pass

Implementation Notes:
- Text mode (~line 2066-2076): add a line like "  Track this: https://github.com/yologdev/yyds-harness/issues/90" after the "Next step:" line
- JSON mode (~line 1930-1944): add `"tracking_issue": "https://github.com/yologdev/yyds-harness/issues/90"` to the json! payload
- Keep existing text unchanged, just add the reference
- No new dependencies or imports needed
- The edit is a 2-line addition in each mode
