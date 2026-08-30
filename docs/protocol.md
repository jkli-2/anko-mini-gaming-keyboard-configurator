# Protocol implementation

All native hidapi write buffers are 65 bytes:

```text
00 <64-byte vendor report>
```

The leading `00` is report ID 0. Vendor payloads begin with family `06`.

## Implemented codecs and transactions

- `06 05`: configuration request and response.
- `06 08`: 56-byte keymap block read request.
- `06 09`: bulk keymap block write, with full-bank readback verification.
- `06 10`: single sparse-index key write, with readback verification.
- `06 0A`, `06 0B`, `06 16`: global lighting read, verified write, and effect-selection request.
- `06 12`, `06 13`, `06 14`: verified bulk RGB write, RGB block read, and verified single-key RGB write.
- `06 0C`, `06 0D`: macro storage block read and verified write with restoration on failure.
- `06 0F FF`: factory reset encoding; never exercised by automated tests.
- Four-byte key-action records, with a raw fallback for unknown/non-canonical values.
- The understood 4096-byte macro pointer table and four-byte event records.

## Bank mapping

The semantic API exposes only `Base` and `Fn`:

```text
Base -> wire layer 0
Fn   -> wire layer 1
```

Wire layers 2 through 5 are never emitted because physical testing established that they alias the Fn bank.

## Safe storage extent

The physical matrix is `5 x 15`, so keymap and RGB diagnostics read indices 0 through 74:

```text
keymap: 75 * 4 = 300 bytes
RGB:    75 * 3 = 225 bytes
```

The vendor client declares a generic `key_index_max` of 126 and constructs a UI array of that size. Hardware reads on 2026-08-29 showed that continuing to 126 crosses storage boundaries: Base reads expose Fn-like data and RGB reads expose keymap-like data. Native code therefore retains 126 only as documented vendor metadata and uses 75 as the safe protocol extent.

## Block layout

Keymap and RGB read replies carry their data beginning at response byte 8. Bulk keymap and RGB writes carry data beginning at vendor-payload byte 8 (hidapi buffer byte 9). Full blocks contain 56 data bytes; final blocks use `data length + 3` in the command length field.

## Test evidence

Small fixed packets are kept inline beside their codecs under `#[cfg(test)]`, where the source-to-fixture relationship remains obvious. The expected bytes come from reverse-engineered captures, the physically confirmed single-key layout, and vendor packet builders. Hardware writes are not part of normal tests. The explicit reversible D-Bus diagnostic is documented separately in `docs/testing.md`.

Both keymap write transactions capture the original state first. A failed write or
readback triggers restoration plus a second readback; if restoration itself fails, the
transport reports both errors distinctly.

Lighting and RGB writes use the same capture, verification, and restoration discipline.
An effect change sends `06 16` before `06 0B`. The `06 0B` request and immediate response
consistently use `color, singleColorIndex, H, S, V` in body bytes 6 through 10. The
separate `06 0A` status response is different: this keyboard returned
`FF,H,S,V,padding` for single colour and `01,index,H,S,V` for dynamic RGB. Separate
encoding/status-decoding paths preserve this distinction.

Lighting verification is capability-aware. It always verifies kind and effect, then
compares only fields meaningful to that effect: Off ignores normalized inactive
parameters, Static ignores Speed/Direction, and only effects 5/6/7/14 compare
Direction. Colour mode and preserved HSV remain strict for every non-off effect; the
read-side index may be firmware-normalized. Genuine mismatches report decoded expected
and actual states.

Per-key RGB remains three direct device bytes. The measured preset table is perceptual
calibration evidence for the semantic/UI layer and is not baked into packet encoding.

Macro transactions read all 4096 bytes in blocks of at most 56 and write blocks of at
most 59. A complete write captures the original raw store, verifies exact raw readback,
and restores the original bytes on failure. On 2026-08-30, FDA1 returned an empty valid
store; temporary M15 A press/release events were written and read back exactly, then
deleted and the empty store was reconfirmed.

The official frontend's global hex/RGB picker does not use a direct global-RGB wire
format: it stores RGB locally, converts the selected colour to HSVA, quantizes H/S/V,
and writes those bytes through `06 0B`. Runtime RE confirmed that it cannot preserve
arbitrary exact RGB values. By contrast, `06 12`, `06 13`, and `06 14` carry direct
per-key RGB bytes.

## Diagnostic serialization

Each diagnostic uses one `KeyboardDevice` handle. Transactions drain pending input before writing, ignore known `AA FA` events while waiting, and use the original 200 ms timeout. A shared Linux advisory lock at `/tmp/anko-keyboard-36ae-fda1.lock` serializes separate project processes. This prevents the reply crossover reproduced when two read-only diagnostics were initially launched concurrently.
