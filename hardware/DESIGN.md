# Design

This document covers design intent, architecture, and components of the Entrylens sensor. Exact
passive values, pin assignments, and net connectivity are included in the netlist/BOM.

## 1. Intended Use

Entrylens is an indoor people-counting sensor intended for use in museums and exhibition areas.
One unit is installed at each door opening to monitor pedestrian traffic and wirelessly transmit
the resulting people counts for occupancy calculation and analysis.

The device is intended for professional installation by technical personnel responsible for
building or museum technology.

It is an open-hardware reference design intended to support subsequent conformity assessment and
reproduction for own use. The target reproduction cost is approximately USD 40 per unit, assuming
a small production batch of approximately 20 units.


## 2. Usage

This section defines the intended deployment conditions, operating envelope, and functional
limitations of the device. The implementation used to meet these requirements is described in
Sections 3–5.

### 2.1 Deployment

The device is intended to be mounted centrally on the lower rear edge of the head jamb, opposite
the door leaf. It is secured with 15 mm wide double-sided adhesive tape. The reference installation
was tested on a double-leaf doorway measuring 250 cm in height and 130 cm in width.

It is continuously powered from an external low-voltage DC power supply via a two-wire cable with
a maximum length of 10 m. The nominal input voltage is 5 V.

Wireless network credentials are configured using a mobile application and transferred to the
device via NFC. Configuration can be set or retrieved even when the device is not powered.

### 2.2 Operation

The device continuously detects and counts people entering and leaving the area using a
distance-measuring sensor, without capturing images or other visual data.

It is designed for bidirectional pedestrian traffic, including individuals and groups with no more
than two people abreast, at walking speeds of up to 1.5 m/s. The target counting accuracy is 95%
under the specified operating conditions.

The device has LED indicators:
- a two-color (red/green) LED controlled by the MCU, providing status indication visible from
  outside the enclosure
- a red power LED, visible only when the housing lid is removed

### 2.3 Maintenance

No routine physical maintenance is required during normal operation. Firmware updates and device
diagnostics can be performed remotely where wireless connectivity is available.

### 2.4 Service

The internal debug/programming connector provides access to detailed device status information and
is used for troubleshooting, firmware updates, and production programming. It is located inside the
enclosure and is not connected during normal operation.


## 3. Mechanical

The device is designed for indoor use in ordinary environments, such as museums and exhibition spaces.
It is not intended for use in industrial environments or hazardous areas.

Operating temperature range: 0 to 40 C.

### 3.1 Composition

The device is composed of:
- Enclosure, described in EntrylensHousing.FCStd, manufactured from Nylon PA12S
- PCB, described in Section 4
- Light pipe for the red/green LED indicator (FIX-LEMB2-4.8V0-F)
- 868 MHz antenna board (2JF0415P), connected directly to the RAK3172 module via a U.FL/IPEX cable

### 3.2 Arrangement

The device features a compact cylindrical enclosure with a removable lid for access to the internal
electronics. There are no external antennas or other protruding mechanical components, except for
the power cable, which enters the enclosure through a strain relief.

The overall external dimensions are 85 mm x 40 mm.
The PCB measures 82 mm x 29 mm.

The PCB and the antenna board are mounted perpendicular to each other inside the enclosure. They are
retained inside the enclosure by rails that engage the clear keepout areas along the longer edges of
the PCB rather than using mounting holes or screws.

A 6 mm non-plated hole in the PCB provides mechanical clearance for the U.FL/IPEX cable routed from
the RAK3172 module to the antenna board (2JF0415P).


## 4. Hardware

### 4.1 Architecture

The PCB integrates the following functional blocks:
- Power input
  - 2-pin JST PH connector (B2B-PH-K-S(LF)(SN))
  - ESD and transient protection (SMAJ10CA TVS)
  - Reverse polarity protection (AO3401A P-MOSFET)
  - Overcurrent and overvoltage protection (TPS25200 eFuse, overvoltage clamp 5.25-5.55 V)
  - MOSFET gate-source clamp (BZT52C10S Zener diode)
  - TPS25200 EN clamp (BZT52C6V2S Zener diode)
  - 3.3 V buck converter (TLV62569)
  - High-frequency noise filtering (MPZ2012S601ATD25 ferrite bead)
- MCU and LoRaWAN transceiver module (RAK3172, based on STM32WLE5)
  - Voltage sensing via a resistive voltage divider
- Debug/programming interface
  - 7-pin JST SR connector (SM07B-SRSS-TB)
  - ESD protection (GMF05LC-HSF)
- 2-Mbit NOR Flash (W25X20CL)
- Voltage and logic level conversion to 1.8 V
  - Voltage regulator (LDK130 LDO, adjustable-output)
  - Level shifters (PI4ULS5V202 for I2C and PI4ULS3V204 for GPIO)
- ToF ranging sensor (VL53L8CX)
- Dynamic NFC/RFID tag (ST25DV04KC, 512-byte EEPROM)
  - Antenna coil on PCB (ST25DV_Discovery_ANT_C6)
- 2 LEDs: power indicator (red, KT-0603R, hardwired to +3V3) and a two-color MCU-controlled
  indicator (LTST-C295KGKRKT, green + red dies)

No active or passive RF circuitry is present between the RAK3172 module and the antenna. The LoRa
antenna is placed away from the NFC antenna coil to minimize coupling.

The PCB is a 4-layer FR-4 board with a Signal / GND / Power / Signal stack-up.

### 4.2 Power

The device is powered by an external low-voltage DC power supply via a JST PH connector. The power
supply and power cable are not part of the device.

