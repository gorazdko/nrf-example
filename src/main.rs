#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_nrf::gpio::{AnyPin, Level, Output, OutputDrive};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

/// Which LED to toggle next.
#[derive(Clone, Copy, Format)]
enum LedCommand {
    Toggle(u8),
    AllOff,
}

/// Raw tick sent from the counter to the processor.
type TickChannel = Channel<CriticalSectionRawMutex, u32, 4>;

/// Processed command sent from the processor to the LED controller.
type CmdChannel = Channel<CriticalSectionRawMutex, LedCommand, 4>;

static TICK_CH: TickChannel = Channel::new();
static CMD_CH: CmdChannel = Channel::new();

/// Task 1 — Counter
///
/// Produces an incrementing tick every 500 ms and sends it to TICK_CH.
#[embassy_executor::task]
async fn counter() {
    let mut tick: u32 = 0;
    loop {
        Timer::after_millis(500).await;
        info!("counter: tick {}", tick);
        TICK_CH.send(tick).await;
        tick = tick.wrapping_add(1);
    }
}

/// Task 2 — Processor
///
/// Reads ticks from TICK_CH, decides which LED to light, and forwards
/// a command to CMD_CH. Every 8th tick it sends an AllOff instead.
#[embassy_executor::task]
async fn processor() {
    loop {
        let tick = TICK_CH.receive().await;

        let cmd = if tick % 8 == 7 {
            LedCommand::AllOff
        } else {
            // Cycle through LEDs 0‥3
            LedCommand::Toggle((tick % 4) as u8)
        };

        info!("processor: tick {} -> {:?}", tick, cmd);
        CMD_CH.send(cmd).await;
    }
}

/// Task 3 — LED controller
///
/// Owns the four on-board LEDs and reacts to commands from CMD_CH.
#[embassy_executor::task]
async fn led_controller(pins: [AnyPin; 4]) {
    // nRF52840-DK LEDs are active-low.
    let mut leds = pins.map(|p| Output::new(p, Level::High, OutputDrive::Standard));

    loop {
        let cmd = CMD_CH.receive().await;

        match cmd {
            LedCommand::Toggle(n) => {
                let led = &mut leds[n as usize];
                led.toggle();
                info!("led_controller: toggled LED {}", n);
            }
            LedCommand::AllOff => {
                for led in leds.iter_mut() {
                    led.set_high(); // active-low: high = off
                }
                info!("led_controller: all LEDs off");
            }
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
    info!("Hello, nRF52840-DK!");

    // DK LED pins: P0.13, P0.14, P0.15, P0.16
    let led_pins: [AnyPin; 4] = [
        p.P0_13.into(),
        p.P0_14.into(),
        p.P0_15.into(),
        p.P0_16.into(),
    ];

    spawner.must_spawn(counter());
    spawner.must_spawn(processor());
    spawner.must_spawn(led_controller(led_pins));
}
