Title: Improve fingerprint clustering in extract_trajectory.py
Files: scripts/extract_trajectory.py
Issue: #345

## What to do

Fix the fingerprint clustering in `scripts/extract_trajectory.py` so that the same error
occurring across N failed CI runs clusters as `[N×]` instead of N individual `[1×]` lines.

This is sub-task #2 from issue #345 (the most impactful of the three — it directly improves
the trajectory data that yoyo sees in every session).

### Current problem

`fingerprint_error_line()` strips timestamps but misses:
1. **Subsecond suffixes** — `.5342991z` after a timestamp defeats clustering for errors
   at different sub-second offsets.
2. **GitHub Actions log prefixes** — the `<workflow> <job> <step> <timestamp>` prefix
   varies per run, so `social unknown step 2026-04-15t15:n:n.5342991z error: auth error`
   and `social unknown step 2026-04-08t07:n:n.8992940z error: api error` don't cluster
   even though they're the same class of error.

### Specific fixes

1. In `fingerprint_error_line()`:
   - Add a regex to strip the GH Actions log prefix pattern: `<word> <word> <word> <timestamp>`.
     The pattern is: one or more space-separated words followed by a timestamp like
     `2026-04-15t15:31:42.5342991z`. Strip the entire prefix up through the timestamp.
   - Ensure the subsecond portion (`.NNNNNNNZ` or `.NZ`) is stripped by the existing
     timestamp regex. The current regex `^\d{4}-\d{2}-\d{2}T?[\d:.,Z+ ]*\s*` should
     handle it, but the GH Actions prefix puts workflow/job/step words BEFORE the timestamp.
   - After all stripping, also normalize hex addresses and UUIDs (replace with `<HEX>`)
     to improve clustering of errors with different memory addresses.

2. Add a simple self-test at the bottom of the script (guarded by `if __name__ == "__main__"`
   and a `--test` flag) that verifies:
   - Two error lines from the trajectory output in the assessment cluster to the same fingerprint
   - The social auth errors from different dates produce the same fingerprint

### Test the fix

Run the script against the real audit-log if available:
```bash
python3 scripts/extract_trajectory.py --test  # self-test
```

### Acceptance criteria
- `python3 scripts/extract_trajectory.py --test` passes (new self-tests)
- The trajectory section's recurring CI errors should show `[N×]` for repeated errors
  instead of N separate `[1×]` lines
- `cargo build && cargo test` still pass (no Rust changes, but verify nothing breaks)
- No changes to Rust source files — Python-only change
