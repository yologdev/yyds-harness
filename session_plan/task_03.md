Title: Add tests for prompt caching config and notification threshold
Files: src/agent_builder.rs, src/format/mod.rs
Issue: none

The Day 71 morning session added two features without tests:
1. Prompt caching via CacheConfig in agent_builder.rs
2. Desktop notifications for completions >10s in format/mod.rs

Add unit tests to verify these features work correctly.

For prompt caching (in src/agent_builder.rs):
- Add a test that verifies `create_model_config` produces a `ModelConfig` with caching enabled when the appropriate conditions are met. Since `CacheConfig` is a yoagent type, test that the builder path sets it (may need to check the config struct fields or test via the public API).
- If direct testing of CacheConfig isn't feasible (private fields), add a test that verifies `build_agent` doesn't panic when cache config is applied — a smoke test that the wiring is correct.

For notifications (in src/format/mod.rs):
- Add a test for `build_notification_message` — verify it produces a reasonable message string given model name and duration inputs.
- Add a test for the notification threshold logic: verify that durations <10s don't trigger notifications and durations >=10s do (test the decision function, not the actual system call).
- Add a test for `send_desktop_notification` platform detection logic — verify the correct command is selected per OS without actually executing it (if the logic is structured to allow this; if not, restructure the notification sending to separate "which command" from "execute command" to make it testable).

Guidelines:
- Don't test external system calls (no actual `osascript` or `notify-send` execution)
- Test the logic/decisions, not the side effects
- Each test should be `#[test]` (not async) where possible
- Follow existing test patterns in those files (look at existing `#[cfg(test)]` modules)
