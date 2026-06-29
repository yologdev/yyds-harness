Title: Add eval fixture scoring command to measure harness fitness
Files: src/commands_eval.rs, src/eval_fixtures.rs
Issue: #37
Origin: planner

Evidence:
- Trajectory Day 121: `fitness_score=unknown` — the harness cannot measure its own coding-agent fitness
- `./target/debug/yyds eval fixtures validate --suite local-smoke` reports 370 valid fixtures but no aggregate score
- `./target/debug/yyds state evals --limit 5` shows only log-feedback evals (score=0.6-0.9 range), no coding eval fixture scores
- 370 eval fixtures exist under `eval/fixtures/local-smoke/` covering DeepSeek-specific behaviors (FIM routing #112, transport errors #036, prompt layout #369, cache #004, state coverage) but none are scored during evolution
- Assessment: "fitness gnomes like coding_log_score, retry_success_rate, task_success_rate lack held-out eval baselines" — the baselines exist but aren't computed

Edit Surface:
- src/commands_eval.rs — add "score" subcommand arm in handle_fixtures(), add handle_fixture_score() function
- src/eval_fixtures.rs — add score_fixture_suite() function that runs all fixture tasks, aggregates pass/fail counts by category/risk, and returns a summary struct

Verifier:
- cargo build && cargo test
- ./target/debug/yyds eval fixtures score --suite local-smoke --json 2>&1 | python3 -c "import sys,json; d=json.load(sys.stdin); assert d['total'] > 0; assert 'score' in d; print(f'OK: {d[\"passed\"]}/{d[\"total\"]} passed, score={d[\"score\"]}')"

Fallback:
- If the --json output from existing fixture run infrastructure doesn't parse as expected, fall back to human-readable summary output without JSON
- If running all 370 fixtures takes >5 minutes, implement a --sample N flag that runs a random subset and computes a sampled score
- If any fixture's test command hangs, add a per-test timeout (default 30s)

Objective:
Add a `yyds eval fixtures score` subcommand that runs all (or a sampled subset of) local-smoke fixture tests, computes an aggregate pass/fail score with per-category breakdown, and outputs the result as JSON. This converts the existing 370 eval fixtures from an unused artifact into a measurable fitness signal, directly addressing the `fitness_score=unknown` gap in the trajectory.

Why this matters:
The trajectory reports `fitness_score=unknown` because the evolution pipeline only runs log-feedback evaluations (checking session log quality), never coding eval fixtures. While we cannot modify scripts/evolve.sh (protected file), we can make fixture scoring observable via CLI. This lets any session (or human operator) run `yyds eval fixtures score` to see how well the harness performs against held-out coding tests.

This directly ties to the DeepSeek harness evolution policy: "Capability fitness is the goal. Prefer tasks that raise yyds/DeepSeek coding-agent fitness gnomes such as task_success_rate, task_verification_rate, coding_log_score..."

Success Criteria:
- `yyds eval fixtures score --suite local-smoke` runs all fixture tests and prints a summary
- `yyds eval fixtures score --suite local-smoke --json` outputs a JSON object with: total, passed, failed, score (passed/total), categories (map of category→{passed, failed, total}), risk_levels (map of risk→{passed, failed, total})
- At least one new unit test in src/eval_fixtures.rs verifies the scoring function with a small synthetic suite
- The score command completes (with --sample 20) in under 30 seconds for fast iteration

Verification:
- cargo build && cargo test
- cargo test eval_fixtures::tests -- --nocapture
- ./target/debug/yyds eval fixtures score --suite local-smoke --sample 20
- ./target/debug/yyds eval fixtures score --suite local-smoke --sample 20 --json | python3 -m json.tool

Expected Evidence:
- Task lineage records changes to src/commands_eval.rs and src/eval_fixtures.rs
- After implementation, running `yyds eval fixtures score --suite local-smoke --sample 50` produces a score >= 0.7 (most fixtures should pass since the codebase is healthy)
- Patches that break DeepSeek-specific behavior will cause fixture score to drop, making regression detection possible

Implementation notes:

1. **In `src/eval_fixtures.rs`**, add a public function:
```rust
pub fn score_fixture_suite(suite: &FixtureSuite, sample: Option<usize>) -> FixtureScore
```
where `FixtureScore` is a new struct with fields: total, passed, failed, score (f64), categories (BTreeMap<String, CategoryScore>), risk_levels (BTreeMap<String, CategoryScore>). Use `run_fixture_task()` for each task in the suite (or a random sample if `sample` is set).

2. **In `src/commands_eval.rs`**, add a "score" arm to the match in `handle_fixtures()`:
```
"score" => handle_fixture_score(args, suite),
```
The handler should:
- Parse `--sample N` flag (default: run all)
- Parse `--domain D` filter flag
- Parse `--json` output flag
- Call `crate::eval_fixtures::score_fixture_suite(&suite, sample)`
- Print a human-readable summary by default, structured JSON with `--json`

3. **Sampling**: When `--sample N` is set, use a deterministic seed (hash of suite name) to select N tasks. This makes scores comparable across runs. Use `std::collections::hash_map::DefaultHasher` for the seed.

4. **Timeout**: Each `run_fixture_task` call runs cargo test commands. These should be fast (individual test functions), but add a comment noting that future iterations could add per-command timeouts via `std::process::Command` timeout wrappers.

5. **Tests**: Add a test in `src/eval_fixtures.rs` that creates a 2-task synthetic suite (one always-pass, one always-fail), scores it, and asserts score == 0.5, categories and risk_levels are populated.
