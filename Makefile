# Makefile for building and running the Rust-based kernel and disk image

TARGET_NAME = x86_64-rustnix
TARGET = $(TARGET_NAME).json
KERNEL_DIR = kernel
FS_LOADER_DIR = fs-loader
KERNEL_BIN = $(KERNEL_DIR)/target/$(TARGET_NAME)/debug/bootimage-rustnix.bin
DISK_IMG = $(FS_LOADER_DIR)/disk.img

FEATURES ?=

.PHONY: all kernel bootimage fs-loader run clean

all: run

kernel:
	@echo "Building kernel..."
	@cd $(KERNEL_DIR) && cargo build --target $(TARGET) $(FEATURES)
	@echo "Kernel built successfully."

bootimage: kernel
	@echo "Creating bootable image..."
	@cd $(KERNEL_DIR) && cargo bootimage --target $(TARGET) $(FEATURES)
	@echo "Bootable image created successfully."

fs-loader:
	@echo "Loading files"
	@cd $(FS_LOADER_DIR) && cargo run
	@echo "Files loaded successfully."

qemu: bootimage fs-loader
	@echo "Running QEMU..."
	qemu-system-x86_64 -drive file=$(KERNEL_BIN),format=raw  -drive file=$(DISK_IMG),format=raw -serial stdio
	@echo "\nQEMU exited."

test: bootimage
	@echo "Running tests..."
	@cd $(KERNEL_DIR) && cargo test
	@echo "Tests completed."

run: bootimage fs-loader qemu
