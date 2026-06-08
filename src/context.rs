//! Project context loading — file listing, git status, recently changed files.
//!
//! Extracted from `cli.rs` to keep context assembly separate from CLI argument parsing.

use crate::commands_project::{detect_project_type, project_type_hints};
use crate::format::{is_quiet, DIM, RESET};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Project instruction files, checked in order. All found files are concatenated.
///
/// YOYO.md is the canonical name for yoyo projects. The remaining entries are
/// compatibility aliases so that yoyo automatically picks up project instructions
/// written for other AI coding tools:
///
/// - **CLAUDE.md** — Claude Code
/// - **.yoyo/instructions.md** — yoyo alternate location
/// - **AGENTS.md** — Google Gemini CLI / generic agents
/// - **.cursorrules** — Cursor
/// - **.github/copilot-instructions.md** — GitHub Copilot
///
/// When a developer already has any of these in their project, yoyo reads them
/// at startup — no configuration needed.
pub const PROJECT_CONTEXT_FILES: &[&str] = &[
    "YOYO.md",
    "CLAUDE.md",
    ".yoyo/instructions.md",
    "AGENTS.md",
    ".cursorrules",
    ".github/copilot-instructions.md",
];

/// Maximum number of files to include in the project file listing.
pub const MAX_PROJECT_FILES: usize = 200;

/// Maximum number of recently changed files to include in context.
pub const MAX_RECENT_FILES: usize = 20;

pub const SEMANTIC_INDEX_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_SEMANTIC_INDEX_PATH: &str = ".yoyo/context-semantic-index.json";
pub const EMBEDDING_INDEX_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_EMBEDDING_INDEX_PATH: &str = ".yoyo/context-embedding-index.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeepSeekContextPhase {
    StablePrefix,
    DynamicSuffix,
}

