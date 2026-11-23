BIN := sheva
BIN_DIR := /usr/local/bin
SYSTEM_UNIT_DIR ?= /etc/systemd/system
USER_UNIT_DIR ?= $(HOME)/.config/systemd/user
RULE_DIR ?= /etc/udev/rules.d
BRIDGE_UNIT := $(BIN)@.service
SINK_UNIT := $(BIN)-sink.service
RULE := 72-$(BIN).rules

.PHONY: build install uninstall

build:
	cargo build --release

install: build
	sudo install -Dm755 target/release/$(BIN) $(BIN_DIR)/$(BIN)
	sudo install -Dm644 dist/$(BRIDGE_UNIT) $(SYSTEM_UNIT_DIR)/$(BRIDGE_UNIT)
	sudo install -Dm644 dist/$(RULE) $(RULE_DIR)/$(RULE)
	sudo systemctl daemon-reload
	sudo udevadm control --reload
	install -Dm644 dist/$(SINK_UNIT) $(USER_UNIT_DIR)/$(SINK_UNIT)
	systemctl --user daemon-reload
	systemctl --user enable --now $(SINK_UNIT)
	sudo udevadm trigger --action=add --subsystem-match=hidraw --subsystem-match=input

uninstall:
	systemctl --user disable --now $(SINK_UNIT)
	rm -f $(USER_UNIT_DIR)/$(SINK_UNIT)
	systemctl --user daemon-reload
	sudo rm -f $(RULE_DIR)/$(RULE)
	sudo rm -rf /etc/$(BIN)
	sudo udevadm control --reload
	sudo systemctl stop '$(BIN)@*.service'
	sudo rm -f $(SYSTEM_UNIT_DIR)/$(BRIDGE_UNIT) $(BIN_DIR)/$(BIN)
	sudo systemctl daemon-reload
	sudo udevadm trigger --action=add --subsystem-match=hidraw --subsystem-match=input
