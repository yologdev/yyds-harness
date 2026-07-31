Verdict: PASS
Reason: The diff adds a guard that skips ModelCallCompleted events with event_id prefix "evt-harness-" or payload retroactive:true, exactly as specified. Normal agent events flow through to the existing counting logic unchanged. Import and build/tests pass.
