# Pin requirements: VL53L8CX

- References:
  - Datasheet: `vl53l8cx.pdf`, pin table in the pinout description section
  - ST's own SATEL-VL53L8 breakout board, used as a reference: `SATEL-VL53L8.pdf`,
    `satel-vl53l8-schematic.pdf`
  - PCB design and handling guidelines: `pcb-manufacturing-guidelines-vl53l8.pdf`
  - I2C bus pull-up resistor sizing (TI app note, shared bus with ST25DV04KC): `slva689.pdf`
- Package / orderable variant this table applies to: LGA-16 (VL53L8CXV0GC/1)

## Per-pin requirements

| Pin | Function | Requirement |
|---|---|---|
| A1 | GPIO1 (general-purpose I/O) | Digital input/output, defaults to open-drain output (tristate). 47 kOhm pullup resistor to IOVDD required. Used as INT output. |
| A2 | LPn | Digital input. Drive to logic 0 to disable I2C communication, logic 1 to enable it; typically used to change the I2C address in multi-device systems. If not used, or if interfacing via SPI, connect to IOVDD with a 47 kOhm pullup resistor. |
| A3 | IOVDD | Power, 1.2 V or 1.8 V I/O supply. |
| A4 | SDA/MOSI | Digital input/output. I2C: data (bidirectional), 2.2 kOhm pullup resistor required to IOVDD. SPI: main output secondary input. |
| A5 | SCL/MCLK | Digital input. I2C: clock (input), 2.2 kOhm pullup resistor required to IOVDD. SPI: main clock. |
| A6 | RSVD1 | Reserved. Connect to ground. |
| A7 | RSVD2 | Reserved. Connect to ground. |
| B1 | GPIO2 (general-purpose I/O) | Digital input/output, defaults to open-drain output (tristate). 47 kOhm pullup resistor required to IOVDD. Used as SYNC input. |
| B4 | THERMALPAD | Ground. Connect to a ground plane to allow good thermal conduction (see AN5897). |
| B7 | CORE_1V8 | Power, 1.8 V analog core supply. |
| C1 | SPI_I2C_N | Digital input. I2C: low value selects I2C mode - connect to GND with 47 kOhm pulldown resistor. Also used as I2C interface reset pin, active high (toggle 0->1->0 to reset the I2C target only, not the sensor itself). SPI: connect to IOVDD with 47 kOhm pullup resistor. |
| C2 | NCS | Digital input. I2C: not used - connect to GND with 47 kOhm pulldown resistor. SPI: active low chip select, 47 kOhm pullup resistor required to IOVDD. |
| C3 | GND | Ground. |
| C4 | AVDD | Power, 3.3 V analog and VCSEL supply. |
| C5 | MISO | Digital output. SPI: main input secondary output, push-pull driven to IOVDD level. I2C: do not connect. |
| C6 | RSVD3 | Reserved. Connect to ground. |
| C7 | GND | Ground. |

Other notes from the datasheet: all digital signals must be driven to the IOVDD level.

## Electrical limits and consumption

| Parameter | Value | Conditions |
|---|---|---|
| AVDD | -0.5 to 3.47 V | Absolute maximum |
| AVDD (recommended operating range) | 3.13 / 3.3 / 3.47 V (min/typ/max) | - |
| AVDD current, active ranging | 43 mA typ / 50 mA max | Peak current is the average value + 10 mA |
| CORE_1V8 | -0.5 to 1.98 V | Absolute maximum |
| CORE_1V8 (recommended operating range) | 1.62 / 1.8 / 1.98 V (min/typ/max) | - |
| CORE_1V8 current, active ranging | 50 mA typ / 80 mA max | Peak current is the average value + 10 mA |
| IOVDD | -0.5 to 1.98 V | Absolute maximum |
| IOVDD (recommended operating range) | 1.62 / 1.8 / 1.98 V (min/typ/max) | 1.8 V configuration (this board's config; a 1.2 V configuration, 1.08/1.2/1.32 V, also exists but isn't used here) |
| IOVDD current, active ranging | 0.003 mA typ / 0.006 mA max | Peak current is the average value + 10 mA |
| Ambient operating temperature | -30 to 85 degC | Recommended operating condition |

Datasheet note: active ranging current is not affected by 4x4 vs. 8x8 zone configuration.
