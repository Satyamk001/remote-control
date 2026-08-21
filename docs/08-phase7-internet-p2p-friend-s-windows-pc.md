# Phase 7 — Internet P2P + Friend's Windows PC
Implement Phase 7 only.

Now test the system across the public Internet.

Machines:

My Windows PC
        │
     Internet
        │
Friend's Windows PC

My Windows PC has NO administrator rights.

The application must not require administrator privileges on my client.

Use:
WebRTC
## ICE
## STUN
Node.js signaling

Do not implement TURN yet.

Requirements:
- ICE configuration
- STUN configuration
- connection state
- candidate types
- direct P2P detection
- connection statistics
- reconnect
- timeout
- stale session cleanup

Test the actual friend's Windows PC.

Measure:
- RTT
- packet loss
- jitter
- bitrate
- FPS
- connection establishment time
- ICE candidate type
- direct P2P success/failure

Document the actual result.

If P2P fails because of NAT/CGNAT/firewall, explain exactly why.

Do not bypass network/security restrictions.

STOP after Phase 7.
