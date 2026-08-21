#!/bin/bash
set -e

# Load nvm
export NVM_DIR="$HOME/.nvm"
if [ -s "$NVM_DIR/nvm.sh" ]; then
    source "$NVM_DIR/nvm.sh"
else
    echo "NVM not found!"
    exit 1
fi

# Use Node 22
nvm use 22

# Load cargo
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
else
    echo "Cargo not found!"
    exit 1
fi

echo "Copying repository..."
rm -rf ~/remote-control-build
cp -r "/mnt/d/Satyam (CE00172125)/learn/remote-control" ~/remote-control-build

echo "Cleaning target directories..."
cd ~/remote-control-build
rm -rf target apps/desktop/src-tauri/target target_linux apps/desktop/src-tauri/target_linux

echo "Building..."
cd apps/desktop
npm install
npx tauri build

echo "Copying .deb..."
mkdir -p "/mnt/d/Satyam (CE00172125)/learn/remote-control/apps/desktop/release_build/"
cp src-tauri/target/release/bundle/deb/*.deb "/mnt/d/Satyam (CE00172125)/learn/remote-control/apps/desktop/release_build/"
echo "Success!"
