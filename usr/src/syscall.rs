use core::{arch::asm, sync::atomic::{fence, Ordering}};

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
        unsafe { $crate::syscall::syscall0($n as usize) }
    };
    ($n:expr, $arg1:expr) => {
        unsafe { $crate::syscall::syscall1($n as usize, $arg1 as usize) }
    };
    ($n:expr, $arg1:expr, $arg2:expr) => {
        unsafe { $crate::syscall::syscall2($n as usize, $arg1 as usize, $arg2 as usize) }
    };
    ($n:expr, $arg1:expr, $arg2:expr, $arg3:expr) => {
        unsafe {
            $crate::syscall::syscall3(
                $n as usize,
                $arg1 as usize,
                $arg2 as usize,
                $arg3 as usize,
            )
        }
    };
    ($n:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr) => {
        unsafe {
            $crate::syscall::syscall4(
                $n as usize,
                $arg1 as usize,
                $arg2 as usize,
                $arg3 as usize,
                $arg4 as usize,
            )
        }
    };
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
/// execute a new process - `exec(path, path_len, args_ptr, args_len)`
pub const SPAWN: usize = 0xA;
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

pub fn spawn(path: &str, args: &[&str]) -> crate::process::ExitCode {
    let path_ptr = path.as_ptr() as usize;
    let args_ptr = args.as_ptr() as usize;
    let path_len = path.len();
    let args_len = args.len();
    let res = syscall!(SPAWN, path_ptr, path_len, args_ptr, args_len);

    // Without the fence `res` would always be `0` instead of the code passed
    // to the `exit` syscall by the child process.
    fence(Ordering::SeqCst);

    crate::process::ExitCode::from(res as u8)
}

pub fn write(fd: usize, string: &str) -> isize {
    let buf = string.as_ptr() as usize;
    let len = string.len();
    syscall!(WRITE, fd, buf, len) as isize
}

pub fn exit(code: u8) -> ! {
    syscall!(EXIT, code as usize);
    loop {}
}
