//! Local benchmark fixture task format and loader.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};

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
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
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

/// Per-category aggregate score from a scored fixture suite.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CategoryScore {
    pub passed: usize,
    pub failed: usize,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
}

impl CategoryScore {
    fn new() -> Self {
        Self {
            passed: 0,
            failed: 0,
            total: 0,
            score: None,
        }
    }

    fn record(&mut self, passed: bool) {
        self.total += 1;
        if passed {
            self.passed += 1;
        } else {
            self.failed += 1;
        }
    }

    fn finalize(&mut self) {
        if self.total > 0 {
            self.score = Some(self.passed as f64 / self.total as f64);
        }
    }
}

/// Aggregate score for a fixture suite run.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FixtureScore {
    pub suite: String,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub score: f64,
    pub categories: BTreeMap<String, CategoryScore>,
    pub risk_levels: BTreeMap<String, CategoryScore>,
    pub task_results: Vec<FixtureTaskResult>,
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
        command_results.push(run_fixture_command(
            command,
            workdir,
            task.timeout_secs.unwrap_or(120),
        ));
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
    let agent_result = run_fixture_command(
        &agent_command,
        Some(worktree),
        task.timeout_secs.unwrap_or(120),
    );
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

/// Group fixtures by primary domain (first segment of category before '/',
/// or first word of category if no '/'), sorted by count descending.
pub fn format_fixture_list_by_domain(suite: &FixtureSuite) -> String {
    let mut domains: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for task in &suite.tasks {
        let domain = if task.category.is_empty() {
            "other".to_string()
        } else if let Some(idx) = task.category.find('/') {
            task.category[..idx].trim().to_string()
        } else {
            // Take first word (split on whitespace)
            task.category
                .split_whitespace()
                .next()
                .unwrap_or("other")
                .to_string()
        };
        domains.entry(domain).or_default().push(&task.task_id);
    }
    let mut sorted: Vec<(String, Vec<&str>)> = domains.into_iter().collect();
    sorted.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));

    let total_tasks: usize = suite.tasks.iter().map(|t| t.tests.len()).sum();
    let mut out = String::new();
    out.push_str(&format!(
        "Eval fixture coverage by domain ({}, {} fixtures, {} tasks):\n",
        suite.suite,
        suite.tasks.len(),
        total_tasks,
    ));
    let max_domain_width = sorted.iter().map(|(d, _)| d.len()).max().unwrap_or(0);
    for (domain, names) in &sorted {
        let fixture_label = if names.len() == 1 {
            "fixture"
        } else {
            "fixtures"
        };
        let compact: Vec<String> = names.iter().take(5).map(|n| n.to_string()).collect();
        let more = if names.len() > 5 {
            format!(" (+{} more)", names.len() - 5)
        } else {
            String::new()
        };
        out.push_str(&format!(
            "  {:<width$} {:>3} {}  ({}{})\n",
            domain,
            names.len(),
            fixture_label,
            compact.join(", "),
            more,
            width = max_domain_width + 2,
        ));
    }
    out.trim_end().to_string()
}

