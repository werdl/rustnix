use core::{
    arch::asm,
    sync::atomic::{AtomicUsize, Ordering},
};

use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::alloc::{GlobalAlloc, Layout};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use linked_list_allocator::LockedHeap;
use log::warn;
use object::{Object, ObjectSegment};
use spin::RwLock;
use x86_64::{
    VirtAddr,
    registers::control::Cr3,
    structures::{
        idt::InterruptStackFrameValue,
        paging::{
            FrameAllocator, OffsetPageTable, PageTable, PageTableFlags, PhysFrame, Translate,
            mapper::TranslateResult,
        },
    },
};

use crate::internal::gdt::GDT;

use super::{
    console::Console,
    devices::null::Null,
    io::{Device, File, Stderr, Stdout},
    memory, user,
};

// todo
#[repr(align(8), C)]
#[derive(Debug, Clone, Copy, Default)]
#[allow(missing_docs)]
pub struct Registers {
    pub r11: usize,
    pub r10: usize,
    pub r9: usize,
    pub r8: usize,
    pub rdi: usize,
    pub rsi: usize,
    pub rdx: usize,
    pub rcx: usize,
    pub rax: isize,
}

/// magic number present in the ELF header
const ELF_MAGIC: [u8; 4] = [0x7F, 0x45, 0x4C, 0x46];

/// magic number present in the BIN file
const BIN_MAGIC: [u8; 4] = [0x7F, 0x42, 0x49, 0x4E];

const MAX_HANDLES: usize = 64;
const MAX_PROCESSES: usize = 4;
const MAX_PROC_SIZE: usize = 10 << 20; // 10 MB

static USER_ADDR: usize = 0x800000;
static CODE_ADDR: AtomicUsize = AtomicUsize::new(0);
static PID: AtomicUsize = AtomicUsize::new(0);
static MAX_PID: AtomicUsize = AtomicUsize::new(0);

lazy_static! {
    static ref PROCESSES: RwLock<[Box<Process>; MAX_PROCESSES]> =
        RwLock::new([(); MAX_PROCESSES].map(|_| Box::new(Process::new(None, "/"))));
}

/// represents a process
#[derive(Clone)]
pub struct Process {
    /// process id
    pub pid: usize,
    /// parent process id
    pub ppid: usize,
    /// code address
    pub code_addr: usize,
    /// stack address
    pub stack_addr: usize,
    /// address of the entry point
    pub entry_point_addr: usize,
    /// page table frame
    pub page_table_frame: PhysFrame,
    /// stack frame
    pub stack_frame: Option<InterruptStackFrameValue>,
    /// registers
    pub registers: Registers,
    /// file handles
    pub handles: [Option<Box<File>>; MAX_HANDLES],
    /// environment variables
    pub env: HashMap<String, String>,
    /// user id
    pub uid: Option<u64>,
    /// working directory
    pub cwd: String,
    /// allocator
    pub allocator: Arc<LockedHeap>,
}

/// get the current process id
pub fn pid() -> usize {
    PID.load(Ordering::SeqCst)
}

/// set the current process id
pub fn set_pid(pid: usize) {
    PID.store(pid, Ordering::SeqCst);
}

/// get an environment variable
pub fn get_env(key: &str) -> Option<String> {
    PROCESSES.read()[pid()].env.get(key).cloned()
}

/// set an environment variable
pub fn set_env(key: &str, value: &str) {
    PROCESSES.write()[pid()]
        .env
        .insert(key.to_string(), value.to_string());
}

/// get a map of all environment variables
pub fn get_full_env() -> HashMap<String, String> {
    PROCESSES.read()[pid()].env.clone()
}

/// get the current working directory
pub fn cwd() -> String {
    PROCESSES.read()[pid()].cwd.clone()
}

/// set the current working directory
pub fn set_cwd(cwd: &str) {
    PROCESSES.write()[pid()].cwd = cwd.to_string();
}

/// get the current user id
pub fn uid() -> Option<u64> {
    PROCESSES.read()[pid()].uid
}

