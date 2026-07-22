Title: Add unit tests for redaction and sensitive-key detection in src/state.rs
Files: src/state.rs
Issue: none
Origin: planner

Evidence:
- Trajectory: task_success_rate=0.0 — no Rust code changes have landed in the current window
- Trajectory: task_verification_rate=0.0 — tasks aren't producing verified evidence
- Assessment: last Rust code change was Day 142 (3 sessions ago); core agent behavior unchanged since Day 141
- `is_sensitive_key()` (line 776) and `redact_state_payload()` (line 752) and `redact_state_string()` (line 812) have zero existing unit tests — grep for `fn test.*redact\|fn test.*sensitive` returns nothing
- These are security-critical functions: they redact API keys, bearer tokens, passwords, and raw reasoning from state payloads before they're written to disk
- `run_completed_payload()` (line 595) also has zero tests — a pure function that constructs a standard JSON shape

Edit Surface:
- src/state.rs

Verifier:
- cargo test state -- --test-threads=1

Fallback:
- If tests for these functions already exist (added since assessment), mark this task obsolete.
- If no functions in the redact module are `pub` or testable from the test module, pick another small pure function with zero coverage (check `run_completed_payload`, `cache_metrics_payload`, `split_secret_assignment`).

Objective:
Add focused unit tests for `is_sensitive_key()` covering at minimum: `api_key`, `Authorization`, `password`, `accessKey`, `secret_key`, `bearer_token`, and a non-sensitive control like `model_name` or `run_id`. Also add a test for `redact_state_payload()` with a JSON object containing a sensitive key, verifying it's replaced with `"[redacted]"`.

Why this matters:
These functions protect API keys and credentials from being written into state event files. Zero test coverage on security-critical redaction means a regression (e.g., a renamed key no longer matching) would go undetected until credentials leak into the state archive. Adding tests makes the security property verifiable across future changes.

Success Criteria:
- At least 3 new `#[test]` functions added to the existing `#[cfg(test)] mod tests` block in src/state.rs
- `cargo test state -- --test-threads=1` passes with all new and existing tests
- Tests cover: `is_sensitive_key` with 5+ sensitive keys and 2+ non-sensitive keys; `redact_state_payload` with a multi-key object containing at least one sensitive key

Verification:
- cargo test state -- --test-threads=1
- cargo build

Expected Evidence:
- Test count in src/state.rs increases by 3+
- All tests pass and no existing tests break
- Future changes to `is_sensitive_key` or `redact_state_payload` have regression protection

Implementation Notes:
- Add tests inside the existing `#[cfg(test)] mod tests { ... }` block at the bottom of src/state.rs
- `is_sensitive_key` is `fn` (not `pub fn`) — it's accessible from within the same module, so `mod tests` can call it directly
- `redact_state_payload` is `pub fn` — callable from tests
- Test `is_sensitive_key` with known-sensitive keys: "api_key", "Authorization", "password", "accessKey", "secret_key", "bearer_token", "private_key"
- Test `is_sensitive_key` with non-sensitive keys: "model", "run_id", "event_type", "tokens"
- Test `redact_state_payload` with: `json!({"api_key": "sk-secret-123", "model": "deepseek-v4"})` → expect api_key redacted, model preserved
- Keep tests focused — no more than ~60 lines total
