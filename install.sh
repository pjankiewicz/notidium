#!/bin/sh
# Notidium installer for macOS and Linux.
#
# Usage (interactive):
#   curl -fsSL https://raw.githubusercontent.com/pjankiewicz/notidium/main/install.sh | sh
#
# Non-interactive (CI / scripted):
#   NOTIDIUM_VAULT=/path/to/vault NOTIDIUM_INSTALL_SERVICE=1 \
#     curl -fsSL .../install.sh | sh -s -- --yes
#
# Environment variables:
#   NOTIDIUM_VAULT             Vault path (skips prompt).
#   NOTIDIUM_INSTALL_SERVICE   0 or 1 (skips prompt).
#   NOTIDIUM_NO_MODIFY_PATH    If set, do not modify shell rc files.
#   NOTIDIUM_VERSION           Pin to a specific release tag (default: latest).
#   NOTIDIUM_REPO              owner/repo to download from (default: pjankiewicz/notidium).

set -eu

REPO="${NOTIDIUM_REPO:-pjankiewicz/notidium}"
INSTALL_DIR="${HOME}/.notidium/bin"
TMP_DIR=""
YES=0

cleanup() {
    if [ -n "$TMP_DIR" ] && [ -d "$TMP_DIR" ]; then
        rm -rf "$TMP_DIR"
    fi
}
trap cleanup EXIT INT TERM

die() {
    printf '\033[31merror:\033[0m %s\n' "$1" >&2
    exit 1
}

info() {
    printf '\033[36m==>\033[0m %s\n' "$1"
}

need() {
    command -v "$1" >/dev/null 2>&1 || die "required command not found: $1"
}

for arg in "$@"; do
    case "$arg" in
        -y|--yes) YES=1 ;;
        -h|--help)
            sed -n '2,20p' "$0"
            exit 0 ;;
        *) die "unknown argument: $arg" ;;
    esac
done

need curl
need tar
need uname

# ---------- detect target triple ----------
detect_target() {
    os="$(uname -s)"
    arch="$(uname -m)"
    case "$os" in
        Darwin)
            case "$arch" in
                arm64|aarch64) echo "aarch64-apple-darwin" ;;
                x86_64)        echo "x86_64-apple-darwin" ;;
                *) die "unsupported macOS arch: $arch" ;;
            esac ;;
        Linux)
            case "$arch" in
                aarch64|arm64) echo "aarch64-unknown-linux-gnu" ;;
                x86_64|amd64)  echo "x86_64-unknown-linux-gnu" ;;
                *) die "unsupported Linux arch: $arch" ;;
            esac ;;
        *) die "unsupported OS: $os (use install.ps1 on Windows)" ;;
    esac
}

TARGET="$(detect_target)"
info "Detected target: $TARGET"

# ---------- resolve release ----------
if [ -n "${NOTIDIUM_VERSION:-}" ]; then
    TAG="$NOTIDIUM_VERSION"
    case "$TAG" in v*) ;; *) TAG="v$TAG" ;; esac
else
    info "Looking up latest release..."
    # GitHub's "latest release" redirects /releases/latest to /releases/tag/<vX>.
    # Follow the redirect, parse the tag from the final URL — no jq needed.
    TAG="$(curl -fsSLI -o /dev/null -w '%{url_effective}' \
        "https://github.com/${REPO}/releases/latest" | \
        sed -E 's|.*/tag/(v[^/]+)/?$|\1|')"
    [ -n "$TAG" ] || die "could not resolve latest release tag"
fi
VERSION="${TAG#v}"
info "Installing ${TAG}"

ASSET="notidium-${VERSION}-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET}"
SUMS_URL="https://github.com/${REPO}/releases/download/${TAG}/SHA256SUMS"

TMP_DIR="$(mktemp -d)"
cd "$TMP_DIR"

info "Downloading $ASSET..."
curl -fsSL -o "$ASSET" "$URL" || die "download failed: $URL"
curl -fsSL -o "SHA256SUMS" "$SUMS_URL" || die "could not download SHA256SUMS"

# ---------- verify checksum ----------
expected="$(grep -E "  ${ASSET}$" SHA256SUMS | awk '{print $1}')"
[ -n "$expected" ] || die "checksum for $ASSET not found in SHA256SUMS"

if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$ASSET" | awk '{print $1}')"
elif command -v shasum >/dev/null 2>&1; then
    actual="$(shasum -a 256 "$ASSET" | awk '{print $1}')"
else
    die "need sha256sum or shasum to verify the download"
fi

