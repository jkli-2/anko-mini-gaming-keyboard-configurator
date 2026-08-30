pub(crate) fn install_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(
        ".keyboard-key { min-width: 0; min-height: 0; padding: 2px 3px; }
         .keyboard-key:checked { outline: 2px solid @accent_color; outline-offset: -2px; }
         .keyboard-key.reassigned { background: alpha(@accent_bg_color, 0.10); }
         .keyboard-assignment { font-weight: 700; font-size: 13px; min-height: 24px; padding: 4px 0 3px 0; }
         .keyboard-legend { font-size: 0.68em; opacity: 0.62; }
         .key-summary-title { font-size: 1.08em; font-weight: 700; }
         .key-summary-detail { opacity: 0.68; }
         .palette-button { padding: 3px 7px; min-height: 28px; }
         .palette-category { min-width: 118px; }
         .palette-strip { min-height: 40px; }
         .header-nav { padding: 4px 12px; margin: 0 2px; }
         .header-nav.active { background: alpha(currentColor, 0.10); }
         .status-dot { min-width: 10px; min-height: 10px; border-radius: 50%; padding: 0; }
         .status-dot.connecting { background: #e5a50a; }
         .status-dot.connected { background: #2ec27e; }
         .status-dot.error { background: #e01b24; }
         .unsaved-label { opacity: 0.72; }",
    );
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
