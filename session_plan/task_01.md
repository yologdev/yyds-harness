Title: Enable prompt caching via yoagent's CacheConfig
Files: src/agent_builder.rs
Issue: none

## What

yoagent 0.8 has a built-in `CacheConfig` with `CacheStrategy` (Auto, Disabled, Manual) and
a `with_cache_config()` builder method on Agent. yoyo never calls it — meaning prompt caching
is left at whatever the default is, potentially missing significant cost savings.

Anthropic's prompt caching can reduce input token costs by ~90% for cached content (system
prompt, tool definitions, conversation history). This is the "don't reinvent the wheel"
principle from CLAUDE.md — the feature exists in yoagent, we just need to wire it up.

## Implementation

1. In `build_agent()` (around line 453 where `with_context_config` is called), add:
   ```rust
   agent = agent.with_cache_config(yoagent::CacheConfig {
       enabled: true,
       strategy: yoagent::CacheStrategy::Auto,
   });
   ```

2. Also add cache config to `build_side_agent()` and `build_architect_agent()` — all agent
   construction paths should enable caching.

3. Add a `--no-cache` CLI flag (in `cli.rs` parse_args and Config struct) that sets
   `CacheConfig { enabled: false, .. }` for debugging/testing. Store in AgentConfig so
   build_agent can read it. Actually — keep this simple. Just enable caching unconditionally
   for now (CacheStrategy::Auto handles everything). A flag can come later if needed.

4. Add a unit test that verifies the cache config is set on the built agent. Check that
   `agent.cache_config.enabled == true` and `agent.cache_config.strategy == CacheStrategy::Auto`.

## Why

- Prompt caching is listed as a competitive gap in the assessment ("Prompt caching (Aider) — yoyo doesn't have explicit prompt caching. This could meaningfully reduce costs.")
- yoyo already tracks cache_read and cache_write tokens in cost display, but never asks the provider to actually cache
- The cost savings compound over multi-turn conversations — exactly yoyo's primary use case
- This is a 1-file change with immediate measurable impact

## Verification

- `cargo build && cargo test`
- The agent should now send cache control headers to Anthropic (visible in verbose/audit mode)
- Cost display should show cache_read tokens increasing after the first turn in a session
