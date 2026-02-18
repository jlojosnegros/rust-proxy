use std::fs;

use serde::Deserialize;
use tracing::{error, info, warn};

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    pub(crate) listen: Address,
    pub(crate) upstream: Address,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Address {
    pub(crate) address: String,
    pub(crate) port: u16,
}

impl Address {
    pub(crate) fn to_string(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen: Address {
                address: "127.0.0.1".to_string(),
                port: 8080,
            },
            upstream: Address {
                address: "127.0.0.1".to_string(),
                port: 9090,
            },
        }
    }
}

pub(crate) fn load_config() -> Config {
    match fs::read_to_string("config.yaml") {
        Ok(content) => match serde_yaml::from_str(&content) {
            Ok(config) => {
                info!("Loaded configuration from config.yaml");
                config
            }
            Err(e) => {
                error!(error= %e, "Failed to parse config.yaml. Using defaults");
                Config::default()
            }
        },
        Err(_) => {
            warn!("No config file found. Using defaults");
            Config::default()
        }
    }
}
