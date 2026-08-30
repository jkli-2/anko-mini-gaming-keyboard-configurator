# GTK client

Milestone 6 adds `keyboard-ui`, a GTK 4/libadwaita client with application ID:

```text
io.github.jkli_2.anko_keyboard_configurator
```

The client depends on `keyboard-core` for semantic layout data and on `zbus` for the
public daemon API. It deliberately has no dependency on `hidapi` or
`keyboard-protocol`; `keyboardd` remains the sole HID owner.

## Running

Start the daemon:

```sh
cargo run -p keyboardd
```

Then start the client:

```sh
cargo run -p keyboard-ui
```

The build requires GTK 4 and libadwaita development packages discoverable as `gtk4`
and `libadwaita-1` through `pkg-config`.

## Source layout

The GTK client is split by responsibility:

```text
src/main.rs       application bootstrap only
src/app.rs        state, window composition, signals, and D-Bus events
src/chord.rs      shared GTK chord capture and GDK-to-HID conversion
src/keys.rs       Keys page, action palette, keyboard construction, KLE matching
src/kle.rs        KLE raw-JSON parser and generated per-key colour CSS
src/lighting.rs   Lighting page builder
src/macros.rs     semantic Macro editor and source-derived offline codec
src/settings.rs   Settings page builder
src/style.rs      shared application CSS
```

`src/dbus.rs` remains the client worker/API boundary and `src/model.rs` contains the
local keymap draft model.

## Screens

### Keys

- Renders the board geometry, keycap sizes, legends, and presentation colours from the
  bundled Keyboard Layout Editor (KLE) raw JSON at
  `resources/layouts/anko-mini-gaming-keyboard.json`.
- Keeps `keyboard-core::PHYSICAL_KEYS` authoritative for the 63 device key IDs and
  sparse protocol indices. KLE controls presentation only; legend/geometry matching
  must never redefine the hardware map.
- Loads Base and Fn independently through the user-facing Layer selector.
- Shows the selected layer's effective friendly assignment on every keycap. When an
  assignment differs, the physical legend remains visible as smaller secondary text.
- Highlights the selected key in the layout; clicking it also shows its canonical
  semantic action in the editor.
- Fits the full keyboard width without horizontal scrolling, with shrinking labels and
  an 800 x 500 minimum window size to keep controls usable. The default window is
  900 x 750.
- Edits remain local to the selected layer until Apply calls `SetKeymap`.
- Revert discards that layer's local draft without touching hardware.
- Tooltips retain the physical legend, friendly assignment, and canonical action.

The keycaps translate the daemon's canonical action strings into friendly labels. A
categorized palette covers letters, digits, symbols, controls, navigation, standalone
modifiers, F1–F24, media/consumer controls, confirmed power-system actions, M0–M15, and
one-chord shortcuts. The Shortcut row reuses the Macro editor's armed GTK capture logic,
requires a modifier plus one HID key, and writes the resulting `keyboard/usage/mask`
action directly into the selected layer draft without consuming a macro slot. Existing
shortcut assignments preload the row. The canonical text editor is confined to Advanced
mode. The Mouse category exposes the official client's confirmed left, right, middle,
back, forward, wheel-up, and wheel-down records. Modifiers also includes the special Fn
layer-switch action, distinct from the ordinary F1–F24 host keys.

If the bundled KLE data is invalid or cannot be matched one-to-one with the physical
keys, the client falls back to its built-in geometry rather than guessing device IDs.

The assignment palette has a separate **Keyboard Controls** category for operations
handled inside the keyboard rather than emitted to the host: LED on/off, brightness
up/down, next effect, next colour, effect speed up/down, keyboard lock, Mac/Windows
mode, F-key/media mode, WASD/arrow mode, and White Light mode. These use canonical
`firmware/0` through `firmware/7`, `firmware/13`, `firmware/14`, `firmware/19`,
`firmware/36`, and `firmware/60` actions. They are deliberately separate from
**System**, which contains host power, sleep, and wake actions.

The stock Fn map assigns `firmware/36` to Fn+W for WASD/arrow mode and `firmware/19`
to Fn+Left Ctrl for F1–F12/media mode. The lighting/lock combinations invoke the other
onboard controls listed above. The client can assign all these confirmed records, but
it cannot currently read or display the active WASD, F-key/media, Mac/Windows, or White
Light mode states.

White Light mode overrides the visible lighting without replacing the stored HSV
configuration. Consequently, the client may successfully apply and verify an underlying
colour while the keyboard remains visibly white until the user turns that firmware mode
off. The mode's active state is not exposed by current readback.

Settings can import one custom KLE raw-JSON layout or return to the bundled default.
Imports are validated for the FDA1's 63 physical keys and positive geometry, then stored
at `glib::user_data_dir()/anko-keyboard/layouts/custom.json`. Layout changes apply on the
next client launch; invalid or externally corrupted custom files fall back to default.

### Lighting

- Reads and edits global effect, brightness, speed, direction, colour-enabled state,
  and the currently understood device HSV bytes.
- Constrains Brightness and Speed to the official client's `0..=4` range.
- Applies the reverse-engineered FDA1 effect capability table: Static has no Speed
  or Direction; Stream, Bloom, and Twinkling Star have horizontal direction; UD Wave
  has vertical direction; the other animated effects have Speed but no Direction.
