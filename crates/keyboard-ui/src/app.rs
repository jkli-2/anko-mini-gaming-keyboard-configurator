use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use adw::prelude::*;
use gtk::glib;
use keyboard_core::{HardwareSnapshot, PHYSICAL_KEYS, canonical_action_label};

use crate::dbus::{Client, Command, Event, Lighting};
use crate::keys::{ShortcutControl, compact_keycap_label, keys_page};
use crate::lighting::{ColorModeControl, DirectionControl, effect_capabilities, lighting_page};
use crate::macros::{MacroPage, macros_page};
use crate::model::KeymapDraft;
use crate::profile::{DeviceProfile, load_active_profile, save_active_profile};
use crate::settings::{SettingsPage, settings_page};
use crate::style::install_css;

struct State {
    client: Client,
    drafts: RefCell<HashMap<String, KeymapDraft>>,
    selected_key: RefCell<Option<String>>,
    updating: Cell<bool>,
    profile_action: RefCell<Option<ProfileAction>>,
}

enum ProfileAction {
    Backup { name: String, reset_after: bool },
    Restore(Box<DeviceProfile>),
}

#[derive(Clone)]
struct Widgets {
    status: gtk::Label,
    status_dot: gtk::Box,
    error: gtk::Label,
    info: gtk::Label,
    bank: gtk::DropDown,
    keys: HashMap<String, gtk::ToggleButton>,
    key_assignments: HashMap<String, gtk::Label>,
    key_legends: HashMap<String, gtk::Label>,
    selected: gtk::Label,
    current: gtk::Label,
    action: gtk::Entry,
    palette_actions: Vec<(gtk::Button, &'static str)>,
    shortcut: ShortcutControl,
    apply_map: gtk::Button,
    revert_map: gtk::Button,
    effect: gtk::DropDown,
    brightness: gtk::SpinButton,
    speed: gtk::SpinButton,
    direction: DirectionControl,
    color_mode: ColorModeControl,
    lighting_hue: gtk::SpinButton,
    lighting_saturation: gtk::SpinButton,
    lighting_value: gtk::SpinButton,
    macros: MacroPage,
    settings: SettingsPage,
}

pub(crate) fn build_ui(application: &adw::Application) {
    install_css();
    let (events_tx, events_rx) = mpsc::channel();
    let state = Rc::new(State {
        client: Client::spawn(events_tx),
        drafts: RefCell::new(HashMap::new()),
        selected_key: RefCell::new(None),
        updating: Cell::new(false),
        profile_action: RefCell::new(None),
    });
    let window = adw::ApplicationWindow::builder()
        .application(application)
        .title("Anko Keyboard")
        .default_width(900)
        .default_height(750)
        .build();
    window.set_size_request(800, 500);
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .vexpand(true)
        .build();

    let header = adw::HeaderBar::new();
    let status_box = gtk::Box::new(gtk::Orientation::Horizontal, 7);
    status_box.set_valign(gtk::Align::Center);
    status_box.set_margin_start(8);
    let status_dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    status_dot.add_css_class("status-dot");
    status_dot.add_css_class("connecting");
    status_dot.set_size_request(10, 10);
    status_dot.set_halign(gtk::Align::Center);
    status_dot.set_valign(gtk::Align::Center);
    let status = gtk::Label::new(Some("Connecting…"));
    status.add_css_class("dim-label");
    status_box.append(&status_dot);
    status_box.append(&status);
    header.pack_start(&status_box);

    // App-level navigation belongs in the header, but should look like separate
    // navigation destinations rather than a connected segmented choice.
    let header_nav = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    header_nav.set_halign(gtk::Align::Center);
    let keys_nav = gtk::Button::with_label("Keys");
    let lighting_nav = gtk::Button::with_label("Lighting");
    let macros_nav = gtk::Button::with_label("Macros");
    let settings_nav = gtk::Button::with_label("Settings");
    for button in [&keys_nav, &lighting_nav, &macros_nav, &settings_nav] {
        button.add_css_class("flat");
        button.add_css_class("header-nav");
    }
    keys_nav.add_css_class("active");
    header_nav.append(&keys_nav);
    header_nav.append(&lighting_nav);
    header_nav.append(&macros_nav);
    header_nav.append(&settings_nav);
    header.set_title_widget(Some(&header_nav));

    let refresh = gtk::Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text("Reconnect and reload")
        .build();
    header.pack_end(&refresh);
    root.append(&header);
    let (
        keys_page,
        bank,
        key_buttons,
        key_assignments,
        key_legends,
        selected,
        current,
        action,
        apply_map,
        revert_map,
        palette_actions,
        shortcut,
    ) = keys_page();
    stack.add_titled(&keys_page, Some("keys"), "Keys");
    let (
        lighting_page,
        effect,
        brightness,
        speed,
        direction,
        color_mode,
        lighting_hue,
        lighting_saturation,
        lighting_value,
        apply_lighting,
    ) = lighting_page();
    stack.add_titled(&lighting_page, Some("lighting"), "Lighting");
    let macros_page = macros_page();
    stack.add_titled(&macros_page.root, Some("macros"), "Macros");
    let settings_page = settings_page();
    stack.add_titled(&settings_page.root, Some("settings"), "Settings");

    {
        let stack = stack.clone();
        keys_nav.connect_clicked(move |_| stack.set_visible_child_name("keys"));
    }
    {
        let stack = stack.clone();
        lighting_nav.connect_clicked(move |_| stack.set_visible_child_name("lighting"));
    }
    {
        let stack = stack.clone();
        macros_nav.connect_clicked(move |_| stack.set_visible_child_name("macros"));
    }
    {
        let stack = stack.clone();
        settings_nav.connect_clicked(move |_| stack.set_visible_child_name("settings"));
    }
    {
        let keys_nav = keys_nav.clone();
        let lighting_nav = lighting_nav.clone();
        let macros_nav = macros_nav.clone();
        let settings_nav = settings_nav.clone();
        stack.connect_visible_child_name_notify(move |stack| {
            for button in [&keys_nav, &lighting_nav, &macros_nav, &settings_nav] {
                button.remove_css_class("active");
            }
            match stack.visible_child_name().as_deref() {
                Some("lighting") => lighting_nav.add_css_class("active"),
                Some("macros") => macros_nav.add_css_class("active"),
                Some("settings") => settings_nav.add_css_class("active"),
                _ => keys_nav.add_css_class("active"),
            }
        });
    }
    stack.set_visible_child_name("keys");
    root.append(&stack);
    window.set_content(Some(&root));

    let widgets = Rc::new(Widgets {
        status,
        status_dot,
        error: settings_page.error.clone(),
        info: settings_page.info.clone(),
        bank,
        keys: key_buttons,
        key_assignments,
        key_legends,
        selected,
        current,
        action,
        palette_actions,
        shortcut,
        apply_map,
        revert_map,
        effect,
        brightness,
        speed,
        direction,
        color_mode,
        lighting_hue,
        lighting_saturation,
        lighting_value,
        macros: macros_page,
        settings: settings_page.clone(),
    });
    connect_key_controls(&state, &widgets);
    connect_lighting_controls(&state, &widgets, &apply_lighting);
    {
        let client = state.client.clone();
        widgets.macros.connect_save(move |id, events| {
            if events.is_empty() {
                client.send(Command::DeleteMacro(id));
            } else {
                client.send(Command::SetMacro { id, events });
            }
        });
    }
    {
        let state = Rc::clone(&state);
        let widgets = Rc::clone(&widgets);
        let backup_profile = widgets.settings.backup_profile.clone();
        backup_profile.connect_clicked(move |_| {
            if state.profile_action.borrow().is_some() {
                widgets
                    .settings
                    .set_profile_error("A profile operation is already in progress");
                return;
            }
            widgets
                .settings
                .set_profile_message("Capturing complete device state…");
            state.profile_action.replace(Some(ProfileAction::Backup {
                name: widgets.settings.profile_name(),
                reset_after: false,
            }));
            state.client.send(Command::CaptureHardwareSnapshot);
        });
    }
    {
        let state = Rc::clone(&state);
        let widgets = Rc::clone(&widgets);
        let window = window.clone();
        let restore_profile = widgets.settings.restore_profile.clone();
        restore_profile.connect_clicked(move |_| {
            if state.profile_action.borrow().is_some() {
                widgets
                    .settings
                    .set_profile_error("A profile operation is already in progress");
                return;
            }
            let profile = match load_active_profile() {
                Ok(Some(profile)) => profile,
                Ok(None) => {
                    widgets.settings.set_profile_error("There is no local profile to restore");
                    return;
                }
                Err(error) => {
                    widgets.settings.set_profile_error(&error);
                    return;
                }
            };
            let dialog = adw::MessageDialog::new(
                Some(&window),
                Some("Restore this profile?"),
                Some(&format!(
                    "This will replace the keyboard's keymaps, macros, lighting, and stored RGB data with ‘{}’ (firmware {}, protocol {}).",
                    profile.name, profile.hardware.firmware_version, profile.hardware.protocol_version
                )),
            );
            dialog.add_response("cancel", "Cancel");
            dialog.add_response("restore", "Restore Profile");
            dialog.set_close_response("cancel");
            dialog.set_default_response(Some("cancel"));
            dialog.set_response_appearance("restore", adw::ResponseAppearance::Destructive);
            let state = Rc::clone(&state);
            let widgets = Rc::clone(&widgets);
            dialog.connect_response(None, move |dialog, response| {
                if response == "restore" {
                    if state.profile_action.borrow().is_some() {
                        widgets
                            .settings
                            .set_profile_error("A profile operation is already in progress");
                        dialog.close();
                        return;
                    }
                    match serde_json::to_string(&profile.hardware) {
                        Ok(snapshot) => {
                            widgets.settings.set_profile_message("Restoring and verifying profile…");
                            state
                                .profile_action
                                .replace(Some(ProfileAction::Restore(Box::new(profile.clone()))));
                            state
                                .client
                                .send(Command::RestoreHardwareSnapshot(snapshot));
                        }
                        Err(error) => widgets.settings.set_profile_error(&error.to_string()),
                    }
                }
                dialog.close();
            });
            dialog.present();
        });
    }
    {
        let window = window.clone();
        let state = Rc::clone(&state);
        let widgets = Rc::clone(&widgets);
        let factory_reset = widgets.settings.factory_reset.clone();
        factory_reset.connect_clicked(move |_| {
            let dialog = adw::MessageDialog::new(
                Some(&window),
                Some("Restore factory values?"),
                Some(
                    "Back up the current configuration first, or explicitly continue without a recoverable profile.",
                ),
            );
            dialog.add_response("cancel", "Cancel");
            dialog.add_response("reset", "Reset Without Backup");
            dialog.add_response("backup-reset", "Back Up and Reset");
            dialog.set_close_response("cancel");
            dialog.set_default_response(Some("backup-reset"));
            dialog.set_response_appearance("reset", adw::ResponseAppearance::Destructive);
            dialog.set_response_appearance("backup-reset", adw::ResponseAppearance::Suggested);
            let state = Rc::clone(&state);
            let widgets = Rc::clone(&widgets);
            dialog.connect_response(None, move |dialog, response| {
                if response != "cancel" && state.profile_action.borrow().is_some() {
                    widgets
                        .settings
                        .set_profile_error("A profile operation is already in progress");
                    dialog.close();
                    return;
                }
                match response {
                    "backup-reset" => {
                        widgets
                            .settings
                            .set_profile_message("Creating pre-reset backup…");
                        state.profile_action.replace(Some(ProfileAction::Backup {
                            name: "Pre-reset Backup".to_string(),
                            reset_after: true,
                        }));
                        state.client.send(Command::CaptureHardwareSnapshot);
                    }
                    "reset" => state.client.send(Command::FactoryReset),
                    _ => {}
                }
                dialog.close();
            });
            dialog.present();
        });
    }
    {
        let state = Rc::clone(&state);
        let widgets = Rc::clone(&widgets);
        refresh.connect_clicked(move |_| {
            set_status(&widgets, "Refreshing…", "connecting");
            state.client.send(Command::Refresh);
        });
    }
    {
        let state = Rc::clone(&state);
        let widgets = Rc::clone(&widgets);
        glib::timeout_add_local(Duration::from_millis(40), move || {
            while let Ok(event) = events_rx.try_recv() {
                handle_event(&state, &widgets, event);
            }
            glib::ControlFlow::Continue
        });
    }
    request_all(&state.client);
    window.present();
}

fn connect_key_controls(state: &Rc<State>, widgets: &Rc<Widgets>) {
    for (key_id, button) in &widgets.keys {
        let key_id = key_id.clone();
        let state = Rc::clone(state);
        let widgets = Rc::clone(widgets);
        button.connect_clicked(move |_| {
            state.selected_key.replace(Some(key_id.clone()));
            update_key_editor(&state, &widgets);
        });
    }
    {
        let state = Rc::clone(state);
        let widgets = Rc::clone(widgets);
        let bank = widgets.bank.clone();
        bank.connect_selected_notify(move |_| {
            state.selected_key.replace(None);
            let name = active_bank(&widgets.bank).to_string();
            if state.drafts.borrow().contains_key(&name) {
                update_key_editor(&state, &widgets);
            } else {
                state.client.send(Command::GetKeymap(name));
            }
        });
    }
    for (button, action_name) in &widgets.palette_actions {
        let state = Rc::clone(state);
        let widgets = Rc::clone(widgets);
        let action_name = (*action_name).to_string();
        button.connect_clicked(move |_| {
            let Some(key_id) = state.selected_key.borrow().clone() else {
                return;
            };
            let bank = active_bank(&widgets.bank);
            if let Some(draft) = state.drafts.borrow_mut().get_mut(bank) {
                draft.set_action(&key_id, action_name.clone());
            }
            update_key_editor(&state, &widgets);
        });
    }
    {
        let state = Rc::clone(state);
        let widgets = Rc::clone(widgets);
        let shortcut = widgets.shortcut.clone();
        shortcut.connect_assign(move |action_name| {
            let Some(key_id) = state.selected_key.borrow().clone() else {
                return;
            };
            let bank = active_bank(&widgets.bank);
            if let Some(draft) = state.drafts.borrow_mut().get_mut(bank) {
                draft.set_action(&key_id, action_name);
            }
            update_key_editor(&state, &widgets);
        });
    }
    {
        let state = Rc::clone(state);
        let widgets = Rc::clone(widgets);
        let action = widgets.action.clone();
        action.connect_changed(move |entry| {
            if state.updating.get() {
                return;
            }
            let Some(key_id) = state.selected_key.borrow().clone() else {
                return;
            };
            let bank = active_bank(&widgets.bank);
            if let Some(draft) = state.drafts.borrow_mut().get_mut(bank) {
                draft.set_action(&key_id, entry.text().to_string());
            }
            update_map_buttons(&state, &widgets);
        });
    }
    {
        let state = Rc::clone(state);
        let widgets = Rc::clone(widgets);
        let revert = widgets.revert_map.clone();
        revert.connect_clicked(move |_| {
            let bank = active_bank(&widgets.bank);
            if let Some(draft) = state.drafts.borrow_mut().get_mut(bank) {
                draft.revert();
            }
            update_key_editor(&state, &widgets);
        });
    }
    {
        let state = Rc::clone(state);
        let widgets = Rc::clone(widgets);
        let apply = widgets.apply_map.clone();
        apply.connect_clicked(move |_| {
            let bank = active_bank(&widgets.bank).to_string();
            if let Some(draft) = state.drafts.borrow().get(&bank) {
                widgets.status.set_text("Applying keymap…");
                state.client.send(Command::SetKeymap {
                    bank,
                    assignments: draft.assignments(),
                });
            }
        });
    }
}

fn connect_lighting_controls(state: &Rc<State>, widgets: &Rc<Widgets>, apply: &gtk::Button) {
    {
        let state = Rc::clone(state);
        let widgets = Rc::clone(widgets);
        apply.connect_clicked(move |_| {
            let effect = widgets.effect.selected() as u8;
            let single_colour = widgets.color_mode.is_single();
            let lighting = (
                1, // The firmware rejects other Kind values.
                effect,
                widgets.brightness.value_as_int() as u8,
                widgets.speed.value_as_int() as u8,
                widgets.direction.value(),
                single_colour,
                0, // The official client currently writes the reserved/index byte as zero.
                widgets.lighting_hue.value_as_int() as u8,
                widgets.lighting_saturation.value_as_int() as u8,
                widgets.lighting_value.value_as_int() as u8,
            );
            widgets.status.set_text(if effect == 0 {
                "Turning lighting off…"
            } else if single_colour {
                "Applying single-colour lighting…"
            } else {
                "Applying dynamic RGB lighting…"
            });
            state.client.send(Command::SetLighting(lighting));
        });
    }
    {
        let widgets = Rc::clone(widgets);
        let effect = widgets.effect.clone();
        effect.connect_selected_notify(move |_| update_lighting_capabilities(&widgets));
    }
    {
        let widgets = Rc::clone(widgets);
        let color_mode = widgets.color_mode.clone();
        color_mode.connect_changed(move || update_lighting_capabilities(&widgets));
    }
    update_lighting_capabilities(widgets);
}

fn update_lighting_capabilities(widgets: &Widgets) {
    let capabilities = effect_capabilities(widgets.effect.selected());
    widgets.brightness.set_sensitive(capabilities.brightness);
    widgets.speed.set_sensitive(capabilities.speed);
    widgets
        .direction
        .set_sensitive(capabilities.direction.is_some());
    widgets.direction.set_kind(capabilities.direction);
    if !capabilities.color {
        widgets.color_mode.set_single(false);
    }
    widgets.color_mode.set_sensitive(capabilities.color);

    let single_colour = capabilities.color && widgets.color_mode.is_single();
    widgets.lighting_hue.set_sensitive(single_colour);
    widgets.lighting_saturation.set_sensitive(single_colour);
    widgets.lighting_value.set_sensitive(single_colour);
}

fn set_status(widgets: &Widgets, text: &str, state_class: &str) {
    widgets.status.set_text(text);
    widgets.status_dot.remove_css_class("connecting");
    widgets.status_dot.remove_css_class("connected");
    widgets.status_dot.remove_css_class("error");
    widgets.status_dot.add_css_class(state_class);
}

fn set_connection_status(widgets: &Widgets, connection: &str) {
    let lower = connection.trim().to_ascii_lowercase();
    if lower == "connected" || lower.starts_with("connected ") {
        set_status(widgets, connection, "connected");
    } else if lower.contains("error") || lower.contains("disconnected") || lower.contains("failed")
    {
        set_status(widgets, connection, "error");
    } else {
        set_status(widgets, connection, "connecting");
    }
}

fn handle_event(state: &Rc<State>, widgets: &Rc<Widgets>, event: Event) {
    match event {
        Event::Info((connection, product, firmware, protocol, layers, last_error)) => {
            set_connection_status(widgets, &connection);
            widgets.info.set_text(&format!("Connection: {connection}\nProduct: {product:04X}\nFirmware: {firmware}\nProtocol: {protocol}\nLayers: {layers}"));
            widgets.error.set_text(&last_error);
        }
        Event::Refreshed => {
            set_status(widgets, "Connected", "connected");
            request_all(&state.client);
        }
        Event::Keymap { bank, assignments } => {
            state
                .drafts
                .borrow_mut()
                .insert(bank.clone(), KeymapDraft::new(assignments));
            if active_bank(&widgets.bank) == bank {
                update_key_editor(state, widgets);
            }
        }
        Event::KeymapApplied(bank) => {
            if let Some(draft) = state.drafts.borrow_mut().get_mut(&bank) {
                draft.commit();
            }
            widgets.status.set_text("Keymap applied");
            update_map_buttons(state, widgets);
        }
        Event::Lighting(lighting) => set_lighting_widgets(widgets, lighting),
        Event::LightingApplied => {
            widgets.status.set_text("Lighting applied");
            state.client.send(Command::GetLighting);
        }
        Event::Macros(macros) => widgets.macros.set_macros(macros),
        Event::MacroApplied => {
            widgets.status.set_text("Macro saved");
            state.client.send(Command::GetMacros);
        }
        Event::HardwareSnapshot(snapshot_json) => {
            let action = state.profile_action.borrow_mut().take();
            let Some(ProfileAction::Backup { name, reset_after }) = action else {
                widgets
                    .settings
                    .set_profile_error("Received an unexpected hardware snapshot");
                return;
            };
            let result = serde_json::from_str::<HardwareSnapshot>(&snapshot_json)
                .map_err(|error| error.to_string())
                .and_then(|hardware| {
                    DeviceProfile::new(name, hardware, widgets.macros.profile_metadata())
                })
                .and_then(|profile| save_active_profile(&profile));
            match result {
                Ok(()) => {
                    widgets.settings.refresh_profile();
                    widgets.settings.set_profile_message(if reset_after {
                        "Pre-reset backup saved; resetting keyboard…"
                    } else {
                        "Profile backup saved"
                    });
                    if reset_after {
                        state.client.send(Command::FactoryReset);
                    }
                }
                Err(error) => widgets.settings.set_profile_error(&error),
            }
        }
        Event::HardwareSnapshotRestored => {
            let action = state.profile_action.borrow_mut().take();
            let Some(ProfileAction::Restore(profile)) = action else {
                widgets
                    .settings
                    .set_profile_error("Received an unexpected profile restore result");
                return;
            };
            if let Err(error) = widgets.macros.restore_profile_metadata(&profile.client) {
                widgets.settings.set_profile_error(&format!(
                    "Hardware restored, but local macro metadata could not be saved: {error}"
                ));
            } else {
                widgets
                    .settings
                    .set_profile_message("Profile restored and verified");
            }
            request_all(&state.client);
        }
        Event::FactoryResetApplied => {
            set_status(
                widgets,
                "Factory reset complete; reconnecting…",
                "connecting",
            );
            let client = state.client.clone();
            glib::timeout_add_local_once(Duration::from_secs(1), move || {
                client.send(Command::Refresh);
            });
        }
        Event::Error(error) => {
            if state.profile_action.borrow_mut().take().is_some() {
                widgets.settings.set_profile_error(&error);
            }
            set_status(widgets, "Error", "error");
            widgets.error.set_text(&error);
        }
    }
}

fn request_all(client: &Client) {
    client.send(Command::GetInfo);
    client.send(Command::GetKeymap("base".to_string()));
    client.send(Command::GetKeymap("fn".to_string()));
    client.send(Command::GetLighting);
    client.send(Command::GetMacros);
}

fn active_bank(dropdown: &gtk::DropDown) -> &'static str {
    if dropdown.selected() == 0 {
        "base"
    } else {
        "fn"
    }
}

