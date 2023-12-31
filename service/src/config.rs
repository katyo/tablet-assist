use std::path::Path;
use serde::Deserialize;
use crate::Result;

/// Service configuration
#[derive(Deserialize)]
pub struct Config {
    pub udev: Vec<UdevConfig>,
    pub device: Vec<DeviceConfig>,
}

impl Default for Config {
    fn default() -> Self {
        let udev = Default::default();
        let device = Default::default();
        let mut cfg = Self { udev, device };
        cfg.validate();
        cfg
    }
}

impl Config {
    /// Read config from file
    pub async fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let raw = smol::fs::read(path).await?;
        let txt = core::str::from_utf8(&raw)?;
        let mut cfg: Self = toml::from_str(txt)?;
        cfg.validate();
        Ok(cfg)
    }

    fn validate(&mut self) {
        if self.udev.is_empty() {
            self.udev.push(UdevConfig::default());
        }
    }
}

/// Service configuration
#[derive(serde::Deserialize)]
pub struct UdevConfig {
    #[serde(default = "UdevConfig::default_seat")]
    pub seat: String,
}

impl Default for UdevConfig {
    fn default() -> Self {
        Self { seat: Self::default_seat() }
    }
}

impl UdevConfig {
    fn default_seat() -> String {
        "seat0".into()
    }
}

/// Service configuration
#[derive(serde::Deserialize)]
pub struct DeviceConfig {
    pub name: Option<String>,
    pub vid: Option<u32>,
    pub pid: Option<u32>,
    #[serde(default = "default_device_enable")]
    pub enable: bool,
}

fn default_device_enable() -> bool {
    true
}
