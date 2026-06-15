Verdict: PASS
Reason: `pub use cli_config::VERSION;` correctly exposes the constant in `src/lib.rs`, and `cargo test --bin yyds` now runs 1 test (`test_version_constant_accessible`) that passes, verifying the version is non-empty and starts with "0.". No regressions.
