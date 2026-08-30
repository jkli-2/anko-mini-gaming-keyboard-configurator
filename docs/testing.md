# Testing

Run all offline tests:

```sh
cargo test --workspace
```

Run all compiler and lint checks:

```sh
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
```

Milestone 1 uses small inline fixed-packet fixtures in each codec module. Tests cover packet framing, full and final chunks, semantic action/RGB round trips, lighting state, malformed inputs, and macro storage boundaries.

Run the read-only hardware diagnostic:

```sh
cargo run -p keyboard-protocol --example info
```

Run the Milestone 2 read-only diagnostics:

```sh
cargo run -p keyboard-protocol --example dump-keymap -- base
cargo run -p keyboard-protocol --example dump-keymap -- fn
cargo run -p keyboard-protocol --example dump-lighting
cargo run -p keyboard-protocol --example dump-rgb
```

Probe one effect's stored defaults without applying it:

```sh
cargo run -p keyboard-protocol --example probe-lighting-effect -- 19
```

The diagnostic reads active lighting before and after the `06 16` request and fails if
it changed. It never sends the `06 0B` commit. Use only a specifically justified effect
ID; do not sweep undefined values. A boundary probe found effect 19 valid and effect 20
undefined-looking while preserving active effect 1.

For focused lighting RE, start the daemon with raw request/response tracing:

```sh
ANKO_KEYBOARD_TRACE_LIGHTING=1 cargo run -p keyboardd
```

This prints only the relevant `06 16`, `06 0B`, and `06 0A` byte ranges. It established
that `06 0B` echoes the stable `color,index,H,S,V` config layout while the separate
`06 0A` status response uses the firmware's single/dynamic status representations.

The diagnostics select only USB `36AE:FDA1` collection `FF00:0002`. They share an advisory lock and issue only established read commands. Normal tests never access or modify hardware.

Run and inspect the Milestone 3 daemon using the commands in `docs/dbus.md`. The verified read-only calls are `GetInfo`, `Refresh`, and `GetKeymap` for `base` and `fn`.

## Reversible keymap-write diagnostic

This is an explicit hardware-write test and is never run by `cargo test`. Start
`keyboardd`, then run in another terminal:

```sh
cargo run -p keyboardd --example reversible-keymap-write
```

The diagnostic captures the original Base keymap, temporarily changes Esc, and tests
both `SetKey` and `SetKeymap`. After each path it verifies the changed value, restores
the captured original state, and verifies restoration. A bulk write compares the full
physical map through D-Bus; the daemon also verifies the complete 75-record image so
unused matrix records are covered.

Do not unplug the keyboard, stop the daemon, or terminate the diagnostic while it is
running. Treat any restoration failure as requiring immediate manual recovery before
further write testing.

## Reversible lighting-write diagnostic

This is also an explicit hardware-write test and is never run by `cargo test`. Start
`keyboardd`, then run in another terminal:

```sh
cargo run -p keyboardd --example reversible-lighting-write
```

It captures global lighting and the complete physical RGB map. It temporarily changes
the lighting effect and Base Esc colour, verifying `SetLighting`, `SetKeyColor`, and
`SetKeyColors`. Each operation is read back, restored, and read back again. The RGB
test uses a calibrated orange or cyan preset; those values are
passed as direct device RGB rather than through a lossy conversion.

Do not unplug the keyboard, stop the daemon, or terminate the diagnostic while it is
running. Treat any restoration failure as requiring immediate manual recovery.

## GTK client

Build and test the GTK client with the normal workspace commands. Its local keymap draft
model has an offline edit/revert/commit test.

For live read-only integration, start `keyboardd`, then run:

```sh
cargo run -p keyboard-ui
```

On 2026-08-30 this originally proved GTK → D-Bus → daemon against the physical
keyboard. The client now loads connection data, Base, Fn, and global lighting only;
per-key RGB was removed from the UI after RE showed that FDA1 cannot render its stored
map. The daemon methods remain covered by the reversible Milestone 5 diagnostic.
