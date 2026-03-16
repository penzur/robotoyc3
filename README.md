# ROBOTOY

New firmware for a salvaged, broken RC monster truck that has been completely reworked using a cheap-ass mini brushed ESC and ESP32-C3.

### SETUP
Review the documentation for [esp-hal](https://github.com/esp-rs/esp-hal), [embedded-hal](https://github.com/rust-embedded/embedded-hal), and [THE BOOK](https://doc.rust-lang.org/beta/embedded-book/intro/index.html).

### FLASH
The WiFi operates in AP mode. To flash the firmware, run:

`SSID="YOUR SSID OF CHOICE" PASSWORD="AP PASSWORD" cargo run --release`

After the firmware is uploaded, follow these steps:

1. Connect to `YOUR SSID OF CHOICE` using a static IP address.
2. Open `http://192.168.1.1:3000` in your browser.

> *NOTE:*
> you can use `W` `A` `S` `D` on desktop (throttle control not yet implemented, mobile only for now).
