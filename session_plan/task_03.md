Title: Add /doctor check for RTK availability and enhance RTK status visibility
Files: src/commands_dev.rs, src/tools.rs
Issue: #229

Issue #229 asks about using Rust Token Killer (RTK) for output compression. The
integration is already in place in `tools.rs` (`detect_rtk`, `maybe_prefix_rtk`,
`RTK_SUPPORTED_COMMANDS`) with auto-detection, supported command filtering, and
`--no-rtk` disable flag. But there's no user-visible way to check RTK status except
the one-time "📦 RTK detected" message on first use.

**What to implement:**

1. In `run_doctor_checks()` in `commands_dev.rs`, add a check for RTK availability
   (check #11). Call `crate::tools::detect_rtk()` (it's already `pub`) and
   `crate::tools::is_rtk_disabled()` (also `pub`):
   - If RTK detected and not disabled: Pass, "installed (auto-compressing tool output)"
   - If RTK detected but disabled: Warn, "installed but disabled (--no-rtk flag)"
   - If RTK not detected: info/pass, "not installed (optional — compresses build output)"

   RTK is optional, so missing RTK should be a Pass with info, not a Warn.

2. Add a test for the new doctor check — verify the check list includes an "RTK" entry.

3. After verifying all of this is working, this issue (#229) can be closed because:
   - RTK auto-detection works (`detect_rtk()` checks PATH)
   - RTK auto-prefix works (`maybe_prefix_rtk()` prefixes supported commands)
   - RTK can be disabled (`--no-rtk` flag → `disable_rtk()`)
   - RTK status is now visible via `/doctor`
   - Supported commands list covers the common build/test tools

The issue response should explain what's implemented and close it.
