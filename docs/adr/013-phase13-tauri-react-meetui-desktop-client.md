# Phase 13: Tauri + React + MeetUI Desktop Client

## Goal
Build the production-style desktop client using Tauri, React, and TypeScript. Establish a clean architecture where React handles purely presentation and control routing while the Rust engine manages performance-critical video/networking via Tauri IPC.

## Decisions Made
1. **Design System & Aesthetics**:
   Since the `../meet-ui` repository was inaccessible due to strict workspace boundaries, I re-implemented the MeetUI design language directly in the frontend (`apps/desktop/src/components/ui/`) using Tailwind CSS and `lucide-react`. I built clean, modern implementations for `Button`, `Card`, and `Badge`, leveraging `tailwind-merge` and `clsx` for composability.
2. **Tauri IPC Integration**:
   Generated `apps/desktop/src-tauri` and configured `tauri.conf.json`. Mapped frontend calls to Rust commands (`get_devices`, `connect_to_device`, `get_diagnostics`, `switch_monitor`), laying out the complete bridge for the `remote-core` pipeline.
3. **Application Views**:
   Built out the primary React views:
   - `Dashboard`: Lists available devices, statuses, and last-seen metrics.
   - `ActiveSession`: The full-screen desktop viewer mock, featuring a MeetUI-styled floating toolbar (Audio, File Transfer, Clipboard, Fullscreen, Monitor Switch).
   - `DiagnosticsPanel`: A real-time data overlay consuming metrics (FPS, Latency, RTT, Jitter) from the Rust backend.
4. **WSL Cargo Workspace**:
   Added `apps/desktop/src-tauri` to the root `Cargo.toml` members, ensuring we can build the entire Desktop App + Engine monolithic binary using `cargo build --target x86_64-pc-windows-gnu` within WSL.

## Status
Completed. UI structure built, React app wired up to Tauri backend, compiled successfully as a native Windows executable under the WSL GNU target.
