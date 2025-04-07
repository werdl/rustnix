use alloc::{vec, vec::Vec};

use crate::internal::{file::FileFlags, fs::get_buffer_size};

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
    let path = &file::canonicalise(path);
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
        0 => crate::internal::acpi::shutdown(), // ACPI shutdown
        1 => unsafe { asm!("xor rax, rax", "mov cr3, rax") }, // reboot
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

/// sleep for a number of nanoseconds (SLEEP)
pub fn sleep(nanos: usize) -> isize {
    // sleep() accepts milliseconds, so convert nanoseconds to milliseconds
    let millis = nanos as f64 / 1_000_000.0;
    crate::internal::clk::sleep(millis);
    0
}

/// wait for a number of nanoseconds (WAIT)
pub fn wait(nanos: usize) -> isize {
    // wait() accepts nano seconds
    crate::internal::clk::wait(nanos as u64);
    0
}

/// Seek to a position in a file descriptor (SEEK)
pub fn seek(fd: usize, pos: usize) -> isize {
    let mut files = FILES.lock();

    let resource: Option<&mut File> = files.get_mut(&(fd as isize));

    match resource {
        Some(resource) => {
            match resource.seek(pos) {
                Ok(new_pos) => new_pos as isize,
                Err(err) => {
                    set_errno(err.into());
                    -1
                }
            }
        },
        None => {
            set_errno(Error::EBADF);
            -1
        }
    }
}

/// get the number of nanoseconds since boot (NANOS)
pub fn nanos() -> usize {
    crate::internal::clk::get_boot_time_ns() as usize // safe as we target x86_64
}

/// get the number of seconds since 1970-01-01T00:00:00Z (TIME)
pub fn time() -> u64 {
    crate::internal::clk::get_unix_time()
}

/// spawn a new process (SPAWN)
pub fn spawn(path: &str, args: &[&str]) -> isize {
    let path = crate::internal::file::canonicalise(path);

    // use open syscall to open the file
    let fd = open(&path, FileFlags::Read as u8);

    if fd < 0 {
        return -1;
    }


    let buf_size = get_buffer_size(0, 1, &path);
    if buf_size.is_err() {
        close(fd as usize);
        set_errno(Error::EINVAL);
        return -1;
    }

    // read the file into a buffer
    let mut buf = vec![0; buf_size.unwrap()];
    let bytes_read = read(fd as usize, &mut buf);
    if bytes_read < 0 {
        close(fd as usize);
        return -1;
    }
    let bytes_read = bytes_read as usize;
    let buf = &buf[..bytes_read];
    close(fd as usize);
    // parse the buffer into a process

    let args_ptr = args.as_ptr() as usize;

    let args_len = args.len();

    if let Err(code) = crate::internal::process::Process::spawn(&buf, args_ptr, args_len) {
        code as isize
    } else {
        unreachable!(); // The kernel switched to the child process
    }
}
