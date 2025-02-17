use std::sync::OnceLock;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

static SYNTAX_CACHE: OnceLock<SyntaxCache> = OnceLock::new();

pub struct SyntaxCache {
    pub syntax_set: SyntaxSet,
    pub theme_set: ThemeSet,
}

impl SyntaxCache {
    pub fn global() -> &'static SyntaxCache {
        SYNTAX_CACHE.get_or_init(|| Self::new())
    }

    fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    pub fn get_syntax(&self, language: &str) -> &syntect::parsing::SyntaxReference {
        self.syntax_set
            .find_syntax_by_token(language)
            .or_else(|| self.syntax_set.find_syntax_by_extension(language))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
    }

    pub fn get_theme(&self) -> &syntect::highlighting::Theme {
        &self.theme_set.themes["base16-ocean.dark"]
    }
} 