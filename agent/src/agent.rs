use crate::{
    Config, ConfigHolder, DeviceAction, DeviceConfig, DeviceId, Orientation, Result, ServiceProxy,
    XClient,
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
            self.auto_tablet_mode_changed(sigctx).await?;

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

    /// Get input device action in tablet mode
    async fn input_device_action(&self, device: DeviceId) -> DeviceAction {
        self.with_config(|config| config.tablet_mode.get_device(&device).action)
            .await
    }

    /// Set input device action in tablet mode
    async fn set_input_device_action(&self, device: DeviceId, action: DeviceAction) {
        self.with_config_mut(|config| {
            config
                .tablet_mode
                .set_device(&device, DeviceConfig { action })
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
            self.auto_orientation_changed(sigctx).await?;

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
            .cache_properties(zbus::CacheProperties::No)
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
                let (time, crtc) = xclient.builtin_crtc().await?;
                xclient.crtc_orientation(time, crtc).await?
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

        if auto_tablet_mode {
            self.detect_tablet_mode(true).await?;
        }

        if auto_orientation {
            self.detect_orientation(true).await?;
        }

        self.monitor_service(true).await
    }

    async fn apply_tablet_mode(&self, mode: Option<bool>) -> Result<()> {
        let had_mode = *self.state.tablet_mode.read().await;

        let mode = if let Some(mode) = mode {
            if had_mode == mode {
                return Ok(());
            }

            *self.state.tablet_mode.write().await = mode;
            mode
        } else {
            had_mode
        };

        let device_actions = self
            .with_config(|config| {
                config
                    .tablet_mode
                    .device
                    .iter()
                    .filter(|(_, cfg)| cfg.action != DeviceAction::Skip)
                    .map(|(id, cfg)| (id.clone(), cfg.action))
                    .collect::<Vec<_>>()
            })
            .await;

        // On/off devices
        if let Some(xclient) = &self.state.xclient {
            if mode {
                // in tablet mode
                for (id, action) in device_actions {
                    xclient
                        .switch_input_device(&id, action == DeviceAction::Enable)
                        .await?;
                }
            } else {
                // in laptop mode
                for (id, action) in device_actions {
                    xclient
                        .switch_input_device(&id, action == DeviceAction::Disable)
                        .await?;
                }
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
        let enabled = self.state.tablet_mode_task.read().await.is_some();

        if enable == enabled {
            return Ok(());
        }

        if enable {
            if self.state.service.has_tablet_mode().await? {
                let mut changes = self.state.service.receive_tablet_mode_changed().await;
                let agent = self.clone();
                *self.state.tablet_mode_task.write().await = spawn(async move {
                    tracing::info!("Start tablet mode detection");
                    while changes.next().await.is_some() {
                        if let Err(error) = agent.update_tablet_mode().await {
                            tracing::warn!("Error while updating tablet mode: {}", error);
                        }
                    }
                    tracing::info!("Stop tablet mode detection");
                })
                .into();
            }
        } else {
            *self.state.tablet_mode_task.write().await = None;
        }

        Ok(())
    }

    async fn apply_orientation(&self, orientation: Option<Orientation>) -> Result<()> {
        let had_orientation = *self.state.orientation.read().await;

        let orientation = if let Some(orientation) = orientation {
            if had_orientation == orientation {
                return Ok(());
            }

            *self.state.orientation.write().await = orientation;
            orientation
        } else {
            had_orientation
        };

        if let Some(xclient) = &self.state.xclient {
            let (time, crtc) = xclient.builtin_crtc().await?;
            xclient
                .set_crtc_orientation(time, crtc, orientation)
                .await?;
        }

        let iface = self.state.interface.read().await;
        let sigctx = iface.as_ref().unwrap().signal_context();

        self.orientation_changed(sigctx).await?;

        Ok(())
    }

    async fn update_orientation(&self) -> Result<()> {
        let orientation = self.state.service.orientation().await?;
        self.apply_orientation(orientation.into()).await
    }

    async fn detect_orientation(&self, enable: bool) -> Result<()> {
        let enabled = self.state.orientation_task.read().await.is_some();

        if enable == enabled {
            return Ok(());
        }

        if enable {
            if self.state.service.has_orientation().await? {
                let mut changes = self.state.service.receive_orientation_changed().await;
                let agent = self.clone();
                *self.state.orientation_task.write().await = spawn(async move {
                    tracing::info!("Start orientation detection");
                    while changes.next().await.is_some() {
                        if let Err(error) = agent.update_orientation().await {
                            tracing::warn!("Error while updating orientation: {}", error);
                        }
                    }
                    tracing::info!("Stop orientation detection");
                })
                .into();
            }
        } else {
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
        let enabled = self.state.service_task.read().await.is_some();

        if enable == enabled {
            return Ok(());
        }

        if enable {
            enum Change {
                HasTabletMode,
                HasOrientation,
            }

            let mut changes = self
                .state
                .service
                .receive_has_tablet_mode_changed()
                .await
                .map(|_| Change::HasTabletMode)
                .race(
                    self.state
                        .service
                        .receive_has_orientation_changed()
                        .await
                        .map(|_| Change::HasOrientation),
                );

            let agent = self.clone();
            *self.state.service_task.write().await = spawn(async move {
                tracing::info!("Start service monitoring");
                while let Some(change) = changes.next().await {
                    match change {
                        Change::HasTabletMode => {
                            if let Err(error) = agent.update_tablet_mode_detection().await {
                                tracing::warn!(
                                    "Error while updating tablet mode detection: {}",
                                    error
                                );
                            }
                        }
                        Change::HasOrientation => {
                            if let Err(error) = agent.update_orientation_detection().await {
                                tracing::warn!(
                                    "Error while updating orientation detection: {}",
                                    error
                                );
                            }
                        }
                    }
                }
                tracing::info!("Stop service monitoring");
            }).into();
        } else {
            *self.state.service_task.write().await = None;
        }

        Ok(())
    }
}
