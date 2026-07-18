#!/usr/bin/env bash
# arctgz man page installer – super simple version
set -euo pipefail

MAN_SECTION=7
MAN_NAME="arctgz.${MAN_SECTION}"
INSTALL_DIR="/usr/local/share/man/man${MAN_SECTION}"
RAW_URL="https://raw.githubusercontent.com/mroczect/arctgz/master/manual/arctgz.7"

if [ "$(id -u)" -ne 0 ]; then
    echo "Requires root. Re-executing with sudo..."
    exec sudo bash "$0" "$@"
fi

if [ "${1:-}" = "uninstall" ]; then
    rm -f "${INSTALL_DIR}/${MAN_NAME}" "${INSTALL_DIR}/${MAN_NAME}.gz"
    mandb -q "${INSTALL_DIR}" 2>/dev/null || true
    echo "Uninstalled."
    exit 0
fi

echo "Downloading man page..."
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
if command -v mandb >/dev/null 2>&1; then
    mandb -q "${INSTALL_DIR}" 2>/dev/null || true
fi
echo "Man page installed. Try 'man 7 arctgz'."