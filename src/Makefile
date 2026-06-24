# Build Options
export ARCH := riscv64
export LOG := warn
export DWARF := y
export MEMTRACK := n
export RUSTUP_DIST_SERVER := https://mirrors.ustc.edu.cn/rust-static
export RUSTUP_UPDATE_ROOT := https://mirrors.ustc.edu.cn/rust-static/rustup
export CARGO_NET_OFFLINE := true

# QEMU Options
export BLK := y
export NET := y
export VSOCK := n
export MEM := 1G
export ICOUNT := n
export TEST_PROFILE ?= full-safe
export LTP_CASE_LIST ?=

# Generated Options
export A := $(CURDIR)
export TARGET_DIR ?= $(CURDIR)/target
export OUT_CONFIG ?= $(CURDIR)/.axconfig-$(ARCH).toml
export RV_OUT_CONFIG ?= $(CURDIR)/.axconfig-riscv64.toml
export LA_OUT_CONFIG ?= $(CURDIR)/.axconfig-loongarch64.toml
export NO_AXSTD := y
export AX_LIB := axfeat
export APP_FEATURES := qemu

ifeq ($(MEMTRACK), y)
	APP_FEATURES += memtrack
endif

default: build

APP_NAME := $(notdir $(CURDIR))
RV_ELF := $(APP_NAME)_riscv64-qemu-virt.elf
LA_ELF := $(APP_NAME)_loongarch64-qemu-virt.elf
RV_LD_SCRIPT := $(CURDIR)/make/linker_riscv64-qemu-virt-contest.lds
LA_LD_SCRIPT := $(CURDIR)/make/linker_loongarch64-qemu-contest.lds

ifeq ($(ARCH),loongarch64)
export LD_SCRIPT := $(LA_LD_SCRIPT)
endif

all: kernel-rv kernel-la

kernel-rv:
	@$(MAKE) ARCH=riscv64 BUS=mmio LD_SCRIPT=$(RV_LD_SCRIPT) OUT_CONFIG=$(RV_OUT_CONFIG) build
	@cp $(RV_ELF) $@

kernel-la:
	@$(MAKE) ARCH=loongarch64 BUS=pci LD_SCRIPT=$(LA_LD_SCRIPT) OUT_CONFIG=$(LA_OUT_CONFIG) build
	@cp $(LA_ELF) $@

ROOTFS_URL = https://github.com/Starry-OS/rootfs/releases/download/20260214
ROOTFS_IMG = rootfs-$(ARCH).img

rootfs:
	@if [ ! -f $(ROOTFS_IMG) ]; then \
		echo "Image not found, downloading..."; \
		curl -f -L $(ROOTFS_URL)/$(ROOTFS_IMG).xz -O; \
		xz -d $(ROOTFS_IMG).xz; \
	fi
	@cp $(ROOTFS_IMG) make/disk.img

img:
	@echo -e "\033[33mWARN: The 'img' target is deprecated. Please use 'rootfs' instead.\033[0m"
	@$(MAKE) --no-print-directory rootfs

defconfig justrun:
	@$(MAKE) -C make $@

clean:
	@$(MAKE) -C make $@
	@rm -f kernel-rv kernel-la \
		$(RV_OUT_CONFIG) $(CURDIR)/.axconfig-riscv64.old.toml \
		$(LA_OUT_CONFIG) $(CURDIR)/.axconfig-loongarch64.old.toml

build run debug disasm: defconfig
	@{ \
		printf 'pub const TEST_PROFILE_ENV: &str = r###"TEST_PROFILE=%s"###;\n' "$(TEST_PROFILE)"; \
		printf 'pub const LTP_TIMEOUT_ENV: &str = r###"LTP_TIMEOUT=%s"###;\n' "$(LTP_TIMEOUT)"; \
		printf 'pub const LTP_CASE_LIST_ENV: &str = r###"LTP_CASE_LIST=%s"###;\n' "$(LTP_CASE_LIST)"; \
	} > init/env.rs.tmp
	@if cmp -s init/env.rs.tmp init/env.rs; then \
		rm -f init/env.rs.tmp; \
	else \
		mv init/env.rs.tmp init/env.rs; \
		touch init/main.rs; \
	fi
	@$(MAKE) -C make $@

ci-test:
	./scripts/ci-test.py $(ARCH)

# Aliases
rv:
	$(MAKE) ARCH=riscv64 OUT_CONFIG=$(RV_OUT_CONFIG) run

la:
	$(MAKE) ARCH=loongarch64 OUT_CONFIG=$(LA_OUT_CONFIG) run

vf2:
	$(MAKE) ARCH=riscv64 APP_FEATURES=vf2 MYPLAT=axplat-riscv64-visionfive2 BUS=mmio build

.PHONY: all kernel-rv kernel-la build run justrun debug disasm clean
