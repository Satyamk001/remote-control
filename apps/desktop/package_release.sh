#!/bin/bash
set -e

echo "======================================"
echo " Packaging Remote Control Portable Release"
echo "======================================"

APP_DIR="/mnt/d/Satyam (CE00172125)/learn/remote-control/apps/desktop"
WORKSPACE_DIR="/mnt/d/Satyam (CE00172125)/learn/remote-control"
TARGET_DIR="/tmp/target-wsl/x86_64-pc-windows-gnu/release"
RELEASE_ZIP_DIR="${WORKSPACE_DIR}/release_build"

echo "1. Building React Frontend (Windows host side)..."
cd "${APP_DIR}"
# Build frontend via NPM
npm run build

echo "2. Building Rust Backend (Release Mode) in WSL..."
cd "${WORKSPACE_DIR}"
# Build Tauri application targeting windows-gnu in release mode
CARGO_TARGET_DIR=/tmp/target-wsl ~/.cargo/bin/cargo build --release --target x86_64-pc-windows-gnu --manifest-path "${APP_DIR}/src-tauri/Cargo.toml"

echo "3. Assembling Portable Release..."
mkdir -p "${RELEASE_ZIP_DIR}"
rm -rf "${RELEASE_ZIP_DIR}/*"

# Copy the Executable
echo "Copying Remote Control.exe..."
cp "${TARGET_DIR}/desktop.exe" "${RELEASE_ZIP_DIR}/Remote Control.exe"

# Copy the WebView2Loader
echo "Copying WebView2Loader.dll..."
cp "${TARGET_DIR}/build/"*"/out/x64/WebView2Loader.dll" "${RELEASE_ZIP_DIR}/" || echo "WebView2Loader not found, assuming static link."

echo "4. Zipping Archive..."
cd "${WORKSPACE_DIR}"
zip -r RemoteControl-Portable-v0.1.0.zip release_build/

echo "======================================"
echo " Packaging Complete! Archive: RemoteControl-Portable-v0.1.0.zip"
echo "======================================"
