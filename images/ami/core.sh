#!/usr/bin/env bash
set -e

echo "Updating apt repositories..."
sudo apt update

# echo "Installing curl..."
# sudo apt install -y curl

# DEB_URL="https://github.com/DefGuard/defguard/releases/download/v${PACKAGE_VERSION}/defguard-${PACKAGE_VERSION}-x86_64-unknown-linux-gnu.deb"
# echo "Downloading Defguard package from: $DEB_URL"
# sudo curl -fsSL -o /tmp/defguard-core.deb "$DEB_URL"

echo "Installing Defguard package..."
sudo dpkg -i /tmp/defguard-core.deb

echo "Cleaning up..."
sudo rm -f /tmp/defguard-core.deb

echo "Defguard installation completed successfully."
