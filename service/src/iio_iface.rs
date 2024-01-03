use crate::{Config, Error, Orientation, OrientationConfig, Result, Service};
use core::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};
use glam::{dvec3 as vec3, DMat3 as Mat3, DVec2 as Vec2, DVec3 as Vec3};
use std::{
    collections::VecDeque,
    ffi::OsStr,
    path::{Path, PathBuf},
    str::FromStr,
    time::{Duration, Instant},
};

impl Config {
    #[cfg(feature = "iio")]
    pub fn find_iio_devices(&self) -> Result<Vec<PathBuf>> {
        let mut enumerator = udev::Enumerator::new()?;

        enumerator.match_subsystem("iio")?;

        let devices = enumerator
            .scan_devices()
            .unwrap()
            .filter(|dev| dev.is_initialized() && dev.device_type().is_some())
            .map(|drv| drv.syspath().into())
            .collect();

        Ok(devices)
    }
}

#[derive(Default)]
pub struct Iio {
    display_accel: Option<Accel>,
    base_accel: Option<Accel>,
    orientation_config: OrientationConfig,
}

impl Iio {
    pub fn from_paths(
        paths: impl IntoIterator<Item = impl AsRef<Path>>,
        orientation_config: &OrientationConfig,
    ) -> Result<Self> {
        let mut iio = Self {
            orientation_config: orientation_config.to_radians(),
            ..Self::default()
        };

        for path in paths {
            let device = udev::Device::from_syspath(path.as_ref())?;
            match device.device_type() {
                Some(DeviceType::Accel) => {
                    let accel = Accel::new(device, 4)?;
                    match accel.location {
                        AccelLocation::Display => iio.display_accel = accel.into(),
                        AccelLocation::Base => iio.base_accel = accel.into(),
                    }
                }
                _ => (),
            }
        }

        Ok(iio)
    }

    pub fn poll(&mut self) -> Result<()> {
        if let Some(accel) = &mut self.display_accel {
            accel.poll()?;
        }
        if let Some(accel) = &mut self.base_accel {
            accel.poll()?;
        }
        Ok(())
    }

    pub fn display_orientation(&self) -> Option<Orientation> {
        self.display_accel
            .as_ref()
            .and_then(|accel| accel.plane_orientation_checked(&self.orientation_config))
    }

    pub fn tablet_mode(&self) -> Option<bool> {
        self.base_accel
            .as_ref()
            .and_then(|accel| accel.value(0))
            .and_then(|base| {
                self.display_accel
                    .as_ref()
                    .and_then(|accel| accel.value(0))
                    .map(|display| base.0.angle_between(display.0))
            })
            .map(|angle| angle < FRAC_PI_2)
        // TODO:
    }

    pub async fn process(
        devices: Vec<PathBuf>,
        service: Service,
        orientation_config: &OrientationConfig,
    ) -> Result<Option<async_signal::Signal>> {
        let mut iio = Self::from_paths(devices, &orientation_config)?;
        let mut last_display_orient = None;
        let mut last_tablet_mode = None;

        loop {
            let timer = smol::Timer::after(Duration::from_secs(1));

            if let Err(error) = iio.poll() {
                log::warn!("Error while polling IIO sensors: {error}");
            }

            if let Some(orient) = iio.display_orientation() {
                if !last_display_orient
                    .map(|last_orient| last_orient != orient)
                    .unwrap_or_default()
                {
                    last_display_orient = orient.into();
                    if let Err(error) = service.set_orientation(orient).await {
                        log::warn!("Error while setting orientation: {error}");
                    }
                }
            }

            if let Some(mode) = iio.tablet_mode() {
                if !last_tablet_mode
                    .map(|last_mode| last_mode != mode)
                    .unwrap_or_default()
                {
                    last_tablet_mode = mode.into();
                    if let Err(error) = service.set_tablet_mode(mode).await {
                        log::warn!("Error while setting tablet mode: {error}");
                    }
                }
            }

            timer.await;
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(u8)]
enum AccelLocation {
    #[default]
    Display,
    Base,
}

impl FromStr for AccelLocation {
    type Err = ();
    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        Ok(match s {
            "display" | "lid" | "accel-display" => Self::Display,
            "base" | "accel-base" => Self::Base,
            _ => return Err(()),
        })
    }
}

struct Accel {
    device: udev::Device,
    location: AccelLocation,
    mount: Mat3,
    offset: Vec3,
    scale: Vec3,
    depth: usize,
    data: VecDeque<(Vec3, Instant)>,
}

impl Accel {
    pub fn new(device: udev::Device, depth: usize) -> Result<Self> {
        let location = device.accel_location().unwrap_or_default();
        let mount = device.accel_mount_matrix().unwrap_or(Mat3::IDENTITY);
        let offset = device.accel_offset().unwrap_or(Vec3::ZERO);
        let scale = device.accel_scale().unwrap_or(Vec3::ONE);
        let data = VecDeque::with_capacity(depth);

        Ok(Self {
            device,
            location,
            mount,
            offset,
            scale,
            depth,
            data,
        })
    }

