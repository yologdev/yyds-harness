Verdict: PASS (obsolete)
Reason: The preseed script already detects assessment_missing.md (line 1689), sets assessment_was_missing=True, and adjusts the fallback task to a narrow single-file scope (lines 877-885). Self-tests pass. No changes needed — the feature already exists.
