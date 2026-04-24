Title: Extract try_dispatch_subcommand from cli.rs into src/dispatch.rs
Files: src/cli.rs, src/dispatch.rs, src/main.rs
Issue: none

cli.rs is 4,132 lines — still the largest file. Of that, 1,762 lines are non-test code and 2,370 are tests. The `try_dispatch_subcommand` function (lines ~910-1226, ~316 lines of implementation) plus its ~40+ test functions (~700+ lines) is a cleanly separable concern: it dispatches CLI subcommands (`yoyo help`, `yoyo lint`, `yoyo diff`, etc.) before the REPL starts. It imports from other command modules but doesn't depend on the config parsing or system prompt logic in cli.rs.

Steps:
1. Create `src/dispatch.rs` with the `try_dispatch_subcommand` function, the `quote_args_as_command` helper (line 884-908), and the `FlagValueCheck` enum + `require_flag_value` + `flag_value` helpers (used by dispatch).
2. Move all `test_try_dispatch_subcommand_*` tests into `dispatch.rs`.
3. In `cli.rs`, remove the moved code and add `pub(crate) mod dispatch;` or just `mod dispatch;` in `main.rs`.
4. Update `cli.rs::parse_args` to call `dispatch::try_dispatch_subcommand` instead of `self::try_dispatch_subcommand`.
5. Make sure all imports are correct — `try_dispatch_subcommand` uses functions from `commands_dev`, `commands_git`, `commands_info`, `commands_file`, `commands_project`, `commands_config`, `commands_search`, `commands_map`, `setup`, `help`, `format::mod`, etc.
6. Run `cargo build && cargo test` to verify.
7. Update CLAUDE.md's Architecture section to add `dispatch.rs` and update the `cli.rs` description.

Expected reduction: cli.rs drops by ~1,000+ lines (implementation + tests). This is the last major structural extraction needed for cli.rs.
