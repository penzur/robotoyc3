## 00. About
New firmware for a salvaged, broken RC monster truck that has been completely reworked using a cheap-ass mini brushed ESC and ESP32-C3.

## 01. Setup
Checkout [The book](https://doc.rust-lang.org/beta/embedded-book/intro/index.html) for setting up dev env and everything.

**Resources:**
- [esp-hal](https://github.com/esp-rs/esp-hal)
- [embedded-hal](https://github.com/rust-embedded/embedded-hal)

## 02. Flash
The WiFi operates in AP mode. To flash the firmware, run: `SSID="YOUR SSID OF CHOICE" PASSWORD="AP PASSWORD" cargo run --release`

**Once the firmware is uploaded, follow these steps:**

1. Connect to `YOUR SSID OF CHOICE` using a static IP address.

3. Open `http://192.168.1.1:3000` in your browser.


