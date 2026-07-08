//! DeepSeek-native shell diagnostics.

use crate::format::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// Maximum events to scan for cache-report before capping.
/// Matching the state doctor and state crashes sampling cap of 20K events.
const CACHE_REPORT_EVENT_CAP: usize = 20_000;

fn default_events_path() -> PathBuf {
    let (config, _) = crate::config::load_deepseek_config_file();
    config
        .get("state_events")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".yoyo/state/events.jsonl"))
}

fn default_store_path_for_events(events_path: &Path) -> Option<PathBuf> {
    events_path
        .parent()
        .map(|parent| parent.join("state.sqlite"))
}

fn record_deepseek_diagnostic_event(event_type: crate::state::EventType, payload: Value) -> bool {
    if !crate::state::harness_internal_enabled() {
        return false;
    }

    if crate::state::is_initialized() {
        crate::state::record(event_type, crate::state::Actor::Harness, payload);
        return true;
    }

    let events_path = default_events_path();
    let config = crate::state::StateConfig {
        enabled: true,
        fail_soft: true,
        store_path: default_store_path_for_events(&events_path),
        events_path,
    };
    let recorder = crate::state::StateRecorder::new(config);
    match recorder.append(event_type, crate::state::Actor::Harness, payload) {
        Ok(_) => true,
        Err(e) => {
            eprintln!("{YELLOW}  warning: failed to record DeepSeek diagnostic state: {e}{RESET}");
            false
        }
    }
}

pub fn handle_deepseek_subcommand(args: &[String]) {
    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("help");
    match sub {
        "doctor" => handle_doctor(args),
        "genome" => handle_genome(args),
        "route" => handle_route(args),
        "models" => handle_models(),
        "schemas" => handle_schemas(args),
        "schema-check" => handle_schema_check(args),
        "test-tool-call" => handle_test_tool_call(args),
        "test-thinking" => handle_test_thinking(args),
        "stream-check" => handle_stream_check(args),
        "transport-check" => handle_transport_check(args),
        "json-check" => handle_json_check(args),
        "prefix-check" => handle_prefix_check(args),
        "fim-check" => handle_fim_check(args),
        "fim-route" => handle_fim_route(args),
        "fim-complete" => handle_fim_complete(args),
        "fim-parse" => handle_fim_parse(args),
        "fim-apply" => handle_fim_apply(args),
        "cache-report" => handle_cache_report(args),
        _ => print_usage(),
    }
}

pub fn handle_fim_slash_command(input: &str) {
    let args = match parse_fim_slash_command(input) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{YELLOW}  {e}{RESET}");
            return;
        }
    };
    handle_deepseek_subcommand(&args);
}

fn parse_fim_slash_command(input: &str) -> Result<Vec<String>, String> {
    let rest = input
        .trim()
        .strip_prefix("/fim")
        .ok_or_else(|| "Usage: /fim (--prompt TEXT [--suffix TEXT] | --file PATH --start N --end N) [--scope SCOPE] [--max-tokens N] [--response JSON] [--apply] [--verify CMD] [--json]".to_string())?
        .trim();
    if rest.is_empty() {
        return Err("Usage: /fim (--prompt TEXT [--suffix TEXT] | --file PATH --start N --end N) [--scope SCOPE] [--max-tokens N] [--response JSON] [--apply] [--verify CMD] [--json]".to_string());
    }
    let mut args = vec![
        "yoyo".to_string(),
        "deepseek".to_string(),
        "fim-complete".to_string(),
    ];
    args.extend(tokenize_slash_args(rest)?);
    Ok(args)
}

fn tokenize_slash_args(input: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;
    for ch in input.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        match quote {
            Some(q) if ch == q => quote = None,
            Some(_) => current.push(ch),
            None if ch == '\'' || ch == '"' => quote = Some(ch),
            None if ch.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            None => current.push(ch),
        }
    }
    if escaped {
        current.push('\\');
    }
    if quote.is_some() {
        return Err("unterminated quote in /fim arguments".to_string());
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    Ok(tokens)
}

fn handle_doctor(args: &[String]) {
    let (deepseek_config, _) = crate::config::load_deepseek_config_file();
    let genome = crate::deepseek::active_harness_genome();
    let base_url = deepseek_config.get("deepseek_base_url").map(String::as_str);
    let config = crate::agent_builder::create_model_config(
        "deepseek",
        &genome.model_routing_policy.final_patch_model,
        base_url,
    );

    if args.iter().any(|arg| arg == "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&doctor_summary_payload(&config, &genome))
                .unwrap_or_else(|_| "{}".to_string())
        );
        return;
    }

    println!("{}", render_doctor_summary(&config, &genome));
}

fn render_doctor_summary(
    config: &yoagent::provider::ModelConfig,
    genome: &crate::deepseek::DeepSeekHarnessGenome,
) -> String {
    let compat = config.compat.as_ref();
    let mut out = String::new();
    out.push_str("DeepSeek native doctor\n");
    out.push_str(&format!(
        "  default model: {}\n",
        genome.model_routing_policy.final_patch_model
    ));
    out.push_str(&format!(
        "  fast model:    {}\n",
        genome.model_routing_policy.quick_summary_model
    ));
    out.push_str(&format!("  base url:      {}\n", config.base_url));
    out.push_str(&format!(
        "  context:       {} tokens\n",
        config.context_window
    ));
    out.push_str(&format!("  max output:    {} tokens\n", config.max_tokens));
    out.push_str(&format!(
        "  context policy: failures={} changed_files={} repo_map={} instructions={}\n",
        genome.context_policy.recent_failure_limit,
        genome.context_policy.changed_file_limit,
        bool_label(genome.context_policy.include_repo_map),
        genome.context_policy.include_instruction_files.join(", ")
    ));
    out.push_str(&format!(
        "  json output:   {}\n",
        crate::deepseek::response_format_json_object()["type"]
            .as_str()
            .unwrap_or("json_object")
    ));
    out.push_str(&format!(
        "  fim beta url:  {}\n",
        crate::deepseek::FIM_BETA_BASE_URL
    ));
    out.push_str(&format!(
        "  prefix beta:   {}{}\n",
        crate::deepseek::FIM_BETA_BASE_URL,
        crate::deepseek::CHAT_COMPLETIONS_PATH
    ));
    out.push_str(&format!(
        "  reasoning:     {}\n",
        bool_label(config.reasoning)
    ));
    out.push_str(&format!(
        "  effort param:  {}\n",
        bool_label(compat.map(|c| c.supports_reasoning_effort).unwrap_or(false))
    ));
    out.push_str(&format!(
        "  thinking ctrl: {}\n",
        bool_label(compat.map(|c| c.supports_thinking_control).unwrap_or(false))
    ));
    out.push_str(&format!(
        "  stream usage:  {}\n",
        bool_label(
            compat
                .map(|c| c.supports_usage_in_streaming)
                .unwrap_or(false)
        )
    ));
    out.push_str(&format!(
        "  retry policy:  max_retries={} request_timeout={}ms\n",
        genome.transport_policy.max_retries, genome.transport_policy.request_timeout_ms
    ));
    out.push_str("  state default: harness-internal only\n");
    out.push_str(&format!(
        "  genome:       {}",
        crate::deepseek::HARNESS_GENOME_VERSION
    ));
    out
}

fn doctor_summary_payload(
    config: &yoagent::provider::ModelConfig,
    genome: &crate::deepseek::DeepSeekHarnessGenome,
) -> Value {
    let compat = config.compat.as_ref();
    json!({
        "diagnostic": "deepseek_doctor",
        "provider": config.provider.as_str(),
        "primary_model": genome.model_routing_policy.final_patch_model.as_str(),
        "fast_model": genome.model_routing_policy.quick_summary_model.as_str(),
        "base_url": config.base_url.as_str(),
        "context_window_tokens": config.context_window,
        "max_output_tokens": config.max_tokens,
        "context_policy": {
            "recent_failure_limit": genome.context_policy.recent_failure_limit,
            "changed_file_limit": genome.context_policy.changed_file_limit,
            "include_repo_map": genome.context_policy.include_repo_map,
            "include_instruction_files": genome.context_policy.include_instruction_files.clone(),
        },
        "json_output": crate::deepseek::response_format_json_object()["type"]
            .as_str()
            .unwrap_or("json_object"),
        "fim_beta_url": crate::deepseek::FIM_BETA_BASE_URL,
        "prefix_beta_url": format!(
            "{}{}",
            crate::deepseek::FIM_BETA_BASE_URL,
            crate::deepseek::CHAT_COMPLETIONS_PATH
        ),
        "reasoning": config.reasoning,
        "supports_reasoning_effort": compat
            .map(|compat| compat.supports_reasoning_effort)
            .unwrap_or(false),
        "supports_thinking_control": compat
            .map(|compat| compat.supports_thinking_control)
            .unwrap_or(false),
        "supports_usage_in_streaming": compat
            .map(|compat| compat.supports_usage_in_streaming)
            .unwrap_or(false),
        "retry_policy": {
            "max_retries": genome.transport_policy.max_retries,
            "request_timeout_ms": genome.transport_policy.request_timeout_ms,
        },
        "state_default": "harness_internal_only",
        "harness_genome_version": crate::deepseek::HARNESS_GENOME_VERSION,
    })
}

fn handle_genome(args: &[String]) {
    let genome = crate::deepseek::active_harness_genome();
    if args.iter().any(|arg| arg == "--json") {
        let raw = serde_json::to_string_pretty(&genome).unwrap_or_else(|_| "{}".to_string());
        println!("{raw}");
        return;
    }

    println!("{}", render_genome_summary(&genome));
}

fn render_genome_summary(genome: &crate::deepseek::DeepSeekHarnessGenome) -> String {
    let mut out = String::new();
    out.push_str("DeepSeek harness genome\n");
    out.push_str(&format!("  version:          {}\n", genome.version));
    out.push_str(&format!(
        "  root-cause model: {}\n",
        genome.model_routing_policy.root_cause_model
    ));
    out.push_str(&format!(
        "  fast model:       {}\n",
        genome.model_routing_policy.quick_summary_model
    ));
    out.push_str(&format!(
        "  prompt layout:    v{}\n",
        genome.prompt_layout_policy.version
    ));
    out.push_str(&format!(
        "  context:          failures={} changed_files={} repo_map={} instructions={}\n",
        genome.context_policy.recent_failure_limit,
        genome.context_policy.changed_file_limit,
        bool_label(genome.context_policy.include_repo_map),
        genome.context_policy.include_instruction_files.join(", ")
    ));
    out.push_str(&format!(
        "  cache:            server-side=default stable_prefix={} record_metrics={} optimize_order={}\n",
        bool_label(genome.cache_policy.stable_prefix),
        bool_label(genome.cache_policy.record_metrics),
        bool_label(genome.cache_policy.optimize_prompt_order)
    ));
    out.push_str(&format!(
        "  transport:        timeout={}ms retries={}\n",
        genome.transport_policy.request_timeout_ms, genome.transport_policy.max_retries
    ));
    out.push_str(&format!(
        "  strict schemas:   {}\n",
        genome.tool_schema_policy.critical_schemas.join(", ")
    ));
    out.push_str(&format!(
        "  fim:              {}\n",
        if genome.fim_policy.enabled {
            "enabled"
        } else {
            "disabled"
        }
    ));
    out.push_str(&format!(
        "  raw reasoning:    {}\n",
        if genome.memory_policy.persist_raw_reasoning {
            "persisted"
        } else {
            "not persisted"
        }
    ));
    out.push_str(&format!(
        "  human gates:      permission-policy patches={}",
        bool_label(
            genome
                .permission_policy
                .permission_policy_patches_need_human
        )
    ));
    out
}

fn handle_route(args: &[String]) {
    let Some(task) = args.get(3) else {
        eprintln!(
            "{YELLOW}  Usage: yoyo deepseek route <task> [--json]\n  tasks: root-cause, planning, risky-edit, final-patch, summary, memory, extraction, local-edit{RESET}"
        );
        return;
    };
    let genome = crate::deepseek::active_harness_genome();
    match crate::deepseek::route_for_task(task, &genome) {
        Ok(decision) if args.iter().any(|arg| arg == "--json") => {
            println!(
                "{}",
                serde_json::to_string_pretty(&decision).unwrap_or_else(|_| "{}".to_string())
            );
        }
        Ok(decision) => {
            println!("DeepSeek route");
            println!("  task:     {}", decision.task);
            println!("  model:    {}", decision.model);
            println!("  thinking: {}", thinking_label(decision.thinking));
            println!("  fim:      {}", bool_label(decision.use_fim));
            println!("  reason:   {}", decision.reason);
        }
        Err(e) => eprintln!("{YELLOW}  {e}{RESET}"),
    }
}

fn handle_models() {
    println!("DeepSeek native models");
    println!(
        "  {:<20} default  context={} max_output={}",
        crate::deepseek::DEFAULT_MODEL,
        crate::deepseek::CONTEXT_WINDOW_TOKENS,
        crate::deepseek::MAX_OUTPUT_TOKENS
    );
    println!(
        "  {:<20} fast     context={} max_output={}",
        crate::deepseek::FAST_MODEL,
        crate::deepseek::CONTEXT_WINDOW_TOKENS,
        crate::deepseek::MAX_OUTPUT_TOKENS
    );
}

