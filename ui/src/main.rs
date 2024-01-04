use appindicator3::{prelude::*, Indicator, IndicatorCategory, IndicatorStatus};
use gtk::prelude::*;
use smol::{
    channel::{Receiver, Sender},
    future::FutureExt,
    stream::StreamExt,
};
use tablet_assist_agent::{AgentProxy, Orientation};
use zbus::Connection;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("DBus error: {0}")]
    DBus(#[from] zbus::Error),
    #[error("DBus FDO error: {0}")]
    DBusFdo(#[from] zbus::fdo::Error),
    #[error("Channel send error")]
    Send,
    #[error("Channel receive error")]
    Recv,
}

impl<T> From<smol::channel::SendError<T>> for Error {
    fn from(_: smol::channel::SendError<T>) -> Self {
        Self::Send
    }
}

impl From<smol::channel::RecvError> for Error {
    fn from(_: smol::channel::RecvError) -> Self {
        Self::Recv
    }
}

pub enum Action {
    AutoTabletMode(bool),
    TabletMode(bool),
    AutoOrientation(bool),
    Orientation(Orientation),
}

pub enum Update {
    AutoTabletMode(bool),
    TabletModeDetection(bool),
    TabletMode(bool),
    AutoOrientation(bool),
    OrientationDetection(bool),
    Orientation(Orientation),
}

async fn agent(actions: Receiver<Action>, updates: Sender<Update>) -> Result<()> {
    let connection = Connection::session().await?;

    let agent = AgentProxy::builder(&connection)
        .cache_properties(zbus::CacheProperties::No)
        .build()
        .await?;

    updates
        .send(Update::AutoTabletMode(agent.auto_tablet_mode().await?))
        .await?;
    updates
        .send(Update::TabletMode(agent.tablet_mode().await?))
        .await?;
    updates
        .send(Update::TabletModeDetection(
            agent.tablet_mode_detection().await?,
        ))
        .await?;

    updates
        .send(Update::AutoOrientation(agent.auto_orientation().await?))
        .await?;
    updates
        .send(Update::Orientation(agent.orientation().await?))
        .await?;
    updates
        .send(Update::OrientationDetection(
            agent.orientation_detection().await?,
        ))
        .await?;

    async fn update_auto_tablet_mode(updates: Sender<Update>, agent: AgentProxy<'_>) -> Result<()> {
        let mut changes = agent.receive_auto_tablet_mode_changed().await;
        while changes.next().await.is_some() {
            updates
                .send(Update::AutoTabletMode(agent.auto_tablet_mode().await?))
                .await?;
        }
        Ok(())
    }

    async fn update_tablet_mode_detection(
        updates: Sender<Update>,
        agent: AgentProxy<'_>,
    ) -> Result<()> {
        let mut changes = agent.receive_tablet_mode_detection_changed().await;
        while changes.next().await.is_some() {
            updates
                .send(Update::TabletModeDetection(
                    agent.tablet_mode_detection().await?,
                ))
                .await?;
        }
        Ok(())
    }

    async fn update_tablet_mode(updates: Sender<Update>, agent: AgentProxy<'_>) -> Result<()> {
        let mut changes = agent.receive_tablet_mode_changed().await;
        while changes.next().await.is_some() {
            updates
                .send(Update::TabletMode(agent.tablet_mode().await?))
                .await?;
        }
        Ok(())
    }

    async fn update_auto_orientation(updates: Sender<Update>, agent: AgentProxy<'_>) -> Result<()> {
        let mut changes = agent.receive_auto_orientation_changed().await;
        while changes.next().await.is_some() {
            updates
                .send(Update::AutoOrientation(agent.auto_orientation().await?))
                .await?;
        }
        Ok(())
    }

    async fn update_orientation_detection(
        updates: Sender<Update>,
        agent: AgentProxy<'_>,
    ) -> Result<()> {
        let mut changes = agent.receive_orientation_detection_changed().await;
        while changes.next().await.is_some() {
            updates
                .send(Update::OrientationDetection(
                    agent.orientation_detection().await?,
                ))
                .await?;
        }
        Ok(())
    }

    async fn update_orientation(updates: Sender<Update>, agent: AgentProxy<'_>) -> Result<()> {
        let mut changes = agent.receive_orientation_changed().await;
        while changes.next().await.is_some() {
            updates
                .send(Update::Orientation(agent.orientation().await?))
                .await?;
        }
        Ok(())
    }

    async fn process_actions(actions: Receiver<Action>, agent: AgentProxy<'_>) -> Result<()> {
        while let Ok(action) = actions.recv().await {
            match action {
                Action::AutoTabletMode(is) => agent.set_auto_tablet_mode(is).await?,
                Action::TabletMode(mode) => agent.set_tablet_mode(mode).await?,
                Action::AutoOrientation(is) => agent.set_auto_orientation(is).await?,
                Action::Orientation(orientation) => agent.set_orientation(orientation).await?,
            }
        }
        Ok(())
    }

    update_auto_tablet_mode(updates.clone(), agent.clone())
        .race(update_tablet_mode_detection(updates.clone(), agent.clone()))
        .race(update_tablet_mode(updates.clone(), agent.clone()))
        .race(update_auto_orientation(updates.clone(), agent.clone()))
        .race(update_orientation_detection(updates.clone(), agent.clone()))
        .race(update_orientation(updates.clone(), agent.clone()))
        .race(process_actions(actions, agent.clone()))
        .await?;

    Ok(())
}

fn main() {
    gtk::init().unwrap();

    let (action_sender, action_receiver) = smol::channel::bounded(1);
    let (update_sender, update_receiver) = smol::channel::bounded(10);

    let tablet_mode = gtk::CheckMenuItem::with_label("Tablet mode");
    tablet_mode.connect_toggled({
        let sender = action_sender.clone();
        move |tablet_mode| {
            let _ = sender.try_send(Action::TabletMode(tablet_mode.is_active()));
        }
    });

    let auto_tablet_mode = gtk::CheckMenuItem::with_label("Auto tablet");
    auto_tablet_mode.connect_toggled({
        let sender = action_sender.clone();
        let tablet_mode = tablet_mode.clone();
        move |auto_tablet_mode| {
            let _ = sender.try_send(Action::AutoTabletMode(auto_tablet_mode.is_active()));
            tablet_mode.set_sensitive(!auto_tablet_mode.is_active());
        }
    });
    //auto_tablet_mode.set_active(true);

    let top_up = gtk::RadioMenuItem::with_label("Top up");
    top_up.connect_toggled({
        let sender = action_sender.clone();
        move |top_up| {
            if top_up.is_active() {
                let _ = sender.try_send(Action::Orientation(Orientation::TopUp));
            }
        }
    });

    let left_up = gtk::RadioMenuItem::with_label_from_widget(&top_up, Some("Left up"));
    left_up.connect_toggled({
        let sender = action_sender.clone();
        move |left_up| {
            if left_up.is_active() {
                let _ = sender.try_send(Action::Orientation(Orientation::LeftUp));
            }
        }
    });

    let right_up = gtk::RadioMenuItem::with_label_from_widget(&top_up, Some("Right up"));
    right_up.connect_toggled({
        let sender = action_sender.clone();
        move |right_up| {
            if right_up.is_active() {
                let _ = sender.try_send(Action::Orientation(Orientation::RightUp));
            }
        }
    });

    let bottom_up = gtk::RadioMenuItem::with_label_from_widget(&top_up, Some("Bottom up"));
    bottom_up.connect_toggled({
        let sender = action_sender.clone();
        move |bottom_up| {
            if bottom_up.is_active() {
                let _ = sender.try_send(Action::Orientation(Orientation::BottomUp));
            }
        }
    });

    let auto_orientation = gtk::CheckMenuItem::with_label("Auto rotate");
    auto_orientation.connect_toggled({
        let sender = action_sender.clone();
        let orientation = top_up.clone();
        move |auto_orientation| {
            let _ = sender.try_send(Action::AutoOrientation(auto_orientation.is_active()));
            for radio in orientation.group() {
                radio.set_sensitive(!auto_orientation.is_active());
            }
        }
    });
    //auto_orientation.set_active(true);

    let exit = gtk::MenuItem::with_label("Quit");
    exit.connect_activate(|_| {
        let dialog = gtk::MessageDialog::new(
            None as Option<&gtk::Window>,
            gtk::DialogFlags::empty(),
            gtk::MessageType::Question,
            gtk::ButtonsType::OkCancel,
            "Exit now",
        );
        dialog.connect_response(|dialog, resp| {
            dialog.emit_close();
            if resp == gtk::ResponseType::Ok {
                gtk::main_quit();
            }
        });
        dialog.show();
    });

    let menu = gtk::Menu::new();

    menu.add(&auto_tablet_mode);
    menu.add(&tablet_mode);
    menu.add(&gtk::SeparatorMenuItem::new());
    menu.add(&auto_orientation);
    menu.add(&top_up);
    menu.add(&left_up);
    menu.add(&right_up);
    menu.add(&bottom_up);
    menu.add(&gtk::SeparatorMenuItem::new());
    menu.add(&exit);

    menu.show_all();

    let _indicator = Indicator::builder("Tablet mode")
        .category(IndicatorCategory::ApplicationStatus)
        .menu(&menu)
        .icon("input-tablet", "icon")
        .status(IndicatorStatus::Active)
        .build();

    glib::spawn_future_local(async move {
        while let Ok(update) = update_receiver.recv().await {
            match update {
                Update::AutoTabletMode(is) => auto_tablet_mode.set_active(is),
                Update::TabletModeDetection(has) => auto_tablet_mode.set_sensitive(has),
                Update::TabletMode(mode) => tablet_mode.set_active(mode),
                Update::AutoOrientation(is) => auto_orientation.set_active(is),
                Update::OrientationDetection(has) => auto_orientation.set_sensitive(has),
                Update::Orientation(orientation) => match orientation {
                    Orientation::TopUp => top_up.set_active(true),
                    Orientation::LeftUp => left_up.set_active(true),
                    Orientation::RightUp => right_up.set_active(true),
                    Orientation::BottomUp => bottom_up.set_active(true),
                },
            }
        }
    });

    glib::spawn_future_local(agent(action_receiver, update_sender));

    gtk::main();
}
