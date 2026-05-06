# Necessary dependencies for the build system

TOOLS_BIN ?= $(TARGET_DIR)/tools/bin
TOOLS_TARGET_DIR ?= $(TARGET_DIR)/tools-target
CARGO_AXPLAT ?= $(TOOLS_BIN)/cargo-axplat axplat
AXCONFIG_GEN ?= $(TOOLS_BIN)/axconfig-gen
export PATH := $(TOOLS_BIN):$(PATH)

# Tool to parse information about the target package
ifeq ($(shell test -x $(TOOLS_BIN)/cargo-axplat && $(TOOLS_BIN)/cargo-axplat --version >/dev/null 2>&1 && echo ok),)
  $(info Building cargo-axplat from tools...)
  $(shell cd $(APP) && mkdir -p $(TOOLS_BIN) && cargo build --manifest-path tools/cargo-axplat-0.3.0/Cargo.toml --release --locked --offline --target-dir $(TOOLS_TARGET_DIR) >&2 && cp $(TOOLS_TARGET_DIR)/release/cargo-axplat $(TOOLS_BIN)/cargo-axplat)
endif
ifeq ($(shell test -x $(TOOLS_BIN)/cargo-axplat && $(TOOLS_BIN)/cargo-axplat --version >/dev/null 2>&1 && echo ok),)
  $(error failed to build cargo-axplat from tools/cargo-axplat-0.3.0)
endif

# Tool to generate platform configuration files
ifeq ($(shell test -x $(TOOLS_BIN)/axconfig-gen && $(TOOLS_BIN)/axconfig-gen --version >/dev/null 2>&1 && echo ok),)
  $(info Building axconfig-gen from tools...)
  $(shell cd $(APP) && mkdir -p $(TOOLS_BIN) && cargo build --manifest-path tools/axconfig-gen-0.2.1/Cargo.toml --release --locked --offline --target-dir $(TOOLS_TARGET_DIR) >&2 && cp $(TOOLS_TARGET_DIR)/release/axconfig-gen $(TOOLS_BIN)/axconfig-gen)
endif
ifeq ($(shell test -x $(TOOLS_BIN)/axconfig-gen && $(TOOLS_BIN)/axconfig-gen --version >/dev/null 2>&1 && echo ok),)
  $(error failed to build axconfig-gen from tools/axconfig-gen-0.2.1)
endif

# Cargo binutils
ifeq ($(shell command -v rust-objcopy >/dev/null 2>&1 && command -v rust-objdump >/dev/null 2>&1 && echo ok),)
  $(error rust-objcopy/rust-objdump not found; install cargo-binutils in the build image)
endif
