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
    #[cfg(feature = "industrial_io")]
    /// Industrial IO error
    #[error("IIO error: {0}")]
    Iio(#[from] industrial_io::errors::Error),
    /// UTF-8 error
    #[error("UTF8 error: {0}")]
    Utf8(#[from] core::str::Utf8Error),
    /// TOML error
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
    /// Termination error
    #[error("Unexpected termination")]
    Term,
    /// Add seat error
    #[error("Add seat error: {0}")]
    AddSeat(String),
    /// Add path error
    #[error("Add path error: {0}")]
    AddPath(String),
}
