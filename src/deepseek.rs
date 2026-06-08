//! DeepSeek-native policy and protocol helpers.
//!
//! The runtime still delegates transport to `yoagent`'s OpenAI-compatible
//! provider. This module keeps DeepSeek-specific model names, prompt policy,
//! server-side cache metrics, and strict-schema helpers explicit so they can
//! evolve without turning the generic provider abstraction into a dumping
//! ground.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Component, Path};

pub const DEFAULT_MODEL: &str = "deepseek-v4-pro";
pub const FAST_MODEL: &str = "deepseek-v4-flash";
pub const DEFAULT_BASE_URL: &str = "https://api.deepseek.com/v1";
pub const FIM_BETA_BASE_URL: &str = "https://api.deepseek.com/beta";
pub const CHAT_COMPLETIONS_PATH: &str = "/chat/completions";
pub const FIM_COMPLETIONS_PATH: &str = "/completions";
pub const FIM_MAX_OUTPUT_TOKENS: u32 = 4_096;
pub const CONTEXT_WINDOW_TOKENS: u32 = 1_000_000;
pub const MAX_OUTPUT_TOKENS: u32 = 384_000;
pub const HARNESS_GENOME_VERSION: &str = "ds-harness-genome-v1";
pub const DEEPSEEK_PROMPT_CONTRACT_VERSION: u32 = 2;
pub const STRICT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeepSeekModel {
    V4Flash,
    V4Pro,
}

