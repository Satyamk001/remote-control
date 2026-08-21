# Architecture Overview

This document outlines the architecture for the Private Remote Workstation project.

## High-Level Architecture

The application follows a modular structure separated into two main domains:

1. **Host/Client Desktop Application**: A Tauri + React application that functions as both the remote host and the local client.
2. **Signaling Server**: A Node.js control plane to facilitate P2P WebRTC connectivity between devices.

### 1. Desktop Application (Tauri + React + Rust)

- **UI (React/TypeScript)**: The frontend presentation layer responsible for session management, settings, device discovery, and viewing the remote stream. It communicates with the backend native engine entirely through Tauri IPC.
- **Native Engine (Rust)**: The core logic orchestrator.
  - Controls signaling communication.
  - Manages WebRTC sessions.
  - Bridges native platform APIs with the UI.
- **Platform Modules (`crates/platform-windows`)**: Contains OS-specific implementations, preventing the core logic from being tightly coupled to Windows UAC, capture, or input APIs.
- **Media Plane**: Captures the screen (Windows Graphics Capture/Desktop Duplication), encodes video (e.g., NVENC), and streams it over WebRTC.

### 2. Signaling Server (Node.js)

- Manages authentication, identity, and session authorization.
- Exchanges ICE candidates and SDP payloads between the host and client.
- Operates strictly as a control plane; it does not relay media streams (WebRTC is strictly P2P).

## Security Model

- Uses mutual device authentication.
- Explicit host approval required for all sessions.
- Granular permissions (Screen, Mouse, Keyboard, Clipboard).
- The client application must function without UAC/Administrator privileges. UAC handling and secure desktop access are explicitly outside the scope of Phase 0/MVP and may be introduced later via an optional Windows service.

## Core Libraries (Rust Workspaces)

The Rust engine is heavily modularized into workspaces:

- `remote-core`: Domain logic and session orchestration.
- `screen-capture`: Native screen and window capture.
- `video`: Video stream encode/decode pipeline.
- `input`: Mouse/keyboard event simulation and hooking.
- `audio`: System audio capture.
- `file-transfer`: Clipboard sync and file transmission.
- `networking`: General socket connections and signaling client.
- `webrtc`: ICE, STUN, and P2P session handling.
- `security`: Cryptography, session validation.
- `platform-windows`: Abstract implementations specifically for Windows desktop.

This structure guarantees clean dependency boundaries and testability for each sub-component.
