use crate::{
    BabataResult,
    config::{Config, EmbeddingConfig},
    error::BabataError,
};

use super::Args;

pub fn add(_args: &Args, embedding_config_json: &str) {
    if let Err(err) = run_add(embedding_config_json) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

pub fn delete(_args: &Args, name: &str) {
    if let Err(err) = run_delete(name) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

pub fn list(_args: &Args) {
    if let Err(err) = run_list() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run_add(embedding_config_json: &str) -> BabataResult<()> {
    let embedding_config: EmbeddingConfig =
        serde_json::from_str(embedding_config_json).map_err(|err| {
            BabataError::config(format!(
                "Invalid embedding config JSON '{}': {}",
                embedding_config_json, err
            ))
        })?;
    embedding_config.validate()?;

    let mut config = Config::load()?;
    let embedding_name = embedding_config.embedding_name().to_string();
    config.upsert_embedding(embedding_config);
    config.save()?;

    println!("Added/updated embedding '{}' in config", embedding_name);
    Ok(())
}

fn run_list() -> BabataResult<()> {
    let config = Config::load()?;

    for embedding_config in &config.embeddings {
        let payload = serde_json::to_string(embedding_config).map_err(|err| {
            BabataError::internal(format!(
                "Failed to serialize embedding '{}' config to JSON: {}",
                embedding_config.embedding_name(),
                err
            ))
        })?;
        println!("{payload}");
    }

    Ok(())
}

fn run_delete(name: &str) -> BabataResult<()> {
    let mut config = Config::load()?;

    let index = config
        .embeddings
        .iter()
        .position(|embedding| embedding.matches_name(name))
        .ok_or_else(|| BabataError::config(format!("Embedding '{}' not found in config", name)))?;

    let deleted = config.embeddings.remove(index);
    config.save()?;

    println!("Deleted embedding '{}'", deleted.embedding_name());
    Ok(())
}