impl DeepSeekModel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V4Flash => FAST_MODEL,
            Self::V4Pro => DEFAULT_MODEL,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingEffort {
    High,
    Max,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum ThinkingMode {
    Disabled,
    Enabled { effort: ThinkingEffort },
}

impl ThinkingMode {
    pub fn native_default() -> Self {
        Self::Enabled {
            effort: ThinkingEffort::High,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeepSeekUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_hit_tokens: Option<u64>,
    pub cache_miss_tokens: Option<u64>,
}

impl DeepSeekUsage {
    pub fn cache_hit_ratio(&self) -> Option<f64> {
        let hit = self.cache_hit_tokens?;
        let miss = self.cache_miss_tokens?;
        let total = hit + miss;
        if total == 0 {
            None
        } else {
            Some(hit as f64 / total as f64)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeepSeekTransportPolicy {
    pub connect_timeout_ms: u64,
    pub request_timeout_ms: u64,
    pub max_retries: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub retry_statuses: Vec<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeepSeekTransportErrorClass {
    RateLimited,
    ServerError,
    Timeout,
    Network,
    Authentication,
    PermissionDenied,
    NotFound,
    ContextLength,
    InvalidRequest,
    MalformedResponse,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeepSeekTransportDecision {
    pub class: DeepSeekTransportErrorClass,
    pub status: Option<u16>,
    pub attempt: u32,
    pub max_retries: u32,
    pub retryable: bool,
    pub next_backoff_ms: Option<u64>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeepSeekRequestMeta {
    pub profile: String,
    pub prompt_layout_version: u32,
    pub thinking: ThinkingMode,
}

impl Default for DeepSeekRequestMeta {
    fn default() -> Self {
        Self {
            profile: "deepseek-native".to_string(),
            prompt_layout_version: 1,
            thinking: ThinkingMode::native_default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeepSeekHarnessGenome {
    pub version: String,
    pub model_routing_policy: ModelRoutingPolicy,
    pub transport_policy: DeepSeekTransportPolicy,
    pub thinking_policy: ThinkingPolicy,
    pub context_policy: ContextPolicy,
    pub prompt_layout_policy: PromptLayoutPolicy,
    pub cache_policy: CachePolicy,
    pub tool_schema_policy: ToolSchemaPolicy,
    pub json_policy: JsonPolicy,
    pub fim_policy: FimPolicy,
    pub test_policy: TestPolicy,
    pub repair_policy: RepairPolicy,
    pub memory_policy: MemoryPolicy,
    pub permission_policy: PermissionPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRoutingPolicy {
    pub root_cause_model: String,
    pub patch_planning_model: String,
    pub final_patch_model: String,
    pub memory_compression_model: String,
    pub quick_summary_model: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinkingPolicy {
    pub root_cause: ThinkingMode,
    pub patch_planning: ThinkingMode,
    pub risky_edits: ThinkingMode,
    pub deterministic_extraction: ThinkingMode,
    pub memory_compression: ThinkingMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextPolicy {
    pub recent_failure_limit: u32,
    pub changed_file_limit: u32,
    pub include_repo_map: bool,
    pub include_instruction_files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptLayoutPolicy {
    pub version: u32,
    pub stable_prefix_blocks: Vec<String>,
    pub dynamic_suffix_blocks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachePolicy {
    /// Keep high-reuse prompt blocks before dynamic evidence so DeepSeek's
    /// default server-side context cache can match repeated prefixes.
    pub stable_prefix: bool,
    /// Record `usage.prompt_cache_hit_tokens` and
    /// `usage.prompt_cache_miss_tokens`. DeepSeek does not require
    /// request-side `cache_control` markers for context caching.
    pub record_metrics: bool,
    /// Prefer prompt ordering that maximizes prefix reuse. This is a layout
    /// policy, not a request-side switch to enable caching.
    pub optimize_prompt_order: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSchemaPolicy {
    pub schema_version: u32,
    pub strict_critical_tools: bool,
    pub critical_schemas: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonPolicy {
    pub allow_for_extraction: bool,
    pub retry_once_on_invalid_or_empty: bool,
    pub record_failure_event: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonOutputAttempt {
    pub attempt: u32,
    pub status: JsonOutputAttemptStatus,
    pub content_preview: String,
    pub error_preview: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JsonOutputAttemptStatus {
    Empty,
    Invalid,
    Parsed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonOutputParseResult {
    pub value: Value,
    pub attempts: Vec<JsonOutputAttempt>,
    pub retry_used: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonOutputFailure {
    pub source: String,
    pub schema_name: Option<String>,
    pub attempts: Vec<JsonOutputAttempt>,
    pub retry_allowed: bool,
}

impl JsonOutputFailure {
    pub fn to_state_payload(&self) -> Value {
        json!({
            "source": "json_output",
            "operation": self.source,
            "schema_name": self.schema_name,
            "retry_allowed": self.retry_allowed,
            "attempts": self.attempts,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSchemaValidationReport {
    pub schema_name: String,
    pub valid: bool,
    pub missing_required: Vec<String>,
    pub unexpected_fields: Vec<String>,
    pub type_errors: Vec<String>,
    pub enum_errors: Vec<String>,
    pub repair_instruction: Option<String>,
}

impl ToolSchemaValidationReport {
    pub fn to_state_payload(&self) -> Value {
        json!({
            "source": "strict_tool_schema",
            "tool_name": self.schema_name,
            "schema_name": self.schema_name,
            "schema_version": STRICT_SCHEMA_VERSION,
            "error_preview": self.repair_instruction.as_deref().unwrap_or("strict tool schema validation failed"),
            "valid": self.valid,
            "missing_required": self.missing_required,
            "unexpected_fields": self.unexpected_fields,
            "type_errors": self.type_errors,
            "enum_errors": self.enum_errors,
            "repair_instruction": self.repair_instruction,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSchemaRepairAction {
    Accept,
    Retry,
    Abort,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSchemaRepairDecision {
    pub schema_name: String,
    pub action: ToolSchemaRepairAction,
    pub failed_attempt: u32,
    pub max_repair_turns: u32,
    pub should_record_failure: bool,
    pub instruction: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonOutputParseOptions {
    pub source: String,
    pub schema_name: Option<String>,
    pub retry_once_on_invalid_or_empty: bool,
}

impl JsonOutputParseOptions {
    pub fn extraction(source: impl Into<String>, schema_name: Option<String>) -> Self {
        Self {
            source: source.into(),
            schema_name,
            retry_once_on_invalid_or_empty: DeepSeekHarnessGenome::default()
                .json_policy
                .retry_once_on_invalid_or_empty,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FimPolicy {
    pub enabled: bool,
    pub allowed_scopes: Vec<String>,
    pub disallowed_scopes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FimRequestOptions {
    pub prompt: String,
    pub suffix: Option<String>,
    pub scope: String,
    pub max_tokens: u32,
    pub temperature: Option<f64>,
    pub stream: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FimFileRequestOptions {
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub scope: String,
    pub max_tokens: u32,
    pub temperature: Option<f64>,
    pub stream: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeepSeekFimRequest {
    pub endpoint: String,
    pub model: String,
    pub scope: String,
    pub auto_routing_enabled: bool,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FimRoutePromptOptions {
    pub input: String,
    pub tracked_files: Vec<String>,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FimRouteDecision {
    pub route: String,
    pub use_fim: bool,
    pub reason: String,
    pub file_path: Option<String>,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
    pub scope: String,
    pub command: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatPrefixRequestOptions {
    pub user_prompt: String,
    pub assistant_prefix: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: Option<f64>,
    pub stream: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeepSeekChatPrefixRequest {
    pub endpoint: String,
    pub model: String,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrictToolCallRequestOptions {
    pub user_prompt: String,
    pub model: String,
    pub tool_names: Vec<String>,
    pub thinking: ThinkingMode,
    pub max_tokens: u32,
    pub stream: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeepSeekStrictToolCallRequest {
    pub endpoint: String,
    pub model: String,
    pub tool_names: Vec<String>,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonOutputRequestOptions {
    pub user_prompt: String,
    pub model: String,
    pub schema_name: Option<String>,
    pub max_tokens: u32,
    pub stream: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeepSeekJsonOutputRequest {
    pub endpoint: String,
    pub model: String,
    pub schema_name: Option<String>,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeepSeekFimCompletion {
    pub text: String,
    pub finish_reason: Option<String>,
    pub usage: DeepSeekUsage,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeepSeekStreamingChatSummary {
    pub content: String,
    pub reasoning_content: String,
    pub tool_call_ids: Vec<String>,
    pub finish_reason: Option<String>,
    pub usage: DeepSeekUsage,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FimEditOptions {
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub completion: String,
    pub scope: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FimEditPlan {
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub scope: String,
    pub inserted_lines: usize,
    pub removed_lines: usize,
    pub risk_label: String,
    pub requires_explicit_apply: bool,
    pub patch: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeepSeekProtocolCheck {
    pub name: String,
    pub passed: bool,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestPolicy {
    pub required_gates: Vec<String>,
    pub protocol_gates: Vec<String>,
    pub benchmark_subset: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairPolicy {
    pub max_repair_turns: u32,
    pub record_failure_hypotheses: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryPolicy {
    pub inject_project_memory: bool,
    pub persist_raw_reasoning: bool,
    pub distill_reasoning_to_state: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionPolicy {
    pub shell_requires_approval: bool,
    pub destructive_requires_explicit_approval: bool,
    pub permission_policy_patches_need_human: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeepSeekRouteDecision {
    pub task: String,
    pub model: String,
    pub thinking: ThinkingMode,
    pub use_fim: bool,
    pub reason: String,
}

impl Default for DeepSeekHarnessGenome {
    fn default() -> Self {
        Self {
            version: HARNESS_GENOME_VERSION.to_string(),
            model_routing_policy: ModelRoutingPolicy {
                root_cause_model: DEFAULT_MODEL.to_string(),
                patch_planning_model: DEFAULT_MODEL.to_string(),
                final_patch_model: DEFAULT_MODEL.to_string(),
                memory_compression_model: FAST_MODEL.to_string(),
                quick_summary_model: FAST_MODEL.to_string(),
            },
            transport_policy: DeepSeekTransportPolicy {
                connect_timeout_ms: 10_000,
                request_timeout_ms: 120_000,
                max_retries: 2,
                initial_backoff_ms: 1_000,
                max_backoff_ms: 20_000,
                retry_statuses: vec![408, 409, 425, 429, 500, 502, 503, 504],
            },
            thinking_policy: ThinkingPolicy {
                root_cause: ThinkingMode::native_default(),
                patch_planning: ThinkingMode::native_default(),
                risky_edits: ThinkingMode::native_default(),
                deterministic_extraction: ThinkingMode::Disabled,
                memory_compression: ThinkingMode::Disabled,
            },
            context_policy: ContextPolicy {
                recent_failure_limit: 5,
                changed_file_limit: 12,
                include_repo_map: true,
                include_instruction_files: vec![
                    "YOYO.md".to_string(),
                    "AGENTS.md".to_string(),
                    "CLAUDE.md".to_string(),
                ],
            },
            prompt_layout_policy: PromptLayoutPolicy {
                version: 1,
                stable_prefix_blocks: vec![
                    "deepseek_native_system_contract".to_string(),
                    "safety_and_permissions".to_string(),
                    "strict_tool_schemas".to_string(),
                    "harness_policy_version".to_string(),
                    "project_instructions".to_string(),
                    "repo_map".to_string(),
                ],
                dynamic_suffix_blocks: vec![
                    "current_task".to_string(),
                    "selected_recent_events".to_string(),
                    "failing_test_files".to_string(),
                    "selected_files".to_string(),
                    "context_index_status".to_string(),
                    "latest_tool_outputs".to_string(),
                    "failure_evidence".to_string(),
                    "current_goal".to_string(),
                    "current_budget".to_string(),
                ],
            },
            cache_policy: CachePolicy {
                stable_prefix: true,
                record_metrics: true,
                optimize_prompt_order: true,
            },
            tool_schema_policy: ToolSchemaPolicy {
                schema_version: STRICT_SCHEMA_VERSION,
                strict_critical_tools: true,
                critical_schemas: strict_schema_names(),
            },
            json_policy: JsonPolicy {
                allow_for_extraction: true,
                retry_once_on_invalid_or_empty: true,
                record_failure_event: true,
            },
            fim_policy: FimPolicy {
                enabled: false,
                allowed_scopes: vec![
                    "single_function_body".to_string(),
                    "small_missing_block".to_string(),
                    "localized_completion".to_string(),
                    "one_method_repair".to_string(),
                ],
                disallowed_scopes: vec![
                    "multi_file_refactor".to_string(),
                    "architecture_change".to_string(),
                    "unknown_module_change".to_string(),
                    "security_sensitive_patch".to_string(),
                ],
            },
            test_policy: TestPolicy {
                required_gates: vec![
                    "cargo fmt --check".to_string(),
                    "cargo check".to_string(),
                    "cargo test --bin yyds -- --test-threads=1".to_string(),
                    "cargo test --test integration -- --test-threads=1".to_string(),
                ],
                protocol_gates: vec![
                    "cargo run --quiet --bin yyds -- deepseek test-tool-call --record --json".to_string(),
                    "cargo run --quiet --bin yyds -- deepseek test-thinking --record --json".to_string(),
                    "cargo run --quiet --bin yyds -- deepseek stream-check --record --json".to_string(),
                    "cargo run --quiet --bin yyds -- deepseek json-check --input '{\"ok\":true}' --record --json"
                        .to_string(),
                    "cargo run --quiet --bin yyds -- deepseek transport-check --status 429 --error 'rate limit' --record --json"
                        .to_string(),
                ],
                benchmark_subset: "local-smoke".to_string(),
            },
            repair_policy: RepairPolicy {
                max_repair_turns: 2,
                record_failure_hypotheses: true,
            },
            memory_policy: MemoryPolicy {
                inject_project_memory: true,
                persist_raw_reasoning: false,
                distill_reasoning_to_state: true,
            },
            permission_policy: PermissionPolicy {
                shell_requires_approval: true,
                destructive_requires_explicit_approval: true,
                permission_policy_patches_need_human: true,
            },
        }
    }
}

impl DeepSeekHarnessGenome {
    pub fn from_config(config: &HashMap<String, String>) -> Self {
        let mut genome = Self::default();

        let default_model = config
            .get("deepseek_model")
            .filter(|value| !value.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());
        let fast_model = config
            .get("deepseek_fast_model")
            .filter(|value| !value.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| FAST_MODEL.to_string());

        genome.model_routing_policy.root_cause_model = default_model.clone();
        genome.model_routing_policy.patch_planning_model = default_model.clone();
        genome.model_routing_policy.final_patch_model = default_model;
        genome.model_routing_policy.memory_compression_model = fast_model.clone();
        genome.model_routing_policy.quick_summary_model = fast_model;

        if let Some(mode) = config
            .get("deepseek_thinking")
            .and_then(|value| parse_thinking_mode_config(value))
        {
            genome.thinking_policy.root_cause = mode;
            genome.thinking_policy.patch_planning = mode;
            genome.thinking_policy.risky_edits = mode;
        }

        apply_route_config(
            config.get("deepseek_routing_planning"),
            &mut genome.model_routing_policy.patch_planning_model,
            Some(&mut genome.thinking_policy.patch_planning),
        );
        apply_route_config(
            config.get("deepseek_routing_root_cause"),
            &mut genome.model_routing_policy.root_cause_model,
            Some(&mut genome.thinking_policy.root_cause),
        );
        apply_route_config(
            config.get("deepseek_routing_summary"),
            &mut genome.model_routing_policy.quick_summary_model,
            Some(&mut genome.thinking_policy.memory_compression),
        );
        genome.model_routing_policy.memory_compression_model =
            genome.model_routing_policy.quick_summary_model.clone();

        if let Some(route) = config.get("deepseek_routing_local_edit") {
            let spec = parse_route_spec(route);
            if spec.fim {
                genome.fim_policy.enabled = true;
            }
        }

        genome.cache_policy.stable_prefix = crate::state::parse_bool(
            config.get("deepseek_cache_stable_prefix"),
            genome.cache_policy.stable_prefix,
        );
        genome.cache_policy.record_metrics = crate::state::parse_bool(
            config.get("deepseek_cache_record_metrics"),
            genome.cache_policy.record_metrics,
        );
        genome.cache_policy.optimize_prompt_order = crate::state::parse_bool(
            config.get("deepseek_cache_optimize_prompt_order"),
            genome.cache_policy.optimize_prompt_order,
        );
        if let Some(value) = config
            .get("deepseek_context_recent_failure_limit")
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|value| (1..=50).contains(value))
        {
            genome.context_policy.recent_failure_limit = value;
        }
        if let Some(value) = config
            .get("deepseek_context_changed_file_limit")
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|value| (1..=100).contains(value))
        {
            genome.context_policy.changed_file_limit = value;
        }
        genome.context_policy.include_repo_map = crate::state::parse_bool(
            config.get("deepseek_context_include_repo_map"),
            genome.context_policy.include_repo_map,
        );
        if let Some(files) = config
            .get("deepseek_context_include_instruction_files")
            .map(|raw| parse_instruction_file_list(raw))
            .filter(|files| !files.is_empty())
        {
            genome.context_policy.include_instruction_files = files;
        }
        if let Some(value) = config
            .get("deepseek_transport_request_timeout_ms")
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
        {
            genome.transport_policy.request_timeout_ms = value;
        }
        if let Some(value) = config
            .get("deepseek_transport_max_retries")
            .and_then(|value| value.parse::<u32>().ok())
        {
            genome.transport_policy.max_retries = value;
        }

        genome
    }
}

fn parse_instruction_file_list(raw: &str) -> Vec<String> {
    raw.trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|item| item.trim().trim_matches('"').trim_matches('\'').trim())
        .filter(|item| is_safe_instruction_file_path(item))
        .map(ToString::to_string)
        .collect()
}

fn is_safe_instruction_file_path(path: &str) -> bool {
    if path.is_empty() || path.contains(['\n', '\r', '\t']) {
        return false;
    }
    let parsed = Path::new(path);
    !parsed.is_absolute()
        && !parsed
            .components()
            .any(|component| matches!(component, Component::ParentDir))
}

pub fn active_harness_genome() -> DeepSeekHarnessGenome {
    let (config, _) = crate::config::load_deepseek_config_file();
    DeepSeekHarnessGenome::from_config(&config)
}

pub fn route_for_task(
    task: &str,
    genome: &DeepSeekHarnessGenome,
) -> Result<DeepSeekRouteDecision, String> {
    let task = normalize_route_task(task);
    let decision = match task.as_str() {
        "root_cause" => DeepSeekRouteDecision {
            task,
            model: genome.model_routing_policy.root_cause_model.clone(),
            thinking: genome.thinking_policy.root_cause,
            use_fim: false,
            reason: "root-cause analysis needs careful diagnosis".to_string(),
        },
        "planning" | "patch_planning" => DeepSeekRouteDecision {
            task,
            model: genome.model_routing_policy.patch_planning_model.clone(),
            thinking: genome.thinking_policy.patch_planning,
            use_fim: false,
            reason: "patch planning needs structured reasoning".to_string(),
        },
        "risky_edit" => DeepSeekRouteDecision {
            task,
            model: genome.model_routing_policy.final_patch_model.clone(),
            thinking: genome.thinking_policy.risky_edits,
            use_fim: false,
            reason: "risky edits keep thinking enabled by policy".to_string(),
        },
        "final_patch" => DeepSeekRouteDecision {
            task,
            model: genome.model_routing_policy.final_patch_model.clone(),
            thinking: genome.thinking_policy.risky_edits,
            use_fim: false,
            reason: "final patch synthesis uses the primary patch model".to_string(),
        },
        "summary" | "quick_summary" => DeepSeekRouteDecision {
            task,
            model: genome.model_routing_policy.quick_summary_model.clone(),
            thinking: ThinkingMode::Disabled,
            use_fim: false,
            reason: "quick summaries optimize for speed and cost".to_string(),
        },
        "memory" | "memory_compression" => DeepSeekRouteDecision {
            task,
            model: genome.model_routing_policy.memory_compression_model.clone(),
            thinking: genome.thinking_policy.memory_compression,
            use_fim: false,
            reason: "memory compression should be cheap and deterministic".to_string(),
        },
        "extraction" | "json_extraction" => DeepSeekRouteDecision {
            task,
            model: genome.model_routing_policy.quick_summary_model.clone(),
            thinking: genome.thinking_policy.deterministic_extraction,
            use_fim: false,
            reason: "structured extraction should be deterministic".to_string(),
        },
        "local_edit" | "fim" => DeepSeekRouteDecision {
            task,
            model: DeepSeekModel::V4Pro.as_str().to_string(),
            thinking: ThinkingMode::Disabled,
            use_fim: genome.fim_policy.enabled,
            reason: "localized edits use non-thinking FIM only when policy enables it".to_string(),
        },
        other => {
            return Err(format!(
                "unknown DeepSeek route task '{other}' (expected root-cause, planning, risky-edit, final-patch, summary, memory, extraction, or local-edit)"
            ));
        }
    };
    Ok(decision)
}

fn normalize_route_task(task: &str) -> String {
    task.trim().to_ascii_lowercase().replace(['-', ' '], "_")
}

fn apply_route_config(
    route: Option<&String>,
    model_slot: &mut String,
    thinking_slot: Option<&mut ThinkingMode>,
) {
    let Some(route) = route else {
        return;
    };
    let spec = parse_route_spec(route);
    if let Some(model) = spec.model {
        *model_slot = model;
    }
    if let (Some(mode), Some(slot)) = (spec.thinking, thinking_slot) {
        *slot = mode;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RouteSpec {
    model: Option<String>,
    thinking: Option<ThinkingMode>,
    fim: bool,
}

fn parse_route_spec(raw: &str) -> RouteSpec {
    let normalized = raw.trim().to_ascii_lowercase().replace('-', "_");
    let model = if normalized.contains("flash") {
        Some(FAST_MODEL.to_string())
    } else if normalized.contains("pro") {
        Some(DEFAULT_MODEL.to_string())
    } else if normalized.starts_with("deepseek_") {
        Some(normalized.replace('_', "-"))
    } else {
        None
    };
    let thinking = if normalized.contains("non_thinking")
        || normalized.contains("no_thinking")
        || normalized.contains("disabled")
    {
        Some(ThinkingMode::Disabled)
    } else if normalized.contains("thinking_max") || normalized.ends_with("_max") {
        Some(ThinkingMode::Enabled {
            effort: ThinkingEffort::Max,
        })
    } else if normalized.contains("thinking_high")
        || normalized.ends_with("_high")
        || normalized.contains("thinking")
    {
        Some(ThinkingMode::Enabled {
            effort: ThinkingEffort::High,
        })
    } else {
        None
    };

    RouteSpec {
        model,
        thinking,
        fim: normalized.contains("fim"),
    }
}

fn parse_thinking_mode_config(raw: &str) -> Option<ThinkingMode> {
    match raw.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "off" | "false" | "no" | "disabled" | "none" | "non_thinking" => {
            Some(ThinkingMode::Disabled)
        }
        "on" | "true" | "yes" | "enabled" | "high" | "thinking_high" => {
            Some(ThinkingMode::Enabled {
                effort: ThinkingEffort::High,
            })
        }
        "max" | "thinking_max" => Some(ThinkingMode::Enabled {
            effort: ThinkingEffort::Max,
        }),
        _ => None,
    }
}

fn normalize_deepseek_model(raw: &str) -> Result<String, String> {
    let normalized = raw.trim();
    if normalized.is_empty() {
        return Ok(DEFAULT_MODEL.to_string());
    }
    match normalized {
        DEFAULT_MODEL | FAST_MODEL => Ok(normalized.to_string()),
        _ => Err(format!(
            "unsupported DeepSeek model '{normalized}' (expected {DEFAULT_MODEL} or {FAST_MODEL})"
        )),
    }
}

pub fn classify_deepseek_transport_failure(
    status: Option<u16>,
    error_text: &str,
    attempt: u32,
    policy: &DeepSeekTransportPolicy,
) -> DeepSeekTransportDecision {
    let class = classify_transport_error(status, error_text);
    let transient = is_transient_transport_error(class)
        || status
            .map(|status| policy.retry_statuses.contains(&status))
            .unwrap_or(false);
    let retryable = transient && attempt < policy.max_retries;
    let next_backoff_ms = retryable.then(|| transport_backoff_ms(attempt, policy));
    DeepSeekTransportDecision {
        class,
        status,
        attempt,
        max_retries: policy.max_retries,
        retryable,
        next_backoff_ms,
        reason: transport_decision_reason(class, retryable, attempt, policy.max_retries),
    }
}

pub fn extract_deepseek_transport_status(error_text: &str) -> Option<u16> {
    error_text
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| part.len() == 3)
        .filter_map(|part| part.parse::<u16>().ok())
        .find(|status| (100..=599).contains(status))
}

pub fn deepseek_transport_failure_state_payload(
    source: &str,
    model: &str,
    error_text: &str,
    attempt: u32,
    policy: &DeepSeekTransportPolicy,
) -> Value {
    let status = extract_deepseek_transport_status(error_text);
    let decision = classify_deepseek_transport_failure(status, error_text, attempt, policy);
    json!({
        "source": source,
        "provider": "deepseek",
        "model": model,
        "failure_class": "transport",
        "transport_class": decision.class,
        "status": decision.status,
        "attempt": decision.attempt,
        "max_retries": decision.max_retries,
        "retryable": decision.retryable,
        "next_backoff_ms": decision.next_backoff_ms,
        "reason": decision.reason,
        "error_preview": error_text.chars().take(500).collect::<String>(),
    })
}

pub fn record_deepseek_transport_failure(source: &str, model: &str, error_text: &str) {
    let policy = active_harness_genome().transport_policy;
    crate::state::record(
        crate::state::EventType::FailureObserved,
        crate::state::Actor::Harness,
        deepseek_transport_failure_state_payload(
            source,
            model,
            error_text,
            policy.max_retries,
            &policy,
        ),
    );
}

fn classify_transport_error(status: Option<u16>, error_text: &str) -> DeepSeekTransportErrorClass {
    let lower = error_text.to_ascii_lowercase();
    if lower.contains("context length")
        || lower.contains("context_length")
        || lower.contains("maximum context")
        || lower.contains("too many tokens")
    {
        return DeepSeekTransportErrorClass::ContextLength;
    }
    if lower.contains("timed out") || lower.contains("timeout") || lower.contains("deadline") {
        return DeepSeekTransportErrorClass::Timeout;
    }
    if lower.contains("connection")
        || lower.contains("dns")
        || lower.contains("tls")
        || lower.contains("reset")
        || lower.contains("refused")
        || lower.contains("network")
    {
        return DeepSeekTransportErrorClass::Network;
    }
    if lower.contains("malformed")
        || lower.contains("deserialize")
        || lower.contains("invalid json")
        || lower.contains("parse")
    {
        return DeepSeekTransportErrorClass::MalformedResponse;
    }

    match status {
        Some(401) => DeepSeekTransportErrorClass::Authentication,
        Some(403) => DeepSeekTransportErrorClass::PermissionDenied,
        Some(404) => DeepSeekTransportErrorClass::NotFound,
        Some(408) => DeepSeekTransportErrorClass::Timeout,
        Some(429) => DeepSeekTransportErrorClass::RateLimited,
        Some(400 | 422) => DeepSeekTransportErrorClass::InvalidRequest,
        Some(500 | 502 | 503 | 504) => DeepSeekTransportErrorClass::ServerError,
        Some(status) if status >= 500 => DeepSeekTransportErrorClass::ServerError,
        Some(_) => DeepSeekTransportErrorClass::Unknown,
        None => DeepSeekTransportErrorClass::Unknown,
    }
}

fn is_transient_transport_error(class: DeepSeekTransportErrorClass) -> bool {
    matches!(
        class,
        DeepSeekTransportErrorClass::RateLimited
            | DeepSeekTransportErrorClass::ServerError
            | DeepSeekTransportErrorClass::Timeout
            | DeepSeekTransportErrorClass::Network
    )
}

fn transport_backoff_ms(attempt: u32, policy: &DeepSeekTransportPolicy) -> u64 {
    let exponent = attempt.min(16);
    let multiplier = 1_u64.checked_shl(exponent).unwrap_or(u64::MAX);
    policy
        .initial_backoff_ms
        .saturating_mul(multiplier)
        .min(policy.max_backoff_ms)
}

fn transport_decision_reason(
    class: DeepSeekTransportErrorClass,
    retryable: bool,
    attempt: u32,
    max_retries: u32,
) -> String {
    if retryable {
        return match class {
            DeepSeekTransportErrorClass::RateLimited => {
                "rate limit response can be retried with bounded backoff".to_string()
            }
            DeepSeekTransportErrorClass::ServerError => {
                "server-side DeepSeek error can be retried with bounded backoff".to_string()
            }
            DeepSeekTransportErrorClass::Timeout => {
                "timeout can be retried before the retry budget is exhausted".to_string()
            }
            DeepSeekTransportErrorClass::Network => {
                "network failure can be retried before the retry budget is exhausted".to_string()
            }
            _ => "transient failure can be retried".to_string(),
        };
    }
    if attempt >= max_retries && is_transient_transport_error(class) {
        "retry budget exhausted for transient DeepSeek failure".to_string()
    } else {
        "failure is not safe to retry without changing request or credentials".to_string()
    }
}

pub const DEEPSEEK_SYSTEM_CONTRACT_VERSION: &str = "deepseek_native_contract@v2";

pub fn stable_system_contract() -> &'static str {
    r#"# DeepSeek Native Harness Contract

- Preserve a deterministic prompt layout: stable policy, tools, project instructions, repo map, then dynamic task context.
- Use state, eval, logs, command output, and git evidence before drawing conclusions.
- Keep source inspection bounded: search first, then read targeted files, functions, or line ranges.
- Use thinking for root-cause analysis, patch planning, risky edits, and failed-task repair.
- Do not treat raw reasoning content as durable truth; distill decisions into hypotheses, evidence, risks, and patch intent.
- Do not claim completion unless code, artifacts, state events, or verification results support it.
- Prefer structured outputs for decisions that affect state, evaluation, or harness evolution.
- Keep edits narrow, verify with tests when practical, and record failures as evidence."#
}

pub fn response_format_json_object() -> Value {
    json!({ "type": "json_object" })
}

pub fn build_json_output_request(
    options: JsonOutputRequestOptions,
) -> Result<DeepSeekJsonOutputRequest, String> {
    let user_prompt = options.user_prompt.trim();
    if user_prompt.is_empty() {
        return Err("JSON output prompt cannot be empty".to_string());
    }
    if options.max_tokens == 0 || options.max_tokens > MAX_OUTPUT_TOKENS {
        return Err(format!(
            "JSON output max_tokens must be between 1 and {MAX_OUTPUT_TOKENS}"
        ));
    }
    let model = normalize_deepseek_model(&options.model)?;
    let mut content = user_prompt.to_string();
    if let Some(schema_name) = options.schema_name.as_deref() {
        content.push_str("\n\n");
        content.push_str(&json_retry_instruction(Some(schema_name)));
    } else {
        content.push_str("\n\n");
        content.push_str(&json_retry_instruction(None));
    }

    Ok(DeepSeekJsonOutputRequest {
        endpoint: format!("{DEFAULT_BASE_URL}{CHAT_COMPLETIONS_PATH}"),
        model: model.clone(),
        schema_name: options.schema_name,
        payload: json!({
            "model": model,
            "messages": [{
                "role": "user",
                "content": content,
            }],
            "response_format": response_format_json_object(),
            "thinking": deepseek_thinking_payload(ThinkingMode::Disabled),
            "max_tokens": options.max_tokens,
            "stream": options.stream,
        }),
    })
}

pub fn build_strict_tool_call_request(
    options: StrictToolCallRequestOptions,
) -> Result<DeepSeekStrictToolCallRequest, String> {
    let user_prompt = options.user_prompt.trim();
    if user_prompt.is_empty() {
        return Err("strict tool-call prompt cannot be empty".to_string());
    }
    if options.max_tokens == 0 || options.max_tokens > MAX_OUTPUT_TOKENS {
        return Err(format!(
            "strict tool-call max_tokens must be between 1 and {MAX_OUTPUT_TOKENS}"
        ));
    }
    let model = normalize_deepseek_model(&options.model)?;
    let mut tools = Vec::new();
    let mut tool_names = Vec::new();
    if options.tool_names.is_empty() {
        tools = strict_schema_suite();
        tool_names = strict_schema_names();
    } else {
        for name in options.tool_names {
            if tool_names.iter().any(|existing| existing == &name) {
                return Err(format!("duplicate strict schema '{name}'"));
            }
            let schema = strict_schema_by_name(&name)
                .ok_or_else(|| format!("unknown strict schema '{name}'"))?;
            tools.push(schema);
            tool_names.push(name);
        }
    }
    if tools.is_empty() {
        return Err("strict tool-call request needs at least one tool schema".to_string());
    }

    let mut payload = json!({
        "model": model,
        "messages": [{
            "role": "user",
            "content": user_prompt
        }],
        "tools": tools,
        "tool_choice": "auto",
        "max_tokens": options.max_tokens,
        "stream": options.stream,
        "thinking": deepseek_thinking_payload(options.thinking),
    });
    if let ThinkingMode::Enabled { effort } = options.thinking {
        payload["reasoning_effort"] = json!(match effort {
            ThinkingEffort::High => "high",
            ThinkingEffort::Max => "max",
        });
    }

    Ok(DeepSeekStrictToolCallRequest {
        endpoint: format!("{DEFAULT_BASE_URL}{CHAT_COMPLETIONS_PATH}"),
        model,
        tool_names,
        payload,
    })
}

fn deepseek_thinking_payload(mode: ThinkingMode) -> Value {
    match mode {
        ThinkingMode::Disabled => json!({ "type": "disabled" }),
        ThinkingMode::Enabled { .. } => json!({ "type": "enabled" }),
    }
}

pub fn json_retry_instruction(schema_name: Option<&str>) -> String {
    match schema_name {
        Some(schema) => format!(
            "Return only valid JSON matching schema '{schema}'. Do not include markdown, commentary, or empty content."
        ),
        None => "Return only valid JSON. Do not include markdown, commentary, or empty content."
            .to_string(),
    }
}

pub fn parse_json_output_attempts<I, S>(
    attempts: I,
    options: JsonOutputParseOptions,
) -> Result<JsonOutputParseResult, JsonOutputFailure>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut observed = Vec::new();
    let max_attempts = if options.retry_once_on_invalid_or_empty {
        2
    } else {
        1
    };

    for (index, raw) in attempts.into_iter().take(max_attempts).enumerate() {
        let attempt_number = (index + 1) as u32;
        let content = raw.as_ref();
        if content.trim().is_empty() {
            observed.push(JsonOutputAttempt {
                attempt: attempt_number,
                status: JsonOutputAttemptStatus::Empty,
                content_preview: String::new(),
                error_preview: Some("empty JSON output".to_string()),
            });
            continue;
        }

        match serde_json::from_str::<Value>(content) {
            Ok(value) => {
                observed.push(JsonOutputAttempt {
                    attempt: attempt_number,
                    status: JsonOutputAttemptStatus::Parsed,
                    content_preview: preview(content, 1000),
                    error_preview: None,
                });
                return Ok(JsonOutputParseResult {
                    value,
                    retry_used: observed.len() > 1,
                    attempts: observed,
                });
            }
            Err(e) => {
                observed.push(JsonOutputAttempt {
                    attempt: attempt_number,
                    status: JsonOutputAttemptStatus::Invalid,
                    content_preview: preview(content, 1000),
                    error_preview: Some(preview(&e.to_string(), 500)),
                });
            }
        }
    }

    Err(JsonOutputFailure {
        source: options.source,
        schema_name: options.schema_name,
        attempts: observed,
        retry_allowed: options.retry_once_on_invalid_or_empty,
    })
}

pub fn record_json_output_failure(failure: &JsonOutputFailure) {
    crate::state::record(
        crate::state::EventType::JsonOutputFailure,
        crate::state::Actor::Harness,
        failure.to_state_payload(),
    );
}

pub fn build_chat_prefix_request(
    options: ChatPrefixRequestOptions,
) -> Result<DeepSeekChatPrefixRequest, String> {
    let user_prompt = options.user_prompt.trim();
    if user_prompt.is_empty() {
        return Err("chat prefix user prompt cannot be empty".to_string());
    }
    let assistant_prefix = options.assistant_prefix.trim();
    if assistant_prefix.is_empty() {
        return Err("chat prefix assistant prefix cannot be empty".to_string());
    }
    if options.max_tokens == 0 || options.max_tokens > MAX_OUTPUT_TOKENS {
        return Err(format!(
            "chat prefix max_tokens must be between 1 and {MAX_OUTPUT_TOKENS}"
        ));
    }
    let model = normalize_deepseek_model(&options.model)?;
    let mut payload = json!({
        "model": model,
        "messages": [
            {
                "role": "user",
                "content": user_prompt,
            },
            {
                "role": "assistant",
                "content": assistant_prefix,
                "prefix": true,
            },
        ],
        "max_tokens": options.max_tokens,
        "stream": options.stream,
    });
    if let Some(temperature) = options.temperature {
        payload["temperature"] = json!(temperature);
    }

    Ok(DeepSeekChatPrefixRequest {
        endpoint: format!("{FIM_BETA_BASE_URL}{CHAT_COMPLETIONS_PATH}"),
        model,
        payload,
    })
}

pub fn build_fim_completion_request(
    options: FimRequestOptions,
    policy: &FimPolicy,
) -> Result<DeepSeekFimRequest, String> {
    let prompt = options.prompt.trim();
    if prompt.is_empty() {
        return Err("FIM prompt cannot be empty".to_string());
    }
    if options.max_tokens == 0 || options.max_tokens > FIM_MAX_OUTPUT_TOKENS {
        return Err(format!(
            "FIM max_tokens must be between 1 and {FIM_MAX_OUTPUT_TOKENS}"
        ));
    }

    let scope = normalize_scope(&options.scope);
    if policy.disallowed_scopes.iter().any(|item| item == &scope) {
        return Err(format!("FIM scope '{scope}' is disallowed by policy"));
    }
    if !policy.allowed_scopes.iter().any(|item| item == &scope) {
        return Err(format!(
            "FIM scope '{scope}' is not in the allowed local-edit scope set"
        ));
    }

    let mut payload = json!({
        "model": DeepSeekModel::V4Pro.as_str(),
        "prompt": prompt,
        "max_tokens": options.max_tokens,
        "stream": options.stream,
    });
    if let Some(suffix) = options
        .suffix
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        payload["suffix"] = Value::String(suffix.to_string());
    }
    if let Some(temperature) = options.temperature {
        payload["temperature"] = json!(temperature);
    }

    Ok(DeepSeekFimRequest {
        endpoint: format!("{FIM_BETA_BASE_URL}{FIM_COMPLETIONS_PATH}"),
        model: DeepSeekModel::V4Pro.as_str().to_string(),
        scope,
        auto_routing_enabled: policy.enabled,
        payload,
    })
}

pub fn build_fim_completion_request_for_file(
    options: FimFileRequestOptions,
    policy: &FimPolicy,
) -> Result<DeepSeekFimRequest, String> {
    validate_repo_relative_path(&options.file_path)?;
    if options.start_line == 0 || options.end_line < options.start_line {
        return Err("FIM file range must be a 1-based inclusive line range".to_string());
    }
    let content = std::fs::read_to_string(&options.file_path)
        .map_err(|e| format!("could not read {}: {e}", options.file_path))?;
    let normalized_content = content.replace("\r\n", "\n").replace('\r', "\n");
    let lines = normalized_content
        .lines()
        .map(|line| line.to_string())
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return Err("FIM file target is empty".to_string());
    }
    if options.end_line > lines.len() {
        return Err(format!(
            "FIM file range {}-{} is past end of file ({} lines)",
            options.start_line,
            options.end_line,
            lines.len()
        ));
    }

    let prompt = lines[..options.start_line - 1].join("\n");
    let suffix = lines[options.end_line..].join("\n");
    build_fim_completion_request(
        FimRequestOptions {
            prompt,
            suffix: Some(suffix),
            scope: options.scope,
            max_tokens: options.max_tokens,
            temperature: options.temperature,
            stream: options.stream,
        },
        policy,
    )
}

pub fn route_fim_for_prompt(
    options: FimRoutePromptOptions,
    policy: &FimPolicy,
) -> FimRouteDecision {
    let input = options.input.trim();
    if input.is_empty() {
        return fim_route_declined("prompt is empty");
    }
    if !policy.enabled {
        return fim_route_declined("FIM policy is disabled");
    }
    let normalized = input.to_ascii_lowercase();
    if contains_any(
        &normalized,
        &[
            "multi-file",
            "multifile",
            "refactor",
            "architecture",
            "security",
            "credential",
            "secret",
            "permission policy",
            "shell policy",
        ],
    ) {
        return fim_route_declined("prompt requests a broad or sensitive edit");
    }
    if !contains_any(
        &normalized,
        &[
            "complete",
            "fill",
            "insert",
            "replace",
            "repair",
            "fix",
            "missing block",
            "method",
            "function",
        ],
    ) {
        return fim_route_declined("prompt does not look like a localized edit");
    }

    let Some((start_line, end_line)) = extract_line_range(input) else {
        return fim_route_declined("prompt does not include an explicit line range");
    };
    let Some(file_path) = extract_prompt_file_path(input, &options.tracked_files) else {
        return fim_route_declined("prompt does not include a tracked repo-relative file path");
    };
    if let Err(e) = validate_repo_relative_path(&file_path) {
        return fim_route_declined(&e);
    }
    if start_line == 0 || end_line < start_line {
        return fim_route_declined("line range must be 1-based and inclusive");
    }
    if end_line.saturating_sub(start_line) + 1 > 80 {
        return fim_route_declined("line range is too large for automatic FIM routing");
    }

    let scope = infer_fim_route_scope(&normalized, start_line, end_line);
    if policy.disallowed_scopes.iter().any(|item| item == &scope)
        || !policy.allowed_scopes.iter().any(|item| item == &scope)
    {
        return fim_route_declined("inferred FIM scope is not allowed by policy");
    }
    let command = vec![
        "yoyo".to_string(),
        "deepseek".to_string(),
        "fim-complete".to_string(),
        "--file".to_string(),
        file_path.clone(),
        "--start".to_string(),
        start_line.to_string(),
        "--end".to_string(),
        end_line.to_string(),
        "--scope".to_string(),
        scope.clone(),
        "--max-tokens".to_string(),
        options
            .max_tokens
            .clamp(1, FIM_MAX_OUTPUT_TOKENS)
            .to_string(),
    ];
    FimRouteDecision {
        route: "fim_complete".to_string(),
        use_fim: true,
        reason: "explicit single-file line-range edit matches safe FIM routing policy".to_string(),
        file_path: Some(file_path),
        start_line: Some(start_line),
        end_line: Some(end_line),
        scope,
        command,
    }
}

pub fn fim_route_decision_state_payload(
    decision: &FimRouteDecision,
    source: &str,
    input: &str,
    execute: bool,
) -> Value {
    json!({
        "source": source,
        "decision_type": "fim_route",
        "decision": if decision.use_fim { "accepted" } else { "declined" },
        "status": if decision.use_fim { "accepted" } else { "declined" },
        "route": decision.route,
        "reason": decision.reason,
        "file_path": decision.file_path,
        "start_line": decision.start_line,
        "end_line": decision.end_line,
        "scope": decision.scope,
        "execute": execute,
        "command": decision.command,
        "prompt_preview": input.chars().take(240).collect::<String>(),
    })
}

pub fn record_fim_route_decision(
    decision: &FimRouteDecision,
    source: &str,
    input: &str,
    execute: bool,
) {
    crate::state::record(
        crate::state::EventType::DecisionRecorded,
        crate::state::Actor::Harness,
        fim_route_decision_state_payload(decision, source, input, execute),
    );
}

fn fim_route_declined(reason: &str) -> FimRouteDecision {
    FimRouteDecision {
        route: "agent_loop".to_string(),
        use_fim: false,
        reason: reason.to_string(),
        file_path: None,
        start_line: None,
        end_line: None,
        scope: "none".to_string(),
        command: Vec::new(),
    }
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn extract_line_range(input: &str) -> Option<(usize, usize)> {
    let normalized = input
        .chars()
        .map(|ch| {
            if ch.is_ascii_digit() || ch == '-' {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>();
    for token in normalized.split_whitespace() {
        let Some((left, right)) = token.split_once('-') else {
            continue;
        };
        let start = left.parse::<usize>().ok()?;
        let end = right.parse::<usize>().ok()?;
        return Some((start, end));
    }
    None
}

fn extract_prompt_file_path(input: &str, tracked_files: &[String]) -> Option<String> {
    let normalized = input.replace('\\', "/");
    let mut files = tracked_files.iter().collect::<Vec<_>>();
    files.sort_by_key(|path| std::cmp::Reverse(path.len()));
    for path in files {
        if normalized.contains(path) || normalized.contains(&format!("./{path}")) {
            return Some(path.clone());
        }
    }
    normalized
        .split(|ch: char| ch.is_whitespace() || matches!(ch, '\'' | '"' | '`' | ',' | ':' | ';'))
        .map(|token| token.trim_start_matches("./"))
        .find(|token| looks_like_repo_file_path(token))
        .map(ToString::to_string)
}

fn looks_like_repo_file_path(token: &str) -> bool {
    token.contains('/')
        && matches!(
            Path::new(token)
                .extension()
                .and_then(|extension| extension.to_str()),
            Some(
                "rs" | "md" | "toml" | "json" | "yaml" | "yml" | "py" | "js" | "ts" | "tsx" | "jsx"
            )
        )
}

fn infer_fim_route_scope(input: &str, start_line: usize, end_line: usize) -> String {
    if contains_any(input, &["method", "function", "fn "]) {
        "single_function_body".to_string()
    } else if contains_any(input, &["missing block", "fill", "insert"]) {
        "small_missing_block".to_string()
    } else if end_line.saturating_sub(start_line) < 40 {
        "localized_completion".to_string()
    } else {
        "one_method_repair".to_string()
    }
}

pub fn parse_fim_completion_response(value: &Value) -> Result<DeepSeekFimCompletion, String> {
    let choice = value
        .get("choices")
        .and_then(|v| v.as_array())
        .and_then(|choices| choices.first())
        .ok_or_else(|| "FIM response has no choices[0]".to_string())?;
    let text = choice
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "FIM response choices[0].text is missing".to_string())?
        .to_string();
    let finish_reason = choice
        .get("finish_reason")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let usage = value.get("usage").unwrap_or(&Value::Null);
    Ok(DeepSeekFimCompletion {
        text,
        finish_reason,
        usage: DeepSeekUsage {
            input_tokens: usage
                .get("prompt_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            output_tokens: usage
                .get("completion_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            cache_hit_tokens: usage
                .get("prompt_cache_hit_tokens")
                .and_then(|v| v.as_u64()),
            cache_miss_tokens: usage
                .get("prompt_cache_miss_tokens")
                .and_then(|v| v.as_u64()),
        },
    })
}

pub fn parse_chat_completion_sse(stream: &str) -> Result<DeepSeekStreamingChatSummary, String> {
    let mut content = String::new();
    let mut reasoning_content = String::new();
    let mut tool_call_ids = Vec::new();
    let mut finish_reason = None;
    let mut usage = DeepSeekUsage {
        input_tokens: 0,
        output_tokens: 0,
        cache_hit_tokens: None,
        cache_miss_tokens: None,
    };
    let mut saw_data = false;

    for line in stream.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(':') || !line.starts_with("data:") {
            continue;
        }
        let data = line.trim_start_matches("data:").trim();
        if data == "[DONE]" {
            break;
        }
        saw_data = true;
        let value: Value = serde_json::from_str(data)
            .map_err(|e| format!("invalid DeepSeek streaming JSON chunk: {e}"))?;
        if let Some(chunk_usage) = value.get("usage") {
            usage = parse_chat_usage(chunk_usage);
        }
        let Some(choice) = value
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|choices| choices.first())
        else {
            continue;
        };
        if let Some(delta) = choice.get("delta") {
            if let Some(text) = delta.get("content").and_then(Value::as_str) {
                content.push_str(text);
            }
            if let Some(text) = delta.get("reasoning_content").and_then(Value::as_str) {
                reasoning_content.push_str(text);
            }
            if let Some(tool_calls) = delta.get("tool_calls").and_then(Value::as_array) {
                for tool_call in tool_calls {
                    if let Some(id) = tool_call.get("id").and_then(Value::as_str) {
                        if !tool_call_ids.iter().any(|existing| existing == id) {
                            tool_call_ids.push(id.to_string());
                        }
                    }
                }
            }
        }
        if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
            finish_reason = Some(reason.to_string());
        }
    }

    if !saw_data {
        return Err("DeepSeek streaming response contained no data chunks".to_string());
    }

    Ok(DeepSeekStreamingChatSummary {
        content,
        reasoning_content,
        tool_call_ids,
        finish_reason,
        usage,
    })
}

fn parse_chat_usage(usage: &Value) -> DeepSeekUsage {
    DeepSeekUsage {
        input_tokens: usage
            .get("prompt_tokens")
            .or_else(|| usage.get("input_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or(0),
        output_tokens: usage
            .get("completion_tokens")
            .or_else(|| usage.get("output_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or(0),
        cache_hit_tokens: usage
            .get("prompt_cache_hit_tokens")
            .or_else(|| usage.get("cache_hit_tokens"))
            .and_then(Value::as_u64),
        cache_miss_tokens: usage
            .get("prompt_cache_miss_tokens")
            .or_else(|| usage.get("cache_miss_tokens"))
            .and_then(Value::as_u64),
    }
}

pub fn build_fim_edit_plan(
    options: FimEditOptions,
    policy: &FimPolicy,
) -> Result<FimEditPlan, String> {
    let scope = normalize_scope(&options.scope);
    if policy.disallowed_scopes.iter().any(|item| item == &scope) {
        return Err(format!("FIM scope '{scope}' is disallowed by policy"));
    }
    if !policy.allowed_scopes.iter().any(|item| item == &scope) {
        return Err(format!(
            "FIM scope '{scope}' is not in the allowed local-edit scope set"
        ));
    }
    if options.start_line == 0 || options.end_line < options.start_line {
        return Err("FIM edit range must be a 1-based inclusive line range".to_string());
    }

    validate_repo_relative_path(&options.file_path)?;
    let content = std::fs::read_to_string(&options.file_path)
        .map_err(|e| format!("could not read {}: {e}", options.file_path))?;
    let normalized_content = content.replace("\r\n", "\n").replace('\r', "\n");
    let original_lines: Vec<String> = normalized_content
        .lines()
        .map(|line| line.to_string())
        .collect();
    if original_lines.is_empty() {
        return Err("FIM edit target file is empty".to_string());
    }
    if options.end_line > original_lines.len() {
        return Err(format!(
            "FIM edit range {}-{} is past end of file ({} lines)",
            options.start_line,
            options.end_line,
            original_lines.len()
        ));
    }

    let replacement = options.completion.replace("\r\n", "\n").replace('\r', "\n");
    if replacement.trim().is_empty() {
        return Err("FIM completion cannot be empty".to_string());
    }
    let replacement_lines: Vec<String> = replacement.lines().map(|line| line.to_string()).collect();
    let removed_lines = options.end_line - options.start_line + 1;
    let inserted_lines = replacement_lines.len();
    let patch = build_localized_unified_diff(
        &options.file_path,
        &original_lines,
        &replacement_lines,
        options.start_line,
        options.end_line,
    );
    let risk_label = if inserted_lines.max(removed_lines) <= 40 {
        "low"
    } else {
        "medium"
    }
    .to_string();

    Ok(FimEditPlan {
        file_path: options.file_path,
        start_line: options.start_line,
        end_line: options.end_line,
        scope,
        inserted_lines,
        removed_lines,
        risk_label,
        requires_explicit_apply: true,
        patch,
    })
}

pub fn deepseek_thinking_tool_call_probe_messages() -> Vec<Value> {
    vec![
        json!({
            "role": "assistant",
            "reasoning_content": "Need to inspect the target file before editing.",
            "tool_calls": [{
                "id": "call-inspect-1",
                "type": "function",
                "function": {
                    "name": "inspect_file",
                    "arguments": "{\"path\":\"src/main.rs\",\"line_start\":1,\"line_end\":80,\"reason\":\"Locate the CLI entrypoint before editing.\"}"
                }
            }]
        }),
        json!({
            "role": "tool",
            "tool_call_id": "call-inspect-1",
            "name": "inspect_file",
            "content": "src/main.rs:1: mod agent_builder;"
        }),
    ]
}

pub fn validate_deepseek_thinking_tool_call_messages(
    messages: &[Value],
) -> Result<DeepSeekProtocolCheck, String> {
    let mut assistant_tool_call_turns = 0usize;
    let mut tool_call_links = 0usize;
    let mut pending_tool_call_ids = Vec::<String>::new();

    for (message_idx, message) in messages.iter().enumerate() {
        match message.get("role").and_then(|v| v.as_str()) {
            Some("assistant") => {
                if let Some(first_pending) = pending_tool_call_ids.first() {
                    return Err(format!(
                        "tool call '{first_pending}' has no later matching tool result before the next assistant turn"
                    ));
                }

                let Some(tool_calls) = message.get("tool_calls").and_then(|v| v.as_array()) else {
                    continue;
                };
                if tool_calls.is_empty() {
                    continue;
                }

                assistant_tool_call_turns += 1;
                let reasoning = message
                    .get("reasoning_content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if reasoning.trim().is_empty() {
                    return Err(format!(
                        "assistant tool-call turn {} is missing DeepSeek reasoning_content",
                        message_idx + 1
                    ));
                }

                for tool_call in tool_calls {
                    let tool_call_id =
                        tool_call
                            .get("id")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| {
                                format!(
                                    "assistant tool-call turn {} has a tool call with no id",
                                    message_idx + 1
                                )
                            })?;
                    if pending_tool_call_ids
                        .iter()
                        .any(|pending| pending == tool_call_id)
                    {
                        return Err(format!("duplicate tool call id '{tool_call_id}'"));
                    }
                    pending_tool_call_ids.push(tool_call_id.to_string());
                }
            }
            Some("tool") => {
                let tool_call_id = message
                    .get("tool_call_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        format!(
                            "tool result message {} has no tool_call_id",
                            message_idx + 1
                        )
                    })?;
                let Some(position) = pending_tool_call_ids
                    .iter()
                    .position(|pending| pending == tool_call_id)
                else {
                    return Err(format!(
                        "tool result '{tool_call_id}' has no pending assistant tool call"
                    ));
                };
                pending_tool_call_ids.remove(position);
                tool_call_links += 1;
            }
            _ => {}
        }
    }

    if assistant_tool_call_turns == 0 {
        return Err("probe has no assistant tool-call turns".to_string());
    }
    if let Some(first_pending) = pending_tool_call_ids.first() {
        return Err(format!(
            "tool call '{first_pending}' has no later matching tool result"
        ));
    }

    Ok(DeepSeekProtocolCheck {
        name: "deepseek-thinking-tool-call".to_string(),
        passed: true,
        details: vec![
            format!(
                "assistant reasoning_content preserved for {assistant_tool_call_turns} tool-call turn(s)"
            ),
            format!("{tool_call_links} tool call result link(s) validated"),
        ],
    })
}

pub fn validate_strict_tool_schema_suite() -> Result<DeepSeekProtocolCheck, String> {
    let schemas = strict_schema_suite();
    if schemas.is_empty() {
        return Err("strict schema suite is empty".to_string());
    }
    let mut names = Vec::new();
    for schema in &schemas {
        let name = schema["function"]["name"]
            .as_str()
            .ok_or_else(|| "strict schema is missing function.name".to_string())?;
        if schema["function"]["strict"] != true {
            return Err(format!("schema '{name}' is not strict"));
        }
        if schema["function"]["parameters"]["additionalProperties"] != false {
            return Err(format!("schema '{name}' allows additional properties"));
        }
        let required_count = schema["function"]["parameters"]["required"]
            .as_array()
            .map(|fields| fields.len())
            .unwrap_or(0);
        if required_count == 0 {
            return Err(format!("schema '{name}' has no required fields"));
        }
        names.push(name.to_string());
    }

    Ok(DeepSeekProtocolCheck {
        name: "deepseek-strict-tool-call-schemas".to_string(),
        passed: true,
        details: names,
    })
}

pub fn validate_strict_tool_arguments(
    schema_name: &str,
    arguments: &Value,
) -> Result<ToolSchemaValidationReport, String> {
    let schema = strict_schema_by_name(schema_name)
        .ok_or_else(|| format!("unknown strict schema '{schema_name}'"))?;
    let params = &schema["function"]["parameters"];
    let properties = params["properties"]
        .as_object()
        .ok_or_else(|| format!("schema '{schema_name}' has no properties object"))?;
    let required = params["required"]
        .as_array()
        .ok_or_else(|| format!("schema '{schema_name}' has no required field list"))?;

    let mut report = ToolSchemaValidationReport {
        schema_name: schema_name.to_string(),
        valid: false,
        missing_required: Vec::new(),
        unexpected_fields: Vec::new(),
        type_errors: Vec::new(),
        enum_errors: Vec::new(),
        repair_instruction: None,
    };

    let Some(args) = arguments.as_object() else {
        report
            .type_errors
            .push("arguments must be a JSON object".to_string());
        report.repair_instruction = Some(tool_schema_repair_instruction(&report));
        return Ok(report);
    };

    for field in required.iter().filter_map(Value::as_str) {
        if !args.contains_key(field) {
            report.missing_required.push(field.to_string());
        }
    }
    for field in args.keys() {
        if !properties.contains_key(field) {
            report.unexpected_fields.push(field.to_string());
        }
    }
    for (field, value) in args {
        let Some(spec) = properties.get(field) else {
            continue;
        };
        if let Some(expected_type) = spec.get("type").and_then(Value::as_str) {
            if !value_matches_json_schema_type(value, expected_type) {
                report.type_errors.push(format!(
                    "{field} must be {expected_type}, got {}",
                    json_value_type_label(value)
                ));
            }
        }
        if let Some(allowed) = spec.get("enum").and_then(Value::as_array) {
            if !allowed.iter().any(|allowed_value| allowed_value == value) {
                report.enum_errors.push(format!(
                    "{field} must be one of {}",
                    allowed
                        .iter()
                        .map(Value::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
        if let Some(item_type) = spec
            .get("items")
            .and_then(|items| items.get("type"))
            .and_then(Value::as_str)
        {
            if let Some(items) = value.as_array() {
                for (idx, item) in items.iter().enumerate() {
                    if !value_matches_json_schema_type(item, item_type) {
                        report.type_errors.push(format!(
                            "{field}[{idx}] must be {item_type}, got {}",
                            json_value_type_label(item)
                        ));
                    }
                }
            }
        }
    }

    report.valid = report.missing_required.is_empty()
        && report.unexpected_fields.is_empty()
        && report.type_errors.is_empty()
        && report.enum_errors.is_empty();
    if !report.valid {
        report.repair_instruction = Some(tool_schema_repair_instruction(&report));
    }
    Ok(report)
}

pub fn decide_tool_schema_repair(
    report: &ToolSchemaValidationReport,
    failed_attempt: u32,
    policy: &RepairPolicy,
) -> ToolSchemaRepairDecision {
    if report.valid {
        return ToolSchemaRepairDecision {
            schema_name: report.schema_name.clone(),
            action: ToolSchemaRepairAction::Accept,
            failed_attempt,
            max_repair_turns: policy.max_repair_turns,
            should_record_failure: false,
            instruction: None,
            reason: "strict tool arguments match schema".to_string(),
        };
    }

    if failed_attempt <= policy.max_repair_turns {
        return ToolSchemaRepairDecision {
            schema_name: report.schema_name.clone(),
            action: ToolSchemaRepairAction::Retry,
            failed_attempt,
            max_repair_turns: policy.max_repair_turns,
            should_record_failure: false,
            instruction: report.repair_instruction.clone(),
            reason: format!(
                "strict tool arguments failed validation; retry {failed_attempt}/{} is allowed",
                policy.max_repair_turns
            ),
        };
    }

    ToolSchemaRepairDecision {
        schema_name: report.schema_name.clone(),
        action: ToolSchemaRepairAction::Abort,
        failed_attempt,
        max_repair_turns: policy.max_repair_turns,
        should_record_failure: true,
        instruction: None,
        reason: format!(
            "strict tool arguments failed validation after {failed_attempt} attempts; repair budget exhausted"
        ),
    }
}

pub fn normalize_scope(raw: &str) -> String {
    raw.trim().replace(['-', ' '], "_").to_ascii_lowercase()
}

fn validate_repo_relative_path(path: &str) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("FIM edit file path cannot be empty".to_string());
    }
    if path.contains('\n') || path.contains('\r') || path.contains('\t') {
        return Err("FIM edit file path cannot contain control characters".to_string());
    }
    let parsed = Path::new(path);
    if parsed.is_absolute() {
        return Err("FIM edit file path must be repo-relative".to_string());
    }
    if parsed
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("FIM edit file path cannot traverse parent directories".to_string());
    }
    if is_sensitive_fim_target_path(path) {
        return Err(
            "FIM edit file path targets security-sensitive material; use the normal reviewed agent path"
                .to_string(),
        );
    }
    Ok(())
}

fn is_sensitive_fim_target_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    let segments: Vec<&str> = normalized
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    segments.iter().any(|segment| {
        matches!(
            *segment,
            ".env"
                | ".envrc"
                | ".npmrc"
                | ".pypirc"
                | ".netrc"
                | "credentials"
                | "credentials.json"
                | "secrets"
                | "secrets.json"
                | "id_rsa"
                | "id_ed25519"
                | "known_hosts"
        ) || segment.ends_with(".pem")
            || segment.ends_with(".key")
            || segment.ends_with(".p12")
            || segment.ends_with(".pfx")
            || segment.contains("secret")
            || segment.contains("credential")
            || segment.contains("private_key")
            || segment.contains("api_key")
            || segment.contains("token")
    })
}

fn build_localized_unified_diff(
    file_path: &str,
    original_lines: &[String],
    replacement_lines: &[String],
    start_line: usize,
    end_line: usize,
) -> String {
    let context = 3usize;
    let hunk_start = start_line.saturating_sub(context).max(1);
    let hunk_end = (end_line + context).min(original_lines.len());
    let before_start = hunk_start;
    let before_end = start_line.saturating_sub(1);
    let after_start = end_line + 1;
    let after_end = hunk_end;
    let before_count =
        before_end.saturating_sub(before_start) + usize::from(before_end >= before_start);
    let after_count = after_end.saturating_sub(after_start) + usize::from(after_end >= after_start);
    let removed_count = end_line - start_line + 1;
    let old_count = before_count + removed_count + after_count;
    let new_count = before_count + replacement_lines.len() + after_count;

    let mut patch = String::new();
    patch.push_str(&format!("diff --git a/{file_path} b/{file_path}\n"));
    patch.push_str(&format!("--- a/{file_path}\n"));
    patch.push_str(&format!("+++ b/{file_path}\n"));
    patch.push_str(&format!(
        "@@ -{},{} +{},{} @@\n",
        hunk_start, old_count, hunk_start, new_count
    ));
    if before_end >= before_start {
        for line in &original_lines[before_start - 1..before_end] {
            patch.push(' ');
            patch.push_str(line);
            patch.push('\n');
        }
    }
    for line in &original_lines[start_line - 1..end_line] {
        patch.push('-');
        patch.push_str(line);
        patch.push('\n');
    }
    for line in replacement_lines {
        patch.push('+');
        patch.push_str(line);
        patch.push('\n');
    }
    if after_end >= after_start {
        for line in &original_lines[after_start - 1..after_end] {
            patch.push(' ');
            patch.push_str(line);
            patch.push('\n');
        }
    }
    patch
}

pub fn strict_tool_schema(
    name: &str,
    description: &str,
    properties: serde_json::Value,
) -> serde_json::Value {
    let mut properties = properties.as_object().cloned().unwrap_or_default();
    properties.insert(
        "schema_version".to_string(),
        json!({
            "type": "integer",
            "enum": [STRICT_SCHEMA_VERSION]
        }),
    );
    let required = properties.keys().cloned().collect::<Vec<_>>();

    json!({
        "type": "function",
        "function": {
            "name": name,
            "strict": true,
            "description": description,
            "parameters": {
                "type": "object",
                "properties": properties,
                "required": required,
                "additionalProperties": false
            }
        }
    })
}

pub fn strict_schema_names() -> Vec<String> {
    vec![
        "plan_task".to_string(),
        "request_context".to_string(),
        "inspect_file".to_string(),
        "propose_edit".to_string(),
        "record_failure".to_string(),
        "propose_harness_patch".to_string(),
        "record_eval_result".to_string(),
        "promote_or_reject_patch".to_string(),
        "request_human_approval".to_string(),
    ]
}

pub fn strict_schema_versioned_names() -> Vec<String> {
    strict_schema_names()
        .into_iter()
        .map(|name| format!("{name}@v{STRICT_SCHEMA_VERSION}"))
        .collect()
}

pub fn strict_schema_suite() -> Vec<serde_json::Value> {
    vec![
        plan_task_schema(),
        request_context_schema(),
        inspect_file_schema(),
        propose_edit_schema(),
        record_failure_schema(),
        propose_harness_patch_schema(),
        record_eval_result_schema(),
        promote_or_reject_patch_schema(),
        request_human_approval_schema(),
    ]
}

pub fn strict_schema_by_name(name: &str) -> Option<serde_json::Value> {
    strict_schema_suite().into_iter().find(|schema| {
        schema["function"]["name"]
            .as_str()
            .map(|schema_name| schema_name == name)
            .unwrap_or(false)
    })
}

pub fn plan_task_schema() -> serde_json::Value {
    strict_tool_schema(
        "plan_task",
        "Plan a coding task with concrete context, risk, and verification steps.",
        json!({
            "task_summary": { "type": "string" },
            "required_context": { "type": "array", "items": { "type": "string" } },
            "risk_level": { "type": "string", "enum": ["low", "medium", "high"] },
            "verification_steps": { "type": "array", "items": { "type": "string" } }
        }),
    )
}

pub fn request_context_schema() -> serde_json::Value {
    strict_tool_schema(
        "request_context",
        "Request focused project context with explicit rationale.",
        json!({
            "paths": { "type": "array", "items": { "type": "string" } },
            "symbols": { "type": "array", "items": { "type": "string" } },
            "recent_event_ids": { "type": "array", "items": { "type": "string" } },
            "why": { "type": "string" }
        }),
    )
}

pub fn inspect_file_schema() -> serde_json::Value {
    strict_tool_schema(
        "inspect_file",
        "Request file inspection before editing.",
        json!({
            "path": { "type": "string" },
            "line_start": { "type": "integer" },
            "line_end": { "type": "integer" },
            "reason": { "type": "string" }
        }),
    )
}

pub fn propose_edit_schema() -> serde_json::Value {
    strict_tool_schema(
        "propose_edit",
        "Propose one scoped edit with expected verification impact.",
        json!({
            "path": { "type": "string" },
            "intent": { "type": "string" },
            "expected_effect": { "type": "string" },
            "risk_level": { "type": "string", "enum": ["low", "medium", "high"] },
            "verification_steps": { "type": "array", "items": { "type": "string" } }
        }),
    )
}

pub fn record_failure_schema() -> serde_json::Value {
    strict_tool_schema(
        "record_failure",
        "Record a failure with cause hypothesis and evidence links.",
        json!({
            "failure_summary": { "type": "string" },
            "hypothesis": { "type": "string" },
            "evidence_event_ids": { "type": "array", "items": { "type": "string" } },
            "affected_component": { "type": "string" },
            "next_repair_step": { "type": "string" }
        }),
    )
}

pub fn propose_harness_patch_schema() -> serde_json::Value {
    strict_tool_schema(
        "propose_harness_patch",
        "Propose one harness policy change with evidence and an evaluation plan.",
        json!({
            "target_component": {
                "type": "string",
                "enum": [
                    "context_policy",
                    "tool_schema",
                    "test_policy",
                    "repair_policy",
                    "thinking_policy",
                    "model_routing_policy",
                    "memory_policy"
                ]
            },
            "problem_summary": { "type": "string" },
            "evidence_event_ids": {
                "type": "array",
                "items": { "type": "string" }
            },
            "proposed_change": { "type": "string" },
            "expected_effect": { "type": "string" },
            "risk_level": {
                "type": "string",
                "enum": ["low", "medium", "high"]
            },
            "eval_plan": { "type": "string" }
        }),
    )
}

pub fn record_eval_result_schema() -> serde_json::Value {
    strict_tool_schema(
        "record_eval_result",
        "Record an evaluation result for a harness version or patch.",
        json!({
            "eval_id": { "type": "string" },
            "harness_version": { "type": "string" },
            "patch_id": { "type": "string" },
            "suite": { "type": "string" },
            "status": { "type": "string", "enum": ["passed", "failed", "error"] },
            "score": { "type": "number" },
            "passed": { "type": "integer" },
            "failed": { "type": "integer" },
            "failure_event_ids": { "type": "array", "items": { "type": "string" } }
        }),
    )
}

pub fn promote_or_reject_patch_schema() -> serde_json::Value {
    strict_tool_schema(
        "promote_or_reject_patch",
        "Record a gated promotion or rejection decision for a harness patch.",
        json!({
            "patch_id": { "type": "string" },
            "decision": { "type": "string", "enum": ["promote", "reject", "needs_human"] },
            "baseline_eval_id": { "type": "string" },
            "candidate_eval_id": { "type": "string" },
            "criterion": { "type": "string" },
            "rationale": { "type": "string" },
            "risk_level": { "type": "string", "enum": ["low", "medium", "high", "critical"] },
            "approval_event_ids": { "type": "array", "items": { "type": "string" } }
        }),
    )
}

pub fn request_human_approval_schema() -> serde_json::Value {
    strict_tool_schema(
        "request_human_approval",
        "Request human approval for high-risk harness or tool activity.",
        json!({
            "approval_scope": {
                "type": "string",
                "enum": [
                    "harness_patch_promotion",
                    "tool_execution",
                    "permission_policy_change",
                    "shell_policy_change",
                    "core_runtime_code"
                ]
            },
            "patch_id": { "type": "string" },
            "tool_name": { "type": "string" },
            "risk_level": { "type": "string", "enum": ["medium", "high", "critical"] },
            "reason": { "type": "string" },
            "evidence_event_ids": { "type": "array", "items": { "type": "string" } }
        }),
    )
}

fn value_matches_json_schema_type(value: &Value, expected_type: &str) -> bool {
    match expected_type {
        "array" => value.is_array(),
        "boolean" => value.is_boolean(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "number" => value.is_number(),
        "object" => value.is_object(),
        "string" => value.is_string(),
        _ => true,
    }
}

fn json_value_type_label(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(number) if number.as_i64().is_some() || number.as_u64().is_some() => {
            "integer"
        }
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn tool_schema_repair_instruction(report: &ToolSchemaValidationReport) -> String {
    let mut parts = vec![format!(
        "Retry tool call '{}' with strict JSON arguments only.",
        report.schema_name
    )];
    if !report.missing_required.is_empty() {
        parts.push(format!(
            "Add required fields: {}.",
            report.missing_required.join(", ")
        ));
    }
    if !report.unexpected_fields.is_empty() {
        parts.push(format!(
            "Remove unsupported fields: {}.",
            report.unexpected_fields.join(", ")
        ));
    }
    if !report.type_errors.is_empty() {
        parts.push(format!("Fix types: {}.", report.type_errors.join("; ")));
    }
    if !report.enum_errors.is_empty() {
        parts.push(format!(
            "Use allowed enum values: {}.",
            report.enum_errors.join("; ")
        ));
    }
    parts.join(" ")
}

fn preview(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let mut out: String = trimmed.chars().take(max_chars).collect();
    out.push_str("\n...[truncated]");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn contains_json_key(value: &Value, key: &str) -> bool {
        match value {
            Value::Object(map) => {
                map.contains_key(key) || map.values().any(|child| contains_json_key(child, key))
            }
            Value::Array(items) => items.iter().any(|child| contains_json_key(child, key)),
            _ => false,
        }
    }

    #[test]
    fn native_models_use_v4_names() {
        assert_eq!(DeepSeekModel::V4Pro.as_str(), "deepseek-v4-pro");
        assert_eq!(DeepSeekModel::V4Flash.as_str(), "deepseek-v4-flash");
    }

    #[test]
    fn stable_system_contract_keeps_prompt_quality_rules() {
        let contract = stable_system_contract();
        assert_eq!(
            DEEPSEEK_SYSTEM_CONTRACT_VERSION,
            "deepseek_native_contract@v2"
        );
        assert!(contract.contains("deterministic prompt layout"));
        assert!(contract.contains("state, eval, logs"));
        assert!(contract.contains("source inspection bounded"));
        assert!(contract.contains("Do not claim completion"));
        assert!(contract.contains("structured outputs"));
    }

    #[test]
    fn cache_ratio_handles_missing_and_zero_totals() {
        assert_eq!(DeepSeekUsage::default().cache_hit_ratio(), None);
        let usage = DeepSeekUsage {
            cache_hit_tokens: Some(0),
            cache_miss_tokens: Some(0),
            ..DeepSeekUsage::default()
        };
        assert_eq!(usage.cache_hit_ratio(), None);
        let usage = DeepSeekUsage {
            cache_hit_tokens: Some(75),
            cache_miss_tokens: Some(25),
            ..DeepSeekUsage::default()
        };
        assert_eq!(usage.cache_hit_ratio(), Some(0.75));
    }

    #[test]
    fn strict_schema_has_required_fields_and_no_extra_properties() {
        let schema = propose_harness_patch_schema();
        let params = &schema["function"]["parameters"];
        assert_eq!(schema["function"]["strict"], true);
        assert_eq!(params["additionalProperties"], false);
        let required = params["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "target_component"));
        assert!(required.iter().any(|v| v == "eval_plan"));
    }

    #[test]
    fn deepseek_compat_enables_reasoning_effort() {
        let compat = yoagent::provider::OpenAiCompat::deepseek();
        assert!(compat.supports_reasoning_effort);
        assert!(compat.supports_thinking_control);
        assert!(compat.supports_usage_in_streaming);
    }

    #[test]
    fn harness_genome_constrains_default_mutation_surface() {
        let genome = DeepSeekHarnessGenome::default();
        assert_eq!(genome.version, HARNESS_GENOME_VERSION);
        assert_eq!(genome.model_routing_policy.root_cause_model, DEFAULT_MODEL);
        assert_eq!(
            genome.model_routing_policy.memory_compression_model,
            FAST_MODEL
        );
        assert_eq!(genome.transport_policy.request_timeout_ms, 120_000);
        assert!(genome.transport_policy.retry_statuses.contains(&429));
        assert!(genome.tool_schema_policy.strict_critical_tools);
        assert!(genome
            .tool_schema_policy
            .critical_schemas
            .contains(&"propose_harness_patch".to_string()));
        assert!(genome
            .test_policy
            .protocol_gates
            .iter()
            .any(|gate| gate.contains("deepseek test-thinking --record --json")));
        assert!(
            !genome.memory_policy.persist_raw_reasoning,
            "raw reasoning must not become durable truth"
        );
        assert!(
            genome
                .permission_policy
                .permission_policy_patches_need_human
        );
    }

    #[test]
    fn harness_genome_applies_deepseek_config_routing_and_cache_policy() {
        let config = HashMap::from([
            (
                "deepseek_model".to_string(),
                "deepseek-v4-pro-custom".to_string(),
            ),
            (
                "deepseek_fast_model".to_string(),
                "deepseek-v4-flash-custom".to_string(),
            ),
            (
                "deepseek_routing_root_cause".to_string(),
                "pro_thinking_max".to_string(),
            ),
            (
                "deepseek_routing_summary".to_string(),
                "flash_non_thinking".to_string(),
            ),
            (
                "deepseek_routing_local_edit".to_string(),
                "fim_non_thinking".to_string(),
            ),
            (
                "deepseek_cache_record_metrics".to_string(),
                "false".to_string(),
            ),
            (
                "deepseek_cache_optimize_prompt_order".to_string(),
                "false".to_string(),
            ),
            (
                "deepseek_context_recent_failure_limit".to_string(),
                "9".to_string(),
            ),
            (
                "deepseek_context_changed_file_limit".to_string(),
                "24".to_string(),
            ),
            (
                "deepseek_context_include_repo_map".to_string(),
                "false".to_string(),
            ),
            (
                "deepseek_context_include_instruction_files".to_string(),
                "[\"YOYO.md\", \"TEAM.md\", \"../outside.md\", \"/tmp/abs.md\"]".to_string(),
            ),
            (
                "deepseek_transport_request_timeout_ms".to_string(),
                "45000".to_string(),
            ),
            (
                "deepseek_transport_max_retries".to_string(),
                "4".to_string(),
            ),
        ]);

        let genome = DeepSeekHarnessGenome::from_config(&config);

        assert_eq!(
            genome.model_routing_policy.final_patch_model,
            "deepseek-v4-pro-custom"
        );
        assert_eq!(genome.model_routing_policy.root_cause_model, DEFAULT_MODEL);
        assert_eq!(genome.model_routing_policy.quick_summary_model, FAST_MODEL);
        assert_eq!(
            genome.thinking_policy.root_cause,
            ThinkingMode::Enabled {
                effort: ThinkingEffort::Max,
            }
        );
        assert_eq!(
            genome.thinking_policy.memory_compression,
            ThinkingMode::Disabled
        );
        assert!(genome.fim_policy.enabled);
        assert!(!genome.cache_policy.record_metrics);
        assert!(!genome.cache_policy.optimize_prompt_order);
        assert_eq!(genome.context_policy.recent_failure_limit, 9);
        assert_eq!(genome.context_policy.changed_file_limit, 24);
        assert!(!genome.context_policy.include_repo_map);
        assert_eq!(
            genome.context_policy.include_instruction_files,
            vec!["YOYO.md".to_string(), "TEAM.md".to_string()]
        );
        assert_eq!(genome.transport_policy.request_timeout_ms, 45_000);
        assert_eq!(genome.transport_policy.max_retries, 4);
    }

    #[test]
    fn harness_genome_ignores_invalid_context_config_limits() {
        let config = HashMap::from([
            (
                "deepseek_context_recent_failure_limit".to_string(),
                "0".to_string(),
            ),
            (
                "deepseek_context_changed_file_limit".to_string(),
                "1000".to_string(),
            ),
        ]);

        let genome = DeepSeekHarnessGenome::from_config(&config);
        let default = DeepSeekHarnessGenome::default();

        assert_eq!(
            genome.context_policy.recent_failure_limit,
            default.context_policy.recent_failure_limit
        );
        assert_eq!(
            genome.context_policy.changed_file_limit,
            default.context_policy.changed_file_limit
        );
    }

    #[test]
    fn harness_genome_keeps_default_instruction_policy_when_config_list_is_empty_or_unsafe() {
        let config = HashMap::from([(
            "deepseek_context_include_instruction_files".to_string(),
            "[\"\", \"../outside.md\", \"/tmp/abs.md\"]".to_string(),
        )]);

        let genome = DeepSeekHarnessGenome::from_config(&config);
        let default = DeepSeekHarnessGenome::default();

        assert_eq!(
            genome.context_policy.include_instruction_files,
            default.context_policy.include_instruction_files
        );
    }

    #[test]
    fn route_for_task_exposes_model_thinking_and_fim_policy() {
        let mut genome = DeepSeekHarnessGenome::default();
        genome.fim_policy.enabled = true;

        let root = route_for_task("root-cause", &genome).unwrap();
        assert_eq!(root.task, "root_cause");
        assert_eq!(root.model, DEFAULT_MODEL);
        assert_eq!(root.thinking, ThinkingMode::native_default());
        assert!(!root.use_fim);

        let summary = route_for_task("summary", &genome).unwrap();
        assert_eq!(summary.model, FAST_MODEL);
        assert_eq!(summary.thinking, ThinkingMode::Disabled);

        let local_edit = route_for_task("local-edit", &genome).unwrap();
        assert_eq!(local_edit.thinking, ThinkingMode::Disabled);
        assert!(local_edit.use_fim);

        assert!(route_for_task("unknown", &genome).is_err());
    }

    #[test]
    fn transport_policy_classifies_retryable_failures_with_backoff() {
        let policy = DeepSeekHarnessGenome::default().transport_policy;

        let rate_limited = classify_deepseek_transport_failure(Some(429), "rate limit", 0, &policy);
        assert_eq!(rate_limited.class, DeepSeekTransportErrorClass::RateLimited);
        assert!(rate_limited.retryable);
        assert_eq!(rate_limited.next_backoff_ms, Some(1_000));

        let server_error =
            classify_deepseek_transport_failure(Some(503), "temporarily unavailable", 1, &policy);
        assert_eq!(server_error.class, DeepSeekTransportErrorClass::ServerError);
        assert!(server_error.retryable);
        assert_eq!(server_error.next_backoff_ms, Some(2_000));

        let exhausted =
            classify_deepseek_transport_failure(Some(503), "temporarily unavailable", 2, &policy);
        assert!(!exhausted.retryable);
        assert_eq!(exhausted.next_backoff_ms, None);
        assert!(exhausted.reason.contains("retry budget exhausted"));
    }

    #[test]
    fn transport_policy_does_not_retry_request_or_auth_failures() {
        let policy = DeepSeekHarnessGenome::default().transport_policy;

        let auth = classify_deepseek_transport_failure(Some(401), "bad key", 0, &policy);
        assert_eq!(auth.class, DeepSeekTransportErrorClass::Authentication);
        assert!(!auth.retryable);

        let context = classify_deepseek_transport_failure(
            Some(400),
            "maximum context length exceeded",
            0,
            &policy,
        );
        assert_eq!(context.class, DeepSeekTransportErrorClass::ContextLength);
        assert!(!context.retryable);

        let network =
            classify_deepseek_transport_failure(None, "connection reset by peer", 0, &policy);
        assert_eq!(network.class, DeepSeekTransportErrorClass::Network);
        assert!(network.retryable);
    }

    #[test]
    fn transport_failure_payload_is_state_ready_for_main_loop_errors() {
        let policy = DeepSeekHarnessGenome::default().transport_policy;
        let payload = deepseek_transport_failure_state_payload(
            "single_prompt_api_failure",
            DEFAULT_MODEL,
            "provider returned 503 Service Unavailable after retries",
            policy.max_retries,
            &policy,
        );

        assert_eq!(payload["source"], "single_prompt_api_failure");
        assert_eq!(payload["provider"], "deepseek");
        assert_eq!(payload["model"], DEFAULT_MODEL);
        assert_eq!(payload["failure_class"], "transport");
        assert_eq!(payload["transport_class"], "server_error");
        assert_eq!(payload["status"], 503);
        assert_eq!(payload["retryable"], false);
        assert!(payload["reason"]
            .as_str()
            .unwrap()
            .contains("retry budget exhausted"));
        assert!(payload["error_preview"]
            .as_str()
            .unwrap()
            .contains("Service Unavailable"));
    }

    #[test]
    fn strict_schema_suite_covers_critical_state_mutations() {
        let suite = strict_schema_suite();
        let names: Vec<String> = suite
            .iter()
            .map(|schema| schema["function"]["name"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(names, strict_schema_names());
        assert!(strict_schema_versioned_names()
            .iter()
            .any(|name| name == "propose_harness_patch@v1"));
        assert!(names.contains(&"promote_or_reject_patch".to_string()));
        assert!(names.contains(&"request_human_approval".to_string()));
        for schema in suite {
            assert_eq!(schema["function"]["strict"], true);
            assert_eq!(
                schema["function"]["parameters"]["additionalProperties"],
                false
            );
            assert_eq!(
                schema["function"]["parameters"]["properties"]["schema_version"]["enum"][0]
                    .as_u64(),
                Some(STRICT_SCHEMA_VERSION as u64)
            );
            assert!(schema["function"]["parameters"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|field| field == "schema_version"));
            assert!(
                schema["function"]["parameters"]["required"]
                    .as_array()
                    .unwrap()
                    .len()
                    >= 3
            );
        }
    }

    #[test]
    fn strict_tool_argument_validator_accepts_schema_matching_arguments() {
        let report = validate_strict_tool_arguments(
            "propose_edit",
            &json!({
                "schema_version": STRICT_SCHEMA_VERSION,
                "path": "src/main.rs",
                "intent": "tighten DeepSeek schema validation",
                "expected_effect": "malformed tool arguments are rejected before mutation",
                "risk_level": "medium",
                "verification_steps": ["cargo test"]
            }),
        )
        .unwrap();

        assert!(report.valid);
        assert!(report.missing_required.is_empty());
        assert!(report.unexpected_fields.is_empty());
        assert!(report.type_errors.is_empty());
        assert!(report.enum_errors.is_empty());
        assert!(report.repair_instruction.is_none());
    }

    #[test]
    fn strict_tool_argument_validator_reports_repairable_failures() {
        let report = validate_strict_tool_arguments(
            "propose_edit",
            &json!({
                "schema_version": 999,
                "path": "src/main.rs",
                "intent": 42,
                "expected_effect": "test failure",
                "risk_level": "risky",
                "extra": true,
                "verification_steps": ["cargo test", 123]
            }),
        )
        .unwrap();

        assert!(!report.valid);
        assert_eq!(report.missing_required, Vec::<String>::new());
        assert_eq!(report.unexpected_fields, vec!["extra".to_string()]);
        assert!(report
            .type_errors
            .iter()
            .any(|error| error.contains("intent must be string")));
        assert!(report
            .type_errors
            .iter()
            .any(|error| error.contains("verification_steps[1] must be string")));
        assert!(report
            .enum_errors
            .iter()
            .any(|error| error.contains("schema_version")));
        assert!(report
            .enum_errors
            .iter()
            .any(|error| error.contains("risk_level")));
        let repair = report.repair_instruction.as_deref().unwrap();
        assert!(repair.contains("Retry tool call 'propose_edit'"));
        assert!(repair.contains("Remove unsupported fields: extra"));

        let payload = report.to_state_payload();
        assert_eq!(payload["source"], "strict_tool_schema");
        assert_eq!(payload["tool_name"], "propose_edit");
        assert_eq!(payload["schema_version"], STRICT_SCHEMA_VERSION);
        assert_eq!(payload["valid"], false);
        assert!(payload["repair_instruction"]
            .as_str()
            .unwrap()
            .contains("Retry tool call"));
    }

    #[test]
    fn strict_tool_repair_policy_retries_then_aborts() {
        let report = validate_strict_tool_arguments(
            "propose_edit",
            &json!({
                "schema_version": 999,
                "path": "src/main.rs",
                "intent": 42,
                "expected_effect": "bad",
                "risk_level": "risky",
                "verification_steps": ["cargo test"]
            }),
        )
        .unwrap();
        let policy = RepairPolicy {
            max_repair_turns: 2,
            record_failure_hypotheses: true,
        };

        let retry = decide_tool_schema_repair(&report, 2, &policy);
        assert_eq!(retry.action, ToolSchemaRepairAction::Retry);
        assert!(!retry.should_record_failure);
        assert!(retry
            .instruction
            .as_deref()
            .unwrap()
            .contains("Retry tool call"));

        let abort = decide_tool_schema_repair(&report, 3, &policy);
        assert_eq!(abort.action, ToolSchemaRepairAction::Abort);
        assert!(abort.should_record_failure);
        assert!(abort.instruction.is_none());
        assert!(abort.reason.contains("repair budget exhausted"));
    }

    #[test]
    fn strict_tool_repair_policy_accepts_valid_arguments() {
        let report = validate_strict_tool_arguments(
            "record_failure",
            &json!({
                "schema_version": STRICT_SCHEMA_VERSION,
                "failure_summary": "schema repair test",
                "hypothesis": "arguments match schema",
                "evidence_event_ids": ["evt-1"],
                "affected_component": "tool_schema",
                "next_repair_step": "none"
            }),
        )
        .unwrap();
        let decision =
            decide_tool_schema_repair(&report, 1, &DeepSeekHarnessGenome::default().repair_policy);

        assert_eq!(decision.action, ToolSchemaRepairAction::Accept);
        assert!(!decision.should_record_failure);
        assert!(decision.instruction.is_none());
    }

    #[test]
    fn promotion_and_approval_schemas_are_strict_and_versioned() {
        let promotion = promote_or_reject_patch_schema();
        let approval = request_human_approval_schema();

        assert_eq!(promotion["function"]["strict"], true);
        assert_eq!(
            promotion["function"]["parameters"]["properties"]["decision"]["enum"][0].as_str(),
            Some("promote")
        );
        assert_eq!(
            promotion["function"]["parameters"]["properties"]["schema_version"]["enum"][0].as_u64(),
            Some(STRICT_SCHEMA_VERSION as u64)
        );
        assert_eq!(approval["function"]["strict"], true);
        assert_eq!(
            approval["function"]["parameters"]["properties"]["approval_scope"]["enum"][0].as_str(),
            Some("harness_patch_promotion")
        );
        assert_eq!(
            approval["function"]["parameters"]["properties"]["schema_version"]["enum"][0].as_u64(),
            Some(STRICT_SCHEMA_VERSION as u64)
        );
    }

    #[test]
    fn strict_tool_call_request_builds_deepseek_chat_payload() {
        let request = build_strict_tool_call_request(StrictToolCallRequestOptions {
            user_prompt: "Inspect the target file before editing.".into(),
            model: DEFAULT_MODEL.into(),
            tool_names: vec!["inspect_file".into(), "record_failure".into()],
            thinking: ThinkingMode::Enabled {
                effort: ThinkingEffort::High,
            },
            max_tokens: 512,
            stream: false,
        })
        .unwrap();

        assert_eq!(
            request.endpoint,
            format!("{DEFAULT_BASE_URL}{CHAT_COMPLETIONS_PATH}")
        );
        assert_eq!(request.model, DEFAULT_MODEL);
        assert_eq!(request.tool_names, vec!["inspect_file", "record_failure"]);
        assert_eq!(request.payload["model"], DEFAULT_MODEL);
        assert_eq!(request.payload["thinking"]["type"], "enabled");
        assert_eq!(request.payload["reasoning_effort"], "high");
        assert_eq!(request.payload["tool_choice"], "auto");
        assert_eq!(request.payload["tools"].as_array().unwrap().len(), 2);
        assert_eq!(request.payload["tools"][0]["function"]["strict"], true);
        assert_eq!(
            request.payload["tools"][0]["function"]["parameters"]["additionalProperties"],
            false
        );
        assert_eq!(
            request.payload["messages"][0]["content"],
            "Inspect the target file before editing."
        );
    }

    #[test]
    fn strict_tool_call_request_rejects_unknown_tool_or_empty_prompt() {
        let empty = build_strict_tool_call_request(StrictToolCallRequestOptions {
            user_prompt: " ".into(),
            model: DEFAULT_MODEL.into(),
            tool_names: vec!["inspect_file".into()],
            thinking: ThinkingMode::Disabled,
            max_tokens: 128,
            stream: false,
        })
        .unwrap_err();
        assert!(empty.contains("prompt cannot be empty"));

        let unknown = build_strict_tool_call_request(StrictToolCallRequestOptions {
            user_prompt: "Use a strict tool.".into(),
            model: DEFAULT_MODEL.into(),
            tool_names: vec!["missing_tool".into()],
            thinking: ThinkingMode::Disabled,
            max_tokens: 128,
            stream: false,
        })
        .unwrap_err();
        assert!(unknown.contains("unknown strict schema"));

        let duplicate = build_strict_tool_call_request(StrictToolCallRequestOptions {
            user_prompt: "Use a strict tool.".into(),
            model: DEFAULT_MODEL.into(),
            tool_names: vec!["inspect_file".into(), "inspect_file".into()],
            thinking: ThinkingMode::Disabled,
            max_tokens: 128,
            stream: false,
        })
        .unwrap_err();
        assert!(duplicate.contains("duplicate strict schema"));
    }

    #[test]
    fn json_output_parses_valid_first_attempt() {
        let result = parse_json_output_attempts(
            [r#"{"summary":"ok"}"#],
            JsonOutputParseOptions::extraction("test-json", Some("summary".to_string())),
        )
        .unwrap();

        assert_eq!(result.value["summary"], "ok");
        assert_eq!(result.attempts.len(), 1);
        assert!(!result.retry_used);
        assert_eq!(result.attempts[0].status, JsonOutputAttemptStatus::Parsed);
    }

    #[test]
    fn json_output_retries_empty_or_invalid_once() {
        let result = parse_json_output_attempts(
            ["", r#"{"summary":"retry-ok"}"#],
            JsonOutputParseOptions::extraction("test-json", None),
        )
        .unwrap();

        assert_eq!(result.value["summary"], "retry-ok");
        assert!(result.retry_used);
        assert_eq!(result.attempts[0].status, JsonOutputAttemptStatus::Empty);
        assert_eq!(result.attempts[1].status, JsonOutputAttemptStatus::Parsed);
    }

    #[test]
    fn json_output_failure_payload_is_state_ready() {
        let failure = parse_json_output_attempts(
            ["not-json", "still-not-json"],
            JsonOutputParseOptions::extraction("structured-extraction", Some("decision".into())),
        )
        .unwrap_err();

        assert_eq!(failure.attempts.len(), 2);
        assert_eq!(failure.attempts[0].status, JsonOutputAttemptStatus::Invalid);
        let payload = failure.to_state_payload();
        assert_eq!(payload["source"], "json_output");
        assert_eq!(payload["operation"], "structured-extraction");
        assert_eq!(payload["schema_name"], "decision");
        assert_eq!(payload["retry_allowed"], true);
        assert_eq!(payload["attempts"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn json_retry_instruction_names_schema_when_available() {
        assert!(json_retry_instruction(Some("record_eval_result")).contains("record_eval_result"));
        assert!(response_format_json_object()
            .to_string()
            .contains("json_object"));
    }

    #[test]
    fn json_output_request_builds_response_format_payload() {
        let request = build_json_output_request(JsonOutputRequestOptions {
            user_prompt: "Summarize as JSON.".into(),
            model: DEFAULT_MODEL.into(),
            schema_name: Some("summary".into()),
            max_tokens: 128,
            stream: false,
        })
        .unwrap();

        assert_eq!(
            request.endpoint,
            format!("{DEFAULT_BASE_URL}{CHAT_COMPLETIONS_PATH}")
        );
        assert_eq!(request.model, DEFAULT_MODEL);
        assert_eq!(request.schema_name.as_deref(), Some("summary"));
        assert_eq!(request.payload["response_format"]["type"], "json_object");
        assert_eq!(request.payload["thinking"]["type"], "disabled");
        assert_eq!(request.payload["max_tokens"], 128);
        assert!(request.payload["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains("schema 'summary'"));
    }

    #[test]
    fn json_output_request_rejects_empty_prompt_or_unknown_model() {
        let empty = build_json_output_request(JsonOutputRequestOptions {
            user_prompt: " ".into(),
            model: DEFAULT_MODEL.into(),
            schema_name: None,
            max_tokens: 128,
            stream: false,
        })
        .unwrap_err();
        assert!(empty.contains("prompt cannot be empty"));

        let unknown = build_json_output_request(JsonOutputRequestOptions {
            user_prompt: "Return JSON.".into(),
            model: "not-deepseek".into(),
            schema_name: None,
            max_tokens: 128,
            stream: false,
        })
        .unwrap_err();
        assert!(unknown.contains("unsupported DeepSeek model"));
    }

    #[test]
    fn chat_prefix_request_builds_beta_chat_payload() {
        let request = build_chat_prefix_request(ChatPrefixRequestOptions {
            user_prompt: "Write a concise release note.".into(),
            assistant_prefix: "Release note:".into(),
            model: DEFAULT_MODEL.into(),
            max_tokens: 64,
            temperature: Some(0.2),
            stream: false,
        })
        .unwrap();

        assert_eq!(
            request.endpoint,
            format!("{FIM_BETA_BASE_URL}{CHAT_COMPLETIONS_PATH}")
        );
        assert_eq!(request.model, DEFAULT_MODEL);
        assert_eq!(request.payload["model"], DEFAULT_MODEL);
        assert_eq!(request.payload["max_tokens"], 64);
        assert_eq!(request.payload["temperature"], 0.2);
        let messages = request.payload["messages"].as_array().unwrap();
        assert_eq!(messages[0]["role"], "user");
        assert_eq!(messages[1]["role"], "assistant");
        assert_eq!(messages[1]["content"], "Release note:");
        assert_eq!(messages[1]["prefix"], true);
    }

    #[test]
    fn chat_prefix_request_rejects_empty_or_unknown_model() {
        let empty = build_chat_prefix_request(ChatPrefixRequestOptions {
            user_prompt: " ".into(),
            assistant_prefix: "Prefix".into(),
            model: DEFAULT_MODEL.into(),
            max_tokens: 64,
            temperature: None,
            stream: false,
        });
        assert!(empty.unwrap_err().contains("user prompt"));

        let unknown = build_chat_prefix_request(ChatPrefixRequestOptions {
            user_prompt: "Task".into(),
            assistant_prefix: "Prefix".into(),
            model: "not-deepseek".into(),
            max_tokens: 64,
            temperature: None,
            stream: false,
        });
        assert!(unknown.unwrap_err().contains("unsupported DeepSeek model"));
    }

    #[test]
    fn fim_request_builds_beta_completion_payload_for_allowed_scope() {
        let policy = DeepSeekHarnessGenome::default().fim_policy;
        let request = build_fim_completion_request(
            FimRequestOptions {
                prompt: "fn add(a: i32, b: i32) -> i32 {".into(),
                suffix: Some("}".into()),
                scope: "localized-completion".into(),
                max_tokens: 128,
                temperature: Some(0.1),
                stream: false,
            },
            &policy,
        )
        .unwrap();

        assert_eq!(
            request.endpoint,
            "https://api.deepseek.com/beta/completions"
        );
        assert_eq!(request.model, DEFAULT_MODEL);
        assert_eq!(request.scope, "localized_completion");
        assert_eq!(request.payload["model"], DEFAULT_MODEL);
        assert_eq!(request.payload["suffix"], "}");
        assert_eq!(request.payload["max_tokens"], 128);
        assert_eq!(request.payload["temperature"], 0.1);
        assert!(!request.auto_routing_enabled);
    }

    #[test]
    fn deepseek_request_builders_do_not_emit_request_side_cache_control() {
        let json_request = build_json_output_request(JsonOutputRequestOptions {
            user_prompt: "Summarize as JSON.".into(),
            model: DEFAULT_MODEL.into(),
            schema_name: Some("summary".into()),
            max_tokens: 128,
            stream: false,
        })
        .unwrap();
        let tool_request = build_strict_tool_call_request(StrictToolCallRequestOptions {
            user_prompt: "Use one tool.".into(),
            model: DEFAULT_MODEL.into(),
            tool_names: vec!["record_failure".into()],
            thinking: ThinkingMode::Disabled,
            max_tokens: 128,
            stream: false,
        })
        .unwrap();
        let prefix_request = build_chat_prefix_request(ChatPrefixRequestOptions {
            user_prompt: "Write a concise release note.".into(),
            assistant_prefix: "Release note:".into(),
            model: DEFAULT_MODEL.into(),
            max_tokens: 64,
            temperature: Some(0.2),
            stream: false,
        })
        .unwrap();
        let fim_request = build_fim_completion_request(
            FimRequestOptions {
                prompt: "fn add(a: i32, b: i32) -> i32 {".into(),
                suffix: Some("}".into()),
                scope: "localized-completion".into(),
                max_tokens: 128,
                temperature: Some(0.1),
                stream: false,
            },
            &DeepSeekHarnessGenome::default().fim_policy,
        )
        .unwrap();

        for payload in [
            &json_request.payload,
            &tool_request.payload,
            &prefix_request.payload,
            &fim_request.payload,
        ] {
            assert!(
                !contains_json_key(payload, "cache_control"),
                "DeepSeek uses default server-side context caching; requests must not add cache_control"
            );
        }
    }

    #[test]
    fn fim_request_rejects_nonlocal_or_oversized_requests() {
        let policy = DeepSeekHarnessGenome::default().fim_policy;
        let err = build_fim_completion_request(
            FimRequestOptions {
                prompt: "x".into(),
                suffix: None,
                scope: "multi-file-refactor".into(),
                max_tokens: 128,
                temperature: None,
                stream: false,
            },
            &policy,
        )
        .unwrap_err();
        assert!(err.contains("disallowed"));

        let err = build_fim_completion_request(
            FimRequestOptions {
                prompt: "x".into(),
                suffix: None,
                scope: "localized_completion".into(),
                max_tokens: FIM_MAX_OUTPUT_TOKENS + 1,
                temperature: None,
                stream: false,
            },
            &policy,
        )
        .unwrap_err();
        assert!(err.contains("max_tokens"));
    }

    #[test]
    fn fim_route_accepts_explicit_single_file_line_range_when_policy_enabled() {
        let mut policy = DeepSeekHarnessGenome::default().fim_policy;
        policy.enabled = true;
        let decision = route_fim_for_prompt(
            FimRoutePromptOptions {
                input: "complete the missing block in src/deepseek.rs lines 12-18".into(),
                tracked_files: vec!["src/deepseek.rs".into(), "src/context.rs".into()],
                max_tokens: 512,
            },
            &policy,
        );

        assert!(decision.use_fim);
        assert_eq!(decision.route, "fim_complete");
        assert_eq!(decision.file_path.as_deref(), Some("src/deepseek.rs"));
        assert_eq!(decision.start_line, Some(12));
        assert_eq!(decision.end_line, Some(18));
        assert_eq!(decision.scope, "small_missing_block");
        assert_eq!(decision.command[0..3], ["yoyo", "deepseek", "fim-complete"]);
        assert!(decision.command.contains(&"--file".to_string()));
        assert!(decision.command.contains(&"src/deepseek.rs".to_string()));
    }

    #[test]
    fn fim_route_declines_disabled_broad_or_ambiguous_prompts() {
        let mut policy = DeepSeekHarnessGenome::default().fim_policy;
        let disabled = route_fim_for_prompt(
            FimRoutePromptOptions {
                input: "complete src/deepseek.rs lines 12-18".into(),
                tracked_files: vec!["src/deepseek.rs".into()],
                max_tokens: 256,
            },
            &policy,
        );
        assert!(!disabled.use_fim);
        assert!(disabled.reason.contains("disabled"));

        policy.enabled = true;
        let broad = route_fim_for_prompt(
            FimRoutePromptOptions {
                input: "refactor architecture across src/deepseek.rs lines 12-18".into(),
                tracked_files: vec!["src/deepseek.rs".into()],
                max_tokens: 256,
            },
            &policy,
        );
        assert!(!broad.use_fim);
        assert!(broad.reason.contains("broad"));

        let missing_range = route_fim_for_prompt(
            FimRoutePromptOptions {
                input: "fix the missing block in src/deepseek.rs".into(),
                tracked_files: vec!["src/deepseek.rs".into()],
                max_tokens: 256,
            },
            &policy,
        );
        assert!(!missing_range.use_fim);
        assert!(missing_range.reason.contains("line range"));

        let sensitive_target = route_fim_for_prompt(
            FimRoutePromptOptions {
                input: "complete .env lines 1-1".into(),
                tracked_files: vec![".env".into()],
                max_tokens: 256,
            },
            &policy,
        );
        assert!(!sensitive_target.use_fim);
        assert!(sensitive_target.reason.contains("security-sensitive"));
    }

    #[test]
    fn fim_route_decision_payload_is_state_ready_without_full_prompt() {
        let mut policy = DeepSeekHarnessGenome::default().fim_policy;
        policy.enabled = true;
        let decision = route_fim_for_prompt(
            FimRoutePromptOptions {
                input: "complete the missing block in src/deepseek.rs lines 12-18".into(),
                tracked_files: vec!["src/deepseek.rs".into()],
                max_tokens: 512,
            },
            &policy,
        );

        let payload =
            fim_route_decision_state_payload(&decision, "test_fim_route", &"x".repeat(400), true);

        assert_eq!(payload["decision_type"], "fim_route");
        assert_eq!(payload["decision"], "accepted");
        assert_eq!(payload["status"], "accepted");
        assert_eq!(payload["source"], "test_fim_route");
        assert_eq!(payload["file_path"], "src/deepseek.rs");
        assert_eq!(payload["start_line"], 12);
        assert_eq!(payload["end_line"], 18);
        assert_eq!(payload["execute"], true);
        assert_eq!(
            payload["prompt_preview"].as_str().unwrap().chars().count(),
            240
        );
    }

    #[test]
    #[serial]
    fn fim_file_request_builds_prefix_suffix_from_target_range() {
        let dir = tempfile::TempDir::new().unwrap();
        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        std::fs::write(
            "src.rs",
            "fn add(a: i32, b: i32) -> i32 {\n    0\n}\n\nfn keep() {}\n",
        )
        .unwrap();

        let request = build_fim_completion_request_for_file(
            FimFileRequestOptions {
                file_path: "src.rs".to_string(),
                start_line: 2,
                end_line: 2,
                scope: "localized_completion".to_string(),
                max_tokens: 64,
                temperature: None,
                stream: false,
            },
            &DeepSeekHarnessGenome::default().fim_policy,
        )
        .unwrap();

        std::env::set_current_dir(old_dir).unwrap();
        assert_eq!(request.payload["prompt"], "fn add(a: i32, b: i32) -> i32 {");
        assert_eq!(request.payload["suffix"], "}\n\nfn keep() {}");
        assert_eq!(request.payload["max_tokens"], 64);
    }

    #[test]
    fn fim_response_parser_extracts_text_usage_and_cache_metrics() {
        let parsed = parse_fim_completion_response(&json!({
            "choices": [{
                "text": "a + b",
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 12,
                "completion_tokens": 3,
                "prompt_cache_hit_tokens": 8,
                "prompt_cache_miss_tokens": 4
            }
        }))
        .unwrap();

        assert_eq!(parsed.text, "a + b");
        assert_eq!(parsed.finish_reason.as_deref(), Some("stop"));
        assert_eq!(parsed.usage.input_tokens, 12);
        assert_eq!(parsed.usage.output_tokens, 3);
        assert_eq!(parsed.usage.cache_hit_ratio(), Some(8.0 / 12.0));
    }

    #[test]
    fn chat_stream_parser_preserves_reasoning_tool_calls_and_usage() {
        let stream = concat!(
            "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"Need to inspect.\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"id\":\"call-1\",\"type\":\"function\"}]}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"id\":\"call-1\"}]}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"Done\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":12,\"completion_tokens\":3,\"prompt_cache_hit_tokens\":8,\"prompt_cache_miss_tokens\":4}}\n\n",
            "data: [DONE]\n\n",
        );

        let parsed = parse_chat_completion_sse(stream).unwrap();

        assert_eq!(parsed.content, "Done");
        assert_eq!(parsed.reasoning_content, "Need to inspect.");
        assert_eq!(parsed.tool_call_ids, vec!["call-1"]);
        assert_eq!(parsed.finish_reason.as_deref(), Some("stop"));
        assert_eq!(parsed.usage.input_tokens, 12);
        assert_eq!(parsed.usage.output_tokens, 3);
        assert_eq!(parsed.usage.cache_hit_ratio(), Some(8.0 / 12.0));
    }

    #[test]
    fn chat_stream_parser_rejects_empty_streams() {
        let err = parse_chat_completion_sse(": keep-alive\n\n").unwrap_err();
        assert_eq!(err, "DeepSeek streaming response contained no data chunks");
    }

    #[test]
    #[serial]
    fn fim_edit_plan_builds_single_file_patch_for_allowed_scope() {
        let dir = tempfile::TempDir::new().unwrap();
        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        std::fs::write(
            "src.rs",
            "fn add(a: i32, b: i32) -> i32 {\n    0\n}\n\nfn keep() {}\n",
        )
        .unwrap();

        let plan = build_fim_edit_plan(
            FimEditOptions {
                file_path: "src.rs".to_string(),
                start_line: 2,
                end_line: 2,
                completion: "    a + b".to_string(),
                scope: "localized-completion".to_string(),
            },
            &DeepSeekHarnessGenome::default().fim_policy,
        )
        .unwrap();

        assert_eq!(plan.scope, "localized_completion");
        assert_eq!(plan.inserted_lines, 1);
        assert_eq!(plan.removed_lines, 1);
        assert!(plan.requires_explicit_apply);
        assert!(plan.patch.contains("--- a/src.rs"));
        assert!(plan.patch.contains("-    0"));
        assert!(plan.patch.contains("+    a + b"));

        std::env::set_current_dir(old_dir).unwrap();
    }

    #[test]
    fn fim_edit_plan_rejects_nonlocal_scope_and_unsafe_path() {
        let policy = DeepSeekHarnessGenome::default().fim_policy;
        let err = build_fim_edit_plan(
            FimEditOptions {
                file_path: "../src.rs".to_string(),
                start_line: 1,
                end_line: 1,
                completion: "x".to_string(),
                scope: "localized_completion".to_string(),
            },
            &policy,
        )
        .unwrap_err();
        assert!(err.contains("parent directories"));

        let err = build_fim_edit_plan(
            FimEditOptions {
                file_path: "src.rs".to_string(),
                start_line: 1,
                end_line: 1,
                completion: "x".to_string(),
                scope: "multi-file-refactor".to_string(),
            },
            &policy,
        )
        .unwrap_err();
        assert!(err.contains("disallowed"));

        let err = build_fim_edit_plan(
            FimEditOptions {
                file_path: ".env".to_string(),
                start_line: 1,
                end_line: 1,
                completion: "DEEPSEEK_API_KEY=test".to_string(),
                scope: "localized_completion".to_string(),
            },
            &policy,
        )
        .unwrap_err();
        assert!(err.contains("security-sensitive"));
    }

    // ── classify_deepseek_transport_failure ──────────────────────────

    #[test]
    fn transport_failure_classifies_401_as_authentication() {
        let policy = DeepSeekTransportPolicy {
            max_retries: 2,
            retry_statuses: vec![429],
            ..default_transport_policy()
        };
        let decision = classify_deepseek_transport_failure(Some(401), "", 0, &policy);
        assert_eq!(decision.class, DeepSeekTransportErrorClass::Authentication);
        assert!(!decision.retryable);
        assert_eq!(decision.status, Some(401));
        assert!(decision.next_backoff_ms.is_none());
    }

    #[test]
    fn transport_failure_classifies_403_as_permission_denied() {
        let policy = default_transport_policy();
        let decision = classify_deepseek_transport_failure(Some(403), "", 0, &policy);
        assert_eq!(
            decision.class,
            DeepSeekTransportErrorClass::PermissionDenied
        );
        assert!(!decision.retryable);
    }

    #[test]
    fn transport_failure_classifies_429_as_rate_limited_and_retryable() {
        let policy = default_transport_policy();
        let decision = classify_deepseek_transport_failure(Some(429), "", 0, &policy);
        assert_eq!(decision.class, DeepSeekTransportErrorClass::RateLimited);
        assert!(decision.retryable);
        assert!(decision.next_backoff_ms.is_some());
        assert!(decision.reason.contains("rate limit"));
    }

    #[test]
    fn transport_failure_classifies_5xx_as_server_error_and_retryable() {
        let policy = default_transport_policy();
        for status in [500, 502, 503, 504] {
            let decision = classify_deepseek_transport_failure(Some(status), "", 1, &policy);
            assert_eq!(decision.class, DeepSeekTransportErrorClass::ServerError);
            assert!(decision.retryable);
            assert!(decision.next_backoff_ms.is_some());
        }
    }

    #[test]
    fn transport_failure_classifies_404_as_not_found_not_retryable() {
        let policy = default_transport_policy();
        let decision = classify_deepseek_transport_failure(Some(404), "", 0, &policy);
        assert_eq!(decision.class, DeepSeekTransportErrorClass::NotFound);
        assert!(!decision.retryable);
        assert!(decision.reason.contains("not safe to retry"));
    }

    #[test]
    fn transport_failure_classifies_timeout_from_error_text() {
        let policy = DeepSeekTransportPolicy {
            max_retries: 3,
            ..default_transport_policy()
        };
        // Timeout in error text overrides None status
        let decision = classify_deepseek_transport_failure(None, "request timed out", 1, &policy);
        assert_eq!(decision.class, DeepSeekTransportErrorClass::Timeout);
        assert!(decision.retryable);
    }

    #[test]
    fn transport_failure_classifies_connection_refused_as_network() {
        let policy = default_transport_policy();
        let decision = classify_deepseek_transport_failure(None, "connection refused", 0, &policy);
        assert_eq!(decision.class, DeepSeekTransportErrorClass::Network);
        assert!(decision.retryable);
    }

    #[test]
    fn transport_failure_classifies_context_length_as_non_retryable() {
        let policy = default_transport_policy();
        let decision =
            classify_deepseek_transport_failure(None, "context length exceeded", 0, &policy);
        assert_eq!(decision.class, DeepSeekTransportErrorClass::ContextLength);
        assert!(!decision.retryable);
    }

    #[test]
    fn transport_failure_retry_budget_exhausted_stops_retrying() {
        let policy = DeepSeekTransportPolicy {
            max_retries: 2,
            ..default_transport_policy()
        };
        // 429 is transient, but attempt == max_retries, so no more retries
        let decision = classify_deepseek_transport_failure(Some(429), "", 2, &policy);
        assert_eq!(decision.class, DeepSeekTransportErrorClass::RateLimited);
        assert!(!decision.retryable);
        assert!(decision.reason.contains("retry budget exhausted"));
        assert!(decision.next_backoff_ms.is_none());
    }

    // ── extract_deepseek_transport_status ────────────────────────────

    #[test]
    fn extract_status_from_http_error_message() {
        assert_eq!(
            extract_deepseek_transport_status("HTTP 429 Too Many Requests"),
            Some(429)
        );
        assert_eq!(
            extract_deepseek_transport_status("status code 500 server error"),
            Some(500)
        );
        assert_eq!(
            extract_deepseek_transport_status("got 503 Service Unavailable"),
            Some(503)
        );
    }

    #[test]
    fn extract_status_returns_none_when_no_valid_http_code() {
        assert_eq!(
            extract_deepseek_transport_status("connection reset by peer"),
            None
        );
        assert_eq!(
            extract_deepseek_transport_status("timeout after 30000ms"),
            None
        );
        assert_eq!(extract_deepseek_transport_status(""), None);
    }

    #[test]
    fn extract_status_prefers_first_valid_http_range_code() {
        // "666" and "999" are not valid HTTP status codes, "500" is
        assert_eq!(
            extract_deepseek_transport_status("error 666 then 500 then 999"),
            Some(500)
        );
    }

    // ── normalize_scope ──────────────────────────────────────────────

    #[test]
    fn normalize_scope_trims_and_lowercases() {
        assert_eq!(
            normalize_scope("  Localized-Completion "),
            "localized_completion"
        );
        assert_eq!(
            normalize_scope("SINGLE_FUNCTION_BODY"),
            "single_function_body"
        );
    }

    #[test]
    fn normalize_scope_replaces_hyphens_and_spaces_with_underscores() {
        assert_eq!(
            normalize_scope("multi-file-refactor"),
            "multi_file_refactor"
        );
        assert_eq!(normalize_scope("one method repair"), "one_method_repair");
    }

    #[test]
    fn normalize_scope_handles_mixed_input() {
        assert_eq!(
            normalize_scope("  Multi-File Refactor "),
            "multi_file_refactor"
        );
    }

    // ── validate_strict_tool_arguments (additional edge cases) ────────

    #[test]
    fn strict_tool_arguments_missing_required_fields() {
        let report = validate_strict_tool_arguments(
            "record_failure",
            &json!({
                "schema_version": STRICT_SCHEMA_VERSION
                // missing: failure_summary, hypothesis, evidence_event_ids,
                //          affected_component, next_repair_step
            }),
        )
        .unwrap();
        assert!(!report.valid);
        assert!(!report.missing_required.is_empty());
        assert!(report
            .missing_required
            .contains(&"failure_summary".to_string()));
        assert!(report.missing_required.contains(&"hypothesis".to_string()));
    }

    #[test]
    fn strict_tool_arguments_rejects_non_object_input() {
        let report =
            validate_strict_tool_arguments("propose_edit", &json!("not an object")).unwrap();
        assert!(!report.valid);
        assert!(report
            .type_errors
            .iter()
            .any(|e| e.contains("must be a JSON object")));
        assert!(report.repair_instruction.is_some());
    }

    #[test]
    fn strict_tool_arguments_rejects_unknown_schema_name() {
        let err = validate_strict_tool_arguments("nonexistent_schema", &json!({})).unwrap_err();
        assert!(err.contains("unknown strict schema"));
    }

    fn default_transport_policy() -> DeepSeekTransportPolicy {
        DeepSeekTransportPolicy {
            connect_timeout_ms: 10_000,
            request_timeout_ms: 120_000,
            max_retries: 2,
            initial_backoff_ms: 1_000,
            max_backoff_ms: 20_000,
            retry_statuses: vec![408, 409, 425, 429, 500, 502, 503, 504],
        }
    }
}
