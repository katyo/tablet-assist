use crate::{Agent, InputDeviceInfo, Result};
use std::sync::Arc;
use zbus::{dbus_interface, zvariant::ObjectPath, Connection};

struct State {
    info: InputDeviceInfo,
    agent: Agent,
}

#[derive(Clone)]
pub struct InputDevice {
    state: Arc<State>,
}

impl InputDevice {
    pub fn new(agent: &Agent, info: InputDeviceInfo) -> Self {
        let agent = agent.clone();
        Self {
            state: Arc::new(State { agent, info }),
        }
    }

    fn path(&self) -> zbus::Result<ObjectPath<'static>> {
        Ok(format!("/tablet/assist/input_device/{}", self.state.info.id).try_into()?)
    }

    pub async fn add(&self, conn: &Connection) -> Result<()> {
        conn.object_server().at(self.path()?, self.clone()).await?;
        Ok(())
    }

    pub async fn remove(&self, conn: &Connection) -> Result<()> {
        conn.object_server().remove::<Self, _>(self.path()?).await?;
        Ok(())
    }
}

/// Input device control interface
#[dbus_interface(name = "tablet.assist.InputDevice1")]
impl InputDevice {
    /// Input device id
    #[dbus_interface(property)]
    fn device_id(&self) -> u32 {
        self.state.info.id
    }

    /// Input device name
    #[dbus_interface(property)]
    fn device_name(&self) -> &str {
        self.state.info.name.as_ref()
    }

    /// Input device type
    #[dbus_interface(property)]
    fn device_type(&self) -> &str {
        &self.state.info.type_
    }

    /// Whether to enable device in tablet mode
    #[dbus_interface(property)]
    async fn enable_tablet(&self) -> bool {
        self.state
            .agent
            .with_config(|config| config.get_device(&self.state.info).tablet)
            .await
    }

    #[dbus_interface(property)]
    async fn set_enable_tablet(&self, enable: bool) -> zbus::Result<()> {
        let enabled = self
            .state
            .agent
            .with_config_mut(|config| {
                config.with_device(&self.state.info, |config| {
                    let had = config.tablet;
                    config.tablet = enable;
                    had
                })
            })
            .await;
        if enable != enabled {
            self.state
                .agent
                .update_input_device_state(self.state.info.id, enable, true)
                .await?;
        }
        Ok(())
    }

    /// Whether to enable device in laptop mode
    #[dbus_interface(property)]
    async fn enable_laptop(&self) -> bool {
        self.state
            .agent
            .with_config(|config| config.get_device(&self.state.info).laptop)
            .await
    }

    #[dbus_interface(property)]
    async fn set_enable_laptop(&self, enable: bool) -> zbus::Result<()> {
        let enabled = self
            .state
            .agent
            .with_config_mut(|config| {
                config.with_device(&self.state.info, |config| {
                    let had = config.laptop;
                    config.laptop = enable;
                    had
                })
            })
            .await;
        if enable != enabled {
            self.state
                .agent
                .update_input_device_state(self.state.info.id, enable, false)
                .await?;
        }
        Ok(())
    }

    /// Whether to change device orientation with screen
    #[dbus_interface(property)]
    async fn enable_rotation(&self) -> bool {
        self.state
            .agent
            .with_config(|config| config.get_device(&self.state.info).rotate)
            .await
    }

    #[dbus_interface(property)]
    async fn set_enable_rotation(&self, enable: bool) -> zbus::Result<()> {
        let enabled = self
            .state
            .agent
            .with_config_mut(|config| {
                config.with_device(&self.state.info, |config| {
                    let had = config.rotate;
                    config.rotate = enable;
                    had
                })
            })
            .await;
        if enable != enabled {
            self.state
                .agent
                .update_input_device_orientation(self.state.info.id, enable)
                .await?;
        }
        Ok(())
    }
}
