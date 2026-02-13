use crate::{
    BabataResult,
    config::{Config, ProviderConfig},
    error::BabataError,
};

use super::Args;

pub fn add(_args: &Args, provider_config_json: &str) {
    if let Err(err) = run_add(provider_config_json) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

pub fn delete(_args: &Args, _name: &str) {
    // TODO: implement provider delete flow.
}

pub fn list(_args: &Args) {
    if let Err(err) = run_list() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run_add(provider_config_json: &str) -> BabataResult<()> {
    let provider_config: ProviderConfig =
        serde_json::from_str(provider_config_json).map_err(|err| {
            BabataError::config(format!(
                "Invalid provider config JSON '{}': {}",
                provider_config_json, err
            ))
        })?;
    provider_config.validate()?;

    let mut config = Config::load()?;

    let provider_name = provider_config.provider_name().to_string();
    config.upsert_provider(provider_config);
    config.save()?;

    println!("Added/updated provider '{}' in config", provider_name);
    Ok(())
}

fn run_list() -> BabataResult<()> {
    let config = Config::load()?;

    for provider_config in &config.providers {
        let payload = serde_json::to_string(provider_config).map_err(|err| {
            BabataError::internal(format!(
                "Failed to serialize provider '{}' config to JSON: {}",
                provider_config.provider_name(),
                err
            ))
        })?;
        println!("{payload}");
    }

    Ok(())
}
