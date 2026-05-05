Title: Add --output-format stream-json for streaming JSON event output
Files: src/cli.rs, src/main.rs, src/prompt.rs
Issue: none

## Goal

Close the #1 competitive gap vs Claude Code: headless streaming JSON output for CI/SDK integration. Claude Code has `--output-format stream-json` that emits newline-delimited JSON events in real-time. yoyo currently has `--json` which outputs a single JSON blob at the end. Add a streaming mode.

## What to implement

1. **In `src/cli.rs`**: Add an `OutputFormat` enum (`Text`, `Json`, `StreamJson`) and a `output_format` field to `Config`. Parse `--output-format stream-json` and `--output-format json` flags (keep `--json` as a shorthand for `--output-format json`). Add `--output-format` to KNOWN_FLAGS.

2. **In `src/main.rs`**: When `output_format == StreamJson` in `run_single_prompt` or `run_piped_mode`:
   - Instead of collecting and printing text, emit NDJSON events to stdout as they arrive:
     - `{"type":"message_start","model":"..."}` at prompt start
     - `{"type":"content_delta","text":"..."}` for each text chunk
     - `{"type":"tool_use","name":"...","input":{...}}` when a tool is called
     - `{"type":"tool_result","name":"...","output":"..."}` when a tool returns
     - `{"type":"message_end","usage":{"input_tokens":N,"output_tokens":N},"cost_usd":F}` at the end
   - Suppress all stderr formatting (spinners, progress) when stream-json is active.

3. **In `src/prompt.rs`**: The `run_prompt` function already processes `AgentEvent` variants. Add a parameter or check for output format, and when stream-json is active, emit JSON lines to stdout for each event instead of rendering formatted text. Keep the existing `PromptOutcome` return unchanged.

## Scoping notes

- Only implement for `--prompt` and piped modes (non-interactive). REPL mode does NOT need stream-json.
- Keep `--json` working exactly as before (single blob at end).
- Add tests: at minimum a unit test that verifies the JSON event serialization produces valid NDJSON.
- Update `cli_help_text()` in `src/help.rs` if the `--output-format` flag needs mention (but help.rs is a 4th file — if needed, just add to existing flag list in cli.rs help constant).

## Verification

```bash
cargo build && cargo test
# Manual: echo "hello" | cargo run -- --output-format stream-json
# Should emit NDJSON lines to stdout
```
