use alloc::string::ToString;
use spin::Mutex;

use crate::internal::{
    console::Console,
    devices::{null::Null, rand::Rand, zero::Zero},
    file::Stream,
    fs::{FileHandle, FsError},
    io::{Device, FILES, File},
};

/// Error number of the last error
pub static ERRNO: Mutex<u8> = Mutex::new(0);

/// Error codes
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

/// read from a file descriptor
pub const READ: u64 = 0x1;
/// write to a file descriptor
pub const WRITE: u64 = 0x2;
/// open a file and return a file descriptor
pub const OPEN: u64 = 0x3;
/// close a file descriptor
pub const CLOSE: u64 = 0x4;
/// flush a file descriptor
pub const FLUSH: u64 = 0x5;
/// exit the current process
pub const EXIT: u64 = 0x6;
/// sleep for a number of milliseconds
pub const SLEEP: u64 = 0x7;
/// wait for a child process to exit
pub const WAIT: u64 = 0x8;
/// get the process ID
pub const GETPID: u64 = 0x9;
/// execute a new process
pub const EXEC: u64 = 0xA;
/// fork the current process
pub const FORK: u64 = 0xB;
/// get the thread ID
pub const GETTID: u64 = 0xC;
/// stop the current process
pub const STOP: u64 = 0xD;
/// wait for a child process to exit
pub const WAITPID: u64 = 0xE;
/// connect to a socket
pub const CONNECT: u64 = 0xF;
/// accept a connection on a socket
pub const ACCEPT: u64 = 0x10;
/// listen for connections on a socket
pub const LISTEN: u64 = 0x11;
/// allocate memory
pub const ALLOC: u64 = 0x12;
/// free memory
pub const FREE: u64 = 0x13;
/// get the kind of the current process
pub const KIND: u64 = 0x14;
/// get the last error number
pub const GETERRNO: u64 = 0x15;

/// get the last error number (GETERRNO)
pub fn get_errno() -> u8 {
    *ERRNO.lock()
}

/// allocate memory (ALLOC)
pub fn alloc(size: usize) -> *mut u8 {
    let layout = core::alloc::Layout::from_size_align(size, 1).unwrap();
    unsafe { alloc::alloc::alloc(layout) }
}

/// free memory (FREE)
pub fn free(ptr: *mut u8, size: usize) {
    let layout = core::alloc::Layout::from_size_align(size, 1).unwrap();
    unsafe { alloc::alloc::dealloc(ptr, layout) }
}

/// test the alloc and free functions
#[test_case]
fn test_alloc_free() {
    let heap_value = alloc(1024);
    assert!(!heap_value.is_null());
    free(heap_value, 1024);
}

/// open a file (OPEN)
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

/// write to a file descriptor (WRITE)
pub fn write(fd: u8, buf: &[u8]) -> Result<usize, crate::internal::file::FileError> {
    let mut files = FILES.lock();

    let resource: Option<&mut File> = files.get_mut(&(fd as i8));

    match resource {
        Some(resource) => resource.write(buf),
        None => Err(crate::internal::file::FileError::WriteError(FsError::InvalidFileDescriptor.into())),
    }
}

/// read from a file descriptor (READ)
pub fn read(fd: u8, buf: &mut [u8]) -> Result<usize, crate::internal::file::FileError> {
    let mut files = FILES.lock();

    let resource: Option<&mut File> = files.get_mut(&(fd as i8));

    match resource {
        Some(resource) => resource.read(buf),
        None => Err(crate::internal::file::FileError::ReadError(FsError::InvalidFileDescriptor.into())),
    }
}

/// close a file descriptor (CLOSE)
pub fn close(fd: u8) -> Result<(), crate::internal::file::FileError> {
    let mut files = FILES.lock();

    let resource: Option<File> = files.remove(&(fd as i8));

    match resource {
        Some(mut resource) => resource.close(),
        None => Err(crate::internal::file::FileError::CloseError(FsError::InvalidFileDescriptor.into(),
        )),
    }
}

/// flush a file descriptor (FLUSH)
pub fn flush(fd: u8) -> Result<(), crate::internal::file::FileError> {
    let mut files = FILES.lock();

    let resource: Option<&mut File> = files.get_mut(&(fd as i8));

    match resource {
        Some(resource) => resource.flush(),
        None => Err(crate::internal::file::FileError::FlushError(FsError::InvalidFileDescriptor.into(),
        )),
    }
}
