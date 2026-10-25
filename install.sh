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
ICON_DIR="$INSTALL_PREFIX/share/icons/hicolor/scalable/apps"
ICON_DIR_PNG="$INSTALL_PREFIX/share/icons/hicolor/128x128/apps"

USER_HOME="${HOME:-}"
if [ "${EUID:-$(id -u)}" -eq 0 ] && [ -n "${SUDO_USER:-}" ] && [ "$SUDO_USER" != root ]; then
    USER_HOME="$(getent passwd "$SUDO_USER" | cut -d: -f6)"
fi
if [ -z "$USER_HOME" ]; then
    echo "Error: could not determine the target user's home directory." >&2
    exit 1
fi

USER_CONFIG_HOME="$USER_HOME/.config"
USER_DATA_HOME="$USER_HOME/.local/share"
if [[ "${XDG_CONFIG_HOME:-}" = /* ]]; then
    USER_CONFIG_HOME="$XDG_CONFIG_HOME"
fi
if [[ "${XDG_DATA_HOME:-}" = /* ]]; then
    USER_DATA_HOME="$XDG_DATA_HOME"
fi

remove_input_method_config() {
    local binary="$1"
    local python_cleanup='
import ast
import subprocess

schemas = set(subprocess.check_output(["gsettings", "list-schemas"], text=True).splitlines())
for schema, key, kind in (
    ("org.gnome.desktop.input-sources", "sources", "sources"),
    ("org.freedesktop.ibus.general", "preload-engines", "engines"),
    ("org.freedesktop.ibus.general", "engines-order", "engines"),
):
    if schema not in schemas:
        continue
    value = subprocess.check_output(["gsettings", "get", schema, key], text=True).strip()
    for prefix in ("@a(ss)", "@as"):
        if value.startswith(prefix):
            value = value[len(prefix):].strip()
            break
    current = ast.literal_eval(value)
    if kind == "sources":
        cleaned = [source for source in current if tuple(source) != ("ibus", "bornona")]
    else:
        cleaned = [engine for engine in current if engine != "bornona"]
    if cleaned != current:
        subprocess.run(["gsettings", "set", schema, key, repr(cleaned)], check=True)
'
    if [ "${EUID:-$(id -u)}" -eq 0 ] && [ -n "${SUDO_USER:-}" ] && [ "$SUDO_USER" != root ]; then
        local user_uid user_home user_runtime
        user_uid="$(id -u "$SUDO_USER")" || return 1
        user_home="$(getent passwd "$SUDO_USER" | cut -d: -f6)" || return 1
        user_runtime="/run/user/$user_uid"
        [ -S "$user_runtime/bus" ] || return 1
        command -v runuser >/dev/null 2>&1 || return 1
        runuser -u "$SUDO_USER" -- env \
            HOME="$user_home" \
            XDG_RUNTIME_DIR="$user_runtime" \
            XDG_CONFIG_HOME="$USER_CONFIG_HOME" \
            DBUS_SESSION_BUS_ADDRESS="unix:path=$user_runtime/bus" \
            "$binary" remove-source >/dev/null 2>&1 || runuser -u "$SUDO_USER" -- env \
                HOME="$user_home" \
                XDG_RUNTIME_DIR="$user_runtime" \
                XDG_CONFIG_HOME="$USER_CONFIG_HOME" \
                DBUS_SESSION_BUS_ADDRESS="unix:path=$user_runtime/bus" \
                python3 -c "$python_cleanup"
    else
        if [ -n "$binary" ]; then
            "$binary" remove-source >/dev/null 2>&1 || python3 -c "$python_cleanup"
        else
            python3 -c "$python_cleanup"
        fi
    fi
}

if [ "$UNINSTALL" = true ]; then
    echo "Uninstalling ibus-bornona..."

    REMOVE_SOURCE_BIN=""
    for candidate in "$BINDIR/ibus-bornona" "$USER_HOME/.local/bin/ibus-bornona" /usr/local/bin/ibus-bornona /usr/bin/ibus-bornona; do
        if [ -x "$candidate" ]; then
            REMOVE_SOURCE_BIN="$candidate"
            break
        fi
    done
    if ! remove_input_method_config "$REMOVE_SOURCE_BIN"; then
        echo "Warning: could not remove Bornona from desktop or IBus input settings." >&2
    fi

    rm -f "$BINDIR/ibus-bornona"
    rm -f "$USER_HOME/.local/bin/ibus-bornona"
    if [ "${EUID:-$(id -u)}" -eq 0 ]; then
        rm -f /usr/local/bin/ibus-bornona /usr/bin/ibus-bornona
    fi

    rm -f "$COMPONENT_DIR/ibus-bornona.xml"
    rm -f "$USER_DATA_HOME/ibus/component/ibus-bornona.xml"
    rm -f "$USER_HOME/.local/share/ibus/component/ibus-bornona.xml"
    if [ "${EUID:-$(id -u)}" -eq 0 ]; then
        rm -f /usr/share/ibus/component/ibus-bornona.xml /usr/local/share/ibus/component/ibus-bornona.xml
    fi

    rm -f "$ICON_DIR/ibus-bornona.svg"
    rm -f "$ICON_DIR_PNG/ibus-bornona.png"
    rm -f "$USER_DATA_HOME/icons/hicolor/scalable/apps/ibus-bornona.svg"
    rm -f "$USER_DATA_HOME/icons/hicolor/128x128/apps/ibus-bornona.png"
    rm -f "$USER_HOME/.local/share/icons/hicolor/scalable/apps/ibus-bornona.svg"
    rm -f "$USER_HOME/.local/share/icons/hicolor/128x128/apps/ibus-bornona.png"
    if [ "${EUID:-$(id -u)}" -eq 0 ]; then
        rm -f /usr/share/icons/hicolor/scalable/apps/ibus-bornona.svg /usr/local/share/icons/hicolor/scalable/apps/ibus-bornona.svg
        rm -f /usr/share/icons/hicolor/128x128/apps/ibus-bornona.png /usr/local/share/icons/hicolor/128x128/apps/ibus-bornona.png
    fi

    rm -rf "$DATA_DIR"
    if [ "${EUID:-$(id -u)}" -eq 0 ]; then
        rm -rf /usr/share/ibus-bornona /usr/local/share/ibus-bornona
    fi
    rm -rf "$USER_CONFIG_HOME/ibus-bornona" "$USER_HOME/.config/ibus-bornona"
    rm -rf "$USER_DATA_HOME/ibus-bornona" "$USER_HOME/.local/share/ibus-bornona"

    # Remove only desktop integration files owned by ibus-bornona.
    for config_home in "$USER_CONFIG_HOME" "$USER_HOME/.config"; do
        autostart_dir="$config_home/autostart"
        rm -f "$autostart_dir/ibus-bornona-cosmic.desktop" "$autostart_dir/ibus-bornona.desktop"
        if [ -f "$autostart_dir/ibus-daemon.desktop" ] \
            && grep -Fq 'Comment=Start IBus daemon with Bornona support' "$autostart_dir/ibus-daemon.desktop"; then
            rm -f "$autostart_dir/ibus-daemon.desktop"
        fi
        rm -f "$config_home/environment.d/50-ibus-bornona.conf"
        cosmic_shortcuts="$config_home/cosmic/com.system76.CosmicSettings.Shortcuts/v1/custom"
        if [ -f "$cosmic_shortcuts" ] && grep -Fq 'ibus-bornona toggle' "$cosmic_shortcuts"; then
            sed -i '/ibus-bornona toggle/d' "$cosmic_shortcuts"
        fi

        dropin_dir="$config_home/systemd/user/org.freedesktop.IBus.session.GNOME.service.d"
        rm -f "$dropin_dir/ibus-bornona.conf"
        rmdir "$dropin_dir" 2>/dev/null || true
        cosmic_unit="$config_home/systemd/user/ibus-bornona.service"
        cosmic_link="$config_home/systemd/user/cosmic-session.target.wants/ibus-bornona.service"
        if grep -Fq '# Managed by ibus-bornona.' "$cosmic_unit" 2>/dev/null; then
            if grep -Fq '# ibus-bornona owns the COSMIC session link' "$cosmic_unit"; then
                rm -f "$cosmic_link"
                rmdir "$(dirname "$cosmic_link")" 2>/dev/null || true
            fi
            rm -f "$cosmic_unit"
        fi
        legacy_cosmic_dropin="$config_home/systemd/user/org.freedesktop.IBus.session.generic.service.d/ibus-bornona.conf"
        legacy_cosmic_link="$config_home/systemd/user/cosmic-session.target.wants/org.freedesktop.IBus.session.generic.service"
        if grep -Fq '# ibus-bornona owns the COSMIC session link' "$legacy_cosmic_dropin" 2>/dev/null; then
            rm -f "$legacy_cosmic_link" "$legacy_cosmic_dropin"
            rmdir "$(dirname "$legacy_cosmic_dropin")" 2>/dev/null || true
        fi
    done
    if command -v systemctl >/dev/null 2>&1; then
        if [ "${EUID:-$(id -u)}" -eq 0 ] && [ -n "${SUDO_USER:-}" ] && [ "$SUDO_USER" != root ]; then
            user_uid="$(id -u "$SUDO_USER")"
            user_runtime="/run/user/$user_uid"
            if [ -S "$user_runtime/bus" ] && command -v runuser >/dev/null 2>&1; then
                runuser -u "$SUDO_USER" -- env HOME="$USER_HOME" XDG_RUNTIME_DIR="$user_runtime" \
                    DBUS_SESSION_BUS_ADDRESS="unix:path=$user_runtime/bus" \
                    systemctl --user daemon-reload || echo "Warning: could not reload the user's systemd manager." >&2
            fi
        else
            systemctl --user daemon-reload || echo "Warning: could not reload the user systemd manager." >&2
        fi
    fi

    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -f -t "$INSTALL_PREFIX/share/icons/hicolor" 2>/dev/null || true
        gtk-update-icon-cache -f -t "$USER_DATA_HOME/icons/hicolor" 2>/dev/null || true
        gtk-update-icon-cache -f -t "$USER_HOME/.local/share/icons/hicolor" 2>/dev/null || true
    fi

    pkill -x ibus-bornona 2>/dev/null || true
    if command -v ibus >/dev/null 2>&1; then
        if [ "${EUID:-$(id -u)}" -eq 0 ]; then
            ibus write-cache --system </dev/null >/dev/null 2>&1 || true
        fi
        ibus write-cache </dev/null >/dev/null 2>&1 || true
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
mkdir -p "$ICON_DIR"
mkdir -p "$ICON_DIR_PNG"

install -m 755 "$SRC_DIR/target/release/ibus-bornona" "$BINDIR/ibus-bornona"
sed -e "s|@BINDIR@|$BINDIR|g" -e "s|@ICONPATH@|$DATA_DIR/ibus-bornona.png|g" "$SRC_DIR/ibus-bornona.xml.in" > "$COMPONENT_DIR/ibus-bornona.xml"
install -m 644 "$SRC_DIR/data/bornona-keyboard.html" "$DATA_DIR/bornona-keyboard.html"
install -m 644 "$SRC_DIR/data/Bornona.json" "$DATA_DIR/Bornona.json"
install -m 644 "$SRC_DIR/data/logo/ibus-bornona.svg" "$ICON_DIR/ibus-bornona.svg"
install -m 644 "$SRC_DIR/data/logo/ibus-bornona.png" "$ICON_DIR_PNG/ibus-bornona.png"
install -m 644 "$SRC_DIR/data/logo/ibus-bornona.svg" "$DATA_DIR/ibus-bornona.svg"
install -m 644 "$SRC_DIR/data/logo/ibus-bornona.png" "$DATA_DIR/ibus-bornona.png"

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$INSTALL_PREFIX/share/icons/hicolor" 2>/dev/null || true
fi

if command -v ibus >/dev/null 2>&1; then
    if [ "${EUID:-$(id -u)}" -eq 0 ]; then
        ibus write-cache --system </dev/null >/dev/null 2>&1 || true
    fi
    ibus write-cache </dev/null >/dev/null 2>&1 || true
fi

export PATH="$BINDIR:$PATH"

if [ "${EUID:-$(id -u)}" -eq 0 ] && [ -n "${SUDO_USER:-}" ] && [ "$SUDO_USER" != root ]; then
    user_uid="$(id -u "$SUDO_USER")"
    user_home="$(getent passwd "$SUDO_USER" | cut -d: -f6)"
    user_runtime="/run/user/$user_uid"
    if [ -S "$user_runtime/bus" ] && command -v runuser >/dev/null 2>&1; then
        DESKTOP_ENV=(
            "HOME=$user_home"
            "XDG_RUNTIME_DIR=$user_runtime"
            "DBUS_SESSION_BUS_ADDRESS=unix:path=$user_runtime/bus"
            "PATH=$BINDIR:$PATH"
        )
        [ -z "${XDG_CONFIG_HOME:-}" ] || DESKTOP_ENV+=("XDG_CONFIG_HOME=$XDG_CONFIG_HOME")
        [ -z "${XDG_DATA_HOME:-}" ] || DESKTOP_ENV+=("XDG_DATA_HOME=$XDG_DATA_HOME")
        [ -z "${XDG_CURRENT_DESKTOP:-}" ] || DESKTOP_ENV+=("XDG_CURRENT_DESKTOP=$XDG_CURRENT_DESKTOP")
        [ -z "${XDG_SESSION_DESKTOP:-}" ] || DESKTOP_ENV+=("XDG_SESSION_DESKTOP=$XDG_SESSION_DESKTOP")
        [ -z "${WAYLAND_DISPLAY:-}" ] || DESKTOP_ENV+=("WAYLAND_DISPLAY=$WAYLAND_DISPLAY")
        [ -z "${XDG_SESSION_TYPE:-}" ] || DESKTOP_ENV+=("XDG_SESSION_TYPE=$XDG_SESSION_TYPE")
        runuser -u "$SUDO_USER" -- env "${DESKTOP_ENV[@]}" "$BINDIR/ibus-bornona" setup
        echo ""
        runuser -u "$SUDO_USER" -- env "${DESKTOP_ENV[@]}" "$BINDIR/ibus-bornona" doctor
    else
        echo "System install complete. Run 'ibus-bornona setup' from your desktop session to configure Bornona."
    fi
else
    "$BINDIR/ibus-bornona" setup
    echo ""
    "$BINDIR/ibus-bornona" doctor
fi
echo ""
echo "ibus-bornona installed successfully!"
echo "Switch layouts with your configured keyboard shortcut (default: Alt+Space)."
