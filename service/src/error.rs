/// Result type
pub type Result<T> = core::result::Result<T, Error>;

/// Error type
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// I/O error
    #[error("Io error: {0}")]
    Io(#[from] std::io::Error),
    /// DBus error
    #[error("DBus error: {0}")]
    DBus(#[from] zbus::Error),
    /// DBus internal error
    #[error("DBus error: {0}")]
    DBusFdo(#[from] zbus::fdo::Error),
    /// Tracing set error
    #[error("Tracing error: {0}")]
    Tracing(#[from] tracing::subscriber::SetGlobalDefaultError),
    /// UTF-8 error
    #[error("UTF8 error: {0}")]
    Utf8(#[from] core::str::Utf8Error),
    /// TOML parsing error
    #[error("TOML deserialize error: {0}")]
    TomlDe(#[from] toml::de::Error),
    /// TOML formatting error
    #[error("TOML serialize error: {0}")]
    TomlSer(#[from] toml::ser::Error),
    /// Termination error
    #[error("Unexpected termination")]
    Term,
    #[cfg(feature = "input")]
    /// Add seat error
    #[error("Add seat error: {0}")]
    AddSeat(String),
    #[cfg(feature = "input")]
    /// Add path error
    #[error("Add path error: {0}")]
    AddPath(String),
    #[cfg(feature = "iio")]
    /// Polling error
    #[error("Unable to poll sensor: {0}")]
    Poll(String),
}

/*
impl From<Error> for zbus::Error {
    fn from(error: Error) -> Self {

    }
}
*/

impl AsRef<str> for Error {
    fn as_ref(&self) -> &str {
        match self {
            Self::Io(_) => "io",
            Self::DBus(_) => "dbus",
            Self::DBusFdo(_) => "dbus-fdo",
            Self::Tracing(_) => "tracing",
            Self::Utf8(_) => "utf8",
            Self::TomlDe(_) => "toml-de",
            Self::TomlSer(_) => "toml-ser",
            Self::Term => "term",
            #[cfg(feature = "input")]
            Self::AddSeat(_) => "input-add-seat",
            #[cfg(feature = "input")]
            Self::AddPath(_) => "input-add-path",
            #[cfg(feature = "iio")]
            Self::Poll(_) => "iio-poll",
        }
    }
}

impl zbus::DBusError for Error {
    fn create_reply(&self, msg: &zbus::MessageHeader<'_>) -> zbus::Result<zbus::Message> {
        zbus::MessageBuilder::error(msg, self.name())?.build(&self.to_string())
    }

    fn name(&self) -> zbus::names::ErrorName<'_> {
        zbus::names::ErrorName::from_str_unchecked(self.as_ref())
    }

    fn description(&self) -> Option<&str> {
        Some(self.as_ref())
    }
}
