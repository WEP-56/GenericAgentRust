#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ga_core::agent::{AgentEvent, AgentMessage, AgentRunResult};
use ga_core::config::{load_config, save_config, AppConfig, ConfigState};
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::Emitter;

#[derive(Clone, Serialize)]
struct LlmChunkEvent {
    request_id: String,
    delta: String,
}

#[derive(Clone, Serialize)]
struct LlmDoneEvent {
    request_id: String,
}

fn resolve_path(base_dir: &str, candidate: &str) -> String {
    let path = Path::new(candidate);
    if path.is_absolute() {
        candidate.to_string()
    } else {
        PathBuf::from(base_dir)
            .join(path)
            .to_string_lossy()
            .to_string()
    }
}

#[tauri::command]
async fn execute_agent_step(
    tool_name: String,
    args: Value,
    workspace_dir: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<Value, String> {
    let cfg = state.0.lock().unwrap().clone();
    let effective_workspace = if workspace_dir.trim().is_empty() {
        cfg.workspace_dir.clone()
    } else {
        workspace_dir
    };
    let memory_dir = resolve_path(&effective_workspace, &cfg.memory_dir);

    ga_core::tools::execute_tool(&tool_name, &args, &effective_workspace, &memory_dir).await
}

#[tauri::command]
fn get_app_config(state: tauri::State<'_, ConfigState>) -> AppConfig {
    state.0.lock().unwrap().clone()
}

#[tauri::command]
fn save_app_config(config: AppConfig, state: tauri::State<'_, ConfigState>) -> Result<(), String> {
    save_config(&config)?;
    *state.0.lock().unwrap() = config;
    Ok(())
}

#[tauri::command]
async fn chat_with_llm(
    messages: Vec<Value>,
    tools: Option<Value>,
    state: tauri::State<'_, ConfigState>,
) -> Result<Value, String> {
    let cfg = state.0.lock().unwrap().clone();

    // Find active provider
    let provider = cfg
        .providers
        .iter()
        .find(|p| p.id == cfg.active_provider_id)
        .ok_or_else(|| "Active provider not found".to_string())?;

    let client = ga_core::llm::LlmClient::new(
        &provider.base_url,
        &provider.api_key,
        &provider.default_model,
        provider.is_native_anthropic,
        provider.max_retries,
        provider.max_tokens,
        provider.temperature,
    );

    match client.chat_completion(messages, tools).await {
        Ok(res) => Ok(res),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
async fn chat_with_llm_stream(
    window: tauri::Window,
    request_id: String,
    messages: Vec<Value>,
    tools: Option<Value>,
    state: tauri::State<'_, ConfigState>,
) -> Result<Value, String> {
    let cfg = state.0.lock().unwrap().clone();
    let provider = cfg
        .providers
        .iter()
        .find(|p| p.id == cfg.active_provider_id)
        .ok_or_else(|| "Active provider not found".to_string())?;

    let client = ga_core::llm::LlmClient::new(
        &provider.base_url,
        &provider.api_key,
        &provider.default_model,
        provider.is_native_anthropic,
        provider.max_retries,
        provider.max_tokens,
        provider.temperature,
    );

    let rid = request_id.clone();
    let w = window.clone();
    let result = client
        .chat_completion_stream(messages, tools, move |delta| {
            let _ = w.emit(
                "llm_chunk",
                LlmChunkEvent {
                    request_id: rid.clone(),
                    delta: delta.to_string(),
                },
            );
        })
        .await
        .map_err(|e| e.to_string())?;

    let _ = window.emit("llm_done", LlmDoneEvent { request_id });
    Ok(result)
}

#[tauri::command]
async fn get_system_prompt(workspace_dir: String, memory_dir: String) -> Result<String, String> {
    let resolved_memory_dir = resolve_path(&workspace_dir, &memory_dir);
    ga_core::memory::get_system_prompt_with_memory(&workspace_dir, &resolved_memory_dir).await
}

#[tauri::command]
async fn run_agent_stream(
    window: tauri::Window,
    request_id: String,
    messages: Vec<AgentMessage>,
    workspace_dir: Option<String>,
    state: tauri::State<'_, ConfigState>,
) -> Result<AgentRunResult, String> {
    let cfg = state.0.lock().unwrap().clone();
    let effective_workspace = workspace_dir
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| cfg.workspace_dir.clone());
    let memory_dir = resolve_path(&effective_workspace, &cfg.memory_dir);

    let provider = cfg
        .providers
        .iter()
        .find(|p| p.id == cfg.active_provider_id)
        .ok_or_else(|| "Active provider not found".to_string())?;

    let client = ga_core::llm::LlmClient::new(
        &provider.base_url,
        &provider.api_key,
        &provider.default_model,
        provider.is_native_anthropic,
        provider.max_retries,
        provider.max_tokens,
        provider.temperature,
    );

    let w = window.clone();
    let rid = request_id.clone();
    let result = ga_core::agent::run_agent_loop(
        &client,
        messages,
        &effective_workspace,
        &memory_dir,
        &request_id,
        move |event: AgentEvent| {
            let _ = w.emit("agent_event", event);
        },
    )
    .await?;

    let _ = window.emit(
        "agent_event",
        AgentEvent {
            request_id: rid,
            kind: "finished".to_string(),
            message_id: None,
            message: None,
            interrupt: Some(result.interrupted),
        },
    );

    Ok(result)
}

fn main() {
    let config = load_config();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(ConfigState(Mutex::new(config)))
        .invoke_handler(tauri::generate_handler![
            get_app_config,
            save_app_config,
            execute_agent_step,
            chat_with_llm,
            chat_with_llm_stream,
            get_system_prompt,
            run_agent_stream
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
