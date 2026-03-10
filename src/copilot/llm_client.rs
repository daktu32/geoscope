use std::sync::{Arc, Mutex};

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
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .or_else(|_| std::env::var("CLAUDE_API_KEY"))
            .unwrap_or_default();
        Self {
            api_key,
            model: "claude-sonnet-4-20250514".to_string(),
        }
    }

    pub fn has_api_key(&self) -> bool {
        !self.api_key.is_empty()
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

    pub fn is_loading(&self) -> bool {
        *self.is_loading.lock().unwrap()
    }
}
