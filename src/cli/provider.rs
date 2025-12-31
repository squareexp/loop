use serde::{Serialize, Deserialize};
use crate::parser::ast::{ToolDecl, Value};
use crate::runtime::engine::{AgentProvider, AgentResponse, ToolCallRequest};

#[derive(Serialize, Deserialize)]
struct LLMJsonSchema {
    reasoning: String,
    tool_call: Option<LLMToolCall>,
}

#[derive(Serialize, Deserialize)]
struct LLMToolCall {
    name: String,
    arguments: Vec<serde_json::Value>,
}

pub struct GeminiProvider {
    pub api_key: Option<String>,
}

impl GeminiProvider {
    pub fn new() -> Self {
        Self {
            api_key: std::env::var("GEMINI_API_KEY").ok(),
        }
    }
}

impl AgentProvider for GeminiProvider {
    async fn reason(&self, prompt: &str, tools: &[ToolDecl]) -> Result<AgentResponse, String> {
        let Some(ref key) = self.api_key else {
            // Mock mode if API key not found
            return mock_reason(prompt, tools);
        };

        let client = reqwest::Client::new();
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-pro:generateContent?key={}",
            key
        );

        let payload = serde_json::json!({
            "contents": [{
                "parts": [{ "text": prompt }]
            }],
            "generationConfig": {
                "responseMimeType": "application/json"
            }
        });

        let res = client.post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Gemini HTTP request failed: {}", e))?;

        let res_json: serde_json::Value = res.json()
            .await
            .map_err(|e| format!("Failed to parse Gemini response: {}", e))?;

        let text = res_json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| format!("Empty response from Gemini. Full response: {:?}", res_json))?;

        parse_llm_json(text)
    }
}

pub struct ClaudeProvider {
    pub api_key: Option<String>,
}

impl ClaudeProvider {
    pub fn new() -> Self {
        Self {
            api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
        }
    }
}

impl AgentProvider for ClaudeProvider {
    async fn reason(&self, prompt: &str, tools: &[ToolDecl]) -> Result<AgentResponse, String> {
        let Some(ref key) = self.api_key else {
            return mock_reason(prompt, tools);
        };

        let client = reqwest::Client::new();
        let url = "https://api.anthropic.com/v1/messages";

        let payload = serde_json::json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 1024,
            "messages": [{ "role": "user", "content": prompt }]
        });

        let res = client.post(url)
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Claude HTTP request failed: {}", e))?;

        let res_json: serde_json::Value = res.json()
            .await
            .map_err(|e| format!("Failed to parse Claude response: {}", e))?;

        let text = res_json["content"][0]["text"]
            .as_str()
            .ok_or_else(|| format!("Empty response from Claude. Response: {:?}", res_json))?;

        parse_llm_json(text)
    }
}

pub struct OllamaProvider {
    pub model: String,
}

impl OllamaProvider {
    pub fn new(model: String) -> Self {
        Self { model }
    }
}

impl AgentProvider for OllamaProvider {
    async fn reason(&self, prompt: &str, tools: &[ToolDecl]) -> Result<AgentResponse, String> {
        let client = reqwest::Client::new();
        let url = "http://localhost:11434/api/generate";

        let payload = serde_json::json!({
            "model": &self.model,
            "prompt": prompt,
            "stream": false,
            "format": "json"
        });

        let res = client.post(url)
            .json(&payload)
            .send()
            .await;

        match res {
            Ok(response) => {
                let res_json: serde_json::Value = response.json()
                    .await
                    .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;

                let text = res_json["response"]
                    .as_str()
                    .ok_or_else(|| format!("Empty response from Ollama. Response: {:?}", res_json))?;

                parse_llm_json(text)
            }
            Err(_) => {
                // If local Ollama isn't running, default to mock mode instead of failing
                mock_reason(prompt, tools)
            }
        }
    }
}

fn parse_llm_json(text: &str) -> Result<AgentResponse, String> {
    let parsed: LLMJsonSchema = serde_json::from_str(text)
        .map_err(|e| format!("JSON parsing of model response failed: {}. Raw: {}", e, text))?;

    let tool_call = parsed.tool_call.map(|tc| {
        let args = tc.arguments.into_iter().map(|arg| {
            if let Some(s) = arg.as_str() {
                Value::String(s.to_string())
            } else if let Some(i) = arg.as_i64() {
                Value::Integer(i)
            } else if let Some(b) = arg.as_bool() {
                Value::Boolean(b)
            } else {
                Value::String(arg.to_string())
            }
        }).collect();
        ToolCallRequest { name: tc.name, args }
    });

    Ok(AgentResponse {
        reasoning: parsed.reasoning,
        tool_call,
    })
}

fn mock_reason(prompt: &str, tools: &[ToolDecl]) -> Result<AgentResponse, String> {
    // Intelligent simulation for testing:
    // Check if the prompt references write_file, counter, or other vars
    if prompt.contains("is_done") && prompt.contains("false") {
        if tools.iter().any(|t| t.name == "write_file") {
            return Ok(AgentResponse {
                reasoning: "Let's call write_file to proceed with work.".to_string(),
                tool_call: Some(ToolCallRequest {
                    name: "write_file".to_string(),
                    args: vec![
                        Value::String("src/main.rs".to_string()),
                        Value::String("fn main() { println!(\"fixed\"); }".to_string()),
                    ],
                }),
            });
        }
    }

    Ok(AgentResponse {
        reasoning: "We have nothing left to do. The state is clean.".to_string(),
        tool_call: None,
    })
}

pub enum Provider {
    Gemini(GeminiProvider),
    Claude(ClaudeProvider),
    Ollama(OllamaProvider),
}

impl AgentProvider for Provider {
    async fn reason(&self, prompt: &str, tools: &[ToolDecl]) -> Result<AgentResponse, String> {
        match self {
            Provider::Gemini(p) => p.reason(prompt, tools).await,
            Provider::Claude(p) => p.reason(prompt, tools).await,
            Provider::Ollama(p) => p.reason(prompt, tools).await,
        }
    }
}
