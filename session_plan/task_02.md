Title: Add /model info <name> subcommand to show model details
Files: src/commands_info.rs, src/dispatch.rs, src/help.rs
Issue: none

## Goal

Users switching models need to know: what does this model cost? What's its context window? Which provider serves it? Currently `/model list` shows names only. Add `/model info <name>` to display pricing, context size, and provider details for any known model.

## Implementation

### 1. Add `handle_model_info` in `src/commands_info.rs`

Create a new public function:

```rust
pub fn handle_model_info(model_name: &str, current_model: &str) {
    // 1. Determine which provider serves this model
    //    Scan KNOWN_PROVIDERS + known_models_for_provider() to find which provider lists it
    //    (or "unknown" if not in any list)
    
    // 2. Get pricing via estimate_cost with synthetic Usage
    //    Create Usage { input: 1_000_000, output: 0, ... } and call estimate_cost
    //    to get input cost. Then { input: 0, output: 1_000_000, ... } for output cost.
    //    This avoids needing to change cost.rs visibility.
    
    // 3. Context window size — add a simple fn model_context_window(model: &str) -> Option<u64>
    //    Anthropic Claude: 200_000
    //    GPT-4.1: 1_048_576, GPT-4o/GPT-5: 1_048_576, GPT-4.1-mini/nano: 1_048_576
    //    Gemini 2.5 Pro/Flash: 1_048_576
    //    DeepSeek: 128_000
    //    Others: None (show "unknown")
    
    // 4. Formatted output:
    //    ─── claude-sonnet-4-20250514 ───
    //    Provider:  anthropic
    //    Context:   200k tokens
    //    Pricing:   $3.00 in / $15.00 out (per MTok)
    //    Default:   ✓ (for anthropic)
    //    Active:    ✓
    //
    //    If model is unknown, show what we can and note "not in known model registry"
}
```

Add helper `fn model_context_window(model: &str) -> Option<u64>` in the same file. Cover major models with a simple if-chain (like model_pricing does in cost.rs). Return `None` for unknown models.

Add helper `fn find_provider_for_model(model: &str) -> Option<&'static str>` that scans all known providers.

### 2. Route in `src/dispatch.rs`

In the existing `/model` match arm (around line 165), add before the `list` check:

```rust
if arg == "info" || arg.starts_with("info ") {
    let model_name = arg.strip_prefix("info").unwrap_or("").trim();
    let target = if model_name.is_empty() {
        &ctx.agent_config.model
    } else {
        model_name
    };
    commands::handle_model_info(target, &ctx.agent_config.model);
    return CommandResult::Continue;
}
```

### 3. Update help in `src/help.rs`

- In `/model` help section, add the `info` subcommand
- In `command_short_description`, ensure `/model` description mentions info
- In any completions lists, add "info" to model subcommands

### 4. Tests in `src/commands_info.rs`

- `test_model_context_window_known_models` — verify returns Some for major models
- `test_model_context_window_unknown` — verify returns None for "totally-unknown-xyz"
- `test_find_provider_for_model` — verify correct provider detection
- `test_handle_model_info_no_panic` — call with known + unknown model names, verify no panic

## Verification

```bash
cargo build && cargo test && cargo clippy --all-targets -- -D warnings
```

Exactly 3 source files modified: `commands_info.rs`, `dispatch.rs`, `help.rs`.
