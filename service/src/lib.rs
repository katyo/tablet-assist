use zbus::dbus_proxy;

mod types;

pub use types::*;

/// Tablet-mode watch service
#[dbus_proxy(
    interface = "tablet.assist.Service1",
    default_service = "tablet.assist.Service",
    default_path = "/tablet/assist"
)]
pub trait Service {
    /// Current tablet-mode state
    #[dbus_proxy(property)]
    fn tablet_mode(&self) -> zbus::fdo::Result<bool>;

    /// Whether tablet-mode is available
    #[dbus_proxy(property)]
    fn has_tablet_mode(&self) -> zbus::fdo::Result<bool>;

    /// Current screen orientation
    #[dbus_proxy(property)]
    fn orientation(&self) -> zbus::fdo::Result<Orientation>;

    /// Whether orientation is available
    #[dbus_proxy(property)]
    fn has_orientation(&self) -> zbus::fdo::Result<bool>;

    /// Whether orientation polling is enabled
    #[dbus_proxy(property)]
    fn oritentation_poll(&self) -> zbus::fdo::Result<bool>;

    /// Enable/disable orientation polling
    #[dbus_proxy(property)]
    fn set_oritentation_poll(&self, enable: bool) -> zbus::fdo::Result<()>;
}
