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
        gpio::{Level, Output, Pin, PushPull},
        pac::{self, NVIC, interrupt},
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

// critical section
// used for sharing variables between the interrupt handler and the main code
use critical_section_lock_mut::LockMut;
// =======================================================
// constants for what to show on the MB2 display
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

const LED_DISPLAY_LIST: [[[u8; 5]; 5]; 3] = [H_VIEW, S_VIEW, V_VIEW];

// Define the max potentiometer value.
// It is about (2^14) - 1
const MAX_POT_VALUE: f32 = 16383.0;
// =======================================================
// struct definitions

// struct for a display
// this will be shared between the main loop and the interrupt handler
struct LedDisplay {
    cycles: u32,
    timer0: Timer<pac::TIMER0>,           // The timer that will interrupt
    led_pins: [Pin<Output<PushPull>>; 3], // The LED pins
    led_cycles: [u32; 3],                 // determine when to turn off the LED.
    next_cycles: Option<[u32; 3]>, // do we have another cycle set? If so, overwrite the current set of cycles
}

// define what functions are available for an LedDisplay
impl LedDisplay {
    // Initialize the display
    fn new(
        pins: [Pin<Output<PushPull>>; 3],
        timer0: Timer<pac::TIMER0>,

    ) -> Self {
        Self {
            cycles: 0,
            led_pins: pins,
            led_cycles: [0, 0, 0],
            timer0,
            next_cycles: None,

        }
    }

    // Start by turning all of RGB on and setting the timer for
    // the time when you will turn one or more of them off.
    // Keep doing this until they are all off, and then set the timer
    // for the start of the next frame.
    //
    // called inside the interrupt handler
    // https://docs.rust-embedded.org/discovery-mb2/15-interrupts/my-solution.html
    fn display(&mut self) {
        // turn all of RGB on
        self.led_pins[0].set_low();
        self.led_pins[1].set_low();
        self.led_pins[2].set_low();

        // determine difference in cycle values
        let next_cycles = match self.next_cycles {
            Some(cycles) => cycles,
            None => [0, 0, 0],
        };

        rprintln!("{:?}", self.led_cycles);

        // set the LED to a specific color
        for i in 0..3 {
            let time_val = self.led_cycles[i] * 100;
            let intr_time = time_val * 100;
            self.timer0.start(intr_time);
            self.led_pins[i].set_high();
            self.timer0.reset_event();
        }

        rprintln!("Cycle count: {}", self.cycles);
        self.cycles += 1;

        // TODO - make the interrupt work on an RGB change
        self.timer0.reset_event();
        self.timer0.start(1_000_000);

        if self.cycles > 10 {
            self.cycles = 0;
        }

    }

    // convert rgb values to cycles
    fn calculate_cycle_values(&mut self, hsv: &hsv::Hsv) {
        let rgb = hsv.to_rgb();
        self.led_cycles[0] = (rgb.r * 100.0) as u32;
        self.led_cycles[1] = (rgb.g * 100.0) as u32;
        self.led_cycles[2] = (rgb.b * 100.0) as u32;
    }

}
// =======================================================

// =======================================================
// Interrupt variables
// Share the display between the interrupt handler and the main loop
static DISPLAY_LOCK: LockMut<LedDisplay> = LockMut::new();

// =======================================================

// Timer interrupt handler
#[interrupt]
fn TIMER0() {
    // Always start a new timer with time > 0.
    DISPLAY_LOCK.with_lock(|leddisplay| {
        // start the timer and do things with leds
        leddisplay.display();
        // reset the event
        leddisplay.timer0.reset_event();
    }) // end of DISPLAY_LOCK
}

