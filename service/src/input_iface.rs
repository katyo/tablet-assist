use crate::{Config, Result, Service};
use input::{
    event::{Event, EventTrait},
    Device, Libinput, LibinputInterface,
};
use libc::{O_RDONLY, O_RDWR, O_WRONLY};
use smol::Async;
use std::{
    fs::{File, OpenOptions},
    os::unix::{fs::OpenOptionsExt, io::OwnedFd},
    path::{Path, PathBuf},
};

/// Input error type
#[derive(thiserror::Error, Debug)]
pub enum InputError {
    /// Add seat
    #[error("Add seat: {0}")]
    AddSeat(String),
    /// Add path
    #[error("Add path: {0}")]
    AddPath(PathBuf),
}

impl AsRef<str> for InputError {
    fn as_ref(&self) -> &str {
        match self {
            Self::AddSeat(_) => "input-add-seat",
            Self::AddPath(_) => "input-add-path",
        }
    }
}

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
    pub fn from_udev(seats: impl IntoIterator<Item = impl AsRef<str>>) -> Result<Self> {
        let mut this = Self(Async::new(Libinput::new_with_udev(InputInterface))?);

        for seat in seats {
            let seat = seat.as_ref();
            this.udev_assign_seat(seat)
                .map_err(|_| InputError::AddSeat(seat.into()))?
        }

        this.dispatch()?;

        Ok(this)
    }

    pub fn from_paths(paths: impl IntoIterator<Item = impl AsRef<Path>>) -> Result<Self> {
        let mut this = Self(Async::new(Libinput::new_from_path(InputInterface))?);

        for path in paths {
            let path = path.as_ref();
            if let Some(path_str) = path.to_str() {
                this.path_add_device(path_str)
                    .ok_or_else(|| InputError::AddPath(path.into()))?;
            }
        }

        this.dispatch()?;

        Ok(this)
    }

    pub fn devices(&mut self) -> Result<impl Iterator<Item = Device> + '_> {
        use input::event::DeviceEvent;

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
        use input::{
            event::{
                switch::{Switch, SwitchState},
                DeviceEvent, SwitchEvent,
            },
            DeviceCapability,
        };

        let mut input = Self::from_paths(devices)?;

        loop {
            for event in &mut *input {
                tracing::debug!("Got event: {event:?}");
                match event {
                    Event::Device(DeviceEvent::Added(event)) => {
                        let device = event.device();
                        if device.has_capability(DeviceCapability::Switch)
                            && device
                                .switch_has_switch(Switch::TabletMode)
                                .unwrap_or(false)
                        {
                            service.set_tablet_mode(false).await?;
                        }
                    }
                    Event::Switch(SwitchEvent::Toggle(event)) => {
                        if event.switch() == Some(Switch::TabletMode) {
                            service
                                .set_tablet_mode(event.switch_state() == SwitchState::On)
                                .await?;
                        }
                    }
                    _ => (),
                }
            }

            input.wait().await.map_err(|error| {
                tracing::error!("Libinput error: {error}");
                error
            })?;
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
                tracing::trace!("Open {fd:?}");
                fd
            })
            .map_err(|err| err.raw_os_error().unwrap())
    }
    fn close_restricted(&mut self, fd: OwnedFd) {
        tracing::trace!("Close {fd:?}");
        let _ = File::from(fd);
    }
}

impl Config {
    pub fn find_input_devices(&self) -> Result<Vec<PathBuf>> {
        use input::{event::switch::Switch, DeviceCapability};

        let mut input = Input::from_udev(self.udev.iter().map(|cfg| &cfg.seat))?;

        let path_prefix = Path::new("/dev/input");

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
                tracing::info!("Use input device: {device:?}");
                path_prefix.join(device.sysname())
            })
            .collect::<Vec<_>>();

        Ok(input_devices)
    }
}
