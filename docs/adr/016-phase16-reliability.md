# ADR 016: Phase 16 - Reliability & Auto-Reconnection

## Status
Accepted

## Context
During standard operations over public networks, transient failures are expected (e.g. NAT mapping expiry, ISP routing changes). The application needs to gracefully handle temporary network interruptions without immediately terminating the remote session, meeting the core requirement to *"Reconnect after temporary network failures."*

## Decision
We implemented a self-healing UI architecture and backend reconnection signaling:
1. **Event-Driven UI**: The Rust backend emits a `connection_state_changed` event via Tauri IPC to the React frontend.
2. **Reconnection Overlay**: When the state transitions to `reconnecting`, the `ActiveSession` UI automatically renders a full-screen, blurred overlay indicating an active reconnection attempt, preventing the user from interacting with a stale frame.
3. **Rust PeerConnection Loop (Mocked for Now)**: The backend architecture supports monitoring `RTCPeerConnectionState::Failed`, taking the necessary steps to perform ICE Restarts, and reporting `connected` once the tunnel is re-established. 

## Consequences
**Positive:**
- Users experience a smooth transition during network hiccups rather than abrupt application closures.
- Separates the concerns of state management (Rust) and visual presentation (React).

**Negative:**
- Requires careful lifecycle management in Rust to prevent dangling reconnection loops if the peer permanently disconnects.
