use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct ChatMessage {
    pub(crate) role: String,
    pub(crate) content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

pub(crate) struct MistralClient {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl MistralClient {
    pub(crate) fn new(api_key: String, model: String) -> Self {
        let client = reqwest::Client::new();
        Self {
            client,
            api_key,
            model,
        }
    }

    fn extract_language_hint(input: &str) -> Option<String> {
        let input = input.to_lowercase();
        let keywords = [
            // Systems Programming
            ("rust", "rust"),
            ("cpp", "cpp"),
            ("c++", "cpp"),
            ("c#", "cs"),
            ("csharp", "cs"),
            ("c lang", "c"),
            (" c ", "c"),
            ("objective-c", "objc"),
            ("objc", "objc"),
            ("assembly", "asm"),
            ("asm", "asm"),
            // Web Development
            ("javascript", "javascript"),
            ("js", "javascript"),
            ("typescript", "typescript"),
            ("ts", "typescript"),
            ("html", "html"),
            ("css", "css"),
            ("scss", "scss"),
            ("sass", "scss"),
            ("less", "less"),
            ("php", "php"),
            ("webassembly", "wasm"),
            ("wasm", "wasm"),
            // Scripting Languages
            ("python", "python"),
            ("py", "python"),
            ("ruby", "ruby"),
            ("perl", "perl"),
            ("lua", "lua"),
            ("powershell", "powershell"),
            ("ps1", "powershell"),
            ("shell", "shell"),
            ("bash", "shell"),
            ("zsh", "shell"),
            ("fish", "shell"),
            // JVM Languages
            ("java", "java"),
            ("kotlin", "kotlin"),
            ("scala", "scala"),
            ("groovy", "groovy"),
            ("clojure", "clojure"),
            // Mobile Development
            ("swift", "swift"),
            ("kotlin android", "kotlin"),
            ("objective-c", "objc"),
            ("dart", "dart"),
            ("flutter", "dart"),
            // Data & ML
            ("r lang", "r"),
            (" r ", "r"),
            ("julia", "julia"),
            ("matlab", "matlab"),
            ("octave", "matlab"),
            // Databases
            ("sql", "sql"),
            ("mysql", "sql"),
            ("postgresql", "sql"),
            ("postgres", "sql"),
            ("plsql", "sql"),
            ("oracle", "sql"),
            ("tsql", "sql"),
            ("mongodb", "javascript"), // For MongoDB queries
            // Configuration & Data Formats
            ("json", "json"),
            ("yaml", "yaml"),
            ("yml", "yaml"),
            ("toml", "toml"),
            ("xml", "xml"),
            ("ini", "ini"),
            ("dockerfile", "dockerfile"),
            ("docker", "dockerfile"),
            // Modern Languages
            ("go", "go"),
            ("golang", "go"),
            ("elixir", "elixir"),
            ("erlang", "erlang"),
            ("haskell", "haskell"),
            ("ocaml", "ocaml"),
            ("f#", "fsharp"),
            ("fsharp", "fsharp"),
            ("nim", "nim"),
            ("crystal", "crystal"),
            ("zig", "zig"),
            // Build & Config
            ("makefile", "makefile"),
            ("cmake", "cmake"),
            ("gradle", "gradle"),
            ("maven", "xml"),
            ("pom", "xml"),
            // Version Control
            ("git", "git"),
            ("gitignore", "gitignore"),
            ("gitconfig", "gitconfig"),
            // Markup
            ("markdown", "markdown"),
            ("md", "markdown"),
            ("tex", "tex"),
            ("latex", "tex"),
            ("restructuredtext", "rst"),
            ("rst", "rst"),
            ("asciidoc", "asciidoc"),
            // Protocol & Schema
            ("protobuf", "protobuf"),
            ("proto", "protobuf"),
            ("thrift", "thrift"),
            ("graphql", "graphql"),
            ("gql", "graphql"),
        ];

        for (keyword, lang) in keywords.iter() {
            if input.contains(keyword) {
                return Some(lang.to_string());
            }
        }

        // Check for common programming questions
        if input.contains("code")
            || input.contains("function")
            || input.contains("program")
            || input.contains("algorithm")
            || input.contains("class")
            || input.contains("method")
        {
            return Some("txt".to_string());
        }

        None
    }

    pub(crate) async fn send_message(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<(String, Option<String>)> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.api_key))?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        // Extract language hint from the last user message
        let language_hint = messages
            .last()
            .filter(|msg| msg.role == "user")
            .and_then(|msg| Self::extract_language_hint(&msg.content));

        let request = ChatRequest {
            model: self.model.clone(),
            messages,
        };

        let response = self
            .client
            .post("https://api.mistral.ai/v1/chat/completions")
            .headers(headers)
            .json(&request)
            .send()
            .await?
            .json::<ChatResponse>()
            .await?;

        Ok((response.choices[0].message.content.clone(), language_hint))
    }
}

#[cfg(test)]
mod tests {
    use super::MistralClient;

    #[test]
    fn language_hint_detects_python() {
        assert_eq!(
            MistralClient::extract_language_hint("please write a python script"),
            Some("python".to_string())
        );
    }

    #[test]
    fn language_hint_is_case_insensitive() {
        assert_eq!(
            MistralClient::extract_language_hint("Explain Rust ownership"),
            Some("rust".to_string())
        );
    }

    #[test]
    fn language_hint_maps_alias_to_canonical() {
        assert_eq!(
            MistralClient::extract_language_hint("convert this to py"),
            Some("python".to_string())
        );
        assert_eq!(
            MistralClient::extract_language_hint("using ts here"),
            Some("typescript".to_string())
        );
    }

    #[test]
    fn language_hint_returns_txt_for_code_questions() {
        assert_eq!(
            MistralClient::extract_language_hint("how do I write a function"),
            Some("txt".to_string())
        );
    }

    #[test]
    fn language_hint_returns_none_for_non_code() {
        assert_eq!(
            MistralClient::extract_language_hint("what's the weather today?"),
            None
        );
    }
}
