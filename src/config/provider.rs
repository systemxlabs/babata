use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    BabataResult,
    error::BabataError,
    utils::{babata_dir, provider_dir},
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
        let validated_name = validate_provider_name(name)?;
        let config_path = provider_dir(validated_name)?.join(PROVIDER_CONFIG_FILE_NAME);
        load_from_path(&config_path, validated_name)
    }

    pub fn load_all() -> BabataResult<Vec<Self>> {
        Self::load_all_from_root(&providers_root()?)
    }

    pub fn save(&self) -> BabataResult<()> {
        self.save_to_root(&providers_root()?)
    }

    pub fn delete(name: &str) -> BabataResult<()> {
        let validated_name = validate_provider_name(name)?;
        let provider_dir = provider_dir(validated_name)?;
        if !provider_dir.exists() {
            return Err(BabataError::not_found(format!(
                "Provider '{}' not found",
                validated_name
            )));
        }

        std::fs::remove_dir_all(&provider_dir).map_err(|err| {
            BabataError::config(format!(
                "Failed to delete provider directory '{}': {}",
                provider_dir.display(),
                err
            ))
        })?;

        Ok(())
    }

    pub(crate) fn load_from_root(root: &Path, name: &str) -> BabataResult<Self> {
        let config_path = provider_config_path(root, name)?;
        load_from_path(&config_path, name)
    }

    pub(crate) fn load_all_from_root(root: &Path) -> BabataResult<Vec<Self>> {
        if !root.exists() {
            return Ok(Vec::new());
        }

        if !root.is_dir() {
            return Err(BabataError::config(format!(
                "Providers path '{}' is not a directory",
                root.display()
            )));
        }

        let entries = std::fs::read_dir(root).map_err(|err| {
            BabataError::config(format!(
                "Failed to read providers directory '{}': {}",
                root.display(),
                err
            ))
        })?;

        let mut providers = Vec::new();
        let mut provider_names = HashSet::new();
        for entry in entries {
            let entry = entry.map_err(|err| {
                BabataError::config(format!(
                    "Failed to read providers directory entry in '{}': {}",
                    root.display(),
                    err
                ))
            })?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let Some(provider_dir_name) = path.file_name().and_then(|name| name.to_str()) else {
                return Err(BabataError::config(format!(
                    "Provider directory '{}' is not valid UTF-8",
                    path.display()
                )));
            };

            let provider = Self::load_from_root(root, provider_dir_name)?;
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

    pub(crate) fn save_to_root(&self, root: &Path) -> BabataResult<()> {
        self.validate()?;

        let provider_dir = root.join(validate_provider_name(&self.name)?);
        std::fs::create_dir_all(&provider_dir).map_err(|err| {
            BabataError::config(format!(
                "Failed to create provider directory '{}': {}",
                provider_dir.display(),
                err
            ))
        })?;

        let payload = serde_json::to_string_pretty(self)
            .map_err(|err| BabataError::config(format!("Failed to serialize provider: {}", err)))?;
        let config_path = provider_dir.join(PROVIDER_CONFIG_FILE_NAME);
        std::fs::write(&config_path, payload).map_err(|err| {
            BabataError::config(format!(
                "Failed to write provider config file '{}': {}",
                config_path.display(),
                err
            ))
        })?;

        Ok(())
    }
}

fn providers_root() -> BabataResult<PathBuf> {
    Ok(babata_dir()?.join("providers"))
}

fn load_from_path(config_path: &Path, name: &str) -> BabataResult<ProviderConfig> {
    let raw = std::fs::read_to_string(config_path).map_err(|err| {
        BabataError::config(format!(
            "Failed to read provider config file '{}': {}",
            config_path.display(),
            err
        ))
    })?;
    let provider_config = serde_json::from_str::<ProviderConfig>(&raw).map_err(|err| {
        BabataError::config(format!(
            "Failed to parse provider config file '{}': {}",
            config_path.display(),
            err
        ))
    })?;
    provider_config.validate()?;
    if !provider_config.matches_name(name) {
        return Err(BabataError::config(format!(
            "Provider config file '{}' does not match provider directory '{}'",
            config_path.display(),
            name
        )));
    }
    Ok(provider_config)
}

fn provider_config_path(root: &Path, name: &str) -> BabataResult<PathBuf> {
    Ok(root
        .join(validate_provider_name(name)?)
        .join(PROVIDER_CONFIG_FILE_NAME))
}

fn validate_provider_name(name: &str) -> BabataResult<&str> {
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
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("babata-{prefix}-{timestamp}"))
    }

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

    #[test]
    fn save_and_load_provider_configs_from_directory() {
        let root = unique_temp_dir("provider-save-load");
        std::fs::create_dir_all(&root).expect("create provider root");

        let openai = ProviderConfig {
            name: "openai-main".to_string(),
            api_key: "test-openai-key".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            compatible_api: CompatibleApi::Openai,
        };
        let gateway = ProviderConfig {
            name: "gateway".to_string(),
            api_key: "test-custom-key".to_string(),
            base_url: "https://gateway.example.com/v1".to_string(),
            compatible_api: CompatibleApi::Openai,
        };

        openai.save_to_root(&root).expect("save openai provider");
        gateway.save_to_root(&root).expect("save gateway provider");

        let loaded = ProviderConfig::load_all_from_root(&root).expect("load all provider configs");
        assert_eq!(loaded, vec![gateway, openai]);

        std::fs::remove_dir_all(root).expect("cleanup provider root");
    }

    #[test]
    fn load_all_rejects_provider_directory_name_mismatch() {
        let root = unique_temp_dir("provider-dir-mismatch");
        std::fs::create_dir_all(&root).expect("create provider root");

        let provider_dir = root.join("gateway");
        std::fs::create_dir_all(&provider_dir).expect("create provider directory");
        let payload = serde_json::to_string_pretty(&ProviderConfig {
            name: "openai-main".to_string(),
            api_key: "key-1".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            compatible_api: CompatibleApi::Openai,
        })
        .expect("serialize provider config");
        std::fs::write(provider_dir.join(PROVIDER_CONFIG_FILE_NAME), payload)
            .expect("write provider config");

        let result = ProviderConfig::load_all_from_root(&root);
        assert!(result.is_err());
        let err = result.expect_err("expected provider directory mismatch error");
        assert!(
            err.to_string()
                .contains("does not match provider directory")
        );

        std::fs::remove_dir_all(root).expect("cleanup provider root");
    }
}