    pub fn poll(&mut self) -> Result<()> {
        let time = Instant::now();
        let raw = self
            .device
            .accel_raw()
            .ok_or_else(|| Error::Poll("accel".into()))?;
        let val = (raw - self.offset) * self.scale;
        let val = self.mount * val;
        self.push(val, time);
        Ok(())
    }

    fn push(&mut self, val: Vec3, time: Instant) {
        while self.data.len() >= self.depth {
            self.data.pop_front();
        }
        self.data.push_back((val, time));
    }

    pub fn value(&self, depth: usize) -> Option<(Vec3, Instant)> {
        let len = self.data.len();
        let nth = depth + 1;
        if nth > len {
            return None;
        }
        Some(self.data[len - nth])
    }

    pub fn angular_velocity(&self, depth: usize) -> Option<(f64, Instant)> {
        let pv = self.value(depth + 1)?;
        let lv = self.value(depth)?;
        let dv = lv.0.angle_between(pv.0);
        let dt = (lv.1 - pv.1).as_secs_f64();
        Some((dv / dt, lv.1))
    }

    pub fn angular_acceleration(&self, depth: usize) -> Option<(f64, Instant)> {
        let pv = self.angular_velocity(depth + 1)?;
        let lv = self.angular_velocity(depth)?;
        let da = lv.0 - pv.0;
        let dt = (lv.1 - pv.1).as_secs_f64();
        Some((da / dt, lv.1))
    }

    pub fn plane_orientation(&self) -> Option<(Orientation, f64, f64)> {
        let value = self.value(0)?.0;

        let z_angle = value.angle_between(Vec3::Z) - FRAC_PI_2;

        let xy_value = value.truncate();

        let xy_angle = xy_value.angle_between(Vec2::NEG_Y);

        let orientation = if xy_angle < -FRAC_PI_4 * 3.0 {
            Orientation::BottomUp
        } else if xy_angle < -FRAC_PI_4 {
            Orientation::LeftUp
        } else if xy_angle < FRAC_PI_4 {
            Orientation::TopUp
        } else if xy_angle < FRAC_PI_4 * 3.0 {
            Orientation::RightUp
        } else {
            Orientation::BottomUp
        };

        let angle = match orientation {
            Orientation::TopUp => xy_angle,
            Orientation::LeftUp => xy_angle + FRAC_PI_2,
            Orientation::RightUp => xy_angle - FRAC_PI_2,
            Orientation::BottomUp => {
                if xy_angle < 0.0 {
                    xy_angle + PI
                } else {
                    xy_angle - PI
                }
            }
        };

        Some((orientation, z_angle, angle))
    }

