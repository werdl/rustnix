#!/bin/bash

set -e  # Exit on any error

# Step 1: Compile the kernel
if [[ -n "$1" ]]; then
    cargo build --target x86_64-rustnix.json --features $(echo $1_log)
else
    cargo build --target x86_64-rustnix.json
fi

cargo bootimage --target x86_64-rustnix.json --features $(echo $1_log)

# Ensure the build was successful
if [[ ! $? -eq 0 ]]; then
    echo "Error: Build failed!"
    exit 1
fi

kernel="target/x86_64-rustnix/debug/bootimage-rustnix.bin"
disk_image="target/x86_64-rustnix/debug/disk.img"

# Create a disk image
dd if=/dev/zero of=$disk_image bs=1M count=64

# Run QEMU with the kernel binary and the disk image as a second drive
qemu-system-x86_64 -drive file=$kernel,format=raw  -drive file=$disk_image,format=raw -serial stdio
