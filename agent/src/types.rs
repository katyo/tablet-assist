use serde::{Deserialize, Serialize};
pub use tablet_assist_service::Orientation;
use zbus::zvariant::{Type, Value, OwnedValue};

macro_rules! enum_types {
    ($( $(#[$($tmeta:meta)*])* $type:ident { $( $(#[$($vmeta:meta)*])* $var:ident = $val:literal, )* } )*) => {
        $(
            $(#[$($tmeta)*])*
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Type, Serialize, Deserialize)]
            #[serde(rename_all = "kebab-case")]
            #[zvariant(signature = "s")]
            #[repr(u8)]
            pub enum $type {
                $(
                    $(#[$($vmeta)*])*
                    $var,
                )*
            }

            impl $type {
                pub const ALL: [Self; enum_types!(@count $($var)*)] = [
                    $(
                        Self::$var,
                    )*
                ];
            }

            impl core::str::FromStr for $type {
                type Err = ();
                fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
                    Ok(match s {
                        $(
                            $val => Self::$var,
                        )*
                            _ => return Err(()),
                    })
                }
            }

            impl AsRef<str> for $type {
                fn as_ref(&self) -> &str {
                    match self {
                        $(
                            Self::$var => $val,
                        )*
                    }
                }
            }

            impl core::fmt::Display for $type {
                fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                    self.as_ref().fmt(f)
                }
            }

            impl TryFrom<zbus::zvariant::Value<'_>> for $type {
                type Error = zbus::zvariant::Error;

                #[inline]
                fn try_from(value: zbus::zvariant::Value<'_>) -> zbus::zvariant::Result<Self> {
                    <&str>::try_from(&value)?.parse().map_err(|_| zbus::zvariant::Error::IncorrectType)
                }
            }

            impl From<$type> for zbus::zvariant::Value<'_> {
                #[inline]
                fn from(e: $type) -> Self {
                    <zbus::zvariant::Value as From<_>>::from(e.to_string())
                }
            }

            impl TryFrom<zbus::zvariant::OwnedValue> for $type {
                type Error = zbus::zvariant::Error;

                #[inline]
                fn try_from(value: zbus::zvariant::OwnedValue) -> zbus::zvariant::Result<Self> {
                    <&str>::try_from(&value)?.parse().map_err(|_| zbus::zvariant::Error::IncorrectType)
                }
            }

            impl From<$type> for zbus::zvariant::OwnedValue {
                #[inline]
                fn from(e: $type) -> Self {
                    <zbus::zvariant::Value as From<_>>::from(e.to_string()).into()
                }
            }
        )*
    };

    (@count $id:ident $($ids:ident)* ) => {
        1 + enum_types!(@count $($ids)*)
    };

    (@count ) => {
        0
    };
}

enum_types! {
    /// Input device type
    InputDeviceType {
        Keyboard = "keyboard",
        Mouse = "mouse",
        Tablet = "tablet",
        TouchScreen = "touchscreen",
        TouchPad = "touchpad",
        BarCode = "barcode",
        ButtonBox = "buttonbox",
        KnobBox = "knob-box",
        OneKnob = "one-knob",
        NineKnob = "nine-knob",
        TrackBall = "trackball",
        Quadrature = "quadrature",
        IdModule = "in-module",
        SpaceBall = "spaceball",
        DataGlove = "dataglove",
        EyeTracker = "eyetracker",
        CursorKeys = "cursorkeys",
        FootMouse = "footmouse",
    }
}

impl InputDeviceType {
    pub fn xi_name(&self) -> &str {
        use InputDeviceType::*;
        match self {
            Keyboard => "KEYBOARD",
            Mouse => "MOUSE",
            Tablet => "TABLET",
            TouchScreen => "TOUCHSCREEN",
            TouchPad => "TOUCHPAD",
            BarCode => "BARCODE",
            ButtonBox => "BUTTONBOX",
            KnobBox => "KNOB_BOX",
            OneKnob => "ONE_KNOB",
            NineKnob => "NINE_KNOB",
            TrackBall => "TRACKBALL",
            Quadrature => "QUADRATURE",
            IdModule => "ID_MODULE",
            SpaceBall => "SPACEBALL",
            DataGlove => "DATAGLOVE",
            EyeTracker => "EYETRACKER",
            CursorKeys => "CURSORKEYS",
            FootMouse => "FOOTMOUSE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Type, Value, OwnedValue)]
pub struct InputDeviceInfo {
    pub id: u32,
    #[zvariant(rename = "type")]
    pub type_: InputDeviceType,
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

/// Device config
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, Type, Value, OwnedValue)]
pub struct InputDeviceConfig {
    /// Enable in tablet mode
    pub tablet: bool,
    /// Enable in laptop mode
    pub laptop: bool,
    /// Rotate with screen
    pub rotate: bool,
}

impl InputDeviceConfig {
    pub const DEFAULT: Self = Self {
        tablet: true,
        laptop: true,
        rotate: false,
    };
}
