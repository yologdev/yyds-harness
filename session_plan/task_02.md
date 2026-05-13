Title: Add tests for prompt.rs — run_prompt_stream_json, StreamEvent, PromptOutcome
Files: src/prompt.rs
Issue: none

`prompt.rs` has 1,713 lines but only ~18 tests (ratio ~95:1), making it one of the
worst-covered core files. The prompt module is the critical path — every interaction
flows through it. Add tests to improve coverage.

## What to test

Focus on the pure/testable functions and types in prompt.rs. Do NOT try to test
functions that require a live Agent — focus on:

1. **StreamEvent enum** — test serialization, display, or any utility methods
2. **StreamUsage struct** — test construction, default values, field access
3. **PromptOutcome** — test construction of different variants, any helper methods
4. **run_prompt_stream_json parsing** — if there are any JSON parsing helpers or
   response extraction logic, test those
5. **Any pure helper functions** — look for functions that take simple inputs and
   return outputs without needing an Agent instance

## Approach

- Read `prompt.rs` to identify all testable pure functions and types
- Write unit tests in the existing `#[cfg(test)] mod tests` block
- Each test should be small and focused — test one behavior
- Target: add 15-25 new tests
- Do NOT mock the Agent or provider — only test code that can be tested in isolation

## What NOT to do

- Don't restructure prompt.rs
- Don't add new public APIs just for testing
- Don't test functions that require network calls or Agent instances
- Don't change any existing tests
