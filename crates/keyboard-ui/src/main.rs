mod app;
mod chord;
mod dbus;
mod keys;
mod kle;
mod lighting;
mod macros;
mod model;
mod settings;
mod style;

use adw::prelude::*;
use gtk::glib;

const APP_ID: &str = "io.github.jkli_2.anko_keyboard_configurator";

fn main() -> glib::ExitCode {
    let application = adw::Application::builder().application_id(APP_ID).build();
    application.connect_activate(app::build_ui);
    application.run()
}
