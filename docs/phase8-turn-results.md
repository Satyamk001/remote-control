# Phase 8: TURN Fallback Diagnostics

## Architecture

We have integrated a `coturn` server (`docker-compose.turn.yml`) to act as a fallback relay for WebRTC connections when direct P2P is blocked by Symmetric NATs or strict firewalls.

The Rust client uses the following ICE gathering strategy:

1. Try gathering local Host candidates.
2. Try gathering Server Reflexive (`srflx`) candidates via `stun.l.google.com`.
3. Try gathering Relay (`relay`) candidates via the local `coturn` server (`127.0.0.1:3478`).

## Forced TURN Testing

To manually test and force the TURN fallback to be used instead of direct P2P:

1. **Start the TURN server**:

   ```bash
   docker-compose -f docker-compose.turn.yml up -d
   ```

2. **Force Relay Only**:
   In `crates/webrtc/src/transport.rs`, temporarily change the WebRTC configuration to strictly prefer relays:

   ```rust
   let mut config = RTCConfiguration { ... };
   config.ice_transport_policy = RTCIceTransportPolicy::Relay;
   ```

3. **Observe Logs**:
   When the client connects, observe the ICE candidate logs. You should see:
   `---> DISCOVERED TURN RELAY CANDIDATE <---`

   This confirms the WebRTC stack successfully authenticated with the TURN server using the `testuser:testpassword` credentials and allocated a relay port.

## Expected Metrics

- **P2P Latency**: ~10-50ms (Direct Path)
- **TURN Latency**: ~50-150ms+ (Routed through the relay server)
- **CPU Impact**: Slightly higher on the host due to maintaining the relay heartbeat, but minimal overall impact compared to video encoding.
