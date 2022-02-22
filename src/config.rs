use core::{
	mem::{take, MaybeUninit},
	str,
};

use ufmt::{derive::uDebug, uDisplay, uWrite};

use crate::{display::ROW_LENGTH, menu::MENU_ITEM_PADDING};

/// Default amount of time in minutes which the system should be activated
const DEFAULT_ACTIVATE_MINS: u16 = 10;
/// Default minimum amount of light required for the system to potentially activate
const DEFAULT_MIN_LIGHT: u16 = 100;
/// Default minimum amount of moisture required for the system to potentially activate
const DEFAULT_MIN_MOISTURE: u16 = 100;

/// The shortest amount of time in minutes that can be configured for the system activation time
const ACTIVATION_TIME_MIN: u16 = 5;
/// The longest amount of time in minutes that can be configured for the system activation time
const ACTIVATION_TIME_MAX: u16 = 60;
/// The smallest minimum value for available light
const MIN_LIGHT_MIN: u16 = 0;
/// The largest minimum value for available light
const MIN_LIGHT_MAX: u16 = 1050;
/// The smallest minimum value for moisture
const MIN_MOISTURE_MIN: u16 = 0;
/// The largest minimum value for moisture
const MIN_MOISTURE_MAX: u16 = 1050;

/// Amount in minutes to increment the activation time by
const ACTIVATION_TIME_INCREMENT: u16 = 5;
/// Amount to increment the minimum light value by
const MIN_LIGHT_INCREMENT: u16 = 25;
/// Amount to increment the minimum moisture value by
const MIN_MOISTURE_INCREMENT: u16 = 25;

/// Display representation of a value in [`SystemConfig`]
#[derive(uDebug)]
pub enum SystemValue {
	/// Activation time minutes
	Time(u16),
	/// Minimum light value
	Light(u16),
	/// Minimum moisture value
	Moisture(u16),
	/// Activation suspended
	Suspend(ActivationState),
	/// Activated
	Activate(ActivationState),
}

/// Format a u16 value as a &str
/// Courtesy of
/// https://github.com/japaric/ufmt/blob/master/src/impls/uxx.rs#L5-L23
fn format_u16<'val, 'buf>(value: &'val u16, buf: &'buf mut [u8; 5]) -> &'buf str {
	let mut idx = buf.len() - 1;
	let mut n = *value;
	loop {
		*buf.get_mut(idx).unwrap() = (n % 10) as u8 + b'0';

		n /= 10;

		if n == 0 {
			break;
		} else {
			idx -= 1;
		}
	}
	unsafe { str::from_utf8_unchecked(buf.get(idx..).unwrap()) }
}

/// Format a bool value as a &str
fn format_bool<'val, 'buf>(value: &'val bool, buf: &'buf mut [u8; 5]) -> &'buf str {
	let symbol = if *value { '@' } else { '-' };
	let idx = buf.len() - 1;
	*buf.get_mut(idx).unwrap() = symbol as u8;
	unsafe { str::from_utf8_unchecked(buf.get(idx..).unwrap()) }
}

impl uDisplay for SystemValue {
	/// Used when rendering the [`crate::menu::Menu`]
	fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
	where
		W: uWrite + ?Sized,
	{
		let mut buf = unsafe { MaybeUninit::<[u8; 5]>::uninit().assume_init() };
		let (label, value) = match self {
			Self::Time(value) => ("Time", format_u16(value, &mut buf)),
			Self::Light(value) => ("Light", format_u16(value, &mut buf)),
			Self::Moisture(value) => ("Moisture", format_u16(value, &mut buf)),
			Self::Suspend(value) => {
				let is_suspended = value.is_suspending() || value.is_suspended();
				(
					if !is_suspended { "Suspend" } else { "Resume" },
					format_bool(&is_suspended, &mut buf),
				)
			}
			Self::Activate(value) => {
				let is_activated = value.is_activating() || value.is_activated();
				(
					if !is_activated { "Activate" } else { "Cancel" },
					format_bool(&is_activated, &mut buf),
				)
			}
		};

		// Working out how much whitespace exists between the label and the value, with the value
		// aligned to the right of the display
		let separator = ":";
		let remaining =
			ROW_LENGTH - (label.len() as u8 + MENU_ITEM_PADDING + separator.len() as u8);
		let whitespace_count = remaining - value.len() as u8;

		ufmt::uwrite!(f, "{}{}", label, separator)?;
		for _ in 0..whitespace_count {
			ufmt::uwrite!(f, " ")?;
		}
		ufmt::uwrite!(f, "{}", value)
	}
}

