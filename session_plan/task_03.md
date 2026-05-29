Title: Add per-tool recovery hint tests to prevent future hint-format reverts
Files: src/prompt_retry.rs
Issue: #437

The morning revert exposed a deeper issue: tool_recovery_hint() has 12 match arms (6 tools × 
2 attempt levels) but only a few are tested. When someone changes any hint text, only the
exact tests for that arm would catch it — and those tests assert on exact strings.

Add a comprehensive test that validates ALL arms of tool_recovery_hint() have *some* non-empty
content and contain at least one actionable word. This acts as a regression net.

Specific changes:

1. In `src/prompt_retry.rs` test module, add a new test `test_all_recovery_hints_are_actionable`:
   ```rust
   #[test]
   fn test_all_recovery_hints_are_actionable() {
       let tools = ["bash", "edit_file", "write_file", "read_file", "search", "rename_symbol", "unknown_tool"];
       let action_words = ["try", "use", "check", "verify", "retry", "break", "approach", "alternative", "different"];
       for tool in &tools {
           for attempt in [1, 2] {
               let hint = tool_recovery_hint(tool, attempt);
               assert!(!hint.is_empty(), "{tool} attempt {attempt} should have a hint");
               let has_action = action_words.iter().any(|w| hint.to_lowercase().contains(w));
               assert!(has_action, "{tool} attempt {attempt} hint should contain an actionable word: {hint}");
           }
       }
   }
   ```

2. Also update the existing individual hint tests (`test_tool_recovery_hint_edit_file_attempt1`,
   `test_tool_recovery_hint_edit_file_attempt2`, etc.) to use semantic checks instead of
   exact string matches. For example, instead of:
   ```rust
   assert!(hint.contains("read_file to see"));
   ```
   Use:
   ```rust
   assert!(hint.contains("read_file") || hint.contains("current contents"));
   ```

   Only change the assertions, not the test names or structure.

3. Run `cargo test` to verify all tests pass.

This task complements task_01 — task_01 fixes the two tests that caused the revert,
this task hardens the underlying hint function tests to prevent future reverts from
the same class of change.
