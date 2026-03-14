# HSV: An HSV color space demo with a Microbit v2 and Rust
# Lee Hoang 2026

mb2-hsv uses the Microbit v2 and an RGB LED to show off the [HSV color space](https://en.wikipedia.org/wiki/HSL_and_HSV). 

## Build and Run

Compile and run the project with `cargo embed --release`.  

To run in debug mode, use `cargo embed`. 

## What I did

I created a program that uses a breadboard, an LED, a potentiometer, and a microbit to show off the HSV color space. The user can turn the potentiometer to adjust the color of the LED. 

Pressing the buttons on the microbit allows the user to switch between adjusting hue, saturation, or value depending on the letter shown.

## How it went

I found this project challenging due to three primary factors:

1) Setting up the wires/hardware.

2) Understanding timer interrupts.

3) LED timing.

More information TBD

## Acknowledgements

- Documentation for the `microbit-v2`, `hsv`, and other crates. Links can be found in the source code. 
- Classmates from the Winter 2026 Rust Embedded class for help on setup and expected program behavior.
- [MB2 Discovery Book](https://docs.rust-embedded.org/discovery-mb2/index.html) - provided starting points for Microbit v2 code.
- [pdx-cs-rust-embedded](https://github.com/pdx-cs-rust-embedded) - provided starting points for setting up the project.
- [Rust Cargo book](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html) - used to specify git repo for dependencies.
- [hsv crate documentation](https://pdx-cs-rust-embedded.github.io/hsv/hsv/index.html)

## License

This work is made available under the "Apache 2.0 or MIT
License". See the file `LICENSE.txt` in this distribution for
license terms.
