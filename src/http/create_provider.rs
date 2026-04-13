use axum::Json;

use crate::{
    BabataResult,
    config::{Config, ProviderConfig},
    error::BabataError,
};

pub(super) async fn handle(Json(provider_config): Json<ProviderConfig>) -> BabataResult<()> {
    provider_config.validate()?;

    let mut config = Config::load_or_init()?;
    let provider_name = provider_config.name().to_string();

    if config
        .providers
        .iter()
        .any(|provider| provider.matches_name(&provider_name))
    {
        return Err(BabataError::invalid_input(format!(
            "Provider '{}' already exists",
            provider_name
        )));
    }

    config.providers.push(provider_config);
    config.validate()?;
    config.save()?;
    Ok(())
}
