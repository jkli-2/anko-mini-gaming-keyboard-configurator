use adw::prelude::*;

pub(crate) const EFFECT_NAMES: [&str; 20] = [
    "Off",
    "Static",
    "Breath",
    "Spin",
    "ColorLoop",
    "Stream",
    "Bloom",
    "UD Wave",
    "TrigSingle",
    "Sine Wave",
    "Chase",
    "Ripple Off",
    "Cross",
    "Rain",
    "Twinkling Star",
    "Ripple On",
    "Soduko",
    "Tide",
    "Snake",
    "Laser",
];

#[derive(Clone, Copy)]
struct HsvPreset {
    label: &'static str,
    h: u8,
    s: u8,
    v: u8,
    // UI-only sRGB colour. This deliberately does NOT use the calibrated
    // keyboard RGB/HSV lookup value.
    display_rgb: (u8, u8, u8),
}

const VIVID_PRESETS: [HsvPreset; 13] = [
    HsvPreset {
        label: "Red",
        h: 0,
        s: 255,
        v: 255,
        display_rgb: (255, 0, 0),
    },
    HsvPreset {
        label: "Orange",
        h: 14,
        s: 255,
        v: 255,
        display_rgb: (255, 128, 0),
    },
    HsvPreset {
        label: "Yellow",
        h: 27,
        s: 255,
        v: 255,
        display_rgb: (255, 255, 0),
    },
    HsvPreset {
        label: "Lime",
        h: 53,
        s: 255,
        v: 255,
        display_rgb: (191, 255, 0),
    },
    HsvPreset {
        label: "Green",
        h: 85,
        s: 255,
        v: 255,
        display_rgb: (0, 255, 0),
    },
    HsvPreset {
        label: "Spring Green",
        h: 98,
        s: 254,
        v: 255,
        display_rgb: (0, 255, 127),
    },
    HsvPreset {
        label: "Cyan",
        h: 111,
        s: 254,
        v: 255,
        display_rgb: (0, 255, 255),
    },
    HsvPreset {
        label: "Teal",
        h: 112,
        s: 255,
        v: 255,
        display_rgb: (0, 128, 128),
    },
    HsvPreset {
        label: "Azure",
        h: 136,
        s: 255,
        v: 255,
        display_rgb: (0, 127, 255),
    },
    HsvPreset {
        label: "Blue",
        h: 170,
        s: 255,
        v: 255,
        display_rgb: (0, 0, 255),
    },
    HsvPreset {
        label: "Violet",
        h: 220,
        s: 255,
        v: 255,
        display_rgb: (143, 0, 255),
    },
    HsvPreset {
        label: "Magenta",
        h: 238,
        s: 254,
        v: 255,
        display_rgb: (255, 0, 255),
    },
    HsvPreset {
        label: "Rose",
        h: 246,
        s: 255,
        v: 255,
        display_rgb: (255, 0, 96),
    },
];