impl DeepSeekContextPhase {
    pub fn label(&self) -> &'static str {
        match self {
            Self::StablePrefix => "stable_prefix",
            Self::DynamicSuffix => "dynamic_suffix",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepSeekContextBlock {
    pub name: String,
    pub phase: DeepSeekContextPhase,
    pub source: String,
    pub included: bool,
    pub reason: String,
    pub content: String,
}

impl DeepSeekContextBlock {
    pub fn token_estimate(&self) -> usize {
        estimate_tokens(&self.content)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepSeekContextPreview {
    pub layout_version: u32,
    pub blocks: Vec<DeepSeekContextBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistentSemanticIndex {
    pub schema_version: u32,
    pub generated_at_ms: u128,
    pub file_count: usize,
    pub files: Vec<PersistentSemanticIndexFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistentSemanticIndexFile {
    pub path: String,
    pub len_bytes: u64,
    pub modified_ms: u128,
    pub terms: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistentEmbeddingIndex {
    pub schema_version: u32,
    pub generated_at_ms: u128,
    pub files: Vec<PersistentEmbeddingIndexFile>,
    pub terms: Vec<PersistentEmbeddingIndexTerm>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistentEmbeddingIndexFile {
    pub path: String,
    pub len_bytes: u64,
    pub modified_ms: u128,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistentEmbeddingIndexTerm {
    pub term: String,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticIndexReport {
    pub path: PathBuf,
    pub file_count: usize,
    pub term_count: usize,
    pub written: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ContextIndexDiagnostics {
    lines: Vec<String>,
}

impl SemanticIndexReport {
    pub fn render(&self) -> String {
        format!(
            "DeepSeek context semantic index\n  path:    {}\n  files:   {}\n  terms:   {}\n  written: {}",
            self.path.display(),
            self.file_count,
            self.term_count,
            self.written
        )
    }

    pub fn payload(&self) -> Value {
        json!({
            "diagnostic": "deepseek_context_semantic_index",
            "schema_version": SEMANTIC_INDEX_SCHEMA_VERSION,
            "path": self.path.display().to_string(),
            "file_count": self.file_count,
            "term_count": self.term_count,
            "written": self.written,
        })
    }
}

impl DeepSeekContextPreview {
    pub fn render_prompt_suffix(&self) -> String {
        let mut out = String::new();
        for block in self.blocks.iter().filter(|block| block.included) {
            if !out.is_empty() {
                out.push_str("\n\n");
            }
            out.push_str(&format!(
                "# DeepSeek Context: {}\n\n{}",
                block.name, block.content
            ));
        }
        out
    }

    pub fn render_preview(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "DeepSeek context preview\nprompt version: deepseek_native_prompt@v{}\nlayout version: {}\n",
            self.layout_version, self.layout_version
        ));
        for block in &self.blocks {
            out.push_str(&format!(
                "  {:<14} {:<32} {:<8} tokens~{}\n",
                block.phase.label(),
                block.name,
                if block.included {
                    "included"
                } else {
                    "deferred"
                },
                block.token_estimate()
            ));
        }
        out.trim_end().to_string()
    }

    pub fn render_explain(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "DeepSeek context explain\nprompt version: deepseek_native_prompt@v{}\nlayout version: {}\n",
            self.layout_version, self.layout_version
        ));
        for block in &self.blocks {
            out.push_str(&format!(
                "\n{} ({})\n  status: {}\n  source: {}\n  reason: {}\n  tokens~{}",
                block.name,
                block.phase.label(),
                if block.included {
                    "included"
                } else {
                    "deferred"
                },
                block.source,
                block.reason,
                block.token_estimate()
            ));
            let item_lines = block_item_lines(block, 8);
            if !item_lines.is_empty() {
                out.push_str("\n  items:");
                for item in item_lines {
                    out.push_str(&format!("\n    - {item}"));
                }
            }
        }
        out.trim_end().to_string()
    }

    pub fn state_payload(&self) -> Value {
        self.state_payload_for_genome(&crate::deepseek::active_harness_genome())
    }

    fn state_payload_for_genome(&self, genome: &crate::deepseek::DeepSeekHarnessGenome) -> Value {
        let included_blocks = self
            .blocks
            .iter()
            .filter(|block| block.included)
            .collect::<Vec<_>>();
        json!({
            "context_policy": "deepseek_native",
            "prompt_version": format!("deepseek_native_prompt@v{}", self.layout_version),
            "prompt_contract_version": crate::deepseek::DEEPSEEK_PROMPT_CONTRACT_VERSION,
            "system_contract_version": crate::deepseek::DEEPSEEK_SYSTEM_CONTRACT_VERSION,
            "layout_version": self.layout_version,
            "tool_schema_version": crate::deepseek::STRICT_SCHEMA_VERSION,
            "strict_tool_schema_versions": crate::deepseek::strict_schema_versioned_names(),
            "include_instruction_files": genome.context_policy.include_instruction_files.clone(),
            "block_count": self.blocks.len(),
            "included_block_count": included_blocks.len(),
            "estimated_tokens": included_blocks
                .iter()
                .map(|block| block.token_estimate())
                .sum::<usize>(),
            "stable_prefix_blocks": self.blocks
                .iter()
                .filter(|block| block.phase == DeepSeekContextPhase::StablePrefix)
                .map(|block| block.name.as_str())
                .collect::<Vec<_>>(),
            "dynamic_suffix_blocks": self.blocks
                .iter()
                .filter(|block| block.phase == DeepSeekContextPhase::DynamicSuffix)
                .map(|block| block.name.as_str())
                .collect::<Vec<_>>(),
            "included_blocks": included_blocks
                .iter()
                .map(|block| json!({
                    "name": block.name,
                    "phase": block.phase.label(),
                    "source": block.source,
                    "reason": block.reason,
                    "estimated_tokens": block.token_estimate(),
                    "item_count": block_item_count(block),
                    "items_preview": block_item_lines(block, 8),
                }))
                .collect::<Vec<_>>(),
        })
    }
}

fn block_item_count(block: &DeepSeekContextBlock) -> usize {
    block
        .content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .count()
}

fn block_item_lines(block: &DeepSeekContextBlock, limit: usize) -> Vec<String> {
    if !block.included {
        return Vec::new();
    }
    block
        .content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(limit)
        .map(|line| truncate(line, 140))
        .collect()
}

/// Get a listing of project files using `git ls-files`.
/// Returns a newline-separated list of tracked files, capped at MAX_PROJECT_FILES.
/// Returns None if git is not available or the directory is not a git repo.
pub fn get_project_file_listing() -> Option<String> {
    let stdout = crate::git::run_git(&["ls-files"]).ok()?;
    let files: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    if files.is_empty() {
        return None;
    }
    let total = files.len();
    let capped: Vec<&str> = files.into_iter().take(MAX_PROJECT_FILES).collect();
    let mut listing = capped.join("\n");
    if total > MAX_PROJECT_FILES {
        listing.push_str(&format!(
            "\n... and {} more files",
            total - MAX_PROJECT_FILES
        ));
    }
    Some(listing)
}

/// Build and write the persistent semantic index if it does not already exist.
///
/// This is a one-time cost per clone/checkout.  Once the index file is on disk
/// subsequent calls are a no-op.
fn ensure_semantic_index() {
    let path = Path::new(DEFAULT_SEMANTIC_INDEX_PATH);
    if path.exists() {
        // Check freshness before deciding it's good enough.
        // If the index is stale (all files changed or >2/3 stale),
        // delete it so the build path below recreates it.
        if let Some(index) = load_persistent_semantic_index(path) {
            let (fresh, stale, _missing) = count_fresh_index_files(
                index
                    .files
                    .iter()
                    .map(|f| (&f.path, f.len_bytes, f.modified_ms)),
            );
            if fresh == 0 || stale > fresh * 2 {
                eprintln!(
                    "\x1b[33m  semantic index is stale (fresh={fresh} stale={stale}) — rebuilding\x1b[0m"
                );
                let _ = std::fs::remove_file(path);
                // Fall through to rebuild below
            } else {
                return;
            }
        } else {
            // Corrupt or schema-mismatched file — treat as needing rebuild
            eprintln!("\x1b[33m  semantic index is corrupt — rebuilding\x1b[0m");
            let _ = std::fs::remove_file(path);
        }
    }
    // Build-and-write with a 60-second timeout so a slow filesystem or huge
    // repository can't block the caller forever.
    let (tx, rx) = std::sync::mpsc::channel();
    let path_buf = path.to_path_buf();
    std::thread::spawn(move || {
        let result = build_and_maybe_write_semantic_index(&path_buf, true);
        let _ = tx.send(result);
    });
    match rx.recv_timeout(std::time::Duration::from_secs(60)) {
        Ok(Ok(_)) => {} // built and written
        Ok(Err(e)) => eprintln!("\x1b[33m  semantic index could not be built: {e}\x1b[0m"),
        Err(_timeout) => eprintln!(
            "\x1b[33m  semantic index build timed out after 60s — proceeding without index\x1b[0m"
        ),
    }
}

fn ensure_embedding_index() {
    let path = Path::new(DEFAULT_EMBEDDING_INDEX_PATH);
    if path.exists() {
        // Check freshness before deciding it's good enough.
        // If the index is stale (all files changed or >2/3 stale),
        // delete it and rebuild.
        if let Some(index) = load_persistent_embedding_index(path) {
            let (fresh, stale, _missing) = count_fresh_index_files(
                index
                    .files
                    .iter()
                    .map(|f| (&f.path, f.len_bytes, f.modified_ms)),
            );
            if fresh == 0 || stale > fresh * 2 {
                eprintln!(
                    "\x1b[33m  embedding index is stale (fresh={fresh} stale={stale}) — removing\x1b[0m"
                );
                let _ = std::fs::remove_file(path);
            } else {
                return;
            }
        } else {
            // Corrupt or schema-mismatched file — remove it
            eprintln!("\x1b[33m  embedding index is corrupt — removing\x1b[0m");
            let _ = std::fs::remove_file(path);
        }
    }
    // Build from semantic index using deterministic hash-based embeddings.
    // No external model required — term vectors are computed via hash projection
    // and file embeddings are the average of their constituent term vectors.
    let semantic_path = Path::new(DEFAULT_SEMANTIC_INDEX_PATH);
    let Some(semantic_index) = load_persistent_semantic_index(semantic_path) else {
        eprintln!("\x1b[33m  embedding index cannot be built — semantic index is missing\x1b[0m");
        return;
    };
    let index = build_persistent_embedding_index(&semantic_index);
    if index.files.is_empty() {
        eprintln!("\x1b[33m  embedding index is empty — no file embeddings computed\x1b[0m");
        return;
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(&index) {
        Ok(raw) => {
            if std::fs::write(path, format!("{raw}\n")).is_ok() {
                eprintln!(
                    "\x1b[32m  embedding index built: {} files, {} terms\x1b[0m",
                    index.files.len(),
                    index.terms.len()
                );
            } else {
                eprintln!(
                    "\x1b[33m  embedding index failed to write to {}\x1b[0m",
                    path.display()
                );
            }
        }
        Err(e) => {
            eprintln!("\x1b[33m  embedding index serialization failed: {e}\x1b[0m");
        }
    }
}

pub fn build_deepseek_context_preview() -> DeepSeekContextPreview {
    // Auto-build the semantic index on first access so that context
    // preview/explain don't time out building it on every call.  The
    // embedding index needs an external embedding model so we only
    // build the semantic index here.
    ensure_semantic_index();
    ensure_embedding_index();

    let genome = crate::deepseek::active_harness_genome();
    let recent_files =
        get_recently_changed_files(genome.context_policy.changed_file_limit as usize)
            .unwrap_or_default();
    let state_events = read_recent_state_events(20);
    let tracked_files = tracked_project_files().unwrap_or_default();
    let failing_files = select_failing_output_files(
        &state_events,
        &tracked_files,
        genome.context_policy.changed_file_limit as usize,
    );
    let ranked_files = rank_context_files(
        &state_events,
        &tracked_files,
        &recent_files,
        genome.context_policy.changed_file_limit as usize,
    );
    let index_diagnostics = context_index_diagnostics();
    let recent_event_lines = summarize_events(&state_events, 6);
    let failure_lines =
        summarize_failures(&state_events, genome.context_policy.recent_failure_limit);
    let project_instructions = load_instruction_context_for_files(
        genome
            .context_policy
            .include_instruction_files
            .iter()
            .map(String::as_str),
    );
    let repo_map = if genome.context_policy.include_repo_map {
        crate::commands_map::generate_repo_map_for_prompt()
    } else {
        None
    };
    let current_goal = crate::commands_goal::load_goal();

    let mut blocks = Vec::new();
    blocks.push(block(
        DeepSeekContextPhase::StablePrefix,
        "deepseek_native_system_contract",
        "src/deepseek.rs",
        true,
        "anchors DeepSeek-specific protocol and durable-reasoning policy",
        crate::deepseek::stable_system_contract().to_string(),
    ));
    blocks.push(block(
        DeepSeekContextPhase::StablePrefix,
        "safety_and_permissions",
        "harness_genome.permission_policy",
        true,
        "keeps permission-sensitive behavior stable for cache reuse",
        format!(
            "shell_requires_approval: {}\ndestructive_requires_explicit_approval: {}\npermission_policy_patches_need_human: {}",
            genome.permission_policy.shell_requires_approval,
            genome
                .permission_policy
                .destructive_requires_explicit_approval,
            genome
                .permission_policy
                .permission_policy_patches_need_human
        ),
    ));
    blocks.push(block(
        DeepSeekContextPhase::StablePrefix,
        "strict_tool_schemas",
        "deepseek.strict_schema_suite",
        true,
        "lists critical state/action schemas in deterministic order",
        crate::deepseek::strict_schema_versioned_names().join("\n"),
    ));
    blocks.push(block(
        DeepSeekContextPhase::StablePrefix,
        "harness_policy_version",
        "DeepSeekHarnessGenome",
        true,
        "pins the active model, context, JSON, FIM, test, repair, memory, and permission policy version",
        format!(
            "version: {}\nlayout_version: {}\ncache_stable_prefix: {}\ncache_record_metrics: {}\ncache_optimize_prompt_order: {}\nbenchmark_subset: {}\nrequired_gates:\n{}",
            genome.version,
            genome.prompt_layout_policy.version,
            genome.cache_policy.stable_prefix,
            genome.cache_policy.record_metrics,
            genome.cache_policy.optimize_prompt_order,
            genome.test_policy.benchmark_subset,
            genome
                .test_policy
                .required_gates
                .iter()
                .map(|gate| format!("- {gate}"))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    ));
    blocks.push(block(
        DeepSeekContextPhase::StablePrefix,
        "project_instructions",
        "YOYO.md/CLAUDE.md/AGENTS.md-compatible files",
        project_instructions.is_some(),
        "injects stable project guidance before task-specific evidence",
        project_instructions.unwrap_or_else(|| "No project instruction file found.".to_string()),
    ));
    blocks.push(block(
        DeepSeekContextPhase::StablePrefix,
        "repo_map",
        "commands_map::generate_repo_map_for_prompt",
        repo_map.is_some(),
        "keeps a stable structural map near the reusable prompt prefix",
        repo_map.unwrap_or_else(|| "No repository map available.".to_string()),
    ));
    blocks.push(block(
        DeepSeekContextPhase::DynamicSuffix,
        "current_task",
        "user prompt",
        false,
        "injected at prompt time so it does not disturb the stable prefix",
        String::new(),
    ));
    blocks.push(block(
        DeepSeekContextPhase::DynamicSuffix,
        "selected_recent_events",
        ".yoyo/state/events.jsonl",
        !recent_event_lines.is_empty(),
        "surfaces recent state only after stable policy blocks",
        recent_event_lines.join("\n"),
    ));
    blocks.push(block(
        DeepSeekContextPhase::DynamicSuffix,
        "failing_test_files",
        ".yoyo/state/events.jsonl + git ls-files",
        !failing_files.is_empty(),
        "prioritizes tracked files named in recent failing command or test output before generic recent files",
        failing_files.join("\n"),
    ));
    blocks.push(block(
        DeepSeekContextPhase::DynamicSuffix,
        "selected_files",
        ".yoyo/state/events.jsonl + git recent changes + .yoyo/context-semantic-index.json + .yoyo/context-embedding-index.json",
        !ranked_files.is_empty(),
        "ranks candidate files by recent failures, historical repair evidence, symbol hints, recent git changes, fresh persistent semantic index entries, and fresh embedding-index matches",
        ranked_context_file_lines(&ranked_files).join("\n"),
    ));
    blocks.push(block(
        DeepSeekContextPhase::DynamicSuffix,
        "context_index_status",
        ".yoyo/context-semantic-index.json + .yoyo/context-embedding-index.json",
        true,
        "explains whether persistent context indexes can influence selected_files ranking",
        index_diagnostics.lines.join("\n"),
    ));
    blocks.push(block(
        DeepSeekContextPhase::DynamicSuffix,
        "latest_tool_outputs",
        "conversation runtime",
        false,
        "available during active turns, not during static preview",
        String::new(),
    ));
    blocks.push(block(
        DeepSeekContextPhase::DynamicSuffix,
        "failure_evidence",
        ".yoyo/state/events.jsonl",
        !failure_lines.is_empty(),
        "keeps recent failures close to the task for repair and harness-patch evidence",
        failure_lines.join("\n"),
    ));
    blocks.push(block(
        DeepSeekContextPhase::DynamicSuffix,
        "current_goal",
        ".yoyo/goal.md",
        current_goal.is_some(),
        "keeps persistent objective awareness in the dynamic suffix",
        current_goal.unwrap_or_default(),
    ));
    blocks.push(block(
        DeepSeekContextPhase::DynamicSuffix,
        "current_budget",
        "DeepSeek context policy",
        true,
        "documents DeepSeek-native budget assumptions for context selection",
        format!(
            "context_window_tokens: {}\nmax_output_tokens: {}",
            crate::deepseek::CONTEXT_WINDOW_TOKENS,
            crate::deepseek::MAX_OUTPUT_TOKENS
        ),
    ));

    DeepSeekContextPreview {
        layout_version: genome.prompt_layout_policy.version,
        blocks,
    }
}

pub fn build_and_maybe_write_semantic_index(
    path: &Path,
    write: bool,
) -> Result<SemanticIndexReport, String> {
    let tracked_files = tracked_project_files().unwrap_or_default();
    let index = build_persistent_semantic_index(&tracked_files)?;
    let term_count = index.files.iter().map(|file| file.terms.len()).sum();
    if write {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "create semantic index directory '{}': {e}",
                    parent.display()
                )
            })?;
        }
        let raw = serde_json::to_string_pretty(&index)
            .map_err(|e| format!("serialize semantic index: {e}"))?;
        std::fs::write(path, format!("{raw}\n"))
            .map_err(|e| format!("write semantic index '{}': {e}", path.display()))?;
    }
    Ok(SemanticIndexReport {
        path: path.to_path_buf(),
        file_count: index.file_count,
        term_count,
        written: write,
    })
}

pub fn render_deepseek_native_context_for_prompt() -> String {
    let preview = build_deepseek_context_preview();
    crate::state::record(
        crate::state::EventType::ContextBuilt,
        crate::state::Actor::Harness,
        preview.state_payload(),
    );
    preview.render_prompt_suffix()
}

fn block(
    phase: DeepSeekContextPhase,
    name: &str,
    source: &str,
    included: bool,
    reason: &str,
    content: String,
) -> DeepSeekContextBlock {
    DeepSeekContextBlock {
        name: name.to_string(),
        phase,
        source: source.to_string(),
        included,
        reason: reason.to_string(),
        content,
    }
}

fn load_instruction_context_for_files<'a>(
    instruction_files: impl IntoIterator<Item = &'a str>,
) -> Option<String> {
    let mut context = String::new();
    let mut found = Vec::new();
    for name in instruction_files {
        if let Ok(content) = std::fs::read_to_string(name) {
            let content = content.trim();
            if !content.is_empty() {
                if !context.is_empty() {
                    context.push_str("\n\n");
                }
                if !found.is_empty() {
                    context.push_str(&format!("--- From {name} ---\n"));
                }
                context.push_str(content);
                found.push(name);
            }
        }
    }
    if context.is_empty() {
        None
    } else {
        Some(context)
    }
}

fn tracked_project_files() -> Option<Vec<String>> {
    let stdout = crate::git::run_git(&["ls-files"]).ok()?;
    let mut files = stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    files.sort_by_key(|path| std::cmp::Reverse(path.len()));
    Some(files)
}

fn select_failing_output_files(
    events: &[Value],
    tracked_files: &[String],
    limit: usize,
) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut selected = Vec::new();
    for event in events.iter().rev().filter(is_failure_or_test_event) {
        let Some(payload) = event.get("payload") else {
            continue;
        };
        for text in payload_text_fields(payload) {
            for path in extract_tracked_file_refs(&text, tracked_files) {
                if seen.insert(path.clone()) {
                    selected.push(path);
                    if selected.len() >= limit {
                        selected.reverse();
                        return selected;
                    }
                }
            }
        }
    }
    selected.reverse();
    selected
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScoredContextFile {
    path: String,
    score: i32,
    first_seen: usize,
    signals: Vec<String>,
}

fn rank_context_files(
    events: &[Value],
    tracked_files: &[String],
    recent_files: &[String],
    limit: usize,
) -> Vec<ScoredContextFile> {
    let mut entries: HashMap<String, ScoredContextFile> = HashMap::new();
    let mut next_seen = 0usize;

    for (idx, path) in recent_files.iter().enumerate() {
        let score = 20_i32.saturating_sub(idx as i32).max(1);
        add_context_file_score(&mut entries, &mut next_seen, path, score, "recent_change");
    }

    let ownership_rules = load_context_ownership_rules();
    let semantic_terms = semantic_terms_from_events(events);
    let semantic_index = load_persistent_semantic_index(Path::new(DEFAULT_SEMANTIC_INDEX_PATH));
    let embedding_index = load_persistent_embedding_index(Path::new(DEFAULT_EMBEDDING_INDEX_PATH));
    let query_embedding = context_query_embedding(&semantic_terms, embedding_index.as_ref());

    for event in events.iter().rev() {
        let Some(payload) = event.get("payload") else {
            continue;
        };
        let event_kind = event
            .get("event_type")
            .and_then(Value::as_str)
            .unwrap_or("");
        let failure_signal = event_is_context_failure_signal(event);
        let historical_signal = matches!(
            event_kind,
            "HypothesisCreated"
                | "PatchPromoted"
                | "PatchRejected"
                | "RevertPerformed"
                | "DecisionRecorded"
        );
        if !failure_signal && !historical_signal {
            continue;
        }

        for text in payload_text_fields(payload) {
            for path in extract_tracked_file_refs(&text, tracked_files) {
                add_context_file_score(
                    &mut entries,
                    &mut next_seen,
                    &path,
                    if failure_signal { 100 } else { 70 },
                    if failure_signal {
                        "failure_ref"
                    } else {
                        "historical_repair_ref"
                    },
                );
            }

            for path in tracked_files {
                if file_symbol_hint_matches(&text, path) {
                    add_context_file_score(
                        &mut entries,
                        &mut next_seen,
                        path,
                        if failure_signal { 35 } else { 25 },
                        "symbol_hint",
                    );
                }
            }
        }
    }

    for rule in &ownership_rules {
        for term in &semantic_terms {
            if ownership_rule_matches_term(rule, term) {
                for path in tracked_files
                    .iter()
                    .filter(|path| path_matches_owner_rule(path, rule))
                {
                    add_context_file_score(
                        &mut entries,
                        &mut next_seen,
                        path,
                        45,
                        "ownership_match",
                    );
                }
            }
        }
    }

    for path in tracked_files {
        let path_terms = semantic_terms_from_path(path);
        let overlap = path_terms
            .iter()
            .filter(|term| semantic_terms.contains(*term))
            .count();
        if overlap >= 2 {
            add_context_file_score(
                &mut entries,
                &mut next_seen,
                path,
                (overlap as i32).min(4) * 12,
                "semantic_match",
            );
        }

        let content_terms = semantic_terms_for_file(path, semantic_index.as_ref());
        let content_overlap = content_terms
            .iter()
            .filter(|term| semantic_terms.contains(*term))
            .count();
        if content_overlap >= 3 {
            add_context_file_score(
                &mut entries,
                &mut next_seen,
                path,
                (content_overlap as i32).min(5) * 8,
                "content_semantic_match",
            );
        }

        if let Some(query_embedding) = query_embedding.as_ref() {
            if let Some(file_embedding) =
                embedding_for_file(path, embedding_index.as_ref(), query_embedding.len())
            {
                let similarity = cosine_similarity(query_embedding, &file_embedding);
                if similarity >= 0.70 {
                    add_context_file_score(
                        &mut entries,
                        &mut next_seen,
                        path,
                        ((similarity * 50.0).round() as i32).max(1),
                        "embedding_match",
                    );
                }
            }
        }
    }

    let mut ranked = entries.into_values().collect::<Vec<_>>();
    ranked.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.first_seen.cmp(&b.first_seen))
            .then_with(|| a.path.cmp(&b.path))
    });
    ranked.truncate(limit);
    ranked
}

fn add_context_file_score(
    entries: &mut HashMap<String, ScoredContextFile>,
    next_seen: &mut usize,
    path: &str,
    score: i32,
    signal: &str,
) {
    let entry = entries.entry(path.to_string()).or_insert_with(|| {
        let first_seen = *next_seen;
        *next_seen += 1;
        ScoredContextFile {
            path: path.to_string(),
            score: 0,
            first_seen,
            signals: Vec::new(),
        }
    });
    entry.score += score;
    if !entry.signals.iter().any(|existing| existing == signal) {
        entry.signals.push(signal.to_string());
    }
}

fn file_symbol_hint_matches(text: &str, path: &str) -> bool {
    let Some(stem) = std::path::Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
    else {
        return false;
    };
    let normalized_stem = normalize_symbol_text(stem);
    if normalized_stem.len() < 5 || matches!(normalized_stem.as_str(), "main" | "lib" | "mod") {
        return false;
    }
    normalize_symbol_text(text).contains(&normalized_stem)
}

fn normalize_symbol_text(text: &str) -> String {
    text.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ContextOwnershipRule {
    pattern: String,
    owners: Vec<String>,
}

fn load_context_ownership_rules() -> Vec<ContextOwnershipRule> {
    [
        "CODEOWNERS",
        ".github/CODEOWNERS",
        "docs/CODEOWNERS",
        ".yoyo/CODEOWNERS",
    ]
    .iter()
    .find_map(|path| {
        std::fs::read_to_string(path)
            .ok()
            .map(|raw| parse_codeowners_rules(&raw))
    })
    .unwrap_or_default()
}

fn parse_codeowners_rules(raw: &str) -> Vec<ContextOwnershipRule> {
    raw.lines()
        .filter_map(|line| {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                return None;
            }
            let mut parts = line.split_whitespace();
            let pattern = parts.next()?.trim().to_string();
            let owners = parts
                .map(|owner| owner.trim_start_matches('@').to_ascii_lowercase())
                .filter(|owner| !owner.is_empty())
                .collect::<Vec<_>>();
            if owners.is_empty() {
                None
            } else {
                Some(ContextOwnershipRule { pattern, owners })
            }
        })
        .collect()
}

fn ownership_rule_matches_term(rule: &ContextOwnershipRule, term: &str) -> bool {
    rule.owners.iter().any(|owner| {
        split_semantic_terms(owner)
            .iter()
            .any(|owner_term| owner_term == term)
    }) || split_semantic_terms(&rule.pattern)
        .iter()
        .any(|pattern_term| pattern_term == term)
}

fn path_matches_owner_rule(path: &str, rule: &ContextOwnershipRule) -> bool {
    let pattern = rule.pattern.trim_start_matches('/');
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return path.starts_with(prefix.trim_start_matches('/'));
    }
    if pattern.ends_with('/') {
        return path.starts_with(pattern);
    }
    if pattern.contains('*') {
        let parts = pattern.split('*').collect::<Vec<_>>();
        let mut remaining = path;
        for part in parts.iter().filter(|part| !part.is_empty()) {
            let Some(idx) = remaining.find(part) else {
                return false;
            };
            remaining = &remaining[idx + part.len()..];
        }
        return true;
    }
    path == pattern || path.starts_with(&format!("{pattern}/")) || path.ends_with(pattern)
}

