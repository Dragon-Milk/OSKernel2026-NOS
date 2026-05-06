# Build Options
export ARCH := riscv64
export LOG := warn
export DWARF := y
export MEMTRACK := n

# QEMU Options
export BLK := y
export NET := y
export VSOCK := n
export MEM := 1G
export ICOUNT := n

# Generated Options
export A := $(CURDIR)
export TARGET_DIR ?= $(CURDIR)/target
export OUT_CONFIG ?= $(CURDIR)/.axconfig.toml
export NO_AXSTD := y
export AX_LIB := axfeat
export APP_FEATURES := qemu

ifeq ($(MEMTRACK), y)
	APP_FEATURES += starry-api/memtrack
endif

default: build

APP_NAME := $(notdir $(CURDIR))
RV_ELF := $(APP_NAME)_riscv64-qemu-virt.elf
LA_ELF := $(APP_NAME)_loongarch64-qemu-virt.elf
RV_LD_SCRIPT := $(CURDIR)/make/linker_riscv64-qemu-virt-contest.lds
LA_LD_SCRIPT := $(CURDIR)/make/linker_loongarch64-qemu-contest.lds

all: kernel-rv kernel-la

kernel-rv:
	@$(MAKE) ARCH=riscv64 BUS=mmio LD_SCRIPT=$(RV_LD_SCRIPT) build
	@cp $(RV_ELF) $@

kernel-la:
	@$(MAKE) ARCH=loongarch64 BUS=pci LD_SCRIPT=$(LA_LD_SCRIPT) build
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
	@rm -f kernel-rv kernel-la

build run debug disasm: defconfig
	@$(MAKE) -C make $@

ci-test:
	./scripts/ci-test.py $(ARCH)

# Aliases
rv:
	$(MAKE) ARCH=riscv64 run

la:
	$(MAKE) ARCH=loongarch64 run

vf2:
	$(MAKE) ARCH=riscv64 APP_FEATURES=vf2 MYPLAT=axplat-riscv64-visionfive2 BUS=mmio build

.PHONY: all kernel-rv kernel-la build run justrun debug disasm clean
