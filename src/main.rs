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
    hal::{
        Timer,
        gpio::Level,
        gpiote,
        pac::{pwm0, NVIC, interrupt},
        saadc::{Saadc, SaadcConfig},
    }, // used for controlling LED brightness
};

// embedded-hal crate: For button and LED pin state
// https://docs.rs/embedded-hal/1.0.0/embedded_hal/
use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
};

// hsv crate for converting HSV to RGB
// https://github.com/pdx-cs-rust-embedded/hsv
// https://pdx-cs-rust-embedded.github.io/hsv/hsv/index.html
use hsv::Hsv;

// =======================================================
// Given the RGB value, determine when to turn off the LED
fn calculate_turn_off_steps(rgb_val: f32) -> i16 {
    let turn_off_value = rgb_val * 100.0;
    let turn_off_value_int: i16 = turn_off_value as i16;
    turn_off_value_int
}

// given the turn_off_value as an integer, determine when to set the timer to interrupt in us (microseconds)
fn calculate_timer_interrupt_val_us(turn_off_val: i16) -> i16 {
    turn_off_val * 100 // this value is in us
}

// =======================================================
#[interrupt]
fn GPIOTE() {

}

// =======================================================
#[entry]
fn main() -> ! {
    rtt_init_print!();

    // Define the max potentiometer value.
    // It is about (2^14) - 1
    const MAX_POT_VALUE: f32 = 16383.0;

    // initialize the board and timer
    let board = Board::take().unwrap();
    let mut timer = Timer::new(board.TIMER0);
    //let pwm = board.PWM0;

    // Enable timer interrupts
    timer.enable_interrupt();

    // Initialize the buttons
    let mut back_button = board.buttons.button_a; // right to left: H < S < V
    let mut forward_button = board.buttons.button_b; // left to right: H > S > V

    // initialize the display
    let mut display = Display::new(board.display_pins);

    // initialize the SAADC
    // Docs: https://docs.rs/microbit-v2/0.16.0/microbit/hal/saadc/index.html
    let saadc_config = SaadcConfig::default();
    let mut saadc = Saadc::new(board.ADC, saadc_config);
    let mut saadc_pin = board.edge.e02.into_floating_input(); // For the rotary device, the other pins are for ground and 3.3V

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
    const H_VIEW: [[u8; 5]; 5] = [
        // Hue
        [1u8, 0u8, 0u8, 0u8, 1u8],
        [1u8, 0u8, 0u8, 0u8, 1u8],
        [1u8, 1u8, 1u8, 1u8, 1u8],
        [1u8, 0u8, 0u8, 0u8, 1u8],
        [1u8, 0u8, 0u8, 0u8, 1u8],
    ];
    const S_VIEW: [[u8; 5]; 5] = [
        // Saturation
        [1u8, 1u8, 1u8, 1u8, 1u8],
        [1u8, 0u8, 0u8, 0u8, 0u8],
        [1u8, 1u8, 1u8, 1u8, 1u8],
        [0u8, 0u8, 0u8, 0u8, 1u8],
        [1u8, 1u8, 1u8, 1u8, 1u8],
    ];
    const V_VIEW: [[u8; 5]; 5] = [
        // Value
        [1u8, 0u8, 0u8, 0u8, 1u8],
        [1u8, 0u8, 0u8, 0u8, 1u8],
        [1u8, 0u8, 0u8, 0u8, 1u8],
        [0u8, 1u8, 0u8, 1u8, 0u8],
        [0u8, 0u8, 1u8, 0u8, 0u8],
    ];

    // set up the list of displays.
    let mut current_display_index: usize = 0;
    let display_list = [H_VIEW, S_VIEW, V_VIEW];

    // set up the list of HSV values to convert to RGB
    let mut hsv_values: hsv::Hsv = Hsv {
        h: 1.0,
        s: 1.0,
        v: 1.0,
    };

    // loop
    loop {
        // turn off the led
        for pin in color_pins.iter_mut() {
            pin.set_high().unwrap();
        }

        // Read the potentiometer value and scale it to be between 0 and 1
        let potentiometer_res = saadc.read_channel(&mut saadc_pin).unwrap();
        let mut pot_val: f32 = potentiometer_res.into();
        pot_val /= MAX_POT_VALUE;

        // clamp the potentiometer value between 0 and 1
        pot_val = pot_val.clamp(0.0, 1.0);

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

        // determine which value to update based on index.
        if current_display_index == 0 {
            hsv_values.h = pot_val;
        } else if current_display_index == 1 {
            hsv_values.s = pot_val;
        } else if current_display_index == 2 {
            hsv_values.v = pot_val;
        }

        rprintln!(
            "HSV values: {} {} {}",
            hsv_values.h,
            hsv_values.s,
            hsv_values.v
        );
        // Convert HSV to RGB
        let rgb_values: hsv::Rgb = hsv_values.to_rgb();

        // test code - turn on the LED and show a different color when the mode is changed
        color_pins[current_display_index].set_low().unwrap();

        rprintln!(
            "RGB values: {} {} {}",
            rgb_values.r,
            rgb_values.g,
            rgb_values.b
        );

        // show the current display based on the index.
        display.show(&mut timer, display_list[current_display_index], 100);
    }
}
