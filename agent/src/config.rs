pub struct Config {
    pub system_prompt: String,
    pub providers: Vec<ProviderConfig>,
    pub skills: Vec<SkillConfig>,
}

pub struct ProviderConfig {
    // Whether the provider is enabled
    pub enabled: bool,
    // The name of the provider, e.g., "azure", "openai", "custom"
    pub name: String,

    pub openai: Option<OpenAIProviderConfig>,
    pub anthropic: Option<AnthropicProviderConfig>,
}

pub struct OpenAIProviderConfig {
    // The model to use, e.g., "gpt-4", "gpt-3.5-turbo"
    pub model: String,
    // The API kind, e.g., "generate" for one-shot generation, "interact" for multi-turn interaction
    pub api_kind: ApiKind,
    // The completed URL for the provider's API
    pub base_url: String,
    // The API key for authentication
    pub api_key: String,
}

pub enum ApiKind {
    // One-shot
    Generation,
    // Multi-turn interaction
    Interaction,
}

pub struct AnthropicProviderConfig {
    // The model to use, e.g., "gpt-4", "gpt-3.5-turbo"
    pub model: String,
    // The completed URL for the provider's API
    pub base_url: String,
    // The API key for authentication
    pub api_key: String,
}

pub struct SkillConfig {
    // Whether the skill is enabled
    pub enabled: bool,
    // Whether the whole skill.md is inlined in prompt
    pub inlined: bool,
    // Absolute path to the skill dir
    pub path: String,
}
