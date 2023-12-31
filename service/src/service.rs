use std::sync::{Arc, RwLock};
use serde::{Serialize, Deserialize};
use zbus::{dbus_interface, InterfaceRef, zvariant::{Type, Value}};
use crate::Result;

/// Internal service state
struct State {
    tablet_mode: RwLock<Option<bool>>,
    orientation: RwLock<Option<Orientation>>,
    interface: RwLock<Option<InterfaceRef<Service>>>,
}

#[derive(Clone)]
pub struct Service {
    state: Arc<State>
}

#[derive(Debug, Clone, Copy, Default, Type, Value, PartialEq, Serialize, Deserialize)]
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

/// Tablet-mode watch service
#[dbus_interface(name = "tablet.assist.Service1")]
impl Service {
    /// Current tablet-mode state property
    #[dbus_interface(property)]
    async fn tablet_mode(&self) -> bool {
        self.state.tablet_mode.read().unwrap().unwrap_or_default()
    }

    /// Tablet-mode available property
    #[dbus_interface(property)]
    async fn tablet_mode_available(&self) -> bool {
        self.state.tablet_mode.read().unwrap().is_some()
    }

    /// Current screen orientation property
    #[dbus_interface(property)]
    async fn orientation(&self) -> Orientation {
        self.state.orientation.read().unwrap().unwrap_or_default()
    }

    /// Orientation available property
    #[dbus_interface(property)]
    async fn orientation_available(&self) -> bool {
        self.state.orientation.read().unwrap().is_some()
    }
}

impl Service {
    pub fn new() -> Result<Self> {
        Ok(Service {
            state: Arc::new(State {
                tablet_mode: RwLock::new(None),
                orientation: RwLock::new(None),
                interface: RwLock::new(None),
            })
        })
    }

    pub fn set_interface(&self, interface: InterfaceRef<Self>) {
        *self.state.interface.write().unwrap() = Some(interface);
    }

    pub async fn set_tablet_mode(&self, mode: bool) -> Result<()> {
        let avail = {
            let mut val = self.state.tablet_mode.write().unwrap();
            let avail = val.is_some();
            *val = Some(mode);
            avail
        };

        let iface = self.state.interface.read().unwrap();
        let sigctx = iface.as_ref().unwrap().signal_context();

        self.tablet_mode_changed(sigctx).await?;
        if !avail {
            self.tablet_mode_available_changed(sigctx).await?;
        }

        Ok(())
    }

    pub async fn set_orientation(&self, orientation: Orientation) -> Result<()> {
        let avail = {
            let mut val = self.state.orientation.write().unwrap();
            let avail = val.is_some();
            *val = Some(orientation);
            avail
        };

        let iface = self.state.interface.read().unwrap();
        let sigctx = iface.as_ref().unwrap().signal_context();

        self.orientation_changed(sigctx).await?;
        if !avail {
            self.orientation_available_changed(sigctx).await?;
        }

        Ok(())
    }
}
