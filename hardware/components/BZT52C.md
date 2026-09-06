# Pin requirements: BZT52C10S / BZT52C6V2S

- References:
  - Datasheet: `BZT52C.pdf` (Zhuhai Hongjiacheng BZT52C2V0S through BZT52C75S series), "Maximum
    Ratings" and "Electrical Characteristics" tables - covers both part numbers used in this
    design, distinguished by row below
- Package / orderable variant this table applies to: SOD-323, BZT52C10S and BZT52C6V2S

## Per-pin requirements

2-terminal part (Zener diode) - the datasheet does not assign a distinct requirement per
terminal.

## Electrical limits and consumption

Series-wide maximum ratings (apply to both part numbers):

| Parameter | Value | Conditions |
|---|---|---|
| Power dissipation (PD) | 200 mW | Absolute maximum |
| Forward voltage (VF) | 0.9 V | At IF = 10 mA |
| Junction temperature | -55 to 150 degC | Absolute maximum |
| Typical thermal resistance (junction-ambient) | 417 degC/W | - |

Per-part electrical characteristics:

| Parameter | BZT52C10S | BZT52C6V2S | Conditions |
|---|---|---|---|
| Zener voltage (VZ), min/nom/max | 9.4 / 10 / 10.6 V | 5.8 / 6.2 / 6.6 V | At IZT |
| Test current (IZT) | 5 mA | 5 mA | - |
| Max zener impedance at IZT (ZZT) | 20 Ohm | 10 Ohm | At IZT |
| Max zener impedance at IZK (ZZK) | 150 Ohm | 150 Ohm | At IZK |
| Knee reverse current (IZK) | 1.0 mA | 1.0 mA | - |
| Reverse leakage current (IR) | 0.2 uA | 3 uA | At VR |
| Reverse voltage for IR test (VR) | 7.0 V | 4.0 V | - |
| Typical temperature coefficient | 4.5 to 8.0 mV/degC | 0.4 to 3.7 mV/degC | At IZT |
