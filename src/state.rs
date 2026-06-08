//! Append-only state recorder backed by `yoagent-state` event primitives.
//!
//! The harness keeps its product-specific redaction, compatibility fields, and
//! SQLite projection here, while event identity, schema version, actor shape, and
//! canonical JSONL records come from the local `yoagent-state` package.
//!
//! Keep this module boring: append-only JSONL first, fail-soft by default, and a
//! SQLite projection for indexed production queries.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Once, OnceLock};

thread_local! {
    static LAST_RUN_ERROR: Cell<Option<String>> = const { Cell::new(None) };
}

static PANIC_HOOK_INSTALLED: Once = Once::new();

/// Install a panic hook that records a `FailureObserved` event before the
/// process aborts. The hook preserves any previously-registered hook by
/// calling it after the event is recorded.
///
/// This function is idempotent and safe to call multiple times — only the
/// first call installs the hook.
pub fn install_panic_hook() {
    PANIC_HOOK_INSTALLED.call_once(|| {
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info: &std::panic::PanicHookInfo<'_>| {
            let msg = info
                .payload()
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()))
                .unwrap_or("<unknown panic>");
            let location = info
                .location()
                .map(|loc| format!("{}:{}", loc.file(), loc.line()))
                .unwrap_or_else(|| "unknown location".to_string());
            let payload = json!({
                "failure_class": "rust_panic",
                "panic_message": msg,
                "panic_location": location,
                "recorded_at_ms": now_ms(),
            });
            // fail-soft: the global recorder might not be initialized yet
            record(EventType::FailureObserved, Actor::Harness, payload);
            prev_hook(info);
        }));
    });
}

/// Store an error message for the current run. This is used by
/// `exit_with_state` to include error context in the `RunCompleted`
/// event before process exit.
pub fn store_run_error(msg: &str) {
    LAST_RUN_ERROR.with(|cell| {
        cell.set(Some(msg.to_string()));
    });
}

static GLOBAL_RECORDER: OnceLock<StateRecorder> = OnceLock::new();
pub const STATE_ADAPTER_NAME: &str = "yoagent-state";
pub const STATE_ADAPTER_MODE: &str = "path-dependency";
pub const STATE_SQLITE_SCHEMA_VERSION: u32 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateConfig {
    pub enabled: bool,
    pub fail_soft: bool,
    pub events_path: PathBuf,
    pub store_path: Option<PathBuf>,
}

impl Default for StateConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            fail_soft: true,
            events_path: PathBuf::from(".yoyo/state/events.jsonl"),
            store_path: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum EventType {
    RunStarted,
    RunCompleted,
    ModelCallStarted,
    ModelCallCompleted,
    ToolCallStarted,
    ToolCallCompleted,
    FileRead,
    FileEdited,
    CommandStarted,
    CommandCompleted,
    TestStarted,
    TestCompleted,
    FailureObserved,
    HypothesisCreated,
    PatchProposed,
    PatchApplied,
    PatchEvaluated,
    PatchPromoted,
    PatchRejected,
    MemoryProposed,
    MemoryPromoted,
    MemoryRejected,
    DecisionRecorded,
    HumanApprovalRequested,
    HumanApprovalReceived,
    CommitCreated,
    RevertPerformed,
    ContextBuilt,
    CacheMetricsRecorded,
    JsonOutputFailure,
    ToolSchemaFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Actor {
    Yoyo,
    User,
    Tool,
    Harness,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessPatchKind {
    ContextPolicy,
    PromptPolicy,
    ThinkingPolicy,
    ModelRoutingPolicy,
    ToolSchema,
    TestPolicy,
    RepairPolicy,
    MemoryPolicy,
    PermissionPolicy,
    ShellPolicy,
    StateProjection,
    Eval,
    Transport,
    Safety,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessPatchStatus {
    Proposed,
    RiskScored,
    ApprovedForFork,
    AppliedInFork,
    Evaluated,
    Promoted,
    Rejected,
    NeedsHuman,
    Reverted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessPatchRisk {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessPatch {
    pub patch_id: String,
    pub kind: HarnessPatchKind,
    pub status: HarnessPatchStatus,
    pub base_harness_version: String,
    pub base_git_commit: Option<String>,
    pub state_version: u32,
    pub intent: String,
    pub evidence_event_ids: Vec<String>,
    pub expected_effects: Vec<String>,
    pub risk_level: HarnessPatchRisk,
    pub eval_plan: Vec<String>,
    pub rollback_plan: Vec<String>,
    pub created_at_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalStatus {
    Passed,
    Failed,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalResult {
    pub eval_id: String,
    pub harness_version: String,
    pub patch_id: Option<String>,
    pub suite: String,
    pub status: EvalStatus,
    pub score: Option<f64>,
    pub passed: u64,
    pub failed: u64,
    pub metrics: Value,
    pub failure_event_ids: Vec<String>,
    pub created_at_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateEvent {
    pub event_id: String,
    pub event_type: EventType,
    pub schema_version: u32,
    pub timestamp_ms: u128,
    pub actor: Actor,
    pub run_id: Option<String>,
    pub session_id: Option<String>,
    pub trace_id: String,
    pub parent_event_ids: Vec<String>,
    pub payload: Value,
}

#[derive(Debug, Clone)]
pub struct StateRecorder {
    events_path: PathBuf,
    store_path: Option<PathBuf>,
    fail_soft: bool,
    run_id: String,
    trace_id: String,
}

impl StateRecorder {
    pub fn new(config: StateConfig) -> Self {
        let now = now_ms();
        Self {
            events_path: config.events_path,
            store_path: config.store_path,
            fail_soft: config.fail_soft,
            run_id: format!("run-{now}-{}", std::process::id()),
            trace_id: format!("trace-{now}-{}", std::process::id()),
        }
    }

    pub fn append(
        &self,
        event_type: EventType,
        actor: Actor,
        payload: Value,
    ) -> Result<String, String> {
        let foundation_event =
            yoagent_state::Event::new(actor_ref(&actor), event_type_label(&event_type), payload);
        let event = StateEvent {
            event_id: foundation_event.id.to_string(),
            event_type,
            schema_version: foundation_event.schema_version,
            timestamp_ms: foundation_event.ts_ms.max(0) as u128,
            actor,
            run_id: Some(self.run_id.clone()),
            session_id: None,
            trace_id: self.trace_id.clone(),
            parent_event_ids: Vec::new(),
            payload: foundation_event.payload,
        };
        append_event_with_projection(&self.events_path, self.store_path.as_deref(), &event)?;
        Ok(event.event_id)
    }
}

pub fn init_global(config: StateConfig, startup_payload: Value) -> Result<(), String> {
    if !config.enabled {
        return Ok(());
    }
    let fail_soft = config.fail_soft;
    let recorder = StateRecorder::new(config);
    let init_result = recorder.append(EventType::RunStarted, Actor::Harness, startup_payload);
    match init_result {
        Ok(_) => {
            let _ = GLOBAL_RECORDER.set(recorder);
            Ok(())
        }
        Err(e) if fail_soft => {
            eprintln!("warning: state recorder disabled after init failure: {e}");
            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub fn is_initialized() -> bool {
    GLOBAL_RECORDER.get().is_some()
}

pub fn record(event_type: EventType, actor: Actor, payload: Value) {
    let Some(recorder) = GLOBAL_RECORDER.get() else {
        return;
    };
    if let Err(e) = recorder.append(event_type, actor, payload) {
        if recorder.fail_soft {
            eprintln!("warning: failed to record state event: {e}");
        } else {
            panic!("failed to record state event: {e}");
        }
    }
}

#[derive(Debug)]
pub struct RunCompletionGuard {
    status: &'static str,
}

impl RunCompletionGuard {
    pub fn completed_on_drop() -> Self {
        Self {
            status: "completed",
        }
    }
}

impl Drop for RunCompletionGuard {
    fn drop(&mut self) {
        mark_run_completed(self.status);
    }
}

pub fn mark_run_completed(status: &str) {
    record(
        EventType::RunCompleted,
        Actor::Harness,
        run_completed_payload(status, None),
    );
}

/// Like `mark_run_completed("error")` but includes an error message.
/// This also stores the error via `store_run_error` so it can be
/// retrieved later if needed.
pub fn mark_run_completed_with_error(msg: &str) {
    store_run_error(msg);
    record(
        EventType::RunCompleted,
        Actor::Harness,
        run_completed_payload("error", Some(msg)),
    );
}

pub fn run_completed_payload(status: &str, error: Option<&str>) -> Value {
    let mut payload = json!({
        "status": status,
        "completed_at_ms": now_ms(),
    });
    if let Some(msg) = error {
        if !msg.is_empty() {
            payload["error"] = json!(msg);
        }
    }
    payload
}

pub fn record_cache_metrics(model: &str, usage: &yoagent::Usage) {
    let Some(payload) = cache_metrics_payload(model, usage) else {
        return;
    };
    record(EventType::CacheMetricsRecorded, Actor::Harness, payload);
}

fn cache_metrics_payload(model: &str, usage: &yoagent::Usage) -> Option<Value> {
    if !model.starts_with("deepseek") {
        return None;
    }
    // yoagent's OpenAI-compatible provider maps DeepSeek
    // `usage.prompt_cache_hit_tokens` to `cache_read` and
    // `usage.prompt_cache_miss_tokens` to `input`. DeepSeek does not expose
    // Anthropic-style cache creation/write tokens for request-side markers.
    let deepseek_usage = crate::deepseek::DeepSeekUsage {
        input_tokens: usage.input,
        output_tokens: usage.output,
        cache_hit_tokens: Some(usage.cache_read),
        cache_miss_tokens: Some(usage.input),
    };
    if deepseek_usage.cache_hit_tokens == Some(0) && deepseek_usage.cache_miss_tokens == Some(0) {
        return None;
    }
    Some(json!({
        "model": model,
        "prompt_cache_hit_tokens": deepseek_usage.cache_hit_tokens,
        "prompt_cache_miss_tokens": deepseek_usage.cache_miss_tokens,
        "cache_hit_ratio": deepseek_usage.cache_hit_ratio(),
    }))
}

#[cfg(test)]
pub fn append_event(path: &Path, event: &StateEvent) -> Result<(), String> {
    append_event_with_projection(path, None, event)
}

fn append_event_with_projection(
    path: &Path,
    store_path: Option<&Path>,
    event: &StateEvent,
) -> Result<(), String> {
    let mut safe_event = event.clone();
    safe_event.payload = redact_state_payload(&event.payload);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create state directory '{}': {e}", parent.display()))?;
    }
    append_with_yoagent_state_store(path, to_yoagent_state_event(&safe_event))?;
    let sqlite_path = store_path
        .map(PathBuf::from)
        .unwrap_or_else(|| sqlite_projection_path(path));
    if let Err(e) = project_event_to_sqlite(&sqlite_path, &safe_event) {
        eprintln!(
            "warning: failed to update state sqlite projection '{}': {e}",
            sqlite_path.display()
        );
    }
    Ok(())
}

fn append_with_yoagent_state_store(path: &Path, event: yoagent_state::Event) -> Result<(), String> {
    let path = path.to_path_buf();
    let thread = std::thread::Builder::new()
        .name("yoagent-state-jsonl-append".to_string())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("create yoagent-state append runtime: {e}"))?;
            runtime.block_on(async move {
                use yoagent_state::EventStore;
                yoagent_state::JsonlEventStore::new(&path)
                    .append(vec![event])
                    .await
                    .map(|_| ())
                    .map_err(|e| format!("append canonical yoagent-state event: {e}"))
            })
        })
        .map_err(|e| format!("start yoagent-state append thread: {e}"))?;
    thread
        .join()
        .map_err(|_| "yoagent-state append thread panicked".to_string())?
}

pub fn redact_state_payload(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut redacted = serde_json::Map::new();
            for (key, value) in map {
                if is_raw_reasoning_key(key) {
                    redacted.insert(
                        key.clone(),
                        Value::String("[redacted raw reasoning]".to_string()),
                    );
                } else if is_sensitive_key(key) {
                    redacted.insert(key.clone(), Value::String("[redacted]".to_string()));
                } else {
                    redacted.insert(key.clone(), redact_state_payload(value));
                }
            }
            Value::Object(redacted)
        }
        Value::Array(items) => Value::Array(items.iter().map(redact_state_payload).collect()),
        Value::String(text) => Value::String(redact_state_string(text)),
        _ => value.clone(),
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized: String = key
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect();
    matches!(
        normalized.as_str(),
        "apikey"
            | "accesskey"
            | "secretkey"
            | "sessiontoken"
            | "refreshtoken"
            | "authtoken"
            | "bearertoken"
            | "authorization"
            | "password"
            | "passwd"
            | "credential"
            | "credentials"
            | "privatekey"
    ) || normalized.contains("secret")
}

fn is_raw_reasoning_key(key: &str) -> bool {
    let normalized: String = key
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect();
    matches!(
        normalized.as_str(),
        "reasoningcontent" | "rawreasoning" | "chainofthought"
    )
}

fn redact_state_string(text: &str) -> String {
    static SK_TOKEN_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"\bsk-[A-Za-z0-9_-]{8,}\b").unwrap());
    static GITHUB_TOKEN_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"\bgh[pousr]_[A-Za-z0-9_]{20,}\b").unwrap());
    static BEARER_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)\bBearer\s+[A-Za-z0-9._~+/=-]{8,}").unwrap());
    static AWS_ACCESS_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"\bAKIA[0-9A-Z]{12,}\b").unwrap());
    static PRIVATE_KEY_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(
            r"(?s)-----BEGIN [A-Z ]*PRIVATE KEY-----.*?-----END [A-Z ]*PRIVATE KEY-----",
        )
        .unwrap()
    });
    static THINK_BLOCK_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?is)<think>.*?</think>").unwrap());
    static ASSIGNMENT_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(
            r#"(?i)\b(api[_-]?key|secret[_-]?key|password|passwd|auth[_-]?token|access[_-]?token|refresh[_-]?token|session[_-]?token|github[_-]?token|private[_-]?key|authorization|credential)\s*[:=]\s*["']?[^"'\s,;]+"#,
        )
        .unwrap()
    });

    let text = SK_TOKEN_RE.replace_all(text, "sk-[redacted]").to_string();
    let text = GITHUB_TOKEN_RE
        .replace_all(&text, "gh[redacted]")
        .to_string();
    let text = BEARER_RE
        .replace_all(&text, "Bearer [redacted]")
        .to_string();
    let text = AWS_ACCESS_RE
        .replace_all(&text, "AKIA[redacted]")
        .to_string();
    let text = PRIVATE_KEY_RE
        .replace_all(&text, "[redacted private key]")
        .to_string();
    let text = THINK_BLOCK_RE
        .replace_all(&text, "[redacted raw reasoning]")
        .to_string();
    ASSIGNMENT_RE
        .replace_all(&text, |caps: &regex::Captures| {
            let full = caps.get(0).map(|m| m.as_str()).unwrap_or("");
            if let Some((key, sep)) = split_secret_assignment(full) {
                format!("{key}{sep}[redacted]")
            } else {
                "[redacted]".to_string()
            }
        })
        .to_string()
}

fn split_secret_assignment(raw: &str) -> Option<(&str, &str)> {
    let idx = raw.find('=').or_else(|| raw.find(':'))?;
    let (key, rest) = raw.split_at(idx);
    let sep_len = rest
        .chars()
        .take_while(|ch| *ch == '=' || *ch == ':' || ch.is_whitespace())
        .map(char::len_utf8)
        .sum::<usize>();
    Some((key, &rest[..sep_len]))
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectionReport {
    pub events: usize,
    pub patches: usize,
    pub evals: usize,
    pub failures: usize,
    pub hypotheses: usize,
    pub decisions: usize,
    pub cache_metrics: usize,
    pub relations: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateMigrationReport {
    pub from_version: u32,
    pub to_version: u32,
    pub applied_versions: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateRelation {
    pub src_id: String,
    pub relation: String,
    pub dst_id: String,
    pub dst_kind: String,
}

pub fn sqlite_projection_path(events_path: &Path) -> PathBuf {
    events_path
        .parent()
        .map(|parent| parent.join("state.sqlite"))
        .unwrap_or_else(|| PathBuf::from("state.sqlite"))
}

pub fn rebuild_sqlite_projection(
    events_path: &Path,
    sqlite_path: &Path,
) -> Result<ProjectionReport, String> {
    let raw = std::fs::read_to_string(events_path)
        .map_err(|e| format!("read events '{}': {e}", events_path.display()))?;
    if sqlite_path.exists() {
        std::fs::remove_file(sqlite_path)
            .map_err(|e| format!("remove existing sqlite '{}': {e}", sqlite_path.display()))?;
    }
    let mut conn = open_projection(sqlite_path)?;
    ensure_projection_schema(&conn)?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("start sqlite projection transaction: {e}"))?;
    let mut report = ProjectionReport::default();
    for (idx, line) in raw.lines().enumerate() {
        let event = parse_state_event_line(line)
            .map_err(|e| format!("parse event line {}: {e}", idx + 1))?;
        project_event_with_conn(&tx, &event, &mut report)?;
    }
    tx.commit()
        .map_err(|e| format!("commit sqlite projection: {e}"))?;
    Ok(report)
}

fn project_event_to_sqlite(sqlite_path: &Path, event: &StateEvent) -> Result<(), String> {
    let conn = open_projection(sqlite_path)?;
    ensure_projection_schema(&conn)?;
    let mut report = ProjectionReport::default();
    project_event_with_conn(&conn, event, &mut report)
}

fn parse_state_event_line(line: &str) -> Result<StateEvent, String> {
    let value: Value = serde_json::from_str(line).map_err(|e| e.to_string())?;
    if value.get("event_id").is_some()
        && value
            .get("actor")
            .map(|actor| actor.is_string())
            .unwrap_or(false)
    {
        return serde_json::from_value(value).map_err(|e| e.to_string());
    }

    let mut payload = value.get("payload").cloned().unwrap_or(Value::Null);
    let yoyo_meta = payload.get("_yoyo").cloned().unwrap_or(Value::Null);
    if let Value::Object(map) = &mut payload {
        map.remove("_yoyo");
        if map.len() == 1 && map.contains_key("value") {
            payload = map.remove("value").unwrap_or(Value::Null);
        }
    }

    let event_id = value
        .get("event_id")
        .or_else(|| value.get("id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing event id".to_string())?
        .to_string();
    let event_type_value = value
        .get("event_type")
        .or_else(|| value.get("kind"))
        .and_then(|v| v.as_str())
        .or_else(|| yoyo_meta.get("event_type").and_then(|v| v.as_str()))
        .ok_or_else(|| "missing 'event_type'".to_string())?;
    let event_type = parse_event_type_label(event_type_value)?;
    let timestamp_ms = value
        .get("timestamp_ms")
        .or_else(|| value.get("ts_ms"))
        .and_then(|v| v.as_i64())
        .unwrap_or_default()
        .max(0) as u128;
    let actor = parse_state_actor(&value, &yoyo_meta)?;
    let parent_event_ids = value
        .get("parent_event_ids")
        .and_then(|v| v.as_array())
        .map(|ids| {
            ids.iter()
                .filter_map(|id| id.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_else(|| {
            value
                .get("causation_id")
                .and_then(|v| v.as_str())
                .map(|id| vec![id.to_string()])
                .unwrap_or_default()
                .into_iter()
                .chain(
                    yoyo_meta
                        .get("parent_event_ids")
                        .and_then(|v| v.as_array())
                        .into_iter()
                        .flatten()
                        .filter_map(|id| id.as_str().map(|s| s.to_string())),
                )
                .collect()
        });

    Ok(StateEvent {
        event_id,
        event_type,
        schema_version: value
            .get("schema_version")
            .and_then(|v| v.as_u64())
            .unwrap_or(yoagent_state::CURRENT_EVENT_SCHEMA_VERSION as u64)
            as u32,
        timestamp_ms,
        actor,
        run_id: value
            .get("run_id")
            .and_then(|v| v.as_str())
            .or_else(|| yoyo_meta.get("run_id").and_then(|v| v.as_str()))
            .map(|s| s.to_string()),
        session_id: value
            .get("session_id")
            .and_then(|v| v.as_str())
            .or_else(|| yoyo_meta.get("session_id").and_then(|v| v.as_str()))
            .map(|s| s.to_string()),
        trace_id: value
            .get("trace_id")
            .and_then(|v| v.as_str())
            .or_else(|| value.get("correlation_id").and_then(|v| v.as_str()))
            .or_else(|| yoyo_meta.get("trace_id").and_then(|v| v.as_str()))
            .unwrap_or("")
            .to_string(),
        parent_event_ids,
        payload,
    })
}

fn parse_state_actor(value: &Value, yoyo_meta: &Value) -> Result<Actor, String> {
    if let Some(actor) = value.get("actor_label").and_then(|v| v.as_str()) {
        return actor_from_label(actor);
    }
    if let Some(actor) = yoyo_meta.get("actor").and_then(|v| v.as_str()) {
        return actor_from_label(actor);
    }
    match value.get("actor") {
        Some(Value::String(actor)) => actor_from_label(actor),
        Some(Value::Object(actor)) => {
            let kind = actor.get("kind").and_then(|v| v.as_str()).unwrap_or("");
            let id = actor.get("id").and_then(|v| v.as_str()).unwrap_or("");
            match (kind, id) {
                ("agent", "yoyo") => Ok(Actor::Yoyo),
                ("user", _) => Ok(Actor::User),
                ("tool", _) => Ok(Actor::Tool),
                ("system", _) => Ok(Actor::Harness),
                _ => Err(format!("unknown actor {kind}/{id}")),
            }
        }
        _ => Err("missing actor".to_string()),
    }
}

fn actor_from_label(actor: &str) -> Result<Actor, String> {
    match actor {
        "yoyo" => Ok(Actor::Yoyo),
        "user" => Ok(Actor::User),
        "tool" => Ok(Actor::Tool),
        "harness" => Ok(Actor::Harness),
        other => Err(format!("unknown actor '{other}'")),
    }
}

fn open_projection(path: &Path) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            format!(
                "create sqlite projection directory '{}': {e}",
                parent.display()
            )
        })?;
    }
    Connection::open(path).map_err(|e| format!("open sqlite projection '{}': {e}", path.display()))
}

pub fn migrate_sqlite_projection(sqlite_path: &Path) -> Result<StateMigrationReport, String> {
    let conn = open_projection(sqlite_path)?;
    ensure_projection_schema(&conn)
}

fn ensure_projection_schema(conn: &Connection) -> Result<StateMigrationReport, String> {
    conn.execute_batch("PRAGMA journal_mode = WAL;")
        .map_err(|e| format!("configure sqlite projection journal: {e}"))?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS state_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at_ms INTEGER NOT NULL
        );
        "#,
    )
    .map_err(|e| format!("ensure sqlite migration ledger: {e}"))?;

    let from_version = sqlite_user_version(conn)?;
    if from_version > STATE_SQLITE_SCHEMA_VERSION {
        return Err(format!(
            "state sqlite schema version {from_version} is newer than supported version {STATE_SQLITE_SCHEMA_VERSION}"
        ));
    }

    let mut applied_versions = Vec::new();
    if from_version < 1 {
        apply_projection_schema_v1(conn)?;
        set_sqlite_user_version(conn, 1)?;
        applied_versions.push(1);
    } else {
        apply_projection_schema_v1(conn)?;
    }
    if from_version < 2 {
        apply_projection_schema_v2(conn)?;
        set_sqlite_user_version(conn, 2)?;
        applied_versions.push(2);
    } else {
        apply_projection_schema_v2(conn)?;
    }
    if from_version < 3 {
        apply_projection_schema_v3(conn)?;
        set_sqlite_user_version(conn, 3)?;
        applied_versions.push(3);
    } else {
        apply_projection_schema_v3(conn)?;
    }

    Ok(StateMigrationReport {
        from_version,
        to_version: STATE_SQLITE_SCHEMA_VERSION,
        applied_versions,
    })
}

