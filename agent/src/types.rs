use serde::{Deserialize, Serialize};
pub use tablet_assist_service::Orientation;
use zbus::zvariant::{OwnedValue, Type, Value};

macro_rules! enum_types {
    ($( $(#[$($tmeta:meta)*])* $type:ident { $( $(#[$($vmeta:meta)*])* $var:ident, )* } )*) => {
        $(
            $(#[$($tmeta)*])*
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Type, Value, OwnedValue, Serialize, Deserialize)]
            #[serde(rename_all = "kebab-case")]
            #[zvariant(signature = "s", rename_all = "snake_case")]
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
                            stringify!($var) => Self::$var,
                        )*
                            _ => return Err(()),
                    })
                }
            }

            impl AsRef<str> for $type {
                fn as_ref(&self) -> &str {
                    match self {
                        $(
                            Self::$var => stringify!($var),
                        )*
                    }
                }
            }

            impl core::fmt::Display for $type {
                fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                    self.as_ref().fmt(f)
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
        Keyboard,
        Mouse,
        Tablet,
        TouchScreen,
        TouchPad,
        BarCode,
        ButtonBox,
        KnobBox,
        OneKnob,
        NineKnob,
        TrackBall,
        Quadrature,
        IdModule,
        SpaceBall,
        DataGlove,
        EyeTracker,
        CursorKeys,
        FootMouse,
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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type, Value, OwnedValue)]
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
