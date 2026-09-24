#!/usr/bin/env bash
set -euo pipefail

# Uninstaller script for toold and toolctl
# Must be executed as root

if [[ "${EUID}" -ne 0 ]]; then
    echo "Error: uninstall.sh must be executed with root privileges." >&2
    exit 1
fi

echo "==> Stopping and disabling toold service and socket..."
systemctl disable --now toold.service toold.socket 2>/dev/null || true

echo "==> Removing systemd unit files..."
rm -f /usr/lib/systemd/system/toold.service
rm -f /usr/lib/systemd/system/toold.socket
rm -f /usr/lib/sysusers.d/toold.conf
rm -f /usr/lib/tmpfiles.d/toold.conf

echo "==> Reloading systemd daemon..."
systemctl daemon-reload

echo "==> Removing installed binaries..."
rm -f /usr/local/bin/toold
rm -f /usr/local/bin/toolctl

echo "==> Cleaning up runtime sockets..."
rm -rf /run/syntrop/io.syntrop.Tool1

echo "==> toold uninstallation completed."
echo "Note: Historical rollback journals in /var/lib/toold are preserved."
echo "To remove all historical state, run: rm -rf /var/lib/toold"
