#!/bin/bash
set -e

REPO="/mnt/d/Satyam (CE00172125)/learn/remote-control"
APP="$REPO/apps/desktop/src-tauri/Cargo.toml"
CARGO=~/.cargo/bin/cargo

cd "$REPO/apps/desktop"
npm run build

cd "$REPO"

CARGO_TARGET_DIR=/tmp/target-wsl "$CARGO" build --release --features custom-protocol \
  --target x86_64-pc-windows-gnu --manifest-path "$APP"
cp /tmp/target-wsl/x86_64-pc-windows-gnu/release/desktop.exe "$REPO/release_build/Remote Control.exe"

CARGO_TARGET_DIR=/tmp/target-linux "$CARGO" build --release --features custom-protocol \
  --target x86_64-unknown-linux-gnu --manifest-path "$APP"
cp /tmp/target-linux/x86_64-unknown-linux-gnu/release/desktop "$REPO/release_build/linux/remote-control"

echo "Done. Windows: release_build/Remote Control.exe | Linux: release_build/linux/remote-control"
