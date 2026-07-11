Title: Fix subcommand --help flag to show subcommand-specific help
Files: src/dispatch_sub.rs
Issue: none
Origin: planner
validated_against_assessment: true

Evidence:
- Running `yyds state --help` shows the main CLI help text instead of state
  subcommand-specific help. Same for `yyds state graph --help`, `yyds deepseek
  --help`, and other subcommands.
- The assessment confirms: "Subcommand --help flags (yyds state --help, yyds
  state graph --help) show the main CLI help instead of subcommand-specific
  help. This is a minor UX papercut."
- Root cause is in `src/dispatch_sub.rs` line 49:
  ```rust
  if args.iter().any(|a| a == "--help" || a == "-h") {
      print_help();
      return Some(None);
  }
  ```
  This catches `--help` anywhere in args including after a subcommand
  (e.g. `yyds state --help`), prints main help, and returns — never reaching
  the subcommand routing on line 62.
- The `--version` flag on line 53 has the same issue but is less impactful
  since version output is the same regardless of subcommand.

Edit Surface:
- src/dispatch_sub.rs

Verifier:
- cargo build
- cargo test --bin yyds -- --test-threads=1

Fallback:
- If the subcommand handler (e.g. `handle_state_subcommand`) doesn't have its
  own help output, the fix should still route `--help` to the subcommand
  handler so it can print something better than the main help. A fallback
  fallback: if the subcommand handler ignores `--help`, at minimum print
  "Subcommand 'state': use yyds state --help for usage" instead of the full
  main CLI help.
- If the existing test assertions (line 665, 732, 794) already cover the
  corrected behavior, verify those tests still pass after the change.

Objective:
Make `yyds <subcommand> --help` route `--help` to the subcommand handler
instead of short-circuiting with the main CLI help.

Why this matters:
Users who type `yyds state --help` expect to learn about the state subcommand,
not re-read the main CLI help. This is a basic CLI expectation — every mature
CLI tool routes `--help` contextually. While it's a minor UX friction, fixing
it removes a papercut that every new user hits when exploring subcommands.

Success Criteria:
- `yyds state --help` shows state-specific help (not main CLI help).
- `yyds --help` still shows main CLI help.
- `yyds state` (without --help) still works normally.
- `yyds deepseek --help` shows deepseek-specific help.
- `cargo build && cargo test --bin yyds -- --test-threads=1` passes.
- `cargo fmt --check` is clean.

Verification:
- cargo build
- ./target/debug/yyds state --help 2>&1 | head -5
- ./target/debug/yyds --help 2>&1 | head -5
- cargo test --bin yyds -- --test-threads=1

Expected Evidence:
- The `yyds state --help` output no longer matches `yyds --help` output.
- Existing tests in dispatch_sub.rs that assert `--help` behavior continue to
  pass.
- No regression in subcommand routing for non-help invocations.

Implementation Notes:
- The fix is in `src/dispatch_sub.rs`, function `try_dispatch_subcommand`.
- Current code (lines 48-56):
  ```rust
  if args.iter().any(|a| a == "--help" || a == "-h") {
      print_help();
      return Some(None);
  }
  if args.iter().any(|a| a == "--version" || a == "-V") {
      println!("{}", crate::commands_info::version_line());
      return Some(None);
  }
  ```
- Fix: Move the `--help`/`--version` early-return check to AFTER determining
  whether args[1] is a recognized subcommand. Only short-circuit with main help
  when there's no recognized subcommand in args[1].
- Approach:
  1. Extract the subcommand name from args[1] first (like the existing code on
     line 62 does).
  2. If args[1] is a recognized subcommand, skip the early `--help` check and
     let the match arm handle it (the subcommand handler receives all args
     including --help).
  3. If args[1] is NOT a recognized subcommand, THEN check for top-level
     --help/--version.
- Recognized subcommands are the match arms on lines 63+: "doctor", "health",
  "help", "version", "setup", "state", "deepseek", "eval", "evolve", "tools",
  "complete", "init", "lint", "test", "tree", "map", "outline", "run", "diff",
  "commit", "context", "review", etc.
- The simplest fix: check if args.len() > 1 and args[1] doesn't start with '-'
  before triggering the early --help. If args[1] is a subcommand name, skip the
  early --help and fall through to the match. This is a one-line change to the
  condition.
- The `--version` check on line 53 should get the same treatment.
- The existing test at line 665 tests `yyds --help` → should still pass.
- The existing test at line 732 tests `--help` with other flags → check if it
  needs updating. If it tests `yyds --model foo --help`, that should still show
  main help since args[1] starts with `--`.
- The existing test at line 794 tests `yyds help` → should still pass (routes
  to "help" subcommand which calls print_help()).

Edge cases to consider:
- `yyds --model foo --help`: args[1] is "--model", not a subcommand → show main help. Correct.
- `yyds state --help`: args[1] is "state", a recognized subcommand → skip early help, route to state handler. Correct.
- `yyds --help state`: args[1] is "--help" → this currently shows main help and exits. After fix, should still show main help since "--help" starts with "-". Correct.
- `yyds help state`: args[1] is "help" → routes to "help" subcommand which calls print_help(). Correct.
