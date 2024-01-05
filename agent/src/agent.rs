use crate::{
    Config, ConfigHolder, DeviceConfig, DeviceId, Orientation, Result, ServiceProxy, XClient,
};
use smol::{lock::RwLock, spawn, stream::StreamExt, Task};
use std::sync::Arc;
use zbus::{dbus_interface, Connection, InterfaceRef};

/// Internal service state
struct State {
    /// System service interface
    service: ServiceProxy<'static>,
    /// Keep service task running
    service_task: RwLock<Option<Task<()>>>,
    /// Current configuration
    config: RwLock<ConfigHolder<Config>>,
    /// X server client
    xclient: Option<XClient>,
    /// Current tablet mode
    tablet_mode: RwLock<bool>,
    /// Keep tablet mode detection task running
    tablet_mode_task: RwLock<Option<Task<()>>>,
    /// Current orientation
    orientation: RwLock<Orientation>,
    /// Keep orientation detection task running
    orientation_task: RwLock<Option<Task<()>>>,
    /// DBus interface reference for signaling
    interface: RwLock<Option<InterfaceRef<Agent>>>,
}

#[derive(Clone)]
pub struct Agent {
    state: Arc<State>,
}

/// Tablet assist agent
#[dbus_interface(name = "tablet.assist.Agent1")]
impl Agent {
    /// Whether tablet-mode detection available
    #[dbus_interface(property)]
    async fn tablet_mode_detection(&self) -> zbus::fdo::Result<bool> {
        self.state.service.has_tablet_mode().await
    }

    /// Current tablet-mode state
    #[dbus_interface(property)]
    async fn tablet_mode(&self) -> bool {
        *self.state.tablet_mode.read().await
    }

    /// Manual tablet-mode switch
    #[dbus_interface(property)]
    async fn set_tablet_mode(&self, enable: bool) -> zbus::Result<()> {
        let iface = self.state.interface.read().await;
        let sigctx = iface.as_ref().unwrap().signal_context();

        let had_auto = self
            .with_config_mut(|config| {
                let had_auto = config.tablet_mode.auto;

                config.tablet_mode.auto = false;
                config.tablet_mode.manual = enable;

                had_auto
            })
            .await;

        if had_auto {
            self.auto_tablet_mode_changed(sigctx).await?;
        }

        self.apply_tablet_mode(enable.into()).await?;

        Ok(())
    }

    /// Get auto tablet-mode switch
    #[dbus_interface(property)]
    async fn auto_tablet_mode(&self) -> bool {
        self.with_config(|config| config.tablet_mode.auto).await
    }

    /// Set auto tablet-mode switch
    #[dbus_interface(property)]
    async fn set_auto_tablet_mode(&self, auto: bool) -> zbus::Result<()> {
        let iface = self.state.interface.read().await;
        let sigctx = iface.as_ref().unwrap().signal_context();

        let (had_auto, manual) = self
            .with_config_mut(|config| {
                let res = (config.tablet_mode.auto, config.tablet_mode.manual);
                config.tablet_mode.auto = auto;
                res
            })
            .await;

        if auto != had_auto {
            tracing::debug!("Auto tablet-mode changed to: {auto}");

            self.auto_tablet_mode_changed(sigctx).await?;

            self.detect_tablet_mode(auto).await?;

            let mode = if auto {
                self.state.service.tablet_mode().await?
            } else {
                manual
            };

            self.apply_tablet_mode(mode.into()).await?;
        }

        Ok(())
    }

    /// Get available input devices
    #[dbus_interface(property)]
    async fn input_devices(&self) -> zbus::fdo::Result<Vec<DeviceId>> {
        Ok(if let Some(xclient) = &self.state.xclient {
            xclient.input_devices().await?
        } else {
            Default::default()
        })
    }

    /// Get input device config
    async fn input_device_config(&self, device: DeviceId) -> DeviceConfig {
        self.with_config(|config| config.get_device(&device).clone())
            .await
    }

