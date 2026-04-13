use axum::{Json, extract::Path};

use crate::{
    BabataResult,
    config::{ChannelConfig, Config},
    error::BabataError,
};

pub(super) async fn handle(
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

    let mut config = Config::load_or_init()?;
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
