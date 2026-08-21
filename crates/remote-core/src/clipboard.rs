use arboard::Clipboard;
use tracing::{error, info};

pub struct ClipboardManager {
    clipboard: Option<Clipboard>,
}

impl ClipboardManager {
    pub fn new() -> Self {
        let clipboard = match Clipboard::new() {
            Ok(c) => Some(c),
            Err(e) => {
                error!("Failed to initialize clipboard: {}", e);
                None
            }
        };
        Self { clipboard }
    }

    pub fn set_text(&mut self, text: &str) {
        if let Some(clip) = &mut self.clipboard {
            if let Err(e) = clip.set_text(text) {
                error!("Failed to set clipboard text: {}", e);
            } else {
                info!("Clipboard updated successfully.");
            }
        } else {
            error!("Clipboard is not available.");
        }
    }

    pub fn get_text(&mut self) -> Option<String> {
        if let Some(clip) = &mut self.clipboard {
            match clip.get_text() {
                Ok(text) => Some(text),
                Err(e) => {
                    error!("Failed to read clipboard text: {}", e);
                    None
                }
            }
        } else {
            None
        }
    }
}
