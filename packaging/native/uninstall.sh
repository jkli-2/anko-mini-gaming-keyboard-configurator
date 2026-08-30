#!/bin/sh
set -eu

destdir=${DESTDIR:-}
if [ -z "$destdir" ] && [ "$(id -u)" -ne 0 ]; then
    echo "run this uninstaller as root, or set DESTDIR for staged packaging" >&2
    exit 1
fi

rm -f -- \
    "$destdir/usr/libexec/anko-keyboard/keyboardd" \
    "$destdir/usr/lib/systemd/user/io.github.jkli_2.anko_keyboard_configurator.Daemon.service" \
    "$destdir/usr/share/dbus-1/services/io.github.jkli_2.anko_keyboard_configurator.Daemon.service" \
    "$destdir/usr/lib/systemd/user/io.github.AnkoKeyboard.service" \
    "$destdir/usr/share/dbus-1/services/io.github.AnkoKeyboard.service" \
    "$destdir/usr/lib/udev/rules.d/70-anko-keyboard.rules"

if [ -z "$destdir" ]; then
    udevadm control --reload-rules
    echo "Native companion removed. Run: systemctl --user daemon-reload"
else
    echo "Native companion removed from $destdir"
fi
