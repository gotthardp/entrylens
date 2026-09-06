# Pin requirements: <PART NUMBER>

List what the datasheet itself requires for each pin. Nothing here should be re-derived from the
schematic/netlist/BOM. Fill this in by reading the datasheet's pin description table directly,
not from memory or a prior review's prose.

Skip this entirely when the datasheet is already simple to read directly (e.g. a single-series
connector catalog with one plain rating). Use it where extraction is genuinely error-prone:
dense pin-function tables, multiple bundled variants, or a numeric limit worth pinning down.

- References:
  - Datasheet: `<filename.pdf>`, revision `<rev/date>`, pin table on page/section `<X>`
  - `<other supporting document, if any>`: `<filename.pdf>` or URL
- Package / orderable variant this table applies to: `<e.g. LGA-16>`

## Per-pin requirements

Include every pin, including power/GND and reserved/NC ones. If a requirement is mode-dependent
(e.g. SPI vs. I2C), say so in the cell rather than picking one.

For a simple 2-terminal part (diode, inductor, ferrite bead) where the datasheet doesn't
assign a distinct requirement per pin, skip this table (one line noting that) and rely on
"Electrical limits and consumption" below as the primary content instead.

| Pin | Function | Requirement |
|---|---|---|
| <e.g. C2> | <e.g. NCS, digital input> | <e.g. "SPI: 47k pullup to IOVDD. I2C: not used, 47k pulldown to GND."> |

## Electrical limits and consumption

Include only figures worth double-checking - absolute maximum ratings (stress limits) and/or
typical/worst-case current or power consumption figures, when either is used elsewhere as a
specific number (e.g. a budget or margin calculation).

| Parameter | Value | Conditions |
|---|---|---|
| <e.g. pin voltage> | <e.g. 7 V> | <e.g. absolute maximum> |
| <e.g. supply current, active state> | <e.g. 50 mA typ / 80 mA max> | <e.g. worst case, 85 degC> |
