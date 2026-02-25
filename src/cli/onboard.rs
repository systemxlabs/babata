use std::path::Path;

use crate::{
    BabataResult,
    channel::{Channel, TelegramChannel},
    config::{
        AgentConfig, ChannelConfig, Config, DeepSeekProviderConfig, MoonshotProviderConfig,
        OpenAIProviderConfig, ProviderConfig, TelegramChannelConfig,
    },
    error::BabataError,
    provider::{DeepSeekProvider, MoonshotProvider, OpenAIProvider, Provider},
};

use super::Args;

const EMBEDDED_SYSTEM_PROMPTS: &[(&str, &str)] = &[
    ("AGENTS.md", include_str!("../../system_prompts/AGENTS.md")),
    (
        "IDENTITY.md",
        include_str!("../../system_prompts/IDENTITY.md"),
    ),
    ("SOUL.md", include_str!("../../system_prompts/SOUL.md")),
    ("USER.md", include_str!("../../system_prompts/USER.md")),
];

const EMBEDDED_BABATA_SKILL: &str = include_str!("../../skills/babata/SKILL.md");
const EMBEDDED_MACOS_SERVICE_TEMPLATE: &str =
    include_str!("../../services/babata.server.plist.template");
const EMBEDDED_LINUX_SERVICE_TEMPLATE: &str =
    include_str!("../../services/babata.server.service.template");

pub fn run(_args: &Args) {
    if let Err(err) = run_onboard() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run_onboard() -> BabataResult<()> {
    ensure_default_directories()?;

    let mut config = load_or_init_config()?;

    if let Some(provider_config) = prompt_provider_setup()? {
        config.upsert_provider(provider_config);
    }

    if let Some(agent_config) = prompt_main_agent_setup(&config)? {
        config.upsert_agent(agent_config);
    }

    if let Some(channel_config) = prompt_channel_setup()? {
        config.upsert_channel(channel_config);
    }

    config.validate()?;
    config.save()?;
    println!("Config updated at ~/.babata/config.json");
    let config_json = serde_json::to_string_pretty(&config)
        .map_err(|err| BabataError::config(format!("Failed to serialize config: {}", err)))?;
    println!("{config_json}");

    let should_start_service = configure_service_from_template()?;
    if should_start_service {
        start_service_after_onboard()?;
    }

    Ok(())
}

fn ensure_default_directories() -> BabataResult<()> {
    let base = crate::utils::babata_dir()?;
    let workspace = base.join("workspace");
    let system_prompts_dir = base.join("system_prompts");
    let skills_dir = base.join("skills");

    ensure_directory_if_missing(&system_prompts_dir, "system_prompts")?;
    ensure_directory_if_missing(&skills_dir, "skills")?;
    ensure_directory_if_missing(&workspace, "workspace")?;

    for (file_name, content) in EMBEDDED_SYSTEM_PROMPTS {
        let target = system_prompts_dir.join(file_name);
        write_embedded_file_if_missing(&target, content, "system prompt")?;
    }

    let babata_skill_dir = skills_dir.join("babata");
    ensure_directory_if_missing(&babata_skill_dir, "skill")?;
    let babata_skill_file = babata_skill_dir.join("SKILL.md");
    write_embedded_file_if_missing(&babata_skill_file, EMBEDDED_BABATA_SKILL, "skill")?;

    Ok(())
}

fn ensure_directory_if_missing(path: &Path, kind: &str) -> BabataResult<()> {
    if path.exists() {
        return Ok(());
    }

    std::fs::create_dir_all(path).map_err(|err| {
        BabataError::config(format!(
            "Failed to create {} directory '{}': {}",
            kind,
            path.display(),
            err
        ))
    })?;
    println!("Created directory {}", path.display());
    Ok(())
}

fn write_embedded_file_if_missing(path: &Path, content: &str, kind: &str) -> BabataResult<()> {
    if path.exists() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            BabataError::config(format!(
                "Failed to create parent directory '{}' for {}: {}",
                parent.display(),
                kind,
                err
            ))
        })?;
    }

    std::fs::write(path, content).map_err(|err| {
        BabataError::config(format!(
            "Failed to write {} file '{}': {}",
            kind,
            path.display(),
            err
        ))
    })?;
    println!("Wrote default {} {}", kind, path.display());
    Ok(())
}

fn prompt_provider_setup() -> BabataResult<Option<ProviderConfig>> {
    println!("Select provider:");
    let providers = available_provider_names();
    for (idx, provider) in providers.iter().enumerate() {
        println!("{}. {}", idx + 1, provider);
    }
    println!("{}. skip", providers.len() + 1);

    let selection = prompt_line(&format!("Choice (1-{})", providers.len() + 1))?;
    let selection = selection.trim();
    if selection.eq_ignore_ascii_case("skip") {
        return Ok(None);
    }
    let idx: usize = selection
        .parse()
        .map_err(|_| BabataError::config("Invalid provider selection"))?;
    if idx == providers.len() + 1 {
        return Ok(None);
    }
    let Some(provider_name) = providers.get(idx.saturating_sub(1)) else {
        return Err(BabataError::config("Invalid provider selection"));
    };

    let api_key = prompt_line("API key")?;
    Ok(Some(build_provider_config(provider_name, api_key)?))
}

