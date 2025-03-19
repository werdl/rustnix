#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![feature(core_intrinsics)]
#![test_runner(rustnix::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::{intrinsics::unreachable, panic::PanicInfo};

use bootloader::{entry_point, BootInfo};
#[allow(unused_imports)]
use rustnix::kprintln;
use rustnix::{internal::task::{executor::Executor, keyboard, Task}};


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

    let mut executor = Executor::new();
    executor.spawn(Task::new(keyboard::handle_keypresses()));
    executor.run();

    unreachable!(); // executor.run() should never return



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
