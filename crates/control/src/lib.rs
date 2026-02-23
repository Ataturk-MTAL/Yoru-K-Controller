pub mod joystick;
pub mod keyboard;

pub use joystick::{JoystickInput, MotorSpeeds};
pub use joystick::calculate as joystick_calculate;
pub use keyboard::{KeyboardState};
pub use keyboard::calculate as keyboard_calculate;
