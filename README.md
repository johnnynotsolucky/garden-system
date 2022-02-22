# Garden System

![Garden system wiring schematics](/schematic/schematic.png?raw=true)

**Note**: This is still a work in progress.

- [ ] Update activation time to minutes instead of seconds (for quicker dev)
- [ ] Update suspend time to use minutes instead of reusing the activation time (for quicker dev)
- [ ] Solder Nano, buttons and OLED display onto a protoboard so they can easily fit in a box
- [ ] Wire up the pump
- [ ] Connect solenoid valve to the pump with 20mm LDPE pipe
- [ ] Connect connect pump to a water tank with 20mm LDPE pipe
- [ ] Hook up 15mm irrigation pipes to the reducer on the solenoid valve

## Requirements

### Hardware

- Arduino Nano v3;
- Ambient Light sensor (Analog);
- Moisture sensore (Analog);
- 2x Relays;
- 12V PSU;
- 12V Solenoid valve;
- 230V water pump;
- Ssd1306 OLED display;
- 3x push buttons;
- 4x 220Ω resistors;
- Breadboard;
- Jumper wires.

### Software

- ~[ravedude](https://crates.io/crates/ravedude)~
  - Requires https://github.com/Rahix/avr-hal/pull/247 so that ravedude uses the correct baudrate
  for this version of the Arduino Nano.
- avrdude
  - `apt install avrdude` on Ubuntu.

## Setup

### Wiring

Schematics created with [KiCad](https://www.kicad.org/). See [/schematic](/schematic).

### Program

With the Arduino Nano connected via USB:

```bash
cargo run -- <connection port>
```

For example:

```bash
cargo run -- /dev/ttyUSB0
```