fn semantic_terms_from_events(events: &[Value]) -> BTreeSet<String> {
    let mut terms = BTreeSet::new();
    for event in events.iter().rev().take(12) {
        if let Some(payload) = event.get("payload") {
            for text in payload_text_fields(payload) {
                terms.extend(split_semantic_terms(&text));
            }
        }
    }
    expand_semantic_terms(terms)
}

fn semantic_terms_from_path(path: &str) -> BTreeSet<String> {
    expand_semantic_terms(
        split_semantic_terms(path)
            .into_iter()
            .filter(|term| !matches!(term.as_str(), "src" | "test" | "tests" | "rs" | "py" | "js"))
            .collect(),
    )
}

fn build_persistent_semantic_index(
    tracked_files: &[String],
) -> Result<PersistentSemanticIndex, String> {
    let mut files = Vec::new();
    for path in tracked_files {
        if !is_semantic_index_candidate(path) {
            continue;
        }
        let Some(metadata) = semantic_file_metadata(path)? else {
            continue;
        };
        if metadata.len_bytes > 128 * 1024 {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        let terms = expand_semantic_terms(split_semantic_terms(&content))
            .into_iter()
            .collect::<Vec<_>>();
        if terms.is_empty() {
            continue;
        }
        files.push(PersistentSemanticIndexFile {
            path: path.to_string(),
            len_bytes: metadata.len_bytes,
            modified_ms: metadata.modified_ms,
            terms,
        });
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(PersistentSemanticIndex {
        schema_version: SEMANTIC_INDEX_SCHEMA_VERSION,
        generated_at_ms: now_ms(),
        file_count: files.len(),
        files,
    })
}

fn hash_term_to_vector(term: &str, dimensions: usize) -> Vec<f32> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut vec = vec![0.0f32; dimensions];

    // Deterministic hash → pseudo-random unit vector via xorshift64.
    let mut hasher = DefaultHasher::new();
    term.hash(&mut hasher);
    let mut state: u64 = hasher.finish();

    for value in vec.iter_mut().take(dimensions) {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        // Map u64 → [-1.0, 1.0]
        *value = ((state as f64) / (u64::MAX as f64)) as f32 * 2.0 - 1.0;
    }

    // Normalise to unit length so cosine similarity is well-behaved.
    let norm: f32 = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in &mut vec {
            *v /= norm;
        }
    }

    vec
}

fn build_persistent_embedding_index(
    semantic_index: &PersistentSemanticIndex,
) -> PersistentEmbeddingIndex {
    const EMBEDDING_DIMENSIONS: usize = 128;

    // Collect every unique term and compute its hash-based embedding.
    let mut term_map: HashMap<String, Vec<f32>> = HashMap::new();
    for file in &semantic_index.files {
        for term in &file.terms {
            term_map
                .entry(term.clone())
                .or_insert_with(|| hash_term_to_vector(term, EMBEDDING_DIMENSIONS));
        }
    }

    let terms: Vec<PersistentEmbeddingIndexTerm> = term_map
        .into_iter()
        .map(|(term, embedding)| PersistentEmbeddingIndexTerm { term, embedding })
        .collect();

    // Build a fast lookup: term → &[f32] slice.
    let lookup: HashMap<&str, &[f32]> = terms
        .iter()
        .map(|t| (t.term.as_str(), t.embedding.as_slice()))
        .collect();

    // File embedding = average of its term vectors.
    let mut files = Vec::new();
    for file in &semantic_index.files {
        if file.terms.is_empty() {
            continue;
        }
        let vectors: Vec<&[f32]> = file
            .terms
            .iter()
            .filter_map(|t| lookup.get(t.as_str()).copied())
            .collect();
        let embedding = match average_embedding_vectors(&vectors) {
            Some(v) => v,
            None => continue,
        };
        files.push(PersistentEmbeddingIndexFile {
            path: file.path.clone(),
            len_bytes: file.len_bytes,
            modified_ms: file.modified_ms,
            embedding,
        });
    }

    PersistentEmbeddingIndex {
        schema_version: EMBEDDING_INDEX_SCHEMA_VERSION,
        generated_at_ms: now_ms(),
        files,
        terms,
    }
}

