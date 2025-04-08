use core::arch::asm;

use alloc::string::ToString;
use log::{trace, warn};
use spin::Mutex;

use crate::{internal::{
    file::Stream,
    fs::FileHandle,
    io::{Device, File, FILES}, process::ExitCode,
}, kprintln};

use super::{
    file::{self, IOEvent},
    io,
};

/// Error number of the last error
pub static ERRNO: Mutex<usize> = Mutex::new(0);

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

fn utf8_from_raw_parts(ptr: *mut u8, len: usize) -> &'static str {
    unsafe {
        let slice = core::slice::from_raw_parts(ptr, len);
        core::str::from_utf8_unchecked(slice)
    }
}

/// read from a file descriptor - `read(fd, buf, len)`
pub const READ: usize = 0x1;
/// write to a file descriptor - `write(fd, buf, len)`
pub const WRITE: usize = 0x2;
/// open a file and return a file descriptor - `open(path, path_len, flags)`
pub const OPEN: usize = 0x3;
/// close a file descriptor - `close(fd)`
pub const CLOSE: usize = 0x4;
/// flush a file descriptor - `flush(fd)`
pub const FLUSH: usize = 0x5;
/// exit the current process - `exit(status)`
pub const EXIT: usize = 0x6;
/// sleep for a number of milliseconds - `sleep(ms)`
pub const SLEEP: usize = 0x7;
/// wait for a number of nanoseconds - `wait(nanos)`. Note that this is not POSIX/Linux WAIT (waitpid-like) syscall.
pub const WAIT: usize = 0x8;
/// get the process ID // `getpid()`
pub const GETPID: usize = 0x9;
/// execute a new process - `exec(path, path_len)`
pub const EXEC: usize = 0xA;
/// fork the current process - `fork()`
pub const FORK: usize = 0xB;
/// get the thread ID - `gettid()`
pub const GETTID: usize = 0xC;
/// stop the current process - `stop(type)`
pub const STOP: usize = 0xD;
/// wait for a child process to exit - `waitpid(pid, status)`
pub const WAITPID: usize = 0xE;
/// connect to a socket - `connect(fd, addr, addr_len)`
pub const CONNECT: usize = 0xF;
/// accept a connection on a socket - `accept(fd, addr, addr_len)`
pub const ACCEPT: usize = 0x10;
/// listen for connections on a socket - `listen(fd, backlog)`
pub const LISTEN: usize = 0x11;
/// allocate memory - `alloc(size)`
pub const ALLOC: usize = 0x12;
/// free memory - `free(ptr)`
pub const FREE: usize = 0x13;
/// get the kind of the current process - `kind()`
pub const KIND: usize = 0x14;
/// get the last error number - `get_errno()`
pub const GETERRNO: usize = 0x15;
/// poll a file descriptor - `poll(fd, event)`
pub const POLL: usize = 0x16;
/// get the number of nanoseconds since boot - `boot_time()`
pub const BOOTTIME: usize = 0x17;
/// get the number of seconds since 1970-01-01T00:00:00Z - `unix_time()`
pub const TIME: usize = 0x18;
/// seek to a position in a file descriptor - `seek(fd, pos)`
pub const SEEK: usize = 0x19;

fn syscall_name(n: usize) -> &'static str {
    match n {
        READ => "read",
        WRITE => "write",
        OPEN => "open",
        CLOSE => "close",
        FLUSH => "flush",
        EXIT => "exit",
        SLEEP => "sleep",
        WAIT => "wait",
        GETPID => "getpid",
        EXEC => "exec",
        FORK => "fork",
        GETTID => "gettid",
        STOP => "stop",
        WAITPID => "waitpid",
        CONNECT => "connect",
        ACCEPT => "accept",
        LISTEN => "listen",
        ALLOC => "alloc",
        FREE => "free",
        KIND => "kind",
        GETERRNO => "get_errno",
        POLL => "poll",
        BOOTTIME => "boot_time",
        TIME => "unix_time",
        SEEK => "seek",
        _ => "<unknown>",
    }
}

/// internal syscall module
mod service;
pub use service::init;

