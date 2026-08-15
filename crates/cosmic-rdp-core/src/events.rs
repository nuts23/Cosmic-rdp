use crate::frame::FrameBuffer;

/// Lifecycle state of an RDP connection
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    Disconnected,
    Connecting { message: String },
    Connected,
    Reconnecting { attempt: u32 },
    Failed { reason: String },
}

/// Pointer mouse button
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    Other(u16),
}

/// Mouse input event to forward to remote RDP host
#[derive(Debug, Clone, PartialEq)]
pub enum MouseInput {
    Move { x: u16, y: u16 },
    ButtonDown { button: MouseButton, x: u16, y: u16 },
    ButtonUp { button: MouseButton, x: u16, y: u16 },
    Scroll { delta_x: i16, delta_y: i16 },
}

/// Keyboard key event to forward to remote RDP host
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyInput {
    pub scancode: u16,
    pub is_pressed: bool,
    pub is_extended: bool,
}

/// Commands sent from the UI to the active RDP engine
#[derive(Debug, Clone, PartialEq)]
pub enum SessionCommand {
    /// Send mouse pointer movement or button action
    Mouse(MouseInput),
    /// Send keyboard scancode
    Key(KeyInput),
    /// Request server-side dynamic resolution resize
    Resize { width: u32, height: u32, scale_factor: u32 },
    /// Request sending Ctrl+Alt+Del sequence
    SendCtrlAltDel,
    /// Toggle remote microphone mute
    MuteMicrophone(bool),
    /// Disconnect session
    Disconnect,
}

/// Events produced by the RDP engine and dispatched to the UI
#[derive(Debug, Clone, PartialEq)]
pub enum SessionEvent {
    StateChanged(SessionState),
    FrameUpdate(FrameBuffer),
    ServerResolutionChanged { width: u32, height: u32 },
    CertificatePrompt {
        host: String,
        fingerprint: String,
        common_name: String,
    },
    AudioStreamStatus { active: bool },
    CameraStreamStatus { active: bool },
    Error(String),
}