fn update_key_editor(state: &State, widgets: &Widgets) {
    state.updating.set(true);
    if let Some(key_id) = state.selected_key.borrow().as_deref() {
        let physical_label = PHYSICAL_KEYS
            .iter()
            .find(|key| key.id == key_id)
            .map(|key| key.label)
            .unwrap_or(key_id);
        widgets.selected.set_text(physical_label);
        let action = state
            .drafts
            .borrow()
            .get(active_bank(&widgets.bank))
            .and_then(|draft| draft.action(key_id))
            .unwrap_or_default()
            .to_string();
        let assignment = if action.is_empty() {
            physical_label.to_string()
        } else {
            canonical_action_label(&action)
        };
        widgets.current.set_text(&format!(
            "Current: {assignment}  •  {} layer  •  {key_id}",
            if active_bank(&widgets.bank) == "base" {
                "Base"
            } else {
                "Fn"
            }
        ));
        widgets.action.set_text(&action);
        widgets.action.set_sensitive(true);
        widgets.shortcut.set_selected_action(true, &action);
        for (button, _) in &widgets.palette_actions {
            button.set_sensitive(true);
        }
    } else {
        widgets.selected.set_text("Select a key");
        widgets
            .current
            .set_text("Choose a key above to edit its assignment");
        widgets.action.set_text("");
        widgets.action.set_sensitive(false);
        widgets.shortcut.set_selected_action(false, "");
        for (button, _) in &widgets.palette_actions {
            button.set_sensitive(false);
        }
    }
    state.updating.set(false);
    update_map_buttons(state, widgets);
}

