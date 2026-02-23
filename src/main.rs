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
use microbit::board::Board;

// =======================================================

#[entry]
fn main() -> ! {
    rtt_init_print!();
    // initialize the board
    let _board = Board::take().unwrap();
    let mut counter = 0u64;

    // loop
    loop {
        rprintln!("{}", counter);
        counter += 1;
    }
}