/// Represents a future change to a value in [`SystemConfig`]
pub enum UpdateSystemValue {
	/// Update activation time according to the [`ValueAction`] variant
	Time(ValueAction),
	/// Update the minimum light value according to the [`ValueAction`] variant
	Light(ValueAction),
	/// Update the minimum moisture value according to the [`ValueAction`] variant
	Moisture(ValueAction),
	/// Put the system in the activated state
	Activate,
	/// Put the system in the suspended state
	Suspend,
	/// Move the activation state to the next logical state
	ActivationState,
	/// Reset [`SystemConfig`]
	Reset,
}

impl UpdateSystemValue {
	/// Get a new [`UpdateSystemValue`] from a [`SystemValue`]
	pub fn from_value(system_value: &SystemValue, action: ValueAction) -> Self {
		match system_value {
			SystemValue::Time(_) => Self::Time(action),
			SystemValue::Light(_) => Self::Light(action),
			SystemValue::Moisture(_) => Self::Moisture(action),
			SystemValue::Suspend(_) => Self::Suspend,
			SystemValue::Activate(_) => Self::Activate,
		}
	}

	/// Get a new [`SystemValue`] from the current [`UpdateSystemValue`]
	pub fn to_value(&self, system_config: &SystemConfig) -> Option<SystemValue> {
		match self {
			Self::Time(_) => Some(SystemValue::Time(system_config.activate_mins)),
			Self::Light(_) => Some(SystemValue::Light(system_config.min_light)),
			Self::Moisture(_) => Some(SystemValue::Moisture(system_config.min_moisture)),
			Self::Activate => Some(SystemValue::Activate(
				system_config.activation_state.clone(),
			)),
			Self::Suspend => Some(SystemValue::Suspend(system_config.activation_state.clone())),
			Self::ActivationState => Some(SystemValue::Activate(
				system_config.activation_state.clone(),
			)),
			Self::Reset => None,
		}
	}

	/// Get a reference to the inner [`ValueAction`]
	pub fn inner_as_ref(&self) -> Option<&ValueAction> {
		match self {
			Self::Time(action) => Some(action),
			Self::Light(action) => Some(action),
			Self::Moisture(action) => Some(action),
			Self::Activate | Self::Suspend | Self::ActivationState | Self::Reset => None,
		}
	}
}

/// Type of action to perform for the [`SystemConfig`] update
pub enum ValueAction {
	/// Increment the value
	Increment,
	/// Decrement the value
	Decrement,
}

/// System state of activation
#[derive(uDebug, Clone)]
pub enum ActivationState {
	/// Activating - The next update will put the system into the activated state.
	///
	/// Helper variant to handle moving to the correct state from menu actions.
	Activating,
	/// Currently activated - No sensor readings are performed
	Activated,
	/// Waiting to be activated - Sensor readings are being performed
	Waiting,
	/// Suspending - The next update will put the system into suspended state.
	///
	/// Helper variant to handle moving to the correct state from menu actions.
	Suspending,
	/// Suspended - No sensor readings are performed
	Suspended,
}

impl ActivationState {
	/// Whether the system is currently being activated
	pub fn is_activating(&self) -> bool {
		matches!(self, Self::Activating)
	}

	/// Whether the system is currently activated
	pub fn is_activated(&self) -> bool {
		matches!(self, Self::Activated)
	}

	/// Whether the system is currently being suspended
	pub fn is_suspending(&self) -> bool {
		matches!(self, Self::Suspending)
	}
	/// Whether the system is currently suspended
	pub fn is_suspended(&self) -> bool {
		matches!(self, Self::Suspended)
	}

	/// Whether the system is currently waiting
	pub fn is_waiting(&self) -> bool {
		matches!(self, Self::Waiting)
	}
}

/// Configuration used to drive the system
pub struct SystemConfig {
	/// How long the system should be activated for
	pub activate_mins: u16,
	/// Minimum amount of light required for the system to potentially activate
	pub min_light: u16,
	/// Minimum amount of moisture required for the system to potentially activate
	pub min_moisture: u16,
	/// Current activation state of the system
	pub activation_state: ActivationState,
	/// Indicates the next update, if any, to make for a value
	update: Option<UpdateSystemValue>,
}

macro_rules! update_value {
	(add $current:expr, $add:expr, $max:expr) => {{
		let max_diff = $max - $add;
		if $current >= max_diff {
			$max
		} else {
			$current + $add
		}
	}};

	(subtract $current:expr, $subtract:expr, $min:expr) => {{
		let min_diff = $min + $subtract;
		if $current <= min_diff {
			$min
		} else {
			$current - $subtract
		}
	}};
}