fn handle_schemas(args: &[String]) {
    let schemas = crate::deepseek::strict_schema_suite();
    if args.iter().any(|arg| arg == "--json") {
        let raw = serde_json::to_string_pretty(&schemas).unwrap_or_else(|_| "[]".to_string());
        println!("{raw}");
        return;
    }

    println!("DeepSeek strict schemas");
    for schema in schemas {
        println!("  {}", format_schema_summary_line(&schema));
    }
}

fn format_schema_summary_line(schema: &serde_json::Value) -> String {
    let name = schema["function"]["name"].as_str().unwrap_or("?");
    let schema_version = schema["function"]["parameters"]["properties"]["schema_version"]["enum"]
        [0]
    .as_u64()
    .unwrap_or(0);
    let required_count = schema["function"]["parameters"]["required"]
        .as_array()
        .map(|fields| fields.len())
        .unwrap_or(0);
    format!("{name:<24} schema_version={schema_version} required_fields={required_count}")
}

fn handle_schema_check(args: &[String]) {
    let Some(name) = flag_value(args, "--name") else {
        eprintln!(
            "{YELLOW}  Usage: yoyo deepseek schema-check --name SCHEMA --arguments JSON [--attempt N] [--record] [--json]{RESET}"
        );
        return;
    };
    let Some(arguments) = flag_value(args, "--arguments") else {
        eprintln!(
            "{YELLOW}  Usage: yoyo deepseek schema-check --name SCHEMA --arguments JSON [--attempt N] [--record] [--json]{RESET}"
        );
        return;
    };
    let value = match serde_json::from_str::<Value>(arguments) {
        Ok(value) => value,
        Err(e) => {
            eprintln!("{YELLOW}  invalid --arguments JSON: {e}{RESET}");
            return;
        }
    };
    let report = match crate::deepseek::validate_strict_tool_arguments(name, &value) {
        Ok(report) => report,
        Err(e) => {
            eprintln!("{YELLOW}  DeepSeek schema check failed: {e}{RESET}");
            return;
        }
    };
    let failed_attempt = flag_value(args, "--attempt")
        .and_then(|raw| raw.parse::<u32>().ok())
        .unwrap_or(1);
    let genome = crate::deepseek::active_harness_genome();
    let repair_decision =
        crate::deepseek::decide_tool_schema_repair(&report, failed_attempt, &genome.repair_policy);
    if !report.valid
        && repair_decision.should_record_failure
        && args.iter().any(|arg| arg == "--record")
    {
        record_tool_schema_failure(&report, Some(&repair_decision));
    }
    if args.iter().any(|arg| arg == "--json") {
        let output = json!({
            "validation": report,
            "repair_decision": repair_decision,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
        );
        return;
    }
    println!("DeepSeek strict schema argument check");
    println!("  schema: {}", report.schema_name);
    println!("  valid:  {}", bool_label(report.valid));
    println!("  action: {:?}", repair_decision.action);
    println!("  reason: {}", repair_decision.reason);
    if let Some(instruction) = report.repair_instruction {
        println!("  repair: {instruction}");
    }
}

fn handle_test_tool_call(args: &[String]) {
    let record = args.iter().any(|arg| arg == "--record");
    let check = match crate::deepseek::validate_strict_tool_schema_suite() {
        Ok(check) => check,
        Err(e) => {
            eprintln!("{YELLOW}  DeepSeek strict tool-call check failed: {e}{RESET}");
            return;
        }
    };
    let request = match crate::deepseek::build_strict_tool_call_request(
        crate::deepseek::StrictToolCallRequestOptions {
            user_prompt: "Use a strict tool call to inspect src/main.rs before proposing an edit."
                .to_string(),
            model: crate::deepseek::DEFAULT_MODEL.to_string(),
            tool_names: vec![
                "inspect_file".to_string(),
                "propose_edit".to_string(),
                "record_failure".to_string(),
            ],
            thinking: crate::deepseek::ThinkingMode::native_default(),
            max_tokens: 512,
            stream: false,
        },
    ) {
        Ok(request) => request,
        Err(e) => {
            eprintln!("{YELLOW}  DeepSeek strict tool-call request rejected: {e}{RESET}");
            return;
        }
    };

    let recorded = record && record_strict_tool_call_pass(&check, &request);

    if args.iter().any(|arg| arg == "--json") {
        let output = json!({
            "check": check,
            "request": request,
            "recorded": recorded,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!("DeepSeek strict tool-call check passed");
        println!("  schemas: {}", check.details.len());
        for name in check.details {
            println!("  - {name}");
        }
        println!("  request endpoint: {}", request.endpoint);
        println!("  request tools:    {}", request.tool_names.join(", "));
        println!(
            "  thinking:         {}",
            request.payload["thinking"]["type"]
        );
        if recorded {
            println!("  recorded:         true");
        }
    }
}

fn tool_schema_failure_payload(
    report: &crate::deepseek::ToolSchemaValidationReport,
    decision: Option<&crate::deepseek::ToolSchemaRepairDecision>,
) -> Value {
    let mut payload = report.to_state_payload();
    if let Some(decision) = decision {
        if let Some(object) = payload.as_object_mut() {
            object.insert("failed_attempt".to_string(), json!(decision.failed_attempt));
            object.insert(
                "max_repair_turns".to_string(),
                json!(decision.max_repair_turns),
            );
            object.insert("repair_action".to_string(), json!(decision.action));
            object.insert("repair_reason".to_string(), json!(decision.reason.clone()));
        }
    }
    payload
}

fn record_tool_schema_failure(
    report: &crate::deepseek::ToolSchemaValidationReport,
    decision: Option<&crate::deepseek::ToolSchemaRepairDecision>,
) -> bool {
    if report.valid {
        return false;
    }
    record_deepseek_diagnostic_event(
        crate::state::EventType::ToolSchemaFailure,
        tool_schema_failure_payload(report, decision),
    )
}

fn strict_tool_call_pass_payload(
    check: &crate::deepseek::DeepSeekProtocolCheck,
    request: &crate::deepseek::DeepSeekStrictToolCallRequest,
) -> Value {
    json!({
        "source": "deepseek_protocol_check",
        "decision_type": "deepseek_strict_tool_call_check",
        "check": "test-tool-call",
        "decision": "passed",
        "schema_count": check.details.len(),
        "schema_names": check.details,
        "selected_tool_names": request.tool_names,
        "selected_tool_count": request.tool_names.len(),
        "model": request.model,
        "thinking": request.payload["thinking"]["type"].as_str().unwrap_or("-"),
        "reasoning_effort": request
            .payload
            .get("reasoning_effort")
            .and_then(Value::as_str),
        "stream": request.payload["stream"].as_bool().unwrap_or(false),
        "max_tokens": request.payload["max_tokens"].as_u64().unwrap_or(0),
    })
}

fn record_strict_tool_call_pass(
    check: &crate::deepseek::DeepSeekProtocolCheck,
    request: &crate::deepseek::DeepSeekStrictToolCallRequest,
) -> bool {
    record_deepseek_diagnostic_event(
        crate::state::EventType::DecisionRecorded,
        strict_tool_call_pass_payload(check, request),
    )
}

fn handle_test_thinking(args: &[String]) {
    let (messages, source) = match thinking_protocol_messages_from_args(args) {
        Ok(result) => result,
        Err(e) => {
            record_thinking_protocol_failure("provided-messages", &[], &e);
            eprintln!("{YELLOW}  DeepSeek thinking/tool-call protocol check failed: {e}{RESET}");
            return;
        }
    };
    let summary = thinking_protocol_probe_summary(&messages, source);
    match crate::deepseek::validate_deepseek_thinking_tool_call_messages(&messages) {
        Ok(check) if args.iter().any(|arg| arg == "--json") => {
            if args.iter().any(|arg| arg == "--record") {
                record_thinking_protocol_pass(source, summary.clone());
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "check": check,
                    "probe": summary,
                }))
                .unwrap_or_else(|_| "{}".to_string())
            );
        }
        Ok(check) => {
            println!("DeepSeek thinking/tool-call protocol check passed");
            println!("  source: {source}");
            println!("  messages: {}", summary["message_count"]);
            for detail in check.details {
                println!("  - {detail}");
            }
            if args.iter().any(|arg| arg == "--record") {
                record_thinking_protocol_pass(source, summary);
            }
        }
        Err(e) => {
            record_thinking_protocol_failure(source, &messages, &e);
            eprintln!("{YELLOW}  DeepSeek thinking/tool-call protocol check failed{RESET}");
        }
    }
}

fn thinking_protocol_messages_from_args(
    args: &[String],
) -> Result<(Vec<Value>, &'static str), String> {
    let Some(raw) = flag_value(args, "--messages") else {
        return Ok((
            crate::deepseek::deepseek_thinking_tool_call_probe_messages(),
            "builtin-probe",
        ));
    };
    let value = serde_json::from_str::<Value>(raw)
        .map_err(|e| format!("invalid --messages JSON for thinking protocol check: {e}"))?;
    let messages = if let Some(messages) = value.as_array() {
        messages.clone()
    } else if let Some(messages) = value.get("messages").and_then(Value::as_array) {
        messages.clone()
    } else {
        return Err(
            "--messages must be a JSON array or an object with a messages array".to_string(),
        );
    };
    if messages.is_empty() {
        return Err("--messages cannot be empty".to_string());
    }
    Ok((messages, "provided-messages"))
}

fn thinking_protocol_probe_summary(messages: &[Value], source: &str) -> Value {
    let mut assistant_tool_call_turns = 0usize;
    let mut reasoning_content_present = 0usize;
    let mut tool_result_turns = 0usize;
    for message in messages {
        match message.get("role").and_then(Value::as_str) {
            Some("assistant") => {
                let has_tool_calls = message
                    .get("tool_calls")
                    .and_then(Value::as_array)
                    .map(|tool_calls| !tool_calls.is_empty())
                    .unwrap_or(false);
                if has_tool_calls {
                    assistant_tool_call_turns += 1;
                    let has_reasoning = message
                        .get("reasoning_content")
                        .and_then(Value::as_str)
                        .map(|reasoning| !reasoning.trim().is_empty())
                        .unwrap_or(false);
                    if has_reasoning {
                        reasoning_content_present += 1;
                    }
                }
            }
            Some("tool") => tool_result_turns += 1,
            _ => {}
        }
    }
    serde_json::json!({
        "source": source,
        "message_count": messages.len(),
        "assistant_tool_call_turns": assistant_tool_call_turns,
        "assistant_tool_call_turns_with_reasoning_content": reasoning_content_present,
        "assistant_tool_call_turns_missing_reasoning_content": assistant_tool_call_turns.saturating_sub(reasoning_content_present),
        "tool_result_turns": tool_result_turns,
    })
}

fn thinking_protocol_pass_payload(source: &str, summary: Value) -> Value {
    serde_json::json!({
        "source": "deepseek_protocol_check",
        "decision_type": "deepseek_thinking_protocol_check",
        "check": "test-thinking",
        "decision": "passed",
        "diagnostic_source": source,
        "probe": summary,
    })
}

fn record_thinking_protocol_pass(source: &str, summary: Value) -> bool {
    record_deepseek_diagnostic_event(
        crate::state::EventType::DecisionRecorded,
        thinking_protocol_pass_payload(source, summary),
    )
}

fn record_thinking_protocol_failure(source: &str, messages: &[Value], error: &str) -> bool {
    record_deepseek_diagnostic_event(
        crate::state::EventType::FailureObserved,
        serde_json::json!({
            "source": "deepseek_protocol_check",
            "check": "test-thinking",
            "failure_class": "thinking_tool_call_protocol",
            "owner": "upstream_provider_or_message_replay",
            "retryable": false,
            "diagnostic_source": source,
            "probe": thinking_protocol_probe_summary(messages, source),
            "error_preview": error,
        }),
    )
}

fn handle_transport_check(args: &[String]) {
    let status = flag_value(args, "--status").and_then(|s| s.parse::<u16>().ok());
    let error = flag_value(args, "--error")
        .or_else(|| flag_value(args, "--body"))
        .map(|s| s.as_str())
        .unwrap_or("");
    let record = args.iter().any(|arg| arg == "--record");
    let attempt = flag_value(args, "--attempt")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    let genome = crate::deepseek::active_harness_genome();
    let decision = crate::deepseek::classify_deepseek_transport_failure(
        status,
        error,
        attempt,
        &genome.transport_policy,
    );
    let recorded = record && record_transport_check_pass(&decision, error);

    if args.iter().any(|arg| arg == "--json") {
        let output = json!({
            "decision": decision,
            "recorded": recorded,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
        );
        return;
    }

    println!("DeepSeek transport check");
    println!("  class:       {:?}", decision.class);
    println!("  retryable:   {}", bool_label(decision.retryable));
    println!(
        "  attempt:     {}/{}",
        decision.attempt, decision.max_retries
    );
    println!(
        "  backoff:     {}",
        decision
            .next_backoff_ms
            .map(|value| format!("{value}ms"))
            .unwrap_or_else(|| "-".to_string())
    );
    println!("  reason:      {}", decision.reason);
    if recorded {
        println!("  recorded:    true");
    }
}

fn handle_stream_check(args: &[String]) {
    let json_output = args.iter().any(|arg| arg == "--json");
    let record = args.iter().any(|arg| arg == "--record");
    let stream = match flag_value(args, "--sse") {
        Some(value) => value.as_str(),
        None => default_stream_check_sse(),
    };
    match crate::deepseek::parse_chat_completion_sse(stream) {
        Ok(parsed) => {
            let recorded = record && record_stream_check_pass(&parsed);
            if json_output {
                let output = json!({
                    "parsed": parsed,
                    "recorded": recorded,
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
                );
                return;
            }
            println!("DeepSeek stream check passed");
            println!("  content chars:   {}", parsed.content.chars().count());
            println!(
                "  reasoning chars: {}",
                parsed.reasoning_content.chars().count()
            );
            println!("  tool calls:      {}", parsed.tool_call_ids.len());
            println!(
                "  finish:          {}",
                parsed.finish_reason.as_deref().unwrap_or("-")
            );
            println!("  input tokens:    {}", parsed.usage.input_tokens);
            println!("  output tokens:   {}", parsed.usage.output_tokens);
            if let Some(ratio) = parsed.usage.cache_hit_ratio() {
                println!("  cache hit ratio: {:.2}%", ratio * 100.0);
            }
            if recorded {
                println!("  recorded:        true");
            }
        }
        Err(e) => {
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "parsed": false,
                        "error": e,
                    }))
                    .unwrap_or_else(|_| "{}".to_string())
                );
                return;
            }
            eprintln!("{YELLOW}  DeepSeek stream check failed: {e}{RESET}");
        }
    }
}