if [ "$actual" != "$expected" ]; then
    die "checksum mismatch for $ASSET: expected $expected, got $actual"
fi
info "Checksum OK"

# ---------- extract ----------
tar -xzf "$ASSET"
BIN_SRC="${TMP_DIR}/notidium-${VERSION}-${TARGET}/notidium"
[ -f "$BIN_SRC" ] || die "archive did not contain notidium binary at expected path"

mkdir -p "$INSTALL_DIR"
mv "$BIN_SRC" "$INSTALL_DIR/notidium"
chmod +x "$INSTALL_DIR/notidium"
info "Installed binary: $INSTALL_DIR/notidium"

# ---------- onboarding prompts ----------
read_tty() {
    prompt="$1"
    default="$2"
    if [ "$YES" -eq 1 ]; then
        printf '%s\n' "$default"
        return
    fi
    if [ -r /dev/tty ]; then
        printf '\033[33m?\033[0m %s [%s]: ' "$prompt" "$default" > /dev/tty
        IFS= read -r ans < /dev/tty || ans=""
        [ -n "$ans" ] || ans="$default"
        printf '%s\n' "$ans"
    else
        printf '%s\n' "$default"
    fi
}

if [ -n "${NOTIDIUM_VAULT:-}" ]; then
    VAULT="$NOTIDIUM_VAULT"
else
    VAULT="$(read_tty "Vault path" "${HOME}/Notidium")"
fi

# Expand leading ~ so downstream tools get an absolute path.
case "$VAULT" in
    "~"|"~/"*) VAULT="${HOME}${VAULT#~}" ;;
esac

if [ -n "${NOTIDIUM_INSTALL_SERVICE:-}" ]; then
    case "$NOTIDIUM_INSTALL_SERVICE" in
        1|true|yes|y|Y) SETUP_SERVICE=1 ;;
        *) SETUP_SERVICE=0 ;;
    esac
else
    ans="$(read_tty "Set up auto-start at login? (Y/n)" "Y")"
    case "$ans" in
        n|N|no|NO) SETUP_SERVICE=0 ;;
        *) SETUP_SERVICE=1 ;;
    esac
fi

# ---------- initialize vault ----------
if [ ! -d "$VAULT/.notidium" ]; then
    info "Initializing vault at $VAULT"
    "$INSTALL_DIR/notidium" init "$VAULT" >/dev/null
fi

# ---------- install service ----------
if [ "$SETUP_SERVICE" -eq 1 ]; then
    info "Installing auto-start service"
    "$INSTALL_DIR/notidium" install-service --vault "$VAULT" --force
fi

# ---------- patch shell rc for PATH ----------
patch_rc() {
    rc="$1"
    [ -f "$rc" ] || return 0
    if grep -Fq ".notidium/bin" "$rc" 2>/dev/null; then
        return 0
    fi
    {
        printf '\n# Added by notidium installer\n'
        printf 'export PATH="$HOME/.notidium/bin:$PATH"\n'
    } >> "$rc"
    info "Updated PATH in $rc"
}

if [ -z "${NOTIDIUM_NO_MODIFY_PATH:-}" ]; then
    case ":$PATH:" in
        *":${INSTALL_DIR}:"*) ;;
        *)
            patch_rc "$HOME/.zshrc"
            patch_rc "$HOME/.bashrc"
            patch_rc "$HOME/.profile"
            if [ -f "$HOME/.config/fish/config.fish" ]; then
                if ! grep -Fq ".notidium/bin" "$HOME/.config/fish/config.fish"; then
                    {
                        printf '\n# Added by notidium installer\n'
                        printf 'set -gx PATH $HOME/.notidium/bin $PATH\n'
                    } >> "$HOME/.config/fish/config.fish"
                    info "Updated PATH in ~/.config/fish/config.fish"
                fi
            fi
            ;;
    esac
fi

# ---------- next steps ----------
printf '\n\033[32m✓ Notidium installed successfully\033[0m\n\n'
printf '  Binary: %s\n' "$INSTALL_DIR/notidium"
printf '  Vault:  %s\n' "$VAULT"
if [ "$SETUP_SERVICE" -eq 1 ]; then
    printf '  Server: http://localhost:3939 (auto-starts at login)\n'
    printf '  Logs:   %s/.notidium/logs/server.log\n' "$VAULT"
else
    printf '  Start manually: notidium serve %s\n' "$VAULT"
fi

case ":$PATH:" in
    *":${INSTALL_DIR}:"*) ;;
    *) printf '\nRestart your shell or run: export PATH="%s:$PATH"\n' "$INSTALL_DIR" ;;
esac
