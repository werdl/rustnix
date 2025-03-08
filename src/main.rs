#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rustnix::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;
use alloc::{boxed::Box, rc::Rc, vec, vec::Vec};
use bootloader::{BootInfo, entry_point};
use core::panic::PanicInfo;
use rustnix::{
    allocator, ata, exit_qemu, memory::{self, BootInfoFrameAllocator}, print, println, serial_print, serial_println, task::{executor::Executor, keyboard, simple_executor::SimpleExecutor, Task}, QemuExitCode
};
use x86_64::{VirtAddr, structures::paging::Page};

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    rustnix::hlt_loop()
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rustnix::test_panic_handler(info)
}

async fn async_number() -> u32 {
    42
}

async fn example_task() {
    let number = async_number().await;
    println!("async number: {}", number);
}

entry_point!(kmain);

fn kmain(boot_info: &'static BootInfo) -> ! {
    println!("Hello, World!");

    rustnix::init(boot_info);


 

    // let mut executor = Executor::new();
    // executor.spawn(Task::new(keyboard::print_keypresses()));
    // executor.run();


    let mut buf = vec![0;512];
    
    ata::read(0, 1, 1, &mut buf);

    println!("Data read from sector 0: {:?}", buf);

    // now write some data to the disk
    let data = b"Hello from the other side!";
    // puff up the data to 512 bytes
    let mut data = data.to_vec();
    data.resize(512, 0);

    ata::write(0, 1, 1, &data);

    // read the data back
    let mut buffer = vec![0; 512];
    ata::read(0, 1, 1, &mut buffer);

    println!("Data read from sector 1: {:?}", buffer);
    

    // print the data
    // println!("Data read from sector {}: {:?}", sector, core::str::from_utf8(&buffer).unwrap());

    #[cfg(test)]
    test_main();

    println!("It did not crash!");

    rustnix::hlt_loop()
}