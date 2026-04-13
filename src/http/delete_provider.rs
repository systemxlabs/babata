use axum::extract::Path;

use crate::{BabataResult, config::Config, error::BabataError};

pub(super) async fn handle(Path(name): Path<String>) -> BabataResult<()> {
    let mut config = Config::load_or_init()?;

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