const WHITE_PRESETS: [HsvPreset; 9] = [
    HsvPreset {
        label: "2700 K",
        h: 14,
        s: 218,
        v: 255,
        display_rgb: (255, 167, 87),
    },
    HsvPreset {
        label: "3000 K",
        h: 15,
        s: 211,
        v: 255,
        display_rgb: (255, 180, 107),
    },
    HsvPreset {
        label: "3500 K",
        h: 15,
        s: 197,
        v: 255,
        display_rgb: (255, 196, 137),
    },
    HsvPreset {
        label: "4000 K",
        h: 15,
        s: 186,
        v: 255,
        display_rgb: (255, 209, 163),
    },
    HsvPreset {
        label: "4500 K",
        h: 16,
        s: 178,
        v: 255,
        display_rgb: (255, 219, 186),
    },
    HsvPreset {
        label: "5000 K",
        h: 16,
        s: 170,
        v: 255,
        display_rgb: (255, 228, 206),
    },
    HsvPreset {
        label: "5500 K",
        h: 17,
        s: 164,
        v: 255,
        display_rgb: (255, 236, 224),
    },
    HsvPreset {
        label: "6000 K",
        h: 17,
        s: 156,
        v: 255,
        display_rgb: (255, 243, 239),
    },
    HsvPreset {
        label: "6500 K",
        h: 18,
        s: 150,
        v: 255,
        display_rgb: (255, 249, 253),
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DirectionKind {
    Horizontal,
    Vertical,
}

#[derive(Clone)]
pub(crate) struct DirectionControl {
    first: gtk::ToggleButton,
    second: gtk::ToggleButton,
}

#[derive(Clone)]
pub(crate) struct ColorModeControl {
    dynamic: gtk::ToggleButton,
    single: gtk::ToggleButton,
}

impl ColorModeControl {
    fn new(grid: &gtk::Grid, row: i32) -> Self {
        let label = gtk::Label::new(Some("Colour mode"));
        label.set_xalign(0.0);
        grid.attach(&label, 0, row, 1, 1);

        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        buttons.add_css_class("linked");
        buttons.set_hexpand(true);
        let dynamic = gtk::ToggleButton::with_label("Dynamic RGB");
        let single = gtk::ToggleButton::with_label("Single Colour");
        single.set_group(Some(&dynamic));
        dynamic.set_active(true);
        dynamic.set_hexpand(true);
        single.set_hexpand(true);
        buttons.append(&dynamic);
        buttons.append(&single);
        grid.attach(&buttons, 1, row, 1, 1);

        Self { dynamic, single }
    }

    pub(crate) fn is_single(&self) -> bool {
        self.single.is_active()
    }

    pub(crate) fn set_single(&self, single: bool) {
        if single {
            self.single.set_active(true);
        } else {
            self.dynamic.set_active(true);
        }
    }

    pub(crate) fn set_sensitive(&self, sensitive: bool) {
        self.dynamic.set_sensitive(sensitive);
        self.single.set_sensitive(sensitive);
    }

    pub(crate) fn connect_changed<F: Fn() + 'static>(&self, callback: F) {
        self.single.connect_toggled(move |_| callback());
    }
}

impl DirectionControl {
    fn new(grid: &gtk::Grid, row: i32) -> Self {
        let label = gtk::Label::new(Some("Direction"));
        label.set_xalign(0.0);
        grid.attach(&label, 0, row, 1, 1);

        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        buttons.add_css_class("linked");
        buttons.set_hexpand(true);
        let first = gtk::ToggleButton::with_label("Left");
        let second = gtk::ToggleButton::with_label("Right");
        second.set_group(Some(&first));
        first.set_active(true);
        first.set_hexpand(true);
        second.set_hexpand(true);
        buttons.append(&first);
        buttons.append(&second);
        grid.attach(&buttons, 1, row, 1, 1);

        Self { first, second }
    }

    pub(crate) fn value(&self) -> u8 {
        u8::from(self.second.is_active())
    }

    pub(crate) fn set_value(&self, value: u8) {
        if value == 0 {
            self.first.set_active(true);
        } else {
            self.second.set_active(true);
        }
    }

    pub(crate) fn set_sensitive(&self, sensitive: bool) {
        self.first.set_sensitive(sensitive);
        self.second.set_sensitive(sensitive);
    }

    pub(crate) fn set_kind(&self, kind: Option<DirectionKind>) {
        match kind {
            Some(DirectionKind::Vertical) => {
                self.first.set_label("Up");
                self.second.set_label("Down");
            }
            _ => {
                self.first.set_label("Left");
                self.second.set_label("Right");
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct EffectCapabilities {
    pub(crate) brightness: bool,
    pub(crate) speed: bool,
    pub(crate) direction: Option<DirectionKind>,
    pub(crate) color: bool,
}

const STATIC_CAPABILITIES: EffectCapabilities = EffectCapabilities {
    brightness: true,
    speed: false,
    direction: None,
    color: true,
};

const OFF_CAPABILITIES: EffectCapabilities = EffectCapabilities {
    brightness: false,
    speed: false,
    direction: None,
    color: false,
};

const ANIMATED_CAPABILITIES: EffectCapabilities = EffectCapabilities {
    brightness: true,
    speed: true,
    direction: None,
    color: true,
};

pub(crate) fn effect_capabilities(effect: u32) -> EffectCapabilities {
    match effect {
        0 => OFF_CAPABILITIES,
        1 => STATIC_CAPABILITIES,
        // Stream, Bloom, and Twinkling Star expose left/right direction.
        5 | 6 | 14 => EffectCapabilities {
            direction: Some(DirectionKind::Horizontal),
            ..ANIMATED_CAPABILITIES
        },
        // UD Wave exposes up/down direction.
        7 => EffectCapabilities {
            direction: Some(DirectionKind::Vertical),
            ..ANIMATED_CAPABILITIES
        },
        _ => ANIMATED_CAPABILITIES,
    }
}

pub(crate) type LightingPage = (
    gtk::ScrolledWindow,
    gtk::DropDown,
    gtk::SpinButton,
    gtk::SpinButton,
    DirectionControl,
    ColorModeControl,
    gtk::SpinButton,
    gtk::SpinButton,
    gtk::SpinButton,
    gtk::Button,
);

pub(crate) fn lighting_page() -> LightingPage {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 18);
    page.set_margin_top(18);
    page.set_margin_start(24);
    page.set_margin_end(24);
    page.set_margin_bottom(18);

    let top = gtk::Box::new(gtk::Orientation::Horizontal, 18);
    top.set_homogeneous(true);
    top.set_hexpand(true);

    // GNOME-style section: heading outside the card, padded content inside.
    let effect_section = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let effect_title = gtk::Label::new(Some("Lighting effect"));
    effect_title.add_css_class("heading");
    effect_title.set_xalign(0.0);
    effect_section.append(&effect_title);

    let effect_panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    effect_panel.add_css_class("card");
    let effect_content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    effect_content.set_margin_top(16);
    effect_content.set_margin_bottom(16);
    effect_content.set_margin_start(16);
    effect_content.set_margin_end(16);

    let effect_grid = gtk::Grid::builder()
        .row_spacing(10)
        .column_spacing(12)
        .hexpand(true)
        .build();

    let effect_label = gtk::Label::new(Some("Effect"));
    effect_label.set_xalign(0.0);
    effect_grid.attach(&effect_label, 0, 0, 1, 1);
    let effect = gtk::DropDown::from_strings(&EFFECT_NAMES);
    effect.set_hexpand(true);
    effect_grid.attach(&effect, 1, 0, 1, 1);

    let brightness = add_spin(&effect_grid, 1, "Brightness", 0.0, 4.0);
    let speed = add_spin(&effect_grid, 2, "Speed", 0.0, 4.0);
    let direction = DirectionControl::new(&effect_grid, 3);
    effect_content.append(&effect_grid);
    effect_panel.append(&effect_content);
    effect_section.append(&effect_panel);

    let colour_section = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let colour_title = gtk::Label::new(Some("Colour"));
    colour_title.add_css_class("heading");
    colour_title.set_xalign(0.0);
    colour_section.append(&colour_title);

    let colour_panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    colour_panel.add_css_class("card");
    let colour_content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    colour_content.set_margin_top(16);
    colour_content.set_margin_bottom(16);
    colour_content.set_margin_start(16);
    colour_content.set_margin_end(16);

    let global_hsv = gtk::Grid::builder()
        .row_spacing(10)
        .column_spacing(12)
        .hexpand(true)
        .build();
    let color_mode = ColorModeControl::new(&global_hsv, 0);
    let lighting_hue = add_spin(&global_hsv, 1, "Hue", 0.0, 255.0);
    let lighting_saturation = add_spin(&global_hsv, 2, "Saturation", 0.0, 255.0);
    let lighting_value = add_spin(&global_hsv, 3, "Value", 0.0, 255.0);
    colour_content.append(&global_hsv);

    // Compact preset picker. Keeping the library in a popover avoids making the
    // Lighting page's natural height depend on all 22 preset buttons.
    let preset_label = gtk::Label::new(Some("Preset"));
    preset_label.set_xalign(0.0);
    global_hsv.attach(&preset_label, 0, 4, 1, 1);

    // gtk::MenuButton in the GTK version used by this project does not expose
    // a custom child API, so keep the colour preview beside the menu button.
    // Visually this still reads as one compact preset control.
    let preset_control = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    preset_control.set_hexpand(true);

    let preset_preview_swatch = colour_swatch((128, 128, 128), 24, 24);

    let preset_menu = gtk::MenuButton::new();
    preset_menu.set_label("Custom");
    preset_menu.set_hexpand(true);

    preset_control.append(&preset_preview_swatch);
    preset_control.append(&preset_menu);

    let preset_popover = gtk::Popover::new();
    let preset_list = gtk::Box::new(gtk::Orientation::Vertical, 2);
    preset_list.set_margin_top(8);
    preset_list.set_margin_bottom(8);
    preset_list.set_margin_start(8);
    preset_list.set_margin_end(8);
    preset_list.set_size_request(280, -1);

    let vivid_heading = gtk::Label::new(Some("Vivid"));
    vivid_heading.add_css_class("heading");
    vivid_heading.set_xalign(0.0);
    vivid_heading.set_margin_start(8);
    vivid_heading.set_margin_end(8);
    vivid_heading.set_margin_top(2);
    vivid_heading.set_margin_bottom(4);
    preset_list.append(&vivid_heading);

    let mut preset_buttons: Vec<(gtk::Button, HsvPreset)> = Vec::new();
    for preset in VIVID_PRESETS {
        let button = preset_menu_row(preset);
        preset_list.append(&button);
        preset_buttons.push((button, preset));
    }

    let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
    separator.set_margin_top(6);
    separator.set_margin_bottom(6);
    preset_list.append(&separator);

    let white_heading = gtk::Label::new(Some("White"));
    white_heading.add_css_class("heading");
    white_heading.set_xalign(0.0);
    white_heading.set_margin_start(8);
    white_heading.set_margin_end(8);
    white_heading.set_margin_bottom(4);
    preset_list.append(&white_heading);

    for preset in WHITE_PRESETS {
        let button = preset_menu_row(preset);
        preset_list.append(&button);
        preset_buttons.push((button, preset));
    }

    // The preset list is taller than the application window. Put it in its own
    // scroller so the popover has a bounded size; an oversized popover can be
    // mapped and immediately unmapped when GTK cannot place it on screen.
    let preset_scroll = gtk::ScrolledWindow::new();
    preset_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    preset_scroll.set_min_content_width(280);
    preset_scroll.set_min_content_height(220);
    preset_scroll.set_max_content_height(360);
    preset_scroll.set_propagate_natural_height(true);
    preset_scroll.set_child(Some(&preset_list));

    preset_popover.set_child(Some(&preset_scroll));
    preset_menu.set_popover(Some(&preset_popover));
    global_hsv.attach(&preset_control, 1, 4, 1, 1);

    // Keep the preset picker available whenever colour controls are available.
    // Choosing a preset implicitly selects Single Colour, so Dynamic RGB does
    // not make the dropdown itself unclickable.
    preset_menu.set_sensitive(color_mode.single.is_sensitive());
    {
        let preset_menu = preset_menu.clone();
        let single = color_mode.single.clone();
        single.connect_sensitive_notify(move |button| {
            preset_menu.set_sensitive(button.is_sensitive());
        });
    }

    // Selecting a preset updates HSV and the collapsed menu preview.
    // Direct HSV edits switch the picker back to an implicit Custom state.
    let updating_preset = std::rc::Rc::new(std::cell::Cell::new(false));

    for (index, (button, preset)) in preset_buttons.iter().enumerate() {
        let button = button.clone();
        let preset = *preset;
        let all_buttons: Vec<gtk::Button> = preset_buttons
            .iter()
            .map(|(button, _)| button.clone())
            .collect();
        let hue = lighting_hue.clone();
        let saturation = lighting_saturation.clone();
        let value = lighting_value.clone();
        let updating = updating_preset.clone();
        let preview_swatch = preset_preview_swatch.clone();
        let preset_menu_label = preset_menu.clone();
        let popover = preset_popover.clone();
        let single = color_mode.single.clone();

        button.connect_clicked(move |clicked| {
            updating.set(true);
            single.set_active(true);

            for (other_index, other) in all_buttons.iter().enumerate() {
                if other_index == index {
                    other.add_css_class("suggested-action");
                } else {
                    other.remove_css_class("suggested-action");
                }
            }

            hue.set_value(f64::from(preset.h));
            saturation.set_value(f64::from(preset.s));
            value.set_value(f64::from(preset.v));

            set_swatch_colour(&preview_swatch, preset.display_rgb);
            preset_menu_label.set_label(preset.label);

            updating.set(false);
            popover.popdown();

            // Keep the clicked row focused only while the popover is open.
            clicked.grab_focus();
        });
    }

    for spin in [
        lighting_hue.clone(),
        lighting_saturation.clone(),
        lighting_value.clone(),
    ] {
        let buttons: Vec<gtk::Button> = preset_buttons
            .iter()
            .map(|(button, _)| button.clone())
            .collect();
        let updating = updating_preset.clone();
        let preview_swatch = preset_preview_swatch.clone();
        let preset_menu_label = preset_menu.clone();
        let hue = lighting_hue.clone();
        let saturation = lighting_saturation.clone();
        let value = lighting_value.clone();

        spin.connect_value_changed(move |_| {
            if updating.get() {
                return;
            }

            for button in &buttons {
                button.remove_css_class("suggested-action");
            }

            // This is an on-screen approximation only. Preset swatches use their
            // dedicated sRGB values because the keyboard's calibrated HSV values
            // are not suitable for monitor display.
            let rgb = hsv_to_display_rgb(
                hue.value() as u8,
                saturation.value() as u8,
                value.value() as u8,
            );
            set_swatch_colour(&preview_swatch, rgb);
            preset_menu_label.set_label("Custom");
        });
    }

    colour_panel.append(&colour_content);
    colour_section.append(&colour_panel);

    top.append(&effect_section);
    top.append(&colour_section);
    page.append(&top);

    let apply_lighting = gtk::Button::with_label("Apply Lighting");
    apply_lighting.add_css_class("suggested-action");
    apply_lighting.set_halign(gtk::Align::End);
    page.append(&apply_lighting);

    let scroll = gtk::ScrolledWindow::new();
    scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroll.set_child(Some(&page));
    scroll.set_hexpand(true);
    scroll.set_vexpand(true);

    (
        scroll,
        effect,
        brightness,
        speed,
        direction,
        color_mode,
        lighting_hue,
        lighting_saturation,
        lighting_value,
        apply_lighting,
    )
}

fn colour_swatch(rgb: (u8, u8, u8), width: i32, height: i32) -> gtk::Box {
    let swatch = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    swatch.set_size_request(width, height);
    swatch.set_halign(gtk::Align::Center);
    swatch.set_valign(gtk::Align::Center);
    swatch.set_hexpand(false);
    swatch.set_vexpand(false);
    swatch.add_css_class("colour-swatch");
    set_swatch_colour(&swatch, rgb);
    swatch
}

fn set_swatch_colour(swatch: &gtk::Box, rgb: (u8, u8, u8)) {
    let (r, g, b) = rgb;
    let provider = gtk::CssProvider::new();
    provider.load_from_data(&format!(
        ".colour-swatch {{
            background: rgb({r}, {g}, {b});
            border: 1px solid alpha(#000000, 0.30);
            border-radius: 4px;
        }}"
    ));
    swatch
        .style_context()
        .add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
}

fn preset_menu_row(preset: HsvPreset) -> gtk::Button {
    let button = gtk::Button::new();
    button.add_css_class("flat");
    button.set_hexpand(true);
    button.set_tooltip_text(Some(&format!(
        "{} — HSV {}/{}/{}",
        preset.label, preset.h, preset.s, preset.v
    )));

    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    row.set_margin_start(4);
    row.set_margin_end(4);

    let swatch = colour_swatch(preset.display_rgb, 44, 24);
    let label = gtk::Label::new(Some(preset.label));
    label.set_xalign(0.0);
    label.set_hexpand(true);

    let hsv = gtk::Label::new(Some(&format!(
        "{:02X}{:02X}{:02X}",
        preset.display_rgb.0, preset.display_rgb.1, preset.display_rgb.2
    )));
    hsv.add_css_class("dim-label");
    hsv.add_css_class("monospace");

    row.append(&swatch);
    row.append(&label);
    row.append(&hsv);
    button.set_child(Some(&row));
    button
}

fn hsv_to_display_rgb(h: u8, s: u8, v: u8) -> (u8, u8, u8) {
    let h = f64::from(h) / 255.0 * 360.0;
    let s = f64::from(s) / 255.0;
    let v = f64::from(v) / 255.0;

    let c = v * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = v - c;

    let (r1, g1, b1) = match h {
        h if h < 60.0 => (c, x, 0.0),
        h if h < 120.0 => (x, c, 0.0),
        h if h < 180.0 => (0.0, c, x),
        h if h < 240.0 => (0.0, x, c),
        h if h < 300.0 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    (
        ((r1 + m) * 255.0).round() as u8,
        ((g1 + m) * 255.0).round() as u8,
        ((b1 + m) * 255.0).round() as u8,
    )
}

fn add_spin(grid: &gtk::Grid, row: i32, text: &str, min: f64, max: f64) -> gtk::SpinButton {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    grid.attach(&label, 0, row, 1, 1);
    let spin = gtk::SpinButton::with_range(min, max, 1.0);
    spin.set_hexpand(true);
    grid.attach(&spin, 1, row, 1, 1);
    spin
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_capabilities_match_the_fda1_frontend_table() {
        assert_eq!(EFFECT_NAMES.len(), 20);

        let off = effect_capabilities(0);
        assert!(!off.brightness);
        assert!(!off.speed);
        assert_eq!(off.direction, None);
        assert!(!off.color);

        let static_mode = effect_capabilities(1);
        assert!(static_mode.brightness);
        assert!(!static_mode.speed);
        assert_eq!(static_mode.direction, None);
        assert!(static_mode.color);

        for effect in 2..=19 {
            let capabilities = effect_capabilities(effect);
            assert!(capabilities.brightness, "effect {effect}");
            assert!(capabilities.speed, "effect {effect}");
            assert!(capabilities.color, "effect {effect}");
        }

        for effect in 0..=19 {
            let expected = match effect {
                5 | 6 | 14 => Some(DirectionKind::Horizontal),
                7 => Some(DirectionKind::Vertical),
                _ => None,
            };
            assert_eq!(effect_capabilities(effect).direction, expected);
        }
    }
}
