Verdict: PASS
Reason: The implementation adds exactly one `stash_diagnostic_error()` call after the existing `state::record` in `record_deepseek_transport_failure`, with the correct message format including source, model, error class, and safe 200-char truncation. All tests pass (124 deepseek, 266 state, full suite clean).
