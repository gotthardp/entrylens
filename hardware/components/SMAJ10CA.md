# Pin requirements: SMAJ10CA

- References:
  - Datasheet: `SMAJ10CA.pdf`, "Maximum Ratings and Characteristics" and "Electrical
    Characteristics" tables
- Package / orderable variant this table applies to: SMA, SMAJ10CA (bidirectional TVS)

## Per-pin requirements

2-terminal part (bidirectional TVS diode) - the datasheet does not assign a distinct
requirement per terminal.

## Electrical limits and consumption

| Parameter | Value | Conditions |
|---|---|---|
| Reverse stand-off voltage (VRWM) | 10.0 V | - |
| Breakdown voltage (VBR) | 11.10 to 12.30 V | At test current IT = 1 mA |
| Maximum clamping voltage (VC) | 17.0 V | At peak pulse current IPP = 23.5 A |
| Peak pulse current (IPPM) | 23.5 A | 10/1000 us waveform |
| Reverse leakage current (IR) | 5 uA max | At VRWM |
| Peak pulse power dissipation (PPPM) | 400 W min | 10/1000 us waveform, mounted on 5x5 mm copper pads |
| Steady-state power dissipation (PM(AV)) | 3.3 W | At TA = 50 degC |
| Peak forward surge current (IFSM) | 40 A | 8.3 ms single half sine-wave, JEDEC method |
| ESD immunity (IEC 61000-4-2) | 30 kV air / 30 kV contact | Device-level rating |
| Operating junction temperature | -65 to 150 degC | Absolute maximum |
