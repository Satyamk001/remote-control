use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputEvent {
    MouseMove { x: i32, y: i32 },
    MouseButton { button: u8, down: bool },
    KeyPress { key_code: u32, down: bool },
    Scroll { delta_x: i32, delta_y: i32 },
}

pub trait WindowsInputInjector {
    fn inject_event(&self, event: &InputEvent) -> Result<(), String>;
}

pub struct DummyInjector;

impl WindowsInputInjector for DummyInjector {
    fn inject_event(&self, event: &InputEvent) -> Result<(), String> {
        // Dummy implementation for MVP/testing
        println!("Injected event: {:?}", event);
        Ok(())
    }
}
