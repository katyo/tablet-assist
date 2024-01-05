use crate::{DeviceId, Error, Orientation, Result};
use smol::{spawn, Task};
use x11rb::{
    connection::Connection,
    protocol::{
        randr::{
            Connection as RandrConnection, ConnectionExt as RandrConnectionExt, ModeInfo,
            RefreshRates, Rotation, ScreenSize,
        },
        xinput::{ChangeDevicePropertyAux, ConnectionExt as InputConnectionExt},
        xproto::{ConnectionExt as ProtoConnectionExt, PropMode, Screen},
    },
    rust_connection::RustConnection,
};
use x11rb_async as x11rb;

pub struct XClient {
    /// Keep connection background task running
    #[allow(unused)]
    task: Task<()>,
    conn: RustConnection,
    screen: Screen,
}

impl XClient {
    pub async fn new() -> Result<Self> {
        let (conn, screen_num, reader) = RustConnection::connect(None).await?;

        let task = spawn(async move {
            if let Err(error) = reader.await {
                tracing::error!("Xserver reader dead: {error}");
            }
        });

        let setup = conn.setup();

        tracing::debug!(
            "Proto version: {}.{}",
            setup.protocol_major_version,
            setup.protocol_minor_version
        );

        let screen = setup.roots[screen_num].clone();

        tracing::debug!("Screen: {}", screen.root);

        Ok(Self { task, conn, screen })
    }

    async fn atom(&self, name: impl AsRef<[u8]>) -> Result<u32> {
        Ok(self
            .conn
            .intern_atom(true, name.as_ref())
            .await?
            .reply()
            .await?
            .atom)
    }

    /*
    async fn atom_name(&self, atom: u32) -> Result<String> {
        Ok(String::from_utf8(
            self.conn.get_atom_name(atom).await?.reply().await?.name,
        )?)
    }
    */

    pub async fn input_devices(&self) -> Result<Vec<DeviceId>> {
        let res = self.conn.xinput_list_input_devices().await?;
        let reply = res.reply().await?;

        let devices = reply
            .devices
            .into_iter()
            .zip(reply.names.into_iter())
            .filter_map(|(info, name)| {
                String::from_utf8(name.name)
                    .map(|name| DeviceId {
                        id: info.device_id as _,
                        name,
                    })
                    .ok()
            })
            //.filter(|device| !device.name.contains("Virtual"))
            .collect();

        tracing::debug!("{devices:?}");

        Ok(devices)
    }

    /*
    pub async fn input_device_status(&self, device: &DeviceId) -> Result<bool> {
        let prop = self.atom("Device Enabled").await?;

        let reply = self
            .conn
            .xinput_get_device_property(prop, ANY_PROPERTY_TYPE, 0, 1, device.id as _, false)
            .await?
            .reply()
            .await?;

        Ok(reply
            .items
            .as_data8()
            .map(|data| if data.is_empty() { false } else { data[0] == 1 })
            .unwrap_or_default())
    }
    */

    pub async fn switch_input_device(&self, device: &DeviceId, enable: bool) -> Result<()> {
        let prop = self.atom("Device Enabled").await?;

        let reply = self
            .conn
            .xinput_get_device_property(prop, ANY_PROPERTY_TYPE, 0, 1, device.id as _, false)
            .await?
            .reply()
            .await?;

        let type_ = reply.type_;

        let value = ChangeDevicePropertyAux::Data8(vec![if enable { 1 } else { 0 }]);

        self.conn
            .xinput_change_device_property(
                prop,
                type_,
                device.id as _,
                PropMode::REPLACE,
                1,
                &value,
            )
            .await?;

        Ok(())
    }

    async fn get_screen_resources(&self, window: u32) -> Result<(ScreenResources, u32, u32)> {
        tracing::debug!("Request get screen 0x{window:x?} resources");

        let reply = self
            .conn
            .randr_get_screen_resources_current(window)
            .await?
            .reply()
            .await?;

        let res = ScreenResources {
            crtcs: reply.crtcs,
            outputs: reply.outputs,
            modes: reply.modes,
        };

        let time = reply.timestamp;
        let conf_time = reply.config_timestamp;

        tracing::debug!(
            "Reply get screen 0x{window:x?} resources, time {time}, conf_time {conf_time}"
        );

        Ok((res, time, conf_time))
    }

