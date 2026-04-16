use axum::{Json, extract::Path};
use serde::{Deserialize, Serialize};

use crate::{
    BabataResult,
    error::BabataError,
    provider::{ProviderConfig, test_provider_connection},
};

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

pub(super) async fn test(
    Path(name): Path<String>,
    Json(request): Json<TestProviderConnectionRequest>,
) -> BabataResult<Json<TestProviderConnectionResponse>> {
    let provider_config = ProviderConfig::load(&name)?;
    let result = test_provider_connection(&provider_config, &request.model).await?;
    Ok(Json(TestProviderConnectionResponse {
        latency_ms: result.latency_ms,
    }))
}

#[derive(Debug, Serialize)]
pub(crate) struct ListProvidersResponse {
    pub providers: Vec<ProviderConfig>,
}

#[derive(Debug, Serialize)]
pub(crate) struct TestProviderConnectionResponse {
    pub latency_ms: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TestProviderConnectionRequest {
    pub model: String,
}
