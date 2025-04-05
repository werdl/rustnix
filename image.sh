#!/bin/bash

set -e  # Exit on any error

trap 'echo failed && cd ../' ERR # cd back to parent on error

cd kernel

# Step 1: Compile the kernel
if [[ -n "$1" ]]; then
    cargo build --target x86_64-rustnix.json --features $(echo $1_log);
else
    cargo build --target x86_64-rustnix.json
fi

if [[ -n "$1" ]]; then
    cargo bootimage --target x86_64-rustnix.json --features $(echo $1_log)
else
    cargo bootimage --target x86_64-rustnix.json
fi

cd ../

# Step 2: Build disk image

cd fs-loader
touch disk.img
cargo run
cd ../


trap - ERR # we don't want to go any further

# Ensure the build was successful
if [[ ! $? -eq 0 ]]; then
    echo "Error: Build failed!"
    exit 1
fi

kernel="kernel/target/x86_64-rustnix/debug/bootimage-rustnix.bin"
disk_image="fs-loader/disk.img"

# Create a disk image
#dd if=/dev/zero of=$disk_image bs=1M count=64

# Run QEMU with the kernel binary and the disk image as a second drive
qemu-system-x86_64 -drive file=$kernel,format=raw  -drive file=$disk_image,format=raw -serial stdio
