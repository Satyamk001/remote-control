# Phase 8: TURN Fallback

## Goal
Implement a WebRTC TURN fallback mechanism using `coturn` to guarantee connectivity when direct P2P connections are impossible due to strict NATs.

## Decisions Made
1. **Coturn Deployment**: Created a `docker-compose.turn.yml` to easily spin up a local instance of `coturn` for testing and development.
2. **Authentication**: Configured long-term credentials (`testuser:testpassword`) in both the Docker configuration and the Rust `RTCIceServer` array. Unauthenticated TURN is disabled.
3. **Diagnostics**: Added specific parsing in the `on_ice_candidate` event listener to log `typ relay` and `typ srflx` candidates, fulfilling the requirement to distinguish between P2P and TURN connectivity states.
4. **Signaling Separation**: The signaling server remains completely unaware of the TURN server. The credentials and URLs are injected directly into the Rust ICE configuration.

## Status
Fully implemented in the Rust codebase and `docker-compose`. Verified compilation via WSL cross-compilation. Ready for integration into the UI phases.
