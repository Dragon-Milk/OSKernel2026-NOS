.DEFAULT_GOAL := all

ROOT_DIR := $(CURDIR)
SRC_DIR := $(ROOT_DIR)/src
RV_OUT_CONFIG := $(SRC_DIR)/.axconfig-riscv64.toml
LA_OUT_CONFIG := $(SRC_DIR)/.axconfig-loongarch64.toml
RUN_OUT_CONFIG := $(if $(filter loongarch64,$(ARCH)),$(LA_OUT_CONFIG),$(RV_OUT_CONFIG))

export RUSTUP_DIST_SERVER := https://mirrors.ustc.edu.cn/rust-static
export RUSTUP_UPDATE_ROOT := https://mirrors.ustc.edu.cn/rust-static/rustup
export CARGO_NET_OFFLINE := true

all: prepare-vendor kernel-rv kernel-la
	@rm -rf $(SRC_DIR)/target
	@rm -f $(SRC_DIR)/*.elf $(SRC_DIR)/*.bin
	@rm -f $(SRC_DIR)/.axconfig.toml $(SRC_DIR)/.axconfig-*.toml $(SRC_DIR)/.axconfig-*.old.toml
	@rm -f $(SRC_DIR)/kernel-rv $(SRC_DIR)/kernel-la

prepare-vendor:
	@mkdir -p $(SRC_DIR)/.cargo
	@if [ -f $(SRC_DIR)/cargo-config.toml ] && [ ! -f $(SRC_DIR)/.cargo/config.toml ]; then cp $(SRC_DIR)/cargo-config.toml $(SRC_DIR)/.cargo/config.toml; fi
	@if [ -f $(SRC_DIR)/cargo-config ] && [ ! -f $(SRC_DIR)/.cargo/config ]; then cp $(SRC_DIR)/cargo-config $(SRC_DIR)/.cargo/config; fi
	@if [ -d $(SRC_DIR)/vendor ]; then find $(SRC_DIR)/vendor -mindepth 2 -maxdepth 2 -name cargo-checksum.json -exec sh -c 'dst="$$(dirname "$$1")/.cargo-checksum.json"; [ -f "$$dst" ] || cp "$$1" "$$dst"' sh {} \; ; fi
	@if [ -d $(SRC_DIR)/vendor ]; then find $(SRC_DIR)/vendor -mindepth 2 -maxdepth 2 -name cargo-vcs-info.json -exec sh -c 'dst="$$(dirname "$$1")/.cargo_vcs_info.json"; [ -f "$$dst" ] || cp "$$1" "$$dst"' sh {} \; ; fi

kernel-rv kernel-la: prepare-vendor

kernel-rv:
	@$(MAKE) -C $(SRC_DIR) A=$(SRC_DIR) TARGET_DIR=$(SRC_DIR)/target OUT_CONFIG=$(RV_OUT_CONFIG) $@
	@cp $(SRC_DIR)/$@ $(ROOT_DIR)/$@

kernel-la:
	@$(MAKE) -C $(SRC_DIR) A=$(SRC_DIR) TARGET_DIR=$(SRC_DIR)/target OUT_CONFIG=$(LA_OUT_CONFIG) $@
	@cp $(SRC_DIR)/$@ $(ROOT_DIR)/$@

run:
	@$(MAKE) -C $(SRC_DIR) A=$(SRC_DIR) TARGET_DIR=$(SRC_DIR)/target OUT_CONFIG=$(RUN_OUT_CONFIG) $@

perf-rv:
	@$(MAKE) -C $(SRC_DIR) A=$(SRC_DIR) TARGET_DIR=$(SRC_DIR)/target OUT_CONFIG=$(RV_OUT_CONFIG) ARCH=riscv64 TEST_PROFILE=perf run

perf-la:
	@$(MAKE) -C $(SRC_DIR) A=$(SRC_DIR) TARGET_DIR=$(SRC_DIR)/target OUT_CONFIG=$(LA_OUT_CONFIG) ARCH=loongarch64 TEST_PROFILE=perf run

clean:
	@$(MAKE) -C $(SRC_DIR)/make \
		APP=$(SRC_DIR) \
		TARGET_DIR=$(SRC_DIR)/target \
		OUT_CONFIG=$(SRC_DIR)/.axconfig.toml \
		$@
	@rm -f $(ROOT_DIR)/kernel-rv $(ROOT_DIR)/kernel-la \
		$(SRC_DIR)/kernel-rv $(SRC_DIR)/kernel-la \
		$(SRC_DIR)/.axconfig-riscv64.toml $(SRC_DIR)/.axconfig-riscv64.old.toml \
		$(SRC_DIR)/.axconfig-loongarch64.toml $(SRC_DIR)/.axconfig-loongarch64.old.toml

.PHONY: all prepare-vendor kernel-rv kernel-la run clean perf-rv perf-la
