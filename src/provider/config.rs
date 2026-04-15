use std::{collections::HashSet, path::Path};

use serde::{Deserialize, Serialize};

use crate::{
    BabataResult,
    error::BabataError,
    utils::{provider_dir, providers_dir},
};

const PROVIDER_CONFIG_FILE_NAME: &str = "config.json";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ProviderConfig {
    pub name: String,
    pub api_key: String,
    pub base_url: String,
    pub compatible_api: CompatibleApi,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CompatibleApi {
    Openai,
    Anthropic,
}

impl ProviderConfig {
    pub fn validate(&self) -> BabataResult<()> {
        validate_provider_name(&self.name)?;

        if self.api_key.trim().is_empty() {
            return Err(BabataError::config(
                "Provider api_key cannot be empty or whitespace",
            ));
        }

        if self.base_url.trim().is_empty() {
            return Err(BabataError::config(
                "Provider base_url cannot be empty or whitespace",
            ));
        }

        Ok(())
    }

    pub fn matches_name(&self, name: &str) -> bool {
        self.name.eq_ignore_ascii_case(name)
    }

    pub fn load(name: &str) -> BabataResult<Self> {
        validate_provider_name(name)?;
        let config_path = provider_dir(name)?.join(PROVIDER_CONFIG_FILE_NAME);
        load_from_path(&config_path)
    }

    pub fn load_all() -> BabataResult<Vec<Self>> {
        let providers_dir = providers_dir()?;
        if !providers_dir.exists() {
            return Ok(Vec::new());
        }

        if !providers_dir.is_dir() {
            return Err(BabataError::config(format!(
                "Providers path '{}' is not a directory",
                providers_dir.display()
            )));
        }

        let entries = std::fs::read_dir(&providers_dir)?;

        let mut providers = Vec::new();
        let mut provider_names = HashSet::new();
        for entry in entries {
            let entry = entry.map_err(|err| {
                BabataError::config(format!(
                    "Failed to read providers directory entry in '{}': {}",
                    providers_dir.display(),
                    err
                ))
            })?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let Some(provider_name) = path.file_name().and_then(|name| name.to_str()) else {
                return Err(BabataError::config(format!(
                    "Provider directory '{}' is not valid UTF-8",
                    path.display()
                )));
            };

            let provider = Self::load(provider_name)?;
            let normalized_name = provider.name.to_ascii_lowercase();
            if !provider_names.insert(normalized_name) {
                return Err(BabataError::config(format!(
                    "Duplicate provider name '{}' found in providers directory",
                    provider.name
                )));
            }
            providers.push(provider);
        }

        providers.sort_by_cached_key(|provider| provider.name.to_ascii_lowercase());
        Ok(providers)
    }

    pub fn save(&self) -> BabataResult<()> {
        self.validate()?;

        let provider_dir = provider_dir(&self.name)?;
        std::fs::create_dir_all(&provider_dir)?;

        let payload = serde_json::to_string_pretty(self)
            .map_err(|err| BabataError::config(format!("Failed to serialize provider: {}", err)))?;
        let config_path = provider_dir.join(PROVIDER_CONFIG_FILE_NAME);
        std::fs::write(&config_path, payload)?;

        Ok(())
    }

    pub fn delete(name: &str) -> BabataResult<()> {
        validate_provider_name(name)?;
        let provider_dir = provider_dir(name)?;
        if !provider_dir.exists() {
            return Err(BabataError::not_found(format!(
                "Provider '{}' not found",
                name
            )));
        }

        std::fs::remove_dir_all(&provider_dir)?;

        Ok(())
    }
}

fn load_from_path(config_path: &Path) -> BabataResult<ProviderConfig> {
    let raw = std::fs::read_to_string(config_path)?;
    let provider_config = serde_json::from_str::<ProviderConfig>(&raw).map_err(|err| {
        BabataError::config(format!(
            "Failed to parse provider config file '{}': {}",
            config_path.display(),
            err
        ))
    })?;
    provider_config.validate()?;
    Ok(provider_config)
}

fn validate_provider_name(name: &str) -> BabataResult<()> {
    if name.is_empty() {
        return Err(BabataError::config("Provider name cannot be empty"));
    }
    if name.trim() != name {
        return Err(BabataError::config(
            "Provider name cannot have leading or trailing whitespace",
        ));
    }
    if name == "." || name == ".." || name.contains('/') || name.contains('\\') {
        return Err(BabataError::config(
            "Provider name cannot contain path separators or reserved relative segments",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_provider_rejects_empty_base_url() {
        let config = ProviderConfig {
            name: "custom".to_string(),
            api_key: "test-key".to_string(),
            base_url: "   ".to_string(),
            compatible_api: CompatibleApi::Openai,
        };

        let result = config.validate();
        assert!(result.is_err());
        let err = result.expect_err("expected base_url validation error");
        assert!(err.to_string().contains("base_url"));
    }

    #[test]
    fn validate_provider_rejects_invalid_name() {
        let config = ProviderConfig {
            name: "../openai".to_string(),
            api_key: "test-key".to_string(),
            base_url: "https://example.com/v1".to_string(),
            compatible_api: CompatibleApi::Openai,
        };

        let result = config.validate();
        assert!(result.is_err());
        let err = result.expect_err("expected provider name validation error");
        assert!(err.to_string().contains("path separators"));
    }

    #[test]
    fn parse_provider_config_from_json() {
        let payload = r#"{
            "name": "custom-dev",
            "api_key": "test-key",
            "base_url": "https://example.com/v1",
            "compatible_api": "openai"
        }"#;
        let parsed: ProviderConfig = serde_json::from_str(payload).expect("parse provider json");

        assert_eq!(parsed.name, "custom-dev");
        assert_eq!(parsed.api_key, "test-key");
        assert_eq!(parsed.base_url, "https://example.com/v1");
        assert_eq!(parsed.compatible_api, CompatibleApi::Openai);
    }
}
