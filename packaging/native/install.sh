#!/bin/sh
set -eu

if [ "$#" -gt 1 ]; then
    echo "usage: $0 [path-to-keyboardd]" >&2
    exit 2
fi

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
keyboardd_binary=${1:-"$script_dir/../../target/release/keyboardd"}
destdir=${DESTDIR:-}

if [ ! -x "$keyboardd_binary" ]; then
    echo "keyboardd binary not found or not executable: $keyboardd_binary" >&2
    echo "build it first with: cargo build --release -p keyboardd" >&2
    exit 1
fi

if [ -z "$destdir" ] && [ "$(id -u)" -ne 0 ]; then
    echo "run this installer as root, or set DESTDIR for staged packaging" >&2
    exit 1
fi

install -Dm0755 "$keyboardd_binary" "$destdir/usr/libexec/anko-keyboard/keyboardd"
install -Dm0644 "$script_dir/io.github.jkli_2.anko_keyboard_configurator.Daemon.service" \
    "$destdir/usr/lib/systemd/user/io.github.jkli_2.anko_keyboard_configurator.Daemon.service"
install -Dm0644 "$script_dir/io.github.jkli_2.anko_keyboard_configurator.Daemon.dbus.service" \
    "$destdir/usr/share/dbus-1/services/io.github.jkli_2.anko_keyboard_configurator.Daemon.service"
install -Dm0644 "$script_dir/70-anko-keyboard.rules" \
    "$destdir/usr/lib/udev/rules.d/70-anko-keyboard.rules"

# Remove the pre-release service names when upgrading an earlier installation.
rm -f -- \
    "$destdir/usr/lib/systemd/user/io.github.AnkoKeyboard.service" \
    "$destdir/usr/share/dbus-1/services/io.github.AnkoKeyboard.service"

if [ -z "$destdir" ]; then
    udevadm control --reload-rules
    echo "Native companion installed."
    echo "Reconnect the keyboard, then run: systemctl --user daemon-reload"
else
    echo "Native companion staged below $destdir"
fi
