#![no_main]
#![no_std]

// =======================================================

// Use statements
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

// cortex_m_rt crate
// define the entry point
// docs: https://docs.rs/cortex-m-rt/0.7.5/cortex_m_rt/
use cortex_m_rt::entry;

// Microbit crate
// docs: https://docs.rs/microbit-v2/0.16.0/microbit/
use microbit::{
    board::Board,
    display::blocking::Display,
    hal::{Timer, gpio::Level}, // used for controlling LED brightness
};

// embedded-hal crate: For button and LED pin state
// https://docs.rs/embedded-hal/1.0.0/embedded_hal/
use embedded_hal::digital::{InputPin, OutputPin};

// hsv crate for converting HSV to RGB
// https://github.com/pdx-cs-rust-embedded/hsv
// https://pdx-cs-rust-embedded.github.io/hsv/hsv/index.html
// use hsv::Hsv;

// =======================================================

#[entry]
fn main() -> ! {
    rtt_init_print!();

    // initialize the board and timer
    let board = Board::take().unwrap();
    let mut timer = Timer::new(board.TIMER0);

    // Initialize the buttons
    let mut back_button = board.buttons.button_a; // right to left: H < S < V
    let mut forward_button = board.buttons.button_b; // left to right: H > S > V

    // initialize the display
    let mut display = Display::new(board.display_pins);

    // initialize the LED pins
    // inspired by https://github.com/pdx-cs-rust-embedded/hello-rgb/tree/pwm
    // https://docs.rs/microbit-v2/0.16.0/microbit/hal/gpio/p0/struct.P0_10.html#method.into_push_pull_output
    // https://docs.rs/microbit-v2/0.16.0/microbit/hal/gpio/struct.Pin.html
    let pin_r = board.edge.e08.into_push_pull_output(Level::Low); // Red goes into P8
    let pin_g = board.edge.e09.into_push_pull_output(Level::Low); // Green goes into P9
    let pin_b = board.edge.e16.into_push_pull_output(Level::Low); // Blue goes into P16

    let mut color_pins = [pin_r.degrade(), pin_g.degrade(), pin_b.degrade()]; // set up the list of pins and use them as generic structs

    // define the three displays for H, S, and V
    // default starting display is H (Hue)
    let h_view = [
        // Hue
        [1u8, 0u8, 0u8, 0u8, 1u8],
        [1u8, 0u8, 0u8, 0u8, 1u8],
        [1u8, 1u8, 1u8, 1u8, 1u8],
        [1u8, 0u8, 0u8, 0u8, 1u8],
        [1u8, 0u8, 0u8, 0u8, 1u8],
    ];
    let s_view = [
        // Saturation
        [1u8, 1u8, 1u8, 1u8, 1u8],
        [1u8, 0u8, 0u8, 0u8, 0u8],
        [1u8, 1u8, 1u8, 1u8, 1u8],
        [0u8, 0u8, 0u8, 0u8, 1u8],
        [1u8, 1u8, 1u8, 1u8, 1u8],
    ];
    let v_view = [
        // Value
        [1u8, 0u8, 0u8, 0u8, 1u8],
        [1u8, 0u8, 0u8, 0u8, 1u8],
        [1u8, 0u8, 0u8, 0u8, 1u8],
        [0u8, 1u8, 0u8, 1u8, 0u8],
        [0u8, 0u8, 1u8, 0u8, 0u8],
    ];

    // set up the list of displays.
    let mut current_display_index: usize = 0;
    let display_list = [h_view, s_view, v_view];

    // loop
    loop {
        // move to the next state
        // if doing so causes the index to be 3 or more, wrap back to index 0 (H)
        if forward_button.is_low().unwrap() && current_display_index < 2 {
            current_display_index += 1;
        } else if forward_button.is_low().unwrap() && current_display_index == 2 {
            current_display_index = 0;
        }

        // move to the previous state
        // if doing so causes the index to become negative, wrap back to 2 (V)
        if back_button.is_low().unwrap() && current_display_index > 0 {
            current_display_index -= 1;
        } else if back_button.is_low().unwrap() && current_display_index == 0 {
            current_display_index = 2;
        }

        // *** TBD ***

        // turn off the led
        for pin in color_pins.iter_mut() {
            pin.set_high().unwrap();
        }

        // test code - turn on the LED and show a red color
        color_pins[0].set_low().unwrap(); // R
        color_pins[1].set_high().unwrap(); // G
        color_pins[2].set_high().unwrap(); // B

        // *** End of TBD section ***

        // show the current display based on the index.
        rprintln!("{}", current_display_index);
        display.show(&mut timer, display_list[current_display_index], 100);
    }
}
