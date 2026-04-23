Title: Enrich `yoyo version` with build metadata
Files: src/commands_info.rs, src/cli.rs
Issue: none

## What

The assessment notes: "`yoyo version` output is bare. Just prints `yoyo v0.1.9` — no build date, no commit hash, no provider info. Claude Code shows much richer version info."

Enrich the version output to show:
- Version number (already exists)
- Build date (compile-time via `env!("VERGEN_BUILD_DATE")` or a simpler approach)
- Git commit hash (short, compile-time)
- Target triple (compile-time via `env!("TARGET")` or `std::env::consts`)
- Default provider and model

## How

### Approach: Use `std::env::consts` + build script or compile-time env vars

The simplest approach that doesn't require adding the `vergen` crate:

1. **In `src/cli.rs`** (or a new build.rs): Add compile-time constants:
   - Use `env!("CARGO_PKG_VERSION")` (already done)
   - Get target with `std::env::consts::ARCH` and `std::env::consts::OS`
   - For git hash: use `option_env!("GIT_HASH")` — set during CI builds via `GIT_HASH=$(git rev-parse --short HEAD) cargo build`. If not set, show "dev".
   - For build date: use `option_env!("BUILD_DATE")` — set during CI. If not set, show "dev".

2. **In `src/commands_info.rs`**, update `handle_version()`:
   ```
   yoyo v0.1.9 (abc1234 2026-04-23) linux-x86_64
   ```
   When verbose (`-v` or `--verbose` flag passed): also show provider, model, and yoagent version.

3. **In `src/cli.rs`**, update the `try_dispatch_subcommand` version branch to pass through any `-v`/`--verbose` flag.

4. The `print_help()` / `print_banner()` can stay as-is — this only changes the `yoyo version` / `yoyo --version` output.

## Simpler alternative if build.rs is too heavy

Just use runtime detection:
- Git hash: `git rev-parse --short HEAD 2>/dev/null` at startup (cached)
- Build date: skip, or use compile-time `env!("CARGO_PKG_VERSION")` only
- Target: `std::env::consts::ARCH` + `std::env::consts::OS`

This is less elegant but avoids build.rs complexity.

**Recommended:** Use `option_env!` for git hash and build date (set by CI/release builds), fall back to runtime git for dev builds. Keep it simple.

## Verification

- `cargo build && cargo test`
- `cargo clippy --all-targets -- -D warnings`
- `yoyo version` shows enriched output
- `yoyo --version` shows enriched output
- Tests for version display formatting

## Docs

No doc changes needed — this is a display-only improvement.
