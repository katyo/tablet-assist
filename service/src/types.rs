use serde::{Deserialize, Serialize};
use zbus::zvariant::{Error, OwnedValue, Result, Type, Value};

#[derive(Debug, Clone, Copy, Default, Type, PartialEq, Serialize, Deserialize)]
#[zvariant(signature = "s")]
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

#[derive(Debug, Clone, Copy, Default, Type, PartialEq, Serialize, Deserialize)]
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

impl TryFrom<Value<'_>> for Orientation {
    type Error = Error;

    #[inline]
    fn try_from(value: Value<'_>) -> Result<Self> {
        let v = <&str>::try_from(&value)?;

        Ok(match v {
            "top-up" => Self::TopUp,
            "left-up" => Self::LeftUp,
            "right-up" => Self::RightUp,
            "bottom-up" => Self::BottomUp,
            _ => return Err(Error::IncorrectType),
        })
    }
}

impl From<Orientation> for Value<'_> {
    #[inline]
    fn from(e: Orientation) -> Self {
        let u: &str = match e {
            Orientation::TopUp => "top-up",
            Orientation::LeftUp => "left-up",
            Orientation::RightUp => "right-up",
            Orientation::BottomUp => "bottom-up",
        };

        <Value as From<_>>::from(u).into()
    }
}

impl TryFrom<OwnedValue> for Orientation {
    type Error = Error;

    #[inline]
    fn try_from(value: OwnedValue) -> Result<Self> {
        let v = <&str>::try_from(&value)?;

        Ok(match v {
            "top-up" => Self::TopUp,
            "left-up" => Self::LeftUp,
            "right-up" => Self::RightUp,
            "bottom-up" => Self::BottomUp,
            _ => return Err(Error::IncorrectType),
        })
    }
}

impl From<Orientation> for OwnedValue {
    #[inline]
    fn from(e: Orientation) -> Self {
        let u: &str = match e {
            Orientation::TopUp => "top-up",
            Orientation::LeftUp => "left-up",
            Orientation::RightUp => "right-up",
            Orientation::BottomUp => "bottom-up",
        };

        <Value as From<_>>::from(u).into()
    }
}
