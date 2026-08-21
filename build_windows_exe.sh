#!/bin/bash
set -e

# Load nvm
export NVM_DIR="$HOME/.nvm"
if [ -s "$NVM_DIR/nvm.sh" ]; then
    source "$NVM_DIR/nvm.sh"
fi
nvm use 22

# Load cargo
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
fi

# Ensure windows target is added
rustup target add x86_64-pc-windows-gnu || true

echo "Updating source files in native Linux cache..."
mkdir -p ~/remote-control-build/apps/desktop/src/components/devices ~/remote-control-build/apps/desktop/src/components/host ~/remote-control-build/apps/desktop/src/components/session ~/remote-control-build/apps/desktop/src/components/ui
cp "/mnt/d/Satyam (CE00172125)/learn/remote-control/apps/desktop/src/App.tsx" ~/remote-control-build/apps/desktop/src/
cp "/mnt/d/Satyam (CE00172125)/learn/remote-control/apps/desktop/src/components/ui/hyper-text.tsx" ~/remote-control-build/apps/desktop/src/components/ui/
cp "/mnt/d/Satyam (CE00172125)/learn/remote-control/apps/desktop/src/components/devices/Dashboard.tsx" ~/remote-control-build/apps/desktop/src/components/devices/
cp "/mnt/d/Satyam (CE00172125)/learn/remote-control/apps/desktop/src/components/host/HostAgentPanel.tsx" ~/remote-control-build/apps/desktop/src/components/host/
cp "/mnt/d/Satyam (CE00172125)/learn/remote-control/apps/desktop/src/components/session/ActiveSession.tsx" ~/remote-control-build/apps/desktop/src/components/session/
cp "/mnt/d/Satyam (CE00172125)/learn/remote-control/crates/networking/Cargo.toml" ~/remote-control-build/crates/networking/
cp "/mnt/d/Satyam (CE00172125)/learn/remote-control/crates/networking/src/signaling.rs" ~/remote-control-build/crates/networking/src/
cp "/mnt/d/Satyam (CE00172125)/learn/remote-control/apps/desktop/src-tauri/src/main.rs" ~/remote-control-build/apps/desktop/src-tauri/src/

echo "Building frontend & Windows binary..."
cd ~/remote-control-build/apps/desktop
npx tauri build --target x86_64-pc-windows-gnu

echo "Copying Windows executable..."
mkdir -p "/mnt/d/Satyam (CE00172125)/learn/remote-control/release_build"
cp ~/remote-control-build/target/x86_64-pc-windows-gnu/release/desktop.exe "/mnt/d/Satyam (CE00172125)/learn/remote-control/release_build/Remote Control.exe"

echo "Windows .exe build complete!"
