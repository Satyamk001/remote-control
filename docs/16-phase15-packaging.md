# Phase 15 — Packaging
Implement Phase 15 only.

Package the application for personal use.

Targets:
Windows client
Windows host agent
Ubuntu client/agent where supported

Windows packaging must NOT require administrator privileges for the normal client application.

Do not install a Windows service.

Provide:
- executable/package
- configuration storage
- logs
- version information
- clean uninstall
- configuration reset
- crash logging
- update strategy documentation

If any feature requires administrator privileges, document it explicitly.

Do not silently elevate.

Test installation and execution on:
1. My Windows PC with NO administrator rights.
2. Ubuntu PC.
3. Friend's Windows PC.

STOP after Phase 15.
