Title: Add Claude 4.5 Sonnet and Claude 4.6 Haiku to model registry and pricing
Files: src/providers.rs, src/format/cost.rs
Issue: none

## Problem

Anthropic has released Claude Sonnet 4.6 and Claude Haiku 4.6 (or Claude 4.5 Sonnet depending on naming — verify via Anthropic docs). The model registry in `providers.rs` only lists:
- `claude-opus-4-6`
- `claude-sonnet-4-20250514` (Sonnet 4, not 4.6)
- `claude-haiku-4-5-20250414`

Both Aider and Claude Code have already added aliases for the newer Claude model variants. yoyo should too.

## What to do

### Step 1: Research current Anthropic model names

The implementation agent MUST run:
```bash
curl -s https://docs.anthropic.com/en/docs/about-claude/models | head -200
```
or check the Anthropic API models endpoint to find the exact current model IDs for:
- Claude Sonnet 4.5 / 4.6 (whatever the latest Sonnet is)
- Claude Haiku 4.5 / 4.6 (whatever the latest Haiku is)

### Step 2: Update `known_models_for_provider` in `src/providers.rs`

Add any new model IDs to the "anthropic" match arm. For example, if Claude Sonnet 4.6 exists:
```rust
"anthropic" => &[
    "claude-opus-4-6",
    "claude-sonnet-4-6",           // NEW — add if exists
    "claude-sonnet-4-20250514",
    "claude-haiku-4-6",            // NEW — add if exists
    "claude-haiku-4-5-20250414",
],
```

Also update the Bedrock and OpenRouter entries if applicable.

### Step 3: Update pricing in `src/format/cost.rs`

If new model variants have different pricing, add entries. The current `model_pricing` function uses broad matching (`model.contains("opus")`, `model.contains("sonnet")`, `model.contains("haiku")`), so new variants with the same pricing may already be covered. But verify:

- If Claude Sonnet 4.6 has different pricing than Sonnet 4, add a specific check for "4-6" or "4.6" in the sonnet block (similar to how opus already checks for "4-6" vs "4-5")
- Same for Haiku 4.6 vs Haiku 4.5

### Step 4: Update tests

Add or update tests to cover the new model IDs in both `providers.rs` and `cost.rs`.

## Important notes

- Do NOT guess model names. Research first, then implement.
- If no new Claude models have been released since the last update (i.e., the registry is actually current), this task becomes: verify the registry is current and add a comment documenting the verification date. Still a useful task — it eliminates uncertainty.
- The `default_model_for_provider` function should still default to `claude-opus-4-6` for anthropic.

## Verification

- `cargo build && cargo test` must pass
- `cargo test` for any new model-related tests
- Grep `known_models_for_provider` to confirm new entries are present
