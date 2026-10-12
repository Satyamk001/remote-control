use serde::{Deserialize, Serialize};

/// A platform-neutral input event sent from the viewer to the host.
///
/// Coordinate convention for `MouseMove`:
/// `x` and `y` are **normalized absolute** coordinates in the range `0..=65535`
/// spanning the host's primary display (what the Win32 `SendInput` absolute
/// mouse API expects directly). The viewer is responsible for mapping its local
/// cursor position — accounting for letterboxing of the streamed image — into
/// this normalized space. This keeps injection resolution-independent on the host.
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

/// No-op injector used on non-Windows targets (and as an explicit fallback).
pub struct DummyInjector;

impl WindowsInputInjector for DummyInjector {
    fn inject_event(&self, event: &InputEvent) -> Result<(), String> {
        tracing_or_print(event);
        Ok(())
    }
}

fn tracing_or_print(event: &InputEvent) {
    // Avoid a hard tracing dependency here; a debug print is fine for the dummy.
    #[cfg(debug_assertions)]
    println!("[DummyInjector] {:?}", event);
    #[cfg(not(debug_assertions))]
    let _ = event;
}

/// Returns the best available injector for the current platform.
pub fn create_injector() -> Box<dyn WindowsInputInjector + Send + Sync> {
    #[cfg(windows)]
    {
        Box::new(windows_impl::SendInputInjector)
    }
    #[cfg(not(windows))]
    {
        Box::new(DummyInjector)
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::{InputEvent, WindowsInputInjector};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYBD_EVENT_FLAGS,
        KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_HWHEEL,
        MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP,
        MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_VIRTUALDESK,
        MOUSEEVENTF_WHEEL, MOUSEINPUT, MOUSE_EVENT_FLAGS, VIRTUAL_KEY,
    };

    /// Real injector backed by the Win32 `SendInput` API. Works as a normal,
    /// non-elevated user process (no driver / service required), matching the
    /// project's no-admin constraint. It cannot drive the secure desktop
    /// (UAC prompts, login screen) without elevation — that is out of MVP scope.
    pub struct SendInputInjector;

    impl SendInputInjector {
        fn send(input: INPUT) -> Result<(), String> {
            let sent = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
            if sent == 1 {
                Ok(())
            } else {
                Err("SendInput failed to dispatch event".to_string())
            }
        }

        fn mouse(flags: MOUSE_EVENT_FLAGS, dx: i32, dy: i32, mouse_data: i32) -> INPUT {
            INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx,
                        dy,
                        mouseData: mouse_data as u32,
                        dwFlags: flags,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            }
        }
    }

    impl WindowsInputInjector for SendInputInjector {
        fn inject_event(&self, event: &InputEvent) -> Result<(), String> {
            match *event {
                InputEvent::MouseMove { x, y } => {
                    // x/y are already normalized to 0..=65535 across the virtual
                    // desktop by the viewer.
                    let dx = x.clamp(0, 65535);
                    let dy = y.clamp(0, 65535);
                    let flags =
                        MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK;
                    Self::send(Self::mouse(flags, dx, dy, 0))
                }
                InputEvent::MouseButton { button, down } => {
                    let flags = match (button, down) {
                        (0, true) => MOUSEEVENTF_LEFTDOWN,
                        (0, false) => MOUSEEVENTF_LEFTUP,
                        (1, true) => MOUSEEVENTF_MIDDLEDOWN,
                        (1, false) => MOUSEEVENTF_MIDDLEUP,
                        (2, true) => MOUSEEVENTF_RIGHTDOWN,
                        (2, false) => MOUSEEVENTF_RIGHTUP,
                        _ => return Ok(()), // ignore unsupported buttons
                    };
                    Self::send(Self::mouse(flags, 0, 0, 0))
                }
                InputEvent::Scroll { delta_x, delta_y } => {
                    // One wheel notch == WHEEL_DELTA (120).
                    if delta_y != 0 {
                        Self::send(Self::mouse(MOUSEEVENTF_WHEEL, 0, 0, delta_y * 120))?;
                    }
                    if delta_x != 0 {
                        Self::send(Self::mouse(MOUSEEVENTF_HWHEEL, 0, 0, delta_x * 120))?;
                    }
                    Ok(())
                }
                InputEvent::KeyPress { key_code, down } => {
                    // key_code is a Windows virtual-key code (JS keyCode aligns
                    // closely for the common keys). Use the scancode where known,
                    // but SendInput with wVk set is sufficient for most keys.
                    let mut flags = KEYBD_EVENT_FLAGS(0);
                    if !down {
                        flags |= KEYEVENTF_KEYUP;
                    }
                    // Clear scancode path; rely on virtual-key injection.
                    let _ = KEYEVENTF_SCANCODE;

                    let input = INPUT {
                        r#type: INPUT_KEYBOARD,
                        Anonymous: INPUT_0 {
                            ki: KEYBDINPUT {
                                wVk: VIRTUAL_KEY(key_code as u16),
                                wScan: 0,
                                dwFlags: flags,
                                time: 0,
                                dwExtraInfo: 0,
                            },
                        },
                    };
                    Self::send(input)
                }
            }
        }
    }
}
