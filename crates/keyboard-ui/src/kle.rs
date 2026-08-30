use keyboard_core::PHYSICAL_KEYS;
use serde_json::Value;

pub(crate) const KLE_KEY_UNIT: f64 = 64.0;
pub(crate) const KLE_GRID_UNITS_PER_KEY: f64 = 4.0;
const BUILTIN_KLE_JSON: &str = include_str!("../resources/layouts/anko-mini-gaming-keyboard.json");
const CUSTOM_LAYOUT_FILENAME: &str = "custom.json";

#[derive(Clone, Debug)]
pub(crate) struct KleKey {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) w: f64,
    pub(crate) h: f64,
    pub(crate) legend: String,
    pub(crate) color: Option<String>,
    pub(crate) text_color: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct KleLayout {
    pub(crate) keys: Vec<KleKey>,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

pub(crate) fn parse_kle_layout(source: &str) -> Result<KleLayout, String> {
    let root: Value =
        serde_json::from_str(source).map_err(|error| format!("Invalid KLE JSON: {error}"))?;
    let rows = root
        .as_array()
        .ok_or_else(|| "KLE root must be an array".to_string())?;

    let mut keys = Vec::new();
    let mut y = 0.0_f64;
    let mut max_x = 0.0_f64;
    let mut max_y = 0.0_f64;

    // KLE properties are stateful. Width/height apply to the next key and then
    // reset; colours/alignment-style properties persist until changed.
    let mut color: Option<String> = None;
    let mut text_color: Option<String> = None;

    for row_value in rows {
        // The optional metadata object at the beginning of a KLE export is not a row.
        let Some(row) = row_value.as_array() else {
            continue;
        };

        let mut x = 0.0_f64;
        let mut row_y = y;
        let mut next_w = 1.0_f64;
        let mut next_h = 1.0_f64;

        for item in row {
            if let Some(properties) = item.as_object() {
                if let Some(value) = properties.get("x").and_then(Value::as_f64) {
                    x += value;
                }
                if let Some(value) = properties.get("y").and_then(Value::as_f64) {
                    row_y += value;
                }
                if let Some(value) = properties.get("w").and_then(Value::as_f64) {
                    next_w = value;
                }
                if let Some(value) = properties.get("h").and_then(Value::as_f64) {
                    next_h = value;
                }
                if let Some(value) = properties.get("c").and_then(Value::as_str) {
                    color = Some(value.to_string());
                }
                if let Some(value) = properties.get("t").and_then(Value::as_str) {
                    text_color = Some(value.to_string());
                }
                continue;
            }

            let Some(legend) = item.as_str() else {
                continue;
            };

            keys.push(KleKey {
                x,
                y: row_y,
                w: next_w,
                h: next_h,
                legend: legend.to_string(),
                color: color.clone(),
                text_color: text_color.clone(),
            });
            max_x = max_x.max(x + next_w);
            max_y = max_y.max(row_y + next_h);
            x += next_w;

            // These are per-key KLE properties.
            next_w = 1.0;
            next_h = 1.0;
        }

        y = row_y + 1.0;
    }

    if keys.is_empty() {
        return Err("KLE layout contains no keys".to_string());
    }

    Ok(KleLayout {
        keys,
        width: max_x,
        height: max_y,
    })
}

pub(crate) fn custom_kle_path() -> std::path::PathBuf {
    gtk::glib::user_data_dir()
        .join("anko-keyboard")
        .join("layouts")
        .join(CUSTOM_LAYOUT_FILENAME)
}

pub(crate) fn validate_kle_layout(source: &str) -> Result<KleLayout, String> {
    let layout = parse_kle_layout(source)?;
    if layout.keys.len() != PHYSICAL_KEYS.len() {
        return Err(format!(
            "Layout has {} keys; this keyboard requires {}",
            layout.keys.len(),
            PHYSICAL_KEYS.len()
        ));
    }
    if !layout.width.is_finite()
        || !layout.height.is_finite()
        || layout.width <= 0.0
        || layout.height <= 0.0
        || layout
            .keys
            .iter()
            .any(|key| !key.x.is_finite() || !key.y.is_finite() || key.w <= 0.0 || key.h <= 0.0)
    {
        return Err("Layout contains invalid key geometry".to_string());
    }
    Ok(layout)
}

pub(crate) fn active_kle_layout() -> Option<KleLayout> {
    let custom_path = custom_kle_path();
    if custom_path.exists() {
        match std::fs::read_to_string(&custom_path)
            .map_err(|error| error.to_string())
            .and_then(|source| validate_kle_layout(&source))
        {
            Ok(layout) => return Some(layout),
            Err(error) => eprintln!("Custom KLE layout is invalid: {error}; using default"),
        }
    }

    match validate_kle_layout(BUILTIN_KLE_JSON) {
        Ok(layout) => Some(layout),
        Err(error) => {
            eprintln!("{error}; using fallback keyboard geometry");
            None
        }
    }
}

fn kle_css_color(raw: &str) -> Option<&str> {
    // KLE text colour may contain multiple semicolon-separated colours for
    // different legend positions. The configurator currently renders one
    // primary assignment/legend, so use the first colour.
    let color = raw.split(';').next()?.trim();
    let hex = color.strip_prefix('#')?;
    if matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(color)
    } else {
        None
    }
}

pub(crate) fn install_kle_key_css(layout: &KleLayout) {
    let mut css = String::new();

    for (index, key) in layout.keys.iter().enumerate() {
        let class_name = format!("kle-key-{index}");

        if let Some(background) = key.color.as_deref().and_then(kle_css_color) {
            // Use background-color rather than the `background` shorthand so we do
            // not accidentally wipe GTK's state styling. Explicit hover/active
            // shades keep KLE-coloured keys feeling like normal GTK buttons.
            css.push_str(&format!(
                ".{class_name} {{ background-image: none; background-color: {background}; }}\n                 .{class_name}:hover {{ background-color: shade({background}, 0.92); }}\n                 .{class_name}:active {{ background-color: shade({background}, 0.84); }}\n                 .{class_name}:checked {{ background-color: shade({background}, 0.90); }}\n"
            ));
        }

        if let Some(foreground) = key.text_color.as_deref().and_then(kle_css_color) {
            // Explicitly target the labels too. Depending on the Adwaita theme,
            // button foreground colour is not always inherited by child labels.
            css.push_str(&format!(
                ".{class_name}, .{class_name} label {{ color: {foreground}; }}\n"
            ));
        }
    }

    if css.is_empty() {
        return;
    }

    let provider = gtk::CssProvider::new();
    provider.load_from_data(&css);
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
        );
    }
}

pub(crate) fn kle_primary_legend(raw: &str) -> Option<&str> {
    raw.lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("<i "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_layout_passes_custom_layout_validation() {
        let layout = validate_kle_layout(BUILTIN_KLE_JSON).unwrap();
        assert_eq!(layout.keys.len(), PHYSICAL_KEYS.len());
    }

    #[test]
    fn custom_layout_must_describe_every_physical_key() {
        let error = validate_kle_layout(r#"[["Esc"]]"#).unwrap_err();
        assert!(error.contains("requires 63"));
    }
}
