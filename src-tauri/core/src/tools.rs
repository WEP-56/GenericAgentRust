use regex::Regex;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Stdio;
use std::{collections::VecDeque, path::Path};
use tokio::fs;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

async fn expand_file_refs(text: &str, base_dir: &str) -> Result<String, String> {
    let re = Regex::new(r"\{\{file:(.+?):(\d+):(\d+)\}\}").unwrap();
    let mut result = text.to_string();
    let mut replacements = Vec::new();

    for cap in re.captures_iter(text) {
        let full_match = cap[0].to_string();
        let rel_path = &cap[1];
        let start: usize = cap[2].parse().unwrap_or(1);
        let end: usize = cap[3].parse().unwrap_or(1);

        let path = PathBuf::from(base_dir).join(rel_path);
        if !path.exists() {
            return Err(format!("Referenced file does not exist: {:?}", path));
        }

        let content = fs::read_to_string(&path).await.map_err(|e| e.to_string())?;
        let lines: Vec<&str> = content.lines().collect();

        if start < 1 || end > lines.len() || start > end {
            return Err(format!(
                "Line numbers out of bounds for {:?}: total lines {}, requested {}-{}",
                path,
                lines.len(),
                start,
                end
            ));
        }

        let slice = lines[(start - 1)..end].join("\n");
        replacements.push((full_match, slice));
    }

    for (from, to) in replacements {
        result = result.replace(&from, &to);
    }

    Ok(result)
}

fn extract_plan_mode_eval(script: &str) -> Option<String> {
    let patterns = [
        r#"handler\.enter_plan_mode\("([^"]+)"\)"#,
        r#"handler\.enter_plan_mode\('([^']+)'\)"#,
    ];

    for pattern in patterns {
        if let Some(captures) = Regex::new(pattern).ok()?.captures(script) {
            if let Some(path) = captures.get(1) {
                return Some(path.as_str().to_string());
            }
        }
    }

    None
}

