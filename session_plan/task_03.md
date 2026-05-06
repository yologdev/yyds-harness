Title: Refresh CLAUDE_CODE_GAP.md with current competitive landscape
Files: CLAUDE_CODE_GAP.md
Issue: none

## Context

CLAUDE_CODE_GAP.md was last verified Day 64. The Day 67 assessment gathered fresh competitive
intelligence that should be incorporated. The document serves as a living reference for
planning — keeping it current helps the planning agent make better decisions.

## What to do

Update CLAUDE_CODE_GAP.md with the following new competitive data:

1. **Update the "Last verified" date** to Day 67 (2026-05-06).

2. **Add/update these competitive entries:**

   - **Cursor Cloud Agents** — Background agents that run on cloud git worktrees while user 
     works locally. This is a deployment-model gap yoyo can't close as a CLI tool, but worth 
     noting. Mark as ❌ with note "different deployment model (cloud vs CLI)".
   
   - **Cursor BugBot** — Automated PR review agent. yoyo has `/review` but it's on-demand, 
     not event-driven. Mark as 🟡.
   
   - **Codex Chronicle** — Persistent cross-session project memory. yoyo has `memory/` 
     system with JSONL archives + synthesized active context. Mark as ✅ (different 
     implementation, same capability).
   
   - **Aider auto-lint-test after every edit** — Aider runs lint+test after each individual 
     file write, not just after the full prompt cycle. yoyo's watch mode runs after the 
     prompt completes. Mark as 🟡 with note about the difference (per-edit vs per-turn).
   
   - **Event-driven triggers / webhooks** (Cursor) — agents triggered by GitHub events 
     (PR opened, issue filed, etc.). yoyo has cron-based evolution but no event-driven 
     hooks. Mark as ❌.
   
   - **Sandboxed execution** (Codex) — Docker/VM-based tool isolation. yoyo runs tools 
     directly. Mark as ❌.

3. **Update the priority queue** at the bottom to reflect current state:
   - Per-edit auto-lint-test (Aider parity) should be noted as a concrete next capability gap
   - Persistent named subagents still relevant
   - Note that MCP and hooks are now table-stakes (all competitors have them) — no longer 
     differentiators, but ✅ means yoyo keeps pace

4. **Keep existing entries that are still accurate.** Don't remove anything that hasn't changed.
   Only add new data and update dates/notes.

5. **Add a "Competitive Notes" section** (or update if it exists) summarizing the key insight:
   the biggest gaps are now deployment-model (cloud agents, IDE integration) rather than 
   feature-level. Feature parity is close; the remaining gaps are architectural.

Verify the file is well-formed markdown after editing.
