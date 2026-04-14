use axum::{Json, extract::Path};
use serde::Serialize;

use crate::{
    BabataResult,
    config::{Config, ProviderConfig},
    error::BabataError,
};

pub(super) async fn list() -> BabataResult<Json<ListProvidersResponse>> {
    let config = Config::load()?;
    Ok(Json(ListProvidersResponse {
        providers: config.providers,
    }))
}

pub(super) async fn create(Json(provider_config): Json<ProviderConfig>) -> BabataResult<()> {
    provider_config.validate()?;

    let mut config = Config::load()?;
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

pub(super) async fn update(
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

    let mut config = Config::load()?;
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

pub(super) async fn delete(Path(name): Path<String>) -> BabataResult<()> {
    let mut config = Config::load()?;
    let index = config
        .providers
        .iter()
        .position(|provider| provider.matches_name(&name))
        .ok_or_else(|| BabataError::not_found(format!("Provider '{}' not found", name)))?;

    config.providers.remove(index);
    config.validate()?;
    config.save()?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub(crate) struct ListProvidersResponse {
    pub providers: Vec<ProviderConfig>,
}
