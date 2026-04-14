use axum::{Json, extract::Path};
use serde::Serialize;

use crate::{
    BabataResult,
    config::{ChannelConfig, Config},
    error::BabataError,
};

pub(super) async fn list() -> BabataResult<Json<ListChannelsResponse>> {
    let config = Config::load_or_init()?;
    Ok(Json(ListChannelsResponse {
        channels: config.channels,
    }))
}

pub(super) async fn create(Json(channel_config): Json<ChannelConfig>) -> BabataResult<()> {
    channel_config.validate()?;

    let mut config = Config::load()?;
    let channel_name = channel_config.name().to_string();
    if config
        .channels
        .iter()
        .any(|channel| channel.matches_name(&channel_name))
    {
        return Err(BabataError::invalid_input(format!(
            "Channel '{}' already exists",
            channel_name
        )));
    }

    config.channels.push(channel_config);
    config.validate()?;
    config.save()?;
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

    let mut config = Config::load()?;
    if !config
        .channels
        .iter()
        .any(|channel| channel.matches_name(&name))
    {
        return Err(BabataError::not_found(format!(
            "Channel '{}' not found",
            name
        )));
    }

    config.upsert_channel(channel_config);
    config.validate()?;
    config.save()?;
    Ok(())
}

pub(super) async fn delete(Path(name): Path<String>) -> BabataResult<()> {
    let mut config = Config::load()?;
    let index = config
        .channels
        .iter()
        .position(|channel| channel.matches_name(&name))
        .ok_or_else(|| BabataError::not_found(format!("Channel '{}' not found", name)))?;

    config.channels.remove(index);
    config.validate()?;
    config.save()?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub(crate) struct ListChannelsResponse {
    pub channels: Vec<ChannelConfig>,
}