/// Dispatch a syscall, given the syscall number and arguments
pub fn dispatch(n: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize) -> isize {
    trace!("syscall: {}: {} {} {}, {}", syscall_name(n), arg1, arg2, arg3, arg4);
    match n {
        READ => {
            let fd = arg1;
            let actual_addr = crate::internal::process::ptr_from_addr(arg2 as u64);
            let buf = unsafe { core::slice::from_raw_parts_mut(actual_addr, arg3) };


            service::read(fd, buf)
        }
        WRITE => {
            let fd = arg1;
            let actual_addr = crate::internal::process::ptr_from_addr(arg2 as u64);
            let buf = unsafe { core::slice::from_raw_parts(actual_addr, arg3) };

            service::write(fd, buf)
        }
        OPEN => {
            let path_addr = crate::internal::process::ptr_from_addr(arg1 as u64);
            let path = utf8_from_raw_parts(path_addr, arg2);
            let flags = arg3;

            service::open(path, flags as u8)
        }
        CLOSE => {
            let fd = arg1;

            service::close(fd)
        }
        FLUSH => {
            let fd = arg1;

            service::flush(fd)
        }
        EXIT => {
            service::exit(ExitCode::from(arg1 as u8)) as isize
        }
        SLEEP => {
            let ns = arg1;

            service::sleep(ns)
        }
        WAIT => {
            let nanos: usize = arg1;

            service::wait(nanos)
        }
        GETPID => {
            unimplemented!("GETPID")
        }
        EXEC => {
            let path_addr = crate::internal::process::ptr_from_addr(arg1 as u64);
            let path = utf8_from_raw_parts(path_addr, arg2);

            service::spawn(path, arg3, arg4)
        }
        FORK => {
            unimplemented!("FORK")
        }
        GETTID => {
            unimplemented!("GETTID")
        }
        STOP => {
            let kind = arg1;
            service::stop(kind)
        }
        WAITPID => {
            unimplemented!("WAITPID")
        }
        CONNECT => {
            unimplemented!("CONNECT")
        }
        ACCEPT => {
            unimplemented!("ACCEPT")
        }
        LISTEN => {
            unimplemented!("LISTEN")
        }
        ALLOC => {
            let size = arg1;
            let align = arg2;

            service::alloc(size, align) as isize
        }
        FREE => {
            let ptr = arg1 as *mut u8;
            let size = arg2;
            let align = arg3;

            service::free(ptr, size, align);
            0
        }
        KIND => {
            unimplemented!("KIND")
        }
        GETERRNO => *ERRNO.lock() as isize,
        POLL => {
            let fd = arg1;
            let event = arg2;

            service::poll(fd, event)
        }
        BOOTTIME => {
            service::nanos() as isize
        }
        TIME => {
            service::time() as isize
        }
        SEEK => {
            let fd = arg1;
            let pos = arg2;

            service::seek(fd, pos)
        }
        _ => {
            warn!("Unknown syscall: {}", n);
            -1
        }
    }
}

#[doc(hidden)]
pub unsafe fn syscall0(n: usize) -> usize {
    let res: usize;
    unsafe {
        asm!(
            "int 0x80", in("rax") n,
            lateout("rax") res
        );
    }
    res
}

#[doc(hidden)]
pub unsafe fn syscall1(n: usize, arg1: usize) -> usize {
    let res: usize;
    unsafe {
        asm!(
            "int 0x80", in("rax") n,
            in("rdi") arg1,
            lateout("rax") res
        );
    }
    res
}

#[doc(hidden)]
pub unsafe fn syscall2(n: usize, arg1: usize, arg2: usize) -> usize {
    let res: usize;
    unsafe {
        asm!(
            "int 0x80", in("rax") n,
            in("rdi") arg1, in("rsi") arg2,
            lateout("rax") res
        );
    }
    res
}

#[doc(hidden)]
pub unsafe fn syscall3(n: usize, arg1: usize, arg2: usize, arg3: usize) -> usize {
    let res: usize;
    unsafe {
        asm!(
            "int 0x80", in("rax") n,
            in("rdi") arg1, in("rsi") arg2, in("rdx") arg3,
            lateout("rax") res
        );
    }
    res
}

#[doc(hidden)]
pub unsafe fn syscall4(n: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize) -> usize {
    let res: usize;
    unsafe {
        asm!(
            "int 0x80", in("rax") n,
            in("rdi") arg1, in("rsi") arg2, in("rdx") arg3, in("r8") arg4,
            lateout("rax") res
        );
    }
    res
}

/// syscall! macro
#[macro_export]
macro_rules! syscall {
    ($n:expr) => {
        unsafe { $crate::internal::syscall::syscall0($n as usize) }
    };
    ($n:expr, $arg1:expr) => {
        unsafe { $crate::internal::syscall::syscall1($n as usize, $arg1 as usize) }
    };
    ($n:expr, $arg1:expr, $arg2:expr) => {
        unsafe { $crate::internal::syscall::syscall2($n as usize, $arg1 as usize, $arg2 as usize) }
    };
    ($n:expr, $arg1:expr, $arg2:expr, $arg3:expr) => {
        unsafe {
            $crate::internal::syscall::syscall3(
                $n as usize,
                $arg1 as usize,
                $arg2 as usize,
                $arg3 as usize,
            )
        }
    };
    ($n:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr) => {
        unsafe {
            $crate::internal::syscall::syscall4(
                $n as usize,
                $arg1 as usize,
                $arg2 as usize,
                $arg3 as usize,
                $arg4 as usize,
            )
        }
    };
}
