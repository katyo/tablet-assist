use crate::{Orientation, Result};
use smol::lock::RwLock;
use std::sync::Arc;
use zbus::{dbus_interface, InterfaceRef};

/// Internal service state
struct State {
    tablet_mode: RwLock<Option<bool>>,
    orientation: RwLock<Option<Orientation>>,
    interface: RwLock<Option<InterfaceRef<Service>>>,
}

#[derive(Clone)]
pub struct Service {
    state: Arc<State>,
}

/// Tablet-mode watch service
#[dbus_interface(name = "tablet.assist.Service1")]
impl Service {
    /// Current tablet-mode state property
    #[dbus_interface(property)]
    async fn tablet_mode(&self) -> bool {
        self.state.tablet_mode.read().await.unwrap_or_default()
    }

    /// Tablet-mode available property
    #[dbus_interface(property)]
    async fn has_tablet_mode(&self) -> bool {
        self.state.tablet_mode.read().await.is_some()
    }

    /// Current screen orientation property
    #[dbus_interface(property)]
    async fn orientation(&self) -> Orientation {
        self.state.orientation.read().await.unwrap_or_default()
    }

    /// Orientation available property
    #[dbus_interface(property)]
    async fn has_orientation(&self) -> bool {
        self.state.orientation.read().await.is_some()
    }
}

impl Service {
    pub fn new() -> Result<Self> {
        Ok(Service {
            state: Arc::new(State {
                tablet_mode: RwLock::new(None),
                orientation: RwLock::new(None),
                interface: RwLock::new(None),
            }),
        })
    }

    pub async fn set_interface(&self, interface: InterfaceRef<Self>) {
        *self.state.interface.write().await = Some(interface);
    }

    pub async fn set_tablet_mode(&self, mode: bool) -> Result<()> {
        let avail = {
            let mut val = self.state.tablet_mode.write().await;
            let avail = val.is_some();
            *val = Some(mode);
            avail
        };

        let iface = self.state.interface.read().await;
        let sigctx = iface.as_ref().unwrap().signal_context();

        self.tablet_mode_changed(sigctx).await?;
        if !avail {
            self.has_tablet_mode_changed(sigctx).await?;
        }

        Ok(())
    }

    pub async fn set_orientation(&self, orientation: Orientation) -> Result<()> {
        let avail = {
            let mut val = self.state.orientation.write().await;
            let avail = val.is_some();
            *val = Some(orientation);
            avail
        };

        let iface = self.state.interface.read().await;
        let sigctx = iface.as_ref().unwrap().signal_context();

        self.orientation_changed(sigctx).await?;
        if !avail {
            self.has_orientation_changed(sigctx).await?;
        }

        Ok(())
    }
}
