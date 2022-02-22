//! Logic for coordinating peripheral inputs and outputs

use arduino_hal::{
	clock::MHz16,
	hal::{
		port::{PC0, PC1, PD3},
		Adc,
	},
	port::{
		mode::{Analog, Output},
		Pin,
	},
};
use core::sync::atomic::Ordering;

use crate::{
	config::{SystemConfig, UpdateSystemValue},
	control_pad::ControlPad,
	display::Display,
	menu::Menu,
	timer::TIMER,
};

/// Holds peripherals for reading sensor values and controlling hardware
pub struct SystemPeripherals {
	/// Solenoid valve relay
	valve: Pin<Output, PD3>,
	/// Light sensor
	light_sensor: Pin<Analog, PC0>,
	/// Moisture sensor
	moisture_sensor: Pin<Analog, PC1>,
}

impl SystemPeripherals {
	/// Create a new [`SystemPeripherals`] from [Pin]'s
	pub fn new(
		valve: Pin<Output, PD3>,
		light_sensor: Pin<Analog, PC0>,
		moisture_sensor: Pin<Analog, PC1>,
	) -> Self {
		Self {
			valve,
			light_sensor,
			moisture_sensor,
		}
	}

	/// Toggles valve activation if necessary
	pub fn update(&mut self, system_config: &SystemConfig) {
		if self.valve.is_set_high() && !system_config.activation_state.is_activated() {
			// If the valve is on but the system is not activated, turn the valve off.
			self.valve.set_low();
		} else if self.valve.is_set_low() && system_config.activation_state.is_activated() {
			// If the valve is off, but the system is activated, turn it on.
			self.valve.set_high();
		}
	}

	/// Whether the valve should be turned on
	pub fn should_activate(&self, system_config: &SystemConfig, adc: &mut Adc<MHz16>) -> bool {
		let light = self.light_sensor.analog_read(adc);
		let moisture = self.moisture_sensor.analog_read(adc);

		moisture < system_config.min_moisture && light < system_config.min_light
	}
}

/// Central type which connects the components of the system
pub struct System {
	/// Analog to digital converter used for reading analog input values
	adc: Adc<MHz16>,
	/// Relevant peripherals
	peripherals: SystemPeripherals,
	/// Menu
	menu: Menu,
	/// Display controller
	display: Display,
	/// Buttons
	control_pad: ControlPad,
	/// System configuration
	system_config: SystemConfig,
}

impl System {
	pub fn new(
		adc: Adc<MHz16>,
		peripherals: SystemPeripherals,
		display: Display,
		control_pad: ControlPad,
	) -> Self {
		let system_config = SystemConfig::new();
		let menu = Menu::new(&system_config);

		Self {
			adc,
			peripherals,
			display,
			control_pad,
			menu,
			system_config,
		}
	}

	/// Setup the display and render system header and menu
	pub fn init(&mut self) {
		self.display.init();
		self.render_header();
		self.menu.render(&mut self.display);
	}

	/// Update the state of the system
	pub fn tick(&mut self) {
		// Check for button presses
		self.control_pad.update(&mut self.adc);

		// If a button was pressed, tell the menu about it.
		if let Some(button_state) = &self.control_pad.state {
			self.menu
				.on_press(button_state, &mut self.display, &mut self.system_config)
		}

		let timer_paused = TIMER.paused.load(Ordering::SeqCst);

		// If the system is either _suspending_ or activated, but the timer is paused, then reset
		// the timer and resume timing.
		let should_reset_timer = self.system_config.activation_state.is_suspending()
			|| (self.system_config.activation_state.is_activated() && timer_paused);
		if should_reset_timer {
			TIMER.reset();
			TIMER.resume();
		}

		// If the system is in a waiting state, but the timer hasn't been paused yet, pause it.
		if self.system_config.activation_state.is_waiting() && !timer_paused {
			TIMER.pause();
		}

		// If the system is suspending, make sure it is moved to the suspended state.
		if self.system_config.activation_state.is_suspending() {
			self.system_config
				.update_next_tick(UpdateSystemValue::ActivationState);
		}

		if self.system_config.activation_state.is_suspended() {
			// If the system is suspended and the timer has reached the suspension time, move it
			// into the waiting state.
			// TODO do minute conversion
			// TODO add suspension time value
			if TIMER.elapsed_s() >= self.system_config.activate_mins {
				self.system_config
					.update_next_tick(UpdateSystemValue::ActivationState);
			}
		} else {
			if self.system_config.activation_state.is_activated() {
				// If the system is activated and the timer has reached the activation time, move
				// it into the waiting state.
				// TODO do minute conversion
				if TIMER.elapsed_s() >= self.system_config.activate_mins {
					self.system_config
						.update_next_tick(UpdateSystemValue::ActivationState);
				}
			} else if self
				.peripherals
				.should_activate(&mut self.system_config, &mut self.adc)
			{
				// If the sensors indicate that the system should be activated, move it into the
				// activated state.
				self.system_config
					.update_next_tick(UpdateSystemValue::ActivationState);
			}
		}

		// Perform the update to the configuration if necessary and...
		if let Some(update_value) = self.system_config.update() {
			match update_value {
				// If there was any update to the activation state, update both the suspend and
				// activate menu items so that they're consistent with the configuration state.
				UpdateSystemValue::Suspend
				| UpdateSystemValue::Activate
				| UpdateSystemValue::ActivationState => {
					self.menu.update(
						UpdateSystemValue::Suspend,
						&self.system_config,
						&mut self.display,
					);
					self.menu.update(
						UpdateSystemValue::Activate,
						&self.system_config,
						&mut self.display,
					);
				}
				// Otherwise, update the relevant menu item.
				_ => {
					self.menu
						.update(update_value, &self.system_config, &mut self.display);
				}
			}
		}

		// Toggle relays if necessary.
		self.peripherals.update(&self.system_config);
	}

	/// Render the system header
	fn render_header(&mut self) {
		let _ = self.display.set_position(0, 0);
		let _ = ufmt::uwriteln!(self.display, "Garden System\nv0.1");
	}
}
