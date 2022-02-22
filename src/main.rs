#![feature(llvm_asm)]
#![feature(abi_avr_interrupt)]
#![no_std]
#![no_main]

#[macro_use]
mod serial;

mod config;
mod control_pad;
mod display;
mod menu;
mod system;

mod timer;

use arduino_hal::{Peripherals, Pins};
use control_pad::ControlPad;
use core::panic::PanicInfo;
use display::Display;
use serial::set_serial;
use system::{System, SystemPeripherals};
use timer::Timer;

#[arduino_hal::entry]
fn main() -> ! {
	let dp: Peripherals = arduino_hal::Peripherals::take().unwrap();
	let pins: Pins = arduino_hal::pins!(dp);

	// Initialize the serial interface for writing output when needed.
	set_serial(arduino_hal::default_serial!(dp, pins, 57600));

	// Initialize the timer.
	Timer::init(dp.TC0);

	// Turn on interrupts for this device.
	unsafe { avr_device::interrupt::enable() };

	// Get all the peripherals attached to the device.
	let mut adc = arduino_hal::Adc::new(dp.ADC, Default::default());
	let light_sensor = pins.a0.into_analog_input(&mut adc);
	let moisture_sensor = pins.a1.into_analog_input(&mut adc);
	let buttons = pins.a2.into_analog_input(&mut adc);
	let valve = pins.d3.into_output();

	// The OLED display is using the I2C interface, not SPI.
	let i2c = arduino_hal::I2c::new(
		dp.TWI,
		pins.a4.into_pull_up_input(),
		pins.a5.into_pull_up_input(),
		100_000,
	);

	let display = Display::new(i2c);
	let control_pad = ControlPad::new(buttons);

	let peripherals = SystemPeripherals::new(valve, light_sensor, moisture_sensor);
	let mut control = System::new(adc, peripherals, display, control_pad);
	control.init();

	loop {
		// Run through control logic.
		control.tick();
	}
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
	loop {}
}
