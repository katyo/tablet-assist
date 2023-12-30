use ::input::{Device, DeviceCapability, event::{switch::{Switch, SwitchState}, Event, SwitchEvent}};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock, atomic::{Ordering, AtomicBool}},
};
use zbus::{ConnectionBuilder, dbus_interface, InterfaceRef};
use async_signal::{Signal, Signals};
use futures_util::{select, FutureExt, StreamExt};

mod error;
mod args;
mod config;
mod input;

pub use error::*;
pub use args::*;
pub use config::*;
pub use input::*;

/// Internal service state
struct State {
    tablet_mode: AtomicBool,
    interface: RwLock<Option<InterfaceRef<Service>>>,
}

#[derive(Clone)]
struct Service(Arc<State>);

impl core::ops::Deref for Service {
    type Target = Arc<State>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Tablet-mode watch service
#[dbus_interface(name = "tablet.assist.Service1")]
impl Service {
    /// Current tablet-mode state property
    #[dbus_interface(property)]
    async fn tablet_mode(&self) -> bool {
        self.tablet_mode.load(Ordering::SeqCst)
    }
}

impl Service {
    pub fn new() -> Result<Self> {
        Ok(Service(Arc::new(State {
            tablet_mode: AtomicBool::new(false),
            interface: RwLock::new(None),
        })))
    }

    pub fn set_interface(&self, interface: InterfaceRef<Self>) {
        *self.interface.write().unwrap() = Some(interface);
    }

    pub async fn set_tablet_mode(&self, mode: bool) -> Result<()> {
        self.tablet_mode.store(mode, Ordering::SeqCst);

        let iface = self.interface.read().unwrap();
        let sigctx = iface.as_ref().unwrap().signal_context();

        self.tablet_mode_changed(sigctx).await?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub path: PathBuf,
    pub vid: u32,
    pub pid: u32,
}

impl From<Device> for DeviceInfo {
    fn from(device: Device) -> Self {
        let name = device.name().into();
        let path = Path::new("/dev/input").join(device.sysname());
        let vid = device.id_vendor();
        let pid = device.id_product();
        Self { name, path, vid, pid }
    }
}

// Although we use `async-std` here, you can use any async runtime of choice.
#[smol_potat::main]
async fn main() -> Result<()> {
    let args = Args::new();

    env_logger::init();
    log::info!("Start");

    let config = if let Some(path) = &args.config {
        Config::from_file(path).await?
    } else {
        Config::default()
    };

    let mut input = Input::new_udev()?;
    for udev in config.udev {
        input.add_seat(&udev.seat)?;
    }

    let devices = input.devices()?
        .filter(|device| device.has_capability(DeviceCapability::Switch) && device.switch_has_switch(Switch::TabletMode).unwrap_or(false))
    // skip devices which disabled via config
        .filter(|device| !config.device.iter()
                .any(|config| (config.name.as_ref().map(|name| name == device.name()).unwrap_or_default() ||
                     config.vid.and_then(|vid| config.pid.map(|pid| vid == device.id_vendor() && pid == device.id_product())).unwrap_or_default()) &&
                     config.enable == false)
        )
        .map(DeviceInfo::from)
        .collect::<Vec<_>>();

    drop(input);

    let mut input = Input::new_path()?;

    for device in devices {
        log::info!("Add tablet mode device: {device:?}");
        input.add_path(&device.path)?;
    }

    if !args.dbus {
        return Ok(());
    }

    let mut signals = Signals::new(&[
        Signal::Term,
        Signal::Quit,
        Signal::Int,
    ])?;

    let service = Service::new()?;

    let service_name = "tablet.assist.Service";
    let service_path = "/tablet/assist";

    let connection = ConnectionBuilder::system()?
        .name(service_name)?
        .serve_at(service_path, service.clone())?
        .build()
        .await?;

    service.set_interface(connection.object_server().interface(service_path).await?);

    let res = loop {
        select! {
            res = input.wait().fuse() => match res {
                Ok(_) => {
                    for event in &mut *input {
                        log::debug!("Got event: {event:?}");
                        if let Event::Switch(SwitchEvent::Toggle(event)) = &event {
                            if event.switch() == Some(Switch::TabletMode) {
                                service.set_tablet_mode(event.switch_state() == SwitchState::On).await?;
                            }
                        }
                    }
                }
                Err(error) => {
                    log::error!("Libinput error: {error}");
                    break Err(Error::from(error));
                }
            },
            sig = signals.next().fuse() => match sig {
                Some(Ok(sig)) => {
                    log::info!("Received signal {:?}", sig);

                    break Ok(Some(sig));
                }
                Some(Err(error)) => {
                    log::error!("Signal error: {error}");
                    break Err(Error::from(error));
                }
                None => {
                    log::error!("Signal receiver terminated");
                    break Err(Error::Term);
                }
            },
        }
    };

    drop(connection);

    match res {
        Ok(Some(sig)) => {
            signal_hook::low_level::emulate_default_handler(sig as i32)?;
            Ok(())
        }
        Err(error) => {
            Err(error)
        }
        _ => Ok(()),
    }
}