fn load_persistent_semantic_index(path: &Path) -> Option<PersistentSemanticIndex> {
    let raw = std::fs::read_to_string(path).ok()?;
    let index = serde_json::from_str::<PersistentSemanticIndex>(&raw).ok()?;
    (index.schema_version == SEMANTIC_INDEX_SCHEMA_VERSION).then_some(index)
}

fn load_persistent_embedding_index(path: &Path) -> Option<PersistentEmbeddingIndex> {
    let raw = std::fs::read_to_string(path).ok()?;
    let index = serde_json::from_str::<PersistentEmbeddingIndex>(&raw).ok()?;
    (index.schema_version == EMBEDDING_INDEX_SCHEMA_VERSION).then_some(index)
}

fn context_index_diagnostics() -> ContextIndexDiagnostics {
    ContextIndexDiagnostics {
        lines: vec![
            semantic_index_diagnostic_line(Path::new(DEFAULT_SEMANTIC_INDEX_PATH)),
            embedding_index_diagnostic_line(Path::new(DEFAULT_EMBEDDING_INDEX_PATH)),
        ],
    }
}

fn semantic_index_diagnostic_line(path: &Path) -> String {
    let label = "semantic_index";
    if !path.exists() {
        return format!("{label}: missing path={}", path.display());
    }
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(e) => return format!("{label}: unreadable path={} error={e}", path.display()),
    };
    let index = match serde_json::from_str::<PersistentSemanticIndex>(&raw) {
        Ok(index) => index,
        Err(e) => return format!("{label}: invalid_json path={} error={e}", path.display()),
    };
    if index.schema_version != SEMANTIC_INDEX_SCHEMA_VERSION {
        return format!(
            "{label}: unsupported_schema path={} expected={} actual={}",
            path.display(),
            SEMANTIC_INDEX_SCHEMA_VERSION,
            index.schema_version
        );
    }

    let (fresh, stale, missing) = count_fresh_index_files(
        index
            .files
            .iter()
            .map(|entry| (&entry.path, entry.len_bytes, entry.modified_ms)),
    );
    let status = index_status(index.files.len(), fresh, stale, missing);
    let term_count = index
        .files
        .iter()
        .map(|file| file.terms.len())
        .sum::<usize>();
    format!(
        "{label}: {status} files={} fresh={} stale={} missing={} terms={} path={}",
        index.files.len(),
        fresh,
        stale,
        missing,
        term_count,
        path.display()
    )
}

fn embedding_index_diagnostic_line(path: &Path) -> String {
    let label = "embedding_index";
    if !path.exists() {
        return format!("{label}: missing path={}", path.display());
    }
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(e) => return format!("{label}: unreadable path={} error={e}", path.display()),
    };
    let index = match serde_json::from_str::<PersistentEmbeddingIndex>(&raw) {
        Ok(index) => index,
        Err(e) => return format!("{label}: invalid_json path={} error={e}", path.display()),
    };
    if index.schema_version != EMBEDDING_INDEX_SCHEMA_VERSION {
        return format!(
            "{label}: unsupported_schema path={} expected={} actual={}",
            path.display(),
            EMBEDDING_INDEX_SCHEMA_VERSION,
            index.schema_version
        );
    }

    let (fresh, stale, missing) = count_fresh_index_files(
        index
            .files
            .iter()
            .map(|entry| (&entry.path, entry.len_bytes, entry.modified_ms)),
    );
    let status = index_status(index.files.len(), fresh, stale, missing);
    let dimensions = index
        .terms
        .first()
        .map(|term| term.embedding.len())
        .or_else(|| index.files.first().map(|file| file.embedding.len()))
        .unwrap_or(0);
    format!(
        "{label}: {status} files={} fresh={} stale={} missing={} terms={} dimensions={} path={}",
        index.files.len(),
        fresh,
        stale,
        missing,
        index.terms.len(),
        dimensions,
        path.display()
    )
}

fn count_fresh_index_files<'a>(
    files: impl Iterator<Item = (&'a String, u64, u128)>,
) -> (usize, usize, usize) {
    let mut fresh = 0;
    let mut stale = 0;
    let mut missing = 0;
    for (path, len_bytes, modified_ms) in files {
        match semantic_file_metadata(path) {
            Ok(Some(metadata))
                if metadata.len_bytes == len_bytes && metadata.modified_ms == modified_ms =>
            {
                fresh += 1;
            }
            Ok(Some(_)) | Err(_) => stale += 1,
            Ok(None) => missing += 1,
        }
    }
    (fresh, stale, missing)
}

fn index_status(total: usize, fresh: usize, stale: usize, missing: usize) -> &'static str {
    if total == 0 {
        "empty"
    } else if stale == 0 && missing == 0 {
        "fresh"
    } else if fresh > 0 {
        "partial"
    } else {
        "stale"
    }
}

fn context_query_embedding(
    terms: &BTreeSet<String>,
    index: Option<&PersistentEmbeddingIndex>,
) -> Option<Vec<f32>> {
    let index = index?;
    let vectors = terms
        .iter()
        .filter_map(|term| {
            index
                .terms
                .iter()
                .find(|entry| entry.term == *term)
                .map(|entry| entry.embedding.as_slice())
        })
        .collect::<Vec<_>>();
    average_embedding_vectors(&vectors)
}

fn embedding_for_file(
    path: &str,
    index: Option<&PersistentEmbeddingIndex>,
    expected_dimensions: usize,
) -> Option<Vec<f32>> {
    let index = index?;
    let entry = index.files.iter().find(|entry| entry.path == path)?;
    if entry.embedding.len() != expected_dimensions {
        return None;
    }
    let metadata = semantic_file_metadata(path).ok()??;
    if metadata.len_bytes != entry.len_bytes || metadata.modified_ms != entry.modified_ms {
        return None;
    }
    Some(entry.embedding.clone())
}

fn average_embedding_vectors(vectors: &[&[f32]]) -> Option<Vec<f32>> {
    let first = vectors.first()?;
    if first.is_empty() {
        return None;
    }
    let dimensions = first.len();
    if vectors.iter().any(|vector| vector.len() != dimensions) {
        return None;
    }
    let mut average = vec![0.0_f32; dimensions];
    for vector in vectors {
        for (idx, value) in vector.iter().enumerate() {
            average[idx] += *value;
        }
    }
    let count = vectors.len() as f32;
    for value in &mut average {
        *value /= count;
    }
    Some(average)
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.len() != right.len() || left.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0_f32;
    let mut left_norm = 0.0_f32;
    let mut right_norm = 0.0_f32;
    for (left_value, right_value) in left.iter().zip(right.iter()) {
        dot += left_value * right_value;
        left_norm += left_value * left_value;
        right_norm += right_value * right_value;
    }
    if left_norm == 0.0 || right_norm == 0.0 {
        return 0.0;
    }
    dot / (left_norm.sqrt() * right_norm.sqrt())
}

fn semantic_terms_for_file(
    path: &str,
    index: Option<&PersistentSemanticIndex>,
) -> BTreeSet<String> {
    if let Some(terms) = semantic_terms_from_persistent_index(path, index) {
        return terms;
    }
    semantic_terms_from_file_content(path)
}

fn semantic_terms_from_persistent_index(
    path: &str,
    index: Option<&PersistentSemanticIndex>,
) -> Option<BTreeSet<String>> {
    let index = index?;
    let entry = index.files.iter().find(|entry| entry.path == path)?;
    let metadata = semantic_file_metadata(path).ok()??;
    if metadata.len_bytes != entry.len_bytes || metadata.modified_ms != entry.modified_ms {
        return None;
    }
    Some(entry.terms.iter().cloned().collect())
}

fn semantic_terms_from_file_content(path: &str) -> BTreeSet<String> {
    if !is_semantic_index_candidate(path) {
        return BTreeSet::new();
    }
    let Ok(Some(metadata)) = semantic_file_metadata(path) else {
        return BTreeSet::new();
    };
    if metadata.len_bytes > 128 * 1024 {
        return BTreeSet::new();
    }
    let Ok(content) = std::fs::read_to_string(path) else {
        return BTreeSet::new();
    };
    expand_semantic_terms(split_semantic_terms(&content))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SemanticFileMetadata {
    len_bytes: u64,
    modified_ms: u128,
}

fn semantic_file_metadata(path: &str) -> Result<Option<SemanticFileMetadata>, String> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("read semantic index metadata for '{path}': {e}")),
    };
    if !metadata.is_file() {
        return Ok(None);
    }
    let modified_ms = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    Ok(Some(SemanticFileMetadata {
        len_bytes: metadata.len(),
        modified_ms,
    }))
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn is_semantic_index_candidate(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    if normalized.contains("/target/")
        || normalized.contains("/node_modules/")
        || normalized.contains("/.git/")
        || normalized.ends_with("Cargo.lock")
        || normalized.ends_with("package-lock.json")
    {
        return false;
    }
    matches!(
        std::path::Path::new(&normalized)
            .extension()
            .and_then(|extension| extension.to_str()),
        Some(
            "rs" | "md"
                | "toml"
                | "json"
                | "yaml"
                | "yml"
                | "txt"
                | "py"
                | "js"
                | "ts"
                | "tsx"
                | "jsx"
        )
    )
}

fn split_semantic_terms(text: &str) -> BTreeSet<String> {
    const STOPWORDS: &[&str] = &[
        "about", "after", "again", "because", "before", "build", "cargo", "change", "code",
        "command", "could", "error", "failed", "failure", "file", "fixed", "from", "into",
        "missing", "patch", "repair", "should", "state", "test", "tests", "that", "this", "with",
        "without",
    ];
    normalize_symbol_text(text)
        .split('_')
        .filter(|term| term.len() >= 4)
        .filter(|term| !STOPWORDS.contains(term))
        .map(|term| term.to_string())
        .collect()
}

fn expand_semantic_terms(mut terms: BTreeSet<String>) -> BTreeSet<String> {
    const ALIASES: &[(&str, &[&str])] = &[
        ("authentication", &["auth", "login", "session"]),
        ("authorization", &["authz", "permission", "policy"]),
        ("configuration", &["config", "settings"]),
        ("database", &["db", "storage"]),
        ("documentation", &["docs"]),
        ("evaluation", &["eval", "benchmark"]),
        ("repository", &["repo"]),
        ("serialization", &["serde", "json"]),
        ("synchronization", &["sync", "lock"]),
        ("transport", &["http", "network"]),
    ];

    let original = terms.iter().cloned().collect::<Vec<_>>();
    for term in original {
        for (canonical, aliases) in ALIASES {
            if term == *canonical || aliases.iter().any(|alias| term == *alias) {
                terms.insert((*canonical).to_string());
                terms.extend(aliases.iter().map(|alias| (*alias).to_string()));
            }
        }
    }
    terms
}

fn ranked_context_file_lines(files: &[ScoredContextFile]) -> Vec<String> {
    files
        .iter()
        .map(|file| {
            format!(
                "{} score={} signals={}",
                file.path,
                file.score,
                file.signals.join(",")
            )
        })
        .collect()
}

fn is_failure_or_test_event(event: &&Value) -> bool {
    event_is_context_failure_signal(event)
}

fn event_is_context_failure_signal(event: &Value) -> bool {
    let Some(kind) = event.get("event_type").and_then(|v| v.as_str()) else {
        return false;
    };
    if kind == "FailureObserved" {
        return true;
    }
    if !matches!(
        kind,
        "CommandCompleted" | "TestCompleted" | "PatchEvaluated"
    ) {
        return false;
    }
    event
        .get("payload")
        .map(payload_indicates_context_failure)
        .unwrap_or(false)
}

