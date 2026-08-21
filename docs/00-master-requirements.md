# Master Project Requirements

PROJECT: Private Remote Workstation

I want to build a private, self-hosted remote desktop/workstation application for personal use.

The application is conceptually similar to AnyDesk/Parsec, but it is NOT intended to be a commercial product or a multi-tenant SaaS platform.

## Primary Purpose

I want to remotely access a high-spec Windows PC located on another network.

I should be able to:

- View the complete remote desktop.
- Control mouse and keyboard.
- Use VS Code and IDEs.
- Run backend/frontend development tools.
- Run Docker and databases.
- Run CPU/GPU-intensive applications.
- Use the remote machine as a high-performance workstation.
- Transfer files.
- Synchronize clipboard.
- Use audio.
- Support multiple monitors.
- Reconnect after temporary network failures.
- Monitor connection performance.
- Eventually achieve low-latency 1080p/60 FPS remote desktop.

## Test Environment

Machine 1:
My Windows PC.
Important: I DO NOT have administrator rights on this machine.

Machine 2:
My Ubuntu PC.
Machine 1 and Machine 2 are on the same LAN.

Machine 3:
My friend's Windows PC.
It is located on another network and will be used for Internet/P2P testing.

## Important Administrator Requirement

My Windows client machine does NOT have administrator privileges.

Therefore:

- The client application MUST work as a normal user process.
- Do not require administrator privileges for the client.
- Do not require Windows services on my machine.
- Do not require kernel drivers.
- Do not modify firewall rules automatically.
- Do not install system-wide components.
- Do not assume administrator access exists.
- Detect and clearly report operations that require elevation.

The first Windows remote-host implementation must also run as a normal user-level application.

DO NOT initially implement a Windows service.

The architecture should allow an optional privileged Windows service to be introduced later for:

- UAC/secure desktop interaction.
- Login-screen access.
- Unattended access.
- System-level operations.

But these are NOT MVP requirements.

## Platform Priority

Primary host: Windows.
Primary client: Windows.
Secondary testing platform: Ubuntu.

Linux support should be architected cleanly but does not need feature parity with Windows in the initial MVP.

## Browser

A browser client is NOT required.

Do NOT spend development effort building a browser-based remote client.

The primary client must be:
Tauri + React + TypeScript.

A browser client may be added in the future, but it is explicitly out of scope for the initial project.

## UI

Use the locally cloned MeetUI project as the primary visual/component reference.
Local path: ../meet-ui (cloned at the back of the project root, i.e. sibling to the remote-desktop/ project folder).
Upstream (for reference/updates only): <https://github.com/meetdarji006/meet-ui>

MeetUI is MIT licensed.

Use appropriate MeetUI components/design patterns for:

- Buttons
- Cards
- Dialogs
- Inputs
- Tabs
- Dropdowns
- Badges
- Toasts
- Settings
- Status indicators
- Animations
- Dashboard components

## Important

- Do not copy the entire MeetUI project.
- Do not introduce Next.js into the Tauri application just because MeetUI's showcase uses Next.js.
- Keep the application as React + TypeScript + Tauri.
- Reuse/adapt individual components where appropriate.
- Keep UI components in our own component layer.
- Avoid unnecessary dependencies.
- Preserve relevant MIT license requirements.
- Inspect the actual MeetUI components in ../meet-ui before using them.

## Technology

Desktop UI:
React
TypeScript
Tauri

Native engine:
Rust

Backend:
Node.js
TypeScript
WebSocket

P2P transport:
WebRTC

NAT traversal:

## STUN

TURN/coturn

Primary Windows APIs:
Windows Graphics Capture and/or Desktop Duplication API
Direct3D
Windows input APIs such as SendInput
Appropriate Windows audio APIs

Video:
Start with a practical codec.
Eventually support hardware encoding and decoding.

Hardware encoding targets:

## Nvidia NVENC

Intel Quick Sync
AMD hardware encoding where practical

## Database

Do NOT introduce PostgreSQL initially.
Do NOT introduce Redis initially.
Do NOT introduce Kafka/RabbitMQ initially.
Do NOT introduce Kubernetes.
Do NOT introduce microservices.

This is a personal application.

## Control Plane

Node.js should handle:

- Authentication
- Device registration
- Session creation
- Device discovery
- WebSocket signaling
- SDP exchange
- ICE candidate exchange
- Session lifecycle

## Media/data Plane

The Node.js server should NOT normally relay the remote desktop stream.

Prefer:
Client <──── WebRTC P2P ────> Host

TURN may relay traffic only when direct P2P connectivity fails.

## Security

Security must be designed from the beginning.

Requirements:

- Device identity
- Authentication
- Authorization
- Temporary session credentials
- Session expiration
- Explicit host approval
- Replay protection
- Message validation
- Rate limiting
- Secure signaling
- Encrypted WebRTC transport
- Permission model
- Audit logging

Permissions should eventually include:

## Screen

## Mouse

## Keyboard

## Clipboard

## Files

## Audio

## Terminal

Do NOT implement unrestricted remote shell access.
Do NOT implement hidden remote access.

The host must clearly indicate when a remote session is active.

## Remote Session Approval

Example:
"Satyam wants to connect."

Permissions:
Screen       ✓
Mouse        ✓
Keyboard     ✓
Clipboard    ✓
Files        ✗
Terminal     ✗

[Accept] [Reject]

## Performance Goals

Initial:
720p/30 FPS

## MVP

1080p/30 FPS

Target:
1080p/60 FPS

Future:
1440p/60 FPS
4K/60 FPS where hardware/network conditions allow.

Track:

- Capture latency
- Encode latency
- Network latency
- Decode latency
- Render latency
- End-to-end latency
- FPS
- Bitrate
- Packet loss
- Jitter
- RTT
- CPU
- GPU
- Memory

## Architecture Principles

- Modular architecture.
- Clear separation of concerns.
- SOLID where applicable.
- Dependency inversion.
- Platform-specific code isolated.
- Domain logic independent from UI.
- UI independent from native media pipeline.
- Native engine independent from React.
- Control plane separate from media plane.
- No unnecessary abstractions.
- No premature optimization.
- Measure before optimizing.
- Test every significant module.
- Do not rewrite working components without evidence.

## Development Rule

Implement ONE phase at a time.

After each phase:

1. Build.
2. Run tests.
3. Run integration tests where applicable.
4. Fix errors.
5. Document decisions.
6. Report exactly what works.
7. Report known limitations.

## 8. Stop

Do NOT automatically proceed to the next phase.

Never implement future phases unless explicitly instructed.
