# Phase 7: Internet P2P Results (Diagnostic Simulation)

## Blockers

The actual P2P connection cannot be established directly at this time because the `remote-host` and `remote-client` Rust applications cannot be compiled on the Windows host machine (missing Microsoft Visual C++ Build Tools `link.exe`).

## Architecture & Configuration

To support Internet traversal in Phase 7, the `WebRtcTransport` in `crates/webrtc/src/transport.rs` has been configured with public STUN servers:

- `stun:stun.l.google.com:19302`
- `stun:stun1.l.google.com:19302`

These STUN servers allow the endpoints to discover their public IP addresses and port mappings created by their NATs.

## Theoretical NAT Traversal Outcomes

If the test were executed over the public internet, the connection success depends entirely on the NAT types of the two Windows PCs:

1. **Full Cone / Restricted Cone NATs (Success)**
   - **Result**: Direct P2P connection established.
   - **Candidates Used**: `srflx` (Server Reflexive) candidates.
   - **Metrics Expected**: RTT < 50ms, Jitter < 10ms, Bitrate ~ 5-10 Mbps depending on bandwidth.

2. **Symmetric NAT on both sides (Failure)**
   - **Result**: Connection Fails.
   - **Why**: Symmetric NATs map different external ports for different destinations. The port discovered by the STUN server will not match the port used to talk to the peer. Since WebRTC ICE requires predicting the external port, both sides being Symmetric means hole punching will definitively fail.
   - **Solution**: A TURN (Traversal Using Relays around NAT) server is required to relay the media. Phase 7 requirements explicitly state "Do not implement TURN yet", so this scenario will result in a hard failure (Peer Connection State: `Failed`).

## Mock Diagnostic Test

A mock script representing the statistics polling we would perform:

```rust
pub async fn poll_webrtc_stats(pc: Arc<RTCPeerConnection>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
    loop {
        interval.tick().await;
        let stats = pc.get_stats().await;
        for (id, stat) in stats.iter() {
            println!("Stat [{}]: {:?}", id, stat);
            // This would print actual RTT, Jitter, Packets Lost, and Bitrate
        }
    }
}
```
