# Task 01: Add bin-level test and expose VERSION constant

Title: Add bin-level test and expose VERSION constant
Files: src/lib.rs, src/bin/yyds.rs
Issue: none
Origin: planner

## Objective
Give `cargo test --bin yyds` actual verification power (currently runs 0 tests) by adding a minimal integration test that validates the binary links correctly and the version constant is accessible.

## Why this matters
The harness policy's required gate `cargo test --bin yyds -- --test-threads=1` currently passes trivially because `src/bin/yyds.rs` has zero `#[test]` functions. This means:
- A broken library linkage wouldn't be caught until `cargo test --test integration` runs
- The gate provides a false sense of verification
- Adding even one test converts this from a no-op to a real smoke check

The task also exposes `VERSION` in the public API (minor Rust hygiene) so the bin test can verify it matches expectations.

## Success Criteria
- `cargo test --bin yyds` runs at least 1 test and it passes
- `yyds::VERSION` is accessible from external code (= "0.1.14" or later)
- Existing tests (`cargo test --test integration`) continue to pass
- `cargo check` and `cargo fmt --check` remain green

## Verification
```bash
cargo test --bin yyds -- --test-threads=1
cargo test --test integration -- --test-threads=1
cargo check
cargo fmt --check
```

## Expected Evidence
- `cargo test --bin yyds` reports "1 passed" (or more)
- No regressions in integration test suite
- State event: FileEdited on src/lib.rs, FileEdited on src/bin/yyds.rs

## Implementation Notes

### Step 1: Expose VERSION in lib.rs (1 line)
Add a re-export after the `mod cli_config;` declaration:
```rust
pub use cli_config::VERSION;
```
This makes `yoyo_ds_harness::VERSION` accessible to the bin target and external consumers.

### Step 2: Add test to src/bin/yyds.rs (~10 lines)
Add a `#[test]` function that verifies:
- The version constant is non-empty
- The version follows semver format (contains at least "0.")

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_version_constant_accessible() {
        let version = yoyo_ds_harness::VERSION;
        assert!(!version.is_empty(), "VERSION must not be empty");
        assert!(
            version.starts_with("0."),
            "VERSION should be 0.x.y, got: {version}"
        );
    }
}
```

### Scope boundaries
- Do NOT modify Cargo.toml, any other source files, or test files
- Do NOT test actual CLI output (requires process spawning — too complex for this task)
- If the crate name in Cargo.toml is `yoyo-ds-harness` (with hyphens), the Rust import uses underscores: `yoyo_ds_harness::VERSION`
