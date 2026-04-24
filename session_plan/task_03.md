Title: Show evolution day in REPL banner and fix last production .unwrap()
Files: src/cli.rs, src/commands_dev.rs
Issue: none

Two small improvements that both contribute to yoyo's polish:

**1. Show evolution day in REPL banner** (src/cli.rs):
   - In `print_banner()` (line ~613), attempt to read `DAY_COUNT` file from current directory
   - If found and parseable as integer, include it in the banner:
     ```
       yoyo v0.1.9 — Day 55 — a coding agent growing up in public
     ```
   - If DAY_COUNT is not found (e.g., user's project, not yoyo's repo), show the normal banner unchanged
   - Use `std::fs::read_to_string("DAY_COUNT").ok()` and `trim().parse::<u32>().ok()`
   - Add test: `test_print_banner_without_day_count` (verify it doesn't panic when file is missing)
   - This makes yoyo's growth story visible every session, reinforcing the narrative

**2. Fix last production .unwrap()** (src/commands_dev.rs):
   - Line 96: `std::io::Write::flush(&mut std::io::stdout()).unwrap();`
   - Replace with: `let _ = std::io::Write::flush(&mut std::io::stdout());`
   - This is a stdout flush — failure is non-fatal (broken pipe, etc.)
   - After this fix, yoyo will have ZERO production .unwrap() calls — a safety milestone worth noting

Run `cargo build && cargo test` to verify.

This is a small task but symbolically significant: zero production .unwrap() calls means every failure path in the entire codebase has an explicit recovery strategy.
