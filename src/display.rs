use arduino_hal::I2c;
use core::{convert::Infallible, fmt::Write, str};

use ssd1306::{mode::TerminalMode, prelude::*, I2CDisplayInterface, Ssd1306};
use ufmt::uWrite;

///
pub struct Display {
	inner: Ssd1306<I2CInterface<I2c>, DisplaySize128x64, TerminalMode>,
}

/// The first 2 rows are yellow (header) rows, the rest are blue
pub const BODY_START_ROW: u8 = 2;

/// Amount of rows available in the body section of the display
pub const BODY_ROW_COUNT: u8 = 6;

/// A single row is 16 characters across.
pub const ROW_LENGTH: u8 = 16;

/// Slice of whitespace to clear a row in the display
pub const CLEAR_ROW: &str = "                ";

impl Display {
	pub fn new(i2c: I2c) -> Self {
		let interface = I2CDisplayInterface::new(i2c);

		let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
			.into_terminal_mode();
		let _ = display.init();

		Self { inner: display }
	}

	pub fn init(&mut self) {
		let _ = self.inner.clear();
	}

	pub fn clear_body(&mut self) {
		for row in 0..BODY_ROW_COUNT {
			let _ = self.inner.set_position(0, BODY_START_ROW + row as u8);
			let _ = ufmt::uwrite!(self, "{}", CLEAR_ROW);
		}
	}

	pub fn set_position(&mut self, column: u8, row: u8) {
		let _ = self.inner.set_position(column, row);
	}
}

impl uWrite for Display {
	type Error = Infallible;

	fn write_str(&mut self, s: &str) -> Result<(), Self::Error> {
		let _ = self.inner.write_str(s);
		Ok(())
	}
}
