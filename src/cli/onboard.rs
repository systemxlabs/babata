use std::path::Path;

use crate::{
    BabataResult,
    channel::{Channel, TelegramChannel, WechatChannel},
    config::{ChannelConfig, Config, TelegramChannelConfig, WechatChannelConfig},
    error::BabataError,
};

const EMBEDDED_MACOS_SERVICE_TEMPLATE: &str =
    include_str!("../../services/babata.server.plist.template");
const EMBEDDED_LINUX_SERVICE_TEMPLATE: &str =
    include_str!("../../services/babata.server.service.template");

pub fn run() {
    if let Err(err) = run_onboard() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run_onboard() -> BabataResult<()> {
    let mut config = Config::load_or_init()?;

    if let Some(channel_config) = prompt_channel_setup()? {
        config.upsert_channel(channel_config);
    }

    config.validate()?;
    config.save()?;
    println!("Config updated at ~/.babata/config.json");
    let config_json = serde_json::to_string_pretty(&config)
        .map_err(|err| BabataError::config(format!("Failed to serialize config: {}", err)))?;
    println!("{config_json}");

    if prompt_background_service_setup()? {
        let should_start_background_service = configure_background_service()?;
        if should_start_background_service {
            start_background_service_after_onboard()?;
        }
    } else {
        println!("Skipped background service setup.");
    }

    Ok(())
}

fn prompt_channel_setup() -> BabataResult<Option<ChannelConfig>> {
    println!("Configure channel:");
    let channel_names = available_channel_names();
    for (idx, channel_name) in channel_names.iter().enumerate() {
        println!("{}. {}", idx + 1, channel_name);
    }
    println!("{}. skip", channel_names.len() + 1);

    let selection = prompt_line(&format!(
        "Choice (1-{}, or press Enter to skip)",
        channel_names.len() + 1
    ))?;
    let selection = selection.trim();
    if selection.is_empty() || selection.eq_ignore_ascii_case("skip") {
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
    vec![
        TelegramChannel::name().to_string(),
        WechatChannel::name().to_string(),
    ]
}

fn build_channel_config(channel_name: &str) -> BabataResult<ChannelConfig> {
    if channel_name.eq_ignore_ascii_case(TelegramChannel::name())
        || channel_name.eq_ignore_ascii_case("telegram")
    {
        return Ok(ChannelConfig::Telegram(prompt_telegram_channel_config()?));
    }

    if channel_name.eq_ignore_ascii_case(WechatChannel::name())
        || channel_name.eq_ignore_ascii_case("wechat")
    {
        return Ok(ChannelConfig::Wechat(prompt_wechat_channel_config()?));
    }

    Err(BabataError::config(format!(
        "Unsupported channel '{}'",
        channel_name
    )))
}

fn prompt_telegram_channel_config() -> BabataResult<TelegramChannelConfig> {
    let bot_token = prompt_line("Telegram bot token")?;

    let user_id_raw = prompt_line("Telegram user ID (required, e.g. 123456789)")?;
    let user_id = parse_telegram_user_id(&user_id_raw)?;

    Ok(TelegramChannelConfig { bot_token, user_id })
}

fn prompt_wechat_channel_config() -> BabataResult<WechatChannelConfig> {
    let bot_token = prompt_line("Wechat bot token")?;
    if bot_token.trim().is_empty() {
        return Err(BabataError::config("Wechat bot token cannot be empty"));
    }

    let user_id = prompt_line("Wechat user ID (required, e.g. wxid_xxx)")?;
    if user_id.trim().is_empty() {
        return Err(BabataError::config("Wechat user ID cannot be empty"));
    }

    Ok(WechatChannelConfig { bot_token, user_id })
}

fn parse_telegram_user_id(raw: &str) -> BabataResult<i64> {
    raw.trim()
        .parse::<i64>()
        .map_err(|_| BabataError::config("Invalid Telegram user id"))
}

fn prompt_background_service_setup() -> BabataResult<bool> {
    let selection = prompt_line(
        "Configure background server service? (Press Enter to skip, or Y to continue)",
    )?;
    match selection.trim() {
        "" | "N" | "n" | "no" => Ok(false),
        "Y" | "y" | "yes" => Ok(true),
        _ => Err(BabataError::config("Invalid selection")),
    }
}

fn prompt_line(label: &str) -> BabataResult<String> {
    use std::io::{self, Write};
    print!("{label}: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim_end().to_string())
}

fn configure_background_service() -> BabataResult<bool> {
    let (template_content, template_name, output_name, output_dir) = match std::env::consts::OS {
        "macos" => (
            EMBEDDED_MACOS_SERVICE_TEMPLATE,
            "babata.server.plist.template",
            "babata.server.plist",
            crate::utils::user_home_dir()?
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

    render_background_service_template(template_content, template_name, &output_path)?;

    println!("Generated service file: {}", output_path.display());
    Ok(true)
}

fn start_background_service_after_onboard() -> BabataResult<()> {
    super::server::start_background_service()?;
    println!("Started service.");
    Ok(())
}

fn render_background_service_template(
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

    let home_dir = crate::utils::user_home_dir()?;
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
