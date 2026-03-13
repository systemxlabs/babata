use crate::{
    BabataResult,
    config::{ChannelConfig, Config},
    error::BabataError,
};

pub fn add(channel_config_json: &str) {
    if let Err(err) = run_add(channel_config_json) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

pub fn delete(name: &str) {
    if let Err(err) = run_delete(name) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

pub fn list() {
    if let Err(err) = run_list() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run_add(channel_config_json: &str) -> BabataResult<()> {
    let channel_config: ChannelConfig =
        serde_json::from_str(channel_config_json).map_err(|err| {
            BabataError::config(format!(
                "Invalid channel config JSON '{}': {}",
                channel_config_json, err
            ))
        })?;

    let channel_name = channel_config.name().to_string();
    let mut config = Config::load()?;
    config.upsert_channel(channel_config);
    config.validate()?;
    config.save()?;

    println!("Added/updated channel '{}' in config", channel_name);
    Ok(())
}

fn run_delete(name: &str) -> BabataResult<()> {
    let mut config = Config::load()?;

    let index = config
        .channels
        .iter()
        .position(|channel| channel.matches_name(name))
        .ok_or_else(|| BabataError::config(format!("Channel '{}' not found in config", name)))?;

    let deleted = config.channels.remove(index);
    config.validate()?;
    config.save()?;

    println!("Deleted channel '{}'", deleted.name());
    Ok(())
}

fn run_list() -> BabataResult<()> {
    let config = Config::load()?;

    for channel_config in &config.channels {
        let payload = serde_json::to_string(channel_config).map_err(|err| {
            BabataError::internal(format!(
                "Failed to serialize channel '{}' config to JSON: {}",
                channel_config.name(),
                err
            ))
        })?;
        println!("{payload}");
    }

    Ok(())
}
