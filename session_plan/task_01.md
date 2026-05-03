Title: Update model registry — add GPT-5/5.5, Grok-4, Gemini 2.5 Flash Lite
Files: src/providers.rs, src/commands.rs
Issue: none

## What

The model registry is a full generation behind. Users trying `--model gpt-5` or `--model grok-4` today get no tab-completion and no validation hints. This is the single highest-impact change for making yoyo feel current.

## Changes to `src/providers.rs`

Update `known_models_for_provider()`:

**OpenAI** — add:
- `gpt-5`
- `gpt-5-mini`
- `gpt-5.5`
- `gpt-5.5-mini`

Keep existing models (gpt-4o, gpt-4o-mini, gpt-4.1, gpt-4.1-mini, gpt-4.1-nano, o3, o3-mini, o4-mini).

**xAI** — add:
- `grok-4`

Keep existing (grok-3, grok-3-mini, grok-2).

**Google** — add:
- `gemini-2.5-flash-lite`

Keep existing (gemini-2.5-pro, gemini-2.5-flash, gemini-2.0-flash).

**Anthropic** — update to include the short alias:
- Keep `claude-opus-4-6`, `claude-sonnet-4-20250514`, `claude-haiku-4-5-20250414`

Update `default_model_for_provider()`:
- OpenAI default: change from `gpt-4o` to `gpt-4o` (keep — GPT-5 isn't universally available yet)
- xAI default: keep `grok-3` (grok-4 is new)
- No other defaults change

## Changes to `src/commands.rs`

Update `KNOWN_MODELS` constant to add:
- `gpt-5`
- `gpt-5-mini`
- `gpt-5.5`
- `gpt-5.5-mini`
- `grok-4`
- `gemini-2.5-flash-lite`

This is the tab-completion list used for `/model <Tab>`.

## Tests

Add tests in `src/providers.rs`:
- `test_openai_known_models_includes_gpt5` — verify gpt-5 and gpt-5.5 are in the list
- `test_xai_known_models_includes_grok4` — verify grok-4 is present
- `test_google_known_models_includes_flash_lite` — verify gemini-2.5-flash-lite is present

## Verify

`cargo build && cargo test && cargo clippy --all-targets -- -D warnings`
