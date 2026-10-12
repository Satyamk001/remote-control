use crate::error::WebRtcError;
use std::sync::Arc;
use webrtc::{
    api::{media_engine::MediaEngine, APIBuilder},
    data_channel::RTCDataChannel,
    ice_transport::ice_server::RTCIceServer,
    peer_connection::{
        configuration::RTCConfiguration,
        peer_connection_state::RTCPeerConnectionState,
        RTCPeerConnection,
    },
    track::track_local::{track_local_static_rtp::TrackLocalStaticRTP, TrackLocal},
    ice_transport::ice_candidate::RTCIceCandidateInit,
};
use tokio::sync::mpsc;
use tracing::{info, error};

pub struct WebRtcTransport {
    pub peer_connection: Arc<RTCPeerConnection>,
    pub video_channel: Option<Arc<RTCDataChannel>>,
    pub data_channel: Option<Arc<RTCDataChannel>>,
}

impl WebRtcTransport {
    pub async fn new(
        is_host: bool, 
        ice_tx: mpsc::Sender<RTCIceCandidateInit>,
        data_tx: mpsc::Sender<Vec<u8>>,
        video_tx: tokio::sync::broadcast::Sender<Vec<u8>>
    ) -> Result<Self, WebRtcError> {
        let mut media_engine = MediaEngine::default();
        media_engine.register_default_codecs().map_err(|e| WebRtcError::SdkError(e.to_string()))?;
        
        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .build();

        // ICE server configuration is environment-driven so the same binary
        // works for LAN (STUN often enough) and real cross-NAT internet tests
        // (where a reachable TURN relay is required for symmetric NATs).
        //
        //   REMOTE_CONTROL_STUN_URL   (default: Google public STUN)
        //   REMOTE_CONTROL_TURN_URL   (optional; TURN is only added if set)
        //   REMOTE_CONTROL_TURN_USER
        //   REMOTE_CONTROL_TURN_PASS
        //
        // The previous hardcoded `turn:127.0.0.1:3478` / testuser default was
        // only reachable on the dev box and useless for the friend's-PC test,
        // so TURN now defaults to *disabled* unless explicitly configured.
        let stun_url = std::env::var("REMOTE_CONTROL_STUN_URL")
            .unwrap_or_else(|_| "stun:stun.l.google.com:19302".to_string());

        let mut ice_servers = vec![
            // Primary STUN (direct P2P resolution)
            RTCIceServer {
                urls: vec![stun_url],
                ..Default::default()
            },
        ];

        // Optional TURN relay for symmetric-NAT fallback.
        if let Ok(turn_url) = std::env::var("REMOTE_CONTROL_TURN_URL") {
            if !turn_url.is_empty() {
                ice_servers.push(RTCIceServer {
                    urls: vec![turn_url],
                    username: std::env::var("REMOTE_CONTROL_TURN_USER").unwrap_or_default(),
                    credential: std::env::var("REMOTE_CONTROL_TURN_PASS").unwrap_or_default(),
                    credential_type:
                        webrtc::ice_transport::ice_credential_type::RTCIceCredentialType::Password,
                    ..Default::default()
                });
            }
        }

        let config = RTCConfiguration {
            ice_servers,
            ..Default::default()
        };

        let peer_connection = Arc::new(api.new_peer_connection(config).await
            .map_err(|e| WebRtcError::SdkError(e.to_string()))?);
            
        // Monitor state
        peer_connection.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
            info!("Peer Connection State has changed: {}", s);
            if s == RTCPeerConnectionState::Failed {
                error!("Peer Connection has gone to failed state. P2P NAT Traversal may have failed.");
            }
            Box::pin(async {})
        }));

        peer_connection.on_ice_candidate(Box::new(move |candidate| {
            let tx = ice_tx.clone();
            Box::pin(async move {
                if let Some(c) = candidate {
                    let candidate_str = c.to_string();
                    info!("Gathered ICE Candidate: {:?}", candidate_str);
                    
                    // Identify if it's a TURN relay candidate for Phase 8 diagnostics
                    if candidate_str.contains("typ relay") {
                        info!("---> DISCOVERED TURN RELAY CANDIDATE <---");
                    } else if candidate_str.contains("typ srflx") {
                        info!("---> DISCOVERED STUN (P2P) CANDIDATE <---");
                    }

                    if let Ok(json) = c.to_json() {
                        let _ = tx.send(json).await;
                    }
                }
            })
        }));

        let mut video_opts = webrtc::data_channel::data_channel_init::RTCDataChannelInit::default();
        video_opts.negotiated = Some(1);
        let video_channel = peer_connection.create_data_channel("video-stream", Some(video_opts)).await
            .map_err(|e| WebRtcError::SdkError(e.to_string()))?;

        let mut input_opts = webrtc::data_channel::data_channel_init::RTCDataChannelInit::default();
        input_opts.negotiated = Some(2);
        let data_channel = peer_connection.create_data_channel("input-events", Some(input_opts)).await
            .map_err(|e| WebRtcError::SdkError(e.to_string()))?;

        if !is_host {
            let vtx = video_tx.clone();
            video_channel.on_message(Box::new(move |msg| {
                let _ = vtx.send(msg.data.to_vec());
                Box::pin(async {})
            }));
        }

        let tx = data_tx.clone();
        data_channel.on_message(Box::new(move |msg| {
            let tx_inner = tx.clone();
            let bytes = msg.data.to_vec();
            Box::pin(async move {
                let _ = tx_inner.send(bytes).await;
            })
        }));

        let transport = Self {
            peer_connection,
            video_channel: Some(video_channel),
            data_channel: Some(data_channel),
        };

        Ok(transport)
    }
}
