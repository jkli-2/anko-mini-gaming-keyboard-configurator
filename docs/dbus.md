# D-Bus API

The daemon exposes one keyboard on the user session bus:

```text
service:   io.github.jkli_2.anko_keyboard_configurator.Daemon
object:    /io/github/jkli_2/anko_keyboard_configurator/Daemon
interface: io.github.jkli_2.anko_keyboard_configurator.Daemon
```

The daemon is the sole HID owner. Every D-Bus hardware request is sent to one worker thread through a channel and completes only after the HID response has been decoded.

## Properties

```text
Connected          b  True only while a live device handle is cached
ConnectionState    s  disconnected, connecting, connected, or error
FirmwareVersion    q  Firmware version, or 0 while unavailable
```

Milestone 3 properties are uncached from the D-Bus client's perspective and do not yet emit `PropertiesChanged`. State signals will be added with the broader daemon lifecycle work.

## Methods

### GetInfo

Signature:

```text
() -> (s q q q y s)
```

Returns:

```text
state
product ID
firmware version
protocol version
effective layer count
last error (empty on success)
```

This method reads cached worker state and does not transact with hardware.

### Refresh

Signature:

```text
() -> ()
```

Drops any old HID handle, reacquires the exact target collection, reads `06 05`, and updates cached state. It returns only after success or a concrete error.

### GetKeymap

Signature:

```text
(s bank) -> a(ss)
```

The bank must be `base` or `fn`. The result contains 63 `(stable key ID, canonical action)` pairs. Sparse protocol indices and unused matrix positions are not exposed.

Canonical actions currently use:

```text
keyboard/<usage>/<modifiers>
function-layer
consumer/<usage>
mouse-button/<buttons>/<vertical-wheel>
mouse-move/<x>/<y>/<wheel>
macro/<id>
firmware/<code>
power/<code>
raw/<kind>/<code1>/<code2>/<code3>
```

The `raw` form is reserved for unknown or non-canonical records so they remain lossless.

### SetKey

Signature:

```text
(s bank, s key_id, s canonical_action) -> ()
```

Writes one of the 63 stable physical key IDs. The bank, key ID, action shape, numeric
ranges, and macro ID (`0..=15`) are validated before the worker receives the command.
The call returns only after a full keymap readback confirms the requested record. On a
write or verification failure, the daemon attempts to restore and verify the captured
original assignment before returning the error.

### SetKeymap

Signature:

```text
(s bank, a(ss) assignments) -> ()
```

Requires exactly one assignment for every physical key: 63 known, unique stable IDs.
The worker reads the complete 75-record bank, overlays the physical assignments, and
writes that complete image in `06 09` blocks. This preserves all 12 unused sparse-matrix
records. It verifies all 75 records by readback and attempts a verified restoration of
the original image on failure.

### GetLighting / SetLighting

Signatures:

```text
GetLighting() -> (y kind, y effect, y brightness, y speed, y direction,
                  b color_enabled, y single_color_index,
                  y hue, y saturation, y value)
SetLighting(y kind, y effect, y brightness, y speed, y direction,
            b color_enabled, y single_color_index,
            y hue, y saturation, y value) -> ()
```

These are exact device bytes. Kind must be `1`, effects are `0..=19`, and an off
effect (`0`) cannot have colour enabled. An effect change performs `06 16` before
committing with `06 0B`. Writes are read back exactly and restoration is attempted on
failure. The index is retained in the semantic API, although the GTK client follows the
official client and currently writes zero. `06 0B` config and `06 0A` status responses
have distinct tail layouts as documented in `docs/protocol.md`.

### GetKeyColors / SetKeyColor / SetKeyColors

Signatures:

```text
GetKeyColors() -> a(syyy)
SetKeyColor(s key_id, y red, y green, y blue) -> ()
SetKeyColors(a(syyy) assignments) -> ()
```

Colours are exact device RGB bytes. Bulk replacement requires all 63 known, unique
physical IDs. The daemon overlays them onto a freshly read 75-record image, preserving
all unused matrix records. Single and bulk writes are verified and attempt restoration
on failure.

