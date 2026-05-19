Title: Add --model flag to /spawn for configurable sub-agent models
Files: src/commands_spawn.rs, src/agent_builder.rs
Issue: none

## Problem

Claude Code's agent dispatch lets users configure which model sub-agents use. Our `/spawn`
always inherits the parent's model. A developer might want to spawn a cheap model (e.g.
haiku) for simple tasks or a powerful model (e.g. opus) for complex analysis, without
changing their main session model.

## What to do

1. **Add `model` field to `SpawnArgs`** in `commands_spawn.rs`:
   ```rust
   pub struct SpawnArgs {
       pub task: String,
       pub output_path: Option<String>,
       pub background: bool,
       pub collect_id: Option<usize>,
       pub model: Option<String>,  // NEW
   }
   ```

2. **Parse `--model <name>` in `parse_spawn_args`**: Add parsing logic similar to how `-o`
   is parsed. Consume `--model` and the next token as the model name.

3. **Use the model override in `handle_spawn`**: When building the sub-agent, if
   `args.model` is `Some(m)`, create a modified `AgentConfig` with that model instead of
   the parent's. Use it for `build_agent()` and pass it to `run_prompt()`. The provider
   should be auto-detected from the model name (check `providers.rs` for
   `find_provider_for_model` or similar helpers, or use the same provider as the parent).

4. **Same for `handle_spawn_bg`**: Apply the model override in background spawns too.

5. **Update the usage help** in the `/spawn` help text (shown when called with no args)
   to mention `--model`:
   ```
   /spawn --model claude-haiku-3 summarize this file
   ```

6. **Add tests**:
   - Test that `parse_spawn_args` correctly extracts `--model <name>`
   - Test that `--model` works with `--bg` and `-o` combined
   - Test that model is `None` when flag is absent

7. **Update help.rs** if `/spawn` has detailed help text there.

## Why this matters

Agent dispatch configurability is a concrete competitive gap vs Claude Code. This is a
small, self-contained change that gives users meaningful control over sub-agent behavior.
A developer doing a quick lookup doesn't need opus; a developer debugging a subtle issue
might want it.
