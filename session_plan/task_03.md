Title: Add /doctor checks for Java, Ruby, and C/C++ projects and tests for health check coverage
Files: src/commands_dev.rs
Issue: none

The `/doctor` command runs health checks for the current project type (toolchain installed, config valid, dependencies up to date). With Task 2 adding Java, Ruby, and Cpp project types, `/doctor` needs corresponding health checks so developers in those ecosystems get useful diagnostics.

**In `commands_dev.rs`:**

1. **Add health checks in `health_checks_for_project`** (or equivalent function that maps ProjectType to checks):
   - **Java**: Check `java --version`, `mvn --version` or `gradle --version` (whichever matches), `JAVA_HOME` env var set
   - **Ruby**: Check `ruby --version`, `bundle --version`, `gem --version`
   - **Cpp**: Check `cmake --version`, `make --version`, compiler (`cc --version` or `g++ --version`)

2. **Add tests** for the new project type health checks:
   - Test that `health_checks_for_project` returns the expected check names/commands for Java, Ruby, Cpp
   - Test that the doctor report formatting handles the new project types
   - These should be unit tests that verify the check list generation, not integration tests that actually run the commands

Also add tests for existing `/doctor` functionality that may be under-tested:
   - Test `run_doctor_checks` with a mock/known project type returns expected structure
   - Test `print_doctor_report` formatting with various check statuses (pass, fail, warning)
   - Test `DoctorCheck` struct construction and `DoctorStatus` enum coverage

Target: 8-12 tests covering both the new project types and existing doctor infrastructure.
