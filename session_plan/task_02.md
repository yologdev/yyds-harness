Title: Extract error diagnosis and retry logic from prompt.rs into prompt_retry.rs
Files: src/prompt_retry.rs (new), src/prompt.rs
Issue: none

## Motivation

`prompt.rs` is 2,425 lines — the 3rd largest source file. It contains a cohesive block of
error diagnosis and retry logic (~340 lines of functions + ~400 lines of tests) that has no
dependency on the prompt execution machinery. Extracting it reduces prompt.rs by ~740 lines
and creates a focused, independently testable module.

## What to extract into `src/prompt_retry.rs`

### Functions (lines ~58–430 of prompt.rs):
- `build_retry_prompt` — builds a retry prompt from input + last error
- `MAX_RETRIES` constant (if it exists and relates to retry)
- `MAX_AUTO_RETRIES` constant
- `build_auto_retry_prompt` — builds auto-retry prompt
- `tool_recovery_hint` — returns tool-specific recovery suggestions
- `is_overflow_error` — detects context overflow errors
- `OVERFLOW_PHRASES` constant
- `build_overflow_retry_prompt` — builds prompt for overflow recovery
- `retry_delay` — exponential backoff with jitter
- `is_retriable_error` — determines if an error is worth retrying
- `diagnose_api_error` — produces human-readable API error diagnostics
- `infer_provider_from_model` — helper used by diagnose_api_error

### Tests to move (from the `mod tests` block):
- `test_retry_delay_exponential_backoff_ranges`
- `test_retry_delay_capped_at_60s`
- `test_retry_delay_zero_attempt_floor`
- `test_is_retriable_rate_limit`
- `test_is_retriable_server_errors`
- `test_is_retriable_network_errors`
- `test_is_not_retriable_auth_errors`
- `test_is_not_retriable_client_errors`
- `test_is_not_retriable_unknown_error`
- `test_is_retriable_stream_errors`
- `test_stream_ended_not_retriable`
- `test_diagnose_stream_ended`
- `test_diagnose_stream_closed`
- `test_diagnose_unexpected_eof`
- `test_diagnose_broken_pipe`
- `test_diagnose_incomplete`

## How to do it

1. Create `src/prompt_retry.rs` with the extracted functions and their tests
2. In `prompt.rs`, replace the extracted code with `pub use crate::prompt_retry::*;` or
   specific imports. Add `mod prompt_retry;` to `main.rs`.
3. Make sure visibility is correct — functions that prompt.rs's remaining code calls
   need to be `pub` or `pub(crate)`.
4. Check that `run_prompt_auto_retry` and `run_prompt_auto_retry_with_content` in prompt.rs
   can still call the extracted functions.

## Verification

- `cargo build` — compiles
- `cargo test` — all 2,385+ tests pass (including the moved ones)
- `cargo clippy --all-targets -- -D warnings` — clean

## Constraints
- Do NOT change any function signatures or behavior — pure extraction
- Do NOT rename anything — keep all public API names identical
- Move tests alongside their functions
