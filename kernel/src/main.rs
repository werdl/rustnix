#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rustnix::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::panic::PanicInfo;

use bootloader::{BootInfo, entry_point};
use rustnix::{syscall, EXEC};

#[allow(unused_imports)]
use rustnix::kprintln;

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


    // let mut buf = vec![0; 512];
    // // insert string to write into buffer
    // let data = b"Hello from the other side!";
    // buf[..data.len()].copy_from_slice(data);

    // syscall!(WRITE, 1, buf.as_ptr() as usize, 26); // write to /dev/stdout

    let args: &[&str] = &[];
    let args_ptr = args.as_ptr() as usize;
    let args_len = args.len();

    let path = "/bin/hello.bin";
    let path_ptr = path.as_ptr() as usize;
    let path_len = path.len();

    syscall!(EXEC, path_ptr, path_len, args_ptr, args_len);

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

// make sure heap init function is calling init_process_addr: FIXME
