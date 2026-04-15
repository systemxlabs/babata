use axum::{Json, extract::Path};
use serde::Serialize;

use crate::{BabataResult, config::ProviderConfig, error::BabataError};

pub(super) async fn list() -> BabataResult<Json<ListProvidersResponse>> {
    Ok(Json(ListProvidersResponse {
        providers: ProviderConfig::load_all()?,
    }))
}

pub(super) async fn create(Json(provider_config): Json<ProviderConfig>) -> BabataResult<()> {
    provider_config.validate()?;

    if ProviderConfig::load_all()?
        .iter()
        .any(|provider| provider.matches_name(&provider_config.name))
    {
        return Err(BabataError::invalid_input(format!(
            "Provider '{}' already exists",
            provider_config.name
        )));
    }

    provider_config.save()?;
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
            name, provider_config.name
        )));
    }

    provider_config.save()?;
    Ok(())
}

pub(super) async fn delete(Path(name): Path<String>) -> BabataResult<()> {
    ProviderConfig::delete(&name)?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub(crate) struct ListProvidersResponse {
    pub providers: Vec<ProviderConfig>,
}
