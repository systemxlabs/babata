use axum::{Json, extract::Path};

use crate::{
    BabataResult,
    config::{Config, ProviderConfig},
    error::BabataError,
};

pub(super) async fn handle(
    Path(name): Path<String>,
    Json(provider_config): Json<ProviderConfig>,
) -> BabataResult<()> {
    provider_config.validate()?;

    if !provider_config.matches_name(&name) {
        return Err(BabataError::invalid_input(format!(
            "Provider path '{}' does not match request body provider '{}'",
            name,
            provider_config.name()
        )));
    }

    let mut config = Config::load_or_init()?;
    if !config
        .providers
        .iter()
        .any(|provider| provider.matches_name(&name))
    {
        return Err(BabataError::not_found(format!(
            "Provider '{}' not found",
            name
        )));
    }

    config.upsert_provider(provider_config);
    config.validate()?;
    config.save()?;
    Ok(())
}
