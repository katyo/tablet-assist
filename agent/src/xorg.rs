use crate::{DeviceId, Error, Orientation, Result};
use smol::{spawn, Task};
use x11rb::{
    connection::Connection,
    protocol::{
        randr::{Connection as RandrConnection, ConnectionExt as RandrConnectionExt, Rotation},
        xinput::{ChangeDevicePropertyAux, ConnectionExt, ConnectionExt as InputConnectionExt},
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

    async fn atom_name(&self, atom: u32) -> Result<String> {
        Ok(String::from_utf8(
            self.conn.get_atom_name(atom).await?.reply().await?.name,
        )?)
    }

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

    pub async fn monitors_info(&self) -> Result<Vec<MonitorInfo>> {
        let reply = self
            .conn
            .randr_get_monitors(self.screen.root, true)
            .await?
            .reply()
            .await?;

        let mut monitors = Vec::default();

        for monitor in reply.monitors {
            monitors.push(MonitorInfo::new(
                reply.timestamp,
                monitor.name,
                self.atom_name(monitor.name).await?,
                monitor.primary,
                monitor.automatic,
                monitor.outputs,
            ));
        }

        Ok(monitors)
    }

    pub async fn screen_info(&self, id: Option<u32>) -> Result<ScreenInfo> {
        let id = id.unwrap_or(self.screen.root);

        let reply = self
            .conn
            .randr_get_screen_resources_current(id)
            .await?
            .reply()
            .await?;

        Ok(ScreenInfo::new(
            reply.timestamp,
            id,
            reply.outputs,
            reply.crtcs,
        ))
    }

    pub async fn output_info(&self, time: u32, id: u32) -> Result<OutputInfo> {
        let reply = self
            .conn
            .randr_get_output_info(id, time)
            .await?
            .reply()
            .await?;

        let crtc = if reply.connection == RandrConnection::CONNECTED {
            Some(reply.crtc)
        } else {
            None
        };

        Ok(OutputInfo::new(
            reply.timestamp,
            id,
            String::from_utf8(reply.name)?,
            crtc,
            reply.crtcs,
        ))
    }

    pub async fn crtc_info(&self, time: u32, id: u32) -> Result<CrtcInfo> {
        let reply = self
            .conn
            .randr_get_crtc_info(id, time)
            .await?
            .reply()
            .await?;

        Ok(CrtcInfo::new(
            reply.timestamp,
            id,
            Rect::new(reply.x, reply.y, reply.width, reply.height),
            reply.mode,
            reply.rotation,
            reply.outputs,
            reply.possible,
        ))
    }

    pub async fn set_crtc_config(&self, crtc: &CrtcInfo) -> Result<u32> {
        let reply = self
            .conn
            .randr_set_crtc_config(
                crtc.id,
                crtc.time,
                crtc.time,
                crtc.rect.left,
                crtc.rect.top,
                crtc.mode,
                crtc.rotation,
                &crtc.outputs,
            )
            .await?
            .reply()
            .await?;

        Ok(reply.timestamp)
    }

    pub async fn crtc_panning(&self, id: u32) -> Result<CrtcPan> {
        let reply = self.conn.randr_get_panning(id).await?.reply().await?;

        Ok(CrtcPan::new(
            reply.timestamp,
            id,
            Rect::new(reply.left, reply.top, reply.width, reply.height),
            Rect::new(
                reply.track_left,
                reply.track_top,
                reply.track_width,
                reply.track_height,
            ),
            Border::new(
                reply.border_left,
                reply.border_top,
                reply.border_right,
                reply.border_bottom,
            ),
        ))
    }

    pub async fn set_crtc_panning(&self, crtc: &CrtcPan) -> Result<u32> {
        let reply = self
            .conn
            .randr_set_panning(
                crtc.id,
                crtc.time,
                crtc.rect.left,
                crtc.rect.top,
                crtc.rect.width,
                crtc.rect.height,
                crtc.track.left,
                crtc.track.top,
                crtc.track.width,
                crtc.track.height,
                crtc.border.left,
                crtc.border.top,
                crtc.border.right,
                crtc.border.bottom,
            )
            .await?
            .reply()
            .await?;

        Ok(reply.timestamp)
    }

    pub async fn builtin_crtc(&self) -> Result<(u32, u32)> {
        let monitors = self.monitors_info().await?;

        let (time, outputs) = if let Some(monitor) = monitors
            .into_iter()
            .find(|monitor| monitor.name.starts_with("LVDS") || monitor.name.starts_with("eDP"))
        {
            (monitor.time, monitor.outputs)
        } else {
            let screen = self.screen_info(None).await?;
            (screen.time, screen.outputs)
        };

        for id in outputs {
            let output = self.output_info(time, id).await?;
            if output.name.starts_with("LVDS") || output.name.starts_with("eDP") {
                if let Some(id) = output.crtc {
                    return Ok((output.time, id));
                }
            }
        }

        Err(Error::NotFound)
    }

    pub async fn crtc_orientation(&self, time: u32, id: u32) -> Result<Orientation> {
        let crtc = self.crtc_info(time, id).await?;

        let orientation = rotation_to_orientation(crtc.rotation)?;

        tracing::debug!("{orientation:?}");

        Ok(orientation)
    }

    pub async fn set_crtc_orientation(
        &self,
        time: u32,
        id: u32,
        orientation: Orientation,
    ) -> Result<bool> {
        let rotation = orientation_to_rotation(orientation);

        let crtc = self.crtc_info(time, id).await?;

        tracing::debug!("crtc: {crtc:?}");

        if crtc.rotation == rotation {
            return Ok(false);
        }

        let pan = self.crtc_panning(id).await?;

        tracing::debug!("pan: {pan:?}");

        let mut crtc = crtc;

        crtc.rotation = rotation;

        self.set_crtc_config(&crtc).await?;

        let pan = self.crtc_panning(id).await?;

        tracing::debug!("pan: {pan:?}");

        Ok(true)
    }
}

#[derive(Clone, Debug)]
pub struct ScreenInfo {
    pub time: u32,
    pub id: u32,
    pub outputs: Vec<u32>,
    pub crtcs: Vec<u32>,
}

#[derive(Clone, Debug)]
pub struct MonitorInfo {
    pub time: u32,
    pub id: u32,
    pub name: String,
    pub primary: bool,
    pub auto: bool,
    pub outputs: Vec<u32>,
}

#[derive(Clone, Debug)]
pub struct OutputInfo {
    pub time: u32,
    pub id: u32,
    pub name: String,
    pub crtc: Option<u32>,
    pub crtcs: Vec<u32>,
}

#[derive(Clone, Copy, Debug)]
pub struct Rect<O> {
    pub left: O,
    pub top: O,
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Copy, Debug)]
pub struct Border<T> {
    pub left: T,
    pub top: T,
    pub right: T,
    pub bottom: T,
}