    /*
    async fn get_screen_info(&self, window: u32) -> Result<(ScreenInfo, u32, u32, u32)> {
        tracing::debug!("Request get screen 0x{window:x?} info");

        let reply = self
            .conn
            .randr_get_screen_info(window)
            .await?
            .reply()
            .await?;

        let info = ScreenInfo {
            config: ScreenConfig {
                size_id: reply.size_id,
                rotation: reply.rotation,
                rate: reply.rate,
            },
            sizes: reply.sizes,
            rates: reply.rates,
            rotations: reply.rotations,
        };

        let root = reply.root;
        let time = reply.timestamp;
        let conf_time = reply.config_timestamp;

        tracing::debug!(
            "Reply get screen 0x{root:x?} info {info:?}, time {time}, conf_time {conf_time}"
        );

        Ok((info, root, time, conf_time))
    }

    async fn set_screen_config(
        &self,
        window: u32,
        time: u32,
        conf_time: u32,
        config: &ScreenConfig,
    ) -> Result<(u32, u32, u32)> {
        tracing::debug!(
            "Request set screen 0x{window:x?} config {config:?}, time {time}, conf_time {conf_time}"
        );

        let reply = self
            .conn
            .randr_set_screen_config(
                window,
                time,
                conf_time,
                config.size_id,
                config.rotation,
                config.rate,
            )
            .await?
            .reply()
            .await?;

        let root = reply.root;
        let time = reply.new_timestamp;
        let conf_time = reply.config_timestamp;

        tracing::debug!("Reply set screen 0x{root:x?}, time {time}, conf_time {conf_time}");

        Ok((root, time, conf_time))
    }
    */

    async fn set_screen_size(
        &self,
        window: u32,
        size: &Size<u16>,
        size_mm: &Size<u32>,
    ) -> Result<()> {
        tracing::debug!("Request set screen 0x{window:x?} size {size:?}px {size_mm:?}mm");

        self.conn
            .randr_set_screen_size(
                window,
                size.width,
                size.height,
                size_mm.width,
                size_mm.height,
            )
            .await?;

        tracing::debug!("Reply set screen 0x{window:x?} size");

        Ok(())
    }

    async fn get_output_info(&self, output: u32, conf_time: u32) -> Result<(OutputInfo, u32)> {
        tracing::debug!("Request get output 0x{output:x?} info, conf_time {conf_time}");

        let reply = self
            .conn
            .randr_get_output_info(output, conf_time)
            .await?
            .reply()
            .await?;

        let crtc = if reply.connection == RandrConnection::CONNECTED {
            Some(reply.crtc)
        } else {
            None
        };

        let info = OutputInfo {
            name: String::from_utf8(reply.name)?,
            size_mm: Size {
                width: reply.mm_width,
                height: reply.mm_height,
            },
            crtc,
            crtcs: reply.crtcs,
        };

        let time = reply.timestamp;

        tracing::debug!("Reply get output 0x{output:x?} info {info:?}, time {time}");

        Ok((info, time))
    }

    async fn get_crtc_info(&self, crtc: u32, conf_time: u32) -> Result<(CrtcInfo, u32)> {
        tracing::debug!("Request get crtc 0x{crtc:x?} info, conf_time {conf_time}");

        let reply = self
            .conn
            .randr_get_crtc_info(crtc, conf_time)
            .await?
            .reply()
            .await?;

        let info = CrtcInfo {
            config: CrtcConfig {
                x: reply.x,
                y: reply.y,
                mode: reply.mode,
                rotation: reply.rotation,
                outputs: reply.outputs,
            },
            size: Size {
                width: reply.width,
                height: reply.height,
            },
            rotations: reply.rotations,
            outputs: reply.possible,
        };

        let time = reply.timestamp;

        tracing::debug!("Reply get crtc 0x{crtc:x?} info {info:?}, time {time}");

        Ok((info, time))
    }

    async fn set_crtc_config(
        &self,
        crtc: u32,
        time: u32,
        conf_time: u32,
        config: &CrtcConfig,
    ) -> Result<u32> {
        tracing::debug!(
            "Request set crtc 0x{crtc:x?} config {config:?}, time {time}, conf_time {conf_time}"
        );

        let reply = self
            .conn
            .randr_set_crtc_config(
                crtc,
                time,
                conf_time,
                config.x,
                config.y,
                config.mode,
                config.rotation,
                &config.outputs,
            )
            .await?
            .reply()
            .await?;

        let time = reply.timestamp;

        tracing::debug!("Reply set crtc 0x{crtc:x?} config, time {time}");

        Ok(time)
    }

    async fn find_builtin(&self, window: u32) -> Result<(u32, u32, u32)> {
        let (res, _time, conf_time) = self.get_screen_resources(window).await?;

        for output in res.outputs {
            let (info, time) = self.get_output_info(output, conf_time).await?;
            if let Some(crtc) = &info.crtc {
                if res.crtcs.contains(crtc)
                    && (info.name.starts_with("LVDS") || info.name.starts_with("eDP"))
                {
                    return Ok((*crtc, output, time));
                }
            }
        }

        Err(Error::XNotFound)
    }

