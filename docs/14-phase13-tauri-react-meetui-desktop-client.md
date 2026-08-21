# Phase 13 — Tauri + React + MeetUI Desktop Client
Implement Phase 13 only.

Build the production-style desktop client using:
Tauri
React
TypeScript

Use the locally cloned MeetUI project as the primary UI design/component reference.
Local path: ../meet-ui (sibling to the remote-desktop/ project folder).
Upstream (for reference/updates only): https://github.com/meetdarji006/meet-ui

Do not add browser support.
Do not introduce Next.js.
Do not move the video pipeline into React.

The Rust engine remains responsible for performance-sensitive operations.

React communicates with Rust through a clean Tauri IPC boundary.

UI sections:
Dashboard
Devices
Connection
Active Session
Settings
Security
Diagnostics

Dashboard:
- device list
- online/offline state
- last seen
- connect button

Connection:
- device ID
- connection state
- host approval
- permissions

Active session:
- remote desktop
- toolbar
- fullscreen
- resolution
- FPS
- clipboard
- file transfer
- audio
- monitor selection

Diagnostics:
- FPS
- bitrate
- RTT
- packet loss
- jitter
- codec
- resolution
- P2P/TURN
- CPU
- GPU
- latency breakdown

Use MeetUI-inspired/reused components for:
Button
Card
Dialog
Tabs
Badge
Tooltip
Toast
Input
Dropdown
Progress
Status indicators

Keep reusable components under:
src/components/ui/

Keep application-specific components under:
src/components/devices/
src/components/session/
src/components/settings/
src/components/diagnostics/

Do not copy the entire MeetUI repository.

Do not introduce unnecessary dependencies.

Ensure the Tauri application works as a NORMAL USER PROCESS on my Windows PC.

Do not require administrator privileges.

STOP after Phase 13.