    /// Set input device config
    async fn set_input_device(&self, device: DeviceId, device_config: DeviceConfig) {
        self.with_config_mut(|config| {
            config
                .set_device(&device, device_config)
        })
        .await;
    }

    /// Whether orientation detection available
    #[dbus_interface(property)]
    async fn orientation_detection(&self) -> zbus::fdo::Result<bool> {
        self.state.service.has_orientation().await
    }

    /// Current orientation
    #[dbus_interface(property)]
    async fn orientation(&self) -> Orientation {
        *self.state.orientation.read().await
    }

    /// Manual orientation change
    #[dbus_interface(property)]
    async fn set_orientation(&self, orientation: Orientation) -> zbus::Result<()> {
        let iface = self.state.interface.read().await;
        let sigctx = iface.as_ref().unwrap().signal_context();

        let had_auto = self
            .with_config_mut(|config| {
                let had_auto = config.orientation.auto;

                config.orientation.auto = false;
                config.orientation.manual = orientation;

                had_auto
            })
            .await;

        if had_auto {
            self.auto_orientation_changed(sigctx).await?;
        }

        self.apply_orientation(orientation.into()).await?;

        Ok(())
    }

    /// Auto orientation change
    #[dbus_interface(property)]
    async fn auto_orientation(&self) -> bool {
        self.with_config(|config| config.orientation.auto).await
    }

    /// Auto orientation change
    #[dbus_interface(property)]
    async fn set_auto_orientation(&self, auto: bool) -> zbus::Result<()> {
        let iface = self.state.interface.read().await;
        let sigctx = iface.as_ref().unwrap().signal_context();

        let (had_auto, manual) = self
            .with_config_mut(|config| {
                let res = (config.orientation.auto, config.orientation.manual);
                config.orientation.auto = auto;
                res
            })
            .await;

        if auto != had_auto {
            tracing::debug!("Auto orientation changed to: {auto}");

            self.auto_orientation_changed(sigctx).await?;

            self.detect_orientation(auto).await?;

            let orientation = if auto {
                self.state.service.orientation().await?
            } else {
                manual
            };

            self.apply_orientation(orientation.into()).await?;
        }

        Ok(())
    }
}

impl Agent {
    pub async fn new(config: ConfigHolder<Config>) -> Result<Self> {
        let connection = Connection::system().await?;

        let service = ServiceProxy::builder(&connection)
            .cache_properties(zbus::CacheProperties::Yes)
            .build()
            .await?;

        let xclient = XClient::new()
            .await
            .map_err(|error| {
                tracing::warn!("Unable to connect to X server due to: {error}");
            })
            .ok();

        let auto_tablet_mode = config.tablet_mode.auto;
        let auto_orientation = config.orientation.auto;

        let tablet_mode = if auto_tablet_mode && service.has_tablet_mode().await? {
            service.tablet_mode().await?
        } else {
            config.tablet_mode.manual
        };

        let orientation = if auto_orientation {
            if service.has_orientation().await? {
                service.orientation().await?
            } else if let Some(xclient) = &xclient {
                xclient.screen_orientation(None).await?
            } else {
                Orientation::default()
            }
        } else {
            config.orientation.manual
        };

        let agent = Agent {
            state: Arc::new(State {
                service,
                service_task: RwLock::new(None),
                config: RwLock::new(config),
                xclient,
                tablet_mode: RwLock::new(tablet_mode),
                tablet_mode_task: RwLock::new(None),
                orientation: RwLock::new(orientation),
                orientation_task: RwLock::new(None),
                interface: RwLock::new(None),
            }),
        };

        Ok(agent)
    }

    pub async fn with_config<T>(&self, func: impl FnOnce(&Config) -> T) -> T {
        let config = self.state.config.read().await;
        func(&config)
    }

