use axum::{Json, extract::Path};
use serde::Serialize;

use crate::{BabataResult, channel::ChannelConfig, error::BabataError};

pub(super) async fn list() -> BabataResult<Json<ListChannelsResponse>> {
    Ok(Json(ListChannelsResponse {
        channels: ChannelConfig::load_all()?,
    }))
}

pub(super) async fn create(Json(channel_config): Json<ChannelConfig>) -> BabataResult<()> {
    channel_config.validate()?;

    let channel_name = channel_config.name().to_string();
    if ChannelConfig::load_all()?
        .iter()
        .any(|channel| channel.matches_name(&channel_name))
    {
        return Err(BabataError::invalid_input(format!(
            "Channel '{}' already exists",
            channel_name
        )));
    }

    channel_config.save()?;
    Ok(())
}

pub(super) async fn update(
    Path(name): Path<String>,
    Json(channel_config): Json<ChannelConfig>,
) -> BabataResult<()> {
    channel_config.validate()?;

    if !channel_config.matches_name(&name) {
        return Err(BabataError::invalid_input(format!(
            "Channel path '{}' does not match request body channel '{}'",
            name,
            channel_config.name()
        )));
    }

    if !ChannelConfig::load_all()?
        .iter()
        .any(|channel| channel.matches_name(&name))
    {
        return Err(BabataError::not_found(format!(
            "Channel '{}' not found",
            name
        )));
    }

    channel_config.save()?;
    Ok(())
}

pub(super) async fn delete(Path(name): Path<String>) -> BabataResult<()> {
    ChannelConfig::delete(&name)?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub(crate) struct ListChannelsResponse {
    pub channels: Vec<ChannelConfig>,
}
