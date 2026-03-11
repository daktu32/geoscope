use std::sync::{Arc, Mutex};

/// Config file path: ~/.geoscope/config.json
fn config_path() -> Option<std::path::PathBuf> {
    dirs_or_home().map(|d| d.join("config.json"))
}

fn dirs_or_home() -> Option<std::path::PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|h| std::path::PathBuf::from(h).join(".geoscope"))
}

/// Load API key from ~/.geoscope/config.json
pub fn load_saved_api_key() -> Option<String> {
    let path = config_path()?;
    let data = std::fs::read_to_string(path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&data).ok()?;
    json["anthropic_api_key"].as_str().map(|s| s.to_string())
}

/// Save API key to ~/.geoscope/config.json
pub fn save_api_key(key: &str) {
    if let Some(dir) = dirs_or_home() {
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("config.json");
        // Read existing config or create new
        let mut json: serde_json::Value = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        json["anthropic_api_key"] = serde_json::Value::String(key.to_string());
        if let Ok(s) = serde_json::to_string_pretty(&json) {
            let _ = std::fs::write(&path, s);
        }
    }
}

#[derive(Clone)]
pub struct LlmClient {
    api_key: String,
    model: String,
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Shared state for async LLM communication.
pub struct LlmChannel {
    pub response: Arc<Mutex<Option<Result<String, String>>>>,
    pub is_loading: Arc<Mutex<bool>>,
}

impl LlmClient {
    pub fn new() -> Self {
        // Priority: env var > config file
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .or_else(|_| std::env::var("CLAUDE_API_KEY"))
            .ok()
            .or_else(load_saved_api_key)
            .unwrap_or_default();
        Self {
            api_key,
            model: "claude-sonnet-4-20250514".to_string(),
        }
    }

    pub fn has_api_key(&self) -> bool {
        !self.api_key.is_empty()
    }

    pub fn set_api_key(&mut self, key: &str) {
        self.api_key = key.to_string();
        save_api_key(key);
    }

    /// Send a chat request in a background thread. Returns a channel to poll for results.
    pub fn send_async(&self, messages: Vec<ChatMessage>, system_prompt: &str) -> LlmChannel {
        let channel = LlmChannel {
            response: Arc::new(Mutex::new(None)),
            is_loading: Arc::new(Mutex::new(true)),
        };

        let response_ref = channel.response.clone();
        let loading_ref = channel.is_loading.clone();
        let api_key = self.api_key.clone();
        let model = self.model.clone();
        let system = system_prompt.to_string();

        std::thread::spawn(move || {
            let result = Self::call_api(&api_key, &model, &messages, &system);
            *response_ref.lock().unwrap() = Some(result);
            *loading_ref.lock().unwrap() = false;
        });

        channel
    }

    fn call_api(
        api_key: &str,
        model: &str,
        messages: &[ChatMessage],
        system: &str,
    ) -> Result<String, String> {
        let msgs: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                })
            })
            .collect();

        let body = serde_json::json!({
            "model": model,
            "max_tokens": 2048,
            "system": system,
            "messages": msgs,
        });

        let resp = ureq::post("https://api.anthropic.com/v1/messages")
            .set("x-api-key", api_key)
            .set("anthropic-version", "2023-06-01")
            .set("content-type", "application/json")
            .send_json(&body)
            .map_err(|e| format!("API error: {}", e))?;

        let json: serde_json::Value =
            resp.into_json().map_err(|e| format!("JSON parse error: {}", e))?;

        json["content"][0]["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "No text in response".to_string())
    }
}

impl LlmChannel {
    /// Poll for response (non-blocking). Returns None if still loading.
    pub fn poll(&self) -> Option<Result<String, String>> {
        self.response.lock().unwrap().take()
    }

    #[allow(dead_code)]
    pub fn is_loading(&self) -> bool {
        *self.is_loading.lock().unwrap()
    }
}
