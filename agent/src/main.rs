use async_signal::{Signal, Signals};
use smol::stream::StreamExt;
use smol_potat::main;
use tablet_assist_service::{Orientation, ServiceProxy};
use zbus::ConnectionBuilder;

mod agent;
mod args;
mod config;
mod error;
mod types;
mod xorg;

use agent::*;
use args::*;
use config::*;
use error::*;
use types::*;
use xorg::*;

#[main]
async fn main() -> Result<()> {
    let args = Args::new();

    if args.version {
        println!("{}", env!("CARGO_PKG_NAME"));
        println!("{}", env!("CARGO_PKG_VERSION"));
        println!("{}", env!("CARGO_PKG_DESCRIPTION"));
        return Ok(());
    }

    #[cfg(feature = "tracing-subscriber")]
    if let Some(trace) = args.trace {
        use tracing_subscriber::prelude::*;

        let registry = tracing_subscriber::registry().with(trace);

        #[cfg(feature = "stderr")]
        let registry = registry.with(if args.log {
            Some(tracing_subscriber::fmt::Layer::default().with_writer(std::io::stderr))
        } else {
            None
        });

        #[cfg(feature = "journal")]
        let registry = registry.with(if args.journal {
            Some(tracing_journald::Layer::new()?)
        } else {
            None
        });

        registry.init();
    }

    tracing::info!("Start");

    let agent_name = "tablet.assist.Agent";
    let agent_path = "/tablet/assist";

    let agent = Agent::new().await?;

    let connection = ConnectionBuilder::session()?
        .name(agent_name)?
        .serve_at(agent_path, agent.clone())?
        .build()
        .await?;

    agent
        .set_interface(connection.object_server().interface(agent_path).await?)
        .await;

    let mut signals = Signals::new(&[Signal::Term, Signal::Quit, Signal::Int])?;

    let tasks = async {
        loop {
            match signals.next().await {
                Some(Ok(sig)) => {
                    tracing::info!("Received signal {:?}", sig);

                    break Ok(Some(sig));
                }
                Some(Err(error)) => {
                    tracing::error!("Signal error: {error}");
                    break Err(Error::from(error));
                }
                None => {
                    tracing::error!("Signal receiver terminated");
                    break Err(Error::Term);
                }
            }
        }
    };

    let res = tasks.await;

    drop(agent);
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
