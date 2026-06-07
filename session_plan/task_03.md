Title: Build context indexes on first use and add timeout guard to context explain
Files: src/context.rs, src/commands_context.rs
Issue: none

## Goal
The semantic and embedding context indexes are both missing (shown in context_index_status). The `context explain` command has timed out before (trajectory evidence: "context explain timed out after 30s"). Fix both: auto-build indexes when missing, and add a timeout guard to prevent hangs.

## What to do

### Step 1: Auto-build indexes when accessed
When `yyds context preview` or `yyds context explain` is called and either index file is missing:
- Build the missing index(es) automatically before proceeding
- This should be a one-time cost per clone/checkout
- The index paths are: `.yoyo/context-semantic-index.json` and `.yoyo/context-embedding-index.json`

Look at how `build_and_maybe_write_semantic_index` and related functions in `context.rs` work. The key change: instead of silently reporting indexes as missing, trigger their construction when they're first needed.

### Step 2: Add timeout guard to context explain
Add a 30-second timeout to the `context explain` operation. If it exceeds 30s:
- Log a warning: "context explain timed out after 30s — returning partial results"
- Return whatever was computed so far (partial is better than nothing)
- Don't panic or crash

Implementation approach: use `tokio::time::timeout` around the expensive parts of context explain (file listing, index building, token estimation). If the timeout fires, return a partial result with a warning.

### Step 3: Add timeout guard to context index building
Similarly, add a 60-second timeout to index building. If it times out:
- Log a warning
- Return without writing the index (better to have no index than a partial one)
- Don't block the main flow

## Verification
- `cargo build && cargo test` must pass
- `yyds context preview` should work without error (and build indexes if missing)
- `yyds context index --write` should build and write indexes
- `yyds context explain` should complete within a reasonable time (or timeout gracefully)
- After indexes are built, `yyds context preview` should show non-empty index status

## Notes
- Focus on reliability, not feature completeness. The goal is: indexes get built, explain doesn't hang.
- The `build_deepseek_context_preview` and `render_deepseek_native_context_for_prompt` functions in context.rs are the main entry points. The index building happens inside these or is called by them.
- Check if there's already a `build_and_maybe_write_semantic_index` function (referenced in the repo map) — the fix may just be ensuring it's called when indexes are missing.
- Don't add new CLI flags or change the public API. This is an internal reliability improvement.
