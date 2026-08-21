# Phase 21 — Interactive Session Approval

Implement Phase 21 only.

Implement a security authorization gate on the host side. Instead of automatically accepting incoming WebRTC connection attempts, the host must manually accept or reject the session from the UI.

Target:
Host receives a clear visual prompt to accept incoming connections.

Steps:

1. Intercept `SdpOffer` in the background loop and store it in a pending buffer.
2. Emit an IPC event to React (`connection_requested`).
3. Display the `[Accept] / [Reject]` prompt in the Host Agent panel.
4. If approved, retrieve the pending `SdpOffer`, instantiate the WebRTC tunnel, compute the `SdpAnswer`, and proceed.

Produce:

- Interactive host UI prompt.
- Secure connection gate mechanism.

STOP after Phase 21.
