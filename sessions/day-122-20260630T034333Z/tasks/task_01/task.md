Title: Fix yyds eval fixtures score timeout — add default sampling
Files: src/commands_eval.rs, src/eval_fixtures.rs
Issue: none
Origin: planner

Evidence:
- `yyds eval fixtures score` timed out at 30s during Day 122 preflight self-test.
  `yyds eval fixtures list` completes instantly (lists 30 fixtures), so the bottleneck
  is in the scoring loop, not fixture discovery.
- The feature was added Day 121 (`src/commands_eval.rs` +65, `src/eval_fixtures.rs` +139).
  `score_fixture_suite` accepts an `Option<usize>` sample parameter — but the CLI
  handler at `handle_fixture_score` (commands_eval.rs:437) may call it with `None`,
  causing it to score ALL 30 fixtures sequentially, each running bash commands.
- Graph pressure: `bash_tool_error=6` and `tool_error_count=4` — commands timing out
  mirrors the broader pattern of unbounded command execution.

Edit Surface:
- src/commands_eval.rs (handle_fixture_score, ~line 437)
- src/eval_fixtures.rs (score_fixture_suite, ~line 398)

Verifier:
- cargo build && cargo test --lib eval_fixtures::tests
- timeout 15 yyds eval fixtures score

Fallback:
- If the timeout is caused by fixture tasks running bash commands that hang (not volume),
  add a per-fixture timeout guard in `run_fixture_task_in` instead of sampling.
- If `cargo test` reveals the score function was already covered by tests and the
  timeout is a test-environment-only issue, write an obsolete note.

Objective:
Make `yyds eval fixtures score` complete within 15 seconds by default, using
a reasonable default sample size when the user doesn't specify `--sample N`.

Why this matters:
The eval fixture scoring command is a Day 121 feature that doesn't work — it times
out before producing output. This blocks the measurement workflow it was designed
to enable (quickly assessing eval coverage health). The trajectory's graph pressure
reinforces this: "recover failed tool actions before scoring" (tool_error_count=4)
and "bound failing shell commands before retrying" (bash_tool_error=6). A scoring
tool that times out can't help recover anything.

Success Criteria:
- `timeout 15 yyds eval fixtures score` completes and prints a score with category breakdown
- `yyds eval fixtures score --sample 5` still works and respects explicit sample
- `yyds eval fixtures score --sample 0` or `--sample all` scores all fixtures (preserves escape hatch)
- `cargo build && cargo test --lib eval_fixtures::tests` passes with no regressions

Verification:
- cargo build
- cargo test --lib eval_fixtures::tests
- timeout 15 cargo run -- yyds eval fixtures score
- cargo run -- yyds eval fixtures score --sample 3
- cargo run -- yyds eval fixtures score --sample 0

Expected Evidence:
- `yyds eval fixtures score` completes and prints FixtureScore with categories
- Task lineage shows file edits in src/commands_eval.rs and src/eval_fixtures.rs
- Dashboard artifact shows fixture scoring command usable in self-test reports

Implementation:
1. In `src/commands_eval.rs`, `handle_fixture_score`: when the user does not provide
   `--sample N`, default to `sample = Some(5)` — score at most 5 random fixtures.
   When `--sample 0` or `--sample all`, pass `None` to score all fixtures.
2. In `src/eval_fixtures.rs`, `score_fixture_suite`: when `sample` is `Some(n)`,
   randomly select `min(n, suite.tasks.len())` tasks from the suite before scoring.
   Use a deterministic seed (e.g., hash of task names) so repeated runs with the
   same sample size produce consistent results, or document that results vary.
3. Add a usage hint in the output: when sampling is active, print
   "Scored N/N total fixtures (use --sample 0 to score all)".
4. Update `--help` text for `yyds eval fixtures score` to document the default
   sample behavior.
