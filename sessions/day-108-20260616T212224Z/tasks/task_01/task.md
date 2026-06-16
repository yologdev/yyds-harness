Title: Fix `state summary` command dispatch
Files: src/commands_state.rs
Issue: none
Origin: planner

Evidence:
- Assessment self-test: "`state summary` output is help text instead of summary, suggesting command dispatch misroute"
- Source confirmation: `handle_state_subcommand` (line 27-106) has no `"summary"` match arm. The function `handle_state_summary` exists at line 1108 and is used by the `"why"` arm when no explicit ID is given, but `state summary` falls through to `_ => print_usage()`.
- `state tail`, `state why`, and other subcommands work — the dispatch infrastructure is sound, only the "summary" arm is missing.

Edit Surface:
- src/commands_state.rs

Verifier:
- cargo build && cargo check && cargo test --bin yyds -- commands_state --test-threads=1

Fallback:
- If `handle_state_summary` signature or behavior has changed since the assessment was written, scope the fix to what actually matches the current source. If `state summary` already works (unlikely given the missing match arm), mark obsolete.

Objective:
Make `yoyo state summary` print a structured state summary instead of the help/usage text.

Why this matters:
`state summary` is a primary diagnostic entry point — the first thing someone types to understand their state. When it prints help text instead of a summary, it erodes trust in the entire state command family. This is a one-line dispatch fix: the summary function already exists and works (it's used by `state why` without arguments), it just needs a direct entry point.

Success Criteria:
- `cargo run -- state summary` prints a structured state summary (not help/usage text)
- `cargo run -- state why` behavior is unchanged (still prints summary when no ID given)
- All existing state tests pass

Verification:
- cargo build
- cargo test --bin yyds -- commands_state --test-threads=1
- Manual: `cargo run -- state summary` produces non-help output

Expected Evidence:
- Task lineage links src/commands_state.rs change to this task
- `state summary` returns structured output in future self-tests

Implementation:
1. In `handle_state_subcommand` (src/commands_state.rs, line ~27-106), add a "summary" match arm before the `_ => print_usage()` default.
2. The arm should call `handle_state_summary` with appropriate arguments. The function signature is `fn handle_state_summary(args: &[String], limit: usize)` at line 1108.
3. Parse `--limit` from `args` the same way `handle_failures` does (using `flag_value`), default to 200 (matching `handle_state_summary`'s current call site in the "why" arm).
4. The arm should look like:
   ```
   "summary" => {
       let limit = flag_value(&args[3..], "--limit")
           .and_then(|raw| raw.parse::<usize>().ok())
           .unwrap_or(200);
       handle_state_summary(&args[3..], limit);
   }
   ```
