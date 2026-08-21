# Phase 10: Clipboard & File Transfer

## Goal
Implement a secure, chunked, and resumable file transfer protocol and bidirectional clipboard synchronization over the existing WebRTC DataChannel, completely bound by the host's explicit permission model.

## Decisions Made
1. **File Transfer State Machine**: 
   Implemented `FileReceiver` (`transfer.rs`) to manage chunked uploads. Each chunk is verified with CRC32 hashes. Out-of-order sequence numbers are rejected to ensure integrity.
2. **Path Traversal Protection**: 
   Filenames sent by the client are strictly sanitized. Any strings containing `/`, `\`, or `..` are immediately rejected with a `PathTraversal` error, ensuring the client cannot write outside the designated `downloads` directory.
3. **Clipboard Management**: 
   Integrated the `arboard` crate (`clipboard.rs`) to securely interact with the host OS clipboard. 
4. **Data Multiplexing**: 
   Added `ClipboardData` and `FileTransfer` message variants to the core `ProtocolMessage` enum, meaning all data (Video, Input, Files, Clipboard) flows efficiently over the same WebRTC peer connection without requiring extra ports.
5. **Security Enforcement**: 
   The host's `streaming.rs` pipeline explicitly prompts the user to grant `clipboard` and `files` permissions on connection. If denied, the background receiver task will intercept and drop any incoming file or clipboard data with a warning.

## Status
Completed. Rust codebase cross-compiled successfully with `arboard` and `crc32fast` dependencies.
