Verdict: PASS
Reason: All 12 exit_with_state calls in run_single_prompt and run_piped_mode now have stash_diagnostic_error calls with appropriate keys (api_error, empty_input, checkpoint_triggered, etc.), matching the task spec exactly. Build and tests pass.
