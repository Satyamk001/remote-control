# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

React, TypeScript, Tauri, Tailwind (via MeetUI components)

## Users

Personal use by the developer. Accessing a high-spec Windows PC remotely from a restricted laptop on another network.

## Product Purpose

Private, self-hosted, low-latency 1080p/60 FPS remote desktop access for development (VS Code, backend/frontend tools, Docker) without commercial/SaaS bloat.

## Positioning

A completely self-hosted, non-commercial alternative to AnyDesk/Parsec that works natively without requiring administrator privileges on the client machine.

## Operating Context

Connecting over the internet or LAN to a powerful workstation from a client machine that lacks admin rights. Relies on WebRTC for P2P transport and a centralized WebSocket signaling server for discovery.

## Capabilities and Constraints

- Must NOT require administrator privileges on the client machine.
- No Windows services or kernel drivers for the client.
- Uses WebRTC for P2P transport and Node.js for WebSocket signaling.
- Hardware encoding (NVENC/QuickSync) targets for future phases; software encoding for MVP.
- Do NOT introduce Postgres, Redis, Kafka, Kubernetes, or microservices.

## Brand Commitments

- UI leverages the "MeetUI" design system (`../meet-ui`).
- Clean, modern desktop application look.

## Evidence on Hand

- Master requirements document (`docs/00-master-requirements.md`).
- Final architecture specification (`docs/final-architecture.md`).
- Existing React/Tauri frontend and Rust WebRTC/Signaling backend.

## Product Principles

1. Self-hosted and entirely private (no commercial telemetry or SaaS dependencies).
2. Works seamlessly without elevated privileges on the client.
3. Modular and cleanly architected (distinct Presentation, Domain, Infrastructure layers).
4. P2P-first media and data transport via WebRTC.
