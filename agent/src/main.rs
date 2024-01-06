use async_signal::{Signal, Signals};
use smol::stream::StreamExt;
use smol_potat::main;
use tablet_assist_service::ServiceProxy;
use zbus::ConnectionBuilder;

mod agent;
mod args;
mod config;
mod error;
mod types;
mod xclient;

use agent::*;
use args::*;
use config::*;
use error::*;
use types::*;
use xclient::*;

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

    let mut config = ConfigHolder::<Config>::from_path(&args.config);
    config.load().await?;

    let agent_name = "tablet.assist.Agent";
    let agent_path = "/tablet/assist";

    let agent = Agent::new(config).await?;

    let connection = ConnectionBuilder::session()?
        .name(agent_name)?
        .serve_at(agent_path, agent.clone())?
        .build()
        .await?;

    agent
        .init(connection.object_server().interface(agent_path).await?)
        .await?;

    let mut signals = Signals::new([Signal::Term, Signal::Quit, Signal::Int])?;

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
