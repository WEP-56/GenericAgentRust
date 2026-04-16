use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub count: u32,
    pub last: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkingCheckpoint {
    pub key_info: String,
    pub related_sop: String,
    #[serde(default)]
    pub plan_path: String,
    pub updated_at: String,
}

fn assets_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets")
}

async fn read_asset_file(filename: &str) -> String {
    let path = assets_dir().join(filename);
    fs::read_to_string(path).await.unwrap_or_default()
}

fn trim_for_prompt(content: &str, max_chars: usize) -> String {
    if content.chars().count() <= max_chars {
        return content.to_string();
    }

    let head_len = max_chars / 2;
    let tail_len = max_chars / 2;
    let head: String = content.chars().take(head_len).collect();
    let tail: String = content
        .chars()
        .rev()
        .take(tail_len)
        .collect::<String>()
        .chars()
        .rev()
        .collect();

    format!("{head}\n\n[omitted long content]\n\n{tail}")
}

fn collect_markdown_files(base_dir: &Path, current_dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(current_dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            if name.starts_with('.') || name == "L4_raw_sessions" || name == "__pycache__" {
                continue;
            }
            collect_markdown_files(base_dir, &path, out);
            continue;
        }

        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }

        let rel = match path.strip_prefix(base_dir) {
            Ok(rel) => rel,
            Err(_) => continue,
        };
        let rel_str = rel.to_string_lossy();
        if rel_str.starts_with("working_checkpoint")
            || rel_str.starts_with("global_mem")
            || rel_str == "README.md"
        {
            continue;
        }

        out.push(path);
    }
}

async fn ensure_memory_bootstrap(memory_dir: &str) -> Result<(), String> {
    fs::create_dir_all(memory_dir)
        .await
        .map_err(|e| e.to_string())?;

    let global_mem = PathBuf::from(memory_dir).join("global_mem.txt");
    if !global_mem.exists() {
        fs::write(&global_mem, "")
            .await
            .map_err(|e| e.to_string())?;
    }

    let global_insight = PathBuf::from(memory_dir).join("global_mem_insight.txt");
    if !global_insight.exists() {
        let template = read_asset_file("global_mem_insight_template.txt").await;
        fs::write(&global_insight, template)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub async fn read_memory_file(memory_dir: &str, filename: &str) -> Result<String, String> {
    let path = PathBuf::from(memory_dir).join(filename);
    if !path.exists() {
        return Ok(String::new());
    }
    fs::read_to_string(&path).await.map_err(|e| e.to_string())
}

pub async fn write_memory_file(
    memory_dir: &str,
    filename: &str,
    content: &str,
) -> Result<(), String> {
    let path = PathBuf::from(memory_dir).join(filename);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent).await;
    }
    fs::write(&path, content).await.map_err(|e| e.to_string())
}

pub async fn read_working_checkpoint(
    memory_dir: &str,
) -> Result<Option<WorkingCheckpoint>, String> {
    let path = PathBuf::from(memory_dir).join("working_checkpoint.json");
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path).await.map_err(|e| e.to_string())?;
    let checkpoint = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(Some(checkpoint))
}

pub async fn write_working_checkpoint(
    memory_dir: &str,
    key_info: &str,
    related_sop: &str,
) -> Result<WorkingCheckpoint, String> {
    ensure_memory_bootstrap(memory_dir).await?;

    let previous = read_working_checkpoint(memory_dir)
        .await?
        .unwrap_or_default();

    let checkpoint = WorkingCheckpoint {
        key_info: key_info.trim().to_string(),
        related_sop: related_sop.trim().to_string(),
        plan_path: previous.plan_path,
        updated_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };

    let content = serde_json::to_string_pretty(&checkpoint).map_err(|e| e.to_string())?;
    write_memory_file(memory_dir, "working_checkpoint.json", &content).await?;
    Ok(checkpoint)
}

pub async fn set_plan_mode(memory_dir: &str, plan_path: &str) -> Result<WorkingCheckpoint, String> {
    ensure_memory_bootstrap(memory_dir).await?;
    let mut checkpoint = read_working_checkpoint(memory_dir)
        .await?
        .unwrap_or_default();
    checkpoint.plan_path = plan_path.trim().to_string();
    checkpoint.updated_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let content = serde_json::to_string_pretty(&checkpoint).map_err(|e| e.to_string())?;
    write_memory_file(memory_dir, "working_checkpoint.json", &content).await?;
    Ok(checkpoint)
}

pub async fn clear_plan_mode(memory_dir: &str) -> Result<WorkingCheckpoint, String> {
    ensure_memory_bootstrap(memory_dir).await?;
    let mut checkpoint = read_working_checkpoint(memory_dir)
        .await?
        .unwrap_or_default();
    checkpoint.plan_path.clear();
    checkpoint.updated_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let content = serde_json::to_string_pretty(&checkpoint).map_err(|e| e.to_string())?;
    write_memory_file(memory_dir, "working_checkpoint.json", &content).await?;
    Ok(checkpoint)
}

