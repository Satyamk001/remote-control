# ADR 015: Phase 15 - Packaging and Distribution

## Status
Accepted

## Context
After successfully implementing the Remote Control application up through Phase 14 (Host Agent UI), the application needed to be packaged for distribution. 

Tauri supports native application bundlers (producing MSI or NSIS installers on Windows), which generally require Windows-native toolchains (WiX, NSIS). However, our host machine is strictly constrained to WSL cross-compilation (`x86_64-pc-windows-gnu`) due to the absence of the MSVC build tools on the host. 

## Decision
We decided to adopt a **Portable ZIP Architecture** instead of relying on standard installers. 

1. We explicitly disabled Tauri's native bundling (`"targets": []`) in `tauri.conf.json`.
2. We wrote a custom shell script (`package_release.sh`) that cross-compiles the heavily optimized release executable in WSL.
3. The script manually extracts the compiled `desktop.exe` and its core dependency (`WebView2Loader.dll`) into a portable `release_build` directory.
4. It compresses the directory into a standalone `.zip` distribution.

## Consequences
**Positive**: 
- We completely circumvented the MSVC/WiX requirement.
- The build pipeline stays fully functional in our restricted environment.
- Portable applications are highly preferred by power users.

**Negative**:
- Users don't get an automatic uninstaller or Start Menu shortcuts.
- There is no automatic updater via Tauri's native updater infrastructure without further work.
