Title: Fix yyds eval fixtures score timeout — add default sampling (Issue #49 redux)
Files: src/commands_eval.rs, src/eval_fixtures.rs
Issue: #49
Origin: planner

Evidence:
- `yyds eval fixtures score` timed out at 30s during Day 122 preflight self-test
  (Assessment §Self-Test Results, line 30). `yyds eval fixtures score --sample 1`
  completes instantly.
- This was Day 122 Task 1 — reverted because the evaluator timed out without a
  verdict (evaluator_unverified_count=1 in trajectory graph pressure).
- The scoring feature was added Day 121 (src/commands_eval.rs +65,
  src/eval_fixtures.rs +139). `score_fixture_suite` already handles `Some(n)`
  correctly; the CLI handler `handle_fixture_score` (commands_eval.rs:437) was
  never given a default sample size.
- Issue #49 body contains a complete implementation plan with success criteria.

Edit Surface:
- src/commands_eval.rs (handle_fixture_score, ~line 437)
- src/eval_fixtures.rs (score_fixture_suite, ~line 398 — sampling logic)

Verifier:
- timeout 15 cargo run -- yyds eval fixtures score
- cargo test --lib eval_fixtures::tests
- cargo test --lib commands_eval

Fallback:
- If `cargo run -- yyds eval fixtures score` still times out after defaulting
  --sample to 5, the bottleneck is per-fixture bash execution, not fixture count.
  Switch to a per-fixture timeout guard in run_fixture_task_in instead of sampling.
- If the score function was already fixed by a concurrent session, verify and
  write an obsolete note.

Objective:
Make `yyds eval fixtures score` complete within 15 seconds by default, using
a default sample of 5 when no `--sample` flag is provided.

Why this matters:
The eval fixture scoring command is a Day 121 feature that doesn't work at
default settings — it times out before producing output. This blocks the
measurement workflow it was designed to enable (quickly assessing eval coverage
health). The trajectory graph pressure directly calls for this: "Raise verified
task success rate" (task_success_rate=0.5), "Bound evaluator checks"
(evaluator_unverified_count=1). Fixing the reverted task is the fastest path
to raising both gnomes.

Success Criteria:
- `timeout 15 yyds eval fixtures score` completes and prints a FixtureScore
  with category breakdown
- `yyds eval fixtures score --sample 5` still works and respects explicit sample
- `yyds eval fixtures score --sample 0` scores all fixtures (preserves escape hatch)
- `cargo build && cargo test --lib eval_fixtures::tests` passes

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
1. In `src/commands_eval.rs`, `handle_fixture_score`: when the user does not
   provide `--sample N`, default to `sample = Some(5)` — score at most 5 random
   fixtures. When `--sample 0`, pass `None` (or `Some(0)`) to score all fixtures.
2. In `src/eval_fixtures.rs`, `score_fixture_suite`: when `sample` is
   `Some(n)` and `n > 0`, randomly select `min(n, suite.tasks.len())` tasks
   from the suite before scoring. Use a deterministic seed (hash of task names)
   so repeated runs produce consistent results, or document that results vary.
3. Add a usage hint in the output: when sampling is active, print
   "Scored N of M total fixtures (use --sample 0 to score all)."
4. Update `--help` text for `yyds eval fixtures score` to document the default
   sample behavior.
