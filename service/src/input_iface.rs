use crate::{Config, Error, Result, Service};
use input::{
    event::{device::DeviceEvent, Event, EventTrait},
    Device, Libinput, LibinputInterface,
};
use libc::{O_RDONLY, O_RDWR, O_WRONLY};
use smol::Async;
use std::{
    fs::{File, OpenOptions},
    os::unix::{fs::OpenOptionsExt, io::OwnedFd},
    path::{Path, PathBuf},
};

pub struct Input(Async<Libinput>);

impl core::ops::Deref for Input {
    type Target = Libinput;

    fn deref(&self) -> &Self::Target {
        self.0.get_ref()
    }
}

impl core::ops::DerefMut for Input {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.get_mut() }
    }
}

impl Input {
    pub fn new_udev() -> Result<Self> {
        Ok(Self(Async::new(Libinput::new_with_udev(InputInterface))?))
    }

    pub fn add_seat(&mut self, seat: impl AsRef<str>) -> Result<()> {
        let seat = seat.as_ref();
        self.udev_assign_seat(seat)
            .map_err(|_| Error::AddSeat(seat.into()))
    }

    pub fn new_path() -> Result<Self> {
        Ok(Self(Async::new(Libinput::new_from_path(InputInterface))?))
    }

    pub fn add_path(&mut self, path: impl AsRef<Path>) -> Result<Device> {
        let path = path.as_ref();
        let path = path.to_str().unwrap();
        self.path_add_device(path)
            .ok_or_else(|| Error::AddPath(path.into()))
    }

    pub fn from_paths(paths: impl IntoIterator<Item = impl AsRef<Path>>) -> Result<Self> {
        let mut this = Self::new_path()?;

        for path in paths {
            this.add_path(path)?;
        }

        Ok(this)
    }

    pub fn devices(&mut self) -> Result<impl Iterator<Item = Device> + '_> {
        self.dispatch()?;

        Ok((&mut **self).filter_map(move |event| {
            if let Event::Device(DeviceEvent::Added(event)) = &event {
                Some(event.device())
            } else {
                None
            }
        }))
    }

    pub async fn wait(&mut self) -> Result<()> {
        self.0.readable().await?;
        self.dispatch()?;
        Ok(())
    }

    pub async fn process(
        devices: Vec<PathBuf>,
        service: Service,
    ) -> Result<Option<async_signal::Signal>> {
        use input::event::{
            switch::{Switch, SwitchState},
            SwitchEvent,
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

struct InputInterface;

impl LibinputInterface for InputInterface {
    fn open_restricted(&mut self, path: &Path, flags: i32) -> core::result::Result<OwnedFd, i32> {
        OpenOptions::new()
            .custom_flags(flags)
            .read((flags & O_RDONLY != 0) | (flags & O_RDWR != 0))
            .write((flags & O_WRONLY != 0) | (flags & O_RDWR != 0))
            .open(path)
            .map(|file| {
                let fd = file.into();
                log::info!("Open {fd:?}");
                fd
            })
            .map_err(|err| err.raw_os_error().unwrap())
    }
    fn close_restricted(&mut self, fd: OwnedFd) {
        log::info!("Close {fd:?}");
        let _ = File::from(fd);
    }
}

impl Config {
    pub fn find_input_devices(&self) -> Result<Vec<PathBuf>> {
        use input::{event::switch::Switch, DeviceCapability};

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
}
