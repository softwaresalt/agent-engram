#!/bin/sh
# install.sh — one-liner installer for engram
# Usage: curl -fsSL https://raw.githubusercontent.com/softwaresalt/agent-engram/main/scripts/install.sh | sh
set -eu

REPO="softwaresalt/agent-engram"
INSTALL_DIR="${ENGRAM_INSTALL_DIR:-$HOME/.engram/bin}"

main() {
    detect_platform
    fetch_latest_tag
    download_archive
    verify_archive
    extract_binary
    update_path
    print_success
}

detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "${OS}" in
        Linux)
            case "${ARCH}" in
                x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
                *)
                    echo "Error: unsupported Linux architecture: ${ARCH}" >&2
                    echo "Supported: x86_64" >&2
                    exit 1
                    ;;
            esac
            ;;
        Darwin)
            case "${ARCH}" in
                arm64) TARGET="aarch64-apple-darwin" ;;
                *)
                    echo "Error: unsupported macOS architecture: ${ARCH}" >&2
                    echo "Supported: arm64 (Apple Silicon)" >&2
                    exit 1
                    ;;
            esac
            ;;
        *)
            echo "Error: unsupported operating system: ${OS}" >&2
            echo "Supported: Linux, macOS" >&2
            exit 1
            ;;
    esac

    EXT="tar.gz"
    echo "Detected platform: ${OS} ${ARCH} (${TARGET})"
}

fetch_latest_tag() {
    echo "Fetching latest release..."
    TAG="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' \
        | head -1 \
        | sed 's/.*"tag_name": *"//;s/".*//')"

    if [ -z "${TAG}" ]; then
        echo "Error: could not determine latest release tag" >&2
        echo "Check your network connection and try again." >&2
        exit 1
    fi

    echo "Latest release: ${TAG}"
}

download_archive() {
    ARCHIVE_NAME="engram-${TAG}-${TARGET}.${EXT}"
    URL="https://github.com/${REPO}/releases/download/${TAG}/${ARCHIVE_NAME}"
    TMP_DIR="$(mktemp -d)"
    ARCHIVE_PATH="${TMP_DIR}/${ARCHIVE_NAME}"

    echo "Downloading ${ARCHIVE_NAME}..."
    if ! curl -fSL --progress-bar -o "${ARCHIVE_PATH}" "${URL}"; then
        echo "Error: download failed" >&2
        echo "URL: ${URL}" >&2
        rm -rf "${TMP_DIR}"
        exit 1
    fi

    # Download SHA256 checksum sidecar if available
    CHECKSUM_PATH="${ARCHIVE_PATH}.sha256"
    if curl -fsSL -o "${CHECKSUM_PATH}" "${URL}.sha256" 2>/dev/null; then
        HAVE_CHECKSUM=1
    else
        HAVE_CHECKSUM=0
        echo "SHA256 checksum not available for this release — skipping verification."
    fi
}

verify_archive() {
    # Verify SHA256 checksum when available
    if [ "${HAVE_CHECKSUM}" = "1" ]; then
        EXPECTED="$(cat "${CHECKSUM_PATH}" | awk '{print $1}')"
        if command -v sha256sum > /dev/null 2>&1; then
            ACTUAL="$(sha256sum "${ARCHIVE_PATH}" | awk '{print $1}')"
        elif command -v shasum > /dev/null 2>&1; then
            ACTUAL="$(shasum -a 256 "${ARCHIVE_PATH}" | awk '{print $1}')"
        else
            echo "Warning: no sha256sum or shasum found — skipping checksum verification." >&2
            ACTUAL="${EXPECTED}"
        fi

        if [ "${EXPECTED}" != "${ACTUAL}" ]; then
            echo "Error: SHA256 checksum mismatch" >&2
            echo "  Expected: ${EXPECTED}" >&2
            echo "  Got:      ${ACTUAL}" >&2
            rm -rf "${TMP_DIR}"
            exit 1
        fi
        echo "SHA256 checksum verified."
    fi

    # Verify tar can list the archive contents
    if ! tar tzf "${ARCHIVE_PATH}" > /dev/null 2>&1; then
        echo "Error: archive integrity check failed — not a valid tar.gz" >&2
        rm -rf "${TMP_DIR}"
        exit 1
    fi

    echo "Archive verified."
}

extract_binary() {
    mkdir -p "${INSTALL_DIR}"
    tar xzf "${ARCHIVE_PATH}" -C "${TMP_DIR}"

    if [ ! -f "${TMP_DIR}/engram" ]; then
        echo "Error: engram binary not found in archive" >&2
        rm -rf "${TMP_DIR}"
        exit 1
    fi

    mv "${TMP_DIR}/engram" "${INSTALL_DIR}/engram"
    chmod +x "${INSTALL_DIR}/engram"
    rm -rf "${TMP_DIR}"

    echo "Installed engram to ${INSTALL_DIR}/engram"
}

update_path() {
    # Check if already on PATH
    case ":${PATH}:" in
        *":${INSTALL_DIR}:"*)
            return
            ;;
    esac

    PROFILE=""
    SHELL_NAME="$(basename "${SHELL:-/bin/sh}")"

    case "${SHELL_NAME}" in
        zsh)  PROFILE="$HOME/.zshrc" ;;
        bash) PROFILE="$HOME/.bashrc" ;;
        fish) PROFILE="$HOME/.config/fish/config.fish" ;;
        *)    PROFILE="$HOME/.profile" ;;
    esac

    EXPORT_LINE="export PATH=\"${INSTALL_DIR}:\$PATH\""

    if [ -n "${PROFILE}" ] && [ -f "${PROFILE}" ]; then
        if grep -qF "${INSTALL_DIR}" "${PROFILE}" 2>/dev/null; then
            return
        fi
    fi

    if [ -n "${PROFILE}" ]; then
        mkdir -p "$(dirname "${PROFILE}")"
        echo "" >> "${PROFILE}"
        echo "# Added by engram installer" >> "${PROFILE}"
        if [ "${SHELL_NAME}" = "fish" ]; then
            echo "set -gx PATH \"${INSTALL_DIR}\" \$PATH" >> "${PROFILE}"
        else
            echo "${EXPORT_LINE}" >> "${PROFILE}"
        fi
        echo "Updated ${PROFILE} with PATH entry."
        echo "Run 'source ${PROFILE}' or open a new terminal to use engram."
    fi
}

print_success() {
    echo ""
    echo "engram ${TAG} installed successfully!"
    echo ""
    echo "Next steps:"
    echo "  engram install    # Initialize a workspace"
    echo "  engram sync       # Build the first index"
    echo ""
}

main
