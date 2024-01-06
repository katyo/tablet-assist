use zbus::dbus_proxy;

mod types;

pub use types::*;

/// Tablet-mode watch service
#[dbus_proxy(
    interface = "tablet.assist.Agent1",
    default_service = "tablet.assist.Agent",
    default_path = "/tablet/assist"
)]
pub trait Agent {
    /// Whether tablet-mode detection available
    #[dbus_proxy(property)]
    fn tablet_mode_detection(&self) -> zbus::fdo::Result<bool>;

    /// Current tablet-mode state
    #[dbus_proxy(property)]
    fn tablet_mode(&self) -> zbus::fdo::Result<bool>;

    /// Manual tablet-mode switch
    #[dbus_proxy(property)]
    fn set_tablet_mode(&self, enable: bool) -> zbus::fdo::Result<()>;

    /// Auto tablet-mode switch
    #[dbus_proxy(property)]
    fn auto_tablet_mode(&self) -> zbus::fdo::Result<bool>;

    /// Auto tablet-mode switch
    #[dbus_proxy(property)]
    fn set_auto_tablet_mode(&self, enable: bool) -> zbus::fdo::Result<()>;

    /// Get available input devices
    #[dbus_proxy(property)]
    fn input_devices(&self) -> zbus::fdo::Result<Vec<InputDeviceInfo>>;

    /// Get input device config
    fn input_device_config(&self, device: &InputDeviceInfo)
        -> zbus::fdo::Result<InputDeviceConfig>;

    /// Set input device config
    fn set_input_device_config(
        &self,
        device: &InputDeviceInfo,
        config: &InputDeviceConfig,
    ) -> zbus::fdo::Result<()>;

    /// Whether orientation detection available
    #[dbus_proxy(property)]
    fn orientation_detection(&self) -> zbus::fdo::Result<bool>;

    /// Current orientation
    #[dbus_proxy(property)]
    fn orientation(&self) -> zbus::fdo::Result<Orientation>;

    /// Manual orientation change
    #[dbus_proxy(property)]
    fn set_orientation(&self, orientation: Orientation) -> zbus::fdo::Result<()>;

    /// Auto orientation change
    #[dbus_proxy(property)]
    fn auto_orientation(&self) -> zbus::fdo::Result<bool>;

    /// Auto orientation change
    #[dbus_proxy(property)]
    fn set_auto_orientation(&self, enable: bool) -> zbus::fdo::Result<()>;
}
