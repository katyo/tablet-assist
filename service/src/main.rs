use async_signal::{Signal, Signals};
use smol::{future::FutureExt, stream::StreamExt};
use smol_potat::main;
#[cfg(feature = "libinput")]
use std::path::PathBuf;
use zbus::ConnectionBuilder;

mod args;
mod config;
mod error;
#[cfg(feature = "libinput")]
mod input;
mod service;
mod types;

use args::*;
use config::*;
use error::*;
#[cfg(feature = "libinput")]
use input::*;
use service::*;
use types::*;

impl Config {
    #[cfg(feature = "libinput")]
    pub fn find_input_devices(&self) -> Result<Vec<PathBuf>> {
        use libinput::{event::switch::Switch, DeviceCapability};

        let mut input = Input::new_udev()?;

        for udev in &self.udev {
            input.add_seat(&udev.seat)?;
        }

        let path_prefix = std::path::Path::new("/dev/input");

        let input_devices = input
            .devices()?
            .filter(|device| {
                device.has_capability(DeviceCapability::Switch)
                    && device
                        .switch_has_switch(Switch::TabletMode)
                        .unwrap_or(false)
            })
            // skip devices which disabled via config
            .filter(|device| {
                !self.device.iter().any(|config| {
                    (config
                        .name
                        .as_ref()
                        .map(|name| name == device.name())
                        .unwrap_or_default()
                        || config
                            .vid
                            .and_then(|vid| {
                                config.pid.map(|pid| {
                                    vid == device.id_vendor() && pid == device.id_product()
                                })
                            })
                            .unwrap_or_default())
                        && config.enable == false
                })
            })
            .map(|device| {
                log::info!("Use input device: {device:?}");
                path_prefix.join(device.sysname())
            })
            .collect::<Vec<_>>();

        Ok(input_devices)
    }

    #[cfg(feature = "iio")]
    pub fn find_iio_devices(&self) -> Result<Option<()>> {
        if let Ok(context) = iio::Context::with_backend(iio::Backend::Local) {
            //let context = industrial_io::Context::new()?;

            for device in context.devices() {
                log::debug!("IIO device: {device:?}");
            }

            Ok(Some(()))
        } else {
            Ok(None)
        }
    }
}

#[cfg(feature = "libinput")]
impl Input {
    async fn process(devices: Vec<PathBuf>, service: Service) -> Result<Option<Signal>> {
        use libinput::event::{
            switch::{Switch, SwitchState},
            Event, SwitchEvent,
        };

        let mut input = Self::from_paths(devices)?;

        loop {
            input.wait().await.map_err(|error| {
                log::error!("Libinput error: {error}");
                error
            })?;

            for event in &mut *input {
                log::debug!("Got event: {event:?}");
                if let Event::Switch(SwitchEvent::Toggle(event)) = &event {
                    if event.switch() == Some(Switch::TabletMode) {
                        service
                            .set_tablet_mode(event.switch_state() == SwitchState::On)
                            .await?;
                    }
                }
            }
        }
    }
}

// Although we use `async-std` here, you can use any async runtime of choice.
#[main]
async fn main() -> Result<()> {
    let args = Args::new();

    env_logger::init();
    log::info!("Start");

    #[cfg(any(feature = "libinput", feature = "iio"))]
    let config = if let Some(path) = &args.config {
        Config::from_file(path).await?
    } else {
        Config::default()
    };

    #[cfg(feature = "libinput")]
    let input_devices = config.find_input_devices()?;

    #[cfg(feature = "iio")]
    config.find_iio_devices()?;

    if !args.dbus {
        return Ok(());
    }

    let mut signals = Signals::new(&[Signal::Term, Signal::Quit, Signal::Int])?;

    let service = Service::new()?;

    let service_name = "tablet.assist.Service";
    let service_path = "/tablet/assist";

    let connection = ConnectionBuilder::system()?
        .name(service_name)?
        .serve_at(service_path, service.clone())?
        .build()
        .await?;

    service
        .set_interface(connection.object_server().interface(service_path).await?)
        .await;

    let tasks = async {
        loop {
            match signals.next().await {
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
            }
        }
    }
    .boxed_local();

    #[cfg(feature = "libinput")]
    let tasks = if !input_devices.is_empty() {
        // Add input task
        tasks
            .race(Input::process(input_devices, service.clone()))
            .boxed_local()
    } else {
        tasks
    };

    let res = tasks.await;

    drop(connection);

    log::info!("Stop");

    match res {
        Ok(Some(sig)) => {
            signal_hook::low_level::emulate_default_handler(sig as _)?;
            Ok(())
        }
        Err(error) => Err(error),
        _ => Ok(()),
    }
}
