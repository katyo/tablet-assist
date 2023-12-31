use zbus::{Connection, dbus_proxy, PropertyStream};
pub use tablet_assist_core::Orientation;

#[cfg(feature = "iio-sensor-proxy")]
use tablet_assist_core::{OrientationIsp, LightLevelUnit};

/// Result type
pub type Result<T> = core::result::Result<T, Error>;

/// Error type
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// DBus error
    #[error("DBus error: {0}")]
    DBus(#[from] zbus::Error),
}

/// Tablet-mode watch service
#[dbus_proxy(
    interface = "tablet.assist.Service1",
    default_service = "tablet.assist.Service",
    default_path = "/tablet/assist"
)]
pub trait Service {
    /// Current tablet-mode state property
    #[dbus_proxy(property)]
    fn tablet_mode(&self) -> Result<bool>;

    /// Tablet-mode available property
    #[dbus_proxy(property)]
    fn has_tablet_mode(&self) -> Result<bool>;

    /// Current screen orientation property
    #[dbus_proxy(property)]
    fn orientation(&self) -> Result<Orientation>;

    /// Orientation available property
    #[dbus_proxy(property)]
    fn has_orientation(&self) -> Result<bool>;
}

/// IIO Sensor Proxy service
#[cfg(feature = "iio-sensor-proxy")]
#[dbus_proxy(
    interface = "net.hadess.SensorProxy",
    default_service = "net.hadess.SensorProxy",
    default_path = "/net/hadess/SensorProxy"
)]
pub trait IioSensor {
    /// Whether a supported accelerometer is present on the system
    #[dbus_proxy(property)]
    fn has_accelerometer(&self) -> Result<bool>;

    /// The detected orientation of the tablet or laptop
    #[dbus_proxy(property)]
    fn accelerometer_orientation(&self) -> Result<OrientationIsp>;

    /// Whether a supported ambient light sensor is present on the system
    #[dbus_proxy(property)]
    fn has_ambient_light(&self) -> Result<bool>;

    /// The unit used in Ambient Light Sensor readings
    #[dbus_proxy(property)]
    fn light_level_unit(&self) -> Result<LightLevelUnit>;

    /// The ambient light sensor reading
    #[dbus_proxy(property)]
    fn light_level(&self) -> Result<f64>;

    /// Whether a supported proximity sensor is present on the system
    #[dbus_proxy(property)]
    fn has_proximity(&self) -> Result<bool>;

    /// Whether an object is near to the proximity sensor
    #[dbus_proxy(property)]
    fn proximity_near(&self) -> Result<bool>;

    /// Start receiving accelerometer reading updates from the proxy
    fn claim_accelerometer(&self) -> Result<()>;

    /// Stop receiving accelerometer reading updates from the proxy
    fn release_accelerometer(&self) -> Result<()>;

    /// Start receiving ambient light sensor reading updates from the proxy
    fn claim_light(&self) -> Result<()>;

    /// Stop receiving ambient light sensor reading updates from the proxy
    fn release_light(&self) -> Result<()>;

    /// Start receiving proximity updates from the proxy
    fn claim_proximity(&self) -> Result<()>;

    /// Stop receiving proximity updates from the proxy
    fn release_proximity(&self) -> Result<()>;
}

#[derive(Clone)]
pub struct Client {
    //connection: Connection,
    service: ServiceProxy<'static>,
    #[cfg(feature = "iio-sensor-proxy")]
    iio_sensor: Option<IioSensorProxy<'static>>,
}

impl Client {
    pub async fn new() -> Result<Self> {
        let connection = Connection::system().await?;

        let service = ServiceProxy::builder(&connection)
            .cache_properties(zbus::CacheProperties::No)
            .build()
            .await?;

        #[cfg(feature = "iio-sensor-proxy")]
        let iio_sensor = IioSensorProxy::builder(&connection)
            .cache_properties(zbus::CacheProperties::No)
            .build()
            .await.ok();

        if let Some(iio_sensor) = &iio_sensor {
            iio_sensor.claim_accelerometer().await?;
        }

        Ok(Self {
            // connection,
            service,
            #[cfg(feature = "iio-sensor-proxy")]
            iio_sensor,
        })
    }

    pub async fn tablet_mode(&self) -> bool {
        self.service.tablet_mode().await.unwrap_or_default()
    }

    pub async fn has_tablet_mode(&self) -> bool {
        self.service.has_tablet_mode().await.unwrap_or_default()
    }

    pub async fn tablet_mode_changes(&self) -> PropertyStream<bool> {
        self.service.receive_tablet_mode_changed().await
    }

    pub async fn orientation(&self) -> Orientation {
        #[cfg(feature = "iio-sensor-proxy")]
        if let Some(iio_sensor) = &self.iio_sensor {
            return iio_sensor.accelerometer_orientation().await
                .map(|ori| Orientation::try_from(ori).unwrap_or_default())
                .unwrap_or_default()
        }

        self.service.orientation().await.unwrap_or_default()
    }

    pub async fn orientation_changes(&self) -> PropertyStream<Orientation> {
        /*
        #[cfg(feature = "iio-sensor-proxy")]
        if let Some(iio_sensor) = &self.iio_sensor {
            iio_sensor.receive_accelerometer_orientation_changed().await
                .map(|ori| Orientation::try_from(ori).unwrap_or_default())
                .unwrap_or_default();
        }
        */

        self.service.receive_orientation_changed().await
    }

    pub async fn has_orientation(&self) -> bool {
        #[cfg(feature = "iio-sensor-proxy")]
        if let Some(iio_sensor) = &self.iio_sensor {
            return iio_sensor.has_accelerometer().await.unwrap_or_default();
        }

        self.service.has_orientation().await.unwrap_or_default()
    }
}