fn default_stream_check_sse() -> &'static str {
    concat!(
        "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"Need to inspect.\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"id\":\"call-1\",\"type\":\"function\"}]}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"Done\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":12,\"completion_tokens\":3,\"prompt_cache_hit_tokens\":8,\"prompt_cache_miss_tokens\":4}}\n\n",
        "data: [DONE]\n\n",
    )
}

fn stream_check_pass_payload(parsed: &crate::deepseek::DeepSeekStreamingChatSummary) -> Value {
    json!({
        "source": "deepseek_protocol_check",
        "decision_type": "deepseek_streaming_protocol_check",
        "check": "stream-check",
        "decision": "passed",
        "content_chars": parsed.content.chars().count(),
        "reasoning_content_chars": parsed.reasoning_content.chars().count(),
        "tool_call_count": parsed.tool_call_ids.len(),
        "finish_reason": parsed.finish_reason,
        "input_tokens": parsed.usage.input_tokens,
        "output_tokens": parsed.usage.output_tokens,
        "cache_hit_tokens": parsed.usage.cache_hit_tokens,
        "cache_miss_tokens": parsed.usage.cache_miss_tokens,
    })
}

fn record_stream_check_pass(parsed: &crate::deepseek::DeepSeekStreamingChatSummary) -> bool {
    record_deepseek_diagnostic_event(
        crate::state::EventType::DecisionRecorded,
        stream_check_pass_payload(parsed),
    )
}

fn transport_check_pass_payload(
    decision: &crate::deepseek::DeepSeekTransportDecision,
    error_text: &str,
) -> Value {
    json!({
        "source": "deepseek_protocol_check",
        "decision_type": "deepseek_transport_policy_check",
        "check": "transport-check",
        "decision": "passed",
        "transport_class": decision.class,
        "status": decision.status,
        "attempt": decision.attempt,
        "max_retries": decision.max_retries,
        "retryable": decision.retryable,
        "next_backoff_ms": decision.next_backoff_ms,
        "reason": decision.reason,
        "error_preview": preview(error_text, 160),
    })
}

fn record_transport_check_pass(
    decision: &crate::deepseek::DeepSeekTransportDecision,
    error_text: &str,
) -> bool {
    record_deepseek_diagnostic_event(
        crate::state::EventType::DecisionRecorded,
        transport_check_pass_payload(decision, error_text),
    )
}

fn handle_json_check(args: &[String]) {
    let Some(input) = flag_value(args, "--input") else {
        eprintln!(
            "{YELLOW}  Usage: yoyo deepseek json-check --input TEXT [--retry-input TEXT] [--schema NAME] [--record] [--json]{RESET}"
        );
        return;
    };
    let json_output = args.iter().any(|arg| arg == "--json");
    let record = args.iter().any(|arg| arg == "--record");
    let schema_name = flag_value(args, "--schema").map(|s| s.to_string());
    let request_probe =
        crate::deepseek::build_json_output_request(crate::deepseek::JsonOutputRequestOptions {
            user_prompt:
                "Return a compact JSON object for DeepSeek JSON-output protocol diagnostics."
                    .to_string(),
            model: crate::deepseek::DEFAULT_MODEL.to_string(),
            schema_name: schema_name.clone(),
            max_tokens: 256,
            stream: false,
        });
    let attempts = flag_value(args, "--retry-input")
        .map(|retry| vec![input.as_str(), retry.as_str()])
        .unwrap_or_else(|| vec![input.as_str()]);
    let options = crate::deepseek::JsonOutputParseOptions::extraction(
        "deepseek-json-check",
        schema_name.clone(),
    );

    match crate::deepseek::parse_json_output_attempts(attempts, options) {
        Ok(result) => {
            let recorded = record && record_json_check_pass(schema_name.as_deref(), &result);
            if json_output {
                let output = json!({
                    "parsed": true,
                    "result": result,
                    "request": request_probe.ok(),
                    "recorded": recorded,
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
                );
                return;
            }
            println!("DeepSeek JSON output check passed");
            println!("  attempts:   {}", result.attempts.len());
            println!("  retry used: {}", bool_label(result.retry_used));
            if let Ok(request) = request_probe {
                println!(
                    "  request response_format: {}",
                    request.payload["response_format"]["type"]
                );
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&result.value).unwrap_or_else(|_| "{}".to_string())
            );
        }
        Err(failure) => {
            let recorded = should_record_json_check_failure(json_output, record);
            let recorded = recorded
                && record_deepseek_diagnostic_event(
                    crate::state::EventType::JsonOutputFailure,
                    failure.to_state_payload(),
                );
            if json_output {
                let output = json!({
                    "parsed": false,
                    "failure": failure,
                    "request": request_probe.ok(),
                    "recorded": recorded,
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
                );
                return;
            }
            eprintln!("{YELLOW}  DeepSeek JSON output check failed{RESET}");
            eprintln!("  attempts: {}", failure.attempts.len());
            eprintln!(
                "  retry hint: {}",
                crate::deepseek::json_retry_instruction(failure.schema_name.as_deref())
            );
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&failure.to_state_payload())
                    .unwrap_or_else(|_| "{}".to_string())
            );
        }
    }
}

fn should_record_json_check_failure(json_output: bool, record_flag: bool) -> bool {
    record_flag || !json_output
}

fn json_check_pass_payload(
    schema_name: Option<&str>,
    result: &crate::deepseek::JsonOutputParseResult,
) -> Value {
    json!({
        "source": "json_output",
        "decision_type": "deepseek_json_output_check",
        "check": "json-check",
        "decision": "passed",
        "schema_name": schema_name,
        "attempt_count": result.attempts.len(),
        "retry_used": result.retry_used,
        "attempt_statuses": result
            .attempts
            .iter()
            .map(|attempt| json_output_attempt_status_label(&attempt.status))
            .collect::<Vec<_>>(),
    })
}

fn record_json_check_pass(
    schema_name: Option<&str>,
    result: &crate::deepseek::JsonOutputParseResult,
) -> bool {
    record_deepseek_diagnostic_event(
        crate::state::EventType::DecisionRecorded,
        json_check_pass_payload(schema_name, result),
    )
}

fn json_output_attempt_status_label(
    status: &crate::deepseek::JsonOutputAttemptStatus,
) -> &'static str {
    match status {
        crate::deepseek::JsonOutputAttemptStatus::Empty => "empty",
        crate::deepseek::JsonOutputAttemptStatus::Invalid => "invalid",
        crate::deepseek::JsonOutputAttemptStatus::Parsed => "parsed",
    }
}

fn handle_prefix_check(args: &[String]) {
    let Some(prompt) = flag_value(args, "--prompt") else {
        eprintln!(
            "{YELLOW}  Usage: yoyo deepseek prefix-check --prompt TEXT --prefix TEXT [--model MODEL] [--max-tokens N] [--json]{RESET}"
        );
        return;
    };
    let Some(prefix) = flag_value(args, "--prefix") else {
        eprintln!(
            "{YELLOW}  Usage: yoyo deepseek prefix-check --prompt TEXT --prefix TEXT [--model MODEL] [--max-tokens N] [--json]{RESET}"
        );
        return;
    };
    let model = flag_value(args, "--model")
        .map(|s| s.to_string())
        .unwrap_or_else(|| crate::deepseek::DEFAULT_MODEL.to_string());
    let max_tokens = flag_value(args, "--max-tokens")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(256);
    let request =
        crate::deepseek::build_chat_prefix_request(crate::deepseek::ChatPrefixRequestOptions {
            user_prompt: prompt.to_string(),
            assistant_prefix: prefix.to_string(),
            model,
            max_tokens,
            temperature: None,
            stream: false,
        });
    match request {
        Ok(request) if args.iter().any(|arg| arg == "--json") => {
            println!(
                "{}",
                serde_json::to_string_pretty(&request).unwrap_or_else(|_| "{}".to_string())
            );
        }
        Ok(request) => {
            println!("DeepSeek chat prefix request check passed");
            println!("  endpoint:     {}", request.endpoint);
            println!("  model:        {}", request.model);
            println!("  max tokens:   {}", request.payload["max_tokens"]);
            println!("  stream:       {}", request.payload["stream"]);
            let messages = request
                .payload
                .get("messages")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            println!("  messages:     {}", messages.len());
            println!(
                "  prefix flag:  {}",
                bool_label(
                    messages
                        .iter()
                        .any(|message| message.get("prefix").and_then(Value::as_bool) == Some(true))
                )
            );
        }
        Err(e) => eprintln!("{YELLOW}  DeepSeek chat prefix request rejected: {e}{RESET}"),
    }
}

fn handle_fim_check(args: &[String]) {
    let prompt = flag_value(args, "--prompt");
    let file_path = flag_value(args, "--file");
    if prompt.is_none() && file_path.is_none() {
        eprintln!(
            "{YELLOW}  Usage: yoyo deepseek fim-check (--prompt TEXT [--suffix TEXT] | --file PATH --start N --end N) [--scope SCOPE] [--max-tokens N] [--json]{RESET}"
        );
        return;
    };
    let scope = flag_value(args, "--scope")
        .map(|s| s.as_str())
        .unwrap_or("localized_completion");
    let max_tokens = flag_value(args, "--max-tokens")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(256);
    let genome = crate::deepseek::active_harness_genome();
    let request = if let Some(file_path) = file_path {
        let Some(start_line) = flag_value(args, "--start").and_then(|s| s.parse::<usize>().ok())
        else {
            eprintln!("{YELLOW}  missing or invalid --start line{RESET}");
            return;
        };
        let Some(end_line) = flag_value(args, "--end").and_then(|s| s.parse::<usize>().ok()) else {
            eprintln!("{YELLOW}  missing or invalid --end line{RESET}");
            return;
        };
        crate::deepseek::build_fim_completion_request_for_file(
            crate::deepseek::FimFileRequestOptions {
                file_path: file_path.to_string(),
                start_line,
                end_line,
                scope: scope.to_string(),
                max_tokens,
                temperature: None,
                stream: false,
            },
            &genome.fim_policy,
        )
    } else {
        crate::deepseek::build_fim_completion_request(
            crate::deepseek::FimRequestOptions {
                prompt: prompt.unwrap().to_string(),
                suffix: flag_value(args, "--suffix").map(|s| s.to_string()),
                scope: scope.to_string(),
                max_tokens,
                temperature: None,
                stream: false,
            },
            &genome.fim_policy,
        )
    };
    match request {
        Ok(request) if args.iter().any(|arg| arg == "--json") => {
            println!(
                "{}",
                serde_json::to_string_pretty(&request).unwrap_or_else(|_| "{}".to_string())
            );
        }
        Ok(request) => {
            println!("DeepSeek FIM request check passed");
            println!("  endpoint:     {}", request.endpoint);
            println!("  model:        {}", request.model);
            println!("  scope:        {}", request.scope);
            println!(
                "  auto routing: {}",
                bool_label(request.auto_routing_enabled)
            );
            println!("  max tokens:   {}", request.payload["max_tokens"]);
            println!(
                "  suffix:       {}",
                bool_label(request.payload.get("suffix").is_some())
            );
            if let Some(prompt) = request.payload.get("prompt").and_then(Value::as_str) {
                println!("  prompt chars: {}", prompt.chars().count());
            }
        }
        Err(e) => eprintln!("{YELLOW}  DeepSeek FIM request rejected: {e}{RESET}"),
    }
}

