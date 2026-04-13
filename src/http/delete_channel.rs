use axum::extract::Path;

use crate::{BabataResult, config::Config, error::BabataError};

pub(super) async fn handle(Path(name): Path<String>) -> BabataResult<()> {
    let mut config = Config::load_or_init()?;

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
