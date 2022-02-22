use arduino_hal::{
	clock::MHz16,
	hal::{port::PC2, Adc},
	port::{mode::Analog, Pin},
};
use core::convert::TryFrom;

/// The lower and upper bounds of the analog read for button 1 (Select)
const BUTTON_1_THRESHOLD: (u16, u16) = (195, 220);
/// The lower and upper bounds of the analog read for button 2 (Left)
const BUTTON_2_THRESHOLD: (u16, u16) = (395, 415);
/// The lower and upper bounds of the analog read for button 3 (Right)
const BUTTON_3_THRESHOLD: (u16, u16) = (990, 1023);

/// Variants representing a button
#[derive(PartialEq, Eq)]
pub enum ButtonType {
	/// Button 1
	Select,
	/// Button 2
	Left,
	/// Button 3
	Right,
}

/// Variants representing the current stage of a button press
pub enum ButtonStage {
	/// Button has been pressed down
	Down,
	/// Button is being held down
	Hold,
	/// Button was released
	Release,
}

/// Represents the current state of a button
pub struct ButtonState {
	/// Button stage
	pub stage: ButtonStage,
	/// Button type
	pub button: ButtonType,
}

impl ButtonState {
	/// Create a new button state with the button in the [`ButtonStage::Down`] stage
	fn new(button: ButtonType) -> Self {
		Self {
			stage: ButtonStage::Down,
			button,
		}
	}
}

impl TryFrom<u16> for ButtonType {
	type Error = ();

	/// Attempt to convert the analog reading into a [`ButtonType`] based on the lower and upper
	/// bounds for each button
	fn try_from(value: u16) -> Result<Self, Self::Error> {
		match value {
			value if value >= BUTTON_1_THRESHOLD.0 && value < BUTTON_1_THRESHOLD.1 => {
				Ok(Self::Select)
			}
			value if value >= BUTTON_2_THRESHOLD.0 && value < BUTTON_2_THRESHOLD.1 => {
				Ok(Self::Left)
			}
			value if value >= BUTTON_3_THRESHOLD.0 && value < BUTTON_3_THRESHOLD.1 => {
				Ok(Self::Right)
			}
			_ => Err(()),
		}
	}
}

/// Current state of the "control pad", i.e. buttons
pub struct ControlPad {
	/// Holds the pin for taking analog readings
	buttons_input: Pin<Analog, PC2>,
	/// Whether a button is in a state, and what state it is in
	///
	/// `None` means that no button is being pressed.
	pub state: Option<ButtonState>,
}

impl ControlPad {
	/// Create a new `ControlPad` using the A2 pin
	pub fn new(buttons_input: Pin<Analog, PC2>) -> Self {
		Self {
			buttons_input,
			state: None,
		}
	}

	/// Takes an analog reading and updates the control pad's state
	pub fn update(&mut self, adc: &mut Adc<MHz16>) {
		// Take an analog reading.
		let value = self.buttons_input.analog_read(adc);
		// Convert the `Result<ButtonType, ()>` to an `Option<ButtonType>`.
		let button = ButtonType::try_from(value).ok();

		// Compare the current state with the new state.
		//
		// This ignores a possible state where one button can be pressed in the next tick from
		// releasing a different button. I think thats a rare (nearly impossible physically) edge
		// case and its quicker to write a comment about it, than handle it.
		match (&mut self.state, button) {
			(None, Some(button)) => {
				// Set the button as pressed.
				self.state = Some(ButtonState::new(button));
			}
			(Some(button_state), button) => match (&button_state.stage, button) {
				(ButtonStage::Down, Some(button)) if button == button_state.button => {
					// If the current stage is `Down`, and the same button is being pressed, then
					// move the state into `Hold`.
					button_state.stage = ButtonStage::Hold;
				}
				(ButtonStage::Hold, None) => {
					// If the current stage is `Hold` and no button is being pressed any longer,
					// move the current stage into `Release`.
					button_state.stage = ButtonStage::Release;
				}
				(ButtonStage::Release, _) => {
					// If the current stage is `Release`, then update the state so that no button
					// is being pressed.
					self.state = None;
				}
				_ => {}
			},
			_ => {}
		};
	}
}
