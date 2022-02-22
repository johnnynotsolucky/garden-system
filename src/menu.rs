use ufmt::{uDisplay, uWrite};

use crate::{
	config::{SystemConfig, SystemValue, UpdateSystemValue, ValueAction},
	control_pad::{ButtonStage, ButtonState, ButtonType},
	display::{Display, BODY_START_ROW},
};

/// Amount of padding to add infront of a menu item
pub const MENU_ITEM_PADDING: u8 = 2;

/// The menu. Keeps track of the currently selected item, and holds a list of menu items to display
/// in order.
pub struct Menu {
	current_idx: u8,
	items: [MenuItem; 6],
}

impl Menu {
	/// Create a new menu from current [`SystemConfig`] values
	pub fn new(system_config: &SystemConfig) -> Self {
		Self {
			current_idx: 0,
			items: [
				MenuItem::Time(SystemValue::Time(system_config.activate_mins)),
				MenuItem::Light(SystemValue::Light(system_config.min_light)),
				MenuItem::Moisture(SystemValue::Moisture(system_config.min_moisture)),
				MenuItem::Activate(SystemValue::Activate(
					system_config.activation_state.clone(),
				)),
				MenuItem::Suspend(SystemValue::Suspend(system_config.activation_state.clone())),
				MenuItem::Reset,
			],
		}
	}

	/// Similar to [`Menu::new`], but resets the value for each menu item to the corresponding
	/// value in [`SystemConfig`]
	fn reset(&mut self, system_config: &SystemConfig) {
		self.current_idx = 0;
		self.items.iter_mut().for_each(|item| match item {
			MenuItem::Time(value) => *value = SystemValue::Time(system_config.activate_mins),
			MenuItem::Light(value) => *value = SystemValue::Light(system_config.min_light),
			MenuItem::Moisture(value) => *value = SystemValue::Moisture(system_config.min_moisture),
			MenuItem::Activate(value) => {
				*value = SystemValue::Activate(system_config.activation_state.clone())
			}
			MenuItem::Suspend(value) => {
				*value = SystemValue::Suspend(system_config.activation_state.clone())
			}
			_ => {}
		})
	}

	/// Render the entire menu
	///
	/// The OLED (that I have) renders a full menu slowly so calling this should be limited to when
	/// the program launches, and whenever the menu resets only.
	pub fn render(&self, display: &mut Display) {
		display.clear_body();
		for (idx, item) in self.items.iter().enumerate() {
			Self::render_item(idx, item, display);
		}

		Self::render_selector(display, None, self.current_idx);
	}

	/// Render a single menu item
	///
	/// Faster than [`Menu::render`] - Should be limit calls to only whenever a system value
	/// changes.
	fn render_item(idx: usize, item: &MenuItem, display: &mut Display) {
		let _ = display.set_position(0, BODY_START_ROW + idx as u8);

		// Render the padding first.
		for _ in 0..MENU_ITEM_PADDING {
			let _ = ufmt::uwrite!(display, " ");
		}

		// Continue from the last position and render the item.
		let _ = ufmt::uwriteln!(display, "{}", item);
	}

	/// Render the selection indicator
	///
	/// First clears the previous selection, and then renders the new selection indicator.
	fn render_selector(display: &mut Display, previous_idx: Option<u8>, current_idx: u8) {
		// Clear the previous selection
		if let Some(previous_idx) = previous_idx {
			let _ = display.set_position(0, BODY_START_ROW + previous_idx);
			let _ = ufmt::uwrite!(display, " ");
		}

		let _ = display.set_position(0, BODY_START_ROW + current_idx);
		let _ = ufmt::uwrite!(display, ">");
	}

	/// Update a menu item associated with a system value change from [`UpdateSystemValue`]
	///
	/// - Updates the value stored in the corresponding [`MenuItem`];
	/// - Rerenders the menu item;
	/// - And, rerenders the selection (because rendering a menu item writes a full line).
	pub fn update(
		&mut self,
		update_value: UpdateSystemValue,
		system_config: &SystemConfig,
		display: &mut Display,
	) {
		if let UpdateSystemValue::Reset = update_value {
			self.reset(system_config);
			self.render(display);
		} else {
			// Find the menu item associated with the UpdateSystemValue.
			let item = self
				.items
				.iter_mut()
				.enumerate()
				.find(|(_idx, item)| match update_value {
					UpdateSystemValue::Time(_) => matches!(item, MenuItem::Time(_)),
					UpdateSystemValue::Light(_) => matches!(item, MenuItem::Light(_)),
					UpdateSystemValue::Moisture(_) => matches!(item, MenuItem::Moisture(_)),
					UpdateSystemValue::Suspend => matches!(item, MenuItem::Suspend(_)),
					UpdateSystemValue::Activate => matches!(item, MenuItem::Activate(_)),
					_ => false,
				});

			if let Some((idx, item)) = item {
				// If a menu item is found, update its value from the value in system_config.
				let system_value = update_value.to_value(system_config);
				item.set_value(system_value);

				// Rerender the item.
				Self::render_item(idx, item, display);
				// Rerender the selector.
				Self::render_selector(display, None, self.current_idx);
			}
		}
	}

