# Runtime Configuration

The desktop client and signaling server are configured via environment
variables. All variables are optional and fall back to the defaults below, so
the app runs with zero configuration against the deployed public signaling
server.

## Desktop client (Tauri / Rust)

Set these in the environment that launches the `.exe`.

| Variable | Default | Purpose |
| --- | --- | --- |
| `REMOTE_CONTROL_SIGNALING_URL` | `wss://remote-control-y1jb.onrender.com` | WebSocket signaling endpoint. Point this at a self-hosted/LAN server, e.g. `ws://192.168.1.50:8081`. |
| `REMOTE_CONTROL_STUN_URL` | `stun:stun.l.google.com:19302` | STUN server for direct P2P ICE resolution. |
| `REMOTE_CONTROL_TURN_URL` | *(unset — TURN disabled)* | TURN relay URL for symmetric-NAT fallback, e.g. `turn:relay.example.com:3478`. TURN is only added to the ICE config when this is set. |
| `REMOTE_CONTROL_TURN_USER` | *(empty)* | TURN username (required if `REMOTE_CONTROL_TURN_URL` is set). |
| `REMOTE_CONTROL_TURN_PASS` | *(empty)* | TURN credential/password. |

### Example: fully self-hosted LAN test

```bash
# On both host and viewer machines
set REMOTE_CONTROL_SIGNALING_URL=ws://192.168.1.50:8081
```

### Example: internet test with a TURN relay

```bash
set REMOTE_CONTROL_TURN_URL=turn:relay.example.com:3478
set REMOTE_CONTROL_TURN_USER=myuser
set REMOTE_CONTROL_TURN_PASS=mysecret
```

> Note: the previous build hardcoded `turn:127.0.0.1:3478` with test
> credentials, which was only reachable on the dev machine. TURN now defaults
> to disabled; configure the variables above to enable relay fallback for real
> cross-network sessions.

## Signaling server (Node.js)

| Variable | Default | Purpose |
| --- | --- | --- |
| `PORT` | `8081` | WebSocket listen port. |

```bash
cd apps/signaling-server
PORT=8081 npm start
```

A `coturn` TURN server is also provided via `docker-compose.turn.yml` for local
relay testing.
