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
use microbit::{board::Board, display::blocking::Display, hal::Timer};

// embedded-hal crate: For button state
// https://docs.rs/embedded-hal/1.0.0/embedded_hal/
use embedded_hal::digital::InputPin;

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
    let mut current_display_index = 0;
    let display_list = [h_view, s_view, v_view];

    // initialize the display
    let mut display = Display::new(board.display_pins);

    // loop
    loop {
        // move to the next state
        if forward_button.is_low().unwrap() && current_display_index < 2 {
            current_display_index += 1;
        }

        // move to the previous state
        if back_button.is_low().unwrap() && current_display_index > 0 {
            current_display_index -= 1;
        }

        // show the current display based on the index.
        rprintln!("{}", current_display_index);
        display.show(&mut timer, display_list[current_display_index], 100);
    }
}