fn normalized_key_meaning(label: &str) -> String {
    let compact: String = label
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();

    match compact.as_str() {
        "ctrl" | "control" | "lctrl" | "leftctrl" | "leftcontrol" => "ctrl".into(),
        "alt" | "lalt" | "leftalt" => "alt".into(),
        "shift" | "lshift" | "leftshift" => "shift".into(),
        "super" | "win" | "windows" | "meta" | "lsuper" | "leftsuper" | "lmeta" | "leftmeta" => {
            "super".into()
        }
        "caps" | "capslock" => "capslock".into(),
        "esc" | "escape" => "escape".into(),
        "bksp" | "backspace" => "backspace".into(),
        "del" | "delete" => "delete".into(),
        "pgup" | "pageup" => "pageup".into(),
        "pgdn" | "pagedown" => "pagedown".into(),
        other => other.into(),
    }
}

fn assignment_matches_legend(legend: &str, assignment: &str) -> bool {
    normalized_key_meaning(legend) == normalized_key_meaning(assignment)
}

fn update_map_buttons(state: &State, widgets: &Widgets) {
    let drafts = state.drafts.borrow();
    let draft = drafts.get(active_bank(&widgets.bank));
    let selected_key = state.selected_key.borrow();
    let dirty = draft.is_some_and(KeymapDraft::is_dirty);
    widgets.apply_map.set_sensitive(dirty);
    widgets.revert_map.set_sensitive(dirty);
    for key in PHYSICAL_KEYS {
        let Some(button) = widgets.keys.get(key.id) else {
            continue;
        };
        let action = draft
            .and_then(|draft| draft.action(key.id))
            .unwrap_or_default();
        let assignment = if action.is_empty() {
            key.label.to_string()
        } else {
            canonical_action_label(action)
        };
        if let Some(label) = widgets.key_assignments.get(key.id) {
            label.set_text(&compact_keycap_label(&assignment));
        }
        let reassigned = !assignment_matches_legend(key.label, &assignment);
        if let Some(legend) = widgets.key_legends.get(key.id) {
            legend.set_text(&compact_keycap_label(key.label));
            legend.set_visible(reassigned);
        }
        if reassigned {
            button.add_css_class("reassigned");
        } else {
            button.remove_css_class("reassigned");
        }
        button.set_active(selected_key.as_deref() == Some(key.id));
        let tooltip = if action.is_empty() {
            key.label.to_string()
        } else {
            format!("{} → {assignment}\n{action}", key.label)
        };
        button.set_tooltip_text(Some(&tooltip));
    }
}

fn set_lighting_widgets(widgets: &Widgets, lighting: Lighting) {
    // The dropdown now includes Off, so its index equals the firmware effect ID.
    widgets.effect.set_selected(u32::from(lighting.1).min(19));
    widgets.brightness.set_value(f64::from(lighting.2));
    widgets.speed.set_value(f64::from(lighting.3));
    widgets.direction.set_value(lighting.4);
    widgets.color_mode.set_single(lighting.5);

    widgets.lighting_hue.set_value(f64::from(lighting.7));
    widgets.lighting_saturation.set_value(f64::from(lighting.8));
    widgets.lighting_value.set_value(f64::from(lighting.9));
    update_lighting_capabilities(widgets);
}
