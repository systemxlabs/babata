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

    configure_service_from_template()?;

    Ok(())
}

fn ensure_default_directories() -> BabataResult<()> {
    let base = crate::utils::babata_dir()?;
    let system_prompts = base.join("system_prompts");
    let skills = base.join("skills");
    let project_root = std::env::current_dir().map_err(|err| {
        BabataError::internal(format!("Failed to resolve current directory: {err}"))
    })?;
    let project_system_prompts = project_root.join("system_prompts");
    let project_skills = project_root.join("skills");

    if !system_prompts.exists() {
        std::fs::create_dir_all(&system_prompts).map_err(|err| {
            BabataError::config(format!(
                "Failed to create system_prompts directory '{}': {}",
                system_prompts.display(),
                err
            ))
        })?;
        println!("Created directory {}", system_prompts.display());
        if project_system_prompts.is_dir() {
            copy_dir_all(&project_system_prompts, &system_prompts)?;
            println!(
                "Copied system prompts from {}",
                project_system_prompts.display()
            );
        }
    }

    if !skills.exists() {
        std::fs::create_dir_all(&skills).map_err(|err| {
            BabataError::config(format!(
                "Failed to create skills directory '{}': {}",
                skills.display(),
                err
            ))
        })?;
        println!("Created directory {}", skills.display());
        if project_skills.is_dir() {
            copy_dir_all(&project_skills, &skills)?;
            println!("Copied skills from {}", project_skills.display());
        }
    }

    Ok(())
}

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> BabataResult<()> {
    std::fs::create_dir_all(dst).map_err(|err| {
        BabataError::config(format!(
            "Failed to create directory '{}': {}",
            dst.display(),
            err
        ))
    })?;

    for entry in std::fs::read_dir(src).map_err(|err| {
        BabataError::config(format!(
            "Failed to read directory '{}': {}",
            src.display(),
            err
        ))
    })? {
        let entry = entry.map_err(|err| {
            BabataError::config(format!(
                "Failed to read directory entry in '{}': {}",
                src.display(),
                err
            ))
        })?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &dest_path)?;
        } else if path.is_file() {
            std::fs::copy(&path, &dest_path).map_err(|err| {
                BabataError::config(format!(
                    "Failed to copy '{}' to '{}': {}",
                    path.display(),
                    dest_path.display(),
                    err
                ))
            })?;
        }
    }

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

fn configure_service_from_template() -> BabataResult<()> {
    let (template_name, output_name, output_dir) = match std::env::consts::OS {
        "macos" => (
            "babata.server.plist.template",
            "babata.server.plist",
            crate::utils::resolve_home_dir()?
                .join("Library")
                .join("LaunchAgents"),
        ),
        "linux" => (
            "babata.server.service.template",
            "babata.server.service",
            crate::utils::babata_dir()?.join("services"),
        ),
        _ => {
            return Ok(());
        }
    };

    let project_root = std::env::current_dir().map_err(|err| {
        BabataError::internal(format!("Failed to resolve current directory: {err}"))
    })?;
    let template_path = project_root.join("services").join(template_name);
    std::fs::create_dir_all(&output_dir).map_err(|err| {
        BabataError::config(format!(
            "Failed to create service output directory '{}': {}",
            output_dir.display(),
            err
        ))
    })?;
    let output_path = output_dir.join(output_name);

    render_service_template(&template_path, &output_path)?;

    println!("Generated service file: {}", output_path.display());
    Ok(())
}

fn render_service_template(template_path: &Path, output_path: &Path) -> BabataResult<()> {
    let template = std::fs::read_to_string(template_path).map_err(|err| {
        BabataError::config(format!(
            "Failed to read service template '{}': {}",
            template_path.display(),
            err
        ))
    })?;

    if !template.contains("{{HOME_DIR}}") {
        return Err(BabataError::config(format!(
            "Service template '{}' is missing '{{{{HOME_DIR}}}}' placeholder",
            template_path.display()
        )));
    }

    let home_dir = crate::utils::resolve_home_dir()?;
    let rendered = template.replace("{{HOME_DIR}}", &home_dir.to_string_lossy());

    std::fs::write(output_path, rendered).map_err(|err| {
        BabataError::config(format!(
            "Failed to write rendered service file '{}': {}",
            output_path.display(),
            err
        ))
    })?;

    Ok(())
}
