use crate::{EncodedFrame, error::VideoError};
use networking::protocol::{MessagePayload, ProtocolMessage};

pub struct VideoPacketizer {
    seq: u64,
}

impl VideoPacketizer {
    pub fn new() -> Self {
        Self { seq: 0 }
    }

    pub fn packetize(&mut self, frame: &EncodedFrame) -> Result<Vec<ProtocolMessage>, VideoError> {
        self.seq += 1;
        
        // MVP: Send the entire frame as one packet.
        // A production packetizer would fragment `frame.data` based on MTU for UDP.
        let msg = ProtocolMessage {
            seq: self.seq,
            timestamp_ms: frame.timestamp_ms,
            payload: MessagePayload::VideoPacket {
                frame_id: frame.frame_id,
                data: frame.data.clone(),
            },
        };
        Ok(vec![msg])
    }
}

pub struct VideoDepacketizer;

impl VideoDepacketizer {
    pub fn new() -> Self {
        Self
    }

    pub fn depacketize(&mut self, msg: &ProtocolMessage) -> Result<Option<EncodedFrame>, VideoError> {
        if let MessagePayload::VideoPacket { frame_id, data } = &msg.payload {
            // MVP: We assume the frame is complete in one packet.
            Ok(Some(EncodedFrame {
                frame_id: *frame_id,
                timestamp_ms: msg.timestamp_ms,
                data: data.clone(),
                is_keyframe: true,
            }))
        } else {
            Ok(None)
        }
    }
}
