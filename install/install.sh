#!/usr/bin/env bash
set -euo pipefail

# Installer script for toold and toolctl
# Must be executed as root

if [[ "${EUID}" -ne 0 ]]; then
    echo "Error: install.sh must be executed with root privileges." >&2
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "==> Building toold and toolctl in release mode..."
cargo build --release --manifest-path "${ROOT_DIR}/Cargo.toml"

echo "==> Installing binaries to /usr/local/bin..."
install -m 0755 "${ROOT_DIR}/target/release/toold" /usr/local/bin/toold
install -m 0755 "${ROOT_DIR}/target/release/toolctl" /usr/local/bin/toolctl

echo "==> Setting up systemd sysusers and tmpfiles..."
if [[ -f "${ROOT_DIR}/sysusers.d/toold.conf" ]]; then
    install -m 0644 "${ROOT_DIR}/sysusers.d/toold.conf" /usr/lib/sysusers.d/toold.conf
    systemd-sysusers /usr/lib/sysusers.d/toold.conf || true
fi

if [[ -f "${ROOT_DIR}/tmpfiles.d/toold.conf" ]]; then
    install -m 0644 "${ROOT_DIR}/tmpfiles.d/toold.conf" /usr/lib/tmpfiles.d/toold.conf
    systemd-tmpfiles --create /usr/lib/tmpfiles.d/toold.conf || true
fi

echo "==> Installing systemd units..."
install -m 0644 "${ROOT_DIR}/systemd/toold.socket" /usr/lib/systemd/system/toold.socket
install -m 0644 "${ROOT_DIR}/systemd/toold.service" /usr/lib/systemd/system/toold.service

echo "==> Reloading systemd daemon..."
systemctl daemon-reload
systemctl enable --now toold.socket

echo "==> toold socket activated successfully."
echo "Verify status: systemctl status toold.socket"
