use async_signal::{Signal, Signals};
use smol::{future::FutureExt, stream::StreamExt};
use smol_potat::main;
use zbus::ConnectionBuilder;

mod args;
mod config;
mod error;
#[cfg(feature = "iio")]
mod iio_iface;
#[cfg(feature = "input")]
mod input_iface;
mod service;
mod types;

use args::*;
use config::*;
use error::*;
#[cfg(feature = "iio")]
use iio_iface::*;
#[cfg(feature = "input")]
use input_iface::*;
use service::*;
use types::*;

#[main]
async fn main() -> Result<()> {
    let args = Args::new();

    if args.version {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        if !env!("CARGO_PKG_DESCRIPTION").is_empty() {
            println!("{}", env!("CARGO_PKG_DESCRIPTION"));
        }
        return Ok(());
    }

    #[cfg(feature = "tracing-subscriber")]
    if let Some(log) = args.log {
        use tracing_subscriber::prelude::*;

        let registry = tracing_subscriber::registry().with(log);

        #[cfg(all(feature = "stderr", feature = "journal"))]
        let registry = registry.with(if !args.journal {
            Some(tracing_subscriber::fmt::Layer::default().with_writer(std::io::stderr))
        } else {
            None
        });

        #[cfg(all(feature = "stderr", not(feature = "journal")))]
        let registry =
            registry.with(tracing_subscriber::fmt::Layer::default().with_writer(std::io::stderr));

        #[cfg(feature = "journal")]
        let registry = registry.with(if args.journal {
            Some(tracing_journald::Layer::new()?)
        } else {
            None
        });

        registry.init();
    }

    tracing::info!("Start");

    #[cfg(any(feature = "libinput", feature = "iio"))]
    let config = if let Some(path) = &args.config {
        Config::from_file(path).await?
    } else {
        Config::default()
    };

    #[cfg(feature = "input")]
    let input_devices = config.find_input_devices()?;

    #[cfg(feature = "iio")]
    let iio_devices = config.find_iio_devices()?;

    if !args.dbus {
        return Ok(());
    }

    let mut signals = Signals::new([Signal::Term, Signal::Quit, Signal::Int])?;

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
        match signals.next().await {
            Some(Ok(sig)) => {
                tracing::info!("Received signal {:?}", sig);
                Ok(Some(sig))
            }
            Some(Err(error)) => {
                tracing::error!("Signal error: {error}");
                Err(Error::from(error))
            }
            None => {
                tracing::error!("Signal receiver terminated");
                Err(Error::Term)
            }
        }
    }
    .boxed_local();

    #[cfg(feature = "input")]
    let tasks = if !input_devices.is_empty() {
        // Add input task
        tasks
            .race(Input::process(input_devices, service.clone()))
            .boxed_local()
    } else {
        tasks
    };

    #[cfg(feature = "iio")]
    let tasks = if !iio_devices.is_empty() {
        // Add iio task
        tasks
            .race(Iio::process(
                iio_devices,
                service.clone(),
                &config.orientation,
            ))
            .boxed_local()
    } else {
        tasks
    };

    let res = tasks.await;

    drop(connection);

    tracing::info!("Stop");

    match res {
        Ok(Some(sig)) => {
            signal_hook::low_level::emulate_default_handler(sig as _)?;
            Ok(())
        }
        Err(error) => Err(error),
        _ => Ok(()),
    }
}
