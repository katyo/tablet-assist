use serde::{Deserialize, Serialize};
pub use tablet_assist_service::Orientation;
use zbus::zvariant::{OwnedValue, Type, Value};

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Type, Value, OwnedValue)]
pub struct DeviceId {
    pub id: u32,
    pub name: String,
}

impl core::fmt::Display for DeviceId {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        self.id.fmt(f)?;
        ": ".fmt(f)?;
        self.name.fmt(f)
    }
}

impl core::str::FromStr for DeviceId {
    type Err = &'static str;
    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        if let Some((id, name)) = s.split_once(": ") {
            let id = id.parse().map_err(|_| "Invalid device id number")?;
            let name = name.into();
            Ok(Self { id, name })
        } else {
            Err("Invalid device id format")
        }
    }
}

impl Serialize for DeviceId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for DeviceId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct DeviceIdVisitor;

        impl<'de> serde::de::Visitor<'de> for DeviceIdVisitor {
            type Value = DeviceId;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                formatter.write_str("a string identifier prefixed by number")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                value.parse().map_err(|e| {
                    serde::de::Error::invalid_value(serde::de::Unexpected::Str(e), &self)
                })
            }
        }

        deserializer.deserialize_str(DeviceIdVisitor)
    }
}

/// Device config
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type, Value, OwnedValue)]
pub struct DeviceConfig {
    /// Enable in tablet mode
    pub tablet: bool,
    /// Enable in laptop mode
    pub laptop: bool,
    /// Rotate with screen
    pub rotate: bool,
}

impl DeviceConfig {
    pub const DEFAULT: Self = Self {
        tablet: true,
        laptop: true,
        rotate: false,
    };
}
