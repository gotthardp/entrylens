# Pin requirements: ST25DV04KC

- References:
  - Datasheet: `st25dv04kc.pdf`, Table 1 (8-pin packages signal names) and section 2 (Signal
    descriptions)
  - Energy harvesting / V_EH behavior during RF communication (app note):
    `an4913-energy-harvesting-delivery-impact-on-st25dvi2c-series-behaviour-during-rf-communication-stmicroelectronics.pdf`
  - I2C bus pull-up resistor sizing (TI app note, shared bus with VL53L8CX): `slva689.pdf`
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
| Operating supply current (I2C read or write, active) | 220 uA max (180 uA typ) | VCC = 1.8 V, fC = 1 MHz |
| Static standby supply current | 110 uA max (78 uA typ) | VCC = 1.8 V |