    pub async fn with_config_mut<T>(&self, func: impl FnOnce(&mut Config) -> T) -> T {
        let mut config = self.state.config.write().await;
        let res = func(&mut config);
        if let Err(error) = config.save().await {
            tracing::error!("Error while saving config: {error}");
        }
        res
    }

    pub async fn init(&self, interface: InterfaceRef<Self>) -> Result<()> {
        let (auto_tablet_mode, auto_orientation) = self
            .with_config(|config| (config.tablet_mode.auto, config.orientation.auto))
            .await;

        *self.state.interface.write().await = Some(interface);

        self.apply_tablet_mode(None).await?;
        self.apply_orientation(None).await?;

        self.detect_tablet_mode(auto_tablet_mode).await?;
        self.detect_orientation(auto_orientation).await?;

        self.monitor_service(true).await
    }

    async fn apply_tablet_mode(&self, mode: Option<bool>) -> Result<()> {
        let had_mode = *self.state.tablet_mode.read().await;

        let mode = if let Some(mode) = mode {
            if had_mode == mode {
                tracing::debug!("Tablet mode already set to: {mode}");
                return Ok(());
            }

            *self.state.tablet_mode.write().await = mode;
            mode
        } else {
            had_mode
        };

        tracing::debug!("Switch tablet mode: {mode}");

        let devices_to_switch = self
            .with_config(|config| {
                config
                    .device
                    .iter()
                    .filter(|(_, config)| !config.tablet || !config.laptop)
                    .map(if mode {
                        |(device, config): (&DeviceId, &DeviceConfig)| (device.id, config.tablet)
                    } else {
                        |(device, config): (&DeviceId, &DeviceConfig)| (device.id, config.laptop)
                    })
                    .collect::<Vec<_>>()
            })
            .await;

        // On/off devices
        if let Some(xclient) = &self.state.xclient {
            // in tablet mode
            for (id, action) in devices_to_switch {
                xclient.switch_input_device(id, action).await?;
            }
        }

        let iface = self.state.interface.read().await;
        let sigctx = iface.as_ref().unwrap().signal_context();

        self.tablet_mode_changed(sigctx).await?;

        Ok(())
    }

    async fn update_tablet_mode(&self) -> Result<()> {
        let mode = self.state.service.tablet_mode().await?;
        self.apply_tablet_mode(mode.into()).await
    }

    async fn detect_tablet_mode(&self, enable: bool) -> Result<()> {
        let enabled = { self.state.tablet_mode_task.read().await.is_some() };

        if enable == enabled {
            return Ok(());
        }

        if enable {
            if self.state.service.has_tablet_mode().await? {
                let agent = self.clone();

                let task = spawn(async move {
                    tracing::info!("Start tablet mode detection");
                    let mut changes = agent.state.service.receive_tablet_mode_changed().await;
                    while changes.next().await.is_some() {
                        if let Err(error) = agent.update_tablet_mode().await {
                            tracing::error!("Error while updating tablet mode: {}", error);
                        }
                    }
                    tracing::error!("Enexpected stop tablet mode detection");
                    *agent.state.tablet_mode_task.write().await = None;
                })
                .into();

                *self.state.tablet_mode_task.write().await = task;
            }
        } else {
            tracing::info!("Stop tablet mode detection");
            *self.state.tablet_mode_task.write().await = None;
        }

        Ok(())
    }

