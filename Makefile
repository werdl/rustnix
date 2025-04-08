# Makefile for building and running the Rust-based kernel and disk image

TARGET_NAME = x86_64-rustnix
TARGET = $(TARGET_NAME).json
KERNEL_DIR = kernel
FS_LOADER_DIR = fs-loader
KERNEL_BIN = $(KERNEL_DIR)/target/$(TARGET_NAME)/debug/bootimage-rustnix.bin
DISK_IMG = $(FS_LOADER_DIR)/disk.img
ASM_OUT_DIR = disk/bin
ASM_FILES = $(wildcard disk/src/*.S)

FEATURES ?= debug_log

.PHONY: all assemble kernel bootimage fs-loader run clean

all: run

assemble:
	@mkdir -p $(ASM_OUT_DIR)
	@echo "Assembling assembly files..."
	@for file in $(ASM_FILES); do \
		nasm $$file -o $(ASM_OUT_DIR)/$$(basename $$file .S).bin; \
		echo -ne '\x7FBIN' | cat - $(ASM_OUT_DIR)/$$(basename $$file .S).bin > $(ASM_OUT_DIR)/$$(basename $$file .S).bin.tmp && mv $(ASM_OUT_DIR)/$$(basename $$file .S).bin.tmp $(ASM_OUT_DIR)/$$(basename $$file .S).bin; \
	done
	@echo "Assembly completed."

	@echo "Compiling C files..."
	@for file in $(wildcard disk/src/c/*.c); do \
		tcc $$file -o $(ASM_OUT_DIR)/$$(basename $$file .c).o -nostdlib -static -nostdinc; \
		echo "Compiled $$file to $(ASM_OUT_DIR)/$$(basename $$file .c).o"; \
	done
	@echo "C file compilation completed."


kernel: assemble
	@echo "Building kernel..."
	@cd $(KERNEL_DIR) && cargo build --target $(TARGET) --features $(FEATURES)
	@echo "Kernel built successfully."

bootimage: kernel
	@echo "Creating bootable image..."
	@cd $(KERNEL_DIR) && cargo bootimage --target $(TARGET) --features $(FEATURES)
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
