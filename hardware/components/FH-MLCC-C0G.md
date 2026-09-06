# Pin requirements: Fenghua C0G MLCCs

- References:
  - Datasheet: `MLCC-NPO.pdf` (Guangdong Fenghua, www.fenghua.com, "MLCC - NPO (COG)", 5 pages).
    The document carries no revision or date. How to Order (page 1), size code, capacitance and
    voltage table (page 3), dielectric characteristics and test methods (page 4)
- Package / orderable variant this table applies to: 0402 (1005 metric), 0402CG330J500NT (C1)

## Per-pin requirements

2-terminal part - the datasheet assigns no distinct requirement per terminal.

## Part-number structure

`0402` `CG` `330` `J` `500` `N` `T`

| Field | Chars | Meaning | Code in C1 |
|---|---|---|---|
| Size | 4 digits | EIA size | `0402` |
| Dielectric | 2 | `CG` = C0G (NPO) | `CG` |
| Capacitance | 3 | Two significant digits then decade exponent, in pF | `330` = 33 pF |
| Tolerance | 1 | `J` = +/-5.0 % | `J` |
| Rated voltage | 3 | `500` = 50 V | `500` |
| Termination | 1 | `N` = nickel barrier, tin plating | `N` |
| Packaging | 1 | `T` = tape and reel | `T` |

## Electrical limits and consumption

| Parameter | Value | Conditions |
|---|---|---|
| Capacitance | 33 pF +/-5 % | 25 degC, 1.0 V, 1 MHz |
| Rated voltage | 50 V | 0402 C0G at 50 V is offered from 1 pF to 220 pF |
| Dissipation factor | 0.15 % max | C < 1000 pF, 1 MHz |
| Insulation resistance | 5 x 10^10 Ohm min | At rated voltage, 60 s |
| Withstanding voltage | 3 x rated voltage | 60 s |
| Operating temperature range | -55 to 125 degC | |
| Dimensions (0402) | 1.00 +/-0.05 x 0.50 +/-0.05 x 0.50 +/-0.05 mm | L x W x T |
