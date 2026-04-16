use crate::llm::LlmClient;
use crate::memory;
use crate::tools;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    pub request_id: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<AgentMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interrupt: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentRunResult {
    pub messages: Vec<AgentMessage>,
    pub interrupted: bool,
}

#[derive(Debug, Clone, Default)]
struct PlanModeState {
    plan_path: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct TurnPromptControl {
    override_prompt: Option<String>,
    append_fragments: Vec<String>,
}

fn clean_for_context(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }

    let code_re = regex::Regex::new(r"```[\s\S]*?```").unwrap();
    let mut cleaned = code_re
        .replace_all(text, |caps: &regex::Captures| {
            let block = caps.get(0).map(|m| m.as_str()).unwrap_or("");
            let lines: Vec<&str> = block.lines().collect();
            if lines.len() <= 8 {
                return block.to_string();
            }

            let lang = lines
                .first()
                .map(|line| line.replace("```", "").trim().to_string())
                .unwrap_or_default();
            let body: Vec<&str> = lines[1..lines.len() - 1]
                .iter()
                .copied()
                .filter(|line| !line.trim().is_empty())
                .collect();
            if body.len() <= 6 {
                return block.to_string();
            }

            let preview = body[..5].join("\n");
            format!("```{lang}\n{preview}\n  ... ({} lines)\n```", body.len())
        })
        .to_string();

    for (pattern, replacement) in [
        (r"<file_content>[\s\S]*?</file_content>", ""),
        (r"<tool_(?:use|call)>[\s\S]*?</tool_(?:use|call)>", ""),
        (r"\n{3,}", "\n\n"),
    ] {
        cleaned = regex::Regex::new(pattern)
            .unwrap()
            .replace_all(&cleaned, replacement)
            .to_string();
    }

    if cleaned.len() > 12000 {
        cleaned = format!(
            "{}\n\n[omitted long content]\n\n{}",
            &cleaned[..6000],
            &cleaned[cleaned.len() - 6000..]
        );
    }

    cleaned.trim().to_string()
}

fn assets_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets")
}

fn should_use_cn_tools(model: &str) -> bool {
    let lower = model.to_lowercase();
    ["glm", "minimax", "kimi", "qwen", "doubao"]
        .iter()
        .any(|needle| lower.contains(needle))
}

async fn load_tools_schema(model: &str) -> Result<Value, String> {
    let file = if should_use_cn_tools(model) {
        "tools_schema_cn.json"
    } else {
        "tools_schema.json"
    };
    let content = tokio::fs::read_to_string(assets_dir().join(file))
        .await
        .map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

fn to_payload_message(message: &AgentMessage) -> Value {
    let mut payload = json!({
        "role": message.role,
        "content": clean_for_context(&message.content)
    });

    if let Some(name) = &message.name {
        payload["name"] = json!(name);
    }
    if let Some(tool_call_id) = &message.tool_call_id {
        payload["tool_call_id"] = json!(tool_call_id);
    }
    if let Some(tool_calls) = &message.tool_calls {
        payload["tool_calls"] = json!(tool_calls);
    }

    payload
}

fn build_payload_messages(
    visible_messages: &[AgentMessage],
    system_prompt: &str,
    hidden_user_prompt: Option<&str>,
) -> Vec<Value> {
    let mut payload = vec![json!({
        "role": "system",
        "content": system_prompt
    })];

    payload.extend(visible_messages.iter().map(to_payload_message));

    if let Some(prompt) = hidden_user_prompt {
        if !prompt.trim().is_empty() {
            payload.push(json!({
                "role": "user",
                "content": prompt
            }));
        }
    }

    payload
}

fn extract_tag_content(text: &str, tag: &str) -> Option<String> {
    let pattern = format!(r"(?is)<{tag}[^>]*>(.*?)</{tag}>");
    regex::Regex::new(&pattern)
        .ok()?
        .captures(text)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().trim().to_string())
}

