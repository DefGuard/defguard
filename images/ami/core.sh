#!/usr/bin/env bash
set -e

echo "Updating apt repositories..."
sudo apt update

echo "Installing Defguard package..."
sudo dpkg -i /tmp/defguard-core.deb

echo "Cleaning up..."
sudo rm -f /tmp/defguard-core.deb

echo "Defguard installation completed successfully."
