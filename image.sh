cargo build && cargo bootimage --target x86_64-rustnix.json
if [[ ! $? -eq 0 ]]; then
    echo "Error: Build failed!"
    exit 1
fi

kernel="target/x86_64-rustnix/debug/bootimage-rustnix.bin"
disk_image="target/x86_64-rustnix/debug/disk.img"

if [[ ! -f $disk_image ]]; then
    qemu-img create -f raw $disk_image 64M
fi

# Run QEMU with the kernel binary and the disk image as a second drive
qemu-system-x86_64 -drive file=$kernel,format=raw -drive file=$disk_image,format=raw -serial stdio