fn payload_indicates_context_failure(payload: &Value) -> bool {
    if payload
        .get("is_error")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return true;
    }
    if payload
        .get("failed")
        .and_then(Value::as_u64)
        .map(|failed| failed > 0)
        .unwrap_or(false)
    {
        return true;
    }
    for field in ["status_code", "exit_code"] {
        if payload
            .get(field)
            .and_then(Value::as_i64)
            .map(|code| code != 0)
            .unwrap_or(false)
        {
            return true;
        }
    }
    payload
        .get("status")
        .and_then(Value::as_str)
        .map(|status| matches!(status, "failed" | "failure" | "error" | "errored"))
        .unwrap_or(false)
}

fn payload_text_fields(payload: &Value) -> Vec<String> {
    let mut out = Vec::new();
    collect_payload_text(payload, &mut out);
    out
}

fn collect_payload_text(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(text) => out.push(text.clone()),
        Value::Array(values) => {
            for value in values {
                collect_payload_text(value, out);
            }
        }
        Value::Object(map) => {
            for (key, value) in map {
                if key.ends_with("preview")
                    || key.ends_with("output")
                    || key == "error"
                    || key == "failure_summary"
                    || key == "hypothesis"
                    || key == "summary"
                    || key == "reason"
                    || key == "rationale"
                    || key == "decision"
                    || key == "intent"
                    || key == "proposed_change"
                    || key == "stderr"
                    || key == "stdout"
                    || key == "metrics"
                    || key == "gates"
                {
                    collect_payload_text(value, out);
                }
            }
        }
        _ => {}
    }
}

fn extract_tracked_file_refs(text: &str, tracked_files: &[String]) -> Vec<String> {
    let normalized = text.replace('\\', "/");
    let mut matches = Vec::new();
    for path in tracked_files {
        if path.len() < 3 {
            continue;
        }
        if normalized.contains(path) || normalized.contains(&format!("./{path}")) {
            matches.push(path.clone());
        }
    }
    matches
}

fn read_recent_state_events(limit: usize) -> Vec<Value> {
    read_recent_state_events_from(std::path::Path::new(".yoyo/state/events.jsonl"), limit)
}

fn read_recent_state_events_from(path: &std::path::Path, limit: usize) -> Vec<Value> {
    let Ok(mut events) = crate::state::read_compatibility_events(path) else {
        return Vec::new();
    };
    if events.len() > limit {
        events = events.split_off(events.len() - limit);
    }
    events
}