#[derive(Clone, Debug)]
pub struct CrtcInfo {
    pub time: u32,
    pub id: u32,
    pub rect: Rect<i16>,
    pub mode: u32,
    pub rotation: Rotation,
    pub outputs: Vec<u32>,
    pub possible: Vec<u32>,
}

#[derive(Clone, Debug)]
pub struct CrtcPan {
    pub time: u32,
    pub id: u32,
    pub rect: Rect<u16>,
    pub track: Rect<u16>,
    pub border: Border<i16>,
}

impl<O> Rect<O> {
    pub fn new(left: O, top: O, width: u16, height: u16) -> Self {
        Self {
            left,
            top,
            width,
            height,
        }
    }
}

impl<T> Border<T> {
    pub fn new(left: T, top: T, right: T, bottom: T) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }
}

impl ScreenInfo {
    pub fn new(time: u32, id: u32, outputs: Vec<u32>, crtcs: Vec<u32>) -> Self {
        Self {
            time,
            id,
            outputs,
            crtcs,
        }
    }
}

impl MonitorInfo {
    pub fn new(
        time: u32,
        id: u32,
        name: impl Into<String>,
        primary: bool,
        auto: bool,
        outputs: Vec<u32>,
    ) -> Self {
        Self {
            time,
            id,
            name: name.into(),
            primary,
            auto,
            outputs,
        }
    }
}

impl OutputInfo {
    pub fn new(
        time: u32,
        id: u32,
        name: impl Into<String>,
        crtc: Option<u32>,
        crtcs: Vec<u32>,
    ) -> Self {
        Self {
            time,
            id,
            name: name.into(),
            crtc,
            crtcs,
        }
    }
}

impl CrtcInfo {
    pub fn new(
        time: u32,
        id: u32,
        rect: Rect<i16>,
        mode: u32,
        rotation: Rotation,
        outputs: Vec<u32>,
        possible: Vec<u32>,
    ) -> Self {
        Self {
            time,
            id,
            rect,
            mode,
            rotation,
            outputs,
            possible,
        }
    }
}

impl CrtcPan {
    pub fn new(time: u32, id: u32, rect: Rect<u16>, track: Rect<u16>, border: Border<i16>) -> Self {
        Self {
            time,
            id,
            rect,
            track,
            border,
        }
    }
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
