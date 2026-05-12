Title: Add tests for untested functions in prompt_retry.rs — diagnose_api_error, build_retry_prompt, infer_provider_from_model
Files: src/prompt_retry.rs
Issue: none

The core error handling path in `prompt_retry.rs` has three important functions with zero test coverage:

1. **`build_retry_prompt`** (line 14) — builds the retry message when a manual `/retry` is used. Tests should cover:
   - `last_error` is `None` → returns input unchanged
   - `last_error` is `Some(short_error)` → wraps with "[Previous attempt failed: ...]" prefix
   - `last_error` is `Some(very_long_error)` → truncates error to 200 chars with `…` via `safe_truncate`

2. **`diagnose_api_error`** (line 284) — produces human-readable diagnostic messages for API errors. This is what users see when things break. Tests should cover each branch:
   - 401/unauthorized → mentions the correct env var for the provider, distinguishes key-set vs not-set
   - model_not_found → lists available models for the inferred provider
   - connection refused → network error message, special case for ollama ("Is Ollama running?")
   - 403/forbidden → access forbidden message
   - "stream ended" → MiniMax-specific guidance
   - "stream closed" / "unexpected eof" → transient error message
   - Unrecognized error → returns None

   Note: `diagnose_api_error` calls `crate::cli::provider_api_key_env` and `crate::cli::known_models_for_provider`. These are real functions that take a provider string, so they work fine in tests — just use known providers like "anthropic", "openai", "ollama".

3. **`infer_provider_from_model`** (line 389) — maps model names to provider strings. Tests should cover:
   - "claude-*" → "anthropic"
   - "gpt-*" → "openai"
   - "gemini-*" → "google"
   - "llama-*" → "groq" or similar
   - Unknown model → "custom" or whatever the fallback is

Target: 15-20 tests covering all branches of these three functions. All pure/near-pure functions, no async needed, no mocking needed.