#[entry]
fn main() -> ! {
    rtt_init_print!();

    // initialize the board and timer
    let board = Board::take().unwrap();
    let timer = Timer::new(board.TIMER0);
    let mut timer1 = Timer::new(board.TIMER1);

    let mut current_display_index: usize = 0;

    // initialize the LED pins
    // inspired by https://github.com/pdx-cs-rust-embedded/hello-rgb/tree/pwm
    // https://docs.rs/microbit-v2/0.16.0/microbit/hal/gpio/p0/struct.P0_10.html#method.into_push_pull_output
    // https://docs.rs/microbit-v2/0.16.0/microbit/hal/gpio/struct.Pin.html
    let pin_r = board.edge.e08.into_push_pull_output(Level::Low); // Red goes into P8
    let pin_g = board.edge.e09.into_push_pull_output(Level::Low); // Green goes into P9
    let pin_b = board.edge.e16.into_push_pull_output(Level::Low); // Blue goes into P16

    let color_pins = [pin_r.degrade(), pin_g.degrade(), pin_b.degrade()]; // set up the list of pins and use them as generic structs

    // Set up the struct
    // Initialize the buttons and GPIOTE
    let mut back_button = board.buttons.button_a; // right to left: H < S < V
    let mut forward_button = board.buttons.button_b; // left to right: H > S > V
    //let gpiote = Gpiote::new(board.GPIOTE);
    // initialize the display
    let mut display = Display::new(board.display_pins);
    let mut leddisplay = LedDisplay::new(
        color_pins,
        timer,
    );

    // Enable timer interrupts
    leddisplay.timer0.enable_interrupt();
    // Get the timer started
    leddisplay.display();
    // Move the display into the global
    DISPLAY_LOCK.init(leddisplay);

    // initialize the SAADC for the potentiometer
    // Docs: https://docs.rs/microbit-v2/0.16.0/microbit/hal/saadc/index.html
    let saadc_config = SaadcConfig::default();
    let mut saadc = Saadc::new(board.ADC, saadc_config);
    let mut saadc_pin = board.edge.e02.into_floating_input(); // For the rotary device, the other pins are for ground and 3.3V

    // set up the list of HSV values to convert to RGB
    let mut hsv_values: hsv::Hsv = Hsv {
        h: 1.0,
        s: 1.0,
        v: 1.0,
    };

    // set up the previous potentiometer value
    let mut prev_pot_val = 16383.0;

    // Enable interrupts for the NVIC
    // Requires unsafe block to run
    unsafe {
        NVIC::unmask(pac::Interrupt::GPIOTE); // button interrupt handler
        NVIC::unmask(pac::Interrupt::TIMER0); // timer interrupt handler
    }

    // clear interrupt state
    NVIC::unpend(pac::Interrupt::GPIOTE);
    NVIC::unpend(pac::Interrupt::TIMER0);

    // **********************************************************************************

    // Main loop
    loop {
        // ******* STEP 1: Check for button presses *******

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

        // ******* STEP 2: Read potentiometer (pot) value *******

        // Read the potentiometer value and scale it to be between 0 and 1
        let potentiometer_res = saadc.read_channel(&mut saadc_pin).unwrap();
        let mut pot_val: f32 = potentiometer_res.into(); // convert to floating point

        // clamp the potentiometer value to be between 0 and 2^14 - 1
        pot_val = pot_val.clamp(0.0, MAX_POT_VALUE);
        pot_val /= MAX_POT_VALUE; // scale the pot value to be between 0 and 1

        //rprintln!("Knob val: {}", pot_val);

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

        // ******* STEP 3: Convert HSV values to RGB *******
        // XXX Deal with LED display in `LedDisplay::display()` method.
        // turn off the led
        DISPLAY_LOCK.with_lock(|leddisplay| {
            // calculate the new cycle values
            leddisplay.calculate_cycle_values(&hsv_values);
        });
        
        // ******* STEP 4: Block in the display *******
        // show the display
        display.show(
            &mut timer1,
            LED_DISPLAY_LIST[current_display_index],
            100,
        );
        
        // TBD - show current LED settings
        // Return to the top of the loop
    }

    // **********************************************************************************
}
