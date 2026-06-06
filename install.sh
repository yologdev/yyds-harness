#!/bin/sh
set -eu

REPO="yologdev/yyds-harness"
ARCHIVE_PREFIX="yyds-harness"
INSTALL_DIR="$HOME/.yoyo/bin"

main() {
    os=$(uname -s)
    arch=$(uname -m)

    case "$os" in
        Linux)  target_os="unknown-linux-gnu" ;;
        Darwin) target_os="apple-darwin" ;;
        *)
            echo "Unsupported OS: $os. Falling back to cargo install."
            cargo_fallback
            return
            ;;
    esac

    case "$arch" in
        x86_64)  target_arch="x86_64" ;;
        aarch64|arm64) target_arch="aarch64" ;;
        *)
            echo "Unsupported architecture: $arch. Falling back to cargo install."
            cargo_fallback
            return
            ;;
    esac

    # Linux only has x86_64 builds for now
    if [ "$os" = "Linux" ] && [ "$target_arch" = "aarch64" ]; then
        echo "No pre-built binary for Linux aarch64. Falling back to cargo install."
        cargo_fallback
        return
    fi

    target="${target_arch}-${target_os}"

    echo "Detected platform: ${target}"

    # Get latest release tag
    if command -v curl >/dev/null 2>&1; then
        api_response=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest") || {
            echo "Error: failed to fetch release info from GitHub API."
            echo "You may be rate-limited. Try building from source instead."
            exit 1
        }
    elif command -v wget >/dev/null 2>&1; then
        api_response=$(wget -qO- "https://api.github.com/repos/${REPO}/releases/latest") || {
            echo "Error: failed to fetch release info from GitHub API."
            echo "You may be rate-limited. Try building from source instead."
            exit 1
        }
    else
        echo "Error: curl or wget is required."
        exit 1
    fi

    version=$(echo "$api_response" | grep '"tag_name"' | sed 's/.*"tag_name": *"//;s/".*//')

    if [ -z "$version" ]; then
        echo "Error: could not determine latest release version."
        echo "Try building from source instead."
        exit 1
    fi

    echo "Installing Yoyo DS Harness ${version}..."

    tarball="${ARCHIVE_PREFIX}-${version}-${target}.tar.gz"
    url="https://github.com/${REPO}/releases/download/${version}/${tarball}"
    checksum_url="${url}.sha256"

    # Download to temp directory
    tmpdir=$(mktemp -d) || {
        echo "Error: could not create temporary directory."
        exit 1
    }
    trap 'rm -rf "$tmpdir"' EXIT

    echo "Downloading ${url}..."
    if command -v curl >/dev/null 2>&1; then
        if ! curl -fSL "$url" -o "${tmpdir}/${tarball}"; then
            echo "Error: failed to download ${tarball}"
            echo "The release may not exist yet. Try building from source instead."
            exit 1
        fi
        curl -fsSL "$checksum_url" -o "${tmpdir}/${tarball}.sha256" 2>/dev/null || true
    else
        if ! wget -q "$url" -O "${tmpdir}/${tarball}"; then
            echo "Error: failed to download ${tarball}"
            echo "The release may not exist yet. Try building from source instead."
            exit 1
        fi
        wget -q "$checksum_url" -O "${tmpdir}/${tarball}.sha256" 2>/dev/null || true
    fi

    # Verify checksum if available
    if [ -f "${tmpdir}/${tarball}.sha256" ]; then
        (
            cd "$tmpdir"
            if command -v sha256sum >/dev/null 2>&1; then
                sha256sum -c "${tarball}.sha256" >/dev/null 2>&1
            elif command -v shasum >/dev/null 2>&1; then
                shasum -a 256 -c "${tarball}.sha256" >/dev/null 2>&1
            else
                exit 0
            fi
        ) || {
            echo "Error: checksum verification failed. The download may be corrupted."
            exit 1
        }
        echo "Checksum verified."
    fi

    # Extract
    if ! tar xzf "${tmpdir}/${tarball}" -C "$tmpdir"; then
        echo "Error: failed to extract ${tarball}. The download may be corrupted."
        exit 1
    fi

    if [ ! -f "${tmpdir}/yyds" ]; then
        echo "Error: binary 'yyds' not found in archive."
        echo "Please report this: https://github.com/${REPO}/issues"
        exit 1
    fi

    # Install
    mkdir -p "$INSTALL_DIR"
    mv "${tmpdir}/yyds" "${INSTALL_DIR}/yyds"
    chmod +x "${INSTALL_DIR}/yyds"

    echo "Installed yyds to ${INSTALL_DIR}/yyds"

    # Check PATH
    case ":${PATH:-}:" in
        *":${INSTALL_DIR}:"*) ;;
        *)
            echo ""
            echo "Add yyds to your PATH by adding this to your shell profile:"
            echo ""
            echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
            echo ""
            ;;
    esac

    echo "Run 'yyds --help' to get started."
}

cargo_fallback() {
    if command -v cargo >/dev/null 2>&1; then
        echo "Building from source requires the sibling yoagent-state checkout until it is published."
        echo "Clone ${REPO} and ../yoagent-state, then run: cargo install --path ."
    else
        echo "Error: cargo is not installed. Install Rust first: https://rustup.rs"
        exit 1
    fi
    exit 1
}

main
