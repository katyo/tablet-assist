use serde::{Deserialize, Serialize};
use zbus::zvariant::Type;

macro_rules! enum_types {
    ($( $(#[$($tmeta:meta)*])* $type:ident { $( $(#[$($vmeta:meta)*])* $var:ident = $val:literal, )* } )*) => {
        $(
            $(#[$($tmeta)*])*
            #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Type, Serialize, Deserialize)]
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
    OrientationType {
        #[default]
        Landscape = "lansdcape",
        Portrait = "portrait",
    }

    Orientation {
        #[default]
        TopUp = "top-up",
        LeftUp = "left-up",
        RightUp = "right-up",
        BottomUp = "bottom-up",
    }
}

impl Orientation {
    pub fn get_type(self) -> OrientationType {
        self.into()
    }
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
