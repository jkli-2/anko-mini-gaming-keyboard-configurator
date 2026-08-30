use adw::prelude::*;

use crate::kle::{custom_kle_path, validate_kle_layout};
use crate::profile::{DeviceProfile, load_active_profile, save_active_profile, write_profile_file};

#[derive(Clone)]
pub(crate) struct SettingsPage {
    pub root: gtk::ScrolledWindow,
    pub info: gtk::Label,
    pub error: gtk::Label,
    pub factory_reset: gtk::Button,
    pub backup_profile: gtk::Button,
    pub restore_profile: gtk::Button,
    profile_name: gtk::Entry,
    profile_status: gtk::Label,
    profile_error: gtk::Label,
    export_profile: gtk::Button,
}

impl SettingsPage {
    pub(crate) fn profile_name(&self) -> String {
        self.profile_name.text().to_string()
    }

    pub(crate) fn set_profile_message(&self, message: &str) {
        self.profile_status.set_text(message);
        self.profile_error.set_text("");
        self.profile_error.set_visible(false);
    }

    pub(crate) fn set_profile_error(&self, message: &str) {
        self.profile_error.set_text(message);
        self.profile_error.set_visible(true);
    }

    pub(crate) fn refresh_profile(&self) {
        refresh_profile_widgets(
            &self.profile_name,
            &self.profile_status,
            &self.profile_error,
            &self.restore_profile,
            &self.export_profile,
        );
    }
}

fn store_custom_layout(source: &str) -> Result<(), String> {
    validate_kle_layout(source)?;
    let path = custom_kle_path();
    let parent = path
        .parent()
        .ok_or_else(|| "Custom layout path has no parent".to_string())?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    std::fs::write(path, source).map_err(|error| error.to_string())
}

fn info_row(title: &str, value: &gtk::Label) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    row.set_margin_top(10);
    row.set_margin_bottom(10);
    row.set_margin_start(16);
    row.set_margin_end(16);

    let title = gtk::Label::new(Some(title));
    title.set_xalign(0.0);
    title.set_hexpand(true);

    value.set_xalign(1.0);
    value.set_selectable(true);
    value.add_css_class("dim-label");

    row.append(&title);
    row.append(value);
    row
}

fn append_separator(card: &gtk::Box) {
    card.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
}