fn extract_code_block(text: &str, language: &str) -> Option<String> {
    let pattern = format!(r"(?is)```{}\s*(.*?)```", regex::escape(language));
    regex::Regex::new(&pattern)
        .ok()?
        .captures_iter(text)
        .last()
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().trim().to_string())
}

fn extract_any_code_block(text: &str) -> Option<String> {
    regex::Regex::new(r"(?is)```[a-zA-Z0-9_]*\s*(.*?)```")
        .ok()?
        .captures_iter(text)
        .last()
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().trim().to_string())
}

fn maybe_inject_response_body_args(tool_name: &str, args: &mut Value, assistant_content: &str) {
    match tool_name {
        "code_run" => {
            let has_script = args["script"]
                .as_str()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
                || args["code"]
                    .as_str()
                    .map(|s| !s.trim().is_empty())
                    .unwrap_or(false);
            if has_script {
                return;
            }
            let code_type = args["type"]
                .as_str()
                .or_else(|| args["code_type"].as_str())
                .unwrap_or("python");
            if let Some(code) = extract_code_block(assistant_content, code_type) {
                args["script"] = json!(code);
            }
        }
        "web_execute_js" => {
            if args["script"]
                .as_str()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
            {
                return;
            }
            if let Some(script) = extract_code_block(assistant_content, "javascript") {
                args["script"] = json!(script);
            }
        }
        "file_write" => {
            if args["content"]
                .as_str()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
            {
                return;
            }
            if let Some(content) = extract_tag_content(assistant_content, "file_content")
                .or_else(|| extract_any_code_block(assistant_content))
            {
                args["content"] = json!(content);
            }
        }
        _ => {}
    }
}

fn build_no_tool_warning() -> String {
    "[System] 检测到你在上一轮回复中输出了大量文本但未调用任何工具。\n\
如果你只是在解释，这没有问题。\n\
但如果你本意是执行某些操作，请在同一轮回复中显式调用对应工具；若任务已完成，请直接明确说明。"
        .to_string()
}

fn build_anchor_prompt(
    turn: usize,
    history_info: &[String],
    checkpoint: Option<&memory::WorkingCheckpoint>,
) -> String {
    let mut prompt = String::from("\n### [WORKING MEMORY]\n<history>\n");
    let history_slice = if history_info.len() > 20 {
        &history_info[history_info.len() - 20..]
    } else {
        history_info
    };
    prompt.push_str(&history_slice.join("\n"));
    prompt.push_str("\n</history>\n");
    prompt.push_str(&format!("Current turn: {turn}\n"));

    if let Some(checkpoint) = checkpoint {
        if !checkpoint.key_info.trim().is_empty() {
            prompt.push_str("<key_info>");
            prompt.push_str(checkpoint.key_info.trim());
            prompt.push_str("</key_info>\n");
        }
        if !checkpoint.related_sop.trim().is_empty() {
            prompt.push_str("有不清晰的地方请再次读取");
            prompt.push_str(checkpoint.related_sop.trim());
            prompt.push('\n');
        }
    }

    prompt
}

fn summarize_turn(assistant_content: &str, tool_calls: &[Value]) -> (String, bool) {
    let cleaned = regex::Regex::new(r"(?is)```.*?```|<thinking>.*?</thinking>")
        .unwrap()
        .replace_all(assistant_content, "")
        .to_string();

    if let Some(summary) = extract_tag_content(&cleaned, "summary") {
        return (summary, false);
    }

    if let Some(tool_call) = tool_calls.first() {
        let tool_name = tool_call["function"]["name"]
            .as_str()
            .unwrap_or("unknown_tool");
        if tool_name == "no_tool" {
            return ("直接回答了用户问题".to_string(), true);
        }
        return (format!("调用工具{tool_name}"), true);
    }

    ("直接回答了用户问题".to_string(), true)
}

