# Arch User Repository package

Until the project has a tagged release, the AUR packaging is a VCS split package:

- `anko-mini-gaming-keyboard-configurator-git` installs the native GTK client and
  depends on the daemon package; and
- `anko-mini-gaming-keyboard-configurator-daemon-git` installs only the native daemon,
  D-Bus/systemd activation files, and udev rule. It is suitable for use with the
  Flatpak client.

To test both packages from this directory:

```sh
makepkg --syncdeps --install
```

To install only the daemon package after building:

```sh
sudo pacman -U anko-mini-gaming-keyboard-configurator-daemon-git-*.pkg.tar.zst
```

Pacman's systemd and udev hooks reload the installed unit and rule. Reconnect the
keyboard after installation so logind applies its device-access ACL.

Before publishing or after changing `PKGBUILD`, regenerate `.SRCINFO`:

```sh
makepkg --printsrcinfo > .SRCINFO
```

The AUR repository should contain `PKGBUILD` and `.SRCINFO` (plus this optional
README), not a copy of the application source tree.
