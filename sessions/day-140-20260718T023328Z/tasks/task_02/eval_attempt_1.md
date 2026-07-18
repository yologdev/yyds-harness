Verdict: FAIL
Reason: Implementation adds AgentExitReason variant and emit logic correctly, but the task's explicit success criterion "An existing or new test in src/prompt.rs or src/state.rs verifies that an AgentExitReason event is recorded when handle_prompt_events completes" is unmet — no test exists for this feature anywhere in the codebase.