The reverse-engineered calibration data contains measured perceptual presets for the UI. The
D-Bus API deliberately does not apply a hidden calibration transform, keeping reads,
writes, and restoration lossless.

### GetMacros / SetMacro / DeleteMacro

Signatures:

```text
GetMacros() -> a(ya(qyby))
SetMacro(y id, a(qyby) events) -> ()
DeleteMacro(y id) -> ()
```

Each event is `(delay_ms, kind, pressed, code)`. Kinds `0..=3` mean mouse button,
keyboard, vertical scroll, and horizontal scroll. IDs are restricted to `0..=15`,
saved macros must contain an event, and delays are limited to `0..=10000` ms. Writes
replace the complete 4096-byte store, verify exact readback, and restore the captured
original bytes on failure while preserving every other macro slot.

### FactoryReset

Signature: `FactoryReset() -> ()`. This sends the identified destructive `06 0F FF`
operation. The GTK client is the policy boundary and requires an explicit destructive
confirmation dialog. Factory reset is never called by automated tests.

### CaptureHardwareSnapshot / RestoreHardwareSnapshot

Signatures:

```text
CaptureHardwareSnapshot() -> s snapshot_json
RestoreHardwareSnapshot(s snapshot_json) -> ()
```

The versioned snapshot contains both complete 75-record keymaps, exact global lighting,
all 75 RGB storage records, device/firmware/protocol identity, and the complete raw
4096-byte macro region. Raw key records and unknown-but-structurally-valid macro events
are preserved byte-for-byte rather than being translated through the semantic APIs.

Restore parses and validates the complete snapshot before its first write. It rejects a
different VID/PID or protocol version, malformed region sizes and pointers, and invalid
lighting state. The daemon captures a fresh pre-restore snapshot, writes each region with
exact readback verification, and attempts to restore the entire captured state if any
section fails. A returned rollback error identifies both the original failure and the
failed recovery; the caller must treat that as potentially mixed device state.

These methods contain hardware state only. The GTK profile envelope adds client-local
macro names, semantic steps, and non-applied layout metadata.

## Manual verification

Start the daemon:

```sh
cargo run -p keyboardd
```

In another terminal:

```sh
busctl --user introspect io.github.jkli_2.anko_keyboard_configurator.Daemon /io/github/jkli_2/anko_keyboard_configurator/Daemon
busctl --user call io.github.jkli_2.anko_keyboard_configurator.Daemon /io/github/jkli_2/anko_keyboard_configurator/Daemon io.github.jkli_2.anko_keyboard_configurator.Daemon GetInfo
busctl --user call io.github.jkli_2.anko_keyboard_configurator.Daemon /io/github/jkli_2/anko_keyboard_configurator/Daemon io.github.jkli_2.anko_keyboard_configurator.Daemon Refresh
busctl --user call io.github.jkli_2.anko_keyboard_configurator.Daemon /io/github/jkli_2/anko_keyboard_configurator/Daemon io.github.jkli_2.anko_keyboard_configurator.Daemon GetKeymap s base
busctl --user call io.github.jkli_2.anko_keyboard_configurator.Daemon /io/github/jkli_2/anko_keyboard_configurator/Daemon io.github.jkli_2.anko_keyboard_configurator.Daemon GetKeymap s fn
busctl --user call io.github.jkli_2.anko_keyboard_configurator.Daemon /io/github/jkli_2/anko_keyboard_configurator/Daemon io.github.jkli_2.anko_keyboard_configurator.Daemon GetLighting
busctl --user call io.github.jkli_2.anko_keyboard_configurator.Daemon /io/github/jkli_2/anko_keyboard_configurator/Daemon io.github.jkli_2.anko_keyboard_configurator.Daemon GetKeyColors
busctl --user call io.github.jkli_2.anko_keyboard_configurator.Daemon /io/github/jkli_2/anko_keyboard_configurator/Daemon io.github.jkli_2.anko_keyboard_configurator.Daemon GetMacros
```

`SetKey` can be called directly, but manual writes are easy to forget to restore. Use
the reversible Milestone 4 diagnostic documented in `docs/testing.md` for hardware
verification.

Use the corresponding reversible Milestone 5 diagnostic for all lighting writes.
