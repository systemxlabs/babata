use axum::Json;
use serde::Serialize;

use crate::{
    BabataResult,
    config::{Config, ProviderConfig},
};

pub(super) async fn handle() -> BabataResult<Json<ListProvidersResponse>> {
    let config = Config::load_or_init()?;
    Ok(Json(ListProvidersResponse {
        providers: config.providers,
    }))
}

#[derive(Debug, Serialize)]
pub(crate) struct ListProvidersResponse {
    pub providers: Vec<ProviderConfig>,
}
