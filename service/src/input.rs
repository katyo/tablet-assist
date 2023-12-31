use ::input::{
    event::{device::DeviceEvent, Event, EventTrait},
    Device, Libinput, LibinputInterface,
};
use async_io::Async;
use libc::{O_RDONLY, O_RDWR, O_WRONLY};
use std::{
    fs::{File, OpenOptions},
    os::unix::{fs::OpenOptionsExt, io::OwnedFd},
    path::Path,
};

use crate::{Error, Result};

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

        for path in paths.into_iter() {
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
