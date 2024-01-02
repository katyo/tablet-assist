use serde::{Deserialize, Serialize};
use zbus::zvariant::{OwnedValue, Type, Value};

#[derive(
    Debug, Clone, Copy, Default, Type, Value, OwnedValue, PartialEq, Serialize, Deserialize,
)]
#[zvariant(signature = "s")]
#[serde(rename_all = "kebab-case")]
#[repr(u8)]
pub enum Orientation {
    #[default]
    TopUp = 0,
    LeftUp = 1,
    RightUp = 2,
    BottomUp = 3,
}

impl From<Orientation> for u8 {
    fn from(orientation: Orientation) -> Self {
        orientation as _
    }
}

impl TryFrom<u8> for Orientation {
    type Error = u8;
    fn try_from(raw: u8) -> core::result::Result<Self, Self::Error> {
        if raw >= Self::TopUp as _ && raw <= Self::BottomUp as _ {
            Ok(unsafe { *(&raw as *const _ as *const _) })
        } else {
            Err(raw)
        }
    }
}
