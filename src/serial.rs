//! Write formatted data to the USART peripheral.

use arduino_hal::{clock::MHz16, hal::usart::Usart0};
use core::{convert::Infallible, str};
use ufmt::uWrite;

pub struct SerialWriter {
	inner: Option<Usart0<MHz16>>,
}

impl uWrite for SerialWriter {
	type Error = Infallible;

	fn write_str(&mut self, s: &str) -> Result<(), Self::Error> {
		match &mut self.inner {
			Some(serial) => {
				let _ = serial.write_str(s);
			}
			None => {
				panic!();
			}
		}

		Ok(())
	}
}

pub static mut SERIAL: SerialWriter = SerialWriter { inner: None };

pub fn set_serial(serial: Usart0<MHz16>) {
	unsafe {
		if SERIAL.inner.is_none() {
			SERIAL.inner = Some(serial);
		}
	}
}

/// Convenience wrapper so that `unsafe { ... }` isn't required whenever something should be
/// logged to serial output.
///
/// This macro requires that `SERIAL` is in scope whenever it is used.
///
/// ```
/// log!("{}, {}", my_value_1, my_value_2);
/// ```
#[allow(unused_macros)]
macro_rules! log {
    ($fmt:expr) => {{
		let _ = unsafe { ufmt::uwriteln!(SERIAL, $fmt) };
	}};
    ($fmt:expr, $($values:expr),*) => {{
		let _ = unsafe { ufmt::uwriteln!(SERIAL, $fmt, $($values),*) };
	}}
}
