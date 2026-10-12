# Single-PC Loopback Self-Test (Option C)

Run two instances of the app on **one Windows PC** — one acting as the host, one
as the viewer — to validate the full real pipeline (DXGI capture → H.264 →
WebCodecs → canvas, input injection, audio, clipboard, file transfer, metrics)
without a second machine or LAN.

This works because:

- The two instances discover each other through the **deployed signaling
  server** (no local server needed).
- Each instance gets its **own Device ID** by pointing `REMOTE_CONTROL_DATA_DIR`
  at a different folder (otherwise both would share one identity and collide).
- Each instance binds its own ephemeral localhost ports, so there is no clash.

## Prerequisites

- Windows 10/11 with the **WebView2 Runtime** installed (standard on current
  Windows; it provides the H.264 decoder + WebCodecs the viewer needs).
- The freshly built binary: `release_build/Remote Control.exe` and
  `release_build/WebView2Loader.dll` (kept side by side).
- Internet access (to reach `wss://remote-control-y1jb.onrender.com`; the free
  Render instance may take a few seconds to wake on first connect).

## Steps

Open two terminals (PowerShell or CMD). The key is the different
`REMOTE_CONTROL_DATA_DIR` per instance.

### Terminal 1 — HOST instance

PowerShell:

```powershell
$env:REMOTE_CONTROL_DATA_DIR = "$env:TEMP\rc-host"
& ".\release_build\Remote Control.exe"
```

CMD:

```cmd
set REMOTE_CONTROL_DATA_DIR=%TEMP%\rc-host
"release_build\Remote Control.exe"
```

In this window:

1. Go to the **Host Agent** tab.
2. Note it shows a Device ID. Optionally toggle **auto-accept** on (simplifies
   the test so you don't have to approve from this window).

### Terminal 2 — VIEWER instance

PowerShell:

```powershell
$env:REMOTE_CONTROL_DATA_DIR = "$env:TEMP\rc-viewer"
& ".\release_build\Remote Control.exe"
```

CMD:

```cmd
set REMOTE_CONTROL_DATA_DIR=%TEMP%\rc-viewer
"release_build\Remote Control.exe"
```

In this window:

1. Stay on the **Connect** tab.
2. Within a few seconds the host instance should appear in the device list
   (named `Host-xxxxxxxx`). Click it to connect.
3. If auto-accept is OFF, switch to the host window and click **Accept**.

## What to verify

Once connected, in the viewer window:

- **Video**: you should see your own desktop mirrored (it will recursively show
  the windows — that's expected on loopback).
- **Input**: moving the mouse / typing in the viewer drives the real cursor
  (both windows are on the same desktop, so expect feedback loops — move slowly).
- **Audio**: play any system sound; the viewer should play it back.
- **Clipboard**: copy text on one side; the clipboard button syncs it.
- **File transfer**: click **Upload**, pick a file; it lands in a `downloads/`
  folder next to the exe's working directory on the "host" instance.
- **Metrics HUD**: click the top-right status pill → live FPS, bitrate, RTT,
  jitter, capture/encode/decode latency.

## Tips & gotchas

- **Mouse/keyboard feedback loop**: because both apps share one physical
  desktop, injected input also affects the viewer window. For a cleaner input
  test, keep the viewer windowed and move the mouse deliberately, or test input
  last. Use the viewer's Disconnect (X) to stop.
- **Black video but input works** → WebView2/H.264 problem. Open the viewer's
  devtools console (if enabled) and look for `VideoDecoder` errors.
- **Host never appears in the list** → the signaling server may still be waking;
  wait ~10s and ensure both instances used *different* `REMOTE_CONTROL_DATA_DIR`
  values (same dir = same Device ID = collision).
- **Reset identities**: delete `%TEMP%\rc-host` and `%TEMP%\rc-viewer` to force
  fresh Device IDs.

## Logs

Run from a terminal (as above) to see `tracing` logs in that console — look for
`DXGI capture active`, `OpenH264 software encoder initialized`,
`Connected to signaling server as <DeviceId>`, and
`WASAPI loopback audio capture active`.
