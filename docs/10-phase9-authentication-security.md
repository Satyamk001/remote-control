# Phase 9 — Authentication + Security
Implement Phase 9 only.

Perform a security-focused implementation.

The application controls another computer, so security is critical.

Threats to address:
- unauthorized access
- session hijacking
- guessed session IDs
- replay attacks
- malicious signaling messages
- input injection
- token theft
- file path traversal
- unauthorized file access
- privilege escalation

Implement:
1. Device identity.
2. Public/private key pair per installation.
3. Authentication.
4. Temporary session credentials.
5. Session expiration.
6. Explicit host approval.
7. Authorization.
8. Message validation.
9. Signaling rate limiting.
10. Audit logging.
11. Permission model.

Permissions:
## Screen
## Mouse
## Keyboard
## Clipboard
## Files
## Audio
## Terminal

Do NOT implement arbitrary remote shell execution.

Host approval UI:

"Satyam wants to connect."

Screen       ✓
Mouse        ✓
Keyboard     ✓
Clipboard    ✓
Files        ✗
Audio        ✗
Terminal     ✗

[Accept] [Reject]

The host must visibly indicate an active remote session.

No hidden remote access.

Do not bypass Windows security boundaries.

Add security tests.

STOP after Phase 9.
