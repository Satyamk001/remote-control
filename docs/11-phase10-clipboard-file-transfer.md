# Phase 10 — Clipboard + File Transfer
Implement Phase 10 only.

Add secure clipboard and file transfer.

Clipboard:
- text initially
- explicit permission
- no silent sensitive clipboard synchronization

File transfer:
Client → Host
Host → Client

Requirements:
- explicit authorization
- chunking
- sequence numbers
- checksum
- progress
- cancellation
- retry
- resume where practical
- integrity verification
- filename sanitization
- path traversal protection
- safe temporary files
- no arbitrary absolute-path writes
- size limits

Implement over authenticated WebRTC DataChannels.

Test:
- small file
- large file
- corrupted chunk
- duplicate chunk
- interrupted transfer
- resumed transfer
- invalid filename
- path traversal attempt
- insufficient permission
- cancelled transfer

STOP after Phase 10.
