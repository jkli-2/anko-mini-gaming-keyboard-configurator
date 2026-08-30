# Packaging

## v1 decision

The first packaged release uses two components:

```text
Flatpak GTK client
        │
        │ session D-Bus: io.github.jkli_2.anko_keyboard_configurator.Daemon
        ▼
host-installed keyboardd user service
        │
        ▼
udev-authorized /dev/hidraw device
```

The Flatpak contains only `keyboard-ui`. The host companion package contains
`keyboardd`, its D-Bus/systemd activation files, and the FDA1 udev rule. This preserves
the existing rule that the GTK process never opens HID directly and avoids granting the
Flatpak broad access to host devices.

A Flatpak cannot install host udev rules or host systemd user units. Therefore the v1
Flatpak is not a standalone installation: its release notes and first-run error state
must point users to the native daemon package or manual daemon installer.

## Flatpak client

Use application ID:

```text
io.github.jkli_2.anko_keyboard_configurator
```

The manifest at
`packaging/flatpak/io.github.jkli_2.anko_keyboard_configurator.yml` builds only the UI package
and exports its desktop file, AppStream metadata, icon, and executable. Its relevant
finish arguments are:

```yaml
finish-args:
  - --share=ipc
  - --socket=wayland
  - --socket=fallback-x11
  - --device=dri
  - --talk-name=io.github.jkli_2.anko_keyboard_configurator.Daemon
```

Do not add `--device=all`, direct hidraw access, host filesystem access, or system-bus
access. The native GTK file chooser can use the desktop portal inside Flatpak. GLib's
per-user data directory resolves inside the application sandbox, so macro names,
semantic macro steps, and the one custom KLE layout remain private client data.

