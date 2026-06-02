//! Local benchmark fixture task format and loader.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

pub const DEFAULT_FIXTURE_ROOT: &str = "eval/fixtures";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkTask {
    pub task_id: String,
    pub category: String,
    pub repo_fixture: String,
    pub initial_commit: String,
    pub goal: String,
    pub tests: Vec<String>,
    pub hidden_failure_mode: String,
    pub expected_files: Vec<String>,
    pub risk_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureSuite {
    pub suite: String,
    pub root: PathBuf,
    pub tasks: Vec<BenchmarkTask>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixtureTaskResult {
    pub task_id: String,
    pub passed: bool,
    pub command_results: Vec<FixtureCommandResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixtureCommandResult {
    pub command: String,
    pub passed: bool,
    pub status_code: Option<i32>,
    pub duration_ms: u64,
    pub stdout_preview: String,
    pub stderr_preview: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixtureAgentAttemptResult {
    pub task_id: String,
    pub passed: bool,
    pub worktree: String,
    pub agent_command: String,
    pub agent_result: FixtureCommandResult,
    #[serde(default = "default_true")]
    pub mutation_scope_passed: bool,
    #[serde(default)]
    pub changed_files: Vec<String>,
    #[serde(default)]
    pub unexpected_changed_files: Vec<String>,
    pub command_results: Vec<FixtureCommandResult>,
}

impl FixtureSuite {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        let mut seen = std::collections::HashSet::new();
        if self.tasks.is_empty() {
            errors.push(format!("suite '{}' has no tasks", self.suite));
        }
        for task in &self.tasks {
            errors.extend(validate_task(task));
            if !seen.insert(task.task_id.clone()) {
                errors.push(format!("duplicate task_id '{}'", task.task_id));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

pub fn load_fixture_suite(suite: &str) -> Result<FixtureSuite, String> {
    load_fixture_suite_from(DEFAULT_FIXTURE_ROOT, suite)
}

pub fn load_fixture_suite_from(
    root: impl AsRef<Path>,
    suite: &str,
) -> Result<FixtureSuite, String> {
    let root = root.as_ref().join(suite);
    let mut entries = std::fs::read_dir(&root)
        .map_err(|e| format!("read fixture suite '{}': {e}", root.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("read fixture suite '{}': {e}", root.display()))?;
    entries.sort_by_key(|entry| entry.path());

    let mut tasks = Vec::new();
    for entry in entries {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| format!("read fixture task '{}': {e}", path.display()))?;
        let value: Value = serde_json::from_str(&raw)
            .map_err(|e| format!("parse fixture task '{}': {e}", path.display()))?;
        reject_unknown_fields(&path, &value)?;
        let task: BenchmarkTask = serde_json::from_value(value)
            .map_err(|e| format!("decode fixture task '{}': {e}", path.display()))?;
        validate_task_manifest_path(&path, &task)?;
        tasks.push(task);
    }

    let suite = FixtureSuite {
        suite: suite.to_string(),
        root,
        tasks,
    };
    suite
        .validate()
        .map_err(|errors| format!("fixture validation failed:\n{}", errors.join("\n")))?;
    Ok(suite)
}

pub fn run_fixture_task(task: &BenchmarkTask) -> FixtureTaskResult {
    run_fixture_task_in(task, None)
}

pub fn run_fixture_task_in(task: &BenchmarkTask, workdir: Option<&Path>) -> FixtureTaskResult {
    let mut command_results = Vec::new();
    for command in &task.tests {
        command_results.push(run_fixture_command(command, workdir));
    }
    let passed = command_results.iter().all(|result| result.passed);
    FixtureTaskResult {
        task_id: task.task_id.clone(),
        passed,
        command_results,
    }
}

pub fn run_fixture_agent_attempt(
    task: &BenchmarkTask,
    worktree: &Path,
    agent_command_template: &str,
) -> Result<FixtureAgentAttemptResult, String> {
    if !worktree.is_dir() {
        return Err(format!(
            "fixture agent attempt worktree '{}' is not a directory",
            worktree.display()
        ));
    }

    let agent_command = render_agent_command(agent_command_template, task);
    let agent_result = run_fixture_command(&agent_command, Some(worktree));
    let changed_files = collect_worktree_changed_files(worktree);
    let unexpected_changed_files = unexpected_changed_files_for_task(task, &changed_files);
    let mutation_scope_passed = unexpected_changed_files.is_empty();
    let command_results = if agent_result.passed {
        run_fixture_task_in(task, Some(worktree)).command_results
    } else {
        Vec::new()
    };
    let passed = agent_result.passed
        && mutation_scope_passed
        && command_results.iter().all(|result| result.passed);

    Ok(FixtureAgentAttemptResult {
        task_id: task.task_id.clone(),
        passed,
        worktree: worktree.display().to_string(),
        agent_command,
        agent_result,
        mutation_scope_passed,
        changed_files,
        unexpected_changed_files,
        command_results,
    })
}

fn default_true() -> bool {
    true
}

fn unexpected_changed_files_for_task(
    task: &BenchmarkTask,
    changed_files: &[String],
) -> Vec<String> {
    let mut unexpected = changed_files
        .iter()
        .filter(|file| !fixture_expected_file_allows(&task.expected_files, file))
        .cloned()
        .collect::<Vec<_>>();
    unexpected.sort();
    unexpected.dedup();
    unexpected
}

fn fixture_expected_file_allows(expected_files: &[String], changed_file: &str) -> bool {
    let changed_file = changed_file.trim();
    expected_files.iter().any(|expected| {
        let expected = expected.trim();
        if expected.ends_with('/') {
            changed_file.starts_with(expected)
        } else {
            changed_file == expected
        }
    })
}

fn collect_worktree_changed_files(worktree: &Path) -> Vec<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(worktree)
        .arg("status")
        .arg("--short")
        .arg("-uall")
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let mut files = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(parse_git_status_path)
        .collect::<Vec<_>>();
    files.sort();
    files.dedup();
    files
}

fn parse_git_status_path(line: &str) -> Option<String> {
    if line.len() < 4 {
        return None;
    }
    let raw_path = line.get(3..).unwrap_or("");
    let path = raw_path
        .split_once(" -> ")
        .map(|(_, new_path)| new_path)
        .unwrap_or(raw_path)
        .trim();
    if path.is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}

pub fn render_agent_command(template: &str, task: &BenchmarkTask) -> String {
    template
        .replace("{task_id}", &shell_quote(&task.task_id))
        .replace("{category}", &shell_quote(&task.category))
        .replace("{goal}", &shell_quote(&task.goal))
        .replace(
            "{hidden_failure_mode}",
            &shell_quote(&task.hidden_failure_mode),
        )
        .replace(
            "{expected_files}",
            &shell_quote(&task.expected_files.join(" ")),
        )
        .replace("{risk_label}", &shell_quote(&task.risk_label))
        .replace("{tests}", &shell_quote(&task.tests.join(" && ")))
}

pub fn format_fixture_list(suite: &FixtureSuite) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Eval fixture suite: {}\nroot: {}\n",
        suite.suite,
        suite.root.display()
    ));
    for task in &suite.tasks {
        out.push_str(&format!(
            "  {:<28} {:<22} {:<8} {}\n",
            task.task_id, task.category, task.risk_label, task.goal
        ));
    }
    out.trim_end().to_string()
}

fn validate_task(task: &BenchmarkTask) -> Vec<String> {
    let mut errors = Vec::new();
    required(&mut errors, &task.task_id, "task_id");
    validate_task_id_slug(&mut errors, &task.task_id);
    required(&mut errors, &task.category, "category");
    required(&mut errors, &task.repo_fixture, "repo_fixture");
    required(&mut errors, &task.initial_commit, "initial_commit");
    required(&mut errors, &task.goal, "goal");
    required(
        &mut errors,
        &task.hidden_failure_mode,
        "hidden_failure_mode",
    );
    required(&mut errors, &task.risk_label, "risk_label");
    if task.tests.is_empty() {
        errors.push(format!("task '{}' has no tests", task.task_id));
    }
    for command in &task.tests {
        if command.trim().is_empty() {
            errors.push(format!("task '{}' has empty test command", task.task_id));
        }
    }
    if task.expected_files.is_empty() {
        errors.push(format!("task '{}' has no expected_files", task.task_id));
    }
    for expected_file in &task.expected_files {
        validate_expected_file_entry(&mut errors, &task.task_id, expected_file);
    }
    if !matches!(task.risk_label.as_str(), "low" | "medium" | "high") {
        errors.push(format!(
            "task '{}' has invalid risk_label '{}'",
            task.task_id, task.risk_label
        ));
    }
    errors
}

fn validate_task_id_slug(errors: &mut Vec<String>, task_id: &str) {
    let value = task_id.trim();
    if value.is_empty() {
        return;
    }
    let chars_are_valid = value
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-');
    if !chars_are_valid || value.starts_with('-') || value.ends_with('-') || value.contains("--") {
        errors.push(format!(
            "task '{value}' has invalid task_id slug; use lowercase letters, digits, and single hyphens"
        ));
    }
}

fn validate_task_manifest_path(path: &Path, task: &BenchmarkTask) -> Result<(), String> {
    let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
        return Ok(());
    };
    let slug = stem
        .split_once('-')
        .filter(|(prefix, _)| prefix.chars().all(|ch| ch.is_ascii_digit()))
        .map(|(_, slug)| slug)
        .unwrap_or(stem);
    if slug != task.task_id {
        return Err(format!(
            "fixture task '{}' has task_id '{}' but filename slug is '{}'",
            path.display(),
            task.task_id,
            slug
        ));
    }
    Ok(())
}

fn validate_expected_file_entry(errors: &mut Vec<String>, task_id: &str, expected_file: &str) {
    let value = expected_file.trim();
    if value.is_empty() {
        errors.push(format!("task '{task_id}' has empty expected_files entry"));
        return;
    }
    if value.starts_with('/') {
        errors.push(format!(
            "task '{task_id}' expected file '{value}' must be repo-relative"
        ));
    }
    if value.contains('\\') {
        errors.push(format!(
            "task '{task_id}' expected file '{value}' must use forward slashes"
        ));
    }
    if value
        .split('/')
        .any(|segment| matches!(segment, ".." | "."))
    {
        errors.push(format!(
            "task '{task_id}' expected file '{value}' must not contain . or .. path segments"
        ));
    }
    if value.starts_with(".git/") || value == ".git" {
        errors.push(format!(
            "task '{task_id}' expected file '{value}' must not target .git"
        ));
    }
}

fn required(errors: &mut Vec<String>, value: &str, field: &str) {
    if value.trim().is_empty() {
        errors.push(format!("missing required field '{field}'"));
    }
}

fn reject_unknown_fields(path: &Path, value: &Value) -> Result<(), String> {
    let Some(obj) = value.as_object() else {
        return Err(format!(
            "fixture task '{}' must be an object",
            path.display()
        ));
    };
    let allowed = [
        "task_id",
        "category",
        "repo_fixture",
        "initial_commit",
        "goal",
        "tests",
        "hidden_failure_mode",
        "expected_files",
        "risk_label",
    ];
    for key in obj.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!(
                "fixture task '{}' has unknown field '{}'",
                path.display(),
                key
            ));
        }
    }
    Ok(())
}

