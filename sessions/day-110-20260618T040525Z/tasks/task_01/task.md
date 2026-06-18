Title: Improve `state graph clusters` discoverability with ID discovery hints
Files: src/commands_state_graph.rs
Issue: none
Origin: planner

Evidence:
- Self-test on Day 110: `yyds state graph clusters` shows the usage line requires `<event-id|patch-id|eval-id|commit>` but does not tell the user how to discover valid IDs. The assessment recorded: "UX friction: help doesn't explain what arguments to pass."
- The current error path (line 466 of src/commands_state_graph.rs) prints a wall of usage text without any guidance on how to find valid event/patch/eval/commit IDs.
- This is a real usability gap: the user must guess or read source to learn that `state tail`, `state graph hotspots`, `state evals`, or `state patches` provide discoverable IDs.

Edit Surface:
- src/commands_state_graph.rs — the `handle_graph_subcommand` function's error path and usage text

Verifier:
- cargo test commands_state_graph
- cargo test -- state

Fallback:
- If the usage/error text already contains ID discovery hints in current HEAD (unlikely per assessment), close this task as already-done.

Objective:
Make `yyds state graph clusters` (and all `state graph *` subcommands) tell users how to discover valid IDs when they don't provide one.

Why this matters:
Every `state graph` subcommand requires an ID but none explain discovery. A first-time user who types `yyds state graph clusters` gets a 40-line usage wall and no hint about what to do next. The fix is a two-line hint: "To find valid IDs, try: state tail | state graph hotspots | state evals | state patches". This is a 5-minute UX improvement that removes friction from every state graph interaction.

Success Criteria:
- Running `yyds state graph clusters` (no args) prints a concise error that includes a hint about how to discover valid IDs
- Running `yyds state graph clusters <valid-id>` still works unchanged
- Running `yyds state graph <valid-id>` (base graph subcommand) still works unchanged
- All existing `state graph` subcommands still accept valid IDs and produce correct output
- No regression in `cargo test` for state or graph modules

Verification:
- cargo build
- cargo test commands_state_graph
- cargo test -- state
- Manual: `cargo run -- state graph clusters` → should suggest ID discovery commands
- Manual: `cargo run -- state graph evt-someid` → should still work

Expected Evidence:
- Assessment self-tests will show `state graph clusters` gives helpful guidance
- Future users won't get stuck on the "what ID do I use" question
- No new test failures or command regressions

Implementation Notes:
- The dispatch for `graph clusters` is at line 69 of `src/commands_state_graph.rs`:
  ```rust
  if args.get(3).map(|arg| arg.as_str()) == Some("clusters") {
  ```
- When no ID follows, it currently falls through to the generic usage error at line 466.
- Add a specific check BEFORE the generic usage: if the user typed `graph clusters` (or any `graph <subcommand>`) without an ID, print a focused error with discovery hints.
- Example hint text: "No ID provided. To find valid IDs, try:\n  yyds state tail --limit 5\n  yyds state graph hotspots\n  yyds state evals\n  yyds state patches"
- Also update the usage text at line 466 to add a trailing hint line like: "Tip: use 'state tail', 'state graph hotspots', or 'state evals' to discover valid IDs."
- Keep the change minimal — this is a help-text improvement, not a refactor.
