# Pin requirements: SMAJ10CA

- References:
  - Datasheet: `Littelfuse-TVS-Diode-SMAJ-Datasheet.pdf` (Littelfuse SMAJ series), revised
    12/02/25 V.1. "Maximum Ratings and Thermal Characteristics" table and "Electrical
    Characteristics" table (row SMAJ10CA, single-die part)
- Package / orderable variant this table applies to: DO-214AC (SMA), Littelfuse SMAJ10CA
  (bidirectional TVS)

## Per-pin requirements

2-terminal part (bidirectional TVS diode) - the datasheet does not assign a distinct
requirement per terminal. Bidirectional parts carry no cathode band.

## Electrical limits and consumption

| Parameter | Value | Conditions |
|---|---|---|
| Reverse stand-off voltage (VR) | 10.0 V | - |
| Breakdown voltage (VBR) | 11.10 to 12.30 V | At test current IT = 1 mA, TA = 25 degC |
| VBR temperature coefficient | 0.1 %/degC typ | VBR at TJ = VBR at 25 degC x (1 + aT x (TJ - 25)) |
| Maximum clamping voltage (VC) | 17.0 V | At peak pulse current IPP = 23.5 A |
| Maximum peak pulse current (IPP) | 23.5 A | 10/1000 us waveform |
| Maximum reverse leakage current (IR) | 5 uA | At VR |
| Peak pulse power dissipation (PPPM) | 400 W | 10/1000 us waveform, TA = 25 degC, non-repetitive, mounted on 5.0 x 5.0 mm copper pad per terminal; derated above TJ = 25 degC |
| Power dissipation (PD) | 3.3 W | Infinite heat sink, TL = 50 degC |
| ESD immunity (IEC 61000-4-2) | 30 kV air / 30 kV contact | Device-level rating |
| Operating junction temperature | -65 to 150 degC | |
| Thermal resistance | 30 degC/W junction to lead, 120 degC/W junction to ambient | Typical |
