PREFIX ?= /usr/local
DESTDIR ?=
BINDIR ?= $(PREFIX)/bin
DATADIR ?= $(PREFIX)/share
IBUS_COMPONENT_DIR ?= $(DATADIR)/ibus/component
PKGDATADIR ?= $(DATADIR)/ibus-bornona

all: build

build:
	cargo build --release
	sed -e 's|@BINDIR@|$(BINDIR)|g' ibus-bornona.xml.in > ibus-bornona.xml

install: build
	install -d $(DESTDIR)$(BINDIR)
	install -d $(DESTDIR)$(IBUS_COMPONENT_DIR)
	install -d $(DESTDIR)$(PKGDATADIR)
	install -m 755 target/release/ibus-bornona $(DESTDIR)$(BINDIR)/ibus-bornona
	install -m 644 ibus-bornona.xml $(DESTDIR)$(IBUS_COMPONENT_DIR)/ibus-bornona.xml
	install -m 644 data/bornona-keyboard.html $(DESTDIR)$(PKGDATADIR)/bornona-keyboard.html
	install -m 644 data/Bornona.json $(DESTDIR)$(PKGDATADIR)/Bornona.json
	@if [ -z "$(DESTDIR)" ]; then \
		echo "Updating IBus cache..."; \
		if [ "$$(id -u)" -eq 0 ]; then \
			ibus write-cache --system 2>/dev/null || true; \
		fi; \
		ibus write-cache 2>/dev/null || true; \
		echo "Installation complete!"; \
	fi

install-user:
	$(MAKE) PREFIX=$(HOME)/.local IBUS_COMPONENT_DIR=$(HOME)/.local/share/ibus/component install

uninstall:
	@set -e; \
	if [ -n "$(DESTDIR)" ]; then \
		rm -f "$(DESTDIR)$(BINDIR)/ibus-bornona"; \
		rm -f "$(DESTDIR)$(IBUS_COMPONENT_DIR)/ibus-bornona.xml"; \
		rm -rf "$(DESTDIR)$(PKGDATADIR)"; \
	else \
		bindir="$(BINDIR)"; \
		component_dir="$(IBUS_COMPONENT_DIR)"; \
		pkgdatadir="$(PKGDATADIR)"; \
		remove_system_component=false; \
		if [ "$(PREFIX)" = "/usr/local" ]; then remove_system_component=true; fi; \
		if [ "$(PREFIX)" = "/usr/local" ] \
			&& [ ! -e "$$bindir/ibus-bornona" ] \
			&& [ ! -e "$$component_dir/ibus-bornona.xml" ] \
			&& { [ -e "$(HOME)/.local/bin/ibus-bornona" ] || [ -e "$(HOME)/.local/share/ibus/component/ibus-bornona.xml" ]; }; then \
			echo "Removing user installation from $(HOME)/.local"; \
			bindir="$(HOME)/.local/bin"; \
			component_dir="$(HOME)/.local/share/ibus/component"; \
			pkgdatadir="$(HOME)/.local/share/ibus-bornona"; \
			remove_system_component=false; \
		fi; \
		if [ -x "$$bindir/ibus-bornona" ]; then \
			remove_source_status=0; \
			source_cleanup_code='import ast, subprocess; schema="org.gnome.desktop.input-sources"; key="sources"; value=subprocess.check_output(["gsettings","get",schema,key],universal_newlines=True).strip(); value=value[len("@a(ss)"):].strip() if value.startswith("@a(ss)") else value; sources=ast.literal_eval(value); cleaned=[source for source in sources if tuple(source)!=("ibus","bornona")]; subprocess.run(["gsettings","set",schema,key,repr(cleaned)],check=True) if cleaned!=sources else None'; \
			if [ "$$(id -u)" -eq 0 ] && [ -n "$${SUDO_USER:-}" ] && [ "$$SUDO_USER" != root ]; then \
				user_uid=$$(id -u "$$SUDO_USER") || remove_source_status=1; \
				user_home=$$(getent passwd "$$SUDO_USER" | cut -d: -f6) || remove_source_status=1; \
				user_runtime="/run/user/$$user_uid"; \
				if [ "$$remove_source_status" -eq 0 ] && [ -S "$$user_runtime/bus" ] && command -v runuser >/dev/null 2>&1; then \
					if ! runuser -u "$$SUDO_USER" -- env HOME="$$user_home" XDG_RUNTIME_DIR="$$user_runtime" DBUS_SESSION_BUS_ADDRESS="unix:path=$$user_runtime/bus" "$$bindir/ibus-bornona" remove-source >/dev/null 2>&1; then \
						runuser -u "$$SUDO_USER" -- env HOME="$$user_home" XDG_RUNTIME_DIR="$$user_runtime" DBUS_SESSION_BUS_ADDRESS="unix:path=$$user_runtime/bus" python3 -c "$$source_cleanup_code" || remove_source_status=1; \
					fi; \
				else \
					remove_source_status=1; \
				fi; \
			else \
				if ! "$$bindir/ibus-bornona" remove-source >/dev/null 2>&1; then \
					python3 -c "$$source_cleanup_code" || remove_source_status=1; \
				fi; \
			fi; \
			if [ "$$remove_source_status" -ne 0 ]; then \
				echo "Warning: could not remove Bornona from GNOME input sources." >&2; \
			fi; \
		fi; \
		rm -f "$$bindir/ibus-bornona"; \
		rm -f "$$component_dir/ibus-bornona.xml"; \
		rm -rf "$$pkgdatadir"; \
		if [ "$$remove_system_component" = true ] && [ -x "$(BINDIR)/ibus-bornona" ]; then \
			rm -f /usr/share/ibus/component/ibus-bornona.xml; \
		fi; \
		rm -f "$(HOME)/.config/systemd/user/org.freedesktop.IBus.session.GNOME.service.d/ibus-bornona.conf"; \
		rmdir "$(HOME)/.config/systemd/user/org.freedesktop.IBus.session.GNOME.service.d" 2>/dev/null || true; \
		if command -v systemctl >/dev/null 2>&1; then \
			if ! systemctl --user daemon-reload; then echo "Warning: could not reload the user systemd manager." >&2; fi; \
		fi; \
		if command -v ibus >/dev/null 2>&1; then \
			if [ "$$(id -u)" -eq 0 ] && ! ibus write-cache --system; then echo "Warning: could not refresh the system IBus registry." >&2; fi; \
			if ! ibus write-cache; then echo "Warning: could not refresh the user IBus registry." >&2; fi; \
			if ! ibus restart; then echo "Warning: could not restart IBus." >&2; fi; \
		fi; \
		echo "Uninstallation complete."; \
	fi

uninstall-user:
	$(MAKE) PREFIX=$(HOME)/.local IBUS_COMPONENT_DIR=$(HOME)/.local/share/ibus/component uninstall

test:
	cargo test

clean:
	cargo clean
	rm -f ibus-bornona.xml

.PHONY: all build install install-user uninstall uninstall-user test clean
