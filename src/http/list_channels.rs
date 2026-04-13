use axum::Json;
use serde::Serialize;

use crate::{
    BabataResult,
    config::{ChannelConfig, Config},
};

pub(super) async fn handle() -> BabataResult<Json<ListChannelsResponse>> {
    let config = Config::load_or_init()?;
    Ok(Json(ListChannelsResponse {
        channels: config.channels,
    }))
}

#[derive(Debug, Serialize)]
pub(crate) struct ListChannelsResponse {
    pub channels: Vec<ChannelConfig>,
}