fn handle_fim_route(args: &[String]) {
    let Some(input) = flag_value(args, "--input")
        .or_else(|| flag_value(args, "--prompt"))
        .map(|value| value.to_string())
    else {
        eprintln!(
            "{YELLOW}  Usage: yoyo deepseek fim-route --input TEXT [--max-tokens N] [--enable] [--record] [--execute] [--response JSON] [--apply] [--verify CMD] [--json]{RESET}"
        );
        return;
    };
    let tracked_files = crate::git::run_git(&["ls-files"])
        .ok()
        .map(|raw| {
            raw.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut genome = crate::deepseek::active_harness_genome();
    if args.iter().any(|arg| arg == "--enable") {
        genome.fim_policy.enabled = true;
    }
    let decision = crate::deepseek::route_fim_for_prompt(
        crate::deepseek::FimRoutePromptOptions {
            input: input.clone(),
            tracked_files,
            max_tokens: flag_value(args, "--max-tokens")
                .and_then(|raw| raw.parse::<u32>().ok())
                .unwrap_or(256),
        },
        &genome.fim_policy,
    );
    let execute = args.iter().any(|arg| arg == "--execute");
    if args.iter().any(|arg| arg == "--record") || execute {
        crate::deepseek::record_fim_route_decision(
            &decision,
            "deepseek_fim_route",
            &input,
            execute,
        );
    }
    if execute {
        match build_fim_route_execution_args(&decision, args) {
            Ok(execution_args) => handle_deepseek_subcommand(&execution_args),
            Err(e) if args.iter().any(|arg| arg == "--json") => {
                let output = json!({
                    "routed": false,
                    "decision": decision,
                    "error": e,
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
                );
            }
            Err(e) => eprintln!("{YELLOW}  DeepSeek FIM route declined: {e}{RESET}"),
        }
        return;
    }
    if args.iter().any(|arg| arg == "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&decision).unwrap_or_else(|_| "{}".to_string())
        );
        return;
    }

    println!("DeepSeek FIM route");
    println!("  route:  {}", decision.route);
    println!("  fim:    {}", bool_label(decision.use_fim));
    println!("  reason: {}", decision.reason);
    if decision.use_fim {
        println!(
            "  target: {}:{}-{}",
            decision.file_path.as_deref().unwrap_or("-"),
            decision.start_line.unwrap_or_default(),
            decision.end_line.unwrap_or_default()
        );
        println!("  scope:  {}", decision.scope);
        println!("  command: {}", shell_join(&decision.command));
    }
}

fn build_fim_route_execution_args(
    decision: &crate::deepseek::FimRouteDecision,
    source_args: &[String],
) -> Result<Vec<String>, String> {
    if !decision.use_fim {
        return Err(decision.reason.clone());
    }
    let mut args = decision.command.clone();
    if let Some(response) = flag_value(source_args, "--response") {
        args.push("--response".to_string());
        args.push(response.to_string());
    }
    if source_args.iter().any(|arg| arg == "--apply") {
        args.push("--apply".to_string());
    }
    if let Some(verify) = flag_value(source_args, "--verify") {
        args.push("--verify".to_string());
        args.push(verify.to_string());
    }
    if source_args.iter().any(|arg| arg == "--json") {
        args.push("--json".to_string());
    }
    Ok(args)
}

fn handle_fim_parse(args: &[String]) {
    let Some(response) = flag_value(args, "--response") else {
        eprintln!("{YELLOW}  Usage: yoyo deepseek fim-parse --response JSON{RESET}");
        return;
    };
    let value: Value = match serde_json::from_str(response) {
        Ok(value) => value,
        Err(e) => {
            eprintln!("{YELLOW}  invalid JSON response: {e}{RESET}");
            return;
        }
    };
    match crate::deepseek::parse_fim_completion_response(&value) {
        Ok(parsed) => {
            println!("DeepSeek FIM response parsed");
            println!("  text chars:    {}", parsed.text.chars().count());
            println!(
                "  finish reason: {}",
                parsed.finish_reason.as_deref().unwrap_or("-")
            );
            println!("  input tokens:  {}", parsed.usage.input_tokens);
            println!("  output tokens: {}", parsed.usage.output_tokens);
            if let Some(ratio) = parsed.usage.cache_hit_ratio() {
                println!("  cache ratio:   {:.2}%", ratio * 100.0);
            }
        }
        Err(e) => eprintln!("{YELLOW}  DeepSeek FIM response rejected: {e}{RESET}"),
    }
}

fn handle_fim_complete(args: &[String]) {
    let prompt = flag_value(args, "--prompt");
    let file_path = flag_value(args, "--file");
    if prompt.is_none() && file_path.is_none() {
        eprintln!(
            "{YELLOW}  Usage: yoyo deepseek fim-complete (--prompt TEXT [--suffix TEXT] | --file PATH --start N --end N) [--scope SCOPE] [--max-tokens N] [--response JSON] [--apply] [--verify CMD] [--json]{RESET}"
        );
        return;
    };
    let scope = flag_value(args, "--scope")
        .map(|s| s.as_str())
        .unwrap_or("localized_completion");
    let max_tokens = flag_value(args, "--max-tokens")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(256);
    let apply = args.iter().any(|arg| arg == "--apply");
    let json_output = args.iter().any(|arg| arg == "--json");
    let verify_command = flag_value(args, "--verify").map(|command| command.to_string());
    if verify_command.is_some() && !apply {
        eprintln!(
            "{YELLOW}  --verify requires --apply so the command checks the edited file{RESET}"
        );
        return;
    }
    if apply && file_path.is_none() {
        eprintln!("{YELLOW}  --apply requires --file with --start and --end{RESET}");
        return;
    }

    let genome = crate::deepseek::active_harness_genome();
    let request = if let Some(file_path) = file_path {
        let Some(start_line) = flag_value(args, "--start").and_then(|s| s.parse::<usize>().ok())
        else {
            eprintln!("{YELLOW}  missing or invalid --start line{RESET}");
            return;
        };
        let Some(end_line) = flag_value(args, "--end").and_then(|s| s.parse::<usize>().ok()) else {
            eprintln!("{YELLOW}  missing or invalid --end line{RESET}");
            return;
        };
        crate::deepseek::build_fim_completion_request_for_file(
            crate::deepseek::FimFileRequestOptions {
                file_path: file_path.to_string(),
                start_line,
                end_line,
                scope: scope.to_string(),
                max_tokens,
                temperature: None,
                stream: false,
            },
            &genome.fim_policy,
        )
    } else {
        crate::deepseek::build_fim_completion_request(
            crate::deepseek::FimRequestOptions {
                prompt: prompt.unwrap().to_string(),
                suffix: flag_value(args, "--suffix").map(|s| s.to_string()),
                scope: scope.to_string(),
                max_tokens,
                temperature: None,
                stream: false,
            },
            &genome.fim_policy,
        )
    };
    let request = match request {
        Ok(request) => request,
        Err(e) => {
            print_fim_complete_error(json_output, e);
            return;
        }
    };

    let response_value = match flag_value(args, "--response") {
        Some(raw) => match serde_json::from_str::<Value>(raw) {
            Ok(value) => value,
            Err(e) => {
                print_fim_complete_error(json_output, format!("invalid --response JSON: {e}"));
                return;
            }
        },
        None => match invoke_deepseek_fim_completion(&request, &genome.transport_policy) {
            Ok(value) => value,
            Err(e) => {
                print_fim_complete_error(json_output, e);
                return;
            }
        },
    };

    let completion = match crate::deepseek::parse_fim_completion_response(&response_value) {
        Ok(completion) => completion,
        Err(e) => {
            print_fim_complete_error(json_output, e);
            return;
        }
    };

    let mut plan = None;
    let mut message = "completion generated".to_string();
    let mut ok = true;
    let mut verification = None;
    if let Some(file_path) = file_path {
        let start_line = flag_value(args, "--start")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        let end_line = flag_value(args, "--end")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        match crate::deepseek::build_fim_edit_plan(
            crate::deepseek::FimEditOptions {
                file_path: file_path.to_string(),
                start_line,
                end_line,
                completion: completion.text.clone(),
                scope: scope.to_string(),
            },
            &genome.fim_policy,
        ) {
            Ok(edit_plan) => {
                let check_only = !apply;
                let (apply_ok, apply_message) =
                    crate::commands_file::apply_patch_from_string(&edit_plan.patch, check_only);
                if apply_ok && apply {
                    verification = verify_command.as_deref().map(run_fim_verify_command);
                }
                ok = apply_ok;
                message = apply_message;
                plan = Some(edit_plan);
            }
            Err(e) => {
                print_fim_complete_error(json_output, e);
                return;
            }
        }
    }

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&fim_complete_output_json(
                ok,
                if apply { "apply" } else { "check" },
                &request,
                &completion,
                plan.as_ref(),
                &message,
                verification.as_ref(),
            ))
            .unwrap_or_else(|_| "{}".to_string())
        );
    } else if let Some(plan) = &plan {
        println!("DeepSeek FIM completion");
        println!("  file:     {}", plan.file_path);
        println!("  range:    {}-{}", plan.start_line, plan.end_line);
        println!("  scope:    {}", plan.scope);
        println!("  mode:     {}", if apply { "apply" } else { "check" });
        println!("  tokens:   output={}", completion.usage.output_tokens);
        println!("\n{}", plan.patch);
        if ok {
            println!("{GREEN}  {message}{RESET}");
        } else {
            eprintln!("{YELLOW}  {message}{RESET}");
        }
    } else {
        println!("{}", completion.text);
    }

    if ok && apply {
        if let Some(plan) = &plan {
            crate::state::record(
                crate::state::EventType::FileEdited,
                crate::state::Actor::Harness,
                build_fim_apply_state_payload(plan, verification.as_ref()),
            );
        }
    }
}

fn invoke_deepseek_fim_completion(
    request: &crate::deepseek::DeepSeekFimRequest,
    policy: &crate::deepseek::DeepSeekTransportPolicy,
) -> Result<Value, String> {
    let api_key = std::env::var("DEEPSEEK_API_KEY")
        .map_err(|_| "DEEPSEEK_API_KEY is required for live FIM completion".to_string())?;
    invoke_deepseek_fim_completion_with_key(request, policy, &api_key)
}

fn invoke_deepseek_fim_completion_with_key(
    request: &crate::deepseek::DeepSeekFimRequest,
    policy: &crate::deepseek::DeepSeekTransportPolicy,
    api_key: &str,
) -> Result<Value, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(policy.request_timeout_ms))
        .build()
        .map_err(|e| format!("build DeepSeek FIM HTTP client: {e}"))?;

    for attempt in 0..=policy.max_retries {
        let response = client
            .post(&request.endpoint)
            .bearer_auth(api_key)
            .json(&request.payload)
            .send();
        let response = match response {
            Ok(response) => response,
            Err(e) => {
                let error = e.to_string();
                let decision = crate::deepseek::classify_deepseek_transport_failure(
                    None, &error, attempt, policy,
                );
                if decision.retryable {
                    sleep_before_deepseek_retry(decision.next_backoff_ms);
                    continue;
                }
                return Err(format!(
                    "DeepSeek FIM request failed status=- class={:?} retryable={} attempt={}/{} error={}",
                    decision.class,
                    decision.retryable,
                    decision.attempt,
                    decision.max_retries,
                    preview(&error, 500)
                ));
            }
        };

        let status = response.status();
        let raw = response
            .text()
            .map_err(|e| format!("read DeepSeek FIM response: {e}"))?;
        if status.is_success() {
            return serde_json::from_str::<Value>(&raw)
                .map_err(|e| format!("parse DeepSeek FIM response JSON: {e}"));
        }

        let decision = crate::deepseek::classify_deepseek_transport_failure(
            Some(status.as_u16()),
            &raw,
            attempt,
            policy,
        );
        if decision.retryable {
            sleep_before_deepseek_retry(decision.next_backoff_ms);
            continue;
        }
        return Err(format!(
            "DeepSeek FIM request failed status={} class={:?} retryable={} attempt={}/{} body={}",
            status.as_u16(),
            decision.class,
            decision.retryable,
            decision.attempt,
            decision.max_retries,
            preview(&raw, 500)
        ));
    }

    Err("DeepSeek FIM request failed after retry budget was exhausted".to_string())
}

fn sleep_before_deepseek_retry(backoff_ms: Option<u64>) {
    if let Some(backoff_ms) = backoff_ms.filter(|value| *value > 0) {
        std::thread::sleep(Duration::from_millis(backoff_ms));
    }
}

fn fim_complete_output_json(
    ok: bool,
    mode: &str,
    request: &crate::deepseek::DeepSeekFimRequest,
    completion: &crate::deepseek::DeepSeekFimCompletion,
    plan: Option<&crate::deepseek::FimEditPlan>,
    message: &str,
    verification: Option<&FimVerifyResult>,
) -> Value {
    json!({
        "ok": ok,
        "mode": mode,
        "request": {
            "endpoint": request.endpoint,
            "model": request.model,
            "scope": request.scope,
            "max_tokens": request.payload.get("max_tokens").cloned().unwrap_or(Value::Null),
        },
        "completion": completion,
        "plan": plan,
        "message": message,
        "verification": verification.map(fim_verify_result_json),
    })
}

fn print_fim_complete_error(json_output: bool, error: String) {
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": false,
                "error": error,
            }))
            .unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        eprintln!("{YELLOW}  DeepSeek FIM completion failed: {error}{RESET}");
    }
}

