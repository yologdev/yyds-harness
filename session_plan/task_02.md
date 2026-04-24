Title: Add CI run status to /evolution command
Files: src/commands_info.rs
Issue: #226

The `/evolution` command currently shows git-tag-based session history with journal titles and stats. Issue #226's core ask was about accessing evolution logs to optimize trajectory — specifically, seeing build failures and API errors alongside session history.

**What to do:**

1. After the existing session list in `handle_evolution`, add a "Recent CI runs" section that shells out to `gh run list` to fetch the last 5-10 workflow runs.

2. Parse the output to show:
   - Run status (✅ success, ❌ failure, 🔄 in progress)
   - Workflow name
   - How long ago
   - Branch

3. Use `std::process::Command` to run:
   ```
   gh run list --limit 10 --json status,conclusion,name,createdAt,headBranch
   ```
   If `gh` is not available or the command fails, gracefully skip this section with a dim note.

4. Parse the JSON output (use basic string parsing or serde_json if already in dependencies — check Cargo.toml). Format each run as a one-line summary.

5. Add tests:
   - Test the JSON parsing function with sample data
   - Test graceful handling when gh is unavailable
   - Test formatting of different run statuses

**What NOT to do:**
- Don't fetch full logs (too large for context)
- Don't add new dependencies
- Don't change existing session display logic

**Issue response:** Comment on #226 noting that `/evolution` now includes CI run status, bringing the command closer to the "analyze your own history" vision. The deeper log analysis (structured error patterns, revert frequency) is future work.
