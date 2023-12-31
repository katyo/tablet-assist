use serde::{Serialize, Deserialize};
use zvariant::{Type, Value, OwnedValue};

#[derive(Debug, Clone, Copy, Default, Type, Value, OwnedValue, PartialEq, Serialize, Deserialize)]
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

#[cfg(feature = "iio-sensor-proxy")]
#[derive(Debug, Clone, Copy, Default, Type, Value, OwnedValue, PartialEq, Serialize, Deserialize)]
#[zvariant(signature = "s")]
#[serde(rename_all = "kebab-case")]
#[repr(u8)]
pub enum OrientationIsp {
    #[default]
    Undefined = 0,
    Normal = 1,
    BottomUp = 2,
    LeftUp = 3,
    RightUp = 4,
}

#[cfg(feature = "iio-sensor-proxy")]
impl TryFrom<OrientationIsp> for Orientation {
    type Error = OrientationIsp;

    fn try_from(orientation: OrientationIsp) -> Result<Self, Self::Error> {
        Ok(match orientation {
            OrientationIsp::Normal => Orientation::TopUp,
            OrientationIsp::BottomUp => Orientation::BottomUp,
            OrientationIsp::LeftUp => Orientation::LeftUp,
            OrientationIsp::RightUp => Orientation::RightUp,
            asis => return Err(asis),
        })
    }
}

#[cfg(feature = "iio-sensor-proxy")]
#[derive(Debug, Clone, Copy, Default, Type, Value, OwnedValue, PartialEq, Serialize, Deserialize)]
#[zvariant(signature = "s")]
#[serde(rename_all = "kebab-case")]
#[repr(u8)]
pub enum LightLevelUnit {
    #[default]
    Lux = 0,
    Vendor = 1,
}
