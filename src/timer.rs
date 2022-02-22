//! Ref: https://blog.rahix.de/005-avr-hal-millis/

use avr_device::interrupt::Mutex;
use core::{
	cell::Cell,
	sync::atomic::{AtomicBool, Ordering},
};

const PRESCALER: u16 = 64;
const TIMER_COUNTS: u16 = 250;

const MILLIS_INCREMENT: u16 = PRESCALER * TIMER_COUNTS / 16000;

pub struct Timer {
	pub paused: AtomicBool,
	pub millis: Mutex<Cell<u16>>,
	pub seconds: Mutex<Cell<u16>>,
}

impl Timer {
	pub fn init(tc0: arduino_hal::pac::TC0) {
		// Configure the timer for the above interval (in CTC mode)
		// and enable its interrupt.
		tc0.tccr0a.write(|w| w.wgm0().ctc());
		tc0.ocr0a.write(|w| unsafe { w.bits(TIMER_COUNTS as u8) });
		tc0.tccr0b.write(|w| match PRESCALER {
			8 => w.cs0().prescale_8(),
			64 => w.cs0().prescale_64(),
			256 => w.cs0().prescale_256(),
			1024 => w.cs0().prescale_1024(),
			_ => panic!(),
		});
		tc0.timsk0.write(|w| w.ocie0a().set_bit());
	}

	pub fn pause(&self) {
		avr_device::interrupt::free(|_cs| {
			self.paused.store(true, Ordering::SeqCst);
		});
	}

	pub fn resume(&self) {
		avr_device::interrupt::free(|_cs| {
			self.paused.store(false, Ordering::SeqCst);
		});
	}

	pub fn reset(&self) {
		avr_device::interrupt::free(|cs| {
			self.millis.borrow(cs).set(0);
			self.seconds.borrow(cs).set(0);
		});
	}

	pub fn elapsed_s(&self) -> u16 {
		avr_device::interrupt::free(|cs| self.seconds.borrow(cs).get())
	}
}

pub static TIMER: Timer = Timer {
	paused: AtomicBool::new(true),
	millis: Mutex::new(Cell::new(0)),
	seconds: Mutex::new(Cell::new(0)),
};

#[avr_device::interrupt(atmega328p)]
#[allow(non_snake_case)]
fn TIMER0_COMPA() {
	avr_device::interrupt::free(|cs| {
		if !TIMER.paused.load(Ordering::SeqCst) {
			let millis_cell = TIMER.millis.borrow(cs);
			let millis = millis_cell.get();
			if millis >= 1_000 {
				millis_cell.set(0);
				let seconds_cell = TIMER.seconds.borrow(cs);
				let seconds = seconds_cell.get();
				seconds_cell.set(seconds + 1);
			} else {
				millis_cell.set(millis + MILLIS_INCREMENT);
			}
		}
	})
}
