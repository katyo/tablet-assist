use appindicator3::{prelude::*, Indicator, IndicatorCategory, IndicatorStatus};
use gtk::prelude::*;

pub enum Change {
    AutoTablet(bool),
    TabletMode(bool),
    AutoRotate(bool),
    Orientation()
}

pub enum Action {

}

fn main() {
    gtk::init().unwrap();

    let tablet_mode = gtk::CheckMenuItem::with_label("Tablet mode");
    tablet_mode.connect_toggled(|_| {});

    let auto_tablet = gtk::CheckMenuItem::with_label("Auto tablet");
    auto_tablet.connect_toggled({
        let tablet_mode = tablet_mode.clone();
        move |auto_tablet| {
            tablet_mode.set_sensitive(!auto_tablet.is_active());
        }
    });
    auto_tablet.set_active(true);

    let top_up = gtk::RadioMenuItem::with_label("Top up");
    let left_up = gtk::RadioMenuItem::with_label_from_widget(&top_up, Some("Left up"));
    let right_up = gtk::RadioMenuItem::with_label_from_widget(&top_up, Some("Right up"));
    let bottom_up = gtk::RadioMenuItem::with_label_from_widget(&top_up, Some("Bottom up"));

    let auto_rotate = gtk::CheckMenuItem::with_label("Auto rotate");
    auto_rotate.connect_toggled({
        let orientation = top_up.clone();
        move |auto_rotate| {
            for radio in orientation.group() {
                radio.set_sensitive(!auto_rotate.is_active());
            }
        }
    });
    auto_rotate.set_active(true);

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

    menu.add(&auto_tablet);
    menu.add(&tablet_mode);
    menu.add(&gtk::SeparatorMenuItem::new());
    menu.add(&auto_rotate);
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

    gtk::main();
}
