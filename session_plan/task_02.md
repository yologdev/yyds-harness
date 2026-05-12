Title: Expand ProjectType to support Java, Ruby, and C/C++ projects
Files: src/commands_project.rs, src/commands_lint.rs
Issue: none

yoyo currently detects 5 project types: Rust, Node, Python, Go, Make. Java, Ruby, and C/C++ are among the most popular languages but get `Unknown` — which means no auto-watch, no `/test`, no `/lint`, no project line in the banner. This is a first-contact experience gap: a Java developer opens yoyo and it doesn't even recognize their project.

**Add three new ProjectType variants:**

1. **`Java`** — detect by `pom.xml` (Maven) or `build.gradle`/`build.gradle.kts` (Gradle)
   - Display: `"Java (Maven)"` or `"Java (Gradle)"` — use sub-detection
   - Actually, simpler: just `"Java"` for the Display impl
   - Test command: Maven → `"mvn test"`, Gradle → `"./gradlew test"`
   - Lint command: Maven → `"mvn checkstyle:check"` (common but not universal), Gradle → `"./gradlew check"` — or just `None` to be safe (many Java projects don't have a standard lint)
   - For simplicity: use a single `Java` variant. In `detect_project_type`, check `pom.xml` first, then `build.gradle`/`build.gradle.kts`. In `test_command_for_project`, detect which build tool exists at runtime (check if `pom.xml` exists → mvn, else → gradlew).

2. **`Ruby`** — detect by `Gemfile`
   - Display: `"Ruby"`
   - Test command: `"bundle exec rake test"` or `"bundle exec rspec"` — detect by checking if `spec/` directory exists (rspec) vs default (rake test). For simplicity: `"bundle exec rake test"`.
   - Lint command: `"bundle exec rubocop"` (very standard in Ruby)

3. **`Cpp`** — detect by `CMakeLists.txt` or `Makefile` + `*.cpp`/`*.c` files
   - Actually, `Makefile` is already handled by the `Make` variant. Better: detect `CMakeLists.txt` specifically.
   - Display: `"C/C++ (CMake)"`
   - Test command: `"cmake --build build && ctest --test-dir build"` — or just `"ctest --test-dir build"` since build is separate
   - Lint command: None (too varied — clang-tidy, cppcheck, etc.)

**Detection order matters** — check specific build files before generic ones. Order should be:
Cargo.toml → package.json → pom.xml/build.gradle → Gemfile → pyproject.toml/setup.py → go.mod → CMakeLists.txt → Makefile → Unknown

**In `commands_project.rs`:**
- Add `Java`, `Ruby`, `Cpp` to `enum ProjectType`
- Add `Display` impl cases
- Add detection in `detect_project_type` (in correct priority order)
- Add build commands in `build_commands_for_project` (line ~421)
- Update `scan_important_files` and `scan_important_dirs` if they have project-type branches

**In `commands_lint.rs`:**
- Add `test_command_for_project` cases for Java, Ruby, Cpp
- Add `lint_command_for_project` cases (Ruby → rubocop, Java/Cpp → None)

**Tests to add:**
- Test `detect_project_type` with temp dirs containing each new marker file
- Test `test_command_for_project` returns correct command for each new type
- Test `lint_command_for_project` returns correct command for Ruby, None for Java/Cpp
- Test Display output for each new variant

No CLAUDE.md update needed — project type detection is internal infrastructure.
