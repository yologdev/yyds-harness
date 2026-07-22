Verdict: PASS
Reason: Implementation already present — lines 993-994 return `_healthy_codebase_fallback()` when `analysis_only_active` is True, and self-test at line 1961 validates this behavior. No new diff needed.
