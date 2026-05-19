Title: Add /spawn --system flag for custom sub-agent system prompts
Files: src/commands_spawn.rs
Issue: none

## Problem

Continuing to close the agent dispatch configurability gap: users should be able to give
sub-agents a custom persona or instruction set. Claude Code's agent dispatch supports
configurable system prompts per agent. Adding `--system <prompt>` to `/spawn` lets users
shape sub-agent behavior beyond just the task.

## What to do

1. **Add `system_prompt` field to `SpawnArgs`**:
   ```rust
   pub struct SpawnArgs {
       pub task: String,
       pub output_path: Option<String>,
       pub background: bool,
       pub collect_id: Option<usize>,
       pub model: Option<String>,
       pub system_prompt: Option<String>,  // NEW
   }
   ```

2. **Parse `--system <prompt>` in `parse_spawn_args`**: The prompt is the next token after
   `--system`. If it starts with a quote, consume until closing quote to support multi-word
   prompts. Example: `--system "You are a security auditor"`.

3. **Use the system prompt in `handle_spawn`**: When building the sub-agent config, if
   `args.system_prompt` is `Some(s)`, prepend it to the context prompt (before the default
   subagent preamble in `spawn_context_prompt`). Don't replace the context — augment it:
   ```
   [custom system prompt]
   
   [standard spawn context prompt with project context and conversation summary]
   ```
   
   This way the user's instruction takes precedence but the sub-agent still gets
   project context.

4. **Same for `handle_spawn_bg`**: Apply the system prompt override in background spawns.

5. **Update usage help** for `/spawn`:
   ```
   /spawn --system "You are a security reviewer" review src/safety.rs
   ```

6. **Add tests**:
   - Test `parse_spawn_args` extracts `--system "quoted prompt"` correctly
   - Test `--system` with single-word prompt (no quotes needed)
   - Test combination of `--model`, `--system`, `--bg`, `-o`
   - Test system_prompt is None when flag absent

7. **Update help.rs** if `/spawn` has detailed help there.

## Why this matters

Combined with task 2's `--model` flag, this gives users full control over sub-agent
configuration: what model to use, what persona to adopt, where to save output, and whether
to run in background. This matches the configurability of Claude Code's agent dispatch
in a simpler CLI-native way.