    async fn apply_orientation(&self, orientation: Option<Orientation>) -> Result<()> {
        let had_orientation = *self.state.orientation.read().await;

        let orientation = if let Some(orientation) = orientation {
            if had_orientation == orientation {
                tracing::debug!("Orientation already set to: {orientation:?}");
                return Ok(());
            }

            *self.state.orientation.write().await = orientation;
            orientation
        } else {
            had_orientation
        };

        tracing::debug!("Apply orientation: {orientation:?}");

        let devices_to_rotate = self
            .with_config(|config| {
                config
                    .device
                    .iter()
                    .filter(|(_, config)| config.rotate)
                    .map(|(device, _)| device.id)
                    .collect::<Vec<_>>()
            })
            .await;

        if let Some(xclient) = &self.state.xclient {
            if let Err(error) = xclient.set_screen_orientation(None, orientation).await {
                tracing::error!("Error while rotating screen: {error}");
            }

            for id in devices_to_rotate {
                if let Err(error) = xclient.set_input_device_orientation(id, orientation).await {
                    tracing::error!("Error while rotating input device: {error}");
                }
            }
        }

        let iface = self.state.interface.read().await;
        let sigctx = iface.as_ref().unwrap().signal_context();

        self.orientation_changed(sigctx).await?;

        Ok(())
    }

    async fn update_orientation(&self) -> Result<()> {
        let orientation = self.state.service.orientation().await?;
        tracing::debug!("Update orientation: {orientation:?}");
        self.apply_orientation(orientation.into()).await
    }

    async fn detect_orientation(&self, enable: bool) -> Result<()> {
        let enabled = { self.state.orientation_task.read().await.is_some() };

        if enable == enabled {
            return Ok(());
        }

        if enable {
            if self.state.service.has_orientation().await? {
                let agent = self.clone();

                let task = spawn(async move {
                    tracing::info!("Start orientation detection");
                    let mut changes = agent.state.service.receive_orientation_changed().await;
                    while changes.next().await.is_some() {
                        if let Err(error) = agent.update_orientation().await {
                            tracing::error!("Error while updating orientation: {}", error);
                        }
                    }
                    tracing::error!("Unexpected stop orientation detection");
                    *agent.state.orientation_task.write().await = None;
                })
                .into();

                *self.state.orientation_task.write().await = task;
            }
        } else {
            tracing::info!("Stop orientation detection");
            *self.state.orientation_task.write().await = None;
        }

        Ok(())
    }

    async fn update_tablet_mode_detection(&self) -> Result<()> {
        let iface = self.state.interface.read().await;
        let sigctx = iface.as_ref().unwrap().signal_context();

        self.tablet_mode_detection_changed(sigctx).await?;

        Ok(())
    }

    async fn update_orientation_detection(&self) -> Result<()> {
        let iface = self.state.interface.read().await;
        let sigctx = iface.as_ref().unwrap().signal_context();

        self.orientation_detection_changed(sigctx).await?;

        Ok(())
    }

    async fn monitor_service(&self, enable: bool) -> Result<()> {
        let enabled = { self.state.service_task.read().await.is_some() };

        if enable == enabled {
            return Ok(());
        }

        if enable {
            enum Change {
                HasTabletMode,
                HasOrientation,
            }

            let agent = self.clone();

            let task = spawn(async move {
                tracing::info!("Start service monitoring");
                let mut changes = agent
                    .state
                    .service
                    .receive_has_tablet_mode_changed()
                    .await
                    .map(|_| Change::HasTabletMode)
                    .race(
                        agent
                            .state
                            .service
                            .receive_has_orientation_changed()
                            .await
                            .map(|_| Change::HasOrientation),
                    );

                while let Some(change) = changes.next().await {
                    match change {
                        Change::HasTabletMode => {
                            if let Err(error) = agent.update_tablet_mode_detection().await {
                                tracing::error!(
                                    "Error while updating tablet mode detection: {error}"
                                );
                            }
                        }
                        Change::HasOrientation => {
                            if let Err(error) = agent.update_orientation_detection().await {
                                tracing::error!(
                                    "Error while updating orientation detection: {error}"
                                );
                            }
                        }
                    }
                }
                tracing::error!("Unexpected stop service monitoring");
            })
            .into();

            *self.state.service_task.write().await = task;
        } else {
            tracing::info!("Stop service monitoring");
            *self.state.service_task.write().await = None;
        }

        Ok(())
    }
}
