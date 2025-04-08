typedef unsigned long usize;
typedef unsigned char u8;
typedef unsigned long long u64;

// Inline assembly for system calls
static inline usize syscall0(usize n) {
    usize res;
    asm volatile (
        "int $0x80"
        : "=a"(res)
        : "a"(n)
        : "memory"
    );
    return res;
}

static inline usize syscall1(usize n, usize arg1) {
    usize res;
    asm volatile (
        "int $0x80"
        : "=a"(res)
        : "a"(n), "D"(arg1)
        : "memory"
    );
    return res;
}

static inline usize syscall2(usize n, usize arg1, usize arg2) {
    usize res;
    asm volatile (
        "int $0x80"
        : "=a"(res)
        : "a"(n), "D"(arg1), "S"(arg2)
        : "memory"
    );
    return res;
}

static inline usize syscall3(usize n, usize arg1, usize arg2, usize arg3) {
    usize res;
    asm volatile (
        "int $0x80"
        : "=a"(res)
        : "a"(n), "D"(arg1), "S"(arg2), "d"(arg3)
        : "memory"
    );
    return res;
}

static inline usize syscall4(usize n, usize arg1, usize arg2, usize arg3, usize arg4) {
    usize res;
    register usize r8 asm("r8") = arg4;
    asm volatile (
        "int $0x80"
        : "=a"(res)
        : "a"(n), "D"(arg1), "S"(arg2), "d"(arg3), "r"(r8)
        : "memory"
    );
    return res;
}

// System call constants
#define READ 0x1
#define WRITE 0x2
#define OPEN 0x3
#define CLOSE 0x4
#define FLUSH 0x5
#define EXIT 0x6
#define SLEEP 0x7
#define WAIT 0x8
#define GETPID 0x9
#define SPAWN 0xA
#define FORK 0xB
#define GETTID 0xC
#define STOP 0xD
#define WAITPID 0xE
#define CONNECT 0xF
#define ACCEPT 0x10
#define LISTEN 0x11
#define ALLOC 0x12
#define FREE 0x13
#define KIND 0x14
#define GETERRNO 0x15
#define POLL 0x16
#define BOOTTIME 0x17
#define TIME 0x18
#define SEEK 0x19

typedef long isize;

// Function implementations
usize spawn(const char *path, const char **args, usize path_len, usize args_len) {
    usize res = syscall4(SPAWN, (usize)path, path_len, (usize)args, args_len);
    return res;
}

isize write(usize fd, const char *string, usize len) {
    usize res = syscall3(WRITE, fd, (usize)string, len);
    return (isize)res;
}

isize open(const char *path, u8 flags, usize path_len) {
    usize res = syscall3(OPEN, (usize)path, path_len, (usize)flags);
    return (isize)res;
}

void *alloc(usize size, usize align) {
    return (void *)syscall2(ALLOC, size, align);
}

void free(void *ptr, usize size, usize align) {
    syscall3(FREE, (usize)ptr, size, align);
}

usize boot_time() {
    return syscall0(BOOTTIME);
}

u64 unix_time() {
    return (u64)syscall0(TIME);
}

usize get_errno() {
    return syscall0(GETERRNO);
}

void exit(u8 code) {
    syscall1(EXIT, (usize)code);
    while (1) {} // Infinite loop to prevent returning
}
