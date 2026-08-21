use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum SignalMessage {
    Offer(String),
    Answer(String),
    IceCandidate(String),
}

/// A rudimentary manual signaling interface for local testing
/// Since there is no Node.js server yet, users can manually copy-paste JSON payload 
/// between terminal instances to exchange SDP and ICE candidates.
pub struct ManualSignaling;

impl ManualSignaling {
    pub fn display_signal(signal: &SignalMessage) {
        let json = serde_json::to_string(signal).unwrap();
        println!(">>> Please copy the following signal string and paste it on the other peer <<<");
        println!("{}", json);
        println!(">>> END SIGNAL <<<");
    }

    pub fn parse_signal(json: &str) -> Result<SignalMessage, serde_json::Error> {
        serde_json::from_str(json)
    }
}
