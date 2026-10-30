# ibus-bornona

**ibus-bornona** is a native, lightweight Bengali input method engine for IBus implementing the **Bornona (বর্ণনা)** fixed layout in pure Rust.

Designed for Linux desktop environments (GNOME, COSMIC, KDE, XFCE), it provides instant layout switching with <kbd>Alt</kbd>+<kbd>Space</kbd>, built-in typing automation rules, and zero runtime dependencies beyond standard `libibus-1.0` and `gtk+-3.0` / `webkit2gtk-4.1`.

---

## Keyboard Layout Reference

You can display the native graphical layout viewer at any time by running:

```bash
ibus-bornona layout
```

### Primary Layout (Normal Keys)

The standard typing layer for consonants, Bengali digits, and primary vowel signs (Kars).

![Bornona Layout Primary Keys](data/screenshots/Bornona_layout_primary_keys.png)

### Secondary Layout (Shift / Alt Keys)

The shifted layer accessed with <kbd>Shift</kbd> or <kbd>Alt</kbd>, providing independent vowels, conjuncts, special characters, Assamese letters, currency symbols, and signs.

![Bornona Layout Secondary Keys](data/screenshots/Bornona_layout_secondary_keys.png)

---

## Quick Install

Install with a single command (no root privileges required):

```bash
curl -LsSf https://raw.githubusercontent.com/rmahbub01/ibus-bornona/master/install.sh | sh
```

### Uninstallation

To remove `ibus-bornona`:

```bash
curl -LsSf https://raw.githubusercontent.com/rmahbub01/ibus-bornona/master/install.sh | sh -s -- --uninstall
```

For system-wide installation or removal from a cloned repository:

```bash
sudo make install
sudo make uninstall
```

These Make targets use `install.sh`. Uninstall removes Bornona's installed
binary, IBus component, icons, settings, learned words, and desktop integration
files while preserving other configured input sources.

The install script also removes Bornona from the current user's GNOME input
sources while preserving any other configured sources.

`install.sh --uninstall` also removes ibus-bornona's per-user settings and
learned-word dictionary, along with its installed component, icons, and
Bornona-specific desktop integration files. Use the same `--prefix` value used
for a custom-prefix installation.

---

## CLI Usage

The `ibus-bornona` binary provides built-in utilities for layout viewing, setup, diagnostics, and testing:

```text
ibus-bornona: Bornona Bengali Fixed Layout Input Method for IBus
USAGE:
    ibus-bornona <command>

COMMANDS:
    setup            Add Bornona to your DE input sources
    tui              Open the interactive terminal interface
    doctor           Check that everything is installed and running
    config           Show or change typing feature settings
    reset            Restore all typing features to their defaults
    layout           Show native graphical layout popup
    compose <keys>   Show what a key sequence types, e.g. `ibus-bornona compose bFNna`
    version          Print the version
```

Run `ibus-bornona tui` to open the full-screen terminal interface. It includes a
saved typing-settings screen and a scrollable system health check. Use arrow
keys to navigate, Enter to select, and Esc to return. Press `q` to quit. The TUI
needs a terminal at least 64 columns by 24 rows. Running `ibus-bornona` with no
arguments opens the interface in a terminal; IBus startup without a terminal
keeps its existing behavior. Use `ibus-bornona --interactive` for the legacy
line-based typing preview.

### Typing Feature Settings (`config`, `reset`)

Show the current settings or change typing feature configurations:

```bash
ibus-bornona config
ibus-bornona config suggestions on
ibus-bornona config suggestions off
ibus-bornona config suggestion-trailing-space off
ibus-bornona config orientation system
ibus-bornona config keyboard-switch-shortcut Ctrl+Space
ibus-bornona config learning on
ibus-bornona learn clear
ibus-bornona config auto-vowel off
ibus-bornona config auto-chandra off
ibus-bornona config old-reph off
ibus-bornona config traditional-kar on
```

Feature names and options:
- `suggestions` (aliases: `word-suggestion`, `suggestion`): Show dictionary suggestions (`on` / `off`, default: `off`).
- `suggestion-trailing-space` (alias: `trailing-space`): Add a space after committing a selected suggestion (`on` / `off`, default: `off`).
- `orientation` (alias: `suggestion-orientation`): Arrange popup suggestions horizontally, vertically, or according to the IBus system setting (`horizontal` / `vertical` / `system`, default: `horizontal`).
- `keyboard-switch-shortcut`: Set the input-source switching shortcut using modifier+key notation (for example `Alt+Space`, `Ctrl+Space`, or `Shift+Alt+Space`; default: `Alt+Space`). Supported modifiers are Ctrl, Alt, Shift, and Super.
- `learning`: Remember selected dictionary suggestions to rank them higher (`on` / `off`, default: `on`). `ibus-bornona learn clear` removes learned preferences.
- `auto-vowel`: Automatic vowel forming (`on` / `off`, default: `on`).
- `auto-chandra`: Automatic chandrabindu position fix (`on` / `off`, default: `on`).
- `old-reph`: Old style reph placement (`on` / `off`, default: `on`).
- `traditional-kar`: Traditional Kar joining (`on` / `off`, default: `off`).

Settings are stored in `$XDG_CONFIG_HOME/ibus-bornona/config.json` (or `~/.config/ibus-bornona/config.json` when `XDG_CONFIG_HOME` is not set) and apply immediately to running sessions.
The learned word store is versioned and atomically updated under
`$XDG_DATA_HOME/ibus-bornona/user_dict.json` (or `~/.local/share/ibus-bornona/user_dict.json`).

Restore the defaults with:

```bash
ibus-bornona reset
```

### Visual Layout Popup (`layout`)

Open a native, rendered HTML keyboard layout reference window:

```bash
ibus-bornona layout
```

### System Health Check (`doctor`)

Verify your installation and desktop integration:

```bash
ibus-bornona doctor
```

Output:
```text
[ ok ] IBus component file installed
[ ok ] ibus-daemon is running
[ ok ] ibus lists the bornona engine
[ ok ] Bornona is in GNOME input sources
[ ok ] GNOME Alt+Space switches input sources
[ ok ] IBus Alt+Space trigger is configured
```

### Key Sequence Preview (`compose`)

Inspect what a keystroke sequence produces without opening a GUI editor:

```bash
ibus-bornona compose bFNna
# বর্ণনা

ibus-bornona compose "ami baqlaB gan gai"
# আমি বাংলায় গান গাই
```

## License

GNU General Public License v3.0 or later ([GPL-3.0-or-later](LICENSE)).
