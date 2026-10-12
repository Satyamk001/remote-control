# Running the Viewer on Ubuntu (22.04)

The app runs on Linux as a **viewer** (the Windows machine stays the host —
screen capture + input injection are Windows-only). On Linux the webview is
WebKitGTK (no WebCodecs/H.264), so the app automatically negotiates the **MJPEG
fallback** and renders frames in an `<img>`.

## Option 1 — Prebuilt binary (no building required)

A prebuilt Linux binary is included:

```
release_build/linux/remote-control
```

Copy it to the Ubuntu machine and make it executable:

```bash
chmod +x remote-control
./remote-control
```

### IMPORTANT: one-time library compatibility

The prebuilt binary is linked against **webkit2gtk-4.0** (built on Ubuntu 20.04).
Ubuntu 22.04 ships 4.1 by default, so you must install the 4.0 runtime once:

```bash
sudo apt update
sudo apt install -y libwebkit2gtk-4.0-37 libjavascriptcoregtk-4.0-18
```

(These are the runtime libraries only — not the full dev packages.) After that,
`./remote-control` starts directly, no building involved.

> Why this is needed: a remote-desktop viewer embeds a browser engine, and the
> prebuilt binary records the exact engine version it was compiled against. The
> 4.0 runtime package provides it on 22.04. If you prefer a binary that uses
> your system's native 4.1 engine instead, use Option 2.

## Option 2 — Build natively (uses your system's webkit 4.1)

Only needed if you don't want to install the 4.0 runtime, or want a native
package. See the dependency list and steps below.

```bash
sudo apt update
sudo apt install -y build-essential curl wget file \
  libwebkit2gtk-4.1-dev libssl-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev nodejs npm
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

cd remote-control/apps/desktop && npm install && npm run build
cd ../..
cargo build --release --features custom-protocol \
  --manifest-path apps/desktop/src-tauri/Cargo.toml
./apps/desktop/src-tauri/target/release/desktop   # or target/release/desktop
```

`--features custom-protocol` is REQUIRED, else the app loads
`http://localhost:5173` instead of the embedded UI.

## Running a session

1. Start `Remote Control.exe` on the **Windows** host (Host Agent tab;
   optionally enable auto-accept).
2. Start the viewer on **Ubuntu**; go to the **Connect** tab; your host appears
   in the device list (shared deployed signaling server).
3. Click it to connect. Video arrives as MJPEG; input, clipboard, audio, and
   file upload work as on Windows.

Logs: `/tmp/remote-control.log` (or set `REMOTE_CONTROL_DATA_DIR=/some/dir`).

## Honest constraints (cannot be engineered away)

- **Host must be Windows.** DXGI capture + Win32 `SendInput` are Windows-only;
  an Ubuntu machine can only be the *viewer*.
- **Not network-independent.** This is a remote-desktop app — it fundamentally
  requires connectivity between the two machines and a reachable signaling
  server. On the same LAN, direct P2P works. Across different networks (e.g. a
  friend's PC), strict/symmetric NAT may require a TURN relay — configure
  `REMOTE_CONTROL_TURN_URL` / `_USER` / `_PASS` (see `configuration.md`).
- **MJPEG uses more bandwidth** than H.264 (full JPEG per frame). Fine on LAN.
