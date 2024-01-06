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
    /// Not found error
    #[error("Resource not found")]
    NotFound,
    /// X connect error
    #[error("X connect error: {0}")]
    XClient(#[from] crate::XError),
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(error: std::string::FromUtf8Error) -> Self {
        Error::Utf8(error.utf8_error())
    }
}

impl From<Error> for zbus::fdo::Error {
    fn from(error: Error) -> Self {
        use zbus::fdo::Error::*;
        match error {
            Error::Io(e) => IOError(e.to_string()),
            Error::DBus(e) => ZBus(e),
            Error::DBusFdo(e) => e,
            Error::Utf8(e) => Failed(e.to_string()),
            Error::TomlDe(e) => Failed(e.to_string()),
            Error::TomlSer(e) => Failed(e.to_string()),
            Error::Term => Failed("terminated".to_string()),
            Error::NotFound => Failed("not found".to_string()),
            Error::XClient(e) => Failed(format!("XClient: {e}")),
        }
    }
}

impl From<Error> for zbus::Error {
    fn from(error: Error) -> Self {
        use zbus::Error::*;
        match error {
            Error::Io(e) => InputOutput(std::sync::Arc::new(e)),
            Error::DBus(e) => e,
            Error::DBusFdo(e) => FDO(Box::new(e)),
            Error::Utf8(e) => Failure(e.to_string()),
            Error::TomlDe(e) => Failure(e.to_string()),
            Error::TomlSer(e) => Failure(e.to_string()),
            Error::Term => Failure("terminated".to_string()),
            Error::NotFound => Failure("not found".to_string()),
            Error::XClient(e) => Failure(format!("XClient: {e}")),
        }
    }
}

/*
impl AsRef<str> for Error {
    fn as_ref(&self) -> &str {
        match self {
            Self::Io(_) => "io",
            Self::DBus(_) => "dbus",
            Self::DBusFdo(_) => "dbus-fdo",
            Self::Utf8(_) => "utf8",
            Self::TomlDe(_) => "toml-de",
            Self::TomlSer(_) => "toml-ser",
            Self::Term => "term",
            Self::XConnect(_) => "x-connect",
            Self::XConnection(_) => "x-connection",
            Self::XReply(_) => "x-reply",
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
*/
