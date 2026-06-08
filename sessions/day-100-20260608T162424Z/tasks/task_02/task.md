Title: Add session-start diagnostic event for silent pre-tool-call crash visibility
Files: src/state.rs, src/lib.rs
Issue: none
Origin: planner

Objective:
Add a state event that fires at session startup before the first prompt or tool call, capturing enough context to diagnose the "8 sessions crashed before first tool call" problem reported in today's journal. Currently, sessions that crash before firing a single tool produce only RunStarted and Error markers with no diagnostic content. A SessionStarted event with environment info (API key present, model name, skills loaded, context token count) would make these failures diagnosable.

Why this matters:
The journal reports 8 sessions today crashed before the first tool call — silent red lights. We don't know if it's API auth failures, prompt construction panics, tokio runtime issues, or something else. Adding one state event at session init gives future crash sessions a diagnostic breadcrumb. This directly improves harness reliability observability — a core DeepSeek harness KPI.

Success Criteria:
- A state event is recorded before the first prompt/tool call in every session
- The event includes: timestamp, model name, API key presence (boolean, NOT the key), context token count, skills loaded count
- On next session startup, `yyds state tail --limit 5` shows the SessionStarted event
- If a session crashes before first tool call, the SessionStarted event survives in the state log providing diagnostic context

Verification:
- cargo build && cargo test --lib (verify no regressions)
- Run a quick prompt: DEEPSEEK_API_KEY=... yyds -c "hello" 
- Then: yyds state tail --limit 5 (verify SessionStarted event appears)
- Check that the event payload includes model, api_key_present, context_tokens, skills_count

Expected Evidence:
- state events include a new EventType variant (SessionStarted or similar)
- state tail shows the event at session start
- Future pre-tool-call crashes will have at least one diagnostic event in the state log
- gnome metrics may show reduced "unknown failure" rate over time

Detailed plan:

1. Add a new EventType variant in src/state.rs:
   - `SessionStarted` — fires once per session, before any tool calls
   - Payload fields: model (String), api_key_present (bool), context_tokens (u64), skills_count (usize), binary_version (String)

2. Add a helper function in src/state.rs:
   - `record_session_started(model: &str, api_key_present: bool, context_tokens: u64, skills_count: usize)`
   - Calls the global state recorder's append method
   - Must be infallible — if state recording fails, log to stderr but don't crash

3. Call the helper from src/lib.rs:
   - In `run_cli()`, after config/model setup but before the first prompt/tool call
   - The exact location: after `AgentConfig` is built, after `build_agent()` or equivalent, before `run_prompt()` or REPL start
   - Need to determine api_key_present by checking env var presence (NOT value)
   - Need to determine skills_count from the loaded skill set

4. Ensure the event is recorded even for non-interactive modes:
   - Single-prompt mode (`-c "..."`)
   - Piped input mode
   - REPL mode
   - All should get the SessionStarted event

5. Important safety rules:
   - NEVER log the API key value, only a boolean for presence
   - Use `std::env::var("DEEPSEEK_API_KEY").is_ok()` NOT `std::env::var("DEEPSEEK_API_KEY").unwrap()`
   - The event must not panic if state recording isn't initialized

DO NOT:
- Log any API key values or secrets
- Add the event inside hot loops (only once per session)
- Change the prompt execution path — just add an event before it
- Touch more than 2 source files (state.rs and lib.rs)
