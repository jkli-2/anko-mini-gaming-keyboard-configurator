# Anko Mini Gaming Keyboard Configurator

A native Linux configurator for the [Anko Mini Gaming Keyboard 43721375](https://www.kmart.com.au/product/mini-gaming-keyboard-43721375/)
(`36ae:fda1`). It provides key remapping for the Base and Fn layers, global lighting
controls, and a semantic editor for the keyboard's 16 macro slots.

The project has two runtime components:

- `keyboard-ui`, a GTK 4/libadwaita client; and
- `keyboardd`, a session D-Bus service which is the sole owner of the HID device.

The client never opens the keyboard directly. Device access is restricted to the known
vendor collection (`ff00:0002`), and write paths verify device readback.

## Build from source

Rust 1.85 or newer, GTK 4, libadwaita, and hidapi development dependencies are
required.

```sh
cargo build --workspace
cargo test --workspace
```

Start the daemon and UI in separate terminals:

```sh
cargo run -p keyboardd
cargo run -p keyboard-ui
```

The daemon also needs permission to access the keyboard's hidraw device. See
[the packaging guide](docs/packaging.md) for the udev rule and the supported split
Flatpak/native installation, including the Arch Linux/AUR packages.

## Documentation

- [GTK client](docs/ui.md)
- [D-Bus API](docs/dbus.md)
- [Protocol notes](docs/protocol.md)
- [Testing and hardware-write safety](docs/testing.md)
- [Packaging](docs/packaging.md)

Hardware-write examples are deliberately excluded from normal tests. Read the safety
and restoration instructions before running them against a keyboard.

## Acknowledgments

Developed with assistance from [OpenAI Codex](https://developers.openai.com/codex/).
Hardware validation and final review were performed by the project maintainer.

## License

MIT. See [LICENSE](LICENSE).
