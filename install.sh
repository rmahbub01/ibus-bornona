#!/usr/bin/env bash
#
# ibus-bornona: Bornona Bengali Fixed Layout Input Method for IBus
#
# Automated install and uninstall script.
# Can be run directly from repository:
#   ./install.sh
#   ./install.sh --uninstall
#
# Or piped via curl:
#   curl -LsSf https://raw.githubusercontent.com/rmahbub01/ibus-bornona/master/install.sh | sh
#   curl -LsSf https://raw.githubusercontent.com/rmahbub01/ibus-bornona/master/install.sh | sh -s -- --uninstall
#

set -euo pipefail

REPO_URL="${REPO_URL:-https://github.com/rmahbub01/ibus-bornona.git}"
UNINSTALL=false
PREFIX=""

for arg in "$@"; do
    case "$arg" in
        --uninstall|-u)
            UNINSTALL=true
            ;;
        --prefix=*)
            PREFIX="${arg#*=}"
            ;;
        --help|-h)
            echo "Usage: ./install.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --uninstall, -u    Uninstall ibus-bornona"
            echo "  --prefix=DIR       Custom install prefix (default: ~/.local or /usr/local)"
            echo "  --help, -h         Show this message"
            exit 0
            ;;
    esac
done

if [ -n "$PREFIX" ]; then
    INSTALL_PREFIX="$PREFIX"
elif [ "${EUID:-$(id -u)}" -eq 0 ]; then
    INSTALL_PREFIX="/usr/local"
else
    INSTALL_PREFIX="$HOME/.local"
fi

BINDIR="$INSTALL_PREFIX/bin"
if [ "$INSTALL_PREFIX" = "/usr" ] || [ "$INSTALL_PREFIX" = "/usr/local" ]; then
    COMPONENT_DIR="/usr/share/ibus/component"
else
    COMPONENT_DIR="$INSTALL_PREFIX/share/ibus/component"
fi
DATA_DIR="$INSTALL_PREFIX/share/ibus-bornona"

if [ "$UNINSTALL" = true ]; then
    echo "Uninstalling ibus-bornona..."

    rm -f "$BINDIR/ibus-bornona"
    rm -f "$HOME/.local/bin/ibus-bornona"
    if [ "${EUID:-$(id -u)}" -eq 0 ]; then
        rm -f /usr/local/bin/ibus-bornona /usr/bin/ibus-bornona
    fi

    rm -f "$COMPONENT_DIR/ibus-bornona.xml"
    rm -f "$HOME/.local/share/ibus/component/ibus-bornona.xml"
    if [ "${EUID:-$(id -u)}" -eq 0 ]; then
        rm -f /usr/share/ibus/component/ibus-bornona.xml /usr/local/share/ibus/component/ibus-bornona.xml
    fi

    rm -rf "$DATA_DIR"
    rm -rf "$HOME/.local/share/ibus-bornona"

    # Remove any legacy systemd override if previously created
    OVERRIDE_FILE="$HOME/.config/systemd/user/org.freedesktop.IBus.session.GNOME.service.d/override.conf"
    if [ -f "$OVERRIDE_FILE" ]; then
        rm -f "$OVERRIDE_FILE"
    fi
    rm -f "$HOME/.config/systemd/user/org.freedesktop.IBus.session.GNOME.service.d/ibus-bornona.conf"
    rmdir "$HOME/.config/systemd/user/org.freedesktop.IBus.session.GNOME.service.d" 2>/dev/null || true
    if command -v systemctl >/dev/null 2>&1; then
        systemctl --user daemon-reload 2>/dev/null || true
    fi

    PYTHON_BIN="$(command -v python3 || command -v /usr/bin/python3 || echo "")"
    if command -v gsettings >/dev/null 2>&1 && [ -n "$PYTHON_BIN" ]; then
        CURRENT_SOURCES=$(gsettings get org.gnome.desktop.input-sources sources 2>/dev/null || echo "")
        if [[ "$CURRENT_SOURCES" == *"('ibus', 'bornona')"* ]]; then
            CLEANED_SOURCES=$($PYTHON_BIN -c "
import ast
try:
    s = ast.literal_eval('''$CURRENT_SOURCES''')
    s = [x for x in s if x != ('ibus', 'bornona')]
    print(repr(s))
except:
    pass
" 2>/dev/null || true)
            if [ -n "$CLEANED_SOURCES" ]; then
                gsettings set org.gnome.desktop.input-sources sources "$CLEANED_SOURCES" 2>/dev/null || true
            fi
        fi
    fi

    if command -v ibus >/dev/null 2>&1; then
        if [ "${EUID:-$(id -u)}" -eq 0 ]; then
            ibus write-cache --system 2>/dev/null || true
        fi
        ibus write-cache 2>/dev/null || true
        ibus restart 2>/dev/null || true
    fi

    echo "ibus-bornona uninstalled successfully."
    exit 0
fi

echo "Installing ibus-bornona..."

if ! command -v cargo >/dev/null 2>&1; then
    echo "Error: cargo (Rust) is required to build ibus-bornona." >&2
    echo "Install Rust via https://rustup.rs or your system package manager." >&2
    exit 1
fi

TMP_DIR=""
cleanup() {
    if [ -n "$TMP_DIR" ] && [ -d "$TMP_DIR" ]; then
        rm -rf "$TMP_DIR"
    fi
}
trap cleanup EXIT

if [ -f "./Cargo.toml" ] && [ -d "./src" ]; then
    SRC_DIR="$(pwd)"
else
    TMP_DIR=$(mktemp -d -t ibus-bornona-build-XXXXXX)
    echo "Cloning ibus-bornona repository..."
    git clone --depth 1 "$REPO_URL" "$TMP_DIR"
    SRC_DIR="$TMP_DIR"
fi

echo "Building ibus-bornona..."
cargo build --release --manifest-path "$SRC_DIR/Cargo.toml"

echo "Installing files to $INSTALL_PREFIX..."
mkdir -p "$BINDIR"
mkdir -p "$COMPONENT_DIR"
mkdir -p "$DATA_DIR"

install -m 755 "$SRC_DIR/target/release/ibus-bornona" "$BINDIR/ibus-bornona"
sed -e "s|@BINDIR@|$BINDIR|g" "$SRC_DIR/ibus-bornona.xml.in" > "$COMPONENT_DIR/ibus-bornona.xml"
install -m 644 "$SRC_DIR/data/bornona-keyboard.html" "$DATA_DIR/bornona-keyboard.html"
install -m 644 "$SRC_DIR/data/Bornona.json" "$DATA_DIR/Bornona.json"

if command -v ibus >/dev/null 2>&1; then
    if [ "${EUID:-$(id -u)}" -eq 0 ]; then
        ibus write-cache --system 2>/dev/null || true
    fi
    ibus write-cache 2>/dev/null || true
    ibus restart 2>/dev/null || true
fi

export PATH="$BINDIR:$PATH"

"$BINDIR/ibus-bornona" setup
echo ""
"$BINDIR/ibus-bornona" doctor
echo ""
echo "ibus-bornona installed successfully!"
echo "Switch layouts with Alt+Space."
