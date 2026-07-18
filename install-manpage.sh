#!/usr/bin/env bash
# arctgz man page installer – simple, safe, no auto‑sudo
set -euo pipefail

MAN_SECTION=7
MAN_NAME="arctgz.${MAN_SECTION}"
INSTALL_DIR="/usr/local/share/man/man${MAN_SECTION}"
RAW_URL="https://raw.githubusercontent.com/mroczect/arctgz/master/manual/arctgz.7"

# Cek apakah dijalankan sebagai root
if [ "$(id -u)" -ne 0 ]; then
    echo "This script requires root privileges."
    echo "Please re-run it with sudo:"
    echo
    echo "  curl -fsSL ${RAW_URL%/*}/install-manpage.sh | sudo bash"
    echo
    exit 1
fi

# Uninstall mode
if [ "${1:-}" = "uninstall" ]; then
    rm -f "${INSTALL_DIR}/${MAN_NAME}" "${INSTALL_DIR}/${MAN_NAME}.gz"
    mandb -q "${INSTALL_DIR}" 2>/dev/null || true
    echo "Man page uninstalled."
    exit 0
fi

echo "Downloading arctgz(7) man page..."
mkdir -p "${INSTALL_DIR}"

if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$RAW_URL" -o "${INSTALL_DIR}/${MAN_NAME}"
elif command -v wget >/dev/null 2>&1; then
    wget -q "$RAW_URL" -O "${INSTALL_DIR}/${MAN_NAME}"
else
    echo "Error: curl or wget required."
    exit 1
fi

chmod 644 "${INSTALL_DIR}/${MAN_NAME}"
echo "Man page installed to ${INSTALL_DIR}/${MAN_NAME}"

if command -v mandb >/dev/null 2>&1; then
    mandb -q "${INSTALL_DIR}" 2>/dev/null || true
    echo "Man database updated."
fi

echo
echo "Try 'man 7 arctgz' to view the manual."
