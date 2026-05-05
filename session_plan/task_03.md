Title: Introduce ConfigDisplay struct and remove dead code
Files: src/commands_config.rs, src/commands_lint.rs, src/commands_tree.rs
Issue: none

## Goal

1. Remove the `#[allow(clippy::too_many_arguments)]` on `handle_config` (line 201 in commands_config.rs) by bundling its parameters into a struct.
2. Remove or use the dead code flagged by `#[allow(dead_code)]` in `commands_lint.rs` (line 535) and `commands_tree.rs` (line 230).

## What to implement

### Part A: ConfigDisplay struct in commands_config.rs

Define a `ConfigDisplay` struct that holds all the display parameters:

```rust
pub struct ConfigDisplay<'a> {
    pub provider: &'a str,
    pub model: &'a str,
    pub base_url: &'a Option<String>,
    pub thinking: ThinkingLevel,
    pub max_tokens: Option<u32>,
    pub max_turns: Option<usize>,
    pub temperature: Option<f32>,
    pub skills: &'a yoagent::skills::SkillSet,
    pub system_prompt: &'a str,
    pub mcp_count: u32,
    pub openapi_count: u32,
    pub hook_count: usize,
    pub agent: &'a Agent,
    pub cwd: &'a str,
}
```

Rewrite `handle_config` to take `cfg: &ConfigDisplay<'_>` and remove the allow attribute. Update all call sites (likely in `dispatch.rs` or `commands.rs` — check where `handle_config` is called and update the call to construct the struct).

**Important**: If `handle_config` is called from a file not listed above, the implementer should update that call site too (it's fine to touch the caller since it's a one-line struct construction change).

### Part B: Dead code cleanup

1. **`commands_lint.rs` line 535**: Check what the `#[allow(dead_code)]` is on. If it's a function or struct that's genuinely unused, remove it entirely. If it's planned for future use, add a `// TODO: used by future /lint fix-all` comment and keep the allow.

2. **`commands_tree.rs` line 230**: Same — check what it's on, remove if unused, or justify with a comment.

## Verification

```bash
cargo build && cargo clippy --all-targets -- -D warnings && cargo test
```

No behavioral changes — purely structural cleanup and dead code removal.
