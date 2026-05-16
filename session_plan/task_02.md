Title: Add --no-tools convenience flag
Files: src/cli.rs, src/cli_config.rs, src/help.rs
Issue: none

## Description

Users who want a tool-free session (chat-only, no bash/file access) currently need the unwieldy:
```
--disallowed-tools bash,read_file,write_file,edit_file,list_files,search,rename_symbol,sub_agent,todo
```

Every other coding agent has a simple `--no-tools` flag. Trying `--no-tools` today gives a confusing "Unknown flag" warning. This is a DX gap identified in the assessment.

## Implementation

1. **src/cli_config.rs** — Add `pub no_tools: bool` field to the `Config` struct (default false).

2. **src/cli.rs** — 
   - Add `"--no-tools"` to `KNOWN_FLAGS` array
   - In `parse_args()`, detect `--no-tools` and set `config.no_tools = true`
   - When `no_tools` is true, populate `config.disallowed_tools` with ALL tool names. Use a constant list: `["bash", "read_file", "write_file", "edit_file", "list_files", "search", "rename_symbol", "sub_agent", "todo", "shared_state"]`
   - This means `--no-tools` is syntactic sugar — it just fills `disallowed_tools` with everything, so the existing filtering in `agent_builder.rs` handles the rest with zero changes.

3. **src/help.rs** — Add `--no-tools` to the CLI help text in the flags section with description: "Disable all tools (chat-only mode, no file access or commands)"

## Tests

Add to cli.rs tests:
```rust
#[test]
fn test_no_tools_flag() {
    let args = vec!["yoyo".to_string(), "--no-tools".to_string(), "hello".to_string()];
    let config = parse_args(args).unwrap();
    assert!(config.no_tools);
    assert!(!config.disallowed_tools.is_empty());
    // Should contain all builtin tool names
    assert!(config.disallowed_tools.contains(&"bash".to_string()));
    assert!(config.disallowed_tools.contains(&"read_file".to_string()));
}

#[test]
fn test_no_tools_in_known_flags() {
    assert!(KNOWN_FLAGS.contains(&"--no-tools"));
}
```

## Verification

```bash
cargo build && cargo test
```
