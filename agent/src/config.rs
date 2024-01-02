use crate::{DeviceAction, DeviceId, Orientation, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

/// Agent config
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// Tablet mode config
    pub tablet_mode: TabletModeConfig,
    /// Orientation config
    pub orientation: OrientationConfig,
}

/// Tablet mode config
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TabletModeConfig {
    /// Switch to tablet mode and back using auto-detection
    pub auto: bool,
    /// Tablet mode for manual setting
    pub manual: bool,
    /// Device configs for tablet mode
    pub device: HashMap<DeviceId, DeviceConfig>,
    /// Show cursor in tablet mode
    pub cursor: bool,
}

/// Device config
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DeviceConfig {
    /// Device action
    pub action: DeviceAction,
}

/// Orientation config
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrientationConfig {
    /// Set orientation using auto-detection
    pub auto: bool,
    /// Orientation for manual setting
    pub manual: Orientation,
}

impl DeviceConfig {
    pub const DEFAULT: Self = Self {
        action: DeviceAction::Skip,
    };
}

impl TabletModeConfig {
    pub fn get_device(&self, id: &DeviceId) -> &DeviceConfig {
        self.device.get(id).unwrap_or(&DeviceConfig::DEFAULT)
    }

    pub fn set_device(&mut self, id: &DeviceId, config: DeviceConfig) {
        if config != DeviceConfig::DEFAULT {
            self.device.insert(id.clone(), config);
        } else {
            self.device.remove(id);
        }
    }
}

impl Config {
    /// Read config from file
    pub async fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let raw = smol::fs::read(path).await?;
        let txt = core::str::from_utf8(&raw)?;
        let cfg = toml::from_str(txt)?;
        Ok(cfg)
    }

    /// Write config into file
    pub async fn into_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(dir) = path.parent() {
            if !dir.is_dir() {
                smol::fs::create_dir_all(dir).await?;
            }
        }
        let raw = toml::to_string_pretty(self)?;
        smol::fs::write(path, raw).await?;
        Ok(())
    }

    fn path() -> PathBuf {
        let prefix = dirs::config_dir().unwrap();
        prefix.join("tablet-assist").join("config.toml")
    }

    /// Load config from user directory if exists
    pub async fn load() -> Result<Option<Self>> {
        let path = Self::path();
        if path.is_file() {
            Self::from_file(path).await.map(Some)
        } else {
            Ok(None)
        }
    }

    /// Save config into user directory
    pub async fn save(&self) -> Result<()> {
        let path = Self::path();
        self.into_file(path).await
    }
}