fn summarize_events(events: &[Value], limit: usize) -> Vec<String> {
    events
        .iter()
        .rev()
        .take(limit)
        .filter_map(format_state_event_summary)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn summarize_failures(events: &[Value], limit: u32) -> Vec<String> {
    events
        .iter()
        .rev()
        .filter(|event| {
            event
                .get("event_type")
                .and_then(|v| v.as_str())
                .map(|kind| kind == "FailureObserved")
                .unwrap_or(false)
        })
        .take(limit as usize)
        .filter_map(format_state_event_summary)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn format_state_event_summary(event: &Value) -> Option<String> {
    let event_id = event.get("event_id").and_then(|v| v.as_str())?;
    let event_type = event.get("event_type").and_then(|v| v.as_str())?;
    let payload = event.get("payload").unwrap_or(&Value::Null);
    let summary = payload
        .get("failure_summary")
        .or_else(|| payload.get("error"))
        .or_else(|| payload.get("error_preview"))
        .or_else(|| payload.get("intent"))
        .or_else(|| payload.get("status"))
        .and_then(|v| v.as_str())
        .unwrap_or("-");
    Some(format!(
        "{event_id} {event_type}: {}",
        truncate(summary, 180)
    ))
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut out = text.chars().take(max_chars).collect::<String>();
    out.push_str("...");
    out
}

fn estimate_tokens(text: &str) -> usize {
    if text.trim().is_empty() {
        0
    } else {
        text.chars().count().div_ceil(4)
    }
}

/// Get a brief git status summary for system prompt injection.
/// Returns None if not in a git repo or git is unavailable.
pub fn get_git_status_context() -> Option<String> {
    let branch = crate::git::git_branch()?;

    let uncommitted = crate::git::run_git(&["status", "--porcelain"])
        .ok()
        .map(|s| s.lines().filter(|l| !l.is_empty()).count())
        .unwrap_or(0);

    let staged = crate::git::run_git(&["diff", "--cached", "--name-only"])
        .ok()
        .map(|s| s.lines().filter(|l| !l.is_empty()).count())
        .unwrap_or(0);

    let mut result = String::from("## Git Status\n\n");
    result.push_str(&format!("Branch: {branch}\n"));
    if uncommitted > 0 {
        result.push_str(&format!(
            "Uncommitted changes: {} file{}\n",
            uncommitted,
            if uncommitted == 1 { "" } else { "s" }
        ));
    }
    if staged > 0 {
        result.push_str(&format!(
            "Staged: {} file{}\n",
            staged,
            if staged == 1 { "" } else { "s" }
        ));
    }

    Some(result)
}

/// Get the most recently changed files from git log, deduplicated.
/// Returns up to `max_files` unique file paths that were modified in recent commits.
/// Returns None if not in a git repo or git is unavailable.
pub fn get_recently_changed_files(max_files: usize) -> Option<Vec<String>> {
    let stdout = crate::git::run_git(&[
        "log",
        "--diff-filter=M",
        "--name-only",
        "--pretty=format:",
        "-n",
        "20",
    ])
    .ok()?;
    let mut seen = std::collections::HashSet::new();
    let files: Vec<String> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .filter(|l| seen.insert(l.to_string()))
        .take(max_files)
        .map(|l| l.to_string())
        .collect();
    if files.is_empty() {
        None
    } else {
        Some(files)
    }
}

/// Load project context from instruction files (YOYO.md, CLAUDE.md, AGENTS.md,
/// .cursorrules, .github/copilot-instructions.md, etc.).
/// When multiple instruction files are found, each section is labeled with its
/// origin so the model knows which file each block came from.
/// Appends project file listing, recently changed files, git status, and memories
/// when available.
pub fn load_project_context() -> Option<String> {
    let mut context = String::new();
    let mut found = Vec::new();
    for name in PROJECT_CONTEXT_FILES {
        if let Ok(content) = std::fs::read_to_string(name) {
            let content = content.trim();
            if !content.is_empty() {
                if !context.is_empty() {
                    context.push_str("\n\n");
                }
                // When loading multiple files, label each section so the model
                // knows where the instructions came from.
                if !found.is_empty() {
                    context.push_str(&format!("--- From {name} ---\n"));
                }
                context.push_str(content);
                found.push(*name);
            }
        }
    }

    // Append project file listing if available
    if let Some(file_listing) = get_project_file_listing() {
        if !context.is_empty() {
            context.push_str("\n\n");
        }
        context.push_str("## Project Files\n\n");
        context.push_str(&file_listing);
        if found.is_empty() && !is_quiet() {
            // Even without context files, file listing alone is useful
            eprintln!("{DIM}  context: project file listing{RESET}");
        }
    }

    // Append recently changed files if available
    if let Some(recent_files) = get_recently_changed_files(MAX_RECENT_FILES) {
        if !context.is_empty() {
            context.push_str("\n\n");
        }
        context.push_str("## Recently Changed Files\n\n");
        context.push_str(&recent_files.join("\n"));
    }

    // Append git status if available
    let git_branch_name = if let Some(git_status) = get_git_status_context() {
        if !context.is_empty() {
            context.push_str("\n\n");
        }
        let branch = crate::git::git_branch();
        context.push_str(&git_status);
        branch
    } else {
        None
    };

    // Append project-type conventions (always, regardless of context files —
    // conventions complement explicit instructions rather than replacing them)
    let mut conventions_injected = false;
    let project_type = detect_project_type(std::path::Path::new("."));
    if let Some(hints) = project_type_hints(&project_type) {
        if !context.is_empty() {
            context.push_str("\n\n");
        }
        context.push_str("## Development Conventions\n\n");
        context.push_str(&hints);
        conventions_injected = true;
    }

    // Append project memories if available
    let memory = crate::memory::load_memories();
    if let Some(memories_section) = crate::memory::format_memories_for_prompt(&memory) {
        if !context.is_empty() {
            context.push_str("\n\n");
        }
        context.push_str(&memories_section);
    }

    if found.is_empty() && context.is_empty() {
        None
    } else {
        if !is_quiet() {
            for name in &found {
                eprintln!("{DIM}  context: {name}{RESET}");
            }
            if conventions_injected {
                eprintln!("{DIM}  context: {project_type} conventions{RESET}");
            }
            if context.contains("## Recently Changed Files") {
                eprintln!("{DIM}  context: recently changed files{RESET}");
            }
            if let Some(branch) = &git_branch_name {
                eprintln!("{DIM}  context: git status (branch: {branch}){RESET}");
            }
            if !memory.entries.is_empty() {
                eprintln!(
                    "{DIM}  context: {} project memories{RESET}",
                    memory.entries.len()
                );
            }
        }
        Some(context)
    }
}

/// List which project context files exist and their sizes.
/// Returns a vec of (filename, line_count) for display by /context.
pub fn list_project_context_files() -> Vec<(&'static str, usize)> {
    let mut result = Vec::new();
    for name in PROJECT_CONTEXT_FILES {
        if let Ok(content) = std::fs::read_to_string(name) {
            let content = content.trim();
            if !content.is_empty() {
                let lines = content.lines().count();
                result.push((*name, lines));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use serial_test::serial;

    #[test]
    fn test_project_context_file_names_not_empty() {
        assert_eq!(PROJECT_CONTEXT_FILES.len(), 6);
        // YOYO.md must be first — it's the canonical context file name
        assert_eq!(PROJECT_CONTEXT_FILES[0], "YOYO.md");
        // CLAUDE.md is a compatibility alias
        assert_eq!(PROJECT_CONTEXT_FILES[1], "CLAUDE.md");
        assert_eq!(PROJECT_CONTEXT_FILES[2], ".yoyo/instructions.md");
        // Cross-tool compatibility files
        assert_eq!(PROJECT_CONTEXT_FILES[3], "AGENTS.md");
        assert_eq!(PROJECT_CONTEXT_FILES[4], ".cursorrules");
        assert_eq!(PROJECT_CONTEXT_FILES[5], ".github/copilot-instructions.md");
        for name in PROJECT_CONTEXT_FILES {
            assert!(!name.is_empty());
        }
    }

    #[test]
    fn deepseek_context_preview_keeps_stable_blocks_before_dynamic_blocks() {
        let preview = build_deepseek_context_preview();
        let first_dynamic = preview
            .blocks
            .iter()
            .position(|block| block.phase == DeepSeekContextPhase::DynamicSuffix)
            .expect("dynamic suffix block");
        assert!(preview.blocks[..first_dynamic]
            .iter()
            .all(|block| block.phase == DeepSeekContextPhase::StablePrefix));
        assert_eq!(preview.blocks[0].name, "deepseek_native_system_contract");
        assert!(preview
            .blocks
            .iter()
            .any(|block| block.name == "strict_tool_schemas"));
        let schema_block = preview
            .blocks
            .iter()
            .find(|block| block.name == "strict_tool_schemas")
            .unwrap();
        assert!(schema_block.content.contains("propose_harness_patch@v1"));
    }

    #[test]
    fn deepseek_context_preview_renders_explanations() {
        let preview = DeepSeekContextPreview {
            layout_version: 7,
            blocks: vec![DeepSeekContextBlock {
                name: "test_block".to_string(),
                phase: DeepSeekContextPhase::StablePrefix,
                source: "unit-test".to_string(),
                included: true,
                reason: "prove rendering".to_string(),
                content: "hello world".to_string(),
            }],
        };
        assert!(preview.render_preview().contains("test_block"));
        assert!(preview
            .render_preview()
            .contains("prompt version: deepseek_native_prompt@v7"));
        let explain = preview.render_explain();
        assert!(explain.contains("prompt version: deepseek_native_prompt@v7"));
        assert!(explain.contains("source: unit-test"));
        assert!(explain.contains("reason: prove rendering"));
        assert!(explain.contains("items:"));
        assert!(explain.contains("- hello world"));
        assert!(preview
            .render_prompt_suffix()
            .contains("# DeepSeek Context: test_block"));
    }

    #[test]
    fn deepseek_context_explain_lists_selected_items_compactly() {
        let preview = DeepSeekContextPreview {
            layout_version: 7,
            blocks: vec![DeepSeekContextBlock {
                name: "failing_test_files".to_string(),
                phase: DeepSeekContextPhase::DynamicSuffix,
                source: ".yoyo/state/events.jsonl + git ls-files".to_string(),
                included: true,
                reason: "prioritizes tracked files named in recent failing output".to_string(),
                content: "src/context.rs\nsrc/commands_eval.rs\n".to_string(),
            }],
        };

        let explain = preview.render_explain();

        assert!(explain.contains("failing_test_files"));
        assert!(explain.contains("- src/context.rs"));
        assert!(explain.contains("- src/commands_eval.rs"));
    }

    #[test]
    fn deepseek_context_state_payload_records_policy_blocks() {
        let preview = DeepSeekContextPreview {
            layout_version: 7,
            blocks: vec![
                DeepSeekContextBlock {
                    name: "stable".to_string(),
                    phase: DeepSeekContextPhase::StablePrefix,
                    source: "unit-test".to_string(),
                    included: true,
                    reason: "stable reason".to_string(),
                    content: "hello world".to_string(),
                },
                DeepSeekContextBlock {
                    name: "dynamic".to_string(),
                    phase: DeepSeekContextPhase::DynamicSuffix,
                    source: "state".to_string(),
                    included: false,
                    reason: "deferred".to_string(),
                    content: "later".to_string(),
                },
            ],
        };

        let mut genome = crate::deepseek::DeepSeekHarnessGenome::default();
        genome.context_policy.include_instruction_files =
            vec!["YOYO.md".to_string(), "TEAM.md".to_string()];

        let payload = preview.state_payload_for_genome(&genome);

        assert_eq!(payload["context_policy"], "deepseek_native");
        assert_eq!(payload["prompt_version"], "deepseek_native_prompt@v7");
        assert_eq!(payload["prompt_contract_version"], 2);
        assert_eq!(
            payload["system_contract_version"],
            "deepseek_native_contract@v2"
        );
        assert_eq!(payload["layout_version"], 7);
        assert_eq!(payload["tool_schema_version"], 1);
        assert!(payload["strict_tool_schema_versions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("propose_harness_patch@v1")));
        assert_eq!(
            payload["include_instruction_files"],
            json!(["YOYO.md", "TEAM.md"])
        );
        assert_eq!(payload["block_count"], 2);
        assert_eq!(payload["included_block_count"], 1);
        assert_eq!(payload["stable_prefix_blocks"][0], "stable");
        assert_eq!(payload["dynamic_suffix_blocks"][0], "dynamic");
        assert_eq!(payload["included_blocks"][0]["name"], "stable");
        assert_eq!(payload["included_blocks"][0]["phase"], "stable_prefix");
        assert_eq!(payload["included_blocks"][0]["item_count"], 1);
        assert_eq!(
            payload["included_blocks"][0]["items_preview"][0],
            "hello world"
        );
    }

    #[test]
    #[serial]
    fn default_prompt_layout_policy_matches_rendered_context_blocks() {
        let dir = tempfile::tempdir().unwrap();
        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let preview = build_deepseek_context_preview();

        std::env::set_current_dir(old_dir).unwrap();
        let genome = crate::deepseek::DeepSeekHarnessGenome::default();
        let stable_blocks = preview
            .blocks
            .iter()
            .filter(|block| block.phase == DeepSeekContextPhase::StablePrefix)
            .map(|block| block.name.clone())
            .collect::<Vec<_>>();
        let dynamic_blocks = preview
            .blocks
            .iter()
            .filter(|block| block.phase == DeepSeekContextPhase::DynamicSuffix)
            .map(|block| block.name.clone())
            .collect::<Vec<_>>();

        assert_eq!(
            stable_blocks, genome.prompt_layout_policy.stable_prefix_blocks,
            "default stable-prefix policy must match rendered DeepSeek context blocks"
        );
        assert_eq!(
            dynamic_blocks, genome.prompt_layout_policy.dynamic_suffix_blocks,
            "default dynamic-suffix policy must match rendered DeepSeek context blocks"
        );
    }

    #[test]
    fn extracts_tracked_files_from_failure_output() {
        let tracked = vec![
            "src/context.rs".to_string(),
            "src/main.rs".to_string(),
            "README.md".to_string(),
        ];
        let text =
            "error[E0425]: cannot find value\n --> src/context.rs:42:9\nnote: see ./src/main.rs";
        let refs = extract_tracked_file_refs(text, &tracked);
        assert_eq!(refs, vec!["src/context.rs", "src/main.rs"]);
    }

    #[test]
    fn selects_failing_files_from_recent_state_events() {
        let events = vec![
            json!({
                "event_type": "CommandCompleted",
                "payload": {
                    "is_error": true,
                    "result_preview": "error: failed at src/old.rs:1"
                }
            }),
            json!({
                "event_type": "FailureObserved",
                "payload": {
                    "error_preview": "thread failed at tests/context_layout.rs:12"
                }
            }),
        ];
        let tracked = vec![
            "src/old.rs".to_string(),
            "tests/context_layout.rs".to_string(),
        ];

        let selected = select_failing_output_files(&events, &tracked, 4);
        assert_eq!(selected, vec!["src/old.rs", "tests/context_layout.rs"]);
    }

    #[test]
    fn ignores_success_events_when_selecting_failing_context_files() {
        let events = vec![
            json!({
                "event_type": "CommandCompleted",
                "payload": {
                    "status": "passed",
                    "status_code": 0,
                    "stdout": "checked src/alpha.rs"
                }
            }),
            json!({
                "event_type": "PatchEvaluated",
                "payload": {
                    "status": "passed",
                    "failed": 0,
                    "summary": "candidate touched src/beta.rs"
                }
            }),
            json!({
                "event_type": "PatchEvaluated",
                "payload": {
                    "status": "failed",
                    "failed": 1,
                    "summary": "candidate failed in src/real_failure.rs"
                }
            }),
        ];
        let tracked = vec![
            "src/alpha.rs".to_string(),
            "src/beta.rs".to_string(),
            "src/real_failure.rs".to_string(),
        ];

        let selected = select_failing_output_files(&events, &tracked, 4);

        assert_eq!(selected, vec!["src/real_failure.rs"]);
    }

    #[test]
    fn ranks_context_files_by_failure_history_symbols_and_recent_changes() {
        let events = vec![
            json!({
                "event_type": "HypothesisCreated",
                "payload": {
                    "summary": "retry_state context was missing during repair",
                    "evidence_event_ids": ["evt-failure"]
                }
            }),
            json!({
                "event_type": "FailureObserved",
                "payload": {
                    "error_preview": "thread failed at tests/retry_state_test.rs:12 because retry_state did not reset"
                }
            }),
            json!({
                "event_type": "DecisionRecorded",
                "payload": {
                    "decision": "promote",
                    "reason": "patch modified src/retry_state.rs and fixed the failure class"
                }
            }),
        ];
        let tracked = vec![
            "src/retry_state.rs".to_string(),
            "tests/retry_state_test.rs".to_string(),
            "src/unrelated.rs".to_string(),
        ];
        let recent = vec![
            "src/unrelated.rs".to_string(),
            "src/retry_state.rs".to_string(),
        ];

        let ranked = rank_context_files(&events, &tracked, &recent, 3);

        assert_eq!(ranked[0].path, "src/retry_state.rs");
        assert!(ranked[0].score > ranked[1].score);
        assert!(ranked[0]
            .signals
            .contains(&"historical_repair_ref".to_string()));
        assert!(ranked[0].signals.contains(&"symbol_hint".to_string()));
        assert!(ranked[0].signals.contains(&"recent_change".to_string()));
        assert!(ranked
            .iter()
            .any(|file| file.path == "tests/retry_state_test.rs"
                && file.signals.contains(&"failure_ref".to_string())));
        assert!(ranked_context_file_lines(&ranked)[0].contains("score="));
    }

    #[test]
    fn ranks_only_failed_patch_evals_as_failure_context_evidence() {
        let events = vec![
            json!({
                "event_type": "PatchEvaluated",
                "payload": {
                    "status": "passed",
                    "failed": 0,
                    "summary": "green eval mentioned src/alpha.rs"
                }
            }),
            json!({
                "event_type": "PatchEvaluated",
                "payload": {
                    "status": "failed",
                    "failed": 1,
                    "summary": "red eval failed at src/real_failure.rs"
                }
            }),
        ];
        let tracked = vec![
            "src/alpha.rs".to_string(),
            "src/real_failure.rs".to_string(),
        ];

        let ranked = rank_context_files(&events, &tracked, &[], 4);

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].path, "src/real_failure.rs");
        assert!(ranked[0].signals.contains(&"failure_ref".to_string()));
    }

    #[test]
    #[serial]
    fn ranks_context_files_by_ownership_and_semantic_relevance() {
        let dir = tempfile::tempdir().unwrap();
        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        std::fs::write(
            "CODEOWNERS",
            "/src/context/ @context-team\n/src/billing/ @billing-team\n",
        )
        .unwrap();
        let events = vec![json!({
            "event_type": "FailureObserved",
            "payload": {
                "error_preview": "context team needs semantic routing for prompt layout relevance"
            }
        })];
        let tracked = vec![
            "src/context/semantic_routing.rs".to_string(),
            "src/context/layout.rs".to_string(),
            "src/billing/invoice.rs".to_string(),
        ];

        let ranked = rank_context_files(&events, &tracked, &[], 3);

        std::env::set_current_dir(old_dir).unwrap();
        assert_eq!(ranked[0].path, "src/context/semantic_routing.rs");
        assert!(ranked[0].signals.contains(&"ownership_match".to_string()));
        assert!(ranked[0].signals.contains(&"semantic_match".to_string()));
        assert!(ranked[0].score > ranked[1].score);
    }

    #[test]
    fn ranks_context_files_with_semantic_aliases() {
        let events = vec![json!({
            "event_type": "FailureObserved",
            "payload": {
                "error_preview": "authentication flow drops OAuth session configuration"
            }
        })];
        let tracked = vec![
            "src/auth/session.rs".to_string(),
            "src/config/session.rs".to_string(),
            "src/payments/invoice.rs".to_string(),
        ];

        let ranked = rank_context_files(&events, &tracked, &[], 3);

        assert_eq!(ranked[0].path, "src/auth/session.rs");
        assert!(ranked[0].signals.contains(&"semantic_match".to_string()));
        assert!(ranked
            .iter()
            .any(|file| file.path == "src/config/session.rs"
                && file.signals.contains(&"semantic_match".to_string())));
        assert!(!ranked
            .iter()
            .any(|file| file.path == "src/payments/invoice.rs"));
    }

    #[test]
    #[serial]
    fn ranks_context_files_with_content_semantic_index() {
        let dir = tempfile::tempdir().unwrap();
        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        std::fs::create_dir_all("src").unwrap();
        std::fs::write(
            "src/runtime.rs",
            r#"
            // Handles cache eviction deadlines for scheduler fairness.
            fn rebalance_retry_deadline() {}
            "#,
        )
        .unwrap();
        std::fs::write(
            "src/payments.rs",
            r#"
            fn render_invoice_total() {}
            "#,
        )
        .unwrap();
        let events = vec![json!({
            "event_type": "FailureObserved",
            "payload": {
                "error_preview": "cache eviction deadline caused scheduler fairness regression"
            }
        })];
        let tracked = vec!["src/payments.rs".to_string(), "src/runtime.rs".to_string()];

        let ranked = rank_context_files(&events, &tracked, &[], 2);

        std::env::set_current_dir(old_dir).unwrap();
        assert_eq!(ranked[0].path, "src/runtime.rs");
        assert!(ranked[0]
            .signals
            .contains(&"content_semantic_match".to_string()));
        assert!(!ranked.iter().any(|file| file.path == "src/payments.rs"
            && file.signals.contains(&"content_semantic_match".to_string())));
    }

    #[test]
    #[serial]
    fn persistent_semantic_index_records_terms_and_rejects_stale_entries() {
        let dir = tempfile::tempdir().unwrap();
        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        std::fs::create_dir_all("src").unwrap();
        std::fs::write(
            "src/runtime.rs",
            "cache eviction deadline scheduler fairness",
        )
        .unwrap();
        let tracked = vec!["src/runtime.rs".to_string()];

        let index = build_persistent_semantic_index(&tracked).unwrap();
        let terms = semantic_terms_from_persistent_index("src/runtime.rs", Some(&index)).unwrap();
        assert!(terms.contains("cache"));
        assert!(terms.contains("scheduler"));

        let mut stale = index.clone();
        stale.files[0].len_bytes += 1;
        assert!(semantic_terms_from_persistent_index("src/runtime.rs", Some(&stale)).is_none());

        std::env::set_current_dir(old_dir).unwrap();
    }

    #[test]
    #[serial]
    fn ranks_context_files_with_fresh_embedding_index() {
        let dir = tempfile::tempdir().unwrap();
        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        std::fs::create_dir_all("src").unwrap();
        std::fs::create_dir_all(".yoyo").unwrap();
        std::fs::write("src/runtime.rs", "fn runtime_scheduler() {}\n").unwrap();
        std::fs::write("src/payments.rs", "fn render_invoice() {}\n").unwrap();
        let runtime_metadata = semantic_file_metadata("src/runtime.rs").unwrap().unwrap();
        let payments_metadata = semantic_file_metadata("src/payments.rs").unwrap().unwrap();
        let index = PersistentEmbeddingIndex {
            schema_version: EMBEDDING_INDEX_SCHEMA_VERSION,
            generated_at_ms: now_ms(),
            terms: vec![PersistentEmbeddingIndexTerm {
                term: "fairness".to_string(),
                embedding: vec![1.0, 0.0],
            }],
            files: vec![
                PersistentEmbeddingIndexFile {
                    path: "src/runtime.rs".to_string(),
                    len_bytes: runtime_metadata.len_bytes,
                    modified_ms: runtime_metadata.modified_ms,
                    embedding: vec![0.95, 0.05],
                },
                PersistentEmbeddingIndexFile {
                    path: "src/payments.rs".to_string(),
                    len_bytes: payments_metadata.len_bytes,
                    modified_ms: payments_metadata.modified_ms,
                    embedding: vec![0.0, 1.0],
                },
            ],
        };
        std::fs::write(
            DEFAULT_EMBEDDING_INDEX_PATH,
            serde_json::to_string_pretty(&index).unwrap(),
        )
        .unwrap();
        let events = vec![json!({
            "event_type": "FailureObserved",
            "payload": {
                "error_preview": "fairness regression"
            }
        })];
        let tracked = vec!["src/payments.rs".to_string(), "src/runtime.rs".to_string()];

        let ranked = rank_context_files(&events, &tracked, &[], 2);

        std::env::set_current_dir(old_dir).unwrap();
        assert_eq!(ranked[0].path, "src/runtime.rs");
        assert!(ranked[0].signals.contains(&"embedding_match".to_string()));
        assert!(!ranked.iter().any(|file| file.path == "src/payments.rs"
            && file.signals.contains(&"embedding_match".to_string())));
    }

    #[test]
    #[serial]
    fn context_index_diagnostics_report_fresh_stale_and_missing_entries() {
        let dir = tempfile::tempdir().unwrap();
        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        std::fs::create_dir_all("src").unwrap();
        std::fs::create_dir_all(".yoyo").unwrap();
        std::fs::write("src/runtime.rs", "fn runtime_scheduler() {}\n").unwrap();
        std::fs::write("src/stale.rs", "fn old_context() {}\n").unwrap();
        let runtime_metadata = semantic_file_metadata("src/runtime.rs").unwrap().unwrap();
        let stale_metadata = semantic_file_metadata("src/stale.rs").unwrap().unwrap();

        let semantic_index = PersistentSemanticIndex {
            schema_version: SEMANTIC_INDEX_SCHEMA_VERSION,
            generated_at_ms: now_ms(),
            file_count: 2,
            files: vec![
                PersistentSemanticIndexFile {
                    path: "src/runtime.rs".to_string(),
                    len_bytes: runtime_metadata.len_bytes,
                    modified_ms: runtime_metadata.modified_ms,
                    terms: vec!["runtime".to_string(), "scheduler".to_string()],
                },
                PersistentSemanticIndexFile {
                    path: "src/missing.rs".to_string(),
                    len_bytes: 10,
                    modified_ms: 1,
                    terms: vec!["missing".to_string()],
                },
            ],
        };
        let embedding_index = PersistentEmbeddingIndex {
            schema_version: EMBEDDING_INDEX_SCHEMA_VERSION,
            generated_at_ms: now_ms(),
            terms: vec![PersistentEmbeddingIndexTerm {
                term: "scheduler".to_string(),
                embedding: vec![1.0, 0.0],
            }],
            files: vec![
                PersistentEmbeddingIndexFile {
                    path: "src/runtime.rs".to_string(),
                    len_bytes: runtime_metadata.len_bytes,
                    modified_ms: runtime_metadata.modified_ms,
                    embedding: vec![0.95, 0.05],
                },
                PersistentEmbeddingIndexFile {
                    path: "src/stale.rs".to_string(),
                    len_bytes: stale_metadata.len_bytes + 1,
                    modified_ms: stale_metadata.modified_ms,
                    embedding: vec![0.1, 0.9],
                },
            ],
        };
        std::fs::write(
            DEFAULT_SEMANTIC_INDEX_PATH,
            serde_json::to_string_pretty(&semantic_index).unwrap(),
        )
        .unwrap();
        std::fs::write(
            DEFAULT_EMBEDDING_INDEX_PATH,
            serde_json::to_string_pretty(&embedding_index).unwrap(),
        )
        .unwrap();

        let diagnostics = context_index_diagnostics();

        std::env::set_current_dir(old_dir).unwrap();
        assert_eq!(diagnostics.lines.len(), 2);
        assert!(diagnostics.lines[0].contains("semantic_index: partial"));
        assert!(diagnostics.lines[0].contains("fresh=1"));
        assert!(diagnostics.lines[0].contains("missing=1"));
        assert!(diagnostics.lines[1].contains("embedding_index: partial"));
        assert!(diagnostics.lines[1].contains("fresh=1"));
        assert!(diagnostics.lines[1].contains("stale=1"));
        assert!(diagnostics.lines[1].contains("dimensions=2"));
    }

    #[test]
    #[serial]
    fn writes_persistent_semantic_index_report() {
        let dir = tempfile::tempdir().unwrap();
        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        std::fs::create_dir_all("src").unwrap();
        std::fs::write("src/runtime.rs", "transport retry timeout cache metrics").unwrap();
        let tracked = vec!["src/runtime.rs".to_string()];
        let index = build_persistent_semantic_index(&tracked).unwrap();
        std::fs::create_dir_all(".yoyo").unwrap();
        let path = Path::new(DEFAULT_SEMANTIC_INDEX_PATH);
        std::fs::write(path, serde_json::to_string_pretty(&index).unwrap()).unwrap();

        let loaded = load_persistent_semantic_index(path).unwrap();
        let report = SemanticIndexReport {
            path: path.to_path_buf(),
            file_count: loaded.file_count,
            term_count: loaded.files.iter().map(|file| file.terms.len()).sum(),
            written: true,
        };

        std::env::set_current_dir(old_dir).unwrap();
        let rendered = report.render();
        assert!(rendered.contains("DeepSeek context semantic index"));
        assert!(rendered.contains("written: true"));
        let payload = report.payload();
        assert_eq!(payload["diagnostic"], "deepseek_context_semantic_index");
        assert_eq!(payload["schema_version"], SEMANTIC_INDEX_SCHEMA_VERSION);
        assert_eq!(payload["path"], DEFAULT_SEMANTIC_INDEX_PATH);
        assert_eq!(payload["file_count"], 1);
        assert_eq!(payload["term_count"], report.term_count);
        assert_eq!(payload["written"], true);
        assert_eq!(loaded.file_count, 1);
    }

    #[test]
    fn parses_codeowners_rules_for_context_ranking() {
        let rules = parse_codeowners_rules(
            r#"
            # team ownership
            /src/context/ @context-team
            *.md @docs
            "#,
        );

        assert_eq!(rules.len(), 2);
        assert!(ownership_rule_matches_term(&rules[0], "context"));
        assert!(path_matches_owner_rule("src/context/ranking.rs", &rules[0]));
        assert!(path_matches_owner_rule("README.md", &rules[1]));
    }

    #[test]
    fn reads_recent_state_events_through_yoagent_state_adapter() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let event = crate::state::StateEvent {
            event_id: "evt-context-failure".into(),
            event_type: crate::state::EventType::CommandCompleted,
            schema_version: yoagent_state::CURRENT_EVENT_SCHEMA_VERSION,
            timestamp_ms: 1,
            actor: crate::state::Actor::Tool,
            run_id: Some("run-context".into()),
            session_id: None,
            trace_id: "trace-context".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "is_error": true,
                "result_preview": "error[E0425] at src/context.rs:10"
            }),
        };
        crate::state::append_event(&path, &event).unwrap();

        let events = read_recent_state_events_from(&path, 10);
        let tracked = vec!["src/context.rs".to_string()];
        let selected = select_failing_output_files(&events, &tracked, 4);

        assert_eq!(events[0]["event_type"], "CommandCompleted");
        assert_eq!(events[0]["run_id"], "run-context");
        assert_eq!(selected, vec!["src/context.rs"]);
    }

    #[test]
    fn test_max_project_files_constant() {
        assert_eq!(MAX_PROJECT_FILES, 200);
    }

    #[test]
    fn test_max_recent_files_constant() {
        assert_eq!(MAX_RECENT_FILES, 20);
    }

    #[test]
    fn test_list_project_context_files_returns_vec() {
        // This test verifies the function runs without panicking.
        // In CI the project may or may not have YOYO.md present.
        let files = list_project_context_files();
        for (name, lines) in &files {
            assert!(!name.is_empty());
            assert!(*lines > 0);
        }
    }

    #[test]
    fn test_get_project_file_listing_no_panic() {
        // Should not panic regardless of whether we're in a git repo or not.
        // In CI this runs inside a git repo, so we expect Some with files.
        let result = get_project_file_listing();
        // If we're in a git repo (likely in CI), verify the output is reasonable
        if let Some(listing) = &result {
            assert!(!listing.is_empty(), "File listing should not be empty");
            let lines: Vec<&str> = listing.lines().collect();
            assert!(
                lines.len() <= MAX_PROJECT_FILES + 1, // +1 for possible "... and N more" line
                "File listing should be capped at {} files",
                MAX_PROJECT_FILES
            );
            // Should contain at least Cargo.toml (we're in a Rust project)
            assert!(
                listing.contains("Cargo.toml"),
                "File listing should contain Cargo.toml"
            );
        }
    }

    #[test]
    #[serial]
    fn test_load_project_context_includes_file_listing() {
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(env!("CARGO_MANIFEST_DIR")).unwrap();

        // load_project_context should include project file listing when in a git repo
        let result = load_project_context();
        if let Some(context) = &result {
            // If we're in a git repo, context should include the file listing section
            if get_project_file_listing().is_some() {
                assert!(
                    context.contains("## Project Files"),
                    "Context should contain Project Files section"
                );
            }
        }

        let _ = std::env::set_current_dir(original_dir);
    }

    #[test]
    fn test_get_recently_changed_files_in_git_repo() {
        // We're running in a git repo (CI or local), so this should return Some
        let result = get_recently_changed_files(20);
        if let Some(files) = &result {
            assert!(!files.is_empty(), "Should have recently changed files");
            // Files should be deduplicated
            let unique: std::collections::HashSet<&String> = files.iter().collect();
            assert_eq!(
                files.len(),
                unique.len(),
                "Recently changed files should be deduplicated"
            );
            // Should respect the max limit
            assert!(files.len() <= 20, "Should not exceed max_files limit");
        }
    }

    #[test]
    fn test_get_recently_changed_files_respects_limit() {
        // Request only 2 files — should return at most 2
        let result = get_recently_changed_files(2);
        if let Some(files) = &result {
            assert!(
                files.len() <= 2,
                "Should respect max_files=2, got {}",
                files.len()
            );
        }
    }

    #[test]
    fn test_get_recently_changed_files_no_duplicates() {
        let result = get_recently_changed_files(50);
        if let Some(files) = &result {
            let unique: std::collections::HashSet<&String> = files.iter().collect();
            assert_eq!(files.len(), unique.len(), "Files should be deduplicated");
        }
    }

    #[test]
    #[serial]
    fn test_load_project_context_includes_recently_changed() {
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(env!("CARGO_MANIFEST_DIR")).unwrap();

        let result = load_project_context();
        if let Some(context) = &result {
            if get_recently_changed_files(MAX_RECENT_FILES).is_some() {
                assert!(
                    context.contains("## Recently Changed Files"),
                    "Context should contain Recently Changed Files section"
                );
            }
        }

        let _ = std::env::set_current_dir(original_dir);
    }

    #[test]
    #[serial]
    fn test_get_git_status_context_in_repo() {
        // We're running inside a git repo, so this should return Some
        let result = get_git_status_context();
        assert!(result.is_some(), "Should return Some when in a git repo");
        assert!(
            result.as_ref().unwrap().contains("Branch:"),
            "Should contain 'Branch:' label"
        );
    }

    #[test]
    #[serial]
    fn test_get_git_status_context_contains_branch() {
        let result = get_git_status_context().expect("Should be in a git repo");
        // Get the actual branch name to verify it's in the output
        let branch = crate::git::git_branch().expect("Should get branch name");
        assert!(
            result.contains(&format!("Branch: {branch}")),
            "Should contain actual branch name: {branch}"
        );
    }

    #[test]
    #[serial]
    fn test_git_status_context_format() {
        let result = get_git_status_context().expect("Should be in a git repo");
        assert!(
            result.starts_with("## Git Status\n\n"),
            "Should start with '## Git Status' header"
        );
    }

    #[test]
    #[serial]
    fn test_load_project_context_includes_git_status() {
        // In a git repo, load_project_context should include git status
        let result = load_project_context();
        if let Some(context) = &result {
            if get_git_status_context().is_some() {
                assert!(
                    context.contains("## Git Status"),
                    "Context should contain Git Status section"
                );
            }
        }
    }

    #[test]
    fn test_yoyo_md_is_primary_context_file() {
        // YOYO.md should be the first (primary) context file
        assert_eq!(
            PROJECT_CONTEXT_FILES[0], "YOYO.md",
            "YOYO.md must be the primary context file"
        );
        // CLAUDE.md should be present as compatibility alias but not first
        assert!(
            PROJECT_CONTEXT_FILES.contains(&"CLAUDE.md"),
            "CLAUDE.md should still be supported for compatibility"
        );
        assert_ne!(
            PROJECT_CONTEXT_FILES[0], "CLAUDE.md",
            "CLAUDE.md should not be the primary context file"
        );
        // Cross-tool compatibility files
        assert!(
            PROJECT_CONTEXT_FILES.contains(&"AGENTS.md"),
            "AGENTS.md should be supported (Gemini CLI)"
        );
        assert!(
            PROJECT_CONTEXT_FILES.contains(&".cursorrules"),
            ".cursorrules should be supported (Cursor)"
        );
        assert!(
            PROJECT_CONTEXT_FILES.contains(&".github/copilot-instructions.md"),
            ".github/copilot-instructions.md should be supported (GitHub Copilot)"
        );
    }

    #[test]
    #[serial]
    fn test_project_context_includes_conventions() {
        // When run in a directory with no YOYO.md but with a Cargo.toml,
        // load_project_context should include development conventions.
        // We run in a temp dir with a git repo and Cargo.toml but no YOYO.md.
        use std::process::Command;
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        // Initialize a git repo so file listing works
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .ok();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .ok();

        // Change to temp dir, call load_project_context, change back
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let ctx = load_project_context();
        // Restore original dir; ignore errors from concurrent test interference
        let _ = std::env::set_current_dir(&original_dir);

        let ctx = ctx.unwrap();
        assert!(
            ctx.contains("## Development Conventions"),
            "Should include conventions section"
        );
        assert!(
            ctx.contains("cargo"),
            "Rust conventions should mention cargo"
        );
    }

    #[test]
    #[serial]
    fn test_project_context_includes_conventions_with_context_file() {
        // When YOYO.md exists, conventions should STILL be injected
        // (they complement explicit instructions)
        use std::process::Command;
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        std::fs::write(
            dir.path().join("YOYO.md"),
            "# My Project\nCustom instructions",
        )
        .unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .ok();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .ok();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let ctx = load_project_context();
        // Restore original dir; ignore errors from concurrent test interference
        let _ = std::env::set_current_dir(&original_dir);

        let ctx = ctx.unwrap();
        assert!(
            ctx.contains("## Development Conventions"),
            "Should include conventions even when YOYO.md exists"
        );
        assert!(
            ctx.contains("cargo"),
            "Rust conventions should mention cargo"
        );
        assert!(
            ctx.contains("Custom instructions"),
            "Should include YOYO.md content"
        );
        // Verify ordering: context file content comes BEFORE conventions
        let context_pos = ctx.find("Custom instructions").unwrap();
        let conventions_pos = ctx.find("## Development Conventions").unwrap();
        assert!(
            context_pos < conventions_pos,
            "Context file content should appear before conventions"
        );
    }

    #[test]
    #[serial]
    fn test_load_cursorrules_file() {
        // A .cursorrules file should be loaded as project context
        use std::process::Command;
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(".cursorrules"),
            "Always use TypeScript strict mode.",
        )
        .unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .ok();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .ok();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let ctx = load_project_context();
        let _ = std::env::set_current_dir(&original_dir);

        let ctx = ctx.unwrap();
        assert!(
            ctx.contains("Always use TypeScript strict mode"),
            "Should load .cursorrules content"
        );
    }

    #[test]
    #[serial]
    fn deepseek_instruction_loader_honors_explicit_policy_files() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("YOYO.md"), "Primary yoyo policy").unwrap();
        std::fs::write(dir.path().join("AGENTS.md"), "Agent compatibility policy").unwrap();
        std::fs::write(dir.path().join(".cursorrules"), "Cursor-only rules").unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let ctx = load_instruction_context_for_files(["YOYO.md", "AGENTS.md"]);
        let _ = std::env::set_current_dir(&original_dir);

        let ctx = ctx.unwrap();
        assert!(ctx.contains("Primary yoyo policy"));
        assert!(ctx.contains("--- From AGENTS.md ---"));
        assert!(ctx.contains("Agent compatibility policy"));
        assert!(
            !ctx.contains("Cursor-only rules"),
            "DeepSeek instruction context should honor the genome instruction-file policy"
        );
    }

    #[test]
    #[serial]
    fn test_load_agents_md_file() {
        // An AGENTS.md file should be loaded as project context
        use std::process::Command;
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("AGENTS.md"),
            "# Agent Instructions\nUse pytest for testing.",
        )
        .unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .ok();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .ok();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let ctx = load_project_context();
        let _ = std::env::set_current_dir(&original_dir);

        let ctx = ctx.unwrap();
        assert!(
            ctx.contains("Use pytest for testing"),
            "Should load AGENTS.md content"
        );
    }

    #[test]
    #[serial]
    fn test_load_copilot_instructions_file() {
        // A .github/copilot-instructions.md file should be loaded as project context
        use std::process::Command;
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join(".github")).unwrap();
        std::fs::write(
            dir.path().join(".github/copilot-instructions.md"),
            "Follow Google style guide.",
        )
        .unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .ok();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .ok();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let ctx = load_project_context();
        let _ = std::env::set_current_dir(&original_dir);

        let ctx = ctx.unwrap();
        assert!(
            ctx.contains("Follow Google style guide"),
            "Should load .github/copilot-instructions.md content"
        );
    }

    #[test]
    #[serial]
    fn test_multiple_context_files_get_separators() {
        // When multiple instruction files exist, secondary files should have
        // a "--- From <file> ---" separator for model clarity.
        use std::process::Command;
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("YOYO.md"), "Primary instructions").unwrap();
        std::fs::write(dir.path().join("AGENTS.md"), "Agent instructions").unwrap();
        std::fs::write(dir.path().join(".cursorrules"), "Cursor rules").unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .ok();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .ok();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let ctx = load_project_context();
        let _ = std::env::set_current_dir(&original_dir);

        let ctx = ctx.unwrap();
        // First file (YOYO.md) should NOT have a separator
        assert!(
            !ctx.contains("--- From YOYO.md ---"),
            "Primary file should not have a separator prefix"
        );
        // Secondary files should have separators
        assert!(
            ctx.contains("--- From AGENTS.md ---"),
            "AGENTS.md should have a separator: got: {ctx}"
        );
        assert!(
            ctx.contains("--- From .cursorrules ---"),
            ".cursorrules should have a separator: got: {ctx}"
        );
        // Content from all files should be present
        assert!(ctx.contains("Primary instructions"));
        assert!(ctx.contains("Agent instructions"));
        assert!(ctx.contains("Cursor rules"));
    }
}
