use headless_chrome::{Browser, LaunchOptions};
use serde_json::{json, Value};
use std::sync::{Mutex, OnceLock};
use thiserror::Error;

pub fn browser_manager() -> &'static Mutex<BrowserManager> {
    static MANAGER: OnceLock<Mutex<BrowserManager>> = OnceLock::new();
    MANAGER.get_or_init(|| Mutex::new(BrowserManager::new()))
}

#[derive(Debug, Error)]
pub enum BrowserError {
    #[error("Browser failed to start: {0}")]
    LaunchError(String),
    #[error("Tab operation failed: {0}")]
    TabError(String),
    #[error("Execution failed: {0}")]
    ExecError(String),
}

pub struct BrowserManager {
    browser: Option<Browser>,
}

impl BrowserManager {
    pub fn new() -> Self {
        Self { browser: None }
    }

    fn get_or_launch_browser(&mut self) -> Result<&Browser, BrowserError> {
        if self.browser.is_none() {
            let options = LaunchOptions::default_builder()
                .headless(false)
                .build()
                .map_err(|e| BrowserError::LaunchError(e.to_string()))?;

            let b = Browser::new(options).map_err(|e| BrowserError::LaunchError(e.to_string()))?;
            self.browser = Some(b);
        }
        Ok(self.browser.as_ref().unwrap())
    }

    pub fn web_scan(
        &mut self,
        tabs_only: bool,
        switch_tab_id: Option<String>,
        text_only: bool,
    ) -> Result<Value, BrowserError> {
        let browser = self.get_or_launch_browser()?;

        let tabs_guard = browser
            .get_tabs()
            .lock()
            .map_err(|e| BrowserError::TabError(e.to_string()))?;
        if tabs_guard.is_empty() {
            return Err(BrowserError::TabError("No tabs found".to_string()));
        }

        let mut tab_infos = Vec::new();
        for tab in tabs_guard.iter() {
            let url = tab.get_url();
            let title = tab
                .get_title()
                .map_err(|e| BrowserError::TabError(e.to_string()))?;
            let id = tab.get_target_id().to_string();
            let url = if url.chars().count() > 50 {
                format!("{}...", url.chars().take(50).collect::<String>())
            } else {
                url
            };

            tab_infos.push(json!({
                "id": id,
                "url": url,
                "title": title
            }));
        }

        let active_tab = match switch_tab_id {
            Some(id) => tabs_guard
                .iter()
                .find(|t| t.get_target_id().to_string() == id)
                .cloned()
                .unwrap_or_else(|| tabs_guard[0].clone()),
            None => tabs_guard[0].clone(),
        };

        let mut result = json!({
            "status": "success",
            "metadata": {
                "tabs_count": tabs_guard.len(),
                "tabs": tab_infos,
                "active_tab": active_tab.get_target_id().to_string()
            }
        });

        if !tabs_only {
            let content = if text_only {
                active_tab
                    .evaluate("document.body.innerText", false)
                    .map_err(|e| BrowserError::ExecError(e.to_string()))?
                    .value
                    .unwrap_or(json!(""))
                    .as_str()
                    .unwrap_or("")
                    .to_string()
            } else {
                active_tab
                    .evaluate("document.body.innerHTML", false)
                    .map_err(|e| BrowserError::ExecError(e.to_string()))?
                    .value
                    .unwrap_or(json!(""))
                    .as_str()
                    .unwrap_or("")
                    .to_string()
            };

            result["content"] = json!(content);
        }

        Ok(result)
    }

    pub fn web_execute_js(
        &mut self,
        script: &str,
        switch_tab_id: Option<String>,
    ) -> Result<Value, BrowserError> {
        let browser = self.get_or_launch_browser()?;
        let tabs_guard = browser
            .get_tabs()
            .lock()
            .map_err(|e| BrowserError::TabError(e.to_string()))?;
        if tabs_guard.is_empty() {
            return Err(BrowserError::TabError("No tabs found".to_string()));
        }

        let active_tab = match switch_tab_id {
            Some(id) => tabs_guard
                .iter()
                .find(|t| t.get_target_id().to_string() == id)
                .cloned()
                .unwrap_or_else(|| tabs_guard[0].clone()),
            None => tabs_guard[0].clone(),
        };

        let res = active_tab
            .evaluate(script, true)
            .map_err(|e| BrowserError::ExecError(e.to_string()))?;

        Ok(json!({
            "status": "success",
            "js_return": res.value.unwrap_or(json!(null))
        }))
    }
}
