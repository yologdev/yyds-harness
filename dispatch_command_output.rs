#[allow(clippy::too_many_arguments)]
async fn dispatch_command(
    input: &str,
    agent: &mut yoagent::agent::Agent,
    agent_config: &mut AgentConfig,
    session_total: &mut Usage,
    session_changes: &SessionChanges,
    turn_history: &mut TurnHistory,
    bg_tracker: &commands::BackgroundJobTracker,
    spawn_tracker: &commands::SpawnTracker,
    undo_context: &mut Option<String>,
    last_input: &mut Option<String>,
    last_error: &mut Option<String>,
    bookmarks: &mut commands::Bookmarks,
    session_start: Instant,
    turn_count: usize,
    cwd: &str,
    mcp_cli_servers: &[String],
    mcp_server_configs: &[crate::cli::McpServerConfig],
    mcp_count: u32,
    openapi_count: u32,
) -> CommandResult {
    match input {
        "/quit" | "/exit" => return CommandResult::Quit,
        s if s == "/help" || s.starts_with("/help ") => {
            if !commands::handle_help_command(s) {
                commands::handle_help();
            }
            return CommandResult::Continue;
        }
        "/version" => {
            commands::handle_version();
            return CommandResult::Continue;
        }
        "/status" => {
            let ctx_used = total_tokens(agent.messages()) as u64;
            let ctx_max = effective_context_tokens();
            commands::handle_status(
                &agent_config.model,
                cwd,
                session_total,
                session_start.elapsed(),
                turn_count,
                ctx_used,
                ctx_max,
            );
            return CommandResult::Continue;
        }
        "/tokens" => {
            commands::handle_tokens(agent, session_total, &agent_config.model);
            return CommandResult::Continue;
        }
        "/cost" => {
            commands::handle_cost(session_total, &agent_config.model, agent.messages());
            return CommandResult::Continue;
        }
        "/profile" => {
            commands::handle_profile(
                agent,
                &agent_config.model,
                &agent_config.provider,
                session_start,
                session_total,
            );
            return CommandResult::Continue;
        }
        s if s == "/changelog" || s.starts_with("/changelog ") => {
            commands::handle_changelog(input);
            return CommandResult::Continue;
        }
        "/clear" => {
            let messages = agent.messages();
            let msg_count = messages.len();
            let token_count = yoagent::context::total_tokens(messages) as u64;
            if let Some(prompt) = clear_confirmation_message(msg_count, token_count) {
                use std::io::Write;
                print!("{DIM}  {prompt}{RESET}");
                let _ = std::io::stdout().flush();
                let mut answer = String::new();
                if std::io::stdin().read_line(&mut answer).is_ok() {
                    let answer = answer.trim().to_lowercase();
                    if answer != "y" && answer != "yes" {
                        println!("{DIM}  (clear cancelled){RESET}\n");
                        return CommandResult::Continue;
                    }
                } else {
                    println!("{DIM}  (clear cancelled){RESET}\n");
                    return CommandResult::Continue;
                }
            }
            *agent = agent_config.build_agent();
            session_changes.clear();
            turn_history.clear();
            reset_compact_thrash();
            reset_context_budget_warning();
            println!("{DIM}  (conversation cleared){RESET}\n");
            return CommandResult::Continue;
        }
        "/clear!" => {
            *agent = agent_config.build_agent();
            session_changes.clear();
            turn_history.clear();
            reset_compact_thrash();
            reset_context_budget_warning();
            println!("{DIM}  (conversation force-cleared){RESET}\n");
            return CommandResult::Continue;
        }
        "/model" => {
            commands::handle_model_show(&agent_config.model);
            return CommandResult::Continue;
        }
        s if s.starts_with("/model ") => {
            let new_model = s.trim_start_matches("/model ").trim();
            if new_model.is_empty() {
                println!("{DIM}  current model: {}", agent_config.model);
                println!("  usage: /model <name>{RESET}\n");
                return CommandResult::Continue;
            }
            agent_config.model = new_model.to_string();
            // Rebuild agent with new model, preserving conversation
            let saved = agent.save_messages().ok();
            *agent = agent_config.build_agent();
            let restored = if let Some(json) = saved {
                agent.restore_messages(&json).is_ok()
            } else {
                false
            };
            if restored {
                println!("{DIM}  (switched to {new_model}, conversation preserved){RESET}\n");
            } else {
                println!("{YELLOW}  (switched to {new_model}, conversation could not be preserved){RESET}\n");
            }
            return CommandResult::Continue;
        }
        "/provider" => {
            commands::handle_provider_show(&agent_config.provider);
            return CommandResult::Continue;
        }
        s if s.starts_with("/provider ") => {
            let new_provider = s.trim_start_matches("/provider ").trim();
            if new_provider.is_empty() {
                commands::handle_provider_show(&agent_config.provider);
                return CommandResult::Continue;
            }
            commands::handle_provider_switch(new_provider, agent_config, agent);
            return CommandResult::Continue;
        }
        "/think" => {
            commands::handle_think_show(agent_config.thinking);
            return CommandResult::Continue;
        }
        s if s.starts_with("/think ") => {
            let level_str = s.trim_start_matches("/think ").trim();
            if level_str.is_empty() {
                let current = thinking_level_name(agent_config.thinking);
                println!("{DIM}  thinking: {current}");
                println!("  usage: /think <off|minimal|low|medium|high>{RESET}\n");
                return CommandResult::Continue;
            }
            let new_thinking = parse_thinking_level(level_str);
            if new_thinking == agent_config.thinking {
                let current = thinking_level_name(agent_config.thinking);
                println!("{DIM}  thinking already set to {current}{RESET}\n");
                return CommandResult::Continue;
            }
            agent_config.thinking = new_thinking;
            // Rebuild agent with new thinking level, preserving conversation
            let saved = agent.save_messages().ok();
            *agent = agent_config.build_agent();
            let restored = if let Some(json) = saved {
                agent.restore_messages(&json).is_ok()
            } else {
                false
            };
            let level_name = thinking_level_name(agent_config.thinking);
            if restored {
                println!(
                    "{DIM}  (thinking set to {level_name}, conversation preserved){RESET}\n"
                );
            } else {
                println!("{YELLOW}  (thinking set to {level_name}, conversation could not be preserved){RESET}\n");
            }
            return CommandResult::Continue;
        }
        s if s == "/save" || s.starts_with("/save ") => {
            commands::handle_save(agent, input);
            return CommandResult::Continue;
        }
        s if s == "/load" || s.starts_with("/load ") => {
            commands::handle_load(agent, input);
            reset_compact_thrash();
            return CommandResult::Continue;
        }
        s if s == "/stash" || s.starts_with("/stash ") => {
            let result = commands::handle_stash(agent, s);
            print!("{result}");
            return CommandResult::Continue;
        }
        s if s == "/diff" || s.starts_with("/diff ") => {
            commands::handle_diff(s);
            return CommandResult::Continue;
        }
        s if s == "/blame" || s.starts_with("/blame ") => {
            commands::handle_blame(s);
            return CommandResult::Continue;
        }
        s if s == "/undo" || s.starts_with("/undo ") => {
            if let Some(ctx) = commands::handle_undo(s, turn_history) {
                *undo_context = Some(ctx);
            }
            return CommandResult::Continue;
        }
        "/health" => {
            commands::handle_health();
            return CommandResult::Continue;
        }
        "/doctor" => {
            commands::handle_doctor(&agent_config.provider, &agent_config.model);
            return CommandResult::Continue;
        }
        "/test" => {
            commands::handle_test();
            return CommandResult::Continue;
        }
        "/lint fix" => {
            if let Some(fix_prompt) =
                commands::handle_lint_fix(agent, session_total, &agent_config.model).await
            {
                *last_input = Some(fix_prompt);
            }
            return CommandResult::Continue;
        }
        s if s == "/lint" || s.starts_with("/lint ") => {
            if let Some(lint_result) = commands::handle_lint(s) {
                if lint_result.starts_with("Lint FAILED")
                    || lint_result.starts_with("Failed to run")
                {
                    *last_input = Some(lint_result);
                }
            }
            return CommandResult::Continue;
        }
        "/fix" => {
            if let Some(fix_prompt) =
                commands::handle_fix(agent, session_total, &agent_config.model).await
            {
                *last_input = Some(fix_prompt);
            }
            return CommandResult::Continue;
        }
        "/history" => {
            commands::handle_history(agent);
            return CommandResult::Continue;
        }
        "/search" => {
            commands::handle_search(agent, input);
            return CommandResult::Continue;
        }
        s if s.starts_with("/search ") => {
            commands::handle_search(agent, input);
            return CommandResult::Continue;
        }
        "/marks" => {
            commands::handle_marks(bookmarks);
            return CommandResult::Continue;
        }
        s if s == "/changes" || s.starts_with("/changes ") => {
            commands::handle_changes(session_changes, input);
            return CommandResult::Continue;
        }
        s if s == "/export" || s.starts_with("/export ") => {
            commands::handle_export(agent, input);
            return CommandResult::Continue;
        }
        s if s == "/mark" || s.starts_with("/mark ") => {
            commands::handle_mark(agent, input, bookmarks);
            return CommandResult::Continue;
        }
        s if s == "/jump" || s.starts_with("/jump ") => {
            commands::handle_jump(agent, input, bookmarks);
            return CommandResult::Continue;
        }
        "/config" => {
            commands::handle_config(
                &agent_config.provider,
                &agent_config.model,
                &agent_config.base_url,
                agent_config.thinking,
                agent_config.max_tokens,
                agent_config.max_turns,
                agent_config.temperature,
                &agent_config.skills,
                &agent_config.system_prompt,
                mcp_count,
                openapi_count,
                agent_config.shell_hooks.len(),
                agent,
                cwd,
            );
            return CommandResult::Continue;
        }
        s if s == "/config show" || s.starts_with("/config show ") => {
            commands::handle_config_show();
            return CommandResult::Continue;
        }
        s if s == "/config edit" || s.starts_with("/config edit ") => {
            commands::handle_config_edit();
            return CommandResult::Continue;
        }
        "/hooks" => {
            commands::handle_hooks(&agent_config.shell_hooks);
            return CommandResult::Continue;
        }
        "/permissions" => {
            commands::handle_permissions(
                agent_config.auto_approve,
                &agent_config.permissions,
                &agent_config.dir_restrictions,
            );
            return CommandResult::Continue;
        }
        "/compact" => {
            commands::handle_compact(agent);
            return CommandResult::Continue;
        }
        s if s == "/commit" || s.starts_with("/commit ") => {
            commands::handle_commit(input);
            return CommandResult::Continue;
        }
        s if s == "/context" || s.starts_with("/context ") => {
            commands::handle_context(input, &agent_config.system_prompt, agent);
            return CommandResult::Continue;
        }
        s if s == "/add" || s.starts_with("/add ") => {
            let results = commands::handle_add(input);
            if !results.is_empty() {
                // Print summaries
                for result in &results {
                    match result {
                        commands::AddResult::Text { summary, .. } => println!("{summary}"),
                        commands::AddResult::Image { summary, .. } => println!("{summary}"),
                    }
                }
                // Build content blocks with proper text context for images
                let content_blocks = build_add_content_blocks(&results);
                let word = crate::format::pluralize(results.len(), "file", "files");
                println!(
                    "{}  ({} {word} added to conversation){}\n",
                    DIM,
                    results.len(),
                    RESET
                );
                // Inject as a user message so the AI sees the file contents
                let msg = yoagent::types::AgentMessage::Llm(yoagent::types::Message::User {
                    content: content_blocks,
                    timestamp: yoagent::types::now_ms(),
                });
                agent.append_message(msg);
            }
            return CommandResult::Continue;
        }
        "/docs" => {
            commands::handle_docs(input);
            return CommandResult::Continue;
        }
        s if s.starts_with("/docs ") => {
            commands::handle_docs(input);
            return CommandResult::Continue;
        }
        "/find" => {
            commands::handle_find(input);
            return CommandResult::Continue;
        }
        s if s.starts_with("/find ") => {
            commands::handle_find(input);
            return CommandResult::Continue;
        }
        "/grep" => {
            commands::handle_grep(input);
            return CommandResult::Continue;
        }
        s if s.starts_with("/grep ") => {
            commands::handle_grep(input);
            return CommandResult::Continue;
        }
        "/init" => {
            commands::handle_init();
            return CommandResult::Continue;
        }
        s if s == "/rename" || s.starts_with("/rename ") => {
            commands::handle_rename(input);
            return CommandResult::Continue;
        }
        s if s == "/extract" || s.starts_with("/extract ") => {
            commands::handle_extract(input);
            return CommandResult::Continue;
        }
        s if s == "/move" || s.starts_with("/move ") => {
            commands::handle_move(input);
            return CommandResult::Continue;
        }
        s if s == "/refactor" || s.starts_with("/refactor ") => {
            commands::handle_refactor(input);
            return CommandResult::Continue;
        }
        s if s == "/remember" || s.starts_with("/remember ") => {
            commands::handle_remember(input);
            return CommandResult::Continue;
        }
        s if s == "/memories" || s.starts_with("/memories ") => {
            commands::handle_memories(input);
            return CommandResult::Continue;
        }
        s if s == "/forget" || s.starts_with("/forget ") => {
            commands::handle_forget(input);
            return CommandResult::Continue;
        }
        "/index" => {
            commands::handle_index();
            return CommandResult::Continue;
        }
        s if s == "/map" || s.starts_with("/map ") => {
            commands::handle_map(input);
            return CommandResult::Continue;
        }
        "/retry" => {
            *last_error = commands::handle_retry(
                agent,
                last_input,
                last_error,
                session_total,
                &agent_config.model,
            )
            .await;
            return CommandResult::Continue;
        }
        s if s == "/tree" || s.starts_with("/tree ") => {
            commands::handle_tree(input);
            return CommandResult::Continue;
        }
        s if s == "/web" || s.starts_with("/web ") => {
            commands::handle_web(input);
            return CommandResult::Continue;
        }
        s if s == "/watch" || s.starts_with("/watch ") => {
            commands::handle_watch(input);
            return CommandResult::Continue;
        }
        s if s == "/todo" || s.starts_with("/todo ") => {
            let result = commands::handle_todo(input);
            println!("{result}\n");
            return CommandResult::Continue;
        }
        s if s == "/teach" || s.starts_with("/teach ") => {
            commands::handle_teach(input);
            return CommandResult::Continue;
        }
        s if s == "/mcp" || s.starts_with("/mcp ") => {
            commands::handle_mcp(input, mcp_cli_servers, mcp_server_configs, mcp_count);
            return CommandResult::Continue;
        }
        s if s == "/ast" || s.starts_with("/ast ") => {
            commands::handle_ast_grep(input);
            return CommandResult::Continue;
        }
        s if s == "/apply" || s.starts_with("/apply ") => {
            commands::handle_apply(input);
            return CommandResult::Continue;
        }
        s if s == "/bg" || s.starts_with("/bg ") => {
            let args = input.strip_prefix("/bg").unwrap_or("").trim();
            commands::handle_bg(args, bg_tracker).await;
            return CommandResult::Continue;
        }
        s if s.starts_with("/run ") || (s.starts_with('!') && s.len() > 1) => {
            commands::handle_run(input);
            return CommandResult::Continue;
        }
        "/run" => {
            commands::handle_run_usage();
            return CommandResult::Continue;
        }
        s if s == "/pr" || s.starts_with("/pr ") => {
            commands::handle_pr(input, agent, session_total, &agent_config.model).await;
            return CommandResult::Continue;
        }
        s if s == "/git" || s.starts_with("/git ") => {
            commands::handle_git(input);
            return CommandResult::Continue;
        }
        s if s == "/spawn" || s.starts_with("/spawn ") => {
            if let Some(context_msg) = commands::handle_spawn(
                input,
                agent_config,
                session_total,
                &agent_config.model,
                agent.messages(),
                spawn_tracker,
            )
            .await
            {
                *last_input = Some(context_msg.clone());
                let prompt_start = Instant::now();
                let outcome = run_prompt_with_changes(
                    agent,
                    &context_msg,
                    session_total,
                    &agent_config.model,
                    session_changes,
                )
                .await;
                crate::format::maybe_ring_bell(prompt_start.elapsed());
                *last_error = outcome.last_tool_error;
                auto_compact_if_needed(agent);
            }
            return CommandResult::Continue;
        }
        s if s == "/review" || s.starts_with("/review ") => {
            if let Some(review_prompt) =
                commands::handle_review(input, agent, session_total, &agent_config.model)
                    .await
            {
                *last_input = Some(review_prompt);
            }
            return CommandResult::Continue;
        }
        "/update" => {
            match commands::handle_update() {
                Ok(_) => println!("Update completed successfully. Please restart yoyo to use the new version."),
                Err(e) => eprintln!("Update failed: {}", e),
            }
            return CommandResult::Continue;
        }
        s if s == "/skill" || s.starts_with("/skill ") => {
            commands::handle_skill(input, &agent_config.skills);
            return CommandResult::Continue;
        }
        s if s == "/explain" || s.starts_with("/explain ") => {
            if let Some(prompt) = commands::build_explain_prompt(input) {
                *last_input = Some(prompt.clone());
                let prompt_start = Instant::now();
                let outcome = run_prompt_with_changes(
                    agent,
                    &prompt,
                    session_total,
                    &agent_config.model,
                    session_changes,
                )
                .await;
                crate::format::maybe_ring_bell(prompt_start.elapsed());
                *last_error = outcome.last_tool_error;
                auto_compact_if_needed(agent);
            }
            return CommandResult::Continue;
        }
        s if s == "/plan" || s.starts_with("/plan ") => {
            if let Some(plan_prompt) =
                commands::handle_plan(input, agent, session_total, &agent_config.model)
                    .await
            {
                *last_input = Some(plan_prompt);
            }
            return CommandResult::Continue;
        }
        s if s.starts_with('/') && is_unknown_command(s) => {
            let cmd = s.split_whitespace().next().unwrap_or(s);
            eprintln!("{RED}  unknown command: {cmd}{RESET}");
            if let Some(suggestion) = suggest_command(s) {
                eprintln!("{YELLOW}  did you mean {suggestion}?{RESET}");
            }
            eprintln!("{DIM}  type /help for available commands{RESET}\n");
            return CommandResult::Continue;
        }
        _ => return CommandResult::NotACommand,
    }
}
