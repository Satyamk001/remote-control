use crate::{Renderer, error::VideoError};
use screen_capture::Frame;

pub struct DummyRenderer {
    frames_rendered: u32,
}

impl DummyRenderer {
    pub fn new() -> Self {
        Self { frames_rendered: 0 }
    }
}

impl Renderer for DummyRenderer {
    fn render(&mut self, _frame: &Frame) -> Result<(), VideoError> {
        self.frames_rendered += 1;
        if self.frames_rendered % 30 == 0 {
            println!("Renderer: Displayed {} frames.", self.frames_rendered);
        }
        Ok(())
    }
}