fn build_tool_followup(tool_name: &str, args: &Value, tool_result: &Value) -> TurnPromptControl {
    let mut control = TurnPromptControl::default();

    if tool_name == "start_long_term_update" {
        if let Some(prompt) = tool_result["prompt"].as_str() {
            if !prompt.trim().is_empty() {
                control.override_prompt = Some(prompt.to_string());
            }
        }
        return control;
    }

    if tool_name == "file_read" {
        let path = args["path"].as_str().unwrap_or("").to_lowercase();
        if path.contains("memory") || path.contains("sop") {
            control.append_fragments.push(
                "[SYSTEM TIPS] 正在读取记忆或SOP文件，若决定按SOP执行，请提取SOP中的关键点（特别是靠后的）并更新 working checkpoint。"
                    .to_string(),
            );
        }
    }

    control
}

fn build_turn_hint(turn: usize, max_turns: usize, tool_calls: &[Value]) -> Option<String> {
    let mut hints = Vec::new();

    let read_memory = tool_calls.iter().any(|tc| {
        tc["function"]["name"].as_str() == Some("file_read")
            && tc["function"]["arguments"]
                .as_str()
                .map(|args| args.contains("memory") || args.contains("sop"))
                .unwrap_or(false)
    });
    let updated_checkpoint = tool_calls
        .iter()
        .any(|tc| tc["function"]["name"].as_str() == Some("update_working_checkpoint"));

    if read_memory && !updated_checkpoint {
        hints.push(
            "[System] 你刚读取了 memory 或 SOP。如果这些信息影响后续执行，请调用 update_working_checkpoint 保存关键约束。"
                .to_string(),
        );
    }

    if turn > 0 && turn % 7 == 0 {
        hints.push(format!(
            "[System] 已连续执行 {turn} 轮。禁止无信息增量的重试；如果仍然受阻，应总结进展、更新工作记忆，必要时 ask_user。"
        ));
    }

    if turn + 1 >= max_turns {
        hints.push(
            "[System] 已接近最大轮次上限。若仍然存在阻塞，优先 ask_user，而不是继续盲试。"
                .to_string(),
        );
    }

    if hints.is_empty() {
        None
    } else {
        Some(hints.join("\n\n"))
    }
}

fn contains_completion_claim(content: &str) -> bool {
    ["任务完成", "全部完成", "已完成所有", "🏁"]
        .iter()
        .any(|needle| content.contains(needle))
}

fn extract_plan_path_from_text(text: &str) -> Option<String> {
    regex::Regex::new(r#"(?i)(plan_[^"'\s<>]+/plan\.md)"#)
        .ok()?
        .captures(text)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().replace('\\', "/"))
}

fn find_plan_file_in_workspace(workspace_dir: &str) -> Option<String> {
    let root = PathBuf::from(workspace_dir);
    let entries = fs::read_dir(root).ok()?;
    let mut candidates = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("plan_") {
            continue;
        }
        let plan_path = path.join("plan.md");
        if plan_path.is_file() {
            candidates.push(plan_path);
        }
    }

    candidates.sort_by_key(|path| {
        std::fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok()
    });
    candidates
        .pop()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
}

fn detect_plan_mode(
    workspace_dir: &str,
    checkpoint: Option<&memory::WorkingCheckpoint>,
) -> PlanModeState {
    if let Some(checkpoint) = checkpoint {
        if !checkpoint.plan_path.trim().is_empty() {
            let absolute = PathBuf::from(checkpoint.plan_path.trim());
            if absolute.is_file() {
                return PlanModeState {
                    plan_path: Some(absolute.to_string_lossy().replace('\\', "/")),
                };
            }
        }

        for source in [&checkpoint.key_info, &checkpoint.related_sop] {
            if let Some(plan_path) = extract_plan_path_from_text(source) {
                let absolute = PathBuf::from(workspace_dir).join(plan_path);
                if absolute.is_file() {
                    return PlanModeState {
                        plan_path: Some(absolute.to_string_lossy().replace('\\', "/")),
                    };
                }
            }
        }

        let maybe_plan = checkpoint.related_sop.to_lowercase().contains("plan")
            || checkpoint.key_info.to_lowercase().contains("plan");
        if maybe_plan {
            return PlanModeState {
                plan_path: find_plan_file_in_workspace(workspace_dir),
            };
        }
    }

    PlanModeState::default()
}

