# Running Remote Control

This guide covers how to build and run the application for a real end-to-end
test. The app has only been verified to compile/type-check; this is the first
real run.

## 1. Prerequisites

- **Host machine**: Windows 10/11 with **WebView2 Runtime** installed (ships with
  modern Windows / Edge). WebView2 provides the H.264 decoder and WebCodecs API
  the viewer relies on.
- **Viewer machine**: same Windows + WebView2 requirement.
- No administrator rights required on either machine (by design).
- The default signaling server is deployed and live
  (`wss://remote-control-y1jb.onrender.com`) — **nothing to run locally** for a
  basic test. (Being on Render free tier, the first connection may take a few
  seconds to wake the server.)

## 2. The binary

A freshly built portable binary is in `release_build/`:

- `release_build/Remote Control.exe`
- `release_build/WebView2Loader.dll` (keep next to the exe)

To rebuild after code changes (from WSL, per the project's cross-compile setup):

```bash
# Frontend bundle
cd "apps/desktop" && npm run build

# Windows release binary (picks up all crates).
# NOTE: --features custom-protocol is REQUIRED. Without it, Tauri does NOT embed
# the built frontend and instead loads devPath (http://localhost:5173) at
# runtime — so the window shows a blank page or whatever is on port 5173.
cd "<repo root>"
CARGO_TARGET_DIR=/tmp/target-wsl ~/.cargo/bin/cargo build --release \
  --features custom-protocol \
  --target x86_64-pc-windows-gnu \
  --manifest-path "apps/desktop/src-tauri/Cargo.toml"

# Copy out (close any running instance first, or the copy is denied/locked)
cp /tmp/target-wsl/x86_64-pc-windows-gnu/release/desktop.exe \
   "release_build/Remote Control.exe"
```

> The commands above (or `apps/desktop/package_release.sh`) build from the live
> workspace and pick up all crates. Avoid any script that copies only a subset
> of files into a separate build dir — it will produce a stale binary.

## 3. Running a session

1. **Launch the app on BOTH machines** (`Remote Control.exe`). Each instance
   generates a persistent Ed25519 Device ID on first launch.
2. **On the HOST machine** (the one being controlled): go to the **Host Agent**
   tab. It shows this machine's Device ID and listens for incoming requests.
   Optionally enable auto-accept.
3. **On the VIEWER machine** (the one controlling): the **Connect** tab lists
   discovered hosts. Click the target host to connect.
4. **On the HOST**: approve the incoming connection request (unless auto-accept
   is on).
5. The viewer should now see the host's screen (H.264 → WebCodecs → canvas),
   control mouse/keyboard, hear audio, sync clipboard, and upload files.

## 4. Toggles (viewer control dock, bottom center)

- Monitor switcher, audio mute, clipboard sync, **file upload** (sends a local
  file to the host's `downloads/` folder), fullscreen, disconnect.
- Telemetry HUD: click the status pill (top-right) for live FPS, bitrate, RTT,
  jitter, and capture/encode/decode latency.

## 5. Configuration (optional)

Set these env vars before launching the exe to override defaults
(see `docs/configuration.md`):

- `REMOTE_CONTROL_SIGNALING_URL` — e.g. `ws://192.168.1.50:8081` for a
  self-hosted/LAN signaling server (`cd apps/signaling-server && npm start`).
- `REMOTE_CONTROL_TURN_URL` / `_USER` / `_PASS` — enable a TURN relay for
  symmetric-NAT internet traversal (needed for the cross-network / friend's-PC
  test if direct P2P fails).

## 6. Known caveats for first run

- **H.264 decode** depends on WebView2 shipping the proprietary H.264 codec
  (it does on standard Windows). If the canvas stays black but input works,
  check the viewer devtools console for `VideoDecoder` errors.
- **Audio format** is negotiated via an "RCAF" header; if audio is garbled,
  it's likely a header/stream ordering issue (documented in PROJECT_STATUS).
- **TURN**: the default build has TURN disabled. Pure P2P works on most LANs and
  many home NATs; strict/symmetric NATs over the internet need a TURN relay.
