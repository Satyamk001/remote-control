use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessagePayload {
    Heartbeat,
    Ping(u64),
    Pong(u64),
    ConnectionRequest { client_id: String },
    ConnectionResponse { accepted: bool, reason: Option<String> },
    VideoPacket { frame_id: u32, data: Vec<u8> },
    InputEvent { event_type: u8, data: Vec<u8> },
    ClipboardData { text: String },
    FileOffer { file_id: String, filename: String, size: u64 },
    FileAccept { file_id: String, offset: u64 },
    FileChunk { file_id: String, seq: u64, data: Vec<u8>, checksum: u32 },
    FileComplete { file_id: String },
    FileCancel { file_id: String, reason: String },
    AudioPacket { data: Vec<u8> },
    GetMonitors,
    MonitorList { monitors: Vec<String> },
    SwitchMonitor { index: u8 },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Permissions {
    pub screen: bool,
    pub mouse: bool,
    pub keyboard: bool,
    pub clipboard: bool,
    pub files: bool,
    pub audio: bool,
    pub terminal: bool,
}

impl Default for Permissions {
    fn default() -> Self {
        Self {
            screen: true,
            mouse: false,
            keyboard: false,
            clipboard: false,
            files: false,
            audio: false,
            terminal: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProtocolMessage {
    pub seq: u64,
    pub timestamp_ms: u64,
    pub payload: MessagePayload,
}
