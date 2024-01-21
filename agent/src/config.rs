use crate::{InputDeviceInfo, Orientation, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

/// Agent config
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// Tablet mode config
    #[serde(default)]
    pub tablet_mode: TabletModeConfig,
    /// Orientation config
    #[serde(default)]
    pub orientation: OrientationConfig,
    /// Input devices configs
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub device: HashMap<InputDeviceInfo, InputDeviceConfig>,
}

impl Config {
    pub fn get_device(&self, id: &InputDeviceInfo) -> &InputDeviceConfig {
        self.device.get(id).unwrap_or(&InputDeviceConfig::DEFAULT)
    }

    pub fn set_device(&mut self, id: &InputDeviceInfo, config: InputDeviceConfig) {
        if config != InputDeviceConfig::DEFAULT {
            self.device.insert(id.clone(), config);
        } else {
            self.device.remove(id);
        }
    }

    pub fn with_device<T>(
        &mut self,
        id: &InputDeviceInfo,
        func: impl FnOnce(&mut InputDeviceConfig) -> T,
    ) -> T {
        let mut config = *self.get_device(id);
        let res = func(&mut config);
        self.set_device(id, config);
        res
    }
}

/// Tablet mode config
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TabletModeConfig {
    /// Switch to tablet mode and back using auto-detection
    #[serde(default = "yes")]
    pub auto: bool,
    /// Tablet mode for manual setting
    #[serde(default)]
    pub manual: bool,
    /// Show cursor in tablet mode
    #[serde(default)]
    pub cursor: bool,
}

/// Orientation config
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrientationConfig {
    /// Set orientation using auto-detection
    #[serde(default = "yes")]
    pub auto: bool,
    /// Orientation for manual setting
    #[serde(default)]
    pub manual: Orientation,
}

/// Device config
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct InputDeviceConfig {
    /// Enable in tablet mode
    #[serde(default = "yes")]
    pub tablet: bool,
    /// Enable in laptop mode
    #[serde(default = "yes")]
    pub laptop: bool,
    /// Rotate with screen
    #[serde(default)]
    pub rotate: bool,
}

impl InputDeviceConfig {
    pub const DEFAULT: Self = Self {
        tablet: true,
        laptop: true,
        rotate: false,
    };
}

fn yes() -> bool {
    true
}

/// Configuration holder
pub struct ConfigHolder<C> {
    path: PathBuf,
    config: C,
}

impl<C> core::ops::Deref for ConfigHolder<C> {
    type Target = C;
    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

impl<C> core::ops::DerefMut for ConfigHolder<C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.config
    }
}

impl<C> ConfigHolder<C> {
    /// Create config holder from path
    pub fn from_path(path: impl Into<PathBuf>) -> Self
    where
        C: Default,
    {
        let path = path.into();
        let config = C::default();
        Self { path, config }
    }

    /// Load config from user directory if exists
    pub async fn load(&mut self) -> Result<()>
    where
        C: for<'d> Deserialize<'d>,
    {
        if self.path.is_file() {
            self.config = Self::from_file(&self.path).await?;
        }
        Ok(())
    }

    /// Save config into user directory
    pub async fn save(&self) -> Result<()>
    where
        C: Serialize,
    {
        self.to_file(&self.path).await
    }

    /// Read config from file
    async fn from_file(path: impl AsRef<Path>) -> Result<C>
    where
        C: for<'d> Deserialize<'d>,
    {
        let raw = smol::fs::read(path).await?;
        let txt = core::str::from_utf8(&raw)?;
        let cfg = toml::from_str(txt)?;
        Ok(cfg)
    }

    /// Write config into file
    async fn to_file(&self, path: impl AsRef<Path>) -> Result<()>
    where
        C: Serialize,
    {
        let path = path.as_ref();
        if let Some(dir) = path.parent() {
            if !dir.is_dir() {
                smol::fs::create_dir_all(dir).await?;
            }
        }
        let raw = toml::to_string_pretty(&self.config)?;
        smol::fs::write(path, raw).await?;
        Ok(())
    }
}
