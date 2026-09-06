# Pin requirements: GMF05LC-HSF

- References:
  - Datasheet: `gmf05lc.pdf` (Vishay GMF05LC-HSF, 5-line ESD protection diode array), pinout
    diagram and BiAs-mode description
- Package / orderable variant this table applies to: LLP75-6L, GMF05LC-HSF-GS08

## Per-pin requirements

This part protects up to 5 signal/data lines (labeled L1-L5 in the datasheet) against a common
ground pin. All 5 protected pins are electrically identical per the datasheet's own
specification table (same VRWM/VBR/VC/VF/CD for every channel).

| Pin | Function | Requirement |
|---|---|---|
| 1 | Protected signal/data line (L1-L5, BiAs mode) | See note below. |
| 2 | Ground | Common ground reference for BiAs-mode protection (each protected input clamps to this pin). |
| 3 | Protected signal/data line (L1-L5, BiAs mode) | See note below. |
| 4 | Protected signal/data line (L1-L5, BiAs mode) | See note below. |
| 5 | Protected signal/data line (L1-L5, BiAs mode) | See note below. |
| 6 | Protected signal/data line (L1-L5, BiAs mode) | See note below. |

Note (pins 1, 3, 4, 5, 6): normal operating range is 0 V (ground level) to the reverse
stand-off voltage VRWM = 5 V, where the diode presents high isolation to ground. Above the
breakdown voltage VBR (6-8 V), the diode conducts and clamps to VC. Negative transients are
clamped to VF close to ground level.

## Electrical limits and consumption

| Parameter | Value | Conditions |
|---|---|---|
| Peak pulse current (IPPM) | 5 A | BiAs mode, IEC 61000-4-5, tp = 8/20 us, single shot |
| Peak pulse power (PPP) | 70 W | BiAs mode, IEC 61000-4-5, tp = 8/20 us, single shot |
| ESD immunity, contact or air discharge | +/- 30 kV | IEC 61000-4-2, 10 pulses |
| Operating (junction) temperature | -55 to 125 degC | Absolute maximum |
| Reverse stand-off voltage (VRWM) | 5 V | At IR = 1 uA |
| Reverse breakdown voltage (VBR) | 6 to 8 V | At IR = 1 mA |
| Reverse clamping voltage (VC) | 8 to 9.5 V typ/max | At IPP = 1 A, IEC 61000-4-5 |
| Reverse clamping voltage (VC) | 11.5 to 12.5 V typ/max | At IPP = IPPM = 5 A, IEC 61000-4-5 |
| Forward clamping voltage (VF) | 1.5 to 2 V typ/max | At IF = 1 A, IEC 61000-4-5 |
| Forward clamping voltage (VF) | 3.1 to 4 V typ/max | At IPP = IPPM = 5 A, IEC 61000-4-5 |
| Capacitance (CD) | 43 to 50 pF typ/max | At VR = 0 V, f = 1 MHz |
| Capacitance (CD) | 25 pF typ | At VR = 2.5 V, f = 1 MHz |