    pub fn plane_orientation_checked(&self, config: &OrientationConfig) -> Option<Orientation> {
        let acceleration = self.angular_acceleration(0)?.0;
        let velocity = self.angular_velocity(0)?.0;
        let (orientation, z_angle, angle) = self.plane_orientation()?;
        if config.check(
            angle.into(),
            z_angle.into(),
            velocity.into(),
            acceleration.into(),
        ) {
            Some(orientation)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
enum DeviceType {
    Accel,
}

trait DeviceExt {
    fn device_type(&self) -> Option<DeviceType>;

    fn property_value_str(&self, property: impl AsRef<OsStr>) -> Option<&str>;
    fn property_value_typed<T: FromStr>(&self, property: impl AsRef<OsStr>) -> Option<T>;

    fn attribute_value_str(&self, attribute: impl AsRef<OsStr>) -> Option<&str>;
    fn attribute_value_typed<T: FromStr>(&self, attribute: impl AsRef<OsStr>) -> Option<T>;
    fn attribute_value_uncache(&self, attribute: impl AsRef<OsStr>) -> std::io::Result<()>;
    fn attribute_value_typed_uncached<T: FromStr>(&self, attribute: impl AsRef<OsStr>)
        -> Option<T>;

    fn accel_location(&self) -> Option<AccelLocation>;
    fn accel_mount_matrix(&self) -> Option<Mat3>;
    fn accel_offset(&self) -> Option<Vec3>;
    fn accel_scale(&self) -> Option<Vec3>;
    fn accel_raw(&self) -> Option<Vec3>;
}

impl DeviceExt for udev::Device {
    fn device_type(&self) -> Option<DeviceType> {
        self.attribute_value("name")
            .and_then(|name| name.to_str())
            .and_then(|name| {
                if name.contains("accel") {
                    Some(DeviceType::Accel)
                } else {
                    None
                }
            })
    }

    fn property_value_str(&self, property: impl AsRef<OsStr>) -> Option<&str> {
        self.property_value(property)
            .and_then(|value| value.to_str())
    }

    fn attribute_value_str(&self, attribute: impl AsRef<OsStr>) -> Option<&str> {
        self.attribute_value(attribute)
            .and_then(|value| value.to_str())
    }

    fn property_value_typed<T: FromStr>(&self, property: impl AsRef<OsStr>) -> Option<T> {
        self.property_value_str(property)
            .and_then(|value| value.parse().ok())
    }

    fn attribute_value_typed<T: FromStr>(&self, attribute: impl AsRef<OsStr>) -> Option<T> {
        self.attribute_value_str(attribute)
            .and_then(|value| value.parse().ok())
    }

    fn attribute_value_uncache(&self, attribute: impl AsRef<OsStr>) -> std::io::Result<()> {
        use udev::AsRawWithContext;

        let attribute = util::os_str_to_cstring(attribute)?;

        util::errno_to_result(unsafe {
            udev::ffi::udev_device_set_sysattr_value(
                self.as_raw(),
                attribute.as_ptr(),
                core::ptr::null_mut() as *mut std::ffi::c_char,
            )
        })
    }

    fn attribute_value_typed_uncached<T: FromStr>(
        &self,
        attribute: impl AsRef<OsStr>,
    ) -> Option<T> {
        self.attribute_value_uncache(attribute.as_ref()).ok()?;
        self.attribute_value_str(attribute)
            .and_then(|value| value.parse().ok())
    }

    fn accel_location(&self) -> Option<AccelLocation> {
        self.property_value_str("ACCEL_LOCATION")
            .or_else(|| self.attribute_value_str("label"))
            .or_else(|| self.attribute_value_str("location"))
            .and_then(|value: &str| value.parse().ok())
    }

    fn accel_mount_matrix(&self) -> Option<Mat3> {
        self.property_value_str("ACCEL_MOUNT_MATRIX")
            .or_else(|| self.attribute_value_str("mount_matrix"))
            .or_else(|| self.attribute_value_str("in_accel_mount_matrix"))
            .or_else(|| self.attribute_value_str("in_mount_matrix"))
            .and_then(parse_mount_matrix)
    }

    fn accel_offset(&self) -> Option<Vec3> {
        self.attribute_value_typed("in_accel_x_offset")
            .and_then(|x| {
                self.attribute_value_typed("in_accel_y_offset")
                    .map(|y| (x, y))
            })
            .and_then(|(x, y)| {
                self.attribute_value_typed("in_accel_z_offset")
                    .map(|z| vec3(x, y, z))
            })
            .or_else(|| {
                self.attribute_value_typed("in_accel_offset")
                    .map(|s| vec3(s, s, s))
            })
    }

    fn accel_scale(&self) -> Option<Vec3> {
        self.attribute_value_typed("in_accel_x_scale")
            .and_then(|x| {
                self.attribute_value_typed("in_accel_y_scale")
                    .map(|y| (x, y))
            })
            .and_then(|(x, y)| {
                self.attribute_value_typed("in_accel_z_scale")
                    .map(|z| vec3(x, y, z))
            })
            .or_else(|| {
                self.attribute_value_typed("in_accel_scale")
                    .map(|s| vec3(s, s, s))
            })
    }

    fn accel_raw(&self) -> Option<Vec3> {
        self.attribute_value_typed_uncached("in_accel_x_raw")
            .and_then(|x| {
                self.attribute_value_typed_uncached("in_accel_y_raw")
                    .map(|y| (x, y))
            })
            .and_then(|(x, y)| {
                self.attribute_value_typed_uncached("in_accel_z_raw")
                    .map(|z| vec3(x, y, z))
            })
    }
}

/// x1​, y1​, z1​; x2​, y2​, z2​; x3​, y3​, z3
fn parse_mount_matrix(s: &str) -> Option<Mat3> {
    let mut mat = [[0f64; 3]; 3];

    for (row, s) in s.split(';').enumerate() {
        if row >= 3 {
            break;
        }
        for (col, s) in s.split(',').enumerate() {
            if col >= 3 {
                break;
            }
            mat[row][col] = s.trim().parse().ok()?;
        }
    }

    Some(Mat3::from_cols_array_2d(&mat))
}

mod util {
    use std::{
        ffi::{CString, OsStr},
        io::{Error, Result},
        os::unix::ffi::OsStrExt,
    };

    pub fn os_str_to_cstring<T: AsRef<OsStr>>(s: T) -> Result<CString> {
        match CString::new(s.as_ref().as_bytes()) {
            Ok(s) => Ok(s),
            Err(_) => Err(Error::from_raw_os_error(libc::EINVAL)),
        }
    }

    pub fn errno_to_result(errno: libc::c_int) -> Result<()> {
        match errno {
            x if x >= 0 => Ok(()),
            e => Err(Error::from_raw_os_error(-e)),
        }
    }
}
