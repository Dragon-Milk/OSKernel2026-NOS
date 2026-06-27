.DEFAULT_GOAL := all

ROOT_DIR := $(CURDIR)
SRC_DIR := $(ROOT_DIR)/src
RV_OUT_CONFIG := $(SRC_DIR)/.axconfig-riscv64.toml
LA_OUT_CONFIG := $(SRC_DIR)/.axconfig-loongarch64.toml
RUN_OUT_CONFIG := $(if $(filter loongarch64,$(ARCH)),$(LA_OUT_CONFIG),$(RV_OUT_CONFIG))

export RUSTUP_DIST_SERVER := https://mirrors.ustc.edu.cn/rust-static
export RUSTUP_UPDATE_ROOT := https://mirrors.ustc.edu.cn/rust-static/rustup
export CARGO_NET_OFFLINE := true

# Shared flags forwarded to inner src/ builds.
# $(MAKE) inherits environment variables and MAKEFLAGS automatically,
# so TEST_PROFILE, LTP_LIBC, LTP_CASE_LIST, LTP_TIMEOUT, FULL_SAFE_SKIP_WASTE,
# NS_DURATION flow through without extra work.
SRC_MAKE_ARGS := A=$(SRC_DIR) TARGET_DIR=$(SRC_DIR)/target

all: prepare-vendor kernel-rv kernel-la
	@rm -rf $(SRC_DIR)/target
	@rm -f $(SRC_DIR)/*.elf $(SRC_DIR)/*.bin
	@rm -f $(SRC_DIR)/.axconfig.toml $(SRC_DIR)/.axconfig-*.toml $(SRC_DIR)/.axconfig-*.old.toml
	@rm -f $(SRC_DIR)/kernel-rv $(SRC_DIR)/kernel-la

prepare-vendor:
	@mkdir -p $(SRC_DIR)/.cargo
	@if [ -f $(SRC_DIR)/cargo-config.toml ] && [ ! -f $(SRC_DIR)/.cargo/config.toml ]; then \
		cp $(SRC_DIR)/cargo-config.toml $(SRC_DIR)/.cargo/config.toml; \
	fi
	@if [ -f $(SRC_DIR)/cargo-config ] && [ ! -f $(SRC_DIR)/.cargo/config ]; then \
		cp $(SRC_DIR)/cargo-config $(SRC_DIR)/.cargo/config; \
	fi
	@if [ -d $(SRC_DIR)/vendor ]; then \
		find $(SRC_DIR)/vendor -mindepth 2 -maxdepth 2 -name cargo-checksum.json \
			-exec sh -c 'dst="$$(dirname "$$1")/.cargo-checksum.json"; [ -f "$$dst" ] || cp "$$1" "$$dst"' sh {} \; ; \
	fi
	@if [ -d $(SRC_DIR)/vendor ]; then \
		find $(SRC_DIR)/vendor -mindepth 2 -maxdepth 2 -name cargo-vcs-info.json \
			-exec sh -c 'dst="$$(dirname "$$1")/.cargo_vcs_info.json"; [ -f "$$dst" ] || cp "$$1" "$$dst"' sh {} \; ; \
	fi

kernel-rv: prepare-vendor
	@$(MAKE) -C $(SRC_DIR) $(SRC_MAKE_ARGS) OUT_CONFIG=$(RV_OUT_CONFIG) $@
	@cp $(SRC_DIR)/$@ $(ROOT_DIR)/$@

kernel-la: prepare-vendor
	@$(MAKE) -C $(SRC_DIR) $(SRC_MAKE_ARGS) OUT_CONFIG=$(LA_OUT_CONFIG) $@
	@cp $(SRC_DIR)/$@ $(ROOT_DIR)/$@

run:
	@$(MAKE) -C $(SRC_DIR) $(SRC_MAKE_ARGS) OUT_CONFIG=$(RUN_OUT_CONFIG) $@

# clean forwards through src/Makefile so src/make is reached consistently
# and src-level config/artifact cleanup also runs.
clean:
	@$(MAKE) -C $(SRC_DIR) $(SRC_MAKE_ARGS) OUT_CONFIG=$(SRC_DIR)/.axconfig.toml clean
	@rm -f $(ROOT_DIR)/kernel-rv $(ROOT_DIR)/kernel-la \
		$(SRC_DIR)/kernel-rv $(SRC_DIR)/kernel-la \
		$(SRC_DIR)/.axconfig-riscv64.toml $(SRC_DIR)/.axconfig-riscv64.old.toml \
		$(SRC_DIR)/.axconfig-loongarch64.toml $(SRC_DIR)/.axconfig-loongarch64.old.toml

.PHONY: all prepare-vendor kernel-rv kernel-la run clean
