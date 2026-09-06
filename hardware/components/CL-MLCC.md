# Pin requirements: Samsung CL-series MLCCs

- References:
  - Datasheet: no local PDF. Samsung Electro-Mechanics publishes per-part data online at
    `https://weblib.samsungsem.com/mlcc/mlcc-ec-data-sheet.do?partNumber=<PART>`, where `<PART>`
    is the BOM's `Manufacturer Part` with its **trailing suffix letter removed** (the BOM's
    `CL21A226MAQNNNE` is queried as `CL21A226MAQNNN`).
- Package / orderable variant this table applies to: the 0402, 0603 and 0805 chip sizes listed.

## Per-pin requirements

2-terminal part - the datasheet assigns no distinct requirement per terminal.

## Part-number structure

`CL` `21` `A` `226` `M` `A` `Q` `NNN` `E`

| Field | Chars | Meaning | Codes seen |
|---|---|---|---|
| Series | `CL` | Samsung general-purpose MLCC | - |
| Case | 2 digits | EIA size | `05` = 0402, `10` = 0603, `21` = 0805 |
| Dielectric | 1 | Temperature characteristic | `A` = X5R, `B` = X7R, `C` = C0G |
| Capacitance | 3 | Two significant digits then decade exponent, in pF | `103` = 10 nF, `475` = 4.7 uF, `226` = 22 uF |
| Tolerance | 1 | EIA code | `J` = +/-5%, `K` = +/-10%, `M` = +/-20% |
| Rated voltage | 1 | | `Q` = 6.3 V, `P` = 10 V, `O` = 16 V, `A` = 25 V, `B` = 50 V |
| Thickness | 1 | | `5` = 0.50 mm, `8` = 0.80 mm, `Q` = 1.25 mm |
| Remainder | 4 | Not determined | `NNNC`, `NNNE`, `NRNC` observed |

So rated voltage is readable straight from the BOM's `Manufacturer Part` column without fetching
anything. Two traps: `Q` means 6.3 V in the voltage position but 1.25 mm in the thickness position
that follows it, and `A` and `B` mean dielectrics in one position and voltages in another.

This table was derived by decoding nine part numbers against their fetched data - the seven below
plus CL05C100JB5NNNC (10 pF C0G, the former C1, source of the `C` and `J` codes) and
CL10A475KO8NNNC (4.7 uF 16 V 0603, the former C16) - not transcribed
from a Samsung numbering guide. Every field is consistent across all nine, but codes outside those
listed are unverified - fetch the part rather than extrapolate.

## Electrical limits and consumption

Rated voltage is the figure worth pinning down: the BOM records capacitance and case size but not
voltage, and DC-bias derating scales with the fraction of rated voltage actually applied.

| Manufacturer Part | Value | Tol. | Dielectric | Case | Rated voltage |
|---|---|---|---|---|---|
| CL05B103KB5NNNC | 10 nF | +/-10% | X7R | 0402 | 50 V |
| CL05B104KO5NNNC | 100 nF | +/-10% | X7R | 0402 | 16 V |
| CL10A105KB8NNNC | 1 uF | +/-10% | X5R | 0603 | 50 V |
| CL10A226MQ8NRNC | 22 uF | +/-20% | X5R | 0603 | 6.3 V |
| CL10A106KP8NNNC | 10 uF | +/-10% | X5R | 0603 | 10 V |
| CL21A226MAQNNNE | 22 uF | +/-20% | X5R | 0805 | 25 V |
| CL21A475KAQNNNE | 4.7 uF | +/-10% | X5R | 0805 | 25 V |

The online datasheet states "Graphs for the item are not supported", so it publishes no DC-bias
curve. Rated voltage is therefore the only derating indicator the vendor provides.
