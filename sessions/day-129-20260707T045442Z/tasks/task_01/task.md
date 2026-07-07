Title: Fix stale `--bin yoyo` references in eval fixture runner
Files: src/eval_fixtures.rs
Issue: none
Origin: planner

Evidence:
- 275 fixture JSON files under `eval/fixtures/local-smoke/` contain commands using `cargo run --bin yoyo` (confirmed by `grep -rl 'cargo.*--bin yoyo' eval/fixtures/local-smoke/*.json | wc -l` → 275).
- The binary was renamed to `yyds` (confirmed: `yyds --version` → `yyds v0.1.14`; `cargo test --bin yyds` passes).
- The `run_fixture_command` function in `src/eval_fixtures.rs` (line 637) passes fixture command strings directly to `/bin/sh -lc` without any rewrite, so `cargo run --bin yoyo` fails with "no bin target named 'yoyo'".
- Assessment confirms: `yyds eval fixtures score --sample 3` hits failures from stale binary references.
- All 275 fixture files cannot be individually edited (exceeds 3-file task limit), but a single-line auto-rewrite in the fixture runner fixes all of them.

Edit Surface:
- src/eval_fixtures.rs — in `run_fixture_command` (line ~638), add a `.replace("--bin yoyo", "--bin yyds")` on the `command` parameter before it's passed to `Command::new("/bin/sh")`. This is a single-line change to the command string processing.

Verifier:
```
cargo build && cargo test --bin yyds -- --test-threads=1
cargo run --quiet --bin yyds -- eval fixtures validate --suite local-smoke --sample 5 2>&1 | head -20
```
The validate command should show all sampled fixtures passing (no "no bin target named 'yoyo'" errors).

Fallback:
- If the `.replace` approach causes false-positive rewrites in fixture command strings that legitimately contain the literal text `--bin yoyo` in a non-cargo context (e.g., echo messages, comments in command strings), narrow to a regex that only rewrites when preceded by `cargo` on the same line: `re.sub(r'cargo\s+(run|test|build)\s+--bin\s+yoyo\b', r'cargo \1 --bin yyds', command)`. If even that proves fragile, fall back to fixing only the eval scoring path that invokes `cargo test --bin yoyo` directly (but identify the caller first).

Objective:
Eliminate the stale `--bin yoyo` reference class from eval fixture execution so fixture scoring reflects real harness health, not binary-rename artifacts.

Why this matters:
When 275 fixtures contain dead-on-arrival commands, eval fixture scoring loses signal. A passing fixture suite means the harness works; a failing suite inflated by binary-rename artifacts means the eval infrastructure is lying. The trajectory and assessment both report this as a MEDIUM bug because it undermines the primary quality gate for harness evolution. Fixing it in the runner (one line, one file) is cheaper and safer than touching 275 fixture files individually.

Success Criteria:
- `cargo run --bin yyds -- eval fixtures validate --suite local-smoke --sample 20` reports 20/20 passing (or at minimum, no failures due to "no bin target named 'yoyo'").
- No existing tests break.
- The `.replace` is applied only inside `run_fixture_command` so it's scoped to fixture execution, not to the fixture file format or loading.

Verification:
```
# Build and unit tests
cargo build && cargo test --bin yyds -- --test-threads=1

# Smoke: sample evaluation
cargo run --quiet --bin yyds -- eval fixtures validate --suite local-smoke --sample 10 2>&1 | grep -c 'no bin target'

# Full suite (longer but definitive)
cargo run --quiet --bin yyds -- eval fixtures score --sample 5 2>&1 | tail -20
```

Expected Evidence:
- Zero "no bin target named 'yoyo'" errors in eval fixture output.
- The fixture score report shows actual harness health (pass/fail by fixture) rather than binary-rename noise.
- Task lineage shows `src/eval_fixtures.rs` changed with a single-line command rewrite in `run_fixture_command`.

Implementation Notes:
- The rewrite happens early in `run_fixture_command`, before the command is passed to `Command::new("/bin/sh")`.
- Use `command.replace("--bin yoyo", "--bin yyds")` — simple, unambiguous, catches both `cargo run --bin yoyo` and `cargo test --bin yoyo`.
- No fixture files are modified. The rewrite is transparent to fixture authors.
- The rewrite string `--bin yoyo` is unlikely to appear in fixture commands for any purpose other than cargo invocations, making false positives negligible.