fn count_unfinished_plan_steps(plan_path: &str) -> Option<usize> {
    let content = std::fs::read_to_string(plan_path).ok()?;
    Some(
        regex::Regex::new(r"\[ \]")
            .ok()?
            .find_iter(&content)
            .count(),
    )
}

async fn consume_workspace_signal(workspace_dir: &str, filename: &str) -> Option<String> {
    let path = PathBuf::from(workspace_dir).join(filename);
    if !path.exists() || !path.is_file() {
        return None;
    }

    let content = tokio::fs::read_to_string(&path).await.ok()?;
    let _ = tokio::fs::remove_file(path).await;
    Some(content)
}

async fn apply_external_intervention(
    workspace_dir: &str,
    memory_dir: &str,
    checkpoint: Option<memory::WorkingCheckpoint>,
    hidden_user_prompt: Option<String>,
) -> Result<(Option<memory::WorkingCheckpoint>, Option<String>, bool), String> {
    let mut checkpoint = checkpoint;
    let mut hidden_user_prompt = hidden_user_prompt;
    let mut should_stop = false;

    if let Some(key_info) = consume_workspace_signal(workspace_dir, "_keyinfo").await {
        let existing_key_info = checkpoint
            .as_ref()
            .map(|item| item.key_info.trim().to_string())
            .unwrap_or_default();
        let merged = if existing_key_info.is_empty() {
            format!("[MASTER] {}", key_info.trim())
        } else {
            format!("{existing_key_info}\n[MASTER] {}", key_info.trim())
        };
        let related_sop = checkpoint
            .as_ref()
            .map(|item| item.related_sop.clone())
            .unwrap_or_default();
        checkpoint =
            Some(memory::write_working_checkpoint(memory_dir, &merged, &related_sop).await?);
    }

    if let Some(intervene) = consume_workspace_signal(workspace_dir, "_intervene").await {
        let updated_prompt = match hidden_user_prompt {
            Some(prompt) if !prompt.trim().is_empty() => {
                format!("{prompt}\n\n[MASTER] {}\n", intervene.trim())
            }
            _ => format!("[MASTER] {}\n", intervene.trim()),
        };
        hidden_user_prompt = Some(updated_prompt);
    }

    if consume_workspace_signal(workspace_dir, "_stop")
        .await
        .is_some()
    {
        should_stop = true;
    }

    Ok((checkpoint, hidden_user_prompt, should_stop))
}

