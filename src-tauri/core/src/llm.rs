use bytes::Bytes;
use futures_util::StreamExt;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use serde_json::{json, Value};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("API Request Failed: {0}")]
    RequestError(#[from] reqwest_middleware::Error),
    #[error("HTTP Error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("API Response Error: {0}")]
    ApiError(String),
    #[error("Serialization Error: {0}")]
    SerdeError(#[from] serde_json::Error),
    #[error("Unknown Error: {0}")]
    Unknown(String),
}

#[derive(Clone)]
pub struct LlmClient {
    client: ClientWithMiddleware,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub is_anthropic: bool,
    pub max_tokens: u32,
    pub temperature: f32,
}

fn normalize_openai_chat_url(base_url: &str) -> String {
    let b = base_url.trim_end_matches('/');
    if b.ends_with("/v1/chat/completions") {
        b.to_string()
    } else if b.ends_with("/v1") {
        format!("{}/chat/completions", b)
    } else if b.contains("/v1/") {
        format!("{}/chat/completions", b)
    } else {
        format!("{}/v1/chat/completions", b)
    }
}

fn normalize_anthropic_messages_url(base_url: &str) -> String {
    let b = base_url.trim_end_matches('/');
    if b.ends_with("/v1/messages") {
        b.to_string()
    } else if b.ends_with("/v1") {
        format!("{}/messages", b)
    } else {
        format!("{}/v1/messages", b)
    }
}

impl LlmClient {
    pub fn new(
        base_url: &str,
        api_key: &str,
        model: &str,
        is_anthropic: bool,
        max_retries: u32,
        max_tokens: u32,
        temperature: f32,
    ) -> Self {
        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(max_retries);

        let reqwest_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap();

        let client = ClientBuilder::new(reqwest_client)
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();

        let normalized_url = if is_anthropic {
            normalize_anthropic_messages_url(base_url)
        } else {
            normalize_openai_chat_url(base_url)
        };

        Self {
            client,
            base_url: normalized_url,
            api_key: api_key.to_string(),
            model: model.to_string(),
            is_anthropic,
            max_tokens,
            temperature,
        }
    }

    pub async fn chat_completion(
        &self,
        messages: Vec<Value>,
        tools: Option<Value>,
    ) -> Result<Value, LlmError> {
        if self.is_anthropic {
            self.anthropic_chat(messages, tools).await
        } else {
            self.openai_chat(messages, tools).await
        }
    }

    pub async fn chat_completion_stream<F>(
        &self,
        messages: Vec<Value>,
        tools: Option<Value>,
        on_chunk: F,
    ) -> Result<Value, LlmError>
    where
        F: FnMut(&str) + Send,
    {
        if self.is_anthropic {
            self.anthropic_chat_stream(messages, tools, on_chunk).await
        } else {
            self.openai_chat_stream(messages, tools, on_chunk).await
        }
    }

    async fn openai_chat(
        &self,
        messages: Vec<Value>,
        tools: Option<Value>,
    ) -> Result<Value, LlmError> {
        let mut payload = json!({
            "model": self.model,
            "messages": messages,
            "temperature": self.temperature,
            "max_tokens": self.max_tokens,
        });

        if let Some(t) = tools {
            payload["tools"] = t;
        }

        let body = serde_json::to_vec(&payload)?;
        let res = self
            .client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!("{} - {}", status, err_text)));
        }

        let mut data: Value = res.json().await?;
        Ok(data["choices"][0]["message"].take())
    }

    async fn openai_chat_stream<F>(
        &self,
        messages: Vec<Value>,
        tools: Option<Value>,
        mut on_chunk: F,
    ) -> Result<Value, LlmError>
    where
        F: FnMut(&str) + Send,
    {
        let mut payload = json!({
            "model": self.model,
            "messages": messages,
            "temperature": self.temperature,
            "max_tokens": self.max_tokens,
            "stream": true
        });

        if let Some(t) = tools {
            payload["tools"] = t;
        }

        let body = serde_json::to_vec(&payload)?;
        let res = self
            .client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!("{} - {}", status, err_text)));
        }

        let mut full_content = String::new();
        let mut tool_calls_acc: Vec<Value> = Vec::new();
        let mut buffer = String::new();

        let mut stream = res.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk: Bytes = chunk.map_err(|e| LlmError::ApiError(e.to_string()))?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buffer.find('\n') {
                let mut line = buffer[..pos].to_string();
                buffer = buffer[(pos + 1)..].to_string();
                line = line.trim().to_string();

                if !line.starts_with("data:") {
                    continue;
                }

                let data = line.trim_start_matches("data:").trim();
                if data == "[DONE]" {
                    buffer.clear();
                    break;
                }

                let v: Value = match serde_json::from_str(data) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let delta = &v["choices"][0]["delta"];
                if let Some(s) = delta["content"].as_str() {
                    if !s.is_empty() {
                        full_content.push_str(s);
                        on_chunk(s);
                    }
                }

                if let Some(tcs) = delta["tool_calls"].as_array() {
                    for tc in tcs {
                        let idx = tc["index"].as_u64().unwrap_or(0) as usize;
                        while tool_calls_acc.len() <= idx {
                            tool_calls_acc.push(json!({
                                "id": "",
                                "type": "function",
                                "function": { "name": "", "arguments": "" }
                            }));
                        }

                        if let Some(id) = tc["id"].as_str() {
                            tool_calls_acc[idx]["id"] = json!(id);
                        }
                        if let Some(name) = tc["function"]["name"].as_str() {
                            tool_calls_acc[idx]["function"]["name"] = json!(name);
                        }
                        if let Some(args) = tc["function"]["arguments"].as_str() {
                            let cur = tool_calls_acc[idx]["function"]["arguments"]
                                .as_str()
                                .unwrap_or("")
                                .to_string();
                            tool_calls_acc[idx]["function"]["arguments"] =
                                json!(format!("{}{}", cur, args));
                        }
                    }
                }
            }
        }

        let mut result_msg = json!({
            "role": "assistant",
            "content": full_content
        });
        if !tool_calls_acc.is_empty() {
            result_msg["tool_calls"] = json!(tool_calls_acc);
        }
        Ok(result_msg)
    }

    async fn anthropic_chat(
        &self,
        messages: Vec<Value>,
        tools: Option<Value>,
    ) -> Result<Value, LlmError> {
        let mut system_prompt = String::new();
        let mut anthropic_msgs = Vec::new();

        for msg in messages {
            let role = msg["role"].as_str().unwrap_or("");
            if role == "system" {
                system_prompt = msg["content"].as_str().unwrap_or("").to_string();
            } else {
                anthropic_msgs.push(msg);
            }
        }

        let mut payload = json!({
            "model": self.model,
            "messages": anthropic_msgs,
            "max_tokens": self.max_tokens,
            "temperature": self.temperature,
        });

        if !system_prompt.is_empty() {
            payload["system"] = json!(system_prompt);
        }

        if let Some(t) = tools {
            payload["tools"] = t;
        }

        let body = serde_json::to_vec(&payload)?;
        let res = self
            .client
            .post(&self.base_url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!("{} - {}", status, err_text)));
        }

        let data: Value = res.json().await?;

        let content = data["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let mut tool_calls = vec![];
        for block in data["content"].as_array().unwrap_or(&vec![]) {
            if block["type"] == "tool_use" {
                tool_calls.push(json!({
                    "id": block["id"],
                    "type": "function",
                    "function": {
                        "name": block["name"],
                        "arguments": block["input"].to_string()
                    }
                }));
            }
        }

        let mut result_msg = json!({
            "role": "assistant",
            "content": content
        });

        if !tool_calls.is_empty() {
            result_msg["tool_calls"] = json!(tool_calls);
        }

        Ok(result_msg)
    }

    async fn anthropic_chat_stream<F>(
        &self,
        messages: Vec<Value>,
        tools: Option<Value>,
        mut on_chunk: F,
    ) -> Result<Value, LlmError>
    where
        F: FnMut(&str) + Send,
    {
        let mut system_prompt = String::new();
        let mut anthropic_msgs = Vec::new();
        for msg in messages {
            let role = msg["role"].as_str().unwrap_or("");
            if role == "system" {
                system_prompt = msg["content"].as_str().unwrap_or("").to_string();
            } else {
                anthropic_msgs.push(msg);
            }
        }

        let mut payload = json!({
            "model": self.model,
            "messages": anthropic_msgs,
            "max_tokens": self.max_tokens,
            "temperature": self.temperature,
            "stream": true
        });
        if !system_prompt.is_empty() {
            payload["system"] = json!(system_prompt);
        }
        if let Some(t) = tools {
            payload["tools"] = t;
        }

        let body = serde_json::to_vec(&payload)?;
        let res = self
            .client
            .post(&self.base_url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!("{} - {}", status, err_text)));
        }

        let mut full_content = String::new();
        let mut tool_calls_acc: Vec<Value> = Vec::new();
        let mut buffer = String::new();

        let mut stream = res.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk: Bytes = chunk.map_err(|e| LlmError::ApiError(e.to_string()))?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buffer.find('\n') {
                let mut line = buffer[..pos].to_string();
                buffer = buffer[(pos + 1)..].to_string();
                line = line.trim().to_string();

                if !line.starts_with("data:") {
                    continue;
                }
                let data = line.trim_start_matches("data:").trim();
                if data.is_empty() {
                    continue;
                }

                let v: Value = match serde_json::from_str(data) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                match v["type"].as_str().unwrap_or("") {
                    "content_block_delta" => {
                        if let Some(t) = v["delta"]["text"].as_str() {
                            if !t.is_empty() {
                                full_content.push_str(t);
                                on_chunk(t);
                            }
                        }
                        if let Some(partial) = v["delta"]["partial_json"].as_str() {
                            let idx = v["index"].as_u64().unwrap_or(0) as usize;
                            while tool_calls_acc.len() <= idx {
                                tool_calls_acc.push(json!({
                                    "id": "",
                                    "type": "function",
                                    "function": { "name": "", "arguments": "" }
                                }));
                            }
                            let cur = tool_calls_acc[idx]["function"]["arguments"]
                                .as_str()
                                .unwrap_or("")
                                .to_string();
                            tool_calls_acc[idx]["function"]["arguments"] =
                                json!(format!("{}{}", cur, partial));
                        }
                    }
                    "content_block_start" => {
                        if v["content_block"]["type"] == "tool_use" {
                            let idx = v["index"].as_u64().unwrap_or(0) as usize;
                            while tool_calls_acc.len() <= idx {
                                tool_calls_acc.push(json!({
                                    "id": "",
                                    "type": "function",
                                    "function": { "name": "", "arguments": "" }
                                }));
                            }
                            if let Some(id) = v["content_block"]["id"].as_str() {
                                tool_calls_acc[idx]["id"] = json!(id);
                            }
                            if let Some(name) = v["content_block"]["name"].as_str() {
                                tool_calls_acc[idx]["function"]["name"] = json!(name);
                            }
                            if !v["content_block"]["input"].is_null() {
                                tool_calls_acc[idx]["function"]["arguments"] =
                                    json!(v["content_block"]["input"].to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut result_msg = json!({
            "role": "assistant",
            "content": full_content
        });
        if !tool_calls_acc.is_empty() {
            result_msg["tool_calls"] = json!(tool_calls_acc);
        }
        Ok(result_msg)
    }
}
