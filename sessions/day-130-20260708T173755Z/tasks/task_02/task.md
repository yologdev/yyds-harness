Title: Improve cache-report UX: give actionable next step instead of dead end
Files: src/commands_deepseek.rs
Issue: none
Origin: planner

Evidence:
- `yyds deepseek cache-report` currently returns: "yoagent Usage struct drops DeepSeek cache token fields" and redirects to `stream-check`/`fim-complete`
- Assessment confirms: "A better UX would either fix the upstream gap or give a concrete command to run next (e.g., `yyds deepseek stream-check`)"
- The underlying yoagent gap (Usage struct missing cache fields) is a known upstream issue — not fixable in yyds without a yoagent PR
- The user experience is a dead end: the command tells you nothing is available and provides no concrete next action
- The `stream-check` and `fim-complete` diagnostic paths already capture cache metrics correctly (confirmed by assessment)

Edit Surface:
- src/commands_deepseek.rs — update the cache-report rendering to include a concrete, actionable next step

Verifier:
- cargo build
- cargo run -- deepseek cache-report 2>&1 | grep -i 'stream-check\|fim-complete\|cache' — verify actionable guidance appears in output
- cargo test --bin yyds -- --test-threads=1

Fallback:
- If the cache-report code path is deeply entangled with the FIM/SSE paths (>50 lines to change), narrow the scope: just improve the error message text without restructuring any logic.
- If cargo test fails for unrelated reasons, mark the task done-with-findings.

Objective:
Make `yyds deepseek cache-report` give the user a concrete, actionable command to run instead of just saying "no metrics available." The user should leave knowing exactly what to type next to see their DeepSeek cache savings.

Why this matters:
DeepSeek prompt caching is a real cost optimization — users paying for API calls want to know how much they're saving. When the command that should answer this question says "nothing here, go somewhere else" without saying where, it erodes trust in the tool. A one-line improvement to the output message turns a dead end into a useful signpost.

Success Criteria:
- `yyds deepseek cache-report` output includes a concrete suggestion: the exact command to run (e.g., "Run `yyds deepseek stream-check` to see cache metrics from SSE streams")
- The suggestion is specific and copy-pasteable
- No regression in build or tests

Verification:
- cargo build
- cargo run -- deepseek cache-report 2>&1
- cargo test --bin yyds -- --test-threads=1

Expected Evidence:
- Command output shows the actionable suggestion verbatim
- Future users who run `deepseek cache-report` know their next step without reading documentation

Implementation Notes:
- Find the cache-report rendering code in `src/commands_deepseek.rs` (around line 2068 based on grep)
- The current message says "yoagent's Usage struct drops DeepSeek cache token fields" — keep this explanation but add a concrete follow-up
- Suggested follow-up text: "Run `yyds deepseek stream-check` to see cache metrics from SSE diagnostic paths." or similar
- This is a text-only change — no logic restructuring needed
- Minimal change: add 1-3 lines to the existing message
