#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rustnix::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::panic::PanicInfo;

use alloc::vec;
use bootloader::{BootInfo, entry_point};
use rustnix::{internal::file::FileFlags, syscall};

#[allow(unused_imports)]
use rustnix::kprintln;
use rustnix::internal::syscall;

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kprintln!("{}", info);
    rustnix::hlt_loop()
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rustnix::test_panic_handler(info)
}

entry_point!(kmain);

fn kmain(boot_info: &'static BootInfo) -> ! {
    rustnix::init(boot_info);

    #[cfg(test)]
    test_main();


    // print inode table
    kprintln!("inode table (only first 10 entries): {:?}", rustnix::internal::fs::FILESYSTEMS.lock().get(&(0,1)).unwrap().phys_fs.data_blocks[2].data[0..10].to_vec());

    // read from file1
    let mut buf = vec![0; 512];
    let file1 = syscall::service::open("file1", FileFlags::Read as u8);
    kprintln!("errno: {}", syscall!(syscall::GETERRNO));
    let res = syscall::service::read(file1 as usize, &mut buf);
    kprintln!("fd, res: {}, {}", file1, res);

    kprintln!("errno: {}", syscall!(syscall::GETERRNO));

    // kprintln!("Data read from file1: {:?}", buf);

    loop {}

    // let mut buf = vec![0;512];

    // ata::read(0, 1, 1, &mut buf);

    // println!("Data read from sector 0: {:?}", buf);

    // // now write some data to the disk
    // let data = b"Hello from the other side!";
    // // puff up the data to 512 bytes
    // let mut data = data.to_vec();
    // data.resize(512, 0);

    // ata::write(0, 1, 1, &data);

    // // read the data back
    // let mut buffer = vec![0; 512];
    // ata::read(0, 1, 1, &mut buffer);

    // println!("Data read from sector 1: {:?}", buffer);

    // print the data
    // println!("Data read from sector {}: {:?}", sector, core::str::from_utf8(&buffer).unwrap());
}