pub async fn execute_tool(
    tool_name: &str,
    args: &Value,
    workspace_dir: &str,
    memory_dir: &str,
) -> Result<Value, String> {
    match tool_name {
        "code_run" => {
            if args["_inline_eval"].as_bool().unwrap_or(false) {
                let script = args["script"]
                    .as_str()
                    .or_else(|| args["code"].as_str())
                    .unwrap_or("")
                    .trim();

                if let Some(plan_path) = extract_plan_mode_eval(script) {
                    let absolute_plan = PathBuf::from(workspace_dir)
                        .join(plan_path)
                        .to_string_lossy()
                        .replace('\\', "/");
                    crate::memory::set_plan_mode(memory_dir, &absolute_plan).await?;
                    return Ok(json!({
                        "status": "success",
                        "result": absolute_plan,
                        "msg": "Entered plan mode",
                        "max_turns": 80
                    }));
                }

                return Ok(json!({
                    "status": "error",
                    "msg": format!("Unsupported _inline_eval script: {script}")
                }));
            }

            let code = args["code"]
                .as_str()
                .or_else(|| args["script"].as_str())
                .unwrap_or("");
            let code_type = args["code_type"]
                .as_str()
                .or_else(|| args["type"].as_str())
                .unwrap_or("python");
            let cwd = args["cwd"].as_str().unwrap_or(workspace_dir);
            let timeout_secs = args["timeout"].as_u64().unwrap_or(60);

            let mut cmd = if code_type == "python" {
                let tmp_file = PathBuf::from(cwd).join(".ai_temp.py");
                fs::write(&tmp_file, code)
                    .await
                    .map_err(|e| e.to_string())?;
                let mut c = Command::new("python");
                c.arg(&tmp_file).current_dir(cwd);
                c
            } else if code_type == "powershell" || code_type == "bash" {
                let shell = if cfg!(windows) { "powershell" } else { "bash" };
                let flag = if cfg!(windows) { "-Command" } else { "-c" };
                let mut c = Command::new(shell);
                c.arg(flag).arg(code).current_dir(cwd);
                c
            } else {
                return Err(format!("Unsupported code_type: {}", code_type));
            };

            cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

            match timeout(Duration::from_secs(timeout_secs), cmd.output()).await {
                Ok(Ok(output)) => {
                    if code_type == "python" {
                        let _ = fs::remove_file(PathBuf::from(cwd).join(".ai_temp.py")).await;
                    }

                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                    let final_stdout = if stdout.len() > 10000 {
                        format!(
                            "{}\n\n[omitted long output]\n\n{}",
                            &stdout[..5000],
                            &stdout[stdout.len() - 5000..]
                        )
                    } else {
                        stdout
                    };

                    Ok(json!({
                        "status": if output.status.success() { "success" } else { "error" },
                        "stdout": final_stdout,
                        "stderr": stderr,
                        "exit_code": output.status.code().unwrap_or(-1),
                    }))
                }
                Ok(Err(e)) => {
                    if code_type == "python" {
                        let _ = fs::remove_file(PathBuf::from(cwd).join(".ai_temp.py")).await;
                    }
                    Err(format!("Failed to execute command: {}", e))
                }
                Err(_) => {
                    if code_type == "python" {
                        let _ = fs::remove_file(PathBuf::from(cwd).join(".ai_temp.py")).await;
                    }
                    Ok(json!({
                        "status": "error",
                        "stdout": "[Timeout Error] Process killed due to timeout",
                        "exit_code": -1
                    }))
                }
            }
        }
        "file_read" => {
            let path = args["path"].as_str().unwrap_or("");
            let start = args["start"].as_u64().unwrap_or(1) as usize;
            let count = args["count"].as_u64().unwrap_or(200) as usize;
            let keyword = args["keyword"].as_str().map(|value| value.to_lowercase());
            let show_linenos = args["show_linenos"].as_bool().unwrap_or(true);
            let full_path = PathBuf::from(workspace_dir).join(path);

            let _ =
                crate::memory::log_memory_access(memory_dir, &full_path.to_string_lossy()).await;

            let content = fs::read_to_string(&full_path)
                .await
                .map_err(|e| e.to_string())?;
            let lines: Vec<&str> = content.lines().collect();

            let actual_start = start.saturating_sub(1);
            let selected: Vec<(usize, &str)> = if let Some(keyword) = keyword {
                let mut before = VecDeque::with_capacity(count / 3 + 1);
                let mut found = None;
                for (idx, line) in lines.iter().enumerate().skip(actual_start) {
                    if line.to_lowercase().contains(&keyword) {
                        let mut result: Vec<(usize, &str)> = before.iter().copied().collect();
                        result.push((idx, *line));
                        let trailing = count.saturating_sub(result.len());
                        result.extend(
                            lines
                                .iter()
                                .enumerate()
                                .skip(idx + 1)
                                .take(trailing)
                                .map(|(line_idx, value)| (line_idx, *value)),
                        );
                        found = Some(result);
                        break;
                    }
                    before.push_back((idx, *line));
                    if before.len() > count / 3 {
                        before.pop_front();
                    }
                }
                found.unwrap_or_else(|| {
                    lines
                        .iter()
                        .enumerate()
                        .skip(actual_start)
                        .take(count)
                        .map(|(idx, value)| (idx, *value))
                        .collect()
                })
            } else {
                lines
                    .iter()
                    .enumerate()
                    .skip(actual_start)
                    .take(count)
                    .map(|(idx, value)| (idx, *value))
                    .collect()
            };

            let mut rendered = Vec::with_capacity(selected.len());
            for (idx, line) in selected {
                if show_linenos {
                    rendered.push(format!("{}|{}", idx + 1, line));
                } else {
                    rendered.push(line.to_string());
                }
            }

            Ok(json!({
                "content": rendered.join("\n"),
                "total_lines": lines.len(),
                "show_linenos": show_linenos
            }))
        }
        "file_patch" => {
            let path = args["path"].as_str().unwrap_or("");
            let old_content = args["old_content"].as_str().unwrap_or("");
            let new_content_raw = args["new_content"].as_str().unwrap_or("");

            let full_path = PathBuf::from(workspace_dir).join(path);
            let mut file_content = fs::read_to_string(&full_path)
                .await
                .map_err(|e| e.to_string())?;

            let matches = file_content.matches(old_content).count();
            if matches == 0 {
                return Err("old_content not found in file".to_string());
            } else if matches > 1 {
                return Err(format!(
                    "Found {} matches for old_content. It must be unique.",
                    matches
                ));
            }

            let new_content = expand_file_refs(new_content_raw, workspace_dir).await?;
            file_content = file_content.replace(old_content, &new_content);
            fs::write(&full_path, file_content)
                .await
                .map_err(|e| e.to_string())?;

            Ok(json!({ "status": "success" }))
        }
        "file_write" => {
            let path = args["path"].as_str().unwrap_or("");
            let mode = args["mode"].as_str().unwrap_or("overwrite");
            let content_raw = args["content"].as_str().unwrap_or("");

            let full_path = PathBuf::from(workspace_dir).join(path);
            if let Some(parent) = full_path.parent() {
                let _ = fs::create_dir_all(parent).await;
            }

            let content = expand_file_refs(content_raw, workspace_dir).await?;

            match mode {
                "overwrite" => {
                    fs::write(&full_path, content)
                        .await
                        .map_err(|e| e.to_string())?;
                }
                "append" => {
                    let existing = if full_path.exists() {
                        fs::read_to_string(&full_path).await.unwrap_or_default()
                    } else {
                        String::new()
                    };
                    fs::write(&full_path, format!("{}{}", existing, content))
                        .await
                        .map_err(|e| e.to_string())?;
                }
                "prepend" => {
                    let existing = if full_path.exists() {
                        fs::read_to_string(&full_path).await.unwrap_or_default()
                    } else {
                        String::new()
                    };
                    fs::write(&full_path, format!("{}{}", content, existing))
                        .await
                        .map_err(|e| e.to_string())?;
                }
                _ => return Err(format!("Unknown mode: {}", mode)),
            }

            Ok(json!({ "status": "success" }))
        }
        "ask_user" => {
            let question = args["question"].as_str().unwrap_or("");
            let candidates = args["candidates"].clone();
            Ok(json!({
                "status": "INTERRUPT",
                "question": question,
                "candidates": candidates
            }))
        }
        "update_working_checkpoint" => {
            let key_info = args["key_info"].as_str().unwrap_or("");
            let related_sop = args["related_sop"].as_str().unwrap_or("");

            let checkpoint =
                crate::memory::write_working_checkpoint(memory_dir, key_info, related_sop).await?;

            Ok(json!({
                "status": "success",
                "result": "working checkpoint updated",
                "updated_at": checkpoint.updated_at,
            }))
        }
        "start_long_term_update" => {
            let prompt = crate::memory::get_long_term_update_prompt(memory_dir).await?;
            Ok(json!({
                "status": "success",
                "prompt": prompt,
            }))
        }
        "web_scan" => {
            let tabs_only = args["tabs_only"].as_bool().unwrap_or(false);
            let switch_tab_id = args["switch_tab_id"].as_str().map(|s| s.to_string());
            let text_only = args["text_only"].as_bool().unwrap_or(true);

            let mut manager = crate::browser::browser_manager().lock().unwrap();
            match manager.web_scan(tabs_only, switch_tab_id, text_only) {
                Ok(res) => Ok(res),
                Err(e) => Err(e.to_string()),
            }
        }
        "web_execute_js" => {
            let mut script = args["script"].as_str().unwrap_or("").to_string();
            let switch_tab_id = args["switch_tab_id"].as_str().map(|s| s.to_string());
            let save_to_file = args["save_to_file"].as_str().unwrap_or("");

            let script_path = PathBuf::from(workspace_dir).join(script.trim());
            if !script.trim().is_empty() && Path::new(&script_path).is_file() {
                script = fs::read_to_string(&script_path)
                    .await
                    .map_err(|e| e.to_string())?;
            }

            let mut res = {
                let mut manager = crate::browser::browser_manager().lock().unwrap();
                manager
                    .web_execute_js(&script, switch_tab_id)
                    .map_err(|e| e.to_string())?
            };

            if !save_to_file.is_empty() {
                let output_path = PathBuf::from(workspace_dir).join(save_to_file);
                if let Some(parent) = output_path.parent() {
                    let _ = fs::create_dir_all(parent).await;
                }
                let raw = res["js_return"].to_string();
                fs::write(&output_path, raw)
                    .await
                    .map_err(|e| e.to_string())?;
                res["saved_to_file"] = json!(output_path.to_string_lossy().to_string());
            }

            Ok(res)
        }
        _ => Err(format!("Unknown tool: {}", tool_name)),
    }
}