fn handle_fim_apply(args: &[String]) {
    let Some(file_path) = flag_value(args, "--file") else {
        eprintln!(
            "{YELLOW}  Usage: yoyo deepseek fim-apply --file PATH --start N --end N --completion TEXT [--scope SCOPE] [--apply] [--verify CMD] [--json]{RESET}"
        );
        return;
    };
    let Some(start_line) = flag_value(args, "--start").and_then(|s| s.parse::<usize>().ok()) else {
        eprintln!("{YELLOW}  missing or invalid --start line{RESET}");
        return;
    };
    let Some(end_line) = flag_value(args, "--end").and_then(|s| s.parse::<usize>().ok()) else {
        eprintln!("{YELLOW}  missing or invalid --end line{RESET}");
        return;
    };
    let Some(completion) = flag_value(args, "--completion") else {
        eprintln!("{YELLOW}  missing --completion text{RESET}");
        return;
    };
    let scope = flag_value(args, "--scope")
        .map(|s| s.as_str())
        .unwrap_or("localized_completion");
    let apply = args.iter().any(|arg| arg == "--apply");
    let verify_command = flag_value(args, "--verify").map(|command| command.to_string());
    if verify_command.is_some() && !apply {
        eprintln!(
            "{YELLOW}  --verify requires --apply so the command checks the edited file{RESET}"
        );
        return;
    }
    let json_output = args.iter().any(|arg| arg == "--json");
    let genome = crate::deepseek::active_harness_genome();
    let plan = match crate::deepseek::build_fim_edit_plan(
        crate::deepseek::FimEditOptions {
            file_path: file_path.to_string(),
            start_line,
            end_line,
            completion: completion.to_string(),
            scope: scope.to_string(),
        },
        &genome.fim_policy,
    ) {
        Ok(plan) => plan,
        Err(e) => {
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "ok": false,
                        "error": e,
                    }))
                    .unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                eprintln!("{YELLOW}  DeepSeek FIM edit rejected: {e}{RESET}");
            }
            return;
        }
    };

    let check_only = !apply;
    let (ok, message) = crate::commands_file::apply_patch_from_string(&plan.patch, check_only);
    let verification = if ok && apply {
        verify_command.as_deref().map(run_fim_verify_command)
    } else {
        None
    };
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": ok,
                "mode": if apply { "apply" } else { "check" },
                "plan": plan,
                "message": message,
                "verification": verification.as_ref().map(fim_verify_result_json),
            }))
            .unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!("DeepSeek FIM edit plan");
        println!("  file:     {}", plan.file_path);
        println!("  range:    {}-{}", plan.start_line, plan.end_line);
        println!("  scope:    {}", plan.scope);
        println!("  risk:     {}", plan.risk_label);
        println!("  mode:     {}", if apply { "apply" } else { "check" });
        println!("\n{}", plan.patch);
        if ok {
            println!("{GREEN}  {message}{RESET}");
        } else {
            eprintln!("{YELLOW}  {message}{RESET}");
        }
        if let Some(result) = &verification {
            println!(
                "  verify:  {} status={} duration={}ms",
                if result.passed { "passed" } else { "failed" },
                result
                    .status_code
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                result.duration_ms
            );
            if !result.stderr_preview.is_empty() {
                eprintln!("{YELLOW}  verify stderr: {}{RESET}", result.stderr_preview);
            }
        }
    }

    if ok && apply {
        crate::state::record(
            crate::state::EventType::FileEdited,
            crate::state::Actor::Harness,
            build_fim_apply_state_payload(&plan, verification.as_ref()),
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FimVerifyResult {
    command: String,
    passed: bool,
    status_code: Option<i32>,
    duration_ms: u64,
    stdout_preview: String,
    stderr_preview: String,
}

fn run_fim_verify_command(command: &str) -> FimVerifyResult {
    let started = Instant::now();
    let output = Command::new("/bin/sh").arg("-lc").arg(command).output();
    match output {
        Ok(output) => FimVerifyResult {
            command: command.to_string(),
            passed: output.status.success(),
            status_code: output.status.code(),
            duration_ms: started.elapsed().as_millis() as u64,
            stdout_preview: preview(&String::from_utf8_lossy(&output.stdout), 2000),
            stderr_preview: preview(&String::from_utf8_lossy(&output.stderr), 2000),
        },
        Err(e) => FimVerifyResult {
            command: command.to_string(),
            passed: false,
            status_code: None,
            duration_ms: started.elapsed().as_millis() as u64,
            stdout_preview: String::new(),
            stderr_preview: e.to_string(),
        },
    }
}

fn build_fim_apply_state_payload(
    plan: &crate::deepseek::FimEditPlan,
    verification: Option<&FimVerifyResult>,
) -> Value {
    let mut payload = json!({
        "source": "deepseek_fim_apply",
        "file_path": plan.file_path,
        "start_line": plan.start_line,
        "end_line": plan.end_line,
        "scope": plan.scope,
        "inserted_lines": plan.inserted_lines,
        "removed_lines": plan.removed_lines,
        "risk_label": plan.risk_label,
        "patch_preview": plan.patch.chars().take(2000).collect::<String>(),
    });
    if let Some(result) = verification {
        if let Value::Object(object) = &mut payload {
            object.insert("verify_command".to_string(), json!(result.command));
            object.insert("compile_passed".to_string(), json!(result.passed));
            object.insert("verify_status_code".to_string(), json!(result.status_code));
            object.insert("verify_duration_ms".to_string(), json!(result.duration_ms));
            object.insert(
                "verify_stdout_preview".to_string(),
                json!(result.stdout_preview),
            );
            object.insert(
                "verify_stderr_preview".to_string(),
                json!(result.stderr_preview),
            );
        }
    }
    payload
}

fn fim_verify_result_json(result: &FimVerifyResult) -> Value {
    json!({
        "command": result.command,
        "passed": result.passed,
        "status_code": result.status_code,
        "duration_ms": result.duration_ms,
        "stdout_preview": result.stdout_preview,
        "stderr_preview": result.stderr_preview,
    })
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

#[derive(Debug, Clone, PartialEq)]
struct CacheReport {
    event_count: u64,
    hit_tokens: u64,
    miss_tokens: u64,
    hit_ratio: f64,
    models: Vec<CacheModelReport>,
}

#[derive(Debug, Clone, PartialEq)]
struct CacheModelReport {
    model: String,
    event_count: u64,
    hit_tokens: u64,
    miss_tokens: u64,
    hit_ratio: f64,
}

fn handle_cache_report(args: &[String]) {
    let path = default_events_path();
    let json_output = args.iter().any(|arg| arg == "--json");

    // Use capped tail read to avoid timeout on large event files.
    // Matching the state doctor's and state crashes' approach: scan only
    // the most recent CACHE_REPORT_EVENT_CAP events when the log is larger.
    let (events, sampling_note) = match read_tail_cache_events(&path, CACHE_REPORT_EVENT_CAP) {
        Ok((events, total, scanned)) => {
            let note = if total > CACHE_REPORT_EVENT_CAP {
                Some(format!(
                    "Sampled last {scanned} of {total} events (capped for performance). "
                ))
            } else {
                None
            };
            (events, note)
        }
        Err(_) => match read_events_from_sqlite(&path) {
            Ok(events) => {
                eprintln!("{YELLOW}  events.jsonl unavailable - using SQLite projection{RESET}");
                (events, None)
            }
            Err(e) => {
                eprintln!(
                    "{YELLOW}  no state log found at {} ({e}){RESET}",
                    path.display()
                );
                return;
            }
        },
    };
    match build_cache_report(&events) {
        Ok(report) if json_output => println!(
            "{}",
            serde_json::to_string_pretty(&cache_report_payload(&report, sampling_note.as_deref()))
                .unwrap_or_else(|_| "{}".to_string())
        ),
        Ok(report) => println!("{}", render_cache_report(&report, sampling_note.as_deref())),
        Err(e) if json_output => {
            let diag_note = e.lines().next().unwrap_or(&e).to_string();
            let avail_paths: Vec<&str> = e
                .lines()
                .skip_while(|l| !l.contains("stream-check"))
                .filter(|l| l.trim().starts_with("- "))
                .map(|l| l.trim().trim_start_matches("- "))
                .take_while(|l| !l.contains("Use one of"))
                .collect();
            let payload = json!({
                "diagnostic": "deepseek_cache_report",
                "limitation": "no_cache_metrics_for_agent_chat",
                "diagnostic_note": diag_note,
                "reason": "yoagent Usage struct drops DeepSeek cache token fields (cache_read_input_tokens, cache_creation_input_tokens)",
                "available_diagnostic_paths": avail_paths,
                "event_count": 0,
                "hit_tokens": 0,
                "miss_tokens": 0,
                "hit_ratio": 0.0,
                "hit_ratio_percent": 0.0,
                "sampling_note": sampling_note,
                "models": [],
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
            );
        }
        Err(e) => println!("{e}"),
    }
}

#[cfg(test)]
fn read_events(path: &Path) -> Result<Vec<Value>, std::io::Error> {
    crate::state::read_compatibility_events(path).map_err(std::io::Error::other)
}

/// Read only the last `cap` non-empty lines from the events JSONL file,
/// parse each as JSON, and return the parsed events along with the total
/// line count and the number actually scanned.
///
/// Matching the `read_tail_events_capped` pattern from `state crashes`.
fn read_tail_cache_events(path: &Path, cap: usize) -> Result<(Vec<Value>, usize, usize), String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("read cache events '{}': {e}", path.display()))?;
    let total: usize = raw.lines().count();

    // Keep only the last `cap` lines in a sliding window.
    let lines: Vec<&str> = {
        let mut window: Vec<&str> = Vec::with_capacity(cap);
        for line in raw.lines() {
            if window.len() >= cap {
                window.remove(0);
            }
            window.push(line);
        }
        window
    };
    let scanned = lines.len();

    let events: Vec<Value> = lines
        .into_iter()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect();

    Ok((events, total, scanned))
}

/// Fall back to the SQLite projection when the raw events.jsonl is unavailable.
///
/// Reads `CacheMetricsRecorded` events from the `state_events` table and
/// returns them in the same `Vec<Value>` shape that `read_events` produces,
/// so `build_cache_report` can consume them unchanged.
fn read_events_from_sqlite(events_path: &Path) -> Result<Vec<Value>, String> {
    let sqlite_path = crate::state::sqlite_projection_path(events_path);
    if !sqlite_path.exists() {
        return Err(format!(
            "SQLite projection not found at {}",
            sqlite_path.display()
        ));
    }
    let conn = rusqlite::Connection::open(&sqlite_path)
        .map_err(|e| format!("open sqlite projection '{}': {e}", sqlite_path.display()))?;
    let mut stmt = conn
        .prepare("SELECT payload_json FROM state_events WHERE event_type = 'CacheMetricsRecorded' ORDER BY timestamp_ms")
        .map_err(|e| format!("prepare cache events query: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query cache events: {e}"))?;
    let mut events = Vec::new();
    for row in rows {
        let payload_json = row.map_err(|e| format!("read cache event row: {e}"))?;
        let payload: Value = serde_json::from_str(&payload_json)
            .map_err(|e| format!("parse cache event payload: {e}"))?;
        events.push(json!({
            "event_type": "CacheMetricsRecorded",
            "payload": payload,
        }));
    }
    if events.is_empty() {
        return Err("no CacheMetricsRecorded events in SQLite projection".to_string());
    }
    Ok(events)
}

fn build_cache_report(events: &[Value]) -> Result<CacheReport, String> {
    let mut event_count = 0u64;
    let mut hit_tokens = 0u64;
    let mut miss_tokens = 0u64;
    let mut by_model: BTreeMap<String, (u64, u64, u64)> = BTreeMap::new();

    for event in events {
        let event_type = event
            .get("event_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if event_type != "CacheMetricsRecorded" {
            continue;
        }
        event_count += 1;
        let payload = event.get("payload").unwrap_or(&Value::Null);
        let event_hit_tokens = payload
            .get("prompt_cache_hit_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let event_miss_tokens = payload
            .get("prompt_cache_miss_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        hit_tokens += event_hit_tokens;
        miss_tokens += event_miss_tokens;
        let model = payload
            .get("model")
            .and_then(Value::as_str)
            .filter(|model| !model.trim().is_empty())
            .unwrap_or("unknown")
            .to_string();
        let entry = by_model.entry(model).or_insert((0, 0, 0));
        entry.0 += 1;
        entry.1 += event_hit_tokens;
        entry.2 += event_miss_tokens;
    }

    if event_count == 0 {
        return Err(
            "no DeepSeek cache metrics recorded from agent chat completions\n  \
             Reason: yoagent's Usage struct drops DeepSeek cache token fields\n  \
             (cache_read_input_tokens, cache_creation_input_tokens).\n  \
             Cache metrics ARE recorded for these diagnostic paths:\n  \
               - yyds deepseek stream-check  (chat completion SSE parsing)\n  \
               - yyds deepseek fim-complete   (FIM completion parsing)\n  \
             Next step: Run `yyds deepseek stream-check` to populate cache metrics,\n  \
             then re-run `yyds deepseek cache-report`."
                .to_string(),
        );
    }

    let total = hit_tokens + miss_tokens;
    let ratio = if total == 0 {
        0.0
    } else {
        hit_tokens as f64 / total as f64
    };

    let models = by_model
        .into_iter()
        .map(
            |(model, (model_event_count, model_hit_tokens, model_miss_tokens))| {
                let model_total = model_hit_tokens + model_miss_tokens;
                CacheModelReport {
                    model,
                    event_count: model_event_count,
                    hit_tokens: model_hit_tokens,
                    miss_tokens: model_miss_tokens,
                    hit_ratio: if model_total == 0 {
                        0.0
                    } else {
                        model_hit_tokens as f64 / model_total as f64
                    },
                }
            },
        )
        .collect();

    Ok(CacheReport {
        event_count,
        hit_tokens,
        miss_tokens,
        hit_ratio: ratio,
        models,
    })
}

fn render_cache_report(report: &CacheReport, sampling_note: Option<&str>) -> String {
    let sampling_prefix = match sampling_note {
        Some(note) if !note.is_empty() => format!("\n  note:        {note}"),
        _ => String::new(),
    };
    let mut out = format!(
        "DeepSeek server-side cache report\n  events:      {}{}\n  hit tokens:  {}\n  miss tokens: {}\n  hit ratio:   {:.2}%",
        report.event_count,
        sampling_prefix,
        report.hit_tokens,
        report.miss_tokens,
        report.hit_ratio * 100.0
    );
    if !report.models.is_empty() {
        out.push_str("\n  models:");
        for model in &report.models {
            out.push_str(&format!(
                "\n    {} events={} hit={} miss={} ratio={:.2}%",
                model.model,
                model.event_count,
                model.hit_tokens,
                model.miss_tokens,
                model.hit_ratio * 100.0
            ));
        }
    }
    out
}

fn cache_report_payload(report: &CacheReport, sampling_note: Option<&str>) -> Value {
    json!({
        "diagnostic": "deepseek_cache_report",
        "event_count": report.event_count,
        "hit_tokens": report.hit_tokens,
        "miss_tokens": report.miss_tokens,
        "hit_ratio": report.hit_ratio,
        "hit_ratio_percent": report.hit_ratio * 100.0,
        "sampling_note": sampling_note,
        "models": report.models.iter().map(|model| {
            json!({
                "model": model.model,
                "event_count": model.event_count,
                "hit_tokens": model.hit_tokens,
                "miss_tokens": model.miss_tokens,
                "hit_ratio": model.hit_ratio,
                "hit_ratio_percent": model.hit_ratio * 100.0,
            })
        }).collect::<Vec<_>>(),
    })
}

fn bool_label(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn thinking_label(mode: crate::deepseek::ThinkingMode) -> &'static str {
    match mode {
        crate::deepseek::ThinkingMode::Disabled => "disabled",
        crate::deepseek::ThinkingMode::Enabled {
            effort: crate::deepseek::ThinkingEffort::High,
        } => "enabled/high",
        crate::deepseek::ThinkingMode::Enabled {
            effort: crate::deepseek::ThinkingEffort::Max,
        } => "enabled/max",
    }
}

fn flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a String> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|idx| args.get(idx + 1))
}

fn shell_join(args: &[String]) -> String {
    args.iter()
        .map(|arg| {
            if arg
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '/' | '.' | '='))
            {
                arg.clone()
            } else {
                format!("'{}'", arg.replace('\'', "'\\''"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn print_usage() {
    println!(
        "Usage: yoyo deepseek <command>\n\n  doctor [--json]\n  genome [--json]\n  route <task> [--json]\n  models\n  schemas [--json]\n  schema-check --name SCHEMA --arguments JSON [--attempt N] [--record] [--json]\n  test-tool-call [--record] [--json]\n  test-thinking [--messages JSON] [--record] [--json]\n  stream-check [--sse TEXT] [--record] [--json]\n  transport-check [--status N] [--error TEXT] [--attempt N] [--record] [--json]\n  json-check --input TEXT [--retry-input TEXT] [--schema NAME] [--record] [--json]\n  prefix-check --prompt TEXT --prefix TEXT [--model MODEL] [--max-tokens N] [--json]\n  fim-check (--prompt TEXT [--suffix TEXT] | --file PATH --start N --end N) [--scope SCOPE] [--max-tokens N] [--json]\n  fim-route --input TEXT [--max-tokens N] [--enable] [--record] [--execute] [--response JSON] [--apply] [--verify CMD] [--json]\n  fim-complete (--prompt TEXT [--suffix TEXT] | --file PATH --start N --end N) [--scope SCOPE] [--max-tokens N] [--response JSON] [--apply] [--verify CMD] [--json]\n  fim-parse --response JSON\n  fim-apply --file PATH --start N --end N --completion TEXT [--scope SCOPE] [--apply] [--verify CMD] [--json]\n  cache-report [--json]"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn cache_report_aggregates_state_events() {
        let events = vec![
            json!({
                "event_type": "CacheMetricsRecorded",
                "payload": {
                    "model": crate::deepseek::DEFAULT_MODEL,
                    "prompt_cache_hit_tokens": 75,
                    "prompt_cache_miss_tokens": 25
                }
            }),
            json!({
                "event_type": "CacheMetricsRecorded",
                "payload": {
                    "model": crate::deepseek::FAST_MODEL,
                    "prompt_cache_hit_tokens": 25,
                    "prompt_cache_miss_tokens": 75
                }
            }),
        ];
        let report = build_cache_report(&events).unwrap();
        let rendered = render_cache_report(&report, None);
        assert!(rendered.contains("events:      2"));
        assert!(rendered.contains("hit tokens:  100"));
        assert!(rendered.contains("miss tokens: 100"));
        assert!(rendered.contains("50.00%"));
        assert_eq!(report.event_count, 2);
        assert_eq!(report.hit_tokens, 100);
        assert_eq!(report.miss_tokens, 100);
        assert_eq!(report.models.len(), 2);
        assert!(rendered.contains("models:"));
        assert!(rendered.contains("deepseek-v4-flash events=1 hit=25 miss=75 ratio=25.00%"));
        assert!(rendered.contains("deepseek-v4-pro events=1 hit=75 miss=25 ratio=75.00%"));
    }

    #[test]
    fn cache_report_handles_missing_metrics() {
        let err = build_cache_report(&[json!({"event_type": "RunStarted"})]).unwrap_err();
        assert!(
            err.contains("no DeepSeek cache metrics recorded"),
            "error should explain the limitation, got: {err}"
        );
        assert!(
            err.contains("yoagent"),
            "error should name yoagent as the upstream limitation, got: {err}"
        );
        assert!(
            err.contains("stream-check"),
            "error should point to stream-check as a diagnostic path, got: {err}"
        );
        assert!(
            err.contains("Next step: Run"),
            "error should include a concrete next step, got: {err}"
        );
    }

    #[test]
    fn cache_report_reads_canonical_yoagent_state_events() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let event = crate::state::StateEvent {
            event_id: "evt-cache".into(),
            event_type: crate::state::EventType::CacheMetricsRecorded,
            schema_version: 1,
            timestamp_ms: 1,
            actor: crate::state::Actor::Harness,
            run_id: Some("run-1".into()),
            session_id: None,
            trace_id: "trace-1".into(),
            parent_event_ids: Vec::new(),
            payload: json!({
                "model": crate::deepseek::DEFAULT_MODEL,
                "prompt_cache_hit_tokens": 90,
                "prompt_cache_miss_tokens": 10
            }),
        };
        crate::state::append_event(&path, &event).unwrap();

        let events = read_events(&path).unwrap();
        let report = build_cache_report(&events).unwrap();
        let rendered = render_cache_report(&report, None);

        assert!(rendered.contains("events:      1"));
        assert!(rendered.contains("hit tokens:  90"));
        assert!(rendered.contains("miss tokens: 10"));
        assert!(rendered.contains("90.00%"));
        assert!(rendered.contains("deepseek-v4-pro events=1 hit=90 miss=10 ratio=90.00%"));
    }

    #[test]
    fn cache_report_payload_is_machine_readable() {
        let report = CacheReport {
            event_count: 2,
            hit_tokens: 100,
            miss_tokens: 100,
            hit_ratio: 0.5,
            models: vec![
                CacheModelReport {
                    model: crate::deepseek::FAST_MODEL.to_string(),
                    event_count: 1,
                    hit_tokens: 25,
                    miss_tokens: 75,
                    hit_ratio: 0.25,
                },
                CacheModelReport {
                    model: crate::deepseek::DEFAULT_MODEL.to_string(),
                    event_count: 1,
                    hit_tokens: 75,
                    miss_tokens: 25,
                    hit_ratio: 0.75,
                },
            ],
        };

        let payload = cache_report_payload(&report, None);

        assert_eq!(payload["diagnostic"], "deepseek_cache_report");
        assert_eq!(payload["event_count"], 2);
        assert_eq!(payload["hit_tokens"], 100);
        assert_eq!(payload["miss_tokens"], 100);
        assert_eq!(payload["hit_ratio"], 0.5);
        assert_eq!(payload["hit_ratio_percent"], 50.0);
        assert_eq!(payload["models"].as_array().unwrap().len(), 2);
        assert_eq!(payload["models"][0]["model"], crate::deepseek::FAST_MODEL);
        assert_eq!(payload["models"][0]["hit_ratio"], 0.25);
    }

    #[test]
    fn route_command_decision_is_printable_as_json() {
        let genome = crate::deepseek::DeepSeekHarnessGenome::default();
        let decision = crate::deepseek::route_for_task("planning", &genome).unwrap();
        let raw = serde_json::to_string_pretty(&decision).unwrap();
        assert!(raw.contains(crate::deepseek::DEFAULT_MODEL));
        assert!(raw.contains("patch planning"));
        assert_eq!(thinking_label(decision.thinking), "enabled/high");
    }

    #[test]
    fn doctor_summary_uses_active_genome_and_provider_config() {
        let mut genome = crate::deepseek::DeepSeekHarnessGenome::default();
        genome.model_routing_policy.final_patch_model = "deepseek-v4-pro-custom".into();
        genome.model_routing_policy.quick_summary_model = "deepseek-v4-flash-custom".into();
        genome.context_policy.recent_failure_limit = 7;
        genome.context_policy.changed_file_limit = 18;
        genome.context_policy.include_repo_map = false;
        genome.context_policy.include_instruction_files = vec!["YOYO.md".into(), "TEAM.md".into()];
        genome.transport_policy.request_timeout_ms = 45_000;
        genome.transport_policy.max_retries = 4;
        let config = crate::agent_builder::create_model_config(
            "deepseek",
            &genome.model_routing_policy.final_patch_model,
            Some("https://deepseek.example/v1"),
        );

        let summary = render_doctor_summary(&config, &genome);

        assert!(summary.contains("DeepSeek native doctor"));
        assert!(summary.contains("default model: deepseek-v4-pro-custom"));
        assert!(summary.contains("fast model:    deepseek-v4-flash-custom"));
        assert!(summary.contains("base url:      https://deepseek.example/v1"));
        assert!(summary.contains(
            "context policy: failures=7 changed_files=18 repo_map=no instructions=YOYO.md, TEAM.md"
        ));
        assert!(summary.contains("thinking ctrl: yes"));
        assert!(summary.contains("retry policy:  max_retries=4 request_timeout=45000ms"));
    }

    #[test]
    fn doctor_json_payload_is_machine_readable_active_policy() {
        let mut genome = crate::deepseek::DeepSeekHarnessGenome::default();
        genome.model_routing_policy.final_patch_model = "deepseek-v4-pro-custom".into();
        genome.model_routing_policy.quick_summary_model = "deepseek-v4-flash-custom".into();
        genome.context_policy.recent_failure_limit = 7;
        genome.context_policy.changed_file_limit = 18;
        genome.context_policy.include_repo_map = false;
        genome.context_policy.include_instruction_files = vec!["YOYO.md".into(), "TEAM.md".into()];
        genome.transport_policy.request_timeout_ms = 45_000;
        genome.transport_policy.max_retries = 4;
        let config = crate::agent_builder::create_model_config(
            "deepseek",
            &genome.model_routing_policy.final_patch_model,
            Some("https://deepseek.example/v1"),
        );

        let payload = doctor_summary_payload(&config, &genome);

        assert_eq!(payload["diagnostic"], "deepseek_doctor");
        assert_eq!(payload["provider"], "deepseek");
        assert_eq!(payload["primary_model"], "deepseek-v4-pro-custom");
        assert_eq!(payload["fast_model"], "deepseek-v4-flash-custom");
        assert_eq!(payload["base_url"], "https://deepseek.example/v1");
        assert_eq!(payload["context_policy"]["recent_failure_limit"], 7);
        assert_eq!(payload["context_policy"]["changed_file_limit"], 18);
        assert_eq!(payload["context_policy"]["include_repo_map"], false);
        assert_eq!(
            payload["context_policy"]["include_instruction_files"],
            json!(["YOYO.md", "TEAM.md"])
        );
        assert_eq!(payload["supports_reasoning_effort"], true);
        assert_eq!(payload["supports_thinking_control"], true);
        assert_eq!(payload["supports_usage_in_streaming"], true);
        assert_eq!(payload["retry_policy"]["max_retries"], 4);
        assert_eq!(payload["retry_policy"]["request_timeout_ms"], 45_000);
        assert_eq!(
            payload["harness_genome_version"],
            crate::deepseek::HARNESS_GENOME_VERSION
        );
    }

    #[test]
    fn genome_is_printable_as_json() {
        let genome = crate::deepseek::DeepSeekHarnessGenome::default();
        let raw = serde_json::to_string_pretty(&genome).unwrap();
        assert!(raw.contains(crate::deepseek::HARNESS_GENOME_VERSION));
        assert!(raw.contains("propose_harness_patch"));
        assert!(raw.contains("transport_policy"));
        assert!(raw.contains("request_timeout_ms"));
    }

    #[test]
    fn genome_summary_surfaces_context_policy() {
        let mut genome = crate::deepseek::DeepSeekHarnessGenome::default();
        genome.context_policy.recent_failure_limit = 9;
        genome.context_policy.changed_file_limit = 24;
        genome.context_policy.include_repo_map = false;
        genome.context_policy.include_instruction_files = vec!["YOYO.md".into(), "TEAM.md".into()];

        let summary = render_genome_summary(&genome);

        assert!(summary.contains("DeepSeek harness genome"));
        assert!(summary.contains("context:"));
        assert!(summary.contains("failures=9"));
        assert!(summary.contains("changed_files=24"));
        assert!(summary.contains("repo_map=no"));
        assert!(summary.contains("instructions=YOYO.md, TEAM.md"));
    }

    #[test]
    fn transport_decision_is_printable_as_json() {
        let genome = crate::deepseek::DeepSeekHarnessGenome::default();
        let decision = crate::deepseek::classify_deepseek_transport_failure(
            Some(429),
            "rate limit",
            0,
            &genome.transport_policy,
        );
        let raw = serde_json::to_string_pretty(&decision).unwrap();
        assert!(raw.contains("rate_limited"));
        assert!(raw.contains("\"retryable\": true"));
        assert!(raw.contains("next_backoff_ms"));
    }

    #[test]
    fn transport_check_pass_payload_is_compact_and_recordable() {
        let genome = crate::deepseek::DeepSeekHarnessGenome::default();
        let decision = crate::deepseek::classify_deepseek_transport_failure(
            Some(429),
            "rate limit with request id abc123 and extra diagnostic body",
            0,
            &genome.transport_policy,
        );

        let payload = transport_check_pass_payload(&decision, "rate limit with request id abc123");

        assert_eq!(payload["decision_type"], "deepseek_transport_policy_check");
        assert_eq!(payload["decision"], "passed");
        assert_eq!(payload["check"], "transport-check");
        assert_eq!(payload["transport_class"], "rate_limited");
        assert_eq!(payload["status"], 429);
        assert_eq!(payload["retryable"], true);
        assert_eq!(payload["attempt"], 0);
        assert_eq!(payload["max_retries"], genome.transport_policy.max_retries);
        assert!(payload["next_backoff_ms"].as_u64().unwrap() > 0);
        assert_eq!(
            payload["error_preview"],
            "rate limit with request id abc123"
        );
        assert!(payload.get("body").is_none());
    }

    #[test]
    fn stream_check_pass_payload_is_compact_and_recordable() {
        let parsed =
            crate::deepseek::parse_chat_completion_sse(default_stream_check_sse()).unwrap();

        let payload = stream_check_pass_payload(&parsed);

        assert_eq!(
            payload["decision_type"],
            "deepseek_streaming_protocol_check"
        );
        assert_eq!(payload["decision"], "passed");
        assert_eq!(payload["check"], "stream-check");
        assert_eq!(payload["content_chars"], 4);
        assert_eq!(payload["reasoning_content_chars"], 16);
        assert_eq!(payload["tool_call_count"], 1);
        assert_eq!(payload["finish_reason"], "stop");
        assert_eq!(payload["input_tokens"], 12);
        assert_eq!(payload["output_tokens"], 3);
        assert!(payload.get("content").is_none());
        assert!(payload.get("reasoning_content").is_none());
    }

    #[test]
    fn json_check_failure_recording_policy_is_explicit_for_json_mode() {
        assert!(should_record_json_check_failure(false, false));
        assert!(should_record_json_check_failure(false, true));
        assert!(!should_record_json_check_failure(true, false));
        assert!(should_record_json_check_failure(true, true));
    }

    #[test]
    fn json_check_pass_payload_is_compact_and_recordable() {
        let result = crate::deepseek::parse_json_output_attempts(
            ["", r#"{"ok":true}"#],
            crate::deepseek::JsonOutputParseOptions::extraction(
                "deepseek-json-check",
                Some("summary".to_string()),
            ),
        )
        .unwrap();

        let payload = json_check_pass_payload(Some("summary"), &result);

        assert_eq!(payload["decision_type"], "deepseek_json_output_check");
        assert_eq!(payload["decision"], "passed");
        assert_eq!(payload["schema_name"], "summary");
        assert_eq!(payload["attempt_count"], 2);
        assert_eq!(payload["retry_used"], true);
        assert_eq!(payload["attempt_statuses"], json!(["empty", "parsed"]));
        assert!(payload.get("value").is_none());
        assert!(!payload.to_string().contains(r#""ok""#));
    }

    #[test]
    fn schema_suite_is_printable_as_json() {
        let raw = serde_json::to_string_pretty(&crate::deepseek::strict_schema_suite()).unwrap();
        assert!(raw.contains("plan_task"));
        assert!(raw.contains("record_eval_result"));
        assert!(raw.contains("\"strict\": true"));
    }

    #[test]
    fn schema_summary_lines_show_schema_versions() {
        let schema = crate::deepseek::propose_harness_patch_schema();
        let line = format_schema_summary_line(&schema);

        assert!(line.contains("propose_harness_patch"));
        assert!(line.contains("schema_version=1"));
        assert!(line.contains("required_fields="));
    }

    #[test]
    fn deepseek_protocol_checks_pass_for_builtin_probes() {
        let tool_check = crate::deepseek::validate_strict_tool_schema_suite().unwrap();
        assert!(tool_check.passed);
        assert!(tool_check
            .details
            .contains(&"propose_harness_patch".to_string()));

        let messages = crate::deepseek::deepseek_thinking_tool_call_probe_messages();
        let thinking_check =
            crate::deepseek::validate_deepseek_thinking_tool_call_messages(&messages).unwrap();
        assert!(thinking_check.passed);
        assert!(thinking_check.details[0].contains("reasoning_content"));
    }

    #[test]
    fn strict_tool_call_pass_payload_is_compact_and_recordable() {
        let check = crate::deepseek::validate_strict_tool_schema_suite().unwrap();
        let request = crate::deepseek::build_strict_tool_call_request(
            crate::deepseek::StrictToolCallRequestOptions {
                user_prompt: "Inspect the target file before editing.".to_string(),
                model: crate::deepseek::DEFAULT_MODEL.to_string(),
                tool_names: vec!["inspect_file".to_string(), "record_failure".to_string()],
                thinking: crate::deepseek::ThinkingMode::Enabled {
                    effort: crate::deepseek::ThinkingEffort::High,
                },
                max_tokens: 512,
                stream: false,
            },
        )
        .unwrap();

        let payload = strict_tool_call_pass_payload(&check, &request);

        assert_eq!(payload["decision_type"], "deepseek_strict_tool_call_check");
        assert_eq!(payload["decision"], "passed");
        assert_eq!(payload["check"], "test-tool-call");
        assert_eq!(payload["schema_count"], check.details.len());
        assert_eq!(
            payload["selected_tool_names"],
            json!(["inspect_file", "record_failure"])
        );
        assert_eq!(payload["selected_tool_count"], 2);
        assert_eq!(payload["thinking"], "enabled");
        assert_eq!(payload["reasoning_effort"], "high");
        assert_eq!(payload["max_tokens"], 512);
        assert!(payload.get("request").is_none());
        assert!(!payload.to_string().contains("Inspect the target file"));
    }

    #[test]
    fn thinking_protocol_check_accepts_provided_message_replay() {
        let args = vec![
            "yoyo".to_string(),
            "deepseek".to_string(),
            "test-thinking".to_string(),
            "--messages".to_string(),
            serde_json::json!({
                "messages": crate::deepseek::deepseek_thinking_tool_call_probe_messages(),
            })
            .to_string(),
        ];

        let (messages, source) = thinking_protocol_messages_from_args(&args).unwrap();
        let summary = thinking_protocol_probe_summary(&messages, source);
        let check =
            crate::deepseek::validate_deepseek_thinking_tool_call_messages(&messages).unwrap();

        assert!(check.passed);
        assert_eq!(source, "provided-messages");
        assert_eq!(summary["message_count"], 2);
        assert_eq!(summary["assistant_tool_call_turns"], 1);
        assert_eq!(
            summary["assistant_tool_call_turns_with_reasoning_content"],
            1
        );
        assert_eq!(
            summary["assistant_tool_call_turns_missing_reasoning_content"],
            0
        );
    }

    #[test]
    fn thinking_protocol_pass_payload_is_compact_and_json_recordable() {
        let messages = crate::deepseek::deepseek_thinking_tool_call_probe_messages();
        let summary = thinking_protocol_probe_summary(&messages, "builtin-probe");

        let payload = thinking_protocol_pass_payload("builtin-probe", summary);

        assert_eq!(payload["decision_type"], "deepseek_thinking_protocol_check");
        assert_eq!(payload["decision"], "passed");
        assert_eq!(payload["probe"]["assistant_tool_call_turns"], 1);
        assert_eq!(
            payload["probe"]["assistant_tool_call_turns_with_reasoning_content"],
            1
        );
        assert!(payload.get("messages").is_none());
        assert!(!payload.to_string().contains("Need to inspect"));
    }

    #[test]
    fn thinking_protocol_check_rejects_missing_reasoning_replay_without_raw_state_payload() {
        let messages = vec![
            json!({
                "role": "assistant",
                "tool_calls": [{
                    "id": "call-1",
                    "type": "function",
                    "function": {
                        "name": "inspect_file",
                        "arguments": "{\"path\":\"src/main.rs\"}"
                    }
                }]
            }),
            json!({
                "role": "tool",
                "tool_call_id": "call-1",
                "name": "inspect_file",
                "content": "src/main.rs:1: mod agent_builder;"
            }),
        ];

        let error =
            crate::deepseek::validate_deepseek_thinking_tool_call_messages(&messages).unwrap_err();
        let summary = thinking_protocol_probe_summary(&messages, "provided-messages");

        assert!(error.contains("missing DeepSeek reasoning_content"));
        assert_eq!(
            summary["assistant_tool_call_turns_with_reasoning_content"],
            0
        );
        assert_eq!(
            summary["assistant_tool_call_turns_missing_reasoning_content"],
            1
        );
        assert!(summary.get("messages").is_none());
        assert!(summary.to_string().contains("provided-messages"));
        assert!(!summary.to_string().contains("src/main.rs"));
    }

    #[test]
    fn thinking_protocol_check_rejects_later_tool_call_turn_without_reasoning() {
        let messages = vec![
            json!({
                "role": "assistant",
                "reasoning_content": "Inspect the first file.",
                "tool_calls": [{
                    "id": "call-1",
                    "type": "function",
                    "function": {
                        "name": "inspect_file",
                        "arguments": "{\"path\":\"src/main.rs\"}"
                    }
                }]
            }),
            json!({
                "role": "tool",
                "tool_call_id": "call-1",
                "name": "inspect_file",
                "content": "src/main.rs:1: mod agent_builder;"
            }),
            json!({
                "role": "assistant",
                "tool_calls": [{
                    "id": "call-2",
                    "type": "function",
                    "function": {
                        "name": "inspect_file",
                        "arguments": "{\"path\":\"src/deepseek.rs\"}"
                    }
                }]
            }),
            json!({
                "role": "tool",
                "tool_call_id": "call-2",
                "name": "inspect_file",
                "content": "src/deepseek.rs:1: use serde_json;"
            }),
        ];

        let error =
            crate::deepseek::validate_deepseek_thinking_tool_call_messages(&messages).unwrap_err();
        let summary = thinking_protocol_probe_summary(&messages, "provided-messages");

        assert!(error.contains("missing DeepSeek reasoning_content"));
        assert_eq!(summary["assistant_tool_call_turns"], 2);
        assert_eq!(
            summary["assistant_tool_call_turns_with_reasoning_content"],
            1
        );
        assert_eq!(
            summary["assistant_tool_call_turns_missing_reasoning_content"],
            1
        );
    }

    #[test]
    fn thinking_protocol_check_rejects_tool_result_before_tool_call() {
        let messages = vec![
            json!({
                "role": "tool",
                "tool_call_id": "call-early",
                "name": "inspect_file",
                "content": "src/main.rs:1: mod agent_builder;"
            }),
            json!({
                "role": "assistant",
                "reasoning_content": "Inspect the target file before editing.",
                "tool_calls": [{
                    "id": "call-early",
                    "type": "function",
                    "function": {
                        "name": "inspect_file",
                        "arguments": "{\"path\":\"src/main.rs\"}"
                    }
                }]
            }),
        ];

        let error =
            crate::deepseek::validate_deepseek_thinking_tool_call_messages(&messages).unwrap_err();
        let summary = thinking_protocol_probe_summary(&messages, "provided-messages");

        assert!(error.contains("has no pending assistant tool call"));
        assert_eq!(summary["assistant_tool_call_turns"], 1);
        assert_eq!(summary["tool_result_turns"], 1);
    }

    #[test]
    fn thinking_protocol_check_rejects_tool_result_after_next_assistant_turn() {
        let messages = vec![
            json!({
                "role": "assistant",
                "reasoning_content": "Inspect the first file.",
                "tool_calls": [{
                    "id": "call-1",
                    "type": "function",
                    "function": {
                        "name": "inspect_file",
                        "arguments": "{\"path\":\"src/main.rs\"}"
                    }
                }]
            }),
            json!({
                "role": "assistant",
                "content": "I will continue after the tool result."
            }),
            json!({
                "role": "tool",
                "tool_call_id": "call-1",
                "name": "inspect_file",
                "content": "src/main.rs:1: mod agent_builder;"
            }),
        ];

        let error =
            crate::deepseek::validate_deepseek_thinking_tool_call_messages(&messages).unwrap_err();
        let summary = thinking_protocol_probe_summary(&messages, "provided-messages");

        assert!(error.contains("before the next assistant turn"));
        assert_eq!(summary["assistant_tool_call_turns"], 1);
        assert_eq!(summary["tool_result_turns"], 1);
    }

    #[test]
    fn thinking_protocol_check_rejects_unknown_tool_result() {
        let messages = vec![
            json!({
                "role": "assistant",
                "reasoning_content": "Inspect the target file before editing.",
                "tool_calls": [{
                    "id": "call-known",
                    "type": "function",
                    "function": {
                        "name": "inspect_file",
                        "arguments": "{\"path\":\"src/main.rs\"}"
                    }
                }]
            }),
            json!({
                "role": "tool",
                "tool_call_id": "call-other",
                "name": "inspect_file",
                "content": "src/main.rs:1: mod agent_builder;"
            }),
        ];

        let error =
            crate::deepseek::validate_deepseek_thinking_tool_call_messages(&messages).unwrap_err();
        let summary = thinking_protocol_probe_summary(&messages, "provided-messages");

        assert!(error.contains("has no pending assistant tool call"));
        assert_eq!(summary["assistant_tool_call_turns"], 1);
        assert_eq!(summary["tool_result_turns"], 1);
    }

    #[test]
    fn thinking_protocol_check_rejects_duplicate_tool_call_ids() {
        let messages = vec![json!({
            "role": "assistant",
            "reasoning_content": "Inspect two files before editing.",
            "tool_calls": [
                {
                    "id": "call-dup",
                    "type": "function",
                    "function": {
                        "name": "inspect_file",
                        "arguments": "{\"path\":\"src/main.rs\"}"
                    }
                },
                {
                    "id": "call-dup",
                    "type": "function",
                    "function": {
                        "name": "inspect_file",
                        "arguments": "{\"path\":\"src/deepseek.rs\"}"
                    }
                }
            ]
        })];

        let error =
            crate::deepseek::validate_deepseek_thinking_tool_call_messages(&messages).unwrap_err();
        let summary = thinking_protocol_probe_summary(&messages, "provided-messages");

        assert!(error.contains("duplicate tool call id 'call-dup'"));
        assert_eq!(summary["assistant_tool_call_turns"], 1);
        assert_eq!(
            summary["assistant_tool_call_turns_with_reasoning_content"],
            1
        );
    }

    #[test]
    fn fim_apply_state_payload_records_verification_result_for_eval_metrics() {
        let plan = crate::deepseek::FimEditPlan {
            file_path: "src/lib.rs".into(),
            start_line: 3,
            end_line: 5,
            scope: "localized_completion".into(),
            inserted_lines: 2,
            removed_lines: 3,
            risk_label: "low".into(),
            requires_explicit_apply: true,
            patch: "--- a/src/lib.rs\n+++ b/src/lib.rs\n".into(),
        };
        let verification = FimVerifyResult {
            command: "cargo check".into(),
            passed: true,
            status_code: Some(0),
            duration_ms: 12,
            stdout_preview: "ok".into(),
            stderr_preview: String::new(),
        };

        let payload = build_fim_apply_state_payload(&plan, Some(&verification));

        assert_eq!(payload["source"], "deepseek_fim_apply");
        assert_eq!(payload["compile_passed"], true);
        assert_eq!(payload["verify_command"], "cargo check");
        assert_eq!(payload["verify_status_code"], 0);
        assert_eq!(payload["verify_duration_ms"], 12);
    }

    #[test]
    fn fim_verify_result_json_is_stable_for_cli_output() {
        let result = FimVerifyResult {
            command: "cargo test".into(),
            passed: false,
            status_code: Some(101),
            duration_ms: 7,
            stdout_preview: String::new(),
            stderr_preview: "failed".into(),
        };

        let value = fim_verify_result_json(&result);

        assert_eq!(value["command"], "cargo test");
        assert_eq!(value["passed"], false);
        assert_eq!(value["status_code"], 101);
        assert_eq!(value["stderr_preview"], "failed");
    }

    #[test]
    fn fim_complete_json_output_surfaces_mocked_completion() {
        let request = crate::deepseek::build_fim_completion_request(
            crate::deepseek::FimRequestOptions {
                prompt: "fn main() {".into(),
                suffix: Some("}".into()),
                scope: "localized_completion".into(),
                max_tokens: 64,
                temperature: None,
                stream: false,
            },
            &crate::deepseek::DeepSeekHarnessGenome::default().fim_policy,
        )
        .unwrap();
        let completion = crate::deepseek::parse_fim_completion_response(&json!({
            "choices": [{"text": "println!(\"ok\");", "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 12, "completion_tokens": 3}
        }))
        .unwrap();

        let value =
            fim_complete_output_json(true, "check", &request, &completion, None, "ok", None);

        assert_eq!(value["ok"], true);
        assert_eq!(value["request"]["endpoint"], request.endpoint);
        assert_eq!(value["request"]["max_tokens"], 64);
        assert_eq!(value["completion"]["text"], "println!(\"ok\");");
        assert_eq!(value["completion"]["usage"]["output_tokens"], 3);
    }

    #[test]
    fn fim_route_execution_args_delegate_to_fim_complete_path() {
        let decision = crate::deepseek::FimRouteDecision {
            route: "fim_complete".into(),
            use_fim: true,
            reason: "safe".into(),
            file_path: Some("src/deepseek.rs".into()),
            start_line: Some(1),
            end_line: Some(3),
            scope: "localized_completion".into(),
            command: vec![
                "yoyo".into(),
                "deepseek".into(),
                "fim-complete".into(),
                "--file".into(),
                "src/deepseek.rs".into(),
                "--start".into(),
                "1".into(),
                "--end".into(),
                "3".into(),
                "--scope".into(),
                "localized_completion".into(),
                "--max-tokens".into(),
                "256".into(),
            ],
        };
        let source_args = vec![
            "yoyo".into(),
            "deepseek".into(),
            "fim-route".into(),
            "--response".into(),
            r#"{"choices":[{"text":"ok"}]}"#.into(),
            "--apply".into(),
            "--verify".into(),
            "cargo check".into(),
            "--json".into(),
        ];

        let args = build_fim_route_execution_args(&decision, &source_args).unwrap();

        assert_eq!(&args[0..3], ["yoyo", "deepseek", "fim-complete"]);
        assert!(args.contains(&"--file".to_string()));
        assert!(args.contains(&"src/deepseek.rs".to_string()));
        assert!(args.contains(&"--response".to_string()));
        assert!(args.contains(&r#"{"choices":[{"text":"ok"}]}"#.to_string()));
        assert!(args.contains(&"--apply".to_string()));
        assert!(args.contains(&"--verify".to_string()));
        assert!(args.contains(&"cargo check".to_string()));
        assert!(args.contains(&"--json".to_string()));
    }

    #[test]
    fn fim_route_execution_args_reject_declined_route() {
        let decision = crate::deepseek::FimRouteDecision {
            route: "agent_loop".into(),
            use_fim: false,
            reason: "prompt is ambiguous".into(),
            file_path: None,
            start_line: None,
            end_line: None,
            scope: "none".into(),
            command: Vec::new(),
        };

        let err = build_fim_route_execution_args(&decision, &[]).unwrap_err();

        assert_eq!(err, "prompt is ambiguous");
    }

    #[test]
    fn fim_live_invocation_accepts_mock_completion() {
        let Some((endpoint, server)) = spawn_fim_mock_server(
            200,
            r#"{"choices":[{"text":"println!(\"ok\");","finish_reason":"stop"}],"usage":{"prompt_tokens":5,"completion_tokens":2}}"#,
        ) else {
            eprintln!("skipping local TCP mock server test: bind denied");
            return;
        };
        let request = crate::deepseek::DeepSeekFimRequest {
            endpoint,
            model: crate::deepseek::DEFAULT_MODEL.to_string(),
            scope: "localized_completion".to_string(),
            auto_routing_enabled: true,
            payload: json!({
                "model": crate::deepseek::DEFAULT_MODEL,
                "prompt": "fn main() {",
                "suffix": "}",
                "max_tokens": 64,
                "stream": false
            }),
        };

        let value = invoke_deepseek_fim_completion_with_key(
            &request,
            &crate::deepseek::DeepSeekHarnessGenome::default().transport_policy,
            "test-key",
        )
        .unwrap();
        let captured = server.join().unwrap();
        let completion = crate::deepseek::parse_fim_completion_response(&value).unwrap();

        assert!(captured.starts_with("post /completions http/1.1"));
        assert!(captured.contains("authorization: bearer test-key"));
        assert!(captured.contains("\"prompt\":\"fn main() {\""));
        assert_eq!(completion.text, "println!(\"ok\");");
        assert_eq!(completion.usage.input_tokens, 5);
        assert_eq!(completion.usage.output_tokens, 2);
    }

    #[test]
    fn fim_live_invocation_classifies_mock_error_status() {
        let Some((endpoint, server)) =
            spawn_fim_mock_server(429, r#"{"error":{"message":"rate limit"}}"#)
        else {
            eprintln!("skipping local TCP mock server test: bind denied");
            return;
        };
        let request = crate::deepseek::DeepSeekFimRequest {
            endpoint,
            model: crate::deepseek::DEFAULT_MODEL.to_string(),
            scope: "localized_completion".to_string(),
            auto_routing_enabled: true,
            payload: json!({
                "model": crate::deepseek::DEFAULT_MODEL,
                "prompt": "fn main() {",
                "max_tokens": 64,
                "stream": false
            }),
        };

        let mut policy = crate::deepseek::DeepSeekHarnessGenome::default().transport_policy;
        policy.max_retries = 0;
        let err =
            invoke_deepseek_fim_completion_with_key(&request, &policy, "test-key").unwrap_err();
        let captured = server.join().unwrap();

        assert!(captured.starts_with("post /completions http/1.1"));
        assert!(err.contains("status=429"));
        assert!(err.contains("class=RateLimited"));
        assert!(err.contains("retryable=false"));
        assert!(err.contains("attempt=0/0"));
    }

    #[test]
    fn fim_live_invocation_retries_retryable_status_then_succeeds() {
        let Some((endpoint, server)) = spawn_fim_sequence_mock_server(vec![
            (429, r#"{"error":{"message":"rate limit"}}"#),
            (
                200,
                r#"{"choices":[{"text":"retry ok","finish_reason":"stop"}],"usage":{"prompt_tokens":7,"completion_tokens":2}}"#,
            ),
        ]) else {
            eprintln!("skipping local TCP mock server test: bind denied");
            return;
        };
        let request = crate::deepseek::DeepSeekFimRequest {
            endpoint,
            model: crate::deepseek::DEFAULT_MODEL.to_string(),
            scope: "localized_completion".to_string(),
            auto_routing_enabled: true,
            payload: json!({
                "model": crate::deepseek::DEFAULT_MODEL,
                "prompt": "fn retry() {",
                "max_tokens": 64,
                "stream": false
            }),
        };
        let mut policy = crate::deepseek::DeepSeekHarnessGenome::default().transport_policy;
        policy.max_retries = 1;
        policy.initial_backoff_ms = 0;
        policy.max_backoff_ms = 0;

        let value = invoke_deepseek_fim_completion_with_key(&request, &policy, "test-key")
            .expect("retryable 429 should be retried once");
        let captured = server.join().unwrap();
        let completion = crate::deepseek::parse_fim_completion_response(&value).unwrap();

        assert_eq!(captured.matches("post /completions http/1.1").count(), 2);
        assert_eq!(completion.text, "retry ok");
        assert_eq!(completion.usage.input_tokens, 7);
    }

    #[test]
    fn fim_slash_command_maps_to_deepseek_fim_complete_args() {
        let args = parse_fim_slash_command(
            "/fim --prompt 'fn main() {' --suffix '}' --response '{\"choices\":[{\"text\":\"ok\"}]}' --json",
        )
        .unwrap();

        assert_eq!(&args[0..3], ["yoyo", "deepseek", "fim-complete"]);
        assert_eq!(args[3], "--prompt");
        assert_eq!(args[4], "fn main() {");
        assert_eq!(args[5], "--suffix");
        assert_eq!(args[6], "}");
        assert_eq!(args[7], "--response");
        assert!(args[8].contains("\"choices\""));
        assert_eq!(args[9], "--json");
    }

    #[test]
    fn fim_slash_command_rejects_unterminated_quotes() {
        let err = parse_fim_slash_command("/fim --prompt 'fn main() {").unwrap_err();
        assert!(err.contains("unterminated quote"));
    }

    fn spawn_fim_mock_server(
        status: u16,
        body: &'static str,
    ) -> Option<(String, thread::JoinHandle<String>)> {
        spawn_fim_sequence_mock_server(vec![(status, body)])
    }

    fn spawn_fim_sequence_mock_server(
        responses: Vec<(u16, &'static str)>,
    ) -> Option<(String, thread::JoinHandle<String>)> {
        let listener = match TcpListener::bind("127.0.0.1:0") {
            Ok(listener) => listener,
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => return None,
            Err(e) => panic!("bind local FIM mock server: {e}"),
        };
        let endpoint = format!("http://{}/completions", listener.local_addr().unwrap());
        let handle = thread::spawn(move || {
            let mut requests = String::new();
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = Vec::new();
                let mut buffer = [0u8; 1024];
                loop {
                    let read = stream.read(&mut buffer).unwrap_or(0);
                    if read == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..read]);
                    if request_body_complete(&request) {
                        break;
                    }
                }
                let reason = if status == 200 { "OK" } else { "Error" };
                let response = format!(
                    "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).unwrap();
                requests.push_str(&String::from_utf8_lossy(&request).to_ascii_lowercase());
                requests.push('\n');
            }
            requests
        });
        Some((endpoint, handle))
    }

    fn request_body_complete(request: &[u8]) -> bool {
        let raw = String::from_utf8_lossy(request);
        let Some(header_end) = raw.find("\r\n\r\n") else {
            return false;
        };
        let content_length = raw
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                if name.eq_ignore_ascii_case("content-length") {
                    value.trim().parse::<usize>().ok()
                } else {
                    None
                }
            })
            .unwrap_or(0);
        request.len() >= header_end + 4 + content_length
    }
}
