# Phase 9: Authentication & Security

## Goal
Implement foundational security measures to prevent unauthorized access, replay attacks, and abusive signaling, including explicit host approval and strict permission models.

## Decisions Made
1. **Host Approval & Permissions**: 
   Introduced a `Permissions` bitflag/struct to explicitly track granted privileges (Screen, Mouse, Keyboard, Clipboard, Files, Audio, Terminal). Added a simulated `[y/N]` console prompt to the Rust backend before accepting incoming connections, ensuring the host is always aware of and approves remote access.
2. **Device Identity**: 
   Implemented `ed25519-dalek` for pure-Rust device identity generation (`security.rs`). The keys are stored locally to persistently identify returning clients.
3. **Signaling Server Protections**: 
   Added an IP-based token-bucket rate limiter to the Node.js signaling server to mitigate connection spam and DDoS attempts. Included `[AUDIT]` logging tags for traceability of all signaling payloads.
4. **Restricted Shell**: 
   Explicitly disabled Terminal and File access in the default `Permissions` configuration to adhere to the "no unrestricted remote shell access" requirement.

## Status
Security features successfully implemented. Code compiled via WSL cross-compilation and signaling server unit tests pass.