fn run_fixture_command(command: &str, workdir: Option<&Path>) -> FixtureCommandResult {
    let started = Instant::now();
    let mut cmd = Command::new("/bin/sh");
    cmd.arg("-lc").arg(command);
    if let Some(workdir) = workdir {
        cmd.current_dir(workdir);
    }
    let output = cmd.output();
    match output {
        Ok(output) => FixtureCommandResult {
            command: command.to_string(),
            passed: output.status.success(),
            status_code: output.status.code(),
            duration_ms: started.elapsed().as_millis() as u64,
            stdout_preview: preview(&String::from_utf8_lossy(&output.stdout), 2000),
            stderr_preview: preview(&String::from_utf8_lossy(&output.stderr), 2000),
        },
        Err(e) => FixtureCommandResult {
            command: command.to_string(),
            passed: false,
            status_code: None,
            duration_ms: started.elapsed().as_millis() as u64,
            stdout_preview: String::new(),
            stderr_preview: e.to_string(),
        },
    }
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn preview(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    let mut out = if trimmed.chars().count() <= max_chars {
        trimmed.to_string()
    } else {
        let mut out = trimmed.chars().take(max_chars).collect::<String>();
        out.push_str("\n...[truncated]");
        out
    };
    if let Value::String(redacted) = crate::state::redact_state_payload(&Value::String(out.clone()))
    {
        out = redacted;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_task(id: &str) -> BenchmarkTask {
        BenchmarkTask {
            task_id: id.to_string(),
            category: "single-file bug fix".to_string(),
            repo_fixture: "self".to_string(),
            initial_commit: "current".to_string(),
            goal: "Fix a deterministic local bug".to_string(),
            tests: vec!["true".to_string()],
            hidden_failure_mode: "context miss".to_string(),
            expected_files: vec!["src/context.rs".to_string()],
            risk_label: "low".to_string(),
        }
    }

    #[test]
    fn validates_required_task_fields() {
        let suite = FixtureSuite {
            suite: "local-smoke".to_string(),
            root: PathBuf::from("eval/fixtures/local-smoke"),
            tasks: vec![valid_task("task-1")],
        };
        assert!(suite.validate().is_ok());

        let mut bad = valid_task("");
        bad.tests.clear();
        bad.risk_label = "critical".to_string();
        let suite = FixtureSuite {
            suite: "bad".to_string(),
            root: PathBuf::new(),
            tasks: vec![bad],
        };
        let errors = suite.validate().unwrap_err();
        assert!(errors.iter().any(|error| error.contains("task_id")));
        assert!(errors.iter().any(|error| error.contains("no tests")));
        assert!(errors
            .iter()
            .any(|error| error.contains("invalid risk_label")));
    }

    #[test]
    fn rejects_duplicate_task_ids() {
        let suite = FixtureSuite {
            suite: "dupes".to_string(),
            root: PathBuf::new(),
            tasks: vec![valid_task("task-1"), valid_task("task-1")],
        };
        let errors = suite.validate().unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.contains("duplicate task_id")));
    }

    #[test]
    fn rejects_non_slug_task_ids_and_empty_commands() {
        let mut bad = valid_task("Bad Task");
        bad.tests = vec!["true".to_string(), "  ".to_string()];
        let suite = FixtureSuite {
            suite: "bad".to_string(),
            root: PathBuf::new(),
            tasks: vec![bad],
        };

        let errors = suite.validate().unwrap_err();

        assert!(errors
            .iter()
            .any(|error| error.contains("invalid task_id slug")));
        assert!(errors
            .iter()
            .any(|error| error.contains("empty test command")));
    }

    #[test]
    fn rejects_fixture_manifest_filename_task_id_drift() {
        let dir = tempfile::tempdir().unwrap();
        let suite_dir = dir.path().join("local-smoke");
        std::fs::create_dir_all(&suite_dir).unwrap();
        let raw = serde_json::to_string_pretty(&valid_task("actual-task")).unwrap();
        std::fs::write(suite_dir.join("001-different-task.json"), raw).unwrap();

        let error = load_fixture_suite_from(dir.path(), "local-smoke").unwrap_err();

        assert!(error.contains("task_id 'actual-task'"));
        assert!(error.contains("filename slug is 'different-task'"));
    }

    #[test]
    fn rejects_unsafe_expected_file_allowlist_entries() {
        let mut bad = valid_task("unsafe-files");
        bad.expected_files = vec![
            "/tmp/escape.rs".to_string(),
            "../outside.rs".to_string(),
            ".git/config".to_string(),
            "src\\main.rs".to_string(),
        ];
        let suite = FixtureSuite {
            suite: "bad".to_string(),
            root: PathBuf::new(),
            tasks: vec![bad],
        };

        let errors = suite.validate().unwrap_err();

        assert!(errors.iter().any(|error| error.contains("repo-relative")));
        assert!(errors
            .iter()
            .any(|error| error.contains("must not contain . or ..")));
        assert!(errors.iter().any(|error| error.contains(".git")));
        assert!(errors.iter().any(|error| error.contains("forward slashes")));
    }

    #[test]
    fn list_report_contains_task_summary() {
        let suite = FixtureSuite {
            suite: "local-smoke".to_string(),
            root: PathBuf::from("eval/fixtures/local-smoke"),
            tasks: vec![valid_task("context-miss")],
        };
        let report = format_fixture_list(&suite);
        assert!(report.contains("context-miss"));
        assert!(report.contains("single-file bug fix"));
    }

    #[test]
    fn run_fixture_task_executes_commands() {
        let result = run_fixture_task(&valid_task("task-1"));
        assert!(result.passed);
        assert_eq!(result.command_results.len(), 1);
    }

    #[test]
    fn renders_agent_command_with_shell_quoted_task_fields() {
        let mut task = valid_task("task-1");
        task.goal = "fix Bob's retry bug".to_string();
        task.hidden_failure_mode = "retry file's context missing".to_string();
        let command = render_agent_command(
            "agent --task {task_id} --prompt {goal} --risk {risk_label} --why {hidden_failure_mode}",
            &task,
        );
        assert_eq!(
            command,
            "agent --task 'task-1' --prompt 'fix Bob'\\''s retry bug' --risk 'low' --why 'retry file'\\''s context missing'"
        );
    }

    #[test]
    fn agent_attempt_runs_agent_then_fixture_tests_in_worktree() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut task = valid_task("task-1");
        task.expected_files = vec!["src/lib.rs".into()];
        let result = run_fixture_agent_attempt(&task, dir.path(), "true").unwrap();
        assert!(result.passed);
        assert_eq!(result.agent_result.command, "true");
        assert_eq!(result.command_results.len(), 1);
    }

    #[test]
    fn agent_attempt_records_changed_files_after_agent_command() {
        let dir = tempfile::TempDir::new().unwrap();
        assert!(Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .arg("init")
            .status()
            .unwrap()
            .success());
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        let mut task = valid_task("task-1");
        task.tests = vec!["test -f src/lib.rs".into()];
        task.expected_files = vec!["src/lib.rs".into()];
        let result = run_fixture_agent_attempt(
            &task,
            dir.path(),
            "mkdir -p src && printf patched > src/lib.rs",
        )
        .unwrap();

        assert!(result.passed);
        assert_eq!(result.changed_files, vec!["src/lib.rs"]);
    }

    #[test]
    fn agent_attempt_fails_when_changed_file_is_outside_expected_files() {
        let dir = tempfile::TempDir::new().unwrap();
        assert!(Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .arg("init")
            .status()
            .unwrap()
            .success());
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        let mut task = valid_task("task-1");
        task.expected_files = vec!["src/context.rs".into()];

        let result = run_fixture_agent_attempt(
            &task,
            dir.path(),
            "mkdir -p src && printf patched > src/context.rs && printf stray > src/lib.rs",
        )
        .unwrap();

        assert!(!result.passed);
        assert!(!result.mutation_scope_passed);
        assert_eq!(result.changed_files, vec!["src/context.rs", "src/lib.rs"]);
        assert_eq!(result.unexpected_changed_files, vec!["src/lib.rs"]);
        assert_eq!(result.command_results.len(), 1);
        assert!(fixture_expected_file_allows(
            &["src/".to_string()],
            "src/lib.rs"
        ));
    }

    #[test]
    fn parses_git_status_paths_for_renames() {
        assert_eq!(
            parse_git_status_path(" M src/lib.rs").as_deref(),
            Some("src/lib.rs")
        );
        assert_eq!(
            parse_git_status_path("R  src/old.rs -> src/new.rs").as_deref(),
            Some("src/new.rs")
        );
    }

    #[test]
    fn command_preview_redacts_secrets_before_fixture_results() {
        let redacted = preview("api_key=sk-testsecret123456789", 2000);

        assert!(!redacted.contains("sk-testsecret123456789"));
        assert!(redacted.contains("[redacted]") || redacted.contains("sk-[redacted]"));
    }
}