	/// Handle a button press event
	pub fn on_press(
		&mut self,
		button_state: &ButtonState,
		display: &mut Display,
		system_config: &mut SystemConfig,
	) {
		match (&button_state.stage, &button_state.button) {
			(ButtonStage::Release, ButtonType::Select) => {
				// If the select button has been pressed, move the current selection to the next
				// menu item, or the first if the current item is the last menu item.
				let previous_idx = self.current_idx;
				if self.current_idx == (self.items.len() - 1) as u8 {
					self.current_idx = 0;
				} else {
					self.current_idx += 1;
				}
				// Rerender the selector.
				Self::render_selector(display, Some(previous_idx), self.current_idx as u8);
			}
			(ButtonStage::Release, ButtonType::Right) => {
				// If the right button has been pressed, fetch the current selection and...
				let item = &self.items[self.current_idx as usize];
				match item {
					MenuItem::Time(value) | MenuItem::Light(value) | MenuItem::Moisture(value) => {
						// If the current item can be incremented (example: u16), then create a new
						// UpdateSystemValue with the Increment action.
						system_config.update_next_tick(UpdateSystemValue::from_value(
							value,
							ValueAction::Increment,
						));
					}
					MenuItem::Suspend(_) => {
						// If the current item is Suspend/Resume, create a Suspend
						// UpdateSystemValue variant which will toggle the systems suspension
						// state.
						system_config.update_next_tick(UpdateSystemValue::Suspend);
					}
					MenuItem::Activate(_) => {
						// If the current item is Activate/Cancel, create an Activate
						// UpdateSystemValue variant which will toggle the systems activation
						// state.
						system_config.update_next_tick(UpdateSystemValue::Activate);
					}
					MenuItem::Reset => {
						// If the item is Reset, create a Reset variant which will reset the values
						// in system_config, and reset the menu state.
						system_config.update_next_tick(UpdateSystemValue::Reset);
					}
				}
			}
			(ButtonStage::Release, ButtonType::Left) => {
				// If the left button has been pressed, fetch the current selection and...
				let item = &self.items[self.current_idx as usize];
				match item {
					MenuItem::Time(value) | MenuItem::Light(value) | MenuItem::Moisture(value) => {
						// If the current item can be decremented (example: u16), then create a new
						// UpdateSystemValue with the Decrement action.
						system_config.update_next_tick(UpdateSystemValue::from_value(
							value,
							ValueAction::Decrement,
						));
					}
					_ => {}
				}
			}
			_ => {}
		}
	}
}

/// Menu item
///
/// Each variant is a possible menu item.
///
/// This is not well modeled - it is possible to store an incorrect [`SystemValue`] variant inside
/// a [`MenuItem`] variant.
enum MenuItem {
	Time(SystemValue),
	Light(SystemValue),
	Moisture(SystemValue),
	Suspend(SystemValue),
	Activate(SystemValue),
	Reset,
}

impl MenuItem {
	/// Updates the inner value of a [`MenuItem`] variant
	pub fn set_value(&mut self, system_value: Option<SystemValue>) {
		if let Some(system_value) = system_value {
			match self {
				Self::Time(value) => *value = system_value,
				Self::Light(value) => *value = system_value,
				Self::Moisture(value) => *value = system_value,
				Self::Suspend(value) => *value = system_value,
				Self::Activate(value) => *value = system_value,
				Self::Reset => {}
			}
		}
	}
}

impl uDisplay for MenuItem {
	fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
	where
		W: uWrite + ?Sized,
	{
		match self {
			Self::Time(value) => ufmt::uwrite!(f, "{}", value),
			Self::Light(value) => ufmt::uwrite!(f, "{}", value),
			Self::Moisture(value) => ufmt::uwrite!(f, "{}", value),
			Self::Suspend(value) => ufmt::uwrite!(f, "{}", value),
			Self::Activate(value) => ufmt::uwrite!(f, "{}", value),
			Self::Reset => ufmt::uwrite!(f, "Reset"),
		}
	}
}