pub(crate) fn settings_page() -> SettingsPage {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 0);
    page.set_margin_top(18);
    page.set_margin_bottom(18);
    page.set_margin_start(24);
    page.set_margin_end(24);

    // Keep the settings content at a comfortable reading width, similar to
    // GNOME Settings' About page.
    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(900);
    clamp.set_tightening_threshold(700);
    clamp.set_hexpand(true);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 18);
    content.set_hexpand(true);

    // ---------------------------------------------------------------------
    // About
    // ---------------------------------------------------------------------
    let info_section = gtk::Box::new(gtk::Orientation::Vertical, 8);

    let info_title = gtk::Label::new(Some("About"));
    info_title.add_css_class("heading");
    info_title.set_xalign(0.0);
    info_section.append(&info_title);

    let info_card = gtk::Box::new(gtk::Orientation::Vertical, 0);
    info_card.add_css_class("card");

    let connection_value = gtk::Label::new(Some("Connecting…"));
    let product_value = gtk::Label::new(Some("—"));
    let firmware_value = gtk::Label::new(Some("—"));
    let protocol_value = gtk::Label::new(Some("—"));
    let layers_value = gtk::Label::new(Some("—"));
    let app_value = gtk::Label::new(Some(env!("CARGO_PKG_VERSION")));

    info_card.append(&info_row("Connection", &connection_value));
    append_separator(&info_card);
    info_card.append(&info_row("Product", &product_value));
    append_separator(&info_card);
    info_card.append(&info_row("Firmware", &firmware_value));
    append_separator(&info_card);
    info_card.append(&info_row("Protocol", &protocol_value));
    append_separator(&info_card);
    info_card.append(&info_row("Layers", &layers_value));
    append_separator(&info_card);
    info_card.append(&info_row("Application version", &app_value));

    info_section.append(&info_card);
    content.append(&info_section);

    // Keep the original info label as the public update target used by app.rs,
    // but turn it into an internal source for the structured rows above.
    let info = gtk::Label::new(Some("Waiting for keyboardd…"));
    info.set_visible(false);

    {
        let connection_value = connection_value.clone();
        let product_value = product_value.clone();
        let firmware_value = firmware_value.clone();
        let protocol_value = protocol_value.clone();
        let layers_value = layers_value.clone();

        info.connect_label_notify(move |label| {
            let mut connection = None;
            let mut product = None;
            let mut firmware = None;
            let mut protocol = None;
            let mut layers = None;

            for line in label.label().lines() {
                let Some((key, value)) = line.split_once(':') else {
                    continue;
                };
                let value = value.trim();
                match key.trim().to_ascii_lowercase().as_str() {
                    "connection" => connection = Some(value.to_string()),
                    "product" => product = Some(value.to_string()),
                    "firmware" => firmware = Some(value.to_string()),
                    "protocol" => protocol = Some(value.to_string()),
                    "layers" => layers = Some(value.to_string()),
                    _ => {}
                }
            }

            connection_value.set_text(connection.as_deref().unwrap_or("Waiting…"));
            product_value.set_text(product.as_deref().unwrap_or("—"));
            firmware_value.set_text(firmware.as_deref().unwrap_or("—"));
            protocol_value.set_text(protocol.as_deref().unwrap_or("—"));
            layers_value.set_text(layers.as_deref().unwrap_or("—"));
        });
    }

    // ---------------------------------------------------------------------
    // Device profile
    // ---------------------------------------------------------------------
    let profile_section = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let profile_title = gtk::Label::new(Some("Device profile"));
    profile_title.add_css_class("heading");
    profile_title.set_xalign(0.0);
    profile_section.append(&profile_title);

    let profile_card = gtk::Box::new(gtk::Orientation::Vertical, 0);
    profile_card.add_css_class("card");
    let profile_content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    profile_content.set_margin_top(16);
    profile_content.set_margin_bottom(16);
    profile_content.set_margin_start(16);
    profile_content.set_margin_end(16);

    let profile_name = gtk::Entry::builder()
        .placeholder_text("Keyboard Backup")
        .max_length(64)
        .build();
    let profile_name_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let profile_name_label = gtk::Label::new(Some("Profile name"));
    profile_name_label.set_xalign(0.0);
    profile_name_label.set_hexpand(true);
    profile_name.set_hexpand(true);
    profile_name_row.append(&profile_name_label);
    profile_name_row.append(&profile_name);

    let profile_status = gtk::Label::new(Some("No local profile"));
    profile_status.set_xalign(0.0);
    profile_status.set_wrap(true);
    profile_status.add_css_class("dim-label");
    let profile_hint = gtk::Label::new(Some(
        "Backups include both complete keymaps, lighting, raw macro storage, the RGB storage map, and local macro names/steps. Importing a file does not write to the keyboard.",
    ));
    profile_hint.set_xalign(0.0);
    profile_hint.set_wrap(true);
    profile_hint.add_css_class("dim-label");
    let profile_error = gtk::Label::new(None);
    profile_error.set_xalign(0.0);
    profile_error.set_wrap(true);
    profile_error.add_css_class("error");
    profile_error.set_visible(false);

    let profile_actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    profile_actions.set_halign(gtk::Align::End);
    let import_profile = gtk::Button::with_label("Import…");
    let export_profile = gtk::Button::with_label("Export…");
    let restore_profile = gtk::Button::with_label("Restore");
    let backup_profile = gtk::Button::with_label("Back Up Now");
    backup_profile.add_css_class("suggested-action");
    profile_actions.append(&import_profile);
    profile_actions.append(&export_profile);
    profile_actions.append(&restore_profile);
    profile_actions.append(&backup_profile);

    profile_content.append(&profile_name_row);
    profile_content.append(&profile_status);
    profile_content.append(&profile_hint);
    profile_content.append(&profile_error);
    profile_content.append(&profile_actions);
    profile_card.append(&profile_content);
    profile_section.append(&profile_card);
    content.append(&profile_section);

    refresh_profile_widgets(
        &profile_name,
        &profile_status,
        &profile_error,
        &restore_profile,
        &export_profile,
    );

    {
        let profile_name = profile_name.clone();
        let profile_status = profile_status.clone();
        let profile_error = profile_error.clone();
        let restore_profile = restore_profile.clone();
        let export_profile = export_profile.clone();
        import_profile.connect_clicked(move |button| {
            let filter = gtk::FileFilter::new();
            filter.set_name(Some("Anko keyboard profiles"));
            filter.add_pattern("*.json");
            let parent = button.root().and_downcast::<gtk::Window>();
            let dialog = gtk::FileChooserNative::new(
                Some("Import keyboard profile"),
                parent.as_ref(),
                gtk::FileChooserAction::Open,
                Some("Import"),
                Some("Cancel"),
            );
            dialog.add_filter(&filter);
            let profile_name = profile_name.clone();
            let profile_status = profile_status.clone();
            let profile_error = profile_error.clone();
            let restore_profile = restore_profile.clone();
            let export_profile = export_profile.clone();
            dialog.connect_response(move |dialog, response| {
                if response != gtk::ResponseType::Accept {
                    return;
                }
                let result = dialog
                    .file()
                    .and_then(|file| file.path())
                    .ok_or_else(|| "The selected profile is not locally readable".to_string())
                    .and_then(|path| std::fs::read_to_string(path).map_err(|e| e.to_string()))
                    .and_then(|source| DeviceProfile::from_json(&source))
                    .and_then(|profile| save_active_profile(&profile));
                match result {
                    Ok(()) => refresh_profile_widgets(
                        &profile_name,
                        &profile_status,
                        &profile_error,
                        &restore_profile,
                        &export_profile,
                    ),
                    Err(error) => {
                        profile_error.set_text(&error);
                        profile_error.set_visible(true);
                    }
                }
            });
            dialog.show();
        });
    }

    {
        let profile_error = profile_error.clone();
        export_profile.connect_clicked(move |button| {
            let profile = match load_active_profile() {
                Ok(Some(profile)) => profile,
                Ok(None) => {
                    profile_error.set_text("There is no local profile to export");
                    profile_error.set_visible(true);
                    return;
                }
                Err(error) => {
                    profile_error.set_text(&error);
                    profile_error.set_visible(true);
                    return;
                }
            };
            let parent = button.root().and_downcast::<gtk::Window>();
            let dialog = gtk::FileChooserNative::new(
                Some("Export keyboard profile"),
                parent.as_ref(),
                gtk::FileChooserAction::Save,
                Some("Export"),
                Some("Cancel"),
            );
            dialog.set_current_name("anko-keyboard-profile.json");
            let profile_error = profile_error.clone();
            dialog.connect_response(move |dialog, response| {
                if response != gtk::ResponseType::Accept {
                    return;
                }
                let result = dialog
                    .file()
                    .and_then(|file| file.path())
                    .ok_or_else(|| "The selected destination is not writable".to_string())
                    .and_then(|path| write_profile_file(&path, &profile));
                match result {
                    Ok(()) => {
                        profile_error.set_text("");
                        profile_error.set_visible(false);
                    }
                    Err(error) => {
                        profile_error.set_text(&error);
                        profile_error.set_visible(true);
                    }
                }
            });
            dialog.show();
        });
    }

    // ---------------------------------------------------------------------
    // Keyboard layout
    // ---------------------------------------------------------------------
    let layout_section = gtk::Box::new(gtk::Orientation::Vertical, 8);

    let layout_title = gtk::Label::new(Some("Keyboard layout"));
    layout_title.add_css_class("heading");
    layout_title.set_xalign(0.0);
    layout_section.append(&layout_title);

    let layout_card = gtk::Box::new(gtk::Orientation::Vertical, 0);
    layout_card.add_css_class("card");

    let layout_content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    layout_content.set_margin_top(16);
    layout_content.set_margin_bottom(16);
    layout_content.set_margin_start(16);
    layout_content.set_margin_end(16);

    let custom_active = custom_kle_path().exists();

    let layout_status = gtk::Label::new(Some(if custom_active {
        "Custom layout"
    } else {
        "Default layout"
    }));
    layout_status.set_xalign(0.0);
    layout_status.add_css_class("heading");

    let layout_hint = gtk::Label::new(None);
    let reference_url = "https://www.keyboard-layout-editor.com/##@_name=Anko%20Mini%20Gaming%20Keyboard%2043721375&author=Author&notes=From%20https%2F:%2F%2F%2F%2Fwww.kmart.com.au%2F%2Fproduct%2F%2Fmini-gaming-keyboard-43721375%2F%2F&switchMount=cherry&css=*%20%7B%0A%20%20%20%20font-weight%2F:%20bold%2F%3B%0A%7D%0A%0A.fa%20%7B%0A%20%20font-family%2F:%20FontAwesome%20!important%2F%3B%0A%7D%3B&@_c=%23ffc100&t=%234a4a4a&sm=cherry&f:4%3B&=%0A%60%0A%0A~%0A%0A%0A%0A%0AEsc&_c=%23f1ede6%3B&=1%0AF1%0A!%0A%3Ci%20class%2F='fa%20fa-music'%3E%3C%2F%2Fi%3E&=2%0AF2%0A%2F@%0A%3Ci%20class%2F='fa%20fa-volume-off'%3E%3C%2F%2Fi%3E%20-&=3%0AF3%0A%23%0A%3Ci%20class%2F='fa%20fa-volume-off'%3E%3C%2F%2Fi%3E%20+&=4%0AF4%0A$%0A%3Ci%20class%2F='fa%20fa-volume-off'%3E%3C%2F%2Fi%3E%20%C3%97&_c=%232d3238&t=%23f1ede6%3B&=5%0AF5%0A%25%0A%3Ci%20class%2F='fa%20fa-stop'%3E%3C%2F%2Fi%3E&=6%0AF6%0A%5E%0A%3Ci%20class%2F='fa%20fa-fast-backward'%3E%3C%2F%2Fi%3E&=7%0AF7%0A%2F&%0A%3Ci%20class%2F='fa%20fa-play'%3E%3C%2F%2Fi%3E%2F%2F%3Ci%20class%2F='fa%20fa-pause'%3E%3C%2F%2Fi%3E&=8%0AF8%0A*%0A%3Ci%20class%2F='fa%20fa-fast-forward'%3E%3C%2F%2Fi%3E&_c=%23f1ede6&t=%234a4a4a%3B&=9%0AF9%0A(%0A%3Ci%20class%2F='fa%20fa-comments-o'%3E%3C%2F%2Fi%3E&=0%0AF10%0A)%0A%3Ci%20class%2F='fa%20fa-home'%3E%3C%2F%2Fi%3E&=-%0AF11%0A%2F_%0A%3Ci%20class%2F='fa%20fa-laptop'%3E%3C%2F%2Fi%3E&=%2F=%0AF12%0A+%0A%3Ci%20class%2F='fa%20fa-calculator'%3E%3C%2F%2Fi%3E&_c=%232d3238&t=%23f1ede6&w:2%3B&=%0A%0A%0ADel%0A%0A%0A%0A%0A%3Ci%20class%2F='fa%20fa-long-arrow-left'%3E%3C%2F%2Fi%3E%3B&@_a:5&w:1.5%3B&=Tab&_c=%23f1ede6&t=%234a4a4a%3B&=Q&=W%0A%3Ci%20class%2F='fa%20fa-angle-up'%3E%3C%2F%2Fi%3E%0A%0A%0A%0A%0A%E2%87%84&=E&=R&=T&=Y%0APrtSc&=U%0AScrLk&=I%0APause&=O&=P&_a:4%3B&=%5B%0A%0A%0A%0A%0A%0A%0A%0A%7B&=%5D%0A%0A%0A%0A%0A%0A%0A%0A%7D&_w:1.5%3B&=%5C%0A%0A%0A%0A%0A%0A%0A%0A%7C%0A%0A%3Ci%20class%2F='fa%20fa-lightbulb-o'%3E%3C%2F%2Fi%3E%3B&@_c=%232d3238&t=%23f1ede6&a:5&w:1.75%3B&=Caps&_c=%23f1ede6&t=%234a4a4a%3B&=A%0A%3Ci%20class%2F='fa%20fa-angle-left'%3E%3C%2F%2Fi%3E&=S%0A%3Ci%20class%2F='fa%20fa-angle-down'%3E%3C%2F%2Fi%3E&=D%0A%3Ci%20class%2F='fa%20fa-angle-right'%3E%3C%2F%2Fi%3E&_n:true%3B&=F&=G&=H%0AIns&_n:true%3B&=J%0AHome&=K%0APgUp&=L&_a:4%3B&=%2F%3B%0A%0A%0A%0A%0A%0A%0A%0A%2F:&='%0A%0A%0A%0A%0A%0A%0A%0A%22&_c=%232d3238&t=%23f1ede6&a:5&w:2.25%3B&=Enter%0A%3Ci%20class%2F='fa%20fa-lightbulb-o'%3E%3C%2F%2Fi%3E%20%C3%97%3B&@_w:2.25%3B&=Shift&_c=%23f1ede6&t=%234a4a4a%3B&=Z%0AWin&=X%0AMac&=C&=V&=B&=N%0ADel&=M%0AEnd&_a:4%3B&=,%0A%0A%0A%0A%0A%0A%0A%0A%3C%0A%0APgDn&=.%0A%0A%0A%0A%0A%0A%0A%0A%3E&=%2F%2F%0A%0A%0A%0A%0A%0A%0A%0A%3F&_c=%23ffc100&a:5%3B&=%3Ci%20class%2F='fa%20fa-caret-up'%3E%3C%2F%2Fi%3E%0A%3Ci%20class%2F='fa%20fa-lightbulb-o'%3E%3C%2F%2Fi%3E%20+&_c=%232d3238&t=%23f1ede6&w:1.75%3B&=Shift%3B&@_w:1.25%3B&=Ctrl%0AM%E2%87%84V&_w:1.25%3B&=%3Ci%20class%2F='fa%20fa-windows'%3E%3C%2F%2Fi%3E%0A%3Ci%20class%2F='fa%20fa-lock'%3E%3C%2F%2Fi%3E&_w:1.25%3B&=Alt&_c=%23ffc100&t=%234a4a4a&w:6.25%3B&=%E2%8E%AF%E2%8E%AF%E2%8E%AF%E2%8E%AF%E2%8E%AF%E2%8E%AF&_c=%232d3238&t=%23f1ede6%3B&=Alt&_c=%23ffc100&t=%234a4a4a%3B&=%3Ci%20class%2F='fa%20fa-caret-left'%3E%3C%2F%2Fi%3E%0A-%3Ci%20class%2F='fa%20fa-male'%3E%3C%2F%2Fi%3E&=%3Ci%20class%2F='fa%20fa-caret-down'%3E%3C%2F%2Fi%3E%0A%3Ci%20class%2F='fa%20fa-lightbulb-o'%3E%3C%2F%2Fi%3E%20-&=%3Ci%20class%2F='fa%20fa-caret-right'%3E%3C%2F%2Fi%3E%0A%3Ci%20class%2F='fa%20fa-male'%3E%3C%2F%2Fi%3E+&_c=%232d3238&t=%23f1ede6%3B&=Fn";
    let escaped_url = gtk::glib::markup_escape_text(reference_url);
    layout_hint.set_markup(&format!(
        r#"Import a Keyboard Layout Editor (KLE) JSON layout for visual customization of the Keys page, such as keycap and legend colours. The layout must match the Anko Mini Gaming Keyboard’s physical key arrangement; incompatible layouts may not render correctly.

Changes apply after restarting the app. <a href="https://www.keyboard-layout-editor.com/">Learn more</a> · <a href="{escaped_url}">Reference layout (KLE Permalink)</a>."#
    ));
    layout_hint.set_use_markup(true);
    layout_hint.set_xalign(0.0);
    layout_hint.set_wrap(true);
    layout_hint.add_css_class("dim-label");

    let layout_error = gtk::Label::new(None);
    layout_error.set_xalign(0.0);
    layout_error.set_wrap(true);
    layout_error.add_css_class("error");
    layout_error.set_visible(false);

    layout_content.append(&layout_status);
    layout_content.append(&layout_hint);
    layout_content.append(&layout_error);

    let layout_actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    layout_actions.set_halign(gtk::Align::End);

    let import_layout = gtk::Button::with_label("Import JSON…");
    let use_default = gtk::Button::with_label("Use Default");
    use_default.set_sensitive(custom_active);

    layout_actions.append(&import_layout);
    layout_actions.append(&use_default);
    layout_content.append(&layout_actions);

    layout_card.append(&layout_content);
    layout_section.append(&layout_card);
    content.append(&layout_section);

    {
        let layout_status = layout_status.clone();
        let layout_error = layout_error.clone();
        let use_default = use_default.clone();

        import_layout.connect_clicked(move |button| {
            let filter = gtk::FileFilter::new();
            filter.set_name(Some("KLE JSON files"));
            filter.add_pattern("*.json");

            let parent = button.root().and_downcast::<gtk::Window>();
            let dialog = gtk::FileChooserNative::new(
                Some("Import KLE layout"),
                parent.as_ref(),
                gtk::FileChooserAction::Open,
                Some("Import"),
                Some("Cancel"),
            );
            dialog.add_filter(&filter);

            let layout_status = layout_status.clone();
            let layout_error = layout_error.clone();
            let use_default = use_default.clone();

            dialog.connect_response(move |dialog, response| {
                if response != gtk::ResponseType::Accept {
                    return;
                }

                let Some(file) = dialog.file() else {
                    layout_error.set_text("No layout file was selected");
                    layout_error.set_visible(true);
                    return;
                };

                let Some(path) = file.path() else {
                    layout_error.set_text("The selected file is not locally readable");
                    layout_error.set_visible(true);
                    return;
                };

                let result = std::fs::read_to_string(path)
                    .map_err(|error| error.to_string())
                    .and_then(|source| store_custom_layout(&source));

                match result {
                    Ok(()) => {
                        layout_status.set_text("Custom layout — restart to apply");
                        layout_error.set_text("");
                        layout_error.set_visible(false);
                        use_default.set_sensitive(true);
                    }
                    Err(error) => {
                        layout_error.set_text(&error);
                        layout_error.set_visible(true);
                    }
                }
            });

            dialog.show();
        });
    }

    {
        let layout_status = layout_status.clone();
        let layout_error = layout_error.clone();

        use_default.connect_clicked(move |button| {
            let result = std::fs::remove_file(custom_kle_path());
            match result {
                Ok(()) => {
                    layout_status.set_text("Default layout — restart to apply");
                    layout_error.set_text("");
                    layout_error.set_visible(false);
                    button.set_sensitive(false);
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    layout_status.set_text("Default layout — restart to apply");
                    layout_error.set_text("");
                    layout_error.set_visible(false);
                    button.set_sensitive(false);
                }
                Err(error) => {
                    layout_error.set_text(&error.to_string());
                    layout_error.set_visible(true);
                }
            }
        });
    }

    // ---------------------------------------------------------------------
    // Factory reset
    // ---------------------------------------------------------------------
    let reset_section = gtk::Box::new(gtk::Orientation::Vertical, 8);

    let reset_title = gtk::Label::new(Some("Factory reset"));
    reset_title.add_css_class("heading");
    reset_title.set_xalign(0.0);
    reset_section.append(&reset_title);

    let reset_card = gtk::Box::new(gtk::Orientation::Vertical, 0);
    reset_card.add_css_class("card");

    let reset_content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    reset_content.set_margin_top(16);
    reset_content.set_margin_bottom(16);
    reset_content.set_margin_start(16);
    reset_content.set_margin_end(16);

    let reset_label = gtk::Label::new(Some("Reset factory values"));
    reset_label.set_xalign(0.0);
    reset_label.add_css_class("heading");

    let reset_hint = gtk::Label::new(Some(
        "Restore the keyboard's onboard configuration to factory defaults. \
         Custom key mappings, macros and lighting settings may be lost. \
         The keyboard may briefly disconnect or restart.",
    ));
    reset_hint.set_xalign(0.0);
    reset_hint.set_wrap(true);
    reset_hint.add_css_class("dim-label");

    reset_content.append(&reset_label);
    reset_content.append(&reset_hint);

    let reset_actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    reset_actions.set_halign(gtk::Align::End);

    let factory_reset = gtk::Button::with_label("Reset");
    factory_reset.add_css_class("destructive-action");

    reset_actions.append(&factory_reset);
    reset_content.append(&reset_actions);

    reset_card.append(&reset_content);
    reset_section.append(&reset_card);
    content.append(&reset_section);

    let error = gtk::Label::new(None);
    error.set_xalign(0.0);
    error.set_wrap(true);
    error.add_css_class("error");
    content.append(&error);

    clamp.set_child(Some(&content));
    page.append(&clamp);

    let scroll = gtk::ScrolledWindow::new();
    scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroll.set_child(Some(&page));
    scroll.set_hexpand(true);
    scroll.set_vexpand(true);

    SettingsPage {
        root: scroll,
        info,
        error,
        factory_reset,
        backup_profile,
        restore_profile,
        profile_name,
        profile_status,
        profile_error,
        export_profile,
    }
}

fn refresh_profile_widgets(
    name: &gtk::Entry,
    status: &gtk::Label,
    error: &gtk::Label,
    restore: &gtk::Button,
    export: &gtk::Button,
) {
    match load_active_profile() {
        Ok(Some(profile)) => {
            name.set_text(&profile.name);
            status.set_text(&format!(
                "Local profile · firmware {} · protocol {}",
                profile.hardware.firmware_version, profile.hardware.protocol_version
            ));
            restore.set_sensitive(true);
            export.set_sensitive(true);
            error.set_text("");
            error.set_visible(false);
        }
        Ok(None) => {
            if name.text().is_empty() {
                name.set_text("Keyboard Backup");
            }
            status.set_text("No local profile");
            restore.set_sensitive(false);
            export.set_sensitive(false);
        }
        Err(message) => {
            status.set_text("Local profile is invalid");
            restore.set_sensitive(false);
            export.set_sensitive(false);
            error.set_text(&message);
            error.set_visible(true);
        }
    }
}
