# Robotoy C3

Firmware for a salvaged RC monster truck rebuilt with an ESP32-C3 and a mini brushed ESC.

## Hardware

- **MCU:** ESP32-C3 (RISC-V)
- **Motor Driver:** Dual brushed ESC (PWM control)
- **Connectivity:** WiFi Access Point
- **Control Interface:** Web-based (mobile-friendly)

## Prerequisites

This project is written in Rust for embedded systems. If you're new to embedded Rust, read [**The Embedded Rust Book**](https://doc.rust-lang.org/embedded-book/intro/index.html) first — it covers toolchain setup, cross-compilation, and flashing workflow.

### Quick Setup

1. Install [Rust](https://rustup.rs/)
2. Set up the embedded toolchain (see the book above)
3. Install `espflash`:
   ```bash
   cargo install espflash
   ```

### Additional Resources

- [esp-hal](https://github.com/esp-rs/esp-hal) - ESP32 HAL
- [embedded-hal](https://github.com/rust-embedded/embedded-hal) - Hardware abstraction traits

## Building and Flashing

The firmware creates a WiFi access point on boot. Set your AP credentials as environment variables when flashing:

```bash
SSID="MyRobot" PASSWORD="secret123" cargo run --release
```

## Usage

1. **Connect to WiFi:** Join the `MyRobot` network from your phone/computer
2. **Configure static IP:** Set your device IP to `192.168.1.2` (or any in the `192.168.1.x` range)
3. **Open controller:** Navigate to `http://192.168.1.1` in your browser
4. **Drive:** Use the on-screen buttons or WASD keys to control the truck

## Control Interface

The web interface provides:
- **Direction:** Forward, Back, Left, Right
- **Speed:** Adjustable via slider (10-100%)
- **Connection status:** Visual indicator showing WebSocket state

Controls work with both touch (mobile) and keyboard (desktop).

## Project Structure

```
src/
├── bin/main.rs    # Entry point, motor control loop
├── lib.rs         # Module exports
├── wifi.rs        # WiFi AP and network stack
├── ws.rs          # WebSocket server, control handling
└── index.html     # Embedded web controller UI
```

## License

MIT
