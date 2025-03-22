use alloc::string::ToString;
use spin::Mutex;

use crate::internal::{
    console::Console,
    devices::{null::Null, rand::Rand, zero::Zero},
    file::Stream,
    fs::{FileHandle, FsError},
    io::{Device, FILES, File},
};


/// syscall.rs - implements system calls, including the handler (which will be invoked from the interrupt file)

pub static ERRNO: Mutex<u8> = Mutex::new(0);

pub enum Error {
    /// Not super-user
    EPERM = 1,

    /// No such file or directory
    ENOENT = 2,

    /// I/O error
    EIO = 5,

    /// argument list too long
    E2BIG = 7,

    /// Exec format error
    ENOEXEC = 8,

    /// Bad file number
    EBADF = 9,

    /// not enough core (memory)
    ENOMEM = 12,

    /// Permission denied
    EACCES = 13,

    /// file exists
    EEXIST = 17,

    /// No such device
    ENODEV = 19,

    /// Not a directory
    ENOTDIR = 20,

    /// Is a directory
    EISDIR = 21,

    /// Invalid argument
    EINVAL = 22,

    /// too many open files in system
    ENFILE = 23,

    /// file too large
    EFBIG = 27,

    /// No space left on device
    ENOSPC = 28,

    /// Read-only file system
    EROFS = 30,

    /// no csi structure available
    ENOCSI = 43,

    /// function not implemented
    ENOSYS = 88,

    /// file/path name too long
    ENAMETOOLONG = 91,

    /// value too large for defined data type
    EOVERFLOW = 139,
}

fn set_errno(errno: Error) {
    *ERRNO.lock() = errno as u8;
}

pub const READ: u64 = 0x1;
pub const WRITE: u64 = 0x2;
pub const OPEN: u64 = 0x3;
pub const CLOSE: u64 = 0x4;
pub const FLUSH: u64 = 0x5;
pub const EXIT: u64 = 0x6;
pub const SLEEP: u64 = 0x7;
pub const WAIT: u64 = 0x8;
pub const GETPID: u64 = 0x9;
pub const EXEC: u64 = 0xA;
pub const FORK: u64 = 0xB;
pub const GETTID: u64 = 0xC;
pub const STOP: u64 = 0xD;
pub const WAITPID: u64 = 0xE;
pub const CONNECT: u64 = 0xF;
pub const ACCEPT: u64 = 0x10;
pub const LISTEN: u64 = 0x11;
pub const ALLOC: u64 = 0x12;
pub const FREE: u64 = 0x13;
pub const KIND: u64 = 0x14;
pub const GETERRNO: u64 = 0x15;

pub fn get_errno() -> u8 {
    *ERRNO.lock()
}



pub fn alloc(size: usize) -> *mut u8 {
    let layout = core::alloc::Layout::from_size_align(size, 1).unwrap();
    unsafe { alloc::alloc::alloc(layout) }
}

pub fn free(ptr: *mut u8, size: usize) {
    let layout = core::alloc::Layout::from_size_align(size, 1).unwrap();
    unsafe { alloc::alloc::dealloc(ptr, layout) }
}

#[test_case]
fn test_alloc_free() {
    let heap_value = alloc(1024);
    assert!(!heap_value.is_null());
    free(heap_value, 1024);
}

pub fn open(path: &str, flags: u8) -> i8 {
    let resource = match path {
        "/dev/null" => File::Device(Device::Null(Null::new(flags))),
        "/dev/zero" => File::Device(Device::Zero(Zero::new(flags))),
        "/dev/random" => File::Device(Device::Rand(Rand::new(flags))),
        "/dev/stdin" => File::StdStream(Console::new()),
        "/dev/stdout" => File::StdStream(Console::new()),
        "/dev/stderr" => File::StdStream(Console::new()),
        _ => {
            // assume it's a file
            let file_handle = FileHandle::new_with_likely_fs(path.to_string(), flags);

            if file_handle.is_err() {
                set_errno(file_handle.unwrap_err().into());
                return -1;
            }
            File::File(file_handle.unwrap())
        }
    };

    let mut files = FILES.lock();

    let fd = files.len() as i8;

    files.insert(fd, resource);
    fd
}

pub fn write(fd: u8, buf: &[u8]) -> Result<usize, crate::internal::file::FileError> {
    let mut files = FILES.lock();

    let resource: Option<&mut File> = files.get_mut(&(fd as i8));

    match resource {
        Some(resource) => resource.write(buf),
        None => Err(crate::internal::file::FileError::WriteError(FsError::InvalidFileDescriptor.into())),
    }
}

pub fn read(fd: u8, buf: &mut [u8]) -> Result<usize, crate::internal::file::FileError> {
    let mut files = FILES.lock();

    let resource: Option<&mut File> = files.get_mut(&(fd as i8));

    match resource {
        Some(resource) => resource.read(buf),
        None => Err(crate::internal::file::FileError::ReadError(FsError::InvalidFileDescriptor.into())),
    }
}

pub fn close(fd: u8) -> Result<(), crate::internal::file::FileError> {
    let mut files = FILES.lock();

    let resource: Option<File> = files.remove(&(fd as i8));

    match resource {
        Some(mut resource) => resource.close(),
        None => Err(crate::internal::file::FileError::CloseError(FsError::InvalidFileDescriptor.into(),
        )),
    }
}

pub fn flush(fd: u8) -> Result<(), crate::internal::file::FileError> {
    let mut files = FILES.lock();

    let resource: Option<&mut File> = files.get_mut(&(fd as i8));

    match resource {
        Some(resource) => resource.flush(),
        None => Err(crate::internal::file::FileError::FlushError(FsError::InvalidFileDescriptor.into(),
        )),
    }
}