Nominal input voltage: 5 V
Operating input voltage: 3.5 - 5 V
Wrong-polarity / overvoltage withstand: +/- 9 V for at least 1 hour

D1 (SMAJ10CA) absorbs surge energy at the input. D2 (BZT52C10S) is only a local Vgs clamp for Q1's
gate, protecting against the residual voltage after current limiting by R5.

RAK3172 monitors the protected 5 V input rail via an ADC voltage divider.

The input voltage is converted by TLV62569 to 3.318 V (3.199-3.440 V tolerance) and further by
LDK130 to 1.896 V (1.818-1.976 V tolerance). Due to ST25DV04KC operating range the 1V8 rail voltage
range must never be below 1.8 V.

The TPS25200 FAULT output controls the TLV62569 EN input and disables the 3.3 V regulator during an
overcurrent, overtemperature or overvoltage condition.

Maximum peak current budget: 297 mA

| Component       | 3V3 rail     | 1V8 rail |
|-----------------|--------------|----------|
| VL53L8CX        | 60 mA (AVDD) | 90 mA + 10 mA (CORE_1V8 + IOVDD) |
| RAK3172         | 87 mA        |          |
| W25X20CL        | 15 mA        |          |
| ST25DV04KC      |              | 1 mA     |
| KT-0603R        | 12 mA        |          |
| LTST-C295KGKRKT | 11 mA + 11 mA (red + green) | |
| Total           | 196 mA       | 101 mA   |

These values represent the maximum peak current budget used for power-supply sizing. VL53L8CX active
ranging and RAK3172 LoRaWAN transmission can occur simultaneously, so these loads are budgeted as
concurrent.

TPS25200's current limit is set to 440 mA (374-512 mA tolerance), giving margin above the 297 mA
budget even as TLV62569 approaches 100% duty cycle near the minimum input voltage.

The device remains continuously active during normal operation and does not employ deep-sleep modes.

### 4.3 MCU (RAK3172)

BOOT0 is left floating as it is pulled-down internally by RAK3172.

The GPIOs can sink or source up to +/- 20 mA (with a relaxed Vol/Voh).

The MCU controls the VL53L8CX LPn pin, allowing the sensor to be placed in hardware standby
or reinitialized without resetting the rest of the system.

The STM32WLE5 independent watchdog (IWDG) is used to reboot the device if the firmware becomes
unresponsive.

The STM32WLE5 brown-out reset (BOR) remains enabled. If the supply voltage falls below the
BOR threshold, the MCU is held in reset until the supply recovers, after which the device
performs a normal boot.

Short supply interruptions or undervoltage conditions are expected to result in a clean device
restart rather than continued degraded operation.

### 4.4 Debug/programming interface

Debugging and programming are possible via a JST SR connector.

The interface supports programming and debugging using either an ST-LINK or DAPLink
debugger/programmer.

### 4.5 External flash (W25X20CL)

The flash is powered from 3V3.

It stores the VL53L8CX firmware image and the staged MCU application image during FOTA updates.

The /WP and /HOLD pins are tied high and are not MCU-controlled.

### 4.6 ToF sensor (VL53L8CX)

Detection field of view: 45 degrees horizontal/vertical.

SPI_I2C_N is pulled to +1V8 to select SPI mode. The firmware switches the sensor to I2C mode by
driving this pin low.

### 4.7 NFC (ST25DV04KC)

Energy harvesting is disabled in the device configuration; V_EH is intentionally left unconnected.

The antenna is implemented as a PCB trace, ST25DV_Discovery_ANT_C6 reference design (ANT-1-6-ST25DV):
- 15 turns
- 18.5 mm x 22.7 mm
- trace width / spacing: 0.2 mm / 0.2 mm
- copper thickness: 1 oz (35 um)

Equivalent inductance at 13.56 MHz: 4.71 uH


## 5. Software

Firmware is custom, built with STM32CubeIDE (bare-metal; no RUI3), programmed via SWD.

### 5.1 Bootloader

The bootloader checks whether a valid FOTA firmware image is available in external flash. If a
valid image is present, it is installed and the device then boots the sensing application.

An incomplete or invalid firmware image is not installed.

There is no rollback: a device that crashes immediately on boot after an update requires manual
maintenance.

### 5.2 Sensing application

The device operates as a LoRaWAN Class A device in the EU868 band.

Both OTAA and ABP activation are supported; the choice is set via NFC provisioning.

Updated event counters are queued for uplink after detection and transmitted at the next available
LoRaWAN opportunity, subject to duty-cycle restrictions.

The device does not maintain synchronized wall-clock time. Events are therefore associated with the
network-server reception timestamp rather than a precise local detection timestamp.

### 5.3 FOTA update

Firmware chunks received via LoRaWAN downlinks are staged in W25X20CL. Once the image authenticity
and integrity are verified using a digital signature, the bootloader is invoked to install it.


## 6. Compliance

The documented design is intended to support conformity assessment with the applicable
EU requirements. Formal conformity assessment and testing have not yet been completed.

The applicable legislation is:

* Directive 2014/53/EU (Radio Equipment Directive, RED):
  - Article 3(1)(a): health and safety
  - Article 3(1)(b): electromagnetic compatibility
  - Article 3(2): effective and efficient use of radio spectrum

The following harmonised standards are intended to be used to demonstrate conformity
with the applicable requirements:

* ETSI EN 300 220-2: applicable to the 863-870 MHz LoRa radio interface.
* ETSI EN 300 330: applicable to the 13.56 MHz NFC interface.
* ETSI EN 301 489-1 and EN 301 489-3: applicable to electromagnetic compatibility.

EN IEC 62368-1 may be used as a supporting safety standard, subject to confirmation
of its applicability to the final equipment configuration.
