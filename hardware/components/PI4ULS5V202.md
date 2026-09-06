# Pin requirements: PI4ULS5V202

- References:
  - Datasheet: `PI4ULS5V202.pdf`, "Pin Description" table
- Package / orderable variant this table applies to: MSOP-8 (also offered in UDFN-8, same
  pinout), PI4ULS5V202UEX

## Per-pin requirements

| Pin | Function | Requirement |
|---|---|---|
| 1 (VCCA) | A-port supply, power | 1.2 V <= VCCA <= 5.5 V. |
| 2 (A1) | Input/output A, I/O | Referenced to VCCA. |
| 3 (A2) | Input/output A, I/O | Referenced to VCCA. |
| 4 (GND) | Ground | |
| 5 (EN) | Output enable, input | Active high. Pull EN low to place all outputs in 3-state mode. |
| 6 (B2) | Input/output B, I/O | Referenced to VCCB. |
| 7 (B1) | Input/output B, I/O | Referenced to VCCB. |
| 8 (VCCB) | B-port supply, power | 1.2 V <= VCCB <= 5.5 V. |

The translator has integrated 10 kOhm pull-up resistors on the I/O lines A1-2 and B1-2.

## Electrical limits and consumption

| Parameter | Value | Conditions |
|---|---|---|
| DC supply voltage, port A | -0.3 to 5.5 V | Absolute maximum |
| DC supply voltage, port B | -0.3 to 5.5 V | Absolute maximum |
| Vi(A) referenced DC input/output voltage | -0.3 to 5.5 V | Absolute maximum |
| Vi(B) referenced DC input/output voltage | -0.3 to 5.5 V | Absolute maximum |
| Enable control pin DC input voltage | -0.3 to 5.5 V | Absolute maximum |
| Short-circuit duration, I/O to GND | 40 mA | Absolute maximum |
| VCCA / VCCB recommended supply | 1.2 to 5.5 V | Recommended operating condition |
| VEN recommended range | GND to 5.5 V | Recommended operating condition |
| VIO recommended range | GND to 5.5 V | Recommended operating condition |
| Operating temperature range | -40 to 85 degC | |