The exact session-bus permission is enough for the UI to call and activate the host
service. See the official [Flatpak sandbox permission documentation](https://docs.flatpak.org/en/latest/sandbox-permissions.html).

## Native daemon companion

The assets and transparent manual installer live under `packaging/native`. The
distribution-native package or installer owns these files:

```text
/usr/libexec/anko-keyboard/keyboardd
/usr/lib/systemd/user/io.github.jkli_2.anko_keyboard_configurator.Daemon.service
/usr/share/dbus-1/services/io.github.jkli_2.anko_keyboard_configurator.Daemon.service
/usr/lib/udev/rules.d/70-anko-keyboard.rules
```

Distribution-specific libexec paths may differ, but the systemd and D-Bus files must
use the installed executable's real absolute path.

### systemd user service

```ini
[Unit]
Description=Anko Keyboard configuration daemon

[Service]
Type=dbus
BusName=io.github.jkli_2.anko_keyboard_configurator.Daemon
ExecStart=/usr/libexec/anko-keyboard/keyboardd
Restart=on-failure
RestartSec=1
```

The service is activated on demand, so it does not need to be enabled at login.
`PrivateTmp=true` keeps the daemon's advisory lock writable while
`ProtectSystem=strict` is active. Do not add `ProtectHome=true`: live installation
testing showed that hidapi then fails with `Permission denied` even when the FDA1
hidraw nodes have the correct per-user `uaccess` ACL.

### D-Bus activation

```ini
[D-BUS Service]
Name=io.github.jkli_2.anko_keyboard_configurator.Daemon
Exec=/usr/libexec/anko-keyboard/keyboardd
SystemdService=io.github.jkli_2.anko_keyboard_configurator.Daemon.service
```

The activation filename should match the well-known name. D-Bus documents this
session-service/systemd arrangement in its
[integration guidance](https://dbus.freedesktop.org/doc/dbus-daemon.1.html#integrating-session-services).

### udev permission

```udev
ACTION=="add|change", SUBSYSTEM=="hidraw", KERNEL=="hidraw*", \
ATTRS{idVendor}=="36ae", ATTRS{idProduct}=="fda1", TAG+="uaccess"
```

Use logind's `uaccess` ACL rather than world-readable/writable modes. This grants the
active local user access to this product's hidraw nodes. `keyboardd` must still enforce
its existing VID, PID, usage-page, and usage-ID checks before opening a collection.

After installing or upgrading the rule, reload udev rules and reconnect the keyboard.
The installer should also reload the user service manager where practical. Do not start
one daemon per package installation; D-Bus activation should remain the normal start
path.

## Arch Linux / AUR

The project includes a VCS split-package definition under `packaging/aur` while there
is no tagged source release:

- `anko-mini-gaming-keyboard-configurator-git` installs the native GTK client and
  depends on the daemon companion; and
- `anko-mini-gaming-keyboard-configurator-daemon-git` installs only `keyboardd`, its
  activation files, and the udev rule for users of the Flatpak client.

To build and install both packages directly from a source checkout:

```sh
cd packaging/aur
makepkg --syncdeps --install
```

Reconnect the keyboard after the daemon package is installed. Pacman's systemd and
udev hooks handle the corresponding reloads; the user service remains D-Bus activated
and does not need to be enabled.

## Failure presentation

The Flatpak must distinguish these common failures:

- `io.github.jkli_2.anko_keyboard_configurator.Daemon` is unknown: the native companion is not installed or its
  activation files are unavailable;
- activation fails: inspect `systemctl --user status io.github.jkli_2.anko_keyboard_configurator.Daemon.service`;
- the service runs but reports permission denied: install/reload the udev rule and
  reconnect the keyboard; and
- the service connects but no supported device exists: show the existing disconnected
  state and allow Refresh.

The client must not offer to install host files itself or request administrator access.

## Release artifacts and checks

The v1 packaging work now provides:

1. the Flatpak manifest and generated `cargo-sources.json` for an offline Cargo build;
2. desktop, AppStream, and scalable icon metadata named for the application ID;
3. native daemon systemd, D-Bus, and udev assets plus `install.sh` and `uninstall.sh`;
4. the installation procedure below; and
5. a pending installed-system integration test covering D-Bus activation from inside
   the Flatpak, hidraw access, unplug/replug plus Refresh, and a reversible write.

## Build and install

Install the matching GNOME runtime, SDK, and Rust SDK extension. Branch 50 of the GNOME
SDK currently uses the Freedesktop 25.08 extension branch:

```sh
flatpak install flathub org.gnome.Platform//50 org.gnome.Sdk//50 \
  org.freedesktop.Sdk.Extension.rust-stable//25.08
```

From the repository root, build and install the client for the current user:

```sh
flatpak-builder --user --install --force-clean build-flatpak \
  packaging/flatpak/io.github.jkli_2.anko_keyboard_configurator.yml
```

The checked-in Cargo source list makes the compile itself network-independent after
Flatpak Builder has fetched and verified the declared source archives. Regenerate it
with the official `flatpak-cargo-generator.py Cargo.lock -o cargo-sources.json` whenever
`Cargo.lock` changes.

Build and install the native companion separately:

```sh
cargo build --release --locked -p keyboardd
sudo packaging/native/install.sh
systemctl --user daemon-reload
```

Reconnect the keyboard after the udev rule is installed. Starting or enabling the user
unit is unnecessary: the first D-Bus call activates it. Launch the client with:

```sh
flatpak run io.github.jkli_2.anko_keyboard_configurator
```

For a packaging-system staging root, set `DESTDIR` instead of running the native
installer as root. An optional keyboardd path may be its sole positional argument.

## Uninstall

```sh
flatpak uninstall io.github.jkli_2.anko_keyboard_configurator
sudo packaging/native/uninstall.sh
systemctl --user daemon-reload
```

The native uninstaller removes only the four files installed by its matching installer.
The installer and uninstaller also remove the obsolete pre-release
`io.github.AnkoKeyboard` activation files when encountered.

## Validation status

On 2026-08-30, the manifest completed a clean Flatpak release build and AppStream
composition using GNOME 50. The desktop and AppStream files passed their validators;
the resulting metadata grants only graphics/display access and session-bus access to
`io.github.jkli_2.anko_keyboard_configurator.Daemon`. A release daemon build and temporary `DESTDIR` install also
passed without modifying the host installation.

The native companion and Flatpak-to-host activation were verified on 2026-08-30. A
`GetInfo` invocation from inside the installed Flatpak activated the hardened user
service and returned the connected FDA1 firmware/protocol details. The remaining
packaging acceptance check is one reversible write which restores the captured state.

The application ID follows the project's GitHub namespace. GitHub account hyphens are
represented as underscores because dashes are not valid in that reverse-DNS component.

The client stores its active profile through GLib's per-user data directory under
`anko-keyboard/profiles/active.json`. In the Flatpak this resolves inside the app's
private data area. Import and export use the desktop file chooser, so users can move a
portable profile JSON across the sandbox boundary without granting broad home access.

## Future standalone Flatpak

An all-in-one Flatpak is deferred. Its preferred device boundary is the
[USB portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Usb.html),
not `--device=all`.

The portal returns an authorized USB file descriptor and documents wrapping it with
`hid_libusb_wrap_sys_device()`. Supporting that design requires deliberate work:

- change the current shared-hidraw transport to a libusb/libhidapi-usb path capable of
  consuming the portal file descriptor;
- implement portal permission, acquisition, release, and device-event handling;
- package the daemon inside the Flatpak while retaining one HID owner;
- define its sandbox-compatible activation and lifetime; and
- test portal/backend availability across supported distributions and desktops.

This is a future transport and lifecycle milestone, not a v1 packaging shortcut.