/// set the current user id
pub fn set_uid(uid: u64) {
    PROCESSES.write()[pid()].uid = Some(uid);
}

/// create a handle
pub fn create_handle(file: File) -> Result<usize, ()> {
    let handles = &mut PROCESSES.write()[pid()].handles;
    for i in 0..MAX_HANDLES {
        if handles[i].is_none() {
            handles[i] = Some(Box::new(file));
            return Ok(i);
        }
    }
    Err(())
}

/// get a handle
pub fn get_handle(handle: usize) -> Option<Box<File>> {
    PROCESSES.read()[pid()].handles[handle].clone()
}

/// remove a handle
pub fn remove_handle(handle: usize) {
    PROCESSES.write()[pid()].handles[handle] = None;
}

/// update a handle
pub fn update_handle(handle: usize, file: File) {
    PROCESSES.write()[pid()].handles[handle] = Some(Box::new(file));
}

/// get code address
pub fn code_addr() -> usize {
    PROCESSES.read()[pid()].code_addr
}

/// set code address
pub fn set_code_addr(addr: usize) {
    PROCESSES.write()[pid()].code_addr = addr;
}

/// convert an address to a pointer
pub fn addr_to_ptr(addr: u64) -> *mut u8 {
    let base = code_addr();
    if (addr as usize) < base {
        (base + addr as usize) as *mut u8
    } else {
        addr as *mut u8
    }
}

/// get the registers
pub fn get_registers() -> Registers {
    PROCESSES.read()[pid()].registers
}

/// set the registers
pub fn set_registers(registers: Registers) {
    PROCESSES.write()[pid()].registers = registers;
}

/// get the stack frame
pub fn get_stack_frame() -> Option<InterruptStackFrameValue> {
    PROCESSES.read()[pid()].stack_frame.clone()
}

/// set the stack frame
pub fn set_stack_frame(stack_frame: InterruptStackFrameValue) {
    PROCESSES.write()[pid()].stack_frame = Some(stack_frame);
}

/// check if a given address is in userspace
pub fn is_userspace(addr: usize) -> bool {
    USER_ADDR <= addr && addr <= USER_ADDR + MAX_PROC_SIZE
}

/// get the page table frame
pub fn page_table_frame() -> PhysFrame {
    PROCESSES.read()[pid()].page_table_frame
}

/// exit the current process
pub fn exit() {
    let table = PROCESSES.read();
    let proc = &table[pid()];

    MAX_PID.fetch_sub(1, Ordering::SeqCst);
    set_pid(proc.ppid);

    unsafe {
        let (_, flags) = Cr3::read();
        Cr3::write(page_table_frame(), flags);
    }

    proc.free_pages();
}

/// create a page table
pub unsafe fn page_table() -> &'static mut PageTable {
    memory::create_page_table(page_table_frame())
}

/// allocate memory on the current process
pub unsafe fn alloc(layout: Layout) -> *mut u8 {
    unsafe { PROCESSES.write()[pid()].allocator.alloc(layout) }
}

/// free memory on the current process
pub unsafe fn free(ptr: *mut u8, layout: Layout) {
    let proc = &PROCESSES.read()[pid()];
    let bottom = proc.allocator.lock().bottom();
    let top = proc.allocator.lock().top();
    if bottom <= ptr && ptr < top {
        unsafe { proc.allocator.dealloc(ptr, layout) }
    } else {
        // FIXME: Uncomment to see errors
        warn!("freeing failed on {:#x}", ptr as usize);
    }
}

/// program exit codes
pub enum ExitCode {
    /// generic success
    Ok = 0,
    /// generic error
    Err = 1,

    /// used incorrectly
    UsageError = 64,
    /// invalid data
    DataError = 65,

    /// input/output error
    OpenError = 128,
    /// read error
    ReadError = 129,
    /// exec error
    ExecError = 130,
    /// page fault error
    PageFaultError = 200,
    /// shell exited
    ShellExit = 255,
}

