use serde::{Deserialize, Serialize};
pub use tablet_assist_service::Orientation;
use zbus::zvariant::{OwnedValue, Type, Value};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Type, Value, OwnedValue)]
pub struct InputDeviceInfo {
    pub id: u32,
    #[zvariant(rename = "type")]
    pub type_: String,
    pub name: String,
}

impl core::fmt::Display for InputDeviceInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        self.id.fmt(f)?;
        ' '.fmt(f)?;
        self.type_.fmt(f)?;
        ' '.fmt(f)?;
        self.name.fmt(f)
    }
}

impl core::str::FromStr for InputDeviceInfo {
    type Err = &'static str;
    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        if let Some((id, type_, name)) = s
            .split_once(' ')
            .and_then(|(id, s)| s.split_once(' ').map(|(type_, name)| (id, type_, name)))
        {
            Ok(Self {
                id: id.parse().map_err(|_| "Invalid device info number")?,
                type_: type_.parse().map_err(|_| "Invalid device type")?,
                name: name.into(),
            })
        } else {
            Err("Invalid device info format")
        }
    }
}

impl Serialize for InputDeviceInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for InputDeviceInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct DeviceIdVisitor;

        impl<'de> serde::de::Visitor<'de> for DeviceIdVisitor {
            type Value = InputDeviceInfo;

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
