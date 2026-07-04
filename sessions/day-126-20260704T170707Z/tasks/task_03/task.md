Title: Add unit tests for read_events_bounded utility
Files: src/state.rs
Issue: none
Origin: planner

Evidence:
- Assessment flag: "read_events_bounded has no unit tests. The function is simple (32 lines) and the state doctor uses it, but there's no direct test coverage for edge cases (empty file, files smaller than limit, etc.)."
- Day 126 Task 2 added `read_events_bounded()` — a shared bounded reader that six prior tool patches had been copy-pasting individually. The state doctor now uses it.
- Function is in src/state.rs, added recently, no test coverage.
- This is a low-risk, high-confidence task: additive only (new tests, no behavior changes), verifiable with `cargo test`.

Edit Surface:
- src/state.rs (add tests in existing #[cfg(test)] mod, probably near the read_events_bounded definition)

Verifier:
- cargo test state::tests::read_events_bounded -- --nocapture

Fallback:
- If read_events_bounded depends on global state recorder initialization that isn't available in unit tests, test the function's internal logic by extracting the boundary-finding helper (the char-boundary truncation) and testing that separately.
- If test infrastructure for creating temp event files is too complex, test with inline JSONL strings via a small helper that writes to a tempfile.

Objective:
Add unit test coverage for read_events_bounded() covering: empty file, file smaller than limit, file exactly at limit, file larger than limit (truncation), invalid UTF-8 / truncated JSON lines, and the char-boundary safety logic.

Why this matters:
read_events_bounded is now the canonical bounded reader used by the state doctor and other diagnostic commands. It replaced six copy-pasted implementations. Without tests, a regression in this function would silently break state doctor, state why, state crashes, and other diagnostics — all at once. The function's char-boundary truncation logic (finding a safe UTF-8 boundary before truncating) is exactly the kind of subtle correctness that tests catch and code review misses.

Success Criteria:
- At least 3 test cases covering: empty file, small file (under limit), and file exceeding limit (truncation path).
- At least 1 test case verifying that truncation doesn't split a multi-byte UTF-8 character.
- All new tests pass with `cargo test`.
- No changes to production code — tests are additive only.

Verification:
- cargo test state::tests::read_events_bounded -- --nocapture
- cargo test -- --test-threads=1 (full suite, no regressions)

Expected Evidence:
- New passing tests in src/state.rs for read_events_bounded.
- Full test suite green.

Implementation:
1. Read src/state.rs around the read_events_bounded function to understand its signature, the event file format, and existing test patterns in the same file.
2. Add test cases in the existing #[cfg(test)] mod tests block:
   a. empty_file: create an empty temp file, call read_events_bounded with limit=1000, assert 0 events returned.
   b. small_file: create a file with 3 valid JSONL lines (each line is a complete StateEvent JSON), call with limit=10000, assert 3 events returned.
   c. file_exceeds_limit: create a file with 100 lines, call with limit=500 bytes (which should be less than 100 lines), assert fewer than 100 events returned AND the truncation happened at a valid char boundary.
   d. multi_byte_boundary: create a file where the limit falls in the middle of a multi-byte UTF-8 character (e.g., "✓" = 3 bytes), assert the truncation happens at the character boundary before the multi-byte char, not inside it.
3. Use tempfile or std::fs::write to create test files. Follow existing test patterns in src/state.rs for temp file creation.
4. Run `cargo test` to verify.
