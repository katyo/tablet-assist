use async_signal::{Signal, Signals};
use smol::stream::StreamExt;
use smol_potat::main;
use tablet_assist_service::{Orientation, ServiceProxy};
use zbus::ConnectionBuilder;

mod agent;
mod config;
mod error;
mod types;
mod xorg;

use agent::*;
use config::*;
use error::*;
use types::*;
use xorg::*;

#[main]
async fn main() -> Result<()> {
    env_logger::init();
    log::info!("Start");

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
                    log::info!("Received signal {:?}", sig);

                    break Ok(Some(sig));
                }
                Some(Err(error)) => {
                    log::error!("Signal error: {error}");
                    break Err(Error::from(error));
                }
                None => {
                    log::error!("Signal receiver terminated");
                    break Err(Error::Term);
                }
            }
        }
    };

    let res = tasks.await;

    drop(agent);
    drop(connection);

    log::info!("Stop");

    match res {
        Ok(Some(sig)) => {
            signal_hook::low_level::emulate_default_handler(sig as _)?;
            Ok(())
        }
        Err(error) => Err(error),
        _ => Ok(()),
    }
}