fn extract_tool_calls(response: &Value) -> Vec<Value> {
    response["tool_calls"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

pub async fn run_agent_loop<F>(
    client: &LlmClient,
    mut messages: Vec<AgentMessage>,
    workspace_dir: &str,
    memory_dir: &str,
    request_id: &str,
    mut on_event: F,
) -> Result<AgentRunResult, String>
where
    F: FnMut(AgentEvent) + Send,
{
    let tools_schema = load_tools_schema(&client.model).await?;
    let mut hidden_user_prompt: Option<String> = None;
    let mut cached_checkpoint = memory::read_working_checkpoint(memory_dir).await?;
    let mut history_info: Vec<String> = messages
        .iter()
        .filter(|message| message.role == "user")
        .map(|message| format!("[USER]: {}", clean_for_context(&message.content)))
        .collect();
    let mut turn = 0usize;

    loop {
        let current_plan_mode = detect_plan_mode(workspace_dir, cached_checkpoint.as_ref());
        let max_turns = if current_plan_mode.plan_path.is_some() {
            80usize
        } else {
            15usize
        };
        if turn >= max_turns {
            break;
        }
        turn += 1;

        let (checkpoint_after_intervene, hidden_after_intervene, should_stop) =
            apply_external_intervention(
                workspace_dir,
                memory_dir,
                cached_checkpoint.clone(),
                hidden_user_prompt,
            )
            .await?;
        cached_checkpoint = checkpoint_after_intervene;
        hidden_user_prompt = hidden_after_intervene;

        if should_stop {
            return Ok(AgentRunResult {
                messages,
                interrupted: true,
            });
        }

        let system_prompt =
            memory::get_system_prompt_with_memory(workspace_dir, memory_dir).await?;
        let current_hidden_prompt = hidden_user_prompt.take();
        let payload_messages =
            build_payload_messages(&messages, &system_prompt, current_hidden_prompt.as_deref());

        let message_id = format!("{request_id}:assistant:{turn}");
        on_event(AgentEvent {
            request_id: request_id.to_string(),
            kind: "assistant_start".to_string(),
            message_id: Some(message_id.clone()),
            message: None,
            interrupt: None,
        });

        let response = client
            .chat_completion_stream(payload_messages, Some(tools_schema.clone()), |delta| {
                on_event(AgentEvent {
                    request_id: request_id.to_string(),
                    kind: "assistant_delta".to_string(),
                    message_id: Some(message_id.clone()),
                    message: Some(AgentMessage {
                        role: "assistant".to_string(),
                        content: delta.to_string(),
                        request_id: Some(message_id.clone()),
                        ..AgentMessage::default()
                    }),
                    interrupt: None,
                });
            })
            .await
            .map_err(|e| e.to_string())?;

        let assistant_message = AgentMessage {
            role: "assistant".to_string(),
            content: response["content"].as_str().unwrap_or("").to_string(),
            tool_calls: {
                let calls = extract_tool_calls(&response);
                if calls.is_empty() {
                    None
                } else {
                    Some(calls.clone())
                }
            },
            request_id: Some(message_id.clone()),
            ..AgentMessage::default()
        };

        on_event(AgentEvent {
            request_id: request_id.to_string(),
            kind: "assistant_done".to_string(),
            message_id: Some(message_id),
            message: Some(assistant_message.clone()),
            interrupt: None,
        });

        let tool_calls = assistant_message.tool_calls.clone().unwrap_or_default();
        let (summary, summary_missing) = summarize_turn(&assistant_message.content, &tool_calls);
        history_info.push(format!("[Agent] {}", clean_for_context(&summary)));
        messages.push(assistant_message.clone());
        let mut turn_prompt_control = TurnPromptControl::default();

        if tool_calls.is_empty() {
            if assistant_message.content.trim().is_empty() {
                hidden_user_prompt =
                    Some("[System] Blank response, regenerate and tooluse".to_string());
                continue;
            }

            let plan_mode = detect_plan_mode(workspace_dir, cached_checkpoint.as_ref());
            if plan_mode.plan_path.is_some()
                && contains_completion_claim(&assistant_message.content)
                && !assistant_message.content.contains("VERDICT")
                && !assistant_message.content.contains("[VERIFY]")
                && !assistant_message.content.contains("验证subagent")
            {
                hidden_user_prompt = Some(
                    "⛔ [验证拦截] 检测到你在plan模式下声称完成，但未执行[VERIFY]验证步骤。请先按plan_sop启动验证subagent，获得VERDICT后才能声称完成。"
                        .to_string(),
                );
                continue;
            }

            if assistant_message.content.len() > 100 {
                hidden_user_prompt = Some(build_no_tool_warning());
                continue;
            }

            return Ok(AgentRunResult {
                messages,
                interrupted: false,
            });
        }

        for tc in &tool_calls {
            let tool_name = tc["function"]["name"].as_str().unwrap_or("").to_string();
            let tool_call_id = tc["id"].as_str().map(|s| s.to_string());
            let mut args = tc["function"]["arguments"]
                .as_str()
                .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
                .unwrap_or_else(|| json!({}));
            maybe_inject_response_body_args(&tool_name, &mut args, &assistant_message.content);

            let execution = tools::execute_tool(&tool_name, &args, workspace_dir, memory_dir).await;

            let (content, is_interrupt, is_error) = match execution {
                Ok(value) => {
                    let followup = build_tool_followup(&tool_name, &args, &value);
                    if let Some(prompt) = followup.override_prompt {
                        turn_prompt_control.override_prompt = Some(prompt);
                    }
                    turn_prompt_control
                        .append_fragments
                        .extend(followup.append_fragments);

                    if value["status"].as_str() == Some("INTERRUPT") {
                        let candidates = value["candidates"]
                            .as_array()
                            .map(|items| {
                                items
                                    .iter()
                                    .filter_map(|item| item.as_str())
                                    .collect::<Vec<_>>()
                                    .join(" / ")
                            })
                            .filter(|joined| !joined.is_empty());

                        let suffix = candidates
                            .map(|joined| format!("\nCandidates: {joined}"))
                            .unwrap_or_default();
                        (
                            format!(
                                "[INTERRUPT] Agent asked: {}{}\nWaiting for user input...",
                                value["question"].as_str().unwrap_or(""),
                                suffix
                            ),
                            true,
                            false,
                        )
                    } else {
                        (
                            serde_json::to_string_pretty(&value)
                                .unwrap_or_else(|_| value.to_string()),
                            false,
                            false,
                        )
                    }
                }
                Err(err) => (format!("Error executing tool: {err}"), false, true),
            };

            let tool_message = AgentMessage {
                role: "tool".to_string(),
                content,
                name: Some(tool_name),
                tool_call_id,
                is_error: Some(is_error),
                ..AgentMessage::default()
            };

            on_event(AgentEvent {
                request_id: request_id.to_string(),
                kind: "tool_result".to_string(),
                message_id: None,
                message: Some(tool_message.clone()),
                interrupt: Some(is_interrupt),
            });

            messages.push(tool_message);

            if is_interrupt {
                return Ok(AgentRunResult {
                    messages,
                    interrupted: true,
                });
            }
        }

        cached_checkpoint = memory::read_working_checkpoint(memory_dir).await?;
        let plan_mode = detect_plan_mode(workspace_dir, cached_checkpoint.as_ref());
        let mut next_prompt = turn_prompt_control.override_prompt.unwrap_or_else(|| {
            build_anchor_prompt(turn, &history_info, cached_checkpoint.as_ref())
        });
        if !turn_prompt_control.append_fragments.is_empty() {
            next_prompt.push_str("\n");
            next_prompt.push_str(&turn_prompt_control.append_fragments.join("\n"));
        }
        if summary_missing {
            next_prompt.push_str(
                "\n[DANGER] 上一轮遗漏了<summary>，已根据动作自动补全。在下次回复中记得使用<summary>协议。",
            );
        }
        if let Some(turn_hint) = build_turn_hint(turn, max_turns, &tool_calls) {
            next_prompt.push_str("\n\n");
            next_prompt.push_str(&turn_hint);
        }
        if let Some(plan_path) = &plan_mode.plan_path {
            if turn >= 10 && turn % 5 == 0 {
                next_prompt = format!(
                    "[Plan Hint] 你正在计划模式。必须 file_read({plan_path}) 确认当前步骤，回复开头引用：📌 当前步骤：...\n\n{next_prompt}"
                );
            }
            if let Some(remaining) = count_unfinished_plan_steps(plan_path) {
                if remaining == 0 {
                    next_prompt.push_str(
                        "\n\n[Plan Info] plan.md 中已无 [ ] 残留。若尚未执行验证步骤，不得直接声称完成。",
                    );
                    cached_checkpoint = Some(memory::clear_plan_mode(memory_dir).await?);
                }
            }
            if turn >= 70 {
                next_prompt.push_str(
                    "\n\n[DANGER] Plan模式已运行过长。必须 ask_user 汇报进度并确认是否继续。",
                );
            }
        }
        hidden_user_prompt = Some(next_prompt);
    }

    Ok(AgentRunResult {
        messages,
        interrupted: false,
    })
}
