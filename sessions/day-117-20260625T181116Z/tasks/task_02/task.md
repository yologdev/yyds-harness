Title: Fix state doctor test discoverability for event scanning limit
Files: src/commands_state.rs
Issue: none
Origin: planner

Evidence:
- Day 117 (00:35) commit fdb4c14 added event scanning limit (20K tail) to `state doctor` to prevent timeout with 50K+ events. The implementation in `handle_doctor` and `read_tail_events` is correct and working.
- `cargo test -- doctor` finds zero tests. The scanning-limit test lives at line 24690 as `read_tail_events_respects_limit` — it tests the internal helper, not the `handle_doctor` entry point, and uses a name unrelated to "doctor."
- Self-test assessment confirmed: `cargo test -- state_doctor` and `cargo test -- doctor` both return zero matching tests.
- This is a test organization gap: the test module at line 14420 (`mod tests`) contains tail-focused tests but no doctor-named tests, even though `handle_doctor` is a public entry point with a specific behavior contract (20K tail limit on large event stores).

Edit Surface:
- src/commands_state.rs — add a doctor-specific test (or rename/duplicate the existing helper test) so that `cargo test -- doctor` finds and runs the scanning-limit verification at the `handle_doctor` level.

Verifier:
- cargo test -- doctor
- Must find and run at least one test that verifies the 20K event tail limit behavior

Fallback:
- If `handle_doctor` already has adequate test coverage discoverable by `cargo test -- doctor`, write an obsolete note. Do not add duplicate tests just for naming.
- If the test module structure makes a doctor-named test impractical (e.g., doctor tests would need different fixtures), document the discoverability gap in a code comment above the existing test and skip.

Objective:
Make the event scanning limit test discoverable by `cargo test -- doctor` so that future sessions (and humans) can verify the doctor command's behavior with the expected test filter.

Why this matters:
When a feature is added (the 20K tail limit) but its test is invisible to the most natural filter name (`cargo test -- doctor`), the feature appears untested. This creates friction during debugging and assessment — the assessment agent spent time searching for the test and concluded it didn't exist. Making tests discoverable by their command/feature name is a small investment that pays off every time someone checks "does this have tests?"

Success Criteria:
- `cargo test -- doctor` finds and runs at least 1 test
- The test verifies that `handle_doctor` (or its event-reading path) respects the 20K event limit
- Existing tests still pass (`cargo test` full suite green)

Verification:
- cargo test -- doctor
- cargo test (full suite)
- Expected: both pass, `-- doctor` finds tests

Expected Evidence:
- Future assessments can verify doctor test coverage with `cargo test -- doctor`
- State doctor test discoverability is no longer listed as a bug/friction item

Implementation Notes:
- The simplest approach: add a thin test in the existing `mod tests` (line ~14420) that calls `handle_doctor` or exercises the doctor code path with a known event count. Name it `doctor_respects_event_scanning_limit` or similar.
- Alternative: rename the existing `read_tail_events_respects_limit` test to include "doctor" in its name, since `handle_doctor` is the only caller of the tail-limiting path. But check if `read_tail_events` is also used by `handle_tail` — if so, keep the helper-level test and add a doctor-level test.
- Do NOT modify the production code (handle_doctor, read_tail_events, etc.) — only add or rename tests.
- Keep the test small: create a temp events file with 25K lines, call the doctor code path, verify it only reads 20K.
