# Pin requirements: ST25DV04KC

- References:
  - Datasheet: `st25dv04kc.pdf` (DS13519 Rev 8), Table 1 (8-pin packages signal names), section 2
    (Signal descriptions), Table 249 (I2C DC characteristics up to 85 degC), Table 251 and
    Figure 82 (I2C AC characteristics, pull-up sizing at 1 MHz)
  - Energy harvesting / V_EH behavior during RF communication (app note):
    `an4913-energy-harvesting-delivery-impact-on-st25dvi2c-series-behaviour-during-rf-communication-stmicroelectronics.pdf`
  - I2C bus pull-up resistor sizing (TI app note, shared bus with VL53L8CX): `slva689.pdf`
  - Errata sheet ES0617 Rev 5, April 2026 (device limitations, all ST25DVxxKC):
    `es0617-st25dv04kc-st25dv16kc-and-st25dv64kc-device-limitations-stmicroelectronics.pdf`
  - ST reference antenna artwork for the AC0/AC1 coil (net `ANT-C6`): `ant_c6_gerber.zip`
    (ST25DV-ANT-C6-REV-A Gerbers)
- Package / orderable variant this table applies to: SO8N, ST25DV04KC-IE8S3 (8-pin package;
  the 10-ball/12-pin packages add an LPD pin and a CMOS-type GPO with a VDCG supply pin, not
  covered here)

## Per-pin requirements

| Pin | Function | Requirement |
|---|---|---|
| 1 (V_EH) | Energy harvesting analog output | Delivers the analog V_EH voltage when energy harvesting mode is enabled and RF field strength is sufficient. When energy harvesting is disabled, or RF field strength is insufficient, this pin is high-Z. Output is not regulated. |
| 2 (AC0) | Antenna coil, RF | Connect to the external antenna coil. Do not connect to any other DC or AC path. |
| 3 (AC1) | Antenna coil, RF | Connect to the external antenna coil. Do not connect to any other DC or AC path. |
| 4 (VSS) | Ground | Reference for VCC, VDCG (not present on this package), and V_EH. |
| 5 (SDA) | Serial data, I/O | Bidirectional, open-drain output, may be wire-OR'ed with other open-drain/open-collector signals on the bus. A pullup resistor to VCC is required. |
| 6 (SCL) | Serial clock, input | Strobes data in/out. If used by target devices to synchronize to a slower clock, the bus controller must have an open-drain output and a pullup resistor must be connected from SCL to VCC. |
| 7 (GPO) | General-purpose output | Open-drain. Must be connected to an external pullup resistor (>4.7 kOhm) to operate. By default configured as an RF-field-change detector; can also flag RF activity, memory write completion, or fast-transfer events. |
| 8 (VCC) | Supply voltage | External DC supply; an internal regulator lets external VCC supply the device while preventing the internally-rectified RF supply from driving VCC. |

## Electrical limits and consumption

| Parameter | Value | Conditions |
|---|---|---|
| VCC (I2C supply voltage) | -0.5 to 6.5 V | Absolute maximum |
| VCC (I2C operating supply voltage) | 1.8 to 5.5 V | Recommended operating condition |
| VIO (digital I/O range: SDA, SCL, GPO) | -0.5 to 6.5 V | Absolute maximum |
| DC output current on SDA (low) | 5 mA max | Absolute maximum |
| DC output current on GPO (low) | 1.5 mA max | Absolute maximum |
| RF input voltage amplitude, AC0-AC1 (peak-to-peak) | 11 V | Absolute maximum, VSS floating |
| AC voltage, AC0-VSS or AC1-VSS | -0.5 to 5.5 V | Absolute maximum |
| Ambient operating temperature, RF interface | -40 to 105 degC | Absolute maximum, device grade 8, SO8N |
| Ambient operating temperature, I2C interface | -40 to 125 degC | Absolute maximum, device grade 8, SO8N |
| Operating supply current (I2C read or write, active) | 300 uA max (280 uA typ) | VCC = 3.3 V, fC = 1 MHz, worst mode (mailbox write), up to 85 degC |
| Static standby supply current | 100 uA max (76 uA typ) | VCC = 3.3 V, up to 85 degC |
| VIL (SDA, SCL) | 0.3 VCC max | VCC = 3.3 V |
| VIH (SDA, SCL) | 0.75 VCC min | VCC = 3.3 V |
| VOL (SDA, 1 MHz) | 0.4 V max | IOL = 2.1 mA, VCC = 3.3 V (at VCC = 1.8 V the same limit holds only at IOL = 1 mA) |
| Rbus x Cbus (SDA/SCL pull-up x bus capacitance) | below 150 ns | fC = 1 MHz, Figure 82: "must be below 150 ns"; the tCLQV 450 ns access time assumes it |

## Known limitations (ES0617 Rev 5)

Only the limitations this design can hit are listed. The LPD-pin reset workaround offered for the
first one is available only on product codes 51h and 53h; the SO8N ST25DV04KC is product code 50h
and has no LPD pin, so that route does not exist for this part.

| Section | Limitation | Workaround |
|---|---|---|
| 1.1.1 | Potential RF and I2C lock if VCC is disconnected then reconnected while an RF field is present. A floating VCC under RF sits at about 1.4 V; the device leaves that state by itself if the field goes away, if any RF command arrives (VCC then falls to 0 V), or if VCC rises above 1.8 V. But a 1.4 V -> 0 V -> >1.8 V sequence can deadlock it: all I2C accesses unacknowledged, every RF command answered with error 0Fh. The trigger is a negative VCC pulse **shorter than 1 ms**, which leaves internal capacitors partly charged so the internal reset does not complete | Hold VCC at 0 V for **longer than 1 ms** before disconnecting it, or before reconnecting it. Alternatively a capacitor on VCC whose time constant exceeds the glitch filters it out |
| 1.3.1 | Sub-carrier frequency drift on a marginal portion of production parts causes a communication hole at certain field strengths; more likely as ambient temperature falls | Move the tag closer or farther; a tap gesture is less affected because the protocol retries; revision 13h parts are fixed; some readers are insensitive to the drift |
| 1.4.1 | An I2C write carrying only the system device select code and memory address 0900h, with no data byte, corrupts the internal memory address pointer, so a following I2C write can program at the wrong address | Never write address 0900h without at least one data byte. Use 0900h only to present or write passwords. Any I2C random read to a valid address resets the pointer |
