use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use thiserror::Error;

type StringMap = HashMap<String, String>;
type IconMap = HashMap<String, String>;
type OptionMap = HashMap<String, bool>;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    IoError(#[from] io::Error),
    #[error("Failed to parse TOML: {0}")]
    TomlError(#[from] toml::de::Error),
}

/// Represents aliases for different categories
#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Aliases {
    pub class: StringMap,
    pub instance: StringMap,
    pub name: StringMap,
    pub app_id: StringMap,
}

impl Aliases {
    /// Creates a new empty Aliases instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets an alias by category and key
    pub fn get_alias(&self, category: &str, key: &str) -> Option<&String> {
        match category {
            "app_id" => self.app_id.get(key),
            "class" => self.class.get(key),
            "instance" => self.instance.get(key),
            "name" => self.name.get(key),
            _ => None,
        }
    }
}

impl Default for Aliases {
    fn default() -> Self {
        Self {
            class: StringMap::new(),
            instance: StringMap::new(),
            name: StringMap::new(),
            app_id: StringMap::new(),
        }
    }
}

/// Main configuration structure
#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Config {
    pub icons: IconMap,
    pub aliases: Aliases,
    pub general: StringMap,
    pub options: OptionMap,
}

impl Config {
    /// Creates a new Config instance from a file
    pub fn new(filename: &Path) -> Result<Self, ConfigError> {
        let config = Self::from_file(filename)?;
        Ok(config)
    }

    /// Loads configuration from a TOML file
    pub fn from_file(filename: &Path) -> Result<Self, ConfigError> {
        let mut file = File::open(filename)?;
        let mut buffer = String::new();
        file.read_to_string(&mut buffer)?;
        let config: Config = toml::from_str(&buffer)?;
        Ok(config)
    }

    /// Gets a general configuration value
    pub fn get_general(&self, key: &str) -> Option<String> {
        self.general.get(key).map(|s| s.to_string())
    }

    /// Gets an option value
    pub fn get_option(&self, key: &str) -> Option<bool> {
        self.options.get(key).copied()
    }

    /// Gets an icon by key
    pub fn get_icon(&self, key: &str) -> Option<String> {
        self.icons.get(key).map(|s| s.to_string())
    }

    /// Sets a general configuration value
    pub fn set_general(&mut self, key: String, value: String) {
        self.general.insert(key, value);
    }

    /// Sets a an option configuration value
    pub fn set_option(&mut self, key: String, value: bool) {
        self.options.insert(key, value);
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            icons: IconMap::new(),
            aliases: Aliases::default(),
            general: StringMap::new(),
            options: OptionMap::new(),
        }
    }
}
