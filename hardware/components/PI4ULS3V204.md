# Pin requirements: PI4ULS3V204

- References:
  - Datasheet: `PI4ULS3V204.pdf`, "Pin Description" table
- Package / orderable variant this table applies to: TSSOP-14, PI4ULS3V204LEX (pin numbers
  below are the TSSOP column; the datasheet also gives TQFN and CSP numbering for the same
  signals in other columns)

## Per-pin requirements

| Pin (TSSOP) | Function | Requirement |
|---|---|---|
| 1 (VCCA) | A-port supply, power | 1.1 V <= VCCA <= 3.6 V. |
| 2 (A1) | Input/output A1, I/O | Referenced to VCCA. |
| 3 (A2) | Input/output A2, I/O | Referenced to VCCA. |
| 4 (A3) | Input/output A3, I/O | Referenced to VCCA. |
| 5 (A4) | Input/output A4, I/O | Referenced to VCCA. |
| 6 | NC | Not connected. |
| 7 (GND) | Ground | |
| 8 (EN) | Output enable, input | Active high. Pull EN low to place all outputs in 3-state mode. |
| 9 | NC | Not connected. |
| 10 (B4) | Input/output B4, I/O | Referenced to VCCB. |
| 11 (B3) | Input/output B3, I/O | Referenced to VCCB. |
| 12 (B2) | Input/output B2, I/O | Referenced to VCCB. |
| 13 (B1) | Input/output B1, I/O | Referenced to VCCB. |
| 14 (VCCB) | B-port supply, power | 1.1 V <= VCCB <= 3.6 V. |

The translator has integrated 10 kOhm pull-up resistors on the I/O lines A1-4 and B1-4.

## Electrical limits and consumption

| Parameter | Value | Conditions |
|---|---|---|
| DC supply voltage, port A | -0.3 to 5.5 V | Absolute maximum |
| DC supply voltage, port B | -0.3 to 5.5 V | Absolute maximum |
| Vi(A) referenced DC input/output voltage | -0.3 to 5.5 V | Absolute maximum |
| Vi(B) referenced DC input/output voltage | -0.3 to 5.5 V | Absolute maximum |
| Enable control pin DC input voltage | -0.3 to 5.5 V | Absolute maximum |
| Short-circuit duration, I/O to GND | 40 mA | Absolute maximum |
| VCCA / VCCB recommended supply | 1.1 to 3.6 V | Recommended operating condition |
| VEN recommended range | 0 to 3.6 V | Recommended operating condition |
| VIO recommended range | 0 to 3.6 V | Recommended operating condition |
| Operating temperature range | -40 to 85 degC | |
