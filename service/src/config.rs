use crate::Result;
use serde::Deserialize;
use std::path::Path;

/// Service configuration
#[derive(Deserialize)]
pub struct Config {
    pub udev: Vec<UdevConfig>,
    pub device: Vec<DeviceConfig>,
    pub orientation: OrientationConfig,
}

impl Default for Config {
    fn default() -> Self {
        let udev = Default::default();
        let device = Default::default();
        let orientation = Default::default();
        let mut cfg = Self {
            udev,
            device,
            orientation,
        };
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
        Self {
            seat: Self::default_seat(),
        }
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

/// Orientation detection options
#[derive(Deserialize)]
pub struct OrientationConfig {
    /// Plane XY angle tolerance in degrees
    pub max_xy_angle: f64,
    /// Plane Z angle tolerance in degrees
    pub max_z_angle: f64,
    /// Maximum allowed angular velocity in degrees per second
    pub max_velocity: f64,
    /// Maximum allowed angular acceleration in degrees per second^2
    pub max_acceleration: f64,
}

impl Default for OrientationConfig {
    fn default() -> Self {
        Self {
            max_xy_angle: 20.0,
            max_z_angle: 60.0,
            max_velocity: 5.0,
            max_acceleration: 3.0,
        }
    }
}

const DEG_TO_RAD: f64 = core::f64::consts::PI / 180.0;

impl OrientationConfig {
    pub fn to_radians(&self) -> Self {
        Self {
            max_xy_angle: self.max_xy_angle * DEG_TO_RAD,
            max_z_angle: self.max_z_angle * DEG_TO_RAD,
            max_velocity: self.max_velocity * DEG_TO_RAD,
            max_acceleration: self.max_acceleration * DEG_TO_RAD,
        }
    }

    pub fn check(
        &self,
        xy_angle: Option<f64>,
        z_angle: Option<f64>,
        velocity: Option<f64>,
        acceleration: Option<f64>,
    ) -> bool {
        xy_angle
            .map(|xy_angle| xy_angle.abs() <= self.max_xy_angle)
            .unwrap_or_default()
            && z_angle
                .map(|z_angle| z_angle.abs() <= self.max_z_angle)
                .unwrap_or_default()
            && velocity
                .map(|velocity| velocity.abs() <= self.max_velocity)
                .unwrap_or_default()
            && acceleration
                .map(|acceleration| acceleration.abs() <= self.max_acceleration)
                .unwrap_or_default()
    }
}
