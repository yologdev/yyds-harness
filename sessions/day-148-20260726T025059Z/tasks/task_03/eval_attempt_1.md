Verdict: PASS
Reason: The diff adds a conditional `error_detail` at the call site in `handle_prompt_events` (line 962-966) that fires when both `input` and `output` tokens are zero — exactly matching the task's "call-site approach." Two new tests verify zero-token diagnostic inclusion and non-zero-token regression. Build and `cargo test prompt` both pass.
