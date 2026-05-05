Title: Extract shared prompt epilogue and usage accumulation helpers in prompt.rs
Files: src/prompt.rs
Issue: none

The assessment identifies prompt.rs duplication as the #1 internal code quality issue.
Two functions — `run_prompt_with_changes` (188 lines) and `run_prompt_with_content_and_changes`
(115 lines) — share nearly identical post-prompt epilogue code (~20 lines each) and the same
usage accumulation pattern (4-line `total_usage.input += usage.input; ...` block appears 6+
times in the file, plus 4 more in the streaming JSON variants).

**Extract two helpers:**

1. `fn accumulate_usage(total: &mut Usage, delta: &Usage)` — replaces the 4-line pattern:
   ```rust
   total_usage.input += usage.input;
   total_usage.output += usage.output;
   total_usage.cache_read += usage.cache_read;
   total_usage.cache_write += usage.cache_write;
   ```
   This pattern appears ~10 times in prompt.rs. Each call site becomes a one-liner.

2. `async fn finish_prompt_epilogue(agent, total_usage, session_total, model, prompt_start) -> (u64, u64)` — replaces the duplicated 15-line block at the end of both `run_prompt_with_changes` and `run_prompt_with_content_and_changes`:
   ```rust
   session_total.input += total_usage.input;
   // ...accumulate session totals...
   print_usage(...);
   agent.finish().await;
   let ctx_used = total_tokens(agent.messages()) as u64;
   let ctx_max = effective_context_tokens();
   print_context_usage(ctx_used, ctx_max);
   // ...context warning...
   maybe_ring_bell(...);
   println!();
   ```

**What NOT to change:**
- Don't modify the function signatures or behavior of any public `run_prompt*` function
- Don't change the streaming JSON variants' epilogue (they have a different pattern — they
  emit JSON events instead of printing). Just apply `accumulate_usage` to their 4-line blocks.
- Don't touch the retry loop structure — that's a separate, larger refactor

**Verification:** `cargo build && cargo test && cargo clippy --all-targets -- -D warnings`

This should eliminate ~80 lines of pure copy-paste and make the remaining prompt functions
much easier to read and maintain.