impl SystemConfig {
	/// Create a new [`SystemConfig`] with default values
	pub fn new() -> Self {
		Self {
			activate_mins: DEFAULT_ACTIVATE_MINS,
			min_light: DEFAULT_MIN_LIGHT,
			min_moisture: DEFAULT_MIN_MOISTURE,
			activation_state: ActivationState::Waiting,
			update: None,
		}
	}

	/// Reset to defaults
	pub fn reset(&mut self) {
		self.activate_mins = DEFAULT_ACTIVATE_MINS;
		self.min_light = DEFAULT_MIN_LIGHT;
		self.min_moisture = DEFAULT_MIN_MOISTURE;
		self.activation_state = ActivationState::Waiting;
	}

	/// Set an update action to be performed on the next call to [`SystemConfig::update`]
	pub fn update_next_tick(&mut self, update: UpdateSystemValue) {
		self.update = Some(update);
	}

	/// Makes an update to a value if necessary
	pub fn update(&mut self) -> Option<UpdateSystemValue> {
		// Set self.update to None so that the next call to `update` doesn't peform another update.
		let update = take(&mut self.update);
		if let Some(update) = &update {
			match update {
				// If the activation time value has changed, then increment or decrement it
				UpdateSystemValue::Time(_) => match update.inner_as_ref() {
					Some(ValueAction::Increment) => {
						self.activate_mins = update_value!(add self.activate_mins, ACTIVATION_TIME_INCREMENT, ACTIVATION_TIME_MAX);
					}
					Some(ValueAction::Decrement) => {
						self.activate_mins = update_value!(subtract self.activate_mins, ACTIVATION_TIME_INCREMENT, ACTIVATION_TIME_MIN);
					}
					_ => {}
				},
				// If the minimum light value has changed, then increment or decrement it
				UpdateSystemValue::Light(_) => match update.inner_as_ref() {
					Some(ValueAction::Increment) => {
						self.min_light =
							update_value!(add self.min_light, MIN_LIGHT_INCREMENT, MIN_LIGHT_MAX);
					}
					Some(ValueAction::Decrement) => {
						self.min_light = update_value!(subtract self.min_light, MIN_LIGHT_INCREMENT, MIN_LIGHT_MIN);
					}
					_ => {}
				},
				// If the minimum moisture value has changed, then increment or decrement it
				UpdateSystemValue::Moisture(_) => match update.inner_as_ref() {
					Some(ValueAction::Increment) => {
						self.min_moisture = update_value!(add self.min_moisture, MIN_MOISTURE_INCREMENT, MIN_MOISTURE_MAX);
					}
					Some(ValueAction::Decrement) => {
						self.min_moisture = update_value!(subtract self.min_moisture, MIN_MOISTURE_INCREMENT, MIN_MOISTURE_MIN);
					}
					_ => {}
				},
				// If the activation state should be changed...
				UpdateSystemValue::ActivationState => {
					self.activation_state = match self.activation_state {
						// If it is currently suspending, move it to the suspended state;
						ActivationState::Suspending => ActivationState::Suspended,
						// If it is suspended, move it to the wating state;
						ActivationState::Suspended => ActivationState::Waiting,
						// If it is currently activating, move it to the activated state;
						ActivationState::Activating => ActivationState::Activated,
						// If it is activated, move it to the waiting state;
						ActivationState::Activated => ActivationState::Waiting,
						// If it is waiting, move it to the activated state.
						ActivationState::Waiting => ActivationState::Activated,
					}
				}
				// If the suspended state should be toggled...
				UpdateSystemValue::Activate => {
					let is_activated = self.activation_state.is_activating()
						|| self.activation_state.is_activated();
					if !is_activated {
						// If the system is not currently activating or activated, move it to the
						// activating state;
						self.activation_state = ActivationState::Activating;
					} else {
						// Otherwise, if it is activating or activated, move it to the waiting
						// state.
						self.activation_state = ActivationState::Waiting;
					}
				}
				// If the suspended state should be toggled...
				UpdateSystemValue::Suspend => {
					let is_suspended = self.activation_state.is_suspending()
						|| self.activation_state.is_suspended();
					if !is_suspended {
						// If the system is not currently suspending or suspended, move it to the
						// suspending state;
						self.activation_state = ActivationState::Suspending;
					} else {
						// Otherwise, if it is suspending or suspended, move it to the waiting
						// state.
						self.activation_state = ActivationState::Waiting;
					}
				}
				// Reset the configuration values
				UpdateSystemValue::Reset => self.reset(),
			}
		}

		update
	}
}