fn apply_projection_schema_v1(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS state_events (
            event_id TEXT PRIMARY KEY,
            event_type TEXT NOT NULL,
            timestamp_ms INTEGER NOT NULL,
            actor TEXT NOT NULL,
            run_id TEXT,
            session_id TEXT,
            trace_id TEXT NOT NULL,
            parent_event_ids_json TEXT NOT NULL,
            payload_json TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_state_events_type_time
            ON state_events(event_type, timestamp_ms);
        CREATE INDEX IF NOT EXISTS idx_state_events_run
            ON state_events(run_id);
        CREATE INDEX IF NOT EXISTS idx_state_events_trace
            ON state_events(trace_id);

        CREATE TABLE IF NOT EXISTS harness_patches (
            patch_id TEXT PRIMARY KEY,
            status TEXT,
            kind TEXT,
            risk_level TEXT,
            base_git_commit TEXT,
            intent TEXT,
            last_event_id TEXT NOT NULL,
            updated_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_harness_patches_status
            ON harness_patches(status);

        CREATE TABLE IF NOT EXISTS eval_results (
            eval_id TEXT PRIMARY KEY,
            patch_id TEXT,
            harness_version TEXT,
            suite TEXT,
            status TEXT,
            score REAL,
            last_event_id TEXT NOT NULL,
            created_at_ms INTEGER,
            payload_json TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_eval_results_patch
            ON eval_results(patch_id);
        CREATE INDEX IF NOT EXISTS idx_eval_results_harness
            ON eval_results(harness_version);

        CREATE TABLE IF NOT EXISTS failures (
            event_id TEXT PRIMARY KEY,
            event_type TEXT NOT NULL,
            source TEXT,
            error_preview TEXT,
            run_id TEXT,
            timestamp_ms INTEGER NOT NULL,
            payload_json TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_failures_time
            ON failures(timestamp_ms);

        CREATE TABLE IF NOT EXISTS cache_metrics (
            event_id TEXT PRIMARY KEY,
            model TEXT,
            prompt_cache_hit_tokens INTEGER,
            prompt_cache_miss_tokens INTEGER,
            cache_hit_ratio REAL,
            timestamp_ms INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS state_relations (
            src_id TEXT NOT NULL,
            relation TEXT NOT NULL,
            dst_id TEXT NOT NULL,
            dst_kind TEXT NOT NULL,
            event_id TEXT NOT NULL,
            PRIMARY KEY (src_id, relation, dst_id, event_id)
        );
        CREATE INDEX IF NOT EXISTS idx_state_relations_src
            ON state_relations(src_id);
        CREATE INDEX IF NOT EXISTS idx_state_relations_dst
            ON state_relations(dst_id);
        CREATE INDEX IF NOT EXISTS idx_state_relations_relation
            ON state_relations(relation);
        "#,
    )
    .map_err(|e| format!("ensure sqlite projection schema v1: {e}"))?;
    conn.execute(
        r#"
        INSERT OR IGNORE INTO state_migrations (version, name, applied_at_ms)
        VALUES (?1, ?2, ?3)
        "#,
        params![1_i64, "projection-schema-v1", i64_ms(now_ms())],
    )
    .map_err(|e| format!("record sqlite projection migration v1: {e}"))?;
    Ok(())
}

fn apply_projection_schema_v2(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS hypotheses (
            hypothesis_id TEXT PRIMARY KEY,
            event_id TEXT NOT NULL,
            failure_event_id TEXT,
            summary TEXT,
            confidence REAL,
            status TEXT,
            run_id TEXT,
            timestamp_ms INTEGER NOT NULL,
            payload_json TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_hypotheses_failure
            ON hypotheses(failure_event_id);
        CREATE INDEX IF NOT EXISTS idx_hypotheses_status
            ON hypotheses(status);

        CREATE TABLE IF NOT EXISTS decisions (
            decision_id TEXT PRIMARY KEY,
            event_id TEXT NOT NULL,
            decision_type TEXT,
            decision TEXT,
            rationale TEXT,
            status TEXT,
            patch_id TEXT,
            eval_id TEXT,
            run_id TEXT,
            timestamp_ms INTEGER NOT NULL,
            payload_json TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_decisions_patch
            ON decisions(patch_id);
        CREATE INDEX IF NOT EXISTS idx_decisions_eval
            ON decisions(eval_id);
        CREATE INDEX IF NOT EXISTS idx_decisions_status
            ON decisions(status);
        "#,
    )
    .map_err(|e| format!("ensure sqlite projection schema v2: {e}"))?;
    conn.execute(
        r#"
        INSERT OR IGNORE INTO state_migrations (version, name, applied_at_ms)
        VALUES (?1, ?2, ?3)
        "#,
        params![
            2_i64,
            "projection-schema-v2-hypotheses-decisions",
            i64_ms(now_ms())
        ],
    )
    .map_err(|e| format!("record sqlite projection migration v2: {e}"))?;
    Ok(())
}

fn apply_projection_schema_v3(conn: &Connection) -> Result<(), String> {
    add_column_if_missing(conn, "harness_patches", "base_harness_version", "TEXT")?;
    add_column_if_missing(conn, "harness_patches", "state_version", "INTEGER")?;
    add_column_if_missing(conn, "harness_patches", "expected_effects_json", "TEXT")?;
    add_column_if_missing(conn, "harness_patches", "eval_plan_json", "TEXT")?;
    add_column_if_missing(conn, "harness_patches", "rollback_plan_json", "TEXT")?;
    conn.execute(
        r#"
        INSERT OR IGNORE INTO state_migrations (version, name, applied_at_ms)
        VALUES (?1, ?2, ?3)
        "#,
        params![
            3_i64,
            "projection-schema-v3-harness-patch-metadata",
            i64_ms(now_ms())
        ],
    )
    .map_err(|e| format!("record sqlite projection migration v3: {e}"))?;
    Ok(())
}

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|e| format!("inspect sqlite table '{table}': {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| format!("query sqlite columns for '{table}': {e}"))?;
    for row in rows {
        if row.map_err(|e| format!("read sqlite column for '{table}': {e}"))? == column {
            return Ok(());
        }
    }
    conn.execute_batch(&format!(
        "ALTER TABLE {table} ADD COLUMN {column} {definition};"
    ))
    .map_err(|e| format!("add sqlite column '{table}.{column}': {e}"))?;
    Ok(())
}

fn sqlite_user_version(conn: &Connection) -> Result<u32, String> {
    conn.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
        .map(|version| version.max(0) as u32)
        .map_err(|e| format!("read sqlite user_version: {e}"))
}

fn set_sqlite_user_version(conn: &Connection, version: u32) -> Result<(), String> {
    conn.execute_batch(&format!("PRAGMA user_version = {version};"))
        .map_err(|e| format!("set sqlite user_version to {version}: {e}"))
}

pub fn query_sqlite_relations(sqlite_path: &Path, id: &str) -> Result<Vec<StateRelation>, String> {
    let conn = open_projection(sqlite_path)?;
    ensure_projection_schema(&conn)?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT src_id, relation, dst_id, dst_kind
            FROM state_relations
            WHERE src_id = ?1 OR dst_id = ?1
            ORDER BY relation, src_id, dst_id
            LIMIT 100
            "#,
        )
        .map_err(|e| format!("prepare state relation query: {e}"))?;
    let rows = stmt
        .query_map(params![id], |row| {
            Ok(StateRelation {
                src_id: row.get(0)?,
                relation: row.get(1)?,
                dst_id: row.get(2)?,
                dst_kind: row.get(3)?,
            })
        })
        .map_err(|e| format!("query state relations: {e}"))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| format!("read state relation row: {e}"))?);
    }
    Ok(out)
}

fn project_event_with_conn(
    conn: &Connection,
    event: &StateEvent,
    report: &mut ProjectionReport,
) -> Result<(), String> {
    let event_type = event_type_label(&event.event_type);
    let actor = actor_label(&event.actor);
    let parent_ids_json = serde_json::to_string(&event.parent_event_ids)
        .map_err(|e| format!("serialize parent event ids: {e}"))?;
    let payload_json = serde_json::to_string(&event.payload)
        .map_err(|e| format!("serialize event payload: {e}"))?;
    conn.execute(
        r#"
        INSERT OR REPLACE INTO state_events (
            event_id, event_type, timestamp_ms, actor, run_id, session_id, trace_id,
            parent_event_ids_json, payload_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        "#,
        params![
            event.event_id,
            event_type,
            i64_ms(event.timestamp_ms),
            actor,
            event.run_id.as_deref(),
            event.session_id.as_deref(),
            event.trace_id,
            parent_ids_json,
            payload_json,
        ],
    )
    .map_err(|e| format!("insert state event '{}': {e}", event.event_id))?;
    report.events += 1;

    project_patch(conn, event, &payload_json, report)?;
    project_eval(conn, event, &payload_json, report)?;
    project_failure(conn, event, &payload_json, report)?;
    project_hypothesis(conn, event, &payload_json, report)?;
    project_decision(conn, event, &payload_json, report)?;
    project_cache_metric(conn, event, report)?;
    project_relations(conn, event, report)?;
    Ok(())
}

fn project_patch(
    conn: &Connection,
    event: &StateEvent,
    _payload_json: &str,
    report: &mut ProjectionReport,
) -> Result<(), String> {
    let Some(patch_id) = event.payload.get("patch_id").and_then(|v| v.as_str()) else {
        return Ok(());
    };
    let Some(status) =
        patch_status_for_event(event).or_else(|| payload_str(&event.payload, "status"))
    else {
        return Ok(());
    };
    conn.execute(
        r#"
        INSERT INTO harness_patches (
            patch_id, status, kind, risk_level, base_git_commit, intent,
            base_harness_version, state_version, expected_effects_json, eval_plan_json,
            rollback_plan_json, last_event_id, updated_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        ON CONFLICT(patch_id) DO UPDATE SET
            status = excluded.status,
            kind = COALESCE(excluded.kind, harness_patches.kind),
            risk_level = COALESCE(excluded.risk_level, harness_patches.risk_level),
            base_git_commit = COALESCE(excluded.base_git_commit, harness_patches.base_git_commit),
            intent = COALESCE(excluded.intent, harness_patches.intent),
            base_harness_version = COALESCE(excluded.base_harness_version, harness_patches.base_harness_version),
            state_version = COALESCE(excluded.state_version, harness_patches.state_version),
            expected_effects_json = COALESCE(excluded.expected_effects_json, harness_patches.expected_effects_json),
            eval_plan_json = COALESCE(excluded.eval_plan_json, harness_patches.eval_plan_json),
            rollback_plan_json = COALESCE(excluded.rollback_plan_json, harness_patches.rollback_plan_json),
            last_event_id = excluded.last_event_id,
            updated_ms = excluded.updated_ms
        "#,
        params![
            patch_id,
            status,
            payload_str(&event.payload, "kind"),
            payload_str(&event.payload, "risk_level"),
            payload_str(&event.payload, "base_git_commit"),
            payload_str(&event.payload, "intent"),
            payload_str(&event.payload, "base_harness_version"),
            payload_u64(&event.payload, "state_version").map(|value| value as i64),
            payload_json_field(&event.payload, "expected_effects")?,
            payload_json_field(&event.payload, "eval_plan")?,
            payload_json_field(&event.payload, "rollback_plan")?,
            event.event_id,
            i64_ms(event.timestamp_ms),
        ],
    )
    .map_err(|e| format!("project harness patch '{patch_id}': {e}"))?;
    report.patches += 1;
    Ok(())
}

fn project_eval(
    conn: &Connection,
    event: &StateEvent,
    payload_json: &str,
    report: &mut ProjectionReport,
) -> Result<(), String> {
    if event.event_type != EventType::PatchEvaluated {
        return Ok(());
    }
    let Some(eval_id) = event.payload.get("eval_id").and_then(|v| v.as_str()) else {
        return Ok(());
    };
    conn.execute(
        r#"
        INSERT OR REPLACE INTO eval_results (
            eval_id, patch_id, harness_version, suite, status, score, last_event_id,
            created_at_ms, payload_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        "#,
        params![
            eval_id,
            payload_str(&event.payload, "patch_id"),
            payload_str(&event.payload, "harness_version"),
            payload_str(&event.payload, "suite"),
            payload_str(&event.payload, "status"),
            event.payload.get("score").and_then(|v| v.as_f64()),
            event.event_id,
            event
                .payload
                .get("created_at_ms")
                .and_then(|v| v.as_u64())
                .map(|v| v as i64),
            payload_json,
        ],
    )
    .map_err(|e| format!("project eval result '{eval_id}': {e}"))?;
    report.evals += 1;
    Ok(())
}

fn project_failure(
    conn: &Connection,
    event: &StateEvent,
    payload_json: &str,
    report: &mut ProjectionReport,
) -> Result<(), String> {
    if !matches!(
        event.event_type,
        EventType::FailureObserved | EventType::JsonOutputFailure | EventType::ToolSchemaFailure
    ) {
        return Ok(());
    }
    conn.execute(
        r#"
        INSERT OR REPLACE INTO failures (
            event_id, event_type, source, error_preview, run_id, timestamp_ms, payload_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        params![
            event.event_id,
            event_type_label(&event.event_type),
            payload_str(&event.payload, "source")
                .or_else(|| payload_str(&event.payload, "operation")),
            payload_str(&event.payload, "error_preview"),
            event.run_id.as_deref(),
            i64_ms(event.timestamp_ms),
            payload_json,
        ],
    )
    .map_err(|e| format!("project failure '{}': {e}", event.event_id))?;
    report.failures += 1;
    Ok(())
}

fn project_hypothesis(
    conn: &Connection,
    event: &StateEvent,
    payload_json: &str,
    report: &mut ProjectionReport,
) -> Result<(), String> {
    if event.event_type != EventType::HypothesisCreated {
        return Ok(());
    }
    let hypothesis_id = payload_str(&event.payload, "hypothesis_id").unwrap_or(&event.event_id);
    conn.execute(
        r#"
        INSERT OR REPLACE INTO hypotheses (
            hypothesis_id, event_id, failure_event_id, summary, confidence, status, run_id,
            timestamp_ms, payload_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        "#,
        params![
            hypothesis_id,
            event.event_id,
            payload_str(&event.payload, "failure_event_id"),
            payload_str(&event.payload, "summary")
                .or_else(|| payload_str(&event.payload, "hypothesis")),
            event.payload.get("confidence").and_then(|v| v.as_f64()),
            payload_str(&event.payload, "status"),
            event.run_id.as_deref(),
            i64_ms(event.timestamp_ms),
            payload_json,
        ],
    )
    .map_err(|e| format!("project hypothesis '{hypothesis_id}': {e}"))?;
    report.hypotheses += 1;
    Ok(())
}

fn project_decision(
    conn: &Connection,
    event: &StateEvent,
    payload_json: &str,
    report: &mut ProjectionReport,
) -> Result<(), String> {
    if event.event_type != EventType::DecisionRecorded {
        return Ok(());
    }
    let decision_id = payload_str(&event.payload, "decision_id").unwrap_or(&event.event_id);
    conn.execute(
        r#"
        INSERT OR REPLACE INTO decisions (
            decision_id, event_id, decision_type, decision, rationale, status, patch_id, eval_id,
            run_id, timestamp_ms, payload_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        "#,
        params![
            decision_id,
            event.event_id,
            payload_str(&event.payload, "decision_type")
                .or_else(|| payload_str(&event.payload, "type")),
            payload_str(&event.payload, "decision"),
            payload_str(&event.payload, "rationale")
                .or_else(|| payload_str(&event.payload, "reason")),
            payload_str(&event.payload, "status"),
            payload_str(&event.payload, "patch_id"),
            payload_str(&event.payload, "eval_id"),
            event.run_id.as_deref(),
            i64_ms(event.timestamp_ms),
            payload_json,
        ],
    )
    .map_err(|e| format!("project decision '{decision_id}': {e}"))?;
    report.decisions += 1;
    Ok(())
}

fn project_cache_metric(
    conn: &Connection,
    event: &StateEvent,
    report: &mut ProjectionReport,
) -> Result<(), String> {
    if event.event_type != EventType::CacheMetricsRecorded {
        return Ok(());
    }
    conn.execute(
        r#"
        INSERT OR REPLACE INTO cache_metrics (
            event_id, model, prompt_cache_hit_tokens, prompt_cache_miss_tokens,
            cache_hit_ratio, timestamp_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        params![
            event.event_id,
            payload_str(&event.payload, "model"),
            event
                .payload
                .get("prompt_cache_hit_tokens")
                .and_then(|v| v.as_u64())
                .map(|v| v as i64),
            event
                .payload
                .get("prompt_cache_miss_tokens")
                .and_then(|v| v.as_u64())
                .map(|v| v as i64),
            event
                .payload
                .get("cache_hit_ratio")
                .and_then(|v| v.as_f64()),
            i64_ms(event.timestamp_ms),
        ],
    )
    .map_err(|e| format!("project cache metric '{}': {e}", event.event_id))?;
    report.cache_metrics += 1;
    Ok(())
}

fn project_relations(
    conn: &Connection,
    event: &StateEvent,
    report: &mut ProjectionReport,
) -> Result<(), String> {
    if let Some(run_id) = event.run_id.as_deref() {
        insert_relation(
            conn,
            &event.event_id,
            "observed_in",
            run_id,
            "run",
            &event.event_id,
            report,
        )?;
    }
    if !event.trace_id.is_empty() {
        insert_relation(
            conn,
            &event.event_id,
            "traced_by",
            &event.trace_id,
            "trace",
            &event.event_id,
            report,
        )?;
    }
    for parent_id in &event.parent_event_ids {
        insert_relation(
            conn,
            &event.event_id,
            "derived_from",
            parent_id,
            "event",
            &event.event_id,
            report,
        )?;
    }
    match event.event_type {
        EventType::FileRead => {
            for file_path in payload_file_refs(&event.payload) {
                insert_relation(
                    conn,
                    &event.event_id,
                    "references_file",
                    &file_path,
                    "file",
                    &event.event_id,
                    report,
                )?;
            }
        }
        EventType::FileEdited | EventType::CommitCreated | EventType::RevertPerformed => {
            for file_path in payload_file_refs(&event.payload) {
                insert_relation(
                    conn,
                    &event.event_id,
                    "modified_file",
                    &file_path,
                    "file",
                    &event.event_id,
                    report,
                )?;
                insert_relation(
                    conn,
                    &event.event_id,
                    "modified",
                    &file_path,
                    "file",
                    &event.event_id,
                    report,
                )?;
            }
        }
        _ => {}
    }
    if matches!(
        event.event_type,
        EventType::ModelCallStarted | EventType::ModelCallCompleted
    ) {
        let model_call_id = payload_str(&event.payload, "model_call_id").unwrap_or(&event.event_id);
        insert_relation(
            conn,
            &event.event_id,
            "records_model_call",
            model_call_id,
            "model_call",
            &event.event_id,
            report,
        )?;
        if let Some(model) = payload_str(&event.payload, "model") {
            insert_relation(
                conn,
                model_call_id,
                "uses_model",
                model,
                "model",
                &event.event_id,
                report,
            )?;
        }
    }
    if matches!(
        event.event_type,
        EventType::ToolCallStarted | EventType::ToolCallCompleted
    ) {
        if let Some(tool_call_id) = payload_str(&event.payload, "tool_call_id") {
            insert_relation(
                conn,
                &event.event_id,
                "records_tool_call",
                tool_call_id,
                "tool_call",
                &event.event_id,
                report,
            )?;
            if let Some(tool_name) = payload_str(&event.payload, "tool_name") {
                insert_relation(
                    conn,
                    tool_call_id,
                    "invokes_tool",
                    tool_name,
                    "tool",
                    &event.event_id,
                    report,
                )?;
            }
        }
    }
    for task_id in payload_task_ids(&event.payload) {
        insert_relation(
            conn,
            &event.event_id,
            "records_task",
            &task_id,
            "task",
            &event.event_id,
            report,
        )?;
    }
    if let Some(eval_id) = payload_str(&event.payload, "eval_id") {
        for task_id in payload_eval_task_ids(&event.payload) {
            insert_relation(
                conn,
                &task_id,
                "tested_by",
                eval_id,
                "eval",
                &event.event_id,
                report,
            )?;
        }
    }
    if let Some(harness_version) = payload_str(&event.payload, "harness_version") {
        insert_relation(
            conn,
            &event.event_id,
            "uses_harness_version",
            harness_version,
            "harness_version",
            &event.event_id,
            report,
        )?;
    }
    if let Some(base_harness_version) = payload_str(&event.payload, "base_harness_version") {
        insert_relation(
            conn,
            &event.event_id,
            "based_on_harness_version",
            base_harness_version,
            "harness_version",
            &event.event_id,
            report,
        )?;
    }
    for evidence_id in payload_string_array(&event.payload, "evidence_event_ids") {
        insert_relation(
            conn,
            &event.event_id,
            "supported_by",
            &evidence_id,
            "event",
            &event.event_id,
            report,
        )?;
    }
    for failure_id in payload_string_array(&event.payload, "failure_event_ids") {
        insert_relation(
            conn,
            &event.event_id,
            "evaluated_failure",
            &failure_id,
            "event",
            &event.event_id,
            report,
        )?;
    }
    if let Some(failure_id) = payload_str(&event.payload, "failure_event_id") {
        insert_relation(
            conn,
            &event.event_id,
            "explains",
            failure_id,
            "event",
            &event.event_id,
            report,
        )?;
    }
    if let Some(hypothesis_id) = payload_str(&event.payload, "hypothesis_id") {
        insert_relation(
            conn,
            &event.event_id,
            "records_hypothesis",
            hypothesis_id,
            "hypothesis",
            &event.event_id,
            report,
        )?;
    }
    if event.event_type == EventType::HypothesisCreated {
        let hypothesis_id = payload_str(&event.payload, "hypothesis_id").unwrap_or(&event.event_id);
        if let Some(failure_id) = payload_str(&event.payload, "failure_event_id") {
            insert_relation(
                conn,
                failure_id,
                "caused_by",
                hypothesis_id,
                "hypothesis",
                &event.event_id,
                report,
            )?;
        }
        for evidence_id in payload_string_array(&event.payload, "evidence_event_ids") {
            insert_relation(
                conn,
                &evidence_id,
                "supports",
                hypothesis_id,
                "hypothesis",
                &event.event_id,
                report,
            )?;
        }
        for key in [
            "contradicting_evidence_event_ids",
            "contradiction_event_ids",
        ] {
            for evidence_id in payload_string_array(&event.payload, key) {
                insert_relation(
                    conn,
                    &evidence_id,
                    "contradicts",
                    hypothesis_id,
                    "hypothesis",
                    &event.event_id,
                    report,
                )?;
            }
        }
    }
    if let Some(decision_id) = payload_str(&event.payload, "decision_id") {
        insert_relation(
            conn,
            &event.event_id,
            "records_decision",
            decision_id,
            "decision",
            &event.event_id,
            report,
        )?;
    }
    if event.event_type == EventType::DecisionRecorded
        && payload_str(&event.payload, "decision_type") == Some("release_gate")
        && payload_bool(&event.payload, "stale") == Some(true)
    {
        if let Some(eval_id) = payload_str(&event.payload, "last_eval_id") {
            insert_relation(
                conn,
                eval_id,
                "stale_due_to",
                &event.event_id,
                "event",
                &event.event_id,
                report,
            )?;
        }
    }
    if event.event_type == EventType::DecisionRecorded
        && payload_str(&event.payload, "decision_type") == Some("release_gate")
        && payload_bool(&event.payload, "last_eval_git_dirty") == Some(true)
    {
        if let Some(eval_id) = payload_str(&event.payload, "last_eval_id") {
            insert_relation(
                conn,
                &event.event_id,
                "blocked_by_dirty_eval",
                eval_id,
                "eval",
                &event.event_id,
                report,
            )?;
            insert_relation(
                conn,
                eval_id,
                "dirty_eval_blocks_release_gate",
                &event.event_id,
                "event",
                &event.event_id,
                report,
            )?;
        }
    }
    if event.event_type == EventType::DecisionRecorded
        && payload_str(&event.payload, "decision_type") == Some("release_gate")
    {
        for gate in payload_string_array(&event.payload, "missing_required_gates") {
            let gate_id = format!("required_gate:{gate}");
            insert_relation(
                conn,
                &event.event_id,
                "missing_required_gate",
                &gate_id,
                "evidence",
                &event.event_id,
                report,
            )?;
            insert_relation(
                conn,
                &gate_id,
                "blocks_release_gate",
                &event.event_id,
                "event",
                &event.event_id,
                report,
            )?;
        }
        if payload_bool(&event.payload, "fixture_breadth_satisfied") == Some(false) {
            insert_relation(
                conn,
                &event.event_id,
                "fixture_breadth_below_minimum",
                "release_fixture_breadth_minimum",
                "evidence",
                &event.event_id,
                report,
            )?;
            insert_relation(
                conn,
                "release_fixture_breadth_minimum",
                "blocks_release_gate",
                &event.event_id,
                "event",
                &event.event_id,
                report,
            )?;
        }
        let has_fixture_risk_minimum = event
            .payload
            .get("min_fixture_risk_labels")
            .and_then(Value::as_object)
            .map(|risks| !risks.is_empty())
            .unwrap_or(false);
        if has_fixture_risk_minimum
            && payload_bool(&event.payload, "fixture_risk_satisfied") == Some(false)
        {
            insert_relation(
                conn,
                &event.event_id,
                "fixture_risk_coverage_below_minimum",
                "release_fixture_risk_minimum",
                "evidence",
                &event.event_id,
                report,
            )?;
            insert_relation(
                conn,
                "release_fixture_risk_minimum",
                "blocks_release_gate",
                &event.event_id,
                "event",
                &event.event_id,
                report,
            )?;
        }
        if payload_u64(&event.payload, "last_eval_mutation_scope_failures").unwrap_or_default() > 0
            || payload_u64(&event.payload, "last_eval_unexpected_changed_files").unwrap_or_default()
                > 0
        {
            insert_relation(
                conn,
                &event.event_id,
                "fixture_agent_mutation_scope_block",
                "release_fixture_agent_mutation_scope",
                "evidence",
                &event.event_id,
                report,
            )?;
            insert_relation(
                conn,
                "release_fixture_agent_mutation_scope",
                "blocks_release_gate",
                &event.event_id,
                "event",
                &event.event_id,
                report,
            )?;
        }
        if let Some(source_passed) = payload_bool(&event.payload, "source_provenance_passed") {
            let relation = if source_passed {
                "passed_source_provenance_audit"
            } else {
                "blocked_by_source_provenance_audit"
            };
            insert_relation(
                conn,
                &event.event_id,
                relation,
                "release_source_provenance_audit",
                "policy",
                &event.event_id,
                report,
            )?;
            if !source_passed {
                insert_relation(
                    conn,
                    "release_source_provenance_audit",
                    "blocks_release_gate",
                    &event.event_id,
                    "event",
                    &event.event_id,
                    report,
                )?;
            }
        }
        if let Some(scan_source) = payload_str(&event.payload, "source_provenance_scan_source") {
            let scan_id = format!("source_provenance_scan:{scan_source}");
            insert_relation(
                conn,
                &event.event_id,
                "used_source_provenance_scan",
                &scan_id,
                "policy",
                &event.event_id,
                report,
            )?;
            insert_relation(
                conn,
                &scan_id,
                "supports_source_provenance_audit",
                "release_source_provenance_audit",
                "policy",
                &event.event_id,
                report,
            )?;
        }
        for summary in payload_string_array(&event.payload, "source_provenance_finding_summaries") {
            let finding_id = format!("source_provenance_finding:{summary}");
            insert_relation(
                conn,
                &event.event_id,
                "has_source_provenance_finding",
                &finding_id,
                "evidence",
                &event.event_id,
                report,
            )?;
            insert_relation(
                conn,
                &finding_id,
                "supports_source_provenance_audit",
                "release_source_provenance_audit",
                "policy",
                &event.event_id,
                report,
            )?;
            if payload_bool(&event.payload, "source_provenance_passed") == Some(false) {
                insert_relation(
                    conn,
                    &finding_id,
                    "blocks_release_gate",
                    &event.event_id,
                    "event",
                    &event.event_id,
                    report,
                )?;
            }
        }
        if let Some(protocol_eval_id) = payload_str(&event.payload, "protocol_eval_id") {
            insert_relation(
                conn,
                &event.event_id,
                "requires_protocol_eval",
                protocol_eval_id,
                "eval",
                &event.event_id,
                report,
            )?;
            insert_relation(
                conn,
                protocol_eval_id,
                "supports_release_gate",
                &event.event_id,
                "event",
                &event.event_id,
                report,
            )?;
        }
        for (count_key, check_id) in [
            ("strict", "deepseek_protocol_check:strict_tool_call"),
            ("thinking", "deepseek_protocol_check:thinking_protocol"),
            ("stream", "deepseek_protocol_check:streaming"),
            ("json", "deepseek_protocol_check:json_output"),
            ("transport", "deepseek_protocol_check:transport_policy"),
        ] {
            if protocol_check_count_u64(&event.payload, count_key).unwrap_or_default() > 0 {
                insert_relation(
                    conn,
                    &event.event_id,
                    "covers_protocol_check",
                    check_id,
                    "evidence",
                    &event.event_id,
                    report,
                )?;
            }
        }
        if payload_bool(&event.payload, "protocol_eval_git_dirty") == Some(true) {
            if let Some(protocol_eval_id) = payload_str(&event.payload, "protocol_eval_id") {
                insert_relation(
                    conn,
                    &event.event_id,
                    "blocked_by_dirty_eval",
                    protocol_eval_id,
                    "eval",
                    &event.event_id,
                    report,
                )?;
                insert_relation(
                    conn,
                    protocol_eval_id,
                    "dirty_eval_blocks_release_gate",
                    &event.event_id,
                    "event",
                    &event.event_id,
                    report,
                )?;
            }
        }
        if payload_bool(&event.payload, "protocol_older_than_eval") == Some(true) {
            if let (Some(protocol_eval_id), Some(last_eval_id)) = (
                payload_str(&event.payload, "protocol_eval_id"),
                payload_str(&event.payload, "last_eval_id"),
            ) {
                insert_relation(
                    conn,
                    protocol_eval_id,
                    "older_than_release_eval",
                    last_eval_id,
                    "eval",
                    &event.event_id,
                    report,
                )?;
                insert_relation(
                    conn,
                    last_eval_id,
                    "blocked_by_stale_protocol_eval",
                    protocol_eval_id,
                    "eval",
                    &event.event_id,
                    report,
                )?;
            }
        }
    }
    if event.event_type == EventType::DecisionRecorded {
        if payload_str(&event.payload, "decision_type") == Some("harness_patch_promotion") {
            if let Some(protocol_eval_id) =
                promotion_decision_str(&event.payload, "protocol_eval_id")
            {
                insert_relation(
                    conn,
                    &event.event_id,
                    "requires_protocol_eval",
                    protocol_eval_id,
                    "eval",
                    &event.event_id,
                    report,
                )?;
                if promotion_decision_bool(&event.payload, "eligible") == Some(true)
                    || payload_str(&event.payload, "decision") == Some("promote")
                {
                    insert_relation(
                        conn,
                        protocol_eval_id,
                        "supports_promotion",
                        &event.event_id,
                        "event",
                        &event.event_id,
                        report,
                    )?;
                }
                if promotion_decision_str(&event.payload, "reason")
                    == Some("latest protocol eval is older than candidate eval")
                {
                    if let Some(candidate_eval_id) =
                        promotion_decision_str(&event.payload, "candidate_eval_id")
                    {
                        insert_relation(
                            conn,
                            protocol_eval_id,
                            "older_than_candidate_eval",
                            candidate_eval_id,
                            "eval",
                            &event.event_id,
                            report,
                        )?;
                        insert_relation(
                            conn,
                            candidate_eval_id,
                            "blocked_by_stale_protocol_eval",
                            protocol_eval_id,
                            "eval",
                            &event.event_id,
                            report,
                        )?;
                    }
                }
            }
            if promotion_decision_str(&event.payload, "reason")
                == Some("baseline and candidate fixture suite risk-label coverage differ")
            {
                insert_relation(
                    conn,
                    &event.event_id,
                    "promotion_fixture_risk_mismatch",
                    "promotion_fixture_risk_coverage",
                    "evidence",
                    &event.event_id,
                    report,
                )?;
                insert_relation(
                    conn,
                    "promotion_fixture_risk_coverage",
                    "blocks_promotion",
                    &event.event_id,
                    "event",
                    &event.event_id,
                    report,
                )?;
            }
        }
        if let Some(dirty_eval_id) = dirty_promotion_eval_id(&event.payload) {
            insert_relation(
                conn,
                &event.event_id,
                "blocked_by_dirty_eval",
                dirty_eval_id,
                "eval",
                &event.event_id,
                report,
            )?;
            insert_relation(
                conn,
                dirty_eval_id,
                "dirty_eval_blocks_promotion",
                &event.event_id,
                "event",
                &event.event_id,
                report,
            )?;
        }
    }
    if let Some(context_policy) = payload_str(&event.payload, "context_policy") {
        insert_relation(
            conn,
            &event.event_id,
            "uses_context_policy",
            context_policy,
            "context_policy",
            &event.event_id,
            report,
        )?;
    }
    if let Some(layout_version) = payload_u64(&event.payload, "layout_version") {
        let layout_id = format!("prompt_layout_v{layout_version}");
        insert_relation(
            conn,
            &event.event_id,
            "uses_prompt_layout",
            &layout_id,
            "prompt_layout",
            &event.event_id,
            report,
        )?;
    }
    if let Some(prompt_version) = payload_prompt_version_id(&event.payload) {
        insert_relation(
            conn,
            &event.event_id,
            "uses_prompt",
            &prompt_version,
            "prompt_version",
            &event.event_id,
            report,
        )?;
    }
    for block_name in payload_string_array(&event.payload, "stable_prefix_blocks") {
        insert_relation(
            conn,
            &event.event_id,
            "uses_context_block",
            &block_name,
            "context_block",
            &event.event_id,
            report,
        )?;
    }
    for block_name in payload_string_array(&event.payload, "dynamic_suffix_blocks") {
        insert_relation(
            conn,
            &event.event_id,
            "uses_context_block",
            &block_name,
            "context_block",
            &event.event_id,
            report,
        )?;
    }
    for instruction_file in payload_string_array(&event.payload, "include_instruction_files") {
        insert_relation(
            conn,
            &event.event_id,
            "uses_instruction_file",
            &instruction_file,
            "instruction_file",
            &event.event_id,
            report,
        )?;
    }
    if let Some(schema_name) = payload_str(&event.payload, "schema_name")
        .or_else(|| payload_str(&event.payload, "tool_name"))
    {
        insert_relation(
            conn,
            &event.event_id,
            "uses_schema",
            schema_name,
            "tool_schema",
            &event.event_id,
            report,
        )?;
        if let Some(schema_version) = payload_u64(&event.payload, "schema_version") {
            let schema_id = format!("{schema_name}@v{schema_version}");
            insert_relation(
                conn,
                &event.event_id,
                "uses_schema_version",
                &schema_id,
                "tool_schema_version",
                &event.event_id,
                report,
            )?;
        }
        if event.event_type == EventType::DecisionRecorded
            && payload_str(&event.payload, "decision_type") == Some("deepseek_json_output_check")
            && payload_str(&event.payload, "decision") == Some("passed")
        {
            insert_relation(
                conn,
                schema_name,
                "supports_json_output_check",
                &event.event_id,
                "event",
                &event.event_id,
                report,
            )?;
        }
    }
    if event.event_type == EventType::DecisionRecorded
        && payload_str(&event.payload, "decision_type") == Some("deepseek_strict_tool_call_check")
        && payload_str(&event.payload, "decision") == Some("passed")
    {
        for tool_name in payload_string_array(&event.payload, "selected_tool_names") {
            insert_relation(
                conn,
                &event.event_id,
                "uses_schema",
                &tool_name,
                "tool_schema",
                &event.event_id,
                report,
            )?;
        }
        for schema_name in payload_string_array(&event.payload, "schema_names") {
            insert_relation(
                conn,
                &schema_name,
                "supports_strict_tool_call_check",
                &event.event_id,
                "event",
                &event.event_id,
                report,
            )?;
        }
    }
    if event.event_type == EventType::DecisionRecorded
        && payload_str(&event.payload, "decision_type") == Some("deepseek_transport_policy_check")
        && payload_str(&event.payload, "decision") == Some("passed")
    {
        insert_relation(
            conn,
            &event.event_id,
            "uses_transport_policy",
            "deepseek_transport_policy",
            "policy",
            &event.event_id,
            report,
        )?;
        if let Some(transport_class) = payload_str(&event.payload, "transport_class") {
            let class_id = format!("transport_class:{transport_class}");
            insert_relation(
                conn,
                &class_id,
                "supports_transport_policy_check",
                &event.event_id,
                "event",
                &event.event_id,
                report,
            )?;
        }
    }
    if event.event_type == EventType::DecisionRecorded
        && payload_str(&event.payload, "decision_type") == Some("deepseek_thinking_protocol_check")
        && payload_str(&event.payload, "decision") == Some("passed")
    {
        insert_relation(
            conn,
            &event.event_id,
            "uses_thinking_protocol_policy",
            "deepseek_thinking_protocol_policy",
            "policy",
            &event.event_id,
            report,
        )?;
        if let Some(diagnostic_source) = payload_str(&event.payload, "diagnostic_source") {
            let probe_id = format!("thinking_probe:{diagnostic_source}");
            insert_relation(
                conn,
                &probe_id,
                "supports_thinking_protocol_check",
                &event.event_id,
                "event",
                &event.event_id,
                report,
            )?;
        }
    }
    if event.event_type == EventType::DecisionRecorded
        && payload_str(&event.payload, "decision_type") == Some("deepseek_streaming_protocol_check")
        && payload_str(&event.payload, "decision") == Some("passed")
    {
        insert_relation(
            conn,
            &event.event_id,
            "uses_streaming_protocol_policy",
            "deepseek_streaming_protocol_policy",
            "policy",
            &event.event_id,
            report,
        )?;
        if let Some(check) = payload_str(&event.payload, "check") {
            let probe_id = format!("streaming_probe:{check}");
            insert_relation(
                conn,
                &probe_id,
                "supports_streaming_protocol_check",
                &event.event_id,
                "event",
                &event.event_id,
                report,
            )?;
        }
    }
    if let Some(candidate_id) = payload_str(&event.payload, "candidate_id") {
        insert_relation(
            conn,
            &event.event_id,
            "records_memory",
            candidate_id,
            "memory",
            &event.event_id,
            report,
        )?;
        let lifecycle_relation = match event.event_type {
            EventType::MemoryProposed => Some("proposes_memory"),
            EventType::MemoryPromoted => Some("promoted_memory"),
            EventType::MemoryRejected => Some("rejected_memory"),
            _ => None,
        };
        if let Some(relation) = lifecycle_relation {
            insert_relation(
                conn,
                &event.event_id,
                relation,
                candidate_id,
                "memory",
                &event.event_id,
                report,
            )?;
        }
    }
    if let Some(proposed_event_id) = payload_str(&event.payload, "proposed_event_id") {
        insert_relation(
            conn,
            &event.event_id,
            "derived_from",
            proposed_event_id,
            "event",
            &event.event_id,
            report,
        )?;
    }
    if let Some(patch_event_id) = payload_str(&event.payload, "patch_event_id") {
        insert_relation(
            conn,
            &event.event_id,
            "derived_from",
            patch_event_id,
            "event",
            &event.event_id,
            report,
        )?;
    }
    if let Some(patch_id) = payload_str(&event.payload, "patch_id") {
        insert_relation(
            conn,
            &event.event_id,
            "uses_patch",
            patch_id,
            "patch",
            &event.event_id,
            report,
        )?;
        match event.event_type {
            EventType::PatchProposed => {
                for evidence_id in payload_string_array(&event.payload, "evidence_event_ids") {
                    insert_relation(
                        conn,
                        patch_id,
                        "addresses",
                        &evidence_id,
                        "event",
                        &event.event_id,
                        report,
                    )?;
                }
            }
            EventType::PatchEvaluated => {
                if let Some(eval_id) = payload_str(&event.payload, "eval_id") {
                    insert_relation(
                        conn,
                        patch_id,
                        "tested_by",
                        eval_id,
                        "eval",
                        &event.event_id,
                        report,
                    )?;
                    if payload_str(&event.payload, "status") == Some("passed") {
                        insert_relation(
                            conn,
                            patch_id,
                            "validated_by",
                            eval_id,
                            "eval",
                            &event.event_id,
                            report,
                        )?;
                    }
                }
            }
            EventType::PatchPromoted => insert_relation(
                conn,
                patch_id,
                "promoted_by",
                &event.event_id,
                "event",
                &event.event_id,
                report,
            )?,
            EventType::PatchRejected => insert_relation(
                conn,
                patch_id,
                "rejected_by",
                &event.event_id,
                "event",
                &event.event_id,
                report,
            )?,
            EventType::HumanApprovalRequested => insert_relation(
                conn,
                &event.event_id,
                "requests_approval_for_patch",
                patch_id,
                "patch",
                &event.event_id,
                report,
            )?,
            EventType::HumanApprovalReceived => insert_relation(
                conn,
                &event.event_id,
                "approved_patch",
                patch_id,
                "patch",
                &event.event_id,
                report,
            )?,
            EventType::RevertPerformed => {
                insert_relation(
                    conn,
                    &event.event_id,
                    "reverted_patch",
                    patch_id,
                    "patch",
                    &event.event_id,
                    report,
                )?;
                insert_relation(
                    conn,
                    patch_id,
                    "reverted_by",
                    &event.event_id,
                    "event",
                    &event.event_id,
                    report,
                )?;
            }
            _ => {}
        }
    }
    if payload_str(&event.payload, "intake_kind") == Some("issue") {
        if let Some(patch_id) = payload_str(&event.payload, "patch_id") {
            let issue_id = payload_str(&event.payload, "issue_id")
                .map(ToString::to_string)
                .unwrap_or_else(|| format!("issue:{patch_id}"));
            insert_relation(
                conn,
                &event.event_id,
                "records_issue",
                &issue_id,
                "issue",
                &event.event_id,
                report,
            )?;
            insert_relation(
                conn,
                &issue_id,
                "addresses_patch",
                patch_id,
                "patch",
                &event.event_id,
                report,
            )?;
        }
    }
    for approval_event_id in payload_string_array(&event.payload, "approval_event_ids") {
        insert_relation(
            conn,
            &event.event_id,
            "approved_by",
            &approval_event_id,
            "event",
            &event.event_id,
            report,
        )?;
    }
    if let Some(eval_id) = payload_str(&event.payload, "eval_id") {
        insert_relation(
            conn,
            &event.event_id,
            "records_eval",
            eval_id,
            "eval",
            &event.event_id,
            report,
        )?;
        for artifact_uri in payload_artifact_uris(&event.payload) {
            insert_relation(
                conn,
                eval_id,
                "has_artifact",
                &artifact_uri,
                "artifact",
                &event.event_id,
                report,
            )?;
        }
        for file_path in payload_fixture_agent_changed_files(&event.payload) {
            insert_relation(
                conn,
                eval_id,
                "agent_attempt_changed_file",
                &file_path,
                "file",
                &event.event_id,
                report,
            )?;
        }
        for file_path in payload_fixture_agent_unexpected_files(&event.payload) {
            insert_relation(
                conn,
                eval_id,
                "agent_attempt_unexpected_file",
                &file_path,
                "file",
                &event.event_id,
                report,
            )?;
        }
        for (metric_key, check_id) in [
            (
                "deepseek_strict_tool_call_checks",
                "deepseek_protocol_check:strict_tool_call",
            ),
            (
                "deepseek_thinking_protocol_checks",
                "deepseek_protocol_check:thinking_protocol",
            ),
            (
                "deepseek_streaming_protocol_checks",
                "deepseek_protocol_check:streaming",
            ),
            (
                "deepseek_json_output_checks",
                "deepseek_protocol_check:json_output",
            ),
            (
                "deepseek_transport_policy_checks",
                "deepseek_protocol_check:transport_policy",
            ),
        ] {
            if eval_state_metric_u64(&event.payload, metric_key).unwrap_or_default() > 0 {
                insert_relation(
                    conn,
                    eval_id,
                    "covers_protocol_check",
                    check_id,
                    "evidence",
                    &event.event_id,
                    report,
                )?;
            }
        }
    }
    for artifact_uri in payload_artifact_uris(&event.payload) {
        insert_relation(
            conn,
            &event.event_id,
            "references_artifact",
            &artifact_uri,
            "artifact",
            &event.event_id,
            report,
        )?;
    }
    if let Some(commit) =
        payload_str(&event.payload, "commit").or_else(|| payload_str(&event.payload, "commit_id"))
    {
        insert_relation(
            conn,
            &event.event_id,
            "records_commit",
            commit,
            "commit",
            &event.event_id,
            report,
        )?;
    }
    if let Some(branch) = payload_str(&event.payload, "branch") {
        insert_relation(
            conn,
            &event.event_id,
            "on_branch",
            branch,
            "branch",
            &event.event_id,
            report,
        )?;
    }
    if let Some(reverted_commit) = payload_str(&event.payload, "reverted_commit") {
        insert_relation(
            conn,
            &event.event_id,
            "reverted_commit",
            reverted_commit,
            "commit",
            &event.event_id,
            report,
        )?;
    }
    Ok(())
}

fn insert_relation(
    conn: &Connection,
    src_id: &str,
    relation: &str,
    dst_id: &str,
    dst_kind: &str,
    event_id: &str,
    report: &mut ProjectionReport,
) -> Result<(), String> {
    let changed = conn
        .execute(
            r#"
            INSERT OR IGNORE INTO state_relations (
                src_id, relation, dst_id, dst_kind, event_id
            ) VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            params![src_id, relation, dst_id, dst_kind, event_id],
        )
        .map_err(|e| format!("project relation {src_id} -[{relation}]-> {dst_id}: {e}"))?;
    report.relations += changed;
    Ok(())
}

fn payload_u64(payload: &Value, key: &str) -> Option<u64> {
    payload.get(key).and_then(|v| v.as_u64())
}

fn eval_state_metric_u64(payload: &Value, key: &str) -> Option<u64> {
    payload
        .get("metrics")
        .and_then(|metrics| metrics.get("state_metrics"))
        .and_then(|metrics| payload_u64(metrics, key))
}

fn protocol_check_count_u64(payload: &Value, key: &str) -> Option<u64> {
    payload
        .get("protocol_check_counts")
        .and_then(|counts| payload_u64(counts, key))
}

fn payload_bool(payload: &Value, key: &str) -> Option<bool> {
    payload.get(key).and_then(|v| v.as_bool())
}

fn payload_scalar_id(payload: &Value, key: &str) -> Option<String> {
    let value = payload.get(key)?;
    match value {
        Value::String(text) if !text.trim().is_empty() => Some(text.trim().to_string()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

fn payload_prompt_version_id(payload: &Value) -> Option<String> {
    if let Some(prompt_version) = payload_scalar_id(payload, "prompt_version") {
        return Some(prompt_version);
    }
    if let Some(layout_version) = payload_scalar_id(payload, "prompt_layout_version")
        .or_else(|| payload_scalar_id(payload, "layout_version"))
    {
        return Some(format!("prompt_layout_v{layout_version}"));
    }
    None
}

fn patch_status_for_event(event: &StateEvent) -> Option<&'static str> {
    match event.event_type {
        EventType::PatchProposed => Some("proposed"),
        EventType::PatchApplied => Some("applied_in_fork"),
        EventType::PatchEvaluated => Some("evaluated"),
        EventType::PatchPromoted => Some("promoted"),
        EventType::PatchRejected => Some("rejected"),
        EventType::RevertPerformed => Some("reverted"),
        EventType::HumanApprovalRequested => Some("needs_human"),
        EventType::HumanApprovalReceived => Some("approved_for_fork"),
        _ => None,
    }
}

fn payload_str<'a>(payload: &'a Value, key: &str) -> Option<&'a str> {
    payload.get(key).and_then(|v| v.as_str())
}

fn dirty_promotion_eval_id(payload: &Value) -> Option<&str> {
    let decision = payload.get("promotion_decision")?;
    let reason = decision.get("reason").and_then(Value::as_str)?;
    match reason {
        "candidate eval was run from a dirty worktree" => {
            decision.get("candidate_eval_id").and_then(Value::as_str)
        }
        "baseline eval was run from a dirty worktree" => {
            decision.get("baseline_eval_id").and_then(Value::as_str)
        }
        "latest protocol eval was run from a dirty worktree" => {
            decision.get("protocol_eval_id").and_then(Value::as_str)
        }
        _ => None,
    }
}

fn promotion_decision_str<'a>(payload: &'a Value, key: &str) -> Option<&'a str> {
    payload
        .get("promotion_decision")?
        .get(key)
        .and_then(Value::as_str)
}

fn promotion_decision_bool(payload: &Value, key: &str) -> Option<bool> {
    payload
        .get("promotion_decision")?
        .get(key)
        .and_then(Value::as_bool)
}

fn payload_json_field(payload: &Value, key: &str) -> Result<Option<String>, String> {
    payload
        .get(key)
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| format!("serialize payload field '{key}': {e}"))
}

fn payload_string_array(payload: &Value, key: &str) -> Vec<String> {
    payload
        .get(key)
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn payload_file_refs(payload: &Value) -> Vec<String> {
    let mut out = Vec::new();
    collect_payload_file_refs(payload, &mut out);
    if let Some(args) = payload.get("args") {
        collect_payload_file_refs(args, &mut out);
    }
    out
}

fn payload_fixture_agent_changed_files(payload: &Value) -> Vec<String> {
    let mut out = Vec::new();
    collect_fixture_agent_files(payload, "changed_files", &mut out);
    if let Some(metrics) = payload.get("metrics") {
        collect_fixture_agent_files(metrics, "changed_files", &mut out);
    }
    out
}

fn payload_fixture_agent_unexpected_files(payload: &Value) -> Vec<String> {
    let mut out = Vec::new();
    collect_fixture_agent_files(payload, "unexpected_changed_files", &mut out);
    if let Some(metrics) = payload.get("metrics") {
        collect_fixture_agent_files(metrics, "unexpected_changed_files", &mut out);
    }
    out
}

fn collect_fixture_agent_files(payload: &Value, key: &str, out: &mut Vec<String>) {
    let Some(attempts) = payload
        .get("fixture_agent_attempts")
        .and_then(|value| value.as_array())
    else {
        return;
    };
    for attempt in attempts {
        let Some(files) = attempt.get(key).and_then(|value| value.as_array()) else {
            continue;
        };
        for file in files {
            if let Some(path) = file.as_str() {
                push_file_ref(out, path);
            }
        }
    }
}

fn collect_payload_file_refs(payload: &Value, out: &mut Vec<String>) {
    for key in ["path", "file_path", "target_path"] {
        if let Some(path) = payload_str(payload, key) {
            push_file_ref(out, path);
        }
    }
    for key in ["paths", "file_paths", "target_paths", "files"] {
        for path in payload_string_array(payload, key) {
            push_file_ref(out, &path);
        }
    }
}

fn push_file_ref(out: &mut Vec<String>, path: &str) {
    let path = path.trim();
    if path.is_empty() {
        return;
    }
    push_unique(out, path.to_string());
}

fn payload_task_ids(payload: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(task_id) = payload_str(payload, "task_id") {
        push_nonempty_unique(&mut out, task_id);
    }
    for task_id in payload_string_array(payload, "task_ids") {
        push_nonempty_unique(&mut out, &task_id);
    }
    out
}

fn payload_eval_task_ids(payload: &Value) -> Vec<String> {
    let mut out = payload_task_ids(payload);
    if let Some(metrics) = payload.get("metrics") {
        collect_fixture_task_ids(metrics, &mut out);
    }
    collect_fixture_task_ids(payload, &mut out);
    out
}

fn collect_fixture_task_ids(value: &Value, out: &mut Vec<String>) {
    if let Some(items) = value
        .get("fixture_tasks")
        .and_then(|items| items.as_array())
    {
        for item in items {
            if let Some(task_id) = payload_str(item, "task_id") {
                push_nonempty_unique(out, task_id);
            }
        }
    }
}

fn push_nonempty_unique(out: &mut Vec<String>, value: &str) {
    let value = value.trim();
    if value.is_empty() {
        return;
    }
    push_unique(out, value.to_string());
}

fn payload_artifact_uris(payload: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(uri) = payload_str(payload, "artifact_uri") {
        push_unique(&mut out, uri.to_string());
    }
    if let Some(metrics) = payload.get("metrics") {
        if let Some(uri) = payload_str(metrics, "artifact_uri") {
            push_unique(&mut out, uri.to_string());
        }
        if let Some(items) = metrics.get("artifacts").and_then(|value| value.as_array()) {
            for item in items {
                if let Some(uri) = payload_str(item, "uri") {
                    push_unique(&mut out, uri.to_string());
                }
            }
        }
    }
    out
}

fn push_unique(items: &mut Vec<String>, value: String) {
    if !items.iter().any(|item| item == &value) {
        items.push(value);
    }
}

fn i64_ms(value: u128) -> i64 {
    value.min(i64::MAX as u128) as i64
}

fn event_type_label(event_type: &EventType) -> &'static str {
    match event_type {
        EventType::RunStarted => "RunStarted",
        EventType::RunCompleted => "RunCompleted",
        EventType::ModelCallStarted => "ModelCallStarted",
        EventType::ModelCallCompleted => "ModelCallCompleted",
        EventType::ToolCallStarted => "ToolCallStarted",
        EventType::ToolCallCompleted => "ToolCallCompleted",
        EventType::FileRead => "FileRead",
        EventType::FileEdited => "FileEdited",
        EventType::CommandStarted => "CommandStarted",
        EventType::CommandCompleted => "CommandCompleted",
        EventType::TestStarted => "TestStarted",
        EventType::TestCompleted => "TestCompleted",
        EventType::FailureObserved => "FailureObserved",
        EventType::HypothesisCreated => "HypothesisCreated",
        EventType::PatchProposed => "PatchProposed",
        EventType::PatchApplied => "PatchApplied",
        EventType::PatchEvaluated => "PatchEvaluated",
        EventType::PatchPromoted => "PatchPromoted",
        EventType::PatchRejected => "PatchRejected",
        EventType::MemoryProposed => "MemoryProposed",
        EventType::MemoryPromoted => "MemoryPromoted",
        EventType::MemoryRejected => "MemoryRejected",
        EventType::DecisionRecorded => "DecisionRecorded",
        EventType::HumanApprovalRequested => "HumanApprovalRequested",
        EventType::HumanApprovalReceived => "HumanApprovalReceived",
        EventType::CommitCreated => "CommitCreated",
        EventType::RevertPerformed => "RevertPerformed",
        EventType::ContextBuilt => "ContextBuilt",
        EventType::CacheMetricsRecorded => "CacheMetricsRecorded",
        EventType::JsonOutputFailure => "JsonOutputFailure",
        EventType::ToolSchemaFailure => "ToolSchemaFailure",
    }
}

fn parse_event_type_label(label: &str) -> Result<EventType, String> {
    let event_type = match label {
        "RunStarted" | "run.started" => EventType::RunStarted,
        "RunCompleted" | "run.finished" => EventType::RunCompleted,
        "ModelCallStarted" | "model.called" => EventType::ModelCallStarted,
        "ModelCallCompleted" | "model.finished" => EventType::ModelCallCompleted,
        "ToolCallStarted" | "tool.called" => EventType::ToolCallStarted,
        "ToolCallCompleted" | "tool.finished" => EventType::ToolCallCompleted,
        "FailureObserved" | "failure.observed" => EventType::FailureObserved,
        "PatchProposed" | "patch.proposed" => EventType::PatchProposed,
        other => serde_json::from_value(Value::String(other.to_string()))
            .map_err(|e| format!("invalid event type '{other}': {e}"))?,
    };
    Ok(event_type)
}

fn actor_ref(actor: &Actor) -> yoagent_state::ActorRef {
    match actor {
        Actor::Yoyo => yoagent_state::ActorRef::agent("yoyo"),
        Actor::User => yoagent_state::ActorRef::user("user"),
        Actor::Tool => yoagent_state::ActorRef::tool("tool"),
        Actor::Harness => yoagent_state::ActorRef::system("harness"),
    }
}

fn actor_label(actor: &Actor) -> &'static str {
    match actor {
        Actor::Yoyo => "yoyo",
        Actor::User => "user",
        Actor::Tool => "tool",
        Actor::Harness => "harness",
    }
}

fn to_yoagent_state_event(event: &StateEvent) -> yoagent_state::Event {
    let mut payload = event.payload.clone();
    if let Value::Object(map) = &mut payload {
        map.insert(
            "_yoyo".to_string(),
            json!({
                "event_type": event_type_label(&event.event_type),
                "actor": actor_label(&event.actor),
                "run_id": event.run_id,
                "session_id": event.session_id,
                "trace_id": event.trace_id,
                "parent_event_ids": event.parent_event_ids,
            }),
        );
    } else {
        payload = json!({
            "_yoyo": {
                "event_type": event_type_label(&event.event_type),
                "actor": actor_label(&event.actor),
                "run_id": event.run_id,
                "session_id": event.session_id,
                "trace_id": event.trace_id,
                "parent_event_ids": event.parent_event_ids,
            },
            "value": payload,
        });
    }

    yoagent_state::Event {
        id: yoagent_state::EventId::new(event.event_id.clone()),
        schema_version: event.schema_version,
        ts_ms: i64_ms(event.timestamp_ms),
        actor: actor_ref(&event.actor),
        kind: event_type_label(&event.event_type).to_string(),
        payload,
        causation_id: event
            .parent_event_ids
            .first()
            .map(|id| yoagent_state::EventId::new(id.clone())),
        correlation_id: Some(event.trace_id.clone()),
    }
}

fn persisted_event_value(event: &StateEvent) -> Value {
    serde_json::to_value(to_yoagent_state_event(event)).unwrap_or(Value::Null)
}

pub fn normalize_event_json_line(line: &str) -> Result<String, String> {
    let mut event = parse_state_event_line(line)?;
    event.payload = redact_state_payload(&event.payload);
    serde_json::to_string(&persisted_event_value(&event))
        .map_err(|e| format!("serialize normalized state event: {e}"))
}

pub fn compatibility_event_json_line(line: &str) -> Result<String, String> {
    let event = parse_state_event_line(line)?;
    serde_json::to_string(&compatibility_event_value(&event))
        .map_err(|e| format!("serialize compatibility state event: {e}"))
}

pub fn read_compatibility_events(path: &Path) -> Result<Vec<Value>, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("read state events '{}': {e}", path.display()))?;
    raw.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(idx, line)| {
            let normalized = compatibility_event_json_line(line)
                .map_err(|e| format!("parse event line {}: {e}", idx + 1))?;
            serde_json::from_str::<Value>(&normalized)
                .map_err(|e| format!("decode compatibility event line {}: {e}", idx + 1))
        })
        .collect()
}

fn compatibility_event_value(event: &StateEvent) -> Value {
    json!({
        "event_id": event.event_id,
        "event_type": event_type_label(&event.event_type),
        "schema_version": event.schema_version,
        "timestamp_ms": i64_ms(event.timestamp_ms),
        "actor": actor_label(&event.actor),
        "run_id": event.run_id,
        "session_id": event.session_id,
        "trace_id": event.trace_id,
        "parent_event_ids": event.parent_event_ids,
        "payload": event.payload,
    })
}

pub fn parse_bool(value: Option<&String>, default: bool) -> bool {
    match value.map(|s| s.trim().to_ascii_lowercase()) {
        Some(v) if matches!(v.as_str(), "1" | "true" | "yes" | "on") => true,
        Some(v) if matches!(v.as_str(), "0" | "false" | "no" | "off") => false,
        _ => default,
    }
}

pub fn harness_internal_enabled() -> bool {
    std::env::var("YOYO_HARNESS_INTERNAL")
        .map(|value| matches!(value.trim(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_adapter_boundary_is_explicit() {
        assert_eq!(STATE_ADAPTER_NAME, "yoagent-state");
        assert_eq!(STATE_ADAPTER_MODE, "path-dependency");
    }

    #[test]
    fn run_completed_payload_records_terminal_status() {
        let payload = run_completed_payload("completed", None);

        assert_eq!(payload["status"], "completed");
        assert!(payload["completed_at_ms"].as_u64().unwrap_or_default() > 0);
    }

    #[test]
    fn event_append_writes_jsonl() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let event = StateEvent {
            event_id: "evt-test".into(),
            event_type: EventType::RunStarted,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-test".into()),
            session_id: None,
            trace_id: "trace-test".into(),
            parent_event_ids: Vec::new(),
            payload: json!({"ok": true}),
        };
        append_event(&path, &event).unwrap();
        let raw = std::fs::read_to_string(path).unwrap();
        assert!(raw.contains("\"event_type\":\"RunStarted\""));
        assert!(raw.contains("\"kind\":\"RunStarted\""));
        assert!(raw.ends_with('\n'));
        let foundation_event: yoagent_state::Event = serde_json::from_str(raw.trim()).unwrap();
        assert_eq!(foundation_event.id.to_string(), "evt-test");
        assert_eq!(foundation_event.kind, "RunStarted");
    }

    #[test]
    fn event_log_is_readable_by_yoagent_state_store() {
        use yoagent_state::EventStore;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let event = StateEvent {
            event_id: "evt-upstream-readable".into(),
            event_type: EventType::FailureObserved,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-test".into()),
            session_id: None,
            trace_id: "trace-test".into(),
            parent_event_ids: Vec::new(),
            payload: json!({"source": "test", "error_preview": "boom"}),
        };
        append_event(&path, &event).unwrap();

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let events = runtime
            .block_on(async { yoagent_state::JsonlEventStore::new(&path).scan().await })
            .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id.to_string(), "evt-upstream-readable");
        assert_eq!(events[0].kind, "FailureObserved");
        assert_eq!(events[0].actor.kind, "system");
        assert_eq!(events[0].payload["source"], "test");
        assert_eq!(events[0].payload["_yoyo"]["run_id"], "run-test");
    }

    #[test]
    fn event_append_works_inside_existing_tokio_runtime() {
        use yoagent_state::EventStore;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let event = StateEvent {
            event_id: "evt-runtime-safe".into(),
            event_type: EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-runtime".into()),
            session_id: None,
            trace_id: "trace-runtime".into(),
            parent_event_ids: Vec::new(),
            payload: json!({"decision_type": "state_persistence", "decision": "use_yoagent_state_store"}),
        };

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let events = runtime
            .block_on(async {
                append_event(&path, &event).unwrap();
                yoagent_state::JsonlEventStore::new(&path).scan().await
            })
            .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id.to_string(), "evt-runtime-safe");
        assert_eq!(events[0].kind, "DecisionRecorded");
        assert_eq!(events[0].payload["decision"], "use_yoagent_state_store");
    }

    #[test]
    fn compatibility_reader_projects_canonical_yoagent_state_events() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let event = StateEvent {
            event_id: "evt-compat".into(),
            event_type: EventType::PatchEvaluated,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-compat".into()),
            session_id: None,
            trace_id: "trace-compat".into(),
            parent_event_ids: vec!["evt-parent".into()],
            payload: json!({"eval_id": "eval-1", "suite": "local-smoke"}),
        };
        append_event(&path, &event).unwrap();

        let events = read_compatibility_events(&path).unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["event_id"], "evt-compat");
        assert_eq!(events[0]["event_type"], "PatchEvaluated");
        assert_eq!(events[0]["actor"], "harness");
        assert_eq!(events[0]["run_id"], "run-compat");
        assert_eq!(events[0]["trace_id"], "trace-compat");
        assert_eq!(events[0]["parent_event_ids"][0], "evt-parent");
        assert_eq!(events[0]["payload"]["eval_id"], "eval-1");
        assert!(events[0]["payload"].get("_yoyo").is_none());
    }

    #[test]
    fn bool_parser_accepts_common_forms() {
        assert!(parse_bool(Some(&"true".to_string()), false));
        assert!(parse_bool(Some(&"1".to_string()), false));
        assert!(!parse_bool(Some(&"off".to_string()), true));
        assert!(parse_bool(None, true));
    }

    #[test]
    fn redaction_recurses_through_sensitive_keys_and_strings() {
        let payload = json!({
            "api_key": "sk-testsecret123456789",
            "nested": {
                "command": "export DEEPSEEK_API_KEY=sk-live123456789 && export GITHUB_TOKEN=ghp_abcdefghijklmnopqrstuvwxyz123456 && curl -H 'Authorization: Bearer abcdefghijklmnop'",
                "ssh_output": "-----BEGIN OPENSSH PRIVATE KEY-----\nabc123secretmaterial\n-----END OPENSSH PRIVATE KEY-----",
                "reasoning_content": "I should inspect private files before answering.",
                "reasoning_effort": "high",
                "answer_preview": "ok <think>hidden model reasoning</think> done",
                "prompt_cache_hit_tokens": 42,
                "items": [
                    {"password": "correct-horse-battery-staple"},
                    "AWS_ACCESS_KEY_ID=AKIATESTKEY12345678"
                ]
            }
        });

        let redacted = redact_state_payload(&payload);
        let raw = serde_json::to_string(&redacted).unwrap();

        assert!(!raw.contains("sk-testsecret123456789"));
        assert!(!raw.contains("sk-live123456789"));
        assert!(!raw.contains("abcdefghijklmnop"));
        assert!(!raw.contains("correct-horse-battery-staple"));
        assert!(!raw.contains("AKIATESTKEY12345678"));
        assert!(!raw.contains("ghp_abcdefghijklmnopqrstuvwxyz123456"));
        assert!(!raw.contains("abc123secretmaterial"));
        assert!(!raw.contains("I should inspect private files before answering"));
        assert!(!raw.contains("hidden model reasoning"));
        assert_eq!(redacted["api_key"], "[redacted]");
        assert_eq!(
            redacted["nested"]["reasoning_content"],
            "[redacted raw reasoning]"
        );
        assert_eq!(redacted["nested"]["reasoning_effort"], "high");
        assert_eq!(redacted["nested"]["items"][0]["password"], "[redacted]");
        assert_eq!(redacted["nested"]["prompt_cache_hit_tokens"], 42);
        assert!(raw.contains("sk-[redacted]"));
        assert!(raw.contains("GITHUB_TOKEN=[redacted]"));
        assert!(raw.contains("[redacted private key]"));
        assert!(raw.contains("[redacted raw reasoning]"));
        assert!(raw.contains("Authorization: [redacted]") || raw.contains("Bearer [redacted]"));
    }

    #[test]
    fn append_event_redacts_jsonl_and_sqlite_payloads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let event = StateEvent {
            event_id: "evt-secret".into(),
            event_type: EventType::ToolCallCompleted,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Tool,
            run_id: Some("run-test".into()),
            session_id: None,
            trace_id: "trace-test".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "tool_name": "bash",
                "args": {"api_key": "sk-verysecret123456"},
                "result_preview": "Authorization: Bearer secretbearer123456"
            }),
        };

        append_event(&path, &event).unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(!raw.contains("sk-verysecret123456"));
        assert!(!raw.contains("secretbearer123456"));
        assert!(raw.contains("[redacted]"));

        let sqlite_path = sqlite_projection_path(&path);
        let conn = Connection::open(sqlite_path).unwrap();
        let payload_json: String = conn
            .query_row(
                "SELECT payload_json FROM state_events WHERE event_id = 'evt-secret'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!payload_json.contains("sk-verysecret123456"));
        assert!(!payload_json.contains("secretbearer123456"));
    }

    #[test]
    fn sqlite_projection_migration_creates_version_ledger() {
        let dir = tempfile::tempdir().unwrap();
        let sqlite_path = dir.path().join("state.sqlite");

        let report = migrate_sqlite_projection(&sqlite_path).unwrap();

        assert_eq!(report.from_version, 0);
        assert_eq!(report.to_version, STATE_SQLITE_SCHEMA_VERSION);
        assert_eq!(report.applied_versions, vec![1, 2, 3]);

        let conn = Connection::open(&sqlite_path).unwrap();
        let user_version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        let migration_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM state_migrations WHERE version = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let second_migration_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM state_migrations WHERE version = 2",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let third_migration_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM state_migrations WHERE version = 3",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(user_version, i64::from(STATE_SQLITE_SCHEMA_VERSION));
        assert_eq!(migration_count, 1);
        assert_eq!(second_migration_count, 1);
        assert_eq!(third_migration_count, 1);
    }

    #[test]
    fn sqlite_projection_migration_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let sqlite_path = dir.path().join("state.sqlite");

        migrate_sqlite_projection(&sqlite_path).unwrap();
        let report = migrate_sqlite_projection(&sqlite_path).unwrap();

        assert_eq!(report.from_version, STATE_SQLITE_SCHEMA_VERSION);
        assert_eq!(report.to_version, STATE_SQLITE_SCHEMA_VERSION);
        assert!(report.applied_versions.is_empty());
    }

    #[test]
    fn sqlite_projection_rejects_newer_unknown_schema() {
        let dir = tempfile::tempdir().unwrap();
        let sqlite_path = dir.path().join("state.sqlite");
        let conn = Connection::open(&sqlite_path).unwrap();
        conn.execute_batch("PRAGMA user_version = 999;").unwrap();
        drop(conn);

        let err = migrate_sqlite_projection(&sqlite_path).unwrap_err();
        assert!(err.contains("newer than supported"));
    }

    #[test]
    fn harness_patch_serializes_stable_payload_shape() {
        let patch = HarnessPatch {
            patch_id: "patch-1".into(),
            kind: HarnessPatchKind::ToolSchema,
            status: HarnessPatchStatus::Proposed,
            base_harness_version: "0.1.0".into(),
            base_git_commit: Some("abc123".into()),
            state_version: 1,
            intent: "tighten patch proposal schema".into(),
            evidence_event_ids: Vec::new(),
            expected_effects: Vec::new(),
            risk_level: HarnessPatchRisk::Medium,
            eval_plan: Vec::new(),
            rollback_plan: Vec::new(),
            created_at_ms: 1,
        };
        let value = serde_json::to_value(&patch).unwrap();
        assert_eq!(value["patch_id"], "patch-1");
        assert_eq!(value["kind"], "tool_schema");
        assert_eq!(value["status"], "proposed");
        assert_eq!(value["risk_level"], "medium");
        assert_eq!(value["base_git_commit"], "abc123");
    }

    #[test]
    fn eval_result_serializes_status_and_metrics() {
        let eval = EvalResult {
            eval_id: "eval-1".into(),
            harness_version: "harness-1".into(),
            patch_id: Some("patch-1".into()),
            suite: "local-smoke".into(),
            status: EvalStatus::Passed,
            score: Some(0.95),
            passed: 19,
            failed: 1,
            metrics: json!({"cache_hit_ratio": 0.5}),
            failure_event_ids: vec!["evt-failure".into()],
            created_at_ms: 1,
        };
        let value = serde_json::to_value(&eval).unwrap();
        assert_eq!(value["eval_id"], "eval-1");
        assert_eq!(value["status"], "passed");
        assert_eq!(value["metrics"]["cache_hit_ratio"], 0.5);
    }

    #[test]
    fn cache_metrics_payload_uses_deepseek_input_as_miss_tokens() {
        let usage = yoagent::Usage {
            input: 30,
            output: 10,
            cache_read: 70,
            cache_write: 999,
            total_tokens: 110,
        };

        let payload = cache_metrics_payload("deepseek-v4-pro", &usage).unwrap();

        assert_eq!(payload["prompt_cache_hit_tokens"], 70);
        assert_eq!(payload["prompt_cache_miss_tokens"], 30);
        assert_eq!(payload["cache_hit_ratio"], 0.7);
    }

    #[test]
    fn cache_metrics_payload_skips_non_deepseek_models() {
        let usage = yoagent::Usage {
            input: 30,
            output: 10,
            cache_read: 70,
            cache_write: 20,
            total_tokens: 110,
        };

        assert!(cache_metrics_payload("gpt-4o", &usage).is_none());
    }

    #[test]
    fn append_event_updates_sqlite_projection_fail_soft() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let event = StateEvent {
            event_id: "evt-failure".into(),
            event_type: EventType::JsonOutputFailure,
            schema_version: 1,
            timestamp_ms: 10,
            actor: Actor::Harness,
            run_id: Some("run-1".into()),
            session_id: None,
            trace_id: "trace-1".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "json_output",
                "operation": "structured-extraction",
                "error_preview": "invalid json"
            }),
        };

        append_event(&path, &event).unwrap();

        let sqlite_path = sqlite_projection_path(&path);
        assert!(sqlite_path.exists());
        let conn = Connection::open(sqlite_path).unwrap();
        let event_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM state_events", [], |row| row.get(0))
            .unwrap();
        let failure_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM failures", [], |row| row.get(0))
            .unwrap();
        assert_eq!(event_count, 1);
        assert_eq!(failure_count, 1);
    }

    #[test]
    fn sqlite_projection_indexes_tool_schema_failures() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = StateEvent {
            event_id: "evt-schema-failure".into(),
            event_type: EventType::ToolSchemaFailure,
            schema_version: 1,
            timestamp_ms: 10,
            actor: Actor::Harness,
            run_id: Some("run-1".into()),
            session_id: None,
            trace_id: "trace-1".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "strict_tool_schema",
                "tool_name": "propose_edit",
                "schema_name": "propose_edit",
                "error_preview": "Retry tool call 'propose_edit' with strict JSON arguments only.",
                "valid": false
            }),
        };
        let raw = serde_json::to_string(&event).unwrap();
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();

        rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        let conn = Connection::open(sqlite_path).unwrap();
        let failure_kind: String = conn
            .query_row(
                "SELECT event_type FROM failures WHERE event_id = 'evt-schema-failure'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let source: String = conn
            .query_row(
                "SELECT source FROM failures WHERE event_id = 'evt-schema-failure'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(failure_kind, "ToolSchemaFailure");
        assert_eq!(source, "strict_tool_schema");
    }

    #[test]
    fn sqlite_projection_links_events_to_run_and_trace_nodes() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            StateEvent {
                event_id: "evt-run-start".into(),
                event_type: EventType::RunStarted,
                schema_version: 1,
                timestamp_ms: 1,
                actor: Actor::Harness,
                run_id: Some("run-graph".into()),
                session_id: None,
                trace_id: "trace-graph".into(),
                parent_event_ids: Vec::new(),
                payload: json!({"task": "graph run lineage"}),
            },
            StateEvent {
                event_id: "evt-tool".into(),
                event_type: EventType::ToolCallCompleted,
                schema_version: 1,
                timestamp_ms: 2,
                actor: Actor::Tool,
                run_id: Some("run-graph".into()),
                session_id: None,
                trace_id: "trace-graph".into(),
                parent_event_ids: Vec::new(),
                payload: json!({"tool_name": "read_file", "status": "ok"}),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 2);
        assert_eq!(report.relations, 5);
        let run_relations = query_sqlite_relations(&sqlite_path, "run-graph").unwrap();
        assert!(run_relations.iter().any(|relation| {
            relation.relation == "observed_in" && relation.src_id == "evt-run-start"
        }));
        assert!(run_relations.iter().any(|relation| {
            relation.relation == "observed_in" && relation.src_id == "evt-tool"
        }));
        let trace_relations = query_sqlite_relations(&sqlite_path, "trace-graph").unwrap();
        assert!(trace_relations.iter().any(|relation| {
            relation.relation == "traced_by" && relation.src_id == "evt-run-start"
        }));
        assert!(trace_relations
            .iter()
            .all(|relation| relation.dst_kind == "trace"));
    }

    #[test]
    fn sqlite_projection_links_file_events_to_file_refs() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            StateEvent {
                event_id: "evt-file-read".into(),
                event_type: EventType::FileRead,
                schema_version: 1,
                timestamp_ms: 1,
                actor: Actor::Tool,
                run_id: Some("run-file".into()),
                session_id: None,
                trace_id: "trace-file".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "tool_call_id": "call-read",
                    "path": "src/prompt.rs"
                }),
            },
            StateEvent {
                event_id: "evt-file-edited".into(),
                event_type: EventType::FileEdited,
                schema_version: 1,
                timestamp_ms: 2,
                actor: Actor::Harness,
                run_id: Some("run-file".into()),
                session_id: None,
                trace_id: "trace-file".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "deepseek_fim_apply",
                    "file_path": "src/state.rs",
                    "start_line": 10,
                    "end_line": 20
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 2);
        assert_eq!(report.relations, 7);
        let read_relations = query_sqlite_relations(&sqlite_path, "src/prompt.rs").unwrap();
        assert!(read_relations.iter().any(|relation| {
            relation.relation == "references_file"
                && relation.src_id == "evt-file-read"
                && relation.dst_kind == "file"
        }));
        let edit_relations = query_sqlite_relations(&sqlite_path, "src/state.rs").unwrap();
        assert!(edit_relations.iter().any(|relation| {
            relation.relation == "modified_file"
                && relation.src_id == "evt-file-edited"
                && relation.dst_kind == "file"
        }));
        assert!(edit_relations.iter().any(|relation| {
            relation.relation == "modified"
                && relation.src_id == "evt-file-edited"
                && relation.dst_kind == "file"
        }));
        let run_relations = query_sqlite_relations(&sqlite_path, "run-file").unwrap();
        assert!(run_relations.iter().any(|relation| {
            relation.relation == "observed_in" && relation.src_id == "evt-file-edited"
        }));
    }

    #[test]
    fn sqlite_projection_links_model_and_tool_call_nodes() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            StateEvent {
                event_id: "evt-model-started".into(),
                event_type: EventType::ModelCallStarted,
                schema_version: 1,
                timestamp_ms: 1,
                actor: Actor::Yoyo,
                run_id: Some("run-calls".into()),
                session_id: None,
                trace_id: "trace-calls".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "model_call_id": "model-call-1",
                    "model": "deepseek-v4-pro"
                }),
            },
            StateEvent {
                event_id: "evt-model-completed".into(),
                event_type: EventType::ModelCallCompleted,
                schema_version: 1,
                timestamp_ms: 2,
                actor: Actor::Yoyo,
                run_id: Some("run-calls".into()),
                session_id: None,
                trace_id: "trace-calls".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "model_call_id": "model-call-1",
                    "model": "deepseek-v4-pro",
                    "input_tokens": 100,
                    "output_tokens": 20
                }),
            },
            StateEvent {
                event_id: "evt-tool-started".into(),
                event_type: EventType::ToolCallStarted,
                schema_version: 1,
                timestamp_ms: 3,
                actor: Actor::Yoyo,
                run_id: Some("run-calls".into()),
                session_id: None,
                trace_id: "trace-calls".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "tool_call_id": "tool-call-1",
                    "tool_name": "read_file",
                    "args": {"path": "src/state.rs"}
                }),
            },
            StateEvent {
                event_id: "evt-tool-completed".into(),
                event_type: EventType::ToolCallCompleted,
                schema_version: 1,
                timestamp_ms: 4,
                actor: Actor::Tool,
                run_id: Some("run-calls".into()),
                session_id: None,
                trace_id: "trace-calls".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "tool_call_id": "tool-call-1",
                    "tool_name": "read_file",
                    "is_error": false
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 4);
        assert_eq!(report.relations, 18);
        let model_call_relations = query_sqlite_relations(&sqlite_path, "model-call-1").unwrap();
        assert!(model_call_relations.iter().any(|relation| {
            relation.relation == "records_model_call"
                && relation.src_id == "evt-model-started"
                && relation.dst_kind == "model_call"
        }));
        assert!(model_call_relations.iter().any(|relation| {
            relation.relation == "uses_model"
                && relation.src_id == "model-call-1"
                && relation.dst_id == "deepseek-v4-pro"
                && relation.dst_kind == "model"
        }));
        let tool_call_relations = query_sqlite_relations(&sqlite_path, "tool-call-1").unwrap();
        assert!(tool_call_relations.iter().any(|relation| {
            relation.relation == "records_tool_call"
                && relation.src_id == "evt-tool-started"
                && relation.dst_kind == "tool_call"
        }));
        assert!(tool_call_relations.iter().any(|relation| {
            relation.relation == "invokes_tool"
                && relation.src_id == "tool-call-1"
                && relation.dst_id == "read_file"
                && relation.dst_kind == "tool"
        }));
        let tool_relations = query_sqlite_relations(&sqlite_path, "read_file").unwrap();
        assert!(tool_relations.iter().any(|relation| {
            relation.relation == "invokes_tool" && relation.src_id == "tool-call-1"
        }));
    }

    #[test]
    fn sqlite_projection_links_eval_tasks_and_harness_versions() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            StateEvent {
                event_id: "evt-patch".into(),
                event_type: EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 1,
                actor: Actor::Harness,
                run_id: Some("run-task".into()),
                session_id: None,
                trace_id: "trace-task".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-task-1",
                    "kind": "eval",
                    "risk_level": "low",
                    "status": "proposed",
                    "base_harness_version": "genome-v1",
                    "intent": "track task lineage"
                }),
            },
            StateEvent {
                event_id: "evt-eval".into(),
                event_type: EventType::PatchEvaluated,
                schema_version: 1,
                timestamp_ms: 2,
                actor: Actor::Harness,
                run_id: Some("run-task".into()),
                session_id: None,
                trace_id: "trace-task".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "eval_id": "eval-task-1",
                    "harness_version": "genome-v2",
                    "suite": "fixtures:local-smoke",
                    "status": "failed",
                    "score": 0.5,
                    "metrics": {
                        "fixture_tasks": [
                            {"task_id": "context-miss", "passed": false},
                            {"task_id": "state-redaction", "passed": true}
                        ]
                    }
                }),
            },
            StateEvent {
                event_id: "evt-failure".into(),
                event_type: EventType::FailureObserved,
                schema_version: 1,
                timestamp_ms: 3,
                actor: Actor::Harness,
                run_id: Some("run-task".into()),
                session_id: None,
                trace_id: "trace-task".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "eval_fixture_task",
                    "eval_id": "eval-task-1",
                    "harness_version": "genome-v2",
                    "task_id": "context-miss",
                    "error_preview": "fixture task failed"
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 3);
        assert_eq!(report.failures, 1);
        assert_eq!(report.evals, 1);
        assert_eq!(report.relations, 16);
        let task_relations = query_sqlite_relations(&sqlite_path, "context-miss").unwrap();
        assert!(task_relations.iter().any(|relation| {
            relation.relation == "records_task"
                && relation.src_id == "evt-failure"
                && relation.dst_kind == "task"
        }));
        assert!(task_relations.iter().any(|relation| {
            relation.relation == "tested_by"
                && relation.src_id == "context-miss"
                && relation.dst_id == "eval-task-1"
                && relation.dst_kind == "eval"
        }));
        let harness_relations = query_sqlite_relations(&sqlite_path, "genome-v2").unwrap();
        assert!(harness_relations.iter().any(|relation| {
            relation.relation == "uses_harness_version" && relation.src_id == "evt-eval"
        }));
        assert!(harness_relations.iter().any(|relation| {
            relation.relation == "uses_harness_version" && relation.src_id == "evt-failure"
        }));
        let base_relations = query_sqlite_relations(&sqlite_path, "genome-v1").unwrap();
        assert!(base_relations.iter().any(|relation| {
            relation.relation == "based_on_harness_version" && relation.src_id == "evt-patch"
        }));
    }

    #[test]
    fn sqlite_projection_links_context_policy_and_tool_schema_nodes() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            StateEvent {
                event_id: "evt-context".into(),
                event_type: EventType::ContextBuilt,
                schema_version: 1,
                timestamp_ms: 10,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "context_policy": "deepseek_native",
                    "layout_version": 7,
                    "include_instruction_files": ["YOYO.md", "AGENTS.md"],
                    "stable_prefix_blocks": [
                        "deepseek_native_system_contract",
                        "strict_tool_schemas"
                    ],
                    "dynamic_suffix_blocks": [
                        "selected_recent_events",
                        "failure_evidence"
                    ],
                }),
            },
            StateEvent {
                event_id: "evt-schema-failure".into(),
                event_type: EventType::ToolSchemaFailure,
                schema_version: 1,
                timestamp_ms: 11,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "strict_tool_schema",
                    "tool_name": "propose_edit",
                    "schema_name": "propose_edit",
                    "schema_version": 1,
                    "error_preview": "Retry tool call 'propose_edit' with strict JSON arguments only.",
                    "valid": false
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 2);
        assert_eq!(report.failures, 1);
        assert_eq!(report.relations, 15);
        let context_relations = query_sqlite_relations(&sqlite_path, "evt-context").unwrap();
        assert!(context_relations.iter().any(|relation| {
            relation.relation == "uses_context_policy"
                && relation.dst_id == "deepseek_native"
                && relation.dst_kind == "context_policy"
        }));
        assert!(context_relations.iter().any(|relation| {
            relation.relation == "uses_prompt_layout"
                && relation.dst_id == "prompt_layout_v7"
                && relation.dst_kind == "prompt_layout"
        }));
        assert!(context_relations.iter().any(|relation| {
            relation.relation == "uses_prompt"
                && relation.dst_id == "prompt_layout_v7"
                && relation.dst_kind == "prompt_version"
        }));
        assert!(context_relations.iter().any(|relation| {
            relation.relation == "uses_context_block"
                && relation.dst_id == "strict_tool_schemas"
                && relation.dst_kind == "context_block"
        }));
        assert!(context_relations.iter().any(|relation| {
            relation.relation == "uses_instruction_file"
                && relation.dst_id == "YOYO.md"
                && relation.dst_kind == "instruction_file"
        }));

        let schema_relations = query_sqlite_relations(&sqlite_path, "evt-schema-failure").unwrap();
        assert!(schema_relations.iter().any(|relation| {
            relation.relation == "uses_schema"
                && relation.dst_id == "propose_edit"
                && relation.dst_kind == "tool_schema"
        }));
        assert!(schema_relations.iter().any(|relation| {
            relation.relation == "uses_schema_version"
                && relation.dst_id == "propose_edit@v1"
                && relation.dst_kind == "tool_schema_version"
        }));
        let run_relations = query_sqlite_relations(&sqlite_path, "run-1").unwrap();
        assert!(run_relations.iter().any(|relation| {
            relation.relation == "observed_in" && relation.src_id == "evt-context"
        }));
        let trace_relations = query_sqlite_relations(&sqlite_path, "trace-1").unwrap();
        assert!(trace_relations.iter().any(|relation| {
            relation.relation == "traced_by" && relation.src_id == "evt-schema-failure"
        }));
    }

    #[test]
    fn sqlite_projection_links_json_output_check_pass_to_schema_signal() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = StateEvent {
            event_id: "evt-json-pass".into(),
            event_type: EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-json".into()),
            session_id: None,
            trace_id: "trace-json".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "json_output",
                "decision_type": "deepseek_json_output_check",
                "check": "json-check",
                "decision": "passed",
                "schema_name": "summary",
                "attempt_count": 1,
                "retry_used": false,
                "attempt_statuses": ["parsed"]
            }),
        };
        std::fs::write(
            &events_path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        let schema_relations = query_sqlite_relations(&sqlite_path, "summary").unwrap();
        assert!(schema_relations.iter().any(|relation| {
            relation.relation == "supports_json_output_check"
                && relation.src_id == "summary"
                && relation.dst_id == "evt-json-pass"
                && relation.dst_kind == "event"
        }));
        let event_relations = query_sqlite_relations(&sqlite_path, "evt-json-pass").unwrap();
        assert!(event_relations.iter().any(|relation| {
            relation.relation == "uses_schema"
                && relation.dst_id == "summary"
                && relation.dst_kind == "tool_schema"
        }));
    }

    #[test]
    fn sqlite_projection_links_strict_tool_call_check_pass_to_schema_signals() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = StateEvent {
            event_id: "evt-strict-pass".into(),
            event_type: EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-strict".into()),
            session_id: None,
            trace_id: "trace-strict".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "deepseek_protocol_check",
                "decision_type": "deepseek_strict_tool_call_check",
                "check": "test-tool-call",
                "decision": "passed",
                "schema_count": 3,
                "schema_names": ["inspect_file", "propose_edit", "record_failure"],
                "selected_tool_count": 2,
                "selected_tool_names": ["inspect_file", "propose_edit"],
                "model": "deepseek-v4-pro",
                "thinking": "enabled",
                "reasoning_effort": "high",
                "stream": false,
                "max_tokens": 512
            }),
        };
        std::fs::write(
            &events_path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        let event_relations = query_sqlite_relations(&sqlite_path, "evt-strict-pass").unwrap();
        assert!(event_relations.iter().any(|relation| {
            relation.relation == "uses_schema"
                && relation.dst_id == "inspect_file"
                && relation.dst_kind == "tool_schema"
        }));
        assert!(event_relations.iter().any(|relation| {
            relation.relation == "uses_schema"
                && relation.dst_id == "propose_edit"
                && relation.dst_kind == "tool_schema"
        }));
        let schema_relations = query_sqlite_relations(&sqlite_path, "record_failure").unwrap();
        assert!(schema_relations.iter().any(|relation| {
            relation.relation == "supports_strict_tool_call_check"
                && relation.src_id == "record_failure"
                && relation.dst_id == "evt-strict-pass"
                && relation.dst_kind == "event"
        }));
    }

    #[test]
    fn sqlite_projection_links_transport_policy_check_pass_to_policy_signal() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = StateEvent {
            event_id: "evt-transport-pass".into(),
            event_type: EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-transport".into()),
            session_id: None,
            trace_id: "trace-transport".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "deepseek_protocol_check",
                "decision_type": "deepseek_transport_policy_check",
                "check": "transport-check",
                "decision": "passed",
                "transport_class": "rate_limited",
                "status": 429,
                "attempt": 0,
                "max_retries": 2,
                "retryable": true,
                "next_backoff_ms": 1000,
                "reason": "rate limit response can be retried with bounded backoff",
                "error_preview": "rate limit"
            }),
        };
        std::fs::write(
            &events_path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        let event_relations = query_sqlite_relations(&sqlite_path, "evt-transport-pass").unwrap();
        assert!(event_relations.iter().any(|relation| {
            relation.relation == "uses_transport_policy"
                && relation.dst_id == "deepseek_transport_policy"
                && relation.dst_kind == "policy"
        }));
        let class_relations =
            query_sqlite_relations(&sqlite_path, "transport_class:rate_limited").unwrap();
        assert!(class_relations.iter().any(|relation| {
            relation.relation == "supports_transport_policy_check"
                && relation.src_id == "transport_class:rate_limited"
                && relation.dst_id == "evt-transport-pass"
                && relation.dst_kind == "event"
        }));
    }

    #[test]
    fn sqlite_projection_links_thinking_protocol_check_pass_to_policy_signal() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = StateEvent {
            event_id: "evt-thinking-pass".into(),
            event_type: EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-thinking".into()),
            session_id: None,
            trace_id: "trace-thinking".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "deepseek_protocol_check",
                "decision_type": "deepseek_thinking_protocol_check",
                "check": "test-thinking",
                "decision": "passed",
                "diagnostic_source": "builtin-probe",
                "probe": {
                    "source": "builtin-probe",
                    "message_count": 2,
                    "assistant_tool_call_turns": 1,
                    "assistant_tool_call_turns_with_reasoning_content": 1,
                    "assistant_tool_call_turns_missing_reasoning_content": 0,
                    "tool_result_turns": 1
                }
            }),
        };
        std::fs::write(
            &events_path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        let event_relations = query_sqlite_relations(&sqlite_path, "evt-thinking-pass").unwrap();
        assert!(event_relations.iter().any(|relation| {
            relation.relation == "uses_thinking_protocol_policy"
                && relation.dst_id == "deepseek_thinking_protocol_policy"
                && relation.dst_kind == "policy"
        }));
        let probe_relations =
            query_sqlite_relations(&sqlite_path, "thinking_probe:builtin-probe").unwrap();
        assert!(probe_relations.iter().any(|relation| {
            relation.relation == "supports_thinking_protocol_check"
                && relation.src_id == "thinking_probe:builtin-probe"
                && relation.dst_id == "evt-thinking-pass"
                && relation.dst_kind == "event"
        }));
    }

    #[test]
    fn sqlite_projection_links_streaming_protocol_check_pass_to_policy_signal() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = StateEvent {
            event_id: "evt-stream-pass".into(),
            event_type: EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-stream".into()),
            session_id: None,
            trace_id: "trace-stream".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "source": "deepseek_protocol_check",
                "decision_type": "deepseek_streaming_protocol_check",
                "check": "stream-check",
                "decision": "passed",
                "content_chars": 4,
                "reasoning_content_chars": 16,
                "tool_call_count": 1,
                "finish_reason": "stop",
                "input_tokens": 12,
                "output_tokens": 3,
                "cache_hit_tokens": 8,
                "cache_miss_tokens": 4
            }),
        };
        std::fs::write(
            &events_path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        let event_relations = query_sqlite_relations(&sqlite_path, "evt-stream-pass").unwrap();
        assert!(event_relations.iter().any(|relation| {
            relation.relation == "uses_streaming_protocol_policy"
                && relation.dst_id == "deepseek_streaming_protocol_policy"
                && relation.dst_kind == "policy"
        }));
        let probe_relations =
            query_sqlite_relations(&sqlite_path, "streaming_probe:stream-check").unwrap();
        assert!(probe_relations.iter().any(|relation| {
            relation.relation == "supports_streaming_protocol_check"
                && relation.src_id == "streaming_probe:stream-check"
                && relation.dst_id == "evt-stream-pass"
                && relation.dst_kind == "event"
        }));
    }

    #[test]
    fn sqlite_projection_links_string_prompt_versions() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = StateEvent {
            event_id: "evt-context".into(),
            event_type: EventType::ContextBuilt,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-prompt".into()),
            session_id: None,
            trace_id: "trace-prompt".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "context_policy": "context_policy@v3",
                "prompt_version": "prompt@v9",
                "layout_version": "deepseek-native-v1",
                "stable_prefix_blocks": ["deepseek_native_system_contract"],
                "dynamic_suffix_blocks": ["selected_recent_events"]
            }),
        };
        std::fs::write(
            &events_path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        let prompt_relations = query_sqlite_relations(&sqlite_path, "prompt@v9").unwrap();
        assert!(prompt_relations.iter().any(|relation| {
            relation.relation == "uses_prompt"
                && relation.src_id == "evt-context"
                && relation.dst_kind == "prompt_version"
        }));
        let context_relations = query_sqlite_relations(&sqlite_path, "evt-context").unwrap();
        assert!(context_relations.iter().any(|relation| {
            relation.relation == "uses_context_policy" && relation.dst_id == "context_policy@v3"
        }));
    }

    #[test]
    fn rebuild_sqlite_projection_indexes_patch_eval_and_cache_events() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            StateEvent {
                event_id: "evt-patch".into(),
                event_type: EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 1,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-1",
                    "status": "proposed",
                    "kind": "tool_schema",
                    "risk_level": "medium",
                    "intent": "tighten schema",
                    "evidence_event_ids": ["evt-failure"]
                }),
            },
            StateEvent {
                event_id: "evt-eval".into(),
                event_type: EventType::PatchEvaluated,
                schema_version: 1,
                timestamp_ms: 2,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "eval_id": "eval-1",
                    "patch_id": "patch-1",
                    "harness_version": "harness-1",
                    "suite": "local-smoke",
                    "status": "passed",
                    "score": 1.0,
                    "metrics": {
                        "artifact_uri": ".yoyo/state/artifacts/evals/eval-1.json",
                        "artifacts": [{
                            "kind": "eval_report",
                            "uri": ".yoyo/state/artifacts/evals/eval-1.json",
                            "format": "json"
                        }]
                    },
                    "created_at_ms": 2
                }),
            },
            StateEvent {
                event_id: "evt-cache".into(),
                event_type: EventType::CacheMetricsRecorded,
                schema_version: 1,
                timestamp_ms: 3,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "model": "deepseek-v4-pro",
                    "prompt_cache_hit_tokens": 80,
                    "prompt_cache_miss_tokens": 20,
                    "cache_hit_ratio": 0.8
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 3);
        assert_eq!(report.patches, 2);
        assert_eq!(report.evals, 1);
        assert_eq!(report.cache_metrics, 1);
        assert_eq!(report.relations, 16);
        let conn = Connection::open(sqlite_path).unwrap();
        let status: String = conn
            .query_row(
                "SELECT status FROM harness_patches WHERE patch_id = 'patch-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let cache_ratio: f64 = conn
            .query_row("SELECT cache_hit_ratio FROM cache_metrics", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(status, "evaluated");
        assert_eq!(cache_ratio, 0.8);

        let relations =
            query_sqlite_relations(&dir.path().join("state.sqlite"), "patch-1").unwrap();
        assert!(relations
            .iter()
            .any(|relation| relation.relation == "uses_patch" && relation.src_id == "evt-eval"));
        assert!(relations.iter().any(|relation| {
            relation.relation == "addresses"
                && relation.src_id == "patch-1"
                && relation.dst_id == "evt-failure"
                && relation.dst_kind == "event"
        }));
        assert!(relations.iter().any(|relation| {
            relation.relation == "tested_by"
                && relation.src_id == "patch-1"
                && relation.dst_id == "eval-1"
                && relation.dst_kind == "eval"
        }));
        assert!(relations.iter().any(|relation| {
            relation.relation == "validated_by"
                && relation.src_id == "patch-1"
                && relation.dst_id == "eval-1"
                && relation.dst_kind == "eval"
        }));
        let eval_relations =
            query_sqlite_relations(&dir.path().join("state.sqlite"), "eval-1").unwrap();
        assert!(eval_relations.iter().any(|relation| {
            relation.relation == "has_artifact"
                && relation.dst_id == ".yoyo/state/artifacts/evals/eval-1.json"
        }));
        let evidence =
            query_sqlite_relations(&dir.path().join("state.sqlite"), "evt-failure").unwrap();
        assert!(evidence
            .iter()
            .any(|relation| relation.relation == "supported_by" && relation.src_id == "evt-patch"));
    }

    #[test]
    fn sqlite_projection_links_eval_protocol_metric_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = StateEvent {
            event_id: "evt-protocol-eval".into(),
            event_type: EventType::PatchEvaluated,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-protocol".into()),
            session_id: None,
            trace_id: "trace-protocol".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "eval_id": "eval-protocol",
                "harness_version": "genome-v1",
                "suite": "protocol-deepseek",
                "status": "passed",
                "score": 1.0,
                "passed": 5,
                "failed": 0,
                "metrics": {
                    "state_metrics": {
                        "deepseek_protocol_checks": 5,
                        "deepseek_protocol_passes": 5,
                        "deepseek_protocol_failures": 0,
                        "deepseek_strict_tool_call_checks": 1,
                        "deepseek_thinking_protocol_checks": 1,
                        "deepseek_streaming_protocol_checks": 1,
                        "deepseek_json_output_checks": 1,
                        "deepseek_transport_policy_checks": 1
                    }
                },
                "created_at_ms": 1
            }),
        };
        std::fs::write(
            &events_path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        let relations = query_sqlite_relations(&sqlite_path, "eval-protocol").unwrap();
        for check_id in [
            "deepseek_protocol_check:strict_tool_call",
            "deepseek_protocol_check:thinking_protocol",
            "deepseek_protocol_check:streaming",
            "deepseek_protocol_check:json_output",
            "deepseek_protocol_check:transport_policy",
        ] {
            assert!(relations.iter().any(|relation| {
                relation.relation == "covers_protocol_check"
                    && relation.src_id == "eval-protocol"
                    && relation.dst_id == check_id
                    && relation.dst_kind == "evidence"
            }));
        }
    }

    #[test]
    fn sqlite_projection_links_fixture_agent_attempt_changed_files_to_eval() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = StateEvent {
            event_id: "evt-agent-eval".into(),
            event_type: EventType::PatchEvaluated,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-agent-eval".into()),
            session_id: None,
            trace_id: "trace-agent-eval".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "eval_id": "eval-agent-1",
                "patch_id": "patch-agent-1",
                "harness_version": "genome-v1",
                "suite": "fixture-attempts:local-smoke",
                "status": "failed",
                "score": 0.0,
                "metrics": {
                    "fixture_agent_attempts": [
                        {
                            "task_id": "ranked-context-file-selection",
                            "passed": false,
                            "changed_files": [
                                "src/context.rs",
                                "src/eval_fixtures.rs",
                                "src/context.rs"
                            ],
                            "unexpected_changed_files": [
                                "src/eval_fixtures.rs"
                            ]
                        }
                    ]
                },
                "created_at_ms": 1
            }),
        };
        std::fs::write(
            &events_path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        let relations = query_sqlite_relations(&sqlite_path, "eval-agent-1").unwrap();
        let changed_files = relations
            .iter()
            .filter(|relation| relation.relation == "agent_attempt_changed_file")
            .map(|relation| relation.dst_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            changed_files,
            vec!["src/context.rs", "src/eval_fixtures.rs"]
        );
        assert!(relations.iter().any(|relation| {
            relation.relation == "agent_attempt_unexpected_file"
                && relation.src_id == "eval-agent-1"
                && relation.dst_id == "src/eval_fixtures.rs"
                && relation.dst_kind == "file"
        }));
        assert!(relations.iter().any(|relation| {
            relation.relation == "records_eval"
                && relation.src_id == "evt-agent-eval"
                && relation.dst_id == "eval-agent-1"
        }));
    }

    #[test]
    fn sqlite_projection_links_patch_addresses_failure_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = StateEvent {
            event_id: "evt-patch".into(),
            event_type: EventType::PatchProposed,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-addresses".into()),
            session_id: None,
            trace_id: "trace-addresses".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "patch_id": "patch-context-selector",
                "kind": "context_policy",
                "risk_level": "medium",
                "status": "proposed",
                "intent": "include files named by failing test output",
                "evidence_event_ids": ["evt-context-miss"]
            }),
        };
        std::fs::write(
            &events_path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        assert_eq!(report.patches, 1);
        assert_eq!(report.relations, 5);
        let patch_relations =
            query_sqlite_relations(&sqlite_path, "patch-context-selector").unwrap();
        assert!(patch_relations.iter().any(|relation| {
            relation.relation == "addresses"
                && relation.src_id == "patch-context-selector"
                && relation.dst_id == "evt-context-miss"
                && relation.dst_kind == "event"
        }));
        let evidence_relations = query_sqlite_relations(&sqlite_path, "evt-context-miss").unwrap();
        assert!(evidence_relations.iter().any(|relation| {
            relation.relation == "supported_by" && relation.src_id == "evt-patch"
        }));
    }

    #[test]
    fn sqlite_projection_indexes_harness_patch_lifecycle_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = StateEvent {
            event_id: "evt-patch".into(),
            event_type: EventType::PatchProposed,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-metadata".into()),
            session_id: None,
            trace_id: "trace-metadata".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "patch_id": "patch-context-policy",
                "kind": "context_policy",
                "risk_level": "medium",
                "status": "proposed",
                "base_harness_version": "genome-v1",
                "base_git_commit": "abc123",
                "state_version": 1,
                "intent": "rank files mentioned in failing output higher",
                "evidence_event_ids": ["evt-context-miss"],
                "expected_effects": ["lower context miss rate"],
                "eval_plan": ["cargo test context::tests::selects_failing_files_from_recent_state_events -- --nocapture"],
                "rollback_plan": ["reject patch and restore previous context policy"]
            }),
        };
        std::fs::write(
            &events_path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        assert_eq!(report.patches, 1);
        let conn = Connection::open(&sqlite_path).unwrap();
        let row = conn
            .query_row(
                r#"
                SELECT base_harness_version, state_version, expected_effects_json,
                       eval_plan_json, rollback_plan_json
                FROM harness_patches
                WHERE patch_id = 'patch-context-policy'
                "#,
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(row.0, "genome-v1");
        assert_eq!(row.1, 1);
        assert!(row.2.contains("lower context miss rate"));
        assert!(row
            .3
            .contains("selects_failing_files_from_recent_state_events"));
        assert!(row.4.contains("restore previous context policy"));
    }

    #[test]
    fn sqlite_projection_indexes_patch_comparison_lifecycle_status() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = StateEvent {
            event_id: "evt-compare".into(),
            event_type: EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-compare".into()),
            session_id: None,
            trace_id: "trace-compare".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_id": "decision-patch-compare-1",
                "decision_type": "harness_patch_comparison",
                "decision": "not_eligible",
                "rationale": "candidate passed but no promotion metric improved",
                "status": "compared",
                "patch_id": "patch-compare",
                "eval_id": "eval-candidate",
                "promotion_decision": {
                    "eligible": false,
                    "reason": "candidate passed but no promotion metric improved",
                    "candidate_eval_id": "eval-candidate"
                }
            }),
        };
        std::fs::write(
            &events_path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        assert_eq!(report.decisions, 1);
        assert_eq!(report.patches, 1);
        let conn = Connection::open(&sqlite_path).unwrap();
        let patch_status: String = conn
            .query_row(
                "SELECT status FROM harness_patches WHERE patch_id = 'patch-compare'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let decision_status: String = conn
            .query_row(
                "SELECT status FROM decisions WHERE decision_id = 'decision-patch-compare-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(patch_status, "compared");
        assert_eq!(decision_status, "compared");
        let relations = query_sqlite_relations(&sqlite_path, "decision-patch-compare-1").unwrap();
        assert!(relations.iter().any(|relation| {
            relation.relation == "records_decision"
                && relation.src_id == "evt-compare"
                && relation.dst_kind == "decision"
        }));
    }

    #[test]
    fn sqlite_projection_indexes_patch_risk_scored_lifecycle_status() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = StateEvent {
            event_id: "evt-risk-score".into(),
            event_type: EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-risk".into()),
            session_id: None,
            trace_id: "trace-risk".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_id": "decision-patch-risk-1",
                "decision_type": "harness_patch_risk_score",
                "decision": "high",
                "rationale": "patch kind 'permission_policy' assigned 'high' lifecycle risk",
                "status": "risk_scored",
                "patch_id": "patch-risk",
                "patch_event_id": "evt-propose",
                "kind": "permission_policy",
                "risk_level": "high",
                "risk_policy": "explicit_or_default_patch_risk"
            }),
        };
        std::fs::write(
            &events_path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        assert_eq!(report.decisions, 1);
        assert_eq!(report.patches, 1);
        let conn = Connection::open(&sqlite_path).unwrap();
        let row = conn
            .query_row(
                "SELECT status, kind, risk_level FROM harness_patches WHERE patch_id = 'patch-risk'",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(row.0, "risk_scored");
        assert_eq!(row.1, "permission_policy");
        assert_eq!(row.2, "high");
        let relations = query_sqlite_relations(&sqlite_path, "evt-propose").unwrap();
        assert!(relations.iter().any(|relation| {
            relation.relation == "derived_from"
                && relation.src_id == "evt-risk-score"
                && relation.dst_id == "evt-propose"
        }));
    }

    #[test]
    fn rebuild_sqlite_projection_indexes_hypothesis_and_decision_nodes() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            StateEvent {
                event_id: "evt-failure".into(),
                event_type: EventType::FailureObserved,
                schema_version: 1,
                timestamp_ms: 1,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "test",
                    "error_preview": "context omitted retry_state.rs"
                }),
            },
            StateEvent {
                event_id: "evt-hypothesis".into(),
                event_type: EventType::HypothesisCreated,
                schema_version: 1,
                timestamp_ms: 2,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "hypothesis_id": "hyp-1",
                    "failure_event_id": "evt-failure",
                    "summary": "missing retry state context caused the failure",
                    "confidence": 0.75,
                    "status": "open",
                    "evidence_event_ids": ["evt-failure"]
                }),
            },
            StateEvent {
                event_id: "evt-decision".into(),
                event_type: EventType::DecisionRecorded,
                schema_version: 1,
                timestamp_ms: 3,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "decision_id": "dec-1",
                    "decision_type": "promotion",
                    "decision": "reject patch until context selector is evaluated",
                    "rationale": "candidate has no baseline eval",
                    "status": "recorded",
                    "patch_id": "patch-1",
                    "eval_id": "eval-1",
                    "patch_event_id": "evt-promote"
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 3);
        assert_eq!(report.failures, 1);
        assert_eq!(report.hypotheses, 1);
        assert_eq!(report.decisions, 1);
        let conn = Connection::open(&sqlite_path).unwrap();
        let hypothesis_summary: String = conn
            .query_row(
                "SELECT summary FROM hypotheses WHERE hypothesis_id = 'hyp-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let decision: String = conn
            .query_row(
                "SELECT decision FROM decisions WHERE decision_id = 'dec-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(hypothesis_summary.contains("missing retry state"));
        assert!(decision.contains("reject patch"));

        let failure_relations = query_sqlite_relations(&sqlite_path, "evt-failure").unwrap();
        assert!(
            failure_relations
                .iter()
                .any(|relation| relation.relation == "explains"
                    && relation.src_id == "evt-hypothesis")
        );
        assert!(failure_relations.iter().any(|relation| {
            relation.relation == "caused_by"
                && relation.src_id == "evt-failure"
                && relation.dst_id == "hyp-1"
                && relation.dst_kind == "hypothesis"
        }));
        assert!(failure_relations.iter().any(|relation| {
            relation.relation == "supports"
                && relation.src_id == "evt-failure"
                && relation.dst_id == "hyp-1"
                && relation.dst_kind == "hypothesis"
        }));
        let decision_relations = query_sqlite_relations(&sqlite_path, "dec-1").unwrap();
        assert!(decision_relations
            .iter()
            .any(|relation| relation.relation == "records_decision"
                && relation.src_id == "evt-decision"));
        let patch_event_relations = query_sqlite_relations(&sqlite_path, "evt-promote").unwrap();
        assert!(patch_event_relations.iter().any(
            |relation| relation.relation == "derived_from" && relation.src_id == "evt-decision"
        ));
    }

    #[test]
    fn sqlite_projection_links_hypothesis_cause_and_support_edges() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            StateEvent {
                event_id: "evt-failure".into(),
                event_type: EventType::FailureObserved,
                schema_version: 1,
                timestamp_ms: 1,
                actor: Actor::Harness,
                run_id: Some("run-cause".into()),
                session_id: None,
                trace_id: "trace-cause".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "test",
                    "error_preview": "missing context for retry_state.rs"
                }),
            },
            StateEvent {
                event_id: "evt-log".into(),
                event_type: EventType::FailureObserved,
                schema_version: 1,
                timestamp_ms: 2,
                actor: Actor::Harness,
                run_id: Some("run-cause".into()),
                session_id: None,
                trace_id: "trace-cause".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "source": "test_output",
                    "error_preview": "retry_state.rs not found in context"
                }),
            },
            StateEvent {
                event_id: "evt-hypothesis".into(),
                event_type: EventType::HypothesisCreated,
                schema_version: 1,
                timestamp_ms: 3,
                actor: Actor::Harness,
                run_id: Some("run-cause".into()),
                session_id: None,
                trace_id: "trace-cause".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "hypothesis_id": "hyp-context-miss",
                    "failure_event_id": "evt-failure",
                    "summary": "context selector omitted retry_state.rs",
                    "evidence_event_ids": ["evt-failure", "evt-log"],
                    "confidence": 0.8,
                    "status": "open"
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 3);
        assert_eq!(report.failures, 2);
        assert_eq!(report.hypotheses, 1);
        let hypothesis_relations =
            query_sqlite_relations(&sqlite_path, "hyp-context-miss").unwrap();
        assert!(hypothesis_relations.iter().any(|relation| {
            relation.relation == "records_hypothesis"
                && relation.src_id == "evt-hypothesis"
                && relation.dst_kind == "hypothesis"
        }));
        assert!(hypothesis_relations.iter().any(|relation| {
            relation.relation == "caused_by"
                && relation.src_id == "evt-failure"
                && relation.dst_id == "hyp-context-miss"
        }));
        assert!(hypothesis_relations.iter().any(|relation| {
            relation.relation == "supports"
                && relation.src_id == "evt-log"
                && relation.dst_id == "hyp-context-miss"
        }));
    }

    #[test]
    fn sqlite_projection_links_hypothesis_contradiction_edges() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = StateEvent {
            event_id: "evt-hypothesis".into(),
            event_type: EventType::HypothesisCreated,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-contradicts".into()),
            session_id: None,
            trace_id: "trace-contradicts".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "hypothesis_id": "hyp-cache-regression",
                "failure_event_id": "evt-failure",
                "summary": "cache regression caused latency increase",
                "evidence_event_ids": ["evt-failure"],
                "contradicting_evidence_event_ids": ["evt-cache-ok"],
                "contradiction_event_ids": ["evt-selector-log"],
                "confidence": 0.45,
                "status": "open"
            }),
        };
        std::fs::write(
            &events_path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        assert_eq!(report.hypotheses, 1);
        assert_eq!(report.relations, 9);
        let hypothesis_relations =
            query_sqlite_relations(&sqlite_path, "hyp-cache-regression").unwrap();
        assert!(hypothesis_relations.iter().any(|relation| {
            relation.relation == "supports"
                && relation.src_id == "evt-failure"
                && relation.dst_id == "hyp-cache-regression"
                && relation.dst_kind == "hypothesis"
        }));
        assert!(hypothesis_relations.iter().any(|relation| {
            relation.relation == "contradicts"
                && relation.src_id == "evt-cache-ok"
                && relation.dst_id == "hyp-cache-regression"
                && relation.dst_kind == "hypothesis"
        }));
        assert!(hypothesis_relations.iter().any(|relation| {
            relation.relation == "contradicts"
                && relation.src_id == "evt-selector-log"
                && relation.dst_id == "hyp-cache-regression"
                && relation.dst_kind == "hypothesis"
        }));
    }

    #[test]
    fn sqlite_projection_links_stale_release_gate_to_eval() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            StateEvent {
                event_id: "evt-eval".into(),
                event_type: EventType::PatchEvaluated,
                schema_version: 1,
                timestamp_ms: 1,
                actor: Actor::Harness,
                run_id: Some("run-stale".into()),
                session_id: None,
                trace_id: "trace-stale".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "eval_id": "eval-old-pass",
                    "harness_version": "genome-v1",
                    "suite": "fixtures:local-smoke",
                    "status": "passed",
                    "score": 1.0
                }),
            },
            StateEvent {
                event_id: "evt-release-gate".into(),
                event_type: EventType::DecisionRecorded,
                schema_version: 1,
                timestamp_ms: 2,
                actor: Actor::Harness,
                run_id: Some("run-stale".into()),
                session_id: None,
                trace_id: "trace-stale".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "decision_type": "release_gate",
                    "decision": "block_release",
                    "suite": "local-smoke",
                    "reason": "latest eval is older than max age",
                    "last_eval_id": "eval-old-pass",
                    "last_eval_status": "passed",
                    "stale": true,
                    "replay_failures_after_eval": 0
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 2);
        assert_eq!(report.evals, 1);
        assert_eq!(report.decisions, 1);
        let eval_relations = query_sqlite_relations(&sqlite_path, "eval-old-pass").unwrap();
        assert!(eval_relations.iter().any(|relation| {
            relation.relation == "stale_due_to"
                && relation.src_id == "eval-old-pass"
                && relation.dst_id == "evt-release-gate"
                && relation.dst_kind == "event"
        }));
        let decision_relations = query_sqlite_relations(&sqlite_path, "evt-release-gate").unwrap();
        assert!(decision_relations.iter().any(|relation| {
            relation.relation == "stale_due_to" && relation.src_id == "eval-old-pass"
        }));
    }

    #[test]
    fn sqlite_projection_links_protocol_release_gate_freshness_to_evals() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            StateEvent {
                event_id: "evt-suite-eval".into(),
                event_type: EventType::PatchEvaluated,
                schema_version: 1,
                timestamp_ms: 2,
                actor: Actor::Harness,
                run_id: Some("run-release".into()),
                session_id: None,
                trace_id: "trace-release".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "eval_id": "eval-suite-pass",
                    "harness_version": "genome-v1",
                    "suite": "fixtures:local-smoke",
                    "status": "passed",
                    "score": 1.0
                }),
            },
            StateEvent {
                event_id: "evt-protocol-eval".into(),
                event_type: EventType::PatchEvaluated,
                schema_version: 1,
                timestamp_ms: 1,
                actor: Actor::Harness,
                run_id: Some("run-release".into()),
                session_id: None,
                trace_id: "trace-release".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "eval_id": "eval-protocol-pass",
                    "harness_version": "genome-v1",
                    "suite": "deepseek-protocol",
                    "status": "passed",
                    "score": 1.0,
                    "eval_type": "protocol"
                }),
            },
            StateEvent {
                event_id: "evt-release-gate".into(),
                event_type: EventType::DecisionRecorded,
                schema_version: 1,
                timestamp_ms: 3,
                actor: Actor::Harness,
                run_id: Some("run-release".into()),
                session_id: None,
                trace_id: "trace-release".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "decision_type": "release_gate",
                    "decision": "block_release",
                    "suite": "local-smoke",
                    "reason": "protocol eval is older than latest suite eval",
                    "last_eval_id": "eval-suite-pass",
                    "last_eval_status": "passed",
                    "stale": false,
                    "require_protocol": true,
                    "protocol_eval_id": "eval-protocol-pass",
                    "protocol_eval_status": "passed",
                    "protocol_older_than_eval": true,
                    "protocol_check_counts": {
                        "total": 5,
                        "passes": 5,
                        "strict": 1,
                        "thinking": 1,
                        "stream": 1,
                        "json": 1,
                        "transport": 1
                    }
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 3);
        assert_eq!(report.evals, 2);
        assert_eq!(report.decisions, 1);
        let protocol_relations =
            query_sqlite_relations(&sqlite_path, "eval-protocol-pass").unwrap();
        assert!(protocol_relations.iter().any(|relation| {
            relation.relation == "supports_release_gate"
                && relation.src_id == "eval-protocol-pass"
                && relation.dst_id == "evt-release-gate"
                && relation.dst_kind == "event"
        }));
        assert!(protocol_relations.iter().any(|relation| {
            relation.relation == "older_than_release_eval"
                && relation.src_id == "eval-protocol-pass"
                && relation.dst_id == "eval-suite-pass"
                && relation.dst_kind == "eval"
        }));
        let gate_relations = query_sqlite_relations(&sqlite_path, "evt-release-gate").unwrap();
        assert!(gate_relations.iter().any(|relation| {
            relation.relation == "requires_protocol_eval"
                && relation.src_id == "evt-release-gate"
                && relation.dst_id == "eval-protocol-pass"
        }));
        let covered_checks = gate_relations
            .iter()
            .filter(|relation| relation.relation == "covers_protocol_check")
            .map(|relation| relation.dst_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            covered_checks,
            [
                "deepseek_protocol_check:json_output",
                "deepseek_protocol_check:strict_tool_call",
                "deepseek_protocol_check:streaming",
                "deepseek_protocol_check:thinking_protocol",
                "deepseek_protocol_check:transport_policy",
            ]
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
        );
        let suite_relations = query_sqlite_relations(&sqlite_path, "eval-suite-pass").unwrap();
        assert!(suite_relations.iter().any(|relation| {
            relation.relation == "blocked_by_stale_protocol_eval"
                && relation.src_id == "eval-suite-pass"
                && relation.dst_id == "eval-protocol-pass"
        }));
    }

    #[test]
    fn sqlite_projection_links_release_gate_source_provenance_audit() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "source provenance audit found forbidden copied or leaked source claims",
                "source_provenance_passed": false,
                "source_provenance_findings": 2,
                "source_provenance_finding_summaries": [
                    "src/a.rs: source path escapes repository",
                    "src/b.rs: source file unreadable"
                ],
                "source_provenance_scan_source": "git"
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        assert_eq!(report.decisions, 1);
        let gate_relations = query_sqlite_relations(&sqlite_path, "evt-release-gate").unwrap();
        assert!(gate_relations.iter().any(|relation| {
            relation.relation == "blocked_by_source_provenance_audit"
                && relation.dst_id == "release_source_provenance_audit"
                && relation.dst_kind == "policy"
        }));
        assert!(gate_relations.iter().any(|relation| {
            relation.relation == "used_source_provenance_scan"
                && relation.dst_id == "source_provenance_scan:git"
                && relation.dst_kind == "policy"
        }));
        let audit_relations =
            query_sqlite_relations(&sqlite_path, "release_source_provenance_audit").unwrap();
        assert!(audit_relations.iter().any(|relation| {
            relation.relation == "blocks_release_gate"
                && relation.dst_id == "evt-release-gate"
                && relation.dst_kind == "event"
        }));
        let scan_relations =
            query_sqlite_relations(&sqlite_path, "source_provenance_scan:git").unwrap();
        assert!(scan_relations.iter().any(|relation| {
            relation.relation == "supports_source_provenance_audit"
                && relation.dst_id == "release_source_provenance_audit"
                && relation.dst_kind == "policy"
        }));
        assert!(gate_relations.iter().any(|relation| {
            relation.relation == "has_source_provenance_finding"
                && relation.dst_id
                    == "source_provenance_finding:src/a.rs: source path escapes repository"
                && relation.dst_kind == "evidence"
        }));
        let finding_relations = query_sqlite_relations(
            &sqlite_path,
            "source_provenance_finding:src/a.rs: source path escapes repository",
        )
        .unwrap();
        assert!(finding_relations.iter().any(|relation| {
            relation.relation == "supports_source_provenance_audit"
                && relation.dst_id == "release_source_provenance_audit"
                && relation.dst_kind == "policy"
        }));
        assert!(finding_relations.iter().any(|relation| {
            relation.relation == "blocks_release_gate"
                && relation.dst_id == "evt-release-gate"
                && relation.dst_kind == "event"
        }));
    }

    #[test]
    fn sqlite_projection_links_release_gate_missing_required_gates() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "latest eval is missing required gate evidence",
                "missing_required_gates": [
                    "cargo fmt --check",
                    "cargo clippy --all-targets --all-features -- -D warnings"
                ],
                "source_provenance_passed": true
            }),
        }];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        assert_eq!(report.decisions, 1);
        let gate_relations = query_sqlite_relations(&sqlite_path, "evt-release-gate").unwrap();
        assert!(gate_relations.iter().any(|relation| {
            relation.relation == "missing_required_gate"
                && relation.dst_id == "required_gate:cargo fmt --check"
                && relation.dst_kind == "evidence"
        }));
        let required_gate_relations =
            query_sqlite_relations(&sqlite_path, "required_gate:cargo fmt --check").unwrap();
        assert!(required_gate_relations.iter().any(|relation| {
            relation.relation == "blocks_release_gate"
                && relation.dst_id == "evt-release-gate"
                && relation.dst_kind == "event"
        }));
    }

    #[test]
    fn sqlite_projection_links_release_gate_fixture_breadth_minimum() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "latest eval fixture suite breadth is below required minimum",
                "last_eval_fixture_task_count": 244,
                "last_eval_fixture_command_count": 488,
                "min_fixture_task_count": 245,
                "min_fixture_command_count": 490,
                "fixture_breadth_satisfied": false
            }),
        };
        std::fs::write(
            &events_path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        assert_eq!(report.decisions, 1);
        let gate_relations = query_sqlite_relations(&sqlite_path, "evt-release-gate").unwrap();
        assert!(gate_relations.iter().any(|relation| {
            relation.relation == "fixture_breadth_below_minimum"
                && relation.dst_id == "release_fixture_breadth_minimum"
                && relation.dst_kind == "evidence"
        }));
        let breadth_relations =
            query_sqlite_relations(&sqlite_path, "release_fixture_breadth_minimum").unwrap();
        assert!(breadth_relations.iter().any(|relation| {
            relation.relation == "blocks_release_gate"
                && relation.dst_id == "evt-release-gate"
                && relation.dst_kind == "event"
        }));
    }

    #[test]
    fn sqlite_projection_links_release_gate_fixture_risk_minimum() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "latest eval fixture risk coverage is below required minimum",
                "last_eval_fixture_risk_labels": {
                    "high": 4,
                    "medium": 120
                },
                "min_fixture_risk_labels": {
                    "high": 5
                },
                "fixture_risk_satisfied": false
            }),
        };
        std::fs::write(
            &events_path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        assert_eq!(report.decisions, 1);
        let gate_relations = query_sqlite_relations(&sqlite_path, "evt-release-gate").unwrap();
        assert!(gate_relations.iter().any(|relation| {
            relation.relation == "fixture_risk_coverage_below_minimum"
                && relation.dst_id == "release_fixture_risk_minimum"
                && relation.dst_kind == "evidence"
        }));
        let risk_relations =
            query_sqlite_relations(&sqlite_path, "release_fixture_risk_minimum").unwrap();
        assert!(risk_relations.iter().any(|relation| {
            relation.relation == "blocks_release_gate"
                && relation.dst_id == "evt-release-gate"
                && relation.dst_kind == "event"
        }));
    }

    #[test]
    fn sqlite_projection_links_release_gate_fixture_agent_mutation_scope() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "latest eval has fixture agent mutation-scope failures: 1",
                "last_eval_id": "eval-scope-fail",
                "last_eval_status": "passed",
                "last_eval_mutation_scope_failures": 1,
                "last_eval_unexpected_changed_files": 3,
                "source_provenance_passed": true
            }),
        };
        std::fs::write(
            &events_path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        assert_eq!(report.decisions, 1);
        let gate_relations = query_sqlite_relations(&sqlite_path, "evt-release-gate").unwrap();
        assert!(gate_relations.iter().any(|relation| {
            relation.relation == "fixture_agent_mutation_scope_block"
                && relation.dst_id == "release_fixture_agent_mutation_scope"
                && relation.dst_kind == "evidence"
        }));
        let scope_relations =
            query_sqlite_relations(&sqlite_path, "release_fixture_agent_mutation_scope").unwrap();
        assert!(scope_relations.iter().any(|relation| {
            relation.relation == "blocks_release_gate"
                && relation.dst_id == "evt-release-gate"
                && relation.dst_kind == "event"
        }));
    }

    #[test]
    fn sqlite_projection_links_dirty_release_gate_eval() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "latest eval was run from a dirty worktree",
                "last_eval_id": "eval-dirty",
                "last_eval_git_dirty": true
            }),
        };
        std::fs::write(
            &events_path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        assert_eq!(report.decisions, 1);
        let gate_relations = query_sqlite_relations(&sqlite_path, "evt-release-gate").unwrap();
        assert!(gate_relations.iter().any(|relation| {
            relation.relation == "blocked_by_dirty_eval"
                && relation.dst_id == "eval-dirty"
                && relation.dst_kind == "eval"
        }));
        let eval_relations = query_sqlite_relations(&sqlite_path, "eval-dirty").unwrap();
        assert!(eval_relations.iter().any(|relation| {
            relation.relation == "dirty_eval_blocks_release_gate"
                && relation.dst_id == "evt-release-gate"
                && relation.dst_kind == "event"
        }));
    }

    #[test]
    fn sqlite_projection_links_dirty_protocol_release_gate_eval() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = StateEvent {
            event_id: "evt-release-gate".into(),
            event_type: EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-release".into()),
            session_id: None,
            trace_id: "trace-release".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "release_gate",
                "decision": "block_release",
                "suite": "local-smoke",
                "reason": "latest protocol eval was run from a dirty worktree",
                "last_eval_id": "eval-pass",
                "last_eval_git_dirty": false,
                "require_protocol": true,
                "protocol_eval_id": "eval-protocol-dirty",
                "protocol_eval_status": "passed",
                "protocol_eval_git_dirty": true
            }),
        };
        std::fs::write(
            &events_path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        assert_eq!(report.decisions, 1);
        let gate_relations = query_sqlite_relations(&sqlite_path, "evt-release-gate").unwrap();
        assert!(gate_relations.iter().any(|relation| {
            relation.relation == "blocked_by_dirty_eval"
                && relation.dst_id == "eval-protocol-dirty"
                && relation.dst_kind == "eval"
        }));
        let eval_relations = query_sqlite_relations(&sqlite_path, "eval-protocol-dirty").unwrap();
        assert!(eval_relations.iter().any(|relation| {
            relation.relation == "dirty_eval_blocks_release_gate"
                && relation.dst_id == "evt-release-gate"
                && relation.dst_kind == "event"
        }));
    }

    #[test]
    fn sqlite_projection_links_dirty_promotion_eval() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = StateEvent {
            event_id: "evt-promote".into(),
            event_type: EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-promote".into()),
            session_id: None,
            trace_id: "trace-promote".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "harness_patch_promotion",
                "decision": "block_promotion",
                "patch_id": "patch-1",
                "reason": "candidate eval was run from a dirty worktree",
                "promotion_decision": {
                    "eligible": false,
                    "reason": "candidate eval was run from a dirty worktree",
                    "candidate_eval_id": "eval-dirty-candidate"
                }
            }),
        };
        std::fs::write(
            &events_path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        assert_eq!(report.decisions, 1);
        let decision_relations = query_sqlite_relations(&sqlite_path, "evt-promote").unwrap();
        assert!(decision_relations.iter().any(|relation| {
            relation.relation == "blocked_by_dirty_eval"
                && relation.dst_id == "eval-dirty-candidate"
                && relation.dst_kind == "eval"
        }));
        let eval_relations = query_sqlite_relations(&sqlite_path, "eval-dirty-candidate").unwrap();
        assert!(eval_relations.iter().any(|relation| {
            relation.relation == "dirty_eval_blocks_promotion"
                && relation.dst_id == "evt-promote"
                && relation.dst_kind == "event"
        }));
    }

    #[test]
    fn sqlite_projection_links_dirty_promotion_protocol_eval() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = StateEvent {
            event_id: "evt-promote".into(),
            event_type: EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-promote".into()),
            session_id: None,
            trace_id: "trace-promote".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "harness_patch_promotion",
                "decision": "block_promotion",
                "patch_id": "patch-1",
                "reason": "latest protocol eval was run from a dirty worktree",
                "promotion_decision": {
                    "eligible": false,
                    "reason": "latest protocol eval was run from a dirty worktree",
                    "candidate_eval_id": "eval-candidate",
                    "protocol_eval_id": "eval-protocol-dirty"
                }
            }),
        };
        std::fs::write(
            &events_path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        assert_eq!(report.decisions, 1);
        let decision_relations = query_sqlite_relations(&sqlite_path, "evt-promote").unwrap();
        assert!(decision_relations.iter().any(|relation| {
            relation.relation == "blocked_by_dirty_eval"
                && relation.dst_id == "eval-protocol-dirty"
                && relation.dst_kind == "eval"
        }));
        let eval_relations = query_sqlite_relations(&sqlite_path, "eval-protocol-dirty").unwrap();
        assert!(eval_relations.iter().any(|relation| {
            relation.relation == "dirty_eval_blocks_promotion"
                && relation.dst_id == "evt-promote"
                && relation.dst_kind == "event"
        }));
    }

    #[test]
    fn sqlite_projection_links_promotion_protocol_eval_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = StateEvent {
            event_id: "evt-promote".into(),
            event_type: EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-promote".into()),
            session_id: None,
            trace_id: "trace-promote".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "harness_patch_promotion",
                "decision": "promote",
                "patch_id": "patch-1",
                "reason": "candidate score improved",
                "promotion_decision": {
                    "eligible": true,
                    "criterion": "pass_rate_improved",
                    "reason": "candidate score improves over baseline",
                    "candidate_eval_id": "eval-candidate",
                    "protocol_eval_id": "eval-protocol-pass"
                }
            }),
        };
        std::fs::write(
            &events_path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        assert_eq!(report.decisions, 1);
        let decision_relations = query_sqlite_relations(&sqlite_path, "evt-promote").unwrap();
        assert!(decision_relations.iter().any(|relation| {
            relation.relation == "requires_protocol_eval"
                && relation.dst_id == "eval-protocol-pass"
                && relation.dst_kind == "eval"
        }));
        let eval_relations = query_sqlite_relations(&sqlite_path, "eval-protocol-pass").unwrap();
        assert!(eval_relations.iter().any(|relation| {
            relation.relation == "supports_promotion"
                && relation.dst_id == "evt-promote"
                && relation.dst_kind == "event"
        }));
    }

    #[test]
    fn sqlite_projection_links_stale_promotion_protocol_eval() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = StateEvent {
            event_id: "evt-promote".into(),
            event_type: EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-promote".into()),
            session_id: None,
            trace_id: "trace-promote".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "harness_patch_promotion",
                "decision": "block_promotion",
                "patch_id": "patch-1",
                "reason": "latest protocol eval is older than candidate eval",
                "promotion_decision": {
                    "eligible": false,
                    "reason": "latest protocol eval is older than candidate eval",
                    "candidate_eval_id": "eval-candidate",
                    "protocol_eval_id": "eval-protocol-old"
                }
            }),
        };
        std::fs::write(
            &events_path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        assert_eq!(report.decisions, 1);
        let protocol_relations = query_sqlite_relations(&sqlite_path, "eval-protocol-old").unwrap();
        assert!(protocol_relations.iter().any(|relation| {
            relation.relation == "older_than_candidate_eval"
                && relation.dst_id == "eval-candidate"
                && relation.dst_kind == "eval"
        }));
        let candidate_relations = query_sqlite_relations(&sqlite_path, "eval-candidate").unwrap();
        assert!(candidate_relations.iter().any(|relation| {
            relation.relation == "blocked_by_stale_protocol_eval"
                && relation.dst_id == "eval-protocol-old"
                && relation.dst_kind == "eval"
        }));
    }

    #[test]
    fn sqlite_projection_links_promotion_fixture_risk_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = StateEvent {
            event_id: "evt-promote".into(),
            event_type: EventType::DecisionRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-promote".into()),
            session_id: None,
            trace_id: "trace-promote".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "decision_type": "harness_patch_promotion",
                "decision": "block_promotion",
                "patch_id": "patch-1",
                "reason": "baseline and candidate fixture suite risk-label coverage differ",
                "promotion_decision": {
                    "eligible": false,
                    "reason": "baseline and candidate fixture suite risk-label coverage differ",
                    "baseline_eval_id": "eval-base",
                    "candidate_eval_id": "eval-candidate"
                }
            }),
        };
        std::fs::write(
            &events_path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        assert_eq!(report.decisions, 1);
        let decision_relations = query_sqlite_relations(&sqlite_path, "evt-promote").unwrap();
        assert!(decision_relations.iter().any(|relation| {
            relation.relation == "promotion_fixture_risk_mismatch"
                && relation.dst_id == "promotion_fixture_risk_coverage"
                && relation.dst_kind == "evidence"
        }));
        let risk_relations =
            query_sqlite_relations(&sqlite_path, "promotion_fixture_risk_coverage").unwrap();
        assert!(risk_relations.iter().any(|relation| {
            relation.relation == "blocks_promotion"
                && relation.dst_id == "evt-promote"
                && relation.dst_kind == "event"
        }));
    }

    #[test]
    fn sqlite_projection_links_human_approval_to_patch_promotion() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            StateEvent {
                event_id: "evt-patch".into(),
                event_type: EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 1,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-risky",
                    "kind": "safety",
                    "risk_level": "high",
                    "status": "proposed"
                }),
            },
            StateEvent {
                event_id: "evt-approval".into(),
                event_type: EventType::HumanApprovalReceived,
                schema_version: 1,
                timestamp_ms: 2,
                actor: Actor::User,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-risky",
                    "approval_scope": "harness_patch_promotion",
                    "reason": "approved after review"
                }),
            },
            StateEvent {
                event_id: "evt-promote".into(),
                event_type: EventType::PatchPromoted,
                schema_version: 1,
                timestamp_ms: 3,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-risky",
                    "status": "promoted",
                    "approval_event_ids": ["evt-approval"]
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 3);
        assert_eq!(report.patches, 3);
        assert_eq!(report.relations, 12);
        let patch_relations = query_sqlite_relations(&sqlite_path, "patch-risky").unwrap();
        assert!(patch_relations.iter().any(|relation| {
            relation.relation == "approved_patch" && relation.src_id == "evt-approval"
        }));
        assert!(patch_relations.iter().any(|relation| {
            relation.relation == "promoted_by"
                && relation.src_id == "patch-risky"
                && relation.dst_id == "evt-promote"
        }));
        let approval_relations = query_sqlite_relations(&sqlite_path, "evt-approval").unwrap();
        assert!(approval_relations.iter().any(|relation| {
            relation.relation == "approved_by" && relation.src_id == "evt-promote"
        }));
    }

    #[test]
    fn sqlite_projection_links_memory_lifecycle_to_candidate_and_proposal() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            StateEvent {
                event_id: "evt-memory-proposed".into(),
                event_type: EventType::MemoryProposed,
                schema_version: 1,
                timestamp_ms: 1,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "candidate_id": "memory-hypothesis-hyp-context",
                    "source": "high_confidence_hypothesis",
                    "summary": "include files named in test output",
                    "evidence_event_ids": ["evt-failure"],
                    "status": "proposed"
                }),
            },
            StateEvent {
                event_id: "evt-memory-promoted".into(),
                event_type: EventType::MemoryPromoted,
                schema_version: 1,
                timestamp_ms: 2,
                actor: Actor::User,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "candidate_id": "memory-hypothesis-hyp-context",
                    "source": "high_confidence_hypothesis",
                    "summary": "include files named in test output",
                    "reason": "accepted after review",
                    "proposed_event_id": "evt-memory-proposed",
                    "evidence_event_ids": ["evt-failure"],
                    "status": "promoted"
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 2);
        assert_eq!(report.relations, 11);
        let memory_relations =
            query_sqlite_relations(&sqlite_path, "memory-hypothesis-hyp-context").unwrap();
        assert!(memory_relations.iter().any(|relation| {
            relation.relation == "proposes_memory" && relation.src_id == "evt-memory-proposed"
        }));
        assert!(memory_relations.iter().any(|relation| {
            relation.relation == "promoted_memory" && relation.src_id == "evt-memory-promoted"
        }));
        let proposal_relations =
            query_sqlite_relations(&sqlite_path, "evt-memory-proposed").unwrap();
        assert!(proposal_relations.iter().any(|relation| {
            relation.relation == "derived_from" && relation.src_id == "evt-memory-promoted"
        }));
        let evidence_relations = query_sqlite_relations(&sqlite_path, "evt-failure").unwrap();
        assert!(evidence_relations
            .iter()
            .any(|relation| relation.relation == "supported_by"));
    }

    #[test]
    fn sqlite_projection_links_revert_to_patch() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            StateEvent {
                event_id: "evt-patch".into(),
                event_type: EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 1,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-rollback",
                    "kind": "eval",
                    "risk_level": "medium",
                    "status": "proposed"
                }),
            },
            StateEvent {
                event_id: "evt-revert".into(),
                event_type: EventType::RevertPerformed,
                schema_version: 1,
                timestamp_ms: 2,
                actor: Actor::Harness,
                run_id: Some("run-1".into()),
                session_id: None,
                trace_id: "trace-1".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-rollback",
                    "status": "reverted",
                    "reason": "candidate regressed local smoke",
                    "reverted_commit": "abc123"
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.patches, 2);
        let status: String = Connection::open(&sqlite_path)
            .unwrap()
            .query_row(
                "SELECT status FROM harness_patches WHERE patch_id = 'patch-rollback'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "reverted");
        let relations = query_sqlite_relations(&sqlite_path, "patch-rollback").unwrap();
        assert!(relations.iter().any(|relation| {
            relation.relation == "reverted_patch" && relation.src_id == "evt-revert"
        }));
        assert!(relations.iter().any(|relation| {
            relation.relation == "reverted_by"
                && relation.src_id == "patch-rollback"
                && relation.dst_id == "evt-revert"
        }));
        let commit_relations = query_sqlite_relations(&sqlite_path, "abc123").unwrap();
        assert!(commit_relations.iter().any(|relation| {
            relation.relation == "reverted_commit" && relation.src_id == "evt-revert"
        }));
    }

    #[test]
    fn sqlite_projection_links_patch_rejection_lifecycle() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            StateEvent {
                event_id: "evt-patch".into(),
                event_type: EventType::PatchProposed,
                schema_version: 1,
                timestamp_ms: 1,
                actor: Actor::Harness,
                run_id: Some("run-reject".into()),
                session_id: None,
                trace_id: "trace-reject".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-rejected",
                    "kind": "context_policy",
                    "risk_level": "medium",
                    "status": "proposed"
                }),
            },
            StateEvent {
                event_id: "evt-eval".into(),
                event_type: EventType::PatchEvaluated,
                schema_version: 1,
                timestamp_ms: 2,
                actor: Actor::Harness,
                run_id: Some("run-reject".into()),
                session_id: None,
                trace_id: "trace-reject".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "eval_id": "eval-rejected",
                    "patch_id": "patch-rejected",
                    "harness_version": "genome-v2",
                    "suite": "local-smoke",
                    "status": "failed",
                    "score": 0.0
                }),
            },
            StateEvent {
                event_id: "evt-reject".into(),
                event_type: EventType::PatchRejected,
                schema_version: 1,
                timestamp_ms: 3,
                actor: Actor::Harness,
                run_id: Some("run-reject".into()),
                session_id: None,
                trace_id: "trace-reject".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "patch_id": "patch-rejected",
                    "reason": "candidate eval failed"
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 3);
        assert_eq!(report.patches, 3);
        let patch_relations = query_sqlite_relations(&sqlite_path, "patch-rejected").unwrap();
        assert!(patch_relations.iter().any(|relation| {
            relation.relation == "tested_by"
                && relation.src_id == "patch-rejected"
                && relation.dst_id == "eval-rejected"
        }));
        assert!(!patch_relations.iter().any(|relation| {
            relation.relation == "validated_by" && relation.src_id == "patch-rejected"
        }));
        assert!(patch_relations.iter().any(|relation| {
            relation.relation == "rejected_by"
                && relation.src_id == "patch-rejected"
                && relation.dst_id == "evt-reject"
        }));
    }

    #[test]
    fn sqlite_projection_links_commit_events_to_branch_and_files() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let events = [
            StateEvent {
                event_id: "evt-commit".into(),
                event_type: EventType::CommitCreated,
                schema_version: 1,
                timestamp_ms: 1,
                actor: Actor::Harness,
                run_id: Some("run-commit".into()),
                session_id: None,
                trace_id: "trace-commit".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "commit": "abc123",
                    "branch": "harness/file-lineage",
                    "message": "feat: add lineage",
                    "files": ["src/state.rs"]
                }),
            },
            StateEvent {
                event_id: "evt-revert".into(),
                event_type: EventType::RevertPerformed,
                schema_version: 1,
                timestamp_ms: 2,
                actor: Actor::Harness,
                run_id: Some("run-commit".into()),
                session_id: None,
                trace_id: "trace-commit".into(),
                parent_event_ids: Vec::new(),
                payload: json!({
                    "commit": "def456",
                    "reverted_commit": "abc123",
                    "branch": "harness/file-lineage",
                    "files": ["src/state.rs", "src/commands_git.rs"]
                }),
            },
        ];
        let raw = events
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        std::fs::write(&events_path, format!("{raw}\n")).unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 2);
        let branch_relations =
            query_sqlite_relations(&sqlite_path, "harness/file-lineage").unwrap();
        assert!(branch_relations.iter().any(|relation| {
            relation.relation == "on_branch"
                && relation.src_id == "evt-commit"
                && relation.dst_kind == "branch"
        }));
        assert!(branch_relations.iter().any(|relation| {
            relation.relation == "on_branch" && relation.src_id == "evt-revert"
        }));
        let file_relations = query_sqlite_relations(&sqlite_path, "src/commands_git.rs").unwrap();
        assert!(file_relations.iter().any(|relation| {
            relation.relation == "modified_file" && relation.src_id == "evt-revert"
        }));
        assert!(file_relations
            .iter()
            .any(|relation| relation.relation == "modified" && relation.src_id == "evt-revert"));
        let commit_relations = query_sqlite_relations(&sqlite_path, "abc123").unwrap();
        assert!(commit_relations.iter().any(|relation| {
            relation.relation == "records_commit" && relation.src_id == "evt-commit"
        }));
        assert!(commit_relations.iter().any(|relation| {
            relation.relation == "reverted_commit" && relation.src_id == "evt-revert"
        }));
    }

    #[test]
    fn sqlite_projection_links_self_filed_issue_intake_to_patch() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let sqlite_path = dir.path().join("state.sqlite");
        let event = StateEvent {
            event_id: "evt-issue-intake".into(),
            event_type: EventType::PatchProposed,
            schema_version: 1,
            timestamp_ms: 1,
            actor: Actor::Harness,
            run_id: Some("run-issue".into()),
            session_id: None,
            trace_id: "trace-issue".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "patch_id": "patch-issue-1",
                "kind": "eval",
                "risk_level": "low",
                "status": "proposed",
                "intent": "track repair churn",
                "intake_source": "self_filed_improvement_issue",
                "intake_kind": "issue",
                "intake_summary": "Track repair churn"
            }),
        };
        std::fs::write(
            &events_path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();

        let report = rebuild_sqlite_projection(&events_path, &sqlite_path).unwrap();

        assert_eq!(report.events, 1);
        assert_eq!(report.patches, 1);
        assert_eq!(report.relations, 5);
        let issue_relations = query_sqlite_relations(&sqlite_path, "issue:patch-issue-1").unwrap();
        assert!(issue_relations.iter().any(|relation| {
            relation.relation == "records_issue"
                && relation.src_id == "evt-issue-intake"
                && relation.dst_kind == "issue"
        }));
        assert!(issue_relations.iter().any(|relation| {
            relation.relation == "addresses_patch"
                && relation.src_id == "issue:patch-issue-1"
                && relation.dst_id == "patch-issue-1"
                && relation.dst_kind == "patch"
        }));
        let patch_relations = query_sqlite_relations(&sqlite_path, "patch-issue-1").unwrap();
        assert!(patch_relations.iter().any(|relation| {
            relation.relation == "uses_patch" && relation.src_id == "evt-issue-intake"
        }));
    }

    // --- panic hook + error context tests ---

    #[test]
    fn panic_hook_records_to_state() {
        let dir = tempfile::tempdir().unwrap();
        let events_path = dir.path().join("events.jsonl");
        let config = StateConfig {
            enabled: true,
            fail_soft: true,
            events_path: events_path.clone(),
            store_path: None,
        };
        init_global(config, json!({})).unwrap();

        install_panic_hook();

        let result = std::panic::catch_unwind(|| {
            panic!("test panic message 42");
        });

        assert!(result.is_err());

        let raw = std::fs::read_to_string(&events_path).unwrap();
        assert!(
            raw.contains("rust_panic"),
            "should contain rust_panic: {raw}"
        );
        assert!(
            raw.contains("test panic message 42"),
            "should contain panic message: {raw}"
        );
        assert!(
            raw.contains("panic_location"),
            "should contain panic_location: {raw}"
        );
        assert!(
            raw.contains("FailureObserved"),
            "should be FailureObserved: {raw}"
        );
    }

    #[test]
    fn run_completed_payload_includes_error_context() {
        let payload = run_completed_payload("error", Some("something went wrong"));
        assert_eq!(payload["status"], "error");
        assert_eq!(payload["error"], "something went wrong");
        assert!(payload["completed_at_ms"].as_u64().unwrap_or_default() > 0);
    }

    #[test]
    fn run_completed_payload_omits_error_when_none() {
        let payload = run_completed_payload("completed", None);
        assert_eq!(payload["status"], "completed");
        assert!(payload.get("error").is_none());
    }

    #[test]
    fn run_completed_payload_omits_error_when_empty() {
        let payload = run_completed_payload("error", Some(""));
        assert_eq!(payload["status"], "error");
        assert!(payload.get("error").is_none());
    }

    #[test]
    fn store_run_error_is_thread_safe() {
        // Verify thread-local doesn't leak across threads and functions correctly.
        store_run_error("first error");
        LAST_RUN_ERROR.with(|cell| {
            assert_eq!(cell.take(), Some("first error".to_string()));
        });
        // After take, it should be None
        LAST_RUN_ERROR.with(|cell| {
            assert!(cell.take().is_none());
        });
        // Set again
        store_run_error("second error");
        LAST_RUN_ERROR.with(|cell| {
            assert_eq!(cell.take(), Some("second error".to_string()));
        });
    }
}