fn available_provider_names() -> Vec<String> {
    vec![
        OpenAIProvider::name().to_string(),
        MoonshotProvider::name().to_string(),
        DeepSeekProvider::name().to_string(),
    ]
}

fn prompt_main_agent_setup(config: &Config) -> BabataResult<Option<AgentConfig>> {
    println!("Configure main agent:");
    println!("1. yes");
    println!("2. skip");

    let selection = prompt_line("Choice (1-2)")?;
    match selection.trim() {
        "1" | "yes" => {}
        "2" | "skip" => return Ok(None),
        _ => return Err(BabataError::config("Invalid selection")),
    }

    if config.providers.is_empty() {
        return Err(BabataError::config(
            "No providers configured; cannot set main agent",
        ));
    }

    println!("Select provider for main agent:");
    let mut provider_names: Vec<String> = config
        .providers
        .iter()
        .map(|provider| provider.provider_name().to_string())
        .collect();
    provider_names.sort();
    for (idx, name) in provider_names.iter().enumerate() {
        println!("{}. {}", idx + 1, name);
    }
    let choice = prompt_line("Choice")?;
    let idx: usize = choice
        .trim()
        .parse()
        .map_err(|_| BabataError::config("Invalid provider choice"))?;
    let Some(provider_name) = provider_names.get(idx.saturating_sub(1)) else {
        return Err(BabataError::config("Invalid provider choice"));
    };

    let provider_config = config
        .providers
        .iter()
        .find(|provider| provider.matches_name(provider_name))
        .ok_or_else(|| {
            BabataError::config(format!("Provider '{}' not found in config", provider_name))
        })?;
    let model = prompt_model_setup(provider_config)?;
    Ok(Some(AgentConfig {
        name: "main".to_string(),
        provider: provider_name.to_string(),
        model,
    }))
}

fn prompt_model_setup(provider_config: &ProviderConfig) -> BabataResult<String> {
    let supported_models = supported_models_for_provider(provider_config);
    if supported_models.is_empty() {
        return Err(BabataError::config("Provider has no supported models"));
    }

    println!("Select model for main agent:");
    for (idx, model) in supported_models.iter().enumerate() {
        println!("{}. {}", idx + 1, model);
    }

    let choice = prompt_line(&format!("Choice (1-{})", supported_models.len()))?;
    let idx: usize = choice
        .trim()
        .parse()
        .map_err(|_| BabataError::config("Invalid model choice"))?;
    let Some(model) = supported_models.get(idx.saturating_sub(1)) else {
        return Err(BabataError::config("Invalid model choice"));
    };

    Ok((*model).to_string())
}

fn supported_models_for_provider(provider_config: &ProviderConfig) -> &'static [&'static str] {
    match provider_config {
        ProviderConfig::OpenAI(_) => OpenAIProvider::supported_models(),
        ProviderConfig::Moonshot(_) => MoonshotProvider::supported_models(),
        ProviderConfig::DeepSeek(_) => DeepSeekProvider::supported_models(),
    }
}

fn build_provider_config(provider_name: &str, api_key: String) -> BabataResult<ProviderConfig> {
    if provider_name.eq_ignore_ascii_case(OpenAIProvider::name())
        || provider_name.eq_ignore_ascii_case("openai")
    {
        return Ok(ProviderConfig::OpenAI(OpenAIProviderConfig { api_key }));
    }

    if provider_name.eq_ignore_ascii_case(MoonshotProvider::name())
        || provider_name.eq_ignore_ascii_case("moonshot")
    {
        return Ok(ProviderConfig::Moonshot(MoonshotProviderConfig { api_key }));
    }

    if provider_name.eq_ignore_ascii_case(DeepSeekProvider::name())
        || provider_name.eq_ignore_ascii_case("deepseek")
    {
        return Ok(ProviderConfig::DeepSeek(DeepSeekProviderConfig { api_key }));
    }

    Err(BabataError::config(format!(
        "Unsupported provider '{}'",
        provider_name
    )))
}

fn prompt_channel_setup() -> BabataResult<Option<ChannelConfig>> {
    println!("Configure channel:");
    let channel_names = available_channel_names();
    for (idx, channel_name) in channel_names.iter().enumerate() {
        println!("{}. {}", idx + 1, channel_name);
    }
    println!("{}. skip", channel_names.len() + 1);

    let selection = prompt_line(&format!("Choice (1-{})", channel_names.len() + 1))?;
    let selection = selection.trim();
    if selection.eq_ignore_ascii_case("skip") {
        return Ok(None);
    }

    let idx: usize = selection
        .parse()
        .map_err(|_| BabataError::config("Invalid channel selection"))?;
    if idx == channel_names.len() + 1 {
        return Ok(None);
    }

    let Some(channel_name) = channel_names.get(idx.saturating_sub(1)) else {
        return Err(BabataError::config("Invalid channel selection"));
    };

    Ok(Some(build_channel_config(channel_name)?))
}

fn available_channel_names() -> Vec<String> {
    vec![TelegramChannel::name().to_string()]
}

