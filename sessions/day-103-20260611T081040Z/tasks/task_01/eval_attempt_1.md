Verdict: PASS
Reason: All three error paths (spawn, timeout, wait) are correctly wired with `stash_diagnostic_error` before error propagation, the cancellation path remains untouched as required, and build/tests/fmt all pass.
