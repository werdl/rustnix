use super::*;

fn set_errno(errno: Error) {
    *ERRNO.lock() = errno as usize;
}

/// allocate memory (ALLOC)
pub fn alloc(size: usize, align: usize) -> *mut u8 {
    let layout = core::alloc::Layout::from_size_align(size, align).unwrap();
    unsafe { alloc::alloc::alloc(layout) }
}

/// free memory (FREE)
pub fn free(ptr: *mut u8, size: usize, align: usize) {
    let layout = core::alloc::Layout::from_size_align(size, align).unwrap();
    unsafe { alloc::alloc::dealloc(ptr, layout) }
}

/// test the alloc and free functions
#[test_case]
fn test_alloc_free() {
    let heap_value = alloc(1024, 1);
    assert!(!heap_value.is_null());
    free(heap_value, 1024, 1);
}

/// initialize the syscall interface (currently just initializes block devices)
pub fn init() {
    trace!("Initializing syscall interface");
    // initialize all block devices (ie. pop a fd into FILES for each block device)
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut files = FILES.lock();

        for i in 0..io::NUM_DEVICES {
            let device = Device::try_from((i as u8, 0)).unwrap();
            files.insert(i as isize, File::Device(device));
        }
    });
}

/// device path should be something like "/dev/sda" NOT "sda"
fn open_block_device(device_path: &str) -> isize {
    match device_path {
        "/dev/stdin" => io::STDIN as isize,
        "/dev/stdout" => io::STDOUT as isize,
        "/dev/stderr" => io::STDERR as isize,
        "/dev/null" => io::NULL as isize,
        "/dev/zero" => io::ZERO as isize,
        "/dev/random" => io::RAND as isize,
        _ => {
            warn!("Unknown device: {}, failing OPEN", device_path);
            -1
        }
    }
}

/// open a file (OPEN)
pub fn open(path: &str, flags: u8) -> isize {
    let path = &file::absolute_path(path);
    if path.starts_with("/dev/") {
        return open_block_device(path);
    }

    let file_handle = FileHandle::new_with_likely_fs(path.to_string(), flags);

    if file_handle.is_err() {
        set_errno(file_handle.unwrap_err().into());
        return -1;
    }

    let resource = File::File(file_handle.unwrap());

    let mut files = FILES.lock();

    let fd = files.len() as isize;

    files.insert(fd, resource);
    fd
}

/// write to a file descriptor (WRITE)
pub fn write(fd: usize, buf: &[u8]) -> isize {
    let mut files = FILES.lock();

    let resource: Option<&mut File> = files.get_mut(&(fd as isize));

    match resource {
        Some(resource) => {
            match resource.write(buf) {
                Ok(bytes_written) => bytes_written as isize,
                Err(err) => {
                    set_errno(err.into());
                    -1
                }
            }
        },
        None => {
            set_errno(Error::EBADF);
            -1
        },
    }
}

/// read from a file descriptor (READ)
pub fn read(fd: usize, buf: &mut [u8]) -> isize {
    let mut files = FILES.lock();

    let resource: Option<&mut File> = files.get_mut(&(fd as isize));

    match resource {
        Some(resource) => {
            match resource.read(buf) {
                Ok(bytes_read) => bytes_read as isize,
                Err(err) => {
                    set_errno(err.into());
                    -1
                }
            }
        },
        None => {
            set_errno(Error::EBADF);
            -1
        },
    }
}

/// close a file descriptor (CLOSE)
pub fn close(fd: usize) -> isize {
    let mut files = FILES.lock();

    let resource: Option<File> = files.remove(&(fd as isize));

    match resource {
        Some(mut resource) => {
            match resource.close() {
                Ok(()) => 0,
                Err(err) => {
                    set_errno(err.into());
                    -1
                }
            }
        },
        None => {
            set_errno(Error::EBADF);
            -1
        },
    }
}

/// flush a file descriptor (FLUSH)
pub fn flush(fd: usize) -> isize {
    let mut files = FILES.lock();

    let resource: Option<&mut File> = files.get_mut(&(fd as isize));

    match resource {
        Some(resource) => {
            match resource.flush() {
                Ok(()) => 0,
                Err(err) => {
                    set_errno(err.into());
                    -1
                }
            }
        },
        None => {
            set_errno(Error::EBADF);
            -1
        },
    }
}

/// stop the system (STOP)
pub fn stop(stop_type: usize) -> isize {
    match stop_type {
        0 => crate::internal::acpi::shutdown(),
        1 => unsafe { asm!("xor rax, rax", "mov cr3, rax") },
        _ => {
            warn!("Unknown stop type: {}", stop_type);
            set_errno(Error::EINVAL);
            return -1;
        }
    }

    return 0;
}

/// poll a file descriptor (POLL)
pub fn poll(fd: usize, io_event: usize) -> isize {
    let mut files = FILES.lock();

    let resource: Option<&mut File> = files.get_mut(&(fd as isize));

    let io_event = match io_event {
        1 => IOEvent::Read,
        2 => IOEvent::Write,
        _ => {
            set_errno(Error::EINVAL);
            return -1;
        }
    };

    match resource {
        Some(resource) => {
            resource.poll(io_event) as isize
        },
        None => {
            set_errno(Error::EBADF);
            -1
        }
    }
}

pub fn sleep(nanos: usize) -> isize {
    // sleep() accepts milliseconds, so convert nanoseconds to milliseconds
    let millis = nanos as f64 / 1_000_000.0;
    crate::internal::clk::sleep(millis);
    0
}

pub fn wait(nanos: usize) -> isize {
    // wait() accepts nano seconds
    crate::internal::clk::wait(nanos as u64);
    0
}