/// Run all (or a sampled subset of) fixture tasks and compute an aggregate
/// pass/fail score with per-category and per-risk-level breakdowns.
///
/// When `sample` is set, a deterministic subset of tasks is selected using a
/// hash of the suite name as the seed. The score reflects only the sampled tasks.
pub fn score_fixture_suite(suite: &FixtureSuite, sample: Option<usize>) -> FixtureScore {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut total: usize = 0;
    let mut passed: usize = 0;
    let mut failed: usize = 0;
    let mut categories: BTreeMap<String, CategoryScore> = BTreeMap::new();
    let mut risk_levels: BTreeMap<String, CategoryScore> = BTreeMap::new();
    let mut task_results: Vec<FixtureTaskResult> = Vec::new();

    // Select tasks: either all or a deterministic sample
    let selected: Vec<&BenchmarkTask> = if let Some(n) = sample {
        if n == 0 {
            // --sample 0 means "score all"
            suite.tasks.iter().collect()
        } else if suite.tasks.is_empty() {
            Vec::new()
        } else {
            let mut hasher = DefaultHasher::new();
            suite.suite.hash(&mut hasher);
            let seed = hasher.finish();
            let mut indices: Vec<usize> = (0..suite.tasks.len()).collect();
            // Deterministic shuffle using the seed as a u64 rng stand-in
            for i in 0..suite.tasks.len() {
                let j = (seed as usize + i * 2654435761) % suite.tasks.len();
                indices.swap(i, j);
            }
            indices
                .into_iter()
                .take(n.min(suite.tasks.len()))
                .map(|i| &suite.tasks[i])
                .collect()
        }
    } else {
        suite.tasks.iter().collect()
    };

    for task in &selected {
        let result = run_fixture_task(task);
        total += 1;
        if result.passed {
            passed += 1;
        } else {
            failed += 1;
        }

        // Per-category tally
        let cat_entry = categories
            .entry(task.category.clone())
            .or_insert_with(CategoryScore::new);
        cat_entry.record(result.passed);

        // Per-risk-level tally
        let risk_entry = risk_levels
            .entry(task.risk_label.clone())
            .or_insert_with(CategoryScore::new);
        risk_entry.record(result.passed);

        task_results.push(result);
    }

    // Finalize scores
    for cat in categories.values_mut() {
        cat.finalize();
    }
    for risk in risk_levels.values_mut() {
        risk.finalize();
    }

    let score = if total > 0 {
        passed as f64 / total as f64
    } else {
        0.0
    };

    FixtureScore {
        suite: suite.suite.clone(),
        total,
        passed,
        failed,
        score,
        categories,
        risk_levels,
        task_results,
    }
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

fn run_fixture_command(
    command: &str,
    workdir: Option<&Path>,
    timeout_secs: u64,
) -> FixtureCommandResult {
    let started = Instant::now();
    // Rewrite stale --bin yoyo references (binary renamed to yyds)
    let command = command.replace("--bin yoyo", "--bin yyds");
    let mut cmd = Command::new("/bin/sh");
    cmd.arg("-lc").arg(&command);
    if let Some(workdir) = workdir {
        cmd.current_dir(workdir);
    }

    let timeout = Duration::from_secs(timeout_secs.max(1));

    // Spawn the child process
    let child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            return FixtureCommandResult {
                command: command.to_string(),
                passed: false,
                status_code: None,
                duration_ms: started.elapsed().as_millis() as u64,
                stdout_preview: String::new(),
                stderr_preview: e.to_string(),
            };
        }
    };

    let pid = child.id();

    // Spawn a thread to wait for the child
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let output = child.wait_with_output();
        let _ = tx.send(output);
    });

    match rx.recv_timeout(timeout) {
        Ok(output_result) => match output_result {
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
        },
        Err(mpsc::RecvTimeoutError::Timeout) => {
            // Kill the child process
            let duration_ms = started.elapsed().as_millis() as u64;
            // Ignore errors: the process may have already exited
            let _ = Command::new("kill").arg("-9").arg(pid.to_string()).output();
            FixtureCommandResult {
                command: command.to_string(),
                passed: false,
                status_code: None,
                duration_ms,
                stdout_preview: String::new(),
                stderr_preview: format!("command timed out after {}s", timeout_secs),
            }
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            // Thread panicked or child was dropped; treat as failure
            let duration_ms = started.elapsed().as_millis() as u64;
            FixtureCommandResult {
                command: command.to_string(),
                passed: false,
                status_code: None,
                duration_ms,
                stdout_preview: String::new(),
                stderr_preview: "command process ended unexpectedly".to_string(),
            }
        }
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
            timeout_secs: None,
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

    // ── DeepSeek protocol compliance fixtures ───────────────────────

    fn ds_transport_policy() -> crate::deepseek::DeepSeekTransportPolicy {
        crate::deepseek::DeepSeekTransportPolicy {
            connect_timeout_ms: 10_000,
            request_timeout_ms: 120_000,
            max_retries: 2,
            initial_backoff_ms: 1_000,
            max_backoff_ms: 20_000,
            retry_statuses: vec![408, 409, 425, 429, 500, 502, 503, 504],
        }
    }

    fn ds_repair_policy() -> crate::deepseek::RepairPolicy {
        crate::deepseek::RepairPolicy {
            max_repair_turns: 2,
            record_failure_hypotheses: true,
        }
    }

    /// Feed valid tool-calls through validate_strict_tool_arguments for
    /// every strict schema.  All must pass to guard against schema-regression.
    #[test]
    fn deepseek_schema_validate_valid_all_schemas_pass() {
        use crate::deepseek::validate_strict_tool_arguments;
        use serde_json::json;

        let cases: Vec<(&str, serde_json::Value)> = vec![
            (
                "plan_task",
                json!({
                    "task_summary": "Fix bug",
                    "required_context": ["src/foo.rs"],
                    "risk_level": "low",
                    "verification_steps": ["cargo test"],
                    "schema_version": 1
                }),
            ),
            (
                "request_context",
                json!({
                    "paths": ["src/main.rs"],
                    "symbols": ["main"],
                    "recent_event_ids": ["evt-1"],
                    "why": "need context",
                    "schema_version": 1
                }),
            ),
            (
                "inspect_file",
                json!({
                    "path": "src/main.rs",
                    "line_start": 1,
                    "line_end": 42,
                    "reason": "check function body",
                    "schema_version": 1
                }),
            ),
            (
                "propose_edit",
                json!({
                    "path": "src/main.rs",
                    "intent": "Fix typo",
                    "expected_effect": "Typo fixed",
                    "risk_level": "low",
                    "verification_steps": ["cargo test"],
                    "schema_version": 1
                }),
            ),
            (
                "record_failure",
                json!({
                    "failure_summary": "Unexpected None",
                    "hypothesis": "Race with background task",
                    "evidence_event_ids": ["evt-a"],
                    "affected_component": "repl",
                    "next_repair_step": "Wrap in Arc<Mutex<>>",
                    "schema_version": 1
                }),
            ),
            (
                "propose_harness_patch",
                json!({
                    "target_component": "repair_policy",
                    "problem_summary": "Max repair turns too low",
                    "evidence_event_ids": ["evt-2"],
                    "proposed_change": "Increase max_repair_turns 2→3",
                    "expected_effect": "Fewer aborts",
                    "risk_level": "low",
                    "eval_plan": "Run eval suite 3x",
                    "schema_version": 1
                }),
            ),
            (
                "record_eval_result",
                json!({
                    "eval_id": "eval-001",
                    "harness_version": "ds-harness-genome-v1",
                    "patch_id": "patch-001",
                    "suite": "local-smoke",
                    "status": "passed",
                    "score": 0.98,
                    "passed": 12,
                    "failed": 0,
                    "failure_event_ids": [],
                    "schema_version": 1
                }),
            ),
            (
                "promote_or_reject_patch",
                json!({
                    "patch_id": "patch-001",
                    "decision": "promote",
                    "baseline_eval_id": "eval-001",
                    "candidate_eval_id": "eval-002",
                    "criterion": "regression",
                    "rationale": "All tests pass, no regressions",
                    "risk_level": "low",
                    "approval_event_ids": [],
                    "schema_version": 1
                }),
            ),
            (
                "request_human_approval",
                json!({
                    "approval_scope": "tool_execution",
                    "patch_id": "patch-001",
                    "tool_name": "bash",
                    "risk_level": "medium",
                    "reason": "Command requires sudo",
                    "evidence_event_ids": [],
                    "schema_version": 1
                }),
            ),
        ];

        for (schema_name, args) in &cases {
            let report = validate_strict_tool_arguments(schema_name, args)
                .unwrap_or_else(|e| panic!("{schema_name}: {e}"));
            assert!(
                report.valid,
                "schema '{schema_name}' should be valid but missing={:?} unexpected={:?} type_errors={:?} enum_errors={:?}",
                report.missing_required,
                report.unexpected_fields,
                report.type_errors,
                report.enum_errors,
            );
        }
    }

    /// Feed intentionally broken tool-calls and verify they fail with
    /// appropriate errors (missing required, wrong type, bad enum, extra key).
    #[test]
    fn deepseek_schema_validate_invalid_catches_errors() {
        use crate::deepseek::validate_strict_tool_arguments;
        use serde_json::json;

        // missing required field
        let report = validate_strict_tool_arguments(
            "plan_task",
            &json!({
                "risk_level": "low",
                "schema_version": 1
                // missing: task_summary, required_context, verification_steps
            }),
        )
        .unwrap();
        assert!(!report.valid);
        assert!(report
            .missing_required
            .contains(&"task_summary".to_string()));

        // wrong enum value
        let report = validate_strict_tool_arguments(
            "plan_task",
            &json!({
                "task_summary": "Fix",
                "required_context": [],
                "risk_level": "critical",
                "verification_steps": [],
                "schema_version": 1
            }),
        )
        .unwrap();
        assert!(!report.valid);
        assert!(report.enum_errors.iter().any(|e| e.contains("risk_level")));

        // wrong type (integer where string expected)
        let report = validate_strict_tool_arguments(
            "record_failure",
            &json!({
                "failure_summary": 42,
                "hypothesis": "ok",
                "evidence_event_ids": [],
                "affected_component": "ok",
                "next_repair_step": "ok",
                "schema_version": 1
            }),
        )
        .unwrap();
        assert!(!report.valid);
        assert!(report
            .type_errors
            .iter()
            .any(|e| e.contains("failure_summary") && e.contains("string")));

        // extra unknown field
        let report = validate_strict_tool_arguments(
            "inspect_file",
            &json!({
                "path": "src/main.rs",
                "line_start": 1,
                "line_end": 10,
                "reason": "inspect",
                "extra_unknown_field": true,
                "schema_version": 1
            }),
        )
        .unwrap();
        assert!(!report.valid);
        assert!(report
            .unexpected_fields
            .contains(&"extra_unknown_field".to_string()));

        // wrong schema_version enum value
        let report = validate_strict_tool_arguments(
            "propose_edit",
            &json!({
                "path": "src/main.rs",
                "intent": "fix",
                "expected_effect": "done",
                "risk_level": "low",
                "verification_steps": [],
                "schema_version": 99
            }),
        )
        .unwrap();
        assert!(!report.valid);
        assert!(report
            .enum_errors
            .iter()
            .any(|e| e.contains("schema_version")));

        // non-object input
        let report =
            validate_strict_tool_arguments("request_context", &json!("not an object")).unwrap();
        assert!(!report.valid);
        assert!(report
            .type_errors
            .iter()
            .any(|e| e.contains("must be a JSON object")));

        // unknown schema name → Err
        assert!(validate_strict_tool_arguments("no_such_schema", &json!({})).is_err());
    }

    /// Exercise classify_deepseek_transport_failure with representative
    /// HTTP status codes and error texts.
    #[test]
    fn deepseek_transport_classify_http_errors() {
        use crate::deepseek::{classify_deepseek_transport_failure, DeepSeekTransportErrorClass};

        let policy = ds_transport_policy();

        // 401 → Authentication (not retryable)
        let decision = classify_deepseek_transport_failure(Some(401), "", 0, &policy);
        assert_eq!(decision.class, DeepSeekTransportErrorClass::Authentication);
        assert!(!decision.retryable);

        // 429 → RateLimited (retryable)
        let decision = classify_deepseek_transport_failure(Some(429), "", 0, &policy);
        assert_eq!(decision.class, DeepSeekTransportErrorClass::RateLimited);
        assert!(decision.retryable);
        assert!(decision.next_backoff_ms.is_some());

        // 500 → ServerError (retryable)
        let decision = classify_deepseek_transport_failure(Some(500), "", 0, &policy);
        assert_eq!(decision.class, DeepSeekTransportErrorClass::ServerError);
        assert!(decision.retryable);

        // 503 → ServerError (retryable)
        let decision = classify_deepseek_transport_failure(Some(503), "", 0, &policy);
        assert_eq!(decision.class, DeepSeekTransportErrorClass::ServerError);
        assert!(decision.retryable);

        // timeout text → Timeout (retryable)
        let decision =
            classify_deepseek_transport_failure(None, "request timed out after 30s", 0, &policy);
        assert_eq!(decision.class, DeepSeekTransportErrorClass::Timeout);
        assert!(decision.retryable);

        // network error → Network (retryable)
        let decision =
            classify_deepseek_transport_failure(None, "connection reset by peer", 0, &policy);
        assert_eq!(decision.class, DeepSeekTransportErrorClass::Network);
        assert!(decision.retryable);

        // 422 → InvalidRequest (not retryable)
        let decision = classify_deepseek_transport_failure(Some(422), "", 0, &policy);
        assert_eq!(decision.class, DeepSeekTransportErrorClass::InvalidRequest);
        assert!(!decision.retryable);

        // 404 → NotFound (not retryable)
        let decision = classify_deepseek_transport_failure(Some(404), "", 0, &policy);
        assert_eq!(decision.class, DeepSeekTransportErrorClass::NotFound);
        assert!(!decision.retryable);

        // retries exhausted (attempt >= max_retries for transient error)
        let decision = classify_deepseek_transport_failure(Some(429), "", 2, &policy);
        assert_eq!(decision.class, DeepSeekTransportErrorClass::RateLimited);
        assert!(!decision.retryable);
        assert!(decision.reason.contains("budget exhausted"));
    }

    /// Test parse_json_output_attempts with valid JSON, malformed JSON,
    /// JSON with extra noise, empty responses, and retry scenarios.
    #[test]
    fn deepseek_json_output_parse_handles_all_statuses() {
        use crate::deepseek::{
            parse_json_output_attempts, JsonOutputAttemptStatus, JsonOutputParseOptions,
        };

        let options = JsonOutputParseOptions {
            source: "test".to_string(),
            schema_name: None,
            retry_once_on_invalid_or_empty: true,
        };

        // valid JSON
        let result = parse_json_output_attempts([r#"{"ok":true}"#], options.clone()).unwrap();
        assert_eq!(result.value, serde_json::json!({"ok": true}));
        assert!(!result.retry_used);
        assert_eq!(result.attempts.len(), 1);
        assert_eq!(result.attempts[0].status, JsonOutputAttemptStatus::Parsed);

        // empty response → Err with Empty attempts (feed two to hit retry path)
        let err = parse_json_output_attempts(["", ""], options.clone()).unwrap_err();
        assert_eq!(err.attempts.len(), 2);
        assert!(err
            .attempts
            .iter()
            .all(|a| a.status == JsonOutputAttemptStatus::Empty));
        assert!(err.retry_allowed);

        // malformed JSON → Err with Invalid attempts (feed two to hit retry path)
        let err = parse_json_output_attempts(["not json at all", "also bad"], options.clone())
            .unwrap_err();
        assert_eq!(err.attempts.len(), 2);
        assert!(err
            .attempts
            .iter()
            .all(|a| a.status == JsonOutputAttemptStatus::Invalid));
        assert!(err.attempts.iter().all(|a| a.error_preview.is_some()));

        // first invalid, second valid → retry_used = true
        let result = parse_json_output_attempts(["bad", r#"{"x":1}"#], options.clone()).unwrap();
        assert!(result.retry_used);
        assert_eq!(result.attempts.len(), 2);
        assert_eq!(result.attempts[0].status, JsonOutputAttemptStatus::Invalid);
        assert_eq!(result.attempts[1].status, JsonOutputAttemptStatus::Parsed);
        assert_eq!(result.value, serde_json::json!({"x": 1}));

        // first empty, second valid → retry_used = true
        let result = parse_json_output_attempts(["", r#"{"y":2}"#], options.clone()).unwrap();
        assert!(result.retry_used);
        assert_eq!(result.attempts[0].status, JsonOutputAttemptStatus::Empty);
        assert_eq!(result.attempts[1].status, JsonOutputAttemptStatus::Parsed);

        // retry disabled: only one attempt
        let no_retry = JsonOutputParseOptions {
            source: "test".to_string(),
            schema_name: None,
            retry_once_on_invalid_or_empty: false,
        };
        let err = parse_json_output_attempts(["garbage"], no_retry).unwrap_err();
        assert_eq!(err.attempts.len(), 1);
        assert!(!err.retry_allowed);
    }

    /// Exercise decide_tool_schema_repair with valid/invalid reports at
    /// various attempt levels to verify Accept / Retry / Abort transitions.
    #[test]
    fn deepseek_repair_decision_handles_budget() {
        use crate::deepseek::{
            decide_tool_schema_repair, validate_strict_tool_arguments, ToolSchemaRepairAction,
        };
        use serde_json::json;

        let policy = ds_repair_policy(); // max_repair_turns = 2

        // valid report → Accept
        let valid_report = validate_strict_tool_arguments(
            "plan_task",
            &json!({
                "task_summary": "Fix",
                "required_context": [],
                "risk_level": "low",
                "verification_steps": [],
                "schema_version": 1
            }),
        )
        .unwrap();
        assert!(valid_report.valid);
        let decision = decide_tool_schema_repair(&valid_report, 0, &policy);
        assert_eq!(decision.action, ToolSchemaRepairAction::Accept);
        assert!(!decision.should_record_failure);

        // invalid report, attempt 1 (still within budget) → Retry
        let invalid_report = validate_strict_tool_arguments(
            "plan_task",
            &json!({
                "risk_level": "low",
                "schema_version": 1
            }),
        )
        .unwrap();
        assert!(!invalid_report.valid);
        let decision = decide_tool_schema_repair(&invalid_report, 1, &policy);
        assert_eq!(decision.action, ToolSchemaRepairAction::Retry);
        assert!(!decision.should_record_failure);
        assert!(decision.instruction.is_some());

        // invalid report, attempt 3 (budget exhausted for max_repair_turns=2) → Abort
        let decision = decide_tool_schema_repair(&invalid_report, 3, &policy);
        assert_eq!(decision.action, ToolSchemaRepairAction::Abort);
        assert!(decision.should_record_failure);
        assert!(decision.reason.contains("budget exhausted"));
        // Abort should NOT carry a repair instruction
        assert!(decision.instruction.is_none());
    }

    #[test]
    fn format_fixture_list_by_domain_groups_and_sorts() {
        let suite = FixtureSuite {
            suite: "local-smoke".to_string(),
            root: PathBuf::from("eval/fixtures/local-smoke"),
            tasks: vec![
                BenchmarkTask {
                    task_id: "state-redaction".to_string(),
                    category: "state/security".to_string(),
                    repo_fixture: ".".to_string(),
                    initial_commit: "abc".to_string(),
                    goal: "test".to_string(),
                    tests: vec!["cargo test".to_string()],
                    hidden_failure_mode: "none".to_string(),
                    expected_files: vec!["src/lib.rs".to_string()],
                    timeout_secs: None,
                    risk_label: "high".to_string(),
                },
                BenchmarkTask {
                    task_id: "eval-scoring".to_string(),
                    category: "eval/metrics".to_string(),
                    repo_fixture: ".".to_string(),
                    initial_commit: "abc".to_string(),
                    goal: "test".to_string(),
                    tests: vec!["cargo test".to_string()],
                    hidden_failure_mode: "none".to_string(),
                    expected_files: vec!["src/lib.rs".to_string()],
                    timeout_secs: None,
                    risk_label: "medium".to_string(),
                },
                BenchmarkTask {
                    task_id: "eval-state-metrics".to_string(),
                    category: "eval/metrics".to_string(),
                    repo_fixture: ".".to_string(),
                    initial_commit: "abc".to_string(),
                    goal: "test".to_string(),
                    tests: vec!["cargo test".to_string()],
                    hidden_failure_mode: "none".to_string(),
                    expected_files: vec!["src/lib.rs".to_string()],
                    timeout_secs: None,
                    risk_label: "medium".to_string(),
                },
                BenchmarkTask {
                    task_id: "deepseek-profile".to_string(),
                    category: "deepseek/protocol".to_string(),
                    repo_fixture: ".".to_string(),
                    initial_commit: "abc".to_string(),
                    goal: "test".to_string(),
                    tests: vec!["cargo test".to_string()],
                    hidden_failure_mode: "none".to_string(),
                    expected_files: vec!["src/lib.rs".to_string()],
                    timeout_secs: None,
                    risk_label: "low".to_string(),
                },
                BenchmarkTask {
                    task_id: "unknown-domain".to_string(),
                    category: "".to_string(),
                    repo_fixture: ".".to_string(),
                    initial_commit: "abc".to_string(),
                    goal: "test".to_string(),
                    tests: vec!["cargo test".to_string()],
                    hidden_failure_mode: "none".to_string(),
                    expected_files: vec!["src/lib.rs".to_string()],
                    timeout_secs: None,
                    risk_label: "low".to_string(),
                },
            ],
        };

        let output = format_fixture_list_by_domain(&suite);

        // header with suite name and counts
        assert!(output.contains("Eval fixture coverage by domain"));
        assert!(output.contains("local-smoke"));
        assert!(output.contains("5 fixtures"));
        // tasks sum: 1+1+1+1+1 = 5
        assert!(output.contains("5 tasks"));

        // eval domain should come first (2 fixtures), then state (1), then deepseek (1), then other (1)
        let eval_pos = output.find("eval").unwrap();
        let state_pos = output.find("state").unwrap();
        let deepseek_pos = output.find("deepseek").unwrap();
        let other_pos = output.find("other").unwrap();
        assert!(
            eval_pos < state_pos,
            "eval (2 fixtures) should sort before state (1)"
        );
        assert!(
            state_pos < deepseek_pos,
            "state and deepseek tied at 1, alpha order"
        );
        assert!(
            deepseek_pos < other_pos,
            "deepseek should sort before other (empty category)"
        );
    }

    /// Smoke test: validates that the real fixture files on disk can be loaded,
    /// validated, and run through the fixture pipeline end-to-end.
    #[test]
    fn smoke_fixture_pipeline_with_real_fixture_data() {
        // Load the real fixture files from eval/fixtures/local-smoke/
        let suite = load_fixture_suite("local-smoke").expect("should load fixture suite");

        // Assert we have a meaningful number of tasks
        assert!(
            !suite.tasks.is_empty(),
            "fixture suite should contain at least one task"
        );

        // Validate the suite explicitly (load_fixture_suite already validates, but
        // call it again to exercise the API directly)
        suite.validate().expect("suite should validate");

        // Pick the first task and run it through the pipeline
        let first = &suite.tasks[0];
        let result = run_fixture_task(first);

        // Result should contain command_results matching the task's test commands
        assert!(
            !result.command_results.is_empty(),
            "command_results should not be empty for task '{}'",
            first.task_id
        );
        assert_eq!(
            result.task_id, first.task_id,
            "result task_id should match input task_id"
        );

        // Each command result should have basic invariants satisfied
        for (i, cmd_result) in result.command_results.iter().enumerate() {
            assert!(
                !cmd_result.command.is_empty(),
                "command_result[{}] should have a non-empty command",
                i
            );
            // status_code is populated for all commands; passed should match exit==0
            assert!(
                cmd_result.status_code.is_some(),
                "command_result[{}] should have a status_code",
                i
            );
        }
    }

    /// Smoke test: loads a real fixture from the local-smoke suite, constructs
    /// simulated agent output (FixtureCommandResult records), assembles a
    /// FixtureTaskResult, and asserts the result is coherent and non-empty.
    /// Exercises load_fixture_suite → data access → result construction.
    #[test]
    fn smoke_run_fixture_through_eval_pipeline() {
        // 1. Load a real fixture from the local-smoke suite
        let suite = load_fixture_suite("local-smoke").expect("should load fixture suite");

        // Pick a fixture with expected-files that reference actual source files
        let task = suite
            .tasks
            .iter()
            .find(|task| task.task_id == "context-failing-files")
            .expect("should find context-failing-files fixture");

        // Verify the fixture loaded correctly
        assert_eq!(task.task_id, "context-failing-files");
        assert!(!task.tests.is_empty(), "fixture should have test commands");
        assert!(
            !task.expected_files.is_empty(),
            "fixture should have expected files"
        );
        assert!(
            task.expected_files.iter().any(|f| f == "src/context.rs"),
            "expected_files should include src/context.rs"
        );
        assert_eq!(task.risk_label, "medium");
        assert_eq!(task.category, "context-miss challenge");

        // 2. Simulate agent output by constructing FixtureCommandResult records
        let simulated_commands = vec![
            FixtureCommandResult {
                command: task.tests[0].clone(),
                passed: true,
                status_code: Some(0),
                duration_ms: 1234,
                stdout_preview: "test result: ok. 2 passed; 0 failed".to_string(),
                stderr_preview: String::new(),
            },
            FixtureCommandResult {
                command: task.tests[1].clone(),
                passed: true,
                status_code: Some(0),
                duration_ms: 891,
                stdout_preview: "test result: ok. 1 passed; 0 failed".to_string(),
                stderr_preview: String::new(),
            },
        ];

        // 3. Assemble a FixtureTaskResult from the simulated output
        let result = FixtureTaskResult {
            task_id: task.task_id.clone(),
            passed: simulated_commands.iter().all(|cmd| cmd.passed),
            command_results: simulated_commands.clone(),
        };

        // 4. Assert the result is coherent and non-empty
        assert_eq!(result.task_id, task.task_id);
        assert!(result.passed, "all simulated commands passed");
        assert!(
            !result.command_results.is_empty(),
            "command_results should not be empty"
        );
        assert_eq!(result.command_results.len(), task.tests.len());

        // Each command result should satisfy basic invariants
        for (i, cmd_result) in result.command_results.iter().enumerate() {
            assert!(
                !cmd_result.command.is_empty(),
                "command_result[{}] should have a non-empty command",
                i
            );
            assert!(
                cmd_result.status_code.is_some(),
                "command_result[{}] should have a status_code",
                i
            );
            assert!(
                cmd_result.passed,
                "command_result[{}] should be passed in simulation",
                i
            );
            assert!(
                cmd_result.duration_ms > 0,
                "command_result[{}] should have non-zero duration",
                i
            );
            assert!(
                !cmd_result.stdout_preview.is_empty(),
                "command_result[{}] should have non-empty stdout_preview",
                i
            );
        }

        // 5. Verify that run_fixture_task on the real fixture also produces
        //    a well-formed result (exercises run_fixture_task → run_fixture_command)
        let real_result = run_fixture_task(task);
        assert_eq!(real_result.task_id, task.task_id);
        assert!(
            !real_result.command_results.is_empty(),
            "real fixture task should produce command results"
        );
        for cmd_result in &real_result.command_results {
            assert!(
                !cmd_result.command.is_empty(),
                "real command result should have a non-empty command"
            );
            assert!(
                cmd_result.status_code.is_some(),
                "real command result should have a status_code"
            );
        }
    }

    #[test]
    fn run_fixture_command_times_out_on_hanging_process() {
        // Spawn a process that sleeps for 999 seconds, with a 1-second timeout.
        // The result must be a failure and must return within 3 seconds wall time.
        let wall_start = Instant::now();
        let result = run_fixture_command("sleep 999", None, 1);
        let wall_elapsed = wall_start.elapsed().as_secs();

        assert!(!result.passed, "hanging command should not pass");
        assert!(
            result.stderr_preview.contains("timed out after 1s"),
            "stderr_preview should indicate timeout, got: {}",
            result.stderr_preview
        );
        assert!(
            wall_elapsed < 3,
            "timeout should return quickly, but wall time was {}s",
            wall_elapsed
        );
    }
}
