use axum::Json;

use crate::{
    BabataResult,
    config::{ChannelConfig, Config},
    error::BabataError,
};

pub(super) async fn handle(Json(channel_config): Json<ChannelConfig>) -> BabataResult<()> {
    channel_config.validate()?;

    let mut config = Config::load_or_init()?;
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