impl Process {
    /// create a new process - if no user is specified, the process will run as root
    pub fn new(user: Option<&str>, cwd: &str) -> Self {
        let mut handles = [const { None }; MAX_HANDLES];
        // stdin
        handles[0] = Some(Box::new(File::Device(Device::Stdin(Console::new()))));
        // stdout
        handles[1] = Some(Box::new(File::Device(Device::Stdout(Stdout::new()))));
        // stderr
        handles[2] = Some(Box::new(File::Device(Device::Stderr(Stderr::new()))));
        // null
        handles[3] = Some(Box::new(File::Device(Device::Null(Null::new(0)))));

        let user = match user {
            Some(user) => user::get_uid(user),
            None => None,
        };

        Process {
            pid: 0,
            ppid: 0,
            code_addr: 0,
            stack_addr: 0,
            entry_point_addr: 0,
            page_table_frame: Cr3::read().0,
            stack_frame: None,
            registers: Registers::default(),
            handles: handles,
            env: HashMap::new(),
            uid: user,
            cwd: cwd.to_string(),
            allocator: Arc::new(LockedHeap::empty()),
        }
    }

    /// spawn a new process
    pub fn spawn(bin: &[u8], args_ptr: usize, args_len: usize) -> Result<(), ExitCode> {
        if let Ok(id) = Self::create(bin) {
            let proc = {
                let table = PROCESSES.read();
                table[id].clone()
            };
            proc.exec(args_ptr, args_len);
            unreachable!(); // The kernel switched to the child process
        } else {
            Err(ExitCode::ExecError)
        }
    }

    fn create(bin: &[u8]) -> Result<usize, &str> {
        if MAX_PID.load(Ordering::SeqCst) >= MAX_PROCESSES {
            return Err("max processes reached");
        }

        let page_table_frame = memory::frame_allocator()
            .allocate_frame()
            .ok_or("failed to allocate page frame")?;

        let page_table =  memory::create_page_table(page_table_frame);
        let kpage_table = unsafe { memory::active_level_4_table() };

        let pages = page_table.iter_mut().zip(kpage_table.iter());
        for (upage, kpage) in pages {
            *upage = kpage.clone();
        }

        let mut mapper =
            unsafe { OffsetPageTable::new(page_table, memory::physical_memory_offset()) };

        let code_addr = CODE_ADDR.fetch_add(MAX_PROC_SIZE, Ordering::SeqCst);
        let stack_addr = code_addr + MAX_PROC_SIZE - 0x1000; // 4KB stack

        let mut entry_point_addr = 0;

        if bin[0..4] == ELF_MAGIC {
            if let Ok(obj) = object::File::parse(bin) {
                entry_point_addr = obj.entry();

                for segment in obj.segments() {
                    if let Ok(data) = segment.data() {
                        let addr = code_addr + (segment.address() as usize);
                        let size = segment.size() as usize;
                        load_binary(&mut mapper, addr, size, data)
                            .map_err(|_| "failed to load binary")?;
                    }
                }
            } else {
                warn!("failed to parse ELF file");
            }
        } else if bin[0..4] == BIN_MAGIC {
            load_binary(&mut mapper, code_addr, bin.len() - 4, &bin[4..])
                .map_err(|_| "failed to load binary")?;
        } else {
            return Err("invalid binary format");
        }

        let parent = PROCESSES.read()[pid()].clone();

        let handles = parent.handles.clone();
        let env = parent.env.clone();
        let uid = parent.uid;
        let cwd = parent.cwd.clone();
        let registers = parent.registers;

        let allocator = Arc::new(LockedHeap::empty());

        let pid = MAX_PID.fetch_add(1, Ordering::SeqCst);
        let ppid = parent.pid;

        let proc = Process {
            pid,
            ppid,
            code_addr,
            stack_addr,
            entry_point_addr: entry_point_addr as usize,
            page_table_frame,
            stack_frame: None,
            registers,
            handles,
            env,
            uid,
            cwd,
            allocator,
        };

        let mut table = PROCESSES.write();
        table[pid] = Box::new(proc);

        Ok(pid)
    }

