use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtonFunction {
    None,
    Left,
    Right,
    Middle,
    Action,
    Laser,
    ScrollUp,
    ScrollDown,
}

pub const MAX_FUNCTIONS_PER_BUTTON: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ButtonMapping {
    pub fns: [ButtonFunction; MAX_FUNCTIONS_PER_BUTTON],
}

impl ButtonMapping {
    pub const fn new(fns: [ButtonFunction; MAX_FUNCTIONS_PER_BUTTON]) -> Self {
        Self { fns }
    }
}

pub const PHYSICAL_BUTTON_COUNT: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ButtonProfile {
    pub context: ButtonMapping,
    pub action: ButtonMapping,
    pub up: ButtonMapping,
    pub down: ButtonMapping,
    pub primary: ButtonMapping,
    pub secondary: ButtonMapping,
}

const fn none() -> ButtonFunction {
    ButtonFunction::None
}

const fn btn(fns: [ButtonFunction; 4]) -> ButtonMapping {
    ButtonMapping::new(fns)
}

impl Default for ButtonProfile {
    fn default() -> Self {
        Self {
            context: btn([ButtonFunction::Middle, none(), none(), none()]),
            action: btn([ButtonFunction::Action, none(), none(), none()]),
            up: btn([ButtonFunction::ScrollUp, none(), none(), none()]),
            down: btn([ButtonFunction::ScrollDown, none(), none(), none()]),
            primary: btn([ButtonFunction::Left, none(), none(), none()]),
            secondary: btn([ButtonFunction::Right, none(), none(), none()]),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputConfig {
    pub profiles: [ButtonProfile; 3],
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            profiles: [
                ButtonProfile::default(),
                ButtonProfile::default(),
                ButtonProfile::default(),
            ],
        }
    }
}
