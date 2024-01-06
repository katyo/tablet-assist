use serde::{Deserialize, Serialize};
use zbus::zvariant::{OwnedValue, Type, Value};

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Type, Value, OwnedValue, Serialize, Deserialize,
)]
#[zvariant(signature = "s", rename_all = "snake_case")]
#[serde(rename_all = "kebab-case")]
#[allow(clippy::enum_variant_names)]
#[repr(u8)]
pub enum Orientation {
    #[default]
    TopUp = 0,
    LeftUp = 1,
    RightUp = 2,
    BottomUp = 3,
}

impl Orientation {
    pub fn get_type(self) -> OrientationType {
        self.into()
    }
}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Type, Value, OwnedValue, Serialize, Deserialize,
)]
#[zvariant(signature = "s", rename_all = "snake_case")]
#[serde(rename_all = "kebab-case")]
#[repr(u8)]
pub enum OrientationType {
    #[default]
    Landscape = 0,
    Portrait = 1,
}

impl From<Orientation> for OrientationType {
    fn from(orientation: Orientation) -> Self {
        if matches!(orientation, Orientation::TopUp | Orientation::BottomUp) {
            Self::Landscape
        } else {
            Self::Portrait
        }
    }
}