    /// switch to user mode and execute
    fn exec(&self, args_ptr: usize, args_len: usize) {
        let page_table = unsafe { page_table() };
        let mut mapper =
            unsafe { OffsetPageTable::new(page_table, memory::physical_memory_offset()) };

        // copy arguments to userspace
        let args_addr = self.code_addr + (self.stack_addr - self.code_addr) / 2;
        memory::alloc_pages(&mut mapper, args_addr, 1).unwrap();

        let args: &[&str] = unsafe {
            let ptr = addr_to_ptr(args_ptr as u64) as usize;
            core::slice::from_raw_parts(ptr as *const &str, args_len)
        };

        let mut addr = args_addr;
        let vec: Vec<&str> = args
            .iter()
            .map(|arg| {
                let ptr = addr as *mut u8;
                addr += arg.len();
                unsafe {
                    let s = core::slice::from_raw_parts_mut(ptr, arg.len());
                    s.copy_from_slice(arg.as_bytes());
                    core::str::from_utf8_unchecked(s)
                }
            })
            .collect();

        let align = core::mem::align_of::<&str>();
        addr += align - (addr % align);

        let args = vec.as_slice();
        let ptr = addr as *mut &str;
        let args: &[&str] = unsafe {
            let s = core::slice::from_raw_parts_mut(ptr, args.len());
            s.copy_from_slice(args);
            s
        };

        let args_ptr = args.as_ptr() as usize;

        let heap_addr = addr + 4096;

        let heap_size = ((self.stack_addr - heap_addr) / 2) as usize; // 4096 = heap size

        unsafe {
            self.allocator.lock().init(heap_addr as *mut u8, heap_size);
        }

        set_pid(self.pid);

        unsafe {
            let (_, flags) = Cr3::read();
            Cr3::write(self.page_table_frame, flags);

            asm!(
                "cli",        // Disable interrupts
                "push {:r}",  // Stack segment (SS)
                "push {:r}",  // Stack pointer (RSP)
                "push 0x200", // RFLAGS with interrupts enabled
                "push {:r}",  // Code segment (CS)
                "push {:r}",  // Instruction pointer (RIP)
                "iretq",
                in(reg) GDT.1.user_data_selector.0,
                in(reg) self.stack_addr,
                in(reg) GDT.1.user_code_selector.0,
                in(reg) self.code_addr + self.entry_point_addr,
                in("rdi") args_ptr,
                in("rsi") args_len,
            );
        }
    }

    fn mapper(&self) -> OffsetPageTable<'static> {
        let page_table = memory::create_page_table(self.page_table_frame);
        unsafe { OffsetPageTable::new(page_table, memory::physical_memory_offset()) }
    }

    fn free_pages(&self) {
        let mut mapper = self.mapper();

        let size = MAX_PROC_SIZE;
        memory::free_pages(&mut mapper, self.code_addr, size);

        let addr = USER_ADDR;
        match mapper.translate(VirtAddr::new(addr as u64)) {
            TranslateResult::Mapped {
                frame: _,
                flags,
                offset: _,
            } => {
                if flags.contains(PageTableFlags::USER_ACCESSIBLE) {
                    memory::free_pages(&mut mapper, addr, size);
                }
            }
            _ => {}
        }
    }
}

fn load_binary(
    mapper: &mut OffsetPageTable,
    addr: usize,
    size: usize,
    buf: &[u8],
) -> Result<(), ()> {
    debug_assert!(size >= buf.len());
    memory::alloc_pages(mapper, addr, size)?;
    let src = buf.as_ptr();
    let dst = addr as *mut u8;
    unsafe {
        core::ptr::copy_nonoverlapping(src, dst, buf.len());
        if size > buf.len() {
            core::ptr::write_bytes(dst.add(buf.len()), 0, size - buf.len());
        }
    }
    Ok(())
}
