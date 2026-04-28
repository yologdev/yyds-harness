Title: Add /loop command for polling repeat
Files: src/dispatch.rs, src/commands_dev.rs, src/help.rs
Issue: none

## What

Add a `/loop` command that repeats a prompt in a polling loop — the user gives a prompt and a count (or "until success"), and yoyo runs it repeatedly. This is a feature Claude Code ships and is low-cost to implement.

## Behavior

```
/loop 5 run the tests and fix any failures
/loop until-pass run cargo test
/loop 3 check if the server is responding
```

Syntax: `/loop <N|until-pass> <prompt>`
- `N` (integer 1-100): run the prompt exactly N times, stopping early if the agent says "done" or the user sends Ctrl+C
- `until-pass`: run repeatedly until the agent reports success (max 20 iterations as safety cap)

Each iteration:
1. Run the prompt through `run_prompt_auto_retry` (the normal prompt path)
2. Print a separator: `--- loop iteration 2/5 ---`
3. For `until-pass` mode: after each iteration, check if the last tool call was a bash command that exited 0. If so, stop.
4. Between iterations, pause 1 second (gives user time to Ctrl+C)

## Implementation

In `src/commands_dev.rs`:
- Add `fn parse_loop_args(input: &str) -> Option<(LoopMode, String)>` that parses the `/loop` command
- Add `enum LoopMode { Count(usize), UntilPass }` 
- Add `pub async fn handle_loop(input: &str, agent: &mut Agent, agent_config: &AgentConfig, changes: &SessionChanges) -> CommandResult` that runs the loop

In `src/dispatch.rs`:
- Add `/loop` to the dispatch table, routing to `handle_loop`
- It should return `CommandResult::Prompted` after the loop completes

In `src/help.rs`:
- Add `/loop` to the help text in both `help_text()` and `command_help()` 
- Short description: "Repeat a prompt in a polling loop"

In `src/commands.rs`:
- Add `"loop"` to KNOWN_COMMANDS

## Tests

- `parse_loop_args("/loop 5 fix the tests")` → `Some((Count(5), "fix the tests"))`
- `parse_loop_args("/loop until-pass cargo test")` → `Some((UntilPass, "cargo test"))`
- `parse_loop_args("/loop")` → `None` (missing args)
- `parse_loop_args("/loop 0 something")` → `None` (zero not valid)
- `parse_loop_args("/loop 200 something")` → capped at 100

## Notes
- This is a direct competitive gap vs Claude Code's `/loop` command
- Keep it simple — the first version doesn't need fancy output, just iteration separators and a count
- The `until-pass` mode checks the exit code of the last bash tool call in the agent's response
