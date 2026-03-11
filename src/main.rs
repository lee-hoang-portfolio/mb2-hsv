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
        gpio::{Floating, Input, Level, Output, Pin, PushPull},
        gpiote::Gpiote,
        pac::{self, NVIC, interrupt},
        saadc::{Saadc, SaadcConfig},
    }, // used for controlling LED brightness
};

// embedded-hal crate: For button and LED pin state
// https://docs.rs/embedded-hal/1.0.0/embedded_hal/
use embedded_hal::{
    //delay::DelayNs,
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
// =======================================================
// struct definitions

// struct for a display
// this will be shared between the main loop and the interrupt handler
struct LedDisplay {
    timer0: Timer<pac::TIMER0>,           // The timer that will interrupt
    led_pins: [Pin<Output<PushPull>>; 3], // The LED pins
    led_cycles: [u32; 3],                 // determine when to turn off the LED.
}

// define what functions are available for an LedDisplay
impl LedDisplay {
    // Initialize the display
    fn new(pins: [Pin<Output<PushPull>>; 3], timer0: Timer<pac::TIMER0>) -> Self {
        Self {
            led_pins: pins,
            led_cycles: [0, 0, 0],
            timer0,
        }
    }

    // Start by turning all of RGB on and setting the timer for
    // the time when you will turn one or more of them off.
    // Keep doing this until they are all off, and then set the timer
    // for the start of the next frame.
    fn display(&mut self) {
        rprintln!("THIS IS A TEST: {}", self.led_cycles[0]);
        self.timer0.start(1_000_000);
    }

    // convert rgb values to cycles
    fn calculate_cycle_values(&mut self, rgb: &hsv::Rgb) {
        self.led_cycles[0] = (rgb.r * 100.0) as u32;
        self.led_cycles[0] *= 100;

        self.led_cycles[1] = (rgb.g * 100.0) as u32;
        self.led_cycles[1] *= 100;

        self.led_cycles[2] = (rgb.b * 100.0) as u32;
        self.led_cycles[2] *= 100;
    }
}
// =======================================================
// button struct
// contains both the A and B buttons on the Microbit
struct Buttons {
    buttons: [Pin<Input<Floating>>; 2], // back button is the first element, forward button is the second element
    gpiote0: Gpiote,
    current_display_index: usize,    // the current display index
    display_list: [[[u8; 5]; 5]; 3], // a list of 3 displays showing H, S, or V
}

impl Buttons {
    fn new(mb2_buttons: [Pin<Input<Floating>>; 2], mb2_gpiote: Gpiote) -> Self {
        // initialize the struct
        Self {
            buttons: mb2_buttons,
            gpiote0: mb2_gpiote,
            current_display_index: 0,
            display_list: [H_VIEW, S_VIEW, V_VIEW],
        }
    }

    fn setup_button_interrupts(&mut self) {
        self.gpiote0
            .channel0()
            .input_pin(&self.buttons[0])
            .hi_to_lo()
            .enable_interrupt();
        self.gpiote0
            .channel1()
            .input_pin(&self.buttons[1])
            .hi_to_lo()
            .enable_interrupt();

        self.gpiote0.channel0().reset_events();
        self.gpiote0.channel1().reset_events();
    }

    // change the MB2 display
    fn change_mb2_display(&mut self) {
        if self.buttons[1].is_low().unwrap() && self.current_display_index < 2 {
            self.current_display_index += 1;
        } else if self.buttons[1].is_low().unwrap() && self.current_display_index == 2 {
            self.current_display_index = 0;
        }

        // move to the previous state
        // if doing so causes the index to become negative, wrap back to 2 (V)
        if self.buttons[0].is_low().unwrap() && self.current_display_index > 0 {
            self.current_display_index -= 1;
        } else if self.buttons[0].is_low().unwrap() && self.current_display_index == 0 {
            self.current_display_index = 2;
        }
    }

    // Testing button interrupt
    fn test_print(&self) {
        rprintln!("Testing button interrupt");
    }
}

// =======================================================
// Interrupt variables
// Share the display between the interrupt handler and the main loop
static DISPLAY_LOCK: LockMut<LedDisplay> = LockMut::new();
static BUTTON_LOCK: LockMut<Buttons> = LockMut::new(); // TBD

// =======================================================

// Interrupt handler
#[interrupt]
fn GPIOTE() {
    rprintln!("Entering interrupt");
    BUTTON_LOCK.with_lock(|mb2_buttons| {
        rprintln!("Testing");
        let back_button_pressed = mb2_buttons.gpiote0.channel0().is_event_triggered();
        let forward_button_pressed = mb2_buttons.gpiote0.channel1().is_event_triggered();
        if back_button_pressed || forward_button_pressed {
            mb2_buttons.test_print();
        }

        mb2_buttons.gpiote0.channel0().reset_events();
        mb2_buttons.gpiote0.channel1().reset_events();
    });
}

#[interrupt]
fn TIMER0() {
    // Always start a new timer with time > 0.
    DISPLAY_LOCK.with_lock(|leddisplay| {
        leddisplay.display();
    })
}

#[entry]
fn main() -> ! {
    rtt_init_print!();

    // Define the max potentiometer value.
    // It is about (2^14) - 1
    const MAX_POT_VALUE: f32 = 16383.0;

    // initialize the board and timer
    let board = Board::take().unwrap();
    let timer = Timer::new(board.TIMER0);

    // initialize the LED pins
    // inspired by https://github.com/pdx-cs-rust-embedded/hello-rgb/tree/pwm
    // https://docs.rs/microbit-v2/0.16.0/microbit/hal/gpio/p0/struct.P0_10.html#method.into_push_pull_output
    // https://docs.rs/microbit-v2/0.16.0/microbit/hal/gpio/struct.Pin.html
    let pin_r = board.edge.e08.into_push_pull_output(Level::Low); // Red goes into P8
    let pin_g = board.edge.e09.into_push_pull_output(Level::Low); // Green goes into P9
    let pin_b = board.edge.e16.into_push_pull_output(Level::Low); // Blue goes into P16

    let color_pins = [pin_r.degrade(), pin_g.degrade(), pin_b.degrade()]; // set up the list of pins and use them as generic structs

    // Set up the struct
    let mut leddisplay = LedDisplay::new(color_pins, timer);
    // Enable timer interrupts
    leddisplay.timer0.enable_interrupt();
    // Get the timer started
    leddisplay.display();
    // Move the display into the global
    DISPLAY_LOCK.init(leddisplay);

    // Initialize the buttons
    let back_button = board.buttons.button_a; // right to left: H < S < V
    let forward_button = board.buttons.button_b; // left to right: H > S > V
    let gpiote = Gpiote::new(board.GPIOTE);
    let mut buttons = Buttons::new([back_button.degrade(), forward_button.degrade()], gpiote);
    buttons.setup_button_interrupts();
    BUTTON_LOCK.init(buttons);

    // initialize the display
    let mut display = Display::new(board.display_pins);

    // initialize the SAADC
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

    // Enable interrupts for the NVIC
    // Requires unsafe block to run
    unsafe {
        NVIC::unmask(pac::Interrupt::GPIOTE);
        NVIC::unmask(pac::Interrupt::TIMER0);
    }

    // clear interrupt state
    NVIC::unpend(pac::Interrupt::GPIOTE);
    NVIC::unpend(pac::Interrupt::TIMER0);

    // Main loop
    loop {
        //asm::wfi();
        // ******* STEP 1: Check for button presses *******

        // move to the next state
        // if doing so causes the index to be 3 or more, wrap back to index 0 (H)
        BUTTON_LOCK.with_lock(|buttons| {
            buttons.change_mb2_display();
        });

        // ******* STEP 2: Read potentiometer value *******

        // Read the potentiometer value and scale it to be between 0 and 1
        let potentiometer_res = saadc.read_channel(&mut saadc_pin).unwrap();
        let mut pot_val: f32 = potentiometer_res.into();
        pot_val /= MAX_POT_VALUE;

        // clamp the potentiometer value between 0 and 1
        pot_val = pot_val.clamp(0.0, 1.0);

        // determine which value to update based on index.
        BUTTON_LOCK.with_lock(|buttons| {
            if buttons.current_display_index == 0 {
                hsv_values.h = pot_val;
            } else if buttons.current_display_index == 1 {
                hsv_values.s = pot_val;
            } else if buttons.current_display_index == 2 {
                hsv_values.v = pot_val;
            }
        });

        rprintln!(
            "HSV values: {} {} {}",
            hsv_values.h,
            hsv_values.s,
            hsv_values.v
        );

        // ******* STEP 3: Convert HSV values to RGB *******

        // Convert HSV to RGB
        let rgb_values: hsv::Rgb = hsv_values.to_rgb();

        // XXX Deal with LED display in `LedDisplay::display()` method.
        // turn off the led
        DISPLAY_LOCK.with_lock(|leddisplay| {
            for pin in leddisplay.led_pins.iter_mut() {
                pin.set_high().unwrap();
            }
            // test code - turn on the LED and show a different color when the mode is changed
            BUTTON_LOCK.with_lock(|buttons| {
                leddisplay.led_pins[buttons.current_display_index]
                    .set_low()
                    .unwrap()
            });

            // calculate the new cycle values
            leddisplay.calculate_cycle_values(&rgb_values);
            rprintln!(
                "Cycle values: {} {} {}",
                leddisplay.led_cycles[0],
                leddisplay.led_cycles[1],
                leddisplay.led_cycles[2],
            );
        });

        // ******* STEP 4: Block in the display *******

        // XXX Don't block here, move this code to timer0 handler.
        // Show the current LED settings and set next timer.
        DISPLAY_LOCK.with_lock(|leddisplay| {
            BUTTON_LOCK.with_lock(|buttons| {
                display.show(
                    &mut leddisplay.timer0,
                    buttons.display_list[buttons.current_display_index],
                    100,
                );
            });
        })

        // Return to the top of the loop
    }
}