- Includes Off as firmware effect `0`; selecting it disables the inapplicable controls.
- Uses grouped Left/Right or Up/Down toggle buttons instead of exposing the direction
  byte as a number.
- Presents colour mode as grouped `Dynamic RGB` / `Single Colour` buttons for every
  non-off effect. The HSV fields are editable only in single-colour mode; dynamic RGB
  retains their stored values but does not use them.
- Applies global changes through verified `SetLighting`.
- Scrolls vertically when the available window height cannot contain both configuration
  cards and the Apply button.
- Does not expose per-key RGB editing. Exact storage readback exists in the lower-level
  protocol, but this model lacks the official client's required `custom` display effect.

The complete source-derived capability grouping is:

| Effects | Brightness | Speed | Direction | Colour mode |
|---|---:|---:|---|---|
| Off | no | no | no | no |
| Static | yes | no | no | dynamic RGB / single |
| Stream, Bloom, Twinkling Star | yes | yes | left/right | dynamic RGB / single |
| UD Wave | yes | yes | up/down | dynamic RGB / single |
| All other animated effects | yes | yes | no | dynamic RGB / single |

The retained daemon/protocol per-key storage methods use direct RGB device bytes but
are not exposed by this UI. Global single-colour lighting is still exposed as raw HSV
because ordinary RGB↔HSV conversion is lossy. RE of the running
official client confirmed that it stores the requested RGB locally for its UI, converts
it to the same quantized HSV wire values, and cannot restore arbitrary exact RGB either.
A future RGB picker may mirror that convenience only if it clearly treats readback as
an approximation. Measured device values provide the named preset
swatches; no hidden calibration transform is applied.

Unchecked colour mode is transmitted as `color = 0`, index `0`, with the current HSV
preserved. A physical Static-mode retest applied dynamic RGB successfully and switching
back restored single-colour HSV 4/193/244 exactly. The UI follows the official client by
writing zero for the currently reserved/index byte.

### Settings

- Uses a vertically scrollable, width-clamped layout so the structured cards remain
  readable in shorter or wider windows.
- Shows connection, product, firmware, protocol, layer count, and application version as
  individual rows rather than one diagnostic text block.
- The header refresh button calls `Refresh` and reloads all displayed state.
- Provides the single custom/default KLE control, import guidance, and links to KLE plus
  the reference keyboard layout. Changes apply after restart.
- Exposes factory reset only behind an explicit destructive confirmation dialog.
- Does not yet provide device-profile backup or restore. Factory reset returns the
  keyboard to stock state and cannot recover the user's previous Base/Fn maps, lighting,
  or macros; users must treat it as destructive rather than as a profile workflow.

### Macros

- Loads all 16 slots from the keyboard and presents them as semantic keystroke/chord,
  Wait, mouse-click, and scroll steps instead of exposing press/release events by
  default. A GTK capture button temporarily displays `Press a key…` while accepting a
  physical chord, with a compact keycap picker and modifier menu as a mouse-driven
  alternative.
- Compiles semantic steps to the firmware event format only when saving. Adjacent
  chords keep their shared Ctrl, Shift, Alt, or Super modifiers held, reducing redundant
  releases without changing chord order or per-step delays. A read-only Raw Events
  popover shows the exact compiled sequence without consuming editor height.
- Action rows expose an explicit `After` value. Wait is a distinct step with a Duration,
  although the codec necessarily folds it into the preceding firmware event's delay.
  The client preserves that semantic distinction in `anko-keyboard/macro-steps.json` and
  uses it only when its compiled events still match the keyboard. A Wait cannot be first
  because this firmware has no delay-before-first-event representation.
- Rows can be clicked to edit, moved with the up/down controls or drag-and-drop, and
  deleted individually. After/Duration values remain bounded to the confirmed
  `0..=10000` ms range.
- The pencil beside the slot heading assigns an optional local display name while the
  firmware and D-Bus interfaces continue to use M0–M15. Names are stored as JSON below
  GLib's per-user data directory at `anko-keyboard/macro-names.json`.
- Keeps edits local until Save Macro; Clear plus Save deletes the selected definition,
  while Revert restores the last device read.
- Uses exact full-store readback verification and rollback in the daemon. Continuous
  native event recording remains a later refinement.

The user manually verified semantic compilation, local names, physical chord capture,
row reordering, normal macro editing, and cancellation of the factory-reset confirmation
on 2026-08-30.

## Concurrency and errors

All blocking D-Bus calls run on one client worker thread. GTK polls typed results on its
main context, so hardware reads and writes do not block rendering. The daemon still
serializes every HID transaction independently.

The UI reports D-Bus validation and hardware errors without committing local keymap
draft state. A successful `SetKeymap` commits the matching draft; lighting and macro
writes are reloaded after success.

## Packaging

The accepted v1 packaging split is a Flatpak containing only this GTK client plus a
manually/native-installed host daemon with D-Bus/systemd activation and a narrow udev
rule. The future all-in-one Flatpak is reserved for a USB-portal transport. See
[`packaging.md`](packaging.md) for the complete decision and required artifacts.

Automatic daemon/device reconnect signals are not implemented yet. Use the header
refresh button after starting `keyboardd` or reconnecting the keyboard.
