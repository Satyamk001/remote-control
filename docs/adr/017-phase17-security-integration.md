# ADR 017: Phase 17 - Security Identity & Integration

## Status
Accepted

## Context
During the initial UI build (Phase 13/14), the Tauri backend used mocked data for the Local Device ID (e.g. returning `"SATYAM-HOST-9942"`). However, Phase 9 previously established a cryptographic identity model using `ed25519-dalek` to ensure devices could be securely authenticated across the internet without central passwords. We need to integrate this true identity engine into the Tauri desktop shell so the UI represents the actual cryptographic state of the host.

## Decision
We integrated `remote_core::security::Identity` directly into the Tauri lifecycle:
1. **Application State**: We modified the Tauri `setup` hook to automatically resolve the OS-specific application data directory (`app.path_resolver().app_data_dir()`).
2. **Key Persistence**: We pass this directory to the `Identity::load_or_generate` constructor. If `identity.key` exists, it loads the previously generated `ed25519` keypair. Otherwise, it generates a new CSPRNG key and saves it to disk.
3. **State Management**: The generated `Identity` is injected into Tauri's managed state via `app.manage()`.
4. **IPC Bridge**: The `get_local_device_id` IPC command retrieves the `VerifyingKey` (the public key) from the managed state, encodes it to an uppercase Hexadecimal string using the `hex` crate, and returns it to the React UI as the globally unique Device ID.

## Consequences
**Positive:**
- The Device ID shown in the Host Agent Panel is now a true cryptographic public key.
- Identity persists across application restarts.
- The UI is moving away from mocks and properly interfacing with the Rust engine.

**Negative:**
- The hex-encoded Ed25519 public key is 64 characters long, which may require UI truncation or formatting (e.g., grouping by 4 characters) to be user-friendly in the future.