fn build_channel_config(channel_name: &str) -> BabataResult<ChannelConfig> {
    if channel_name.eq_ignore_ascii_case(TelegramChannel::name())
        || channel_name.eq_ignore_ascii_case("telegram")
    {
        return Ok(ChannelConfig::Telegram(prompt_telegram_channel_config()?));
    }

    Err(BabataError::config(format!(
        "Unsupported channel '{}'",
        channel_name
    )))
}

fn prompt_telegram_channel_config() -> BabataResult<TelegramChannelConfig> {
    let bot_token = prompt_line("Telegram bot token")?;

    let base_url = prompt_line(
        "Telegram base url (optional, press Enter to use default https://api.telegram.org)",
    )?;
    let base_url = if base_url.trim().is_empty() {
        None
    } else {
        Some(base_url)
    };

    let polling_timeout_secs_raw =
        prompt_line("Telegram polling timeout seconds (optional, press Enter to use default 30)")?;
    let polling_timeout_secs = if polling_timeout_secs_raw.trim().is_empty() {
        None
    } else {
        Some(
            polling_timeout_secs_raw
                .trim()
                .parse::<u64>()
                .map_err(|_| BabataError::config("Invalid polling timeout seconds"))?,
        )
    };

    let allowed_user_ids_raw =
        prompt_line("Telegram allowed user IDs (comma separated, required, e.g. 123456789)")?;
    let allowed_user_ids = parse_allowed_user_ids(&allowed_user_ids_raw)?;

    Ok(TelegramChannelConfig {
        bot_token,
        base_url,
        polling_timeout_secs,
        last_update_id: None,
        allowed_user_ids,
    })
}

fn parse_allowed_user_ids(raw: &str) -> BabataResult<Vec<i64>> {
    let values = raw
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|_| BabataError::config("Invalid Telegram allowed user id"))
        })
        .collect::<BabataResult<Vec<_>>>()?;

    if values.is_empty() {
        return Err(BabataError::config(
            "Telegram allowed user IDs cannot be empty",
        ));
    }

    Ok(values)
}

fn prompt_line(label: &str) -> BabataResult<String> {
    use std::io::{self, Write};
    print!("{label}: ");
    io::stdout()
        .flush()
        .map_err(|err| BabataError::internal(format!("Failed to flush stdout: {err}")))?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|err| BabataError::internal(format!("Failed to read input: {err}")))?;
    Ok(input.trim_end().to_string())
}

fn load_or_init_config() -> BabataResult<Config> {
    let config_path = Config::path()?;
    if config_path.exists() {
        Config::load()
    } else {
        Ok(Config {
            providers: Vec::new(),
            agents: Vec::new(),
            channels: Vec::new(),
            jobs: Vec::new(),
        })
    }
}

fn configure_service_from_template() -> BabataResult<bool> {
    if std::env::consts::OS == "windows" {
        if let Err(err) = super::server::install_windows_service() {
            if super::server::is_windows_service_permission_denied_message(&err.to_string()) {
                println!(
                    "Warning: Windows service was not created due to missing Administrator privileges."
                );
                println!("Run an elevated shell and execute: babata server start");
                return Ok(false);
            }
            return Err(err);
        }
        println!("Configured Windows service: babata.server");
        return Ok(true);
    }

    let (template_content, template_name, output_name, output_dir) = match std::env::consts::OS {
        "macos" => (
            EMBEDDED_MACOS_SERVICE_TEMPLATE,
            "babata.server.plist.template",
            "babata.server.plist",
            crate::utils::resolve_home_dir()?
                .join("Library")
                .join("LaunchAgents"),
        ),
        "linux" => (
            EMBEDDED_LINUX_SERVICE_TEMPLATE,
            "babata.server.service.template",
            "babata.server.service",
            crate::utils::babata_dir()?.join("services"),
        ),
        _ => {
            return Ok(false);
        }
    };

    std::fs::create_dir_all(&output_dir).map_err(|err| {
        BabataError::config(format!(
            "Failed to create service output directory '{}': {}",
            output_dir.display(),
            err
        ))
    })?;
    let output_path = output_dir.join(output_name);

    render_service_template(template_content, template_name, &output_path)?;

    println!("Generated service file: {}", output_path.display());
    Ok(true)
}

fn start_service_after_onboard() -> BabataResult<()> {
    super::server::start_background_service()?;
    println!("Started service.");
    Ok(())
}

fn render_service_template(
    template_content: &str,
    template_name: &str,
    output_path: &Path,
) -> BabataResult<()> {
    if !template_content.contains("{{HOME_DIR}}") {
        return Err(BabataError::config(format!(
            "Service template '{}' is missing '{{{{HOME_DIR}}}}' placeholder",
            template_name
        )));
    }

    let home_dir = crate::utils::resolve_home_dir()?;
    let rendered = template_content.replace("{{HOME_DIR}}", &home_dir.to_string_lossy());

    std::fs::write(output_path, rendered).map_err(|err| {
        BabataError::config(format!(
            "Failed to write rendered service file '{}': {}",
            output_path.display(),
            err
        ))
    })?;

    Ok(())
}
