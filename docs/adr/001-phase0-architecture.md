# ADR 001: Phase 0 Architecture

## Status
Accepted

## Context
We need to establish a clean, modular repository architecture for a private, self-hosted remote desktop application (Tauri + React + Rust). It must support local Windows capture, P2P WebRTC networking, Node.js signaling, and run without requiring UAC/administrator privileges.

## Decision
We will use a modular structure utilizing a Cargo workspace for Rust crates, and separate directories for the desktop application (Tauri + React) and signaling server (Node.js).

### Monorepo Structure
```text
remote-control/
├── apps/
│   ├── desktop/ (Tauri + React + TypeScript MVP client/host)
│   └── signaling-server/ (Node.js + TypeScript + WebSocket)
├── crates/ (Rust core components)
│   ├── remote-core/ 
│   ├── screen-capture/
│   ├── video/
│   ├── input/
│   ├── audio/
│   ├── file-transfer/
│   ├── networking/
│   ├── webrtc/
│   ├── security/
│   └── platform-windows/
├── Cargo.toml (Workspace Root)
├── tests/ (Integration Tests)
└── docs/ (Architecture and Design Documents)
```

### Component Boundaries
- **UI (React/TypeScript)**: Presentation logic only, using Tauri IPC. We will adapt components from the `../meet-ui` sibling repo.
- **Native Engine (Rust/Tauri)**: Intermediary between UI and core logic.
- **Platform APIs (`platform-windows`)**: OS-level interactions.
- **Core Logic (`remote-core`)**: Orchestrates modules.
- **Networking & WebRTC (`networking`, `webrtc`)**: Manages P2P connections and signaling.
- **Media Plane (`video`, `audio`, `screen-capture`)**: Capture, encode, decode, and render pipelines.
- **Signaling (`signaling-server`)**: Node.js control plane (device discovery, ICE exchange). Does NOT relay media traffic.
- **Security (`security`)**: Cryptographic primitives, payload validation, and access control models.

## Consequences
- The separation of UI and Rust logic prevents tight coupling.
- The use of workspaces allows us to develop and test each Rust component in isolation.
- UAC privileges will not be requested by default.