pub async fn get_long_term_update_prompt(memory_dir: &str) -> Result<String, String> {
    ensure_memory_bootstrap(memory_dir).await?;

    let sop = read_memory_file(memory_dir, "memory_management_sop.md").await?;
    if sop.trim().is_empty() {
        return Ok("Memory Management SOP not found. Do not update memory.".to_string());
    }

    let global_mem = read_memory_file(memory_dir, "global_mem.txt").await?;
    let insight = read_memory_file(memory_dir, "global_mem_insight.txt").await?;

    let mut prompt = String::from(
        "### [总结提炼经验]\n既然你认为当前任务里出现了值得沉淀的事实或流程，请遵循 memory_management_sop.md 做最小化更新。\n\
先读取现有记忆，再判断应该更新 L2 事实还是 L3 SOP；没有稳定新信息就跳过，不要为了记录而记录。\n",
    );

    if !global_mem.trim().is_empty() {
        prompt.push_str("\n[Current L2 Memory]\n");
        prompt.push_str(&trim_for_prompt(&global_mem, 4000));
        prompt.push('\n');
    }

    if !insight.trim().is_empty() {
        prompt.push_str("\n[Current L1 Insight]\n");
        prompt.push_str(&trim_for_prompt(&insight, 4000));
        prompt.push('\n');
    }

    prompt.push_str("\n[Memory Management SOP]\n");
    prompt.push_str(&trim_for_prompt(&sop, 8000));

    Ok(prompt)
}

pub async fn get_system_prompt_with_memory(
    workspace_dir: &str,
    memory_dir: &str,
) -> Result<String, String> {
    ensure_memory_bootstrap(memory_dir).await?;

    let mut prompt = String::new();
    let sys_prompt = read_asset_file("sys_prompt.txt").await;
    if !sys_prompt.trim().is_empty() {
        prompt.push_str(sys_prompt.trim());
        prompt.push_str("\n\n");
    }

    let now = chrono::Local::now();
    prompt.push_str(&format!("Today: {}\n", now.format("%Y-%m-%d %a")));
    prompt.push_str(&format!("cwd = {} (use ./ to reference)\n", workspace_dir));

    let structure = read_asset_file("insight_fixed_structure.txt").await;
    let global_mem = read_memory_file(memory_dir, "global_mem.txt").await?;
    let insight = read_memory_file(memory_dir, "global_mem_insight.txt").await?;

    prompt.push_str("\n[Memory] (../memory)\n");
    if !structure.trim().is_empty() {
        prompt.push_str(structure.trim());
        prompt.push('\n');
    }
    if !global_mem.trim().is_empty() {
        prompt.push_str("../memory/global_mem.txt:\n");
        prompt.push_str(&trim_for_prompt(&global_mem, 6000));
        prompt.push('\n');
    }
    if !insight.trim().is_empty() {
        prompt.push_str("../memory/global_mem_insight.txt:\n");
        prompt.push_str(&trim_for_prompt(&insight, 6000));
        prompt.push('\n');
    }

    if let Some(checkpoint) = read_working_checkpoint(memory_dir).await? {
        if !checkpoint.key_info.is_empty() {
            prompt.push_str("\n### [WORKING MEMORY]\n");
            prompt.push_str("<key_info>");
            prompt.push_str(&checkpoint.key_info);
            prompt.push_str("</key_info>\n");
            if !checkpoint.related_sop.is_empty() {
                prompt.push_str("如果后续步骤有不确定处，请回看这些 SOP / 线索：");
                prompt.push_str(&checkpoint.related_sop);
                prompt.push('\n');
            }
        }
    }

    let memory_path = PathBuf::from(memory_dir);
    let mut sop_paths = Vec::new();
    collect_markdown_files(&memory_path, &memory_path, &mut sop_paths);
    sop_paths.sort();

    for sop_path in sop_paths.into_iter().take(16) {
        let rel = match sop_path.strip_prefix(&memory_path) {
            Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        let content = fs::read_to_string(&sop_path).await.unwrap_or_default();
        if content.trim().is_empty() {
            continue;
        }

        prompt.push_str(&format!("\n[SOP: {}]\n", rel));
        prompt.push_str(&trim_for_prompt(&content, 6000));
        prompt.push('\n');
    }

    Ok(prompt)
}

pub async fn log_memory_access(memory_dir: &str, filepath: &str) -> Result<(), String> {
    if !filepath.contains("memory") {
        return Ok(());
    }

    let stats_path = PathBuf::from(memory_dir).join("file_access_stats.json");
    let mut stats: HashMap<String, MemoryStats> = if stats_path.exists() {
        let content = fs::read_to_string(&stats_path).await.unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    };

    let fname = PathBuf::from(filepath)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let entry = stats.entry(fname).or_insert(MemoryStats {
        count: 0,
        last: String::new(),
    });

    entry.count += 1;
    entry.last = chrono::Local::now().format("%Y-%m-%d").to_string();

    let out = serde_json::to_string_pretty(&stats).map_err(|e| e.to_string())?;
    fs::write(&stats_path, out)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