    pub async fn screen_orientation(&self, screen: Option<u32>) -> Result<Orientation> {
        let window = screen.unwrap_or(self.screen.root);

        //let (info, ..) = self.get_screen_info(window).await?;
        let (crtc, _, time) = self.find_builtin(window).await?;
        let (info, ..) = self.get_crtc_info(crtc, time).await?;

        rotation_to_orientation(info.config.rotation)
    }

    pub async fn set_screen_orientation(
        &self,
        screen: Option<u32>,
        orientation: Orientation,
    ) -> Result<()> {
        let window = screen.unwrap_or(self.screen.root);

        //let (info, root, time, conf_time) = self.get_screen_info(window).await?;
        let (crtc, output, time) = self.find_builtin(window).await?;
        let (crtc_info, conf_time) = self.get_crtc_info(crtc, time).await?;

        let rotation = orientation_to_rotation(orientation);

        if rotation == crtc_info.config.rotation {
            return Ok(());
        }

        let had_orientation = rotation_to_orientation(crtc_info.config.rotation)?;
        let had_orientation_type = had_orientation.get_type();
        let orientation_type = orientation.get_type();

        let mut crtc_info = crtc_info;
        crtc_info.config.rotation = rotation;

        if orientation_type != had_orientation_type {
            let (output_info, ..) = self.get_output_info(output, conf_time).await?;

            let mut size = crtc_info.size;
            let mut size_mm = output_info.size_mm;

            if size.width > size.height {
                size.height = size.width;
            } else {
                size.width = size.height;
            }

            if size_mm.width > size_mm.height {
                size_mm.height = size_mm.width;
            } else {
                size_mm.width = size_mm.height;
            }

            self.set_screen_size(window, &size, &size_mm).await?;
        }

        //let _ = self.set_screen_config(root, time, conf_time, &info).await?;
        self.set_crtc_config(crtc, time, conf_time, &crtc_info.config)
            .await?;

        if orientation_type != had_orientation_type {
            let (output_info, ..) = self.get_output_info(output, conf_time).await?;

            let mut size = crtc_info.size;
            let mut size_mm = output_info.size_mm;

            size.swap();
            size_mm.swap();

            self.set_screen_size(window, &size, &size_mm).await?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct ScreenResources {
    pub crtcs: Vec<u32>,
    pub outputs: Vec<u32>,
    pub modes: Vec<ModeInfo>,
}

#[derive(Clone, Debug)]
pub struct ScreenConfig {
    pub size_id: u16,
    pub rotation: Rotation,
    pub rate: u16,
}

#[derive(Clone, Debug)]
pub struct ScreenInfo {
    pub config: ScreenConfig,
    pub rotations: Rotation,
    pub sizes: Vec<ScreenSize>,
    pub rates: Vec<RefreshRates>,
}

#[derive(Clone, Debug)]
pub struct OutputInfo {
    pub name: String,
    pub size_mm: Size<u32>,
    pub crtc: Option<u32>,
    pub crtcs: Vec<u32>,
}

#[derive(Clone, Debug)]
pub struct CrtcConfig {
    pub x: i16,
    pub y: i16,
    pub mode: u32,
    pub rotation: Rotation,
    pub outputs: Vec<u32>,
}

#[derive(Clone, Copy, Debug)]
pub struct Size<T> {
    pub width: T,
    pub height: T,
}

impl<T> Size<T> {
    pub fn swap(&mut self) {
        core::mem::swap(&mut self.width, &mut self.height);
    }
}

#[derive(Clone, Debug)]
pub struct CrtcInfo {
    pub config: CrtcConfig,
    pub size: Size<u16>,
    pub rotations: Rotation,
    pub outputs: Vec<u32>,
}

fn rotation_to_orientation(rotation: Rotation) -> Result<Orientation> {
    Ok(match rotation {
        Rotation::ROTATE0 => Orientation::TopUp,
        Rotation::ROTATE90 => Orientation::LeftUp,
        Rotation::ROTATE180 => Orientation::BottomUp,
        Rotation::ROTATE270 => Orientation::RightUp,
        _ => return Err(Error::XBadRotation),
    })
}

fn orientation_to_rotation(orientation: Orientation) -> Rotation {
    match orientation {
        Orientation::TopUp => Rotation::ROTATE0,
        Orientation::LeftUp => Rotation::ROTATE90,
        Orientation::BottomUp => Rotation::ROTATE180,
        Orientation::RightUp => Rotation::ROTATE270,
    }
}

const ANY_PROPERTY_TYPE: x11rb::protocol::xproto::Atom = 0;
