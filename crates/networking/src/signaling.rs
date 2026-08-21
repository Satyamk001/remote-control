use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, broadcast, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HostInfo {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "deviceId")]
    pub device_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum SignalingEvent {
    #[serde(rename = "hosts_list")]
    HostsList { hosts: Vec<HostInfo> },
    #[serde(rename = "sdp_offer")]
    SdpOffer {
        #[serde(rename = "sessionId")]
        session_id: String,
        sdp: String,
    },
    #[serde(rename = "sdp_answer")]
    SdpAnswer {
        #[serde(rename = "sessionId")]
        session_id: String,
        sdp: String,
    },
    #[serde(rename = "ice_candidate")]
    IceCandidate {
        #[serde(rename = "sessionId")]
        session_id: String,
        candidate: String,
        #[serde(rename = "sdpMid")]
        sdp_mid: Option<String>,
        #[serde(rename = "sdpMLineIndex")]
        sdp_m_line_index: Option<u16>,
    },
    #[serde(rename = "viewer_joined")]
    ViewerJoined {
        #[serde(rename = "sessionId")]
        session_id: String,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Clone)]
pub struct SignalingClient {
    tx: mpsc::Sender<Message>,
    pub hosts: Arc<RwLock<Vec<HostInfo>>>,
    pub events_tx: broadcast::Sender<SignalingEvent>,
}

impl SignalingClient {
    pub fn subscribe_events(&self) -> broadcast::Receiver<SignalingEvent> {
        self.events_tx.subscribe()
    }

    pub async fn connect(url: &str, device_id: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        let mut request = Url::parse(url)?.into_client_request()?;
        request.headers_mut().insert("ngrok-skip-browser-warning", "1".parse().unwrap());
        
        let (ws_stream, _) = connect_async(request).await?;
        let (mut write, mut read) = ws_stream.split();

        let (tx, mut rx) = mpsc::channel::<Message>(32);
        let hosts = Arc::new(RwLock::new(Vec::new()));
        let hosts_clone = hosts.clone();

        let (events_tx, _events_rx) = broadcast::channel(128);
        let events_tx_clone = events_tx.clone();

        let tx_clone = tx.clone();
        
        // Read loop
        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                if let Ok(Message::Text(text)) = msg {
                    if let Ok(event) = serde_json::from_str::<SignalingEvent>(&text) {
                        if let SignalingEvent::HostsList { ref hosts } = event {
                            *hosts_clone.write().await = hosts.clone();
                        }
                        let _ = events_tx_clone.send(event);
                    }
                }
            }
        });

        // Write loop
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let _ = write.send(msg).await;
            }
        });

        let client = Self { tx: tx_clone, hosts, events_tx };

        // Register host
        let reg_msg = serde_json::json!({
            "type": "register_host",
            "deviceId": device_id
        });
        client.tx.send(Message::Text(reg_msg.to_string())).await?;

        Ok(client)
    }

    pub async fn request_hosts_update(&self) {
        let msg = serde_json::json!({ "type": "get_hosts" });
        let _ = self.tx.send(Message::Text(msg.to_string())).await;
    }

    pub async fn send_join_session(&self, session_id: &str) {
        let msg = serde_json::json!({ "type": "join_session", "sessionId": session_id });
        let _ = self.tx.send(Message::Text(msg.to_string())).await;
    }

    pub async fn send_sdp_offer(&self, session_id: &str, sdp: &str) {
        let msg = SignalingEvent::SdpOffer {
            session_id: session_id.to_string(),
            sdp: sdp.to_string(),
        };
        let _ = self.tx.send(Message::Text(serde_json::to_string(&msg).unwrap())).await;
    }

    pub async fn send_sdp_answer(&self, session_id: &str, sdp: &str) {
        let msg = SignalingEvent::SdpAnswer {
            session_id: session_id.to_string(),
            sdp: sdp.to_string(),
        };
        let _ = self.tx.send(Message::Text(serde_json::to_string(&msg).unwrap())).await;
    }

    pub async fn send_ice_candidate(&self, session_id: &str, candidate: &str, sdp_mid: Option<String>, sdp_m_line_index: Option<u16>) {
        let msg = SignalingEvent::IceCandidate {
            session_id: session_id.to_string(),
            candidate: candidate.to_string(),
            sdp_mid,
            sdp_m_line_index,
        };
        let _ = self.tx.send(Message::Text(serde_json::to_string(&msg).unwrap())).await;
    }
}
