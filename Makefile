.DEFAULT_GOAL := all

ROOT_DIR := $(CURDIR)
SRC_DIR := $(ROOT_DIR)/src

export RUSTUP_DIST_SERVER := https://mirrors.ustc.edu.cn/rust-static
export RUSTUP_UPDATE_ROOT := https://mirrors.ustc.edu.cn/rust-static/rustup
export CARGO_NET_OFFLINE := true

all: kernel-rv kernel-la

kernel-rv:
	@$(MAKE) -C $(SRC_DIR) A=$(SRC_DIR) TARGET_DIR=$(SRC_DIR)/target OUT_CONFIG=$(SRC_DIR)/.axconfig.toml $@
	@cp $(SRC_DIR)/$@ $(ROOT_DIR)/$@

kernel-la:
	@$(MAKE) -C $(SRC_DIR) A=$(SRC_DIR) TARGET_DIR=$(SRC_DIR)/target OUT_CONFIG=$(SRC_DIR)/.axconfig.toml $@
	@cp $(SRC_DIR)/$@ $(ROOT_DIR)/$@

run:
	@$(MAKE) -C $(SRC_DIR) A=$(SRC_DIR) TARGET_DIR=$(SRC_DIR)/target OUT_CONFIG=$(SRC_DIR)/.axconfig.toml $@

clean:
	@$(MAKE) -C $(SRC_DIR)/make \
		APP=$(SRC_DIR) \
		TARGET_DIR=$(SRC_DIR)/target \
		OUT_CONFIG=$(SRC_DIR)/.axconfig.toml \
		$@
	@rm -f $(ROOT_DIR)/kernel-rv $(ROOT_DIR)/kernel-la \
		$(SRC_DIR)/kernel-rv $(SRC_DIR)/kernel-la

.PHONY: all kernel-rv kernel-la run clean
