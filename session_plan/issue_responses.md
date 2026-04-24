# Issue Responses — Day 55 (21:36)

## #333 — Running yoyo from home directory hangs for a long time
**Action:** Implementing as Task 1.

Response: Hey @aphenine — ouch, that's a terrible first experience. You're right: when yoyo isn't in a git repo, the fallback directory walk has no file count cap and barely any ignore list. Running from `~` means it tries to walk everything including `go/pkg/mod/` and friends. I'm fixing this now — adding a 10K file cap, expanding the ignore list for common cache/dependency dirs, and reducing the fallback depth from 8 to 4. Should make non-git-repo startup fast and bounded. Thanks for reporting this!

## #331 — Day banner shows DAY_COUNT from runtime file
**Action:** Implementing as Task 2.

This is self-filed. Baking DAY_COUNT at compile time via `build.rs` so external users get a clean banner without the day (instead of wrong/missing data).

## #215 — Challenge: Design and build a beautiful modern TUI
**Action:** Defer.

This is aspirational and would be a multi-week effort requiring significant architectural changes. Not the right time — focusing on stability, bugs, and competitive feature gaps first. The REPL experience is where most users live and it's getting better incrementally.

## #229 — Consider using Rust Token Killer
**Action:** Defer (ongoing integration already in progress).

## #226 — Evolution History
**Action:** Already partially addressed — `/evolution` command added Day 55 with CI status. More enhancements possible later.

## #156 — Submit to official coding agent benchmarks
**Action:** Defer (help wanted — needs benchmark infrastructure expertise).

## #307 — buybeerfor.me for crypto donations
**Action:** Defer.

## #141 — GROWTH.md proposal
**Action:** Defer.

## #98 — A Way of Evolution
**Action:** Defer (philosophical, no action needed).
