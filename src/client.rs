use crate::models::{ChatMessage, ChatRequest, ChatResponse, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use std::collections::HashMap;
use std::sync::OnceLock;

static LANGUAGE_HINTS: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

pub struct MistralClient {
    client: reqwest::Client,
    api_key: String,
}

impl MistralClient {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .pool_idle_timeout(std::time::Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        
        Self { client, api_key }
    }

    fn get_language_hints() -> &'static HashMap<&'static str, &'static str> {
        LANGUAGE_HINTS.get_or_init(|| {
            let mut map = HashMap::new();
            // Systems Programming
            map.insert("rust", "rust");
            map.insert("cpp", "cpp");
            map.insert("c++", "cpp");
            map.insert("c#", "cs");
            map.insert("csharp", "cs");
            map.insert("c lang", "c");
            map.insert(" c ", "c");
            map.insert("objective-c", "objc");
            map.insert("objc", "objc");
            map.insert("assembly", "asm");
            map.insert("asm", "asm");
            
            // Web Development
            map.insert("javascript", "javascript");
            map.insert("js", "javascript");
            map.insert("typescript", "typescript");
            map.insert("ts", "typescript");
            map.insert("html", "html");
            map.insert("css", "css");
            map.insert("scss", "scss");
            map.insert("sass", "scss");
            map.insert("less", "less");
            map.insert("php", "php");
            map.insert("webassembly", "wasm");
            map.insert("wasm", "wasm");
            
            // Scripting Languages
            map.insert("python", "python");
            map.insert("py", "python");
            map.insert("ruby", "ruby");
            map.insert("perl", "perl");
            map.insert("lua", "lua");
            map.insert("powershell", "powershell");
            map.insert("ps1", "powershell");
            map.insert("shell", "shell");
            map.insert("bash", "shell");
            map.insert("zsh", "shell");
            map.insert("fish", "shell");
            
            // Add more language mappings...
            map
        })
    }

    fn extract_language_hint(input: &str) -> Option<String> {
        let input = input.to_lowercase();
        let hints = Self::get_language_hints();
        
        for (keyword, lang) in hints.iter() {
            if input.contains(keyword) {
                return Some((*lang).to_string());
            }
        }

        // Check for common programming questions
        if input.contains("code") || input.contains("function") || input.contains("program") 
            || input.contains("algorithm") || input.contains("class") || input.contains("method") {
            return Some("txt".to_string());
        }

        None
    }

    pub async fn send_message(&self, messages: Vec<ChatMessage>) -> Result<(String, Option<String>)> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.api_key))
                .map_err(|_| crate::models::Error::Api("Invalid API key format".to_string()))?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        // Extract language hint from the last user message
        let language_hint = messages.last()
            .filter(|msg| msg.role == "user")
            .and_then(|msg| Self::extract_language_hint(&msg.content));

        let request = ChatRequest {
            model: "mistral-small".to_string(),
            messages,
        };

        let response = self
            .client
            .post("https://api.mistral.ai/v1/chat/completions")
            .headers(headers)
            .json(&request)
            .send()
            .await?
            .error_for_status()?
            .json::<ChatResponse>()
            .await?;

        Ok((response.choices[0].message.content.clone(), language_hint))
    }
} 