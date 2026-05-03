Title: Add `/model list` subcommand to browse available models by provider
Files: src/commands_info.rs, src/dispatch.rs
Issue: none

## What

Currently `/model` shows the active model and `/model <name>` switches to it. But there's no way to discover what models are available without knowing them in advance. With the model registry being updated (Task 1), we need a way for users to browse what's available.

Add `/model list [provider]` that displays known models grouped by provider, with the active model highlighted.

## Changes to `src/dispatch.rs`

In the `/model` command handler section (around line 165), add a check: if the argument after `/model ` is `list` or starts with `list`, route to a new `handle_model_list` function instead of treating it as a model name switch.

```
s if s.starts_with("/model ") => {
    let arg = s.trim_start_matches("/model ").trim();
    if arg == "list" || arg.starts_with("list ") {
        let filter = arg.strip_prefix("list").unwrap_or("").trim();
        commands::handle_model_list(&ctx.agent_config.model, &ctx.agent_config.provider, filter);
        return CommandResult::Continue;
    }
    // ... existing model switch logic
}
```

## Changes to `src/commands_info.rs`

Add `pub fn handle_model_list(current_model: &str, current_provider: &str, filter: &str)`:

1. If `filter` is non-empty and matches a provider name, show only that provider's models
2. Otherwise, iterate through all providers in `KNOWN_PROVIDERS`
3. For each provider, call `known_models_for_provider()` and display models
4. Highlight the active model with a marker (e.g., `▶` or `*`)
5. Show the provider's default model with `(default)` suffix
6. Skip providers with no known models (custom, openrouter)

Output format:
```
  Models by provider (active: claude-sonnet-4-20250514)

  anthropic
    claude-opus-4-6
  ▸ claude-sonnet-4-20250514  (default)
    claude-haiku-4-5-20250414

  openai
    gpt-5
    gpt-5-mini
    gpt-5.5
    gpt-5.5-mini
    gpt-4o  (default)
    ...

  Use: /model <name> to switch
```

## Tests

Add tests in `commands_info.rs`:
- Test that `handle_model_list` doesn't panic with empty filter
- Test that `handle_model_list` doesn't panic with a specific provider filter

## Tab completion

In `src/commands.rs`, update `command_arg_completions` for `/model` to include `list` as a completion option. This means when the user types `/model l<Tab>`, "list" should appear.

Wait — that's a 3rd file. Since the dispatch change is minimal (2-line routing check), let's keep it to `commands_info.rs` and `dispatch.rs` as the primary files. The tab-completion addition in commands.rs is optional — skip it if it would exceed the 3-file limit.

## Verify

`cargo build && cargo test && cargo clippy --all-targets -- -D warnings`
