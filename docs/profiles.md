# Device profiles

The client keeps one active, named profile. This is a client-side backup and is unrelated
to the keyboard's unsupported onboard profile command (`profileCnt = 0`).

The active file is stored below GLib's per-user data directory at
`anko-keyboard/profiles/active.json`. Use Settings → Export to create a portable copy;
inside Flatpak, the file chooser provides access outside the app's private data area.

## Version 1 format

The JSON envelope identifies itself with `format: "anko-fda1-profile"` and `version: 1`.
It contains a profile name and creation timestamp, a versioned `hardware` snapshot, and
client metadata. The hardware section holds:

- USB, firmware, and protocol identity;
- all 75 raw records from both Base and Fn keymaps;
- exact global lighting bytes;
- all 75 stored RGB triples; and
- the complete 4096-byte raw macro store.

The client section holds local M0–M15 names and semantic macro steps. Layout selection is
recorded only as descriptive metadata; importing or restoring a profile never installs,
selects, or removes a KLE layout.

## Safety model

Import validates the complete JSON and replaces only the local active profile. It never
writes the keyboard. Restore requires explicit confirmation and asks the daemon to
validate the hardware snapshot again before any write. The daemon captures the current
device, writes and verifies every region, and attempts a complete rollback if a section
fails. Preserve an exported known-good profile before destructive experiments.

Unknown raw key records and structurally valid unknown macro-event kinds are retained so
a backup made by this version can restore device data the semantic editors do not yet
understand. Files for another device, incompatible protocol versions, incomplete arrays,
invalid macro pointers, and invalid lighting states are rejected.
